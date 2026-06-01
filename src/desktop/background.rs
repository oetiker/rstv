//! `TBackground` ported per D2/D5/D7/D8 (row 29).
//!
//! The simplest concrete view: fills its entire extent with a repeated pattern
//! character styled with [`Role::Background`]. There are no overridden event
//! methods — `TBackground` only overrides `draw` in the C++ source
//! (`tbkgrnd.cpp`); `handleEvent` is the inherited no-op base.
//!
//! The C++ `getPalette` (`cpBackground "\x01"`) maps its single colour index to
//! [`Role::Background`] (D7). No `getPalette`/`getColor` methods appear on the
//! Rust type; a view simply calls `ctx.style(Role::Background)` instead.
//!
//! The `gfGrowHiX | gfGrowHiY` grow mode set in the C++ constructor is
//! faithfully carried over as `grow_mode.hi_x = true` and `grow_mode.hi_y =
//! true`, which causes the background to stretch with its owner on the right
//! and bottom edges — the expected desktop behaviour.
//!
//! Streamable `read`/`write`/`build` are dropped per D12.

use crate::theme::Role;
use crate::view::{DrawCtx, Rect, View, ViewState};

/// Desktop background fill — `TBackground` (D2/D7/D8, row 29).
///
/// Fills its extent with a single repeated `pattern` character, styled through
/// [`Role::Background`] from the active [`Theme`](crate::theme::Theme).
///
/// # Example
/// ```ignore
/// let bg = Background::new(Rect::new(0, 0, 80, 25), '▒');
/// ```
pub struct Background {
    st: ViewState,
    /// The fill character. TV's `char pattern`.
    pub pattern: char,
}

impl Background {
    /// `TBackground::TBackground(bounds, aPattern)` — construct a background.
    ///
    /// Sets `growMode = gfGrowHiX | gfGrowHiY` so the background stretches
    /// with its owner on the right and bottom edges, faithful to the C++ ctor.
    pub fn new(bounds: Rect, pattern: char) -> Self {
        let mut st = ViewState::new(bounds);
        // C++: growMode = gfGrowHiX | gfGrowHiY
        st.grow_mode.hi_x = true;
        st.grow_mode.hi_y = true;
        Background { st, pattern }
    }
}

impl View for Background {
    fn state(&self) -> &ViewState {
        &self.st
    }

    fn state_mut(&mut self) -> &mut ViewState {
        &mut self.st
    }

    /// `TBackground::draw` — fill the entire extent with `pattern`.
    ///
    /// C++ body: `b.moveChar(0, pattern, getColor(0x01), size.x)` per row, then
    /// `writeLine(0, 0, size.x, size.y, b)`.  Under D8 this collapses to a single
    /// `ctx.fill` call; D7 maps `getColor(0x01)` (palette index 1 of `cpBackground
    /// "\x01"` → resolved colour 1) to [`Role::Background`].
    fn draw(&mut self, ctx: &mut DrawCtx) {
        let ext = self.st.get_extent();
        let style = ctx.style(Role::Background);
        ctx.fill(ext, self.pattern, style);
    }

    // `TBackground::handleEvent` — NOT overridden in the C++ source.
    // The inherited `TView::handleEvent` is a no-op base (mouse-down select
    // relocated to TGroup, row 26). Default here; no override needed.
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{HeadlessBackend, Renderer};
    use crate::screen::Buffer;
    use crate::theme::Theme;
    use crate::view::{DrawCtx, Point};

    // -- Constructor ---------------------------------------------------------

    #[test]
    fn new_stores_pattern_and_bounds() {
        let bg = Background::new(Rect::new(0, 0, 10, 5), '▒');
        assert_eq!(bg.pattern, '▒');
        assert_eq!(bg.st.origin, Point::new(0, 0));
        assert_eq!(bg.st.size, Point::new(10, 5));
    }

    #[test]
    fn new_sets_grow_hi_x_and_hi_y() {
        let bg = Background::new(Rect::new(0, 0, 10, 5), '▒');
        assert!(bg.st.grow_mode.hi_x, "gfGrowHiX must be set");
        assert!(bg.st.grow_mode.hi_y, "gfGrowHiY must be set");
        // lo_x, lo_y, rel, fixed must stay clear
        assert!(!bg.st.grow_mode.lo_x);
        assert!(!bg.st.grow_mode.lo_y);
        assert!(!bg.st.grow_mode.rel);
        assert!(!bg.st.grow_mode.fixed);
    }

    #[test]
    fn new_inherits_view_state_defaults() {
        let bg = Background::new(Rect::new(5, 3, 25, 10), '░');
        // sfVisible must be set (TView ctor default)
        assert!(bg.st.state.visible);
        // dmLimitLoY must be set (TView ctor default)
        assert!(bg.st.drag_mode.limit_lo_y);
    }

    // -- draw ----------------------------------------------------------------

    #[test]
    fn draw_fills_extent_with_pattern() {
        let theme = Theme::classic_blue();
        let mut bg = Background::new(Rect::new(0, 0, 4, 2), 'X');
        let mut buf = Buffer::new(4, 2);
        {
            let bounds = bg.state().get_bounds();
            let mut ctx = DrawCtx::new(&mut buf, &theme, bounds, bounds.a);
            bg.draw(&mut ctx);
        }
        // Every cell must contain 'X'
        for y in 0..2u16 {
            for x in 0..4u16 {
                assert_eq!(buf.get(x, y).symbol(), "X", "cell ({x},{y}) must be 'X'");
            }
        }
        // Style must be Role::Background
        let expected_style = theme.style(Role::Background);
        assert_eq!(buf.get(0, 0).style(), expected_style);
        assert_eq!(buf.get(3, 1).style(), expected_style);
    }

    // -- Snapshot test -------------------------------------------------------

    /// End-to-end snapshot: `Background` through the real `Renderer` +
    /// `HeadlessBackend` path (the template every widget test copies).
    /// Drawn through `&mut dyn View` so the *trait* dispatch exercises `DrawCtx`.
    #[test]
    fn background_render_pipeline_snapshot() {
        let theme = Theme::classic_blue();
        let mut bg: Box<dyn View> = Box::new(Background::new(Rect::new(0, 0, 6, 3), '▒'));
        let (backend, screen) = HeadlessBackend::new(6, 3);
        let mut r = Renderer::new(Box::new(backend));
        r.render(|buf: &mut Buffer| {
            let bounds = bg.state().get_bounds();
            let mut dc = DrawCtx::new(buf, &theme, bounds, bounds.a);
            bg.draw(&mut dc);
        });
        insta::assert_snapshot!(screen.snapshot());
    }
}
