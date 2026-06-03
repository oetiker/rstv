# Brief — Row 28 `TListViewer` (FOUNDATION)

Port `magiblot-tvision/source/tvision/tlstview.cpp` (+ the class decl in
`include/tvision/views.h`) to `src/widgets/list_viewer.rs`. This is the abstract
base for **all** list widgets (`TListBox` row 48, history, color/file lists). It
drives **two sibling scrollbars** like `TScroller` (row 27) but differs
structurally in two ways the handover's "reuse verbatim" line glosses over — read
the **Design decisions** section first; they are pre-decided, apply them.

You are an Opus implementer working in the **shared tree** (no worktree — this is
a single FOUNDATION row, not a parallel batch). After implementing, run
`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`
(artifacts land in `/home/oetiker/scratch/cargo-target` — `CARGO_TARGET_DIR` is
set), and add a snapshot test (Appendix B step 4 — `cargo-insta` is NOT installed;
generate `.snap` with `INSTA_UPDATE=always cargo test <name>`, verify by hand,
re-run plain, commit the `.snap`). Do **not** commit — the orchestrator integrates.

---

## Design decisions (PRE-DECIDED — apply, do not relitigate)

### D-A. `ListViewer` is a TRAIT, not a concrete struct (≠ the Scroller shape)

`TListBox` reuses `TListViewer::draw` (does NOT override it) but overrides the
virtuals `getText`/`isSelected`. A concrete-struct-embedded-via-D2 base (the
Scroller shape) physically cannot dispatch back into the embedder's `getText` from
the base's own `draw`. So model the abstract base as a **trait** (the `Validator`
pattern, not the `Scroller` pattern):

```rust
/// Shared state of every list-viewer (the non-virtual data members).
pub struct ListViewerState {
    pub state: ViewState,          // the View composition target
    pub num_cols: i32,             // numCols (>= 1)
    pub top_item: i32,             // topItem
    pub focused: i32,              // focused
    pub range: i32,                // range
    pub indent: i32,              // CACHED hScrollBar->value (draw can't read the sibling live — see D-C)
    pub h_scroll_bar: Option<ViewId>,
    pub v_scroll_bar: Option<ViewId>,
}

pub trait ListViewer: View {
    fn lv(&self) -> &ListViewerState;
    fn lv_mut(&mut self) -> &mut ListViewerState;

    /// TListViewer::getText — base returns empty (C++ `*dest = EOS`).
    fn get_text(&self, _item: i32) -> String { String::new() }
    /// TListViewer::isSelected — base: `item == focused`.
    fn is_selected(&self, item: i32) -> bool { item == self.lv().focused }
    /// TListViewer::selectItem — base broadcasts cmListItemSelected (see D-E).
    fn select_item(&mut self, item: i32, ctx: &mut Context) {
        let _ = item;
        let source = self.lv().state.id();
        ctx.broadcast(Command::LIST_ITEM_SELECTED, source);
    }
}
```

`ListViewer` is **not** object-safe (it has `get_text -> String`); that's fine —
concrete widgets are stored as `Box<dyn View>`, and `ListViewer` is only ever a
**generic bound** behind concrete types. The shared draw/event/nav logic lives as
**free functions generic over `<L: ListViewer + ?Sized>`** in this module
(`pub(crate)` or `pub`), which the concrete widget's `View` impl calls:

```rust
pub fn draw<L: ListViewer + ?Sized>(this: &L, ctx: &mut DrawCtx) { … }
pub fn handle_event<L: ListViewer + ?Sized>(this: &mut L, ev: &mut Event, ctx: &mut Context) { … }
pub fn focus_item<L: ListViewer + ?Sized>(this: &mut L, item: i32, ctx: &mut Context) { … }
pub fn focus_item_num<L: ListViewer + ?Sized>(this: &mut L, item: i32, ctx: &mut Context) { … }
pub fn set_range<L: ListViewer + ?Sized>(this: &mut L, range: i32, ctx: &mut Context) { … }
pub fn set_state<L: ListViewer + ?Sized>(this: &mut L, flag: StateFlag, enable: bool, ctx: &mut Context) { … }
pub fn update_steps<L: ListViewer + ?Sized>(this: &L, ctx: &mut Context) { … } // ctor/changeBounds setStep
pub fn apply_scroll<L: ListViewer + ?Sized>(this: &mut L, h: Option<i32>, v: Option<i32>, ctx: &mut Context) { … }
```

> Why free functions, not provided trait methods: `draw` needs `&self` (for
> `get_text`) while building a buffer; the event/nav ones need `&mut self`. Free
> generics keep the borrow shapes obvious and avoid forcing every method into the
> trait's vtable shape. Provided trait methods are acceptable if you prefer them —
> but the `draw`/`handle_event`/etc. logic MUST be reusable by `TListBox` without
> reimplementation, and must call back into `get_text`/`is_selected`/`select_item`.

**No concrete list widget exists yet** (TListBox is row 48). The first real
consumer is a **test-only `FakeList`** (a `Vec<String>` + `HashSet<i32>` selected),
which is a legitimate consumer for the draw/nav/sync tests — NOT a dead stub. Build
it in `#[cfg(test)]`.

### D-B. Read-sync is a NEW `View` method + NEW `Deferred` variant — do NOT touch the scroller path

`Deferred::SyncScrollerDelta`'s apply arm hard-downcasts to `Scroller`; you cannot
downcast `dyn View → dyn ListViewer`. Add a parallel mechanism:

1. New defaulted-no-op `View` method (add to `src/view/view.rs`):
   ```rust
   /// The list-viewer read-sync broker hook (row 28). Defaulted no-op; concrete
   /// list widgets override to delegate to `list_viewer::apply_scroll`. The pump
   /// passes the freshly-read h/v scrollbar values (None if no bar).
   fn apply_list_scroll(&mut self, _h: Option<i32>, _v: Option<i32>, _ctx: &mut Context) {}
   ```
2. New `Deferred::SyncListViewer { list: ViewId, h: Option<ViewId>, v: Option<ViewId> }`
   in `src/view/context.rs` + a `Context::request_sync_list_viewer(list, h, v)`
   pusher (mirror `request_sync_scroller_delta`).
3. New apply arm in `src/app/program.rs` (mirror the `SyncScrollerDelta` arm, but
   call the trait method instead of downcasting — `ctx` IS live in the apply loop,
   see the `ScrollBarSetParams` arm which uses `&mut ctx` alongside `group`):
   ```rust
   Deferred::SyncListViewer { list, h, v } => {
       let hv = h.and_then(|id| group.find_mut(id)).and_then(|w| w.value()).and_then(field_int);
       let vv = v.and_then(|id| group.find_mut(id)).and_then(|w| w.value()).and_then(field_int);
       if let Some(view) = group.find_mut(list) {
           view.apply_list_scroll(hv, vv, &mut ctx);
       }
   }
   ```

Do **not** unify the scroller onto this — that refactors a working, tested
foundation for no row-28 benefit. Leave a one-line note that the two read-sync
mechanisms could later unify; out of scope.

### D-C. The `indent` cache + `apply_scroll` (the read-sync body)

C++ `draw()` reads `indent = hScrollBar->value` live. Under D3 the draw (a
`DrawCtx`) cannot reach the sibling bar, so `indent` is **cached on
`ListViewerState`** and refreshed by the read-sync. `apply_scroll(this, h, v, ctx)`
(the faithful merge of C++'s two `cmScrollBarChanged` branches):

```rust
if let Some(hv) = h { this.lv_mut().indent = hv; }   // hbar branch: C++ just drawView; we update the cached indent
if let Some(vv) = v { focus_item_num(this, vv, ctx); } // vbar branch: focusItemNum(vScrollBar->value)
```

(Always reading both each sync is harmless and matches "draw reads the live hbar":
the vbar write-back is a no-op in steady state — see D-D.)

### D-D. **TERMINATION — the centerpiece correctness property of this row**

The scroller's read-sync writes nothing back. The listviewer's **does**:
`focus_item_num → focus_item → vScrollBar.setValue(item)` → in our model a deferred
`ScrollBarSetParams{value}` → applied next pump → `set_params` → broadcasts
`SCROLL_BAR_CHANGED` **only if the value actually changed** → which would re-enter
the sync. This terminates **only because `ScrollBar::set_params` is change-guarded**
(`src/widgets/scrollbar.rs:219/224`: `scroll_draw` runs iff `old_value != a_value`),
so the write-back of the already-current value is a silent no-op.

- In steady state (vbar value already == focused): `focus_item` issues
  `setValue(focused)` where `focused == vbar.value` → no broadcast → quiescent.
- After a clamp (vbar value beyond range): one extra round, then quiescent.

**You MUST add a discriminating test** that drives a real vbar value change through
**multiple `pump_once` drains** (in `src/app/program.rs` tests, like the existing
scroller broker tests ~line 2240+) and asserts the deferred/out-event queues go
**quiet** (no infinite re-broadcast) AND the list's `focused`/`top_item` settle to
the expected values. Bite-check it (confirm it would spin/fail if the change-guard
were removed, e.g. by reasoning in a comment).

### D-E. Other faithful mappings

- **`cmScrollBarClicked` from an own bar → `select()`** → `ctx.request_focus(self_id)`
  (the row-41 `Deferred::FocusById` seam). Requires the list be inserted (have an
  id); skip if `id()` is `None`. Guard `(options & ofSelectable)` as C++ does.
- **`cmScrollBarChanged`** → request a deferred `SyncListViewer{list, h, v}` iff
  `source ∈ {h_scroll_bar, v_scroll_bar}` (the `source`-as-filter pattern, exactly
  like the scroller). Requires the list have an id.
- **`selectItem` → `ctx.broadcast(Command::LIST_ITEM_SELECTED, source=self_id)`**
  (`message(owner, evBroadcast, cmListItemSelected, this)`; the `infoPtr` → `source`
  successor).
- **Mouse:** ship **single-shot** positioning only. On `evMouseDown` inside the
  view: `newItem = mouse.y + size.y * (mouse.x / colWidth) + topItem` (mouse is
  view-local already in our model — `makeLocal`/`mouseInView` are gone; the group
  delivers view-local coords), `focus_item_num(newItem)`, and on
  `meDoubleClick && range > newItem` → `select_item(newItem)`. Then `clear_event`.
  **DEFER the `do…while(mouseEvent(...))` press-and-hold/auto-scroll loop** with
  `TODO(row 31, D9)` (same as scrollbar/inputline). Check the double-click flag on
  the `MouseEvent` (see how `cluster`/`input_line` read `me.flags`/double-click).
- **Keyboard** (`evKeyDown`): port the switch verbatim using `ctrl_to_arrow`
  (`src/event/key.rs`). Space (charCode==' ' && focused < range) → `select_item`.
  Up/Down/Left/Right (Left/Right only when numCols>1, else `return` = leave event
  uncleared)/PgUp/PgDn/Home/End/CtrlPgUp/CtrlPgDn → compute `newItem`,
  `focus_item_num(newItem)`, `clear_event`. `default` → `return` (leave uncleared).
- **`focus_item`** (C++ `focusItem`): `focused = item`; if `v_scroll_bar` →
  deferred `setValue(item)` (`ctx.request_scroll_bar_params(v, Some(item), None,
  None, None, None)`); the `else drawView()` is dropped (D8). Then the `topItem`
  adjust block **verbatim** (guarded by `size.y > 0`, the numCols==1 vs multi cases).
- **`focus_item_num`** (C++ `focusItemNum`): clamp (`item<0 → 0`; `item>=range &&
  range>0 → range-1`), then `if range != 0 { focus_item(item) }`.
- **`set_range`** (C++ `setRange`): `range = aRange`; if `focused >= aRange` →
  `focused = 0`; if `v_scroll_bar` → deferred `setParams(focused, 0, aRange-1,
  preserve pg, preserve ar)` = `ctx.request_scroll_bar_params(v, Some(focused),
  Some(0), Some(aRange-1), None, None)`; else drawView dropped.
- **`update_steps`** (C++ ctor's + `changeBounds`'s `setStep`): bars can't be
  reached from the no-`ctx` ctor (same constraint the scroller hit). So the ctor
  does NOT touch bars; expose `update_steps(this, ctx)` that the consumer/test calls
  after insertion. Body (faithful to the ctor):
  - vbar: if `numCols == 1` → `pgStep = size.y - 1`, `arStep = 1`; else
    `pgStep = size.y * numCols`, `arStep = size.y`; deferred
    `setStep(pgStep, arStep)` = `request_scroll_bar_params(v, None, None, None,
    Some(pgStep), Some(arStep))`.
  - hbar: `setStep(size.x / numCols, 1)` = `request_scroll_bar_params(h, None,
    None, None, Some(size.x / numCols), Some(1))`.
  - **Breadcrumb** `TODO(resize)`: `changeBounds` does not re-publish steps
    (geometry-only base default), exactly like the scroller's `set_limit`-on-resize
    TODO. The consumer calls `update_steps(ctx)` after a resize.
- **`set_state`** (C++ `setState`): flip the flag (+ the Focused broadcast — copy
  the scroller's `set_state` body for the Focused branch verbatim), then when `flag
  ∈ {Active, Selected, Visible}` (note: Visible too — but we have no
  `StateFlag::Visible`, D8 dropped it; so trigger on `{Active, Selected}` and add a
  comment that the C++ `sfVisible` arm is moot because visibility is not a
  StateFlag) show/hide BOTH bars: visible iff `active && visible` (read
  `self.state.state.active && self.state.state.visible` — note C++ uses
  `getState(sfActive) && getState(sfVisible)`, NOT the scroller's `active ||
  selected`!) via `ctx.request_set_visible`. `drawView` dropped.
- **ctor** (`ListViewerState`/widget `new(bounds, num_cols, h, v)`): `options |=
  ofFirstClick | ofSelectable` (`Options { first_click: true, selectable: true,
  ..Default }` — check the exact field names in `src/view/view.rs` `Options`);
  `topItem = focused = range = 0`, `indent = 0`, `num_cols = aNumCols`. The C++
  `eventMask |= evBroadcast` has **no analogue** (D4 broadcasts delivered
  unconditionally — same note as the scroller).

### D-F. `draw` — the render matrix (port `TListViewer::draw` faithfully)

Theme roles (the orchestrator is reconciling `theme.rs` to the 5-entry C++ palette
`Active(1)/Inactive(2)/Focused(3)/Selected(4)/Divider(5)` → roles
`ListNormalActive` / `ListNormalInactive` / `ListFocused` / `ListSelected` /
`ListDivider`). **Use those role names.** If they are not yet present when you
start, add them yourself (enum + index map + seed + any role arrays in `theme.rs`)
with provisional colors + `TODO(window-scheme remap)` (like `Role::ScrollerNormal`);
the old `ListNormal`/`ListSelectedFocused` roles are unused elsewhere (verified) —
remove/rename them.

Port the draw loop:
- If `(state & (sfSelected|sfActive)) == both`: `normal = ListNormalActive`,
  `focused = ListFocused`, `selected = ListSelected`. Else: `normal =
  ListNormalInactive`, `selected = ListSelected` (focused unused).
- `indent = self.lv().indent` (the CACHE, not a live bar read).
- `colWidth = size.x / numCols + 1`. For each row `i in 0..size.y`, each col `j in
  0..numCols`: `item = j*size.y + i + topItem`; `curCol = j*colWidth`.
  - focused cell (both-bits && `focused == item` && `range > 0`): color = focused,
    `setCursor(curCol+1, i)`, `focusedVis = true`.
  - else if `item < range && is_selected(item)`: color = selected.
  - else: color = normal.
  - Fill `colWidth` cells at `curCol` with ' ' in `color`. If `item < range`:
    `get_text(item)` → draw at `curCol+1` with width `colWidth`, **column-skip by
    `indent`** (the `moveStr(..., colWidth, indent)` begin-offset — reuse
    `DrawCtx::put_str_part`, the row-39 `moveStr begin` seam; check its exact
    signature). Else if `i==0 && j==0` → draw `emptyText` ("<empty>" — C++
    `emptyText` static; confirm the string in tlstview.cpp: it is `"<empty>"`) at
    `curCol+1` in `ListNormalActive` (getColor(1)).
  - Divider: write `'\xB3'` (U+2502 `│` — the box-drawing vertical; use the glyph
    we already use, check `theme.rs` Glyphs / how the frame draws `│`) at
    `curCol+colWidth-1` in `ListDivider`.
  - **`showMarkers` block: DROP** (consistent with button/cluster/statictext —
    `showMarkers` was removed at row 23).
  - `writeLine`/`b.moveChar` etc. → build through `DrawCtx` per row (look at how
    `cluster`/`input_line` build a row and blit — use the same approach; there is
    no `DrawBuffer`+`writeLine` in our model, you draw straight into `DrawCtx`).
- After the loops: `if !focusedVis { setCursor(-1,-1) }` → in our model, hide the
  cursor (set `cursor_request`/state cursor appropriately — check how other widgets
  signal "no cursor"; `Scroller`/`InputLine` show the convention).

> Coordinate type is `i32` (faithful). All string indexing through grapheme
> helpers (D13) if you slice `get_text` output — but `get_text` returns an owned
> `String` and `put_str_part` should handle width/skip, so you likely don't slice
> manually; if you do, use `crate::text` helpers (panic-safe).

---

## Drops / deferrals (faithful, breadcrumb each)
- `shutDown`/`write`/`read`/`build`/`streamableName`/`name` — D12/D2 streaming dropped.
- `getPalette` → Theme roles (D7), above.
- mouse press-and-hold/auto-scroll loop → `TODO(row 31, D9)`.
- `changeBounds` step republish → `TODO(resize)` (consumer calls `update_steps`).
- scroller/listviewer read-sync unification → optional later, out of scope.

## Wiring (shared files you will edit)
- `src/widgets/list_viewer.rs` (new) + `mod list_viewer;` + `pub use` in
  `src/widgets/mod.rs` (export `ListViewer`, `ListViewerState`, and the free fns as
  a `list_viewer` module path or re-exports — match how the codebase exposes module
  functions; `pub mod list_viewer` may be cleanest so callers write
  `list_viewer::draw`).
- `src/view/view.rs`: add the defaulted `apply_list_scroll` View method.
- `src/view/context.rs`: add `Deferred::SyncListViewer{..}` + the pusher.
- `src/app/program.rs`: add the apply arm + the termination test(s).
- `src/theme.rs`: the 5-role reconciliation (if not already done by the orchestrator).
- `src/lib.rs`: re-export if the house style re-exports widgets (check existing).

## Tests (make them DISCRIMINATING + bite-checked)
1. ctor defaults (options, zeroed fields).
2. `focus_item_num` clamp matrix (negative, ≥range, range==0).
3. `focus_item` topItem adjust (numCols==1 and numCols>1 cases, both scroll
   directions) — unit, no ctx-bar assertions needed beyond the deferred setValue.
4. `set_range` (focused reset when ≥range; the deferred vbar setParams shape).
5. `update_steps` (the numCols==1 vs >1 vbar step math + hbar step) — assert the
   deferred `ScrollBarSetParams` step fields.
6. `draw` snapshot: a `FakeList` of a few items, one focused+active, one selected,
   numCols==1; plus a multi-col snapshot; plus the empty (`range==0`) `<empty>`
   case. Use the frozen snapshot format.
7. `handle_event`: keyboard nav (Up/Down/Home/End/PgUp/PgDn; Left/Right no-op when
   numCols==1 leaving the event LIVE); Space→select_item broadcast; the
   `source`-filter on cmScrollBarChanged (own bar → SyncListViewer queued;
   foreign source → nothing; cmScrollBarClicked own bar → FocusById queued).
8. **The termination test (D-D)** through real `pump_once` drains — the headline.

## Definition of done
`cargo test` green (all prior + new), `cargo clippy --all-targets -- -D warnings`
clean, `cargo fmt --check` clean, snapshot `.snap` committed-ready, every drop/defer
breadcrumbed in code. Report the test count delta. Do NOT git commit.
