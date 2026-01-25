use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TmuxPaneMeta {
    pub session_id: String,
    pub window_id: String,
    pub pane_id: String,
    pub pane_index: i32,
    pub pane_pid: i64,
    pub pane_current_command: String,
    pub pane_last_activity: i64,
    pub pane_dead: bool,
    pub pane_in_mode: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub line: String,
    pub reason: String,
}

pub fn parse_tmux_panes(output: &str) -> Result<Vec<TmuxPaneMeta>, ParseError> {
    let mut metas = Vec::new();
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let meta = parse_line(line)?;
        metas.push(meta);
    }
    Ok(metas)
}

fn parse_line(line: &str) -> Result<TmuxPaneMeta, ParseError> {
    let parts: Vec<&str> = line.splitn(9, ':').collect();
    if parts.len() != 9 {
        return Err(ParseError {
            line: line.to_string(),
            reason: "expected 9 fields".to_string(),
        });
    }

    let pane_index = parts[3].parse::<i32>().map_err(|_| ParseError {
        line: line.to_string(),
        reason: "invalid pane_index".to_string(),
    })?;

    let pane_pid = parts[4].parse::<i64>().map_err(|_| ParseError {
        line: line.to_string(),
        reason: "invalid pane_pid".to_string(),
    })?;

    let pane_last_activity = parts[6].parse::<i64>().map_err(|_| ParseError {
        line: line.to_string(),
        reason: "invalid pane_last_activity".to_string(),
    })?;

    let pane_dead = parse_bool(parts[7], line, "pane_dead")?;
    let pane_in_mode = parse_bool(parts[8], line, "pane_in_mode")?;

    Ok(TmuxPaneMeta {
        session_id: parts[0].to_string(),
        window_id: parts[1].to_string(),
        pane_id: parts[2].to_string(),
        pane_index,
        pane_pid,
        pane_current_command: parts[5].to_string(),
        pane_last_activity,
        pane_dead,
        pane_in_mode,
    })
}

fn parse_bool(raw: &str, line: &str, field: &str) -> Result<bool, ParseError> {
    match raw {
        "1" => Ok(true),
        "0" => Ok(false),
        _ => Err(ParseError {
            line: line.to_string(),
            reason: format!("invalid {field}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tmux_line() {
        let line = "$1:@2:%3:0:111:fish:1700000000:0:1";
        let meta = parse_tmux_panes(line).expect("parse").remove(0);
        assert_eq!(meta.session_id, "$1");
        assert_eq!(meta.window_id, "@2");
        assert_eq!(meta.pane_id, "%3");
        assert_eq!(meta.pane_index, 0);
        assert_eq!(meta.pane_pid, 111);
        assert_eq!(meta.pane_current_command, "fish");
        assert!(!meta.pane_dead);
        assert!(meta.pane_in_mode);
    }

    #[test]
    fn parses_multiple_lines() {
        let input = "$1:@2:%3:0:111:fish:1700000000:0:0\n$1:@2:%4:1:222:vim:1700000001:0:0";
        let metas = parse_tmux_panes(input).expect("parse");
        assert_eq!(metas.len(), 2);
        assert_eq!(metas[0].pane_id, "%3");
        assert_eq!(metas[1].pane_id, "%4");
        assert_eq!(metas[1].pane_pid, 222);
    }

    #[test]
    fn handles_empty_input() {
        let metas = parse_tmux_panes("").expect("parse");
        assert_eq!(metas.len(), 0);
    }

    #[test]
    fn handles_whitespace_only() {
        let metas = parse_tmux_panes("   \n  \n   ").expect("parse");
        assert_eq!(metas.len(), 0);
    }

    #[test]
    fn fails_on_insufficient_fields() {
        let line = "$1:@2:%3:0:111:fish";
        let result = parse_tmux_panes(line);
        assert!(result.is_err());
        assert!(result.unwrap_err().reason.contains("expected 9 fields"));
    }

    #[test]
    fn fails_on_invalid_pane_index() {
        let line = "$1:@2:%3:abc:111:fish:1700000000:0:0";
        let result = parse_tmux_panes(line);
        assert!(result.is_err());
        assert!(result.unwrap_err().reason.contains("invalid pane_index"));
    }

    #[test]
    fn fails_on_invalid_pid() {
        let line = "$1:@2:%3:0:notapid:fish:1700000000:0:0";
        let result = parse_tmux_panes(line);
        assert!(result.is_err());
        assert!(result.unwrap_err().reason.contains("invalid pane_pid"));
    }

    #[test]
    fn fails_on_invalid_timestamp() {
        let line = "$1:@2:%3:0:111:fish:notanumber:0:0";
        let result = parse_tmux_panes(line);
        assert!(result.is_err());
        assert!(result.unwrap_err().reason.contains("invalid pane_last_activity"));
    }

    #[test]
    fn fails_on_invalid_bool() {
        let line = "$1:@2:%3:0:111:fish:1700000000:2:0";
        let result = parse_tmux_panes(line);
        assert!(result.is_err());
        assert!(result.unwrap_err().reason.contains("invalid pane_dead"));
    }

    #[test]
    fn handles_dead_pane() {
        let line = "$1:@2:%3:0:111:fish:1700000000:1:0";
        let meta = parse_tmux_panes(line).expect("parse").remove(0);
        assert!(meta.pane_dead);
        assert!(!meta.pane_in_mode);
    }

    #[test]
    fn handles_large_pane_index() {
        let line = "$1:@2:%3:99:111:bash:1700000000:0:0";
        let meta = parse_tmux_panes(line).expect("parse").remove(0);
        assert_eq!(meta.pane_index, 99);
    }

    #[test]
    fn handles_command_with_spaces() {
        // Note: command field is limited by splitn(9) so spaces after 9th field are included
        let line = "$1:@2:%3:0:111:some command:1700000000:0:0";
        let meta = parse_tmux_panes(line).expect("parse").remove(0);
        assert_eq!(meta.pane_current_command, "some command");
    }
}
