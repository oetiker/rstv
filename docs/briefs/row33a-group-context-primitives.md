# Implementer brief — Row 33a: Group/Context primitives for TWindow

You are porting magiblot/tvision behavior to idiomatic Rust in the `tvision`
crate (house alias `tv::`). **Row 33 (`TWindow`) is being implemented in stages.**
This is **stage 33a**: the **`Group` + `Context` layer primitives** the window core
(33b) depends on. Three changes, all FOUNDATION-sensitive. Port **faithfully**
from the C++; the only intentional departures are the pre-decided deviations
named below. Do **not** build any of `TWindow` itself — that is 33b.

C++ sources of truth (read them):
- `/home/oetiker/scratch/tvision-spec/magiblot-tvision/source/tvision/tview.cpp` —
  `TView::select`, `TView::focus`, `TView::makeFirst`, `TView::putInFrontOf`.
- `/home/oetiker/scratch/tvision-spec/magiblot-tvision/source/tvision/tgroup.cpp` —
  `TGroup::focusNext`, `selectNext`, `setCurrent`, `resetCurrent`, `firstMatch`,
  `insertView`/`removeView`.

You work in the shared tree `/home/oetiker/checkouts/rstv` (no worktree; you are
the only agent this stage). Files you will touch: `src/view/context.rs`,
`src/app/program.rs`, `src/view/group.rs`, and every `Context::new` call site
(the ripple below). Do all wiring yourself.

---

## WHY this stage exists (the gating constraint — read first)

`Program::pump_once` (program.rs) **drops disabled commands at the program
boundary**: `let drop_disabled = matches!(ev, Event::Command(c) if
!command_set.has(c)); if drop_disabled { ev.clear(); }`. And `default_command_set`
seeds `cmZoom`/`cmClose`/`cmResize`/`cmNext`/`cmPrev` **disabled**. The only thing
that enables them (in C++) is `TWindow::setState(sfSelected)` →
`enableCommands(...)`. So a window must be able to enable its own commands when it
becomes selected — but a view (D3) has no handle to `Program`'s `command_set`.
**That channel is change #1.** Without it, the whole close/zoom/resize feature is
dead on arrival, so 33b can't be built until 33a lands it.

---

## Change #1 — deferred command-enable channel on `Context`

Mirror the existing `pending_captures` deferral exactly (deferred queue, applied
by `Program` after dispatch — NOT a live `&mut CommandSet` threaded into Context).

### `src/view/context.rs`
- Add a field to `Context`:
  `command_changes: &'a mut Vec<(Command, bool)>` (`true` = enable, `false` =
  disable). Document it like `pending_captures`.
- Extend `Context::new` to take it as a 5th parameter (after
  `pending_captures`). Update the doc.
- Add two methods:
  ```rust
  /// Request `cmd` be enabled in the program's command set — DEFERRED. The loop
  /// applies queued changes after the current dispatch (mirrors push_capture).
  pub fn enable_command(&mut self, cmd: Command) { self.command_changes.push((cmd, true)); }
  /// Request `cmd` be disabled — DEFERRED (see enable_command).
  pub fn disable_command(&mut self, cmd: Command) { self.command_changes.push((cmd, false)); }
  ```
- Add a unit test: a `Context` over a local `Vec`, call `enable_command`/
  `disable_command`, assert the queue contents.

### `src/app/program.rs`
- Add a `Program` field `pending_command_changes: Vec<(Command, bool)>`,
  initialized empty in `Program::new` (alongside `pending_captures`).
- In `pump_once`'s top-of-fn destructure, bind it; pass `&mut pending_command_changes`
  as the new 5th arg to **every** `Context::new` in `pump_once`.
- **Apply after dispatch**, in the same place `pending_captures` is drained (after
  the dispatch block, inside the `Some(ev)` arm). For each `(cmd, enable)` drained
  from `pending_command_changes`, call the existing `Program::enable_command(cmd)`
  / `disable_command(cmd)` logic so `command_set_changed` flips correctly. (Either
  call those `&mut self` methods — note the destructure means you have the fields,
  not `self`; so inline the same body: `if enable && !command_set.has(cmd) {
  command_set.enable_cmd(cmd); *command_set_changed = true; }` and the mirror.) Be
  careful with the borrow discipline (the brief's #1 risk — see how
  `pending_captures` is drained; do the same).
- Update the `Program::new` internal `Context::new` (the one used to
  `set_current` the desktop) and the test-only `with_ctx` helper to pass a
  `&mut Vec` for command_changes (in `new`, a throwaway local is fine — those
  startup changes can be applied immediately or ignored; match how it treats the
  startup focus broadcast).
- Add a test: arm a probe (like the existing probes) that calls
  `ctx.enable_command(Command::ZOOM)` on its event; after a pump, assert
  `program.command_enabled(Command::ZOOM)` is true and that a subsequent
  `Event::Command(Command::ZOOM)` is no longer filtered (reaches routing).

### The ripple — update ALL other `Context::new` call sites
`Context::new` now takes 5 args. Find and fix every caller so the crate compiles:
- `src/view/group.rs` — the test `with_ctx` helper (add a `pending`-style local
  `Vec<(Command,bool)>`).
- `src/frame.rs` — the test `make_ctx` helper.
- `src/desktop/*` and any other test helper that builds a `Context`.
- `src/view/context.rs` own tests.
Grep `Context::new(` across `src/` to be exhaustive. These are mechanical (thread
one more `&mut Vec`).

---

## Change #2 — Z-reorder primitives on `Group`

Port `TView::putInFrontOf` + `makeFirst` into `Group` methods (the Z-order lives
in the group, D3 — a child can't reorder itself; the group does it). Recall the
**Vec↔ring mapping** documented at the top of `group.rs`: `children[0]` == C++
`last`/**bottom** (drawn first); `children.last()` == C++ `first()`/**top**
(drawn last, frontmost). "Put in front" = move toward the **end** of the Vec.

Add:
```rust
/// `TView::putInFrontOf(target)` realized in the owner (D3). Move child `id` so
/// it sits immediately in front of `target` in Z-order (just before `target` in
/// C++ ring terms == just after it toward the top in our Vec). If `id`'s view is
/// selectable, resetCurrent afterward (faithful to putInFrontOf's tail).
///
/// NOTE: do NOT equate `target == None` with C++ `Target == 0`. C++
/// `putInFrontOf(0)` → `insertView(p, 0)` sets `last = p`, i.e. sends to the
/// BOTTOM. This Rust API deliberately repurposes `None` as a **to-top** sentinel
/// for `make_first` (its only consumer); the C++ send-to-bottom path has no
/// consumer and is intentionally unimplemented.
pub fn put_in_front_of(&mut self, id: ViewId, target: Option<ViewId>, ctx: &mut Context) { ... }

/// `TView::makeFirst` == `putInFrontOf(first())` — move child `id` to the top.
pub fn make_first(&mut self, id: ViewId, ctx: &mut Context) { ... }
```

Port `putInFrontOf` faithfully, mapping the ring ops to Vec moves:
- C++ guards: `Target != this && Target != nextView()` (no-op if already in place),
  `Target == 0 || Target->owner == owner`. In our terms: no-op if `id == target`
  or `id` is already immediately in front of `target`; ignore unknown ids.
- The C++ `sfVisible` hide/show + `drawHide`/`drawShow` dance is **dropped (D8)** —
  whole-tree redraw makes it unnecessary. Keep only the **reorder** + the
  trailing `if (options & ofSelectable) owner->resetCurrent()`.
- Determine the Vec index math by reading `putInFrontOf` carefully and matching
  our existing `firstMatch`/`find_next` index conventions in `group.rs`. Get the
  direction right: `make_first` must move the child to `children.last()` (top).

**resetCurrent caveat:** our `reset_current` picks `first_match_visible_selectable`
(documented order: `children[0]`/bottom first, then top→down). Verify against C++
`firstMatch` whether the raised window actually becomes current — see Change #3,
which is where raise-on-click is observed. If `resetCurrent` does NOT pick the
raised window, that's faithful to C++ only if C++ behaves the same; cross-check
`firstMatch`'s ring direction. The raise-on-click test (Change #3) is the ground
truth — make the behavior match what C++ produces, and document the index mapping.

---

## Change #3 — `ofTopSelect` select-path rewire (raise-on-click)

Today `Group`'s mouse-down auto-select (the carryover #1 block in
`Group::handle_event`, ~line 581) and `focus_child` route selection through
`set_current(Normal)`. C++ `TView::select()` is:
```cpp
void TView::select() {
    if( (options & ofSelectable) && owner != 0 ) {
        if( options & ofTopSelect ) makeFirst();
        else owner->setCurrent(this, normalSelect);
    }
}
```
and `focus()` validates the owner's current (the `ofValidate`→`valid(cmReleasedFocus)`
gate) and then calls `select()`. So a **selectable + ofTopSelect** view, when
selected, must be **raised to the top** (`make_first`), not just made current.

Do this faithfully:
- Make the group's selection path honor `top_select`: when a child being selected
  has `options.top_select`, route through `make_first(id, ctx)` (which raises +
  resetCurrent); otherwise `set_current(Some(id), Normal, ctx)` as today.
- Keep the existing **validate gate** where it already lives (`focus_child`
  validates the outgoing current before the switch — that is `focus()`'s gate;
  preserve it). The cleanest shape is to keep `focus_child` doing the validate,
  then branch to `make_first` vs `set_current` on `top_select`. Confirm this
  matches `focus()`→`select()` ordering.
- **Check `focus_next` / `selectNext` against the C++** (`tgroup.cpp`): determine
  whether tab/`focusNext` also goes through `focus()` (and thus may raise an
  `ofTopSelect` view) or through a non-raising `focusView`. Port whatever the C++
  does; do not guess.

### Tests (Change #2 + #3)
- **raise-on-click**: a group with two overlapping selectable+top_select children
  A (bottom) and B (top). Click A → A becomes `children.last()` (top of Z-order)
  AND `current`. Verify both the Vec order (e.g. via `index_of_pub`) and
  `group.current()`. A draw/snapshot showing A now painting over B is a strong
  extra check.
- **put_in_front_of**: direct unit test of the primitive (move a middle child to
  top; move in front of a specific target; the no-op-when-already-in-place guard).
- **non-top-select select does NOT reorder**: a selectable child WITHOUT
  `top_select`, when selected, stays at its Z index and just becomes current
  (regression guard so the rewire doesn't over-fire).

---

## What this stage does NOT do (defer cleanly — NO stubs)
- **`TWindow` itself** (struct/ctor/setState/zoom/getPalette/scrollbar/handle_event)
  → 33b.
- **close-removal channel** (a window removing itself from its owner) → decided in
  33b; do not add it here.
- **child `sfActive`-on-insert** (`tgroup.cpp` insertView's saveState restore) →
  not needed for the milestone path (a selected window activates via `TWindow`'s
  `setState(sfSelected)` override in 33b); leave `Group::insert` unchanged. (One
  sentence in a code comment is enough if you touch nearby code.)
- **shadow casting** in `Group::draw` → still deferred (the existing
  `// TODO(row 33)` stays).
Adding any of these as dead code fails `-D warnings`.

## Definition of done (run; all must pass)
- `cargo test` — all green (existing 244 + your new tests).
- `cargo clippy --all-targets -- -D warnings` — clean.
- `cargo fmt --check` — clean.

## Deviations in play
- **D3** owner-data-down: Z-reorder + command-enable are owner/loop-side; the view
  signals via the downward `Context`, never an up-pointer.
- **D4** events: command-enable is a deferred queue applied after dispatch.
- **D8** whole-tree redraw: `putInFrontOf`'s drawHide/drawShow dropped.

Report status as DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED with what you
built, any faithfulness judgment calls (especially the `firstMatch`/raise-on-click
index direction and the `focusNext` raise question), and the gate results. If the
C++ ring→Vec index mapping is genuinely ambiguous, stop and report rather than
guessing — get it right with a test.
