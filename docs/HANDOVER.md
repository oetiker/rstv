# Session handover — Row 32 `TApplication` DONE. Next (per PORT-ORDER): Phase 4 (menus 46+ / status 47,53)

> Living handover for the **next** rstv session. Read this, then
> [CLAUDE.md](file:///home/oetiker/checkouts/rstv/CLAUDE.md) (orientation /
> Current state / Next step), then start. When the next stage lands, update or
> replace this file for the session after.
>
> **Direction = [`docs/PORT-ORDER.md`](file:///home/oetiker/checkouts/rstv/docs/PORT-ORDER.md).**
> It is dependency-ordered; follow it in sequence rather than treating "tracks" as
> an open choice. Lowest-numbered incomplete rows are the work. The
> "Parallelizable batches" section (e.g. Batch C validators 58–62) lists fan-outs
> that *may* run concurrently — an efficiency, not a competing direction.

## Where things stand (git `main`)

| commit | what |
|--------|------|
| `c1ad789` | **TListViewer (28)** — list base (trait) + write-back broker (FOUNDATION) |
| `fc66637` | **TListBox (48)** — first concrete `TListViewer` (MECHANICAL) |
| `3e6645f` | **TApplication (32)** — thin D2 wrapper over `Program` (MECHANICAL) ← THIS session |

**Build state:** 494 lib + 3 integration + 2 doctests green; `cargo clippy
--all-targets -- -D warnings` and `cargo fmt --check` clean. Working tree clean
(after the docs commit that pairs with this handover update).
(Cargo artifacts land in `/home/oetiker/scratch/cargo-target` — set
`CARGO_TARGET_DIR`.)

**Phase 2 COMPLETE. Batch B (Phase-3 leaves) COMPLETE. Phase-1 row 32 COMPLETE.**
**Row 32 `TApplication` DONE** this session (MECHANICAL, thin wrapper over
`Program` — see below). Phase-4 (46+) + the remaining list/dialog leaf rows
remain. Next incomplete in PORT-ORDER sequence: **Phase 4** (menus 46/49/50/51/52,
status 47/53). Batch C concrete validators 58–62 are an available parallel fan-out.

> **Repo note (this session):** the working checkout was parked on an unmerged
> side branch `feat/delegate-macro` (scaffolds a `tvision-macros` proc-macro
> crate — the deliberately-revisited row-48 delegation-macro idea). Row 32 was
> intentionally branched from **`main`** (independent of the macro WIP, user-
> confirmed) and `main` fast-forwarded to it. `feat/delegate-macro` is untouched.
> Worktrees live under `/scratch/oetiker/claude-worktrees/<project>-<branch>`
> (per global CLAUDE.md) — the Agent tool's `isolation:"worktree"` puts them in
> the wrong place; create them manually + dispatch a non-isolated subagent.

## What landed THIS session — Row 32 `TApplication` (`3e6645f`, MECHANICAL)
The thin D2 embed wrapper over `Program` (row 31): `Application { program: Program }`,
the type a real app constructs. **Genuinely thin by dependency order**
(advisor-confirmed) — all of `TApplication`'s substance is deferred, so the row is
the type + one real body + faithful breadcrumbs, deliberately NOT padded. Built
main-thread/Opus-orchestrated: tight brief (`docs/briefs/row32-tapplication.md`) →
Sonnet implementer (in a `/scratch` worktree) → spec review (fresh C++-adversarial
agent) → fixes → integrate.

- **`Application`** forwards `run`/`pump_once`/`exec_view`/`desktop`/`end_modal`/
  `end_state`/`{enable,disable,command_enabled}_command` + `program()`/`program_mut()`
  escape hatches — hand-written one-liners, **NO delegation macro** (the reverted
  row-48 creep; the macro is now its own deliberate branch `feat/delegate-macro`).
- **`get_tile_rect()` is the one real body** → new **`Program::get_tile_rect`**
  (the desktop child's extent = `deskTop->getExtent()`, local-origin `(0,0,w,h)`,
  `None` if no desktop; `&mut self` because `Group::find_mut` is `&mut`). Placed on
  `Program` (not `Application`) because `Application` can't reach the private `group`,
  and the future command handler — also in `Program` — reuses it.
- **Deferred (NO dead stubs — omit-until-consumer, the row-35/48 rule):**
  `tile`/`cascade` (need `Desktop::tile`/`cascade` geometry [`mostEqualDivisors`/
  `iSqr`/`calcTileRect`/`dividerLoc`/`doCascade`, `tdesktop.cpp`] + a menu to emit
  cmTile/cmCascade + a way to test → Phase 4); `dosShell`/`suspend`/`resume` (need a
  backend terminal-suspend seam + SIGTSTP); `initHistory`/`doneHistory` (history
  subsystem unported); `TAppInit` subsystem init **dropped** (subsumed by the
  `Backend`/`Renderer` construction path).
- **Command handling breadcrumbed, not wired:** `TApplication::handleEvent`'s
  cmTile/cmCascade/cmDosShell are **program-level** → a TODO in `program_handle_event`
  **after** `group.handle_event` (faithful: C++ runs `TProgram::handleEvent` first),
  beside the QUIT catch. Blocked on the deferred bodies. The consts
  `Command::{TILE,CASCADE,DOS_SHELL}` already exist + are enabled in
  `default_command_set`, but **nothing emits them yet (no menus)** — Phase 4 menus
  are the first emitters; when they land, wire this breadcrumb + build the desktop
  geometry together (trigger + body + test in one go).
- **Review caught + fixed a BLOCKER:** the implementer first added empty
  `tile`/`cascade`/`dos_shell` methods on `Application` — dead stubs (the planned
  handler is in `program_handle_event`, which can't reach `Application`); deleted,
  deferral kept in docs + the breadcrumb. Plus 2 MINORs fixed: breadcrumb moved
  post-dispatch; the `get_tile_rect` test made discriminating (inset 80×20 desktop on
  an 80×25 backend pins desktop-extent vs screen-extent — a screen-rect impl fails it).

### Prior session — Row 48 `TListBox` (`fc66637`, MECHANICAL)
The first **concrete** `TListViewer`, proving the row-28 trait seam end to end.
Built main-thread/Opus-orchestrated: tight brief
(`docs/briefs/row48-tlistbox.md`) → Sonnet implementer → full two-stage review
(SPEC then QUALITY, fresh C++-adversarial Opus agents) → integrate.

`ListBox { lv: ListViewerState, items: Vec<String> }` (`src/widgets/list_box.rs`)
reuses **all** of `TListViewer`'s draw/event/nav verbatim via the `ListViewer`
trait, overriding **only `get_text`** (`items.get(item as usize).cloned().
unwrap_or_default()` — collapses the C++ `items==0→EOS` + OOB cases, panic-free);
`is_selected`/`select_item` **inherit the base** (C++ overrides neither). `impl
View` delegates `draw`/`handle_event`/`set_state`/`cursor_request`/
`apply_list_scroll`/`as_any_mut` to the `list_viewer::*` free fns (the `FakeList`
template). Wired into `widgets/mod.rs` + `lib.rs`.

- **D10 value protocol — first consumer beyond `TInputLine`:** `value() →
  FieldValue::Int(focused)` (the `getData` selection half; the collection is
  config `new_list` manages, NOT part of the transferable value — no `List`
  variant, `FieldValue` grows per consumer).
- **`set_value` DEFERRED** (advisor-confirmed): the **`Context`-free**
  `View::set_value` signature can't republish the v-bar (C++ `setData` =
  `newList`+`focusItem`, both need a `Context` in our model), so a partial would
  leave the scroll thumb desynced after a scatter. Lands with the **dialog
  gather/scatter** consumer (inputBox/Batch E), which must itself solve threading
  a `Context` into scatter. `TODO(set_value: dialog gather/scatter)`.
- **Population is post-insert** (the ctor has no `Context`): `new_list(items,
  ctx)` (`set_range` + `focus_item(0)` iff `range>0`) **plus**
  `list_viewer::update_steps(ctx)` for the page/arrow steps — miss either and the
  thumb starts unsynced. Documented on the type.
- **Dropped:** `dataSize`/`TListBoxRec` (→ typed value), streaming (D12),
  `drawView` (D8). The dialog gather/scatter group-walk stays deferred (no
  consumer yet).
- **Process catch — out-of-scope creep reverted:** the implementer also added an
  exported `delegate_view_rest!` macro to `src/view/view.rs` + refactored
  `examples/hello.rs` to use it — unrelated to row 48, unreviewed (both review
  agents were scoped to `list_box.rs`), and touching a FOUNDATION file. Reverted
  before commit. The macro is a genuinely useful D2-embed helper; if wanted, do it
  deliberately as its own reviewed change.

### Prior session — Row 28 `TListViewer` (`c1ad789`, FOUNDATION)
`TListViewer` (base for `TListBox` 48, history, color/file lists) drives two
sibling scrollbars like `TScroller` but **diverges structurally in two ways** the
"reuse the broker verbatim" line glossed over — both confirmed with the advisor
*before* building. Built main-thread/Opus: brief → Opus implementer → two-stage
review (SPEC then QUALITY, fresh C++-adversarial agents) → fixes. Brief:
`docs/briefs/row28-tlistviewer.md`.

**Divergence 1 — `ListViewer` is a TRAIT, not a concrete struct (the `Validator`
pattern, NOT the `Scroller` embed shape).** `TListBox` reuses `TListViewer::draw`
while *overriding* the virtuals `getText`/`isSelected`; a D2 concrete-embed base
physically cannot dispatch back into the embedder's `getText` from the base's own
`draw`. So:
- `ListViewer: View` trait — `lv()`/`lv_mut() -> &ListViewerState` accessor +
  defaulted `get_text`/`is_selected`/`select_item`.
- `ListViewerState` struct holds the data members (`state: ViewState`, `num_cols`,
  `top_item`, `focused`, `range`, `indent`, `h_scroll_bar`/`v_scroll_bar` ids).
- The shared draw/event/nav logic lives as **free functions generic over
  `<L: ListViewer + ?Sized>`** (`list_viewer::draw`/`handle_event`/`focus_item`/
  `focus_item_num`/`set_range`/`update_steps`/`apply_scroll`/`set_state`/
  `focused_cursor`), which a concrete widget's `View` impl calls.
- Object-safety: `ListViewer` is **not** object-safe (`get_text -> String`) — fine,
  it's only ever a generic bound; concrete widgets are still `Box<dyn View>`.
- A `#[cfg(test)] FakeList` (Vec-backed) is the first consumer (a real consumer for
  the draw/nav tests, NOT a dead stub). **Row-48 `TListBox` is the production one.**

**Divergence 2 — the read-sync WRITES BACK (the scroller never did).** C++
`focusItem → vScrollBar->setValue(item)`; in our model the read-sync issues a
deferred `ScrollBarSetParams{value}`. New mechanism, **scroller path untouched**:
- New defaulted-no-op **`View::apply_list_scroll(&mut self, h, v, ctx)`** + new
  **`Deferred::SyncListViewer{list,h,v}`** + a pump apply arm that calls the **trait
  method (NO downcast** — you can't cast `dyn View → dyn ListViewer`, unlike the
  scroller's `as_any_mut` downcast to a single concrete type).
- **TERMINATION (the centerpiece property):** the vbar→sync→setValue cycle
  terminates **only because `ScrollBar::set_params` is change-guarded**
  (`scrollbar.rs:219/224` — broadcasts `SCROLL_BAR_CHANGED` iff `old_value !=
  a_value`), so the write-back of the already-current value is a silent no-op.
  Proven by a discriminating termination test through real `pump_once` drains
  (6 passes asserting quiescence; bite-checked — removing the guard makes it spin).
- **`indent` cached** on `ListViewerState`: draw can't read the sibling hbar live,
  so the hbar `value` is cached and refreshed by the same sync (the hbar
  `cmScrollBarChanged` branch, C++ "just drawView", becomes "update the cache").

**Reused verbatim from row 27:** `Deferred::ScrollBarSetParams` (setRange +
ctor-setStep) and `SetVisible` (setState show/hide), `Broadcast{source}` as the
`source ∈ {h,v}` filter, `View::value() → FieldValue::Int`.
- **`setState`** uses the C++ **`active && visible` AND-condition** for show/hide
  (NOT the scroller's `active || selected` — a spec-review crosshair).
- **`cmScrollBarClicked` from an own bar → `select()`** → `ctx.request_focus(id)`
  (the row-41 `Deferred::FocusById` seam).
- **Theme reconciled** to the 5-entry cpListViewer palette (`Active/Inactive/
  Focused/Selected/Divider`) → roles `ListNormalActive`/`ListNormalInactive`/
  `ListFocused`/`ListSelected`/`ListDivider` (the old guessed `ListNormal`/
  `ListSelectedFocused` were unused; provisional colours, `TODO(window-scheme
  remap)`).
- **Deferred + breadcrumbed:** mouse press-and-hold/auto-scroll `do…while
  (mouseEvent)` loop (`TODO(row 31, D9)`; ship single-shot + double-click select);
  `changeBounds` step republish (`TODO(resize)` — **note the distinct formula**:
  C++ `changeBounds` uses vbar plain `size.y` + **both bars preserve arStep**,
  unlike the ctor's `update_steps`; do NOT call `update_steps` for resize —
  corrected in-doc after a spec catch); `showMarkers` + streaming dropped (D8/D12);
  scroller/listviewer read-sync unification noted optional/out-of-scope.

### Prior session — Row 27 `TScroller` (`543b2c8`, FOUNDATION)
Established THE cross-view scrollbar broker (pump brokers all scroller↔scrollbar
reads/writes at deferred-apply via `group.find_mut(id)` + `as_any_mut`/
`View::value()`; `Broadcast{source}` is the filter, value NOT stuffed into the
message). New `Deferred`: `SyncScrollerDelta` (read → `apply_delta`),
`ScrollBarSetParams` (write, per-field `Option`=preserve), `SetVisible`. New seams
`FieldValue::Int` + `ScrollBar::value()`. Dropped (D8) `drawLock`/`drawFlag`/
`checkDraw`/`drawView`. `Role::ScrollerSelected` + `changeBounds` resize-republish
deferred to `TEditor` 66. Brief: `docs/briefs/row27-tscroller.md`.

## What landed the PRIOR session (validator wave, `43e5c68`)
The full row-35→39 wave + the **D10 typed-value protocol**, built as one Opus
implementer + full two-stage review (SPEC then QUALITY, fresh C++-adversarial
agents). Brief: `docs/briefs/row35-39-validator-inputline.md`.

- **TValidator (35)** → `src/validate.rs`: object-safe abstract `Validator` trait
  (D2) — `is_valid_input(&self,&mut String,bool)` / `is_valid(&self,&str)` /
  `error` / `is_status_ok` (all defaults accept) + provided non-virtual
  `validate`. **`transfer` deliberately omitted** (PORT-ORDER row 35 lists it, but
  it has no overrider until TRangeValidator row 59 → would be a dead stub; the
  row-34 "no dead stubs" rule wins). `tv::Validator`.
- **D10 value protocol** → `src/data.rs`: **`FieldValue`** typed-transfer currency
  — one `Text(String)` variant, **grows per control** (Role/Glyphs convention;
  `Bits(u32)` for cluster + `Int` for range land when those wire their value).
  Defaulted **`View::value(&self)->Option<FieldValue>` / `set_value(&mut self,
  FieldValue)`** (the getData/setData successors). The dialog **gather/scatter
  group-walk is DEFERRED** to its first consumer (inputBox / Batch E) —
  breadcrumbed in `data.rs`.
- **TInputLine (39)** → `src/widgets/input_line.rs`: faithful `tinputli.cpp` port.
  Draw (scrolled `moveStr` + ◄/► arrows + selection redraw + cursor), full
  keyboard (nav / word-nav / edit / Ins-toggle / Shift-block-extend /
  printable-insert with the `maxLen && maxWidth && maxChars` guard / Ctrl-Y),
  single-shot mouse positioning **+ the faithful single edge-click scroll-by-one**,
  validator `save_state`/`restore_state`/`check_valid`, `valid(cmd)` (faithful
  return), `set_state`→`select_all`, `value`/`set_value`.
  **Key correction the implementer caught:** `first_pos` is a display **COLUMN**,
  not a byte offset (the brief mis-stated it; `cur_pos`/`sel_*`/`anchor` ARE byte
  offsets). All `data` indexing steps through grapheme helpers — **D13
  panic-safe** (multi-byte tests over `ä€中` BITE).
- **New seams:** `text::prev` (`TText::prev`), `DrawCtx::put_str_part` (`moveStr`'s
  `begin` column-skip), 3 theme roles `Input{Normal,Selected,Arrow}` (provisional
  gray, `TODO(row 34 gray theming)`) + 2 glyphs (◄ U+25C4 / ► U+25BA), `cmValid`,
  `State::cursor_ins`.
- **End-to-end veto test (`8ea87cb`, advisor-flagged):** the headline
  `InputLine::valid()` behavior — a modal must NOT close on OK while a child's
  validator rejects — lived only in isolated widget tests. The actual veto is in
  `exec_view`'s outer `while !valid(end_state)` loop. New integration test in
  `program.rs`: a `Dialog` + `InputLine` + `RejectAll` validator, driven through
  `exec_view` with pre-queued `[cmOK, cmCancel]`, asserts the result is **cmCancel**
  (cmOK vetoed, modal stayed open) + the `ModalFrame` popped. Bite-verified; **no
  bug in the veto path** (`exec_view` honors `valid()` correctly). The `[OK,
  CANCEL]` shape is deliberate — `[OK]` alone loops forever (a permanently-rejecting
  field can never close, which IS faithful). + a `#[cfg(test)] Dialog::insert_child`
  hook.

### Deferred + breadcrumbed in the validator wave (prior session; grep the TODOs)
- **clipboard** cmCut/cmCopy/cmPaste — no `Context` clipboard seam (backend has
  set/get_clipboard; not surfaced to views). `TODO(clipboard)` in `input_line.rs`.
- **command-graying** `updateCommands`/`canUpdateCommands` (enable/disable cmCut/
  Copy/Paste) — needs the `Context` command-set query that **TButton also
  deferred**. `TODO(button/inputline: command-set query …)`. **Menus (Phase 4)
  force this** — add a read-only command-set accessor to `Context` then.
- **mouse press-and-hold / drag-select loops** — `TODO(row 31, D9)`; single-shot
  positioning + the single edge-click scroll only.
- **`valid()`'s `select()` focus side-effect** — C++ focuses the invalid field
  before returning false; needs `&mut Context` + the **focus-by-ViewId** seam
  (`Deferred::FocusById` / `request_focus`, already built at row 41).
  `TODO(valid-select)`. The **return value is faithful** (gates modal OK).
- **validator `transfer` hook** — `TODO(row 59)` at both `value`/`set_value`
  sites; TRangeValidator will produce a typed non-`Text` value (→ `Int`).
- **`Validator::error`→msgbox** — `TODO(msgbox row 63)`.
- **`cur_pos` re-clamp hazard** — `TODO(row 59/62)`: a future *mutating* validator
  that SHRINKS `data` could leave `cur_pos` past EOS / mid-grapheme → D13 panic.
  Unreachable now (abstract validator never mutates); re-clamp when the first
  auto-fill validator (Range/PXPicture) lands.

## NEXT — follow PORT-ORDER in sequence

Lowest-numbered incomplete rows = the work. Next up:

### Row 32 `TApplication` — ✅ DONE this session (`3e6645f`)
See "What landed THIS session" above. Note for Phase 4: when menus emit
cmTile/cmCascade/cmDosShell, the deferred bodies land **together** — build
`Desktop::tile`/`cascade` geometry (`mostEqualDivisors`/`iSqr`/`calcTileRect`/
`dividerLoc`/`doCascade`, `tdesktop.cpp`) + wire the breadcrumb in
`program_handle_event` (after `group.handle_event`, beside the QUIT catch, calling
`desktop.tile/cascade(get_tile_rect())`) + test it with real tileable windows in
one change. `dosShell` separately needs a backend terminal-suspend seam + SIGTSTP.

### Phase 4 — the immediate next work, in PORT-ORDER order
- **Phase 4 — menus + status line** (the path to a fully drivable app):

  **Menus:** `TMenuItem`/`TSubMenu`/`TMenu` (46, FOUNDATION — the menu data tree;
  C++ `operator+` builders → a Rust builder API) → `TMenuView` (49, FOUNDATION —
  hotkey/shortcut dispatch, the `evBroadcast` mask) → `TMenuBar` (50) / `TMenuBox`
  (51) / `TMenuPopup` (52, popup exec via D9). **Menus force the deferred
  `Context` command-set query** (command graying) — build that read-only accessor
  on `Context` when you hit it (additive; the deferred-effects refactor stabilized
  `Context::new` for *effects*, a read accessor is a separate additive concern).
- **Status line:** `TStatusItem`/`TStatusDef` (47) → `TStatusLine` (53,
  FOUNDATION — hint()/help-ctx→hint mapping).
- Wiring menus + status line into `Program` lets the `examples/hello.rs` demo grow
  a real menu bar + status line (and shifts the desktop down — revisit the
  `ModalFrame`/`DragCapture` "(0,0)-desktop absolute-coords" caveats then).

### Available parallel fan-out (efficiency, not a competing direction) — Batch C: concrete validators (58–62, MECHANICAL)
Fully unblocked by `TValidator` (35); **fully parallel among themselves** → the
clean worktree fan-out cadence (Sonnet implementers, `isolation:"worktree"`,
orchestrator integrates + pre-seeds any shared files). These are PORT-ORDER's
"Parallelizable batches" — run them concurrently whenever convenient; they don't
displace the in-sequence FOUNDATION work above. C++ all in `tvalidat.cpp`:
- **58 `TFilterValidator`** (char allow-list), **59 `TRangeValidator`** (int range;
  **resolves the deferred `transfer` hook + the `cur_pos` re-clamp hazard** above —
  and now has `FieldValue::Int` ready [added by row 27]; so this one is
  FOUNDATION-ish, do it carefully),
  **60 `TLookupValidator`** (abstract lookup), **61 `TStringLookupValidator`**,
  **62 `TPXPictureValidator`** (Paradox picture-mask state machine — the big one;
  `picture()`/`process()`/`scan()`/`group()`/`iteration()` — sets `status=vsSyntax`,
  which is what `is_status_ok()` and TInputLine `valid(cmValid)` already consult).
Each validator's `is_valid_input` may **mutate** `s` (auto-fill) — that's the
trigger for the TInputLine `cur_pos` re-clamp `TODO(row 59/62)`.

### Then `msgbox` (63) + Batch E fan out
`messageBox`/`inputBox` (`msgbox.cpp`) is buildable now (TButton + TStaticText +
TInputLine exist) but is the **first consumer of the D9 view-triggered async-modal
path** (`Deferred::OpenModal` + posted completion `Command`) — guide D9 "exec_view
— corrected" carries that design; build when a menu/msgbox needs it (Phase 4), not
before. Batch E dialog families (color/file/chdir/editor/outline/textview) fan out
once their leaf prereqs exist.

## Standing process reminders
- **Fan-out cadence is for gap-free MECHANICAL leaves only** (parallel worktree
  implementers, `isolation:"worktree"`, Sonnet, orchestrator integrates shared
  `mod.rs`/`lib.rs` + pre-seeds `theme.rs`). **FOUNDATION rows → per-row, Opus,
  full two-stage review.** Commit completed rows before dispatching worktree
  agents that build on them (worktree branches from the last *commit*).
- **Two-stage review stays mandatory** (SPEC then QUALITY, fresh C++-adversarial
  agents against the **C++ + guide, NOT the brief** — the brief can be wrong, as
  the validator wave's `first_pos` mis-statement proved). Make round-trip/unit tests
  **discriminating + bite-checked** (verify a finding fails before/passes after).
  Both stages keep earning their keep: at row 27, **spec** review caught an invented
  active/selected `draw` branch (the base inherits `TView::draw`'s uniform fill) and
  **quality** caught `std::any`-vs-`core::any` + a stale doc; in the validator wave,
  quality caught the untested validator reject/restore path and spec caught a dropped
  double-click scroll.
- **Snapshot workflow** (Appendix B step 4): `cargo-insta` is NOT installed →
  generate a `.snap` with `INSTA_UPDATE=always cargo test <name>`, verify by hand,
  re-run plain, commit the `.snap`.
- Keep per-row briefs **tight + self-contained + inline** (over-long briefs crashed
  a Sonnet implementer's context earlier in Batch B).

## Older standing deferrals (still open, grep the code)
- **`Context` command-set query** (command-graying) — TButton + TInputLine both
  wait on it; **Phase-4 menus force it**.
- **phase signal on `Context`** (plain-letter postProcess accelerator) — 3 waiting
  consumers: button, label, cluster (`is_plain_hotkey` exists but is ungated).
- **`Group::remove` release-after-remove ordering** — a removed selectable child
  never gets `RELEASED_FOCUS{source}`; a `TLabel` whose link is removed at runtime
  keeps a stale `light`. C++ `hide()`s before `removeView`. No consumer hits it yet.
- **`cmResize` keyboard sub-mode** (`window.rs`); **scrollbar auto-repeat +
  thumb-drag** + **cluster drag-cursor** (`TODO(row 31, D9)`); **close
  press-and-hold confirm** (`frame.rs`); **sibling tee-walk** (`framelin.cpp`);
  **shadow casting** (`group.rs`); **gray multi-scheme theming**
  (`TODO(row 34 gray theming)` — realign provisional `*` colours, incl. the 3 new
  Input roles); **row-9 glyphs** continue per-widget.
- **ctrlToArrow / accelerator TODOs** in cluster/scrollbar — shared key helpers
  EXIST (`b53c618`); retire opportunistically.
