use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Mode,
}

#[derive(Subcommand)]
enum Mode {
    Daemon,
    Client {
        #[arg(long, default_value_t = 50)]
        pings: u64,
        #[arg(long, default_value_t = 100)]
        rate_hz: u64,
        #[arg(long, default_value_t = 5)]
        duration_secs: u64,
        #[arg(last = true)]
        spawn: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Mode::Daemon => run_daemon().await,
        Mode::Client {
            pings,
            rate_hz,
            duration_secs,
            spawn,
        } => run_client(pings, rate_hz, duration_secs, spawn).await,
    }
}

async fn run_daemon() -> Result<()> {
    let (tx, mut rx) = mpsc::channel::<String>(1024);
    let writer = tokio::spawn(async move {
        let mut stdout = io::BufWriter::new(io::stdout());
        while let Some(line) = rx.recv().await {
            stdout.write_all(line.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
        Result::<()>::Ok(())
    });

    let stdin = io::stdin();
    let mut lines = BufReader::new(stdin).lines();
    let mut notify_handle: Option<tokio::task::JoinHandle<()>> = None;

    while let Some(line) = lines.next_line().await? {
        let value: Value = match serde_json::from_str(&line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let method = value.get("method").and_then(Value::as_str).unwrap_or("");
        let id = value.get("id").and_then(Value::as_u64);
        let params = value.get("params").cloned().unwrap_or(Value::Null);

        match method {
            "ping" => {
                if let Some(id_value) = id {
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": id_value,
                        "result": { "server_ts_ms": now_ms() }
                    });
                    let _ = tx.send(response.to_string()).await;
                }
            }
            "start_notifications" => {
                if let Some(handle) = notify_handle.take() {
                    handle.abort();
                }
                let rate_hz = params
                    .get("rate_hz")
                    .and_then(Value::as_u64)
                    .unwrap_or(100)
                    .max(1);
                let duration_secs = params
                    .get("duration_secs")
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let tx_clone = tx.clone();
                notify_handle = Some(tokio::spawn(async move {
                    let interval_ms = (1000 / rate_hz).max(1);
                    let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
                    let start = Instant::now();
                    let mut seq: u64 = 0;
                    loop {
                        interval.tick().await;
                        if duration_secs > 0 && start.elapsed() >= Duration::from_secs(duration_secs) {
                            break;
                        }
                        seq += 1;
                        let notification = json!({
                            "jsonrpc": "2.0",
                            "method": "tick",
                            "params": { "seq": seq, "sent_at_ms": now_ms() }
                        });
                        if tx_clone.send(notification.to_string()).await.is_err() {
                            break;
                        }
                    }
                }));
                if let Some(id_value) = id {
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": id_value,
                        "result": { "started": true, "rate_hz": rate_hz, "duration_secs": duration_secs }
                    });
                    let _ = tx.send(response.to_string()).await;
                }
            }
            "stop_notifications" => {
                if let Some(handle) = notify_handle.take() {
                    handle.abort();
                }
                if let Some(id_value) = id {
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": id_value,
                        "result": { "stopped": true }
                    });
                    let _ = tx.send(response.to_string()).await;
                }
            }
            _ => {
                if let Some(id_value) = id {
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": id_value,
                        "error": { "code": -32601, "message": "Method not found" }
                    });
                    let _ = tx.send(response.to_string()).await;
                }
            }
        }
    }

    drop(tx);
    writer.await??;
    Ok(())
}

async fn run_client(pings: u64, rate_hz: u64, duration_secs: u64, spawn: Vec<String>) -> Result<()> {
    let mut command = build_spawn_command(spawn).context("spawn command")?;
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .context("spawn daemon")?;

    let stdout = child.stdout.take().context("capture stdout")?;
    let stdin = child.stdin.take().context("capture stdin")?;

    let (out_tx, mut out_rx) = mpsc::channel::<String>(1024);
    let writer = tokio::spawn(async move {
        let mut writer = io::BufWriter::new(stdin);
        while let Some(line) = out_rx.recv().await {
            writer.write_all(line.as_bytes()).await?;
            writer.write_all(b"\n").await?;
            writer.flush().await?;
        }
        Result::<()>::Ok(())
    });

    let (event_tx, mut event_rx) = mpsc::channel::<ServerEvent>(2048);
    let reader = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Some(line) = lines.next_line().await? {
            let value: Value = match serde_json::from_str(&line) {
                Ok(value) => value,
                Err(_) => continue,
            };
            if let Some(id) = value.get("id").and_then(Value::as_u64) {
                let _ = event_tx.send(ServerEvent::Response(id)).await;
            } else if value
                .get("method")
                .and_then(Value::as_str)
                .map(|m| m == "tick")
                .unwrap_or(false)
            {
                let seq = value
                    .get("params")
                    .and_then(|params| params.get("seq"))
                    .and_then(Value::as_u64)
                    .unwrap_or(0);
                let _ = event_tx.send(ServerEvent::Notification(seq)).await;
            }
        }
        Result::<()>::Ok(())
    });

    let mut pending: HashMap<u64, Instant> = HashMap::new();
    let mut latencies_ms: Vec<f64> = Vec::new();
    let mut notifications: u64 = 0;
    let mut last_seq: u64 = 0;

    for id in 1..=pings {
        pending.insert(id, Instant::now());
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "ping",
            "params": { "client_ts_ms": now_ms() }
        });
        out_tx.send(request.to_string()).await?;
    }

    let start_notifications_id = pings + 1;
    let start_request = json!({
        "jsonrpc": "2.0",
        "id": start_notifications_id,
        "method": "start_notifications",
        "params": { "rate_hz": rate_hz, "duration_secs": duration_secs }
    });
    out_tx.send(start_request.to_string()).await?;
    pending.insert(start_notifications_id, Instant::now());

    let stop_tx = out_tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(duration_secs)).await;
        let stop_request = json!({
            "jsonrpc": "2.0",
            "id": start_notifications_id + 1,
            "method": "stop_notifications"
        });
        let _ = stop_tx.send(stop_request.to_string()).await;
    });

    let deadline = Instant::now() + Duration::from_secs(duration_secs + 3);
    while Instant::now() < deadline || !pending.is_empty() {
        match tokio::time::timeout(Duration::from_millis(250), event_rx.recv()).await {
            Ok(Some(event)) => match event {
                ServerEvent::Response(id) => {
                    if let Some(sent_at) = pending.remove(&id) {
                        let elapsed = sent_at.elapsed().as_secs_f64() * 1000.0;
                        latencies_ms.push(elapsed);
                    }
                }
                ServerEvent::Notification(seq) => {
                    notifications += 1;
                    last_seq = seq;
                }
            },
            Ok(None) => break,
            Err(_) => {}
        }
    }

    drop(out_tx);
    let _ = writer.await?;
    let _ = reader.await?;
    let _ = child.kill().await;

    latencies_ms.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p50 = percentile(&latencies_ms, 50.0);
    let p95 = percentile(&latencies_ms, 95.0);
    let p99 = percentile(&latencies_ms, 99.0);
    let expected_notifications = rate_hz * duration_secs;

    println!("--- stdio rpc spike ---");
    println!("pings sent: {pings}");
    println!("latency p50: {p50:.2} ms");
    println!("latency p95: {p95:.2} ms");
    println!("latency p99: {p99:.2} ms");
    println!(
        "notifications: {notifications} received / {expected_notifications} expected"
    );
    println!("last notification seq: {last_seq}");

    Ok(())
}

fn build_spawn_command(spawn: Vec<String>) -> Result<Command> {
    if spawn.is_empty() {
        let exe = std::env::current_exe()?;
        let mut command = Command::new(exe);
        command.arg("daemon");
        Ok(command)
    } else {
        let mut command = Command::new(&spawn[0]);
        if spawn.len() > 1 {
            command.args(&spawn[1..]);
        }
        Ok(command)
    }
}

fn percentile(values: &[f64], pct: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let rank = ((pct / 100.0) * (values.len().saturating_sub(1)) as f64).round() as usize;
    values[rank.min(values.len() - 1)]
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

enum ServerEvent {
    Response(u64),
    Notification(u64),
}
