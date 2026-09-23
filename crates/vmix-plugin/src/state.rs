use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use streamdeck_plugin::{async_trait, CommandSender, PluginLifecycle, Result, Target};
use tokio::sync::Mutex;
use vmix_pool::{resolve_targets, PoolEvent, PoolOptions, VmixInstanceConfig, VmixPool};

use crate::contracts::{localhost_instance, ActionSettings, GlobalSettings, TargetSelector, LOCALHOST_ID};
use crate::kind::ActionKind;
use crate::ops;
use crate::render::{self, Segment, SegmentState};

struct LiveKey {
    kind: ActionKind,
    action: String,
    settings: ActionSettings,
    segments: HashMap<String, SegmentState>,
    titles: HashMap<String, String>,
    title: String,
    multi: bool,
}

struct Runtime {
    sender: Mutex<Option<CommandSender>>,
    keys: Mutex<HashMap<String, LiveKey>>,
    global: Mutex<GlobalSettings>,
    pending: Mutex<HashMap<String, String>>,
    /// Serializes the provisional localhost connection with the saved settings.
    connect: Mutex<()>,
    /// Set once Stream Deck has delivered global settings. Until then, do not save defaults.
    settings_applied: AtomicBool,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: VmixPool,
    runtime: Arc<Runtime>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::new(PoolOptions::default())
    }
}

impl AppState {
    pub fn new(options: PoolOptions) -> Self {
        let state = Self {
            pool: VmixPool::new(options),
            runtime: Arc::new(Runtime {
                sender: Mutex::new(None),
                keys: Mutex::new(HashMap::new()),
                global: Mutex::new(GlobalSettings::default()),
                pending: Mutex::new(HashMap::new()),
                connect: Mutex::new(()),
                settings_applied: AtomicBool::new(false),
            }),
        };
        state.spawn_events();
        state
    }

    pub async fn note_sender(&self, sender: CommandSender) {
        let start = {
            let mut current = self.runtime.sender.lock().await;
            if current.is_some() {
                false
            } else {
                if std::env::var_os("RUST_LOG").is_none() {
                    std::env::set_var("RUST_LOG", "info");
                }
                streamdeck_plugin::init_tracing(sender.clone(), "dev.mikanseilaboratory.vmix");
                tracing::info!("vmix plugin ready");
                let _ = sender.get_global_settings(None);
                *current = Some(sender);
                true
            }
        };
        if !start {
            return;
        }
        let state = self.clone();
        tokio::spawn(async move {
            for attempt in 1..=4 {
                tokio::time::sleep(Duration::from_millis(500)).await;
                if state.runtime.settings_applied.load(Ordering::SeqCst) {
                    return;
                }
                tracing::info!(attempt, "requesting vMix global settings again");
                let Some(sender) = state.runtime.sender.lock().await.clone() else {
                    return;
                };
                let _ = sender.get_global_settings(None);
            }
            if !state.runtime.settings_applied.load(Ordering::SeqCst) {
                tracing::warn!("vMix global settings were not received; staying on Localhost 127.0.0.1:8099");
            }
        });
    }

    async fn bootstrap_localhost(&self) {
        self.reconcile_provisional().await;
    }

    /// Connect every id a visible key points at, using 127.0.0.1:8099 until saved settings arrive.
    async fn reconcile_provisional(&self) {
        let _guard = self.runtime.connect.lock().await;
        if self.runtime.settings_applied.load(Ordering::SeqCst) {
            return;
        }
        let ids = {
            let keys = self.runtime.keys.lock().await;
            provisional_ids(keys.values().map(|key| &key.settings.common.target))
        };
        let current = self.pool.configs();
        let same = current.len() == ids.len()
            && ids.iter().all(|id| {
                current
                    .iter()
                    .any(|config| config.id == *id && is_loopback_host(&config.host) && config.port == 8099)
            });
        if same {
            return;
        }
        tracing::info!(
            ids = ?ids,
            "connecting visible vMix targets at 127.0.0.1:8099 before saved settings arrive"
        );
        let configs = ids.into_iter().map(local_instance).collect();
        self.pool.reconcile(configs, Vec::new()).await;
    }

    pub async fn apply_global_value(&self, value: &Value) {
        if value.is_null() {
            tracing::warn!("vMix global settings payload was empty; connecting to Localhost without saving");
            self.bootstrap_localhost().await;
            return;
        }
        match serde_json::from_value::<GlobalSettings>(value.clone()) {
            Ok(settings) => self.apply_global(settings).await,
            Err(error) => {
                tracing::warn!(%error, "could not read vMix global settings; connecting to Localhost without saving");
                self.bootstrap_localhost().await;
            }
        }
    }

    pub async fn apply_global(&self, mut settings: GlobalSettings) {
        let _guard = self.runtime.connect.lock().await;
        self.runtime.settings_applied.store(true, Ordering::SeqCst);
        tracing::info!(
            seeded = settings.seeded,
            instances = settings.instances.len(),
            "received vMix settings"
        );
        for instance in &settings.instances {
            tracing::info!(
                id = %instance.id,
                name = %instance.name,
                host = %instance.host,
                port = instance.port,
                enabled = instance.enabled,
                "vmix instance"
            );
        }
        let mut persist = false;
        if !settings.seeded {
            if settings.instances.is_empty() {
                settings.instances.push(localhost_instance());
                tracing::info!("registered default Localhost vMix at 127.0.0.1:8099");
            }
            settings.seeded = true;
            persist = true;
        }
        if persist {
            if let Some(sender) = self.runtime.sender.lock().await.clone() {
                if let Ok(value) = serde_json::to_value(&settings) {
                    let _ = sender.set_global_settings(&value);
                }
            }
        }
        if settings.instances.is_empty() {
            tracing::warn!("saved vMix list is empty; connecting to Localhost without changing saved settings");
            let instance: VmixInstanceConfig = localhost_instance().into();
            let groups = settings.groups.iter().cloned().map(Into::into).collect();
            self.pool.reconcile(vec![instance], groups).await;
        } else {
            let instances = settings.instances.iter().cloned().map(Into::into).collect();
            let groups = settings.groups.iter().cloned().map(Into::into).collect();
            self.pool.reconcile(instances, groups).await;
        }
        *self.runtime.global.lock().await = settings;
        let contexts: Vec<String> = self.runtime.keys.lock().await.keys().cloned().collect();
        for context in contexts {
            self.refresh_key(&context).await;
        }
        self.push_status_to_open_inspectors().await;
    }

    pub async fn upsert_key(
        &self,
        context: &str,
        kind: ActionKind,
        action: &str,
        settings: ActionSettings,
        multi: bool,
    ) {
        let mut keys = self.runtime.keys.lock().await;
        let previous = keys.remove(context);
        let segments = previous.map(|key| key.segments).unwrap_or_default();
        keys.insert(
            context.to_string(),
            LiveKey {
                kind,
                action: action.to_string(),
                settings,
                segments,
                titles: HashMap::new(),
                title: String::new(),
                multi,
            },
        );
        drop(keys);
        // The property inspector matches status by the saved instance id, not "localhost".
        self.reconcile_provisional().await;
        self.refresh_key(context).await;
    }

    pub async fn remove_key(&self, context: &str) {
        self.runtime.keys.lock().await.remove(context);
    }

    pub fn press(&self, context: &str) {
        let context = context.to_string();
        let state = self.clone();
        tokio::spawn(async move {
            state.run_press(&context).await;
        });
    }

    pub fn rotate(&self, context: &str, ticks: i32) {
        let context = context.to_string();
        let state = self.clone();
        tokio::spawn(async move {
            state.run_rotate(&context, ticks).await;
        });
    }

    pub fn dial_down(&self, context: &str) {
        let context = context.to_string();
        let state = self.clone();
        tokio::spawn(async move {
            state.run_dial_down(&context).await;
        });
    }

    pub fn handle_inspector(&self, context: &str, payload: Value) {
        let context = context.to_string();
        let state = self.clone();
        tokio::spawn(async move {
            state.dispatch_inspector(&context, payload).await;
        });
    }

    fn spawn_events(&self) {
        let state = self.clone();
        tokio::spawn(async move {
            let mut events = state.pool.subscribe();
            loop {
                match events.recv().await {
                    Ok(event) => state.on_pool_event(event).await,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }

    async fn targets_for(&self, settings: &ActionSettings) -> Vec<String> {
        let configs = self.pool.configs();
        let resolved = resolve_targets(&(&settings.common.target).into(), &configs, &self.pool.groups());
        if !resolved.is_empty() {
            return resolved;
        }
        loopback_fallback(&settings.common.target, &configs).unwrap_or(resolved)
    }

    async fn refresh_key(&self, context: &str) {
        let Some(snapshot) = self.key_snapshot(context).await else {
            return;
        };
        let targets = self.targets_for(&snapshot.settings).await;
        let mut segments = HashMap::new();
        let mut titles = HashMap::new();
        for id in &targets {
            let connected = self.pool.status(id).is_some_and(|status| status.is_connected());
            let params = snapshot.settings.params_for(id);
            let state = self.pool.state(id).unwrap_or_default();
            segments.insert(
                id.clone(),
                ops::segment_for(snapshot.kind, params, &state, connected),
            );
            let title = ops::title_for(snapshot.kind, params, &state);
            if !title.is_empty() {
                titles.insert(id.clone(), title);
            }
        }
        if let Some(key) = self.runtime.keys.lock().await.get_mut(context) {
            key.segments = segments;
            key.titles = titles.clone();
            key.title = join_titles(&targets, &titles);
        }
        self.paint(context).await;
    }

    async fn refresh_instance(&self, context: &str, instance_id: &str) {
        let Some(snapshot) = self.key_snapshot(context).await else {
            return;
        };
        let targets = self.targets_for(&snapshot.settings).await;
        if !targets.iter().any(|id| id == instance_id) {
            return;
        }
        let connected = self
            .pool
            .status(instance_id)
            .is_some_and(|status| status.is_connected());
        let params = snapshot.settings.params_for(instance_id);
        let cached = self.pool.state(instance_id).unwrap_or_default();
        let segment = ops::segment_for(snapshot.kind, params, &cached, connected);
        let title = ops::title_for(snapshot.kind, params, &cached);
        if let Some(key) = self.runtime.keys.lock().await.get_mut(context) {
            key.segments.insert(instance_id.to_string(), segment);
            if title.is_empty() {
                key.titles.remove(instance_id);
            } else {
                key.titles.insert(instance_id.to_string(), title);
            }
            key.title = join_titles(&targets, &key.titles);
        }
        self.paint(context).await;
    }

    async fn on_pool_event(&self, event: PoolEvent) {
        match event {
            PoolEvent::Status(status) => {
                let contexts: Vec<String> = self.runtime.keys.lock().await.keys().cloned().collect();
                for context in contexts {
                    if status.status.is_connected() {
                        self.refresh_instance(&context, &status.id).await;
                    } else {
                        let settings = self
                            .runtime
                            .keys
                            .lock()
                            .await
                            .get(&context)
                            .map(|key| key.settings.clone());
                        if let Some(settings) = settings {
                            let targets = self.targets_for(&settings).await;
                            if targets.iter().any(|id| id == &status.id) {
                                if let Some(key) = self.runtime.keys.lock().await.get_mut(&context) {
                                    key.segments
                                        .insert(status.id.clone(), SegmentState::Unavailable);
                                }
                            }
                        }
                    }
                    self.paint(&context).await;
                }
                self.push_status_to_open_inspectors().await;
            }
            PoolEvent::Activator { id, name } => {
                let contexts: Vec<String> = self
                    .runtime
                    .keys
                    .lock()
                    .await
                    .iter()
                    .filter(|(_, key)| ops::event_affects(key.kind, &name))
                    .map(|(context, _)| context.clone())
                    .collect();
                for context in contexts {
                    self.refresh_instance(&context, &id).await;
                }
            }
            PoolEvent::Snapshot { id } => {
                let contexts: Vec<String> = self.runtime.keys.lock().await.keys().cloned().collect();
                for context in contexts {
                    self.refresh_instance(&context, &id).await;
                }
                self.push_inputs().await;
            }
            PoolEvent::Function { id, ok } => {
                let Some(context) = self.runtime.pending.lock().await.remove(&id) else {
                    return;
                };
                let kind = self
                    .runtime
                    .keys
                    .lock()
                    .await
                    .get(&context)
                    .map(|key| key.kind);
                if kind.is_some_and(|kind| kind.confirms_press()) {
                    self.feedback(&context, !ok).await;
                }
            }
        }
    }

    async fn run_press(&self, context: &str) {
        let Some(snapshot) = self.key_snapshot(context).await else {
            return;
        };
        let targets = self.targets_for(&snapshot.settings).await;
        let mut failed = false;
        for id in &targets {
            let params = snapshot.settings.params_for(id);
            let Some(command) = ops::command_for(snapshot.kind, params) else {
                failed = true;
                continue;
            };
            if snapshot.kind.confirms_press() {
                self.runtime.pending.lock().await.insert(id.clone(), context.to_string());
            }
            if self.pool.send(id, command).is_err() {
                failed = true;
            }
        }
        if snapshot.kind.confirms_press() && (failed || targets.is_empty()) {
            self.feedback(context, true).await;
        }
    }

    async fn run_rotate(&self, context: &str, ticks: i32) {
        let Some(snapshot) = self.key_snapshot(context).await else {
            return;
        };
        let targets = self.targets_for(&snapshot.settings).await;
        for id in &targets {
            let params = snapshot.settings.params_for(id);
            let cached = self.pool.state(id).unwrap_or_default();
            let percent = ops::adjusted_percent(ops::volume_level(&cached, params), ticks, params.step);
            let _ = self.pool.send(id, ops::volume_command(params, percent));
        }
    }

    async fn run_dial_down(&self, context: &str) {
        let Some(snapshot) = self.key_snapshot(context).await else {
            return;
        };
        let targets = self.targets_for(&snapshot.settings).await;
        for id in &targets {
            let params = snapshot.settings.params_for(id);
            let _ = self.pool.send(id, ops::mute_command(params));
        }
    }

    async fn dispatch_inspector(&self, context: &str, payload: Value) {
        match payload.get("type").and_then(Value::as_str) {
            Some("reconnect") => {
                if let Some(id) = payload.get("id").and_then(Value::as_str) {
                    self.pool.reconnect(id);
                }
            }
            Some("globalSettings") => {
                if let Some(settings) = payload.get("settings") {
                    tracing::info!("property inspector delivered vMix settings");
                    self.apply_global_value(settings).await;
                }
            }
            Some("ready") => {
                tracing::info!(context, "property inspector ready");
                self.bootstrap_localhost().await;
                if !self.runtime.settings_applied.load(Ordering::SeqCst) {
                    if let Some(sender) = self.runtime.sender.lock().await.clone() {
                        let _ = sender.get_global_settings(None);
                    }
                }
                self.push_status(context).await;
                self.push_inputs().await;
            }
            _ => {}
        }
    }

    async fn key_snapshot(&self, context: &str) -> Option<LiveKey> {
        self.runtime.keys.lock().await.get(context).map(|key| LiveKey {
            kind: key.kind,
            action: key.action.clone(),
            settings: key.settings.clone(),
            segments: key.segments.clone(),
            titles: key.titles.clone(),
            title: key.title.clone(),
            multi: key.multi,
        })
    }

    async fn paint(&self, context: &str) {
        let Some(sender) = self.runtime.sender.lock().await.clone() else {
            return;
        };
        let Some(key) = self.key_snapshot(context).await else {
            return;
        };
        let configs = self.pool.configs();
        let targets = self.targets_for(&key.settings).await;
        let visual: Vec<Segment> = targets
            .iter()
            .map(|id| {
                let config = configs.iter().find(|config| config.id == *id);
                Segment {
                    label: config.map(|config| config.name.clone()).unwrap_or_else(|| id.clone()),
                    color: config
                        .map(|config| config.color.clone())
                        .unwrap_or_else(|| "#4c8dff".into()),
                    state: key
                        .segments
                        .get(id)
                        .copied()
                        .unwrap_or(SegmentState::Unavailable),
                }
            })
            .collect();
        let foreground = self.runtime.global.lock().await.fg_color.clone();
        let image = render::key_image(key.kind, &visual, &foreground);
        if sender
            .set_image(context, Some(&image), Target::HardwareAndSoftware, None)
            .is_err()
        {
            return;
        }
        if !key.multi && !key.title.is_empty() {
            let _ = sender.set_title(context, Some(&key.title), Target::HardwareAndSoftware, None);
        }
        if key.kind.has_toggle_state() {
            let active = render::hardware_state(visual.iter().map(|segment| segment.state));
            let _ = sender.set_state(context, active);
        }
    }

    async fn feedback(&self, context: &str, failed: bool) {
        let Some(sender) = self.runtime.sender.lock().await.clone() else {
            return;
        };
        if failed {
            let _ = sender.show_alert(context);
        } else {
            let _ = sender.show_ok(context);
        }
    }

    async fn push_status_to_open_inspectors(&self) {
        let contexts: Vec<String> = self.runtime.keys.lock().await.keys().cloned().collect();
        for context in contexts {
            self.push_status(&context).await;
        }
    }

    pub async fn inspector_opened(&self, context: &str) {
        tracing::info!(context, "property inspector opened");
        self.push_status(context).await;
        self.push_inputs().await;
    }

    async fn push_status(&self, context: &str) {
        let instances = self.pool.statuses();
        tracing::info!(context, count = instances.len(), "push vmix status");
        self.send_inspector(context, &json!({"type": "status", "instances": instances}))
            .await;
    }

    async fn push_inputs(&self) {
        let contexts: Vec<String> = self.runtime.keys.lock().await.keys().cloned().collect();
        let mut items = Vec::new();
        for config in self.pool.configs() {
            let Some(state) = self.pool.state(&config.id) else {
                continue;
            };
            let inputs: Vec<Value> = state
                .inputs
                .iter()
                .map(|input| {
                    json!({
                        "number": input.number,
                        "title": input.title,
                        "shortTitle": input.short_title,
                        "key": input.key,
                    })
                })
                .collect();
            let mut mixes: Vec<u8> = state.mixes_present.iter().copied().collect();
            mixes.sort_unstable();
            items.push(json!({"id": config.id, "inputs": inputs, "mixes": mixes}));
        }
        let input_count: usize = items
            .iter()
            .map(|item| item.get("inputs").and_then(Value::as_array).map(Vec::len).unwrap_or(0))
            .sum();
        tracing::info!(contexts = contexts.len(), inputs = input_count, "push vmix inputs");
        for context in contexts {
            self.send_inspector(&context, &json!({"type": "inputs", "items": items}))
                .await;
        }
    }

    async fn send_inspector(&self, context: &str, payload: &Value) {
        let Some(sender) = self.runtime.sender.lock().await.clone() else {
            return;
        };
        let Some(action) = self
            .runtime
            .keys
            .lock()
            .await
            .get(context)
            .map(|key| key.action.clone())
        else {
            return;
        };
        let command = json!({
            "action": action,
            "event": "sendToPropertyInspector",
            "context": context,
            "payload": payload
        });
        if let Ok(body) = serde_json::to_string(&command) {
            let _ = sender.send_raw(body);
        }
    }
}

fn join_titles(targets: &[String], titles: &HashMap<String, String>) -> String {
    let mut unique = Vec::new();
    for id in targets {
        let Some(line) = titles.get(id) else {
            continue;
        };
        if !line.is_empty() && !unique.iter().any(|have: &String| have == line) {
            unique.push(line.clone());
        }
    }
    unique.join("\n")
}

fn provisional_ids<'a>(targets: impl IntoIterator<Item = &'a TargetSelector>) -> Vec<String> {
    let mut ids = Vec::new();
    for target in targets {
        let TargetSelector::Instances { ids: wanted } = target else {
            continue;
        };
        for id in wanted {
            if !id.is_empty() && !ids.iter().any(|have| have == id) {
                ids.push(id.clone());
            }
        }
    }
    // vMix accepts one TCP client. A second connection to the same API never completes,
    // so only the instance id shown on the keys is opened.
    if ids.is_empty() {
        ids.push(LOCALHOST_ID.to_string());
    }
    ids
}

fn local_instance(id: String) -> VmixInstanceConfig {
    let mut instance = localhost_instance();
    instance.id = id;
    instance.into()
}

fn is_loopback_host(host: &str) -> bool {
    let host = host.trim().trim_matches(['[', ']']);
    host.eq_ignore_ascii_case("localhost") || host == "127.0.0.1" || host == "::1"
}

/// Keys saved against an instance id that is not loaded yet still drive the local vMix.
fn loopback_fallback(selector: &TargetSelector, configs: &[VmixInstanceConfig]) -> Option<Vec<String>> {
    let TargetSelector::Instances { ids } = selector else {
        return None;
    };
    if ids.is_empty() || ids.iter().any(|id| configs.iter().any(|config| config.id == *id)) {
        return None;
    }
    let local: Vec<String> = configs
        .iter()
        .filter(|config| config.enabled && is_loopback_host(&config.host))
        .map(|config| config.id.clone())
        .collect();
    if local.is_empty() {
        None
    } else {
        Some(local)
    }
}

fn summarize_settings(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Object(map) => {
            let instances = map.get("instances").and_then(Value::as_array).map(|items| items.len());
            format!("object keys={} instances={instances:?}", map.len())
        }
        other => other.to_string(),
    }
}

pub struct GlobalHook {
    pub state: AppState,
}

#[async_trait]
impl PluginLifecycle for GlobalHook {
    async fn on_did_receive_global_settings(&mut self, settings: &Value) -> Result<()> {
        tracing::info!(payload = %summarize_settings(settings), "didReceiveGlobalSettings");
        self.state.apply_global_value(settings).await;
        Ok(())
    }
}

#[cfg(test)]
impl AppState {
    pub async fn segment_state(&self, context: &str, id: &str) -> Option<SegmentState> {
        self.runtime
            .keys
            .lock()
            .await
            .get(context)
            .and_then(|key| key.segments.get(id).copied())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::contracts::{ActionParams, CommonSettings, InstanceConfig, TargetSelector};

    #[test]
    fn missing_instance_target_falls_back_to_loopback() {
        let configs = vec![VmixInstanceConfig {
            id: "localhost".into(),
            name: "Localhost".into(),
            host: "127.0.0.1".into(),
            port: 8099,
            color: "#4c8dff".into(),
            enabled: true,
            xml_interval_ms: 2000,
        }];
        let missing = TargetSelector::Instances {
            ids: vec!["36f8850b-70af-425f-8d79-8cf6b009a301".into()],
        };
        assert_eq!(loopback_fallback(&missing, &configs), Some(vec!["localhost".into()]));
        let present = TargetSelector::Instances {
            ids: vec!["localhost".into()],
        };
        assert_eq!(loopback_fallback(&present, &configs), None);
    }

    #[test]
    fn provisional_targets_include_the_saved_instance_id() {
        let ids = provisional_ids([&TargetSelector::Instances {
            ids: vec!["36f8850b-70af-425f-8d79-8cf6b009a301".into()],
        }]);
        assert_eq!(ids, vec!["36f8850b-70af-425f-8d79-8cf6b009a301".to_string()]);
        assert_eq!(provisional_ids(std::iter::empty()), vec!["localhost".to_string()]);
    }
    use vmix_pool::mock::MockVmix;

    fn instance(id: &str, port: u16) -> InstanceConfig {
        InstanceConfig {
            id: id.into(),
            name: id.into(),
            host: "127.0.0.1".into(),
            port,
            color: "#4c8dff".into(),
            enabled: true,
            xml_interval_ms: 0,
        }
    }

    fn program_key(input: &str, mix: u8) -> ActionSettings {
        ActionSettings {
            common: CommonSettings::default(),
            shared: ActionParams {
                input: input.into(),
                mix,
                ..ActionParams::default()
            },
            ..ActionSettings::default()
        }
    }

    async fn connect(state: &AppState, servers: &[(&str, &MockVmix)]) {
        state
            .apply_global(GlobalSettings {
                instances: servers
                    .iter()
                    .map(|(id, server)| instance(id, server.port))
                    .collect(),
                ..GlobalSettings::default()
            })
            .await;
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                let ready = servers.iter().all(|(id, _)| {
                    state.pool.status(id).is_some_and(|status| status.is_connected())
                        && state.pool.state(id).is_some_and(|cached| cached.program.get(&0) == Some(&1))
                });
                if ready {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("both instances connected");
    }

    async fn wait_segment(state: &AppState, context: &str, id: &str, expected: SegmentState) {
        tokio::time::timeout(Duration::from_secs(8), async {
            loop {
                if state.segment_state(context, id).await == Some(expected) {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("{context}/{id} did not become {expected:?}"));
    }

    #[tokio::test]
    async fn program_event_updates_only_the_instance_that_emitted_it() {
        let first = MockVmix::spawn().await;
        let second = MockVmix::spawn().await;
        let state = AppState::new(PoolOptions::for_tests());
        connect(&state, &[("a", &first), ("b", &second)]).await;
        state
            .upsert_key("program", ActionKind::Program, "test.program", program_key("1", 0), false)
            .await;
        wait_segment(&state, "program", "a", SegmentState::Active).await;
        wait_segment(&state, "program", "b", SegmentState::Active).await;

        first.push_acts("Input 1 0");
        first.push_acts("Input 2 1");
        wait_segment(&state, "program", "a", SegmentState::Inactive).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(
            state.segment_state("program", "b").await,
            Some(SegmentState::Active)
        );
    }

    #[tokio::test]
    async fn mix_two_follows_input_mix_two_not_main() {
        let server = MockVmix::spawn().await;
        let state = AppState::new(PoolOptions::for_tests());
        connect(&state, &[("a", &server)]).await;
        state
            .upsert_key("mix2", ActionKind::Program, "test.program", program_key("1", 2), false)
            .await;
        wait_segment(&state, "mix2", "a", SegmentState::Inactive).await;
        server.push_acts("InputMix2 1 1");
        wait_segment(&state, "mix2", "a", SegmentState::Active).await;
        server.push_acts("Input 1 0");
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(
            state.segment_state("mix2", "a").await,
            Some(SegmentState::Active)
        );
    }

    #[tokio::test]
    async fn shortcut_press_sends_the_encoded_function() {
        let server = MockVmix::spawn().await;
        let state = AppState::new(PoolOptions::for_tests());
        connect(&state, &[("a", &server)]).await;
        state
            .upsert_key(
                "shortcut",
                ActionKind::Shortcut,
                "test.shortcut",
                ActionSettings {
                    shared: ActionParams {
                        function_name: "SetText".into(),
                        input: "1".into(),
                        value: "hello world".into(),
                        ..ActionParams::default()
                    },
                    ..ActionSettings::default()
                },
                false,
            )
            .await;
        state.press("shortcut");
        tokio::time::timeout(Duration::from_secs(4), async {
            loop {
                if server.commands().await.iter().any(|line| {
                    line.contains("FUNCTION SetText Input=1&Value=hello%20world")
                }) {
                    return;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("shortcut sent");
    }
}
