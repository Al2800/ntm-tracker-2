use ftui::PackedRgba;
use ftui::Style;

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
        "waiting" => INFO,
        _ => TEXT_SECONDARY,
    }
}

pub fn status_badge(status: &str) -> &'static str {
    match status {
        "active" => BADGE_ACTIVE,
        "idle" => BADGE_IDLE,
        "ended" => BADGE_ENDED,
        "waiting" => BADGE_WAITING,
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
    Style::new().fg(BG_BASE).bg(INFO).bold()
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

/// Unicode box-drawing characters for custom borders.
pub const BOX_HORIZONTAL: &str = "─";

/// Sparkline bar characters (increasing height).
pub const SPARK_CHARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

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
        assert_eq!(status_color("waiting"), INFO);
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
        // Spot-check known RGB values from design tokens
        assert_eq!(BG_BASE, PackedRgba::rgb(2, 6, 23));
        assert_eq!(ACTIVE, PackedRgba::rgb(52, 211, 153));
        assert_eq!(ERROR, PackedRgba::rgb(251, 113, 133));
        assert_eq!(INFO, PackedRgba::rgb(56, 189, 248));
    }

    #[test]
    fn test_spark_chars_length() {
        assert_eq!(SPARK_CHARS.len(), 8);
    }
}
