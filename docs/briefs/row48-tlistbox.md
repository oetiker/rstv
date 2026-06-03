# Row 48 — `TListBox` (MECHANICAL; first concrete `TListViewer`)

Port `tlistbox.cpp` (+ `slistbox.cpp`/`nmlstbox.cpp` = streaming, **dropped**) — the
first **concrete** `TListViewer`. It proves the row-28 trait seam end to end: it
reuses `TListViewer`'s draw/event/nav verbatim and overrides only `getText`.

New file `src/widgets/list_box.rs`; wire it into `src/widgets/mod.rs` (declare
`mod list_box;` + `pub use list_box::ListBox;`) and re-export in `src/lib.rs`
(append `ListBox` to the `pub use widgets::{…}` line — line 110).

`CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target`. After building, run
`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.

## The C++ being ported (verbatim)

```cpp
// ctor: TListViewer(bounds, aNumCols, 0 /*hbar*/, aScrollBar /*vbar*/), items(0); setRange(0);
ushort TListBox::dataSize() { return sizeof(TListBoxRec); }            // TListBoxRec { TCollection* items; ushort selection; }
void   TListBox::getData(void* rec) { p->items = items; p->selection = focused; }
void   TListBox::getText(char* dest, short item, short maxChars) {     // overrides TListViewer::getText
    if (items != 0) { strncpy(dest, items->at(item), maxChars); dest[maxChars]='\0'; }
    else *dest = EOS;
}
void   TListBox::newList(TCollection* aList) {
    destroy(items); items = aList;
    if (aList != 0) setRange(aList->getCount()); else setRange(0);
    if (range > 0) focusItem(0);
    drawView();                                                        // dropped (D8 whole-tree redraw)
}
void   TListBox::setData(void* rec) { newList(p->items); focusItem(p->selection); drawView(); }
TCollection* TListBox::list() { return items; }
```
`TListBox` does **NOT** override `isSelected` or `selectItem` — it inherits the
row-28 base (`is_selected: item == focused`; `select_item` broadcasts
`cmListItemSelected`). Only `getText` is overridden. `Palette layout` = the same
5-entry cpListViewer the base already maps to Theme roles — nothing to add.

## Our shape — copy the `#[cfg(test)] FakeList` template in `list_viewer.rs` exactly

`ListBox` embeds `ListViewerState` + owns the collection as `Vec<String>` (the
`TCollection` of `char*` → `Vec<String>`; `getText`'s `strncpy` → index the Vec).

```rust
pub struct ListBox {
    lv: ListViewerState,
    items: Vec<String>,
}
```

### Inherent API
- `pub fn new(bounds: Rect, num_cols: i32, h: Option<ViewId>, v: Option<ViewId>) -> Self`
  — faithful to the C++ ctor: `ListViewerState::new(bounds, num_cols, h, v)` +
  `items: Vec::new()`. `range` stays 0 (C++ `setRange(0)`). **No `Context` here**
  (ctor can't reach the bars — same constraint as the scroller/`ListViewerState`;
  the post-insert `update_steps` + the `new_list` `set_range` publish the bars).
- `pub fn new_list(&mut self, items: Vec<String>, ctx: &mut Context)` — ports C++
  `newList`: replace `self.items`, `list_viewer::set_range(self, len, ctx)`, then
  `if range > 0 { list_viewer::focus_item(self, 0, ctx) }`. (`destroy(items)` =
  the old Vec drops on assignment; `drawView()` dropped, D8.)
- `pub fn list(&self) -> &[String]` — C++ `list()` accessor.

### `impl ListViewer for ListBox`
- `lv(&self) -> &ListViewerState { &self.lv }`, `lv_mut` likewise.
- `get_text(&self, item: i32) -> String` — `self.items.get(item as usize).cloned().unwrap_or_default()`
  (the `items == 0 → EOS` and bounds cases both collapse to empty; faithful).
- Do **NOT** override `is_selected` / `select_item` (inherit the base).

### `impl View for ListBox` — delegate to the `list_viewer::*` free fns (verbatim from FakeList)
- `state`/`state_mut` → `&self.lv.state` / `&mut self.lv.state`.
- `draw` → `list_viewer::draw(self, ctx)`.
- `handle_event` → `list_viewer::handle_event(self, ev, ctx)`.
- `set_state` → `list_viewer::set_state(self, flag, enable, ctx)`.
- `cursor_request` → `list_viewer::focused_cursor(self)`.
- `apply_list_scroll` → `list_viewer::apply_scroll(self, h, v, ctx)`
  (**forgetting this silently loses scroll-sync — no compile error**).
- `as_any_mut` → `Some(self)` (the broker downcasts through it).
- `value(&self) -> Option<FieldValue>` → **`Some(FieldValue::Int(self.lv.focused))`**.
  This is the D10 selection transfer (`getData`'s `selection = focused`). The
  collection is configuration `new_list` manages, NOT part of the transferable
  value (don't clone the Vec into a `FieldValue`; no `List` variant — `FieldValue`
  grows per consumer, and there's no gather/scatter consumer yet).

## DEFER `set_value` (do NOT override it) — the decided design point

C++ `setData` = `newList(items)` + `focusItem(selection)`, and **both need a
`Context`** in our model (they publish to the v-bar via deferred ops). But the
trait's `fn set_value(&mut self, v: FieldValue)` is **`Context`-free** — so it
**cannot** faithfully port `setData`: it could set `focused` directly but could
not republish the v-bar, leaving the scroll thumb desynced after a scatter (a
visible bug, since `draw` is `&self` and nothing reconciles it until the next
scroll event). So **leave `set_value` unimplemented** (inherit the no-op default)
and add a module-doc breadcrumb:

> `set_value` (the scatter half of `getData`/`setData`) is **deferred**: it needs a
> `Context` (to republish the v-bar via `new_list`/`focus_item`) that the
> `Context`-free `View::set_value` signature does not provide. It lands with the
> dialog **gather/scatter group-walk** consumer (inputBox / Batch E), which must
> itself solve threading a `Context` into scatter. `TODO(set_value: dialog
> gather/scatter)`.

## Population wiring (state this so the implementer doesn't hit the no-Context wall)

A list box is **populated at construction time, before insertion**, where there is
no `Context` — but `set_range`/`focus_item`/`update_steps` all need one. So:
- ctor sets fields only (empty `items`, `range` 0) — no `Context`.
- `new_list(items, ctx)` does the `set_range` (publishes v-bar value/min/max) +
  `focus_item(0)` — call it **post-insert**, with a `Context`.
- The page/arrow **step** publish is `list_viewer::update_steps(self, ctx)`, which
  C++ does in the `TListViewer` ctor but our ctor cannot (no `Context`). The
  **caller** must call `update_steps` post-insert (same as the `FakeList`/scroller
  pattern). Miss either `new_list`'s `set_range` OR `update_steps` and the thumb
  starts unsynced. Document this on `new_list` / the type.

## Tests (Appendix B step 4 — discriminating + bite-checked)
Model them on `FakeList`'s tests. At minimum:
1. **ctor** — empty `items`, `range == 0`, `ofFirstClick`/`ofSelectable` set
   (inherited from `ListViewerState::new`), `num_cols` clamped `>= 1`.
2. **`new_list`** — populating N items sets `range == N`, queues the v-bar
   `set_range` `ScrollBarSetParams{value, min:0, max:N-1}` (insert into a `Group`
   for a real v-bar id, drive a `Context`, inspect `deferred`), and `focus_item(0)`
   queues the v-bar `setValue(0)`. Empty list → `range == 0`, `focus_item` skipped.
   `new_list` over a previously-populated list **replaces** the items (old text gone).
3. **`get_text`** — returns the owned item; out-of-range → empty (bite: assert a
   real item differs from empty).
4. **`value()`** — `Some(FieldValue::Int(focused))`; moving focus (drive a KeyDown
   through `handle_event`) changes the reported value (bite: focused 0 vs 2).
5. **draw snapshot** — a populated active+focused `ListBox` renders item text +
   the focused row (reuse the `FakeList` `render` helper shape; `cargo-insta` not
   installed → generate the `.snap` with `INSTA_UPDATE=always cargo test <name>`,
   verify it by hand, re-run plain, commit the `.snap`).
6. **delegation smoke** — a KeyDown(Down) through `View::handle_event` moves
   `focused` (proves `handle_event` is wired); a broadcast `cmScrollBarChanged`
   from its own v-bar queues `Deferred::SyncListViewer` (proves the broker filter
   is reachable through the concrete type).

## Drops / deferrals (faithful, breadcrumb each)
- `dataSize`/`getData`/`setData` streaming-record shape → the typed `value`/(deferred
  `set_value`) above; `TListBoxRec` has no analogue.
- `write`/`read`/`build`/`streamableName`/`name` → D12 streaming dropped.
- `drawView()` calls → D8 whole-tree redraw.
- Mouse press-and-hold/auto-scroll, `change_bounds` step republish, etc. are all
  **in the row-28 base** already (don't re-port them here).
