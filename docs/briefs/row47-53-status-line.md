# Brief — Rows 47 + 53: `TStatusItem`/`TStatusDef` (data) + `TStatusLine` (view)

**Tags:** 47 = MECHANICAL (pure data); 53 = FOUNDATION (the view).
**C++ sources:** `include/tvision/menus.h:400-527` (the data types + class decl),
`source/tvision/tstatusl.cpp` (the impl), `examples/tvdemo/tvdemo1.cpp:172-191`
(multi-range example).
**Target module:** new `src/status/` (`mod.rs` + `status_line.rs`), wired into
`lib.rs`. Mirror the shape of `src/menu/` (read `src/menu/mod.rs`,
`src/menu/menu_bar.rs`, `src/menu/menu_view.rs` first — they are the direct
template for *every* mechanism below).

This is the **draw/data slice** of the status line — exactly analogous to the
menu bar/box draw layer (rows 50/51). Build a standalone `StatusLine` `View`
that is snapshot-testable on the `HeadlessBackend`. The `TProgram` integration
(getEvent pre-routing, `idle()→update()`) is a **separate later step** — see
*Deferrals*.

---

## Row 47 — the data types (`src/status/mod.rs`)

Port `TStatusItem` and `TStatusDef` as pure data. No `View`.

### `StatusItem` (ports `TStatusItem`, `menus.h:403`)
```rust
pub struct StatusItem {
    pub text: Option<String>,   // C++ char* text; None == C++ text==0 (see below)
    pub key_code: Option<KeyEvent>, // C++ TKey keyCode; None == kbNoKey
    pub command: Command,       // C++ ushort command
}
```
- **`text: Option<String>` is load-bearing.** A `text == 0` status item
  (`None`) is a **hidden global hotkey binding**: it displays **nothing** AND
  **consumes no horizontal space** (in C++ `drawSelect`/`itemMouseIsIn`, the
  `i += l+2` advance is *inside* `if(text != 0)`), but the keyDown loop in
  `handleEvent` **still matches it** to fire its command. Real apps use these
  for e.g. `TStatusItem(0, kbShiftDel, cmCut)` (tvdemo). So: draw and
  mouse-hit-test **skip** `text==None` items entirely; the (deferred) key
  handler does not.
- `key_code: Option<KeyEvent>` — `None` is C++ `kbNoKey`. Mirror
  `MenuItem`'s `key_code`.

### `StatusDef` (ports `TStatusDef`, `menus.h:441`) — **the one real deviation**
C++ `TStatusDef(ushort min, ushort max, items)` selects its `items` when the
current help context falls in the **numeric range `[min, max]`**. Our `HelpCtx`
(D1) is a namespaced `&'static str` with **no ordering** — numeric ranges do not
map. The faithful idiomatic port is a 2-variant matcher (NOT a `Box<dyn Fn>` —
keep `Clone + PartialEq + Eq`, like `Menu`):

```rust
pub enum HelpCtxRange {
    /// C++ `TStatusDef(0, 0xFFFF, ...)` — the universal def every real app uses
    /// (tprogram default, tvedit, tvforms, tvdir). Matches any help context.
    All,
    /// The rare context-split case (tvdemo `[0,50]`/`[50,0xffff]`): an explicit
    /// set of help contexts this def applies to. D1 dropped contiguous integer
    /// blocks, so the range becomes an explicit membership set.
    OneOf(Vec<HelpCtx>),
}

pub struct StatusDef {
    pub range: HelpCtxRange,
    pub items: Vec<StatusItem>,
}
```
Document `HelpCtxRange` as a **one-paragraph corollary of D1** in the module
doc-comment (ranges were contiguous integer blocks used only to index a help
topic table; string identity drops contiguity → explicit set). Multi-def
selection is **faithful-but-unexercised in this row** (nothing here sets a
non-default help context to *select* a non-`All` def) — support it in the data
model (free) and unit-test `find_items` by setting `help_ctx` directly; do not
dress it up as wired.

### Builder
Provide a small fluent builder mirroring `MenuBuilder` (the C++ `operator+`
chains over `TStatusDef`/`TStatusItem`, `menu.cpp:70-94`). Suggested:
`StatusLine::builder()` (or a free `StatusDefBuilder`) producing
`Vec<StatusDef>`, e.g.
```rust
let defs = StatusDef::list()
    .def_all(|d| d
        .item("~F1~ Help", Key::F(1).into(), Command::HELP)
        .item("~Alt-X~ Exit", alt('x'), Command::QUIT)
        .key_item(Key::ShiftDel? , Command::CUT)   // text==None hidden binding
    )
    .build();
```
Match whatever ergonomics read cleanly against the existing `MenuBuilder`; the
escape hatch (`StatusItem` literal) plus the `def_all`/`def_one_of` split is the
requirement. Use the existing `alt(char)` helper from `crate::menu` if useful,
or a local key convenience — do **not** modify `key.rs`.

---

## Row 53 — `TStatusLine` the view (`src/status/status_line.rs`)

A one-row `View` at the bottom of the screen. Structure it like `MenuBar` (a
plain struct embedding `ViewState`, hand-written `View` methods — **not** a D2
`#[delegate]` embed, since it embeds `ViewState`, not a `View`).

### State
```rust
pub struct StatusLine {
    state: ViewState,
    defs: Vec<StatusDef>,          // C++ TStatusDef* defs (owned)
    items_def: usize,              // index into defs of the currently-selected def
                                   //   (C++ `items` = defs[items_def].items); see find_items
    help_ctx: HelpCtx,             // C++ helpCtx (the view's current help context)
    hint: Box<dyn Fn(HelpCtx) -> Option<String>>, // C++ virtual hint(); default -> None
    cmd_set: Option<CommandSet>,   // cached enabled-set for graying (see below)
}
```
**Do NOT add a `disabled`/enabled field to `StatusItem`.** Unlike `TMenuItem`
(which has a real C++ `disabled` field the menu broker mutates), `TStatusItem`
has none — C++ `drawSelect` calls `commandEnabled(T->command)` **live**. Cache a
single `CommandSet` snapshot on the **view** (`cmd_set`), refreshed by the broker
hook below, and have `draw` test `cmd_set.has(item.command)`. `None` (before the
first refresh) means **treat all as enabled** (same startup gap menus have).

Selecting the active def: store `items_def` (resolved by `find_items`) rather
than cloning a `Vec`. `find_items` (C++ `findItems`, `tstatusl.cpp:118`): walk
`defs` and pick the **first** whose `range` matches `help_ctx`
(`All` always matches; `OneOf(set)` matches iff `set.contains(help_ctx)`); if
none match, leave `items` empty (C++ `items = 0`). Represent "no match" however
is cleanest (e.g. `items_def: Option<usize>` or an empty borrow).

### Constructor — ports `TStatusLine::TStatusLine` (`tstatusl.cpp:30`)
`StatusLine::new(bounds, defs)`:
- `growMode = gfGrowLoY | gfGrowHiX | gfGrowHiY` → set the matching
  `state.grow_mode.{lo_y, hi_x, hi_y}` flags.
- `options |= ofPreProcess` → `state.options.pre_process = true`.
- (`eventMask |= evBroadcast` is **moot** — our `Group::handle_event` fans
  broadcasts to every child unconditionally; see the identical note in
  `menu_view::handle_event`. Do not port a mask.)
- `help_ctx = HelpCtx::NO_CONTEXT` initially; call `find_items()`.
- `hint` defaults to `Box::new(|_| None)`. Expose a `with_hint(f)` /
  `set_hint(f)` setter so an app can override (the idiomatic port of the C++
  `virtual hint`). C++ default `hint()` returns `""` → our `None`/empty.

### `draw` — ports `TStatusLine::draw`→`drawSelect(0)` (`tstatusl.cpp:60-114`)
Mirror `MenuBar::draw` mechanics (DrawCtx `fill`/`put_char`/`put_cstr`/`put_str`,
the local `cstrlen` copy ignoring `~`). The algorithm (with `selected = None`
for the plain `draw`; `drawSelect(Some(item))` is the mouse-hover variant — see
deferral note, you only need `draw` = `drawSelect(None)` for this row):

1. Resolve the four color pairs (see *Colors*). Fill the whole row width with
   `cNormal` (`b.moveChar(0, ' ', cNormal, size.x)`).
2. `let mut i = 0;` walk `items` (the selected def's items). For each item with
   `text == Some(t)`:
   - `let l = cstrlen(t);`
   - if `i + l < size.x`:
     - pick color: `enabled = cmd_set.as_ref().map_or(true, |cs| cs.has(cmd))`;
       `selected_this = selected == Some(idx)`. The 2×2 matrix is **identical to
       the menu's** (`MenuColors::item`): `(enabled, selected_this)` →
       select/normal/sel_disabled/norm_disabled. (C++:
       `commandEnabled ? (sel?cSelect:cNormal) : (sel?cSelDisabled:cNormDisabled)`.)
     - `put_char(i, 0, ' ', lo)`; `put_cstr(i+1, 0, t, lo, hi)`;
       `put_char(i+l+1, 0, ' ', lo)`.
   - `i += l + 2;` **(inside the `text != None` arm — `None` items add nothing.)**
3. Hint tail: `if i < size.x - 2`, ask `(self.hint)(self.help_ctx)`; if it
   returns `Some(text)` non-empty: `put_str(i, 0, hintSeparator, cNormal.lo)`
   where `hintSeparator` is the two-char string `"\xB3 "` (C++
   `TStatusLine::hintSeparator`, a vertical bar `│` + space — verify the exact
   bytes in the C++; it is `"│ "`), then `i += 2`, then
   `put_str(i, 0, &text, cNormal.lo)` clipped to `size.x - i`. (Hint text is
   plain — `moveStr`, not a `~`-cstr.)

> **`hintSeparator` exact value:** confirm from the C++ — it is defined as a
> `_NEAR` static. Use the real glyph; if a `Glyphs` entry fits, prefer it,
> otherwise inline the `'│'` (U+2502) + space, matching how `menu_box` decoded
> its frame glyphs.

### `handle_event` — ports `TStatusLine::handleEvent` (`tstatusl.cpp:160`)
Implement the **mouse arm** now; **defer the keyDown arm** (see *Deferrals* for
why — its "transform-in-place, don't clear, propagate" semantics only become
meaningful with the `TProgram` pre-process routing). Also port the broadcast arm.

- **`evBroadcast` `cmCommandSetChanged`** → request the regray broker by the
  view's own id: `ctx.request_update_menu(self.state.id().unwrap())` (the
  **exact** menu pattern — reuse `Deferred::UpdateMenu` + the
  `View::update_menu_commands` hook; see below). C++ does `drawView()` here;
  dropped under D8 whole-tree redraw.
- **`evMouseDown`** → C++ has a press-and-hold `do { drawSelect(itemMouseIsIn) }
  while(mouseEvent(evMouseMove))` drag-highlight loop, then on release fires the
  item under the mouse if `commandEnabled`. **Single-shot port** (the D9
  press-and-hold deferral, identical to scroller/menu/input-line): on the
  mousedown, hit-test `item_mouse_is_in(local_point)` (C++ `itemMouseIsIn`,
  `tstatusl.cpp:135` — `mouse.y != 0 → None`; else walk items accumulating
  `i`/`k = i + cstrlen + 2` over `text != None` items, return the item whose
  `[i, k)` contains `mouse.x`). If the hit item exists **and**
  `cmd_set`-enabled, `ctx.post(item.command)` and `ev.clear()`. (C++
  `putEvent(evCommand) + clearEvent`.) Always `clearEvent` the mousedown (C++
  clears unconditionally after the loop). `TODO(row 31, D9: status-line
  press-and-hold drag-highlight + drawSelect hover)`.
- C++ calls `TView::handleEvent(event)` first (base mouse-move cursor etc.) —
  the base `View` default is a no-op for our purposes; do not port a call unless
  `MenuBar` does (it does not).

### Colors — **themes only, NO palettes** (D-rule: palette+glyphs → `Theme`)
**Critical:** rstv has **no runtime palette / `getPalette` / `getColor`
indirection** — that is a locked deviation. Colors resolve **directly from the
`Theme` via `Role`s** (`ctx.style(Role::...)`), exactly like
`MenuColors::resolve`. **Do NOT port `getPalette`/`getColor` or the
`cpStatusLine` string.** The C++ palette below is shown **only** as the
historical source for choosing the provisional theme *seed* colors — it is
reference, not something to implement.

C++ `getColor`s (`tstatusl.cpp:69-72`) → the four resolved pairs (the names map
to new `Role`s, NOT palette indices):
- `cNormal` → `(StatusNormal, StatusShortcut)`
- `cSelect` → `(StatusSelect, StatusShortcutSelect)`
- `cNormDisabled` → `StatusDisabled` (lo==hi)
- `cSelDisabled` → `StatusSelDisabled` (lo==hi)

For reference only, the classic `cpStatusLine` resolved through `cpAppColor` to
these bytes — use them to pick each `Role`'s provisional fg/bg seed:
| local | role | byte | meaning |
|------|------|------|---------|
| 1 | StatusNormal | `0x70` | black on lightgray |
| 2 | StatusDisabled | `0x78` | darkgray on lightgray |
| 3 | StatusShortcut | `0x74` | red on lightgray |
| 4 | StatusSelect | `0x20` | black on green |
| 5 | StatusSelDisabled | `0x28` | darkgray on green |
| 6 | StatusShortcutSelect | `0x24` | red on green |

Add **6 `Role` variants** to `src/theme.rs` (mirror the 6 `Menu*` roles exactly:
new enum variants, indices in the `index()` match, `set(...)` seeds in the
classic-blue builder, and the `ALL_ROLES`/round-trip list). Decode the bytes:
`0x70` → fg `Color::index(0)`, bg `Color::index(7)`, etc. (follow how the
existing menu/input seeds split a hex attr into fg/bg). Mark them
**provisional** with the same `TODO(row 34 gray theming)` note the menu/input
roles carry. Build a `StatusColors` helper analogous to `MenuColors`
(`resolve(ctx)` + `.item(disabled, selected)`) — or, if it is *identical* in
shape, factor honestly; but a separate `StatusColors` reading the `Status*`
roles is the safe, clear choice. Do **not** reuse `MenuColors` (different roles).

---

## Deferrals (faithful, breadcrumb every one — do NOT stub dead code)

- **`TProgram` integration** — the whole "wire a real status line into Program"
  step: `TProgram::getEvent` pre-routes `evKeyDown` (always) and `evMouseDown`
  (when the mouse is over the status line) to `statusLine->handleEvent` *before*
  normal dispatch; `TProgram::idle` calls `statusLine->update()`. **Out of scope
  here** (matches how the menu draw layer landed before the menu was wired into
  Program). Breadcrumb in the module doc.
- **The keyDown arm of `handleEvent`** (the global accelerator: match
  `event.keyDown == item.key_code && commandEnabled` over **all** items incl.
  `text==None`, then transform the event into `evCommand` **in place and return
  WITHOUT clearing** so it propagates). This is **deferred to the Program-wiring
  step** because the in-place-transform-and-propagate semantics only make sense
  inside `getEvent`'s pre-routing. **Do NOT port it as `ctx.post` + `clear`** (the
  menu hotkey path) — that double-handles. Leave a precise breadcrumb:
  `TODO(status keyDown global accelerator — lands with TProgram getEvent
  pre-routing; transform-in-place, not ctx.post)`.
- **`update()` / `findItems()` help-ctx refresh from `TopView`** (`tstatusl.cpp:283`):
  `update()` reads the modal top view's `getHelpCtx()` and re-runs `find_items` +
  redraw. The `TopView` plumbing is Program-level → lands with the wiring step.
  `find_items` itself **is** ported (unit-testable by setting `help_ctx`
  directly); only the *automatic* `update()` trigger is deferred. Add a
  `set_help_ctx(ctx)` (re-runs `find_items`) for tests and the future wiring.
- **`drawSelect(selected)` hover highlighting** — only `draw` (= `drawSelect(0)`)
  is needed; the `Some(item)` hover path is part of the deferred press-and-hold
  drag loop.
- **Streaming** (`read`/`write`/`build`/`streamableName`, `tstatusl.cpp:222+`) →
  **D12 dropped**.
- **`disposeItems`/destructor** → moot (owned `Vec`, RAII via `Drop`).

---

## Verification (Appendix B step 4)

- **Snapshot(s)** on `HeadlessBackend` (mirror `MenuBar`'s `render` helper +
  `Theme::classic_blue()`):
  1. A normal status line with two visible items + one disabled item (set
     `cmd_set` to a `CommandSet` lacking that command) → proves the color matrix
     and the `i += l+2` layout.
  2. A status line with a `hint` closure returning text → proves the hint tail
     (separator + clipped hint), with `i < size.x - 2` true.
  3. (optional) A narrow line where an item overflows (`i + l < size.x` false) —
     proves the clip-skip branch.
  Generate `.snap`s with `INSTA_UPDATE=always cargo test -p tvision <name>`,
  hand-verify, re-run plain, commit the `.snap`.
- **Unit tests** (bite-checked — verify each fails before / passes after):
  - `find_items`: an `All` def matches any ctx; with `[OneOf([a]), All]`, ctx `a`
    selects def 0, ctx `b` selects def 1 (the `All` fallback); first-match wins.
    BITE: reorder so `All` is first → everything selects def 0.
  - `item_mouse_is_in`: `mouse.y != 0 → None`; a click inside item 2's `[i,k)`
    returns item 2; a `text==None` item is skipped in the accumulator (its
    neighbours' columns are unaffected). BITE: a click in the gap (the trailing
    space col) maps correctly.
  - `text==None` item draws nothing and adds no width (a snapshot or a width
    assertion).
- **One `pump_once` integration test** (the graying broker end-to-end, mirror the
  row-49 `MenuProbe` test): build a `StatusLine`, insert it into a group, run a
  pump cycle where a command is disabled + `cmCommandSetChanged` is broadcast →
  assert the `StatusLine`'s `cmd_set` reflects the disabled command (via an
  inspection hook). Bite-check: without the broadcast arm wiring, `cmd_set`
  stays `None`/stale.

## Commands (Cargo **workspace**; artifacts → `$CARGO_TARGET_DIR`)
```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
cargo test  --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

## Locked conventions
- English for all code/comments/identifiers.
- Wire the module: `src/status/mod.rs` (`pub mod status_line; pub use ...`), add
  `pub mod status;` + `pub use status::{...}` to `lib.rs` (the orchestrator owns
  the `lib.rs` edit if there is a conflict — but for a fresh module you add it).
- If you add a new `View` trait method, add the forwarder to
  `tvision-macros/src/specs.rs` (you should NOT need to — reuse the existing
  `update_menu_commands` hook).
- No commit (the orchestrator integrates + commits).
