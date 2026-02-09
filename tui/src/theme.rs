use ftui::PackedRgba;
use ftui::Style;
use ftui::widgets::block::Block;
use ftui::widgets::borders::Borders;

// ── Base backgrounds ──────────────────────────────────────────────
pub const BG_BASE: PackedRgba = PackedRgba::rgb(2, 6, 23);       // slate-950
pub const BG_RAISED: PackedRgba = PackedRgba::rgb(15, 23, 42);   // slate-900
pub const BG_SURFACE: PackedRgba = PackedRgba::rgb(30, 41, 59);  // slate-800

// ── Text ──────────────────────────────────────────────────────────
pub const TEXT_PRIMARY: PackedRgba = PackedRgba::rgb(241, 245, 249);   // slate-100
pub const TEXT_SECONDARY: PackedRgba = PackedRgba::rgb(148, 163, 184); // slate-400
pub const TEXT_MUTED: PackedRgba = PackedRgba::rgb(100, 116, 139);     // slate-500

// ── Semantic colors ───────────────────────────────────────────────
pub const ACTIVE: PackedRgba = PackedRgba::rgb(52, 211, 153);    // emerald-400
pub const IDLE: PackedRgba = PackedRgba::rgb(251, 191, 36);      // amber-400
pub const ERROR: PackedRgba = PackedRgba::rgb(251, 113, 133);    // rose-400
pub const INFO: PackedRgba = PackedRgba::rgb(56, 189, 248);      // sky-400
pub const ACCENT: PackedRgba = PackedRgba::rgb(129, 140, 248);   // indigo-400
pub const PAUSED: PackedRgba = PackedRgba::rgb(168, 85, 247);    // violet-400

// ── Highlight ────────────────────────────────────────────────────
pub const HIGHLIGHT_BG: PackedRgba = PackedRgba::rgb(30, 58, 95);   // subtle blue tint

// ── Border colors ─────────────────────────────────────────────────
pub const BORDER_DIM: PackedRgba = PackedRgba::rgb(51, 65, 85);  // slate-700
pub const BORDER_FOCUS: PackedRgba = PackedRgba::rgb(56, 189, 248); // sky-400

// ── Status badges ─────────────────────────────────────────────────
pub const BADGE_ACTIVE: &str = "●";
pub const BADGE_IDLE: &str = "○";
pub const BADGE_WAITING: &str = "◉";
pub const BADGE_ENDED: &str = "◌";

pub fn status_color(status: &str) -> PackedRgba {
    match status {
        "active" => ACTIVE,
        "idle" => IDLE,
        "ended" => TEXT_MUTED,
        "waiting" | "paused" => PAUSED,
        _ => TEXT_SECONDARY,
    }
}

pub fn status_badge(status: &str) -> &'static str {
    match status {
        "active" => BADGE_ACTIVE,
        "idle" => BADGE_IDLE,
        "ended" => BADGE_ENDED,
        "waiting" | "paused" => BADGE_WAITING,
        _ => BADGE_IDLE,
    }
}

// ── Style helpers ─────────────────────────────────────────────────

pub fn base_style() -> Style {
    Style::new().fg(TEXT_PRIMARY).bg(BG_BASE)
}

pub fn raised_style() -> Style {
    Style::new().fg(TEXT_PRIMARY).bg(BG_RAISED)
}

pub fn muted_style() -> Style {
    Style::new().fg(TEXT_SECONDARY).bg(BG_BASE)
}

pub fn title_style() -> Style {
    Style::new().fg(TEXT_PRIMARY).bold()
}

pub fn highlight_style() -> Style {
    Style::new().fg(TEXT_PRIMARY).bg(HIGHLIGHT_BG).bold()
}

pub fn active_tab_style() -> Style {
    Style::new().fg(INFO).bold()
}

pub fn inactive_tab_style() -> Style {
    Style::new().fg(TEXT_SECONDARY)
}

pub fn error_style() -> Style {
    Style::new().fg(ERROR)
}

/// Build a block with rounded borders and focus-aware coloring.
pub fn panel_block(title: &str, focused: bool) -> Block<'_> {
    let border_color = if focused { BORDER_FOCUS } else { BORDER_DIM };
    Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::new().fg(border_color))
        .style(raised_style())
}

/// Map event severity to a color.
pub fn severity_color(severity: &str) -> PackedRgba {
    match severity {
        "critical" | "high" => ERROR,
        "medium" | "warning" => IDLE,
        "low" | "info" => INFO,
        _ => TEXT_SECONDARY,
    }
}

/// Map agent type to a short label.
pub fn agent_label(agent: &str) -> &'static str {
    match agent {
        "claude-code" | "cc" => "CC",
        "codex" | "codex-cli" | "cx" => "CX",
        "cursor" => "CU",
        "aider" => "AI",
        _ => "--",
    }
}

/// Agent badge style: color-coded by agent type.
pub fn agent_badge_style(agent: &str) -> Style {
    let color = match agent {
        "claude-code" | "cc" | "CC" => INFO,
        "codex" | "codex-cli" | "cx" | "CX" => ACTIVE,
        "cursor" | "CU" => IDLE,
        "aider" | "AI" => ACCENT,
        _ => TEXT_MUTED,
    };
    Style::new().fg(color).bold()
}

/// Format a unix timestamp as relative time: "just now", "2m", "1h", "3d".
pub fn relative_time(unix_ts: i64) -> String {
    let now = chrono::Utc::now().timestamp();
    let delta = now - unix_ts;
    if delta < 0 {
        return "future".to_string();
    }
    if delta < 10 {
        "just now".to_string()
    } else if delta < 60 {
        format!("{delta}s")
    } else if delta < 3600 {
        format!("{}m", delta / 60)
    } else if delta < 86400 {
        format!("{}h", delta / 3600)
    } else {
        format!("{}d", delta / 86400)
    }
}

/// Format token count in human-readable form: "50K", "1.2M".
pub fn format_tokens(tokens: u64) -> String {
    if tokens >= 1_000_000 {
        let m = tokens as f64 / 1_000_000.0;
        format!("{m:.1}M")
    } else if tokens >= 1_000 {
        let k = tokens as f64 / 1_000.0;
        if k >= 100.0 {
            format!("{:.0}K", k)
        } else if k >= 10.0 {
            format!("{:.0}K", k)
        } else {
            format!("{k:.1}K")
        }
    } else {
        format!("{tokens}")
    }
}

/// Event type icon character.
pub fn event_type_icon(event_type: &str) -> &'static str {
    match event_type {
        "escalation" => "!",
        "compact" => "◆",
        "session_start" => "►",
        "session_end" => "■",
        _ => "·",
    }
}

/// Unicode box-drawing characters for custom borders.
pub const BOX_HORIZONTAL: &str = "─";

/// Sparkline bar characters (increasing height).
pub const SPARK_CHARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Spinner braille dot frames for animations.
pub const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Tree guide characters.
pub const TREE_BRANCH: &str = "├── ";
pub const TREE_LAST: &str = "└── ";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_color_active() {
        assert_eq!(status_color("active"), ACTIVE);
    }

    #[test]
    fn test_status_color_idle() {
        assert_eq!(status_color("idle"), IDLE);
    }

    #[test]
    fn test_status_color_ended() {
        assert_eq!(status_color("ended"), TEXT_MUTED);
    }

    #[test]
    fn test_status_color_waiting() {
        assert_eq!(status_color("waiting"), PAUSED);
    }

    #[test]
    fn test_status_color_paused() {
        assert_eq!(status_color("paused"), PAUSED);
    }

    #[test]
    fn test_status_color_unknown() {
        assert_eq!(status_color("unknown"), TEXT_SECONDARY);
        assert_eq!(status_color(""), TEXT_SECONDARY);
        assert_eq!(status_color("garbage"), TEXT_SECONDARY);
    }

    #[test]
    fn test_status_badge_all_variants() {
        assert_eq!(status_badge("active"), BADGE_ACTIVE);
        assert_eq!(status_badge("idle"), BADGE_IDLE);
        assert_eq!(status_badge("ended"), BADGE_ENDED);
        assert_eq!(status_badge("waiting"), BADGE_WAITING);
        assert_eq!(status_badge("paused"), BADGE_WAITING);
        assert_eq!(status_badge("unknown"), BADGE_IDLE); // default
    }

    #[test]
    fn test_style_helpers_not_panic() {
        let _ = base_style();
        let _ = raised_style();
        let _ = muted_style();
        let _ = title_style();
        let _ = highlight_style();
        let _ = active_tab_style();
        let _ = inactive_tab_style();
        let _ = error_style();
    }

    #[test]
    fn test_color_constants_valid() {
        assert_eq!(BG_BASE, PackedRgba::rgb(2, 6, 23));
        assert_eq!(ACTIVE, PackedRgba::rgb(52, 211, 153));
        assert_eq!(ERROR, PackedRgba::rgb(251, 113, 133));
        assert_eq!(INFO, PackedRgba::rgb(56, 189, 248));
        assert_eq!(PAUSED, PackedRgba::rgb(168, 85, 247));
    }

    #[test]
    fn test_spark_chars_length() {
        assert_eq!(SPARK_CHARS.len(), 8);
    }

    #[test]
    fn test_panel_block_focused() {
        let _ = panel_block(" Test ", true);
    }

    #[test]
    fn test_panel_block_unfocused() {
        let _ = panel_block(" Test ", false);
    }

    #[test]
    fn test_severity_color_critical() {
        assert_eq!(severity_color("critical"), ERROR);
        assert_eq!(severity_color("high"), ERROR);
    }

    #[test]
    fn test_severity_color_medium() {
        assert_eq!(severity_color("medium"), IDLE);
    }

    #[test]
    fn test_severity_color_low() {
        assert_eq!(severity_color("low"), INFO);
    }

    #[test]
    fn test_agent_label_known() {
        assert_eq!(agent_label("claude-code"), "CC");
        assert_eq!(agent_label("codex"), "CX");
        assert_eq!(agent_label("cursor"), "CU");
        assert_eq!(agent_label("aider"), "AI");
    }

    #[test]
    fn test_agent_label_unknown() {
        assert_eq!(agent_label("other"), "--");
    }

    #[test]
    fn test_agent_badge_style_returns_style() {
        let _ = agent_badge_style("claude-code");
        let _ = agent_badge_style("unknown");
    }

    #[test]
    fn test_relative_time_just_now() {
        let now = chrono::Utc::now().timestamp();
        assert_eq!(relative_time(now), "just now");
    }

    #[test]
    fn test_relative_time_seconds() {
        let now = chrono::Utc::now().timestamp();
        let result = relative_time(now - 30);
        assert!(result.contains("s"), "Expected seconds, got: {result}");
    }

    #[test]
    fn test_relative_time_minutes() {
        let now = chrono::Utc::now().timestamp();
        let result = relative_time(now - 120);
        assert!(result.contains("m"), "Expected minutes, got: {result}");
    }

    #[test]
    fn test_relative_time_hours() {
        let now = chrono::Utc::now().timestamp();
        let result = relative_time(now - 7200);
        assert!(result.contains("h"), "Expected hours, got: {result}");
    }

    #[test]
    fn test_relative_time_days() {
        let now = chrono::Utc::now().timestamp();
        let result = relative_time(now - 172800);
        assert!(result.contains("d"), "Expected days, got: {result}");
    }

    #[test]
    fn test_relative_time_future() {
        let now = chrono::Utc::now().timestamp();
        assert_eq!(relative_time(now + 100), "future");
    }

    #[test]
    fn test_format_tokens_small() {
        assert_eq!(format_tokens(500), "500");
    }

    #[test]
    fn test_format_tokens_thousands() {
        assert_eq!(format_tokens(50000), "50K");
    }

    #[test]
    fn test_format_tokens_millions() {
        assert_eq!(format_tokens(1_200_000), "1.2M");
    }

    #[test]
    fn test_format_tokens_small_k() {
        assert_eq!(format_tokens(1500), "1.5K");
    }

    #[test]
    fn test_event_type_icon() {
        assert_eq!(event_type_icon("escalation"), "!");
        assert_eq!(event_type_icon("compact"), "◆");
        assert_eq!(event_type_icon("session_start"), "►");
        assert_eq!(event_type_icon("session_end"), "■");
        assert_eq!(event_type_icon("other"), "·");
    }
}
