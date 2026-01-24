use crate::command::{CommandCategory, CommandRunner, CommandSpec};
use crate::parsers::ntm_markdown::{parse_ntm_markdown, NtmMarkdown};
use crate::parsers::ntm_tail::{parse_ntm_tail, NtmTail};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct NtmConfig {
    pub ntm_path: String,
    pub status_timeout: Duration,
    pub markdown_timeout: Duration,
    pub tail_timeout: Duration,
    pub max_output_bytes: usize,
}

impl Default for NtmConfig {
    fn default() -> Self {
        Self {
            ntm_path: "ntm".to_string(),
            status_timeout: Duration::from_secs(10),
            markdown_timeout: Duration::from_secs(20),
            tail_timeout: Duration::from_secs(15),
            max_output_bytes: 256 * 1024,
        }
    }
}

#[derive(Debug)]
pub struct NtmClient {
    runner: CommandRunner,
    config: NtmConfig,
}

#[derive(Debug)]
pub enum NtmError {
    Unavailable,
    CommandFailed(String),
    ParseFailed(String),
}

impl NtmClient {
    pub fn new(runner: CommandRunner, config: NtmConfig) -> Self {
        Self { runner, config }
    }

    pub async fn robot_markdown(&self) -> Result<NtmMarkdown, NtmError> {
        let spec = CommandSpec {
            program: self.config.ntm_path.clone(),
            args: vec![
                "--robot-markdown".to_string(),
                "--md-compact".to_string(),
                "--md-sections".to_string(),
                "sessions".to_string(),
            ],
            timeout: self.config.markdown_timeout,
            max_output_bytes: self.config.max_output_bytes,
            category: CommandCategory::NtmStatus,
        };
        let output = self
            .runner
            .run(spec)
            .await
            .map_err(|err| map_command_error(err))?;
        let text = String::from_utf8_lossy(&output.stdout);
        parse_ntm_markdown(&text).map_err(|err| NtmError::ParseFailed(err.reason))
    }

    pub async fn robot_tail(&self, session: &str, lines: u32) -> Result<NtmTail, NtmError> {
        let spec = CommandSpec {
            program: self.config.ntm_path.clone(),
            args: vec![
                "--robot-tail".to_string(),
                session.to_string(),
                "--lines".to_string(),
                lines.to_string(),
                "--json".to_string(),
            ],
            timeout: self.config.tail_timeout,
            max_output_bytes: self.config.max_output_bytes,
            category: CommandCategory::NtmTail,
        };
        let output = self
            .runner
            .run(spec)
            .await
            .map_err(|err| map_command_error(err))?;
        let text = String::from_utf8_lossy(&output.stdout);
        parse_ntm_tail(&text).map_err(|err| NtmError::ParseFailed(err.reason))
    }

    pub async fn list_sessions(&self) -> Result<Vec<String>, NtmError> {
        let spec = CommandSpec {
            program: self.config.ntm_path.clone(),
            args: vec!["list".to_string()],
            timeout: self.config.status_timeout,
            max_output_bytes: self.config.max_output_bytes,
            category: CommandCategory::NtmStatus,
        };
        let output = self
            .runner
            .run(spec)
            .await
            .map_err(|err| map_command_error(err))?;
        let text = String::from_utf8_lossy(&output.stdout);
        Ok(parse_session_lines(&text))
    }
}

fn map_command_error(err: crate::command::CommandError) -> NtmError {
    match err {
        crate::command::CommandError::Spawn(_) => NtmError::Unavailable,
        crate::command::CommandError::ExitNonZero(_) => NtmError::CommandFailed("exit code".to_string()),
        crate::command::CommandError::Timeout => NtmError::CommandFailed("timeout".to_string()),
        crate::command::CommandError::OutputTooLarge => {
            NtmError::CommandFailed("output too large".to_string())
        }
        crate::command::CommandError::Io(err) => NtmError::CommandFailed(err.to_string()),
        crate::command::CommandError::CircuitOpen => NtmError::CommandFailed("circuit open".to_string()),
    }
}

fn parse_session_lines(text: &str) -> Vec<String> {
    text.lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_session_lines;

    #[test]
    fn list_sessions_parses_lines() {
        let text = "alpha\nbeta\n";
        let sessions = parse_session_lines(text);
        assert_eq!(sessions, vec!["alpha".to_string(), "beta".to_string()]);
    }
}
