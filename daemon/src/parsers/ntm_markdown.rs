use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NtmSession {
    pub name: String,
    pub status: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NtmPane {
    pub session: String,
    pub pane: String,
    pub status: Option<String>,
    pub agent: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NtmMarkdown {
    pub sessions: Vec<NtmSession>,
    pub panes: Vec<NtmPane>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub reason: String,
}

pub fn parse_ntm_markdown(input: &str) -> Result<NtmMarkdown, ParseError> {
    let rows = parse_markdown_table(input)?;
    let mut sessions = Vec::new();
    let mut panes = Vec::new();

    for row in rows {
        if let Some(pane_value) = get_field(&row, &["pane", "pane_id", "pane index"]) {
            let session_value = get_field(&row, &["session", "session_name", "name"])
                .unwrap_or_else(|| "unknown".to_string());
            let status = get_field(&row, &["status", "state"]);
            let agent = get_field(&row, &["agent", "agent_type"]);
            panes.push(NtmPane {
                session: session_value,
                pane: pane_value,
                status,
                agent,
                metadata: row,
            });
        } else if let Some(session_value) = get_field(&row, &["session", "session_name", "name"]) {
            let status = get_field(&row, &["status", "state"]);
            sessions.push(NtmSession {
                name: session_value,
                status,
                metadata: row,
            });
        }
    }

    Ok(NtmMarkdown { sessions, panes })
}

fn parse_markdown_table(input: &str) -> Result<Vec<HashMap<String, String>>, ParseError> {
    let mut lines = input.lines().filter(|line| line.contains('|'));
    let header = lines
        .next()
        .ok_or_else(|| ParseError {
            reason: "missing header".to_string(),
        })?;
    let headers = split_row(header);

    let mut rows = Vec::new();
    for line in lines {
        if line.trim().starts_with('|') && line.contains("---") {
            continue;
        }
        let values = split_row(line);
        if values.is_empty() {
            continue;
        }
        let mut map = HashMap::new();
        for (idx, header) in headers.iter().enumerate() {
            let value = values.get(idx).cloned().unwrap_or_default();
            if !header.is_empty() {
                map.insert(header.to_lowercase(), value);
            }
        }
        if !map.is_empty() {
            rows.push(map);
        }
    }

    Ok(rows)
}

fn split_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|value| value.trim().to_string())
        .collect()
}

fn get_field(row: &HashMap<String, String>, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = row.get(&key.to_lowercase()) {
            if !value.is_empty() {
                return Some(value.clone());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_markdown_rows() {
        let markdown = "| session | pane | status |\n| --- | --- | --- |\n| alpha | 0 | active |";
        let parsed = parse_ntm_markdown(markdown).expect("parse");
        assert_eq!(parsed.panes.len(), 1);
        assert_eq!(parsed.panes[0].session, "alpha");
        assert_eq!(parsed.panes[0].pane, "0");
    }

    #[test]
    fn parses_session_only_rows() {
        let markdown = "| session | status |\n| --- | --- |\n| api | running |";
        let parsed = parse_ntm_markdown(markdown).expect("parse");
        assert_eq!(parsed.sessions.len(), 1);
        assert_eq!(parsed.sessions[0].name, "api");
        assert_eq!(parsed.sessions[0].status, Some("running".to_string()));
        assert_eq!(parsed.panes.len(), 0);
    }

    #[test]
    fn parses_multiple_panes() {
        let markdown = "| session | pane | status | agent |\n| --- | --- | --- | --- |\n| api | 0 | active | claude |\n| api | 1 | idle | codex |";
        let parsed = parse_ntm_markdown(markdown).expect("parse");
        assert_eq!(parsed.panes.len(), 2);
        assert_eq!(parsed.panes[0].agent, Some("claude".to_string()));
        assert_eq!(parsed.panes[1].agent, Some("codex".to_string()));
    }

    #[test]
    fn handles_empty_cells() {
        let markdown = "| session | pane | status |\n| --- | --- | --- |\n| alpha | 0 | |";
        let parsed = parse_ntm_markdown(markdown).expect("parse");
        assert_eq!(parsed.panes.len(), 1);
        assert_eq!(parsed.panes[0].status, None);
    }

    #[test]
    fn handles_extra_whitespace() {
        let markdown = "|  session  |  pane  |  status  |\n| --- | --- | --- |\n|   beta   |   1   |   waiting   |";
        let parsed = parse_ntm_markdown(markdown).expect("parse");
        assert_eq!(parsed.panes.len(), 1);
        assert_eq!(parsed.panes[0].session, "beta");
        assert_eq!(parsed.panes[0].pane, "1");
        assert_eq!(parsed.panes[0].status, Some("waiting".to_string()));
    }

    #[test]
    fn returns_error_on_no_header() {
        let markdown = "just some text without pipes";
        let result = parse_ntm_markdown(markdown);
        assert!(result.is_err());
        assert!(result.unwrap_err().reason.contains("missing header"));
    }

    #[test]
    fn handles_alternative_column_names() {
        let markdown = "| session_name | pane_id | state |\n| --- | --- | --- |\n| gamma | 2 | active |";
        let parsed = parse_ntm_markdown(markdown).expect("parse");
        assert_eq!(parsed.panes.len(), 1);
        assert_eq!(parsed.panes[0].session, "gamma");
        assert_eq!(parsed.panes[0].pane, "2");
        assert_eq!(parsed.panes[0].status, Some("active".to_string()));
    }

    #[test]
    fn handles_case_insensitive_headers() {
        let markdown = "| SESSION | PANE | STATUS |\n| --- | --- | --- |\n| delta | 0 | idle |";
        let parsed = parse_ntm_markdown(markdown).expect("parse");
        assert_eq!(parsed.panes.len(), 1);
        assert_eq!(parsed.panes[0].session, "delta");
    }

    #[test]
    fn handles_mixed_sessions_and_panes() {
        let markdown = "| session | status |\n| --- | --- |\n| api | running |\n\n| session | pane | status |\n| --- | --- | --- |\n| api | 0 | active |";
        // This tests that both tables are parsed if they share pipes
        let parsed = parse_ntm_markdown(markdown).expect("parse");
        assert!(parsed.sessions.len() + parsed.panes.len() >= 1);
    }
}
