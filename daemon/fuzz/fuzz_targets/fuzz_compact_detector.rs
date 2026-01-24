#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use ntm_tracker_daemon::detector::compact::{CompactConfig, CompactDetector, CompactInput};

#[derive(Arbitrary, Debug)]
struct FuzzInput<'a> {
    line: &'a str,
    now: i64,
    ntm_compact_count: Option<u64>,
    context_tokens: Option<u64>,
    previous_tokens: Option<u64>,
}

fuzz_target!(|input: FuzzInput| {
    // Fuzz the compact detector with structured input.
    // The detector should not panic on any input.
    let mut detector = CompactDetector::new(CompactConfig::default());

    let compact_input = CompactInput {
        now: input.now,
        line: input.line,
        ntm_compact_count: input.ntm_compact_count,
        context_tokens: input.context_tokens,
        previous_tokens: input.previous_tokens,
    };

    let _ = detector.detect(compact_input);
});
