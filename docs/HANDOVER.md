# Session handover — resume at row 33d (drag + close + cmNext/cmPrev + Alt-N)

> Living handover for the **next** rstv session. Read this, then
> [CLAUDE.md](file:///home/oetiker/checkouts/rstv/CLAUDE.md) (orientation /
> Current state / Next step), then start. When 33d lands, update or replace this
> file for the session after.

## Where things stand (git `main`)

| commit | what |
|--------|------|
| `bff4885` | Row 31 `TProgram` — the live event loop (D9) |
| `c80a20d` | Row 30 `TDeskTop` |
| `432c01a` | Row 33c — TWindow zoom |
| `7b15782` | Substrate realignment — global `ViewId` + self-id + `find_mut`/`remove_descendant` |
| **`7efecb3`** | **Phase A — `Event::Broadcast { command, source: Option<ViewId> }` (D4 amendment, buildable slice)** |

**Build state:** 268 unit + 3 integration + 1 doctest green; `cargo clippy
--all-targets -- -D warnings` and `cargo fmt --check` clean. Working tree clean.

## What the last session did (Phase A — read this; it shaped 33d's scope)

Phase A's job per the prior handover was "build `message`/`query` +
`Broadcast{source}`, then remove the kludges that existed because it was missing."
**Investigation collapsed most of it to documentation** — the C++ `infoPtr` is
*polymorphic* (subject view / command target / integer arg), and only the
broadcast-subject case ports to a `ViewId`. What actually shipped:

- **Built:** `Event::Broadcast(Command)` → `Event::Broadcast { command,
  source: Option<ViewId> }`. `source` = the emitting view (C++ `this`), threaded
  from focus broadcasts (`view.rs`/`group.rs` `set_state`) and the scrollbar's 4
  changed/clicked sends; `None` for pump-internal + capture broadcasts.
  **Data-only — no receiver reads `source` yet** (first consumer is a two-scrollbar
  scroller in Batch B). Routing unchanged. `ctx.broadcast(command, source)` is the
  signature now. One new test asserts `source == Some(<inserted scrollbar id>)`.
- **NOT rebuilt — `cmZoom`/`cmClose` self-target guard is provably vacuous.** The
  frame posts these with `infoPtr=owner` **only while `sfActive`** (so its owner is
  always the active window), and `Event::Command` is focused-routed to the
  `current` child only → the command always reaches exactly its target. There is
  nothing for an `infoPtr==this` guard to reject. `window.rs` states this invariant
  + a trip-wire (revisit only if some future emitter targets a *non-active* window).
  **33d's `cmClose` inherits this — do NOT add a target to `Event::Command`.**
- **Deferred to 33d — Alt-N (`cmSelectWindowNum`).** Its payload is an *integer*
  (the window number), not a `ViewId`, so `source` does not serve it; and its
  realization needs `select`/`canMoveFocus`, which land with 33d. Realize it as a
  **direct number-walk** (below), not a payload-carrying broadcast.
- **Deferred to row 34 — the return-consuming `message()`/`query` primitive.** Its
  first real consumer is a dialog `cmCanCloseForm` veto. Design of record is in the
  guide (`D4 "message() — corrected"`, now rewritten for the polymorphism).

Brief: [`docs/briefs/row33-phaseA-broadcast-source.md`](briefs/row33-phaseA-broadcast-source.md).
Two-stage reviewed (SPEC-PASS + QUALITY-PASS).

## NEXT — row 33d (drag + close + cmNext/cmPrev + Alt-N), simplified by the substrate

Design on the main thread; **advisor consult before writing**; Opus implementer
against a written `docs/briefs/row33d-*.md`; fresh-agent two-stage review (spec →
quality); integrate; commit at the boundary. Single main-thread FOUNDATION stage →
**no worktree**.

The substrate dissolves the hard parts the old handover agonized over (the
close-removal channel + drag path-building are **gone** — the window names itself
via `self.state().id()` and the loop applies via `find_mut(id)` /
`remove_descendant(id)`).

1. **Drag = a capture handler** (the D9 replacement for `dragView`'s nested
   `mouseEvent` loop — the capture stack is the centerpiece, do not route around
   it). Flow:
   - The **frame** leaves a row-0 / bottom-corner mouse-down unconsumed (it already
     consumes close/zoom; see `frame.rs` `TODO(row 33, D9)`). The **window**, after
     delegating to its group, detects the unconsumed `MouseDown` and starts the
     drag — it knows its own id via **`self.state().id()`** and its limits via
     **`ctx.owner_size()`** (valid at that point; capture the limits at push time).
   - Push a transient `DragCapture { window_id, kind, anchor, min, max, limits }`.
     Each `MouseMove`: compute new bounds via the faithful **`moveGrow`/`locate`**
     (already ported in `window.rs` for zoom) and request the apply via a small
     deferred channel on `Context` (`ctx.request_bounds(id, rect)` — mirror the
     existing `command_changes`/`pending_captures` deferred-channel pattern). The
     **loop** applies it after dispatch via `root.find_mut(id).change_bounds(rect)`.
     `MouseUp` → `ConsumedPop`; the loop flips `sfDragging` off via
     `find_mut(id).set_state(Dragging, false, ctx)` (the capture can't call
     `set_state`; `find_mut` is the uniform apply primitive).
   - `cmResize` keyboard sub-mode (arrows-until-Enter/Esc) — defer unless cheap.
2. **Close** = `cmClose` → `if valid(cmClose)`: if `sfModal` post `cmCancel` (row 34
   owns modal teardown), else `ctx.request_close(self.state().id())`; the loop
   drains it after dispatch via `root.remove_descendant(id, ctx)`. **No target
   guard** (Phase A proved it vacuous) — a stray `cmClose` can only reach the active
   window, which is correct.
3. **`setState`** — extend the enable set to the full C++ `{cmNext, cmPrev,
   cmResize if (grow|move), cmClose if close, cmZoom if zoom}` (33c shipped cmZoom
   only). Land **TDeskTop cmNext/cmPrev** (`src/desktop/desktop.rs` `TODO(row 33,
   D9)` — now buildable: `focus_next` + `put_in_front_of` exist).
4. **Alt-N (`cmSelectWindowNum`)** — now unblocked, ride here with the selection
   machinery. `TProgram::handleEvent`: an Alt+digit keydown (`Key::Char('1'..='9')`
   + `alt` modifier; the `getAltChar` equivalent) → `if canMoveFocus()`: a **direct
   walk** — the program (a tree owner) asks the desktop to select the child window
   whose `number` matches. Needs `View::number() -> Option<u16>` (default `None`,
   `Window` overrides) + `select()`/`canMoveFocus` (build them with cmNext/cmPrev).
   Returns whether one matched → `clearEvent`. **NOT** a number-carrying broadcast.
   Breadcrumbs corrected in `program.rs`/`window.rs`/`window/mod.rs`.
5. **Scrollbar auto-repeat + thumb-drag** (`src/widgets/scrollbar.rs` `TODO(row 31,
   D9)`) — capture handlers; independent of the window. Land here or split into a
   Batch-B widget pass. (The `source` field shipped in Phase A is *not* needed for
   this — it is for a two-bar owner disambiguating which bar fired.)

## Row 34 — `TDialog` (the modality payoff) — unchanged plan, + builds `message()`

Consumes the row-31 `ModalFrame` seam. Design `exec_view`/`executeDialog` + the
push→run-until-`valid(end_state)`→**pop (conditional on `valid(end_state)`)**
lifecycle on `Program` (the crux: a view can't reach the loop, so `exec_view` owns
it). `cmOK`/`cmCancel`; gather/scatter typed values (D10). Gray window scheme
(`WindowPalette::Gray`) drives the deferred multi-scheme theming. **Also build the
return-consuming `message()`/`query` tree-owner primitive here** — its first real
consumer is the dialog `cmCanCloseForm` veto (design of record: guide D4
"message() — corrected"). Breadcrumbs: `row 34` in `src/app/program.rs`.

## Process reminders
- Subagent-driven worked well (main-thread design + **advisor consult before
  writing** + Opus implementer against a written `docs/briefs/` brief + fresh-agent
  two-stage review — keep reviewers adversarial against the **C++ + corrected
  guide**, not just the brief).
- Single main-thread FOUNDATION stages → **no worktree**. Commit at clean reviewed
  stage boundaries.
- **When a design forces non-obvious machinery, investigate WHY before bandaiding**
  (memory `fix-foundations-not-bandaids`). Phase A is the latest case: the prior
  handover assumed `infoPtr` "ports directly" to a `ViewId`; tracing the 3
  return-consuming sites + the frame/routing invariant showed it is polymorphic and
  most of the planned work was unnecessary. The advisor consult caught this before
  any code was written.

## Outstanding TODOs seeded in code (grep)
- `TODO(33d)` in `src/window/window.rs` — cmResize (drag), cmClose, cmNext/cmPrev
  in the setState set, re-pushing `set_zoomed` on owner resize.
- `cmClose` note in `src/window/window.rs:~361` still says "needs a close-removal
  channel" — stale (the channel is `remove_descendant` now); fix when you do 33d.
- `row 34` in `src/app/program.rs` — `exec_view`/`executeDialog` + the `ModalFrame`
  pop lifecycle (conditional on `valid(end_state)`) + the `message()` primitive.
- `TODO(row 33, D9)` in `src/frame.rs` — close press-and-hold confirm, `wfMove`
  drag, grow drags, middle-button move (now buildable).
- `TODO(row 31, D9)` in `src/widgets/scrollbar.rs` — auto-repeat + thumb-drag.
- `TODO(row 33)` in `src/view/group.rs` — shadow casting in `Group::draw`.
- Sibling tee-walk + full `framelin.cpp` machinery — deferred (`src/frame.rs`).
- Row 9 `Glyphs` continues to fill in per-widget.
- `cargo doc -D warnings` pre-existing-broken on `private_intra_doc_links`
  (not in the gate; opportunistic cleanup).
