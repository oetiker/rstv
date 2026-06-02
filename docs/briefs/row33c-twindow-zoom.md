# Implementer brief — Row 33c: TWindow zoom (the first interactive command)

You are porting magiblot/tvision behavior to idiomatic Rust in the `tvision`
crate (house alias `tv::`). **Row 33 (`TWindow`) is staged.** Stages **33a**
(Group/Context primitives: deferred command-enable channel, Z-reorder
`make_first`/`put_in_front_of`, `ofTopSelect` raise-on-select) and **33b** (the
`TWindow` core — static, selectable window) are **already committed** — build on
them. This is stage **33c**: make the window **zoomable** (the simplest
interactive command), plus the owner-extent-down channel and the downcast seam
that zoom needs (and that 33d's drag will reuse). Port **faithfully**; the only
departures are the pre-decided deviations named below. Do **not** invent features.

## Explicitly DEFERRED to 33d / row 34 — do NOT build, NO dead stubs

- **`cmResize` / move / grow drag loops** → 33d (needs transient drag capture
  handlers; the live loop + capture stack make them buildable, but they are 33d).
- **`close()` / `destroy(this)` self-removal** → 33d (needs the close-removal
  channel on `Context` + `Group`).
- **`cmNext` / `cmPrev`** window cycling (and TDeskTop's deferred handling) → 33d.
- **The `sfModal` → post `cmCancel`** close path → row 34 (modal teardown).
- **`cmSelectWindowNum`** broadcast match → deferred (D4 dropped event payloads;
  the Alt-N deferral already noted in `program.rs`).
- **Multi-scheme theming** (`WindowPalette::Cyan/Gray` → distinct roles) → row 34.

Building any of these half-wired here is worse than a clean defer (the advisor's
explicit guidance, same as 33b). Leave precise grep-able breadcrumbs only.

## C++ source of truth (read it)
- `/home/oetiker/scratch/tvision-spec/magiblot-tvision/source/tvision/twindow.cpp`
  — `TWindow::zoom`, `TWindow::sizeLimits`, `TWindow::setState`,
  `TWindow::handleEvent` (the `cmZoom` arm).
- `.../source/tvision/tview.cpp` — `TView::locate` (line ~585), `TView::sizeLimits`
  (line ~829), and the static `range` helper.

You work in the **shared tree** `/home/oetiker/checkouts/rstv` (no worktree).
Touched files: `src/view/context.rs`, `src/view/group.rs`, `src/view/view.rs`,
`src/frame.rs`, `src/window/window.rs`. (No `lib.rs`/`mod.rs` wiring needed — the
`window` module is already exported.)

Existing relevant code to read first: `src/window/window.rs` (33b core, with the
`TODO(33c)` breadcrumbs naming exactly this work), `src/view/context.rs` (the
`Context` struct + the 33a `command_changes` channel — your model for the new
field), `src/view/group.rs` (`handle_event` three-phase router, `set_state`,
`insert`, `index_of`), `src/frame.rs` (`Frame::set_zoomed` already exists),
`src/view/view.rs` (the `View` trait, `calc_bounds`/`change_bounds`/`size_limits`,
the private `range` static), `src/app/program.rs` (test #9 already proves the
enable→route path — see "Tests" below).

---

## Piece 1 — owner-extent-down channel on `Context` (a defaulted field + setter)

`zoom` needs `owner->size` (= `maxSize`), which a child cannot reach upward (D3).
Add a **transient** owner-extent to `Context`.

**DECISION (advisor-sharpened): a defaulted field + setter, NOT a 6th
`Context::new` parameter.** Rationale: `Group::handle_event` must *mutate*
`owner_size` through `&mut Context` (set-before-deliver / restore-on-exit), so a
setter exists regardless; a constructor param would then always be overwritten by
the root group before it is ever read — pure churn across ~17 call sites. It is
also conceptually distinct from the other four fields: those are `&'a mut`
channels to **loop-owned persistent** state; `owner_size` is **transient routing**
state mutated *during* a single dispatch. Keep it out of the disjoint-borrow param
list.

In `src/view/context.rs`, on `struct Context`:
```rust
/// The size of the view's owner (the group currently routing to it), so a child
/// can reach `owner->size` / `owner->getExtent()` without an up-pointer (D3).
/// Used by `TWindow::zoom`/`sizeLimits` (33c) and the drag limits (33d).
///
/// **Transient routing state**, NOT a loop-owned channel: each
/// `Group::handle_event` sets it to its own size before delivering to children
/// and restores it on exit (so nesting root→desktop→window works). It is valid
/// **only during group-routed dispatch**; a capture handler runs *before* group
/// routing and sees the default `(0,0)`. That is fine — 33d's drag handler must
/// capture its limits at *push time* (inside the window's `handle_event`, where
/// `owner_size` is correctly set), never read them at drag time.
owner_size: Point,
```
- Initialize it to `Point::default()` (= `(0,0)`) in `Context::new` (signature
  **unchanged**).
- Add `pub fn owner_size(&self) -> Point { self.owner_size }` and
  `pub fn set_owner_size(&mut self, size: Point) { self.owner_size = size; }`.
- Add a small unit test (mirror `context_command_changes_queue_*`): set + read
  back, and confirm `Context::new` defaults it to `(0,0)`.

## Piece 2 — `Group::handle_event` sets + restores `owner_size`

In `src/view/group.rs`, `impl View for Group { fn handle_event(...) }`: bracket
the whole routing body with save → set-to-own-size → (route) → **restore on every
exit path**.

```rust
fn handle_event(&mut self, ev: &mut Event, ctx: &mut Context) {
    let saved_owner_size = ctx.owner_size();
    ctx.set_owner_size(self.st.size);   // children's owner is THIS group
    // ... existing three-phase match ...
    ctx.set_owner_size(saved_owner_size);
}
```

**CRITICAL (advisor): the restore must be unconditional.** The positional arm
currently has `let Some(pos) = mouse_pos(ev) else { return; };` — a set-at-top /
restore-at-bottom bracket means that early `return` (and any future one) leaves
`owner_size` corrupted for the parent group's *subsequent* siblings. That `else`
is presently unreachable, but **do not rely on unreachability** — it is a landmine
for the next editor. Restructure so the restore always runs: extract the match
into an inner helper (a nested `fn`/closure or a private method returning `()`) and
restore after it returns, **or** rewrite the positional arm to fall through
instead of `return`ing. Pick the cleaner one; leave a one-line comment on why the
restore is unconditional.

This changes behavior only for `owner_size` reads (nothing reads it yet besides
33c's zoom), so all existing group/desktop/window/program tests must stay green.

## Piece 3 — the downcast seam (zoom pushes `set_zoomed` to the frame child)

`zoom` must mutate the window's frame child after construction. Add the minimal
object-safe seam.

In `src/view/view.rs`, on the `View` trait (defaulted → **no ripple**):
```rust
/// Downcast hook for the rare owner→child push that needs the concrete type
/// (e.g. `TWindow::zoom` pushing `set_zoomed` to its `TFrame`). Base returns
/// `None`; only views that must be reached concretely override it. (`Any`
/// requires `'static`, which every view is.)
fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> {
    None
}
```
In `src/frame.rs`, `impl View for Frame`, override:
```rust
fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> {
    Some(self)
}
```
In `src/view/group.rs`, add a child accessor (mirror the private `index_of` /
the `#[cfg(test)] child_state_mut`, but this one is **non-test** — `Window` uses
it):
```rust
/// Mutably borrow child `id`'s view (for an owner→child push that needs the
/// concrete type via [`View::as_any_mut`]). `None` for a stale/foreign id.
pub fn child_mut(&mut self, id: ViewId) -> Option<&mut dyn View> {
    let i = self.index_of(id)?;
    Some(self.children[i].view.as_mut())
}
```
(Check the exact `Box<dyn View>` deref — `self.children[i].view.as_mut()` yields
`&mut dyn View`.)

Add a small test that `Frame::as_any_mut().downcast_mut::<Frame>()` is `Some` and
a default view's `as_any_mut()` is `None`.

## Piece 4 — `Window::zoom` + `Window::locate` (faithful `TWindow::zoom`)

In `src/window/window.rs`. C++:
```cpp
void TWindow::zoom() {
    TPoint minSize, maxSize;
    sizeLimits( minSize, maxSize );        // max = owner->size (virtual → TWindow::sizeLimits)
    if( size != maxSize ) {
        zoomRect = getBounds();
        TRect r( 0, 0, maxSize.x, maxSize.y );
        locate(r);
    } else
        locate( zoomRect );
}
```
Realize as a private method taking `ctx` (for `owner_size`):
```rust
fn zoom(&mut self, ctx: &mut Context) {
    let owner_size = ctx.owner_size();
    let (_min, max) = self.size_limits(owner_size);   // Window override: min=(16,6), max=owner_size
    let size = self.group.state().size;
    if size != max {
        self.zoom_rect = self.group.state().get_bounds();
        self.locate(Rect::new(0, 0, max.x, max.y), owner_size);
    } else {
        let zr = self.zoom_rect;
        self.locate(zr, owner_size);
    }
    // D3: the C++ TFrame::draw recomputes `owner->size == maxSize` every draw to
    // pick the zoom vs unzoom icon. We can't read the owner, so push the bool down.
    let zoomed = self.group.state().size == max;
    if let Some(frame) = self
        .group
        .child_mut(self.frame_id)
        .and_then(|v| v.as_any_mut())
        .and_then(|a| a.downcast_mut::<Frame>())
    {
        frame.set_zoomed(zoomed);
    }
}
```
`zoom_rect` is no longer dead → drop its `zoom_rect()` accessor only if it becomes
otherwise-read; simplest is to keep the accessor (harmless). `frame_id` likewise
now has a real consumer.

**`locate` — port `TView::locate`** (the owner≠0 `drawUnderRect` is D8-moot):
```rust
/// `TView::locate` — clamp `bounds`'s size to `sizeLimits`, then `change_bounds`
/// iff it differs. `owner_size` feeds the (overridden) `size_limits`. The C++
/// `owner != 0` shadow/redraw tail is dropped (D8: whole-tree redraw + diff).
fn locate(&mut self, mut bounds: Rect, owner_size: Point) {
    let (min, max) = self.size_limits(owner_size);
    bounds.b.x = bounds.a.x + range(bounds.b.x - bounds.a.x, min.x, max.x);
    bounds.b.y = bounds.a.y + range(bounds.b.y - bounds.a.y, min.y, max.y);
    if bounds != self.group.state().get_bounds() {
        self.group.change_bounds(bounds);   // faithful: TGroup::changeBounds (resizes children)
    }
}
```
**`range`**: the `tview.cpp` static `range(val, min, max)` is private to `view.rs`.
Reimplement it locally in `window.rs` (it is two lines) with a `// tview.cpp
`range`` citation — keeps the seam contained. (Alternatively `pub(crate)` the
existing one; local is fine and preferred.)

> **Staleness breadcrumb (33d):** the pushed `zoomed` bool goes stale if the
> desktop resizes while the window is zoomed (C++ recomputes it every draw). Out
> of scope for 33c. Add `// TODO(33d): re-push set_zoomed on owner resize /
> change_bounds (pushed bool vs C++'s per-draw recompute).`

## Piece 5 — `Window::handle_event` handles `cmZoom`

C++ does `TGroup::handleEvent(event)` **first**, then the command switch, then the
keydown switch. Keep that order (the existing kbTab handling stays):
```rust
fn handle_event(&mut self, ev: &mut Event, ctx: &mut Context) {
    self.group.handle_event(ev, ctx);
    // C++ cmZoom arm: zoom() + clearEvent. The C++ `infoPtr == 0 || == this`
    // guard is gone (D4 dropped payloads) — a cmZoom that routes here is for us.
    if let Event::Command(c) = *ev
        && c == Command::ZOOM
        && self.flags.zoom
    {
        self.zoom(ctx);
        ev.clear();
    }
    // existing kbTab / kbShiftTab focus cycling (unchanged) ...
    if let Event::KeyDown(k) = *ev
        && k.key == Key::Tab
    {
        self.group.focus_next(k.modifiers.shift, ctx);
        ev.clear();
    }
}
```
(A consumed event is `Nothing`, so each branch self-guards. Replace the old
`TODO(33c) cmZoom` breadcrumb; leave the `cmResize`/`cmClose`/`cmSelectWindowNum`
ones, retagged `TODO(33d)` / `row 34` as listed in the deferral section.)

## Piece 6 — `Window::set_state` enables/disables `cmZoom` (33a channel)

C++ `TWindow::setState` enables the full set `{cmNext, cmPrev, cmResize(if
grow|move), cmClose(if close), cmZoom(if zoom)}` atomically on `sfSelected`.

**DIVERGENCE (state this prominently — the spec reviewer WILL check it against
C++):** 33c enables **only `cmZoom`** (the one command whose handler now exists).
The rest are enabled in 33d/row-34 as their handlers land. Enabling a command
whose handler is absent would be an *inert* command — the pump would route it to a
window that ignores it (or filter it), a worse state than leaving it disabled.
This is the documented "enable only commands whose handlers exist" staging.

```rust
fn set_state(&mut self, flag: StateFlag, enable: bool, ctx: &mut Context) {
    self.group.set_state(flag, enable, ctx);
    if flag == StateFlag::Selected {
        self.group.set_state(StateFlag::Active, enable, ctx);
        // C++ enables {cmNext,cmPrev, cmResize if grow|move, cmClose if close,
        // cmZoom if zoom} atomically here. STAGED: enable only commands whose
        // handlers exist, to avoid inert commands.
        //   33c: cmZoom (handler in handle_event).
        //   TODO(33d): cmResize (if grow|move), cmClose (if close), cmNext, cmPrev.
        if self.flags.zoom {
            if enable {
                ctx.enable_command(Command::ZOOM);
            } else {
                ctx.disable_command(Command::ZOOM);
            }
        }
    }
}
```

---

## Tests (verification — D11)

Match the `window.rs` / `program.rs` test-harness idiom (the `with_ctx` helper;
`DrawCtx`/`Renderer`/`HeadlessBackend`; `insta::assert_snapshot!`).

1. **`Context::owner_size`** round-trips and defaults to `(0,0)` (context.rs).
2. **`Group::handle_event` restores `owner_size`** — a parent group with a child
   group: after routing an event to the child, the parent's `owner_size` (as seen
   by a *later* sibling delivery, or just by reading `ctx.owner_size()` right after
   `handle_event` returns) is back to the parent's size, not the child's. (You may
   need a tiny probe that records `ctx.owner_size()` during its `handle_event`.)
3. **`as_any_mut` seam**: `Frame::as_any_mut().downcast_mut::<Frame>()` is `Some`;
   a plain view's is `None`. `Group::child_mut(frame_id)` resolves.
4. **`setState` enables/disables cmZoom**: select the window with a `ctx` whose
   `command_changes` you can inspect → contains `(Command::ZOOM, true)`; deselect →
   `(Command::ZOOM, false)`. (If `flags.zoom` were false, no change queued.)
5. **`zoom()` toggles bounds** (the core): build a `Window` smaller than its owner;
   `ctx.set_owner_size(desktop_size)`; feed `Event::Command(Command::ZOOM)` to
   `handle_event` (the window must be **selected/active** first, and you set
   `owner_size` on the ctx directly — see the harness note below). Assert:
   - first zoom: bounds become `(0,0,owner.x,owner.y)`; `zoom_rect` saved the
     original bounds; the frame child's pushed `zoomed` is `true`.
   - second zoom (toggle): bounds restored to the original; frame `zoomed` is
     `false`.
   - Read the frame's `zoomed` via `Group::child_mut(frame_id)` + `as_any_mut` +
     `downcast_ref`/`zoomed()`, or expose it through a small test accessor.
6. **`size_limits` honours owner for max** (already covered in 33b — don't
   duplicate; the zoom test exercises it transitively).
7. **Mandatory milestone snapshot**: one `Window` (title + scrollbar) on a
   `Desktop` (or standalone), rendered **restored** vs **zoomed-to-fill**, two
   snapshots. Drive zoom by selecting + feeding `cmZoom` with `owner_size` set.
   Render through `&mut dyn View`.

### Test-harness note (advisor — this trips people)
**Do NOT test zoom via a real click→post→re-enter chain.** `TWindow` sets
`ofSelectable|ofTopSelect` but **not** `ofFirstClick`, so `Group`'s auto-select
**consumes the first (selecting) click** — it never reaches the frame; only a
*second* click would post `cmZoom`, and the enable+post+re-enter spans multiple
pumps. So in tests: **select the window directly** (`View::set_state(Selected,
true, ctx)`), **`ctx.set_owner_size(desktop_size)`**, then **feed
`Event::Command(Command::ZOOM)` straight to `Window::handle_event`** (or call a
test-visible `zoom`). The "an enabled command reaches routing instead of being
filtered" behavior is **already proven by `program.rs` test #9
(`ctx_enable_command_applies_after_dispatch_and_unblocks_routing`)** — do **not**
re-prove it here.

## Definition of done (run; all must pass)
- `cargo test` — all green (existing 262 + your new ones).
- `cargo clippy --all-targets -- -D warnings` — clean. No dead fields/`#[allow]`:
  `frame_id`/`zoom_rect` now have real consumers (zoom); if anything would be dead,
  add a real consumer rather than silencing.
- `cargo fmt --check` — clean.

## Deviations in play
- **D2** embed-and-delegate (`Window` embeds `Group`).
- **D3** owner-data-down: the new `owner_size` channel replaces `owner->size`;
  the downcast seam replaces the direct `frame->` pointer.
- **D4** events carry no payload → the `cmZoom` `infoPtr` guard is dropped.
- **D7** single blue scheme; multi-scheme deferred (row 34).
- **D8** whole-tree redraw → `locate`'s `drawUnderRect` shadow/redraw tail dropped.
- **D1** string commands; command-enable via the 33a deferred channel; the C++
  full-set enable is **staged** (cmZoom only at 33c).

Report **DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED** with what you built,
any faithfulness judgment calls (the `owner_size` field-vs-param choice, the
`range` reimplementation, the cmZoom-only staging, the `set_zoomed` staleness gap),
and the three gate results. If something forces premature 33d infra (drag capture,
close-removal channel), **STOP and report** rather than building it half-way.
