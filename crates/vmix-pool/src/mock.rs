//! Line-oriented stand-in for a vMix TCP API.

use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::{broadcast, mpsc, Mutex};

pub fn sample_xml() -> &'static str {
    r#"<vmix><version>29.0.0.0</version><edition>4K</edition><inputs><input key="cam-key" number="1" type="Colour" title="Cam" shortTitle="Cam" state="Paused" position="0" duration="0" loop="False" muted="False" volume="80"></input><input key="guest-key" number="2" type="Colour" title="Guest" shortTitle="Guest" state="Paused" position="0" duration="0" loop="False" muted="True" volume="100" selectedIndex="3"></input></inputs><overlays><overlay number="1"></overlay></overlays><preview>2</preview><active>1</active><fadeToBlack>False</fadeToBlack><transitions><transition number="1" effect="Fade" duration="500"></transition></transitions><recording>False</recording><external>False</external><streaming>False</streaming><playList>False</playList><multiCorder>False</multiCorder><fullscreen>False</fullscreen><mix number="2"><preview>1</preview><active>2</active></mix><audio><master volume="100" muted="False" meterF1="0" meterF2="0"></master></audio><dynamic><input1></input1><input2></input2><input3></input3><input4></input4><value1></value1><value2></value2><value3></value3><value4></value4></dynamic></vmix>"#
}

pub struct MockVmix {
    pub port: u16,
    commands: Arc<Mutex<Vec<String>>>,
    pushes: broadcast::Sender<String>,
    xml: Arc<Mutex<String>>,
}

impl MockVmix {
    pub async fn spawn() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind mock");
        let port = listener.local_addr().expect("addr").port();
        let commands = Arc::new(Mutex::new(Vec::new()));
        let xml = Arc::new(Mutex::new(sample_xml().to_string()));
        let (pushes, _) = broadcast::channel(64);
        let commands_task = commands.clone();
        let xml_task = xml.clone();
        let pushes_task = pushes.clone();
        tokio::spawn(async move {
            loop {
                let Ok((socket, _)) = listener.accept().await else {
                    break;
                };
                let commands = commands_task.clone();
                let xml = xml_task.clone();
                let pushes = pushes_task.subscribe();
                tokio::spawn(handle(socket, commands, xml, pushes));
            }
        });
        Self {
            port,
            commands,
            pushes,
            xml,
        }
    }

    pub async fn commands(&self) -> Vec<String> {
        self.commands.lock().await.clone()
    }

    pub fn push_acts(&self, body: &str) {
        let _ = self.pushes.send(format!("ACTS OK {body}\r\n"));
    }

    pub async fn set_xml(&self, xml: &str) {
        *self.xml.lock().await = xml.to_string();
    }
}

async fn handle(
    socket: tokio::net::TcpStream,
    commands: Arc<Mutex<Vec<String>>>,
    xml: Arc<Mutex<String>>,
    mut pushes: broadcast::Receiver<String>,
) {
    let (read, mut write) = socket.into_split();
    let (local_tx, mut local_rx) = mpsc::unbounded_channel::<String>();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                Some(line) = local_rx.recv() => {
                    if write.write_all(line.as_bytes()).await.is_err() {
                        break;
                    }
                }
                Ok(line) = pushes.recv() => {
                    if write.write_all(line.as_bytes()).await.is_err() {
                        break;
                    }
                }
                else => break,
            }
        }
    });
    let mut lines = BufReader::new(read).lines();
    loop {
        let Ok(Ok(Some(line))) =
            tokio::time::timeout(Duration::from_secs(30), lines.next_line()).await
        else {
            break;
        };
        if line.is_empty() {
            continue;
        }
        commands.lock().await.push(line.clone());
        if let Some(reply) = reply_to(&line, &xml).await {
            if local_tx.send(reply).is_err() {
                break;
            }
        }
    }
}

async fn reply_to(line: &str, xml: &Mutex<String>) -> Option<String> {
    let mut parts = line.split_whitespace();
    let command = parts.next()?;
    match command {
        "SUBSCRIBE" => Some("SUBSCRIBE OK ACTS\r\n".into()),
        "UNSUBSCRIBE" => Some("UNSUBSCRIBE OK ACTS\r\n".into()),
        "VERSION" => Some("VERSION OK 29.0.0.0\r\n".into()),
        "XML" => {
            let body = xml.lock().await.clone();
            Some(format!("XML {}\r\n{body}", body.len()))
        }
        "FUNCTION" => {
            let name = parts.next().unwrap_or("");
            Some(format!("FUNCTION OK {name}\r\n"))
        }
        "QUIT" => Some("QUIT\r\n".into()),
        _ => None,
    }
}
