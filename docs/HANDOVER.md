# Session handover — resume at TFrame (row 24)

> Living handover for the **next** rstv session. Read this, then
> [CLAUDE.md](file:///home/oetiker/checkouts/rstv/CLAUDE.md) (orientation /
> Current state / Next step), then start TFrame. When TFrame lands, update or
> replace this file for the session after.

## Where things stand (git `main`)

| commit | what |
|--------|------|
| `8045847` | Phase 0 complete (primitives + INFRA substrate + snapshot format) |
| `a08412d` | **Row 23 `TView`** — `View` trait + `ViewState` + D5 flag structs + `HelpCtx` |
| `91c50a6` | **Batch A — `TBackground` (29) + `TScrollBar` (25)**; `Glyphs` became a real struct |
| `<this>`  | **Row 26 `TGroup`** — view container + 3-phase router + focus machinery + `View::set_state` |

**Build state:** 214 unit + 3 integration tests green; `cargo clippy --all-targets
-- -D warnings` and `cargo fmt --check` clean. Working tree clean.

**Phase 1 rows done:** 23, 25, 29, 26. **Next:** 24 (`TFrame`), then Phase 2
(`TWindow` 33 / `TDialog` 34) + the live loop (`TProgram` 31).

## What row 26 (`TGroup`) delivered — context for what builds on it
`src/view/group.rs` + additive `View` growth in `src/view/view.rs`. `Group` owns a
group-local `ViewArena` + `children: Vec<Child{id,view}>` in **back-to-front paint
order** (`children[0]` = C++ `last`/bottom; `.last()` = `first()`/top;
`forEach`/`firstThat` = `iter().rev()`), `current: Option<ViewId>`. It implements
three-phase `handle_event` (D4), `draw` (drawSubViews back-to-front), focus
machinery (`insert`/`remove`/`set_current`/`reset_current`/`focus_child`/
`find_next`/`focus_next`), `change_bounds`/`valid`/`awaken`, and both row-23
carryovers via the new defaulted **`View::set_state(StateFlag, enable, ctx)`**
(base flips the bit + broadcasts `RECEIVED_FOCUS`/`RELEASED_FOCUS` on `Focused`;
`Group` overrides to propagate `Active`/`Dragging`→all, `Focused`→current).

**Things a later row must honour / pick up:**
- **Mouse position is view-local at each level** (the group subtracts child
  `origin` before delivering a positional event). TFrame/TWindow draw + hit-test
  must assume children get child-local coords.
- **`insert` does NOT activate children** (faithful: C++ `insertBefore`'s
  `sfActive`-restore is a no-op under D8). Child `sfActive` must come from the
  group's `set_state(Active)` propagation / focus logic — **row 31/33 owns this.**
- **Deferred to row 31 (`TProgram`):** `execute`/`execView`/the live blocking loop/
  modal-via-capture/`endModal`/`resetCursor` (hardware cursor). The loop owns the
  capture stack; `Context` only queues deferred pushes — a group can't run a modal
  itself. Row-26 tests drive `handle_event` directly + drain `out_events`.
- **Deferred to row 33 (`TWindow`):** `ofTopSelect`/`makeFirst`/`putInFrontOf`
  Z-reorder (so `select` currently always goes through `set_current(Normal)`);
  shadow casting in `draw` (no infra yet).
- **Deferred to D10/row 39:** `getData`/`setData`/`dataSize`.

## What the next session does: **TFrame (row 24)** — main-thread/Opus

TFrame draws the border + title bar + corner icons (close/zoom/drag) around a
framed view. It is the first row that reaches **across the sibling tree** and into
**owner-supplied data** (title/flags/number), so design those seams now that
`Group` is real.

C++ source of truth: `source/tvision/tframe.cpp`; glyph tables in
`source/tvision/tvtext1.cpp` (`initFrame[19]` + `frameChars[33]` single/double-line
sets; `closeIcon`/`zoomIcon`/`unZoomIcon`/`dragIcon`/`dragLeftIcon`); declaration in
`include/tvision/views.h`. (`<root> = /home/oetiker/scratch/tvision-spec/magiblot-tvision`.)

Run it FOUNDATION-style (the way TView/TGroup went): own the design on the main
thread, **advisor consult before writing**, dispatch an Opus implementer against a
self-contained brief, then **two-stage review** (spec-compliance → code-quality),
fix, integrate, commit at the row boundary.

### Design seams to settle before writing
1. **Owner-data-down (D3).** A `TFrame` in C++ reads its owner `TWindow`'s
   `title`/`flags`/`number`/`state(sfActive,sfDragging)` upward. Under D3 there is
   no owner pointer — so the frame must be **handed** what it needs. Decide:
   does `TFrame` hold its own `title: String` / `flags` / `number` (set by the
   `TWindow` that owns it, owner-data-down), or does the `TWindow` draw the frame
   itself? The C++ has `TFrame` as a child view of the window; the faithful port
   keeps it a child but **pushes** the title/flags down at construction / on
   change. This is the seam to design with `TWindow`'s shape (row 33) in mind — it
   may be cleaner to design TFrame's data inputs now and wire the real `TWindow`
   later.
2. **`frameLine` tee-connectors (`├┬┤┴`).** Where a nested framed subview's border
   meets the group's frame, C++ walks the sibling tree to draw tee-joins. Now that
   `Group` owns `children: Vec`, design the sibling-walk against the real Vec
   (read-only; the frame inspects sibling bounds). This may be deferrable to a
   later polish pass if it complicates the first cut — **decide and record.**
3. **Active/passive/dragging color + `f`-offset.** The C++ frame picks glyph rows
   and palette entries by an `f` offset that depends on `sfActive`/`sfDragging`.
   Port via `Theme`/`Role` (D7) — extend `Role` with the frame roles, and the
   frame glyphs extend the `Glyphs` struct (the row-9 convention TScrollBar/TGroup
   used).
4. **`dragView`** (move/resize by dragging the frame) → **D9, defer to a capture
   handler at TWindow (row 33).** TFrame row 24 draws + hit-tests the icons and
   *posts the intent* (e.g. `cmClose`/`cmZoom`/drag-start); the actual drag loop is
   a row-33 capture handler.

## Open process reminders
- Subagent-driven worked again for TGroup: main-thread design (advisor timed out
  this session — proceed if it does, the C++ is the real spec), Opus implementer
  against a written brief, fresh-agent **two-stage review** (spec then quality).
  The spec reviewer caught a real bug (an `insert`-`sfActive` faithfulness error
  introduced via the brief) — keep the reviewer adversarial against the *C++*, not
  just the brief.
- **`SendMessage` is unavailable here** (Agent Teams, gated). To iterate on a
  subagent's output, spawn a **fresh agent with a precise change-list** — works
  fine because each row's spec is self-contained.
- **Worktrees:** a worktree branches from the last **commit** — commit completed
  rows before dispatching worktree agents that build on them. TFrame is a single
  main-thread row (no parallel fan-out), so no worktree needed.
- **Commit policy:** committing at clean, reviewed **row/batch boundaries**.

## Outstanding TODOs seeded in code (grep for them)
- `TODO(row 31, D9)` in `src/widgets/scrollbar.rs` — scrollbar press-and-hold
  auto-repeat + thumb-drag become capture handlers.
- `TODO/NOTE(ctrlToArrow)` in `src/widgets/scrollbar.rs` — WordStar Ctrl-letter
  nav; port the shared `ctrlToArrow` key-translation helper centrally when first
  needed (multiple widgets use it).
- `TODO(row 33)` in `src/view/group.rs` — shadow casting in `Group::draw`.
- Row 9 `Glyphs` continues to fill in per-widget (frame set next, with TFrame).

## `cargo doc` cleanup (opportunistic)
`cargo doc -D warnings` is pre-existing-broken project-wide on
`private_intra_doc_links` in several committed files (`capture.rs`, `event/mod.rs`,
`event/key.rs`, `view/mod.rs`, `view/context.rs`). Not in the normal gate
(test/clippy/fmt). A small separate cleanup pass would make `cargo doc` clean.
