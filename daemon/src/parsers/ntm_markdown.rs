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
}
