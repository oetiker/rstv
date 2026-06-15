# Dialog design guide + reusable TabBar + color-picker refit — design

**Date:** 2026-06-15
**Status:** draft (awaiting user review)

## Problem

The color picker looks off (a blue chrome background bleeding through a *gray*
dialog, cramped buttons, ad-hoc tabs). The root cause isn't the picker — it's
that rstv has **no documented layout language for dialogs**, so each dialog
invents its own coordinates and roles. Fixing the picker by hand would just be
more drift.

Research findings (from magiblot Turbo Vision + the rstv codebase):

- **Classic TV has crisp, consistent dialog conventions** (recovered from
  `msgbox.cpp`, `tfildlg.cpp`, `colorsel.cpp`, `tbutton.cpp`, `tdialog.cpp`):
  standard button **10×2** (1-row drop shadow), **2-cell** gaps, button row at
  `height − 3`, content inset **3 left / 2 right / 2 top / 3 bottom**, regions
  separated by **labels + whitespace, never internal separator lines**, gray
  palette + drop shadow.
- **rstv's own `msgbox`/`inputbox` already follow these almost exactly.** The
  **color picker is the outlier** (uses blue-window roles `FramePassive` /
  `ScrollerNormal` for chrome inside a gray dialog; no button gap; bespoke tab
  drawing).
- **No dialog-layout guide exists.** `docs/book/src/apps/dialogs.md` covers
  mechanics (modal exec, value protocol), not layout. `docs/design/` has 6 notes,
  none on layout.
- **Tabs are an rstv-original.** Classic `TColorDialog` shows all controls at
  once (no tabs). rstv invented the tabbed picker. So a reusable tab control is a
  net-new rstv extension, not a port.

## Decisions (locked)

- Tab visual style: **corner-cap on the active tab** — `┌Label┐` using the
  `frame_tl`/`frame_tr` glyphs; inactive tabs are plain text; hotkey letter
  accented; **no blue background**.
- Rename picker tabs: **"Plane W" → "Hue/Sat"**, **"6" → "Xterm"** (the
  xterm-256 palette grid). Keep "Presets" and "RGB". Hotkeys stay unique:
  P / R / H / X.
- Codification depth: **guide doc + named layout constants + a `Dialog`
  button-row helper** (prevent recurrence of the drift).
- Retrofit scope: **color picker only** now. `msgbox`/`inputbox` already conform;
  migrating `filedlg`/theme-editor is a later, optional pass.
- The tab selector becomes a **reusable `TabBar` widget**, modelled on
  **`TCluster`/`TRadioButtons`** (single selection, `~X~` hotkeys, arrow nav,
  click-to-select, transfer `value`/`sel`, press-on-release) — **not** on
  `ScrollBar`. TV precedent: `TMonoSelector : public TCluster` is a custom
  single-selector built *inside* the C++ color dialog by subclassing the cluster.
- **The switchable views live in a reusable `PageStack`** — a content
  multiplexer (rstv-original; classic TV has no notebook/tab-page class, only
  `TGroup` + `sfVisible`/`show()`/`hide()`, confirmed by grep). It holds N child
  page Views and keeps exactly one `sfVisible`.
- **`TabBar` and `PageStack` are siblings coupled by the D3 pump broker**,
  mirroring `Scroller`↔`ScrollBar` exactly: `TabBar` broadcasts
  `Command::TAB_BAR_CHANGED` carrying its own `ViewId` as `source`; `PageStack`
  (which stores the bound `tab_bar` id) filters on `source`, queues a new
  `Deferred::PageStackSync { page_stack, tab_bar }`; the pump reads
  `tab_bar.value()` and calls `PageStack::set_active(idx, &mut ctx)`. This is why
  the selector broadcasts at all — it now has a real sibling consumer (the earlier
  "broadcast into the void" smell is gone).
- **The color picker is rebuilt as a `Group` container** (not the current
  monolithic single `View`): a `TabBar` child + a `PageStack` child (the four
  surfaces become real page Views sharing one `Rc<RefCell<ColorModel>>`) + an
  always-visible info-column child. Draggable surfaces are rebuilt on the
  **standard mouse-track capture** (`ctx.start_mouse_track`, the `ScrollBar`
  thumb-drag pattern), retiring the bespoke `drag.rs` broker
  (`ColorDragCapture` + `Deferred::ColorPickerDrag`).

## Design

Three pieces, built in order (the guide is the foundation).

### Piece 1 — Dialog design guide (`docs/design/dialog-layout.md`) + constants

A concise, faithful-to-TV reference. Sections:

1. **Dialog construction** — `Dialog::new` (gray palette, `move | close`, drop
   shadow); when to center / when `grow`.
2. **Interior margins** — content inset from the frame: **3 left, 2 right, 2
   top, 3 bottom** (the `msgbox` figures, confirmed against `tfildlg`). The
   bottom-3 reserves the button row.
3. **The button row** — standard button **10×2**; **2-cell** gap; the row's top
   at **`height − 3`** (one blank row between content and buttons, buttons end
   one row above the frame). Centered for symmetric sets (message boxes),
   right-grouped for action dialogs (OK/Cancel).
4. **Labels & inputs** — label one row **above** its input (file-dialog style)
   or one cell **left** on the same row (compact input box); 1-cell/1-row gap.
   Labels link to their control (`LabelNormal`/`LabelLight`).
5. **Separating regions** — **whitespace + a `StaticText`/label header**, *not*
   drawn lines. (Matches TV; keeps the gray field clean.)
6. **Roles for gray-dialog chrome** — frame: `FrameGray*`; fills/background:
   the gray dialog field (NOT blue `FramePassive`/`ScrollerNormal`); text:
   `StaticText`; buttons: `Button*`; inputs: `Input*`. An explicit "do not use
   blue-window roles inside a gray dialog" rule (the exact bug we're fixing).
7. **Shadows** — buttons self-shadow; windows/dialogs cast a desktop shadow. No
   manual shadow calls.
8. **rstv-original extensions** — the `TabBar` (Piece 2): when to use tabs, the
   corner-cap style, that the tab row sits at the top of a content area on the
   gray field.

Named constants (in `src/dialog/`, e.g. `dialog.rs` or a new `layout.rs`):

```rust
pub const STD_BUTTON: Point = Point { x: 10, y: 2 }; // standard button cells
pub const BUTTON_GAP: i32 = 2;        // cells between adjacent buttons
pub const MARGIN_LEFT: i32 = 3;
pub const MARGIN_RIGHT: i32 = 2;
pub const MARGIN_TOP: i32 = 2;
pub const BUTTON_ROW_FROM_BOTTOM: i32 = 3; // button-row top = height - 3
```

A `Dialog` button-row helper:

```rust
/// Insert a conventional button row: standard 10×2 buttons, BUTTON_GAP apart,
/// top edge at `height - BUTTON_ROW_FROM_BOTTOM`. `align` centers (message-box
/// style) or right-groups (action-dialog style). Returns the inserted ids.
pub fn button_row(
    &mut self,
    buttons: &[(&str, Command, ButtonFlags)],
    align: ButtonRowAlign, // Center | Right
) -> Vec<ViewId>;
```

(The helper encodes items 3 above so callers can't drift. `msgbox`/`inputbox`
may adopt it later; out of scope now.)

### Piece 2 — `TabBar` selector + `PageStack` content multiplexer + the broker

Two composable rstv-original widgets plus the sibling-broker seam that couples
them. Both documented as rstv extensions (no TV ancestor for the *idiom*; the
*mechanisms* — cluster selection, `TGroup` + `sfVisible` — are TV-faithful).

**`TabBar`** (`src/widgets/tab_bar.rs`) — a focusable single-row selector,
**cluster-shaped** (the `TMonoSelector : public TCluster` precedent):

- **State/semantics modelled on `Cluster`:** `value` (selected index) + `sel`
  (cursor); `tabs: Vec<String>` with `~X~` hotkeys; `press(item)`/`moved_to(item)`
  selection verbs; `find_sel(p)` hit-test; **press-on-release** mouse (mirrors
  `cluster.rs`); `~X~` hotkey-when-focused + arrow nav. (Standalone, cluster-
  *shaped* rather than a `ClusterKind` branch, because the shared `Cluster` engine
  is marker-/column-centric and must-not-broadcast for radio/check — a Tabs branch
  would pollute it. The vocabulary and contract still match `TCluster`.)
- **Draw:** corner-cap on the active tab — `┌Label┐` (`glyphs.frame_tl/tr`),
  inactive tabs plain; hotkey letter in the shortcut role; **no fill beyond the
  gray field**; 1-cell gaps. (rstv-original visual, like `TMonoSelector` overrode
  `draw`.)
- **Transfer protocol (D10):** `value()` → `FieldValue::Int(selected)`;
  `set_value(FieldValue::Int)` clamps (the `getData`/`setData` successors).
- **Change notification (D3/D4):** on a selection change, broadcast
  `Command::TAB_BAR_CHANGED` carrying its own `ViewId` as `source` — exactly
  `ScrollBar::scroll_draw`'s shape.
- **`as_any_mut` → `Some(self)`**; `natural_width()` helper for placement.

**`PageStack`** (`src/widgets/page_stack.rs`) — a content multiplexer, a thin
`#[delegate(to = group)]` wrapper (`{ group: Group, pages: Vec<ViewId>, active,
tab_bar: Option<ViewId> }`):

- `insert_page(view) -> ViewId` inserts a child page; all but the active page get
  `state.visible = false`.
- `set_active(idx, ctx)` flips visibility via `group.set_visible_descendant`
  (which auto-`reset_current`s on a selectable visibility change) then
  `group.focus_child(active_page, ctx)`.
- **Sibling reaction:** `handle_event` filters `Event::Broadcast { command:
  TAB_BAR_CHANGED, source }` on `source == self.tab_bar` and queues
  `ctx.request_sync_page_stack(self_id, tab_bar)` — the exact shape of
  `Scroller::handle_event` filtering `SCROLL_BAR_CHANGED`.
- `as_any_mut → Some(self)` (NOT delegated — the pump downcast hatch).

**The broker** (the only new substrate, modelled 1:1 on `SyncScrollerDelta`):

- New `Deferred::PageStackSync { page_stack: ViewId, tab_bar: ViewId }` in the
  `Deferred` enum.
- New `Context::request_sync_page_stack(page_stack, tab_bar)` queues it.
- New pump arm (beside the `SyncScrollerDelta` arm in `program.rs`): read
  `group.find_mut(tab_bar).value()` → `Int` index, then `group.find_mut(page_stack)`
  → `as_any_mut` → `downcast_mut::<PageStack>()` → `set_active(idx, &mut ctx)`.
- New `Command::TAB_BAR_CHANGED`. **No `View`-trait method is added, so no
  `rstv-macros/src/specs.rs` forwarder is needed.**

- **Snapshot/unit tests** (D11): `TabBar` active vs inactive draw, arrow/hotkey/
  click selection, press-on-release, broadcast source; `PageStack` show-one/hide-
  rest, `set_active` moves currency; an integration test that a `TabBar`+`PageStack`
  pair in a `Group` switches the visible page through the pump broker.

### Piece 3 — Rebuild the color picker on `TabBar` + `PageStack`

`ColorPicker` stops being a monolithic single `View` and becomes a **`Group`
container** (`{ group, tab_bar_id, page_stack_id, info_col_id, model:
Rc<RefCell<ColorModel>>, old }`):

- **Children:** a `TabBar` on row 0 (left of the info column); a `PageStack`
  below it whose four pages are the surfaces rebuilt as real Views
  (`PresetsPage`/`RgbPage`/`PlanePage`/`XtermPage`), each holding a clone of the
  shared `Rc<RefCell<ColorModel>>`; an always-visible **info-column** child on
  the right. The `Tab` enum maps to page indices.
- **Shared model:** `ColorModel` (Clone+Copy today) moves behind
  `Rc<RefCell<ColorModel>>`; each page borrows it in `draw`/`handle_event`. This
  removes the "inline match to split borrows" anti-pattern in the current
  `ColorPicker` (it only existed because surfaces took `&mut model` params).
- **Drag rebuilt on the standard track capture:** `RgbPage`/`PlanePage` handle
  their own `MouseDown`-over-a-drag-region by caching `abs_origin` in `draw` and
  calling `ctx.start_mouse_track` (the `ScrollBar` thumb-drag pattern); their
  `MouseMove`/`MouseAuto` arms scrub the shared model. The bespoke `drag.rs`
  (`ColorDragCapture`, `Deferred::ColorPickerDrag`, `Context::request_color_drag`,
  its pump arm) is **retired**.
- **Tab switching:** the `TabBar` broadcasts; the `PageStack` (its `tab_bar` bound
  to the bar's id) switches the visible page via the broker. The picker keeps
  Ctrl+←/→ and Alt+hotkey by driving the `TabBar`'s `set_value`/`press`.
- **Rename** tabs: `Hue/Sat` (was "Plane ~W~"), `Xterm` (was "~6~"); keep
  `Presets`, `RGB`. Hotkeys P/R/H/X.
- **Kill the blue chrome:** the tab row and the info-column fill stop using
  `ScrollerNormal`/`FramePassive` (blue) and use the **gray dialog field** roles.
  Color swatches/gradients stay colorful — only chrome changes.
- **Public API preserved:** `ColorPicker::new(bounds, initial)`, `color()`,
  `select_tab(tab)`, `as_any_mut` — so `Program::color_dialog`,
  `open_color_dialog_for_role`, and the `examples/` embeds keep working.
- **Blank line above the buttons:** `examples/tvdemo.rs` re-lays the picker dialog
  via the new `button_row` helper, leaving one empty row above OK/Cancel.
- **Snapshot tests** updated for the new chrome/tabs; the per-surface standalone
  snapshots migrate to the per-page Views.

## Verification

- D11 snapshot/unit tests for `TabBar` (states + interactions + broadcast),
  `PageStack` (show-one/hide-rest + `set_active` currency), a `TabBar`+`PageStack`
  **pump-broker integration test** (switching the visible page through
  `Deferred::PageStackSync`), and the rebuilt color picker (gray chrome,
  corner-cap tabs, renamed labels, blank line above buttons; per-page surface
  snapshots; drag still scrubs via the standard track capture).
- A regenerated tvdemo frame (the demo already opens the picker) eyeballed to
  confirm the blue is gone and the tabs read as tabs.
- Guide doc cross-checked against the recovered TV metrics.

## Out of scope / follow-ups

- Migrating `msgbox`/`inputbox`/`filedlg`/theme-editor to the constants + helper
  (they already conform behaviorally; a de-duplication pass can come later).
- Tab overflow/scrolling when tabs exceed the width (the picker's four short
  tabs fit; note as a future `TabBar` enhancement).
- Making `TabBar` a true `ClusterKind::Tabs` branch of the shared `Cluster`
  engine (we go standalone-cluster-shaped now to avoid polluting the marker-
  centric engine; revisit if a second tabbed consumer appears).
- **(Now in scope — was previously deferred):** the `PageStack` multi-page
  container. The earlier draft scoped this out; the full rebuild adopts it as the
  faithful answer to "where do the switchable views live."

## Decomposition

Built and reviewed in order (each is independently testable):
1. **Piece 1** — guide doc + layout constants + `Dialog::button_row` helper.
2. **Piece 2a** — `TabBar` (cluster-shaped selector) + tests.
3. **Piece 2b** — `PageStack` (content multiplexer) + the broker seam
   (`Command::TAB_BAR_CHANGED`, `Deferred::PageStackSync`,
   `Context::request_sync_page_stack`, the pump arm) + an integration test.
4. **Piece 3** — rebuild `ColorPicker` as a `TabBar`+`PageStack`+info-column
   group; surfaces → page Views over `Rc<RefCell<ColorModel>>`; retire `drag.rs`
   for the standard track capture; gray chrome; rename tabs; tvdemo re-lay +
   demo regen.
