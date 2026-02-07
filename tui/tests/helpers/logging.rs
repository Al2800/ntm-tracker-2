use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Structured test logger with step tracking.
pub struct TestLogger {
    test_name: String,
    start: Instant,
    step: Mutex<usize>,
    lines: Arc<Mutex<Vec<String>>>,
}

impl TestLogger {
    pub fn new(test_name: &str) -> Self {
        let logger = Self {
            test_name: test_name.to_string(),
            start: Instant::now(),
            step: Mutex::new(0),
            lines: Arc::new(Mutex::new(Vec::new())),
        };
        logger.log(&format!("=== START: {} ===", test_name));
        logger
    }

    /// Log a numbered step.
    pub fn step(&self, description: &str) {
        let mut step = self.step.lock().unwrap();
        *step += 1;
        let elapsed = self.start.elapsed();
        let line = format!(
            "[{:.3}s] [{}] STEP {}: {}",
            elapsed.as_secs_f64(),
            self.test_name,
            *step,
            description
        );
        self.push_line(&line);
    }

    /// Log a step result (PASS/FAIL).
    pub fn step_result(&self, passed: bool, detail: &str) {
        let step = *self.step.lock().unwrap();
        let elapsed = self.start.elapsed();
        let status = if passed { "PASS" } else { "FAIL" };
        let line = format!(
            "[{:.3}s] [{}] STEP {}: {} - {}",
            elapsed.as_secs_f64(),
            self.test_name,
            step,
            status,
            detail
        );
        self.push_line(&line);
    }

    /// Log an arbitrary message.
    pub fn log(&self, msg: &str) {
        let elapsed = self.start.elapsed();
        let line = format!(
            "[{:.3}s] [{}] {}",
            elapsed.as_secs_f64(),
            self.test_name,
            msg
        );
        self.push_line(&line);
    }

    /// Print final result and total duration.
    pub fn finish(&self, passed: bool) {
        let elapsed = self.start.elapsed();
        let status = if passed { "PASS" } else { "FAIL" };
        let line = format!(
            "[{:.3}s] [{}] RESULT: {} ({:.3}s)",
            elapsed.as_secs_f64(),
            self.test_name,
            status,
            elapsed.as_secs_f64()
        );
        self.push_line(&line);
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

    fn push_line(&self, line: &str) {
        eprintln!("{line}");
        self.lines.lock().unwrap().push(line.to_string());
    }
}

impl Drop for TestLogger {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed();
        let line = format!(
            "[{:.3}s] [{}] === END ({:.3}s) ===",
            elapsed.as_secs_f64(),
            self.test_name,
            elapsed.as_secs_f64()
        );
        eprintln!("{line}");
    }
}
