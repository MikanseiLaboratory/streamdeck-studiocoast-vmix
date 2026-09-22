use std::collections::{HashMap, VecDeque};
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TrySendError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use thiserror::Error;
use tokio::sync::{broadcast, oneshot};
use vmix_rs::vmix_tcp::commands::Status;
use vmix_rs::vmix_tcp::{RecvCommand, SUBSCRIBECommand, SendCommand, VmixApi};

use crate::cache::{self, VmixState};
use crate::config::{TargetGroup, VmixInstanceConfig};
use crate::status::{ConnectionStatus, InstanceStatus};

/// Tunables for connection and retry behaviour.
#[derive(Clone, Debug)]
pub struct PoolOptions {
    pub connect_timeout: Duration,
    pub initial_backoff: Duration,
    pub max_backoff: Duration,
}

impl Default for PoolOptions {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            initial_backoff: Duration::from_millis(500),
            max_backoff: Duration::from_secs(30),
        }
    }
}

impl PoolOptions {
    pub fn for_tests() -> Self {
        Self {
            connect_timeout: Duration::from_secs(2),
            initial_backoff: Duration::from_millis(40),
            max_backoff: Duration::from_millis(200),
        }
    }
}

/// Outbound command owned by the instance thread.
#[derive(Clone, Debug)]
pub enum Command {
    Function { name: String, query: Option<String> },
    Raw(String),
}

/// Events emitted by every supervised instance.
#[derive(Clone, Debug)]
pub enum PoolEvent {
    Status(InstanceStatus),
    Activator { id: String, name: String },
    Snapshot { id: String },
    Function { id: String, ok: bool },
}

#[derive(Debug, Error)]
pub enum CallError {
    #[error("instance `{0}` is not configured")]
    Unknown(String),
    #[error("instance `{0}` is not connected")]
    Unavailable(String),
}

struct LiveSend {
    sender: Sender<Command>,
}

struct Slot {
    endpoint: String,
    stop: Arc<AtomicBool>,
    recycle: Arc<AtomicBool>,
}

struct Inner {
    options: PoolOptions,
    configs: Mutex<Vec<VmixInstanceConfig>>,
    groups: Mutex<Vec<TargetGroup>>,
    slots: Mutex<HashMap<String, Slot>>,
    senders: Mutex<HashMap<String, LiveSend>>,
    generations: Mutex<HashMap<String, u64>>,
    next_generation: AtomicU64,
    statuses: Mutex<HashMap<String, ConnectionStatus>>,
    states: Mutex<HashMap<String, VmixState>>,
    events: broadcast::Sender<PoolEvent>,
}

/// Shared supervisor for every configured vMix instance.
#[derive(Clone)]
pub struct VmixPool {
    inner: Arc<Inner>,
    owners: Arc<()>,
}

impl VmixPool {
    pub fn new(options: PoolOptions) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            owners: Arc::new(()),
            inner: Arc::new(Inner {
                options,
                configs: Mutex::new(Vec::new()),
                groups: Mutex::new(Vec::new()),
                slots: Mutex::new(HashMap::new()),
                senders: Mutex::new(HashMap::new()),
                generations: Mutex::new(HashMap::new()),
                next_generation: AtomicU64::new(1),
                statuses: Mutex::new(HashMap::new()),
                states: Mutex::new(HashMap::new()),
                events,
            }),
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<PoolEvent> {
        self.inner.events.subscribe()
    }

    pub async fn reconcile(&self, configs: Vec<VmixInstanceConfig>, groups: Vec<TargetGroup>) {
        let ids: Vec<String> = configs.iter().map(|config| config.id.clone()).collect();
        *self.inner.configs.lock().expect("configs") = configs;
        *self.inner.groups.lock().expect("groups") = groups;
        let mut slots = self.inner.slots.lock().expect("slots");
        let stale: Vec<String> = slots
            .keys()
            .filter(|id| !ids.iter().any(|keep| keep == *id))
            .cloned()
            .collect();
        for id in stale {
            if let Some(slot) = slots.remove(&id) {
                slot.stop.store(true, Ordering::SeqCst);
            }
            self.inner.senders.lock().expect("senders").remove(&id);
            self.inner.statuses.lock().expect("statuses").remove(&id);
            self.inner.states.lock().expect("states").remove(&id);
        }
        for id in ids {
            let Some(config) = self.config_sync(&id) else {
                continue;
            };
            let endpoint = config.endpoint_key();
            if let Some(slot) = slots.get(&id) {
                if slot.endpoint == endpoint {
                    continue;
                }
                slot.stop.store(true, Ordering::SeqCst);
            }
            let stop = Arc::new(AtomicBool::new(false));
            let recycle = Arc::new(AtomicBool::new(false));
            slots.insert(
                id.clone(),
                Slot {
                    endpoint,
                    stop: stop.clone(),
                    recycle: recycle.clone(),
                },
            );
            let inner = self.inner.clone();
            tokio::spawn(async move {
                supervise(inner, id, stop, recycle).await;
            });
        }
    }

    pub fn configs(&self) -> Vec<VmixInstanceConfig> {
        self.inner.configs.lock().expect("configs").clone()
    }

    pub fn groups(&self) -> Vec<TargetGroup> {
        self.inner.groups.lock().expect("groups").clone()
    }

    pub fn state(&self, id: &str) -> Option<VmixState> {
        self.inner.states.lock().expect("states").get(id).cloned()
    }

    pub fn status(&self, id: &str) -> Option<ConnectionStatus> {
        self.inner.statuses.lock().expect("statuses").get(id).cloned()
    }

    pub fn statuses(&self) -> Vec<InstanceStatus> {
        self.inner
            .configs
            .lock()
            .expect("configs")
            .iter()
            .filter_map(|config| self.inner.instance_status(&config.id))
            .collect()
    }

    pub fn send(&self, id: &str, command: Command) -> Result<(), CallError> {
        if self.config_sync(id).is_none() {
            return Err(CallError::Unknown(id.to_string()));
        }
        let senders = self.inner.senders.lock().expect("senders");
        let Some(live) = senders.get(id) else {
            return Err(CallError::Unavailable(id.to_string()));
        };
        live.sender
            .send(command)
            .map_err(|_| CallError::Unavailable(id.to_string()))
    }

    pub fn reconnect(&self, id: &str) {
        if let Some(slot) = self.inner.slots.lock().expect("slots").get(id) {
            slot.recycle.store(true, Ordering::SeqCst);
        }
    }

    fn config_sync(&self, id: &str) -> Option<VmixInstanceConfig> {
        self.inner
            .configs
            .lock()
            .expect("configs")
            .iter()
            .find(|config| config.id == id)
            .cloned()
    }
}

impl Drop for VmixPool {
    fn drop(&mut self) {
        if Arc::strong_count(&self.owners) == 1 {
            for slot in self.inner.slots.lock().expect("slots").values() {
                slot.stop.store(true, Ordering::SeqCst);
                slot.recycle.store(true, Ordering::SeqCst);
            }
        }
    }
}

impl Inner {
    fn instance_status(&self, id: &str) -> Option<InstanceStatus> {
        let configs = self.configs.lock().expect("configs");
        let config = configs.iter().find(|config| config.id == id)?;
        let status = self
            .statuses
            .lock()
            .expect("statuses")
            .get(id)
            .cloned()
            .unwrap_or(ConnectionStatus::Connecting);
        Some(InstanceStatus {
            id: config.id.clone(),
            name: config.name.clone(),
            color: config.color.clone(),
            host: config.host.clone(),
            port: config.port,
            enabled: config.enabled,
            status,
        })
    }

    fn publish_status(&self, id: &str) {
        if let Some(status) = self.instance_status(id) {
            let _ = self.events.send(PoolEvent::Status(status));
        }
    }

    fn set_status(&self, id: &str, status: ConnectionStatus) {
        self.statuses.lock().expect("statuses").insert(id.to_string(), status);
        self.publish_status(id);
    }

    fn bump(&self, id: &str) -> u64 {
        let generation = self.next_generation.fetch_add(1, Ordering::SeqCst);
        self.generations
            .lock()
            .expect("generations")
            .insert(id.to_string(), generation);
        generation
    }

    fn is_current(&self, id: &str, generation: u64) -> bool {
        self.generations.lock().expect("generations").get(id).copied() == Some(generation)
    }
}

async fn supervise(
    inner: Arc<Inner>,
    id: String,
    stop: Arc<AtomicBool>,
    recycle: Arc<AtomicBool>,
) {
    let mut backoff = inner.options.initial_backoff;
    while !stop.load(Ordering::SeqCst) {
        let Some(config) = inner
            .configs
            .lock()
            .expect("configs")
            .iter()
            .find(|config| config.id == id)
            .cloned()
        else {
            break;
        };
        if !config.enabled {
            inner.set_status(&id, ConnectionStatus::Disabled);
            tokio::time::sleep(Duration::from_millis(200)).await;
            continue;
        }
        let addr = match resolve_endpoint(&config.host, config.port) {
            Ok(addr) => addr,
            Err(message) => {
                tracing::warn!(id = %id, host = %config.host, port = config.port, %message, "vmix address rejected");
                inner.set_status(&id, ConnectionStatus::Unreachable { message });
                tokio::time::sleep(backoff).await;
                backoff = (backoff * 2).min(inner.options.max_backoff);
                continue;
            }
        };
        tracing::info!(id = %id, host = %config.host, %addr, "vmix connecting");
        inner.set_status(&id, ConnectionStatus::Connecting);
        let generation = inner.bump(&id);
        let (tx, rx) = mpsc::channel();
        inner.senders.lock().expect("senders").insert(
            id.clone(),
            LiveSend { sender: tx },
        );
        let (done_tx, done_rx) = oneshot::channel();
        let session_inner = inner.clone();
        let session_id = id.clone();
        let timeout = inner.options.connect_timeout;
        let interval = Duration::from_millis(config.xml_interval_ms);
        let session_recycle = recycle.clone();
        thread::spawn(move || {
            let result = session(
                session_inner,
                session_id,
                addr,
                timeout,
                interval,
                rx,
                generation,
                session_recycle,
            );
            let _ = done_tx.send(result);
        });
        let outcome = done_rx.await;
        if inner.is_current(&id, generation) {
            inner.senders.lock().expect("senders").remove(&id);
        }
        if stop.load(Ordering::SeqCst) {
            break;
        }
        if recycle.swap(false, Ordering::SeqCst) {
            backoff = inner.options.initial_backoff;
            continue;
        }
        let message = match outcome {
            Ok(Err(message)) => message,
            Ok(Ok(())) => "connection closed".into(),
            Err(_) => "connection closed".into(),
        };
        tracing::warn!(id = %id, %addr, %message, "vmix session ended");
        inner.set_status(&id, ConnectionStatus::Unreachable { message });
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(inner.options.max_backoff);
    }
}

/// Prefer IPv4. vMix's TCP API listens on IPv4, and `localhost` often resolves to `::1` first.
fn resolve_endpoint(host: &str, port: u16) -> Result<SocketAddr, String> {
    let host = host.trim();
    if host.is_empty() {
        return Err("host is empty".into());
    }
    let mut matched = None;
    let authority = format!("{host}:{port}");
    let addresses = authority
        .to_socket_addrs()
        .map_err(|error| format!("could not resolve {authority}: {error}"))?;
    for addr in addresses {
        if addr.is_ipv4() {
            return Ok(addr);
        }
        matched.get_or_insert(addr);
    }
    matched.ok_or_else(|| format!("could not resolve {authority}"))
}

fn session(
    inner: Arc<Inner>,
    id: String,
    addr: SocketAddr,
    timeout: Duration,
    interval: Duration,
    commands: Receiver<Command>,
    generation: u64,
    recycle: Arc<AtomicBool>,
) -> Result<(), String> {
    let api = match VmixApi::new(addr, timeout) {
        Ok(api) => api,
        Err(error) => {
            let message = error.to_string();
            tracing::warn!(%id, %addr, %message, "vmix connect failed");
            return Err(message);
        }
    };
    tracing::info!(%id, %addr, "vmix tcp connected");
    // vmix-rs queues a single inbound line. Sending the next command before that
    // line is taken stalls the reader, fills the TCP window, and never reaches VERSION.
    let mut outbound: VecDeque<SendCommand> = startup_commands().into();
    let mut next_poll = Instant::now() + interval;
    let mut announced_xml = false;
    loop {
        if !inner.is_current(&id, generation) || recycle.load(Ordering::SeqCst) {
            return Ok(());
        }
        match api.try_receive_command(Duration::from_millis(20)) {
            Ok(message) => {
                let is_xml = matches!(message, RecvCommand::XML(_));
                handle_message(&inner, &id, message);
                if is_xml && !announced_xml {
                    announced_xml = true;
                    if let Some(state) = inner.states.lock().expect("states").get(&id) {
                        tracing::info!(
                            %id,
                            inputs = state.inputs.len(),
                            mixes = state.mixes_present.len(),
                            version = %state.version,
                            edition = %state.edition,
                            "vmix xml snapshot"
                        );
                    }
                }
            }
            Err(error) => {
                let message = error.to_string();
                let closed = !api.is_connected()
                    || message.to_ascii_lowercase().contains("closed")
                    || message.to_ascii_lowercase().contains("disconnect");
                if closed {
                    tracing::warn!(%id, %addr, %message, "vmix receive failed");
                    return Err(message);
                }
            }
        }
        while let Ok(command) = commands.try_recv() {
            enqueue(&mut outbound, command);
        }
        if outbound.is_empty() && !interval.is_zero() && Instant::now() >= next_poll {
            outbound.push_back(SendCommand::XML);
            outbound.push_back(SendCommand::FUNCTION("ActivatorRefresh".into(), None));
            next_poll = Instant::now() + interval;
        }
        if let Some(command) = outbound.pop_front() {
            match api.sender.try_send(command) {
                Ok(()) => {}
                Err(TrySendError::Full(command)) => outbound.push_front(command),
                Err(TrySendError::Disconnected(_)) => {
                    return Err("tcp sender closed".into());
                }
            }
        }
    }
}

fn startup_commands() -> Vec<SendCommand> {
    let mut commands = vec![
        SendCommand::SUBSCRIBE(SUBSCRIBECommand::ACTS),
        SendCommand::VERSION,
        SendCommand::FUNCTION("ActivatorRefresh".into(), None),
        SendCommand::XML,
    ];
    for name in [
        "Recording",
        "Streaming",
        "External",
        "FadeToBlack",
        "Fullscreen",
        "ReplayPlaying",
        "Input",
        "InputPreview",
    ] {
        commands.push(SendCommand::ACTS(name.to_string(), None));
    }
    for channel in 1..=8 {
        commands.push(SendCommand::ACTS(format!("Overlay{channel}"), None));
    }
    commands
}

fn enqueue(outbound: &mut VecDeque<SendCommand>, command: Command) {
    match command {
        Command::Function { name, query } => {
            outbound.push_back(SendCommand::FUNCTION(name, query));
        }
        Command::Raw(raw) => {
            for line in raw.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let mut payload = line.to_string();
                if !payload.ends_with("\r\n") {
                    payload.push_str("\r\n");
                }
                outbound.push_back(SendCommand::RAW(payload));
            }
        }
    }
}

fn handle_message(inner: &Inner, id: &str, message: RecvCommand) {
    match message {
        RecvCommand::VERSION(response) => {
            let version = response.version.clone().unwrap_or_default();
            tracing::info!(%id, %version, "vmix version");
            let edition = inner
                .states
                .lock()
                .expect("states")
                .get(id)
                .map(|state| state.edition.clone())
                .unwrap_or_default();
            inner.set_status(
                id,
                ConnectionStatus::Connected {
                    vmix_version: version,
                    edition,
                },
            );
        }
        RecvCommand::ACTS(response) => {
            let mut states = inner.states.lock().expect("states");
            let state = states.entry(id.to_string()).or_default();
            let name = cache::apply_acts(state, response.body);
            drop(states);
            let _ = inner.events.send(PoolEvent::Activator {
                id: id.to_string(),
                name,
            });
        }
        RecvCommand::XML(response) => {
            let mut states = inner.states.lock().expect("states");
            let state = states.entry(id.to_string()).or_default();
            if let Err(error) = cache::apply_xml(state, &response.body) {
                tracing::warn!(%id, %error, "vmix xml parse failed");
            }
            let edition = state.edition.clone();
            let version = state.version.clone();
            drop(states);
            if !version.is_empty() {
                inner.set_status(
                    id,
                    ConnectionStatus::Connected {
                        vmix_version: version,
                        edition,
                    },
                );
            }
            let _ = inner.events.send(PoolEvent::Snapshot { id: id.to_string() });
        }
        RecvCommand::FUNCTION(response) => {
            let ok = matches!(response.status, Status::OK);
            let _ = inner.events.send(PoolEvent::Function {
                id: id.to_string(),
                ok,
            });
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockVmix;

    fn config(id: &str, port: u16) -> VmixInstanceConfig {
        VmixInstanceConfig {
            id: id.into(),
            name: id.into(),
            host: "127.0.0.1".into(),
            port,
            color: "#4c8dff".into(),
            enabled: true,
            xml_interval_ms: 0,
        }
    }

    async fn wait_connected(pool: &VmixPool, ids: &[&str]) {
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                if ids.iter().all(|id| pool.status(id).is_some_and(|status| status.is_connected()))
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("instances connected");
    }

    #[test]
    fn localhost_resolves_to_ipv4() {
        let addr = resolve_endpoint("localhost", 8099).expect("localhost");
        assert!(addr.is_ipv4());
        assert_eq!(addr.port(), 8099);
        assert_eq!(
            resolve_endpoint("127.0.0.1", 8099).expect("ipv4"),
            "127.0.0.1:8099".parse().unwrap()
        );
        assert!(resolve_endpoint("  ", 8099).is_err());
    }

    #[tokio::test]
    async fn activator_from_one_instance_does_not_change_the_other() {
        let first = MockVmix::spawn().await;
        let second = MockVmix::spawn().await;
        let pool = VmixPool::new(PoolOptions::for_tests());
        pool.reconcile(
            vec![config("a", first.port), config("b", second.port)],
            vec![],
        )
        .await;
        wait_connected(&pool, &["a", "b"]).await;
        tokio::time::timeout(Duration::from_secs(4), async {
            loop {
                let ready = pool
                    .state("a")
                    .is_some_and(|state| state.program.get(&0) == Some(&1));
                if ready {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("xml snapshot");

        first.push_acts("Overlay8 4 1");
        tokio::time::timeout(Duration::from_secs(4), async {
            loop {
                if pool.state("a").is_some_and(|state| state.overlays.get(&8) == Some(&4)) {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("overlay 8");
        assert!(pool
            .state("b")
            .is_some_and(|state| state.overlays.get(&8).is_none()));
        assert_eq!(pool.state("b").unwrap().program.get(&0), Some(&1));
    }

    #[tokio::test]
    async fn function_command_reaches_only_the_addressed_instance() {
        let first = MockVmix::spawn().await;
        let second = MockVmix::spawn().await;
        let pool = VmixPool::new(PoolOptions::for_tests());
        pool.reconcile(
            vec![config("a", first.port), config("b", second.port)],
            vec![],
        )
        .await;
        wait_connected(&pool, &["a", "b"]).await;
        pool.send(
            "a",
            Command::Function {
                name: "Cut".into(),
                query: Some("Input=2&Mix=1".into()),
            },
        )
        .unwrap();
        tokio::time::timeout(Duration::from_secs(4), async {
            loop {
                if first
                    .commands()
                    .await
                    .iter()
                    .any(|line| line.contains("FUNCTION Cut Input=2&Mix=1"))
                {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("function recorded");
        assert!(!second
            .commands()
            .await
            .iter()
            .any(|line| line.contains("FUNCTION Cut")));
    }
}
