#![no_main]

use libfuzzer_sys::fuzz_target;
use ntm_tracker_daemon::parsers::ntm_tail::parse_ntm_tail;

fuzz_target!(|data: &str| {
    // Fuzz the NTM tail JSON parser with arbitrary string input.
    // The parser should not panic on any input.
    let _ = parse_ntm_tail(data);
});
