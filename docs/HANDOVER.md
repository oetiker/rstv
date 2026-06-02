# Session handover — row 33 (`TWindow`) COMPLETE; resume at row 34 (`TDialog`)

> Living handover for the **next** rstv session. Read this, then
> [CLAUDE.md](file:///home/oetiker/checkouts/rstv/CLAUDE.md) (orientation /
> Current state / Next step), then start. When row 34 lands, update or replace
> this file for the session after.

## Where things stand (git `main`)

| commit | what |
|--------|------|
| `7efecb3` | Phase A — `Event::Broadcast { command, source }` (D4 amendment) |
| `2887e95` | Row 33d-1 — TWindow drag + close + setState command set |
| `69897fe` | Refactor — unify the 3 deferred channels into one `Deferred` queue |
| **`15c601d`** | **Row 33d-2 — window selection (cmNext/cmPrev + Alt-N + numbered windows)** |

**Build state:** 287 lib + 3 integration + 1 doctest green; `cargo clippy
--all-targets -- -D warnings` and `cargo fmt --check` clean. Working tree clean.

**Row 33 (`TWindow`) is now COMPLETE** (33a core/primitives → 33b core → 33c zoom
→ substrate realign → Phase A → 33d-1 drag/close → 33d-2 selection). The next
FOUNDATION stage is **row 34 (`TDialog`)** — the modality payoff.

## What 33d-2 did (just landed — `15c601d`)

The *selection* half of `TWindow`. Brief:
[`docs/briefs/row33d-2-selection.md`](briefs/row33d-2-selection.md). Built:

- **`View::number() -> Option<i16>`** (trait, default `None`; `Window` overrides
  `Some(n)` iff `n>0`). Dropped Window's inherent `number()` getter — no
  same-name inherent+trait clash. Field still named `number`.
- **`View::select_window_num(num, ctx) -> bool`** (trait tree-op, default no-op;
  `Desktop` overrides → `group.focus_by_number`). Reached via the **trait method,
  NOT an `as_any_mut` downcast** — keeps `Program` decoupled from concrete
  `Desktop`. Lives in the `find_mut`/`remove_descendant` tree-op family.
- **`Group::focus_by_number`** — selects the **`ofSelectable`** child whose
  `number()` matches, via `focus_child`. **The select-vs-focus crux:** C++ uses
  `select()` (cmNext via `selectNext`, Alt-N via the window's `select()`); we have
  no standalone `select_child`, only `focus_child` (== `select()` + an outgoing
  `valid(cmReleasedFocus)` guard). That guard is **redundant-but-harmless** because
  both call sites are gated upstream (cmNext via the desktop's `valid`; Alt-N via
  `canMoveFocus`), and windows carry **`ofTopSelect`** so `focus_child`→`make_first`
  raises them exactly as `select()`→`makeFirst` does. Reasoning is in code
  comments at each call site (a spec reviewer will ask "why not `select`?").
- **TDeskTop `cmNext`/`cmPrev`** (`desktop.rs` `handle_event`, the old
  `TODO(row 33, D9)` breadcrumb): faithful `tdesktop.cpp` port. `ev.clear()` sits
  **outside** the `valid()` guard (C++ `break` falls through to `clearEvent`);
  other commands → **no** clear (C++ `default: return`). cmNext = `focus_next(false)`
  (== `selectNext(False)`); cmPrev = `put_in_front_of(current, Some(background))`
  (sends current to the back). NB `put_in_front_of`'s `target: None` means *to-top*
  — cmPrev passes the resolved `Some(background)`.
- **Alt-N (`cmSelectWindowNum`)** in `program_handle_event` — **BEFORE**
  `group.handle_event` (faithful `TProgram::handleEvent` order). A **direct walk**
  (the window number is an integer, not a `ViewId`, so the `Broadcast{source}`
  substrate does not serve it). The three-way clear matrix: `can&&matched`→clear,
  `can&&!matched`→**event stays live** (falls through to the group), `!can`→clear.
  `canMoveFocus` checks the **desktop's** `valid(RELEASED_FOCUS)` (via
  `find_mut(desktop_id)`), not the root group's. Added `desktop: Option<ViewId>`
  param to `program_handle_event`.
- **`setState`** — `{cmNext, cmPrev}` enabled **UNCONDITIONALLY** (C++ has no flag
  guard — unlike cmClose/cmZoom). `cmResize` stays **dropped** (no handler yet —
  the keyboard-resize sub-mode deviation).

Two-stage reviewed (SPEC-PASS; QUALITY-PASS after closing two test-discrimination
gaps: the no-match Alt-N test now asserts the event stayed **live** via a recording
probe — verified it bites by temporarily flipping the arm to clear; and the
cmNext-cycle test drives the enable through a **real `pump_once` drain** of a live
Alt+1 selection rather than force-enabling, which let us **delete the
`clear_deferred` test scaffold**).

## NEXT — row 34 (`TDialog`): the modality payoff

Consumes the row-31 `ModalFrame` seam. Design on the main thread; **advisor
consult before writing**; Opus implementer against a written
`docs/briefs/row34-*.md`; fresh-agent two-stage review (spec → quality);
integrate; commit at the boundary. FOUNDATION → main thread, **no worktree**.

The crux (row-31 breadcrumb in `src/app/program.rs`): a view can't reach the loop,
so **`exec_view`/`executeDialog` owns the modal run** — push the `ModalFrame`
capture handler, run the loop until `valid(end_state)`, then **pop the frame
conditional on `valid(end_state)`** (Program state a `CaptureHandler` can't reach;
this is why the pop lives in `exec_view`, not the handler). Also:

- **`cmOK`/`cmCancel`** + the modal `end_modal` path (33d-1 already wired the
  window's `cmClose`→`sfModal`→post `cmCancel` branch; row 34 owns the teardown
  that consumes it).
- **The return-consuming `message()`/`query` tree-owner primitive** — first
  consumer is the dialog `cmCanCloseForm` veto. Design of record: guide
  **D4 "message() — corrected"** (a `message(id, ev) -> Option<ViewId>` over
  `find_mut`; the audit shows every return-consuming `message()` is
  owner-initiated, so the aliasing rule bars only a pattern that never occurs).
- **`getData`/`setData`** typed gather/scatter (D10).
- **Gray window scheme** drives the deferred multi-scheme theming.
- **Any new deferred capability row 34 needs** (e.g. the modal-pop, if it routes
  through the queue) **ADDS A `Deferred` VARIANT**, not a `Context::new` param
  (the `69897fe` unification — `Deferred {PushCapture, EnableCommand,
  DisableCommand, ChangeBounds, SetState, Close}` + one `deferred: Vec<Deferred>`;
  `Context::new` is 4 params `(out_events, timers, now_ms, deferred)`).

C++ source (read it): `tdialog.cpp` `TDialog::handleEvent` (cmOK/cmCancel + the
`cmCanCloseForm`/`valid` veto); `tprogram.cpp` `TProgram::execView`/`execModal`;
`tgroup.cpp` `TGroup::execView`/`execute`/`endModal`/`valid`.

## Still deferred after row 33 (unchanged by 33d-2)
- **`cmResize` keyboard resize sub-mode** (`dragView`'s arrows-until-Enter/Esc
  branch) — no menu can trigger `cmResize` yet; enable it in `setState` only when
  menus land. `TODO(33d-2/later, D9)` breadcrumb in `window.rs`.
- **Scrollbar auto-repeat + thumb-drag** (`scrollbar.rs` `TODO(row 31, D9)`) →
  Batch B widget pass.
- **Close press-and-hold release-confirm loop** (`frame.rs` `TODO(row 33, D9)`) —
  we post `cmClose` on mouse-down.
- Sibling tee-walk, multi-scheme theming, shadow casting, row-9 glyphs — as before.

## Process reminders
- Main-thread design + **advisor consult before writing** + Opus implementer
  against a written `docs/briefs/` brief + fresh-agent two-stage review (keep
  reviewers adversarial against the **C++ + corrected guide**, not just the brief).
- Single main-thread FOUNDATION stages → **no worktree**. Commit at clean reviewed
  stage boundaries.
- **The verification that matters is the `pump_once` round-trip**, not a
  handler/capture unit test in isolation. For row 34 specifically: a modal
  `exec_view` test must drive the full push→run→pop through real pumps and assert
  the frame popped only on `valid(end_state)`.
- **Make round-trip tests discriminating** (33d-2 lesson): a test that passes
  under both the correct and the buggy branch proves nothing — assert the
  *distinguishing* observation (event stayed live / enable came from a real drain),
  and confirm the test bites by temporarily breaking the code.
- **Split a too-large stage at its natural seam** (33d → 33d-1/33d-2 worked). If
  row 34 is large, split modal-loop-mechanism from typed-data gather/scatter.

## Outstanding TODOs seeded in code (grep)
- `TODO(33d-2/later, D9)` in `src/window/window.rs` — cmResize keyboard sub-mode.
- `row 34` in `src/app/program.rs` — `exec_view`/`executeDialog` + the `ModalFrame`
  pop lifecycle + the `message()` primitive (**row 34 — NEXT**).
- `TODO(row 33, D9)` in `src/frame.rs` — close press-and-hold confirm (the frame's
  own drag cases are now handled window-side via `DragCapture`; that part of the
  breadcrumb can be trimmed).
- `TODO(row 31, D9)` in `src/widgets/scrollbar.rs` — auto-repeat + thumb-drag.
- `TODO(row 33)` in `src/view/group.rs` — shadow casting in `Group::draw`.
- Sibling tee-walk + full `framelin.cpp` machinery — deferred (`src/frame.rs`).
- Row 9 `Glyphs` continues to fill in per-widget.
