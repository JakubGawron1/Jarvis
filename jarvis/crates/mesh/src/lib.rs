use jarvis_protocol::{
    DeviceCaps, DeviceInfo, DeviceKind, DistroBoot, SessionSnapshot, CORE_VERSION,
};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const HELLO_TTL: Duration = Duration::from_secs(15);

pub struct Mesh {
    pub local: DeviceInfo,
    devices: HashMap<String, (DeviceInfo, Instant)>,
    pub io_device: String,
    pub leader: String,
}

impl Mesh {
    pub fn local_device() -> DeviceInfo {
        if std::env::var("JARVIS_KIND").ok().as_deref() == Some("cloud") {
            let host = hostname::get()
                .map(|h| h.to_string_lossy().into_owned())
                .unwrap_or_else(|_| "render".into());
            return DeviceInfo {
                id: format!("cloud-{host}"),
                name: host,
                kind: DeviceKind::Cloud,
                boot: None,
                caps: DeviceCaps {
                    llm_local: false,
                    llm_online: true,
                    tts: std::env::var("OPENROUTER_API_KEY")
                        .ok()
                        .is_some_and(|s| !s.trim().is_empty()),
                    stt: false,
                    tools_os: false,
                    rewrite_core: false,
                    pull_core: true,
                    mic: false,
                    speaker: false,
                },
                core_version: CORE_VERSION.into(),
                battery: None,
                core_id: None,
            };
        }
        let kind = if cfg!(target_os = "windows") {
            DeviceKind::Windows
        } else if cfg!(target_os = "android") {
            DeviceKind::Android
        } else {
            let boot = std::env::var("JARVIS_BOOT").ok();
            if boot.is_some() {
                DeviceKind::JarvisLinux
            } else {
                DeviceKind::LinuxDesktop
            }
        };
        let boot = std::env::var("JARVIS_BOOT").ok().and_then(|s| match s.as_str() {
            "qemu" => Some(DistroBoot::Qemu),
            "vbox" => Some(DistroBoot::Vbox),
            "metal" => Some(DistroBoot::Metal),
            _ => None,
        });
        let host = hostname::get()
            .map(|h| h.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "jarvis".into());
        let rewrite = matches!(kind, DeviceKind::Windows | DeviceKind::LinuxDesktop)
            && which_cargo();
        DeviceInfo {
            id: format!("{kind:?}-{host}").to_lowercase(),
            name: host,
            kind,
            boot,
            caps: DeviceCaps {
                llm_local: true,
                llm_online: true,
                tts: true,
                stt: true,
                tools_os: !matches!(kind, DeviceKind::Cloud),
                rewrite_core: rewrite,
                pull_core: true,
                mic: true,
                speaker: true,
            },
            core_version: CORE_VERSION.into(),
            battery: None,
            core_id: None,
        }
    }

    pub fn new() -> Self {
        let local = Self::local_device();
        let id = local.id.clone();
        let mut devices = HashMap::new();
        devices.insert(id.clone(), (local.clone(), Instant::now()));
        Self {
            local,
            devices,
            io_device: id.clone(),
            leader: id,
        }
    }

    pub fn hello(&mut self, device: DeviceInfo) {
        let id = device.id.clone();
        self.devices.insert(id, (device, Instant::now()));
        self.elect();
    }

    pub fn tick_lost(&mut self) -> Vec<String> {
        let now = Instant::now();
        let mut lost = Vec::new();
        self.devices.retain(|id, (_, t)| {
            if *id == self.local.id {
                return true;
            }
            if now.duration_since(*t) > HELLO_TTL {
                lost.push(id.clone());
                false
            } else {
                true
            }
        });
        if !lost.is_empty() {
            self.elect();
        }
        lost
    }

    pub fn elect(&mut self) {
        let mut best: Option<&DeviceInfo> = None;
        let mut best_score = -1i32;
        for (d, _) in self.devices.values() {
            let score = score(d);
            if score > best_score {
                best_score = score;
                best = Some(d);
            }
        }
        if let Some(d) = best {
            self.leader = d.id.clone();
        }
    }

    pub fn claim_io(&mut self, target: &str) {
        if target.is_empty() {
            return;
        }
        self.io_device = target.to_string();
    }

    pub fn handoff_io(&mut self, target: &str) -> bool {
        if self.devices.contains_key(target) {
            self.io_device = target.to_string();
            true
        } else {
            false
        }
    }

    pub fn handoff_leader(&mut self, target: &str) -> bool {
        if let Some((d, _)) = self.devices.get(target) {
            if d.caps.llm_local || d.caps.llm_online {
                self.leader = target.to_string();
                return true;
            }
        }
        false
    }

    pub fn list(&self) -> Vec<DeviceInfo> {
        self.devices.values().map(|(d, _)| d.clone()).collect()
    }

    /// Devices physically connected to this jarvisd (not mirrored from a peer).
    pub fn owned_devices(&self) -> Vec<DeviceInfo> {
        self.list()
            .into_iter()
            .filter(|d| d.core_id.is_none())
            .collect()
    }

    /// Replace the device set advertised by another core.
    pub fn ingest_peer(&mut self, core_id: &str, devices: Vec<DeviceInfo>) {
        if core_id.is_empty() || core_id == self.local.id {
            return;
        }
        self.devices.retain(|id, (d, _)| {
            *id == self.local.id || d.core_id.as_deref() != Some(core_id)
        });
        for mut d in devices {
            if d.id == self.local.id {
                continue;
            }
            d.core_id = Some(core_id.to_string());
            self.devices.insert(d.id.clone(), (d, Instant::now()));
        }
        self.elect();
    }

    pub fn snapshot(&self, transcript: Vec<jarvis_protocol::ChatTurn>) -> SessionSnapshot {
        SessionSnapshot {
            transcript,
            lang: jarvis_protocol::Lang::En,
            open_jobs: vec![],
            io_device: self.io_device.clone(),
            leader: self.leader.clone(),
        }
    }
}

fn score(d: &DeviceInfo) -> i32 {
    match d.kind {
        DeviceKind::Windows | DeviceKind::LinuxDesktop => 100,
        DeviceKind::JarvisLinux => 90,
        DeviceKind::Android | DeviceKind::FlutterLinux => 50,
        DeviceKind::Cloud => 10,
    }
}

fn which_cargo() -> bool {
    std::process::Command::new("cargo")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
