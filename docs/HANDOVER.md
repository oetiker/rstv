# Session handover — resume at the live loop / Phase 2 (TProgram 31 → TWindow 33)

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

**Build state:** 229 unit + 3 integration tests green; `cargo clippy --all-targets
-- -D warnings` and `cargo fmt --check` clean. Working tree clean.

**Phase 1 rows done:** 23, 24, 25, 26, 29. **Next:** the **live event loop**
(`TProgram` 31) — the keystone that unblocks the pile of deferred capture/modal/
drag work — then `TWindow` 33 (validates the TFrame seam) and `TDialog` 34.

## What row 24 (`TFrame`) delivered — context for what builds on it
`src/frame.rs` (`Frame` + `WindowFlags`), plus `DrawCtx::put_cstr`
(`src/view/context.rs`) and a frame glyph set in `Glyphs` (`src/theme.rs`).

- **The owner-data-down seam (D3) is now designed and waiting for `TWindow`.**
  `Frame` holds its **own** `title: Option<String>` / `flags: WindowFlags` /
  `number: Option<u8>` / `zoomed: bool`, set via public setters
  (`set_title`/`set_flags`/`set_number`/`set_zoomed`). **`TWindow` (row 33) must
  push these down** at construction and whenever its own state changes (title
  edit, zoom toggle, flag change). The frame reads its `sfActive`/`sfDragging`
  from `self.st.state` — those arrive via the `Group::set_state(Active/Dragging)`
  propagation TWindow already inherits.
- **`WindowFlags`** (`wf*` → struct-of-bools, fields `r#move`/`grow`/`close`/`zoom`)
  is defined in `src/frame.rs` **for now** — **relocate it to the `window` module
  when `TWindow` lands** (it is conceptually TWindow's). It is re-exported at the
  crate root, so moving it is a module-path change + keeping the re-export.
- **`DrawCtx::put_cstr(x, y, s, lo, hi)`** ports `moveCStr`'s `~`-toggle (starts
  in `lo`, flips lo↔hi at each `~`, `~` not drawn, returns columns advanced).
  Buttons/labels/menus (rows 37/36/51) reuse it for `~hotkey~` highlighting.
- **`handle_event` posts `cmClose`/`cmZoom`** on the resolvable row-0 clicks
  (close zone x∈2..=4; zoom zone x∈(w-5)..=(w-3) or double-click; active+y==0
  only; close checked before zoom). **Deferred to row 33 (`TODO(row 33, D9)` in
  the file):** the close icon's press-and-hold release-confirm loop (we post on
  mouse-down), the `wfMove` frame-drag, the bottom-row grow drags, the
  middle-button move — all need the live loop + capture stack.
- **The `framelin.cpp` sibling tee-walk (`├┬┤┴` joins) is deferred** — under D3 a
  child `draw(DrawCtx)` sees no siblings, so plain corners are drawn (byte-
  identical to C++ for the no-`ofFramed`-sibling-touching-the-border common case).
  When it lands it needs `Group` cooperation to pass sibling bounds down, and the
  full `FrameMask`/`frameChars[33]`/`initFrame` bitmask machinery from
  `framelin.cpp` (the tee/cross glyphs are already seeded in `Glyphs`, unused).
- **Faithfulness note for `TWindow`:** base `TWindow::getTitle(short)` **ignores
  its argument** and returns the full title (`twindow.cpp`); the frame's `-6`/`-4`
  title-budget reductions therefore never cap the drawn title (capped at
  `width-10`). A `TWindow` subclass *could* abbreviate via `getTitle` — if you
  model `getTitle`, keep the base = identity.

## What the next session does: **the live loop (`TProgram` 31), then Phase 2**

The next keystone is **`TProgram` (row 31, FOUNDATION, module `app`)** — TV's
single event loop (D9). It is the thing every deferred item in Phase 1 is waiting
on, so doing it next unblocks the most work in one move. C++ source:
`source/tvision/tprogram.cpp` (+ `tprog*.cpp`). Per PORT-ORDER row 31 it has a
**factory-mixin deferral**: it holds `TStatusLine`/`TMenuBar`/`TDeskTop` via
injected factories, but those classes are Phase 4 — so build `TProgram` against
**injected factory closures / stubs** (a bare `TDeskTop` (row 30) is buildable
now: `TGroup` + `TBackground` both exist) and fill the real status-line/menu-bar
later.

`TProgram` **owns**, and must finally make live, the things the tree has been
deferring **to it**:
- **The capture stack, live** (`src/capture.rs`, row 21) — the loop owns it;
  `Context::push_capture` queues deferred pushes, the loop applies them after each
  dispatch. The `compose_full_protocol` hand-played-loop test in `capture.rs` is
  the blueprint for the real pump.
- **The timer queue** (`src/timer.rs`, row 20) — sample the clock per pass, feed
  `now_ms` into each `Context`, dispatch `cmTimerExpired`.
- **`execView`/`execute`/the modal loop/`endModal`** (deferred from `TGroup` 26
  and `TDialog` 34) — the loop runs a modal view by pushing a capture frame.
- **`resetCursor`** (hardware cursor placement — needs absolute coords from the
  tree walk).

Consider doing **`TDeskTop` (row 30, FOUNDATION, module `desktop`)** *first or
alongside* — it is `TGroup` + an owned `TBackground` via a factory mixin (both
ready), small, and gives `TProgram` something real to loop over. Then the path to
"a window you can see and drive" is **30 → 31 → 33 (`TWindow`) → 34 (`TDialog`)**.

When **`TWindow` (33)** lands it: builds a `TFrame` (row 24) via the factory mixin
and **pushes title/flags/number/zoomed down** (the seam above); adds
`standardScrollBar` (row 25); owns zoom/move/close (the frame posts the intents,
TWindow's capture handlers run the drags); is the **D2 embed-and-delegate
exemplar**. It also lands the row-26/24 deferrals: `ofTopSelect`/`makeFirst`/
`putInFrontOf` Z-reorder, child `sfActive` activation, shadow casting.

Run FOUNDATION rows the way TFrame/TGroup went: own the design on the main
thread, **advisor consult before writing** (it has been timing out — proceed if
it does, the C++ is the real spec), dispatch an Opus implementer against a
self-contained brief, then **two-stage review** (spec-compliance → code-quality),
fix via a fresh agent with a precise change-list, integrate, commit at the row
boundary.

## Open process reminders
- Subagent-driven worked again for TFrame: main-thread design → Opus implementer
  against a written brief → fresh-agent **two-stage review** (spec then quality).
  The spec reviewer independently decoded the `initFrame`/`frameChars` tables and
  confirmed the deferral loses nothing; the implementer caught the `getTitle`
  faithfulness issue mid-implementation. Keep reviewers adversarial against the
  **C++**, not just the brief.
- **`SendMessage` is unavailable here.** To iterate on a subagent's output, spawn
  a **fresh agent with a precise change-list** (worked cleanly for the TFrame
  review fixes).
- **The advisor timed out** again this session — don't block on it.
- **Worktrees:** a worktree branches from the last **commit** — commit completed
  rows before dispatching worktree agents that build on them. `TProgram`/`TWindow`
  are single main-thread FOUNDATION rows (no parallel fan-out), so no worktree
  needed; the later widget batches (B–E) are the parallel ones.
- **Commit policy:** committing at clean, reviewed **row/batch boundaries**.

## Outstanding TODOs seeded in code (grep for them)
- `TODO(row 33, D9)` in `src/frame.rs` — the frame's press-and-hold close
  confirm + `wfMove` drag + grow drags + middle-button move (all need capture).
- `TODO(row 31, D9)` in `src/widgets/scrollbar.rs` — scrollbar press-and-hold
  auto-repeat + thumb-drag become capture handlers.
- `TODO/NOTE(ctrlToArrow)` in `src/widgets/scrollbar.rs` — WordStar Ctrl-letter
  nav; port the shared `ctrlToArrow` key-translation helper centrally when first
  needed (multiple widgets use it).
- `TODO(row 33)` in `src/view/group.rs` — shadow casting in `Group::draw`.
- Sibling tee-walk + full `framelin.cpp` `FrameMask`/`frameChars`/`initFrame`
  machinery — deferred (see `src/frame.rs` module doc); needs `Group` cooperation.
- Row 9 `Glyphs` continues to fill in per-widget (next: button shadows/markers
  row 37, static-text, menus).
- **Relocate `WindowFlags`** from `src/frame.rs` to the `window` module at row 33.

## `cargo doc` cleanup (opportunistic)
`cargo doc -D warnings` is pre-existing-broken project-wide on
`private_intra_doc_links` in several committed files (`capture.rs`, `event/mod.rs`,
`event/key.rs`, `view/mod.rs`, `view/context.rs`). Not in the normal gate
(test/clippy/fmt). A small separate cleanup pass would make `cargo doc` clean.
