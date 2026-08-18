use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;
use thiserror::Error;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("Codex CLI was not found on PATH")]
    BinaryNotFound,
    #[error("failed to start Codex app-server: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("Codex app-server timed out after {0} ms")]
    Timeout(u64),
    #[error("Codex app-server exited before returning a response")]
    ProcessExited,
    #[error("Codex app-server returned malformed JSON")]
    MalformedJson,
    #[error("Codex app-server returned an error: {0}")]
    Remote(String),
    #[error("Codex app-server does not support {0}")]
    Unsupported(String),
}

pub fn discover_binary() -> Result<String, ProtocolError> {
    let mut candidates = Vec::new();
    if let Ok(explicit) = std::env::var("CODEX_BINARY") {
        if !explicit.trim().is_empty() {
            candidates.push(explicit);
        }
    }
    let output = Command::new("where.exe")
        .arg("codex")
        .output()
        .map_err(ProtocolError::Spawn)?;
    if output.status.success() {
        candidates.extend(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string),
        );
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        candidates.push(
            Path::new(&profile)
                .join(".codex")
                .join(".sandbox-bin")
                .join("codex.exe")
                .to_string_lossy()
                .into_owned(),
        );
    }
    candidates
        .into_iter()
        .filter(|candidate| Path::new(candidate).exists())
        .find(|candidate| {
            let mut command = Command::new(candidate);
            command
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            configure_no_console(&mut command);
            command.status().is_ok_and(|status| status.success())
        })
        .ok_or(ProtocolError::BinaryNotFound)
}

pub fn read_rate_limits(
    binary: &str,
    timeout: Duration,
) -> Result<serde_json::Value, ProtocolError> {
    let mut command = Command::new(binary);
    command
        .args(["app-server", "--stdio"])
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    configure_no_console(&mut command);
    let mut child = ChildGuard::new(command.spawn().map_err(ProtocolError::Spawn)?);
    let mut stdin = child
        .child
        .stdin
        .take()
        .ok_or(ProtocolError::ProcessExited)?;
    let stdout = child
        .child
        .stdout
        .take()
        .ok_or(ProtocolError::ProcessExited)?;
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if sender.send(line).is_err() {
                break;
            }
        }
    });

    let messages = [
        r#"{"id":1,"method":"initialize","params":{"clientInfo":{"name":"codex-usage-notch","version":"0.1.0"},"capabilities":{"experimentalApi":false}}}"#,
        r#"{"method":"initialized"}"#,
        r#"{"id":2,"method":"account/rateLimits/read"}"#,
    ];
    for message in messages {
        writeln!(stdin, "{message}").map_err(ProtocolError::Spawn)?;
    }
    stdin.flush().map_err(ProtocolError::Spawn)?;

    loop {
        let initialize = next_json(&receiver, timeout)?;
        if initialize.get("id").and_then(serde_json::Value::as_i64) == Some(1) {
            if initialize.get("error").is_some() {
                child.stop();
                return Err(ProtocolError::Remote("initialization_failed".to_string()));
            }
            break;
        }
    }
    loop {
        let value = next_json(&receiver, timeout)?;
        if value.get("id").and_then(serde_json::Value::as_i64) == Some(2) {
            child.stop();
            if let Some(error) = value.get("error") {
                let code = error
                    .get("code")
                    .and_then(serde_json::Value::as_i64)
                    .unwrap_or(-1);
                if code == -32601 {
                    return Err(ProtocolError::Unsupported(
                        "account/rateLimits/read".to_string(),
                    ));
                }
                return Err(ProtocolError::Remote(format!("rpc_error_{code}")));
            }
            return Ok(value);
        }
    }
}

fn configure_no_console(command: &mut Command) {
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
}

fn next_json(
    receiver: &Receiver<Result<String, std::io::Error>>,
    timeout: Duration,
) -> Result<serde_json::Value, ProtocolError> {
    let line = receiver
        .recv_timeout(timeout)
        .map_err(|_| ProtocolError::Timeout(timeout.as_millis() as u64))?
        .map_err(ProtocolError::Spawn)?;
    serde_json::from_str(&line).map_err(|_| ProtocolError::MalformedJson)
}

struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child }
    }

    fn stop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        self.stop();
    }
}
