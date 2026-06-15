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
- The tab bar becomes a **reusable `TabBar` widget** (like `ScrollBar`).

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

### Piece 2 — `TabBar` widget (rstv-original, `src/widgets/tab_bar/` or `src/widgets/tab_bar.rs`)

A standalone, focusable, single-row view — same standing as `ScrollBar`.

- **State:** `tabs: Vec<String>` (each may carry `~X~` hotkey markup),
  `active: usize`, plus `ViewState`.
- **Draw:** corner-cap style on the gray field — active tab `┌Label┐`
  (`frame_tl`/`frame_tr`), inactive tabs plain; hotkey letter in the accent
  (shortcut) role; **no background fill** beyond the gray field. A 1-cell gap
  between tabs.
- **Input:** when focused, **←/→** move `active` (wrapping); a tab's **hotkey**
  (the `~X~` letter) selects it directly; **mouse click** on a tab selects it.
- **Change notification:** on `active` change, **broadcast** a tab-changed
  command carrying its `ViewId` as `source` (D4), and expose the index via
  `View::value`/`set_value` (D10) — mirrors how `ScrollBar` notifies its owner.
  The owner (color picker) listens for the broadcast and switches surfaces.
- **Sizing:** width = sum of tab widths + caps + gaps; height = 1. Helper to
  compute its natural width so callers can place it.
- **Snapshot tests** (D11): active vs inactive rendering; ←/→ cycling; hotkey
  select; mouse-click select; the broadcast fires with the right source/index.

This is an **rstv-original** (no TV ancestor) — documented as such in its
rustdoc under a "Turbo Vision heritage" note (there is none; it's an extension),
consistent with the Splitter precedent.

### Piece 3 — Refit the color picker (`src/dialog/colorpick/`)

- **Replace the bespoke tab drawing** (`mod.rs` ~232–253) with an embedded
  `TabBar` child. The picker switches the visible surface when the TabBar
  broadcasts a change. The `Tab` enum / `label()` move into TabBar tab strings.
- **Rename** the tabs: `Hue/Sat` (was "Plane ~W~"), `Xterm` (was "~6~"); keep
  `Presets`, `RGB`.
- **Kill the blue chrome:** the tab row and the **info column** fill
  (`mod.rs` ~177, ~235–240) stop using `ScrollerNormal` / `FramePassive` (blue)
  and use the **gray dialog field** roles, so the picker reads as one coherent
  gray dialog. Actual color swatches/gradients (the surfaces' content) stay
  colorful — only the chrome changes.
- **Blank line above the buttons:** in `examples/tvdemo.rs` the picker dialog is
  re-laid so there's one empty row between the picker body and the OK/Cancel row
  (shrink the picker body by one row; place buttons via the new `button_row`
  helper at `height − 3`).
- **Snapshot tests** updated for the new chrome/tabs.

## Verification

- D11 snapshot tests for `TabBar` (states + interactions) and the refit color
  picker (gray chrome, corner-cap tabs, renamed labels, blank line above
  buttons).
- A regenerated tvdemo frame (the demo already opens the picker) eyeballed to
  confirm the blue is gone and the tabs read as tabs.
- Guide doc cross-checked against the recovered TV metrics.

## Out of scope / follow-ups

- Migrating `msgbox`/`inputbox`/`filedlg`/theme-editor to the constants + helper
  (they already conform behaviorally; a de-duplication pass can come later).
- A full notebook/multi-page container (we only need a single-row tab selector).
- Tab overflow/scrolling when tabs exceed the width (the picker's four short
  tabs fit; note as a future `TabBar` enhancement).

## Decomposition

Built and reviewed in order (each is independently testable):
1. Guide doc + constants + `button_row` helper.
2. `TabBar` widget (+ snapshot tests).
3. Color-picker refit (uses 1 and 2) + tvdemo re-lay + demo regen.
