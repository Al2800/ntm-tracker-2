#![no_main]

use libfuzzer_sys::fuzz_target;
use ntm_tracker_daemon::parsers::tmux_panes::parse_tmux_panes;

fuzz_target!(|data: &str| {
    // Fuzz the tmux list-panes parser with arbitrary string input.
    // The parser should not panic on any input.
    let _ = parse_tmux_panes(data);
});
