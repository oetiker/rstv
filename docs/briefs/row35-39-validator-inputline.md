# Brief — validator wave: TValidator (35) + TInputLine (39) + D10 value protocol

You are an **implementer subagent** on **rstv**, a *faithful* Rust port of
magiblot's C++ Turbo Vision. Port behavior **verbatim** from the C++ except where
a D-rule says deviate. This brief is self-contained; do not "go read the plan."

## Source of truth (read these C++ files)
- `/home/oetiker/scratch/tvision-spec/magiblot-tvision/source/tvision/tvalidat.cpp`
  (only the **abstract `TValidator`** part, lines ~38–95 — IGNORE the concrete
  PXPicture/Filter/Range/Lookup validators, those are later rows)
- `/home/oetiker/scratch/tvision-spec/magiblot-tvision/include/tvision/validate.h`
  (the `TValidator` class block + `vsOk/vsSyntax`, `voFill/voTransfer`)
- `/home/oetiker/scratch/tvision-spec/magiblot-tvision/source/tvision/tinputli.cpp`
  (full TInputLine)
- `/home/oetiker/scratch/tvision-spec/magiblot-tvision/include/tvision/dialogs.h`
  lines 145–235 (TInputLine decl + `ilMaxBytes/ilMaxWidth/ilMaxChars`)

## Deliverable = 3 pieces, in order

### 1. `src/validate.rs` — the abstract `Validator` trait (row 35, D2)
Inheritance → trait (D2). `TValidator` is abstract; concrete validators are later
rows. Object-safe (it will be boxed as `Option<Box<dyn Validator>>`): every method
takes `&self`, no generics, no `Self` return.

```rust
pub trait Validator {
    /// `isValidInput` — may auto-fill/modify `s` in place. Default: accept.
    fn is_valid_input(&self, _s: &mut String, _suppress_fill: bool) -> bool { true }
    /// `isValid` — final-form check. Default: accept.
    fn is_valid(&self, _s: &str) -> bool { true }
    /// `error` — concrete validators show a message box.
    /// TODO(msgbox row 63): wire to a real message box; no-op until then.
    fn error(&self) {}
    /// `validate` — non-virtual in C++: report error and fail iff invalid.
    fn validate(&self, s: &str) -> bool {
        if self.is_valid(s) { true } else { self.error(); false }
    }
    /// `status == vsOk` — TInputLine::valid(cmValid) consults this.
    /// Default Ok; TPXPictureValidator (row 62) sets a syntax-error status.
    fn is_status_ok(&self) -> bool { true }
}
```
**Deliberate deviation from PORT-ORDER row 35's note** (which lists `transfer`):
the `transfer(void*, TVTransfer)` D10 hook has **no overrider until
TRangeValidator (row 59)** and no caller until then, so building it now would be a
dead stub. **Do NOT add `transfer`.** Breadcrumb the slot-in point inside
`InputLine::value`/`set_value` (see §3). Faithful + lean (no dead stubs).

Re-export `pub use validate::Validator;` in `src/lib.rs` (house style `tv::Validator`).

### 2. `src/data.rs` — `FieldValue` (D10 typed-value currency)
D10: untyped `getData`/`setData` `memcpy` → a **typed value protocol**. Define the
currency; it **grows per widget** (the Role/Glyphs convention):
```rust
/// The typed unit of dialog data transfer (D10). Grows per control:
/// `Bits(u32)` for `TCluster` and `Int` for `TRangeValidator` land when those
/// controls wire their `value`/`set_value` (deferred — no consumer yet).
#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    Text(String),
}
```
Re-export `pub use data::FieldValue;` in `lib.rs`.

Add to the **`View` trait** (`src/view/view.rs`) two **defaulted** methods (D10):
```rust
/// `getData` — a data-bearing control's typed value; `None` = not a data field.
fn value(&self) -> Option<FieldValue> { None }
/// `setData` — load a typed value into a data control. Default: ignore.
fn set_value(&mut self, _v: FieldValue) {}
```
(Named `value`/`set_value` per guide D10 / PORT-ORDER. The `Cluster.value: u32`
field and `Indicator::set_value` inherent method do NOT clash — different
receivers/signatures; trait dispatch is via the vtable.)
**Defer the dialog gather/scatter group-walk** (`TGroup::getData`/`setData` over
children) — no data dialog consumes it yet. Breadcrumb it in `data.rs`’s module
doc ("the ordered child-walk lands with its first consumer — inputBox/Batch E").

### 3. `src/widgets/input_line.rs` — `TInputLine` (row 39)
Faithful port of `tinputli.cpp`. `InputLine { state: ViewState, data: String,
max_len/max_width/max_chars: i32, cur_pos/first_pos/sel_start/sel_end/anchor: i32,
validator: Option<Box<dyn Validator>>, old_*: <save-state fields> }`.

Coordinates `i32` (faithful to C++ `int`). Add `pub mod input_line;` +
`pub use input_line::InputLine;` to `src/widgets/mod.rs` (do this edit yourself —
serial work, no worktree).

**Ctor** `InputLine::new(bounds, limit, validator: Option<Box<dyn Validator>>,
limit_mode)` — port the `maxLen/maxWidth/maxChars` selection (ilMaxBytes=0 →
maxLen=min(max(limit-1,0),...), else 255; ilMaxWidth=1; ilMaxChars=2). Set
`state.cursor_vis`, `options.selectable | options.first_click`. Provide a
convenience ctor without a validator if natural.

**`draw`** (port `TInputLine::draw`, D7 + D13):
- fill width with the normal color; `moveStr` the data from `first_pos` (use the
  `crate::text` grapheme helpers + the existing `DrawCtx` writer — look at how
  `static_text.rs`/`button.rs` draw).
- right arrow at `size.x-1` if `can_scroll(1)`, left arrow at `0` if
  `can_scroll(-1)`, in the arrow color.
- selection block (`sf_selected`): highlight cols `displayedPos(selStart..selEnd) -
  firstPos`, clamped, in the selected color.
- cursor at `displayedPos(curPos) - firstPos + 1`.
- **Theme (D7):** `cpInputLine "\x13\x13\x14\x15"` → palette idx 1=2=passive/active
  (both dialog 0x13), 3=selected (0x14), 4=arrows (0x15). Add **3 roles**
  `InputNormal`, `InputSelected`, `InputArrow` to `src/theme.rs` (bump
  `ROLE_COUNT`, extend the `index()` match AND the style array) with **provisional
  gray-dialog colors** mirroring how `ClusterNormal`/`StaticText` were seeded
  (`TODO(row 34 gray theming)`). `draw()` uses `getColor((sfFocused)?2:1)` for both
  states — both map to `InputNormal`.
- **Glyphs (D7, row-9 convention):** add `input_left_arrow`/`input_right_arrow` to
  `Glyphs` — C++ `leftArrow='\x11'` (◄ U+25C4), `rightArrow='\x10'` (► U+25BA).

**`handle_event`** (port the `sfSelected` block of `TInputLine::handleEvent`):
- call the base first (mouse-down auto-select is the group's job now — the base
  `View::handle_event` is a no-op; do NOT re-implement focus here).
- **Keyboard** (the meat): `ctrl_to_arrow` (shared helper, already in
  `crate::event` — use it), Home/End/Left/Right/Ctrl-Left/Ctrl-Right (word nav via
  `prev_word`/`next_word` — port the two static fns), Backspace/Ctrl-Back/Del/
  Ctrl-Del (with `delete_select`/`delete_current`), Ins toggles `sfCursorIns`,
  Shift+pad-key block extension (`anchor`/`adjustSelectBlock`/`extendBlock`),
  printable-text insertion with the `maxLen`/`maxWidth`/`maxChars` guard (use
  `text::measure`), Ctrl-Y clears. Run `save_state`/`check_valid` exactly where
  C++ does. **`firstPos` scroll-follow math** at the end — port verbatim.
- **Single-shot mouse** positioning only (`mousePos`/`mouseDelta` → set cur_pos /
  selection). **DEFER the C++ press-and-hold auto-scroll + drag-select
  `do…while(mouseEvent(...))` loops** → `TODO(row 31, D9)` (same pattern as
  scrollbar/cluster — those nested mouseEvent loops become capture handlers later).
- **DEFER the `evCommand` clipboard block** (cmCut/cmCopy/cmPaste) entirely —
  there is no `Context`-level clipboard accessor yet. `TODO(clipboard): cmCut/
  cmCopy/cmPaste need a Context clipboard seam (backend has set/get_clipboard;
  not surfaced to views).` Do not implement.
- **DEFER `updateCommands`/`canUpdateCommands`** (enable/disable cmCut/Copy/Paste)
  — needs the `Context` command-set query that TButton also deferred. `TODO(button/
  inputline: command-set query for command graying)`. Skip the whole
  enable/disable-commands path.

**`value`/`set_value` (D10):**
- `value()` → `Some(FieldValue::Text(self.data.clone()))`.
  Breadcrumb: `// TODO(row 59): a validator transfer hook would produce a typed
  non-Text value here (TRangeValidator → Int); abstract validator has none.`
- `set_value(FieldValue::Text(s))` → set `data`, then `select_all(true, true)`.
  Same transfer breadcrumb. (Other `FieldValue` variants: ignore for now.)

**`valid(cmd)`** (port `TInputLine::valid`) — `fn valid(&self, cmd: Command) ->
bool` is `&self`:
- if `validator`: `cmd == cmValid` → `validator.is_status_ok()`; else if `cmd !=
  cmCancel` → `if !validator.validate(data) { /* see below */ return false }`.
- else true.
- **`select()` side-effect DEFERRED:** C++ calls `select()` (focus the invalid
  field) before returning false. That needs `&mut Context` + your focus-by-ViewId
  seam (`Deferred::FocusById` / `request_focus`), unavailable in `valid(&self)`.
  Implement the **return value faithfully** (this blocks a modal OK — it is
  reachable via `Group::valid` and is a real bite-able test) and breadcrumb:
  `// TODO(valid-select): C++ valid() calls select() on the bad field before
  returning false; needs &mut Context + request_focus. Return value is faithful;
  the focus side-effect is deferred.`

**`set_state`** (port `TInputLine::setState`): call base `View::set_state`, then on
`sfSelected` (or `sfActive` while selected) → `select_all(enable, false)`. Skip the
`updateCommands` half (deferred above).

**`select_all`, `delete_select`, `delete_current`, `adjust_select_block`,
`save_state`, `restore_state`, `check_valid`, `can_scroll`, `displayed_pos`,
`mouse_pos`, `mouse_delta`, `prev_word`, `next_word`** — port verbatim.

## D-rules in play
- **D2** inheritance → trait + composition (Validator trait; InputLine embeds
  `ViewState`).
- **D10** typed `value`/`set_value` over `FieldValue` (this row builds it).
- **D13** text via grapheme helpers — **all of `cur_pos`/`first_pos`/`sel_*` are
  BYTE offsets into `data: String`**. Slicing a `String` at a non-char-boundary
  PANICS. Step with `crate::text::next` (returns `Some((byte_len, width))`) and a
  **`prev`** you add to `text.rs` (port `TText::prev` — byte length of the grapheme
  ending at a given index; use `unicode_segmentation` grapheme boundaries). Mirror
  the existing `text::next`/`scroll` style + doc-comment density.
- **D7** Theme roles + Glyphs (above).
- **D5** states are struct-of-bools on `ViewState` (`cursor_vis`, `selectable`,
  `first_click`, `cursor_ins` — check `State`/`Options` in `view.rs` for exact
  field names; add `cursor_ins` to `State` if missing, faithfully).

## Existing types you build on (read them)
- `src/view/view.rs` — `View` trait, `ViewState`, `State`/`Options`/`StateFlag`,
  `set_state`, `cursor_request`. Add `value`/`set_value` here.
- `src/view/context.rs` — `DrawCtx` (`put_char`/`put_str`/`put_cstr`/`fill`/`sub`),
  `Context`.
- `src/text.rs` — `next`/`scroll`/`measure`/`width` (add `prev`).
- `src/theme.rs` — `Role`, `Glyphs`, `Theme`.
- `src/event/` — `Event`, `Key`, `KeyEvent`, `MouseEvent`, and the shared key
  helpers `ctrl_to_arrow`/`is_alt_hotkey`/`is_plain_hotkey` (re-exported).
- `src/command.rs` — `Command` (need `cmValid`, `cmCancel`; add `cmValid` if
  absent, namespaced like the rest).
- Look at `src/widgets/static_text.rs` + `src/widgets/button.rs` for the house
  draw/handle_event/test idioms.

## Verify (required before you report)
- `CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target cargo test` — all green.
- `CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target cargo clippy --all-targets
  -- -D warnings` — clean.
- `CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target cargo fmt --check` — clean.
- **Snapshot test** (Appendix B step 4): build an `InputLine` on a
  `HeadlessBackend`, render, `assert_snapshot!` against the frozen format. Generate
  with `INSTA_UPDATE=always cargo test <name>`, eyeball it, re-run plain, commit the
  `.snap`. (cargo-insta is NOT installed.) Look at an existing widget test
  (`tests/` or in-module `#[cfg(test)]`) for the harness.
- **Unit tests** — and make them DISCRIMINATING:
  - **Multi-byte/grapheme tests are MANDATORY** (the cluster review caught panics
    of exactly this class): insert, Backspace, Del, and Ctrl-Left/Right word-nav
    across a **2+-byte grapheme** (e.g. `ä`, `€`, an emoji) — assert no panic and
    correct byte positions.
  - `valid()` return: with a stub `Validator` whose `is_valid` returns false,
    assert `valid(cmOk)` (or any non-cancel cmd) is `false`, and `valid(cmCancel)`
    is `true`, and no-validator → `true`.
  - `value`/`set_value` round-trip (`set_value(Text("x"))` then `value()` ==
    `Some(Text("x"))`, and selection state after set).
  - cursor/firstPos scroll-follow on a string wider than the field.

## Report back
A short summary: files added/changed, the exact deferrals you breadcrumbed (with
their `TODO(...)` text), any place the C++ was ambiguous and how you resolved it,
and the final test/clippy/fmt status. Do **not** commit — the orchestrator
integrates and commits after two-stage review.
