mod helpers;

use helpers::logging::TestLogger;
use ntm_tracker_tui::app::NtmApp;
use ntm_tracker_tui::msg::{FocusArea, Msg, Tab};
use ntm_tracker_tui::rpc::types::*;
use ftui::{Cmd, Event, KeyCode, KeyEvent, KeyEventKind, Model, Modifiers};

// ================================================================
// bd-2ajo: E2E full keyboard navigation flow
// ================================================================

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code)
}

fn key_msg(code: KeyCode) -> Msg {
    Msg::Term(Event::Key(key(code)))
}

fn populated_app() -> NtmApp {
    let mut app = NtmApp::new();
    app.sessions = vec![
        SessionView {
            session_id: "s1".into(),
            name: "project-a".into(),
            status: "active".into(),
            pane_count: 2,
            source_id: "tmux".into(),
            ..Default::default()
        },
        SessionView {
            session_id: "s2".into(),
            name: "project-b".into(),
            status: "active".into(),
            pane_count: 3,
            source_id: "tmux".into(),
            ..Default::default()
        },
        SessionView {
            session_id: "s3".into(),
            name: "project-c".into(),
            status: "idle".into(),
            pane_count: 1,
            source_id: "tmux".into(),
            ..Default::default()
        },
    ];
    app.panes = vec![
        PaneView { pane_id: "p1".into(), session_id: "s1".into(), status: "active".into(), ..Default::default() },
        PaneView { pane_id: "p2".into(), session_id: "s1".into(), status: "idle".into(), ..Default::default() },
        PaneView { pane_id: "p3".into(), session_id: "s2".into(), status: "active".into(), ..Default::default() },
        PaneView { pane_id: "p4".into(), session_id: "s2".into(), status: "active".into(), ..Default::default() },
        PaneView { pane_id: "p5".into(), session_id: "s2".into(), status: "idle".into(), ..Default::default() },
        PaneView { pane_id: "p6".into(), session_id: "s3".into(), status: "idle".into(), ..Default::default() },
    ];
    app.events = vec![
        EventView { id: 1, event_type: "session_start".into(), session_id: "s1".into(), pane_id: "p1".into(), detected_at: 1700000001, severity: None, status: None },
        EventView { id: 2, event_type: "compact".into(), session_id: "s1".into(), pane_id: "p1".into(), detected_at: 1700000002, severity: None, status: None },
        EventView { id: 3, event_type: "escalation".into(), session_id: "s2".into(), pane_id: "p3".into(), detected_at: 1700000003, severity: Some("high".into()), status: Some("pending".into()) },
        EventView { id: 4, event_type: "session_start".into(), session_id: "s3".into(), pane_id: "p6".into(), detected_at: 1700000004, severity: None, status: None },
        EventView { id: 5, event_type: "escalation".into(), session_id: "s1".into(), pane_id: "p1".into(), detected_at: 1700000005, severity: Some("high".into()), status: Some("pending".into()) },
    ];
    app.stats = StatsSummary {
        sessions: 3,
        panes: 6,
        total_compacts: 1,
        active_minutes: 90,
        estimated_tokens: 25000,
    };
    app
}

/// Complete walkthrough: navigate sessions, expand, switch panels, switch tabs, help, quit.
#[test]
fn test_full_navigation_flow() {
    let logger = TestLogger::new("test_full_navigation_flow");
    let mut app = populated_app();
    let mut total_keys = 0;

    macro_rules! send_key {
        ($code:expr, $desc:expr) => {{
            let cmd = app.update(key_msg($code));
            total_keys += 1;
            logger.log(&format!(
                "  Key {:>2}: {:<20} -> tab={:?}, focus={:?}, help={}",
                total_keys, $desc, app.tab, app.focus, app.show_help
            ));
            cmd
        }};
    }

    logger.step("Navigate sessions: j, j, k");
    send_key!(KeyCode::Char('j'), "j (session down)");
    assert_eq!(app.session_list_state.borrow().list_state.selected(), Some(1));
    send_key!(KeyCode::Char('j'), "j (session down)");
    assert_eq!(app.session_list_state.borrow().list_state.selected(), Some(2));
    send_key!(KeyCode::Char('k'), "k (session up)");
    assert_eq!(app.session_list_state.borrow().list_state.selected(), Some(1));
    logger.step_result(true, "Session navigation correct");

    logger.step("Expand session with Enter");
    send_key!(KeyCode::Enter, "Enter (expand)");
    assert_eq!(app.session_list_state.borrow().expanded_index, Some(1));
    logger.step_result(true, "Session expanded at index 1");

    logger.step("Switch to pane table with Tab");
    send_key!(KeyCode::Tab, "Tab (focus next)");
    assert_eq!(app.focus, FocusArea::PaneTable);
    logger.step_result(true, "Focus moved to PaneTable");

    logger.step("Navigate panes: j, j");
    send_key!(KeyCode::Char('j'), "j (pane down)");
    assert_eq!(app.pane_table_state.borrow().list_state.selected(), Some(1));
    send_key!(KeyCode::Char('j'), "j (pane down)");
    assert_eq!(app.pane_table_state.borrow().list_state.selected(), Some(2));
    logger.step_result(true, "Pane navigation correct (s2 has 3 panes)");

    logger.step("Switch tabs: 2, 3, 4, 1");
    send_key!(KeyCode::Char('2'), "2 (Sessions tab)");
    assert_eq!(app.tab, Tab::Sessions);
    send_key!(KeyCode::Char('3'), "3 (Events tab)");
    assert_eq!(app.tab, Tab::Events);
    send_key!(KeyCode::Char('4'), "4 (Health tab)");
    assert_eq!(app.tab, Tab::Health);
    send_key!(KeyCode::Char('1'), "1 (Dashboard tab)");
    assert_eq!(app.tab, Tab::Dashboard);
    logger.step_result(true, "Tab switching works");

    logger.step("Toggle help overlay");
    send_key!(KeyCode::Char('?'), "? (toggle help)");
    assert!(app.show_help);
    send_key!(KeyCode::Char('x'), "x (dismiss help)");
    assert!(!app.show_help);
    logger.step_result(true, "Help overlay toggle works");

    logger.step("Navigate to escalations via Tab");
    // Currently at PaneTable, need: Tab -> EscalationInbox
    send_key!(KeyCode::Tab, "Tab");
    assert_eq!(app.focus, FocusArea::EscalationInbox);
    send_key!(KeyCode::Char('j'), "j (escalation down)");
    assert_eq!(app.escalation_state.borrow().list_state.selected(), Some(1));
    logger.step_result(true, "Escalation navigation works");

    logger.step("Quit with q");
    let cmd = send_key!(KeyCode::Char('q'), "q (quit)");
    assert!(matches!(cmd, Cmd::Quit));
    logger.step_result(true, "Quit command received");

    logger.log(&format!("Total keys processed: {total_keys}"));
    logger.finish(true);
}

/// Verify j/Down and k/Up produce identical state.
#[test]
fn test_vim_and_arrow_keys_equivalent() {
    let logger = TestLogger::new("test_vim_and_arrow_keys_equivalent");

    logger.step("Create two apps and compare j vs Down, k vs Up");

    let mut app_vim = populated_app();
    let mut app_arrow = populated_app();

    // Navigate with vim keys
    app_vim.update(key_msg(KeyCode::Char('j')));
    app_vim.update(key_msg(KeyCode::Char('j')));
    app_vim.update(key_msg(KeyCode::Char('k')));

    // Navigate with arrow keys
    app_arrow.update(key_msg(KeyCode::Down));
    app_arrow.update(key_msg(KeyCode::Down));
    app_arrow.update(key_msg(KeyCode::Up));

    assert_eq!(
        app_vim.session_list_state.borrow().list_state.selected(),
        app_arrow.session_list_state.borrow().list_state.selected(),
        "j/Down and k/Up should produce identical session selection"
    );
    logger.step_result(true, "Session list: vim keys == arrow keys");

    // Test in pane table
    app_vim.focus = FocusArea::PaneTable;
    app_arrow.focus = FocusArea::PaneTable;

    app_vim.update(key_msg(KeyCode::Char('j')));
    app_arrow.update(key_msg(KeyCode::Down));

    assert_eq!(
        app_vim.pane_table_state.borrow().list_state.selected(),
        app_arrow.pane_table_state.borrow().list_state.selected(),
        "Pane table: vim keys == arrow keys"
    );
    logger.step_result(true, "Pane table: vim keys == arrow keys");

    // Test in event timeline
    app_vim.focus = FocusArea::EventTimeline;
    app_arrow.focus = FocusArea::EventTimeline;

    app_vim.update(key_msg(KeyCode::Char('j')));
    app_arrow.update(key_msg(KeyCode::Down));

    assert_eq!(
        app_vim.event_timeline_state.borrow().list_state.selected(),
        app_arrow.event_timeline_state.borrow().list_state.selected(),
        "Event timeline: vim keys == arrow keys"
    );
    logger.step_result(true, "Event timeline: vim keys == arrow keys");

    logger.finish(true);
}

/// Verify Tab cycling wraps around after visiting all panels.
#[test]
fn test_tab_cycling_wraps() {
    let logger = TestLogger::new("test_tab_cycling_wraps");
    let mut app = populated_app();

    logger.step("Starting at SessionList, pressing Tab 4 times");
    assert_eq!(app.focus, FocusArea::SessionList);

    app.update(key_msg(KeyCode::Tab));
    assert_eq!(app.focus, FocusArea::PaneTable);
    logger.log("  Tab 1: PaneTable");

    app.update(key_msg(KeyCode::Tab));
    assert_eq!(app.focus, FocusArea::EscalationInbox);
    logger.log("  Tab 2: EscalationInbox");

    app.update(key_msg(KeyCode::Tab));
    assert_eq!(app.focus, FocusArea::EventTimeline);
    logger.log("  Tab 3: EventTimeline");

    app.update(key_msg(KeyCode::Tab));
    assert_eq!(app.focus, FocusArea::SessionList);
    logger.log("  Tab 4: SessionList (wrapped!)");

    logger.step_result(true, "Tab cycling wraps after 4 presses");

    logger.step("BackTab cycling (reverse)");
    app.update(key_msg(KeyCode::BackTab));
    assert_eq!(app.focus, FocusArea::EventTimeline);
    app.update(key_msg(KeyCode::BackTab));
    assert_eq!(app.focus, FocusArea::EscalationInbox);
    app.update(key_msg(KeyCode::BackTab));
    assert_eq!(app.focus, FocusArea::PaneTable);
    app.update(key_msg(KeyCode::BackTab));
    assert_eq!(app.focus, FocusArea::SessionList);
    logger.step_result(true, "BackTab wraps in reverse");

    logger.finish(true);
}

/// Verify navigation doesn't crash with empty data.
#[test]
fn test_navigation_with_empty_data() {
    let logger = TestLogger::new("test_navigation_with_empty_data");
    let mut app = NtmApp::new();

    logger.step("Navigating with empty sessions/panes/events");

    // Try all navigation in every focus area
    for &focus in &[
        FocusArea::SessionList,
        FocusArea::PaneTable,
        FocusArea::EventTimeline,
        FocusArea::EscalationInbox,
    ] {
        app.focus = focus;
        app.update(key_msg(KeyCode::Char('j')));
        app.update(key_msg(KeyCode::Char('k')));
        app.update(key_msg(KeyCode::Down));
        app.update(key_msg(KeyCode::Up));
        logger.log(&format!("  {:?}: j/k/Down/Up OK", focus));
    }
    logger.step_result(true, "All navigation works with empty data");

    logger.step("Tab switching with empty data");
    for c in ['1', '2', '3', '4'] {
        app.update(key_msg(KeyCode::Char(c)));
    }
    logger.step_result(true, "Tab switching OK with empty data");

    logger.step("Help toggle with empty data");
    app.update(key_msg(KeyCode::Char('?')));
    assert!(app.show_help);
    app.update(key_msg(KeyCode::Char('j'))); // dismiss
    assert!(!app.show_help);
    logger.step_result(true, "Help toggle OK with empty data");

    logger.step("Session expand with empty list");
    app.focus = FocusArea::SessionList;
    app.update(key_msg(KeyCode::Enter));
    // Should not panic
    logger.step_result(true, "Enter on empty list: no crash");

    logger.step("Session G (select last) on empty list");
    app.update(key_msg(KeyCode::Char('G')));
    logger.step_result(true, "G on empty list: no crash");

    logger.step("Session g (select first) on empty list");
    app.update(key_msg(KeyCode::Char('g')));
    logger.step_result(true, "g on empty list: no crash");

    logger.finish(true);
}
