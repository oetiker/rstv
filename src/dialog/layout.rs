//! Named layout metrics for dialogs — the recovered classic Turbo Vision
//! conventions (confirmed against `msgbox.cpp`/`tfildlg.cpp`), so dialogs stop
//! inventing their own coordinates. See `docs/design/dialog-layout.md`.

use crate::view::Point;

/// Standard button: 10 columns × 2 rows (row 2 is the drop shadow).
pub const STD_BUTTON: Point = Point::new(10, 2);
/// Cells between adjacent buttons in a button row.
pub const BUTTON_GAP: i32 = 2;
/// Content inset from the left frame.
pub const MARGIN_LEFT: i32 = 3;
/// Content inset from the right frame.
pub const MARGIN_RIGHT: i32 = 2;
/// Content inset from the top frame.
pub const MARGIN_TOP: i32 = 2;
/// Button-row top edge = `dialog_height - BUTTON_ROW_FROM_BOTTOM`.
pub const BUTTON_ROW_FROM_BOTTOM: i32 = 3;

/// How [`Dialog::button_row`](crate::dialog::Dialog::button_row) places buttons.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonRowAlign {
    /// Centered (message-box convention).
    Center,
    /// Right-grouped, ending [`MARGIN_RIGHT`] from the right frame (action dialogs).
    Right,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn metrics_match_recovered_tv() {
        assert_eq!(STD_BUTTON, Point::new(10, 2));
        assert_eq!(
            (
                BUTTON_GAP,
                MARGIN_LEFT,
                MARGIN_RIGHT,
                MARGIN_TOP,
                BUTTON_ROW_FROM_BOTTOM
            ),
            (2, 3, 2, 2, 3)
        );
    }
}
