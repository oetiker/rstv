# Frameless Fullscreen Windows Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a per-window `Fullscreen` property (`Off` / `Desktop` / `Screen`) that hides the window frame and fills the desktop (mode a), and additionally covers the menu row by collapsing the `MenuBar` to a `⋮` kebab at the top-right (mode b).

**Architecture:** A new `Window::set_fullscreen` toggles the frame border **inline** (the only path that reaches the `Frame`, since `Window::as_any_mut` forwards to its inner `Group`) and emits one deferred op `Deferred::SetFullscreen { window, mode }`. The **pump** applies all cross-tree layout (menubar collapse + bounds, desktop bounds, window re-fit) through the `View` trait — no downcast — and tracks the active fullscreen window in loop-owned state for resize re-fit and removal-restore. The collapsed menubar opens a corner `popup_menu` on activation rather than reclaiming width (the menu session freezes its bounds at activation, so reclaiming width is impossible).

**Tech Stack:** Rust (Cargo workspace `tvision-rs` + `tvision-rs-macros`), `insta` snapshot tests on the `HeadlessBackend`.

## Global Constraints

- `export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target` before any cargo command (artifacts land there, not `./target`).
- Build/test on **at most 4 cores**: prefix cargo with `CARGO_BUILD_JOBS=4` and pass `-- --test-threads=4` to tests.
- Verification gate for every task (all must pass):
  - `cargo test --workspace -j4 -- --test-threads=4`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo fmt --all --check`
- English for all code/comments/identifiers.
- Roll `CHANGELOG.md` (`## Unreleased` → `### New`) for the user-visible feature (Task 6).
- Snapshot tests: a new `insta::assert_snapshot!` produces a `.snap.new`; **review it** (`cargo insta review` or read the file) and rename to `.snap` before committing — eyeball the whole frame, not just the asserted glyph (snapshot-at-origin lesson).
- Commit messages end with: `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`
- Faithful seam reuse: the design note is `docs/design/fullscreen-window.md` — consult it for rationale; this plan is the line-level recipe.

---

### Task 1: `Fullscreen` enum + `Window` state plumbing

**Files:**
- Modify: `src/window/window.rs` (struct `Window` ~129, `Window::new` ~173, accessors block)
- Modify: `src/window/mod.rs` (re-export)
- Modify: `src/lib.rs` (crate-root re-export, if `Window` is re-exported there)
- Test: `src/window/window.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Produces: `pub enum Fullscreen { Off, Desktop, Screen }` with `impl Fullscreen { pub fn next(self) -> Self }`; `Window` field `fullscreen: Fullscreen`; `pub fn Window::fullscreen(&self) -> Fullscreen`. Re-exported as `crate::window::Fullscreen` (consumed by Tasks 4–6 and `context.rs` in Task 5).

- [ ] **Step 1: Write the failing test** — append to the `tests` module in `src/window/window.rs`:

```rust
#[test]
fn fullscreen_defaults_off_and_cycles() {
    use crate::window::Fullscreen;
    let win = Window::new(Rect::new(0, 0, 20, 8), Some("W".into()), 0);
    assert_eq!(win.fullscreen(), Fullscreen::Off, "new window is not fullscreen");
    assert_eq!(Fullscreen::Off.next(), Fullscreen::Desktop);
    assert_eq!(Fullscreen::Desktop.next(), Fullscreen::Screen);
    assert_eq!(Fullscreen::Screen.next(), Fullscreen::Off);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target CARGO_BUILD_JOBS=4 cargo test --workspace -j4 fullscreen_defaults_off_and_cycles -- --test-threads=4`
Expected: FAIL to compile — `Fullscreen` and `Window::fullscreen` do not exist.

- [ ] **Step 3: Add the enum.** Near the top of `src/window/window.rs` (after the existing `use`/`WindowPalette` declarations), add:

```rust
/// Whether (and how) a window fills the screen with no frame border.
///
/// `Off` is a normal framed window; `Desktop` hides the border and fills the
/// desktop area; `Screen` additionally covers the menu row (the menu bar
/// collapses to a `⋮` kebab at the top-right). Per-window property, driven by
/// [`Window::set_fullscreen`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Fullscreen {
    /// Normal framed window.
    #[default]
    Off,
    /// Frameless, fills the desktop; menu bar and status line unchanged.
    Desktop,
    /// Frameless, also covers the menu row; the menu bar collapses to `⋮`.
    Screen,
}

impl Fullscreen {
    /// The next state in the `Off → Desktop → Screen → Off` cycle (the
    /// `Command::FULLSCREEN` convenience cycler).
    pub fn next(self) -> Self {
        match self {
            Fullscreen::Off => Fullscreen::Desktop,
            Fullscreen::Desktop => Fullscreen::Screen,
            Fullscreen::Screen => Fullscreen::Off,
        }
    }
}
```

- [ ] **Step 4: Add the field.** In `struct Window` (~129) add after `title`:

```rust
    /// Whether the window is frameless-fullscreen, and in which mode. Driven by
    /// [`set_fullscreen`](Self::set_fullscreen); read by the `Command::FULLSCREEN`
    /// cycler and [`client_rect`](Self::client_rect).
    fullscreen: Fullscreen,
```

In `Window::new` (~208), add `fullscreen: Fullscreen::Off,` to the returned `Window { ... }` literal.

- [ ] **Step 5: Add the getter.** In the accessors block (near `flags()` ~238) add:

```rust
    /// The window's current [`Fullscreen`] state.
    pub fn fullscreen(&self) -> Fullscreen {
        self.fullscreen
    }
```

- [ ] **Step 6: Re-export.** In `src/window/mod.rs` find the `pub use window::Window;` (or `pub use window::{...}`) line and add `Fullscreen`:

Run to locate: `grep -n "pub use window" src/window/mod.rs`
Edit so it reads e.g. `pub use window::{Fullscreen, Window};`. Then locate the crate-root re-export: `grep -n "window::Window\|pub use" src/lib.rs | grep -i window` — if `Window` is re-exported at the crate root, add `Fullscreen` alongside it.

- [ ] **Step 7: Run test to verify it passes**

Run: `CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target CARGO_BUILD_JOBS=4 cargo test --workspace -j4 fullscreen_defaults_off_and_cycles -- --test-threads=4`
Expected: PASS

- [ ] **Step 8: Full gate + commit**

```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
CARGO_BUILD_JOBS=4 cargo test --workspace -j4 -- --test-threads=4
CARGO_BUILD_JOBS=4 cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
git add src/window/window.rs src/window/mod.rs src/lib.rs
git commit -m "feat(window): add Fullscreen enum + per-window state

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: `Frame` frameless rendering + dead hotspots

**Files:**
- Modify: `src/frame.rs` (struct `Frame` ~83, `Frame::new` ~124, `draw` ~262, `handle_event` ~458)
- Test: `src/frame.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Produces: `Frame` field `border_visible: bool` (default `true`); `pub(crate) fn Frame::set_border_visible(&mut self, v: bool)`. Consumed by `Window` in Task 6.

- [ ] **Step 1: Write the failing test** — append to the `tests` module in `src/frame.rs` (mirror the existing frame render helper used by other snapshot tests there; if none, build a `DrawCtx` like `menu_bar.rs:render`):

```rust
#[test]
fn frameless_draws_no_border() {
    // A frame with border_visible=false fills its interior background but draws
    // no box edges, title, or icons.
    let theme = crate::theme::Theme::classic_blue();
    let (backend, screen) = crate::backend::HeadlessBackend::new(12, 4);
    let mut r = crate::backend::Renderer::new(Box::new(backend));
    let mut frame = Frame::new(Rect::new(0, 0, 12, 4));
    frame.set_title(Some("Hi".into()));
    frame.st.state.active = true;
    frame.set_border_visible(false);
    r.render(|buf: &mut crate::screen::Buffer| {
        let b = frame.st.get_bounds();
        let mut dc = crate::view::DrawCtx::new(buf, &theme, b, b.a);
        frame.draw(&mut dc);
    });
    let snap = screen.snapshot();
    assert!(!snap.contains('═') && !snap.contains('║'), "no double-line border");
    assert!(!snap.contains("Hi"), "no title when frameless");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target CARGO_BUILD_JOBS=4 cargo test --workspace -j4 frameless_draws_no_border -- --test-threads=4`
Expected: FAIL — `set_border_visible` does not exist.

- [ ] **Step 3: Add the field + setter.** In `struct Frame` (~109, after `junction_marks`) add:

```rust
    /// Whether to draw the border box, title, and icons. `false` for a frameless
    /// fullscreen window — the interior background fill stays unconditional, but
    /// every edge/title/icon is suppressed. Pushed down by the owning window like
    /// [`zoomed`]. See [`set_border_visible`](Frame::set_border_visible).
    border_visible: bool,
```

In `Frame::new` (~131) add `border_visible: true,` to the literal.

After the `as_any_mut` method (~522), inside `impl Frame` or alongside the other setters (find `pub(crate) fn set_zoomed`), add:

```rust
    /// Show or hide the border box, title, and icons (the interior fill is
    /// unaffected). The owning [`Window`](crate::window::Window) pushes `false`
    /// when it goes frameless-fullscreen, via the same `child_mut` + downcast seam
    /// as [`set_zoomed`](Frame::set_zoomed).
    pub(crate) fn set_border_visible(&mut self, v: bool) {
        self.border_visible = v;
    }
```

(If `set_zoomed` is defined in a separate `impl Frame` block from `as_any_mut`, place `set_border_visible` next to `set_zoomed`.)

- [ ] **Step 4: Split the draw loop.** In `draw` (~335–432) the interior fill is interleaved with the edge draws, so it cannot be a single guard. Replace the block from `// -- 1. The box ...` through the end of section 6 (the bottom-row + title + icon blocks) so the **interior fill runs unconditionally** and **everything else is guarded by `border_visible`**:

```rust
        // -- 1a. Interior background fill (UNCONDITIONAL — a frameless window still
        // paints its body; content overdraws this). Same `border` role that paints
        // a bordered window's interior.
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                ctx.put_char(x, y, ' ', border);
            }
        }

        // -- 1b..6. Border box, title, number, and icons — only when bordered.
        if self.border_visible {
            // Top row: tl, ─ (or a tee at a marked cell) across the interior, tr.
            ctx.put_char(0, 0, tl, border);
            for x in 1..w - 1 {
                let ch = self
                    .junction_at(Edge::Top, x, bar, &glyphs)
                    .unwrap_or(h_edge);
                ctx.put_char(x, 0, ch, border);
            }
            if w >= 2 {
                ctx.put_char(w - 1, 0, tr, border);
            }
            // Middle-row left/right edges (interior already filled above).
            for y in 1..h - 1 {
                let lch = self
                    .junction_at(Edge::Left, y, bar, &glyphs)
                    .unwrap_or(v_edge);
                ctx.put_char(0, y, lch, border);
                if w >= 2 {
                    let rch = self
                        .junction_at(Edge::Right, y, bar, &glyphs)
                        .unwrap_or(v_edge);
                    ctx.put_char(w - 1, y, rch, border);
                }
            }
            // Bottom row.
            if h >= 2 {
                ctx.put_char(0, h - 1, bl, border);
                for x in 1..w - 1 {
                    let ch = self
                        .junction_at(Edge::Bottom, x, bar, &glyphs)
                        .unwrap_or(h_edge);
                    ctx.put_char(x, h - 1, ch, border);
                }
                if w >= 2 {
                    ctx.put_char(w - 1, h - 1, br, border);
                }
            }

            // Window number (top row).
            if let Some(n) = self.number
                && n < 10
            {
                let i = if self.flags.zoom { 7 } else { 3 };
                if let Some(digit) = char::from_digit(u32::from(n), 10) {
                    ctx.put_char(w - i, 0, digit, border);
                }
            }

            // Title (top row), centered.
            if let Some(title) = &self.title {
                let cap = w - 10;
                let (end, lw) = crate::text::scroll(title, cap, false);
                let lw = lw as i32;
                let truncated = &title[..end];
                let i = (w - lw) >> 1;
                ctx.put_char(i - 1, 0, ' ', border);
                ctx.put_str(i, 0, truncated, border);
                ctx.put_char(i + lw, 0, ' ', border);
            }

            // Active-only icons (top row).
            if self.st.state.active {
                if self.flags.close {
                    ctx.put_cstr(2, 0, glyphs.close_icon, border, icon);
                }
                if self.flags.zoom {
                    let zi = if self.zoomed {
                        glyphs.unzoom_icon
                    } else {
                        glyphs.zoom_icon
                    };
                    ctx.put_cstr(w - 5, 0, zi, border, icon);
                }
            }

            // Active + grow resize icons (bottom row).
            if self.st.state.active && self.flags.grow && h >= 2 {
                ctx.put_cstr(0, h - 1, glyphs.drag_left_icon, border, icon);
                ctx.put_cstr(w - 2, h - 1, glyphs.drag_icon, border, icon);
            }
        }
```

(This preserves the existing bordered output byte-for-byte: the interior fill is the same `put_char(x, y, ' ', border)`; everything else is identical, just wrapped.)

- [ ] **Step 5: Guard the hotspots.** In `handle_event` (~458) guard the whole mouse logic so a frameless frame arms nothing. Change the opening of the `Event::MouseDown(m)` arm to bail when frameless, and likewise the `MouseUp` close-confirm:

```rust
            Event::MouseDown(m) => {
                if !self.border_visible {
                    return; // frameless: no close/zoom/drag hotspots
                }
                let w = self.st.size.x;
                // ... existing close/zoom logic unchanged ...
            }

            Event::MouseUp(m) if self.close_pressed && self.border_visible => {
                // ... existing release-confirm unchanged ...
            }
```

- [ ] **Step 6: Run test to verify it passes**, review snapshot if any pending

Run: `CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target CARGO_BUILD_JOBS=4 cargo test --workspace -j4 frameless_draws_no_border -- --test-threads=4`
Expected: PASS. Also confirm the existing `frame.rs` bordered snapshots are UNCHANGED (no `.snap.new` for them).

- [ ] **Step 7: Full gate + commit**

```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
CARGO_BUILD_JOBS=4 cargo test --workspace -j4 -- --test-threads=4
CARGO_BUILD_JOBS=4 cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
git add src/frame.rs
git commit -m "feat(frame): frameless mode (set_border_visible) + dead hotspots

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: `MenuBar` collapse to `⋮` + corner popup activation

**Files:**
- Modify: `src/menu/menu_bar.rs` (struct `MenuBar` ~48, `MenuBar::new` ~64, `draw` ~120, `handle_event` ~151, inherent `impl MenuBar` ~182)
- Test: `src/menu/menu_bar.rs` (`#[cfg(test)] mod tests`)

**Interfaces:**
- Produces: `MenuBar` field `collapsed: bool`; `pub fn MenuBar::set_collapsed(&mut self, v: bool)`; `pub fn MenuBar::collapsed(&self) -> bool`. Consumed by the pump in Task 5.

- [ ] **Step 1: Write the failing test** — append to the `tests` module in `src/menu/menu_bar.rs`:

```rust
#[test]
fn collapsed_bar_draws_only_kebab() {
    // A collapsed bar paints the ⋮ kebab at the top-right cell and nothing else.
    let mut bar = sample_bar(Rect::new(0, 0, 24, 1));
    bar.set_collapsed(true);
    assert!(bar.collapsed());
    let snap = render(&mut bar, 24, 1);
    assert!(snap.contains('⋮'), "kebab drawn");
    assert!(!snap.contains("File"), "top-level items NOT drawn when collapsed");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target CARGO_BUILD_JOBS=4 cargo test --workspace -j4 collapsed_bar_draws_only_kebab -- --test-threads=4`
Expected: FAIL — `set_collapsed`/`collapsed` do not exist.

- [ ] **Step 3: Add the field + accessors.** In `struct MenuBar` (~48) add a field:

```rust
pub struct MenuBar {
    mv: MenuViewState,
    /// When true the bar is rendered as a single `⋮` kebab (top-right cell) and
    /// activation opens a corner popup instead of the in-bar session. Driven by
    /// the pump when a `Fullscreen::Screen` window covers the menu row.
    collapsed: bool,
}
```

In `MenuBar::new` (~68) add `collapsed: false,` to the literal.

In the inherent `impl MenuBar` (~182) add:

```rust
    /// Collapse the bar to a `⋮` kebab (or restore the full bar). The pump drives
    /// this together with a bounds change to the kebab cell.
    pub fn set_collapsed(&mut self, v: bool) {
        self.collapsed = v;
    }

    /// Whether the bar is currently collapsed to a `⋮` kebab.
    pub fn collapsed(&self) -> bool {
        self.collapsed
    }
```

- [ ] **Step 4: Collapsed draw.** At the very top of `draw` (~120), before the normal layout, short-circuit when collapsed:

```rust
    fn draw(&mut self, ctx: &mut DrawCtx) {
        let colors = MenuColors::resolve(ctx);

        if self.collapsed {
            // Render only the ⋮ kebab at the top-right cell; the rest of the row
            // is left transparent so the fullscreen window shows through.
            let size = self.mv.state.size;
            ctx.put_char(size.x - 1, 0, '⋮', colors.normal.0);
            return;
        }

        let size = self.mv.state.size;
        // ... existing fill + item loop unchanged ...
    }
```

- [ ] **Step 5: Corner popup activation.** Replace `handle_event` (~151) so a collapsed bar opens a corner popup on activation:

```rust
    fn handle_event(&mut self, ev: &mut Event, ctx: &mut Context) {
        if self.collapsed {
            // Activation (click the kebab, F10/cmMenu, or any Alt-shortcut) opens a
            // vertical popup anchored at the top-right corner — the session freezes
            // the bar's bounds at activation, so reclaiming width is impossible
            // (see docs/design/fullscreen-window.md). Other events (graying
            // broadcast, accelerator posts) still delegate.
            let activate = match *ev {
                Event::MouseDown(_) => true,
                Event::Command(c) => c == crate::command::Command::MENU,
                Event::KeyDown(k) => k.modifiers.alt,
                _ => false,
            };
            if activate {
                let owner = ctx.owner_size();
                crate::menu::popup_menu(
                    crate::view::Point::new(owner.x - 1, 0),
                    self.mv.menu.clone(),
                    owner,
                    ctx,
                );
                ev.clear();
                return;
            }
        }
        menu_view::handle_event(&self.mv, ev, ctx);
    }
```

Add any missing imports at the top of `menu_bar.rs`: ensure `crate::event::Event` is imported (it is), and that `crate::view::Point` resolves (add `Point` to the existing `use crate::view::{...}` line — currently `use crate::view::{Context, DrawCtx, Rect, View, ViewState};` → add `Point`).

- [ ] **Step 6: Run test to verify it passes**, review the snapshot

Run: `CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target CARGO_BUILD_JOBS=4 cargo test --workspace -j4 collapsed_bar_draws_only_kebab -- --test-threads=4`
Expected: PASS. Confirm existing `menu_bar.rs` snapshots are unchanged.

- [ ] **Step 7: Full gate + commit**

```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
CARGO_BUILD_JOBS=4 cargo test --workspace -j4 -- --test-threads=4
CARGO_BUILD_JOBS=4 cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
git add src/menu/menu_bar.rs
git commit -m "feat(menu): collapsible MenuBar (⋮ kebab + corner popup)

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 4: `Window::client_rect` + edge-aware `standard_scroll_bar`

**Files:**
- Modify: `src/window/window.rs` (`standard_scroll_bar` ~477; new `client_rect`)
- Test: `src/window/window.rs` (`tests`)

**Interfaces:**
- Consumes: `Window::fullscreen` (Task 1).
- Produces: `pub fn Window::client_rect(&self) -> Rect` (full extent when frameless, frame-inset otherwise). Public seam for app content placement.

- [ ] **Step 1: Write the failing test** — append to `tests` in `src/window/window.rs`:

```rust
#[test]
fn client_rect_full_when_frameless() {
    use crate::window::Fullscreen;
    let mut win = Window::new(Rect::new(0, 0, 20, 8), None, 0);
    let ext = win.state().get_extent(); // (0,0,20,8)
    // Bordered: inset by one on every side.
    let cr = win.client_rect();
    assert_eq!((cr.a.x, cr.a.y, cr.b.x, cr.b.y), (1, 1, ext.b.x - 1, ext.b.y - 1));
    // Frameless: the full extent.
    win.fullscreen = Fullscreen::Desktop;
    assert_eq!(win.client_rect(), ext);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target CARGO_BUILD_JOBS=4 cargo test --workspace -j4 client_rect_full_when_frameless -- --test-threads=4`
Expected: FAIL — `client_rect` does not exist.

- [ ] **Step 3: Add `client_rect`.** Just above `standard_scroll_bar` (~477) add:

```rust
    /// The window's interior content rectangle (view-local). Inset by the frame on
    /// every side when bordered; the **full extent** when frameless-fullscreen, so
    /// content and scroll bars reach the screen edge. Apps placing window content
    /// should key off this rather than hardcoding the frame inset.
    pub fn client_rect(&self) -> Rect {
        let ext = self.group.state().get_extent();
        if self.fullscreen == Fullscreen::Off {
            Rect::from_points(
                Point::new(ext.a.x + 1, ext.a.y + 1),
                Point::new(ext.b.x - 1, ext.b.y - 1),
            )
        } else {
            ext
        }
    }
```

- [ ] **Step 4: Rewrite `standard_scroll_bar` to key off `client_rect`** (byte-identical for bordered windows, edge-reaching when frameless). Replace the `let ext = ...; let r = if opts.vertical { ... } else { ... };` head of `standard_scroll_bar` (~478–489) with:

```rust
        let ext = self.group.state().get_extent();
        let cr = self.client_rect();
        let r = if opts.vertical {
            // Right column; spans the client height. Bordered: identical to the
            // previous (ext.b.x-1, ext.a.y+1)..(ext.b.x, ext.b.y-1).
            Rect::from_points(
                Point::new(ext.b.x - 1, cr.a.y),
                Point::new(ext.b.x, cr.b.y),
            )
        } else {
            // Bottom row; spans the client width inset one. Bordered: identical to
            // the previous (ext.a.x+2, ext.b.y-1)..(ext.b.x-2, ext.b.y).
            Rect::from_points(
                Point::new(cr.a.x + 1, ext.b.y - 1),
                Point::new(cr.b.x - 1, ext.b.y),
            )
        };
```

- [ ] **Step 5: Run test + confirm no scrollbar snapshot drift**

Run: `CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target CARGO_BUILD_JOBS=4 cargo test --workspace -j4 -- --test-threads=4`
Expected: PASS, and **no `.snap.new`** for any existing scroller/window snapshot (the bordered geometry is unchanged). If any scrollbar snapshot drifts, the formula was mis-transcribed — fix before committing.

- [ ] **Step 6: Full gate + commit**

```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
CARGO_BUILD_JOBS=4 cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
git add src/window/window.rs
git commit -m "feat(window): client_rect seam + edge-aware standard_scroll_bar

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 5: `Deferred::SetFullscreen` + pump apply (the cross-tree engine)

**Files:**
- Modify: `src/view/context.rs` (`enum Deferred` ~66; `Context` helper block ~1181)
- Modify: `src/app/program.rs` (`struct Program` ~324; `Program::new` field init ~587; `pump_once` destructure ~1965, resize block ~1995, drain match ~2284; new free fn near `centered_msgbox_rect_for` ~3262)
- Test: `src/app/program.rs` (`tests`)

**Interfaces:**
- Consumes: `Fullscreen` (Task 1), `MenuBar::set_collapsed` (Task 3).
- Produces: `Deferred::SetFullscreen { window: ViewId, mode: Fullscreen }`; `pub fn Context::set_fullscreen(&mut self, window: ViewId, mode: Fullscreen)`; `Program` field `fullscreen: Option<FullscreenSlot>`; `struct FullscreenSlot { window: ViewId, mode: Fullscreen, restore: Rect, shadow: bool }` (derives `Clone, Copy`); free fn `apply_fullscreen(...)`. Consumed by `Window::set_fullscreen` in Task 6.

- [ ] **Step 1: Write the failing test** — append to the `tests` module in `src/app/program.rs`. Use the existing desktop+menubar test scaffolding (search for `fn program_with_menu_bar` ~7945 and the desktop-building test helper ~3768); model this on the `Deferred::FocusById` pump test at ~4036. The test pushes `SetFullscreen` directly and asserts the menubar collapsed and the window grew to cover row 0:

```rust
#[test]
fn set_fullscreen_screen_collapses_menu_and_covers_top() {
    use crate::view::Deferred;
    use crate::window::Fullscreen;
    // Build a program with a menu bar, status line, desktop, and one window.
    // (Reuse the richest existing scaffold that yields (program, window_id,
    // menu_bar_id, desktop_id) on, say, an 40x12 screen — see the desktop test
    // harness near line 3768 / program_with_menu_bar near 7945.)
    let (mut program, window_id, menu_bar_id, desktop_id) = program_with_fullscreen_scaffold(40, 12);

    program.deferred.push(Deferred::SetFullscreen { window: window_id, mode: Fullscreen::Screen });
    program.pump_once();

    // Menu bar collapsed to the ⋮ cell (top-right, width 1).
    let mb = program.group_ref().find(menu_bar_id).expect("menu bar").state().get_bounds();
    assert_eq!((mb.a.x, mb.b.x, mb.a.y, mb.b.y), (39, 40, 0, 1), "menu bar is the ⋮ cell");
    // Desktop top moved to row 0.
    let dt = program.group_ref().find(desktop_id).expect("desktop").state().get_bounds();
    assert_eq!(dt.a.y, 0, "desktop covers the menu row");
    // The fullscreen slot is recorded.
    assert!(program.fullscreen_active(), "fullscreen slot set");
}
```

> Implementer note: if the program test module lacks `group_ref`/`find`/`program_with_fullscreen_scaffold`/`fullscreen_active` helpers, add minimal `#[cfg(test)]` accessors next to the existing test helpers (the module already exposes `program.menu_bar()` and pushes to `program.deferred`; follow those patterns). Keep them test-only.

- [ ] **Step 2: Run test to verify it fails**

Run: `CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target CARGO_BUILD_JOBS=4 cargo test --workspace -j4 set_fullscreen_screen_collapses_menu_and_covers_top -- --test-threads=4`
Expected: FAIL to compile — `Deferred::SetFullscreen` does not exist.

- [ ] **Step 3: Add the `Deferred` variant.** In `src/view/context.rs`, add to `enum Deferred` (after `EndModal` ~98 is a good home — it touches the view tree + loop state):

```rust
    /// **Frameless-fullscreen layout broker.** A window requests its own
    /// fullscreen `mode` by id; the pump applies all cross-tree layout (frame
    /// already toggled inline by `Window::set_fullscreen`): collapse/restore the
    /// menu bar, re-bound the desktop, and re-fit the window — through the `View`
    /// trait, no downcast. Carries only the window id (the menu bar / desktop are
    /// singletons the pump supplies from its own state), exactly like
    /// [`UpdateMenu`](Self::UpdateMenu). Touches the **view-tree** family (+ the
    /// loop-owned fullscreen slot), so the insertion-order drain stays
    /// order-equivalent.
    SetFullscreen {
        /// The window changing fullscreen state.
        window: ViewId,
        /// The target mode (`Off` restores the chrome and the window's bounds).
        mode: crate::window::Fullscreen,
    },
```

Add `use crate::window::Fullscreen;` near the top of `context.rs` if `crate::window::Fullscreen` is not already in scope (the variant uses the fully-qualified path above, so an import is optional — keep the qualified path to avoid churn).

- [ ] **Step 4: Add the `Context` helper.** In `context.rs`, next to `request_close` (~1195):

```rust
    /// Request a fullscreen-mode change for `window`. The pump applies the
    /// cross-tree layout (menu bar, desktop, window re-fit). See
    /// [`Deferred::SetFullscreen`].
    pub fn set_fullscreen(&mut self, window: ViewId, mode: crate::window::Fullscreen) {
        self.deferred.push(Deferred::SetFullscreen { window, mode });
    }
```

- [ ] **Step 5: Add `FullscreenSlot` + the `Program` field.** In `src/app/program.rs`, near the top of the file (after imports) add:

```rust
/// Loop-owned record of the window currently in frameless-fullscreen, used to
/// re-fit on resize and restore chrome if the window is removed out from under us.
#[derive(Clone, Copy)]
struct FullscreenSlot {
    window: crate::view::ViewId,
    mode: crate::window::Fullscreen,
    /// Pre-fullscreen window bounds, restored on exit.
    restore: Rect,
    /// Pre-fullscreen shadow flag, restored verbatim on exit.
    shadow: bool,
}
```

In `struct Program` (~324, next to `desktop: Option<ViewId>`) add:

```rust
    /// The window currently in frameless-fullscreen, if any (loop-owned).
    fullscreen: Option<FullscreenSlot>,
```

In `Program::new` (~587, next to `menu_bar,`) add `fullscreen: None,` to the constructed `Program { ... }`.

- [ ] **Step 6: Add the `apply_fullscreen` free function.** Near `centered_msgbox_rect_for` (~3262) add (uses only the destructured fields the pump passes — no `&mut self`):

```rust
/// Apply a fullscreen `mode` to `window` across the tree. Border visibility is
/// toggled inline by `Window::set_fullscreen`; this performs the cross-tree work:
/// collapse/restore the menu bar (+ its bounds), re-bound the desktop, and re-fit
/// the window — all through the `View` trait (no downcast). Tracks/clears the
/// loop-owned `slot`. Reused by the deferred drain and the resize/vanish path.
#[allow(clippy::too_many_arguments)]
fn apply_fullscreen(
    group: &mut Group,
    desktop: Option<ViewId>,
    menu_bar: Option<ViewId>,
    status_line: Option<ViewId>,
    slot: &mut Option<FullscreenSlot>,
    window: ViewId,
    mode: crate::window::Fullscreen,
    ctx: &mut Context,
) {
    use crate::window::Fullscreen;
    let screen = group.state().size;
    let (w, h) = (screen.x, screen.y);
    let menu_present = menu_bar.is_some();
    let status_h = i32::from(status_line.is_some());

    // 1. Edge bookkeeping: capture restore bounds + shadow on first entering;
    //    clear the shadow while fullscreen.
    let entering = slot.as_ref().map_or(true, |s| s.mode == Fullscreen::Off)
        && mode != Fullscreen::Off;
    if entering {
        if let Some(v) = group.find_mut(window) {
            let restore = v.state().get_bounds();
            let shadow = v.state().state.shadow;
            v.state_mut().state.shadow = false;
            *slot = Some(FullscreenSlot { window, mode, restore, shadow });
        }
    } else if let Some(s) = slot.as_mut() {
        s.mode = mode;
    }

    // 2. Menu bar: collapse + bounds (⋮ cell when Screen, full top row otherwise).
    if let Some(mb) = menu_bar {
        let collapsed = mode == Fullscreen::Screen;
        if let Some(v) = group.find_mut(mb) {
            if let Some(bar) = v
                .as_any_mut()
                .and_then(|a| a.downcast_mut::<crate::menu::MenuBar>())
            {
                bar.set_collapsed(collapsed);
            }
            let bounds = if collapsed {
                Rect::new(w - 1, 0, w, 1)
            } else {
                Rect::new(0, 0, w, 1)
            };
            v.change_bounds(bounds);
        }
    }

    // 3. Desktop bounds: top row 0 when Screen, else below the menu bar.
    let top = if mode == Fullscreen::Screen { 0 } else { i32::from(menu_present) };
    if let Some(dt) = desktop {
        if let Some(v) = group.find_mut(dt) {
            v.change_bounds(Rect::new(0, top, w, h - status_h));
        }
    }

    // 4. Window bounds: fill the (now-sized) desktop, or restore on Off.
    let target = if mode == Fullscreen::Off {
        slot.as_ref().map(|s| s.restore)
    } else {
        let dh = (h - status_h) - top;
        Some(Rect::new(0, 0, w, dh)) // desktop-local: the window fills its owner
    };
    if let Some(rect) = target
        && let Some(v) = group.find_mut(window)
    {
        v.change_bounds(rect);
        v.on_bounds_changed(ctx);
    }

    // 5. Exit: restore the shadow verbatim and clear the slot.
    if mode == Fullscreen::Off
        && let Some(s) = slot.take()
        && let Some(v) = group.find_mut(window)
    {
        v.state_mut().state.shadow = s.shadow;
    }
}
```

Ensure `use crate::menu::MenuBar;` is available (the function uses the qualified `crate::menu::MenuBar`, so no new top-level import is required). Confirm `Group`, `ViewId`, `Rect`, `Context` are already imported in `program.rs` (they are).

- [ ] **Step 7: Wire the drain arm.** In `pump_once`'s destructure (~1965), change `menu_bar: _,` to `menu_bar,` and add `fullscreen,` to the destructured fields. In the drain `match effect` (~2284), add an arm (after `EndModal` ~2331):

```rust
                            Deferred::SetFullscreen { window, mode } => {
                                apply_fullscreen(
                                    group, *desktop, *menu_bar, *status_line,
                                    fullscreen, window, mode, &mut ctx,
                                );
                            }
```

- [ ] **Step 8: Wire resize re-fit + vanish restore.** In `pump_once`, the resize check is at ~1995–2003. Replace it so it records whether the size changed, and add the fullscreen-maintenance block **after** `let now = clock.now_ms();` (~2006) so `now` is available and the desktop re-bound has already cascaded:

```rust
        // 1. Resize check — the realization of setScreenMode/cmScreenChanged.
        let (w, h) = renderer.backend().size();
        let cur = group.state().size;
        let size_changed = cur.x != w as i32 || cur.y != h as i32;
        if size_changed {
            renderer.resize(w, h);
            group.change_bounds(Rect::new(0, 0, w as i32, h as i32));
        }

        // 2. Sample the clock once for this pass.
        let now = clock.now_ms();

        // 2a. Fullscreen layout maintenance: re-fit the tracked window after a
        //     resize (the growMode cascade just re-stretched the collapsed menu
        //     bar — re-shrink it), or restore chrome if the window was removed.
        if let Some(slot) = *fullscreen {
            let mut ctx = Context::new(out_events, timers, now, deferred);
            ctx.set_disabled_commands(disabled_commands.clone());
            ctx.set_clipboard_snapshot(*clipboard_editor_id, *clipboard_has_selection);
            if group.find_mut(slot.window).is_none() {
                apply_fullscreen(
                    group, *desktop, *menu_bar, *status_line,
                    fullscreen, slot.window, crate::window::Fullscreen::Off, &mut ctx,
                );
            } else if size_changed {
                apply_fullscreen(
                    group, *desktop, *menu_bar, *status_line,
                    fullscreen, slot.window, slot.mode, &mut ctx,
                );
            }
        }
```

(Delete the original separate `let now = clock.now_ms();` that followed the old resize block so it is not declared twice.)

- [ ] **Step 9: Run test to verify it passes**

Run: `CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target CARGO_BUILD_JOBS=4 cargo test --workspace -j4 set_fullscreen_screen_collapses_menu_and_covers_top -- --test-threads=4`
Expected: PASS

- [ ] **Step 10: Full gate + commit**

```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
CARGO_BUILD_JOBS=4 cargo test --workspace -j4 -- --test-threads=4
CARGO_BUILD_JOBS=4 cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
git add src/view/context.rs src/app/program.rs
git commit -m "feat(app): Deferred::SetFullscreen + pump-applied fullscreen layout

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 6: `Window::set_fullscreen` + `Command::FULLSCREEN` + drag guard (end-to-end)

**Files:**
- Modify: `src/command.rs` (add `FULLSCREEN` const ~102)
- Modify: `src/window/window.rs` (`handle_event` command block ~1166, drag block ~1275; new `set_fullscreen` method)
- Modify: `CHANGELOG.md`
- Test: `src/app/program.rs` (`tests`) — end-to-end snapshot

**Interfaces:**
- Consumes: `Frame::set_border_visible` (Task 2), `Context::set_fullscreen` (Task 5), `Fullscreen` (Task 1).
- Produces: `pub fn Window::set_fullscreen(&mut self, mode: Fullscreen, ctx: &mut Context)`; `Command::FULLSCREEN`.

- [ ] **Step 1: Write the failing end-to-end test** — append to `tests` in `src/app/program.rs`:

```rust
#[test]
fn fullscreen_command_cycles_and_renders_frameless() {
    use crate::command::Command;
    // One window on a desktop with a menu bar; post FULLSCREEN twice to reach
    // Screen, pump, and snapshot the frameless body + ⋮ kebab.
    let (mut program, window_id, _menu_bar_id, _desktop_id) =
        program_with_fullscreen_scaffold(40, 12);

    // Cycle Off -> Desktop.
    program.post_to_window(window_id, Command::FULLSCREEN);
    program.pump_once();
    // Cycle Desktop -> Screen.
    program.post_to_window(window_id, Command::FULLSCREEN);
    program.pump_once();

    let snap = program.snapshot();
    assert!(snap.contains('⋮'), "menu collapsed to kebab");
    assert!(!snap.contains('═'), "window frame is gone");
    insta::assert_snapshot!(snap);
}
```

> Implementer note: route `FULLSCREEN` to the active window the way the existing zoom/close command tests do (a focused `Event::Command`). If no `post_to_window`/`snapshot` test helper exists, add minimal `#[cfg(test)]` ones next to the existing program test helpers (the harness already renders via the `HeadlessBackend`; reuse its screen handle).

- [ ] **Step 2: Run test to verify it fails**

Run: `CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target CARGO_BUILD_JOBS=4 cargo test --workspace -j4 fullscreen_command_cycles_and_renders_frameless -- --test-threads=4`
Expected: FAIL — `Command::FULLSCREEN` / `Window::set_fullscreen` do not exist.

- [ ] **Step 3: Add the command.** In `src/command.rs` after `MENU` (~102):

```rust
    /// Cycle the active window through frameless-fullscreen modes
    /// (`Off → Desktop → Screen → Off`). Handled by the `Window`; no default key
    /// binding (apps bind their own). See [`Window::set_fullscreen`].
    pub const FULLSCREEN: Command = Command("tv.fullscreen");
```

- [ ] **Step 4: Add `Window::set_fullscreen`.** In `src/window/window.rs`, next to `zoom` (~509) or the setters block, add:

```rust
    /// Set the window's frameless-fullscreen [`mode`](Fullscreen). Toggles the
    /// frame border **inline** (the only path that reaches the frame — the pump
    /// cannot downcast to `Window`), records the new mode, and emits
    /// [`Deferred::SetFullscreen`](crate::view::Deferred::SetFullscreen) for the
    /// pump to apply the cross-tree layout (menu bar / desktop / window re-fit).
    pub fn set_fullscreen(&mut self, mode: Fullscreen, ctx: &mut Context) {
        self.fullscreen = mode;
        if let Some(frame) = self.frame_mut() {
            frame.set_border_visible(mode == Fullscreen::Off);
        }
        if let Some(id) = self.group.state().id() {
            ctx.set_fullscreen(id, mode);
        }
    }
```

- [ ] **Step 5: Handle `Command::FULLSCREEN` in `handle_event`.** In `Window::handle_event`, after the `Command::ZOOM` block (~1172), add:

```rust
        if let Event::Command(c) = *ev
            && c == Command::FULLSCREEN
        {
            self.set_fullscreen(self.fullscreen.next(), ctx);
            ev.clear();
        }
```

- [ ] **Step 6: Guard the drag detection.** In the drag block (~1275) add the fullscreen guard so a frameless window never starts a title/grow drag:

```rust
        if let Event::MouseDown(m) = *ev
            && self.fullscreen == Fullscreen::Off
        {
            // ... existing drag-kind detection unchanged ...
        }
```

- [ ] **Step 7: Run test, review the snapshot**

Run: `CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target CARGO_BUILD_JOBS=4 cargo test --workspace -j4 fullscreen_command_cycles_and_renders_frameless -- --test-threads=4`
Expected: PASS after reviewing/accepting the new `.snap` (eyeball the whole 40×12 frame: frameless body, `⋮` at top-right, status line intact).

- [ ] **Step 8: Roll the CHANGELOG.** Add under `## Unreleased` → `### New` in `CHANGELOG.md`:

```markdown
- Frameless fullscreen windows: `Window::set_fullscreen(Fullscreen::{Off,Desktop,Screen})` and a cycling `Command::FULLSCREEN`. `Desktop` hides the frame and fills the desktop; `Screen` also covers the menu row, collapsing the menu bar to a `⋮` kebab that opens a corner popup. `Window::client_rect()` exposes the frameless content area.
```

- [ ] **Step 9: Full gate + commit**

```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
CARGO_BUILD_JOBS=4 cargo test --workspace -j4 -- --test-threads=4
CARGO_BUILD_JOBS=4 cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
git add src/command.rs src/window/window.rs CHANGELOG.md tests/ src/app/program.rs
git commit -m "feat(window): set_fullscreen API + Command::FULLSCREEN cycler

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Manual verification (after all tasks)

Run the demo app and exercise it by hand (the `run` skill / `cargo run --example hello`): there is no default key binding, so add a temporary status-line key → `Command::FULLSCREEN` (or post it from a menu item) to drive the cycle, and confirm: (1) `Desktop` removes the border and the window fills the desktop with the menu still visible; (2) `Screen` covers the menu row with a `⋮` at the top-right; (3) clicking the `⋮` (and F10) opens a corner popup; (4) a click in the former title/close area does nothing; (5) cycling back to `Off` restores the exact prior size, border, menu bar, and shadow.

## Self-review notes (coverage against the design spec)

- Mode a (frameless fill) → Tasks 2 (border) + 5 (desktop fill via pump) + 6 (trigger). ✓
- Mode b (cover menu + ⋮) → Tasks 3 (collapse + popup) + 5 (menubar bounds + desktop row 0). ✓
- Inline border / deferred cross-tree split → Task 6 inline `frame_mut`, Task 5 pump. ✓
- One `Deferred::SetFullscreen` carrying the window id → Task 5. ✓
- `restore`/`shadow` in pump slot, captured via `View` trait → Task 5 `apply_fullscreen`. ✓
- Resize re-fit (after cascade) + removal restore → Task 5 Step 8. ✓
- Dead hotspots (frame + window drag) → Tasks 2 + 6. ✓
- `client_rect` inherent + scrollbar → Task 4. ✓
- Collapse = bounds change (no bubbling) + corner popup (no width reclaim) → Tasks 3 + 5. ✓
- No default key binding; app owns exit; `Command::FULLSCREEN` in `command.rs` → Task 6. ✓
