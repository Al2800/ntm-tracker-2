//! Render test helpers for FrankenTUI-based widget and screen testing.
//!
//! Provides Frame construction macros and Buffer inspection utilities
//! so any `#[cfg(test)]` module can verify rendered output without mocks.
//!
//! # Quick start
//! ```ignore
//! use crate::test_helpers::*;
//!
//! #[test]
//! fn my_render_test() {
//!     test_frame!(pool, frame, 80, 24);
//!     my_widget::render(&mut frame, frame.bounds(), &data);
//!     assert!(row_text(&frame.buffer, 0).contains("expected"));
//! }
//! ```

use ftui::render::buffer::Buffer;
use ftui::PackedRgba;

/// Create a GraphemePool and Frame with the given dimensions.
///
/// Declares two local bindings: a `GraphemePool` and a `Frame`.
/// The frame borrows the pool, so both must stay in scope.
///
/// # Example
/// ```ignore
/// test_frame!(pool, frame, 80, 24);
/// widget.render(frame.bounds(), &mut frame);
/// let line = row_text(&frame.buffer, 0);
/// ```
#[macro_export]
macro_rules! test_frame {
    ($pool:ident, $frame:ident, $w:expr, $h:expr) => {
        let mut $pool = ftui::GraphemePool::new();
        let mut $frame = ftui::render::frame::Frame::new($w, $h, &mut $pool);
    };
}

/// Extract all text from a single row of the buffer as a `String`.
///
/// Empty cells are represented as spaces. Returns an empty string
/// if `y` is out of bounds.
pub fn row_text(buf: &Buffer, y: u16) -> String {
    let mut row = String::with_capacity(buf.width() as usize);
    for x in 0..buf.width() {
        let ch = buf
            .get(x, y)
            .and_then(|c| c.content.as_char())
            .unwrap_or(' ');
        row.push(ch);
    }
    row
}

/// Extract all rows of the buffer as a `Vec<String>`.
///
/// Each row is converted via [`row_text`]. Trailing spaces are preserved
/// to maintain positional accuracy.
pub fn buf_to_lines(buf: &Buffer) -> Vec<String> {
    (0..buf.height()).map(|y| row_text(buf, y)).collect()
}

/// Assert that the cell at `(x, y)` contains the expected character.
///
/// # Panics
/// Panics with a descriptive message if the cell doesn't match or is out of bounds.
pub fn assert_cell_char(buf: &Buffer, x: u16, y: u16, expected: char) {
    let cell = buf
        .get(x, y)
        .unwrap_or_else(|| panic!("Cell ({x}, {y}) is out of bounds ({}x{})", buf.width(), buf.height()));
    let actual = cell.content.as_char().unwrap_or('\0');
    assert_eq!(
        actual, expected,
        "Cell ({x}, {y}): expected '{expected}', got '{actual}'"
    );
}

/// Assert that the cell at `(x, y)` has the expected foreground color.
///
/// # Panics
/// Panics with an RGB diff message if colors don't match.
pub fn assert_cell_fg(buf: &Buffer, x: u16, y: u16, expected: PackedRgba) {
    let cell = buf
        .get(x, y)
        .unwrap_or_else(|| panic!("Cell ({x}, {y}) is out of bounds ({}x{})", buf.width(), buf.height()));
    let actual = cell.fg;
    assert_eq!(
        actual, expected,
        "Cell ({x}, {y}) fg: expected rgb({},{},{}) got rgb({},{},{})",
        expected.r(), expected.g(), expected.b(),
        actual.r(), actual.g(), actual.b()
    );
}

/// Assert that the cell at `(x, y)` has the expected background color.
///
/// # Panics
/// Panics with an RGB diff message if colors don't match.
pub fn assert_cell_bg(buf: &Buffer, x: u16, y: u16, expected: PackedRgba) {
    let cell = buf
        .get(x, y)
        .unwrap_or_else(|| panic!("Cell ({x}, {y}) is out of bounds ({}x{})", buf.width(), buf.height()));
    let actual = cell.bg;
    assert_eq!(
        actual, expected,
        "Cell ({x}, {y}) bg: expected rgb({},{},{}) got rgb({},{},{})",
        expected.r(), expected.g(), expected.b(),
        actual.r(), actual.g(), actual.b()
    );
}

/// Find the first occurrence of `needle` within the buffer text.
///
/// Returns `Some((x, y))` where `x` is the column offset and `y` is the row,
/// or `None` if the text is not found. Only searches within single rows
/// (no cross-line matching).
pub fn find_text(buf: &Buffer, needle: &str) -> Option<(u16, u16)> {
    for y in 0..buf.height() {
        let row = row_text(buf, y);
        if let Some(col) = row.find(needle) {
            return Some((col as u16, y));
        }
    }
    None
}

/// Assert that `needle` appears somewhere in the rendered buffer.
///
/// # Panics
/// Panics with a dump of all buffer lines if the text is not found.
pub fn assert_text_present(buf: &Buffer, needle: &str) {
    if find_text(buf, needle).is_none() {
        let lines = buf_to_lines(buf);
        let dump: String = lines
            .iter()
            .enumerate()
            .map(|(i, l)| format!("  {i:>2}│{l}"))
            .collect::<Vec<_>>()
            .join("\n");
        panic!("Text \"{needle}\" not found in buffer:\n{dump}");
    }
}

/// Assert that `needle` does NOT appear in the rendered buffer.
///
/// # Panics
/// Panics if the text is found, showing its location.
pub fn assert_text_absent(buf: &Buffer, needle: &str) {
    if let Some((x, y)) = find_text(buf, needle) {
        panic!("Text \"{needle}\" unexpectedly found at ({x}, {y})");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ftui::render::cell::Cell;

    #[test]
    fn test_row_text_extracts_characters() {
        let mut buf = Buffer::new(5, 1);
        buf.set_raw(0, 0, Cell::from_char('H'));
        buf.set_raw(1, 0, Cell::from_char('e'));
        buf.set_raw(2, 0, Cell::from_char('l'));
        buf.set_raw(3, 0, Cell::from_char('l'));
        buf.set_raw(4, 0, Cell::from_char('o'));
        assert_eq!(row_text(&buf, 0), "Hello");
    }

    #[test]
    fn test_row_text_empty_cells_are_spaces() {
        let buf = Buffer::new(3, 1);
        assert_eq!(row_text(&buf, 0), "   ");
    }

    #[test]
    fn test_buf_to_lines_multi_row() {
        let mut buf = Buffer::new(2, 2);
        buf.set_raw(0, 0, Cell::from_char('A'));
        buf.set_raw(1, 0, Cell::from_char('B'));
        buf.set_raw(0, 1, Cell::from_char('C'));
        buf.set_raw(1, 1, Cell::from_char('D'));
        assert_eq!(buf_to_lines(&buf), vec!["AB", "CD"]);
    }

    #[test]
    fn test_assert_cell_char_passes() {
        let mut buf = Buffer::new(1, 1);
        buf.set_raw(0, 0, Cell::from_char('X'));
        assert_cell_char(&buf, 0, 0, 'X');
    }

    #[test]
    #[should_panic(expected = "expected 'Y', got 'X'")]
    fn test_assert_cell_char_fails() {
        let mut buf = Buffer::new(1, 1);
        buf.set_raw(0, 0, Cell::from_char('X'));
        assert_cell_char(&buf, 0, 0, 'Y');
    }

    #[test]
    fn test_assert_cell_fg_passes() {
        let mut buf = Buffer::new(1, 1);
        let cell = Cell::from_char('A').with_fg(PackedRgba::rgb(255, 0, 0));
        buf.set_raw(0, 0, cell);
        assert_cell_fg(&buf, 0, 0, PackedRgba::rgb(255, 0, 0));
    }

    #[test]
    fn test_assert_cell_bg_passes() {
        let mut buf = Buffer::new(1, 1);
        let cell = Cell::from_char('A').with_bg(PackedRgba::rgb(0, 0, 255));
        buf.set_raw(0, 0, cell);
        assert_cell_bg(&buf, 0, 0, PackedRgba::rgb(0, 0, 255));
    }

    #[test]
    fn test_find_text_found() {
        let mut buf = Buffer::new(10, 2);
        // Row 0: "  hello   "
        for (i, ch) in "  hello   ".chars().enumerate() {
            buf.set_raw(i as u16, 0, Cell::from_char(ch));
        }
        assert_eq!(find_text(&buf, "hello"), Some((2, 0)));
    }

    #[test]
    fn test_find_text_not_found() {
        let buf = Buffer::new(5, 1);
        assert_eq!(find_text(&buf, "hello"), None);
    }

    #[test]
    fn test_find_text_second_row() {
        let mut buf = Buffer::new(5, 2);
        for (i, ch) in "world".chars().enumerate() {
            buf.set_raw(i as u16, 1, Cell::from_char(ch));
        }
        assert_eq!(find_text(&buf, "world"), Some((0, 1)));
    }

    #[test]
    fn test_assert_text_present_passes() {
        let mut buf = Buffer::new(5, 1);
        for (i, ch) in "hello".chars().enumerate() {
            buf.set_raw(i as u16, 0, Cell::from_char(ch));
        }
        assert_text_present(&buf, "ell");
    }

    #[test]
    #[should_panic(expected = "not found in buffer")]
    fn test_assert_text_present_fails() {
        let buf = Buffer::new(5, 1);
        assert_text_present(&buf, "hello");
    }

    #[test]
    fn test_assert_text_absent_passes() {
        let buf = Buffer::new(5, 1);
        assert_text_absent(&buf, "hello");
    }

    #[test]
    #[should_panic(expected = "unexpectedly found")]
    fn test_assert_text_absent_fails() {
        let mut buf = Buffer::new(5, 1);
        for (i, ch) in "hello".chars().enumerate() {
            buf.set_raw(i as u16, 0, Cell::from_char(ch));
        }
        assert_text_absent(&buf, "hello");
    }

    #[test]
    fn test_frame_macro_creates_usable_frame() {
        test_frame!(pool, frame, 10, 5);
        assert_eq!(frame.width(), 10);
        assert_eq!(frame.height(), 5);
        // Verify we can write and read back
        frame
            .buffer
            .set_raw(0, 0, Cell::from_char('Z'));
        assert_cell_char(&frame.buffer, 0, 0, 'Z');
    }
}
