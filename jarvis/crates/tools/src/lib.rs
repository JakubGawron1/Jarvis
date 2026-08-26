use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub ok: bool,
    pub output: String,
    pub destructive: bool,
    pub needs_confirm: bool,
    pub confirm_prompt: Option<String>,
}

pub struct ToolHost {
    pub vault: PathBuf,
    pub repo_root: PathBuf,
    pub os: HostOs,
}

#[derive(Debug, Clone, Copy)]
pub enum HostOs {
    Windows,
    Linux,
    Android,
    Cloud,
}

impl HostOs {
    pub fn detect() -> Self {
        if cfg!(target_os = "windows") {
            HostOs::Windows
        } else if cfg!(target_os = "android") {
            HostOs::Android
        } else if cfg!(target_os = "linux") {
            HostOs::Linux
        } else {
            HostOs::Linux
        }
    }

    pub fn has_os_tools(self) -> bool {
        !matches!(self, HostOs::Cloud)
    }
}

impl ToolHost {
    pub fn new(repo_root: PathBuf) -> Self {
        let vault = repo_root.join("vault");
        let os = if std::env::var("JARVIS_KIND").ok().as_deref() == Some("cloud") {
            HostOs::Cloud
        } else {
            HostOs::detect()
        };
        Self {
            vault,
            repo_root,
            os,
        }
    }

    pub fn is_destructive(name: &str, args: &str) -> bool {
        let blob = format!("{name} {args}").to_lowercase();
        blob.contains("format")
            || blob.contains("push --force")
            || blob.contains("git push --force")
            || blob.contains("send_email")
            || blob.contains("rm -rf")
            || blob.contains("del /s")
    }

    pub fn run(&self, name: &str, args: &str, confirmed: bool) -> Result<ToolResult> {
        if Self::is_destructive(name, args) && !confirmed {
            return Ok(ToolResult {
                ok: false,
                output: "needs confirmation".into(),
                destructive: true,
                needs_confirm: true,
                confirm_prompt: Some(match jarvis_protocol::detect_lang(args) {
                    jarvis_protocol::Lang::Pl => {
                        format!("To działanie jest destrukcyjne ({name}). Potwierdzić?")
                    }
                    jarvis_protocol::Lang::En => {
                        format!("This action is destructive ({name}). Shall I proceed?")
                    }
                }),
            });
        }
        if matches!(self.os, HostOs::Cloud) && matches!(name, "open_app" | "shell" | "rewrite_core")
        {
            bail!("cloud host has no OS tools");
        }
        match name {
            "vault_write" => self.vault_write(args),
            "calendar_add" => self.calendar_add(args),
            "open_app" => self.open_app(args),
            "list_vault" => self.list_vault(),
            "shell" => self.shell(args),
            other => Ok(ToolResult {
                ok: false,
                output: format!("unknown tool {other}"),
                destructive: false,
                needs_confirm: false,
                confirm_prompt: None,
            }),
        }
    }

    fn vault_write(&self, args: &str) -> Result<ToolResult> {
        let (title, body) = split_title_body(args);
        std::fs::create_dir_all(self.vault.join("notes"))?;
        let slug: String = title
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .take(40)
            .collect();
        let path = self.vault.join("notes").join(format!("{slug}.md"));
        std::fs::write(&path, format!("# {title}\n\n{body}\n"))?;
        Ok(ok(format!("wrote {}", path.display())))
    }

    fn calendar_add(&self, args: &str) -> Result<ToolResult> {
        let path = self.vault.join("calendar.json");
        let mut v: serde_json::Value = if path.exists() {
            serde_json::from_str(&std::fs::read_to_string(&path)?)?
        } else {
            serde_json::json!({ "events": [] })
        };
        let events = v["events"].as_array_mut().unwrap();
        events.push(serde_json::json!({
            "title": args,
            "created": chrono::Utc::now().to_rfc3339(),
        }));
        std::fs::write(&path, serde_json::to_string_pretty(&v)?)?;
        self.export_ics(&path)?;
        Ok(ok(format!("calendar event added: {args}")))
    }

    fn export_ics(&self, json_path: &Path) -> Result<()> {
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(json_path)?)?;
        let mut ics = String::from("BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Jarvis//EN\r\n");
        if let Some(events) = v["events"].as_array() {
            for (i, e) in events.iter().enumerate() {
                let title = e["title"].as_str().unwrap_or("event");
                ics.push_str("BEGIN:VEVENT\r\n");
                ics.push_str(&format!("UID:jarvis-{i}@local\r\n"));
                ics.push_str(&format!("SUMMARY:{title}\r\n"));
                ics.push_str("END:VEVENT\r\n");
            }
        }
        ics.push_str("END:VCALENDAR\r\n");
        std::fs::write(self.vault.join("calendar.ics"), ics)?;
        Ok(())
    }

    fn list_vault(&self) -> Result<ToolResult> {
        let notes = self.vault.join("notes");
        let mut names = Vec::new();
        if notes.is_dir() {
            for e in std::fs::read_dir(notes)? {
                names.push(e?.file_name().to_string_lossy().into_owned());
            }
        }
        Ok(ok(names.join(", ")))
    }

    fn open_app(&self, args: &str) -> Result<ToolResult> {
        let raw = args.trim().trim_matches(|c: char| "\"'.".contains(c));
        if raw.is_empty() {
            return Ok(ToolResult {
                ok: false,
                output: "no app name".into(),
                destructive: false,
                needs_confirm: false,
                confirm_prompt: None,
            });
        }
        let target = resolve_app(raw);
        let launched = match self.os {
            HostOs::Windows => launch_windows(&target),
            HostOs::Linux | HostOs::Android => Command::new("xdg-open")
                .arg(&target)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn()
                .map(|_| ()),
            HostOs::Cloud => {
                return Ok(ToolResult {
                    ok: false,
                    output: "no OS tools in cloud".into(),
                    destructive: false,
                    needs_confirm: false,
                    confirm_prompt: None,
                });
            }
        };
        match launched {
            Ok(()) => Ok(ok(format!("launched {target}"))),
            Err(e) => Ok(ToolResult {
                ok: false,
                output: format!("could not launch {target}: {e}"),
                destructive: false,
                needs_confirm: false,
                confirm_prompt: None,
            }),
        }
    }

    fn shell(&self, args: &str) -> Result<ToolResult> {
        let output = if cfg!(windows) {
            Command::new("cmd").args(["/C", args]).output()?
        } else {
            Command::new("sh").args(["-c", args]).output()?
        };
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        Ok(ok(text))
    }
}

fn split_title_body(args: &str) -> (String, String) {
    if let Some((a, b)) = args.split_once('|') {
        (a.trim().to_string(), b.trim().to_string())
    } else if let Some((a, b)) = args.split_once('\n') {
        (a.trim().to_string(), b.trim().to_string())
    } else {
        (args.trim().to_string(), String::new())
    }
}

fn ok(output: impl Into<String>) -> ToolResult {
    ToolResult {
        ok: true,
        output: output.into(),
        destructive: false,
        needs_confirm: false,
        confirm_prompt: None,
    }
}

fn resolve_app(name: &str) -> String {
    let n = name
        .trim()
        .trim_matches(|c: char| "\"'.".contains(c))
        .to_lowercase();
    match n.as_str() {
        "notepad" | "notatnik" | "notes" => "notepad.exe".into(),
        "calc" | "kalkulator" | "calculator" => "calc.exe".into(),
        "explorer" | "pliki" | "files" | "folder" => "explorer.exe".into(),
        "cmd" | "terminal" | "konsola" => "cmd.exe".into(),
        "powershell" | "ps" => "powershell.exe".into(),
        "settings" | "ustawienia" => "ms-settings:".into(),
        "paint" | "mspaint" => "mspaint.exe".into(),
        "chrome" | "google chrome" | "google-chrome" => {
            chrome_path().unwrap_or_else(|| "chrome".into())
        }
        "edge" | "msedge" => "msedge.exe".into(),
        "firefox" => "firefox.exe".into(),
        "spotify" => "spotify:".into(),
        "code" | "vscode" | "visual studio code" => "code".into(),
        "discord" => "discord".into(),
        "steam" => "steam://".into(),
        other => other.trim().to_string(),
    }
}

fn chrome_path() -> Option<String> {
    let local = std::env::var("LOCALAPPDATA").ok()?;
    let p = PathBuf::from(local).join("Google/Chrome/Application/chrome.exe");
    p.is_file().then(|| p.to_string_lossy().into_owned())
}

fn launch_windows(target: &str) -> std::io::Result<()> {
    let ps = if target.contains(':') && !target.ends_with(".exe") {
        format!("Start-Process '{}'", target.replace('\'', "''"))
    } else {
        format!("Start-Process -FilePath '{}'", target.replace('\'', "''"))
    };
    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-Command", &ps])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    hide_window(&mut cmd);
    match cmd.spawn() {
        Ok(_) => Ok(()),
        Err(_) => {
            let mut start = Command::new("cmd");
            start
                .args(["/C", "start", "", target])
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
            hide_window(&mut start);
            start.spawn().map(|_| ())
        }
    }
}

fn hide_window(cmd: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let _ = cmd;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_polish_notepad() {
        assert_eq!(resolve_app("notatnik"), "notepad.exe");
        assert_eq!(resolve_app("kalkulator"), "calc.exe");
    }
}
