use crate::transport::stdio::StdioTransport;
use crate::transport::ws::WsTransport;
use serde_json::Value;
use std::process::Command;
use std::time::Duration;

#[derive(Debug)]
pub enum DaemonManager {
    Stdio(StdioTransport),
    Ws(WsTransport),
}

impl DaemonManager {
    pub fn start(settings_transport: &str, wsl_distro: Option<&str>) -> Result<Self, String> {
        match settings_transport {
            "wsl-stdio" => Ok(Self::Stdio(StdioTransport::spawn(wsl_stdio_command(
                wsl_distro,
            ))?)),
            "ws" => Ok(Self::Ws(WsTransport::new("ws://127.0.0.1:3847")?)),
            "http" => Err("HTTP transport not implemented yet".to_string()),
            other => Err(format!("Unsupported transport '{other}'")),
        }
    }

    pub fn is_running(&self) -> bool {
        match self {
            Self::Stdio(transport) => transport.is_running(),
            Self::Ws(_) => true,
        }
    }

    pub fn stop(&self) -> Result<(), String> {
        match self {
            Self::Stdio(transport) => transport.stop(),
            Self::Ws(_) => Ok(()),
        }
    }

    pub fn call(&self, method: String, params: Value, timeout: Duration) -> Result<Value, String> {
        match self {
            Self::Stdio(transport) => transport.call(method, params, timeout),
            Self::Ws(transport) => transport.call(method, params, timeout),
        }
    }
}

#[cfg(target_os = "windows")]
fn wsl_stdio_command(wsl_distro: Option<&str>) -> Command {
    let mut cmd = Command::new("wsl.exe");
    if let Some(distro) = wsl_distro {
        if !distro.trim().is_empty() {
            cmd.args(["-d", distro]);
        }
    }
    cmd.arg("--").arg("ntm-tracker-daemon").arg("--stdio");
    cmd
}

#[cfg(not(target_os = "windows"))]
fn wsl_stdio_command(_wsl_distro: Option<&str>) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-lc")
        .arg("echo 'wsl-stdio transport is only supported on Windows'; sleep 3600");
    cmd
}
