use crate::rpc::types::EventView;
use crate::theme;
use ftui::core::geometry::Rect;
use ftui::render::frame::Frame;
use ftui::Style;
use ftui::widgets::block::Block;
use ftui::widgets::borders::Borders;
use ftui::widgets::paragraph::Paragraph;
use ftui::widgets::Widget;

/// Bucket event timestamps into 24 hourly bins relative to `now`.
/// Returns (buckets, max_value).
pub(crate) fn bucket_events(timestamps: &[i64], now: i64) -> ([u32; 24], u32) {
    let day_ago = now - 86400;
    let mut buckets = [0u32; 24];
    for &ts in timestamps {
        if ts >= day_ago && ts <= now {
            let offset = (ts - day_ago) as usize;
            let bucket = (offset / 3600).min(23);
            buckets[bucket] += 1;
        }
    }
    let max_val = buckets.iter().copied().max().unwrap_or(1).max(1);
    (buckets, max_val)
}

/// Map a bucket value to a sparkline character index (0..=7).
pub(crate) fn spark_index(value: u32, max_val: u32) -> usize {
    if max_val > 0 {
        ((value as usize) * 7) / (max_val as usize)
    } else {
        0
    }
    .min(7)
}

/// Render a 24-hour sparkline based on event timestamps.
pub fn render(frame: &mut Frame, area: Rect, events: &[EventView]) {
    let block = Block::new()
        .title(" Activity (24h) ")
        .borders(Borders::ALL)
        .border_style(Style::new().fg(theme::BG_SURFACE))
        .style(theme::raised_style());

    let now = chrono::Utc::now().timestamp();
    let timestamps: Vec<i64> = events.iter().map(|e| e.detected_at).collect();
    let (buckets, max_val) = bucket_events(&timestamps, now);

    // Build sparkline string.
    let spark: String = buckets
        .iter()
        .map(|&v| {
            let idx = spark_index(v, max_val);
            theme::SPARK_CHARS[idx]
        })
        .collect();

    // Build hour labels.
    let day_ago = now - 86400;
    let start_hour = {
        use chrono::prelude::*;
        let dt = chrono::DateTime::from_timestamp(day_ago, 0)
            .unwrap_or_else(|| Utc::now());
        dt.with_timezone(&Local).hour()
    };

    let mut labels = String::from("  ");
    for i in (0..24).step_by(4) {
        let h = (start_hour as usize + i) % 24;
        labels.push_str(&format!("{h:<6}"));
    }

    let text = format!("  {spark}\n{labels}");
    let para = Paragraph::new(text)
        .style(Style::new().fg(theme::ACTIVE).bg(theme::BG_RAISED))
        .block(block);

    para.render(area, frame);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_events() {
        let (buckets, max_val) = bucket_events(&[], 1000000);
        assert!(buckets.iter().all(|&b| b == 0));
        assert_eq!(max_val, 1); // max(0, 1) = 1
    }

    #[test]
    fn test_single_event() {
        let now = 100000;
        let ts = now - 3600; // 1 hour ago -> bucket 22
        let (buckets, max_val) = bucket_events(&[ts], now);
        let total: u32 = buckets.iter().sum();
        assert_eq!(total, 1);
        assert_eq!(max_val, 1);
    }

    #[test]
    fn test_events_same_hour() {
        let now = 100000;
        let ts1 = now - 100;
        let ts2 = now - 200;
        let ts3 = now - 300;
        let (buckets, max_val) = bucket_events(&[ts1, ts2, ts3], now);
        assert_eq!(max_val, 3);
        let total: u32 = buckets.iter().sum();
        assert_eq!(total, 3);
    }

    #[test]
    fn test_events_spread_24h() {
        let now = 200000;
        let timestamps: Vec<i64> = (0..24).map(|h| now - 86400 + h * 3600 + 1800).collect();
        let (buckets, _max_val) = bucket_events(&timestamps, now);
        // Each hour should have 1 event
        for &b in &buckets {
            assert_eq!(b, 1, "Expected 1 event per bucket, got: {buckets:?}");
        }
    }

    #[test]
    fn test_max_scaling() {
        let idx = spark_index(10, 10);
        assert_eq!(idx, 7);
    }

    #[test]
    fn test_zero_max_no_divide_by_zero() {
        let idx = spark_index(0, 0);
        assert_eq!(idx, 0);
    }

    #[test]
    fn test_spark_index_mid() {
        let idx = spark_index(5, 10);
        assert_eq!(idx, 3); // (5*7)/10 = 3
    }

    #[test]
    fn test_spark_chars_length() {
        assert_eq!(theme::SPARK_CHARS.len(), 8);
    }

    #[test]
    fn test_out_of_range_events_ignored() {
        let now = 100000;
        let too_old = now - 90000; // > 24h ago
        let future = now + 1000;   // in future
        let (buckets, _) = bucket_events(&[too_old, future], now);
        assert!(buckets.iter().all(|&b| b == 0));
    }
}
