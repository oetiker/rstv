# Brief — Rows 50/51/52 menu **MODAL layer** (Step 2): `execute()` navigation + submenu recursion + `TMenuPopup`

> Status: **DESIGN SETTLED (advisor-vetted), implementation staged.** The draw/data
> slice (50/51) already landed (`0687530`). This brief is the *modal* slice: the
> interactive `TMenuView::execute()` state machine, flattened onto our single
> event loop. C++ source of truth: `source/tvision/tmnuview.cpp` (`execute`,
> `trackMouse`, `trackKey`, `nextItem`, `prevItem`, `mouseInOwner`, `mouseInMenus`,
> `topMenu`, `findItem`, `findAltShortcut`, `do_a_select`, `handleEvent`),
> `source/tvision/tmenupop.cpp` (`TMenuPopup`).

## The architecture decision (settled — do NOT relitigate the rejected options)

The core challenge: C++ `TMenuView::execute()` (`tmnuview.cpp:179`) is a nested
`getEvent` loop that opens submenus by **recursively** calling
`owner->execView(target)` (one nested modal loop per open box level). We have a
**single** event loop (D9). Two candidate mappings were weighed:

### REJECTED — "re-entrant `exec_view` per level" (Arch 1)
Map each open level to a nested `exec_view` (the row-34 `ModalFrame` + pump loop),
opening a submenu = re-entrant `exec_view(box)`. **Rejected for two code-grounded
reasons:**

1. **The guide does not license it.** The D9 note "`exec_view` = the
   `TGroup::execute` shape" ratifies `exec_view`/`OpenModal` for **`TGroup::execute`**
   (`tgroup.cpp:173`, the *dialog* modal loop). `execView(p)` calls `p->execute()`
   **virtually** (`tgroup.cpp:205`); for a menu, `p` is the menu view, which runs
   the **overridden** `TMenuView::execute` (`menus.h:152`, a hand-rolled switch) —
   a *different function* from `TGroup::execute`. So `OpenModal` was ratified for
   "a menu *command* launches a dialog," never for menu *navigation*. (TMenuBox
   has **no** `execute` override; it runs `TMenuView::execute` with `size.y != 1`.
   Only `TMenuPopup` overrides it — `menus.h:380`, row 52.)
2. **Mouse semantics break it.** `ModalFrame` **swallows** outside-bounds mouse
   (`program.rs` `ModalFrame::handle` → `else => Consumed`); menus must **close**
   on an outside click (`execute()` evMouseDown else-branch:
   `if(putClickEventOnExit) putEvent(e); action = doReturn`). And N stacked
   `ModalFrame`s each gate only their own box, but C++ menu mouse nav flows across
   the **whole** open stack (`mouseInMenus` walks the `parentMenu` chain;
   `mouseInOwner` lets a click on the *parent's* item through). Per-level
   bounds-gating cannot express cross-level mouse.

### CHOSEN — one **`MenuSession`** capture handler owning the whole open stack
A single capture handler (pushed at activation, popped when the last level closes)
is the flattened `execute()` for the *entire* interaction (bar + every open box).
It owns all open-level `ViewId`s + bounds, so cross-level mouse gating and
click-outside-close live in **one** place. It drives nav/open/close via **deferred
tree ops**. `OpenModal`/`exec_view` stays reserved for the menu-command→dialog case
(msgbox / Batch E). This is the HANDOVER's instinct, refined: **one** session
object, not one capture frame per level.

> Both designs turn the `do{getEvent;switch}while` loop inside-out (our
> `pump_once` delivers one event per call) — "faithful to the C++ loop" is **not** a
> differentiator. The deciding constraint is mouse + cross-level routing, which only
> the single-session object expresses. **Pick/verify every mechanism against the
> mouse + cross-level cases, not the single-box-keyboard case** — even though the
> first implementation cut is keyboard-only (below).

## Where state lives — and **who handles events** (the decision that sizes every component)

**Clean Architecture A: while a session is active, the `MenuSession` capture
handler owns EVERY event.** The boxes are *never* focused; no focus moves to them.
This is the line that decides how much code each component gets — commit to it
before coding:

- **Idle menu bar** → the row-49 passive `menu_view::handle_event` runs
  (accelerators + command-graying broker). It *also* detects activation
  (`cmMenu` / alt-shortcut / `evMouseDown`) and pushes the session. **The bar's
  `handle_event` keeps ONLY the activation branches** (the breadcrumbed `_ => {}`
  arm); nothing else changes.
- **Active session** → the `MenuSession` capture handler runs first and **consumes**
  all menu-directed events, runs the flattened `execute()` SM, and the bar + boxes
  are **pure `draw`/`get_item_rect`** (already built). **Boxes get NO event logic.**
  Their `current` is a **write-only display cache** the session pushes. On the last
  close, the session restores the pre-menu focus.

- **The `MenuSession` owns the `execute()` state machine** — all loop-locals live
  here (per-level `current`, `autoSelect`, `lastTargetItem`, `mouseActive`,
  `firstEvent`, `itemShown`). It is the flattened `execute()`.
- **Per-level state, shaped for mouse from day one:** the session holds a stack of
  levels, each `{ view_id, menu: Menu, current: Option<usize>, bounds: Rect,
  is_bar: bool }`. **Bounds are cached at open** and never resynced — a menu box is
  placed once and closed, it **never moves** (unlike a dragged dialog, the case the
  guide's `sync_gate_bounds` addressed; `sync_gate_bounds` resolves one `view()` id
  per handler and can't serve a multi-box session anyway). Stage-2 mouse gates
  against this cached per-level bounds set, so shape the struct to hold it **now**.
- **Menu data — clone-at-open is FAITHFUL, not a compromise.** `execute()`'s switch
  (`tmnuview.cpp:199-354`) has **no `evBroadcast` case**, so a `cmCommandSetChanged`
  arriving during the modal loop is fetched and **ignored** — C++'s `disabled` is
  *frozen for the menu's lifetime*. So: the session **clones the submenu subtree at
  open** (`Menu: Clone`, `disabled` correct from the live parent the broker keeps
  current) and **swallows `evBroadcast` while active** to match C++ exactly (this
  also keeps boxes from regraying mid-menu, so SM and draw never diverge).
  `nextItem`/`prevItem` skip **separators only** (`name==0`), never disabled.
- **Highlight sync:** when the session changes a level's `current`, it writes it to
  that view for `draw` via a **new `Deferred::SetMenuCurrent(ViewId, Option<usize>)`**.
- **Opening a box — pre-mint the id (preferred), downcast fallback.** Add a **new
  `Deferred::OpenMenuBox { id: ViewId, menu: Menu, bounds: Rect }`**. The session
  **pre-mints the `ViewId`** from the global counter so it already knows every box
  id with no callback; the pump builds a `MenuBox` and does an **insert-with-id**
  into the root group (NO focus move). *Verify `Group` has/can-get a clean
  insert-with-given-id path* — if it genuinely can't, fall back to minting in the
  pump + an `as_any_mut` downcast of the top capture handler to register `(id,
  bounds)` (the scroller broker already downcasts). Geometry comes from the issuer's
  `get_item_rect(current)` + origin (`execute()` `r.a.x/r.a.y/r.b`, `tmnuview.cpp:376-381`).
- **Closing a box / the whole session:** reuse `Deferred::Close(id)` per box; the
  session pops itself (`CaptureFlow::ConsumedPop`, or a pump-side pop) when the last
  level closes and restores the pre-menu focus (the `exec_view` save/restore precedent).
- **Re-posting events** (`execute()` `putEvent(e)` on submenu-open + outside-click
  exit): add a small **`ctx.put_event(Event)`** seam, the raw-event sibling of
  `ctx.post(cmd)`.
- **No dead first event:** push the session **and** the first `OpenMenuBox` in the
  **same activation deferred batch** (not on the session's first `handle()`), else
  the menu doesn't appear until a second key.

## Activation wiring (currently breadcrumbed in `menu_view::handle_event`)
The passive layer's `_ => {}` arm leaves three activation branches for here
(`tmnuview.cpp:518 handleEvent` + `do_a_select`):
- **`evMouseDown`** on the bar → open the session at the clicked top item.
- **`evCommand cmMenu`** (kbF10 → `cmMenu`, `tprogram.cpp:275`) → open at the default.
- **`evKeyDown` alt-shortcut** (`findAltShortcut`) → open at the matched item.
`do_a_select` (`tmnuview.cpp:505`): re-post the triggering event so the session
sees it, push the `MenuSession`, then the session's first dispatch runs `execute()`'s
prologue (`current = menu->deflt`) + opens the first box.

## Staging (keyboard-first, mouse-anticipating)

The architecture is chosen for mouse; the first **implementation** cut may be
keyboard-only, but the `MenuSession` struct + deferred variants must be shaped so
mouse drops in without a rewrite (the session already holds all bounds).

1. **Substrate + keyboard nav (this row's first deliverable):** `MenuSession`
   capture handler; `Deferred::OpenMenuBox` + `SetMenuCurrent`; `ctx.put_event`;
   activation via `cmMenu` + alt-shortcut; `execute()` keyboard arms (kbUp/kbDown/
   kbLeft/kbRight/kbHome/kbEnd/kbEnter/kbEsc + `trackKey`/`nextItem`/`prevItem` +
   char `findItem`); submenu recursion (nested `OpenMenuBox`); command post + close.
   **Verify end-to-end through `pump_once`:** bar `cmMenu` → box opens → arrows
   move highlight → Enter posts the command → all boxes close, focus restored.
2. **Mouse** (`trackMouse`/`mouseInOwner`/`mouseInMenus`/`autoSelect`/
   `lastTargetItem` + click-outside-close + `evMouseDown`/`Up`/`Move` arms).
3. **`TMenuPopup` (52)** = `TMenuBox` + `execute`/`handleEvent` overrides
   (`menu->deflt = 0`; `putClickEventOnExit = False`; ctrl-char + hotKey dispatch)
   + the `popupMenu()` free fn (`tmenupop.cpp`).
4. **Wire a real menu bar into `Program`** — first emitter of `cmTile`/`cmCascade`/
   `cmDosShell` → wire the row-32 breadcrumb in `program_handle_event` + build
   `Desktop::tile`/`cascade` geometry; revisit the modal-isolation breadcrumb
   (suppress program-level interception while the session is active). Close the
   **initial-regray gap** (row-49 carry): trigger an initial `Deferred::UpdateMenu`
   on menu-bar insert (or have `Program` broadcast `cmCommandSetChanged` once at
   startup), else a startup-disabled command draws enabled until the first broadcast.

## Verification
Snapshot tests for any new draw states; **`pump_once` integration tests** are the
real proof (the row-49 `MenuProbe` precedent): drive pre-queued events through the
real pump and assert the open-stack, highlight, posted command, and focus
restoration. Bite-check each (remove the fix → test fails). Two-stage review
(SPEC then QUALITY, fresh C++-adversarial Opus agents against the **C++ + guide**,
not this brief).
