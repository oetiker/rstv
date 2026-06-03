# Implementer brief — Row 27 `TScroller` (FOUNDATION)

> Self-contained. Do **not** "go read the plan." Everything you need is inline:
> the C++ source, the cross-view design decision, the exact new seams, the
> D-rules, and the tests + checks to run. Port **faithfully**; deviate only where
> this brief says so (with a `D#`/breadcrumb).

## What this is

`TScroller` is the base class for scrollable content views (`TEditor` row 66,
`TTextDevice`/`TTerminal` 91/92, `TOutlineViewer` 89). It holds references to
**two sibling `TScrollBar`s** (horizontal + vertical) that live on the window
frame, mirrors their `value` into its own `delta` (the scroll offset its
subclasses draw with), and pushes range/value changes back to them.

PORT-ORDER row 27: base `TView`; takes 2×`TScrollBar` (row 25 ✅); `delta`/`limit`.
**Reclassified MECHANICAL → FOUNDATION**: it establishes the **cross-view sibling
broker** pattern reused verbatim by `TListViewer` (28) and `TEditor` (66). Get it
right.

Target module: `src/widgets/scroller.rs` (new) + wire into `src/widgets/mod.rs`
and re-export per house style (`tv::Scroller`). Plus the shared-seam edits listed
under **New seams** below.

## The C++ source (`source/tvision/tscrolle.cpp`, verbatim)

```cpp
#define cpScroller "\x06\x07"

TScroller::TScroller(const TRect& bounds, TScrollBar *aHScrollBar, TScrollBar *aVScrollBar) noexcept :
    TView(bounds), drawLock(0), drawFlag(False),
    hScrollBar(aHScrollBar), vScrollBar(aVScrollBar)
{
    delta.x = delta.y = limit.x = limit.y = 0;
    options |= ofSelectable;
    eventMask |= evBroadcast;
}

void TScroller::changeBounds(const TRect& bounds) {
    setBounds(bounds);
    drawLock++;
    setLimit(limit.x, limit.y);
    drawLock--;
    drawFlag = False;
    drawView();
}

void TScroller::checkDraw() noexcept {
    if (drawLock == 0 && drawFlag != False) { drawFlag = False; drawView(); }
}

TPalette& TScroller::getPalette() const {              // "\x06\x07"
    static TPalette palette(cpScroller, sizeof(cpScroller)-1); return palette;
}

void TScroller::handleEvent(TEvent& event) {
    TView::handleEvent(event);
    if (event.what == evBroadcast && event.message.command == cmScrollBarChanged &&
        (event.message.infoPtr == hScrollBar || event.message.infoPtr == vScrollBar))
        scrollDraw();
}

void TScroller::scrollDraw() {
    TPoint d;
    d.x = (hScrollBar != 0) ? hScrollBar->value : 0;
    d.y = (vScrollBar != 0) ? vScrollBar->value : 0;
    if (d.x != delta.x || d.y != delta.y) {
        setCursor(cursor.x + delta.x - d.x, cursor.y + delta.y - d.y);
        delta = d;
        if (drawLock != 0) drawFlag = True; else drawView();
    }
}

void TScroller::scrollTo(int x, int y) noexcept {
    drawLock++;
    if (hScrollBar != 0) hScrollBar->setValue(x);
    if (vScrollBar != 0) vScrollBar->setValue(y);
    drawLock--;
    checkDraw();
}

void TScroller::setLimit(int x, int y) noexcept {
    limit.x = x; limit.y = y;
    drawLock++;
    if (hScrollBar != 0)
        hScrollBar->setParams(hScrollBar->value, 0, x - size.x, size.x - 1, hScrollBar->arStep);
    if (vScrollBar != 0)
        vScrollBar->setParams(vScrollBar->value, 0, y - size.y, size.y - 1, vScrollBar->arStep);
    drawLock--;
    checkDraw();
}

void TScroller::showSBar(TScrollBar *sBar) {
    if (sBar != 0) {
        if (getState(sfActive | sfSelected) != 0) sBar->show();
        else sBar->hide();
    }
}

void TScroller::setState(ushort aState, Boolean enable) {
    TView::setState(aState, enable);
    if ((aState & (sfActive | sfSelected)) != 0) { showSBar(hScrollBar); showSBar(vScrollBar); }
}

void TScroller::shutDown() { hScrollBar = 0; vScrollBar = 0; TView::shutDown(); }
```

(`drawLock`/`drawFlag`/`checkDraw` are the synchronous re-entrancy guard — see
the **D8 deviation** below. `shutDown`/`write`/`read`/streaming are dropped — D2
ownership/`Drop`, D12.)

## THE design decision — the cross-view scrollbar broker (read **and** write)

**Problem.** In C++ the scroller holds raw pointers to its two scrollbars and both
*reads* their fields (`->value`, `->arStep` in `scrollDraw`/`setLimit`) and
*mutates* them (`setValue`/`setParams`/`show`/`hide`). In our model (D3) a leaf
view holds only `&mut Context` during dispatch — **no tree access; it can neither
read nor mutate a sibling.** The scrollbars are window-frame siblings, so the
scroller cannot own them.

**Solution — the pump is the cross-view broker, in both directions.** The scroller
stores its scrollbars as `Option<ViewId>` handles and issues `Deferred` ops naming
them. The **pump** (which holds the whole tree, and already drains `deferred` after
dispatch — `program.rs` line ~713, *before* `resetCursor`/`render`) performs every
cross-view read and write at apply time via `group.find_mut(id)`.

**Do NOT extend `Event::Broadcast`.** Keep `{command, source}` exactly as is. This
is the *faithful* successor to C++: `cmScrollBarChanged`'s `infoPtr` is the subject
pointer and the receiver reads `value` *off the subject* — our successor is "the
pump resolves the subject and reads `value`," **not** "stuff the value into the
message." `source` stays the **filter** only: the scroller reacts iff
`source ∈ {h_id, v_id}` — this is the first real consumer of `Broadcast{source}`.

### Struct

```rust
pub struct Scroller {
    state: ViewState,
    pub delta: Point,          // scroll offset; subclasses (editor) draw with it
    limit: Point,              // content extent (x,y)
    h_scroll_bar: Option<ViewId>,
    v_scroll_bar: Option<ViewId>,
}
```

`new(bounds, h_scroll_bar: Option<ViewId>, v_scroll_bar: Option<ViewId>)`:
`delta = limit = (0,0)`; `options.selectable = true`; `event_mask` gains
`evBroadcast`. (Match how existing widgets set `ViewState` + mask in their ctors.)

**Drop `drawLock`/`drawFlag`/`checkDraw` entirely (D8).** They are a synchronous
re-entrancy guard around C++'s immediate `drawView()`. Under D8 (whole-tree redraw
+ diff every pass) there is no immediate draw to guard, and our mutations are
**deferred** (applied in one post-dispatch drain), so the batching the lock
provided is structural. This matches how `buffered`/lock were dropped elsewhere.
Add a one-line module-doc breadcrumb noting the drop and why.

### Read direction — `scrollDraw` via `Deferred::SyncScrollerDelta`

`handle_event`: first delegate to the base (`TView::handleEvent` is a no-op in our
trait — call the default / do nothing extra), then:

```text
if event is Broadcast { command: SCROLL_BAR_CHANGED, source: Some(s) }
   and (Some(s) == h_scroll_bar or Some(s) == v_scroll_bar):
       ctx.request_sync_scroller_delta(self.state.id(), h_scroll_bar, v_scroll_bar)
```

i.e. push `Deferred::SyncScrollerDelta { scroller: ViewId, h: Option<ViewId>, v: Option<ViewId> }`.

**Pump apply** (`program.rs`, new match arm — mirror the existing `ChangeBounds`/
`FocusById` arms):
```text
let dx = h.and_then(|id| group.find_mut(id)).and_then(|v| v.value())
            .and_then(int_of).unwrap_or(0);
let dy = v.and_then(|id| group.find_mut(id)).and_then(|v| v.value())
            .and_then(int_of).unwrap_or(0);
if let Some(s) = group.find_mut(scroller)
                      .and_then(|v| v.as_any_mut())
                      .and_then(|a| a.downcast_mut::<Scroller>()) {
    s.apply_delta(Point::new(dx, dy));   // does the setCursor adjust + delta = d
}
```
- Read scrollbar value through **`View::value()` → `FieldValue::Int`** (see seams).
  Read each scrollbar in its own `find_mut` (sequential; only one `&mut` live at a
  time), then `find_mut` the scroller.
- `Scroller::apply_delta(d)` ports `scrollDraw`'s body exactly: if
  `d != self.delta`, set `self.state.cursor = (cursor.x + delta.x - d.x,
  cursor.y + delta.y - d.y)`, then `self.delta = d`. No draw call (D8).
- Reach the concrete `Scroller` via `View::as_any_mut` (the sanctioned
  concrete-reach hatch — same one `TWindow::zoom` uses). Override `as_any_mut` on
  `Scroller`.

### Write direction — `setLimit` / `scrollTo` via `Deferred::ScrollBarSetParams`

One flexible variant serves row 27 **and** rows 28/66 (`tlstview.cpp` uses the same
shape — `setRange`/`setStep`/`setValue`). Per-field `Option` = **"preserve the
scrollbar's live value where `None`"**; the pump fills `None` slots from the live
scrollbar's `pub` fields, then calls `set_params`:

```rust
Deferred::ScrollBarSetParams {
    id: ViewId,
    value: Option<i32>, min: Option<i32>, max: Option<i32>,
    page_step: Option<i32>, arrow_step: Option<i32>,
}
```
Pump apply:
```text
if let Some(sb) = group.find_mut(id).<downcast to ScrollBar> {
    let v  = value.unwrap_or(sb.value);
    let lo = min.unwrap_or(sb.min_value);
    let hi = max.unwrap_or(sb.max_value);
    let pg = page_step.unwrap_or(sb.page_step);
    let ar = arrow_step.unwrap_or(sb.arrow_step);
    sb.set_params(v, lo, hi, pg, ar, &mut ctx);   // set_params already clamps + may scroll_draw
}
```
`ScrollBar` is reached by `as_any_mut`/downcast too — **override `as_any_mut` on
`ScrollBar`** (it does not have one yet). Its fields are already `pub`.

Map the C++ calls onto it:
- **`setLimit(x, y)`**: set `self.limit = (x, y)`; for the H bar (if `Some`) push
  `ScrollBarSetParams { id, value: None, min: Some(0), max: Some(x - size.x),
  page_step: Some(size.x - 1), arrow_step: None }`; for the V bar analogously with
  `y`/`size.y`. (`value`+`arrow_step` preserved = `None`, exactly C++'s
  `setParams(value, 0, …, …, arStep)`.) `size` = `self.state.get_bounds()` size.
- **`scrollTo(x, y)`**: H bar → `{ value: Some(x), rest None }`; V bar →
  `{ value: Some(y), rest None }` (this is `setValue`, which preserves everything
  but value).
- **`changeBounds(bounds)`**: `self.state.set_bounds(bounds)` (or the trait
  `change_bounds` path — match how other views apply bounds), then call your
  `set_limit(limit.x, limit.y)` logic to re-emit params for the new size. (Drop the
  `drawLock`/`drawFlag`/`drawView` lines.) Note `change_bounds` here takes a
  `&mut Context` because it must emit deferred ops — see the trait-signature note
  below.

### Visibility direction — `showSBar` via `Deferred::SetVisible`

There is **no** `StateFlag::Visible` (D8 dropped `sfVisible`'s propagating
side-effects), but visibility *is* `ViewState.state.visible` and the painter
honors it (`group.rs` skips `!visible` children). Add:

```rust
Deferred::SetVisible(ViewId, bool)
```
Pump apply: `if let Some(v) = group.find_mut(id) { v.state_mut().state.visible = b; }`
(no downcast — `state_mut` is on the trait).

`set_state` override (port `TScroller::setState`): after the base
`set_state(flag, enable, ctx)` runs, if `flag` is `Active` or `Selected`, call
`show_sbar(h)` + `show_sbar(v)` where `show_sbar(id)` = for `Some(id)` push
`SetVisible(id, self.state.state.active || self.state.state.selected)`. (C++
`getState(sfActive|sfSelected) != 0` = "either bit set".) Read the *post-update*
active/selected bits from `self` (the base `set_state` already flipped them).

## New seams (shared-file edits — you own them; this row is serial, no worktree)

1. **`src/data.rs`** — add `FieldValue::Int(i32)` variant (the handover earmarked
   it; this is its first consumer). Update the module doc that says "only `Text`
   exists." Add a tiny equality unit test.
2. **`src/widgets/scrollbar.rs`** — `impl View for ScrollBar`: override
   `fn value(&self) -> Option<FieldValue> { Some(FieldValue::Int(self.value)) }`
   and `fn as_any_mut(&mut self) -> Option<&mut dyn Any> { Some(self) }`.
3. **`src/view/context.rs`** — add the three `Deferred` variants
   (`SyncScrollerDelta`, `ScrollBarSetParams`, `SetVisible`) with doc comments in
   the style of the existing variants (note each touches the *view tree* family, so
   the `69897fe` insertion-order drain stays order-equivalent — no dispatch
   co-queues two ops on the *same* scrollbar/scroller in a conflicting order).
   Add matching `Context` request methods:
   `request_sync_scroller_delta(scroller, h, v)`,
   `request_scroll_bar_params(id, value, min, max, page_step, arrow_step)`,
   `request_set_visible(id, visible)`.
4. **`src/app/program.rs`** — add the three apply arms in the deferred drain
   `match` (alongside `ChangeBounds`/`SetState`/`Close`/`FocusById`). Reuse the
   `group.find_mut(id)` pattern. For `SyncScrollerDelta`, sequence the reads as
   described (drop each `&mut` before the next `find_mut`).
5. **`src/theme.rs`** — add `Role::ScrollerNormal` + `Role::ScrollerSelected` to the
   enum and the index map; seed provisional styles from `cpScroller "\x06\x07"` →
   `cpAppColor[6] = 0x28`, `cpAppColor[7] = 0x24` (BIOS byte = `bg<<4 | fg`:
   `0x28` = fg 8 on bg 2, `0x24` = fg 4 on bg 2). Derive the `Style` the same way
   the existing widget roles (e.g. `InputNormal`/`ListNormal`) are seeded from BIOS
   bytes. Add `TODO(row 34 gray theming / window-scheme remap)` — these are the
   app-direct colors; a scroller inside a window remaps via the palette chain
   (deferred, like the other provisional widget colors).
6. **`src/view/view.rs`** — **trait-signature note:** the base `View::change_bounds`
   currently takes `(&mut self, bounds)` with no `Context` (see line ~666). The
   scroller's `change_bounds` must emit deferred `ScrollBarSetParams` ops, which
   needs `&mut Context`. **Check the current signature.** If `change_bounds` has no
   `Context`, do **NOT** widen the trait method for this one case — instead apply
   bounds via the existing path and re-emit limit params from wherever the
   scroller is first given a real size. Simplest faithful option: keep the trait
   `change_bounds(bounds)` as-is for geometry, and expose `Scroller::set_limit(x, y,
   ctx)` as the `Context`-taking entry that subclasses/tests call to (re)publish
   params; have `change_bounds` store bounds only. **Flag this to the orchestrator
   if it forces a trait change** — a `Context` on `change_bounds` is a FOUNDATION
   decision, not a quiet widening. (Most likely: `set_limit(ctx)` + `scroll_to(ctx)`
   are the public `Context`-taking methods; `change_bounds` just sets bounds and a
   `TODO` to republish on resize when a window consumer exists.)

## Deviations / drops (faithful, breadcrumb each)
- **D8:** drop `drawLock`/`drawFlag`/`checkDraw` + all `drawView()` calls (whole-tree
  redraw; deferred mutation batching).
- **D3:** scrollbars referenced by `ViewId`, not pointers; all cross-view read/write
  brokered by the pump at deferred-apply.
- **D12/D2:** drop `shutDown`/`write`/`read`/streaming.
- **getPalette → Theme roles** (D7): `Scroller{Normal,Selected}`.
- The base `Scroller::draw` has **no C++ body** (abstract base; subclasses draw the
  content). Implement `draw` to fill the view rect with `ScrollerNormal` (or
  `ScrollerSelected` when `state.active && state.selected`) so the base is
  observable/snapshot-testable. Breadcrumb: subclasses (editor) override `draw` and
  consume `delta`.

## Tests (state-based — the base draws only a fill)
Build on a `HeadlessBackend`/`Program` or unit-test the pieces directly. Make each
test **discriminating + bite-checked** (confirm it fails before the behavior, passes
after):
1. **ctor**: `ofSelectable` set, `evBroadcast` in mask, `delta == limit == (0,0)`.
2. **scrollDraw via the pump (read broker)**: insert a `Scroller` + an h-bar + a
   v-bar into a `Group`/`Program`; set a scrollbar's value (→ it broadcasts
   `SCROLL_BAR_CHANGED { source }`); pump a cycle; assert the scroller's `delta`
   updated to the bar's value **and** that a broadcast whose `source` is **not** one
   of the two bars leaves `delta` unchanged (the `source` filter bites).
3. **setLimit (write broker)**: call `set_limit(100, 50, ctx)`; drain; assert the
   h-bar's `max_value == 100 - size.x`, `page_step == size.x - 1`, and `value`/
   `arrow_step` **preserved**; analogously the v-bar.
4. **scrollTo (write broker)**: `scroll_to(10, 5, ctx)`; drain; assert bar values
   set (and clamped to range by `set_params`).
5. **setState/showSBar**: focus/select the scroller → drain → both bars `visible`;
   deselect → both hidden. (Drive through real `set_state` + pump drain, not a
   hand-set flag.)
6. **cursor adjust**: a scroller with a non-zero `state.cursor`, scrollbar value
   change → assert `cursor` shifted by `delta - d` (the `setCursor` line).
7. **trivial snapshot** (Appendix B step 4): base scroller fill renders as a block
   of the scroller color. `cargo-insta` not installed → generate with
   `INSTA_UPDATE=always cargo test <name>`, eyeball, re-run plain, commit the
   `.snap`.

## Run before declaring done
`CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target`:
- `cargo test` (all green; report the count)
- `cargo clippy --all-targets -- -D warnings` (clean)
- `cargo fmt --check` (clean)

Report: what you built, every new seam, any place you deviated from this brief and
why (the brief can be wrong — if the C++ or the existing types contradict it, follow
them and say so), and the trait-`change_bounds`/`Context` question's resolution.
```
