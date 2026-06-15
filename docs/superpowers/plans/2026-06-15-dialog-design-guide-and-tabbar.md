# Dialog Layout Guide + TabBar + PageStack + Color-Picker Rebuild — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Codify rstv's dialog layout language (guide + constants + `Dialog::button_row`), add two composable rstv-original widgets — a cluster-shaped **`TabBar`** selector and a **`PageStack`** content multiplexer coupled by a D3 pump broker — and rebuild the color picker as a `TabBar`+`PageStack`+info-column group with gray chrome and renamed tabs.

**Architecture:** `TabBar` ↔ `PageStack` mirror `ScrollBar` ↔ `Scroller` exactly: the selector broadcasts `Command::TAB_BAR_CHANGED` carrying its `ViewId` as `source`; the `PageStack` (which stores the bound `tab_bar` id) filters on `source`, queues `Deferred::PageStackSync`; the pump reads `tab_bar.value()` and calls `PageStack::set_active(idx, &mut ctx)` (flip `sfVisible` via `set_visible_descendant` + `focus_child`). The picker's four surfaces become page Views sharing one `Rc<RefCell<ColorModel>>`; draggable surfaces self-drive via the standard `ctx.start_mouse_track` capture, retiring the bespoke `drag.rs` broker.

**Tech Stack:** Rust (workspace `rstv` + `rstv-macros`), `insta` snapshots (D11), the `View`/`Group`/`Context`/`DrawCtx`/`Deferred` substrate, the `#[delegate(to = …)]` proc-macro.

---

## Conventions for every task

- **Cargo target dir:** every `cargo` command runs with `export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target`.
- **Core limit (shared machine):** pass `-j 2` and `--test-threads=2`.
- **Snapshots:** `cargo-insta` is NOT installed. Generate with `INSTA_UPDATE=always cargo test …`, then **read and hand-verify** the `.snap` before committing.
- **Pre-commit gate:**
  ```bash
  export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
  cargo test --workspace -- --test-threads=2
  cargo clippy --workspace --all-targets -- -D warnings
  cargo fmt --all --check
  ```
- Commit messages end with the project Co-Authored-By trailer. Work on branch `dialog-design-guide`.
- **No new `View` trait method is introduced anywhere in this plan** (TabBar/PageStack only override existing methods), so `rstv-macros/src/specs.rs` needs no new forwarder. A new `Deferred` variant needs no forwarder.

---

## File Structure

- `src/command.rs` — **modify**: `Command::TAB_BAR_CHANGED`.
- `src/dialog/layout.rs` — **create**: constants + `ButtonRowAlign`.
- `src/dialog/dialog.rs` — **modify**: `Dialog::button_row` + tests.
- `src/dialog/mod.rs`, `src/lib.rs` — **modify**: module wiring + re-exports.
- `docs/design/dialog-layout.md` — **create**: the guide.
- `src/widgets/tab_bar.rs` — **create**: `TabBar` (+ tests).
- `src/widgets/page_stack.rs` — **create**: `PageStack` (+ tests).
- `src/widgets/mod.rs` — **modify**: module + re-exports.
- `src/view/context.rs` — **modify**: `Deferred::PageStackSync` + `Context::request_sync_page_stack`.
- `src/app/program.rs` — **modify**: pump broker arm (beside `SyncScrollerDelta`); program-level integration test.
- `src/dialog/colorpick/` — **modify**: `model.rs` (Rc sharing), new `page.rs` (generic `SurfacePage<S>`), new `info.rs` (`InfoColumn`), `mod.rs` (rebuild `ColorPicker` as a group); **delete** `drag.rs`; surfaces lose their `m`-param coupling only as needed.
- `examples/tvdemo.rs` — **modify**: re-lay `color_window()`.

---

# PIECE 1 — Guide, constants, button-row helper

### Task 1: `Command::TAB_BAR_CHANGED`

**Files:** Modify `src/command.rs` (after `SCROLL_BAR_CLICKED`, ~line 159)

- [ ] **Step 1: Add the constant**

```rust
    /// Broadcast by a [`TabBar`](crate::widgets::TabBar) when its selected tab
    /// changes; carries the bar's own [`ViewId`](crate::view::ViewId) as the
    /// broadcast `source` so a sibling [`PageStack`](crate::widgets::PageStack)
    /// can tell which bar fired (the D3/D4 pattern, mirroring SCROLL_BAR_CHANGED).
    pub const TAB_BAR_CHANGED: Command = Command("tv.tab_bar_changed");
```

- [ ] **Step 2: Build** — `cargo build -p rstv -j 2` → clean.
- [ ] **Step 3: Commit** — `git add src/command.rs && git commit -m "feat(command): add Command::TAB_BAR_CHANGED"`

---

### Task 2: Layout constants + `ButtonRowAlign`

**Files:** Create `src/dialog/layout.rs`; modify `src/dialog/mod.rs`, `src/lib.rs`

- [ ] **Step 1: Create `src/dialog/layout.rs`**

```rust
//! Named layout metrics for dialogs — the recovered classic Turbo Vision
//! conventions (confirmed against `msgbox.cpp`/`tfildlg.cpp`), so dialogs stop
//! inventing their own coordinates. See `docs/design/dialog-layout.md`.

use crate::view::Point;

/// Standard button: 10 columns × 2 rows (row 2 is the drop shadow).
pub const STD_BUTTON: Point = Point { x: 10, y: 2 };
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
        assert_eq!(STD_BUTTON, Point { x: 10, y: 2 });
        assert_eq!((BUTTON_GAP, MARGIN_LEFT, MARGIN_RIGHT, MARGIN_TOP, BUTTON_ROW_FROM_BOTTOM), (2, 3, 2, 2, 3));
    }
}
```

- [ ] **Step 2: Wire it.** In `src/dialog/mod.rs` add `mod layout;` (after `mod filedlg;`) and to the `pub use` block:
```rust
pub use layout::{ButtonRowAlign, BUTTON_GAP, BUTTON_ROW_FROM_BOTTOM, MARGIN_LEFT, MARGIN_RIGHT, MARGIN_TOP, STD_BUTTON};
```
In `src/lib.rs`, extend the existing `pub use dialog::{ … }` block with the same seven names (alphabetical).

- [ ] **Step 3: Test** — `cargo test -p rstv layout:: -- --test-threads=2` → PASS.
- [ ] **Step 4: Gate + commit**
```bash
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
git add src/dialog/layout.rs src/dialog/mod.rs src/lib.rs
git commit -m "feat(dialog): named layout constants + ButtonRowAlign"
```

---

### Task 3: `Dialog::button_row`

**Files:** Modify `src/dialog/dialog.rs`

- [ ] **Step 1: Failing tests** — add to `mod tests` (add `use crate::dialog::ButtonRowAlign; use crate::widgets::ButtonFlags;` at the top of the module):

```rust
    #[test]
    fn button_row_center_places_two_buttons_symmetrically() {
        let mut d = Dialog::new(Rect::new(0, 0, 40, 12), Some("D".into()));
        let ids = d.button_row(
            &[("~O~K", Command::OK, ButtonFlags { default: true, ..ButtonFlags::new() }),
              ("~C~ancel", Command::CANCEL, ButtonFlags::new())],
            ButtonRowAlign::Center);
        assert_eq!(ids.len(), 2);
        let b0 = d.child_mut(ids[0]).unwrap().state().get_bounds();
        let b1 = d.child_mut(ids[1]).unwrap().state().get_bounds();
        assert_eq!((b0.a.x, b0.a.y), (9, 9), "centered, row top = h-3");
        assert_eq!(b1.a.x, 9 + 10 + 2, "after gap");
        assert_eq!((b0.b.x - b0.a.x, b0.b.y - b0.a.y), (10, 2));
    }

    #[test]
    fn button_row_right_groups_against_right_margin() {
        let mut d = Dialog::new(Rect::new(0, 0, 40, 12), Some("D".into()));
        let ids = d.button_row(
            &[("~O~K", Command::OK, ButtonFlags::new()),
              ("~C~ancel", Command::CANCEL, ButtonFlags::new())],
            ButtonRowAlign::Right);
        assert_eq!(d.child_mut(ids[1]).unwrap().state().get_bounds().b.x, 38, "right edge at w - MARGIN_RIGHT");
        assert_eq!(d.child_mut(ids[0]).unwrap().state().get_bounds().a.x, 16);
    }
```

- [ ] **Step 2: Verify fail** — `cargo test -p rstv button_row -- --test-threads=2` → FAIL (no method `button_row`).

- [ ] **Step 3: Implement** — add to `impl Dialog` (after `child_mut`):

```rust
    /// Insert a conventional button row: standard 10×2 buttons,
    /// [`BUTTON_GAP`](crate::dialog::BUTTON_GAP) apart, top edge at
    /// `height - BUTTON_ROW_FROM_BOTTOM`. `align` centers or right-groups the row.
    /// Returns the inserted ids in the given order.
    pub fn button_row(
        &mut self,
        buttons: &[(&str, crate::command::Command, crate::widgets::ButtonFlags)],
        align: crate::dialog::ButtonRowAlign,
    ) -> Vec<ViewId> {
        use crate::dialog::layout::{BUTTON_GAP, BUTTON_ROW_FROM_BOTTOM, MARGIN_RIGHT, STD_BUTTON};
        use crate::dialog::ButtonRowAlign;
        use crate::widgets::Button;
        let size = self.state().size;
        let n = buttons.len() as i32;
        if n == 0 { return Vec::new(); }
        let span = n * STD_BUTTON.x + (n - 1) * BUTTON_GAP;
        let left = match align {
            ButtonRowAlign::Center => (size.x - span) / 2,
            ButtonRowAlign::Right => size.x - MARGIN_RIGHT - span,
        };
        let top = size.y - BUTTON_ROW_FROM_BOTTOM;
        let mut ids = Vec::with_capacity(buttons.len());
        let mut x = left;
        for (title, command, flags) in buttons {
            let mut b = Button::new(Rect::new(0, 0, STD_BUTTON.x, STD_BUTTON.y), title, *command, *flags);
            b.state.move_to(x, top);
            ids.push(self.insert_child(Box::new(b)));
            x += STD_BUTTON.x + BUTTON_GAP;
        }
        ids
    }
```

- [ ] **Step 4: Pass** — `cargo test -p rstv button_row -- --test-threads=2` → PASS.
- [ ] **Step 5: Gate + commit**
```bash
cargo test --workspace -- --test-threads=2 && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
git add src/dialog/dialog.rs
git commit -m "feat(dialog): Dialog::button_row helper (center/right, STD_BUTTON metrics)"
```

---

### Task 4: Dialog layout guide doc

**Files:** Create `docs/design/dialog-layout.md`

- [ ] **Step 1: Write the guide** with these sections (real prose, no placeholders):
  1. Dialog construction (`Dialog::new`: gray palette, `move|close`, no grow/zoom, drop shadow).
  2. Interior margins (3 left / 2 right / 2 top / 3 bottom) — cite the `layout.rs` constants.
  3. The button row (10×2, `BUTTON_GAP` 2, top at `height-3`, center vs right) — point at `Dialog::button_row`.
  4. Labels & inputs (label above / left; link via `LabelNormal`/`LabelLight`).
  5. Separating regions (whitespace + a `StaticText` header, never drawn lines).
  6. Roles for gray-dialog chrome — **do not use blue-window roles (`FramePassive`/`ScrollerNormal`) inside a gray dialog** (the picker bug).
  7. Shadows (buttons self-shadow; dialogs cast a desktop shadow; no manual calls).
  8. rstv-original extensions: `TabBar` (corner-cap tab strip) + `PageStack` (content multiplexer) — when to use them, that `TabBar` broadcasts and `PageStack` consumes via the pump broker (cross-link `ScrollBar`/`Scroller`). Note the tab idiom and the multi-page container are rstv-original (TV has only `TGroup` + `sfVisible`).
  9. Conformance note: `msgbox`/`inputbox` already conform; migrating `filedlg`/theme-editor is a later pass.

- [ ] **Step 2: Sanity-read** the doc against `layout.rs`. — [ ] **Step 3: Commit**
```bash
git add docs/design/dialog-layout.md
git commit -m "docs(design): dialog layout guide (margins, button row, gray-chrome roles, TabBar/PageStack)"
```

---

# PIECE 2a — `TabBar` (cluster-shaped selector)

**Design (Tasks 5–7).** `TabBar` is a focusable single-row selector with the `TCluster` contract (`TMonoSelector` precedent). For a tab strip the cursor ≡ the selection (moving commits), so a single `value: usize` suffices. Vocabulary: `selected()`/`press(item, ctx)`/`find_sel(p)`; transfer via `value()`/`set_value`; press-on-release mouse via `ctx.start_mouse_track` (the `ScrollBar`/`cluster.rs` pattern); broadcasts `TAB_BAR_CHANGED` with `source = self.id()` on a real change.

### Task 5: `TabBar` skeleton — value protocol, `find_sel`, `natural_width`

**Files:** Create `src/widgets/tab_bar.rs`; modify `src/widgets/mod.rs`, `src/lib.rs`

- [ ] **Step 1: Create `src/widgets/tab_bar.rs`**

```rust
//! A single-row tab selector: corner-cap active tab (`┌Label┐`), ←/→/hotkey/click
//! selection. An rstv-original widget — no Turbo Vision ancestor for the tab idiom
//! — but **cluster-shaped**: it follows the `TCluster`/`TRadioButtons` contract
//! (single selection, `~X~` hotkeys, `find_sel` hit-test, press-on-release,
//! `value`/`set_value` transfer), the way the C++ color dialog's
//! `TMonoSelector : public TCluster` was a custom selector built on the cluster.
//! On a change it broadcasts [`Command::TAB_BAR_CHANGED`] carrying its own
//! [`ViewId`](crate::view::ViewId) as `source`, so a sibling
//! [`PageStack`](crate::widgets::PageStack) can react (mirrors `ScrollBar`).
//!
//! # Turbo Vision heritage
//! None — the tabbed idiom is an rstv extension; the selection mechanics mirror
//! `TCluster`.

use crate::capture::TrackMask;
use crate::command::Command;
use crate::data::FieldValue;
use crate::event::{hot_key, Event, Key};
use crate::theme::Role;
use crate::view::{Context, DrawCtx, Point, Rect, View, ViewState};

/// A horizontal single-row tab selector. See the [module docs](self).
pub struct TabBar {
    /// View state (geometry, flags) — the composition target.
    pub state: ViewState,
    /// Tab labels, each optionally carrying a `~X~` hotkey marker.
    tabs: Vec<String>,
    /// Selected/active tab (cursor ≡ selection for a tab strip), clamped.
    value: usize,
    /// Absolute origin of bar-local (0,0), cached each `draw` for the track capture.
    abs_origin: Point,
    /// Whether a press-on-release mouse track is in flight.
    tracking: bool,
    /// The tab the mouse went down on (pressed only if MouseUp lands on the same one).
    pressed: Option<usize>,
}

impl TabBar {
    /// Construct at `bounds` with `labels` (each may carry `~X~`). Focusable; starts on tab 0.
    pub fn new(bounds: Rect, labels: &[&str]) -> Self {
        let mut state = ViewState::new(bounds);
        state.options.selectable = true;
        TabBar {
            state,
            tabs: labels.iter().map(|s| s.to_string()).collect(),
            value: 0,
            abs_origin: bounds.a,
            tracking: false,
            pressed: None,
        }
    }

    /// The selected tab index.
    pub fn selected(&self) -> usize { self.value }

    fn label_len(label: &str) -> i32 { label.chars().filter(|&c| c != '~').count() as i32 }

    /// `(start_x, width)` per tab; the active tab is +2 wide (its caps); 1-cell gaps.
    fn tab_layout(&self) -> Vec<(i32, i32)> {
        let mut out = Vec::with_capacity(self.tabs.len());
        let mut x = 0i32;
        for (i, label) in self.tabs.iter().enumerate() {
            let w = Self::label_len(label) + if i == self.value { 2 } else { 0 };
            out.push((x, w));
            x += w + 1;
        }
        out
    }

    /// Natural width to fit all tabs (labels + caps for the one active + gaps). Stable.
    pub fn natural_width(&self) -> i32 {
        let n = self.tabs.len() as i32;
        if n == 0 { return 0; }
        self.tabs.iter().map(|l| Self::label_len(l)).sum::<i32>() + 2 + (n - 1)
    }

    /// The tab under view-local point `p`, or `None`.
    fn find_sel(&self, p: Point) -> Option<usize> {
        if p.y != 0 { return None; }
        for (i, (start, w)) in self.tab_layout().iter().enumerate() {
            if p.x >= *start && p.x < start + w { return Some(i); }
        }
        None
    }

    /// Select `item` (clamped); broadcast `TAB_BAR_CHANGED` (source = self) only on change.
    fn press(&mut self, item: usize, ctx: &mut Context) {
        let item = item.min(self.tabs.len().saturating_sub(1));
        if item != self.value {
            self.value = item;
            ctx.broadcast(Command::TAB_BAR_CHANGED, self.state.id());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn natural_width_sums_labels_caps_gaps() {
        assert_eq!(TabBar::new(Rect::new(0, 0, 20, 1), &["AB", "CDE"]).natural_width(), 8);
        assert_eq!(TabBar::new(Rect::new(0, 0, 20, 1), &["~P~resets"]).natural_width(), 9);
    }

    #[test]
    fn value_protocol_reports_and_sets_clamped() {
        let mut tb = TabBar::new(Rect::new(0, 0, 20, 1), &["A", "B", "C"]);
        assert_eq!(View::value(&tb), Some(FieldValue::Int(0)));
        View::set_value(&mut tb, FieldValue::Int(5)); // no ctx on the trait setter
        assert_eq!(tb.selected(), 2);
    }

    #[test]
    fn find_sel_locates_tab_under_point() {
        // "A"(active w3 [0..3)) gap@3 "B"(w1 [4..5)) gap@5 "C"(w1 [6..7))
        let tb = TabBar::new(Rect::new(0, 0, 20, 1), &["A", "B", "C"]);
        assert_eq!(tb.find_sel(Point::new(1, 0)), Some(0));
        assert_eq!(tb.find_sel(Point::new(4, 0)), Some(1));
        assert_eq!(tb.find_sel(Point::new(6, 0)), Some(2));
        assert_eq!(tb.find_sel(Point::new(3, 0)), None, "the gap");
        assert_eq!(tb.find_sel(Point::new(1, 1)), None, "wrong row");
    }
}

impl View for TabBar {
    fn state(&self) -> &ViewState { &self.state }
    fn state_mut(&mut self) -> &mut ViewState { &mut self.state }

    /// Selected index as the typed transfer currency (getData successor).
    fn value(&self) -> Option<FieldValue> { Some(FieldValue::Int(self.value as i32)) }

    /// Load the selected index (clamped). The trait setter takes NO ctx
    /// (there is a separate `set_value_ctx`) — match that signature.
    fn set_value(&mut self, v: FieldValue) {
        if let FieldValue::Int(i) = v { self.value = (i.max(0) as usize).min(self.tabs.len().saturating_sub(1)); }
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> { Some(self) }

    fn draw(&mut self, _ctx: &mut DrawCtx) { /* Task 6 */ }
    fn handle_event(&mut self, _ev: &mut Event, _ctx: &mut Context) { /* Task 7 */ }
}
```

> The unused imports (`TrackMask`, `hot_key`, `Key`, `Role`) are consumed in Tasks 6–7. To keep this commit warning-free without an `#[allow]`, **commit Tasks 5–7 as one logical unit**: run only `cargo build` + the Task-5 unit tests here (Step 3), and run the full `-D warnings` clippy gate at Task 7. (Alternative: add `#![allow(unused_imports)]` temporarily and remove it in Task 7.)

- [ ] **Step 2: Wire** — in `src/widgets/mod.rs` add `mod tab_bar;` (after `mod static_text;`) and `pub use tab_bar::TabBar;`. In `src/lib.rs` add `pub use widgets::TabBar;` near the other widget re-exports.
- [ ] **Step 3: Build + unit tests** — `cargo build -p rstv -j 2` then `cargo test -p rstv tab_bar::tests -- --test-threads=2` → PASS.
- [ ] **Step 4: Commit** — `git add src/widgets/tab_bar.rs src/widgets/mod.rs src/lib.rs && git commit -m "feat(widgets): TabBar skeleton — value protocol, find_sel, natural_width"`

---

### Task 6: `TabBar::draw` (corner caps) + snapshots

**Files:** Modify `src/widgets/tab_bar.rs`

- [ ] **Step 1: Implement `draw`** (replace the stub):

```rust
    fn draw(&mut self, ctx: &mut DrawCtx) {
        self.abs_origin = ctx.origin();
        let g = *ctx.glyphs();
        let (norm, norm_hi) = (ctx.style(Role::LabelNormal), ctx.style(Role::LabelNormalShortcut));
        let (act, act_hi) = (ctx.style(Role::LabelLight), ctx.style(Role::LabelLightShortcut));
        for (i, (start, _w)) in self.tab_layout().iter().enumerate() {
            let label = &self.tabs[i];
            if i == self.value {
                ctx.put_char(*start, 0, g.frame_tl, act);
                let lw = ctx.put_cstr(start + 1, 0, label, act, act_hi);
                ctx.put_char(start + 1 + lw, 0, g.frame_tr, act);
            } else {
                ctx.put_cstr(*start, 0, label, norm, norm_hi);
            }
        }
    }
```

- [ ] **Step 2: Snapshot tests** — add to `mod tests`:

```rust
    use crate::backend::{HeadlessBackend, Renderer};
    use crate::screen::Buffer;
    use crate::theme::Theme;
    use crate::view::DrawCtx;

    fn render(active: usize) -> String {
        let theme = Theme::classic_blue();
        let mut tb = TabBar::new(Rect::new(0, 0, 30, 1), &["~P~resets", "~R~GB", "~H~ue/Sat"]);
        tb.value = active;
        let (backend, screen) = HeadlessBackend::new(30, 1);
        let mut r = Renderer::new(Box::new(backend));
        r.render(|buf: &mut Buffer| {
            let b = Rect::new(0, 0, 30, 1);
            tb.draw(&mut DrawCtx::new(buf, &theme, b, b.a));
        });
        screen.snapshot()
    }
    #[test] fn snapshot_tabbar_first_active() { insta::assert_snapshot!(render(0)); }
    #[test] fn snapshot_tabbar_middle_active() { insta::assert_snapshot!(render(1)); }
```

- [ ] **Step 3: Generate + verify** — `INSTA_UPDATE=always cargo test -p rstv tab_bar::tests::snapshot -- --test-threads=2`; read `src/widgets/snapshots/*tab_bar*.snap`: active tab shows `┌Presets┐`, others plain, 1-cell gaps, no blue fill.
- [ ] **Step 4: Re-run stable** — `cargo test -p rstv tab_bar::tests::snapshot -- --test-threads=2` → PASS.
- [ ] **Step 5: Commit** — `git add src/widgets/tab_bar.rs src/widgets/snapshots && git commit -m "feat(widgets): TabBar::draw corner-cap rendering + snapshots"`

---

### Task 7: `TabBar::handle_event` (arrows/hotkey + press-on-release + broadcast)

**Files:** Modify `src/widgets/tab_bar.rs`

- [ ] **Step 1: Failing tests** — add to `mod tests` (helpers mirror `scrollbar.rs` tests):

```rust
    use crate::event::{KeyEvent, KeyModifiers, MouseButtons, MouseEvent, MouseEventFlags, MouseWheel};
    use crate::timer::TimerQueue;
    use crate::view::{Deferred, Group};
    use std::collections::VecDeque;

    fn drive(tb: &mut TabBar, ev: &mut Event, out: &mut VecDeque<Event>) {
        let mut timers = TimerQueue::new();
        let mut deferred: Vec<Deferred> = vec![];
        let mut ctx = Context::new(out, &mut timers, 0, &mut deferred);
        tb.handle_event(ev, &mut ctx);
    }
    fn key(k: Key) -> Event { Event::KeyDown(KeyEvent::new(k, KeyModifiers::default())) }
    fn mouse(kind: fn(MouseEvent) -> Event, x: i32) -> Event {
        kind(MouseEvent { position: Point::new(x, 0),
            buttons: MouseButtons { left: true, ..Default::default() },
            flags: MouseEventFlags::default(), wheel: MouseWheel::None, modifiers: KeyModifiers::default() })
    }

    #[test]
    fn right_left_arrows_cycle_with_wrap() {
        let mut tb = TabBar::new(Rect::new(0, 0, 30, 1), &["A", "B", "C"]);
        let mut out = VecDeque::new();
        let mut e = key(Key::Right); drive(&mut tb, &mut e, &mut out); assert_eq!(tb.selected(), 1);
        tb.value = 2; let mut e = key(Key::Right); drive(&mut tb, &mut e, &mut out); assert_eq!(tb.selected(), 0, "wrap");
        let mut e = key(Key::Left); drive(&mut tb, &mut e, &mut out); assert_eq!(tb.selected(), 2, "wrap back");
    }

    #[test]
    fn hotkey_selects() {
        let mut tb = TabBar::new(Rect::new(0, 0, 30, 1), &["~P~resets", "~R~GB", "~X~term"]);
        let mut out = VecDeque::new();
        let mut e = key(Key::Char('x')); drive(&mut tb, &mut e, &mut out);
        assert_eq!(tb.selected(), 2); assert!(e.is_nothing());
    }

    #[test]
    fn mouse_presses_on_release_over_same_tab() {
        // "A"(active w3) gap "B"(w1 @4) gap "C"(w1 @6). Need a ViewId for the track.
        let mut tb = TabBar::new(Rect::new(0, 0, 30, 1), &["A", "B", "C"]);
        tb.state.id = Some(crate::view::ViewId::next());
        let mut out = VecDeque::new();
        let mut down = mouse(Event::MouseDown, 6); drive(&mut tb, &mut down, &mut out);
        assert_eq!(tb.selected(), 0, "no commit on down");
        let mut up = mouse(Event::MouseUp, 6); drive(&mut tb, &mut up, &mut out);
        assert_eq!(tb.selected(), 2, "commit on release over same tab");
    }

    #[test]
    fn mouse_release_on_different_tab_does_not_commit() {
        let mut tb = TabBar::new(Rect::new(0, 0, 30, 1), &["A", "B", "C"]);
        tb.state.id = Some(crate::view::ViewId::next());
        let mut out = VecDeque::new();
        let mut down = mouse(Event::MouseDown, 6); drive(&mut tb, &mut down, &mut out);
        let mut up = mouse(Event::MouseUp, 1); drive(&mut tb, &mut up, &mut out); // released over tab A
        assert_eq!(tb.selected(), 0, "no commit when release lands elsewhere");
    }

    #[test]
    fn change_broadcasts_with_source_id() {
        let mut tb = TabBar::new(Rect::new(0, 0, 30, 1), &["A", "B", "C"]);
        let mut group = Group::new(Rect::new(0, 0, 30, 1));
        let id = group.insert(Box::new(tb));
        let mut out = VecDeque::new();
        { let mut timers = TimerQueue::new(); let mut deferred: Vec<Deferred> = vec![];
          let mut ctx = Context::new(&mut out, &mut timers, 0, &mut deferred);
          let mut e = key(Key::Right);
          group.find_mut(id).unwrap().handle_event(&mut e, &mut ctx); }
        assert!(out.iter().any(|e| matches!(e,
            Event::Broadcast { command, source } if *command == Command::TAB_BAR_CHANGED && *source == Some(id))));
    }
```

- [ ] **Step 2: Verify fail** — `cargo test -p rstv tab_bar:: -- --test-threads=2` → FAIL (stub does nothing).

- [ ] **Step 3: Implement `handle_event`** (replace the stub):

```rust
    fn handle_event(&mut self, ev: &mut Event, ctx: &mut Context) {
        let n = self.tabs.len();
        if n == 0 { return; }
        match *ev {
            Event::KeyDown(ke) => match ke.key {
                Key::Left  => { self.press((self.value + n - 1) % n, ctx); ev.clear(); }
                Key::Right => { self.press((self.value + 1) % n, ctx); ev.clear(); }
                Key::Char(c) => {
                    let up = c.to_ascii_uppercase();
                    if let Some(i) = self.tabs.iter().position(|l| hot_key(l) == Some(up)) {
                        self.press(i, ctx); ev.clear();
                    }
                }
                _ => {}
            },
            // Press-on-release (mirrors cluster.rs): arm a track on down, commit on up.
            Event::MouseDown(me) => {
                if let Some(i) = self.find_sel(me.position) {
                    self.pressed = Some(i);
                    if let Some(id) = self.state.id() {
                        self.tracking = true;
                        ctx.start_mouse_track(id, self.abs_origin, TrackMask { mouse_move: true, ..Default::default() });
                    } else {
                        self.press(i, ctx); // degenerate: no id, single-shot
                    }
                    ev.clear();
                }
            }
            Event::MouseUp(me) if self.tracking => {
                self.tracking = false;
                if let (Some(p), Some(i)) = (self.pressed.take(), self.find_sel(me.position)) {
                    if p == i { self.press(i, ctx); }
                }
                ev.clear();
            }
            _ => {}
        }
    }
```

- [ ] **Step 4: Pass** — `cargo test -p rstv tab_bar:: -- --test-threads=2` → PASS.
- [ ] **Step 5: Full gate (clears Task-5 deferred warnings) + commit**
```bash
cargo test --workspace -- --test-threads=2 && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
git add src/widgets/tab_bar.rs
git commit -m "feat(widgets): TabBar::handle_event — arrows/hotkey + press-on-release + broadcast"
```

---

# PIECE 2b — `PageStack` + the pump broker

### Task 8: `PageStack` widget

**Files:** Create `src/widgets/page_stack.rs`; modify `src/widgets/mod.rs`, `src/lib.rs`

- [ ] **Step 1: Create `src/widgets/page_stack.rs`**

```rust
//! A content multiplexer: a group of child "page" Views of which exactly one is
//! visible at a time. Paired with a [`TabBar`](crate::widgets::TabBar): the bar
//! broadcasts [`Command::TAB_BAR_CHANGED`], this stack (bound to the bar's id)
//! switches the visible page through the pump broker — mirroring how a
//! [`Scroller`](crate::widgets::Scroller) reacts to its `ScrollBar`.
//!
//! # Turbo Vision heritage
//! None — classic Turbo Vision has no notebook/tab-page container, only `TGroup`
//! + `sfVisible`/`show()`/`hide()`. `PageStack` packages exactly that.

use crate::command::Command;
use crate::event::Event;
use crate::view::{Context, Group, Rect, View, ViewId};

/// A stack of page Views showing one at a time. See the [module docs](self).
pub struct PageStack {
    group: Group,
    pages: Vec<ViewId>,
    active: usize,
    /// The bound TabBar id; a `TAB_BAR_CHANGED` from it triggers a page switch.
    tab_bar: Option<ViewId>,
}

impl PageStack {
    /// Empty stack at `bounds`.
    pub fn new(bounds: Rect) -> Self {
        PageStack { group: Group::new(bounds), pages: Vec::new(), active: 0, tab_bar: None }
    }

    /// Bind the `TabBar` whose broadcasts drive this stack.
    pub fn bind_tab_bar(&mut self, id: ViewId) { self.tab_bar = Some(id); }

    /// Insert a page; returns its id. All but the first page start hidden.
    /// Lay each page at the stack's full local extent before inserting.
    pub fn insert_page(&mut self, view: Box<dyn View>) -> ViewId {
        let id = self.group.insert(view);
        self.pages.push(id);
        if self.pages.len() > 1 {
            if let Some(v) = self.group.child_mut(id) { v.state_mut().state.visible = false; }
        }
        id
    }

    /// The active page index.
    pub fn active(&self) -> usize { self.active }

    /// Show page `idx`, hide the rest, move focus to it.
    pub fn set_active(&mut self, idx: usize, ctx: &mut Context) {
        if idx >= self.pages.len() { return; }
        for (i, &pid) in self.pages.clone().iter().enumerate() {
            self.group.set_visible_descendant(pid, i == idx, ctx);
        }
        self.group.focus_child(self.pages[idx], ctx);
        self.active = idx;
    }
}

#[crate::delegate(to = group, skip(as_any_mut, handle_event))]
impl View for PageStack {
    fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> { Some(self) }

    /// React to the bound `TabBar`'s broadcast by queuing a pump sync; then route
    /// the event into the group as usual.
    fn handle_event(&mut self, ev: &mut Event, ctx: &mut Context) {
        if let Event::Broadcast { command, source } = *ev {
            if command == Command::TAB_BAR_CHANGED && source.is_some() && source == self.tab_bar {
                if let Some(id) = self.group.state().id() {
                    ctx.request_sync_page_stack(id, source.unwrap());
                }
            }
        }
        self.group.handle_event(ev, ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view::{Deferred, ViewState, DrawCtx};
    use crate::timer::TimerQueue;
    use std::collections::VecDeque;

    // Minimal selectable page stub.
    struct Page { st: ViewState }
    impl Page {
        fn boxed(b: Rect) -> Box<dyn View> {
            let mut st = ViewState::new(b); st.options.selectable = true;
            Box::new(Page { st })
        }
    }
    impl View for Page {
        fn state(&self) -> &ViewState { &self.st }
        fn state_mut(&mut self) -> &mut ViewState { &mut self.st }
        fn draw(&mut self, _c: &mut DrawCtx) {}
    }

    fn ctx_run<R>(f: impl FnOnce(&mut Context) -> R) -> R {
        let mut out = VecDeque::new(); let mut timers = TimerQueue::new(); let mut d: Vec<Deferred> = vec![];
        let mut ctx = Context::new(&mut out, &mut timers, 0, &mut d); f(&mut ctx)
    }

    #[test]
    fn first_page_visible_rest_hidden_after_insert() {
        let mut ps = PageStack::new(Rect::new(0, 0, 20, 10));
        let p0 = ps.insert_page(Page::boxed(Rect::new(0, 0, 20, 10)));
        let p1 = ps.insert_page(Page::boxed(Rect::new(0, 0, 20, 10)));
        assert!(ps.group.child_mut(p0).unwrap().state().state.visible);
        assert!(!ps.group.child_mut(p1).unwrap().state().state.visible);
    }

    #[test]
    fn set_active_shows_one_hides_rest() {
        let mut ps = PageStack::new(Rect::new(0, 0, 20, 10));
        let p0 = ps.insert_page(Page::boxed(Rect::new(0, 0, 20, 10)));
        let p1 = ps.insert_page(Page::boxed(Rect::new(0, 0, 20, 10)));
        ctx_run(|ctx| ps.set_active(1, ctx));
        assert_eq!(ps.active(), 1);
        assert!(!ps.group.child_mut(p0).unwrap().state().state.visible);
        assert!(ps.group.child_mut(p1).unwrap().state().state.visible);
    }
}
```

> `set_active` clones `self.pages` to avoid borrowing `self.pages` and `self.group` simultaneously. If the borrow checker still complains, copy the ids into a local `Vec<ViewId>` first.

- [ ] **Step 2: Wire** — `src/widgets/mod.rs`: `mod page_stack;` + `pub use page_stack::PageStack;`. `src/lib.rs`: `pub use widgets::PageStack;`.
- [ ] **Step 3: Build + tests** — `cargo build -p rstv -j 2` then `cargo test -p rstv page_stack:: -- --test-threads=2` → PASS.

> `request_sync_page_stack` does not exist yet (Task 9) — but `PageStack::handle_event` references it, so this won't compile. **Sequence Task 9 immediately and commit 8+9 together**, OR add a temporary stub `pub fn request_sync_page_stack(&mut self, _: ViewId, _: ViewId) {}` to `Context` in this task and flesh it out in Task 9. Use the latter so Step 3 compiles in isolation.

- [ ] **Step 4: Commit** — `git add src/widgets/page_stack.rs src/widgets/mod.rs src/lib.rs && git commit -m "feat(widgets): PageStack content multiplexer (show-one/hide-rest)"`

---

### Task 9: The pump broker (`Deferred::PageStackSync` + `Context` method + pump arm) + integration test

**Files:** Modify `src/view/context.rs`, `src/app/program.rs`

- [ ] **Step 1: Add the `Deferred` variant** — in `src/view/context.rs`, inside `pub enum Deferred` (beside `SyncScrollerDelta`):

```rust
    /// Read-broker for a [`PageStack`](crate::widgets::PageStack): on a
    /// `TAB_BAR_CHANGED` broadcast, the pump resolves `tab_bar`, reads its
    /// `value()` (→ `FieldValue::Int` index), downcasts `page_stack` to
    /// `PageStack`, and calls `set_active(index, &mut ctx)`. Mirrors
    /// [`SyncScrollerDelta`](Deferred::SyncScrollerDelta).
    PageStackSync { page_stack: ViewId, tab_bar: ViewId },
```

- [ ] **Step 2: Add the `Context` method** (replace the Task-8 stub if you added one) — beside `request_sync_scroller_delta`:

```rust
    /// Queue a [`PageStack`](crate::widgets::PageStack) sync (see
    /// [`Deferred::PageStackSync`]). Called by `PageStack::handle_event` on a
    /// `TAB_BAR_CHANGED` broadcast from its bound bar.
    pub fn request_sync_page_stack(&mut self, page_stack: ViewId, tab_bar: ViewId) {
        self.deferred.push(Deferred::PageStackSync { page_stack, tab_bar });
    }
```

- [ ] **Step 3: Add the pump arm** — in `src/app/program.rs`, beside the `Deferred::SyncScrollerDelta` arm (~line 1992), add:

```rust
                Deferred::PageStackSync { page_stack, tab_bar } => {
                    use crate::widgets::PageStack;
                    let idx = group
                        .find_mut(tab_bar)
                        .and_then(|v| v.value())
                        .and_then(field_int)
                        .unwrap_or(0);
                    if let Some(ps) = group
                        .find_mut(page_stack)
                        .and_then(|v| v.as_any_mut())
                        .and_then(|a| a.downcast_mut::<PageStack>())
                    {
                        ps.set_active(idx.max(0) as usize, &mut ctx);
                    }
                }
```

> `field_int` is the same helper used by the `SyncScrollerDelta` arm — confirm it is in scope at that site (it is). `&mut ctx` is available here (the `ScrollBarSetParams` arm already uses it).

- [ ] **Step 4: Integration test** — add a program-level test mirroring the existing scroller-sync test (find it: grep `program.rs` for `SyncScrollerDelta` / a scroller test around line 5085). The test should:
  1. Build a `Program` (or the minimal harness that test uses) with a `Group` containing a `TabBar` and a `PageStack` (2 `Page` stubs), `bind_tab_bar`.
  2. Drive a tab change on the `TabBar` (e.g. resolve it by id, send `Key::Right`) so it broadcasts.
  3. Pump once (or invoke the deferred-drain path the scroller test uses).
  4. Assert the `PageStack`'s active index switched to 1 and its page-1 child is visible, page-0 hidden.

  Model it line-for-line on the scroller-sync test; reuse its `Program` construction. If a full `Program` test is heavy, an acceptable lighter test: manually push `Deferred::PageStackSync` into a `Vec`, run the broker arm's logic against a hand-built `Group` (extract no new code — just assert `PageStack::set_active` + `value()` interplay), and separately assert (already done in Task 8 via `page_stack::tests`) that `handle_event` queues the deferred on the broadcast. Prefer the real program-level test if the harness is reusable.

- [ ] **Step 5: Gate + commit**
```bash
cargo test --workspace -- --test-threads=2 && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
git add src/view/context.rs src/app/program.rs src/widgets/page_stack.rs
git commit -m "feat(view,app): PageStack pump broker (Deferred::PageStackSync) + TabBar↔PageStack integration"
```

---

# PIECE 3 — Rebuild the color picker on `TabBar` + `PageStack`

**Context the implementer needs (from the codebase):**
- `ColorModel` (`src/dialog/colorpick/model.rs`) is `Clone + Copy`, ~16 bytes; methods `new`/`set_color`/`set_rgb`/`set_indexed`/`set_hsv`.
- The `Surface` trait (`mod.rs`): `draw(&self, ctx, body: Rect, m: &ColorModel)`, `handle_event(&mut self, ev, body: Rect, m: &mut ColorModel, ctx)`, `drag_region_at(&self, p, body) -> Option<ColorDragRegion>`, `apply_drag(&mut self, region, p, body, m: &mut ColorModel)`.
- Surfaces: `PresetsSurface`/`Xterm256Surface` (click/key only), `RgbSurface`/`PlaneSurface` (draggable). `PresetsSurface::new(&ColorModel)` and `Xterm256Surface::new(&ColorModel)` seed from the model; `RgbSurface::new()`/`PlaneSurface::new()` take no model.
- Current drag plumbing to RETIRE: `drag.rs` (`ColorDragCapture`, `ColorDragRegion` stays — it's the region enum), `Deferred::ColorPickerDrag`, `Context::request_color_drag`, the pump arm (`program.rs` ~2455), and `ColorPicker::apply_drag`. **Keep `ColorDragRegion`** (the surfaces return/consume it).
- Public API to preserve: `ColorPicker::new(bounds, initial) -> Self`, `color() -> Color`, `select_tab(Tab)`, `as_any_mut`. Consumers: `Program::color_dialog` (`program.rs:1087`), `open_color_dialog_for_role` (`program.rs:2766`), `examples/{tvdemo,gallery,hello}.rs`.

### Task 10: Shared `Rc<RefCell<ColorModel>>` + generic `SurfacePage<S>` View; retire `drag.rs`

**Files:** modify `src/dialog/colorpick/model.rs` (or `mod.rs`) for a shared-model alias; create `src/dialog/colorpick/page.rs`; delete `src/dialog/colorpick/drag.rs` (move `ColorDragRegion` into `page.rs` or keep a slim `drag.rs` with only the region enum); modify `src/dialog/colorpick/mod.rs`, `src/view/context.rs`, `src/app/program.rs` (remove the retired drag broker).

- [ ] **Step 1: Keep only `ColorDragRegion`.** In `drag.rs`, delete `ColorDragCapture` and its `impl`. Keep the `ColorDragRegion` enum (rename the file's module role to "drag regions"). Remove `Context::request_color_drag` (`context.rs`), `Deferred::ColorPickerDrag` (`context.rs`), and its pump arm (`program.rs` ~2455). Remove `ColorPicker::apply_drag`. Build will break at the old picker — that's fine; the picker is rewritten in Task 12. To keep the tree compiling between tasks, temporarily `#[allow(dead_code)]` `ColorDragRegion` if needed.

- [ ] **Step 2: Define the shared-model alias** — in `src/dialog/colorpick/mod.rs`:

```rust
use std::cell::RefCell;
use std::rc::Rc;
/// The picker's surfaces and info column share one model.
pub(crate) type SharedModel = Rc<RefCell<model::ColorModel>>;
```

- [ ] **Step 3: Create `src/dialog/colorpick/page.rs`** — the generic page wrapper that turns any `Surface` into a real View, with track-capture drag:

```rust
//! A page View wrapping one picker [`Surface`](super::Surface): it owns the
//! shared [`SharedModel`](super::SharedModel) and bridges the surface's
//! `body`-relative draw/handle to the View trait. Draggable surfaces self-drive
//! via the standard mouse-track capture (the `ScrollBar` thumb-drag pattern).

use super::drag::ColorDragRegion;
use super::{SharedModel, Surface};
use crate::capture::TrackMask;
use crate::event::Event;
use crate::view::{Context, DrawCtx, Point, Rect, View, ViewState};

pub(crate) struct SurfacePage<S: Surface> {
    state: ViewState,
    surface: S,
    model: SharedModel,
    abs_origin: Point,
    tracking: bool,
    region: Option<ColorDragRegion>,
}

impl<S: Surface> SurfacePage<S> {
    pub(crate) fn new(bounds: Rect, surface: S, model: SharedModel) -> Self {
        SurfacePage { state: ViewState::new(bounds), surface, model, abs_origin: bounds.a, tracking: false, region: None }
    }
    fn body(&self) -> Rect { let s = self.state.size; Rect::new(0, 0, s.x, s.y) }
}

impl<S: Surface + 'static> View for SurfacePage<S> {
    fn state(&self) -> &ViewState { &self.state }
    fn state_mut(&mut self) -> &mut ViewState { &mut self.state }
    fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> { Some(self) }

    fn draw(&mut self, ctx: &mut DrawCtx) {
        self.abs_origin = ctx.origin();
        let body = self.body();
        let m = self.model.borrow();
        self.surface.draw(ctx, body, &m);
    }

    fn handle_event(&mut self, ev: &mut Event, ctx: &mut Context) {
        let body = self.body();
        match *ev {
            Event::MouseDown(me) => {
                if let Some(region) = self.surface.drag_region_at(me.position, body) {
                    self.region = Some(region);
                    { let mut m = self.model.borrow_mut(); self.surface.apply_drag(region, me.position, body, &mut m); }
                    if let Some(id) = self.state.id() {
                        self.tracking = true;
                        ctx.start_mouse_track(id, self.abs_origin, TrackMask { mouse_move: true, ..Default::default() });
                    }
                    ev.clear();
                    return;
                }
                let mut m = self.model.borrow_mut();
                self.surface.handle_event(ev, body, &mut m, ctx);
            }
            Event::MouseMove(me) if self.tracking => {
                if let Some(region) = self.region {
                    let mut m = self.model.borrow_mut();
                    self.surface.apply_drag(region, me.position, body, &mut m);
                }
                ev.clear();
            }
            Event::MouseUp(_) if self.tracking => { self.tracking = false; self.region = None; ev.clear(); }
            _ => {
                let mut m = self.model.borrow_mut();
                self.surface.handle_event(ev, body, &mut m, ctx);
            }
        }
    }
}
```

> The track capture localizes delivered coordinates via `abs_origin` (same as `ScrollBar`), so `me.position` in the `MouseMove` arm is already page-local and matches what `apply_drag` expects (`body.a == (0,0)`).

- [ ] **Step 4: Tests** — add a `#[cfg(test)] mod tests` in `page.rs`:
  - Build `SurfacePage::new(Rect::new(0,0,22,18), RgbSurface::new(), Rc::new(RefCell::new(ColorModel::new(Color::Rgb(30,144,255)))))`, stamp an id, send `MouseDown` on a bar row → assert the shared model's color changed (drag applied on down). Then a `MouseMove` along the bar → assert it scrubs.
  - Build `SurfacePage<PresetsSurface>`, send a click that selects a preset → assert the model color changed (delegated to the surface's `handle_event`).
  - Migrate the per-surface standalone snapshots (`presets.rs`/`rgb.rs`/`plane.rs`/`xterm256.rs` `snapshot_*` tests) so they render through a `SurfacePage` if convenient, OR leave the surface-level snapshots as-is (they test the `Surface::draw`, which is unchanged). Prefer leaving them — less churn; they still pass because `Surface::draw` signature is unchanged.

- [ ] **Step 5: Build + tests** — `cargo test -p rstv colorpick::page -- --test-threads=2` → PASS. (The old `ColorPicker` in `mod.rs` is broken until Task 12; if the crate won't build, gate this task's verification on `cargo test -p rstv colorpick::page` after Task 12, and commit Tasks 10+12 together. Recommended: **do Tasks 10–12 on one working copy and commit once at Task 12** — the picker rewrite is one atomic change.)

- [ ] **Step 6:** (Deferred commit — see Step 5; commit at Task 12.)

---

### Task 11: `InfoColumn` View

**Files:** create `src/dialog/colorpick/info.rs`; modify `mod.rs`

- [ ] **Step 1: Create `src/dialog/colorpick/info.rs`** — extract the current `draw_info_column` logic into a View that reads the shared model + the `old` color:

```rust
//! The always-visible info column: old/new swatches + the variant readout.
//! Reads the shared model; never switches with the tabs.

use super::model::color_to_display_rgb;
use super::SharedModel;
use crate::color::{Color, Style};
use crate::theme::Role;
use crate::view::{Context, DrawCtx, Event, Rect, View, ViewState};

pub(crate) struct InfoColumn {
    state: ViewState,
    model: SharedModel,
    old: Color,
}

impl InfoColumn {
    pub(crate) fn new(bounds: Rect, model: SharedModel, old: Color) -> Self {
        InfoColumn { state: ViewState::new(bounds), model, old }
    }
}

impl View for InfoColumn {
    fn state(&self) -> &ViewState { &self.state }
    fn state_mut(&mut self) -> &mut ViewState { &mut self.state }
    fn draw(&mut self, ctx: &mut DrawCtx) {
        // Port draw_info_column verbatim, but: (a) coords are column-local (0-based),
        // (b) the chrome `normal` style is Role::StaticText (gray), NOT ScrollerNormal.
        let normal = ctx.style(Role::StaticText);
        let cur = self.model.borrow().color;
        // ... old/new swatch fills (keep the per-swatch RGB Style) + variant readout,
        //     translated to this view's local origin (was INFO_COL_X-relative → now 0-relative).
        let _ = (normal, cur); // replace with the ported body
    }
    fn handle_event(&mut self, _ev: &mut Event, _ctx: &mut Context) {} // passive
}
```

> Port the body of the existing `ColorPicker::draw_info_column` (`mod.rs:175-221`): the "Old:"/"New:" labels, the two swatch `fill`s (keep their per-swatch RGB `Style`), and the variant readout. Change every `INFO_COL_X + k` to `k` (the column is now its own view at local 0), and change the chrome role from `Role::ScrollerNormal` to `Role::StaticText`.

- [ ] **Step 2:** Add `mod info; mod page;` to `mod.rs`. (Verification with Task 12.)

---

### Task 12: Rebuild `ColorPicker` as a `Group` container

**Files:** modify `src/dialog/colorpick/mod.rs` (the bulk of the rewrite); `model.rs` if needed for `color_to_display_rgb` visibility.

- [ ] **Step 1: Replace the `ColorPicker` struct + impl.** New shape:

```rust
pub struct ColorPicker {
    group: crate::view::Group,
    tab_bar_id: ViewId,
    page_stack_id: ViewId,
    model: SharedModel,
}
```

Layout in `new(bounds, initial)`:
- `let model = Rc::new(RefCell::new(ColorModel::new(initial)));`
- local extent `ext = (0,0,w,h)`; `INFO_COL_W` = the info column width (port `INFO_COL_X`/the current 38..end span).
- Insert a `TabBar::new(Rect::new(0,0,w-INFO_COL_W,1), &Tab::labels())` → `tab_bar_id`.
- Build a `PageStack::new(Rect::new(0,1,w-INFO_COL_W,h))`; `insert_page` four `SurfacePage`s wrapping `PresetsSurface::new(&model.borrow())`, `RgbSurface::new()`, `PlaneSurface::new()`, `Xterm256Surface::new(&model.borrow())` (each cloning `model`), each laid at the PageStack's local extent `(0,0,w-INFO_COL_W,h-1)`; `page_stack.bind_tab_bar(tab_bar_id)`; insert → `page_stack_id`.
- Insert `InfoColumn::new(Rect::new(w-INFO_COL_W,0,w,h), model.clone(), initial)`.
- Set `options.selectable = true`, `first_click = true` on the group (port from the old ctor).

Add `Tab::labels() -> [&'static str;4]`, `Tab::from_index`, and update `Tab::label` renames (`~H~ue/Sat`, `~X~term`; keep `~P~resets`, `~R~GB`).

Preserved API:
```rust
pub fn color(&self) -> Color { self.model.borrow().color }
pub fn select_tab(&mut self, tab: Tab) {
    // set the TabBar value (no broadcast) AND switch the page via the broker on next pump.
    if let Some(tb) = self.group.child_mut(self.tab_bar_id)
        .and_then(|v| v.as_any_mut()).and_then(|a| a.downcast_mut::<crate::widgets::TabBar>()) {
        crate::view::View::set_value(tb, crate::data::FieldValue::Int(tab.idx() as i32));
    }
    // Also reflect immediately in the PageStack so a pre-run select_tab shows the right page.
    // (select_tab is called before the modal loop; do a direct ctx-less visibility set or
    //  defer to first handle_event. Simplest: store a `pending_tab` and apply on first draw/awaken.)
}
```

> **`select_tab` timing:** it's called before the modal loop (examples call it right after `new`), so there's no `Context`. Options: (a) store `pending_tab: Option<usize>` and apply it in `awaken`/first `handle_event` via the broker (`ctx.request_sync_page_stack`); or (b) set the TabBar value now and directly flip page visibility via `group.child_mut(page_stack).downcast::<PageStack>()` + a ctx-less visibility setter. Use (a) — cleanest: stash `pending_tab`, and in the group's first `handle_event` (or an `awaken` override) set the TabBar value + `ctx.request_sync_page_stack(page_stack_id, tab_bar_id)`.

- [ ] **Step 2: View impl** — `#[crate::delegate(to = group, skip(as_any_mut, handle_event, value, set_value))]`:
  - `as_any_mut → Some(self)` (the pump downcasts to `ColorPicker` for nothing now — but `color_dialog` downcasts to read `color()`; keep it).
  - `handle_event`: apply `pending_tab` once; intercept **Ctrl+Left/Right** and **Alt+hotkey** → set the TabBar value (downcast child) + `ctx.request_sync_page_stack(page_stack_id, tab_bar_id)` + `ev.clear()`; otherwise `self.group.handle_event(ev, ctx)` (mouse clicks on the tab strip and plain keys to the focused page route normally; a tab click broadcasts → the PageStack broker switches the page).
  - `value`/`set_value`: keep whatever the old picker did (it had `as_any_mut` for downcast; it likely didn't expose value — check and preserve).

- [ ] **Step 3: Update the picker unit tests** in `mod.rs` (`view_tests`): the old tests poke `p.active` and Ctrl+Right cycling. Rewrite them against the new shape:
  - `color_returns_seed` — unchanged (`p.color()`).
  - tab cycling — drive Ctrl+Right through `handle_event` and assert the TabBar child's `selected()` advanced (resolve the child by `tab_bar_id`), and that a `Deferred::PageStackSync` was queued. (Currency/visibility switching is covered by the Task-9 integration test + Task-8 PageStack tests.)
  - `switching_tab_does_not_change_color` — drive a tab change, assert `p.color()` unchanged.
  - Delete tests that assumed the monolithic surfaces (`plain_tab_is_left_unhandled` etc. — re-express against the new routing or drop if no longer meaningful).

- [ ] **Step 4: Update the picker snapshot** — `snap_tests::snapshot_picker_presets` renders a 40×12 `ColorPicker`. Regenerate (`INSTA_UPDATE=always …`) and **hand-verify**: tab row shows `┌Presets┐ RGB Hue/Sat Xterm`, gray chrome (no blue), gray info column. Remove the stale `snapshot_picker_rgb.snap` if its (skipped) test is gone.

- [ ] **Step 5: Build + full gate** (this is the atomic commit point for Tasks 10–12):
```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
cargo test --workspace -- --test-threads=2
cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
```

- [ ] **Step 6: Commit**
```bash
git add src/dialog/colorpick src/view/context.rs src/app/program.rs
git commit -m "refit(colorpick): rebuild as TabBar+PageStack+InfoColumn group; surfaces→page Views over Rc<RefCell<ColorModel>>; retire bespoke drag broker; gray chrome; rename tabs"
```

---

### Task 13: Verify callers + tvdemo re-lay + demo regen

**Files:** modify `examples/tvdemo.rs`; verify `program.rs` (`color_dialog`, `open_color_dialog_for_role`), `examples/{gallery,hello}.rs`.

- [ ] **Step 1: Verify the embedding callers still compile/behave.** `color_dialog` and `open_color_dialog_for_role` build a dialog with `ColorPicker::new(...)` + downcast to `color()` on OK — unchanged API, should just work. `examples/gallery.rs` + `tvdemo.rs` call `ColorPicker::new` + `select_tab(Tab::Plane)`. Build all examples: `cargo build --examples -j 2`.

- [ ] **Step 2: Re-lay `color_window()`** in `examples/tvdemo.rs` (was ~lines 1623-1644):

```rust
fn color_window() -> Box<dyn View> {
    let mut dlg = Dialog::new(Rect::new(6, 1, 68, 24), Some("Select Color".to_string()));
    let mut picker = ColorPicker::new(Rect::new(2, 2, 60, 19), Color::Rgb(30, 144, 255)); // one row shorter
    picker.select_tab(Tab::Plane);
    dlg.insert_child(Box::new(picker));
    dlg.button_row(
        &[("~O~K", Command::OK, ButtonFlags { default: true, ..ButtonFlags::new() }),
          ("~C~ancel", Command::CANCEL, ButtonFlags::new())],
        ButtonRowAlign::Right);
    Box::new(dlg)
}
```

Add `ButtonRowAlign` to the `use rstv::{…}` import (~line 21).

- [ ] **Step 3: Visual verify** — drive the demo in tmux (single Bash call: launch + open the color window + capture, per the tmux-sandbox rule). Confirm: no blue chrome, corner-cap active tab, renamed tabs (`Hue/Sat`, `Xterm`), blank line above OK/Cancel, and that arrow-dragging the plane still scrubs the color.

- [ ] **Step 4: Regenerate doc captures if any.** Grep `xtask/` and `docs/book` for committed color-picker screenshots (`grep -rl -i color xtask docs/book/src`); if the gallery includes the picker, run the gallery/screens regen (`cargo xtask screens` or the gallery capture per HANDOVER's "Verifying docs edits") and review the regenerated artifact.

- [ ] **Step 5: Gate + commit**
```bash
cargo test --workspace -- --test-threads=2 && cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --all --check
git add examples/tvdemo.rs   # + any regenerated captures
git commit -m "demo(tvdemo): re-lay color picker via button_row; regen captures"
```

---

## Wrap-up (after Task 13)

- [ ] **`docs/IMPLEMENTATION-LOG.md`** — prepend a section covering: the dialog layout guide + constants + `button_row`; the cluster-shaped `TabBar`; the `PageStack` + `Deferred::PageStackSync` broker seam; the color-picker rebuild (surfaces→page Views over `Rc<RefCell<ColorModel>>`, retired `drag.rs`). Note the new cross-cutting seam: **`PageStackSync` is the TabBar↔PageStack broker, the third instance of the D3 sibling-broker pattern after scroller↔scrollbar and listviewer↔scrollbar.**
- [ ] **`docs/HANDOVER.md`** — move the "In flight: dialog-design-guide" block to landed; update HEAD + test count; record the new `TabBar`/`PageStack` widgets and the `PageStackSync` seam.
- [ ] **Final integrated gate** (canonical target dir): `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`.
- [ ] Use `superpowers:finishing-a-development-branch` to decide merge vs PR to `main`.

---

## Self-Review (author check against the updated spec)

- **Piece 1** (guide + constants + `button_row`) → Tasks 1–4. ✅
- **Piece 2a** (`TabBar` cluster-shaped: value/sel≡value, press/find_sel, press-on-release, broadcast, corner-cap draw, natural_width, snapshots) → Tasks 5–7. ✅
- **Piece 2b** (`PageStack` show-one/hide-rest + `set_active` currency; `Command::TAB_BAR_CHANGED`, `Deferred::PageStackSync`, `request_sync_page_stack`, pump arm; integration test) → Tasks 8–9. ✅
- **Piece 3** (picker as `TabBar`+`PageStack`+`InfoColumn` group; surfaces→`SurfacePage<S>` over `Rc<RefCell<ColorModel>>`; track-capture drag; retire `drag.rs`; gray chrome; rename `Hue/Sat`+`Xterm`; preserve `new`/`color`/`select_tab`/`as_any_mut`; tvdemo re-lay) → Tasks 10–13. ✅
- **Broker mirrors scroller↔scrollbar** — `Deferred::PageStackSync` arm reads `tab_bar.value()` via `find_mut`+`value()`+`field_int`, downcasts `PageStack`, calls `set_active(idx, &mut ctx)`. Matches the `SyncScrollerDelta` arm exactly. ✅
- **No new `View` trait method** anywhere (only existing-method overrides) → no `rstv-macros/src/specs.rs` forwarder. ✅
- **Verified APIs** (from investigation): `set_value` takes no `ctx`; `Point{x,y}` const literal compiles; `Group`/`set_visible_descendant`/`focus_child`/`as_any_mut`/`field_int` signatures; `ctx.start_mouse_track(id, abs_origin, TrackMask)` localizes coords; `ColorModel` is `Copy`; `ColorDragRegion` is the region enum to keep.
- **Type consistency:** `selected()`, `press()`, `find_sel()`, `natural_width()`, `tab_layout()`, `set_active()`, `insert_page()`, `bind_tab_bar()`, `SurfacePage<S>`, `SharedModel`, `Deferred::PageStackSync`, `request_sync_page_stack` — used consistently across tasks.
- **Atomicity flagged:** Tasks 10–12 are one working change (the picker won't compile mid-way) → commit once at Task 12; Tasks 8–9 likewise (the `request_sync_page_stack` stub bridges Task 8's isolated build). Both called out inline.
- **Open items to resolve during execution** (flagged inline): `select_tab` pre-modal timing (use `pending_tab` applied on first event); whether per-surface standalone snapshots migrate to `SurfacePage` (recommend: leave them — `Surface::draw` is unchanged); whether a full `Program` integration test or the lighter split test is used in Task 9 (prefer the real one if the harness is reusable).
