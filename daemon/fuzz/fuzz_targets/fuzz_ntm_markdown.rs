#![no_main]

use libfuzzer_sys::fuzz_target;
use ntm_tracker_daemon::parsers::ntm_markdown::parse_ntm_markdown;

fuzz_target!(|data: &str| {
    // Fuzz the NTM markdown parser with arbitrary string input.
    // The parser should not panic on any input.
    let _ = parse_ntm_markdown(data);
});
