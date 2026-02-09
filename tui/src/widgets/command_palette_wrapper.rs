use crate::rpc::types::{PaneView, SessionView};
use ftui::widgets::command_palette::{ActionItem, CommandPalette, PaletteAction};
use ftui::Event;

/// Build command palette actions dynamically from current state.
pub fn build_actions(
    sessions: &[SessionView],
    panes: &[PaneView],
) -> Vec<ActionItem> {
    let mut actions = Vec::new();

    // Tab navigation
    for tab in &["Dashboard", "Sessions", "Events", "Health"] {
        actions.push(
            ActionItem::new(
                format!("tab:{}", tab.to_lowercase()),
                format!("Tab: {tab}"),
            )
            .with_category("Navigation"),
        );
    }

    // Go to session
    for s in sessions {
        actions.push(
            ActionItem::new(
                format!("goto:{}", s.session_id),
                format!("Go to: {}", s.name),
            )
            .with_category("Sessions")
            .with_tags(&[&s.status]),
        );
    }

    // Session actions
    for s in sessions {
        actions.push(
            ActionItem::new(
                format!("kill:{}", s.session_id),
                format!("Kill: {}", s.name),
            )
            .with_category("Actions")
            .with_tags(&["dangerous"]),
        );
    }

    // Send to pane actions
    for s in sessions {
        let session_panes: Vec<&PaneView> = panes
            .iter()
            .filter(|p| p.session_id == s.session_id)
            .collect();
        for pane in session_panes {
            actions.push(
                ActionItem::new(
                    format!("send:{}:{}", pane.tmux_pane_id.as_deref().unwrap_or(&pane.pane_id), s.name),
                    format!("Send to: {} #{}", s.name, pane.pane_index),
                )
                .with_category("Actions")
                .with_tags(&["pane", "send"]),
            );
        }
    }

    actions
}

/// Wrapper state for command palette visibility and actions.
pub struct PaletteState {
    pub palette: CommandPalette,
    pub visible: bool,
}

impl PaletteState {
    pub fn new() -> Self {
        Self {
            palette: CommandPalette::new(),
            visible: false,
        }
    }

    pub fn open(&mut self, sessions: &[SessionView], panes: &[PaneView]) {
        let actions = build_actions(sessions, panes);
        self.palette.replace_actions(actions);
        self.palette.open();
        self.visible = true;
    }

    pub fn close(&mut self) {
        self.palette.close();
        self.visible = false;
    }

    pub fn toggle(&mut self, sessions: &[SessionView], panes: &[PaneView]) {
        if self.visible {
            self.close();
        } else {
            self.open(sessions, panes);
        }
    }

    /// Handle an event, returning the selected action ID if one was picked.
    pub fn handle_event(&mut self, event: &Event) -> Option<String> {
        if !self.visible {
            return None;
        }
        match self.palette.handle_event(event) {
            Some(PaletteAction::Execute(id)) => {
                self.close();
                Some(id)
            }
            Some(PaletteAction::Dismiss) => {
                self.close();
                None
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::types::SessionView;

    fn make_session(id: &str, name: &str) -> SessionView {
        SessionView {
            session_id: id.to_string(),
            name: name.to_string(),
            status: "active".to_string(),
            ..Default::default()
        }
    }

    fn make_pane(id: &str, session_id: &str) -> PaneView {
        PaneView {
            pane_id: id.to_string(),
            session_id: session_id.to_string(),
            pane_index: 0,
            tmux_pane_id: Some(format!("%{}", id.trim_start_matches('p'))),
            ..Default::default()
        }
    }

    #[test]
    fn test_build_actions_includes_tabs() {
        let sessions = vec![make_session("s1", "project")];
        let actions = build_actions(&sessions, &[]);
        assert!(actions.iter().any(|a| a.title == "Tab: Dashboard"));
        assert!(actions.iter().any(|a| a.title == "Tab: Health"));
    }

    #[test]
    fn test_build_actions_includes_sessions() {
        let sessions = vec![
            make_session("s1", "project-a"),
            make_session("s2", "project-b"),
        ];
        let actions = build_actions(&sessions, &[]);
        assert!(actions.iter().any(|a| a.title == "Go to: project-a"));
        assert!(actions.iter().any(|a| a.title == "Kill: project-b"));
    }

    #[test]
    fn test_build_actions_includes_send_entries() {
        let sessions = vec![make_session("s1", "project-a")];
        let panes = vec![make_pane("p1", "s1")];
        let actions = build_actions(&sessions, &panes);
        assert!(actions.iter().any(|a| a.title == "Send to: project-a #0"));
    }

    #[test]
    fn test_palette_state_toggle() {
        let mut state = PaletteState::new();
        assert!(!state.visible);
        state.toggle(&[], &[]);
        assert!(state.visible);
        state.toggle(&[], &[]);
        assert!(!state.visible);
    }

    #[test]
    fn test_palette_state_close() {
        let mut state = PaletteState::new();
        state.open(&[], &[]);
        assert!(state.visible);
        state.close();
        assert!(!state.visible);
    }
}
