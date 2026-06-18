# Consumer-API Coverage (Axis A + Axis C) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close the *mechanical, already-decided* half of the consumer-API coverage gaps from [`docs/specs/2026-06-17-consumer-api-gaps.md`](../../specs/2026-06-17-consumer-api-gaps.md) — Axis A (Window/Dialog config setters are `pub(crate)`-frozen) and Axis C.1 (`Group` doesn't bubble `get_help_ctx`) — and make `examples/tcv.rs` faithful by dropping the two workarounds those gaps forced.

**Architecture:** Axis A widens existing `pub(crate)` setters to `pub` and adds `with_*` builders on `Window`/`Dialog` (the field re-push machinery already exists and is unchanged — we only open the door). Axis C.1 adds one `View::get_help_ctx` override on `Group` that delegates to the current child (faithful `TGroup::getHelpCtx`); `Window`/`Dialog` inherit it through the existing `#[delegate]` forwarder, so no macro change is needed. Both are additive — every current default is preserved.

**Tech Stack:** Rust (workspace `tvision-rs` + `tvision-rs-macros`), `insta` snapshot tests on the `HeadlessBackend`, the `#[delegate]` proc-macro.

## Global Constraints

- `export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target` — artifacts land there, **not** `./target`.
- Shared 128-core machine, **max 4 cores**: build/test with `-j2 --test-threads=2` per agent; at most two building agents in parallel.
- Gates that must pass on the integrated tree before each commit: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, and `cargo xtask test` (guide doctests) when docs/examples change.
- Snapshots: `cargo-insta` is **not installed** — generate with `INSTA_UPDATE=always`, **hand-verify the `.snap` against expected C++ behavior**, then commit the `.snap` file.
- **Faithful by default** (CLAUDE.md): match the C++ behavior; deviations are only the pre-decided D-rules. English for all code/comments/identifiers.
- Commit messages end with the project's `Co-Authored-By` trailer.
- **Additive only:** keep every current construction default unchanged — Axis A only adds knobs; do not change what `Window::new`/`Dialog::new` produce.

---

## File Structure

- `src/window/window.rs` — widen `set_flags`/`set_palette`/`set_grow_mode` to `pub`, add `set_drag_mode` + four `with_*` builders. (Task 1)
- `src/dialog/dialog.rs` — widen `set_flags`/`flags` to `pub`, add `set_palette`/`set_grow_mode`/`set_drag_mode` + `with_*` builders mirroring `Window`. (Task 2)
- `src/view/group.rs` — add the `get_help_ctx` override in `impl View for Group`. (Task 4)
- `examples/tcv.rs` — make the catalog window a fixed panel via the new public API (Task 3); drop the `DataWindow::handle_event` help-ctx caching hack (Task 5).

---

## Task 1: Window public decoration setters + builders

**Files:**
- Modify: `src/window/window.rs` (setters at 230–269; import at line 7; builders added after the setter block; test in the `mod tests` block)
- Test: `src/window/window.rs` `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: existing `WindowFlags` (window.rs:29), `WindowPalette` (window.rs:56), `GrowMode` (view.rs:183), `DragMode` (view.rs:219); existing private re-push seam in `set_flags`/`set_palette`.
- Produces (later tasks rely on these exact signatures):
  - `pub fn Window::set_flags(&mut self, flags: WindowFlags)`
  - `pub fn Window::set_palette(&mut self, palette: WindowPalette)`
  - `pub fn Window::set_grow_mode(&mut self, grow_mode: GrowMode)`
  - `pub fn Window::set_drag_mode(&mut self, drag_mode: DragMode)`
  - `pub fn Window::with_flags(self, flags: WindowFlags) -> Self`
  - `pub fn Window::with_palette(self, palette: WindowPalette) -> Self`
  - `pub fn Window::with_grow_mode(self, grow_mode: GrowMode) -> Self`
  - `pub fn Window::with_drag_mode(self, drag_mode: DragMode) -> Self`

- [ ] **Step 1: Write the failing test**

Add to `src/window/window.rs` `mod tests`:

```rust
#[test]
fn flags_off_window_is_a_fixed_iconless_panel() {
    // A consumer building TCV's fixed full-desktop panel: all decoration off.
    let w = Window::new(Rect::new(0, 0, 24, 8), Some("Catalog".into()), 0)
        .with_flags(WindowFlags::default()) // all four false
        .with_grow_mode(GrowMode::default())
        .with_drag_mode(DragMode::default());
    assert_eq!(
        w.flags(),
        WindowFlags {
            r#move: false,
            grow: false,
            close: false,
            zoom: false,
        },
        "consumer can clear all decoration flags"
    );
    let gm = w.state().grow_mode;
    assert!(
        !gm.lo_x && !gm.lo_y && !gm.hi_x && !gm.hi_y && !gm.rel && !gm.fixed,
        "consumer can clear grow_mode"
    );
}

#[test]
fn with_palette_sets_and_pushes_to_frame() {
    let mut w = Window::new(Rect::new(0, 0, 24, 8), Some("Cyan".into()), 1)
        .with_palette(WindowPalette::Cyan);
    assert_eq!(w.palette(), WindowPalette::Cyan);
    let frame_id = w.frame_id();
    let frame = w
        .child_mut(frame_id)
        .and_then(|v| v.as_any_mut())
        .and_then(|a| a.downcast_mut::<crate::frame::Frame>())
        .expect("window has a Frame child");
    assert_eq!(
        frame.palette(),
        WindowPalette::Cyan,
        "with_palette must propagate to the frame child"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p tvision-rs --lib -j2 -- --test-threads=2 window::tests::flags_off_window_is_a_fixed_iconless_panel`
Expected: FAIL — `with_flags`/`with_grow_mode`/`with_drag_mode` don't exist (method-not-found).

- [ ] **Step 3: Widen the setters and add builders**

In `src/window/window.rs`, change the import at line 7 to add `DragMode` to the `crate::view` group (alongside `GrowMode`).

Change the three setter signatures from `pub(crate) fn` to `pub fn` (keep their bodies exactly as-is): `set_flags` (line 237), `set_palette` (line 253), `set_grow_mode` (line 267). Update the doc comment at line 230 from "used by subtypes such as Dialog to override defaults" to note they are now the public post-construction configuration surface.

Add a `set_drag_mode` next to `set_grow_mode`:

```rust
/// Override the drag mode after construction (the screen-edge limits the window
/// honors while being dragged). Mirrors [`set_grow_mode`](Self::set_grow_mode):
/// a plain write to the embedded group's [`ViewState::drag_mode`].
pub fn set_drag_mode(&mut self, drag_mode: DragMode) {
    self.group.state_mut().drag_mode = drag_mode;
}
```

Add the four builders right after the setter block (before `insert_child`):

```rust
/// Builder form of [`set_flags`](Self::set_flags).
pub fn with_flags(mut self, flags: WindowFlags) -> Self {
    self.set_flags(flags);
    self
}

/// Builder form of [`set_palette`](Self::set_palette).
pub fn with_palette(mut self, palette: WindowPalette) -> Self {
    self.set_palette(palette);
    self
}

/// Builder form of [`set_grow_mode`](Self::set_grow_mode).
pub fn with_grow_mode(mut self, grow_mode: GrowMode) -> Self {
    self.set_grow_mode(grow_mode);
    self
}

/// Builder form of [`set_drag_mode`](Self::set_drag_mode).
pub fn with_drag_mode(mut self, drag_mode: DragMode) -> Self {
    self.set_drag_mode(drag_mode);
    self
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p tvision-rs --lib -j2 -- --test-threads=2 window::tests`
Expected: PASS (including the two new tests). Then `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` clean.

- [ ] **Step 5: Commit**

```bash
git add src/window/window.rs
git commit -m "feat(window): public decoration setters + with_* builders

Un-pub(crate) set_flags/set_palette/set_grow_mode, add set_drag_mode and
with_* builders so a consumer can build a fixed, icon-less panel (Axis A of
the consumer-API gaps). Additive: all construction defaults unchanged.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 2: Dialog public decoration setters + builders (mirror Window)

**Files:**
- Modify: `src/dialog/dialog.rs` (`set_flags`/`flags` at 112–127; add setters/builders after them; test in `mod tests`)
- Test: `src/dialog/dialog.rs` `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: Task 1's `Window` public setters; `WindowFlags`, `WindowPalette`, `GrowMode`, `DragMode`.
- Produces:
  - `pub fn Dialog::set_flags(&mut self, flags: WindowFlags)`
  - `pub fn Dialog::flags(&self) -> WindowFlags`
  - `pub fn Dialog::set_palette(&mut self, palette: WindowPalette)`
  - `pub fn Dialog::set_grow_mode(&mut self, grow_mode: GrowMode)`
  - `pub fn Dialog::set_drag_mode(&mut self, drag_mode: DragMode)`
  - `pub fn Dialog::with_flags/with_palette/with_grow_mode/with_drag_mode(self, …) -> Self`

- [ ] **Step 1: Write the failing test**

Add to `src/dialog/dialog.rs` `mod tests`:

```rust
#[test]
fn consumer_can_add_grow_flag_and_change_palette() {
    use crate::window::{WindowFlags, WindowPalette};
    let d = Dialog::new(Rect::new(0, 0, 40, 12), Some("Resizable".into()))
        .with_flags(WindowFlags {
            r#move: true,
            close: true,
            grow: true,
            ..WindowFlags::default()
        })
        .with_palette(WindowPalette::Cyan);
    assert!(d.flags().grow, "consumer added the grow flag publicly");
    assert!(d.flags().r#move && d.flags().close);
    // palette pushed to the frame child:
    let mut d = d;
    let frame_id = d.window.frame_id();
    let frame = d
        .window
        .child_mut(frame_id)
        .and_then(|v| v.as_any_mut())
        .and_then(|a| a.downcast_mut::<crate::frame::Frame>())
        .expect("dialog window has a Frame child");
    assert_eq!(frame.palette(), WindowPalette::Cyan);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p tvision-rs --lib -j2 -- --test-threads=2 dialog::tests::consumer_can_add_grow_flag_and_change_palette`
Expected: FAIL — `with_flags`/`with_palette` not found on `Dialog`; `flags()` is `pub(crate)` (visible inside the test module, so the failure is the missing builders).

- [ ] **Step 3: Widen + add setters/builders**

In `src/dialog/dialog.rs`: change `set_flags` (line 117) and `flags` (line 125) from `pub(crate) fn` to `pub fn` (bodies unchanged). Add `crate::view::{GrowMode, DragMode}` and `crate::window::WindowPalette` to imports as needed (`GrowMode` and `WindowPalette` are already imported at the top — confirm; add `DragMode`). Then add, after `flags`:

```rust
/// Override the colour scheme after construction. Mirrors [`Window::set_palette`].
pub fn set_palette(&mut self, palette: WindowPalette) {
    self.window.set_palette(palette);
}

/// Override the grow mode after construction. Mirrors [`Window::set_grow_mode`].
pub fn set_grow_mode(&mut self, grow_mode: GrowMode) {
    self.window.set_grow_mode(grow_mode);
}

/// Override the drag mode after construction. Mirrors [`Window::set_drag_mode`].
pub fn set_drag_mode(&mut self, drag_mode: DragMode) {
    self.window.set_drag_mode(drag_mode);
}

/// Builder form of [`set_flags`](Self::set_flags).
pub fn with_flags(mut self, flags: WindowFlags) -> Self {
    self.set_flags(flags);
    self
}

/// Builder form of [`set_palette`](Self::set_palette).
pub fn with_palette(mut self, palette: WindowPalette) -> Self {
    self.set_palette(palette);
    self
}

/// Builder form of [`set_grow_mode`](Self::set_grow_mode).
pub fn with_grow_mode(mut self, grow_mode: GrowMode) -> Self {
    self.set_grow_mode(grow_mode);
    self
}

/// Builder form of [`set_drag_mode`](Self::set_drag_mode).
pub fn with_drag_mode(mut self, drag_mode: DragMode) -> Self {
    self.set_drag_mode(drag_mode);
    self
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p tvision-rs --lib -j2 -- --test-threads=2 dialog::tests`
Expected: PASS. Then clippy + fmt clean as in Task 1 Step 4.

- [ ] **Step 5: Commit**

```bash
git add src/dialog/dialog.rs
git commit -m "feat(dialog): public decoration setters + with_* builders

Mirror the Window Axis A change on Dialog: pub set_flags/flags, add
set_palette/set_grow_mode/set_drag_mode and with_* builders. Additive.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 3: Make `tcv` the fixed panel via the new public API

**Files:**
- Modify: `examples/tcv.rs` (`DataWindow::new` at 604–655; the header workaround note)

**Interfaces:**
- Consumes: `Dialog::with_flags`/`with_grow_mode` from Task 2; `WindowFlags`/`GrowMode` (add to the `tv::` import list at the top of `tcv.rs`).

- [ ] **Step 1: Apply the fixed-panel config**

In `examples/tcv.rs`, change the `Dialog::new(...)` line in `DataWindow::new` (line 606) to clear decoration (TCV's `Flags := 0; DragMode := 0; GrowMode := 0`) — note the original is still framed + titled, so this is purely the decoration/behavior flags:

```rust
let mut dialog = Dialog::new(bounds, Some("Tobis Catalog Vision Version 2.2".to_string()))
    .with_flags(WindowFlags::default())   // TCV: Flags := $00 (fixed, no icons)
    .with_grow_mode(GrowMode::default()); // TCV: GrowMode := $00
```

Add `GrowMode, WindowFlags` to the `tvision_rs::{…}` import list at the top of the file. In the file header comment, update the "workaround" note for the window flags to say it is now faithful via the public `with_flags`/`with_grow_mode` API.

- [ ] **Step 2: Build the example + run its tests**

Run: `cargo build --example tcv -j2` then `cargo test --example tcv -j2 -- --test-threads=2`
Expected: builds; existing tcv tests PASS (no behavioral regression — the panel was already effectively fixed by the desktop-filling bounds; this just makes the *intent* faithful and removes the movable/close-box decoration).

- [ ] **Step 3: Verify the running app (optional, single Bash call per the tmux gotcha)**

If verifying interactively, launch + interact + capture in ONE Bash invocation (tmux sandbox gotcha). Confirm the catalog window shows no close box and does not move on a frame drag.

- [ ] **Step 4: Commit**

```bash
git add examples/tcv.rs
git commit -m "example(tcv): fixed icon-less panel via public Window API

Drop the decoration workaround now that Axis A landed: the catalog window
uses with_flags(default)/with_grow_mode(default) for TCV's Flags/GrowMode:=0.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 4: `Group::get_help_ctx` bubbles to the current child

**Files:**
- Modify: `src/view/group.rs` (add `get_help_ctx` in `impl View for Group`, which starts at line 805)
- Test: `src/view/group.rs` `#[cfg(test)] mod tests`

**Interfaces:**
- Consumes: `Group::current()` (group.rs:137, `Option<ViewId>`), `Group::index_of(&self, ViewId) -> Option<usize>` (group.rs:153), `self.children[i].view: Box<dyn View>`; `HelpCtx::NO_CONTEXT` (help.rs:54); `View::get_help_ctx` default (view.rs:965) which falls through to `ViewState::get_help_ctx` (view.rs:452, returns `DRAGGING` while dragging else own `help_ctx`).
- Produces: a `View::get_help_ctx` override on `Group` that `Window` (delegates `to = group`, does **not** skip `get_help_ctx` — verified window.rs:971 skip list) and `Dialog` (delegates `to = window`) inherit automatically. **No `tvision-rs-macros/src/specs.rs` change** — the `get_help_ctx` forwarder already exists (specs.rs:86–87).

- [ ] **Step 1: Write the failing test**

Add to `src/view/group.rs` `mod tests` (use the existing test imports there; add `crate::help::HelpCtx` and `crate::view::SelectMode` if not already in scope). This needs a minimal selectable leaf with a settable `help_ctx`:

```rust
#[test]
fn get_help_ctx_bubbles_to_current_child() {
    const LEAF: HelpCtx = HelpCtx::custom("test.leaf");
    // Minimal selectable leaf carrying a help context.
    struct Leaf {
        st: ViewState,
    }
    impl View for Leaf {
        fn state(&self) -> &ViewState {
            &self.st
        }
        fn state_mut(&mut self) -> &mut ViewState {
            &mut self.st
        }
        fn draw(&mut self, _ctx: &mut DrawCtx) {}
    }

    let mut g = Group::new(Rect::new(0, 0, 20, 10));
    let mut st = ViewState::new(Rect::new(1, 1, 10, 3));
    st.options.selectable = true;
    st.help_ctx = LEAF;
    let leaf = Box::new(Leaf { st });
    let id = g.insert(leaf);

    // No current child yet -> group's own context (NO_CONTEXT by default).
    assert_eq!(g.get_help_ctx(), HelpCtx::NO_CONTEXT);

    // Make the leaf current -> its context bubbles up.
    let mut out = std::collections::VecDeque::new();
    let mut timers = crate::timer::TimerQueue::new();
    let mut deferred: Vec<crate::view::Deferred> = Vec::new();
    {
        let mut ctx = Context::new(&mut out, &mut timers, 0, &mut deferred);
        g.set_current(Some(id), SelectMode::Normal, &mut ctx);
    }
    assert_eq!(
        g.get_help_ctx(),
        LEAF,
        "TGroup::getHelpCtx returns current child's context"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p tvision-rs --lib -j2 -- --test-threads=2 group::tests::get_help_ctx_bubbles_to_current_child`
Expected: FAIL — the second assertion gets `NO_CONTEXT` (the un-overridden default returns the group's own context, not the child's).

- [ ] **Step 3: Add the override**

In `src/view/group.rs`, inside `impl View for Group` (after `state_mut`, near the other overrides), add:

```rust
/// The focused child's help context (recursively), falling back to the group's
/// own when there is no current child or the chain yields no context.
///
/// # Turbo Vision heritage
/// Ports `TGroup::getHelpCtx`: returns `current->getHelpCtx()`, and only when
/// that is `hcNoContext` (or there is no current) falls back to
/// `TView::getHelpCtx` (own `help_ctx`, or `DRAGGING` while dragging). The
/// dragging case is therefore preserved by the fallback leg.
fn get_help_ctx(&self) -> crate::help::HelpCtx {
    let from_current = self
        .current
        .and_then(|id| self.index_of(id))
        .map(|i| self.children[i].view.get_help_ctx());
    match from_current {
        Some(h) if h != crate::help::HelpCtx::NO_CONTEXT => h,
        _ => self.state().get_help_ctx(),
    }
}
```

- [ ] **Step 4: Run the full view + window + dialog suites to verify pass + no regression**

Run: `cargo test -p tvision-rs --lib -j2 -- --test-threads=2 group:: view:: window:: dialog::`
Expected: the new test PASSES; the existing dragging test (view.rs:1136) and all window/dialog status-line tests still PASS (the override only changes behavior when a current child has a non-default context). Then clippy + fmt clean.

- [ ] **Step 5: Confirm no macro forwarder gap**

Run: `cargo test -p tvision-rs --test delegate_view -j2 -- --test-threads=2`
Expected: PASS — confirms `Window`/`Dialog` still forward `get_help_ctx` to their inner group/window (the forwarder at specs.rs:86 is unchanged and correct). No edit to `specs.rs` is expected; if this test fails, a forwarder is missing — stop and report.

- [ ] **Step 6: Commit**

```bash
git add src/view/group.rs
git commit -m "feat(group): get_help_ctx bubbles to the current child

Port TGroup::getHelpCtx: delegate to current->getHelpCtx() with the own-state
(dragging-aware) fallback. Window/Dialog inherit via the existing delegate
forwarder. Fixes the status line showing the wrong help context for nested
focus (Axis C.1 of the consumer-API gaps).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 5: Drop `tcv`'s help-ctx caching hack; assert the status line

**Files:**
- Modify: `examples/tcv.rs` (`DataWindow::handle_event` at 668–677; the header workaround note; the search test)

**Interfaces:**
- Consumes: Task 4's `Group::get_help_ctx` bubble — the focused `DirBox`'s `help_ctx` now reaches the status line through `DataWindow`(delegates `to = dialog`) → `Dialog` → `Window` → `Group` → current child.

- [ ] **Step 1: Remove the workaround**

In `examples/tcv.rs`, delete the `handle_event` override in `impl View for DataWindow` (lines 668–677) — the help-ctx is now bubbled by the framework, so the manual cache into `self.dialog.state_mut().help_ctx` is dead. Keep the `as_any_mut` override. Update the doc comment / file-header workaround note to record that Axis C.1 made this faithful.

Verify the `DirBox` still sets its own `help_ctx` in its handler (tcv.rs:343–346) — that is the *source* the bubble reads and must stay.

- [ ] **Step 2: Re-enable / add the status-line assertion**

Ensure a tcv test asserts the status-line text follows the mode. If `search_does_not_corrupt_focused_row` (referenced in the spec) has a disabled status-line check, re-enable it; otherwise add a test that drives the list into search mode and asserts the rendered status line contains `"SEARCH MODE"`, and contains `"BROWSE MODE"` when not searching. Model the harness on the existing tcv tests (search the file for `#[test]` and reuse its program/headless setup).

- [ ] **Step 3: Build + test the example**

Run: `cargo build --example tcv -j2` then `cargo test --example tcv -j2 -- --test-threads=2`
Expected: PASS, including the status-line assertion now that the bubble works.

- [ ] **Step 4: Commit**

```bash
git add examples/tcv.rs
git commit -m "example(tcv): drop help-ctx caching hack; assert status line

Axis C.1 landed get_help_ctx bubbling, so DataWindow no longer caches the
focused list's mode into its own state. Re-enable the SEARCH/BROWSE status
line assertion.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Task 6: Update the changelog + spec status

**Files:**
- Modify: `docs/IMPLEMENTATION-LOG.md` (new top section), `docs/specs/2026-06-17-consumer-api-gaps.md` (mark Axis A + C.1 done), `docs/HANDOVER.md` ("Next" — drop the landed items)

- [ ] **Step 1: Write the log entry**

Add a new top section to `docs/IMPLEMENTATION-LOG.md` (newest first) summarizing Axis A (public Window/Dialog config) and Axis C.1 (`Group::get_help_ctx` bubble), the two tcv workarounds dropped, and the commits.

- [ ] **Step 2: Update the spec + handover**

In `docs/specs/2026-06-17-consumer-api-gaps.md`, mark Axis A and C.1 as landed (leave Axis C.2 + Axis B open). In `docs/HANDOVER.md` "Next", remove the now-landed gaps and point to Phase 2 below.

- [ ] **Step 3: Verify docs gate**

Run: `cargo xtask test` (guide doctests) and, if any `{{#rustdoc_include}}`/example doc changed, `cargo xtask docs`.
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add docs/IMPLEMENTATION-LOG.md docs/specs/2026-06-17-consumer-api-gaps.md docs/HANDOVER.md
git commit -m "docs: log Axis A + C.1 consumer-API coverage; update spec/handover

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Phase 2 (NOT in this plan): Axis B + Axis C.2 — decisions before code

Axis B (the closed `Deferred`/`Context` allowlist — generic `ExecView` + siblings) and Axis C.2 (`getData/setData/dataSize` polymorphic vs inherent) are **FOUNDATION**: they require an architectural decision before any TDD steps can be written, so they are deliberately **not** task-decomposed here. Per CLAUDE.md, FOUNDATION rows start with a read-only design investigation.

**Required next step — a design spike (read-only), producing a decision doc, not code:**
1. **Axis B keystone — result delivery for `Deferred::ExecView(Box<dyn View>)`.** Map the existing `Open*Dialog` → `pending_modal` → `ModalCompletion::*` flow in `src/view/context.rs` + `src/app/program.rs` (the C1/C8 machinery). Decide how a custom modal's result returns to the requesting view: reuse `answer_to` + `then_command`, and/or a `ModalCompletion::ExecView` carrying the boxed view back for `as_any` state-read. **This one shape becomes the template for the siblings** (B.2 arbitrary `insert` from a view, B.3 payload `message`, B.4 set-valued command ops).
2. **The architectural choice to record:** keep the allowlist (add a variant per capability, accepting the cost) **or** introduce one genuinely open capability (e.g. a deferred "run this against the owning group at apply time"). Decide deliberately; write the rationale (the "no deferred state" rule: ported OR deliberately-not-with-reason).
3. **Axis C.2 decision:** lift `gather_data`/`scatter_data` (+ a `data_size` analogue) onto the `View` trait, or document them as deliberately inherent-only and point consumers at `value`/`set_value`. Record the choice.

Once the spike resolves the result-delivery shape, write a **separate plan** (`docs/superpowers/plans/YYYY-MM-DD-consumer-api-axis-b.md`) with concrete TDD tasks for `ExecView` first, then the siblings the template unlocks.

---

## Self-Review

- **Spec coverage:** Axis A.1 (flags) + A.2 (palette) + grow/drag → Tasks 1–3. Axis C.1 (`get_help_ctx`) → Tasks 4–5. Axis C.2 + Axis B → Phase 2 spike (explicitly deferred, with the keystone identified). tcv workaround removal for both landed axes → Tasks 3 & 5. Changelog/spec/handover hygiene → Task 6. ✅ All in-scope spec items mapped; out-of-scope items explicitly routed to a spike.
- **Placeholder scan:** every code step shows real code; test bodies are complete; the only "write a test modeled on existing ones" is Task 5 Step 2 (the tcv harness is large and example-specific — the implementer reuses the file's own `#[test]` setup, which is the faithful pattern). No TBD/TODO.
- **Type consistency:** `WindowFlags`/`WindowPalette`/`GrowMode`/`DragMode`/`HelpCtx`/`SelectMode` used consistently with their definition sites; `with_*` builders consume `self` and return `Self`; `Dialog` builders delegate to `Window` setters (Task 2 consumes Task 1's exact signatures); `Group::get_help_ctx` uses `current()`/`index_of`/`children` as they actually exist.
