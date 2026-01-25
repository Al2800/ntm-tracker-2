use ntm_tracker_daemon::detector::compact::{CompactConfig, CompactDetector, CompactInput};
use ntm_tracker_daemon::detector::escalation::{
    EscalationConfig, EscalationDetector, EscalationInput,
};
use ntm_tracker_daemon::detector::status::{detect_status, StatusConfig, StatusInput};
use ntm_tracker_daemon::models::pane::PaneStatus;
use ntm_tracker_daemon::parsers::ntm_markdown::parse_ntm_markdown;
use ntm_tracker_daemon::parsers::ntm_tail::parse_ntm_tail;
use ntm_tracker_daemon::parsers::tmux_panes::parse_tmux_panes;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct FixturePane {
    lines: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct FixtureFile {
    panes: HashMap<String, FixturePane>,
}

#[derive(Debug, Deserialize)]
struct ExpectedCompact {
    line_contains: String,
    reason: String,
    confidence: f32,
}

#[derive(Debug, Deserialize)]
struct ExpectedEscalation {
    line_contains: String,
    severity: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedStatusSample {
    line_contains: String,
    expected_status: String,
    reason: String,
}

#[derive(Debug, Deserialize)]
struct ExpectedPane {
    compacts: Vec<ExpectedCompact>,
    escalations: Vec<ExpectedEscalation>,
    status_samples: Vec<ExpectedStatusSample>,
}

#[derive(Debug, Deserialize)]
struct ExpectedFixture {
    fixture: String,
    #[serde(default)]
    r#type: Option<String>,
    panes: Option<HashMap<String, ExpectedPane>>,
}

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("fixtures")
}

fn read_fixture(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap_or_else(|err| panic!("read {:?}: {err}", path))
}

fn find_line<'a>(lines: &'a [String], needle: &str) -> &'a str {
    lines
        .iter()
        .find(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("missing line containing '{needle}'"))
        .as_str()
}

fn parse_status(value: &str) -> PaneStatus {
    match value {
        "active" => PaneStatus::Active,
        "waiting" => PaneStatus::Waiting,
        "idle" => PaneStatus::Idle,
        "ended" => PaneStatus::Ended,
        other => panic!("unknown status '{other}'"),
    }
}

#[test]
fn snapshot_robot_markdown() {
    let root = fixtures_root();
    let markdown = read_fixture(&root.join("ntm/robot-markdown.md"));
    let parsed = parse_ntm_markdown(&markdown).expect("parse markdown fixture");
    let mut settings = insta::Settings::new();
    settings.set_sort_maps(true);
    settings.bind(|| {
        insta::assert_json_snapshot!(parsed);
    });
}

#[test]
fn snapshot_robot_markdown_compact() {
    let root = fixtures_root();
    let markdown = read_fixture(&root.join("ntm/robot-markdown-compact.md"));
    let err = parse_ntm_markdown(&markdown).expect_err("expected parse error");
    insta::assert_debug_snapshot!(err);
}

#[test]
fn snapshot_robot_tail() {
    let root = fixtures_root();
    let tail = read_fixture(&root.join("ntm/robot-tail.json"));
    let parsed = parse_ntm_tail(&tail).expect("parse tail fixture");
    let mut settings = insta::Settings::new();
    settings.set_sort_maps(true);
    settings.bind(|| {
        insta::assert_json_snapshot!(parsed);
    });
}

#[test]
fn snapshot_tmux_list_panes_parser() {
    let parsed = parse_tmux_panes("$1:@2:%3:0:111:fish:1700000000:0:1\n").expect("parse");
    let mut settings = insta::Settings::new();
    settings.set_sort_maps(true);
    settings.bind(|| {
        insta::assert_json_snapshot!(parsed);
    });
}

#[test]
fn golden_detector_fixtures_match_expectations() {
    let root = fixtures_root();
    let expected_dir = root.join("expected");
    let mut entries: Vec<_> = fs::read_dir(&expected_dir)
        .expect("read expected dir")
        .filter_map(Result::ok)
        .collect();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.ends_with(".expected.json"))
            .unwrap_or(false)
        {
            continue;
        }

        let expected: ExpectedFixture =
            serde_json::from_str(&read_fixture(&path)).expect("parse expected");
        if expected.r#type.as_deref() == Some("status_snapshot") {
            continue;
        }
        let panes = match expected.panes {
            Some(panes) => panes,
            None => continue,
        };

        let fixture_path = root.join("sessions").join(&expected.fixture);
        let fixture: FixtureFile =
            serde_json::from_str(&read_fixture(&fixture_path)).expect("parse fixture");

        for (pane_id, expected_pane) in panes {
            let pane = fixture
                .panes
                .get(&pane_id)
                .unwrap_or_else(|| panic!("missing pane {pane_id} in {}", expected.fixture));

            let mut compact_detector = CompactDetector::new(CompactConfig {
                debounce_secs: 0,
                ..CompactConfig::default()
            });

            for (idx, compact) in expected_pane.compacts.iter().enumerate() {
                let line = find_line(&pane.lines, &compact.line_contains);
                let detection = compact_detector
                    .detect(CompactInput {
                        now: 1_700_000_000 + idx as i64,
                        line,
                        ntm_compact_count: None,
                        context_tokens: None,
                        previous_tokens: None,
                    })
                    .unwrap_or_else(|| panic!("no compact detection for '{}'", compact.line_contains));
                assert_eq!(detection.reason, compact.reason);
                assert!(
                    (detection.confidence - compact.confidence).abs() < 0.01,
                    "confidence mismatch for '{}'",
                    compact.line_contains
                );
            }

            let mut escalation_detector = EscalationDetector::new(EscalationConfig {
                debounce_secs: 0,
                activity_window_secs: 300,
            });

            for (idx, escalation) in expected_pane.escalations.iter().enumerate() {
                let line = find_line(&pane.lines, &escalation.line_contains);
                let event = escalation_detector
                    .detect(EscalationInput {
                        now: 1_700_000_100 + idx as i64,
                        pane_uid: &format!("pane-{pane_id}"),
                        line,
                        pane_last_activity: Some(1_700_000_099 + idx as i64),
                        waiting_hint: true,
                    })
                    .unwrap_or_else(|| {
                        panic!("no escalation detection for '{}'", escalation.line_contains)
                    });
                assert_eq!(event.severity, escalation.severity);
            }

            for sample in expected_pane.status_samples.iter() {
                let line = find_line(&pane.lines, &sample.line_contains);
                let expected_status = parse_status(&sample.expected_status);
                let (pane_last_activity, pane_dead) = match expected_status {
                    PaneStatus::Idle => (Some(1_700_000_000 - 10_000), false),
                    PaneStatus::Ended => (None, true),
                    _ => (Some(1_700_000_000), false),
                };
                let result = detect_status(
                    StatusInput {
                        now: 1_700_000_000,
                        pane_last_activity,
                        pane_dead,
                        pane_current_command: Some("bash"),
                        output_line: Some(line),
                    },
                    StatusConfig::default(),
                );
                assert_eq!(result.status, expected_status);
                assert_eq!(result.reason, sample.reason);
            }
        }
    }
}

#[test]
fn parses_robot_markdown_fixture() {
    let root = fixtures_root();
    let markdown = read_fixture(&root.join("ntm/robot-markdown.md"));
    let parsed = parse_ntm_markdown(&markdown).expect("parse markdown fixture");
    let names: Vec<&str> = parsed.sessions.iter().map(|session| session.name.as_str()).collect();
    assert_eq!(parsed.sessions.len(), 3);
    assert!(names.contains(&"cuas-sim"));
    assert!(names.contains(&"ntracker3"));
    assert!(names.contains(&"speedread-ios"));
}
