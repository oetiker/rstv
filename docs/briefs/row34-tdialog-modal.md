# Implementer brief — Row 34: `TDialog` + the modal `exec_view` lifecycle

You are porting magiblot/tvision behavior to idiomatic Rust in the `tvision`
crate (house alias `tv::`). This is **row 34 (`TDialog`)** — *the modality
payoff*: the first thing you can `exec_view` and have it return `cmOK`/`cmCancel`.
Port **faithfully** from the C++; the only departures are the pre-decided
deviations named below. Do **not** invent features, and do **not** build the
deferred items (no dead stubs).

This is a single **FOUNDATION** stage on the main thread — **no worktree**. It
touches/creates:

- **NEW** `src/dialog/mod.rs` + `src/dialog/dialog.rs` — the `Dialog` type
  (`TDialog`). (Mirror the `window` module layout: `mod.rs` has the module doc +
  `pub use`, the impl lives in `dialog.rs`.)
- `src/app/program.rs` — add `Program::exec_view`; apply the new `Deferred::EndModal`.
- `src/view/context.rs` — add `Deferred::EndModal(Command)` + `Context::end_modal`.
- `src/capture.rs` — add `CaptureStack::pop`.
- `src/lib.rs` — `pub mod dialog;` + `pub use dialog::Dialog;` (orchestrator may
  do the module wiring; do it yourself and the orchestrator will reconcile).

Build verification every step: `cargo test`, `cargo clippy --all-targets -- -D
warnings`, `cargo fmt --check`. Add the tests described in §5.

---

## 0. The C++ you are porting (read it, it is short)

`source/tvision/tdialog.cpp` — `TDialog::TDialog`, `getPalette`, `handleEvent`,
`valid`. `source/tvision/tgroup.cpp` — `TGroup::execView`, `execute`, `endModal`,
`valid`. Reproduced for convenience:

```cpp
TDialog::TDialog( const TRect& bounds, TStringView aTitle ) :
    TWindowInit( &TDialog::initFrame ),
    TWindow( bounds, aTitle, wnNoNumber )       // number = 0 → no number
{
   growMode = 0;                                // dialogs do NOT grow with the owner
   flags = wfMove | wfClose;                    // NOT wfGrow, NOT wfZoom
   palette = dpGrayDialog;                       // gray scheme (theming DEFERRED, see §4)
}

void TDialog::handleEvent(TEvent& event)
{
    TWindow::handleEvent(event);                 // delegate FIRST (faithful order)
    switch (event.what) {
        case evKeyDown:
            switch (event.keyDown.keyCode) {
                case kbEsc:                       // Esc → post cmCancel, clear
                    event.what = evCommand; event.message.command = cmCancel;
                    event.message.infoPtr = 0; putEvent(event); clearEvent(event); break;
                case kbEnter:                     // Enter → broadcast cmDefault, clear
                    event.what = evBroadcast; event.message.command = cmDefault;
                    event.message.infoPtr = 0; putEvent(event); clearEvent(event); break;
            }
            break;
        case evCommand:
            switch( event.message.command ) {
                case cmOK: case cmCancel: case cmYes: case cmNo:
                    if( (state & sfModal) != 0 ) { endModal(event.message.command); clearEvent(event); }
                    break;
            }
            break;
    }
}

Boolean TDialog::valid( ushort command )
{
    if( command == cmCancel ) return True;       // cancelling is ALWAYS valid
    else return TGroup::valid( command );
}
```

```cpp
ushort TGroup::execView( TView* p ) {
    if( p == 0 ) return cmCancel;
    ushort saveOptions = p->options;
    TGroup *saveOwner   = p->owner;
    TView  *saveTopView = TheTopView;
    TView  *saveCurrent = current;
    TCommandSet saveCommands; getCommands( saveCommands );
    TheTopView = p;
    p->options &= ~ofSelectable;                 // a modal view is not tab-selectable among siblings
    p->setState(sfModal, True);
    setCurrent(p, enterSelect);
    if( saveOwner == 0 ) insert(p);              // insert iff not already owned
    ushort retval = p->execute();                // ← the nested modal loop
    if( saveOwner == 0 ) remove(p);
    setCurrent(saveCurrent, leaveSelect);
    p->setState(sfModal, False);
    p->options = saveOptions;
    TheTopView = saveTopView;
    setCommands(saveCommands);
    return retval;
}

ushort TGroup::execute() {                        // the modal loop (we drive pump_once instead)
    do { endState = 0;
         do { TEvent e; getEvent(e); handleEvent(e); if(e.what!=evNothing) eventError(e); }
         while( endState == 0 );
       } while( !valid(endState) );
    return endState;
}
```

---

## 1. Mental model — read this first

**`Dialog` embeds `Window`** exactly as `Window` embeds `Group` (the D2
embed-and-delegate pattern, one level deeper). `Dialog { window: Window }`;
`state`/`state_mut`/`draw`/`size_limits`/`change_bounds`/`cursor_request`/
`find_mut`/`remove_descendant`/`number` all delegate to `self.window`. Its own
behavior is `handle_event` (delegate to `Window::handle_event` first, then the
dialog keys/commands), `valid` (the `cmCancel` veto), and ctor field overrides
(`growMode = 0`, `flags = wfMove|wfClose`, `palette = Gray`).

**The modal loop crux (the FOUNDATION decision — guide D9 "`exec_view` —
corrected").** Under D9 there is **one** non-recursive `pump_once`. `exec_view`
does not nest a *re-entrant* loop; it **drives** `pump_once` in a bounded loop at
the **top level**:

```text
loop { end_state = None; while end_state.is_none() { pump_once(); }
       if valid(end_state) { break } }
```

This is sound because `pump_once` releases the whole view-tree `&mut` borrow
stack between iterations, and **a `View` holds only `&mut Context`, never `&mut
Program`** — so a view *cannot* call `exec_view` from inside `handle_event`. The
compiler enforces that the sync loop only runs at the top level (an app's `main`,
startup, or a test driving pre-queued events). View-/menu-triggered modals (Phase
4) use a different, async path (a `Deferred::OpenModal` + posted completion
command) — **out of scope for row 34**; just leave the guide's breadcrumb intact.

**`endModal` is downward (D3).** `Dialog::handle_event` cannot reach
`Program::end_modal` (it has `&mut Context`). It signals via a new
`Deferred::EndModal(Command)` (`ctx.end_modal(cmd)`); the pump applies it by
setting `Program::end_state`. The nested `exec_view` loop then observes it. This
is the unified-`Deferred`-channel rule (`69897fe`): **a new deferred capability
adds a `Deferred` variant, not a `Context::new` param.**

---

## 2. `Deferred::EndModal` + `Context::end_modal` (do this first)

In `src/view/context.rs`:

- Add a variant to `enum Deferred`:
  ```rust
  /// Request the (modal) loop end with `command` (`TGroup::endModal`). The pump
  /// applies it by setting `Program::end_state`; the nested `exec_view` loop then
  /// observes it. The downward (D3) replacement for a view calling `endModal` up
  /// its owner chain.
  EndModal(Command),
  ```
  It joins the existing families; it touches **loop state** (`end_state`), a
  fourth disjoint target alongside capture-stack / command-set / view-tree — so
  insertion-order draining is still order-equivalent (state it in the doc, the
  reviewer will check the `69897fe` ordering invariant).
- Add `Context::end_modal`:
  ```rust
  /// Request the (modal) loop end with `cmd` — **deferred** ([`Deferred::EndModal`]).
  /// `TGroup::endModal` from a view with no up-pointer to the program (D3).
  pub fn end_modal(&mut self, cmd: Command) {
      self.deferred.push(Deferred::EndModal(cmd));
  }
  ```

In `src/app/program.rs`, in the `pump_once` deferred-drain `match`, add the arm:
```rust
Deferred::EndModal(cmd) => { *end_state = Some(cmd); }
```
(`end_state` is already in the destructure — it is `&mut Option<Command>`.)

---

## 3. `CaptureStack::pop` + `Program::exec_view`

### 3a. `CaptureStack::pop` (`src/capture.rs`)
```rust
/// Remove and return the top handler, if any. Used by `Program::exec_view` to
/// remove the `ModalFrame` it pushed once the modal loop ends — the **one** place
/// a frame is popped other than a handler self-popping via `ConsumedPop`. (The
/// loop owns the stack; a handler cannot reach it to do a `valid(end_state)`-
/// conditional pop, so the owner-side `exec_view` does it.)
pub fn pop(&mut self) -> Option<Box<dyn CaptureHandler>> { self.handlers.pop() }
```
(Match the field name in `capture.rs` — likely `handlers: Vec<...>`.)

### 3b. `Program::exec_view` (`src/app/program.rs`)

Port `TGroup::execView` + `execute` faithfully. Signature:
```rust
/// `TGroup::execView` (run on `TProgram`, the owner group) — insert a view modally,
/// run the loop until it validates an end command, and return that command.
///
/// **Top-level only — the type system enforces it:** a `View` holds only `&mut
/// Context`, never `&mut Program`, so a view cannot call this from inside
/// `handle_event` (which is what makes the nested `pump_once` loop sound — D9
/// "exec_view — corrected"). Call from an app `main`, startup, or a test driving
/// pre-queued events.
///
/// **HEADLESS HANG WARNING:** `pump_once` does not block on a headless backend, so
/// the inner `while end_state.is_none()` loop spins until something sets
/// `end_state`. The caller MUST ensure the modal reaches `end_modal` (e.g. a
/// pre-queued `cmOK`/`cmCancel`, or an Esc that posts `cmCancel`). A modal with no
/// path to `end_modal` hangs.
pub fn exec_view(&mut self, view: Box<dyn View>) -> Command { ... }
```

Faithful body (adapt names to the codebase; use the `Program { .. }`-destructure
discipline only where you need disjoint borrows — most of this is sequential
`&mut self` calls so a destructure may be unnecessary):

1. **Save** `saveCurrent = self.group.current()` and `saveCommands =
   self.command_set.clone()` (the `getCommands`/`setCommands` dance — `CommandSet`
   should be `Clone`; if not, derive it).
2. **Insert** the view into the root group: `let id = self.group.insert(view);`
   (`saveOwner == 0` is always true here — we always own it; the "already owned"
   branch has no caller at row 34, so always insert + always remove). Capture its
   **bounds** *before/after* insert for the `ModalFrame` (`group.find_mut(id)` →
   `state().get_bounds()`; the frame needs the screen-frame rect — for row 31 the
   root group is at `(0,0)` so group-local == absolute, the same `ModalFrame`
   coordinate caveat).
3. Clear `ofSelectable` on the view (`find_mut(id)` → `state_mut().options.selectable
   = false`) — `p->options &= ~ofSelectable`. (Save the prior value to restore;
   for a `Dialog` it is already false-effectively but be faithful.)
4. **`set_state(sfModal, true)`** on the view via a `Context` (you need a ctx; build
   it over the destructured fields the way the `#[cfg(test)] with_ctx` helper does,
   or factor a small private `fn make_ctx(&mut self)`-style helper that respects the
   disjoint-borrow rule). Then **`set_current(Some(id), SelectMode::Enter, ctx)`** —
   C++ `enterSelect` → `SelectMode::Enter` (verified to exist in `group.rs`;
   `leaveSelect` → `SelectMode::Leave` for the restore in step 8). Do NOT invent
   variants.
5. **Push the `ModalFrame` directly** onto `self.captures` (NOT deferred — you hold
   `&mut self`, you are not inside a dispatch): `self.captures.push(Box::new(
   ModalFrame::new(id, bounds)));`. Record the stack depth or rely on it being top.
6. **The loop** (`TGroup::execute`):
   ```rust
   let retval = loop {
       self.end_state = None;
       while self.end_state.is_none() { self.pump_once(); }
       let es = self.end_state.unwrap();
       // TGroup::execute's outer `while(!valid(endState))`. `valid` is VIRTUAL on
       // the modal view `p` (= TDialog::valid), scoped to the DIALOG's own
       // children — NOT the root group.
       let valid = self.group.find_mut(id).map(|v| v.valid(es)).unwrap_or(true);
       if valid { break es; }
   };
   ```
   **CRITICAL scope (corrected post-review — the original brief was WRONG here).**
   C++ `execView` does `p->execute()` (`tgroup.cpp:205`); `execute`'s
   `while(!valid(endState))` (`tgroup.cpp:184`) calls the **virtual `valid` on
   `p` = the modal dialog** → `TDialog::valid` (cmCancel→true, else the dialog's
   OWN children). Do **NOT** use `self.group.valid(es)` (the program's `valid_end`)
   here: the root group ANDs the desktop sibling (`children.iter().all(...)`), a
   scope C++ never uses — and a latent **hang** if a sibling ever vetoed (the outer
   loop re-spins forever with `end_state = None` and nothing re-issuing the
   command). Resolve the modal's own `valid` via `find_mut(id)` (the id still
   resolves — `remove` happens after this loop). `valid_end` stays for `run()`'s
   own modal loop, untouched.
7. **Pop the `ModalFrame`:** `self.captures.pop();` (it is on top — drags self-pop
   on MouseUp, so nothing unbalanced remains when `end_state` is set).
8. **`remove`** the view: resolve + `self.group.remove(id, ctx)` (faithful
   `saveOwner == 0` → remove). Restore: `set_current(saveCurrent, leaveSelect)`,
   the removed view is gone so no `set_state(sfModal,false)`/options-restore on it
   is observable — **but** be faithful where the object still exists: C++ clears
   sfModal & restores options on `p` *after* remove (p still exists as a local). In
   our port the view is consumed by `insert`/owned by the group; after `remove` we
   no longer hold it. Document that the post-remove `setState(sfModal,False)` /
   options-restore are **moot** (the view is dropped on close) and therefore not
   ported — *unless* you keep the removed `Box` from `remove`'s return. Match
   whatever `Group::remove` returns; do not change its signature for this.
9. **Restore commands:** `self.command_set = saveCommands;` (setCommands). Note: do
   NOT set `command_set_changed` here — restoring is not an app-visible toggle the
   way enable/disable is (or set it if you judge a re-broadcast is faithful; state
   your reasoning).
10. **`TheTopView`** (`saveTopView`/`TheTopView = p`): a global "the modal-most
    view" used by `TView::exposed`/draw-clipping under occlusion. **DROP it with a
    breadcrumb** — occlusion/`sfExposed` is dropped (D8, whole-tree redraw + diff),
    so `TheTopView` has no consumer. Comment: `// TheTopView dropped (D8: no
    occlusion/exposed); no consumer.`
11. `return retval;`

> If `exec_view`'s borrow choreography around building a `Context` mid-method gets
> awkward, factor the ctx-needing steps into small free functions taking explicit
> field borrows (the `program_handle_event` pattern already in the file), or a
> private helper. Do not fight the borrow checker with `RefCell`.

---

## 4. `Dialog` (`src/dialog/dialog.rs` + `mod.rs`)

```rust
pub struct Dialog { window: Window }
```

**Ctor** `Dialog::new(bounds: Rect, title: Option<String>) -> Self`:
- `let mut window = Window::new(bounds, title, 0);` (`wnNoNumber == 0` → no number).
- Override the window's fields to the dialog defaults. `Window` currently exposes
  `flags`/`palette` via getters only. **You will need setters** (or a dedicated
  `Window` ctor hook) — add `Window::set_flags(WindowFlags)` and
  `Window::set_palette(WindowPalette)` (and `set_grow_mode` or mutate
  `state_mut().grow_mode`) as minimal pub(crate) methods, mirroring the existing
  `set_*` push-down style. **The flags must also be pushed to the frame** (the
  window ctor pushes `flags` to the frame via `Frame::set_flags`; if you change the
  window's flags after ctor you must re-push to the frame, else the frame draws a
  zoom icon the dialog should not have). Cleanest: give `Window` a
  `set_flags` that **also re-pushes to the frame child** (resolve `frame_id` →
  downcast → `Frame::set_flags`), so `Dialog` calls one method. Verify with a
  snapshot that the dialog frame shows **no zoom icon** and **no number**.
  - `flags = WindowFlags { move: true, close: true, ..default() }` (NOT grow/zoom).
  - `state_mut().grow_mode = GrowMode::default()` (all false — `growMode = 0`; a
    dialog does not track owner resize).
  - `palette = WindowPalette::Gray`.

**`impl View for Dialog`** — delegate everything to `self.window` EXCEPT
`handle_event` and `valid`:
- `state`/`state_mut`/`draw`/`awaken`/`size_limits`/`calc_bounds` (delegate? —
  `Window` does NOT delegate `calc_bounds`, see its note; `Dialog` should delegate
  to `Window`'s so the 16×6 floor applies — i.e. **do not** override `calc_bounds`
  on `Dialog`, let the trait default route through `Dialog::size_limits` →
  `Window::size_limits`. Mirror `Window`'s reasoning.) /`change_bounds`/
  `cursor_request`/`set_state`/`find_mut`/`remove_descendant`/`number` → delegate.
- **`handle_event`** (port `TDialog::handleEvent`):
  ```rust
  fn handle_event(&mut self, ev: &mut Event, ctx: &mut Context) {
      self.window.handle_event(ev, ctx);     // TWindow::handleEvent FIRST
      match *ev {
          Event::KeyDown(k) if k.key == Key::Esc => {
              // kbEsc → post cmCancel, clear. (putEvent = ctx.post.)
              ev.clear(); ctx.post(Command::CANCEL);
          }
          Event::KeyDown(k) if k.key == Key::Enter => {
              // kbEnter → broadcast cmDefault, clear. source = None (no subject view).
              ev.clear(); ctx.broadcast(Command::DEFAULT, None);
          }
          Event::Command(c) if matches!(c, Command::OK | Command::CANCEL | Command::YES | Command::NO) => {
              if self.window.state().state.modal {
                  ctx.end_modal(c);          // endModal(command)
                  ev.clear();
              }
          }
          _ => {}
      }
  }
  ```
  - Check the exact `Key` variant names for Esc/Enter (`src/event/key.rs`). Use the
    real variants; do not assume.
  - **Order/guard nuance:** C++ clears the event *then* puts the new one. Posting
    (`ctx.post`) enqueues for a later pump, so clearing first then posting is
    equivalent. The cmOK/Cancel/Yes/No arm only fires if the window delegation left
    the command live AND `sfModal`.
- **`valid`** (port `TDialog::valid`):
  ```rust
  fn valid(&self, cmd: Command) -> bool {
      if cmd == Command::CANCEL { true } else { self.window.valid(cmd) }
  }
  ```

**`mod.rs`:** module doc (D2 embed-and-delegate one level deeper; the gray-theming
/ getData / message deferrals from §6) + `pub use dialog::Dialog;`.

**Confirm `Command::DEFAULT`, `OK`, `CANCEL`, `YES`, `NO` all exist** in
`src/command.rs` (they are in `default_command_set`, so OK/CANCEL/YES/NO/DEFAULT
should). If `DEFAULT` is missing, add it as a shared-vocabulary const faithfully.

---

## 5. Tests (hang-safe — see the warning in §3b)

Put unit tests for `Dialog::handle_event`/`valid` in `dialog.rs`. Put the
`exec_view` lifecycle + round-trip tests in `program.rs` (it owns `exec_view` and
the headless harness `program_with_desktop`). **Every pump-driven test must reach
`end_modal` within 1–2 pumps or it hangs CI** — pre-queue the exact events.

Mandatory tests:

1. **`Dialog` ctor** — `flags == {move,close}` (no grow/zoom), `grow_mode` all
   false, `palette == Gray`, `number()` is `None`. Snapshot of a drawn dialog
   showing the frame with **no zoom icon and no number** (the flags-pushed-to-frame
   check). (Inherited blue frame is fine — gray is deferred §6.)
2. **Esc posts cmCancel** — feed an Esc `KeyDown` to `Dialog::handle_event`; assert
   `ev` cleared and `Command::CANCEL` enqueued in `out`.
3. **Enter broadcasts cmDefault** — assert `ev` cleared and a
   `Broadcast{command: DEFAULT, source: None}` enqueued.
4. **cmOK/cmCancel end the modal iff sfModal** — with `sfModal` set, an
   `Event::Command(OK)` → `Deferred::EndModal(OK)` queued + `ev` cleared; **without**
   `sfModal`, the same command is left live (not consumed, no EndModal). Make this
   discriminating (assert the no-modal case does NOT queue EndModal).
5. **`valid` veto** — `valid(cmCancel) == true` even when a child would be invalid;
   `valid(other)` delegates to the group. (Insert an always-invalid probe child to
   prove `cmCancel` bypasses it and another command does not.)
6. **`exec_view` full round-trip (the FOUNDATION gate).** Build a `program_with_
   desktop`. Pre-queue an event that makes the dialog reach `end_modal` fast — the
   cleanest is to **pre-queue `Event::Command(Command::OK)`** and rely on it routing
   to the modal dialog (current, sfModal) → `end_modal(OK)`. Call
   `program.exec_view(Box::new(dialog))` and assert it **returns `Command::OK`**.
   Then assert post-conditions: the dialog was **removed** from the group
   (`find_mut(id)` is `None` — but you don't have `id`; instead assert
   `capture_len() == 0` (frame popped) and the group child count returned to its
   pre-exec value), and `current` was restored to the saved value.
   - To make `cmOK` route to the dialog, the dialog must be `current` + `sfModal` —
     `exec_view` does `set_current` + `set_state(sfModal)` itself, so just pre-queue
     `Command::OK` *before* calling `exec_view`. Trace it: `exec_view` inserts +
     selects + pushes frame + enters loop → pump 1 pops the queued `cmOK` → routes
     to current dialog → `end_modal(OK)` deferred → pump applies → `end_state =
     Some(OK)` → inner loop exits → `valid(OK)` true → return OK.
   - **If `cmOK` alone does not route** (e.g. command-set filter — `cmOK` is enabled
     by default, so it should pass), debug with a bounded pump count, never an
     unbounded loop in the test.
7. **`exec_view` returns `cmCancel` via Esc** — pre-queue an Esc `KeyDown`. Trace:
   pump 1 routes Esc to the dialog → posts `cmCancel` (now in queue) → pump 2 routes
   `cmCancel` → `end_modal(Cancel)` → exits. Assert returns `Command::CANCEL`. (Two
   pumps — fine.)
8. **cmQuit during a modal** (the advisor's non-obvious edge): inside a modal,
   `Event::Command(Command::QUIT)` is caught by `program_handle_event` →
   `end_state = Some(QUIT)`. Assert `exec_view` returns... trace it: the inner loop
   exits when `end_state` is set (to QUIT), `valid_end(QUIT)` → `group.valid(QUIT)`
   → likely true → `exec_view` returns `QUIT` and pops the frame. **Then the app's
   outer `run()` would see... nothing — `exec_view` consumed the QUIT.** Decide and
   TEST the faithful behavior: in C++ `cmQuit` inside a modal ends the *modal* with
   `cmQuit`; the caller is expected to re-post/propagate quit. Assert `exec_view`
   returns `Command::QUIT` and the frame is popped (no hang, no panic). Document the
   propagation expectation in a comment; do not build app-level quit propagation
   (no app exists).

For every pump-driven assertion that depends on the *correct* branch, confirm the
test **bites** (temporarily break the branch, see it fail) — the 33d-2 discipline.

---

## 6. Explicitly DEFERRED — do NOT build, NO dead stubs

- **Gray multi-scheme theming.** `palette = Gray` is *recorded* on the window but
  the frame still renders the blue `FrameActive`/`FramePassive` roles (it picks
  roles directly, not via the window palette). Mapping `Gray`/`Cyan` → distinct
  theme `Role`s (pushing the palette down to the frame + branching role selection +
  new `Theme` entries) is a **separate cosmetic chunk with no functional
  dependency on the modal mechanism** — defer it. Leave a `TODO(row 34 gray
  theming)` breadcrumb on `WindowPalette` / `Dialog`. (The window.rs note that
  "multi-scheme theming is deferred to row 34" is hereby narrowed: row 34 builds the
  *modal mechanism*; gray theming is a follow-on.)
- **`getData`/`setData`/`dataSize` (D10).** No data-bearing controls exist until
  Batch B (`TInputLine`, `TCheckBoxes`, …), so the group gather/scatter walk has
  nothing to gather. Defer to the first data control. No stub on `Dialog`/`Group`.
- **`message()`/`query` return-consuming tree-owner primitive + `cmCanCloseForm`
  veto.** `Dialog::valid` needs only `Group::valid` (already built). The
  return-consuming `message()` (guide "D4 message() — corrected") has **no consumer**
  at row 34 (`cmCanCloseForm` is an app pattern needing a validating control). Leave
  the guide's "designed but not built" status; do not add a dead `message()`.
- **View-/menu-triggered async modals** (`Deferred::OpenModal` + posted completion)
  — Phase 4 (no menu/button exists). Guide D9 "exec_view — corrected" carries the
  design; build only the sync `exec_view`.
- **`msgbox`** (`messageBox`/`inputBox`) — needs buttons + static text (Batch B).

---

## 7. Process & deliverable

- This is FOUNDATION on the shared tree, **no worktree**. Run the full
  `cargo test` + `clippy --all-targets -- -D warnings` + `fmt --check` before
  declaring done; report the green counts.
- The verification that matters is the **`exec_view` round-trip through real
  `pump_once` calls** (tests 6–8), not the handler units in isolation.
- Faithfulness > cleverness. Where you deviate from the C++ (the dropped
  `TheTopView`, the moot post-remove restores), say so in a code comment citing the
  D-rule.
- When done, summarize: files changed, the `exec_view` control flow, the deferrals,
  test counts, and any C++ detail you had to interpret.
