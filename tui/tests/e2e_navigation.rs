mod helpers;

use helpers::logging::TestLogger;
use helpers::render::TestFrame;
use ntm_tracker_tui::app::NtmApp;
use ntm_tracker_tui::msg::{ConfirmAction, FocusArea, Msg, Tab, ToastLevel};
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
    // Build row_map so session-aware navigation works
    app.session_list_state.borrow_mut().build_row_map(&app.sessions, &app.panes);
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
    assert_eq!(app.pane_table_state.borrow().table_state.selected, Some(1));
    send_key!(KeyCode::Char('j'), "j (pane down)");
    assert_eq!(app.pane_table_state.borrow().table_state.selected, Some(2));
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

    logger.step("Navigate to event timeline via Tab, then escalations");
    // Currently at PaneTable, need: Tab -> EventTimeline -> EscalationInbox
    send_key!(KeyCode::Tab, "Tab");
    assert_eq!(app.focus, FocusArea::EventTimeline);
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
        app_vim.pane_table_state.borrow().table_state.selected,
        app_arrow.pane_table_state.borrow().table_state.selected,
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
    assert_eq!(app.focus, FocusArea::EventTimeline);
    logger.log("  Tab 2: EventTimeline");

    app.update(key_msg(KeyCode::Tab));
    assert_eq!(app.focus, FocusArea::EscalationInbox);
    logger.log("  Tab 3: EscalationInbox");

    app.update(key_msg(KeyCode::Tab));
    assert_eq!(app.focus, FocusArea::SessionList);
    logger.log("  Tab 4: SessionList (wrapped!)");

    logger.step_result(true, "Tab cycling wraps after 4 presses");

    logger.step("BackTab cycling (reverse)");
    app.update(key_msg(KeyCode::BackTab));
    assert_eq!(app.focus, FocusArea::EscalationInbox);
    app.update(key_msg(KeyCode::BackTab));
    assert_eq!(app.focus, FocusArea::EventTimeline);
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
    app.update(key_msg(KeyCode::Escape)); // dismiss
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

// ================================================================
// bd-2mhi: E2E toast notification lifecycle
// ================================================================

/// Single toast: trigger via ToastShow message, verify it appears in queue and renders.
#[test]
fn test_toast_single_appears_and_renders() {
    let logger = TestLogger::new("test_toast_single_appears_and_renders");
    let mut app = NtmApp::new();

    logger.step("Push a single toast via Msg::ToastShow");
    app.update(Msg::ToastShow {
        message: "File saved".to_string(),
        level: ToastLevel::Success,
    });
    {
        let q = app.toast_queue.borrow();
        assert!(!q.is_empty(), "Toast queue should not be empty after push");
        assert_eq!(q.active().unwrap().message, "File saved");
        assert_eq!(q.active().unwrap().level, ToastLevel::Success);
    }
    logger.step_result(true, "Toast appeared in queue");

    logger.step("Verify toast renders in view");
    let mut tf = TestFrame::new(100, 30);
    tf.render(|frame, _area| {
        app.view(frame);
    });
    tf.assert_contains("File saved");
    logger.step_result(true, "Toast text visible in render output");

    logger.finish(true);
}

/// Toast expiry: push toast with zero duration, tick, verify it's gone.
#[test]
fn test_toast_expiry_via_tick() {
    let logger = TestLogger::new("test_toast_expiry_via_tick");
    let mut app = NtmApp::new();

    logger.step("Push toast and set duration to 0 for immediate expiry");
    app.toast_queue.borrow_mut().duration_secs = 0;
    app.update(Msg::ToastShow {
        message: "Temporary notice".to_string(),
        level: ToastLevel::Info,
    });
    assert!(!app.toast_queue.borrow().is_empty(), "Toast should exist before tick");
    logger.step_result(true, "Toast exists before tick");

    logger.step("Send Msg::Tick to trigger expiry cleanup");
    app.update(Msg::Tick);
    assert!(app.toast_queue.borrow().is_empty(), "Toast should be expired after tick");
    logger.step_result(true, "Toast expired after tick");

    logger.step("Verify expired toast does not render");
    let mut tf = TestFrame::new(100, 30);
    tf.render(|frame, _area| {
        app.view(frame);
    });
    tf.assert_not_contains("Temporary notice");
    logger.step_result(true, "Expired toast not visible in render");

    logger.finish(true);
}

/// Multiple toasts: push 3 rapidly, verify most recent is active and all are in queue.
#[test]
fn test_toast_multiple_stacking() {
    let logger = TestLogger::new("test_toast_multiple_stacking");
    let mut app = NtmApp::new();

    logger.step("Push 3 toasts in rapid succession");
    app.update(Msg::ToastShow {
        message: "Toast A".to_string(),
        level: ToastLevel::Info,
    });
    app.update(Msg::ToastShow {
        message: "Toast B".to_string(),
        level: ToastLevel::Success,
    });
    app.update(Msg::ToastShow {
        message: "Toast C".to_string(),
        level: ToastLevel::Error,
    });

    {
        let q = app.toast_queue.borrow();
        assert_eq!(q.toasts.len(), 3, "All 3 toasts should be in queue");
        assert_eq!(q.active().unwrap().message, "Toast C", "Most recent toast should be active");
        assert_eq!(q.active().unwrap().level, ToastLevel::Error);
    }
    logger.step_result(true, "3 toasts stacked, most recent is active");

    logger.step("Verify only active (most recent) toast renders");
    let mut tf = TestFrame::new(100, 30);
    tf.render(|frame, _area| {
        app.view(frame);
    });
    // The active toast (last pushed) should render
    tf.assert_contains("Toast C");
    logger.step_result(true, "Active toast visible in render");

    logger.step("Expire all and verify queue empty");
    app.toast_queue.borrow_mut().duration_secs = 0;
    app.update(Msg::Tick);
    assert!(app.toast_queue.borrow().is_empty());
    logger.step_result(true, "All toasts expired after tick with zero duration");

    logger.finish(true);
}

/// Different toast levels: verify each level renders and preserves level in queue.
#[test]
fn test_toast_all_levels_render() {
    let logger = TestLogger::new("test_toast_all_levels_render");

    let levels = [
        (ToastLevel::Info, "Info alert"),
        (ToastLevel::Success, "Success alert"),
        (ToastLevel::Error, "Error alert"),
    ];

    for (level, msg) in &levels {
        logger.step(&format!("Test {:?} toast renders", level));
        let mut app = NtmApp::new();
        app.update(Msg::ToastShow {
            message: msg.to_string(),
            level: *level,
        });

        // Verify level is preserved in queue
        {
            let q = app.toast_queue.borrow();
            assert_eq!(q.active().unwrap().level, *level);
            assert_eq!(q.active().unwrap().message, *msg);
        }

        // Verify it renders
        let mut tf = TestFrame::new(100, 30);
        tf.render(|frame, _area| {
            app.view(frame);
        });
        tf.assert_contains(msg);
        logger.step_result(true, &format!("{:?} toast rendered and level preserved", level));
    }

    logger.finish(true);
}

/// Toast during confirm modal: both modal and toast should be visible simultaneously.
#[test]
fn test_toast_visible_during_confirm_modal() {
    let logger = TestLogger::new("test_toast_visible_during_confirm_modal");
    let mut app = populated_app();

    logger.step("Open a KillSession confirm modal");
    app.pending_confirm = Some(ConfirmAction::KillSession {
        session_id: "s1".to_string(),
        session_name: "project-a".to_string(),
    });

    logger.step("Push a toast while modal is open");
    app.update(Msg::ToastShow {
        message: "Background task done".to_string(),
        level: ToastLevel::Success,
    });

    logger.step("Verify both modal and toast render");
    let mut tf = TestFrame::new(100, 30);
    tf.render(|frame, _area| {
        app.view(frame);
    });

    // Modal content should be visible (kill confirmation mentions session name)
    tf.assert_contains("project-a");
    // Toast should also be visible (rendered after modal in view())
    tf.assert_contains("Background task done");
    logger.step_result(true, "Both modal and toast visible simultaneously");

    logger.step("Confirm the kill, verify toast shows kill feedback too");
    app.update(key_msg(KeyCode::Char('y')));
    assert!(app.pending_confirm.is_none(), "Modal should be dismissed after confirm");

    // Now we should have 2 toasts: the background one + the kill confirmation toast
    let q = app.toast_queue.borrow();
    assert!(q.toasts.len() >= 2, "Should have at least 2 toasts (background + kill confirm)");
    logger.step_result(true, "Kill confirmation added another toast");

    logger.finish(true);
}

/// Toast triggered by RpcError message — error toast appears and renders with error styling.
#[test]
fn test_toast_from_rpc_error() {
    let logger = TestLogger::new("test_toast_from_rpc_error");
    let mut app = NtmApp::new();

    logger.step("Send Msg::RpcError to trigger error toast");
    app.update(Msg::RpcError("Connection refused".to_string()));

    {
        let q = app.toast_queue.borrow();
        assert!(!q.is_empty(), "RpcError should create a toast");
        assert_eq!(q.active().unwrap().level, ToastLevel::Error);
        assert!(
            q.active().unwrap().message.contains("Connection refused"),
            "Toast should contain error message"
        );
    }
    logger.step_result(true, "RpcError created error toast");

    logger.step("Verify error toast renders");
    let mut tf = TestFrame::new(100, 30);
    tf.render(|frame, _area| {
        app.view(frame);
    });
    tf.assert_contains("Connection refused");
    logger.step_result(true, "Error toast visible in render output");

    logger.finish(true);
}

/// Toast lifecycle through kill flow: press K, confirm with y, verify toast appears.
#[test]
fn test_toast_from_kill_session_flow() {
    let logger = TestLogger::new("test_toast_from_kill_session_flow");
    let mut app = populated_app();

    logger.step("Select a session and press K to open kill modal");
    // Session is already selected at index 0 via populated_app's build_row_map
    app.update(key_msg(KeyCode::Char('K')));
    assert!(app.pending_confirm.is_some(), "Kill modal should be open");
    logger.step_result(true, "Kill modal opened");

    logger.step("Press 'y' to confirm kill");
    app.update(key_msg(KeyCode::Char('y')));
    assert!(app.pending_confirm.is_none(), "Modal should close after confirm");
    logger.step_result(true, "Modal closed");

    logger.step("Verify kill toast appeared");
    let q = app.toast_queue.borrow();
    assert!(!q.is_empty(), "Kill should produce a toast");
    let toast = q.active().unwrap();
    assert_eq!(toast.level, ToastLevel::Info);
    logger.step_result(true, "Kill toast exists with Info level");

    logger.finish(true);
}

/// Tick does not crash on empty queue, and spinner_frame increments.
#[test]
fn test_tick_increments_spinner_and_handles_empty_toast_queue() {
    let logger = TestLogger::new("test_tick_increments_spinner_and_handles_empty_toast_queue");
    let mut app = NtmApp::new();

    logger.step("Verify initial spinner_frame is 0");
    assert_eq!(app.spinner_frame, 0);
    logger.step_result(true, "Initial spinner is 0");

    logger.step("Send multiple ticks");
    for i in 1..=5 {
        app.update(Msg::Tick);
        assert_eq!(app.spinner_frame, i, "spinner_frame should be {i}");
    }
    logger.step_result(true, "Spinner incremented correctly over 5 ticks");

    logger.step("Toast queue remains empty through all ticks");
    assert!(app.toast_queue.borrow().is_empty());
    logger.step_result(true, "Empty toast queue is fine through ticks");

    logger.finish(true);
}
