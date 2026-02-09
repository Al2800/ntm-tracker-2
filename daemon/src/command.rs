use crate::metrics::METRICS;
use std::collections::HashMap;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command;
use tokio::sync::{Mutex, Semaphore};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandCategory {
    TmuxFast,
    NtmStatus,
    NtmTail,
}

#[derive(Clone, Debug)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub timeout: Duration,
    pub max_output_bytes: usize,
    pub category: CommandCategory,
}

#[derive(Clone, Debug)]
pub struct CommandConfig {
    pub max_concurrent: usize,
    pub max_output_bytes: usize,
    pub tmux_timeout: Duration,
    pub ntm_status_timeout: Duration,
    pub ntm_tail_timeout: Duration,
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            max_concurrent: 4,
            max_output_bytes: 256 * 1024,
            tmux_timeout: Duration::from_secs(2),
            ntm_status_timeout: Duration::from_secs(10),
            ntm_tail_timeout: Duration::from_secs(15),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CommandOutput {
    pub status: std::process::ExitStatus,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub duration: Duration,
}

#[derive(Debug)]
pub enum CommandError {
    Spawn(std::io::Error),
    Io(std::io::Error),
    Timeout,
    OutputTooLarge,
    ExitNonZero(i32),
    CircuitOpen,
}

#[derive(Clone, Debug, Default)]
struct BreakerState {
    consecutive_failures: u32,
    backoff_until: Option<Instant>,
}


#[derive(Debug)]
pub struct CircuitBreaker {
    states: Mutex<HashMap<CommandCategory, BreakerState>>,
}

impl Default for CircuitBreaker {
    fn default() -> Self {
        Self::new()
    }
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            states: Mutex::new(HashMap::new()),
        }
    }

    async fn check(&self, category: CommandCategory) -> Result<(), CommandError> {
        let mut states = self.states.lock().await;
        let state = states.entry(category).or_default();
        if let Some(until) = state.backoff_until {
            if Instant::now() < until {
                return Err(CommandError::CircuitOpen);
            }
            state.backoff_until = None;
        }
        Ok(())
    }

    async fn record_success(&self, category: CommandCategory) {
        let mut states = self.states.lock().await;
        if let Some(state) = states.get_mut(&category) {
            state.consecutive_failures = 0;
            state.backoff_until = None;
        }
    }

    async fn record_failure(&self, category: CommandCategory) -> Result<(), CommandError> {
        let mut states = self.states.lock().await;
        let state = states.entry(category).or_default();
        state.consecutive_failures += 1;

        if state.consecutive_failures >= 10 {
            state.backoff_until = Some(Instant::now() + Duration::from_secs(300));
            return Err(CommandError::CircuitOpen);
        }

        if state.consecutive_failures >= 3 {
            let exponent = state.consecutive_failures.saturating_sub(3);
            let backoff_secs = 2_u64.checked_pow(exponent).unwrap_or(u64::MAX);
            let backoff = Duration::from_secs(backoff_secs.min(60));
            state.backoff_until = Some(Instant::now() + backoff);
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct CommandRunner {
    config: CommandConfig,
    semaphore: Semaphore,
    breaker: CircuitBreaker,
}

impl CommandRunner {
    pub fn new(config: CommandConfig) -> Self {
        Self {
            semaphore: Semaphore::new(config.max_concurrent),
            config,
            breaker: CircuitBreaker::new(),
        }
    }

    pub async fn run(&self, mut spec: CommandSpec) -> Result<CommandOutput, CommandError> {
        self.apply_defaults(&mut spec);
        self.breaker.check(spec.category).await?;
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| CommandError::CircuitOpen)?;

        let start = Instant::now();
        let mut child = Command::new(&spec.program)
            .args(&spec.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(CommandError::Spawn)?;

        let stdout = child.stdout.take().expect("stdout");
        let stderr = child.stderr.take().expect("stderr");

        let stdout_task = read_limited(stdout, spec.max_output_bytes);
        let stderr_task = read_limited(stderr, spec.max_output_bytes);

        let output = tokio::time::timeout(spec.timeout, async {
            let (stdout, stderr) = tokio::try_join!(stdout_task, stderr_task)?;
            let status = child.wait().await.map_err(CommandError::Io)?;
            Ok::<_, CommandError>((stdout, stderr, status))
        })
        .await;

        match output {
            Ok(Ok((stdout, stderr, status))) => {
                let duration = start.elapsed();
                record_metrics(spec.category, duration);
                if status.success() {
                    self.breaker.record_success(spec.category).await;
                    Ok(CommandOutput {
                        status,
                        stdout,
                        stderr,
                        duration,
                    })
                } else {
                    self.breaker.record_failure(spec.category).await?;
                    let code = status.code().unwrap_or(-1);
                    Err(CommandError::ExitNonZero(code))
                }
            }
            Ok(Err(err)) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                record_metrics(spec.category, start.elapsed());
                self.breaker.record_failure(spec.category).await?;
                Err(err)
            }
            Err(_) => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                record_metrics(spec.category, start.elapsed());
                self.breaker.record_failure(spec.category).await?;
                Err(CommandError::Timeout)
            }
        }
    }

    fn apply_defaults(&self, spec: &mut CommandSpec) {
        if spec.max_output_bytes == 0 {
            spec.max_output_bytes = self.config.max_output_bytes;
        }
        if spec.timeout == Duration::from_secs(0) {
            spec.timeout = match spec.category {
                CommandCategory::TmuxFast => self.config.tmux_timeout,
                CommandCategory::NtmStatus => self.config.ntm_status_timeout,
                CommandCategory::NtmTail => self.config.ntm_tail_timeout,
            };
        }
    }
}

fn record_metrics(category: CommandCategory, duration: Duration) {
    match category {
        CommandCategory::TmuxFast => METRICS.tmux_cmd.record(duration),
        CommandCategory::NtmStatus | CommandCategory::NtmTail => METRICS.ntm_cmd.record(duration),
    }
}

async fn read_limited<R: AsyncRead + Unpin>(
    mut reader: R,
    max_bytes: usize,
) -> Result<Vec<u8>, CommandError> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let read = reader.read(&mut chunk).await.map_err(CommandError::Io)?;
        if read == 0 {
            break;
        }
        if buffer.len() + read > max_bytes {
            return Err(CommandError::OutputTooLarge);
        }
        buffer.extend_from_slice(&chunk[..read]);
    }
    Ok(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn echo_spec(text: &str) -> CommandSpec {
        CommandSpec {
            program: "echo".to_string(),
            args: vec![text.to_string()],
            timeout: Duration::from_secs(2),
            max_output_bytes: 4096,
            category: CommandCategory::TmuxFast,
        }
    }

    fn failing_spec() -> CommandSpec {
        CommandSpec {
            program: "bash".to_string(),
            args: vec!["-c".to_string(), "exit 1".to_string()],
            timeout: Duration::from_secs(2),
            max_output_bytes: 4096,
            category: CommandCategory::TmuxFast,
        }
    }

    #[tokio::test]
    async fn timeout_triggers() {
        let runner = CommandRunner::new(CommandConfig::default());
        let spec = CommandSpec {
            program: "bash".to_string(),
            args: vec!["-c".to_string(), "sleep 1".to_string()],
            timeout: Duration::from_millis(10),
            max_output_bytes: 1024,
            category: CommandCategory::TmuxFast,
        };
        let result = runner.run(spec).await;
        assert!(matches!(result, Err(CommandError::Timeout)));
    }

    #[tokio::test]
    async fn output_cap_triggers() {
        let runner = CommandRunner::new(CommandConfig::default());
        let spec = CommandSpec {
            program: "bash".to_string(),
            args: vec!["-c".to_string(), "printf 'a%.0s' {1..2048}".to_string()],
            timeout: Duration::from_secs(1),
            max_output_bytes: 512,
            category: CommandCategory::TmuxFast,
        };
        let result = runner.run(spec).await;
        assert!(matches!(result, Err(CommandError::OutputTooLarge)));
    }

    // --- New tests ---

    #[test]
    fn command_config_defaults() {
        let config = CommandConfig::default();
        assert_eq!(config.max_concurrent, 4);
        assert_eq!(config.max_output_bytes, 256 * 1024);
        assert_eq!(config.tmux_timeout, Duration::from_secs(2));
        assert_eq!(config.ntm_status_timeout, Duration::from_secs(10));
        assert_eq!(config.ntm_tail_timeout, Duration::from_secs(15));
    }

    #[tokio::test]
    async fn successful_echo_command() {
        let runner = CommandRunner::new(CommandConfig::default());
        let result = runner.run(echo_spec("hello world")).await.unwrap();
        assert!(result.status.success());
        let stdout = String::from_utf8_lossy(&result.stdout);
        assert!(stdout.contains("hello world"));
    }

    #[tokio::test]
    async fn captures_stderr() {
        let runner = CommandRunner::new(CommandConfig::default());
        let spec = CommandSpec {
            program: "bash".to_string(),
            args: vec!["-c".to_string(), "echo err >&2; echo out".to_string()],
            timeout: Duration::from_secs(2),
            max_output_bytes: 4096,
            category: CommandCategory::TmuxFast,
        };
        let result = runner.run(spec).await.unwrap();
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(stderr.contains("err"));
        let stdout = String::from_utf8_lossy(&result.stdout);
        assert!(stdout.contains("out"));
    }

    #[tokio::test]
    async fn exit_nonzero_returns_error() {
        let runner = CommandRunner::new(CommandConfig::default());
        let result = runner.run(failing_spec()).await;
        assert!(matches!(result, Err(CommandError::ExitNonZero(1))));
    }

    #[tokio::test]
    async fn spawn_nonexistent_binary() {
        let runner = CommandRunner::new(CommandConfig::default());
        let spec = CommandSpec {
            program: "/nonexistent/binary".to_string(),
            args: vec![],
            timeout: Duration::from_secs(2),
            max_output_bytes: 4096,
            category: CommandCategory::TmuxFast,
        };
        let result = runner.run(spec).await;
        assert!(matches!(result, Err(CommandError::Spawn(_))));
    }

    #[tokio::test]
    async fn duration_is_recorded() {
        let runner = CommandRunner::new(CommandConfig::default());
        let spec = CommandSpec {
            program: "bash".to_string(),
            args: vec!["-c".to_string(), "sleep 0.05".to_string()],
            timeout: Duration::from_secs(2),
            max_output_bytes: 4096,
            category: CommandCategory::TmuxFast,
        };
        let result = runner.run(spec).await.unwrap();
        assert!(result.duration >= Duration::from_millis(30));
    }

    #[tokio::test]
    async fn apply_defaults_fills_zero_timeout() {
        let config = CommandConfig {
            tmux_timeout: Duration::from_secs(5),
            ..CommandConfig::default()
        };
        let runner = CommandRunner::new(config);
        let mut spec = echo_spec("test");
        spec.timeout = Duration::from_secs(0);
        // apply_defaults is called inside run() — just ensure it doesn't fail
        let result = runner.run(spec).await.unwrap();
        assert!(result.status.success());
    }

    #[tokio::test]
    async fn apply_defaults_fills_zero_output_bytes() {
        let runner = CommandRunner::new(CommandConfig::default());
        let mut spec = echo_spec("test");
        spec.max_output_bytes = 0;
        let result = runner.run(spec).await.unwrap();
        assert!(result.status.success());
    }

    #[tokio::test]
    async fn circuit_breaker_starts_closed() {
        let breaker = CircuitBreaker::new();
        // Should not error — circuit is closed initially
        breaker.check(CommandCategory::TmuxFast).await.unwrap();
    }

    #[tokio::test]
    async fn circuit_breaker_opens_after_ten_failures() {
        let breaker = CircuitBreaker::new();
        for _ in 0..10 {
            let _ = breaker.record_failure(CommandCategory::TmuxFast).await;
        }
        // Circuit should now be open
        let result = breaker.check(CommandCategory::TmuxFast).await;
        assert!(matches!(result, Err(CommandError::CircuitOpen)));
    }

    #[tokio::test]
    async fn circuit_breaker_success_resets() {
        let breaker = CircuitBreaker::new();
        for _ in 0..5 {
            let _ = breaker.record_failure(CommandCategory::TmuxFast).await;
        }
        breaker.record_success(CommandCategory::TmuxFast).await;
        // After success, circuit should be closed
        breaker.check(CommandCategory::TmuxFast).await.unwrap();
    }

    #[tokio::test]
    async fn circuit_breaker_per_category() {
        let breaker = CircuitBreaker::new();
        // Fail TmuxFast 10 times
        for _ in 0..10 {
            let _ = breaker.record_failure(CommandCategory::TmuxFast).await;
        }
        // TmuxFast is open
        assert!(breaker.check(CommandCategory::TmuxFast).await.is_err());
        // NtmStatus should still be closed
        breaker.check(CommandCategory::NtmStatus).await.unwrap();
    }

    #[tokio::test]
    async fn circuit_breaker_backoff_starts_at_three() {
        let breaker = CircuitBreaker::new();
        // 1-2 failures: no backoff
        breaker.record_failure(CommandCategory::NtmStatus).await.unwrap();
        breaker.record_failure(CommandCategory::NtmStatus).await.unwrap();
        breaker.check(CommandCategory::NtmStatus).await.unwrap();
        // 3rd failure: backoff starts
        breaker.record_failure(CommandCategory::NtmStatus).await.unwrap();
        let result = breaker.check(CommandCategory::NtmStatus).await;
        assert!(matches!(result, Err(CommandError::CircuitOpen)));
    }
}
