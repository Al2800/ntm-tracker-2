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

    #[test]
    fn test_tick_expires_immediately_with_zero_duration() {
        let mut q = ToastQueue::new();
        q.duration_secs = 0;
        q.push("hello".to_string(), ToastLevel::Info);
        assert!(!q.tick());
        assert!(q.is_empty());
        assert!(q.active().is_none());
    }

    #[test]
    fn test_active_is_most_recent_and_levels_preserved() {
        let mut q = ToastQueue::new();
        q.push("first".to_string(), ToastLevel::Info);
        q.push("second".to_string(), ToastLevel::Success);
        q.push("third".to_string(), ToastLevel::Error);

        assert_eq!(q.toasts.len(), 3);
        let active = q.active().unwrap();
        assert_eq!(active.message, "third");
        assert_eq!(active.level, ToastLevel::Error);
    }

    #[test]
    fn test_tick_cleans_up_all_expired_toasts() {
        let mut q = ToastQueue::new();
        q.duration_secs = 0;
        for i in 0..10 {
            q.push(format!("msg-{i}"), ToastLevel::Info);
        }
        assert!(!q.tick());
        assert!(q.is_empty());
        assert_eq!(q.toasts.len(), 0);
    }

    #[test]
    fn test_very_long_message_does_not_crash() {
        let mut q = ToastQueue::new();
        let msg = "a".repeat(500);
        q.push(msg.clone(), ToastLevel::Success);
        let active = q.active().unwrap();
        assert_eq!(active.message.len(), 500);
        assert_eq!(active.message, msg);
        assert!(q.tick());
    }

    #[test]
    fn test_rapid_push_keeps_order_and_active() {
        let mut q = ToastQueue::new();
        for i in 0..10 {
            q.push(format!("msg-{i}"), ToastLevel::Info);
        }
        assert_eq!(q.toasts.len(), 10);
        assert_eq!(q.active().unwrap().message, "msg-9");
    }

    #[test]
    fn test_tick_keeps_toast_with_large_duration() {
        let mut q = ToastQueue::new();
        q.duration_secs = 3600;
        q.push("kept".to_string(), ToastLevel::Info);
        assert!(q.tick());
        assert!(!q.is_empty());
        assert_eq!(q.active().unwrap().message, "kept");
    }

    #[test]
    fn test_tick_on_empty_queue_returns_false() {
        let mut q = ToastQueue::new();
        assert!(!q.tick());
        assert!(q.is_empty());
    }

    #[test]
    fn test_push_after_tick_cleanup() {
        let mut q = ToastQueue::new();
        q.duration_secs = 0;
        q.push("old".to_string(), ToastLevel::Info);
        q.tick(); // removes "old"
        assert!(q.is_empty());
        q.push("new".to_string(), ToastLevel::Success);
        assert_eq!(q.active().unwrap().message, "new");
        assert_eq!(q.toasts.len(), 1);
    }

    #[test]
    fn test_default_duration_is_3_seconds() {
        let q = ToastQueue::new();
        assert_eq!(q.duration_secs, 3);
    }

    #[test]
    fn test_all_toast_levels_preserved() {
        let mut q = ToastQueue::new();
        q.push("info".to_string(), ToastLevel::Info);
        q.push("success".to_string(), ToastLevel::Success);
        q.push("error".to_string(), ToastLevel::Error);
        assert_eq!(q.toasts[0].level, ToastLevel::Info);
        assert_eq!(q.toasts[1].level, ToastLevel::Success);
        assert_eq!(q.toasts[2].level, ToastLevel::Error);
    }

    #[test]
    fn test_mixed_expiry_keeps_recent() {
        use std::time::Duration;
        let mut q = ToastQueue::new();
        q.duration_secs = 1;
        // Manually insert an expired toast
        q.toasts.push(ToastEntry {
            message: "expired".to_string(),
            level: ToastLevel::Info,
            created_at: Instant::now() - Duration::from_secs(5),
        });
        // Push a fresh one
        q.push("fresh".to_string(), ToastLevel::Success);
        assert_eq!(q.toasts.len(), 2);
        assert!(q.tick()); // should remove expired, keep fresh
        assert_eq!(q.toasts.len(), 1);
        assert_eq!(q.active().unwrap().message, "fresh");
    }
}
