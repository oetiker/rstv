# Session handover — resume at Phase 2 (TDeskTop 30 → TWindow 33 → TDialog 34)

> Living handover for the **next** rstv session. Read this, then
> [CLAUDE.md](file:///home/oetiker/checkouts/rstv/CLAUDE.md) (orientation /
> Current state / Next step), then start the next row. When it lands, update or
> replace this file for the session after.

## Where things stand (git `main`)

| commit | what |
|--------|------|
| `8045847` | Phase 0 complete (primitives + INFRA substrate + snapshot format) |
| `a08412d` | **Row 23 `TView`** — `View` trait + `ViewState` + D5 flag structs + `HelpCtx` |
| `91c50a6` | **Batch A — `TBackground` (29) + `TScrollBar` (25)**; `Glyphs` became a real struct |
| `4d12a32` | **Row 26 `TGroup`** — view container + 3-phase router + focus machinery + `View::set_state` |
| `25d10b6` | **Row 24 `TFrame`** — border/title/icons + `DrawCtx::put_cstr` + frame `Glyphs` |
| `bff4885` | **Row 31 `TProgram`** — the live event loop (D9): capture stack + timer queue made live |

**Build state:** 238 unit + 3 integration tests green; `cargo clippy --all-targets
-- -D warnings` and `cargo fmt --check` clean. Working tree clean.

**Phase 1 rows done:** 23, 24, 25, 26, 29, **31**. The live loop now exists and
unblocks the pile of deferred capture/modal/drag work. **Next:** the path to "a
window you can see and drive" — **`TDeskTop` 30 → `TWindow` 33 → `TDialog` 34**.

## What row 31 (`TProgram`) delivered — context for what builds on it
New module `src/app/` (`mod.rs` + `program.rs`). The implementer brief is
[`docs/briefs/row31-tprogram.md`](file:///home/oetiker/checkouts/rstv/docs/briefs/row31-tprogram.md)
(a good template for future FOUNDATION briefs).

- **`Program` embeds a `Group`** (D2 embed-and-delegate) plus the loop machinery:
  `Renderer`, `CaptureStack`, `TimerQueue`, injected `Box<dyn Clock>`, `Theme`,
  `out_events: VecDeque<Event>`, `pending_captures`, `CommandSet`, the desktop's
  `ViewId`, `end_state: Option<Command>`, `command_set_changed`.
- **`Program::new(backend, clock, theme, create_desktop, create_status_line,
  create_menu_bar)`** — factory-injected ctor (factories are
  `impl FnOnce(Rect) -> Option<Box<dyn View>>`; the factory owns the extent
  shrink). For now desktop is a real `Group`+`Background`; status-line/menu-bar
  are `None` stubs (Phase 4). The ctor sets the group's active/selected/focused/
  modal bits directly and **makes the desktop `current`** (row-26 `insert` never
  auto-selects).
- **`pump_once`** is the D9 realization of `getEvent`→`handleEvent`→(eventError):
  drain `out_events` before polling; offer the event to the **capture stack
  first**; else `program_handle_event` (Alt-N stubbed, `group.handle_event`,
  `cmQuit → end_modal`); apply deferred `pending_captures` *after* dispatch;
  resetCursor; whole-tree redraw + diff each pass. **Borrow model:** a top-of-fn
  `let Program { .. } = self` destructure + free fns with explicit field borrows
  (preserves `Context`'s disjoint-field design — copy this pattern).
- **`run()`** ports `TGroup::execute` incl. the outer `while(!valid(end_state))`
  re-loop; **`end_modal(cmd)`** sets `end_state`. Production entry; tests drive
  `pump_once` directly with `ManualClock` + `HeadlessBackend` (never blocks, D11).
- **resetCursor:** a new defaulted **`View::cursor_request`** (base: `Some(cursor)`
  iff `focused && cursor_vis`) + a **`Group` override** that descends into
  `current` accumulating origin. `Program` turns it into the absolute hardware
  cursor (set *before* `render`, since `Renderer::render` reads `self.cursor`).
- **Command-enable:** `curCommandSet` is an explicit **allowlist** (the
  ">255 always enabled" rule **DROPPED**, D1); `cmZoom`/`cmClose`/`cmResize`/
  `cmNext`/`cmPrev` seeded **disabled** (`tview.cpp`). `enable_command`/
  `disable_command` flip `command_set_changed` (idle broadcasts
  `cmCommandSetChanged`); a disabled `Event::Command` is filtered at the program
  boundary. **Sharp edge for app authors:** a custom command not in the seed is
  silently dropped unless explicitly enabled.

### The modality seam — IMPORTANT for row 34
Row 31 shipped the modality **mechanism only**: a **`ModalFrame`** capture handler
that gates *positional* events to the modal view (mouse outside its bounds →
`Consumed`; keyboard/command `Pass` and reach it via focus; broadcasts `Pass` and
fan to all, by design). **What row 34 must add:**
- The **frame-pop is row 34's job, not the frame's.** `CaptureStack` has no
  external pop API (handlers self-pop via `CaptureFlow::ConsumedPop`), and a
  `ModalFrame` only gets `&mut Context` — it **cannot observe `end_state`**. And
  the pop must be **conditional on `valid(end_state)`** (per `execute`'s outer
  loop). So the push→run-until-valid→pop lifecycle belongs to **`exec_view`** (a
  `Program` method that owns `end_state`/`valid_end`), which is exactly the right
  place: a view never calls into the loop (D3 — it has no `Program` handle), so
  TV's "a view's `handleEvent` calls `execView`" is impossible by construction and
  the capture-frame model is the only faithful path.
- **`exec_view` / `executeDialog` / `getData`/`setData`** (the blocking modal
  wrapper + data marshalling) — design them in row 34 against the real `TDialog`.
  There is **zero test coverage of the pop path** until then; the row-31 breadcrumb
  is in `program.rs` (grep `row 34`).

## What the next session does: **TDeskTop 30 → TWindow 33 → TDialog 34**

Run these the way TProgram/TFrame/TGroup went: own the design on the main thread,
**advisor consult before writing** (proceed if it times out — the C++ is the real
spec), dispatch an Opus implementer against a self-contained brief (the row-31
brief in `docs/briefs/` is the template), then **two-stage review**
(spec-compliance → code-quality, fresh agents), fix via a fresh agent with a
precise change-list, integrate, commit at the row boundary.

1. **`TDeskTop` (row 30, FOUNDATION-ish, module `desktop`)** — small: a `TGroup`
   subclass owning a `TBackground` via a factory mixin (both ready). Adds
   tile/cascade later. Mostly it gives `Program` a *named* real desktop instead of
   the ad-hoc `Group`+`Background` the row-31 tests use. Probably the quickest win
   and a good warm-up; the `Program::new` desktop factory already accepts it.
2. **`TWindow` (row 33, FOUNDATION, module `window`)** — the D2 embed-and-delegate
   exemplar. It:
   - builds a `TFrame` (row 24) via the factory mixin and **pushes title/flags/
     number/zoomed down** (the owner-data-down seam `TFrame` is waiting for — see
     `src/frame.rs` setters `set_title`/`set_flags`/`set_number`/`set_zoomed`);
   - adds `standardScrollBar` (row 25);
   - owns zoom/move/close — the frame *posts the intents* (`cmClose`/`cmZoom`), and
     **TWindow's capture handlers run the drags** (now possible: the loop + capture
     stack are live). This lands the row-24 `TODO(row 33, D9)` deferrals
     (press-and-hold close confirm, `wfMove` frame-drag, grow drags, middle-button
     move) and the row-25 `TODO(row 31, D9)` scrollbar auto-repeat + thumb-drag —
     all as **transient within-frame capture handlers** (distinct from modal
     `exec_view`).
   - lands the **row-26/24 deferrals**: `ofTopSelect`/`makeFirst`/`putInFrontOf`
     Z-reorder, child `sfActive` activation, shadow casting.
   - **Relocate `WindowFlags`** from `src/frame.rs` to the `window` module (keep the
     crate-root re-export).
3. **`TDialog` (row 34)** — consumes the modality seam above: design `exec_view`/
   `executeDialog` + the `ModalFrame` push→pop lifecycle on `Program` (see "The
   modality seam" — the frame-pop conditional on `valid(end_state)` is the crux).

Then the widget batches fan out hard (PORT-ORDER Batches B–E) as parallel worktree
implementer+reviewer trios, committing at batch boundaries.

## Open process reminders
- Subagent-driven worked again for TProgram: main-thread design + **advisor
  consult** (it answered this time and reshaped the modal approach — capture-frame,
  not nested loop), Opus implementer against the written `docs/briefs/` brief,
  fresh-agent two-stage review (SPEC-PASS, then QUALITY-FAIL on two doc bugs →
  fixed by a fresh Sonnet agent → clean). Keep reviewers adversarial against the
  **C++**, not just the brief — the spec reviewer independently re-derived the
  `valid(end_state)` argument for the modal-pop deferral.
- **`SendMessage` IS available here** (agent ids are returned) — but spawning a
  **fresh agent with a precise change-list** for review fixes worked cleanly again.
- **Worktrees:** branch from the last **commit** — commit completed rows before
  dispatching worktree agents that build on them. `TDeskTop`/`TWindow`/`TDialog`
  are single main-thread FOUNDATION rows (no parallel fan-out), so no worktree
  needed; the later widget batches (B–E) are the parallel ones.
- **Commit policy:** committing at clean, reviewed **row/batch boundaries** (the
  project's established workflow; this session committed row 31 as `bff4885`).
- **Briefs:** `docs/briefs/` now holds the row-31 brief — a reusable template for
  the self-contained FOUNDATION implementer prompt.

## Outstanding TODOs seeded in code (grep for them)
- `row 34` in `src/app/program.rs` — the blocking `exec_view`/`executeDialog`/
  `getData`/`setData` + the `ModalFrame` pop lifecycle (conditional on
  `valid(end_state)`).
- `TODO(row 33+)` in `src/app/program.rs` — Alt-1..9 window select (needs numbered
  windows + a D4 payload story for the window number).
- `TODO(timer payload)` in `src/app/program.rs` — the timer-id payload dropped
  (D4); revisit when a widget needs to know *which* timer fired.
- `TODO(TStatusLine row)` in `src/app/program.rs` — `getEvent` status-line
  pre-handling + `statusLine->update()` (Phase 4 stubs).
- `TODO(row 33, D9)` in `src/frame.rs` — frame press-and-hold close confirm +
  `wfMove` drag + grow drags + middle-button move (now buildable: capture is live).
- `TODO(row 31, D9)` in `src/widgets/scrollbar.rs` — scrollbar press-and-hold
  auto-repeat + thumb-drag become capture handlers (now buildable; land at row 33
  with the window's scrollbars).
- `TODO/NOTE(ctrlToArrow)` in `src/widgets/scrollbar.rs` — WordStar nav helper.
- `TODO(row 33)` in `src/view/group.rs` — shadow casting in `Group::draw`.
- Sibling tee-walk + full `framelin.cpp` machinery — deferred (see `src/frame.rs`).
- Row 9 `Glyphs` continues to fill in per-widget.
- **Relocate `WindowFlags`** from `src/frame.rs` to the `window` module at row 33.

## `cargo doc` cleanup (opportunistic)
`cargo doc -D warnings` is pre-existing-broken project-wide on
`private_intra_doc_links` in several committed files. Not in the normal gate
(test/clippy/fmt). A small separate cleanup pass would make `cargo doc` clean.
