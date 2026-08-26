use clap::{Parser, Subcommand};
use futures_util::{SinkExt, StreamExt};
use jarvis_protocol::{ClientMessage, ServerMessage, DEFAULT_BIND};
use tokio_tungstenite::connect_async;
use uuid::Uuid;

#[derive(Parser)]
#[command(name = "jarvis", about = "Jarvis CLI — text in PL or EN")]
struct Cli {
    /// Talk to a running jarvisd (ws://host:port/ws)
    #[arg(long, env = "JARVIS_WS")]
    ws: Option<String>,

    #[command(subcommand)]
    cmd: Option<Cmd>,

    /// Bare text: `jarvis "otwórz notatnik"`
    text: Option<Vec<String>>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Send a text command
    Say { words: Vec<String> },
    /// Ask daemon for presence
    Devices,
    /// Check core version / pull
    Pull,
    /// Desktop-only: cargo test then notify hosts
    Rewrite {
        /// Run cargo test in repo (desktop only)
        #[arg(long)]
        test_only: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let cli = Cli::parse();
    let text = if let Some(Cmd::Say { words }) = &cli.cmd {
        Some(words.join(" "))
    } else if let Some(words) = &cli.text {
        Some(words.join(" "))
    } else {
        None
    };

    let ws = cli
        .ws
        .unwrap_or_else(|| format!("ws://{DEFAULT_BIND}/ws"));

    match cli.cmd {
        Some(Cmd::Devices) => {
            send(&ws, ClientMessage::Ping {}).await?;
        }
        Some(Cmd::Pull) => {
            send(&ws, ClientMessage::PullCore {}).await?;
        }
        Some(Cmd::Rewrite { test_only }) => {
            let root = std::env::var("JARVIS_ROOT").unwrap_or_else(|_| "..".into());
            let jarvis = std::path::Path::new(&root).join("jarvis");
            let dir = if jarvis.exists() { jarvis } else { std::path::PathBuf::from(".") };
            eprintln!("rewrite_core worktree test in {}", dir.display());
            let ok = jarvis_update::cargo_test(&dir)?;
            if !ok {
                anyhow::bail!("cargo test failed — last-known-good kept");
            }
            println!("tests passed; publish artifacts / git push to update Android, Render, distro");
            if !test_only {
                send(&ws, ClientMessage::PullCore {}).await?;
            }
        }
        Some(Cmd::Say { .. }) | None => {
            let content = text.unwrap_or_default();
            if content.is_empty() {
                eprintln!("usage: jarvis \"jaki mam kalendarz\"");
                std::process::exit(1);
            }
            send(
                &ws,
                ClientMessage::Text {
                    id: Uuid::new_v4().to_string(),
                    content,
                    lang: None,
                },
            )
            .await?;
        }
    }
    Ok(())
}

async fn send(url: &str, msg: ClientMessage) -> anyhow::Result<()> {
    let (ws, _) = connect_async(url).await?;
    let (mut sink, mut stream) = ws.split();
    let body = serde_json::to_string(&msg)?;
    sink.send(tokio_tungstenite::tungstenite::Message::Text(body.into()))
        .await?;

    while let Some(frame) = stream.next().await {
        let frame = frame?;
        if let tokio_tungstenite::tungstenite::Message::Text(t) = frame {
            if let Ok(sm) = serde_json::from_str::<ServerMessage>(&t) {
                match sm {
                    ServerMessage::Reply { content, .. } => {
                        println!("{content}");
                        break;
                    }
                    ServerMessage::Confirm { prompt, .. } => {
                        println!("CONFIRM: {prompt}");
                        break;
                    }
                    ServerMessage::JobDeferred { message, .. } => {
                        println!("{message}");
                        break;
                    }
                    ServerMessage::Error { message } => {
                        eprintln!("error: {message}");
                    }
                    ServerMessage::CoreUpdate { version } => {
                        println!("core {version}");
                        break;
                    }
                    ServerMessage::Presence { io_device, leader, devices } => {
                        println!("io={io_device} leader={leader}");
                        for d in devices {
                            println!("  {} ({:?})", d.name, d.kind);
                        }
                        if matches!(msg, ClientMessage::Ping {}) {
                            break;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
