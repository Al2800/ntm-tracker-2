use ftui::core::geometry::Rect;
use ftui::render::buffer::Buffer;
use ftui::render::frame::Frame;
use ftui::render::grapheme_pool::GraphemePool;
use ftui::PackedRgba;

/// Container for testing render output. Provides a closure-based API to avoid
/// borrow checker issues with Frame's lifetime.
///
/// # Usage
/// ```ignore
/// let mut tf = TestFrame::new(80, 24);
/// tf.render(|frame, area| {
///     my_widget.render(area, frame);
/// });
/// assert!(tf.contains_text("Hello"));
/// tf.assert_row_contains(0, "Dashboard");
/// ```
pub struct TestFrame {
    pool: GraphemePool,
    buffer: Buffer,
    width: u16,
    height: u16,
}

impl TestFrame {
    /// Create a test frame with the given dimensions.
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            pool: GraphemePool::new(),
            buffer: Buffer::new(width, height),
            width,
            height,
        }
    }

    /// Render into a fresh frame, then save the buffer for inspection.
    /// The closure receives a mutable Frame and the full area Rect.
    pub fn render<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Frame<'_>, Rect),
    {
        let area = Rect::from_size(self.width, self.height);
        let mut frame = Frame::new(self.width, self.height, &mut self.pool);
        f(&mut frame, area);
        self.buffer = frame.buffer;
    }

    /// Get the full area as a Rect.
    pub fn area(&self) -> Rect {
        Rect::from_size(self.width, self.height)
    }

    /// Get the width.
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Get the height.
    pub fn height(&self) -> u16 {
        self.height
    }

    /// Get the underlying buffer reference (for direct inspection).
    pub fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    // --- Cell inspection helpers ---

    /// Get the character at (x, y), or None if out of bounds or empty/continuation.
    pub fn char_at(&self, x: u16, y: u16) -> Option<char> {
        self.buffer
            .get(x, y)
            .and_then(|cell| cell.content.as_char())
    }

    /// Get the foreground color at (x, y).
    pub fn fg_at(&self, x: u16, y: u16) -> Option<PackedRgba> {
        self.buffer.get(x, y).map(|cell| cell.fg)
    }

    /// Get the background color at (x, y).
    pub fn bg_at(&self, x: u16, y: u16) -> Option<PackedRgba> {
        self.buffer.get(x, y).map(|cell| cell.bg)
    }

    /// Extract all text from a row as a single string.
    /// Empty/continuation cells are mapped to spaces. Trailing spaces trimmed.
    pub fn row_text(&self, y: u16) -> String {
        let mut result = String::with_capacity(self.width as usize);
        for x in 0..self.width {
            match self.char_at(x, y) {
                Some(ch) => result.push(ch),
                None => result.push(' '),
            }
        }
        result.truncate(result.trim_end().len());
        result
    }

    /// Extract all text from all rows, joined with newlines.
    pub fn all_text(&self) -> String {
        (0..self.height)
            .map(|y| self.row_text(y))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Find a substring within the rendered buffer rows.
    /// Returns (x, y) of the first occurrence, or None.
    pub fn find_text(&self, needle: &str) -> Option<(u16, u16)> {
        for y in 0..self.height {
            let row = self.row_text(y);
            if let Some(x) = row.find(needle) {
                return Some((x as u16, y));
            }
        }
        None
    }

    /// Check whether a substring exists anywhere in the rendered buffer.
    pub fn contains_text(&self, needle: &str) -> bool {
        self.find_text(needle).is_some()
    }

    // --- Assertion helpers ---

    /// Assert that a cell at (x, y) contains the expected character.
    pub fn assert_char(&self, x: u16, y: u16, expected: char) {
        let actual = self.char_at(x, y);
        assert_eq!(
            actual,
            Some(expected),
            "Cell at ({x}, {y}): expected '{expected}', got {actual:?}"
        );
    }

    /// Assert that a cell at (x, y) has the expected foreground color.
    pub fn assert_fg(&self, x: u16, y: u16, expected: PackedRgba) {
        let actual = self.fg_at(x, y);
        assert_eq!(
            actual,
            Some(expected),
            "FG at ({x}, {y}): expected {expected:?}, got {actual:?}"
        );
    }

    /// Assert that a cell at (x, y) has the expected background color.
    pub fn assert_bg(&self, x: u16, y: u16, expected: PackedRgba) {
        let actual = self.bg_at(x, y);
        assert_eq!(
            actual,
            Some(expected),
            "BG at ({x}, {y}): expected {expected:?}, got {actual:?}"
        );
    }

    /// Assert that a row contains the expected text (substring match).
    pub fn assert_row_contains(&self, y: u16, needle: &str) {
        let row = self.row_text(y);
        assert!(
            row.contains(needle),
            "Row {y} does not contain '{needle}'. Row text: '{row}'"
        );
    }

    /// Assert that the buffer contains the given text somewhere.
    pub fn assert_contains(&self, needle: &str) {
        assert!(
            self.contains_text(needle),
            "Buffer does not contain '{needle}'. Full text:\n{}",
            self.all_text()
        );
    }

    /// Assert that the buffer does NOT contain the given text.
    pub fn assert_not_contains(&self, needle: &str) {
        assert!(
            !self.contains_text(needle),
            "Buffer unexpectedly contains '{needle}'"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ftui::widgets::paragraph::Paragraph;
    use ftui::widgets::Widget;
    use ftui::Style;

    #[test]
    fn test_frame_creation_and_dimensions() {
        let tf = TestFrame::new(80, 24);
        assert_eq!(tf.width(), 80);
        assert_eq!(tf.height(), 24);
        assert_eq!(tf.area(), Rect::from_size(80, 24));
    }

    #[test]
    fn test_render_paragraph_and_find_text() {
        let mut tf = TestFrame::new(40, 5);
        tf.render(|frame, area| {
            let para = Paragraph::new("Hello, world!").style(Style::new());
            para.render(area, frame);
        });
        assert!(tf.contains_text("Hello, world!"));
        assert_eq!(tf.find_text("Hello"), Some((0, 0)));
    }

    #[test]
    fn test_row_text_extraction() {
        let mut tf = TestFrame::new(20, 3);
        tf.render(|frame, area| {
            let text = "Line 1\nLine 2\nLine 3";
            let para = Paragraph::new(text).style(Style::new());
            para.render(area, frame);
        });
        assert_eq!(tf.row_text(0), "Line 1");
        assert_eq!(tf.row_text(1), "Line 2");
        assert_eq!(tf.row_text(2), "Line 3");
    }

    #[test]
    fn test_char_at_and_empty_cells() {
        let mut tf = TestFrame::new(10, 1);
        tf.render(|frame, area| {
            let para = Paragraph::new("AB").style(Style::new());
            para.render(area, frame);
        });
        assert_eq!(tf.char_at(0, 0), Some('A'));
        assert_eq!(tf.char_at(1, 0), Some('B'));
        // Empty cells return None from CellContent::as_char()
        assert_eq!(tf.char_at(2, 0), None);
    }

    #[test]
    fn test_assert_contains_and_not_contains() {
        let mut tf = TestFrame::new(30, 2);
        tf.render(|frame, area| {
            let para = Paragraph::new("Dashboard").style(Style::new());
            para.render(area, frame);
        });
        tf.assert_contains("Dashboard");
        tf.assert_not_contains("Sessions");
    }

    #[test]
    fn test_fg_color_inspection() {
        let mut tf = TestFrame::new(10, 1);
        let green = PackedRgba::rgb(0, 255, 0);
        tf.render(|frame, area| {
            let para = Paragraph::new("Hi").style(Style::new().fg(green));
            para.render(area, frame);
        });
        tf.assert_fg(0, 0, green);
        tf.assert_fg(1, 0, green);
    }
}
