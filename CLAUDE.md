# rstv — idiomatic Rust port of Turbo Vision

**What this is:** a faithful Rust port of **magiblot/tvision** (modern C++ Turbo
Vision). The goal is a framework a C++ tvision veteran recognizes on sight, but
that is native Rust.

## Read these first
- **`docs/PORTING-GUIDE.md`** — the deviation reference. We port *faithfully*
  from the C++; this guide documents **only the places we deviate** (D1–D13),
  each as *Baseline → Deviation → Integration*. Appendix A = C++→Rust symbol
  lookup. Appendix B = the **mechanical per-class porting recipe**.
- **`docs/PORT-ORDER.md`** — dependency-ordered checklist of 92 classes in 6
  phases, with verified C++ file mappings, target Rust modules, and
  `FOUNDATION`/`MECHANICAL`/`INFRA` tags. Port in this order.

## Source trees (not in this repo)
- **Port FROM:** `/home/oetiker/scratch/tvision-spec/magiblot-tvision/`
  (headers `include/tvision/`, impl `source/tvision/`, platform
  `source/platform/`). This is the source of truth — port its behavior verbatim.
- **Lessons reference only:** `/home/oetiker/scratch/tvision-spec/tvision/` is a
  working **Go** port. It was already mined for lessons. **Never reference the Go
  port in the guide or commits** — the guide is purely C++→Rust.

## Methodology (lean by design)
1. **Faithful by default.** If a class/behavior isn't called out as a deviation,
   translate it straight from the C++. No per-file design.
2. **Deviations are pre-decided** in the guide. Apply the relevant D-rules
   mechanically (Appendix B has the line-level substitution table).
3. **Division of labor:** `INFRA` (net-new substrate) and `FOUNDATION`
   (pattern-setting classes) need careful Opus/human work. `MECHANICAL` leaves
   are handed to **Sonnet** via Appendix B + the PORT-ORDER row — they need
   near-zero judgment. Parallelizable batches are listed in PORT-ORDER.md.
4. **Snapshot tests are the verification** (D11): port a piece, run it on the
   `HeadlessBackend`, snapshot, compare to C++ behavior. No heavy upfront plans.

## How to run the port (subagent-driven, the default from Phase 1 on)

Phase 0 was FOUNDATION/INFRA — interlocking, design-heavy, mostly serial. Phase 1+
is **mostly `MECHANICAL` leaf widgets in parallel batches** (PORT-ORDER Batches
A–E), so the orchestrator runs it as **subagent-driven development**
(`superpowers:subagent-driven-development`). The main thread does **only**
coordination: design FOUNDATION seams, write precise prompts, integrate, decide.
Per row:

1. **Implementer subagent (fresh, isolated context).** Give it a *self-contained*
   prompt — never "go read the plan." Inline: the PORT-ORDER row, the relevant
   C++ source (from `magiblot-tvision/`), the D-rules that apply (Appendix B
   table), the existing types it builds on, and "run `cargo test`/`clippy
   --all-targets`/`fmt --check` + add a snapshot test (Appendix B step 4)."
   **Model by tag:** `MECHANICAL` → Sonnet; `FOUNDATION`/`INFRA` → Opus (or the
   main thread).
2. **Two-stage review (fresh subagents — do NOT just self-review in the main
   thread).** First a **spec-compliance** reviewer (does it match the C++
   behavior + the row's D-rules, nothing extra/missing?), then, once that's ✅, a
   **code-quality** reviewer. Implementer fixes; re-review until clean.
   (`feature-dev:code-reviewer` / `gsd-code-reviewer` agent types fit, or a
   plain agent with the row's spec.)
3. **Integrate + verify in the shared tree**, then mark the row done.

**Parallelism (the reconciliation):** the skill says "never dispatch parallel
implementers" because of shared-tree conflicts — but PORT-ORDER's batches are
*build-disjoint*, so dispatch them **concurrently using `isolation: "worktree"`**
(each agent self-verifies in its own checkout; the orchestrator integrates). Run
serially only for shared files (`lib.rs`, a shared `mod.rs`) and FOUNDATION rows
that gate others. The orchestrator owns the few shared-file edits (module wiring,
re-exports) to avoid races.

**Worktree gotcha (learned this milestone):** an agent worktree is branched from
the last **commit**, so uncommitted work is absent from it. **Commit completed
rows before dispatching worktree subagents that build on them** (or the agent
wastes effort re-deriving prereqs). Commit at batch boundaries.

## Locked decisions (details in the guide)
Crate `tvision`, house style `tv::`; drop `T` prefix; `snake_case` methods;
constant families → open newtypes with SCREAMING_SNAKE assoc consts
(`tv::Command::OK`); inheritance → `View` trait + `ViewState` composition;
pointers → `ViewId` handles + downward `Context`; events → `enum Event` + match;
flag words → struct-of-bools; palette+glyphs → `Theme`; whole-tree redraw + diff
(no damage tracking); modal loops → single loop + capture stack; `TStreamable`
dropped (serde if revived). Stack: crossterm (behind a `Backend` trait) →
vendored ratatui cell-buffer+diff (MIT) → retained view tree + event loop.

## Current state
- Planning docs written; methodology established.
- **Crate scaffolded** (`Cargo.toml` pkg `tvision`, edition 2024; `src/lib.rs`
  with the house-style root re-exports + a Phase 0 module map; `NOTICE`).
- **Phase 0 started** (per `docs/PORT-ORDER.md`):
  - rows 1–2 `Point`/`Rect` → `src/view/geometry.rs` (`r#move`/`r#union` keep the
    keyword-colliding names);
  - rows 3–4 `Color`/`Style`/`Modifiers` → `src/color.rs` (D6; all 7 `sl*` flags;
    `Style::reversed()` ports `reverseAttribute`);
  - row 6 `Cell` → `src/screen/cell.rs` (D6+D13; grapheme symbol + `wide`/`trail`,
    derives `PartialEq` for the row-18 diff);
  - row 8 `Text` → `src/text.rs` (D13; `width`/`measure`/`scroll`/`next`/`draw_one`/
    `draw_str[_ex]`. Built on `unicode-segmentation` graphemes + `unicode-width`;
    drops magiblot's UTF-8 DFA — one grapheme cluster = one cell, width from the
    base char, ZWJ sequences clustered);
  - row 7 `DrawBuffer` → `src/screen/draw_buffer.rs` (`move_char`/`move_str[_part]`/
    `move_cstr[_part]`/`move_buf`/`put_char`/`put_attribute`; delegates text to row
    8; dropped the `0 = retain` sentinel — `move_char` always writes both);
  - row 10 `Key` → `src/event/key.rs` (D4/D5; **`enum Key`** + `KeyModifiers` +
    `KeyEvent`, decomposed — no modifier-combined variants, `Ctrl+C`=`Char('c')`+ctrl);
  - row 11 `Event` → `src/event/mod.rs` (D4 `enum Event` + `MouseEvent` (`position`,
    not `where`) + `EventMask`; `infoPtr`/`MessageEvent` dropped — query/broadcast
    split);
  - row 12 `Command` → `src/command.rs` (D1; **`Command(&'static str)`** namespaced
    open newtype + `Command::custom`; `CommandSet` over `HashSet`, no range guard;
    only shared-vocabulary consts here, view-specific ones live with their view).
  - **INFRA substrate (rows 5, 17, 18, 19, 20):**
    - row 5 quantization ladder → `src/backend/quantize.rs` (D6; faithful port of
      `platform/colors.cpp` + `colors.h` conversions — NOT `mapcolor.cpp`/
      `palette.cpp`, which are the D7 palette-chain walk; PORT-ORDER row 5's file
      cite was corrected). `rgb_to_xterm16/256`, `rgb_to_bios`, `bios_to_xterm16`,
      `xterm256_to_xterm16/rgb`; compile-time LUTs.
    - row 17 `ViewId` arena → `src/view/id.rs` (D3; generational *identity*
      allocator only — NOT a view store; `Option<ViewId>` niche via `NonZeroU32`;
      resolution to `&dyn View` is a later tree-walk via `Context`).
    - row 18 `Buffer` + diff → `src/screen/buffer.rs` (D8; ratatui-adapted diff,
      MIT attribution in-file; wide-char skip driven off our `wide`/`trail` flags;
      no `skip` field).
    - row 19 `Backend` + `Renderer` + `HeadlessBackend`/`HeadlessHandle` +
      `CrosstermBackend` → `src/backend/{traits,renderer,headless,crossterm_backend}.rs`
      (D11). **Object-safe `Box<dyn Backend>`** (slice-based `draw`, `poll_event`
      collapses the associated `EventSource`); `Renderer` owns back/front buffer
      pair + draw cycle; `HeadlessBackend` shares state via `HeadlessHandle` for
      test inspection/injection (no downcast); Crossterm does key/mouse/color
      translation. Deps added: `crossterm`, dev-dep `insta`.
    - row 20 `Clock` + `TimerQueue` → `src/timer.rs` (D9/D11; injected
      `Clock`/`SystemClock`/`ManualClock`; `calc_next_expires_at` verbatim; dropped
      the `collectId` dance — collect-then-dispatch; clock passed in, not stored).
    - row 16 (minimal) `Theme` → `src/theme.rs` (D7; `Role` first-party closed
      enum seeded with D7's enumerated needs, grows per-widget; `Glyphs` empty stub
      [row 9 deferred]; `classic_blue` default with *provisional* BIOS colors).
    - row 21 capture stack → `src/capture.rs` (D9; `CaptureStack` of
      `Box<dyn CaptureHandler>`; `CaptureFlow::{Pass,Consumed,ConsumedPop}` — pop
      is a return value [unambiguous: the handler that just ran], push is deferred
      via `Context`; live loop deferred to row 31).
    - row 22 `Context`/`DrawCtx` → `src/view/context.rs` (D3/D4). `DrawCtx` =
      clipped+themed writer (`put_char`/`put_str`/`fill`/`sub`; clip intersected
      with buffer bounds at construction → panic-safe; reuses row-7/8 text
      primitives). `Context` = event-side ctx (`post`/`broadcast`/`set_timer`/
      `kill_timer`/`push_capture`; distinct `&mut` fields for disjoint borrows);
      `query`/focus deferred to `TGroup` (row 26). Borrow model: loop owns the
      capture stack, `Context` only queues deferred pushes — proven by the
      `compose_full_protocol` hand-played-loop test in `src/capture.rs`.
  - **Snapshot format (FOUNDATION) frozen** in `src/screen/snapshot.rs`
    (`snapshot(&Buffer, cursor) -> String`): `size`/`cursor`/`text`/`attr`/`legend`,
    `|`-framed rows, `.`=default style, per-display-column attr keys, wide glyph
    keyed twice + trail absorbed. First end-to-end snapshot test in
    `tests/render_pipeline.rs`. Every future widget test diffs against this.
  - **Phase 0 is COMPLETE** (all primitives + INFRA substrate). The only
    deferred Phase-0 item is row 9 (glyph tables → `Glyphs`), a stub that fills in
    per-widget. 155 unit/integration tests green; `cargo clippy --all-targets` and
    `cargo fmt --check` clean.
  - **Phase 1 started — row 23 `TView` DONE** (`src/view/view.rs` + `src/help.rs`):
    the `View` trait (`state`/`state_mut`/`draw` required; `handle_event`[no-op
    base]/`valid`/`awaken`/`size_limits`/`calc_bounds`/`change_bounds` defaulted) +
    `ViewState` composition + the four D5 struct-of-bools (`State`/`Options`/
    `GrowMode`/`DragMode`) + `HelpCtx` open newtype (D1). Key decisions: base
    `handle_event` is a no-op — TView's only body (mouse-down→`focus()`) **relocates
    to TGroup row 26** (D3; breadcrumb incl. the `sfSelected`/`sfDisabled` guard is
    in the `view.rs` module doc); no `getColor`/`getPalette`/`mapColor` on the trait
    (views call `ctx.style(Role)`, D7); occlusion/buffered/`sfExposed` dropped (D8);
    `dragView`→row 33, data/`value`→D10/row 39, enabled-command set→TProgram row 31
    (the `>255`-always-enabled rule **dropped**, D1). `calcBounds`/`sizeLimits`
    growMode math ported verbatim (incl. `resize_balance` recovery) and unit-tested.
    Two-stage reviewed (spec PASS, quality PASS). 173 unit + 3 integration tests
    green; clippy/fmt clean.
  - **Batch A DONE — rows 29 `TBackground` + 25 `TScrollBar`** (parallel worktree
    implementers, orchestrator-integrated). `TBackground` → `src/desktop/`
    (pattern fill, `Role::Background`, `gfGrowHiX|HiY`). `TScrollBar` →
    `src/widgets/scrollbar.rs` (value/min/max/steps, draw via the new `Glyphs`
    scrollbar set, broadcasts `cmScrollBarChanged`). The `Glyphs` stub became a
    real struct (theme.rs) seeded with the CP437→Unicode scrollbar chars (▲▼◄►/▒
    trough U+2592/▓ no-range U+2593/■ thumb) — the **row-9 "glyphs fill in
    per-widget" convention**. Reviewed (spec+quality); fixed a CP437 shade
    off-by-one and the mouse-down page-click (must thumb-**jump** to cursor, not
    page-step — page-step is keyboard-only). **DEFERRED to D9/row 31:** scrollbar
    press-and-hold auto-repeat + thumb-drag (the C++ `mouseEvent` loops);
    `ctrlToArrow` WordStar nav (shared helper, port centrally later). 197 unit + 3
    integration tests green; clippy/fmt clean.
  - **Row 26 `TGroup` DONE** (FOUNDATION, `src/view/group.rs` + `View` trait
    growth in `view.rs`): the view container + event router. `Group` owns a
    group-local `ViewArena` + `children: Vec<Child{id,view}>` in **back-to-front
    paint order** (`children[0]`=C++ `last`/bottom, `.last()`=`first()`/top;
    `forEach`/`firstThat`=`iter().rev()`), `current: Option<ViewId>` resolved by
    `index_of`. Implements: three-phase `handle_event` (D4: focusedEvents→pre/
    focused/post, positional→topmost-visible-under-cursor, broadcast→all; with the
    `doHandleEvent` disabled/eventMask/phase gates), `draw` (drawSubViews
    back-to-front, painter's algorithm — **reversed** from C++ occlusion draw, D8),
    `change_bounds`/`valid`/`awaken`/`size_limits`, and the focus machinery
    (`insert`/`remove`/`set_current`/`reset_current`/`focus_child`/`find_next`/
    `focus_next`). **Carryovers landed:** mouse-down→auto-select (relocated
    `TView::handleEvent`) + the `sfFocused` focus broadcast — via a new defaulted
    `View::set_state(StateFlag,enable,ctx)` trait method (base flips bit +
    broadcasts `RECEIVED_FOCUS`/`RELEASED_FOCUS` on `Focused`; `Group` overrides to
    propagate `Active`/`Dragging`→all, `Focused`→current) + `StateFlag` enum +
    `ViewState::set_flag`. **Deviation:** mouse position is **view-local at each
    level** (the group subtracts child `origin` before delivery — the downward
    realization of `makeLocal`/`mouseInView`); `eventError`/bubble = leaving the
    event un-cleared as the `handle_event` stack unwinds (no owner pointer).
    **Deferred (per design):** `execute`/`execView`/the live blocking loop/modal-
    via-capture/`endModal` → row 31 (the loop owns the capture stack, so a group
    can't run a modal itself); `ofTopSelect`/`makeFirst`/`putInFrontOf` Z-reorder →
    row 33 (so `select` always goes through `set_current(Normal)`); `getData`/
    `setData`/`dataSize` → D10/row 39; `Context::query` not needed (group resolves
    `ViewId` internally); shadow casting → row 33 (no infra). **Note for row 31/33:**
    `insert` does **not** activate children (faithful — C++ `insertBefore`'s
    `sfActive`-restore is a no-op under D8); a child's `sfActive` must come from the
    group's `set_state(Active)` propagation / focus logic when that lands.
    Two-stage reviewed (SPEC-PASS + QUALITY-PASS; fixed an insert-`sfActive`
    faithfulness bug the spec reviewer caught). 214 unit + 3 integration tests
    green; clippy/fmt clean.
  - Coordinates are `i32` (faithful to magiblot's `int`).
  - Deps: `unicode-segmentation`, `unicode-width`, `crossterm`; dev: `insta`.
- **Key design decisions** (recorded in `docs/PORTING-GUIDE.md` D1/D4): newtype vs
  enum by *extensibility* — open/app-extensible families (`Command`, `HelpCtx`) →
  open newtype with namespaced `&'static str` identity; closed sets (`Key`) → enum.
  Constants live with their owner (no central registry).
- Git on `main`; Phase 0 rows 1–12 committed (`010584f`); the INFRA substrate
  (rows 5,17,18,19,20 + snapshot format) committed (`7f6edd9`); Phase-0 rows
  16(min), 21, 22 committed (`8045847`); **Phase-1 row 23 (`TView`) committed**
  (`a08412d`); **Batch A rows 29+25 (`TBackground`/`TScrollBar`) committed**
  (`91c50a6`); **Phase-1 row 26 (`TGroup`) committed** at the row boundary.
  Working tree clean.

## Next step
**Phase 1 in progress.** Continue subagent-driven (see "How to run the port"
above). Sequence:

1. ~~**Row 23 `TView`**~~ ✅ DONE. The pattern every widget embeds: embed
   `ViewState`, `impl View`, draw through `DrawCtx`, events through `Context`.
2. ~~**Batch A — `TScrollBar` 25 ∥ `TBackground` 29**~~ ✅ DONE. `Glyphs` is now a
   real per-widget struct.
3. ~~**`TGroup` 26**~~ ✅ DONE (FOUNDATION, see Current state). The view container +
   three-phase router + focus machinery + both row-23 carryovers landed; modal/
   live-loop deferred to row 31.
4. **NEXT — `TFrame` 24** — FOUNDATION-ish, **main thread/Opus**. Deferred to here
   on purpose: its C++ reaches into `TWindow` (flags/`getTitle`/`number`, row 33),
   the `TGroup` sibling tree (`frameLine` tee-connectors `├┬┤┴` where nested framed
   views meet — now that `Group` exists, design the sibling-walk against the real
   `children` Vec), and `dragView` (D9). The frame glyph tables live in
   `tvtext1.cpp` (`initFrame[19]` + `frameChars[33]` single/double-line sets;
   `closeIcon`/`zoomIcon`/`unZoomIcon`/`dragIcon`/`dragLeftIcon`; active/passive/
   dragging color + `f`-offset logic) and extend the `Glyphs` struct (the row-9
   convention TScrollBar/TGroup used). Design the owner-data-down seam (TFrame holds
   title/flags/number, set by its TWindow) when TWindow's shape is designed. A full
   C++ map was captured in an earlier session (see git history / PORT-ORDER row 24).
5. **Then Phase 2 (`TWindow` 33 / `TDialog` 34) + the live loop (`TProgram` 31)** —
   these unlock the deferred modal/`execView`-via-capture, `ofTopSelect`/`makeFirst`
   Z-reorder, and child `sfActive` activation that row 26 left for them.
6. **Then the widget batches fan out hard** (PORT-ORDER Batches B–E): Phase-3
   leaves, validators, menus, dialogs, editor — the bulk `MECHANICAL` rows; run as
   parallel worktree implementer+reviewer trios, committing at batch boundaries.

The snapshot-test workflow (Appendix B step 4) is fully unlocked: build a view on
a `HeadlessBackend`, `render`, `assert_snapshot!` against the frozen format.

## Conventions
- English for all code/comments/identifiers (user-facing strings may be localized).
- Commit messages end with the project's Co-Authored-By trailer; commit/push only
  when asked.
