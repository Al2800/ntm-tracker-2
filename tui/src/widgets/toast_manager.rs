use crate::msg::ToastLevel;
use std::time::Instant;

/// A toast notification in the queue.
#[derive(Debug, Clone)]
pub struct ToastEntry {
    pub message: String,
    pub level: ToastLevel,
    pub created_at: Instant,
}

/// Manages a queue of toast notifications.
pub struct ToastQueue {
    pub toasts: Vec<ToastEntry>,
    pub duration_secs: u64,
}

impl ToastQueue {
    pub fn new() -> Self {
        Self {
            toasts: Vec::new(),
            duration_secs: 3,
        }
    }

    pub fn push(&mut self, message: String, level: ToastLevel) {
        self.toasts.push(ToastEntry {
            message,
            level,
            created_at: Instant::now(),
        });
    }

    /// Remove expired toasts and return whether any remain.
    pub fn tick(&mut self) -> bool {
        let duration = std::time::Duration::from_secs(self.duration_secs);
        self.toasts.retain(|t| t.created_at.elapsed() < duration);
        !self.toasts.is_empty()
    }

    pub fn is_empty(&self) -> bool {
        self.toasts.is_empty()
    }

    pub fn active(&self) -> Option<&ToastEntry> {
        self.toasts.last()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty() {
        let q = ToastQueue::new();
        assert!(q.is_empty());
        assert!(q.active().is_none());
    }

    #[test]
    fn test_push_and_active() {
        let mut q = ToastQueue::new();
        q.push("hello".to_string(), ToastLevel::Info);
        assert!(!q.is_empty());
        assert_eq!(q.active().unwrap().message, "hello");
    }

    #[test]
    fn test_tick_keeps_recent() {
        let mut q = ToastQueue::new();
        q.push("test".to_string(), ToastLevel::Success);
        assert!(q.tick()); // just pushed, should remain
    }

    #[test]
    fn test_multiple_toasts() {
        let mut q = ToastQueue::new();
        q.push("first".to_string(), ToastLevel::Info);
        q.push("second".to_string(), ToastLevel::Error);
        assert_eq!(q.toasts.len(), 2);
        assert_eq!(q.active().unwrap().message, "second");
    }
}
