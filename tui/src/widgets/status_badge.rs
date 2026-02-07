use crate::theme;
use ftui::PackedRgba;
use ftui::Style;

/// Get a styled badge string for a status.
pub fn badge_text(status: &str) -> String {
    let icon = theme::status_badge(status);
    format!("{icon} {status}")
}

/// Get the color for a status string.
pub fn badge_color(status: &str) -> PackedRgba {
    theme::status_color(status)
}

/// Build a Style for the given status.
pub fn badge_style(status: &str) -> Style {
    Style::new().fg(badge_color(status))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_badge_text_active() {
        let text = badge_text("active");
        assert!(text.contains("active"));
        assert!(text.contains(theme::BADGE_ACTIVE));
    }

    #[test]
    fn test_badge_text_idle() {
        let text = badge_text("idle");
        assert!(text.contains("idle"));
        assert!(text.contains(theme::BADGE_IDLE));
    }

    #[test]
    fn test_badge_text_unknown() {
        let text = badge_text("something");
        assert!(text.contains("something"));
    }

    #[test]
    fn test_badge_color_matches_theme() {
        assert_eq!(badge_color("active"), theme::status_color("active"));
        assert_eq!(badge_color("idle"), theme::status_color("idle"));
        assert_eq!(badge_color("ended"), theme::status_color("ended"));
        assert_eq!(badge_color("unknown"), theme::status_color("unknown"));
    }

    #[test]
    fn test_badge_style_has_fg() {
        let _ = badge_style("active");
        let _ = badge_style("idle");
        let _ = badge_style("ended");
        // No panic = success; Style internals aren't easily inspectable
    }
}
