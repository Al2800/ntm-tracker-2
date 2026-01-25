use regex::Regex;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentType {
    Claude,
    Codex,
    Gemini,
    Shell,
    Unknown,
}

impl AgentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentType::Claude => "claude",
            AgentType::Codex => "codex",
            AgentType::Gemini => "gemini",
            AgentType::Shell => "shell",
            AgentType::Unknown => "unknown",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "claude" => AgentType::Claude,
            "codex" => AgentType::Codex,
            "gemini" => AgentType::Gemini,
            "shell" => AgentType::Shell,
            _ => AgentType::Unknown,
        }
    }
}

#[derive(Clone, Debug)]
pub struct AgentDetection {
    pub agent_type: AgentType,
    pub confidence: f32,
    pub matched_pattern: Option<String>,
}

struct CompiledPatterns {
    claude: Regex,
    codex: Regex,
    gemini: Regex,
    shell: Regex,
}

fn patterns() -> &'static CompiledPatterns {
    static PATTERNS: OnceLock<CompiledPatterns> = OnceLock::new();
    PATTERNS.get_or_init(|| CompiledPatterns {
        claude: Regex::new(r"(?i)(claude>|Claude Code|claude-code|anthropic)").unwrap(),
        codex: Regex::new(r"(?i)(codex>|OpenAI Codex|codex-cli|openai codex)").unwrap(),
        gemini: Regex::new(r"(?i)(gemini>|Google Gemini|gemini-cli|google gemini)").unwrap(),
        shell: Regex::new(r"(?m)(\$\s*$|bash-\d|#\s*$|❯\s*|➜\s*|>\s*$)").unwrap(),
    })
}

pub fn detect_agent_type(pane_output: &str) -> AgentDetection {
    let patterns = patterns();
    let stripped = strip_ansi(pane_output);

    // Check for Claude patterns (highest priority for AI agents)
    if let Some(m) = patterns.claude.find(&stripped) {
        return AgentDetection {
            agent_type: AgentType::Claude,
            confidence: 0.9,
            matched_pattern: Some(m.as_str().to_string()),
        };
    }

    // Check for Codex patterns
    if let Some(m) = patterns.codex.find(&stripped) {
        return AgentDetection {
            agent_type: AgentType::Codex,
            confidence: 0.9,
            matched_pattern: Some(m.as_str().to_string()),
        };
    }

    // Check for Gemini patterns
    if let Some(m) = patterns.gemini.find(&stripped) {
        return AgentDetection {
            agent_type: AgentType::Gemini,
            confidence: 0.9,
            matched_pattern: Some(m.as_str().to_string()),
        };
    }

    // Check for shell patterns (lower priority, as agents may show prompts too)
    if let Some(m) = patterns.shell.find(&stripped) {
        return AgentDetection {
            agent_type: AgentType::Shell,
            confidence: 0.6,
            matched_pattern: Some(m.as_str().to_string()),
        };
    }

    AgentDetection {
        agent_type: AgentType::Unknown,
        confidence: 0.0,
        matched_pattern: None,
    }
}

pub fn detect_from_command(command: Option<&str>) -> Option<AgentDetection> {
    let cmd = command?;
    let lowered = cmd.to_lowercase();

    if lowered.contains("claude") {
        return Some(AgentDetection {
            agent_type: AgentType::Claude,
            confidence: 0.95,
            matched_pattern: Some(cmd.to_string()),
        });
    }

    if lowered.contains("codex") {
        return Some(AgentDetection {
            agent_type: AgentType::Codex,
            confidence: 0.95,
            matched_pattern: Some(cmd.to_string()),
        });
    }

    if lowered.contains("gemini") {
        return Some(AgentDetection {
            agent_type: AgentType::Gemini,
            confidence: 0.95,
            matched_pattern: Some(cmd.to_string()),
        });
    }

    // Common shell commands
    if matches!(lowered.as_str(), "bash" | "zsh" | "fish" | "sh" | "dash") {
        return Some(AgentDetection {
            agent_type: AgentType::Shell,
            confidence: 0.8,
            matched_pattern: Some(cmd.to_string()),
        });
    }

    None
}

pub fn combined_detection(
    pane_output: Option<&str>,
    command: Option<&str>,
) -> AgentDetection {
    // Command-based detection has higher priority
    if let Some(detection) = detect_from_command(command) {
        if detection.confidence >= 0.9 {
            return detection;
        }
    }

    // Output-based detection
    if let Some(output) = pane_output {
        let detection = detect_agent_type(output);
        if detection.agent_type != AgentType::Unknown {
            return detection;
        }
    }

    // Fall back to command-based detection even with lower confidence
    if let Some(detection) = detect_from_command(command) {
        return detection;
    }

    AgentDetection {
        agent_type: AgentType::Unknown,
        confidence: 0.0,
        matched_pattern: None,
    }
}

fn strip_ansi(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if matches!(chars.peek(), Some('[')) {
                chars.next();
                while let Some(next) = chars.next() {
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
                continue;
            }
        }
        output.push(ch);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_claude_from_output() {
        let output = "Some text\nclaude> help\nMore text";
        let detection = detect_agent_type(output);
        assert_eq!(detection.agent_type, AgentType::Claude);
        assert!(detection.confidence >= 0.9);
    }

    #[test]
    fn detects_claude_code_pattern() {
        let output = "Starting Claude Code session...";
        let detection = detect_agent_type(output);
        assert_eq!(detection.agent_type, AgentType::Claude);
    }

    #[test]
    fn detects_codex_from_output() {
        let output = "codex> run tests";
        let detection = detect_agent_type(output);
        assert_eq!(detection.agent_type, AgentType::Codex);
    }

    #[test]
    fn detects_openai_codex_pattern() {
        let output = "Welcome to OpenAI Codex CLI";
        let detection = detect_agent_type(output);
        assert_eq!(detection.agent_type, AgentType::Codex);
    }

    #[test]
    fn detects_gemini_from_output() {
        let output = "gemini> analyze code";
        let detection = detect_agent_type(output);
        assert_eq!(detection.agent_type, AgentType::Gemini);
    }

    #[test]
    fn detects_google_gemini_pattern() {
        let output = "Initializing Google Gemini...";
        let detection = detect_agent_type(output);
        assert_eq!(detection.agent_type, AgentType::Gemini);
    }

    #[test]
    fn detects_shell_from_dollar_prompt() {
        let output = "user@host:~$ ";
        let detection = detect_agent_type(output);
        assert_eq!(detection.agent_type, AgentType::Shell);
        assert!(detection.confidence < 0.9);
    }

    #[test]
    fn detects_shell_from_hash_prompt() {
        let output = "root@host:~# ";
        let detection = detect_agent_type(output);
        assert_eq!(detection.agent_type, AgentType::Shell);
    }

    #[test]
    fn detects_shell_from_bash_prompt() {
        let output = "bash-5.1$ ";
        let detection = detect_agent_type(output);
        assert_eq!(detection.agent_type, AgentType::Shell);
    }

    #[test]
    fn returns_unknown_for_ambiguous_output() {
        let output = "some random text without patterns";
        let detection = detect_agent_type(output);
        assert_eq!(detection.agent_type, AgentType::Unknown);
        assert_eq!(detection.confidence, 0.0);
    }

    #[test]
    fn detects_claude_from_command() {
        let detection = detect_from_command(Some("claude")).unwrap();
        assert_eq!(detection.agent_type, AgentType::Claude);
        assert!(detection.confidence >= 0.9);
    }

    #[test]
    fn detects_shell_from_command() {
        let detection = detect_from_command(Some("bash")).unwrap();
        assert_eq!(detection.agent_type, AgentType::Shell);
    }

    #[test]
    fn detects_zsh_from_command() {
        let detection = detect_from_command(Some("zsh")).unwrap();
        assert_eq!(detection.agent_type, AgentType::Shell);
    }

    #[test]
    fn detects_fish_from_command() {
        let detection = detect_from_command(Some("fish")).unwrap();
        assert_eq!(detection.agent_type, AgentType::Shell);
    }

    #[test]
    fn returns_none_for_unknown_command() {
        let detection = detect_from_command(Some("vim"));
        assert!(detection.is_none());
    }

    #[test]
    fn combined_prefers_command_based() {
        let detection = combined_detection(
            Some("$"),  // Would detect as shell
            Some("claude"),  // Should detect as claude
        );
        assert_eq!(detection.agent_type, AgentType::Claude);
    }

    #[test]
    fn combined_falls_back_to_output() {
        let detection = combined_detection(
            Some("claude> help"),
            Some("vim"),  // Unknown command
        );
        assert_eq!(detection.agent_type, AgentType::Claude);
    }

    #[test]
    fn combined_returns_unknown_when_no_match() {
        let detection = combined_detection(
            Some("random text"),
            Some("vim"),
        );
        assert_eq!(detection.agent_type, AgentType::Unknown);
    }

    #[test]
    fn agent_type_as_str() {
        assert_eq!(AgentType::Claude.as_str(), "claude");
        assert_eq!(AgentType::Codex.as_str(), "codex");
        assert_eq!(AgentType::Gemini.as_str(), "gemini");
        assert_eq!(AgentType::Shell.as_str(), "shell");
        assert_eq!(AgentType::Unknown.as_str(), "unknown");
    }

    #[test]
    fn agent_type_from_str() {
        assert_eq!(AgentType::from_str("claude"), AgentType::Claude);
        assert_eq!(AgentType::from_str("CLAUDE"), AgentType::Claude);
        assert_eq!(AgentType::from_str("codex"), AgentType::Codex);
        assert_eq!(AgentType::from_str("gemini"), AgentType::Gemini);
        assert_eq!(AgentType::from_str("shell"), AgentType::Shell);
        assert_eq!(AgentType::from_str("anything"), AgentType::Unknown);
    }

    #[test]
    fn case_insensitive_detection() {
        let output = "CLAUDE CODE session";
        let detection = detect_agent_type(output);
        assert_eq!(detection.agent_type, AgentType::Claude);
    }

    #[test]
    fn strips_ansi_codes() {
        let output = "\x1b[32mclaude>\x1b[0m help";
        let detection = detect_agent_type(output);
        assert_eq!(detection.agent_type, AgentType::Claude);
    }

    #[test]
    fn detects_anthropic_pattern() {
        let output = "Powered by Anthropic";
        let detection = detect_agent_type(output);
        assert_eq!(detection.agent_type, AgentType::Claude);
    }

    #[test]
    fn detects_powerline_prompt() {
        let output = "❯ ";
        let detection = detect_agent_type(output);
        assert_eq!(detection.agent_type, AgentType::Shell);
    }

    #[test]
    fn detects_oh_my_zsh_prompt() {
        let output = "➜ project";
        let detection = detect_agent_type(output);
        assert_eq!(detection.agent_type, AgentType::Shell);
    }

    #[test]
    fn combined_with_none_values() {
        let detection = combined_detection(None, None);
        assert_eq!(detection.agent_type, AgentType::Unknown);
    }
}
