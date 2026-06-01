# Session handover — resume at TGroup (row 26)

> Living handover for the **next** rstv session. Read this, then
> [CLAUDE.md](file:///home/oetiker/checkouts/rstv/CLAUDE.md) (project orientation /
> Current state / Next step), then start TGroup. When TGroup lands, update or
> replace this file for the session after.

## Where things stand (git `main`)

| commit | what |
|--------|------|
| `8045847` | Phase 0 complete (primitives + INFRA substrate + snapshot format) |
| `a08412d` | **Row 23 `TView`** — `View` trait + `ViewState` + D5 flag structs + `HelpCtx` |
| `91c50a6` | **Batch A — `TBackground` (29) + `TScrollBar` (25)**; `Glyphs` became a real per-widget struct |

**Build state:** 197 unit + 3 integration tests green; `cargo clippy --all-targets
-- -D warnings` and `cargo fmt --check` clean. Working tree clean.

**Phase 1 rows done:** 23, 25, 29. **Deferred:** 24 (`TFrame`, see below).
**Next:** 26 (`TGroup`), then back to 24, then Phase 2 (TWindow/TDialog) and the
big MECHANICAL fan-out (Batches B–E).

## What the next session does: **TGroup (row 26)** — FOUNDATION, main-thread/Opus

TGroup is the gate for everything after it (windows, dialogs, menus, the desktop,
the program shell). It is **design-heavy** — treat it like TView was: own the
design on the main thread, likely an **advisor consult** before writing, then a
two-stage review. It is **not** a parallel-fan-out row.

C++ source of truth: `source/tvision/tgroup.cpp`, `grp.cpp`, `tgrmv.cpp`;
declaration in `include/tvision/tvision/views.h`. A full structural map was
captured this session — the digest + the Rust translation is below.

### What TGroup brings (the deferred-until-now pieces converge here)
- The **child tree** (D3): `Vec<Box<dyn View>>`, replacing the circular
  `last`/`next` ring.
- **Three-phase event routing** (D4).
- **Focus machinery** — `current`, `setCurrent`, `focusNext`/`findNext` (tab
  order), `setState` propagation. This is where the **row-23 carryover** lands
  (see below).
- The **`query(ViewId,…)` and focus methods** that row 22 deferred from `Context`.
- The **live event loop** (at least a minimal synchronous `pump_until_idle`) so
  group routing + modal-via-capture are testable — **decision needed** on the
  26-vs-31 split (see "Open decisions").
- It makes the **capture stack (row 21)** + **Context deferred-push (row 22)**
  finally load-bearing: `execView`/modal becomes a capture handler, not a nested
  loop (D9).

### Row-23 carryover that MUST be implemented in TGroup
From the `src/view/view.rs` module doc (verbatim breadcrumb):
1. **Mouse-down → select.** On a mouse-down delivered to the top-most
   `ofSelectable` child **that is not already `sfSelected` and not `sfDisabled`**,
   select that child and pass the event through **iff** (`options.first_click` AND
   focus succeeded), else consume it (`ev.clear()`). This is the relocated body of
   `TView::handleEvent`.
2. **Focus broadcast.** The `sfFocused` transition broadcasts
   `Command::RECEIVED_FOCUS` / `Command::RELEASED_FOCUS` via `ctx.broadcast` — fired
   by TGroup's focus logic, not by any base `View` method.

## C++ → Rust translation map for TGroup

### Data members → Rust
| C++ | Rust (proposed) | notes |
|-----|-----------------|-------|
| `last` + circular `next` ring | `children: Vec<Box<dyn View>>` (Z-order: `[0]`=bottom, last=top) | `first()`→`children[0]`; draw back-to-front = `iter()` |
| `current: TView*` | `current: Option<ViewId>` | identity, not pointer (D3) |
| `buffer`, `ofBuffered`, `lock`/`unlock`/`lockFlag` | **DROPPED** (D8) | whole-tree redraw + diff; no per-group buffer/lock |
| `clip: TRect` | computed via `DrawCtx::sub(child.bounds)` | clip is a draw-time concern (D8) |
| `phase: phaseType` | local `enum Phase { PreProcess, Focused, PostProcess }` during dispatch | not stored long-term |
| `endState: ushort` | modal result via posted `Command` (D9) | see modal section |

You'll need a **ViewId ↔ child** mapping: the group assigns a `ViewId` (via the
row-17 `ViewArena`) to each child on insert, and resolves `ViewId → &mut dyn View`
by walking `children`. Decide where the `ViewArena` lives (group-local vs shared
via `Context`); `Context::query(ViewId)` (deferred from row 22) is implemented by
this walk.

### Three-phase `handleEvent` (D4) — port faithfully
C++ core:
```cpp
if( event.what & focusedEvents ) {          // keyboard | command
    phase = phPreProcess;  forEach(doHandleEvent);   // children with ofPreProcess
    phase = phFocused;     doHandleEvent(current);    // the current child only
    phase = phPostProcess; forEach(doHandleEvent);   // children with ofPostProcess
} else if( event.what ) {
    phase = phFocused;
    if( event.what & positionalEvents )      // mouse (minus wheel)
        doHandleEvent( firstThat(hasMouse) ); // TOPMOST child under cursor only
    else
        forEach(doHandleEvent);              // else broadcast to all
}
```
`doHandleEvent` skips: null child; `sfDisabled` children for positional/focused
events; `ofPreProcess`/`ofPostProcess` gating per phase; and respects the child's
`eventMask`. Rust: `match ev { KeyDown|Command => three-phase, Mouse* =>
topmost-under-cursor, Broadcast => all }`. `positionalEvents = evMouse &
~evMouseWheel`; `focusedEvents = evKeyboard | evCommand`.

**`eventError` (bubble unhandled up to owner):** in our downward model there is no
owner pointer — an unhandled event is simply left **not cleared**, and as the
recursive `handle_event` call stack unwinds, the parent/loop sees it. No explicit
bubble call needed.

### Focus — `setCurrent` / `findNext` / `focusNext`
```cpp
void setCurrent(p, mode) {                  // mode: normal | enterSelect | leaveSelect
  if (current != p) {
    lock();                                  // (drop lock; redraw is automatic, D8)
    focusView(current, False);
    if (mode != enterSelect && current) current->setState(sfSelected, False);
    if (mode != leaveSelect && p)       p->setState(sfSelected, True);
    if ((state & sfFocused) && p)       p->setState(sfFocused, True);
    current = p; unlock();
  }
}
findNext(forwards): step current->next/prev until a child that is
    (sfVisible set, sfDisabled clear, ofSelectable set) OR back to current.
```
Port the tab-order + `selectMode` semantics faithfully; `current` is a `ViewId`;
`setState(sfFocused)` propagation fires the focus broadcast (carryover #2).

### Drawing (D8) — much simpler than C++
Drop `getBuffer`/`lock`/`unlock`/`freeBuffer`/`ofBuffered` entirely. `draw` =
`drawSubViews` back-to-front: for each child in Z-order, `child.draw(&mut
ctx.sub(child.bounds))` (the `DrawCtx::sub` from row 22 already clips +
re-origins). Cast shadows per D8 during the back-to-front pass for children with
`state.shadow`.

### Modal execution (D9) — the heart of the inversion
C++ uses **nested blocking loops**:
```cpp
ushort execute() {                  // TGroup's own event loop
  do { endState = 0;
       do { getEvent(e); handleEvent(e);
            if (e.what != evNothing) eventError(e);
       } while (endState == 0);
  } while (!valid(endState));        // validation-retry
  return endState;
}
ushort execView(p) {                // run a modal sub-view
  ... setCurrent(p, enterSelect); insert(p);
  retval = p->execute();            // NESTED loop
  remove(p); setCurrent(saveCurrent, leaveSelect); return retval;
}
void endModal(cmd){ if (sfModal) endState = cmd; else TView::endModal(cmd); }
```
**Rust replacement (D9, using rows 21/22):** there is **one** non-recursive loop.
`exec_view(dialog)` pushes a **modal `CaptureHandler`** (rows 21) that consumes
every otherwise-unhandled event (that *is* the modal loop) and holds the dialog's
`ViewId`; `end_modal(cmd)` makes the handler return `CaptureFlow::ConsumedPop` and
delivers the result by **posting a completion `Command`** (or callback) to the
owner — `exec_view` cannot block-and-return. The `valid(endState)` retry +
`cmReleasedFocus` validation port faithfully. The `compose_full_protocol` test in
`src/capture.rs` already hand-plays this protocol — model the real loop on it.

### `valid` / `getData` / `setData`
- `valid(cmReleasedFocus)` → if `current` has `options.validate`, return
  `current.valid(cmd)`, else true; otherwise `firstThat(isInvalid)`. Port.
- `getData`/`setData` walk children summing `dataSize` — **defer to D10/row 39**
  (no `FieldValue` / data-bearing widgets exist yet). Note it; don't invent it.

### insert / remove
`Vec` push/insert/remove. Apply `ofCenterX`/`ofCenterY` centering on insert
(`origin = (size - child.size)/2`). The C++ hide/show dance around linking is
mostly moot under D8 (the next redraw repaints) — keep only the Z-order + focus
side effects (`resetCurrent` when the removed/inserted child was selectable).

## Open decisions for the next session (resolve before/with the advisor)
1. **26-vs-31 event-loop split.** Rows 21/22 notes say "live loop deferred to row
   31 (TProgram)." CLAUDE.md says TGroup "brings the live event loop." Likely
   resolution: a **minimal synchronous `pump_until_idle`** lands at 26 (enough to
   test group routing + modal-via-capture deterministically with the
   `HeadlessBackend` + `ManualClock`), and TProgram (31) wraps it with the real
   blocking poll + timer integration. Decide and record.
2. **Where the `ViewArena` lives** and how `ViewId → child` resolution works
   (group-local map vs shared arena via `Context`). This unblocks
   `Context::query` (deferred from row 22).
3. **`Context` growth.** Row 22 deferred `query(ViewId)` + focus ops to here. Add
   them to `src/view/context.rs` (orchestrator owns that shared file).
4. **Root vs nested groups.** TDeskTop/TProgram are groups too; make sure the
   `View`-trait shape for a group composes (a group is a `View` that owns
   `Vec<Box<dyn View>>`). The whole tree is already `Box<dyn View>`, so a group is
   just a `View` whose `draw`/`handle_event` iterate children.

## After TGroup: TFrame (24), then Phase 2
**TFrame (24) was deliberately deferred to after TGroup** — its C++ reaches into
`TWindow` (`flags`/`getTitle`/`number`, row 33), the **Group sibling tree**
(`frameLine` tee-connectors `├┬┤┴` where nested framed views meet), and
`dragView` (D9). A full C++ map of TFrame was captured this session (frame glyph
tables in `tvtext1.cpp`: `initFrame[19]` + `frameChars[33]` single/double-line
sets; `closeIcon`/`zoomIcon`/`unZoomIcon`/`dragIcon`/`dragLeftIcon`; the
active/passive/dragging color+`f`-offset logic). The frame glyphs extend the
`Glyphs` struct (same row-9 convention TScrollBar used). Design the
owner-data-down seam (TFrame holds title/flags/number, set by its TWindow) when
TWindow's shape is being designed.

## Outstanding TODOs seeded in code (grep for them)
- `TODO(row 31, D9)` in `src/widgets/scrollbar.rs` — scrollbar press-and-hold
  auto-repeat + thumb-drag (the C++ `mouseEvent` loops) become capture handlers.
- `TODO/NOTE(ctrlToArrow)` in `src/widgets/scrollbar.rs` — WordStar Ctrl-letter
  navigation; port the shared `ctrlToArrow` key-translation helper centrally when
  first needed (multiple widgets use it).
- Row 9 `Glyphs` continues to fill in per-widget (frame set next, with TFrame).

## Process notes (what worked / gotchas this session)
- **Subagent-driven worked well.** FOUNDATION rows (TView) designed on the main
  thread with an advisor consult; MECHANICAL leaves (Background/ScrollBar) as
  parallel **worktree** implementers (Sonnet), orchestrator integrating the shared
  `lib.rs`/`theme.rs` edits centrally. Two-stage review caught real bugs
  (ScrollBar CP437 shade off-by-one; mouse page-click should thumb-jump).
- **Review weighting:** a trivial verified leaf (TBackground) got a direct
  orchestrator review; the more behavioural ScrollBar got a fresh-agent
  spec+quality review. Scale review to risk.
- **`SendMessage` is unavailable here** (it belongs to the experimental "Agent
  Teams" feature, gated behind `CLAUDE_CODE_EXPERIMENTAL_AGENT_TEAMS=1` and, per
  open GitHub issues, often not injected even then). To iterate on a subagent's
  output, **spawn a fresh agent with a precise change-list** — works fine because
  each row's spec is self-contained.
- **Worktree gotchas:** (1) agent isolation worktrees land under
  `.claude/worktrees/` — now gitignored; remember to `git rm --cached` if they get
  staged as embedded repos, and `git worktree remove --force` + delete the branch
  when done. (2) A worktree branches from the last **commit** — commit completed
  rows before dispatching worktree agents that build on them (we committed row 23
  before Batch A for exactly this).
- **Commit policy:** committing at clean, reviewed **batch boundaries** (matching
  the repo's existing sub-phase commit granularity). Confirm with the user if they
  prefer fewer/larger commits.
- **`cargo doc -D warnings` is pre-existing-broken** project-wide on
  `private_intra_doc_links` in several committed files (`capture.rs`,
  `event/mod.rs`, `event/key.rs`, `view/mod.rs`, `view/context.rs`). Not in the
  normal gate (test/clippy/fmt). A small separate cleanup pass would make
  `cargo doc` clean — do it opportunistically.
