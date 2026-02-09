use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// A recorded step with timing and result information.
#[derive(Clone)]
struct StepRecord {
    number: usize,
    description: String,
    context_path: Vec<String>,
    started_at_ms: f64,
    ended_at_ms: Option<f64>,
    result: Option<bool>,
    detail: Option<String>,
}

/// Structured test logger with step tracking, JSON output, failure summaries,
/// step timing, assertion helpers, and context nesting.
///
/// # Features
/// - **JSON output mode**: Set `TEST_LOG_JSON=1` env var to emit JSON lines
/// - **Failure summary**: `finish()` and `Drop` print all FAIL steps
/// - **Step timing**: Each step records start/end time in milliseconds
/// - **Assertion helpers**: `log_assert()` logs expected vs actual then asserts
/// - **Context nesting**: `enter_context()` / `exit_context()` for nested steps
pub struct TestLogger {
    test_name: String,
    start: Instant,
    step: Mutex<usize>,
    lines: Arc<Mutex<Vec<String>>>,
    steps: Mutex<Vec<StepRecord>>,
    context_stack: Mutex<Vec<String>>,
    json_mode: bool,
    finished: Mutex<bool>,
    failure_summary_printed: Mutex<bool>,
}

impl TestLogger {
    pub fn new(test_name: &str) -> Self {
        let json_mode = std::env::var("TEST_LOG_JSON")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        Self::new_inner(test_name, json_mode)
    }

    /// Create a logger with explicit JSON mode (for testing the logger itself).
    pub fn new_with_json(test_name: &str, json_mode: bool) -> Self {
        Self::new_inner(test_name, json_mode)
    }

    fn new_inner(test_name: &str, json_mode: bool) -> Self {
        let logger = Self {
            test_name: test_name.to_string(),
            start: Instant::now(),
            step: Mutex::new(0),
            lines: Arc::new(Mutex::new(Vec::new())),
            steps: Mutex::new(Vec::new()),
            context_stack: Mutex::new(Vec::new()),
            json_mode,
            finished: Mutex::new(false),
            failure_summary_printed: Mutex::new(false),
        };
        if json_mode {
            logger.emit_json("start", None, None, None);
        } else {
            logger.push_line(&format!("=== START: {} ===", test_name));
        }
        logger
    }

    /// Log a numbered step. Records start time for duration tracking.
    pub fn step(&self, description: &str) {
        let mut step_num = self.step.lock().unwrap();
        *step_num += 1;
        let current = *step_num;
        let elapsed = self.start.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

        let ctx = self.context_stack.lock().unwrap().clone();
        let prefix = self.context_prefix(&ctx);

        self.steps.lock().unwrap().push(StepRecord {
            number: current,
            description: description.to_string(),
            context_path: ctx.clone(),
            started_at_ms: elapsed_ms,
            ended_at_ms: None,
            result: None,
            detail: None,
        });

        if self.json_mode {
            self.emit_json(
                "step",
                Some(current),
                Some(description),
                Some(elapsed_ms),
            );
        } else {
            let line = format!(
                "[{:.3}s] [{}] STEP {}: {}{}",
                elapsed.as_secs_f64(),
                self.test_name,
                current,
                prefix,
                description
            );
            self.push_line(&line);
        }
    }

    /// Log a step result (PASS/FAIL) with duration since step start.
    pub fn step_result(&self, passed: bool, detail: &str) {
        let step_num = *self.step.lock().unwrap();
        let elapsed = self.start.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;
        let status = if passed { "PASS" } else { "FAIL" };

        // Update step record with result and end time
        let mut steps = self.steps.lock().unwrap();
        let duration_ms = if let Some(rec) = steps.iter_mut().rev().find(|r| r.number == step_num)
        {
            rec.ended_at_ms = Some(elapsed_ms);
            rec.result = Some(passed);
            rec.detail = Some(detail.to_string());
            elapsed_ms - rec.started_at_ms
        } else {
            0.0
        };
        drop(steps);

        if self.json_mode {
            self.emit_json_result(step_num, passed, detail, elapsed_ms, duration_ms);
        } else {
            let line = format!(
                "[{:.3}s] [{}] STEP {}: {} - {} ({:.1}ms)",
                elapsed.as_secs_f64(),
                self.test_name,
                step_num,
                status,
                detail,
                duration_ms,
            );
            self.push_line(&line);
        }
    }

    /// Log an arbitrary message.
    pub fn log(&self, msg: &str) {
        let elapsed = self.start.elapsed();
        if self.json_mode {
            self.emit_json_log(msg, elapsed.as_secs_f64() * 1000.0);
        } else {
            let line = format!(
                "[{:.3}s] [{}] {}",
                elapsed.as_secs_f64(),
                self.test_name,
                msg
            );
            self.push_line(&line);
        }
    }

    /// Print final result, total duration, and failure summary.
    pub fn finish(&self, passed: bool) {
        *self.finished.lock().unwrap() = true;
        let elapsed = self.start.elapsed();
        let status = if passed { "PASS" } else { "FAIL" };

        // Print failure summary before final result
        self.print_failure_summary();

        if self.json_mode {
            self.emit_json_finish(passed, elapsed.as_secs_f64() * 1000.0);
        } else {
            let line = format!(
                "[{:.3}s] [{}] RESULT: {} ({:.3}s)",
                elapsed.as_secs_f64(),
                self.test_name,
                status,
                elapsed.as_secs_f64()
            );
            self.push_line(&line);
        }
    }

    /// Get all logged lines (for assertions or output).
    pub fn lines(&self) -> Vec<String> {
        self.lines.lock().unwrap().clone()
    }

    /// Dump all lines to stderr (visible with `cargo test -- --nocapture`).
    pub fn dump(&self) {
        for line in self.lines.lock().unwrap().iter() {
            eprintln!("{line}");
        }
    }

    // --- New features ---

    /// Enter a nested context. Steps logged inside will be prefixed with
    /// the full context path (e.g., "Within RPC call > Validating response > ").
    pub fn enter_context(&self, name: &str) {
        self.context_stack
            .lock()
            .unwrap()
            .push(name.to_string());
        let elapsed = self.start.elapsed();
        if self.json_mode {
            let ctx_str = self.context_stack.lock().unwrap().join(" > ");
            let obj = serde_json::json!({
                "event": "enter_context",
                "test": &self.test_name,
                "context": ctx_str,
                "elapsed_ms": elapsed.as_secs_f64() * 1000.0
            });
            self.push_line(&obj.to_string());
        } else {
            let ctx = self.context_stack.lock().unwrap().clone();
            let prefix = self.context_prefix(&ctx);
            let line = format!(
                "[{:.3}s] [{}] → {}",
                elapsed.as_secs_f64(),
                self.test_name,
                prefix.trim_end_matches(" > ").trim_end_matches("> ")
            );
            self.push_line(&line);
        }
    }

    /// Exit the most recent context level.
    pub fn exit_context(&self) {
        let elapsed = self.start.elapsed();
        let name = self.context_stack.lock().unwrap().pop();
        if self.json_mode {
            if let Some(ref n) = name {
                let obj = serde_json::json!({
                    "event": "exit_context",
                    "test": &self.test_name,
                    "exited": n,
                    "elapsed_ms": elapsed.as_secs_f64() * 1000.0
                });
                self.push_line(&obj.to_string());
            }
        } else if let Some(ref n) = name {
            let line = format!(
                "[{:.3}s] [{}] ← {}",
                elapsed.as_secs_f64(),
                self.test_name,
                n
            );
            self.push_line(&line);
        }
    }

    /// Assert a condition, logging expected vs actual before the assertion.
    /// If the assertion fails, it is logged as FAIL, then panics.
    pub fn log_assert<T: fmt::Debug + PartialEq>(
        &self,
        expected: &T,
        actual: &T,
        context: &str,
    ) {
        let passed = expected == actual;
        let step_num = *self.step.lock().unwrap();
        let elapsed = self.start.elapsed();
        let elapsed_ms = elapsed.as_secs_f64() * 1000.0;

        let detail = if passed {
            format!("{context}: matched {:?}", expected)
        } else {
            format!(
                "{context}: expected {:?}, got {:?}",
                expected, actual
            )
        };

        // Record as a sub-result on the current step
        let mut steps = self.steps.lock().unwrap();
        if let Some(rec) = steps.iter_mut().rev().find(|r| r.number == step_num) {
            if !passed {
                rec.result = Some(false);
                rec.detail = Some(detail.clone());
                rec.ended_at_ms = Some(elapsed_ms);
            }
        }
        drop(steps);

        let status = if passed { "PASS" } else { "FAIL" };
        if self.json_mode {
            let obj = serde_json::json!({
                "event": "assert",
                "test": &self.test_name,
                "step": step_num,
                "status": status.to_lowercase(),
                "context": context,
                "expected": format!("{expected:?}"),
                "actual": format!("{actual:?}"),
                "elapsed_ms": elapsed_ms
            });
            self.push_line(&obj.to_string());
        } else {
            let line = format!(
                "[{:.3}s] [{}] ASSERT {}: {} - {}",
                elapsed.as_secs_f64(),
                self.test_name,
                step_num,
                status,
                detail,
            );
            self.push_line(&line);
        }

        if !passed {
            self.print_failure_summary();
            panic!(
                "Assertion failed in test '{}': {}",
                self.test_name, detail
            );
        }
    }

    /// Assert a boolean condition with a descriptive context message.
    pub fn log_assert_bool(&self, condition: bool, context: &str) {
        self.log_assert(&true, &condition, context);
    }

    /// Get all failed steps.
    pub fn failures(&self) -> Vec<String> {
        self.steps
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.result == Some(false))
            .map(|s| {
                let ctx_str = if s.context_path.is_empty() {
                    String::new()
                } else {
                    format!("[{}] ", s.context_path.join(" > "))
                };
                format!(
                    "STEP {}: {}{}{}",
                    s.number,
                    ctx_str,
                    s.description,
                    s.detail
                        .as_ref()
                        .map(|d| format!(" — {d}"))
                        .unwrap_or_default()
                )
            })
            .collect()
    }

    /// Returns true if JSON output mode is active.
    pub fn is_json_mode(&self) -> bool {
        self.json_mode
    }

    // --- Internal helpers ---

    fn context_prefix(&self, ctx: &[String]) -> String {
        if ctx.is_empty() {
            String::new()
        } else {
            format!("{} > ", ctx.join(" > "))
        }
    }

    fn print_failure_summary(&self) {
        if *self.failure_summary_printed.lock().unwrap() {
            return;
        }
        let failures = self.failures();
        if failures.is_empty() {
            return;
        }
        *self.failure_summary_printed.lock().unwrap() = true;
        if self.json_mode {
            let obj = serde_json::json!({
                "event": "failure_summary",
                "test": &self.test_name,
                "count": failures.len(),
                "failures": failures,
            });
            self.push_line(&obj.to_string());
        } else {
            self.push_line(&format!(
                "--- FAILURE SUMMARY: {} ({} failure{}) ---",
                self.test_name,
                failures.len(),
                if failures.len() == 1 { "" } else { "s" }
            ));
            for f in &failures {
                self.push_line(&format!("  ✗ {f}"));
            }
            self.push_line("--- END FAILURE SUMMARY ---");
        }
    }

    fn push_line(&self, line: &str) {
        eprintln!("{line}");
        self.lines.lock().unwrap().push(line.to_string());
    }

    fn emit_json(&self, event: &str, step: Option<usize>, desc: Option<&str>, elapsed_ms: Option<f64>) {
        let mut obj = serde_json::json!({
            "event": event,
            "test": &self.test_name,
        });
        if let Some(map) = obj.as_object_mut() {
            if let Some(s) = step {
                map.insert("step".to_string(), serde_json::json!(s));
            }
            if let Some(d) = desc {
                map.insert("description".to_string(), serde_json::json!(d));
            }
            if let Some(e) = elapsed_ms {
                map.insert("elapsed_ms".to_string(), serde_json::json!(e));
            }
        }
        self.push_line(&obj.to_string());
    }

    fn emit_json_result(
        &self,
        step: usize,
        passed: bool,
        detail: &str,
        elapsed_ms: f64,
        duration_ms: f64,
    ) {
        let status = if passed { "pass" } else { "fail" };
        let obj = serde_json::json!({
            "event": "step_result",
            "test": &self.test_name,
            "step": step,
            "status": status,
            "detail": detail,
            "elapsed_ms": elapsed_ms,
            "duration_ms": duration_ms,
        });
        self.push_line(&obj.to_string());
    }

    fn emit_json_log(&self, msg: &str, elapsed_ms: f64) {
        let obj = serde_json::json!({
            "event": "log",
            "test": &self.test_name,
            "message": msg,
            "elapsed_ms": elapsed_ms,
        });
        self.push_line(&obj.to_string());
    }

    fn emit_json_finish(&self, passed: bool, elapsed_ms: f64) {
        let status = if passed { "pass" } else { "fail" };
        let obj = serde_json::json!({
            "event": "finish",
            "test": &self.test_name,
            "status": status,
            "elapsed_ms": elapsed_ms,
        });
        self.push_line(&obj.to_string());
    }
}

impl Drop for TestLogger {
    fn drop(&mut self) {
        if !*self.finished.lock().unwrap() && !*self.failure_summary_printed.lock().unwrap() {
            self.print_failure_summary();
        }

        let elapsed = self.start.elapsed();
        if self.json_mode {
            let obj = serde_json::json!({
                "event": "end",
                "test": &self.test_name,
                "elapsed_ms": elapsed.as_secs_f64() * 1000.0
            });
            eprintln!("{}", obj.to_string());
        } else {
            let line = format!(
                "[{:.3}s] [{}] === END ({:.3}s) ===",
                elapsed.as_secs_f64(),
                self.test_name,
                elapsed.as_secs_f64()
            );
            eprintln!("{line}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backward_compat_step_and_result() {
        let logger = TestLogger::new("compat_test");
        logger.step("Do something");
        logger.step_result(true, "It worked");
        logger.step("Do another thing");
        logger.step_result(false, "It failed");
        logger.finish(false);

        let lines = logger.lines();
        assert!(lines[0].contains("=== START: compat_test ==="));
        assert!(lines[1].contains("STEP 1: Do something"));
        assert!(lines[2].contains("PASS - It worked"));
        assert!(lines[3].contains("STEP 2: Do another thing"));
        assert!(lines[4].contains("FAIL - It failed"));
        // Failure summary before finish
        assert!(lines.iter().any(|l| l.contains("FAILURE SUMMARY")));
        assert!(lines.iter().any(|l| l.contains("RESULT: FAIL")));
    }

    #[test]
    fn test_step_timing_shows_duration_ms() {
        let logger = TestLogger::new("timing_test");
        logger.step("Timed step");
        std::thread::sleep(std::time::Duration::from_millis(10));
        logger.step_result(true, "Done");

        let lines = logger.lines();
        // The result line should contain duration in ms format
        let result_line = &lines[2];
        assert!(
            result_line.contains("ms)"),
            "Expected duration in ms, got: {result_line}"
        );
    }

    #[test]
    fn test_context_nesting() {
        let logger = TestLogger::new("context_test");
        logger.enter_context("RPC call");
        logger.step("Send request");
        logger.enter_context("Validating response");
        logger.step("Check status code");
        logger.step_result(true, "200 OK");
        logger.exit_context();
        logger.exit_context();
        logger.finish(true);

        let lines = logger.lines();
        // Should see context entry/exit markers
        assert!(lines.iter().any(|l| l.contains("→ RPC call")));
        assert!(lines
            .iter()
            .any(|l| l.contains("RPC call > Validating response > Check status code")));
        assert!(lines
            .iter()
            .any(|l| l.contains("← Validating response")));
        assert!(lines.iter().any(|l| l.contains("← RPC call")));
    }

    #[test]
    fn test_log_assert_pass() {
        let logger = TestLogger::new("assert_pass_test");
        logger.step("Check value");
        logger.log_assert(&42, &42, "answer");

        let lines = logger.lines();
        assert!(lines.iter().any(|l| l.contains("ASSERT") && l.contains("PASS")));
        assert!(logger.failures().is_empty());
    }

    #[test]
    #[should_panic(expected = "Assertion failed")]
    fn test_log_assert_fail_panics() {
        let logger = TestLogger::new("assert_fail_test");
        logger.step("Check value");
        logger.log_assert(&42, &99, "wrong answer");
    }

    #[test]
    fn test_failure_summary_collected() {
        let logger = TestLogger::new("failure_summary_test");
        logger.step("Good step");
        logger.step_result(true, "OK");
        logger.step("Bad step");
        logger.step_result(false, "Not OK");
        logger.step("Another bad step");
        logger.step_result(false, "Also bad");

        let failures = logger.failures();
        assert_eq!(failures.len(), 2);
        assert!(failures[0].contains("Bad step"));
        assert!(failures[1].contains("Another bad step"));
    }

    #[test]
    fn test_json_mode() {
        // Use explicit constructor to avoid env var leaking between parallel tests
        let logger = TestLogger::new_with_json("json_test", true);
        logger.step("A step");
        logger.step_result(true, "Passed");
        logger.log("A message");
        logger.finish(true);

        let lines = logger.lines();
        // All lines should be valid JSON-ish (start with '{')
        for line in &lines {
            assert!(
                line.starts_with('{'),
                "Expected JSON line, got: {line}"
            );
        }
        assert!(lines[0].contains(r#""event":"start""#));
        assert!(lines[1].contains(r#""event":"step""#));
        assert!(lines[2].contains(r#""event":"step_result""#));
        assert!(lines[3].contains(r#""event":"log""#));
        assert!(lines[4].contains(r#""event":"finish""#));
    }

    #[test]
    fn test_log_assert_bool() {
        let logger = TestLogger::new("assert_bool_test");
        logger.step("Boolean check");
        logger.log_assert_bool(true, "should be true");

        let lines = logger.lines();
        assert!(lines.iter().any(|l| l.contains("PASS")));
    }
}
