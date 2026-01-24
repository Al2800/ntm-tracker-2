use serde_json::Value;

#[derive(Clone, Debug, PartialEq, Eq)]
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
    }

    #[test]
    fn parses_object_tail() {
        let input = "{\"session\":\"alpha\",\"pane\":\"0\",\"lines\":[\"hello\"]}";
        let tail = parse_ntm_tail(input).expect("parse");
        assert_eq!(tail.session.as_deref(), Some("alpha"));
        assert_eq!(tail.pane.as_deref(), Some("0"));
        assert_eq!(tail.lines.len(), 1);
    }
}
