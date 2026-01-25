use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NtmTail {
    pub session: Option<String>,
    pub pane: Option<String>,
    pub lines: Vec<String>,
    pub raw: Value,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub reason: String,
}

pub fn parse_ntm_tail(input: &str) -> Result<NtmTail, ParseError> {
    let value: Value = serde_json::from_str(input).map_err(|err| ParseError {
        reason: format!("invalid json: {err}"),
    })?;

    let mut lines = Vec::new();
    let mut session = None;
    let mut pane = None;

    match &value {
        Value::Array(items) => {
            for item in items {
                if let Some(line) = item.as_str() {
                    lines.push(line.to_string());
                }
            }
        }
        Value::Object(map) => {
            if let Some(Value::String(s)) = map.get("session") {
                session = Some(s.clone());
            }
            if let Some(Value::String(s)) = map.get("pane") {
                pane = Some(s.clone());
            }
            if let Some(Value::Array(items)) = map.get("lines") {
                for item in items {
                    if let Some(line) = item.as_str() {
                        lines.push(line.to_string());
                    }
                }
            }
        }
        _ => {}
    }

    Ok(NtmTail {
        session,
        pane,
        lines,
        raw: value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_array_tail() {
        let input = "[\"line1\",\"line2\"]";
        let tail = parse_ntm_tail(input).expect("parse");
        assert_eq!(tail.lines.len(), 2);
        assert_eq!(tail.lines[0], "line1");
        assert_eq!(tail.lines[1], "line2");
    }

    #[test]
    fn parses_object_tail() {
        let input = "{\"session\":\"alpha\",\"pane\":\"0\",\"lines\":[\"hello\"]}";
        let tail = parse_ntm_tail(input).expect("parse");
        assert_eq!(tail.session.as_deref(), Some("alpha"));
        assert_eq!(tail.pane.as_deref(), Some("0"));
        assert_eq!(tail.lines.len(), 1);
        assert_eq!(tail.lines[0], "hello");
    }

    #[test]
    fn parses_empty_array() {
        let input = "[]";
        let tail = parse_ntm_tail(input).expect("parse");
        assert_eq!(tail.lines.len(), 0);
        assert_eq!(tail.session, None);
        assert_eq!(tail.pane, None);
    }

    #[test]
    fn parses_empty_object() {
        let input = "{}";
        let tail = parse_ntm_tail(input).expect("parse");
        assert_eq!(tail.lines.len(), 0);
        assert_eq!(tail.session, None);
        assert_eq!(tail.pane, None);
    }

    #[test]
    fn parses_object_without_lines() {
        let input = "{\"session\":\"beta\",\"pane\":\"1\"}";
        let tail = parse_ntm_tail(input).expect("parse");
        assert_eq!(tail.session.as_deref(), Some("beta"));
        assert_eq!(tail.pane.as_deref(), Some("1"));
        assert_eq!(tail.lines.len(), 0);
    }

    #[test]
    fn parses_object_with_session_only() {
        let input = "{\"session\":\"gamma\"}";
        let tail = parse_ntm_tail(input).expect("parse");
        assert_eq!(tail.session.as_deref(), Some("gamma"));
        assert_eq!(tail.pane, None);
    }

    #[test]
    fn parses_object_with_pane_only() {
        let input = "{\"pane\":\"2\"}";
        let tail = parse_ntm_tail(input).expect("parse");
        assert_eq!(tail.session, None);
        assert_eq!(tail.pane.as_deref(), Some("2"));
    }

    #[test]
    fn handles_non_string_array_items() {
        let input = "[\"line1\", 123, \"line2\", null]";
        let tail = parse_ntm_tail(input).expect("parse");
        assert_eq!(tail.lines.len(), 2);
        assert_eq!(tail.lines[0], "line1");
        assert_eq!(tail.lines[1], "line2");
    }

    #[test]
    fn handles_non_string_lines_in_object() {
        let input = "{\"lines\":[\"a\", 42, \"b\", true]}";
        let tail = parse_ntm_tail(input).expect("parse");
        assert_eq!(tail.lines.len(), 2);
        assert_eq!(tail.lines[0], "a");
        assert_eq!(tail.lines[1], "b");
    }

    #[test]
    fn handles_multiline_content() {
        let input = "[\"line with\\nnewline\"]";
        let tail = parse_ntm_tail(input).expect("parse");
        assert_eq!(tail.lines.len(), 1);
        assert!(tail.lines[0].contains('\n'));
    }

    #[test]
    fn preserves_raw_value() {
        let input = "{\"session\":\"alpha\",\"extra\":123}";
        let tail = parse_ntm_tail(input).expect("parse");
        assert!(tail.raw.is_object());
        assert_eq!(tail.raw.get("extra"), Some(&Value::Number(123.into())));
    }

    #[test]
    fn fails_on_invalid_json() {
        let input = "not json at all";
        let result = parse_ntm_tail(input);
        assert!(result.is_err());
        assert!(result.unwrap_err().reason.contains("invalid json"));
    }

    #[test]
    fn fails_on_truncated_json() {
        let input = "{\"session\": \"alpha\"";
        let result = parse_ntm_tail(input);
        assert!(result.is_err());
    }

    #[test]
    fn handles_primitive_json_values() {
        // Primitive values (string, number, etc.) should parse but produce no lines
        let input = "\"just a string\"";
        let tail = parse_ntm_tail(input).expect("parse");
        assert_eq!(tail.lines.len(), 0);
        assert_eq!(tail.session, None);
    }

    #[test]
    fn handles_null_value() {
        let input = "null";
        let tail = parse_ntm_tail(input).expect("parse");
        assert_eq!(tail.lines.len(), 0);
        assert_eq!(tail.session, None);
        assert_eq!(tail.pane, None);
    }
}
