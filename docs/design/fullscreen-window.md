# Design note — frameless fullscreen windows (a chrome-less "app body" mode)

> Status: **DESIGN** (not yet landed; two independent expert reviews folded in). A
> modern-TUI extension *alongside* the faithful port (precedent: `RegexValidator`
> next to the picture-mask port). It reuses existing seams — `Window::zoom`'s
> saved-geometry model, the Frame push-down setters, and the post-dispatch
> `Deferred` channel — rather than inventing substrate.

## The idea

Modern TUIs often render their primary content with **no visible window frame**
(it reads as the background) and tuck the menu behind a `⋮` "kebab" affordance in
the top-right corner. We want a window to be drivable into one of three states:

- **`Off`** — a normal framed window (today's behaviour).
- **`Desktop`** (mode *a*) — the frame border disappears and the window fills the
  **desktop** area; the menubar and status line stay put.
- **`Screen`** (mode *b*) — as *Desktop*, but the window also covers the **menu
  row**, and the menubar (if any) collapses to a single `⋮` (U+22EE VERTICAL
  ELLIPSIS) at the top-right corner of the screen. The status line is untouched.

It is a **per-window property** (locked decision), so it rides on the same model
as zoom: a deep child of the desktop asking for a bigger box.

## Triggers (locked decisions)

- **API — the primary entry point.** `Window::set_fullscreen(Fullscreen)` sets the
  mode directly (e.g. straight to `Screen`). Apps use this.
- **Command — a convenience cycler.** `Command::FULLSCREEN`, handled in
  `Window::handle_event` when the window is active, cycles `Off → Desktop → Screen
  → Off` by reading the window's current `fullscreen` and calling `set_fullscreen`
  with the next state. Lives in `src/command.rs` with `ZOOM`/`CLOSE` (the shared
  command vocabulary; `command.rs:108` shows the `Command("tv.zoom")` shape). The
  cycle is a UX convenience — direct-to-mode is the API, not the command.
- **No default key binding** — apps bind their own key to the command.
- **Exit is the app's responsibility** — no built-in Esc escape hatch. A
  chrome-less window has no visible close box by design, and we don't want to fight
  apps that use Esc for their own content; the `⋮` menu is the natural exit home.

## Division of labour: what is inline vs. deferred

The transition splits along a hard architectural line discovered in review:

- **Inline in `Window::set_fullscreen` (it has `&mut self`):** push
  `set_border_visible(mode == Off)` to the frame via `self.group.child_mut(frame_id)`
  + downcast — the **only** path that reaches the Frame, identical to how
  `set_flags`/`set_palette`/`set_zoomed` push owner data (`window.rs:286–330`). Also
  set `self.fullscreen = mode`. **This must be inline:** `Window::as_any_mut` is
  delegate-generated to forward to the inner `Group` (`specs.rs:94–95`), so an
  external `find_mut(window_id).downcast_mut::<Window>()` returns `None` — the pump
  *cannot* reach the Frame or any `Window`-private field. Then emit the deferred op
  (below).
- **Deferred to the pump (cross-tree, needs post-resize sizes):** menubar
  collapse + bounds, desktop bounds, and the window re-fit. These touch siblings
  the borrow-stack forbids inline, and the window re-fit must happen *after* the
  desktop is re-bounded (else `owner_size` is stale). All of this is done through
  the **`View` trait** (`change_bounds` on `find_mut`) — **no downcast needed**, so
  the `as_any_mut` limitation doesn't bite.

## The one coordination primitive

```rust
// new arm on the existing `Deferred` enum (src/view/context.rs:66):
SetFullscreen { window: ViewId, mode: Fullscreen },
```

Carrying only the **window** id is consistent with `Deferred::UpdateMenu(ViewId)`
(`context.rs:197`), which likewise carries one id and lets the pump supply the
other participant from its own destructured state. The pump resolves the singleton
menubar/desktop from its own ids (see below).

## Loop-owned state (in `Program`/the pump)

```rust
fullscreen: Option<FullscreenSlot>,
struct FullscreenSlot { window: ViewId, mode: Fullscreen, restore: Rect }
```

Putting `restore` **here, not on `Window`**, is deliberate: the pump captures the
window's pre-fullscreen bounds via `find_mut(window).get_bounds()` (a `View`
method — no downcast) on the `Off → !Off` edge, and re-applies it via
`change_bounds` on the `!Off → Off` edge. So **`Window` needs no `restore_bounds`
field at all** — only `fullscreen: Fullscreen` so the cycler can read current
state. (`zoom_rect` at `window.rs:139` is untouched and independent.)

## Pump apply: `SetFullscreen { window, mode }`

Factored into a single function `apply_fullscreen(window, mode)` reused by both the
deferred drain **and** the resize arm (DRY — see Lifecycle). The pump already binds
`desktop: Option<ViewId>` in the drain destructure (`program.rs:1975`); it must
also **un-discard** `menu_bar` (currently `menu_bar: _` at `program.rs:1979`).
Sequential `find_mut` borrows in one arm are an established pattern
(`ClipboardEditorPaste`, `program.rs:2619–2633`). Steps:

1. **Edge bookkeeping:** if entering (`slot` was `None`/`Off`, `mode != Off`),
   capture `restore = find_mut(window).get_bounds()` into the slot. If exiting,
   read `restore` from the slot for step 4. Re-apply while already fullscreen
   (resize) must **not** recapture `restore`.
2. **Menubar:** `set_collapsed(mode == Screen)` **and** `change_bounds` it — full
   top row when not collapsed, the single `⋮` cell (`Rect::new(w-1, 0, w, 1)`) when
   collapsed. The bounds change is what makes hit-testing work (see MenuBar below).
   No-op if there is no menubar.
3. **Desktop bounds:** top = row 0 when `mode == Screen`, else row 1 — computed
   from the menu-bar height the pump already knows (layout knowledge stays in the
   pump, not in the `Deferred` enum). Apply via the desktop's `change_bounds`.
4. **Window bounds:** *after* the desktop has its final size — fit the window to the
   desktop's full extent for `Desktop`/`Screen`, or to `slot.restore` for `Off`.
   Same-drain ordering makes the owner size correct.
5. **Slot:** set `fullscreen = (mode != Off).then(|| FullscreenSlot { window, mode,
   restore })`.

(The frame border was already toggled inline in step 0, before the op was emitted.)

## `Window` changes

```rust
pub enum Fullscreen { Off, Desktop, Screen }   // closed set → enum (WindowPalette precedent, window.rs:87)

// Window gains exactly one field:
fullscreen: Fullscreen,
```

`set_fullscreen(mode)`: toggle frame `border_visible` (inline downcast), set
`self.fullscreen = mode`, **clear/restore the shadow flag** (see below), emit
`SetFullscreen { window: self.id(), mode }`. The `Command::FULLSCREEN` arm reads
`self.fullscreen`, computes the next state, and calls `set_fullscreen`.

**Window drag guard:** the row-0 / bottom-row drag-start in `handle_event`
(`window.rs:1275–1298`) must be **suppressed when `fullscreen != Off`** — otherwise
a frameless window starts a title-drag from content at row 0.

## `Frame` changes

```rust
border_visible: bool,           // default true; pushed down like set_zoomed
fn set_border_visible(&mut self, v: bool)
```

- **`draw`** guards the box-drawing + title + icons block (`frame.rs` ~335–432) on
  `border_visible`. **Interior fill stays unconditional and in its existing role**
  (`frame.rs:352–355`): that `border` role *is* the window-body background for
  Blue/Cyan/Gray windows in both bordered and frameless states — no new `Role` is
  needed (verified against the palette resolution; an earlier "swap to a content
  role" idea was a false alarm).
- **`handle_event`** (`frame.rs:458–514`) must guard its entire `MouseDown` arm on
  `border_visible`: when frameless, return immediately, arming **no** close/zoom
  capture. Otherwise the invisible close zone (cols 2–4, row 0) and zoom zone (cols
  w-5..w-3) silently fire `CLOSE`/`ZOOM` on a frameless window's content.

## `client_rect()` seam (content fills to edges — locked decision)

Add **inherent** `Window::client_rect()` → the frame-inset rect when bordered, the
**full bounds** when frameless. Inherent, **not** a `View` trait method, so it
needs **no `#[delegate]` forwarder** in `specs.rs`, and no `&dyn View` consumer
needs it (`standard_scroll_bar` is inherent same-impl, `window.rs:477`). That
method is rewritten to key off `client_rect()` so a frameless window's
scrollbars/content reach the screen edge — the "becomes the background" look.

## `MenuBar` collapse

`MenuBar` gains `collapsed: bool` + `set_collapsed`. Collapse is driven by the
**pump** via `set_collapsed` **plus a `change_bounds`** to the `⋮` cell (step 2
above) — **not** by draw-transparency. This is the key correction from review:
RSTV's `Group` has **no event bubbling** for positional events (`group.rs:25–28`,
`1452–1487`) — it hit-tests one topmost child and delivers once, so a "drawn
transparent but full-width" menubar would still swallow every row-0 click. By
**shrinking the menubar's bounds** to the `⋮` cell, the root group's hit-test
routes the rest of row 0 to the (expanded) desktop, and thus to the fullscreen
window, with no special passthrough logic.

- **Draw:** when collapsed, paint only `⋮` at `(w-1, 0)`.
- **Activation:** a click on the `⋮` cell, or F10 / a menu hotkey (keyboard events
  reach the menubar regardless of its width — they aren't positional), activates
  the bar. **While active it temporarily reclaims full-width bounds** (a deferred
  `change_bounds` driven off its active-state edge) so existing menu rendering +
  dropdowns work unchanged, then shrinks back to the `⋮` cell on close. Menu
  *navigation* logic is untouched; only the bar's bounds track collapsed/active
  state. This active-bounds toggle is the trickiest seam — call it out in the plan.

If there is **no menubar**, `Screen` mode simply covers the top row with no `⋮`.

## Lifecycle & edge cases

- **Resize:** the terminal-resize arm (`program.rs:1995–2003`, which already
  applies layout **inline** via `change_bounds`) calls `apply_fullscreen(window,
  slot.mode)` for the tracked slot — the same function the drain uses (DRY).
  Tracking `mode` in the slot is required (the desktop top differs by mode). The
  re-apply does **not** recapture `restore`.
- **Close / removal:** the `Command::CLOSE` arm (`window.rs:1180–1195`) calls
  `set_fullscreen(Off)` before `request_close`, restoring chrome. **But** a
  programmatic `group.remove_descendant(window_id, …)` bypasses the `CLOSE`
  handler. Mitigation: each pump pass, if `fullscreen` is `Some(slot)` and
  `find_mut(slot.window)` no longer resolves, the pump auto-restores chrome
  (un-collapse + re-bound the menubar, restore desktop bounds) and clears the slot.
  Robust against every removal path.
- **Shadow:** `Window::new` sets `shadow = true` (`window.rs:185`); a fullscreen
  window's shadow is clipped to invisibility but should be cleared for cleanliness.
  `set_fullscreen` clears it on `Off → !Off` and restores it on `!Off → Off`.
- **Other windows / modal dialogs** float on top of a fullscreen window normally —
  it is just a non-modal background window. Mode (a) gives the "frameless app body
  with dialogs over it" look with no app-level mode.
- **Currency/focus:** a fullscreen window participates in desktop window cycling
  like any other; covering the `background` child is purely visual.

## Testing (D11 snapshots, `insta`)

Build on `HeadlessBackend`; eyeball whole snapshots, include non-zero-origin /
resize cases (per the snapshot-at-origin lesson):

1. **Mode a** — frameless window filling the desktop: no border, the window-body
   background shows, content reaches edges; a normal dialog floats on top framed.
2. **Mode b** — a `Screen` window covering row 0, `⋮` at top-right, status line
   intact.
3. **Collapsed-menubar hit routing** — a `MouseDown` off the `⋮` cell reaches the
   fullscreen window (because the menubar bounds are the `⋮` cell only); a click on
   `⋮` (and F10) activates the bar, which reclaims width then re-collapses.
4. **Frameless hotspots are dead** — a click at the old close/zoom/title-drag
   coordinates does nothing (no `CLOSE`/`ZOOM`/drag).
5. **Round-trip** — `Off → Screen → Off` returns to the exact captured `restore`
   bounds, border + menubar + shadow restored.
6. **Resize while fullscreen** — `Screen` window still fills row 0..bottom-1 after
   a terminal resize; `restore` not clobbered.
7. **Removal restore** — removing a `Screen` window via `remove_descendant`
   restores menubar/desktop (the pump vanish check).
8. **No-menubar app** in `Screen` mode — top row covered, no `⋮`.

## What this is *not*

- Not a status-line change (only the menu is covered).
- Not window reparenting — the window stays a child of the desktop; the **desktop**
  grows to expose the menu row.
- Not a new `Context::new` parameter — it **adds one `Deferred` variant**
  (`SetFullscreen`), per the deferred-effects rule.
- Not a new `Role` — the existing frame interior-fill role is the window body.
- Not an app-level singleton "main view" — it is a per-window property.
- Not a `View`-trait addition — `client_rect()` is inherent; the `#[delegate]`
  forwarder list is untouched.
- Not draw-transparency passthrough — collapse is a real bounds change, because
  the `Group` does not bubble positional events.
