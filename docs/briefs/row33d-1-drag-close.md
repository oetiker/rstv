# Implementer brief — Row 33d-1: TWindow drag + close + setState command set

You are porting magiblot/tvision behavior to idiomatic Rust in the `tvision`
crate (house alias `tv::`). **Row 33 (`TWindow`) is staged.** Stages **33a**
(Group/Context primitives), **33b** (TWindow core), **33c** (zoom), the
**substrate realignment** (global `ViewId` + `find_mut`/`remove_descendant`), and
**Phase A** (`Event::Broadcast { command, source }`) are **already committed** —
build on them. This is stage **33d-1**: make the window **draggable** (mouse
move/grow) and **closable**, and extend its `setState` command-enable set. Port
**faithfully**; the only departures are the pre-decided deviations named below.
Do **not** invent features.

**33d is split.** This brief is **33d-1**. The *selection* half — `cmNext`/
`cmPrev` (TDeskTop), Alt-N (`cmSelectWindowNum`), `View::number()`,
`select_window_num`/`Group::focus_by_number` — is **33d-2**, a separate stage you
do **NOT** touch here. (Splitting at this seam keeps 33c's "enable only commands
whose handlers exist" principle clean: 33d-1 enables only `{cmClose, cmZoom}`;
33d-2 adds `{cmNext, cmPrev}` together with the desktop handler.)

## Explicitly DEFERRED — do NOT build, NO dead stubs

- **`cmNext` / `cmPrev` / Alt-N / window selection by number** → **33d-2** (next
  stage). Leave the existing `program.rs` Alt-N breadcrumb and the `desktop.rs`
  cmNext/cmPrev `TODO(row 33, D9)` breadcrumb as they are. Do **not** add
  `cmNext`/`cmPrev`/`cmResize` to the `setState` enable set in this stage.
- **`cmResize` keyboard resize sub-mode** (arrows-until-Enter/Esc — `dragView`'s
  `else` branch) → deferred (no menu can trigger `cmResize` yet, so a handler
  would be unreachable; and per 33c's principle we must not *enable* a command we
  do not handle). Leave a grep-able `TODO(33d-2/later, D9)` breadcrumb.
- **The close icon's press-and-hold release-confirm loop** (`TFrame`'s
  `while(mouseEvent(...))`) → keep the current behavior: the frame posts `cmClose`
  on mouse-**down** (the `frame.rs` `TODO(row 33, D9)` stays).
- **Modal teardown** — the `sfModal → post cmCancel` close branch is *wired* here
  (see Piece 4) but no modal window exists until **row 34**; do not build modal
  machinery, just the one branch.
- **Scrollbar auto-repeat / thumb-drag** (`scrollbar.rs` `TODO(row 31, D9)`) →
  Batch B. Untouched here.
- **Sibling tee-walk**, multi-scheme theming, `tile`/`cascade` → unchanged.

Building any of these half-wired is worse than a clean defer. Leave precise
grep-able breadcrumbs only.

## C++ source of truth (READ IT before coding)

- `/home/oetiker/scratch/tvision-spec/magiblot-tvision/source/tvision/tview.cpp`
  — `TView::dragView` (the `evMouseDown` branch only — the `else` keyboard branch
  is the deferred `cmResize` mode), `TView::moveGrow`, the static `change`,
  `TView::locate`.
- `.../source/tvision/twindow.cpp` — `TWindow::handleEvent` (the `cmClose`/
  `cmResize` arms), `TWindow::setState`, `TWindow::close`.
- `.../source/tvision/tframe.cpp` — `TFrame::handleEvent` + `TFrame::dragWindow`
  (the geometry that decides Move vs Grow vs GrowLeft vs middle-button move — you
  **replicate this geometry in `TWindow`**, see Piece 3).

You work in the **shared tree** `/home/oetiker/checkouts/rstv` (no worktree).
Touched files: `src/view/context.rs`, `src/app/program.rs`,
`src/window/window.rs`, and (mechanically) every `Context::new` call site (Piece 1
note). The `window` module is already exported — no `lib.rs`/`mod.rs` wiring.

Read first: `src/window/window.rs` (33b/33c — the `zoom`/`locate`/`range` you
extend; the `handle_event` whose `cmZoom`/kbTab arms you keep and whose
`TODO(33d)` breadcrumbs name exactly this work; `set_state`'s 33c enable set you
extend), `src/view/context.rs` (the `Context` struct + the `command_changes` /
`pending_captures` deferred channels — your **model** for the new channel),
`src/capture.rs` (`CaptureHandler`/`CaptureFlow` — `DragCapture` implements this;
note `ModalFrame` in `program.rs` as a worked capture-handler example), and
`src/app/program.rs` (`pump_once` — its destructure + deferred-drain discipline is
the pattern you extend; `ModalFrame`).

---

## Piece 1 — a deferred tree-op channel on `Context` (`pending_tree_ops`)

A capture handler holds a view only by **`ViewId`** (D3/D9) and cannot touch the
tree directly. The drag handler must (a) move/resize the window each `MouseMove`
and (b) clear `sfDragging` on `MouseUp`; the close path must remove a window. All
three are realized as a **deferred request queue** the loop drains after dispatch
— exactly mirroring the existing `pending_captures` and `command_changes`
channels.

**Add a `TreeOp` enum** (put it in `src/view/context.rs`, or `src/capture.rs` if
that reads cleaner — your call; re-export from wherever `Context` is, so
`program.rs` can name it):

```rust
/// A deferred tree mutation a capture handler / view requests through `Context`;
/// the loop applies it against the root view after dispatch (D3/D9). A view/
/// handler holds only a `ViewId`, so it cannot mutate the tree inline — it queues
/// the op and the loop resolves the id via `find_mut` / `remove_descendant`.
pub enum TreeOp {
    /// Apply new bounds to the view named by `ViewId` (drag move/grow). No ctx
    /// needed at apply time (`change_bounds` takes none).
    ChangeBounds(ViewId, Rect),
    /// Flip a propagating state flag on the view (drag end → `sfDragging` off).
    SetState(ViewId, StateFlag, bool),
    /// Remove the view from whichever group owns it (`cmClose`).
    Close(ViewId),
}
```

**Add the field + methods to `Context`** (mirror `command_changes`):

- field `pending_tree_ops: &'a mut Vec<TreeOp>` (add as the **6th `Context::new`
  parameter**, after `command_changes`).
- `pub fn request_bounds(&mut self, id: ViewId, bounds: Rect)` → push
  `ChangeBounds`.
- `pub fn request_set_state(&mut self, id: ViewId, flag: StateFlag, enable: bool)`
  → push `SetState`.
- `pub fn request_close(&mut self, id: ViewId)` → push `Close`.

Document each as "deferred — the loop applies it after the current dispatch,
mirroring [`push_capture`]/[`enable_command`]".

**EXPECTED CHURN (not scope creep):** the new `Context::new` parameter touches
**every** `Context::new` call site — all the test harnesses in `group.rs`,
`window.rs`, `frame.rs`, `desktop.rs`, `capture.rs`, `context.rs`, `view.rs`, and
the real construction sites in `program.rs`. This is mechanical, exactly like
`command_changes` was. Add a `let mut tree_ops: Vec<TreeOp> = Vec::new();` local
(or field) at each site and thread it. Do not let it read as new feature work.

---

## Piece 2 — drain + apply `pending_tree_ops` in `pump_once` (BORROW DISCIPLINE)

`Program` gains a field `pending_tree_ops: Vec<TreeOp>` (mirror
`pending_command_changes`; init `Vec::new()`), threaded into every `Context::new`
the program builds (the dispatch ctx in `pump_once`, the `with_ctx` test helper,
and the `Program::new` startup-focus ctx).

In `pump_once`, **after** the existing `pending_captures` and
`pending_command_changes` drains (still inside the `Some(ev)` arm, after the
`Context` borrow block closes), drain and apply the tree ops.

**THE #1 RISK — the row-31 destructure trap.** You **cannot** iterate
`pending_tree_ops` while a fresh apply-`Context` holds `&mut pending_tree_ops`.
**Drain into a local Vec first**, then build the ctx over the now-empty field:

```rust
// Apply deferred tree ops AFTER dispatch (same discipline as pending_captures).
// Drain to a local first: the apply-Context borrows `pending_tree_ops` (so a
// SetState/Close that re-queues lands for the NEXT pump), which would alias the
// iteration otherwise.
let ops: Vec<TreeOp> = pending_tree_ops.drain(..).collect();
if !ops.is_empty() {
    let mut ctx = Context::new(
        out_events, timers, now,
        pending_captures, pending_command_changes, pending_tree_ops,
    );
    for op in ops {
        match op {
            TreeOp::ChangeBounds(id, r) => {
                if let Some(v) = group.find_mut(id) { v.change_bounds(r); }
            }
            TreeOp::SetState(id, f, e) => {
                if let Some(v) = group.find_mut(id) { v.set_state(f, e, &mut ctx); }
            }
            TreeOp::Close(id) => { group.remove_descendant(id, &mut ctx); }
        }
    }
}
```

`group` (for the tree walk) and the `Context` backing fields are disjoint, so this
composes with the top-of-`pump_once` destructure. **One pass only** — anything an
applied op re-queues (none do, in 33d-1) waits for the next pump; do *not* loop
until empty (a bug would spin). Note the apply-ctx may push pushes/command-changes
of its own (e.g. `remove_descendant`'s `reset_current` → `set_current` →
focus broadcast lands in `out_events`); those are already drained on the next
pump, which is correct.

Mark this drain as the third member of the deferred-channel family in a comment
(`pending_captures` / `pending_command_changes` / `pending_tree_ops`).

---

## Piece 3 — `DragCapture` (the D9 replacement for `dragView`'s mouse loop)

`TFrame::dragWindow` → `TView::dragView`'s nested `while(mouseEvent(...))` loop
becomes a **capture handler** (`src/window/window.rs`). Under D3 the *frame*
cannot start the drag (it has no pointer to the window it would move); the
**window** starts it (it knows its own id and its owner's size). The capture runs
at the **capture-stack level**, *before* any group routing, so it sees mouse
events in **absolute screen coordinates**.

### 3a. Coordinate-frame assumption (document it, like `ModalFrame`)

For row 31 the root `Group` covers the whole screen at `(0,0)` and the desktop is
its child at `(0,0)`, so **absolute == root-local == desktop-local**, and a
window's `origin` (relative to its owner) is in that same frame. The drag math
assumes this. **Add a doc comment** on `DragCapture` stating the assumption and
pointing at `ModalFrame`'s identical caveat: when a menu/status bar (Phase 4)
shifts the desktop off `(0,0)`, the capture must translate absolute → desktop
coords; revisit then. (The window cannot know the desktop's offset under D3, so we
do not attempt it now.)

### 3b. The `DragKind` + `DragCapture` types

```rust
/// Which `dragView` form is running (mouse branch only; the keyboard `cmResize`
/// sub-mode is deferred). Selects how each `MouseMove` maps to new bounds.
enum DragKind { Move, Grow, GrowLeft }

struct DragCapture {
    window_id: ViewId,
    kind: DragKind,
    /// Window bounds at drag start (the fixed corner for Grow/GrowLeft).
    init_bounds: Rect,
    /// The constant grab offset (see 3d). Per-kind meaning documented there.
    anchor: Point,
    /// `owner->getExtent()` — captured at push time from `ctx.owner_size()`.
    limits: Rect,
    /// `sizeLimits()` of the window — captured at push time.
    min: Point,
    max: Point,
    /// `owner->dragMode | (flags & ...)` — only the `dmLimit*` bits matter to
    /// `move_grow` (the window's default is `dmLimitLoY`).
    mode: DragMode,
}
```

`impl CaptureHandler for DragCapture`:

```rust
fn handle(&mut self, ev: &mut Event, ctx: &mut Context) -> CaptureFlow {
    match ev {
        Event::MouseMove(m) => {
            let r = self.compute_bounds(m.position); // m.position is ABSOLUTE here
            ctx.request_bounds(self.window_id, r);
            CaptureFlow::Consumed
        }
        Event::MouseUp(_) => {
            // dragView's loop ends on mouse-up; clear sfDragging (deferred — a
            // capture holds no &mut view) and pop ourselves.
            ctx.request_set_state(self.window_id, StateFlag::Dragging, false);
            CaptureFlow::ConsumedPop
        }
        // C++ `mouseEvent(event, evMouseMove)` discards everything that is not a
        // mouse-move/up while the drag runs — the drag is modal. Swallow the rest.
        _ => CaptureFlow::Consumed,
    }
}
fn view(&self) -> Option<ViewId> { Some(self.window_id) }
```

> **NOTE — MouseMove only.** Faithful to `mouseEvent(event, evMouseMove)`:
> `MouseAuto`, keys, commands, broadcasts are *swallowed* (`Consumed`), **not**
> used to update bounds. Only `MouseMove` moves the window; only `MouseUp` ends
> the drag.

### 3c. `move_grow` — faithful port of `TView::moveGrow` (free fn in window.rs)

```rust
/// `TView::moveGrow` (tview.cpp) — clamp size to [min,max] and origin to the
/// limits, honoring the `dmLimit*` mode bits, and return the resulting bounds.
/// We return the rect instead of calling `locate` (the capture has no &mut view;
/// the loop applies it via change_bounds — equivalent, since `move_grow` already
/// clamps to the same sizeLimits `locate` would).
fn move_grow(mut p: Point, mut s: Point, limits: Rect, min: Point, max: Point, mode: DragMode) -> Rect {
    // C++ uses min(max(..)) NOT clamp(): when lo > hi (window larger than the
    // limit), min(max(v,lo),hi) yields `hi`. `i32::clamp` PANICS on lo>hi, so do
    // NOT use it — replicate min/max exactly.
    s.x = s.x.max(min.x).min(max.x);
    s.y = s.y.max(min.y).min(max.y);
    p.x = p.x.max(limits.a.x - s.x + 1).min(limits.b.x - 1);
    p.y = p.y.max(limits.a.y - s.y + 1).min(limits.b.y - 1);
    if mode.limit_lo_x { p.x = p.x.max(limits.a.x); }
    if mode.limit_lo_y { p.y = p.y.max(limits.a.y); }
    if mode.limit_hi_x { p.x = p.x.min(limits.b.x - s.x); }
    if mode.limit_hi_y { p.y = p.y.min(limits.b.y - s.y); }
    Rect::from_points(p, p + s)
}
```

### 3d. `DragCapture::compute_bounds(mouse_abs: Point) -> Rect` (per-kind anchor)

Replicates `dragView`'s three mouse forms. `mouse_abs` is the absolute pointer
position of the current `MouseMove`. The anchor is the **constant** grab offset
captured at push time (see Piece 3e). `o = init_bounds.a` (origin), `sz =
init_bounds.b - init_bounds.a` (size).

- **Move** (`p = origin - mouseDown; origin' = mouse + p`):
  `let new_origin = mouse_abs + self.anchor;`
  `move_grow(new_origin, sz, limits, min, max, mode)`
- **Grow** (`p = size - mouseDown; size' = mouse + p`):
  `let new_size = mouse_abs + self.anchor;`
  `move_grow(o, new_size, limits, min, max, mode)`
- **GrowLeft** (bespoke pre-clamp, then move_grow):
  C++:
  ```cpp
  bounds = getBounds(); s = origin; s.y += size.y; p = s - mouseDown;  // anchor
  // per move:
  mouse += p;
  bounds.a.x = min(max(mouse.x, bounds.b.x - maxSize.x), bounds.b.x - minSize.x);
  bounds.b.y = mouse.y;
  moveGrow(bounds.a, bounds.b - bounds.a, limits, min, max, mode);
  ```
  Rust:
  ```rust
  let corner = mouse_abs + self.anchor;                 // = mouse + (botLeft - mouseDown)
  let b = self.init_bounds.b;                            // fixed top-right anchor
  let ax = corner.x.max(b.x - self.max.x).min(b.x - self.min.x);
  let a = Point::new(ax, self.init_bounds.a.y);          // a.y stays initial
  let by = corner.y;                                     // bottom edge follows mouse
  move_grow(a, Point::new(b.x - a.x, by - a.y), self.limits, self.min, self.max, self.mode)
  ```

### 3e. Anchor values (captured at push time, in the window — see Piece 3-window)

`mouse_down_abs = m.position + window.origin` (the window sees the mouse-down in
**window-local** coords; add its own origin to get absolute). Then:

- **Move:** `anchor = window.origin - mouse_down_abs` (`= -m.position`).
- **Grow:** `anchor = window.size - mouse_down_abs`.
- **GrowLeft:** `anchor = Point::new(window.origin.x, window.origin.y + window.size.y) - mouse_down_abs`
  (the bottom-left corner minus the mouse-down).

---

## Piece 3 (window side) — start the drag in `Window::handle_event`

In `Window::handle_event`, **after** `self.group.handle_event(ev, ctx)` and after
the existing `cmZoom` / kbTab arms (order is fine — those fire on Command/Key
events, the drag on a surviving MouseDown), add drag detection.

**Why "after group delegation"**: the desktop delivered the `MouseDown` to the
window in window-local coords; the window delegates to its group, which routes it
positionally to the **frame** (the bottom-most child filling the extent). The
frame consumes a close/zoom-icon click (→ `Nothing`) but **leaves a title-bar /
bottom-corner click unconsumed** (the `frame.rs` `// else: wfMove …` and bottom-
row `TODO` cases). A click on an interior child (scrollbar, etc.) is consumed
there. So **if `*ev` is still a live `MouseDown` after group routing**, it is a
drag spot, and its position is window-local. (An *inactive* window never reaches
here on its first click: the desktop's positional auto-select consumes the
selecting click — `first_click` is false — so the drag only ever starts on the
already-active window. This is faithful and means you need no `sfActive` re-check.)

Replicate `TFrame::handleEvent`'s geometry to pick the kind (`w = size.x`,
`h = size.y`, `m.position` window-local):

```rust
if let Event::MouseDown(m) = *ev {
    let (w, h) = (self.group.state().size.x, self.group.state().size.y);
    let pos = m.position;
    let kind = if m.buttons.middle
        && self.flags.r#move
        && pos.x > 0 && pos.x < w - 1 && pos.y > 0 && pos.y < h - 1
    {
        Some(DragKind::Move)                            // middle-button interior move
    } else if pos.y == 0 && self.flags.r#move {
        Some(DragKind::Move)                            // title-bar move
    } else if pos.y >= h - 1 && self.flags.grow && pos.x >= w - 2 {
        Some(DragKind::Grow)                            // bottom-right grow
    } else if pos.y >= h - 1 && self.flags.grow && pos.x <= 1 {
        Some(DragKind::GrowLeft)                        // bottom-left grow
    } else {
        None
    };
    if let Some(kind) = kind && let Some(id) = self.group.state().id() {
        self.start_drag(id, kind, m.position, ctx);
        ev.clear();
    }
}
```

> Order vs C++ `TFrame::handleEvent`: C++ checks `y==0` (move) first, then bottom
> row (grow), then middle-button. The branches are mutually exclusive by geometry,
> so any consistent order is equivalent; the middle-button guard `0 < x < w-1 &&
> 0 < y < h-1` keeps it from overlapping the title/corner cases. Match C++'s flag
> guards exactly: title move needs `wfMove`; bottom corners need `wfGrow`; middle
> move needs `wfMove`.

`start_drag` (a private `Window` method) sets `sfDragging` **on** (it has `&mut
self` + `ctx`, so it can call `set_state` directly — which propagates to the
frame, flipping it to the single-line dragging border) and pushes the deferred
capture:

```rust
fn start_drag(&mut self, id: ViewId, kind: DragKind, mouse_local: Point, ctx: &mut Context) {
    // dragView: setState(sfDragging, True). The window has &mut self here, so set
    // it directly (Group::set_state propagates Dragging to children incl. frame).
    View::set_state(self, StateFlag::Dragging, true, ctx);

    let origin = self.group.state().origin;
    let size = self.group.state().size;
    let mouse_abs = mouse_local + origin;           // window-local -> absolute (3a assumption)
    // owner->getExtent() and sizeLimits(), via the owner-extent-down channel +
    // the window's size_limits override. owner_size is valid HERE (group-routed
    // dispatch); the capture must NOT read it at drag time (3a).
    let owner_size = ctx.owner_size();
    let limits = Rect::new(0, 0, owner_size.x, owner_size.y);  // owner->getExtent()
    let (min, max) = View::size_limits(self, owner_size);
    // dragMode | (flags & (wfMove|wfGrow)) — only dmLimit* bits feed move_grow.
    let mode = self.group.state().drag_mode;        // ctor default dmLimitLoY
    let anchor = match kind {
        DragKind::Move     => origin - mouse_abs,
        DragKind::Grow     => size - mouse_abs,
        DragKind::GrowLeft => Point::new(origin.x, origin.y + size.y) - mouse_abs,
    };
    let init_bounds = self.group.state().get_bounds();
    ctx.push_capture(Box::new(DragCapture { window_id: id, kind, init_bounds, anchor, limits, min, max, mode }));
}
```

> The capture is pushed **deferred**, so it sees the *next* event (the first
> `MouseMove`), never this `MouseDown` — exactly the `pending_captures` contract.
> The `MouseDown` is consumed (`ev.clear()`), so normal routing stops.

---

## Piece 4 — `cmClose` in `Window::handle_event`

Add a `cmClose` arm alongside `cmZoom` (after `group.handle_event`). Faithful to
`TWindow::handleEvent`'s `cmClose` case + `TWindow::close`:

```rust
if let Event::Command(c) = *ev
    && c == Command::CLOSE
    && self.flags.close
{
    ev.clear();                                     // C++ clears first
    if self.group.state().state.modal {
        // sfModal: re-issue as cmCancel (row 34 owns modal teardown).
        ctx.post(Command::CANCEL);
    } else if self.valid(Command::CLOSE) {          // close(): if valid(cmClose)
        if let Some(id) = self.group.state().id() {
            ctx.request_close(id);                   // loop drains -> remove_descendant
        }
    }
}
```

> **No target guard** (`infoPtr == 0 || == this`). Phase A proved it vacuous: the
> frame posts `cmClose` only while `sfActive`, and `Event::Command` is focused-
> routed to the desktop's `current` (= active) window — so a `cmClose` always
> reaches exactly its target. Re-state this invariant in a doc comment (the same
> trip-wire `cmZoom` already documents in 33c). **Do NOT add a target to
> `Event::Command`.**

Update the stale `cmClose` breadcrumb in `window.rs` (the one still saying "needs
a close-removal channel" — the channel is `request_close`/`remove_descendant`
now).

---

## Piece 5 — extend `Window::set_state`'s command-enable set

33c enables only `cmZoom` on `sfSelected`. Extend to the **33d-1 subset** of the
C++ `TWindow::setState` window-command set:

```rust
if flag == StateFlag::Selected {
    self.group.set_state(StateFlag::Active, enable, ctx);
    // Window commands enabled together while selected (C++ enableCommands).
    // 33d-1 subset: cmClose (if wfClose), cmZoom (if wfZoom) — both handled here.
    // DEFERRED to 33d-2: cmNext, cmPrev (need the TDeskTop handler).
    // DEFERRED (no handler): cmResize.
    let mut toggle = |cmd: Command, cond: bool| {
        if cond {
            if enable { ctx.enable_command(cmd); } else { ctx.disable_command(cmd); }
        }
    };
    toggle(Command::CLOSE, self.flags.close);
    toggle(Command::ZOOM, self.flags.zoom);
}
```

(Keep the existing `cmZoom` behavior; just add `cmClose`. Mirror however reads
cleanest — the closure is a suggestion, an explicit `if`/`else` is equally fine.)

---

## Deviations in force (do not re-derive)
- **D9** single loop + capture stack: the drag is a `CaptureHandler`, not a nested
  loop. The capture holds a `ViewId`, never a view ref.
- **D3** no up-pointers: the window names itself via `self.state().id()`; owner
  extent via `ctx.owner_size()` (captured at push); tree mutations via the
  deferred `TreeOp` channel resolved by the loop through `find_mut`/
  `remove_descendant`.
- **D8** whole-tree redraw: `change_bounds`'s C++ shadow/`drawUnderRect` tail is
  already dropped; the loop repaints every pass.
- **D4** broadcast `source` is unrelated to this stage; `Event::Command` carries
  only the `Command` (no target — Piece 4).

## Conventions
- English for all code/comments/identifiers.
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
  must all pass.
- Faithful by default; the only deviations are the D-rules named above.

---

## Tests (this is the verification — Appendix B step 4)

The whole bug surface is the **deferred round-trip**: poll → capture consumes
`MouseMove` → `request_bounds` → `pending_tree_ops` drain → `find_mut().change_bounds()`
→ render. A unit test of `DragCapture::handle` in isolation proves **nothing**
about the drain, the borrow discipline, or the deferred-push timing. So the core
tests must **drive `pump_once`** end-to-end on a `HeadlessBackend`.

Add to `src/app/program.rs` tests (reuse the `program_with_desktop` harness;
insert a real `Window` into the **root** group and `set_current` it, exactly as
the existing probe tests insert into `group_mut()`):

1. **Drag move round-trip (mandatory).** Insert a `Window` (e.g. bounds
   `(2,1,22,9)`, `wfMove`) into the root group, select it (so its frame is
   active). Pump a `MouseDown` on the title bar (window-local `y==0`, e.g. abs
   `(8,1)` → window-local `(6,0)`), assert after the pump: the window's
   `state.dragging` is **true** and `capture_len()` is **1** (deferred push
   applied). Pump a `MouseMove` to a new absolute position, assert the window's
   `origin` **moved** by the expected delta (compute via the Move anchor:
   `new_origin = mouse_abs - mouse_local_down`). Pump a second `MouseMove`, assert
   it tracked again. Pump a `MouseUp`, assert `state.dragging` is **false** and
   `capture_len()` is **0** (ConsumedPop + the deferred `SetState` applied). Read
   window state via `group_mut().find_mut(window_id).unwrap().state()`.
2. **Drag clamps to limits.** Drag the window's origin toward a negative/edge
   position and assert `move_grow`'s clamp held it on-screen (origin.y never above
   `limits.a.y` per `dmLimitLoY`; origin within the general `[a-s+1, b-1]` band).
3. **Close round-trip.** Insert + select a `wfClose` window. Pump
   `Event::Command(Command::CLOSE)` (enable it first via select, or post directly).
   Assert after the pump that `find_mut(window_id)` is now `None` (the window was
   removed via `remove_descendant`). Also: a `sfModal` window (set
   `state.modal = true`) instead posts `cmCancel` and is **not** removed.

Add to `src/window/window.rs` tests:

4. **`set_state` enables/disables `cmClose`** (extend the existing `cmZoom` test):
   selecting a `wfClose` window queues `(Command::CLOSE, true)`; deselecting queues
   `(Command::CLOSE, false)`.
5. **Drag-start detection** (unit, no pump): build a `Window`, mark its group
   active, call `handle_event` with a title-bar `MouseDown` (window-local `y==0`),
   assert `state.dragging` is true afterward, the event is consumed, and one
   handler is in the `pending_captures` Vec (the harness's local). A bottom-corner
   `MouseDown` with `wfGrow` starts a `Grow`/`GrowLeft`; an interior non-edge click
   starts nothing.
6. **`move_grow` unit tests** (pure fn): a couple of direct cases — a clamp where
   the window is larger than the limits (proves `min(max())` not `clamp()` — must
   NOT panic), and an ordinary in-range move.
7. Keep all existing tests green (the `Context::new` arity bump touches their
   harnesses).

Snapshots: no new visual snapshot is strictly required (drag is positional, not a
new glyph), but a `dragging`-state frame snapshot is already covered by `frame.rs`.
If you add one, drive it through the real `Renderer` + `HeadlessBackend` per the
frozen format.

When done: run the full gate, list the test count, and hand back for the two-stage
review (spec-compliance, then code-quality).
