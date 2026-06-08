//! Color-dialog data classes (row 81): [`ColorItem`], [`ColorGroup`],
//! [`ColorIndex`].
//!
//! These are **pure data** types — no rendering, no event handling.  They back
//! the color-selector dialog family (rows 85–87).
//!
//! ## Collections → `Vec` deviation
//! The C++ types use `next`-pointer singly-linked lists to form chains of items
//! and groups.  Per the rstv "collections → `Vec`" deviation the linked-list
//! machinery is replaced by owned `Vec`s:
//! * `TColorItem::next` → membership in a `Vec<ColorItem>` inside `ColorGroup`.
//! * `TColorGroup::next` → membership in a `Vec<ColorGroup>` at the call site.
//! * The C++ `operator+` chaining helpers are **not** ported; `vec![]` and the
//!   [`ColorGroup::with_item`] builder are the Rust-idiomatic equivalents.
//!
//! ## `char*` → `String`
//! Both `TColorItem::name` and `TColorGroup::name` were C-string pointers
//! heap-allocated via `newStr`.  In Rust they are plain `String` fields owned
//! by the struct.

// ---------------------------------------------------------------------------
// ColorItem — row 81
// ---------------------------------------------------------------------------

/// `TColorItem` (row 81) — a (name, palette-index) pair in a color group.
///
/// The C++ type heap-allocates a `const char* name` via `newStr` and stores
/// a `uchar index` that is a **palette index** — immutable once constructed.
/// The `next` linked-list pointer is replaced by membership in
/// [`ColorGroup::items`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorItem {
    /// `TColorItem::name` — display name of this color entry.
    name: String,
    /// `TColorItem::index` — palette index; set at construction, never mutated.
    index: u8,
}

impl ColorItem {
    /// `TColorItem::TColorItem(nm, idx)` — construct from any `Into<String>`.
    pub fn new(name: impl Into<String>, index: u8) -> Self {
        ColorItem {
            name: name.into(),
            index,
        }
    }

    /// `TColorItem::name` — the display name of this color entry.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// `TColorItem::index` — the palette index (immutable after construction).
    pub fn index(&self) -> u8 {
        self.index
    }
}

// ---------------------------------------------------------------------------
// ColorGroup — row 81
// ---------------------------------------------------------------------------

/// `TColorGroup` (row 81) — a named group of [`ColorItem`]s with a mutable
/// focus position.
///
/// **Naming trap:** `TColorGroup::index` and `TColorItem::index` share the
/// C++ field name `index` but are completely different concepts:
/// * [`ColorItem::index`] is a **palette index**, set at construction,
///   immutable.
/// * [`ColorGroup::index`] is the **focused-item position within the group**,
///   defaulting to `0` and mutated later by `setGroupIndex` (row 85).  It is
///   *not* a constructor parameter (the C++ constructor leaves it
///   uninitialized; it is always written before it is read).
///
/// The `TColorItem* items` linked list is replaced by an owned
/// `Vec<ColorItem>`.  The `next` pointer is replaced by membership in a
/// `Vec<ColorGroup>` at the call site.  The C++ `operator+` chaining helpers
/// are **not** ported; use `vec![]` + [`ColorGroup::with_item`] instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorGroup {
    /// `TColorGroup::name` — display name of this group.
    name: String,
    /// `TColorGroup::index` — focused-item *position* within the group;
    /// defaults to `0`, mutated by rows 85–87.
    index: u8,
    /// `TColorGroup::items` — owned collection replacing the C++ linked list.
    items: Vec<ColorItem>,
}

impl ColorGroup {
    /// `TColorGroup::TColorGroup(nm, itm)` — construct with an initial item
    /// list.  `index` defaults to `0` (not a parameter; the C++ constructor
    /// leaves it uninitialized, always written before read).
    pub fn new(name: impl Into<String>, items: Vec<ColorItem>) -> Self {
        ColorGroup {
            name: name.into(),
            index: 0,
            items,
        }
    }

    /// Fluent builder: push one [`ColorItem`] and return `self`.
    ///
    /// This is the idiomatic replacement for the C++ `group + item` `operator+`
    /// chaining.  Example:
    /// ```rust
    /// use tvision::dialog::ColorGroup;
    ///
    /// let g = ColorGroup::new("Desktop", vec![])
    ///     .with_item("Color", 1)
    ///     .with_item("Mono", 2);
    /// assert_eq!(g.items().len(), 2);
    /// ```
    pub fn with_item(mut self, name: impl Into<String>, index: u8) -> Self {
        self.items.push(ColorItem::new(name, index));
        self
    }

    /// `TColorGroup::name` — the display name of this group.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// `TColorGroup::index` — focused-item position (mutable focus state).
    pub fn index(&self) -> u8 {
        self.index
    }

    /// Set the focused-item position.  Called by `setGroupIndex` (row 85).
    pub fn set_index(&mut self, index: u8) {
        self.index = index;
    }

    /// `TColorGroup::items` — the items in this group.
    pub fn items(&self) -> &[ColorItem] {
        &self.items
    }
}

// ---------------------------------------------------------------------------
// ColorIndex — row 81
// ---------------------------------------------------------------------------

/// `TColorIndex` (row 81) — the per-group focused-item index table used by the
/// color-selector dialog.
///
/// The C++ struct is a true variable-length allocation (`new uchar[numGroups +
/// 2]`); the `colorIndex[256]` in the header is only a sentinel declaration,
/// never the real size.  The runtime layout is:
/// ```c
/// // runtime layout — allocated as `new uchar[numGroups + 2]`:
/// uchar groupIndex;    // which group is focused
/// uchar colorSize;     // == numGroups
/// uchar colorIndex[];  // numGroups entries (C99 flexible array member)
/// ```
/// In Rust the flexible array member is a `Vec<u8>` sized to `numGroups`, and
/// `colorSize` is implicit as `color_index.len()` (the `Vec` carries its own
/// length), so there is no separate `color_size` field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorIndex {
    /// `TColorIndex::groupIndex` — which color group is currently focused.
    group_index: u8,
    /// `TColorIndex::colorIndex[]` — per-group focused-item positions;
    /// `colorSize` == `self.color_index.len()`.
    color_index: Vec<u8>,
}

impl ColorIndex {
    /// Construct with an initial `group_index` and `color_index` table.
    pub fn new(group_index: u8, color_index: Vec<u8>) -> Self {
        ColorIndex {
            group_index,
            color_index,
        }
    }

    /// `TColorIndex::groupIndex` — the currently focused group.
    pub fn group_index(&self) -> u8 {
        self.group_index
    }

    /// Set the focused group.  Called by `setGroupIndex` (row 85).
    pub fn set_group_index(&mut self, group_index: u8) {
        self.group_index = group_index;
    }

    /// `TColorIndex::colorIndex[]` — per-group focused-item index table.
    pub fn color_index(&self) -> &[u8] {
        &self.color_index
    }

    /// `TColorIndex::colorSize` — number of entries in the index table
    /// (derived from `Vec::len`; no separate field needed).
    pub fn color_size(&self) -> usize {
        self.color_index.len()
    }
}

// ---------------------------------------------------------------------------
// ColorSel — row 82
// ---------------------------------------------------------------------------

/// `TColorSelector::ColorSel` (row 82) — which half of the attribute byte this
/// selector controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSel {
    /// `csBackground` — the selector controls the upper 4 bits (bg nibble).
    Background,
    /// `csForeground` — the selector controls the lower 4 bits (fg nibble).
    Foreground,
}

// ---------------------------------------------------------------------------
// ColorSelector — row 82
// ---------------------------------------------------------------------------

/// `TColorSelector` (row 82) — a 4×4 (or 4×2) grid widget that displays the
/// 16 BIOS colors and lets the user pick one.
///
/// Unlike most rstv widgets, `ColorSelector` draws **raw BIOS colors** (not
/// theme roles): the whole point is to show the literal 16-color palette.
/// `selType == Background` yields a 2-row grid (colors 0–7); `Foreground`
/// yields a 4-row grid (colors 0–15). The view's `size.y` clips the grid —
/// callers use a 12×2 or 12×4 bounds and rely on clip for partial grids.
///
/// `eventMask |= evBroadcast` is implicit in rstv (the group delivers every
/// broadcast unconditionally) — no event mask field is set.
pub struct ColorSelector {
    /// The base-view state (bounds, flags, id, …).
    state: crate::view::ViewState,
    /// `TColorSelector::color` — the currently selected 4-bit color index (0–15).
    color: u8,
    /// `TColorSelector::selType` — which attribute nibble this selector controls.
    sel_type: ColorSel,
}

impl ColorSelector {
    /// `TColorSelector::TColorSelector(bounds, aSelType)` — build the selector.
    ///
    /// Sets `ofSelectable | ofFirstClick | ofFramed` on `options`; `color`
    /// starts at 0.  `eventMask |= evBroadcast` is implicit (see module doc).
    pub fn new(bounds: crate::view::Rect, sel_type: ColorSel) -> Self {
        use crate::view::Options;
        let mut state = crate::view::ViewState::new(bounds);
        state.options = Options {
            selectable: true,
            first_click: true,
            framed: true,
            ..Default::default()
        };
        ColorSelector {
            state,
            color: 0,
            sel_type,
        }
    }

    /// The currently selected color index (0–15).
    ///
    /// Exposed so rows 83/87 broker code can read the color from the resolved
    /// view via `as_any_mut` downcast — the same pattern as `FileList::focused_rec`.
    pub fn color(&self) -> u8 {
        self.color
    }

    /// `TColorSelector::colorChanged` — private helper: broadcast the color-changed
    /// command, using `self`'s `ViewId` as the source so consumers (rows 83/84)
    /// can resolve the color via `color()` (D4 payload-less broadcast pattern).
    fn color_changed(&mut self, ctx: &mut crate::view::Context) {
        let cmd = if self.sel_type == ColorSel::Foreground {
            crate::command::Command::COLOR_FOREGROUND_CHANGED
        } else {
            crate::command::Command::COLOR_BACKGROUND_CHANGED
        };
        // A view with no stamped `ViewId` (not yet inserted into a group) cannot be
        // the `source` of a D4 payload-less broadcast, so the emission is skipped.
        if let Some(id) = self.state.id() {
            ctx.broadcast(cmd, Some(id));
        }
    }
}

impl crate::view::View for ColorSelector {
    fn state(&self) -> &crate::view::ViewState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut crate::view::ViewState {
        &mut self.state
    }

    /// `TColorSelector::draw` — render the 4×4 BIOS-color grid.
    ///
    /// Each color `c` (0–15) occupies 3 cells of `'\u{2588}'` (█, CP437 0xDB)
    /// in the column range `c%4*3 .. c%4*3+3`, on row `c/4`.  The selected
    /// cell has a `'\u{25D8}'` (◘, CP437 0x08) marker at the middle cell.
    /// When `c == 0` the marker uses attr 0x70 (gray) so it is visible against
    /// the otherwise-black-on-black cell.
    ///
    /// The C++ loop runs `for i in 0..=size.y` (inclusive), relying on the
    /// `TDrawBuffer`/`writeLine` clipping to discard out-of-bounds rows.  Here
    /// we loop `0..4` (all four logical grid rows) and rely on `DrawCtx`'s
    /// clip instead — the result is identical.
    fn draw(&mut self, ctx: &mut crate::view::DrawCtx) {
        use crate::color::{Color, Style};
        use crate::view::Rect;

        let w = self.state.size.x;
        // attr 0x70 = fg 0 (black) on bg 7 (gray): row fill + c==0 marker override
        let gray = Style::new(Color::Bios(0), Color::Bios(7));

        for i in 0..4_i32 {
            // Fill the whole row with a gray space (the `moveChar(0,' ',0x70,size.x)` step).
            ctx.fill(Rect::new(0, i, w, i + 1), ' ', gray);
            for j in 0..4_i32 {
                let c = (i * 4 + j) as u8;
                // attr `c`: fg = c (low nibble), bg = 0 (black) — pure BIOS color.
                let cell = Style::new(Color::Bios(c & 0x0F), Color::Bios((c >> 4) & 0x0F));
                // 3 cells of '█' (icon = 0xDB).
                for dx in 0..3_i32 {
                    ctx.put_char(j * 3 + dx, i, '\u{2588}', cell);
                }
                if c == self.color {
                    // Marker '◘' (CP437 0x08) at the middle cell.
                    // c==0 is black-on-black: force attr 0x70 so the marker is visible.
                    let marker_style = if c == 0 { gray } else { cell };
                    ctx.put_char(j * 3 + 1, i, '\u{25D8}', marker_style);
                }
            }
        }
    }

    /// `TColorSelector::handleEvent` — mouse picks a color; arrow keys navigate
    /// the grid with wrap; broadcast `cmColorSet` sets the color (inert pending row 83).
    ///
    /// Control flow faithfully mirrors the C++: the common tail `drawView() +
    /// colorChanged() + clearEvent()` runs **only** for handled `MouseDown` and
    /// matched arrow keys.  Any other branch returns early without clearing.
    ///
    /// The C++ `TView::handleEvent(event)` base call is dropped (inert — the base
    /// only does the `ofSelectable` mouse-select, which is relocated to the group
    /// in rstv, D3).
    fn handle_event(&mut self, ev: &mut crate::event::Event, ctx: &mut crate::view::Context) {
        use crate::event::{Event, Key};

        let width: u8 = 4;
        let max_col: u8 = if self.sel_type == ColorSel::Background {
            7
        } else {
            15
        };

        match *ev {
            Event::MouseDown(m) => {
                // Mouse coords are already view-local in rstv (D3: the group
                // delivers local positions; `makeLocal` is the identity here).
                // TODO(row 31, D9): mouse press-and-hold drag loop not yet wired.
                self.color = (m.position.y * 4 + m.position.x / 3) as u8;
                self.color_changed(ctx);
                ev.clear();
            }
            Event::KeyDown(ke) => {
                let ke = crate::event::ctrl_to_arrow(ke);
                match ke.key {
                    Key::Left => {
                        if self.color > 0 {
                            self.color -= 1;
                        } else {
                            self.color = max_col;
                        }
                    }
                    Key::Right => {
                        if self.color < max_col {
                            self.color += 1;
                        } else {
                            self.color = 0;
                        }
                    }
                    Key::Up => {
                        if self.color > width - 1 {
                            self.color -= width;
                        } else if self.color == 0 {
                            self.color = max_col;
                        } else {
                            self.color += max_col - width;
                        }
                    }
                    Key::Down => {
                        if self.color < max_col - (width - 1) {
                            self.color += width;
                        } else if self.color == max_col {
                            self.color = 0;
                        } else {
                            self.color -= max_col - width;
                        }
                    }
                    // Any other key: return WITHOUT clearing the event (faithful C++ behavior).
                    _ => return,
                }
                self.color_changed(ctx);
                ev.clear();
            }
            Event::Broadcast { command, .. } if command == crate::command::Command::COLOR_SET => {
                // TODO(row 83): on cmColorSet, resolve the TColorDisplay source's attr and set
                // color = (selType==Background) ? attr>>4 : attr&0x0F. Needs the row-83 view as the
                // resolvable broadcast source (the row-77 ResolveFocusedFile / row-80 MakeButtonDefault
                // broker pattern). No consumer/source exists at row 82, so this arm is inert.
                // Do NOT ev.clear() here.
            }
            _ => {}
        }
    }

    /// The pump's broker downcasts the resolved subscriber to `ColorSelector`, so
    /// `as_any_mut` MUST return `self`.
    fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> {
        Some(self)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- ColorItem ---

    #[test]
    fn color_item_construction_and_accessors() {
        let item = ColorItem::new("Desktop", 42);
        assert_eq!(item.name(), "Desktop");
        assert_eq!(item.index(), 42);
    }

    #[test]
    fn color_item_from_string_types() {
        // Accepts &str, String, or any Into<String>.
        let a = ColorItem::new(String::from("A"), 1);
        let b = ColorItem::new("B", 2);
        assert_eq!(a.name(), "A");
        assert_eq!(b.name(), "B");
    }

    // --- ColorGroup ---

    #[test]
    fn color_group_defaults_index_to_zero() {
        let g = ColorGroup::new("Desktop", vec![]);
        assert_eq!(g.index(), 0, "index must default to 0 (not a ctor param)");
    }

    #[test]
    fn color_group_stores_items() {
        let items = vec![ColorItem::new("Color", 1), ColorItem::new("Mono", 2)];
        let g = ColorGroup::new("Desktop", items);
        assert_eq!(g.items().len(), 2);
        assert_eq!(g.items()[0].name(), "Color");
        assert_eq!(g.items()[1].name(), "Mono");
    }

    #[test]
    fn color_group_with_item_builder_chains() {
        let g = ColorGroup::new("Desktop", vec![])
            .with_item("Color", 1)
            .with_item("Mono", 2)
            .with_item("Cyan", 3);
        assert_eq!(g.items().len(), 3);
        assert_eq!(g.items()[2].name(), "Cyan");
        assert_eq!(g.items()[2].index(), 3);
    }

    #[test]
    fn color_group_set_index_mutates() {
        let mut g = ColorGroup::new("Desktop", vec![]);
        assert_eq!(g.index(), 0);
        g.set_index(5);
        assert_eq!(g.index(), 5);
    }

    // --- ColorIndex ---

    #[test]
    fn color_index_construction_and_accessors() {
        let ci = ColorIndex::new(2, vec![0, 1, 3]);
        assert_eq!(ci.group_index(), 2);
        assert_eq!(ci.color_index(), &[0u8, 1, 3]);
    }

    #[test]
    fn color_index_color_size_equals_vec_len() {
        let ci = ColorIndex::new(0, vec![5, 6, 7, 8]);
        assert_eq!(ci.color_size(), 4);
        assert_eq!(ci.color_size(), ci.color_index().len());
    }

    #[test]
    fn color_index_set_group_index_mutates() {
        let mut ci = ColorIndex::new(0, vec![1, 2]);
        ci.set_group_index(3);
        assert_eq!(ci.group_index(), 3);
    }

    #[test]
    fn color_index_empty_table() {
        let ci = ColorIndex::new(0, vec![]);
        assert_eq!(ci.color_size(), 0);
    }

    // =========================================================================
    // ColorSelector tests (row 82)
    // =========================================================================

    use crate::backend::{HeadlessBackend, Renderer};
    use crate::command::Command;
    use crate::event::{Event, Key, KeyEvent, KeyModifiers, MouseButtons, MouseEvent};
    use crate::screen::Buffer;
    use crate::theme::Theme;
    use crate::timer::TimerQueue;
    use crate::view::{Context, Deferred, Rect, View};
    use std::collections::VecDeque;

    /// Run a closure with a fresh `Context`, returning drained out-events + the return value.
    fn with_ctx<R>(
        timers: &mut TimerQueue,
        now_ms: u64,
        f: impl FnOnce(&mut Context) -> R,
    ) -> (Vec<Event>, R) {
        let mut out = VecDeque::new();
        let mut deferred: Vec<Deferred> = Vec::new();
        let r = {
            let mut ctx = Context::new(&mut out, timers, now_ms, &mut deferred);
            f(&mut ctx)
        };
        (out.into_iter().collect(), r)
    }

    fn key_ev(k: Key) -> Event {
        Event::KeyDown(KeyEvent::new(k, KeyModifiers::default()))
    }

    fn mouse_down_ev(x: i32, y: i32) -> Event {
        Event::MouseDown(MouseEvent {
            position: crate::view::Point::new(x, y),
            buttons: MouseButtons {
                left: true,
                ..Default::default()
            },
            ..Default::default()
        })
    }

    // -----------------------------------------------------------------------
    // Accessor test
    // -----------------------------------------------------------------------

    #[test]
    fn color_accessor_returns_current_color() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        assert_eq!(sel.color(), 0);
        sel.color = 7;
        assert_eq!(sel.color(), 7);
    }

    // -----------------------------------------------------------------------
    // Keyboard navigation — Foreground (max_col = 15)
    // -----------------------------------------------------------------------

    #[test]
    fn fg_left_wraps_at_zero_to_max() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        sel.color = 0;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Left);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 15, "Left at 0 should wrap to 15 (max_col)");
        assert!(ev.is_nothing(), "event must be cleared after handled arrow");
    }

    #[test]
    fn fg_left_decrements() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        sel.color = 5;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Left);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 4);
    }

    #[test]
    fn fg_right_wraps_at_max_to_zero() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        sel.color = 15;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Right);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 0, "Right at 15 should wrap to 0");
    }

    #[test]
    fn fg_right_increments() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        sel.color = 3;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Right);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 4);
    }

    #[test]
    fn fg_up_from_zero_wraps_to_max() {
        // Up at 0: color == 0 branch → color = max_col (15)
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        sel.color = 0;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Up);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 15, "Up at 0 → 15");
    }

    #[test]
    fn fg_up_from_row_one_mid_branch() {
        // color = 3 (row 0, not > width-1=3, not == 0) → color += max_col - width = 15-4 = 11 → 14
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        sel.color = 3;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Up);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 14, "Up at 3 → 14 (mid branch: 3 + 11)");
    }

    #[test]
    fn fg_up_from_row_greater_than_zero() {
        // color = 4 (row 1) > width-1=3 → color -= width → 0
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        sel.color = 4;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Up);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 0, "Up at 4 → 0 (color -= 4)");
    }

    #[test]
    fn fg_down_from_max_wraps_to_zero() {
        // color = 15 == max_col → color = 0
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        sel.color = 15;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Down);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 0, "Down at 15 → 0");
    }

    #[test]
    fn fg_down_increments_in_normal_range() {
        // color = 3 < max_col - (width-1) = 15 - 3 = 12 → color += 4 → 7
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        sel.color = 3;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Down);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 7, "Down at 3 → 7");
    }

    #[test]
    fn fg_down_mid_branch() {
        // color = 13 (not < 12, not == 15) → color -= max_col - width = 11 → 2
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        sel.color = 13;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Down);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 2, "Down at 13 → 2 (mid branch: 13-11)");
    }

    // -----------------------------------------------------------------------
    // Keyboard navigation — Background (max_col = 7)
    // -----------------------------------------------------------------------

    #[test]
    fn bg_left_wraps_at_zero_to_seven() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 2), ColorSel::Background);
        sel.color = 0;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Left);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 7, "Left at 0 → 7 (BG max_col)");
    }

    #[test]
    fn bg_right_wraps_at_seven_to_zero() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 2), ColorSel::Background);
        sel.color = 7;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Right);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 0, "Right at 7 → 0 (BG max_col)");
    }

    #[test]
    fn bg_up_from_zero_wraps_to_seven() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 2), ColorSel::Background);
        sel.color = 0;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Up);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 7, "Up at 0 → 7 (BG)");
    }

    #[test]
    fn bg_up_mid_branch() {
        // color = 3 (row 0, not > 3, not == 0) → color += max_col - width = 7 - 4 = 3 → 6
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 2), ColorSel::Background);
        sel.color = 3;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Up);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 6, "Up at 3 → 6 (BG mid branch: 3+3)");
    }

    #[test]
    fn bg_up_from_row_one() {
        // color = 4 > width-1=3 → color -= 4 → 0
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 2), ColorSel::Background);
        sel.color = 4;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Up);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 0, "Up at 4 → 0 (BG: color -= 4)");
    }

    #[test]
    fn bg_down_from_max_wraps_to_zero() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 2), ColorSel::Background);
        sel.color = 7;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Down);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 0, "Down at 7 → 0 (BG max)");
    }

    #[test]
    fn bg_down_increments() {
        // color = 1 < max_col - (width-1) = 7 - 3 = 4 → color += 4 → 5
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 2), ColorSel::Background);
        sel.color = 1;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Down);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 5, "Down at 1 → 5 (BG)");
    }

    #[test]
    fn bg_down_mid_branch() {
        // color = 5 (not < 4, not == 7) → color -= max_col - width = 3 → 2
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 2), ColorSel::Background);
        sel.color = 5;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Down);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 2, "Down at 5 → 2 (BG mid: 5-3)");
    }

    #[test]
    fn bg_down_at_three_hits_first_branch() {
        // max_col=7, width=4; 3 < max_col-(width-1) = 7-3 = 4 → TRUE → color += 4 → 7
        // (The advisor oracle: BG Down 3→7.)
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 2), ColorSel::Background);
        sel.color = 3;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Down);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 7, "Down at 3 (BG) → 7 (first branch: 3 + 4)");
    }

    // -----------------------------------------------------------------------
    // Non-arrow key leaves color unchanged and does NOT clear the event
    // -----------------------------------------------------------------------

    #[test]
    fn non_arrow_key_does_not_clear_event_or_change_color() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        sel.color = 5;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Enter);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 5, "color must not change on non-arrow key");
        assert!(
            !ev.is_nothing(),
            "event must NOT be cleared for non-arrow key"
        );
    }

    #[test]
    fn escape_key_does_not_clear_event() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        sel.color = 3;
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Esc);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 3);
        assert!(!ev.is_nothing(), "Esc must not be cleared");
    }

    // -----------------------------------------------------------------------
    // Mouse pick
    // -----------------------------------------------------------------------

    #[test]
    fn mouse_down_sets_color_from_position() {
        // y=0, x=3..5 → col 1 → color = 0*4 + 3/3 = 1
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        let mut timers = TimerQueue::new();
        let mut ev = mouse_down_ev(3, 0);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 1, "x=3 → x/3=1, y=0 → color=1");
        assert!(ev.is_nothing(), "MouseDown must clear the event");
    }

    #[test]
    fn mouse_down_picks_row_two_col_three() {
        // y=2, x=9 → color = 2*4 + 9/3 = 8 + 3 = 11
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        let mut timers = TimerQueue::new();
        let mut ev = mouse_down_ev(9, 2);
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 11, "x=9,y=2 → color=11");
    }

    // -----------------------------------------------------------------------
    // Broadcast cmColorSet is inert (no-op, no clear)
    // -----------------------------------------------------------------------

    #[test]
    fn broadcast_color_set_is_inert() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        sel.color = 5;
        let mut timers = TimerQueue::new();
        let mut ev = Event::Broadcast {
            command: Command::COLOR_SET,
            source: None,
        };
        with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(
            sel.color(),
            5,
            "cmColorSet must not change color (row 82 inert)"
        );
        // The event must not be cleared either.
        assert!(
            !ev.is_nothing(),
            "cmColorSet must not clear the event at row 82"
        );
    }

    // -----------------------------------------------------------------------
    // color_changed broadcast seam (D4 payload-less + source contract for rows 83/87)
    // -----------------------------------------------------------------------

    /// A handled arrow on a FG selector (with a stamped `ViewId`) must emit
    /// exactly one `COLOR_FOREGROUND_CHANGED` broadcast with `source = Some(id)`.
    #[test]
    fn fg_arrow_broadcasts_foreground_changed_with_source() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        let id = crate::view::ViewId::next();
        sel.state.id = Some(id);
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Right);
        let (out, _) = with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(
            out,
            vec![Event::Broadcast {
                command: Command::COLOR_FOREGROUND_CHANGED,
                source: Some(id),
            }],
            "FG arrow must broadcast COLOR_FOREGROUND_CHANGED with the selector's id"
        );
    }

    /// A handled arrow on a BG selector (with a stamped `ViewId`) must emit
    /// exactly one `COLOR_BACKGROUND_CHANGED` broadcast with `source = Some(id)`.
    #[test]
    fn bg_arrow_broadcasts_background_changed_with_source() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 2), ColorSel::Background);
        let id = crate::view::ViewId::next();
        sel.state.id = Some(id);
        let mut timers = TimerQueue::new();
        let mut ev = key_ev(Key::Left);
        let (out, _) = with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(
            out,
            vec![Event::Broadcast {
                command: Command::COLOR_BACKGROUND_CHANGED,
                source: Some(id),
            }],
            "BG arrow must broadcast COLOR_BACKGROUND_CHANGED with the selector's id"
        );
    }

    /// A MouseDown pick on a FG selector (with a stamped `ViewId`) must set the
    /// color from the position AND emit `COLOR_FOREGROUND_CHANGED` with `source = Some(id)`.
    #[test]
    fn fg_mouse_down_broadcasts_foreground_changed_with_source() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        let id = crate::view::ViewId::next();
        sel.state.id = Some(id);
        let mut timers = TimerQueue::new();
        // x=9, y=2 → color = 2*4 + 9/3 = 11
        let mut ev = mouse_down_ev(9, 2);
        let (out, _) = with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 11, "x=9,y=2 → color=11");
        assert_eq!(
            out,
            vec![Event::Broadcast {
                command: Command::COLOR_FOREGROUND_CHANGED,
                source: Some(id),
            }],
            "FG MouseDown must broadcast COLOR_FOREGROUND_CHANGED with the selector's id"
        );
    }

    /// A MouseDown pick on a BG selector (with a stamped `ViewId`) must emit
    /// `COLOR_BACKGROUND_CHANGED` with `source = Some(id)`.
    #[test]
    fn bg_mouse_down_broadcasts_background_changed_with_source() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 2), ColorSel::Background);
        let id = crate::view::ViewId::next();
        sel.state.id = Some(id);
        let mut timers = TimerQueue::new();
        // x=3, y=1 → color = 1*4 + 3/3 = 5
        let mut ev = mouse_down_ev(3, 1);
        let (out, _) = with_ctx(&mut timers, 0, |ctx| sel.handle_event(&mut ev, ctx));
        assert_eq!(sel.color(), 5, "x=3,y=1 → color=5");
        assert_eq!(
            out,
            vec![Event::Broadcast {
                command: Command::COLOR_BACKGROUND_CHANGED,
                source: Some(id),
            }],
            "BG MouseDown must broadcast COLOR_BACKGROUND_CHANGED with the selector's id"
        );
    }

    // -----------------------------------------------------------------------
    // Snapshot tests
    // -----------------------------------------------------------------------

    fn render_selector(sel: &mut ColorSelector) -> String {
        let theme = Theme::classic_blue();
        let size = sel.state.size;
        let (backend, screen) = HeadlessBackend::new(size.x as u16, size.y as u16);
        let mut r = Renderer::new(Box::new(backend));
        r.render(|buf: &mut Buffer| {
            let bounds = sel.state.get_bounds();
            let mut dc = crate::view::DrawCtx::new(buf, &theme, bounds, bounds.a);
            sel.draw(&mut dc);
        });
        screen.snapshot()
    }

    /// 12×4 FG selector, color=0: marker ◘ on the black cell is rendered with
    /// gray (0x70) so it is visible; the rest of cell 0 is black-on-black.
    #[test]
    fn snapshot_foreground() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        // color defaults to 0
        insta::assert_snapshot!(render_selector(&mut sel));
    }

    /// 12×2 BG selector: shows only colors 0–7 (2 rows, clip drops rows 2–3).
    #[test]
    fn snapshot_background() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 2), ColorSel::Background);
        // color defaults to 0
        insta::assert_snapshot!(render_selector(&mut sel));
    }

    /// 12×4 FG selector with color=10 (row 2, col 2): ◘ marker sits on a
    /// colored cell (color 10 = light green fg on black bg).
    #[test]
    fn snapshot_foreground_selected() {
        let mut sel = ColorSelector::new(Rect::new(0, 0, 12, 4), ColorSel::Foreground);
        sel.color = 10;
        insta::assert_snapshot!(render_selector(&mut sel));
    }
}
