# Brief — Rows 50/51 **draw/data layer**: `MenuView` trait + `TMenuBar` + `TMenuBox` drawing

> **Tag:** FOUNDATION (pattern-setting trait seam) · **Model:** Opus · per-row, two-stage review.
> **Scope this session = the DRAW/DATA layer only.** The modal `execute()` loop,
> `TMenuPopup`, mouse/key navigation, `newSubView`/`do_a_select`, and the D9
> view-triggered async-modal path are **all deferred to a separate, advisor-vetted
> Step-2 design session.** This step lands a *snapshot-tested, independently
> committable* substrate that Step 2 navigates. (HANDOVER bundles 50/51/52 "together";
> that bundling is wrong — draw is independently testable and de-risks the modal work.)

## Why this split is safe

`TMenuBar::draw`/`TMenuBox::draw` and `getItemRect` do **not** read the modal state
(`parentMenu`, the `execute()` loop-locals). They read only `menu` + `current`. So the
draw layer is a clean, verifiable slice. The one cross-step constraint: the **trait
shape + `current` representation** you lock here must be designed against `execute()`'s
known needs (it has been read — see "Step-2 constraints" at the end) so Step 2 *grows*
the trait rather than redoing it.

## C++ sources (port verbatim)

- `magiblot-tvision/source/tvision/tmnuview.cpp` — `getItemRect` (base, returns
  `(0,0,0,0)`), `getPalette` (`cpMenuView`), `updateMenu`/`hotKey` (**already ported**
  as free fns in `menu_view.rs` — reuse, do not rewrite).
- `magiblot-tvision/source/tvision/tmenubar.cpp` — `TMenuBar::draw`, `TMenuBar::getItemRect`.
- `magiblot-tvision/source/tvision/tmenubox.cpp` — `getRect` (static sizing helper),
  `TMenuBox::frameLine`, `TMenuBox::draw`, `TMenuBox::getItemRect`.
- Palette string: `cpMenuView "\x02\x03\x04\x05\x06\x07"` (`tmnuview.cpp:40`).

## What exists already (build on it — do NOT reinvent)

- **`src/menu/menu_view.rs`** (row 49): `MenuViewState { state: ViewState, menu: Menu }`,
  free fns `hot_key(&Menu, KeyEvent) -> Option<Command>`,
  `update_menu_commands(&mut Menu, &CommandSet)`, and the **passive** `handle_event(&MenuViewState, &mut Event, &mut Context)`
  (command-graying broadcast + accelerator post; activation branches breadcrumbed).
- **`src/menu/mod.rs`**: `Menu { items: Vec<MenuItem>, default: Option<usize> }`,
  `MenuItem::{Separator, Command{name,command,key_code,param,help_ctx,disabled},
  SubMenu{name,key_code,help_ctx,disabled,menu}}`. `name` is a `~`-marked label.
- **`src/screen/draw_buffer.rs`**: `DrawBuffer::move_char(indent, ch, style, count)`,
  `move_buf(indent, &[Cell])`, `move_cstr(indent, text, lo, hi) -> usize` (the
  `~`-hotkey toggle: text in `lo`, char after a `~` in `hi`), `put_char(indent, ch)`,
  `move_cstr_part`/`move_str_part`. Use `move_cstr` for item names (passes a `(lo,hi)`
  style pair = the C++ `TAttrPair`).
- **`src/theme.rs` `Glyphs`**: single-line box glyphs already present —
  `frame_tl` ┌, `frame_tr` ┐, `frame_bl` └, `frame_br` ┘, `frame_h` ─, `frame_v` │,
  `frame_tee_l` ├, `frame_tee_r` ┤. **These are exactly the `TMenuBox::frameChars`
  glyphs** (that C++ table is a *separate* static from `TFrame::frameChars`; do not
  conflate). No new glyphs needed.
- **`cstrlen`** (display width stripping `~`): a private `fn cstrlen(&str) -> i32`
  exists in `src/widgets/button.rs:453` and another local copy in `cluster.rs`. The
  C++ menu code calls `cstrlen(p->name)` for widths. Add a **local** `cstrlen` to the
  menu module mirroring `button.rs` (the precedent is per-module copies; a third is
  consistent — or factor into a shared `text` helper if trivial, your call, but match
  `button.rs`'s `UnicodeWidthChar`-based logic exactly).
- **Template for the trait shape:** `src/widgets/list_viewer.rs` — the row-28
  `ListViewer: View` trait with `lv()/lv_mut()` accessors + defaulted virtuals, the
  concrete `impl View` (`list_box.rs:122`) returning `&state.state` for `state()`.

## Deliverables

### 1. The `MenuView` trait (in `src/menu/menu_view.rs`)

This is where row 49's "no trait yet" decision **flips**: `getItemRect` and `draw` are
the overridable virtuals that differ between bar and box. Mirror `ListViewer`:

```rust
pub trait MenuView: View {
    fn mv(&self) -> &MenuViewState;
    fn mv_mut(&mut self) -> &mut MenuViewState;

    /// `TMenuView::getItemRect` — the screen rect of item `index` within this view.
    /// Base returns an empty rect (C++ `TRect(0,0,0,0)`); `TMenuBar`/`TMenuBox` override.
    fn get_item_rect(&self, _index: usize) -> Rect {
        Rect::new(0, 0, 0, 0)
    }
}
```

- Keep `hot_key`/`update_menu_commands`/`handle_event` as the existing **free fns**
  (they take `&Menu`/`&MenuViewState`; no need to move them onto the trait).
- **Note for the reviewer (omit-until-consumer tension, resolve in review):**
  `get_item_rect`'s only callers (`trackMouse`/`execute`/`getHelpCtx`) are Step 2, so by
  the strict row-35/48 "no dead stubs" rule it could be deferred. We implement it **now,
  with `draw`,** deliberately: the item geometry and the draw layout are the *same
  contract* and must agree cell-for-cell; building + unit-testing them together locks
  that contract while the layout is fresh, and gives Step-2 navigation a verified
  substrate. The trait itself is the Step-2 polymorphism seam (`execute()` calls
  `getItemRect`/`draw`/`newSubView` on `MenuView` pointers). This is the advisor-endorsed
  reason the draw layer lands first. (Spec reviewer: confirm this is the right call, or
  flag if `get_item_rect` should be deferred after all.)

### 2. Extend `MenuViewState` with `current`

```rust
pub struct MenuViewState {
    pub state: ViewState,
    pub menu: Menu,
    /// `TMenuView::current` — the highlighted item, an **index** into `menu.items`
    /// (C++ `TMenuItem* current`; `None` == C++ `current == 0`). Consistent with
    /// `Menu::default` (also an index). Draw compares `Some(i) == current` to pick the
    /// selected color; defaults to `None` (nothing highlighted).
    pub current: Option<usize>,
}
```

Update `MenuViewState::new` to take/initialize `current` (default `None` — or add a
`new` that sets `current: None` and let tests set the field directly; it is `pub`).

- **Keep `parentMenu` deferred** — `draw`/`getItemRect`/`getRect` never read it; only the
  Step-2 modal-nav methods (`mouseInOwner`/`mouseInMenus`/`topMenu`/`getHelpCtx`) do.
- Verify `current: Option<usize>` against `execute()`'s mutations (already audited:
  `current = menu->deflt` → index; `nextItem`/`prevItem` wrap by index;
  `current = p` → index; `menu->deflt = current; current = 0` → set default + `None`;
  `p == current` comparisons → index eq). It fits all of them — do not change it.

### 3. `TMenuBar` — `src/menu/menu_bar.rs`

```rust
pub struct MenuBar { mv: MenuViewState }
```

- **Ctor:** `MenuBar::new(bounds, menu)` builds `MenuViewState` over a `ViewState` at
  `bounds`. C++ sets `growMode = gfGrowHiX` and `options |= ofPreProcess` — port both
  (the menu bar stretches with the screen width and pre-processes events). Set them on
  the `ViewState` (`grow_mode`/`options` fields — check their names against `window.rs`/
  `desktop.rs`; `ofPreProcess` → `options.pre_process` or equivalent).
- **`impl MenuView`:** `mv`/`mv_mut`; `get_item_rect` = the **horizontal accumulator**
  from `tmenubar.cpp:94` — start `r=(1,0,1,1)`; for each item, `r.a.x = r.b.x`, and if
  `name != Separator` `r.b.x += cstrlen(name) + 2`; return `r` when the loop index hits
  `index`. (Faithful: separators don't advance x; the C++ loops `while(True)` and is only
  ever called with an index that exists.)
- **`impl View`:** `state()`/`state_mut()` → `&self.mv.state`/`&mut self.mv.state`;
  `draw` = port `tmenubar.cpp:48` (see "draw porting" below); `handle_event` →
  **delegate to the row-49 passive `menu_view::handle_event(&self.mv, ev, ctx)`** (C++
  `TMenuBar::handleEvent` *is* `TMenuView::handleEvent`, not overridden — so the passive
  command-graying + accelerator post is correct now; the activation/modal branches are
  already breadcrumbed inside that free fn for Step 2). All other `View` methods: trait
  defaults (this is **not** a D2 View-embed — `MenuViewState` embeds `ViewState`, not a
  `View` — so `#[delegate]` does **not** apply; hand-write `state`/`state_mut`/`draw`/
  `handle_event` like `list_box.rs`).

### 4. `TMenuBox` — `src/menu/menu_box.rs`

```rust
pub struct MenuBox { mv: MenuViewState }
```

- **`getRect` sizing helper** (`tmenubox.cpp:25`, a free fn `fn menu_box_rect(bounds: Rect, menu: &Menu) -> Rect`):
  `w=10, h=2`; for each named item `l = cstrlen(name) + 6`, `+3` if submenu (`command==0`),
  else `+ cstrlen(param) + 2` if it has a param; `w = max(l, w)`; `h++` per item (named or
  not). Then clamp the box into `bounds`: if `a.x + w < b.x` set `b.x = a.x + w` else
  `a.x = b.x - w` (same for y/h). Port verbatim.
- **Ctor:** `MenuBox::new(bounds, menu)` → bounds = `menu_box_rect(bounds, &menu)`.
  C++ sets `state |= sfShadow` and `options |= ofPreProcess` — port both.
- **`impl MenuView`:** `get_item_rect` = `tmenubox.cpp:125` — walk items counting `y`
  from 1, return `Rect::new(2, y, size.x-2, y+1)` for the matched index.
- **`impl View`:** `draw` = port `tmenubox.cpp:80` via a `frame_line` helper (below);
  `handle_event` → delegate to `menu_view::handle_event` (TMenuBox inherits
  TMenuView::handleEvent); `state`/`state_mut` as for the bar.

#### `frameLine` / the box border (`tmenubox.cpp:73`)

The C++ `frameChars` table (a string indexed by `n` ∈ {0=top,5=bottom,10=middle,15=sep})
decodes to single-line glyphs (all in `Glyphs`). Decoded `frameChars[0..20]`:

```
idx:  0   1   2   3   4   5   6   7   8   9
      ' ' ┌   ─   ┐   ' ' ' ' └   ─   ┘   ' '
idx: 10  11  12  13  14  15  16  17  18  19
      ' ' │   ' ' │   ' ' ' ' ├   ─   ┤   ' '
```

`frameLine(b, n)` writes: cells `0,1 = frameChars[n], frameChars[n+1]` (the `cNormal`
attr); cells `2 .. size.x-2 = frameChars[n+2]` (the `color` attr — varies per line);
cells `size.x-2, size.x-1 = frameChars[n+3], frameChars[n+4]` (`cNormal`). So a border
row leaves **column 0 and column size.x-1 blank** with the corner/edge glyphs at columns
1 and size.x-2 — this is faithful TV (the box frame is inset one column); document it so
it's not mistaken for a bug, and let the snapshot capture it. Build a small lookup keyed
by `n` returning `(left2, fill, right2)` glyph triples from `Glyphs`, or port the index
table directly — either is fine, but the OUTPUT must match the decoded table above.
`draw` order: top frame line (n=0) → one line per item (n=15 frameLine for a separator;
n=10 frameLine + `move_cstr(3, name, color)` + the submenu `►`/param for a named item) →
bottom frame line (n=5). The submenu marker: C++ `b.putChar(size.x-4, 16)` where glyph 16
= `►` (`\x10`, CP437 right-pointing triangle) — use the existing `►` glyph if present
(the input-line arrows added `► U+25BA`; check `Glyphs`), else add it. Param text:
`move_cstr(size.x-3-cstrlen(param), param, color)`.

### 5. Theme — the `cpMenuView` palette → roles (`src/theme.rs`)

The C++ draw uses 4 `TAttrPair`s, each a (text, hotkey-highlight) pair from the 6-entry
`cpMenuView` palette `\x02\x03\x04\x05\x06\x07` (palette idx 1..6):

| C++ | `getColor` | palette idx (text / hi) | role pair to add |
|-----|-----------|--------------------------|------------------|
| `cNormal`        | `0x0301` | 1 / 3 | `MenuNormal` / `MenuNormalShortcut` |
| `cSelect`        | `0x0604` | 4 / 6 | `MenuSelected` / `MenuSelectedShortcut` |
| `cNormDisabled`  | `0x0202` | 2 / 2 | `MenuDisabled` (text==hi) |
| `cSelDisabled`   | `0x0505` | 5 / 5 | `MenuSelectedDisabled` (text==hi) |

Add **6 `Role` variants** (`MenuNormal`, `MenuNormalShortcut`, `MenuSelected`,
`MenuSelectedShortcut`, `MenuDisabled`, `MenuSelectedDisabled`) with **provisional**
colours (mirror the cluster's shortcut-split: normal = bg-ish bar color w/ contrasting
text, selected = inverted/highlight, disabled = dim). Mark `TODO(row 34 gray theming):
realign provisional menu colours` like the existing Input/List roles. The bar background
fill (`b.moveChar(0,' ',cNormal,size.x)`) uses `MenuNormal`. For `move_cstr` pass
`(lo=MenuNormal_or_Selected, hi=MenuNormalShortcut_or_SelectedShortcut)`; for disabled
items pass the same role for both lo and hi (no shortcut highlight when greyed).

Resolve the role→`Style` at the draw site (the centralized per-widget convention), via
the same `DrawCtx`/`Theme` accessor the other widgets use (`ctx.style(Role::…)` or
whatever `list_box.rs`/`button.rs` call — match it).

### 6. Wiring + verification

- Wire `menu_bar`/`menu_box` modules into `src/menu/mod.rs`; re-export `MenuBar`,
  `MenuBox`, `MenuView` from `src/lib.rs` (follow the row-46 `pub use menu::{…}` line).
- **Snapshot tests** (Appendix B step 4; `cargo-insta` NOT installed → generate with
  `INSTA_UPDATE=always cargo test <name>`, verify the `.snap` by hand against the C++
  layout, re-run plain, commit the `.snap`). Build each view on a `HeadlessBackend`,
  `render`, `assert_snapshot!`:
  1. **Menu bar**, e.g. ` File  Edit  Window ` with `current = Some(<Edit's index>)`
     (highlighted) **and** one disabled item (greyed) — proves the 4-color matrix +
     `~`-hotkey highlight + the x-accumulation layout.
  2. **Menu box** with a frame + a highlighted `current` item + a **disabled** item +
     a **separator** + an item with a `param` (shortcut text) + a **submenu** item (the
     `►` marker) — proves `frameLine`, the inset border, the 4 colors, param right-align,
     and the submenu marker.
- **Unit tests** for `get_item_rect` on both bar (horizontal rects, separators don't
  advance x) and box (vertical rects, `y` from 1), **discriminating + bite-checked**:
  assert the rects' x/y match the draw layout (e.g. the bar's item-2 rect starts where
  item-1's name ended + 2). Plus a `menu_box_rect` sizing test (width = longest
  name+padding, height = items+2).
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --all --check` all green. `export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target`.

## Verified substrate (exact names — orchestrator pre-checked these)

- `ViewState` is in `src/view/view.rs`. Flags: `State { shadow: bool, .. }`,
  `Options { pre_process: bool, .. }`, and `grow_mode` (the `gfGrowHiX` variant exists —
  see `view.rs:156`). Set `growMode = gfGrowHiX` / `options.pre_process = true` /
  `state.shadow = true` on the `ViewState` accordingly.
- Draw resolves a role via **`ctx.style(Role::…) -> Style`** (see `list_viewer.rs:491`);
  `move_cstr(indent, text, lo: Style, hi: Style)`. The `View::draw` signature is
  `fn draw(&mut self, ctx: &mut DrawCtx)`.
- The submenu `►` marker already exists as a glyph: `Glyphs::input_right_arrow` /
  `sb_h_arrow_fwd` = `'\u{25BA}'` (= CP437 `\x10`). Reuse one, or add a dedicated
  `menu_submenu_marker` field if you prefer a distinct name (either is fine).
- `Rect::new(x0,y0,x1,y1)` is the `(a,b)` half-open rect (`a`=top-left, `b`=bottom-right),
  matching C++ `TRect`. `state().get_bounds()` / `state().size` are available.

## Explicitly OUT of scope (Step 2 — do not build, do not stub)

`execute()` modal loop, `trackMouse`/`trackKey`/`nextItem`/`prevItem`, `findItem`/
`findAltShortcut`, `do_a_select`/`newSubView`/`mouseInOwner`/`mouseInMenus`/`topMenu`,
`getHelpCtx`, `TMenuPopup`, the activation branches of `handle_event` (mouse-down /
`cmMenu` / alt-shortcut menu-open), the D9 `Deferred::OpenModal`/insert-view path,
`parentMenu`, streaming (D12). Leave the existing row-49 breadcrumbs intact.

## Step-2 constraints (record only — so the trait you lock now survives)

The Step-2 design is **not** "execute() = OpenModal" (likely a HANDOVER mis-read). D9 says
modality is a **handler, not a loop**: menu navigation will be a **capture stack** (one
frame per open level; opening a submenu pushes a capture frame + inserts a `TMenuBox`;
closing pops). `execute()`'s loop-locals (`autoSelect`/`lastTargetItem`/`mouseActive`/
`firstEvent`/`itemShown`) become persistent state on that frame. `Deferred::OpenModal` is
for a dialog a menu *command* launches (msgbox/Batch E), **not** menu nav. The trait +
`current` shape locked here must let Step 2 add `parentMenu` + the nav methods without
reshaping `get_item_rect`/`draw`/`mv()` — design accordingly.
