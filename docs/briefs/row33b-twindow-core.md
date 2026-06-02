# Implementer brief — Row 33b: TWindow core (the static selectable window)

You are porting magiblot/tvision behavior to idiomatic Rust in the `tvision`
crate (house alias `tv::`). **Row 33 (`TWindow`) is staged.** Stage **33a**
(Group/Context primitives: command-enable channel, Z-reorder `make_first`/
`put_in_front_of`, `ofTopSelect` raise-on-select) is **already committed** — build
on it. This is stage **33b**: the `TWindow` **core** as a *static, selectable*
window. Port **faithfully**; the only departures are the pre-decided deviations
named below. Do **not** invent features.

**Explicitly deferred to 33c (do NOT build, NO dead stubs):** `zoom`, `cmResize`/
move/grow **drag** loops, `close`/destroy (self-removal), and the `setState`
**command-enable** (cmZoom/cmResize/cmClose/cmNext/cmPrev) — every one of these
needs infrastructure not present yet (owner-extent-down channel, capture
handlers, a close-removal channel). Building any half-wired here is worse than a
clean defer (the advisor's explicit guidance). Leave precise breadcrumbs.

C++ source of truth (read it):
`/home/oetiker/scratch/tvision-spec/magiblot-tvision/source/tvision/twindow.cpp`
and the decl in `include/tvision/views.h` (`class TWindow : public TGroup, public
virtual TWindowInit`). `minWinSize = {16, 6}`.

You work in the shared tree `/home/oetiker/checkouts/rstv` (no worktree). New
module **`src/window/`** (`mod.rs` + `window.rs`); wire into `src/lib.rs`.

---

## What you are building: `Window` embeds a `Group` (D2), is a `View`

Like `Desktop` (row 30 — read `src/desktop/desktop.rs`, it is the template), a
`Window` embeds a `Group` and delegates the `View` trait to it, overriding
`draw`/`handle_event`/`setState`/`sizeLimits` where `TWindow` does. The frame is a
**group child** (faithful to C++: `TWindow` inserts the frame). Read
`src/frame.rs` (the `Frame` view + its owner-data-down setters
`set_title`/`set_flags`/`set_number`/`set_zoomed`) and `src/view/group.rs`
(insert/set_state/the 33a additions).

```rust
pub struct Window {
    group: Group,
    /// `TWindow::frame` — the frame child's id (33c's zoom pushes `set_zoomed`
    /// through it; kept live by `frame_id()`).
    frame_id: ViewId,
    /// `TWindow::flags` (D5 struct-of-bools, relocated here from frame.rs).
    flags: WindowFlags,
    /// `TWindow::zoomRect` — saved bounds for un-zoom (consumed by 33c's zoom).
    zoom_rect: Rect,
    /// `TWindow::number`.
    number: i16,
    /// `TWindow::palette` — the colour scheme (blue/cyan/gray). See getPalette.
    palette: WindowPalette,
    /// `TWindow::title`.
    title: Option<String>,
}
```

### Relocate `WindowFlags` to this module
Move the `WindowFlags` struct from `src/frame.rs` into `src/window/` (it belongs
to `TWindow`, per the row-24 note "TWindow will own/relocate it"). Keep the
crate-root re-export working (`pub use window::WindowFlags;` in lib.rs; remove the
`frame::WindowFlags` re-export and update `frame.rs`'s `use` to import it from the
window module — or keep `WindowFlags` referenced by `frame.rs` via the new path).
`frame.rs` still needs the type (its `flags` field). Verify the crate compiles and
the existing frame tests pass.

### `WindowPalette` (getPalette under D7)
C++ `getPalette` returns one of three palettes indexed by `palette`
(`wpBlueWindow`/`wpCyanWindow`/`wpGrayWindow`). Under D7 there is no `getPalette`;
instead introduce:
```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum WindowPalette { #[default] Blue, Cyan, Gray }
```
Store it (`palette: WindowPalette`, default `Blue` = `wpBlueWindow`, the ctor
default) and expose `palette()`. **Multi-scheme theming is deferred:** the `Frame`
currently renders the single (blue) scheme via `Role::FrameActive`/`FramePassive`/
`FrameDragging`. Mapping `Cyan`/`Gray` to distinct theme roles is **row 34's job**
(`TDialog` uses `Gray`). Document this seam; do NOT expand the `Theme`/`Role` set
now.

### Construction — `Window::new(...)` ports `TWindow::TWindow` + `TWindowInit`

> **DECISION (post-review):** do **NOT** take a `create_frame` factory param at
> 33b. Under D3 the factory's output is structurally unusable now — a custom
> `Frame` would need title/flags/number pushed *into* it via the downcast seam
> that defers to 33c, so the factory could only be called for an `.is_some()`
> guard and its boxed view discarded. That is hollow, misleading ceremony in a
> pattern-setting view. Build the `Frame` **directly** in `new`; leave a grep-able
> `TODO(33c)` noting the C++ `createFrame`/`TWindowInit` hook reappears once the
> downcast seam lets a custom frame receive pushed-down owner data.

Signature:
```rust
pub fn new(bounds: Rect, title: Option<String>, number: i16) -> Self
```
Faithful ctor (`twindow.cpp`):
1. `let mut group = Group::new(bounds);`
2. `flags = wfMove | wfGrow | wfClose | wfZoom` (all four true).
3. `zoom_rect = bounds` (C++ `zoomRect(getBounds())`).
4. `palette = WindowPalette::Blue`.
5. On the group's state: `state |= sfShadow` (`st.state.shadow = true`);
   `options |= ofSelectable | ofTopSelect` (`selectable = true; top_select = true`);
   `growMode = gfGrowAll | gfGrowRel` — i.e. `lo_x=lo_y=hi_x=hi_y=true, rel=true`
   (check `GrowMode` field names in view.rs; `gfGrowAll = gfGrowLoX|LoY|HiX|HiY`).
6. Create the frame via `create_frame(group.state().get_extent())`; if `Some`,
   **push the owner data down before inserting** — but the factory returns
   `Box<dyn View>`, and you need to call `Frame::set_title`/`set_flags`/
   `set_number` on it. Two clean options; pick one and document:
   - (a) Provide the default factory `Window::init_frame(r) -> Box<dyn View>`
     returning a `Box::new(Frame::new(r))`, and have `new` create the `Frame`
     concretely (not via the boxed factory) so it can call the setters, then box +
     insert. The injected factory is honored for the "is a frame created?" guard.
   - (b) Add the downcast seam now (see below) and push via `child_mut` after
     insert.
   Prefer (a) for 33b (no post-insert mutation needed yet): create the `Frame`,
   `set_title(title.clone())`, `set_flags(flags)`,
   `set_number(number_to_option(number))`, then `insert` it and store `frame_id`.
   (`wnNoNumber` maps to `None`; C++ `wnNoNumber == 0`, so `number == 0` → `None`,
   else `Some(number as u8)` when `0 < number < 10` — match `frame.rs`'s
   `number: Option<u8>` contract and its `n < 10` draw guard. Verify the
   `wnNoNumber` value in `views.h`.)
7. Store `frame_id`; add `pub fn frame_id(&self) -> ViewId` (keeps it live).

> **Downcast seam (only if you choose (b), else skip):** add a defaulted
> `fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> { None }` to the
> `View` trait (object-safe; default returns `None`, so NO ripple — only `Frame`
> overrides it with `{ Some(self) }`), plus `Group::child_mut(id) -> Option<&mut
> dyn View>`. For 33b option (a), you do **not** need this — leave it for 33c.

### `getTitle`
C++ `getTitle(short)` ignores its arg and returns `title` (frame.rs already
documents this). Provide `pub fn title(&self) -> Option<&str>`.

### `sizeLimits` — override
`TWindow::sizeLimits` calls `TView::sizeLimits(min,max)` then sets `min =
minWinSize {16,6}`. Override `View::size_limits(owner_size)` on `Window`: call the
inner group's `size_limits(owner_size)` for `max`, then force `min = Point::new(16,
6)`. (Check the exact `size_limits` signature/return in view.rs — it returns
`(min, max)`.)

### `setState` — override (PARTIAL for 33b)
C++ `TWindow::setState(aState, enable)`:
```cpp
TGroup::setState(aState, enable);
if (aState & sfSelected) {
    setState(sfActive, enable);            // self-recursion
    if (frame) frame->setState(sfActive, enable);
    // ... build windowCommands {cmNext,cmPrev, cmResize?, cmClose?, cmZoom?}
    // enableCommands/disableCommands(windowCommands);
}
```
For 33b, port the **activation** but **DEFER the command-enable block**:
- `fn set_state(&mut self, flag, enable, ctx)`: call `self.group.set_state(flag,
  enable, ctx)` (delegates + propagates to children incl. the frame). Then **if
  `flag == StateFlag::Selected`**: call `self.group.set_state(StateFlag::Active,
  enable, ctx)` (the self-recursion → `Group::set_state(Active)` propagates
  `sfActive` to **all** children incl. the frame, so the frame goes active/passive
  automatically — the explicit C++ `frame->setState(sfActive)` is therefore
  redundant here, as `frame.rs` already notes; do NOT also push it manually).
- **DEFER the windowCommands enable/disable** with a precise breadcrumb:
  ```rust
  // TODO(33c): on sfSelected, enable/disable the window command set via
  // ctx.enable_command/disable_command (33a channel): always cmNext+cmPrev;
  // cmResize if (grow|move); cmClose if close; cmZoom if zoom. Deferred until
  // their handlers exist (zoom/drag/close are 33c) — enabling a command whose
  // handler is absent would be a dead/inert command (pump filters or no-ops it).
  ```
  Do not call `ctx.enable_command` at all in 33b.

### `standardScrollBar` — port fully
`TWindow::standardScrollBar(aOptions)`:
```cpp
TRect r = getExtent();
if (aOptions & sbVertical) r = TRect(r.b.x-1, r.a.y+1, r.b.x, r.b.y-1);
else                       r = TRect(r.a.x+2, r.b.y-1, r.b.x-2, r.b.y);
insert(s = new TScrollBar(r));
if (aOptions & sbHandleKeyboard) s->options |= ofPostProcess;
return s;
```
Realize as `pub fn standard_scroll_bar(&mut self, opts: ScrollBarOptions) ->
ViewId` (return the inserted scrollbar's `ViewId`, since we have no pointer).
`ScrollBar::new(rect)` exists (row 25). Introduce a small `ScrollBarOptions`
struct-of-bools or reuse existing flags for `vertical`/`handle_keyboard` (check
`src/widgets/scrollbar.rs` for any existing options type; if none, a local
`{ vertical: bool, handle_keyboard: bool }` is fine — port `sbVertical`/
`sbHandleKeyboard`; `sbHorizontal == 0` is the default). For `handle_keyboard`,
set the inserted scrollbar's `state_mut().options.post_process = true` via
`Group::child` access — you DO need to set an option on the just-inserted child;
since `insert` returns the id and you have it immediately, the simplest faithful
path is to set `post_process` on the `ScrollBar` **before** boxing+inserting it
(mutate the concrete `ScrollBar`, then insert). Do that.

### `handle_event` — override (PARTIAL for 33b)
C++ `TWindow::handleEvent`: `TGroup::handleEvent(event)` first, then command/key/
broadcast handling. For 33b:
- Delegate: `self.group.handle_event(ev, ctx)`.
- **evKeyDown**: `kbTab` → `self.group.focus_next(false, ctx)` + consume;
  `kbShiftTab` → `self.group.focus_next(true, ctx)` + consume. (Verify the
  forwards/backwards sense against C++ `focusNext(False)` for kbTab — match it;
  33a wired `focus_next`.) Use the project's `Key`/`KeyEvent` types; check how
  Tab/Shift-Tab are represented (`Key::Tab`? `Key::BackTab`? grep `event/key.rs`).
- **DEFER (breadcrumbs, do not handle):**
  - `cmResize` → `dragView(dmDragMove|...)` — **33c** (needs owner-extent channel +
    drag capture handler).
  - `cmClose` → `close()`/post `cmCancel` if modal — **33c** (needs close-removal
    channel; modal path is row 34).
  - `cmZoom` → `zoom()` — **33c** (needs owner-extent channel).
  - `evBroadcast cmSelectWindowNum` matching `number` → `select()` — **deferred**:
    D4 dropped event payloads, so the broadcast cannot carry the target number
    (this is the Alt-N deferral already noted in `program.rs`). Breadcrumb it.

### `draw` — delegate
`TWindow` does not override `draw` (it inherits `TGroup::drawSubViews`). So
`Window::draw` just delegates to `self.group.draw(ctx)` (the frame is a child and
draws as the back-most child; interior children draw over it). **Shadow casting is
still deferred** (the `// TODO(row 33)` in `group.rs` stays — do not implement it).

### Delegate the rest of `View` to the inner group
Exactly like `Desktop`: `state`/`state_mut`/`valid`/`awaken`/`calc_bounds`/
`change_bounds`/`cursor_request` → `self.group.*`. Override only `draw`,
`handle_event`, `set_state`, `size_limits` as above. Verify the full trait method
set in `view.rs`.

---

## Tests (verification — D11)
1. **ctor**: flags all-true; `zoom_rect == bounds`; `palette == Blue`; group state
   has `shadow`, `selectable`, `top_select`; growMode all four + rel; the frame was
   inserted (group has the frame child; `frame_id()` resolves) with title/flags/
   number pushed (render and check the title/border, or expose the frame to assert).
2. **getTitle/sizeLimits**: `title()` returns the set title; `size_limits(owner)`
   returns `min == (16,6)`.
3. **setState activation**: selecting the window (`set_state(Selected, true, ctx)`)
   makes the frame active (the frame draws the double-line active border / icons).
   A render/snapshot before vs after select is a strong check. Verify the frame’s
   `sfActive` flipped via the group propagation (no manual frame push).
4. **standard_scroll_bar**: vertical → inserted at the right edge rect
   `(w-1, 1, w, h-1)`; horizontal → bottom edge `(2, h-1, w-2, h)`;
   `handle_keyboard` → the scrollbar child has `post_process` set.
5. **kbTab** focus cycling among the window's selectable children (insert two
   selectable probe children + scrollbars; Tab moves `current`). Consumes the key.
6. **WindowFlags relocation**: the existing `frame.rs` tests still pass with
   `WindowFlags` imported from the window module; crate-root `tv::WindowFlags`
   still resolves.
7. **Mandatory snapshot**: a `Window` with a title + scrollbar, **selected**
   (active frame), rendered on a `HeadlessBackend` through `&mut dyn View` —
   showing the bordered active window with title, icons, and scrollbar. (And/or a
   window-on-desktop snapshot inserting the `Window` into a `Desktop`.)

Match the `desktop.rs`/`frame.rs` test harness idiom (DrawCtx/Renderer,
`with_ctx`).

## Definition of done (run; all pass)
- `cargo test` — all green (existing 251 + new).
- `cargo clippy --all-targets -- -D warnings` — clean (no dead fields: `frame_id`/
  `zoom_rect`/`flags`/`number`/`palette` kept live by accessors or ctor/test reads;
  if `zoom_rect`/`number` would be dead in 33b, add `zoom_rect()`/`number()`
  accessors rather than `#[allow]`).
- `cargo fmt --check` — clean.

## Deviations in play
- **D2** embed-and-delegate (`Window` embeds `Group`).
- **D3** owner-data-down: frame data pushed at ctor; no owner pointer. (zoom/drag's
  owner-extent need is 33c.)
- **D4** events carry no payload → `cmSelectWindowNum` window-number match deferred.
- **D5** `wf*` → `WindowFlags` (relocated here); `WindowPalette` enum for `palette`.
- **D7** no `getPalette`; `WindowPalette` + the single blue scheme via existing
  frame roles (multi-scheme → row 34).
- **D8** whole-tree redraw; shadow casting still deferred.

Report DONE / DONE_WITH_CONCERNS / NEEDS_CONTEXT / BLOCKED with what you built, any
faithfulness judgment calls (the frame-push-at-ctor approach, the `wnNoNumber`
mapping, the kbTab direction), and gate results. If something forces premature
33c infra (owner-extent, downcast, removal), STOP and report rather than building
it half-way.
