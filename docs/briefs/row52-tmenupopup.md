# Brief — Row 52 `TMenuPopup` (Phase 4, menu modal layer **stage 3**)

**Tag:** light-FOUNDATION (touches the FOUNDATION `menu_session.rs`; additive — no
new seam). **Model:** Opus. **C++:** `source/tvision/tmenupop.cpp`,
`source/tvision/popupmnu.cpp`, decl `include/tvision/menus.h:358-395`. **Target:**
`src/menu/menu_session.rs` (+ a `popup_menu` free fn; tests in
`src/app/program.rs`).

This is the **last modal piece** of the flattened `TMenuView::execute()`. Stages 1
(keyboard) + 2 (mouse) already landed the whole `MenuSession` — see
`src/menu/menu_session.rs` and read it fully before starting. **Do not re-port
`execute()`.** This row is a thin, additive delta.

---

## 0. Context — the architecture you are extending (read first)

The menu modal layer is **not** a per-view nested modal loop. It is **one
`MenuSession` capture handler** (`src/menu/menu_session.rs`) that owns the whole
open stack (`Vec<MenuLevel>`) and runs the flattened `execute()`. The bar/boxes are
display-only views (`MenuBar`/`MenuBox`); the session consumes every event on the
capture stack. Command select → `ctx.post(cmd)`; close → pop boxes + clear bar
highlight + `CaptureFlow::ConsumedPop`. The cross-level re-apply loop is
`MenuSession::run`. Seams already exist: `ctx.request_open_menu_box`,
`request_set_menu_current`, `request_close`, `post`, `put_event`, `push_capture`,
`owner_size`, `ViewId::next`.

In C++, `TMenuPopup` is a `TMenuBox` subclass with three differences. Mapped onto
our flattened model, **two of the three collapse to almost nothing** and the third
(`handleEvent`) is **moot** (justified below). The row is therefore:

1. a `put_click_event_on_exit: bool` on `MenuSession` gating the exit-click re-post,
2. a popup level that starts `current = None` (C++ `menu->deflt = 0`),
3. a `popup_menu(...)` free fn with `auto_place_popup` geometry.

---

## 1. C++ source (the whole of it)

### `tmenupop.cpp`
```cpp
TMenuPopup::TMenuPopup(const TRect& bounds, TMenu* aMenu, TMenuView *aParentMenu) noexcept :
    TMenuBox( bounds, aMenu, aParentMenu )
{
    putClickEventOnExit = False;          // (A) the ONE behavioural difference
}

ushort TMenuPopup::execute()
{
    menu->deflt = 0;                       // (B) no default highlight (looks ugly)
    return TMenuBox::execute();
}

void TMenuPopup::handleEvent(TEvent& event)
{                                         // (C) — MOOT under our model, see §4
    switch (event.what) {
    case evKeyDown:
        TMenuItem* p = findItem(getCtrlChar(event.keyDown.keyCode));
        if (!p) p = hotKey(event.keyDown.keyCode);
        if (p && commandEnabled(p->command)) { ... putEvent(evCommand,p->command); clearEvent; }
        else if (getAltChar(event.keyDown.keyCode)) clearEvent(event);
        break;
    }
    TMenuBox::handleEvent(event);
}
```

### `popupmnu.cpp` — the `popupMenu()` free fn
```cpp
ushort popupMenu(TPoint where, TMenuItem &aMenu, TGroup *receiver) {
    ushort res = 0;
    TGroup *app = TProgram::application;
    if (app) {
        TPoint p = app->makeLocal(where);
        TRect bounds(p, p);                            // ZERO-size rect at p
        TMenu *menu = new TMenu(aMenu);
        TMenuPopup *menuPopup = new TMenuPopup(bounds, menu);
        autoPlacePopup(menuPopup, p);
        res = app->execView(menuPopup);                // BLOCKS, returns the command
        TObject::destroy(menuPopup);
        if (res && receiver) {                         // re-post to the receiver group
            TEvent event = {}; event.what = evCommand;
            event.message.command = res; receiver->putEvent(event);
        }
    }
    return res;
}

static void autoPlacePopup(TMenuPopup *m, TPoint p) {  // Pre: bounds = TRect(p,p)
    TGroup *app = TProgram::application;
    TRect r = m->getBounds();                          // after ctor: (p-size)..(p)
    TPoint d = app->size - p;
    r.move( min(m->size.x, d.x), min(m->size.y + 1, d.y) );
    if (r.contains(p) && r.b.y - r.a.y < p.y)
        r.move(0, -(r.b.y - p.y));
    m->setBounds(r);
}
```

The `TMenuBox` ctor sizes the box from the menu via `getRect(bounds, aMenu)` —
**our `menu_box_rect(bounds, menu)`** (`src/menu/menu_box.rs:61`) is the exact port.
For the zero-size hint `Rect::new(p.x,p.y,p.x,p.y)` it yields
`(p.x-w, p.y-h)..(p.x, p.y)` (the box above-left of `p`), exactly as C++ `getRect`
(verified: `r.a.x + w < r.b.x` is `p.x+w < p.x` → false → `r.a.x = r.b.x - w`).

---

## 2. What to build

### (A) `put_click_event_on_exit` flag on `MenuSession`

Add `put_click_event_on_exit: bool` to `MenuSession`. Set it **`true`** in
`MenuSession::new` (the bar/box default — keeps every existing test green) and flip
it **`false`** only in `popup_menu`.

In `run()`, the bar-return branch (the `else` of `if self.levels.len() > 1`, the
single-level end) currently does:
```rust
let r = self.end_session_with(None, ctx);
if exit_click {
    ctx.put_event(ev);
}
return r;
```
Gate the re-post on the flag:
```rust
if exit_click && self.put_click_event_on_exit {
    ctx.put_event(ev);
}
```
That is the whole of difference (A). **Why a session-wide flag is faithful:** the
exit-click re-post in C++ only fires from the **bottom-most** level's `execute()`
(in our flattened model, `levels.len() == 1` — the bar for a bar session, the
single box for a popup). The bottom level's `putClickEventOnExit` is what governs
it: bar/box default `True`; `TMenuPopup` `False`. (Intermediate boxes in a bar+box
stack have `True` but their exit-clicks are carried up by the re-apply loop, not
re-posted — the existing `click-outside-closes+reposts` test proves the bar does
the single re-post.) So one session flag = the bottom level's
`putClickEventOnExit`. Do **not** add a per-`MenuLevel` field.

Update the `step_mouse` and `run` doc-comments that currently say
"`putClickEventOnExit` is modelled as **always True**" / "stage 3 gates it" — they
are now wrong; describe the flag.

### (B) Popup starts with `current = None`

C++ `TMenuPopup::execute` sets `menu->deflt = 0` before the loop, so the prologue
`current = menu->deflt` makes `current = None`. Build the popup level with
`current: None` **and** set the **level's cloned `menu.default = None`** (so the
`evMouseUp`-on-margin "highlight the default, else the first" arm —
`self.top().menu.default.or(Some(0))` — picks the first item, faithful to C++
`current = menu->deflt; if(current==0) current = menu->items`). The popup box must
never auto-highlight a default on open.

### (C) `popup_menu` free fn + `auto_place_popup`

Add to `menu_session.rs`:

```rust
/// `popupMenu` (`popupmnu.cpp`) — flattened: spawn a single-box `MenuSession` over
/// `menu` placed near `where_`, consuming events on the capture stack. On command
/// select the session `ctx.post`s the command (the faithful `receiver->putEvent`;
/// our model has no per-receiver routing seam — the command reaches the active
/// routing, which IS the receiver for the only consumer, the editor right-click).
/// `where_` is in root-group coords (C++ `app->makeLocal(where)`; the root group is
/// at (0,0), so makeLocal is identity in our model). `owner_size` is the root group
/// size (C++ `app->size`).
pub fn popup_menu(where_: Point, menu: Menu, owner_size: Point, ctx: &mut Context) {
    let bounds = auto_place_popup(where_, &menu, owner_size);
    let id = ViewId::next();
    // The popup box clears its default (menu->deflt = 0) — no highlight on open.
    let mut menu = menu;
    menu.default = None;
    ctx.request_open_menu_box(id, menu.clone(), bounds);
    let level = MenuLevel {
        view_id: id,
        menu,
        current: None,           // menu->deflt = 0
        bounds,
        is_bar: false,
        auto_select: false,
        last_target_item: None,
        mouse_active: false,
        first_event: true,
    };
    let mut session = MenuSession::new(vec![level], owner_size);
    session.put_click_event_on_exit = false;   // (A): TMenuPopup
    ctx.request_set_menu_current(id, None);
    ctx.push_capture(Box::new(session));
}
```

`auto_place_popup` — faithful port of `autoPlacePopup`, using `menu_box_rect` for
the initial sizing and `Rect::r#move` / `Rect::contains`:
```rust
/// `autoPlacePopup` (`popupmnu.cpp`). Initial box = `getRect(TRect(p,p), menu)`
/// (above-left of `p`); move it to sit below-right of `p` (top-left at
/// `(p.x, p.y+1)`), clamped to the desktop bottom-right; if it then covers `p` and
/// there is room above, shift it up so its bottom edge is at `p.y`.
fn auto_place_popup(p: Point, menu: &Menu, owner_size: Point) -> Rect {
    let mut r = menu_box_rect(Rect::new(p.x, p.y, p.x, p.y), menu); // (p-size)..(p)
    let size_x = r.b.x - r.a.x;     // m->size.x
    let size_y = r.b.y - r.a.y;     // m->size.y
    let dx = owner_size.x - p.x;
    let dy = owner_size.y - p.y;
    r.r#move(size_x.min(dx), (size_y + 1).min(dy));
    if r.contains(p) && (r.b.y - r.a.y) < p.y {
        r.r#move(0, -(r.b.y - p.y));
    }
    r
}
```
Then **re-export** `popup_menu` where the module's public items live (check how
`activate`/`activate_mouse` are surfaced from `menu/mod.rs` / `lib.rs` and mirror
it). Keep `auto_place_popup` private (an impl detail; tested via `popup_menu` and a
direct unit test in the same module).

---

## 3. Command delivery / `receiver`

`end_session_with(Some(cmd), ctx)` already `ctx.post(cmd)`s on select — identical
to `put_event(Event::Command(cmd))` (`post` pushes `Event::Command`). That IS the
faithful `receiver->putEvent`. Our model has **no per-receiver (`TGroup*`) routing
seam**, so the `receiver` argument is **dropped** — note it as a deviation in the
`popup_menu` doc. (The only C++ consumer, the editor right-click, passes its own
group as receiver; in our model the command reaches the active routing.) **No
change to `end_session_with`.**

---

## 4. What is DEFERRED / dropped (breadcrumb, do NOT stub)

- **(C) `TMenuPopup::handleEvent` (getCtrlChar/hotKey) is MOOT and dropped.**
  Reason — verified: `handleEvent` only runs when an event is **routed** to the view
  through the view tree. A popup created by `popupMenu` is immediately `execView`'d,
  so `execute()` owns the event loop via `getEvent` and `handleEvent` is **never
  invoked during the popup's modal life**. The only C++ consumer (`teditor1.cpp:536`
  `popupMenu(...)`) uses this `execView` path; a popup is **never** inserted as a
  persistent child whose `handleEvent` would route. So the override is dead on the
  only live path. Moreover, the keyboard accelerators it adds are already covered by
  our flattened loop: `step_default_key` runs `find_item` on the active level (the
  popup box) **and** `hot_key(self.levels[0].menu, k)` — and for a popup
  `levels[0]` **is** the popup box, so `topMenu()->hotKey` is the box's own tree,
  exactly the C++ behaviour. The only un-ported sliver is the **Ctrl+letter**
  (`getCtrlChar`) accelerator variant; breadcrumb it as
  `TODO(TMenuPopup Ctrl+letter accel: dead under the execView/capture-stack model;
  revive iff a persistent-popup consumer appears)`. Confirm in review that no
  persistent-insertion path exists.
- **Synchronous return value** (`ushort popupMenu` returns the command): impossible
  under the async capture-stack model (the function returns immediately; the command
  arrives later via `post`). Faithful to D9. Note it.
- **`receiver: TGroup*`** — dropped, §3.
- **Streaming** (`TMenuPopup::build`/`smenupop.cpp`) — D12.
- **`delete menu` dtor** — Rust ownership; the level owns its `Menu`.

---

## 5. Verification (discriminating + bite-checked `pump_once` tests in `program.rs`)

Use the existing harness (`program_with_desktop`, the `pump_once` drive loop, the
`Cmd`/`key`/`mouse` helpers; mirror the stage-1/2 menu tests at
`program.rs:3130+`). A popup is started by calling `popup_menu` from a test — add a
small helper that pushes it (you need a `Context`; do it the way the menu tests
reach the session, e.g. via a one-shot view or directly through a deferred queue —
inspect how `activate_mouse` is invoked end-to-end in
`click_*` tests and follow that route; if a popup needs a triggering view, add a
tiny `#[cfg(test)]` shim like `MenuProbe`/`FakeList` precedent rather than wiring a
real consumer).

Required tests (each must BITE — verify it fails before the change / passes after):

1. **`popup_opens_box_with_no_highlight`** — after `popup_menu` + a pump, exactly
   one `MenuBox` child exists and its `current == None` (BITE: a popup that copied
   the bar/box `current = menu->default` path would highlight item 0).
2. **THE anchor — `popup_click_outside_does_not_repost`** (the constraint that makes
   a popup a popup, `tmnuview.cpp:217-222` + `putClickEventOnExit=False`): drive a
   click outside the popup bounds → the session closes (box removed, capture popped)
   AND **no `Event::MouseDown` is re-posted** to the queue. Contrast assertion: an
   identical click-outside on a **bar** session (reuse the existing
   `click-outside-closes+reposts` setup) **does** re-post. If both behave the same,
   the flag is not wired. (This is the SPEC reviewer's crosshair.)
3. **`popup_select_command_posts_and_closes`** — click (down+up) on a command item
   → the command is `post`ed and the session closes.
4. **`auto_place_popup` geometry — bottom-edge shift-up** (the one piece with real
   logic): a direct unit test on `auto_place_popup`. Two cases:
   (a) room everywhere → top-left at `(p.x, p.y+1)`;
   (b) `p` near the desktop **bottom** so the clamped box would cover `p` and there
   is room above → the box is shifted up so `r.b.y == p.y` (BITE: drop the
   `if r.contains(p) ...` shift → the box covers the click row).
   Also assert the right-edge clamp keeps `r.b.x <= owner_size.x`.

Run before declaring done:
```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
cargo test  --workspace
cargo clippy --workspace --all-targets -- -D warnings    # force a fresh lint
cargo fmt --all --check
```
No snapshot is needed (no new draw path — `MenuBox` already has snapshots). Report
the exact lib-test count delta.

---

## 6. Scope fence

Touch **only** `src/menu/menu_session.rs` (the flag, `popup_menu`,
`auto_place_popup`, the two stale doc-comments), the `menu/mod.rs` / `lib.rs`
re-export of `popup_menu`, and the new tests in `src/app/program.rs`. **Do not**
modify `execute()`'s step logic, the draw layer, `MenuBox`/`MenuBar`, `Deferred`,
or `Context`. No new `Deferred` variant. No `MenuLevel` field. If you find yourself
needing any of those, stop and flag it — the design says you do not.
