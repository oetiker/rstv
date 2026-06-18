# Spec: consumer-facing API gaps surfaced by the `tcv` example

**Status:** open — for a future session.
**Origin:** building `examples/tcv.rs` (a faithful re-port of the 1993 Turbo
Pascal program *Tobi's Catalog Vision*) as an outside *consumer* of the public
API surfaced three places where tvision-rs can't do what C++ Turbo Vision did
trivially. Internal widgets never hit these — they aren't `pub(crate)`-restricted
and each got a bespoke deferred variant — so the gaps only show when you build a
real app from the published surface.

Two were also live layout bugs in the example, already fixed in commit
`f47eaa3` (search-overlay column accumulation; button-row inset). This spec
covers the *framework* gaps that remain. Verify on the integrated tree with
`CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target`, `-j4`.

When each lands: update `examples/tcv.rs` to drop the corresponding workaround
(documented in its header) and tighten its tests, then add an IMPLEMENTATION-LOG
entry.

---

## Root cause (the deeper issue — read this first)

A three-axis API-surface audit (C++ source of truth vs the exported Rust
surface, 2026-06-18) confirmed the three `tcv` gaps are **not three unrelated
bugs**. They are three faces of **one** root cause:

> **The port faithfully reproduced C++ *internal behavior*, but the public
> surface was only ever exercised by the framework's own widgets. `tcv` is the
> first true outside consumer, so it is the first thing to hit the angles
> internal code never needed.**

That single cause expresses itself through **three mechanisms** — which is why the
gaps looked unrelated. Each known gap is the first symptom of its mechanism; the
audit found **siblings** of each that a *second* consumer would hit. Fixing only
the three named gaps leaves the siblings to be re-discovered. So this spec is
reframed as **one consumer-API coverage pass, organized by mechanism**.

| Mechanism | C++ gives consumers | We scoped it to | Known gap | Siblings found |
|---|---|---|---|---|
| **A. Visibility** of concrete-class config | public mutable fields | exactly the in-crate subtype that needed the setter (`pub(crate)`, comment *"used by subtypes such as Dialog"*) | #1 Window/Dialog flags | Window/Dialog **palette** also hardcoded; `grow_mode`/`drag_mode` setters `pub(crate)` |
| **B. Environment-actions from inside a view** | `owner`/`TProgram` → a view can do *anything* outward | a hand-enumerated `Deferred` allowlist (the rule *is* "a new capability ADDS A VARIANT") | #2 no generic `ExecView` | no `insert(arbitrary view)` from a view; no payload `message(receiver,…)`; no set-valued `enableCommands(TCommandSet&)`; no `selectNext`/`focusNext` from a view |
| **C. `Group` virtual overrides** | `TGroup` overrides delegating to the focused child | only the overrides internal Window/Dialog needed | #3 `get_help_ctx` not overridden on `Group` | `getData/setData/dataSize` exists only as inherent `gather_data`/`scatter_data`, invisible to the polymorphic `View`/`Context` surface |

**What is NOT a gap (verified — do not "fix" these):**
- **Leaf widgets are consumer-shaped**, often *more* open than C++ (`pub
  am_default`, `ButtonFlags`, `pub set_text`, public `InputLine`/`ListViewer`
  fields). The closure is concentrated in the **Window/Dialog container family**
  (axis A) and the **from-a-view `Context`/`Deferred` channel** (axis B).
- **The top-level `Program` surface is already generic** (`exec_view`,
  `desktop_insert`, command enable/disable). Only the *downward-Context* axis —
  the seam the no-up-pointer architecture (D3/D9) forces view-originated effects
  through — is an allowlist.
- **Deliberate locked-decision deviations, not gaps:** `redraw`/`lock`/`unlock`
  (moot under whole-tree redraw); `getPalette` (folded into `Theme`); the `owner`
  up-pointer (dropped by D3); `makeGlobal`/`makeLocal` (Group-internal).
- Audit corrections worth recording: `StaticText::set_text` **is** `pub` (not a
  gap); `TStatusLine::defs` / `TMenuView::menu` are **`protected`** in C++ (built
  before the ctor on both sides) → `StatusLine`/`MenuBar` are **parity**, not gaps.

**Suggested order:** A (visibility) → C (Group overrides) → B (the `Deferred`
channel). A and C are small, high-value, and make `tcv` faithful on the cheap; B
is the larger unlock (custom modals + arbitrary outward actions from a view) and
needs its result-delivery design settled first — and that one design decision
likely templates all of B's siblings.

---

## Axis A — concrete-class configurability is `pub(crate)`-frozen on Window/Dialog  *(low risk; do first)*

### A.1 Window/Dialog decoration flags are not publicly settable *(the known gap #1)*

**Gap.** A consumer cannot configure a window's decoration/behavior flags or its
drag/grow modes. So every `Dialog` is movable and wears a close box; you cannot
build a fixed, icon-less, full-desktop panel.

**Evidence.**
- `src/window/window.rs:237` — `pub(crate) fn set_flags(&mut self, flags: WindowFlags)`
- `src/window/window.rs:267` — `pub(crate) fn set_grow_mode(...)`
- `src/dialog/dialog.rs:117` — `pub(crate) fn set_flags(...)`
- `Window::new(bounds, title, number)` / `Dialog::new(bounds, title)` take **no**
  flags argument; `Dialog::new` hardcodes `move | close`.
- `WindowFlags` (fields `r#move`/`grow`/`close`/`zoom`) and `Window::flags()`
  getter **are** public — only the setters/constructors are closed.
- **Escape hatch (undocumented):** `grow_mode`/`drag_mode` are `pub` fields on
  `ViewState`, reachable via the `pub` `View::state_mut()` (`src/view/view.rs`),
  so `widget.state_mut().grow_mode = …` already works — but there is no
  discoverable setter, and for the `Window` family `state_mut()` reaches the inner
  group's state. This is a missing *convenience setter*, not a true closure — but
  it should be minted and documented.

**C++ baseline.** `TWindow.flags`, `TView.dragMode`, `TView.growMode` are public
fields, freely assigned. TCV.PAS:
`Window^.Flags := $00; Window^.DragMode := $00; Window^.GrowMode := $00;`
→ a fixed, iconless full-desktop panel (still framed + titled).

### A.2 Window/Dialog **palette** is hardcoded too *(sibling of #1 — do in the same change)*

**Gap.** `set_palette` is `pub(crate)` (`src/window/window.rs:253`); the ctor
forces Blue (Window) / Gray (Dialog). A consumer wanting a Cyan window — a
first-class TV concept (`wpCyanWindow`) — has no public path. Found by the audit
in the same file as #1, with the same `pub(crate)` "used by subtypes" comment
(`window.rs:230`); fixing it is the same shape of change.

**C++ baseline.** `TWindow::palette` is a public field; `wpBlueWindow` /
`wpCyanWindow` / `wpGrayWindow` are freely assigned.

### A — proposed fix (covers #1 + palette + grow/drag)

Expose configuration after construction (and/or builders), keeping every current
default unchanged — only add knobs:
- `pub fn Window::set_flags(&mut self, WindowFlags)` (un-`pub(crate)`) +
  `with_flags(self, WindowFlags) -> Self`.
- Public `set_grow_mode`/`with_grow_mode` + a drag-mode equivalent (and document
  the `state_mut()` field as the low-level seam).
- Public `set_palette`/`with_palette` (the `WindowPalette` selector).
- Mirror all on `Dialog` (`set_flags`/`with_flags`/`set_palette`/…).

**Verify.** A headless test builds a desktop-filling window with all flags off and
confirms the frame draws no close/zoom icon and the window doesn't move on a frame
drag; a second asserts a non-default (Cyan) palette renders. Then update `tcv.rs`
to make the catalog window the fixed panel the original was.

---

## Axis C — `Group` does not override the C++ `TGroup` aggregating virtuals  *(low–medium risk)*

### C.1 `get_help_ctx` does not bubble to the focused child *(the known gap #3)*

**Gap.** The status line shows the wrong help context for nested focus — e.g. in
`tcv` it stays on "BROWSE MODE" while the list is actively searching, because the
list's `help_ctx` never reaches the status line.

**Evidence.**
- `src/app/program.rs:1757` — the idle arm reads `captures.top_modal_view()` then
  `v.get_help_ctx()`.
- `src/view/view.rs:965` — the default `View::get_help_ctx` returns the view's
  **own** `state().get_help_ctx()`; `Group`/`Window`/`Dialog` do **not** override
  it to delegate to the focused (`current`) child (confirmed: no `get_help_ctx` in
  `src/view/group.rs`).
- Net: a leaf's help context can't propagate up to the top modal the status line
  reads. (The C7 work wired the *read*; the *bubble* was never added.)
- `tcv.rs` works around it by caching the list's `help_ctx` into the `DataWindow`
  in `handle_event` — a bandaid that still can't reach the status line because the
  window isn't the top modal.

**C++ baseline.** `TGroup::getHelpCtx()` returns `current->getHelpCtx()`
(recursively to the focused leaf), falling back to the group's own when there's
no current / while dragging.

**Proposed fix.** Override `get_help_ctx` on `Group` (so `Window`/`Dialog` inherit
via their embedded group) to return the focused child's `get_help_ctx`
recursively, falling back to own state when there is no focused child. Preserve
the existing dragging-flag behavior (`view.rs:452`, test at `view.rs:1136`). This
is a `View` trait method → check whether a `tvision-rs-macros/src/specs.rs`
forwarder / `delegate_view` spy update is needed (per the HANDOVER process note).

**Verify.** Re-enable a status-line assertion in `tcv.rs`
(`search_does_not_corrupt_focused_row` or a new test) confirming the line reads
"SEARCH MODE" while searching and "BROWSE MODE" otherwise; drop the `DataWindow`
caching hack. Confirm `hello`/`tvedit`/other examples' status lines are
unaffected. Snapshot.

### C.2 `getData`/`setData`/`dataSize` are not on the polymorphic surface *(sibling — decide while in `Group`)*

**Gap.** C++ `TGroup` overrides `getData`/`setData`/`dataSize` to gather/scatter
across children polymorphically. In Rust these exist only as **inherent** methods
`Group::gather_data` / `Group::scatter_data` (`src/view/group.rs:228/244`, `pub`)
— there is no `dataSize` analogue and they are **invisible to the `View` trait /
`Context`**. A consumer with a `&mut dyn View` (or a leaf via `Context`) cannot
gather/scatter a sub-form. The D10 value protocol (`View::value`/`set_value`)
covers the single-field case but not the group walk.

**C++ baseline.** `TView::dataSize/getData/setData` are virtual; `TGroup`
overrides them; consumers call them through the base-class pointer.

**Decision to settle (not necessarily implement now).** Either (a) lift
gather/scatter onto the `View` trait so it's polymorphic, or (b) document that the
group walk is intentionally inherent-only and that consumers use `value`/`set_value`
per field. Pick deliberately; record the reason (per the "no deferred state" rule
— ported OR deliberately-not-with-reason, never silently absent).

---

## Axis B — the `Deferred`/`Context` channel is a closed allowlist  *(medium risk; the bigger one)*

The no-up-pointer architecture (D3/D9) routes every view-originated outward effect
through `&mut Context` → a `Deferred` variant. The audit found this catalog is a
**fixed menu built for internal widgets**: ~22 of its 30+ variants are brokers for
a *specific named widget* (`SyncScrollerDelta`, `SyncListViewer`, …) or launchers
for a *specific built-in dialog* (`OpenMessageBox`, `OpenFindDialog`,
`OpenColorDialogForRole`, …); only ~8 are generic primitives. The design rule "a
new deferred capability ADDS A VARIANT" (`docs/design/deferred-effects.md`) **is**
the closed-allowlist pattern. The arbitrary forms C++ gives a view for free were
never added because no internal widget needed them.

### B.1 No generic deferred modal — a view can't `ExecView` an arbitrary dialog *(the known gap #2)*

**Gap.** A leaf view cannot pop up a *custom* modal dialog. The deferred
modal-from-a-view seam only offers a fixed catalog of built-in popups; there is no
"exec this `Box<dyn View>` I just built."

**Evidence.** `src/view/context.rs` request-modal surface is all specific:
`request_message_box` (1356), `request_save_as_dialog`, `open_color_dialog_for_role`,
`open_find_dialog`, `open_replace_dialog`, `request_open_history`,
`request_open_menu_box`. No `request_exec_view`. `Program::exec_view`
(`src/app/program.rs:917`, `pub`) exists but is **top-level-only** — unreachable
from a `&mut Context`. Design context: `docs/design/async-modal-from-a-view.md`,
`docs/design/deferred-effects.md`.

**C++ baseline.** Inside `TDirBox.HandleEvent`, TCV builds a `TDialog` with six
`TStaticText` fields and an OK button and calls `Desktop^.ExecView(Pinfo)` —
spinning a nested modal loop inline, from within the view.

**Proposed approach.** Add a generic deferred variant, e.g.
`Deferred::ExecView(Box<dyn View>)` + `Context::request_exec_view(view)`, that the
pump executes as a modal at deferred-apply (same machinery as the existing
`Open*Dialog` → `pending_modal` → `ModalCompletion::*` flow used by C1/C8).
**Key design question to settle first — this is the keystone for all of axis B:**
how the result returns to the requester. Reuse the established `answer_to` +
`then_command` pattern (deliver the modal's end command to the requesting view),
and/or a `ModalCompletion::ExecView` carrying the end command + the boxed view back
for the caller to read state off via `as_any`. Follow the C1 reuse note in
HANDOVER (don't invent a new seam shape). Whatever shape settles here is the
template for B.2–B.4.

**Verify.** `tcv.rs`'s Info box becomes a real custom `Dialog` (six labelled
fields) launched from the list via `request_exec_view`; headless test opens it,
asserts the fields render, closes it. Snapshot.

### B.2–B.4 Siblings — other arbitrary outward actions missing from the view surface

Once B.1's result-delivery shape is settled, evaluate each (don't necessarily
build all — but decide and record, not leave silently absent):

- **B.2 — `insert` an arbitrary view into the owning group from within a view.**
  C++ `TGroup::insert(TView*)`; Rust `Group::insert`/`insert_with_id`
  (`src/view/group.rs:273/318`, `pub`) require owning `&mut Group` (top-level
  only). A leaf has no `request_insert`. Only `request_close`-by-id exists for the
  inverse.
- **B.3 — payload-carrying `message(receiver, what, cmd, infoPtr)`.** C++
  `message()` targets a receiver with a payload. Rust `Context::post` (command
  only) and `Context::broadcast` (payload-less; `source` is a D4 subject *filter*,
  not a carrier) cover the common cases; the one internal payload case got a
  bespoke broker (`ResolveFocusedFile`). No general targeted-message-with-payload.
- **B.4 — set-valued command ops.** C++ `enableCommands`/`disableCommands`/
  `setCommands`/`setCmdState`/`getCommands` take a `TCommandSet`. Rust offers only
  single-command `enable_command`/`disable_command` (`Context` 1119/1125; `Program`
  638/664) and single-command `command_enabled` read. The classic TV idiom (build
  a command set, toggle in one call) requires a consumer loop today.
- **(Also noted)** `selectNext`/`focusNext` from a view: only `request_focus(id)`
  (no mode, no relative next) — `Group::find_next`/`focus_next` need `&mut Group`.

**The architectural decision** (settle with B.1): keep the allowlist (add variants
as real consumers ask, accepting the per-capability cost) **or** introduce one
genuinely open capability (e.g. a deferred "run this closure against the owning
group at apply-time") that subsumes insert/select/etc. This is a real design
choice, not a mechanical port — make it deliberately and record the rationale.

---

## Doing the work

Each axis is its own small, two-stage-reviewed change (CLAUDE.md methodology:
fresh implementer → spec-compliance reviewer → code-quality reviewer → integrate).
Axis A and C.1 are mechanical and high-value — land them first to make `tcv`
faithful. C.2 and axis B are **decisions before code**: settle the design
(FOUNDATION read-only investigation first), then implement. B.1's result-delivery
design is the keystone for the rest of B.

The tcv header documents each workaround; make it faithful as each gap lands.
