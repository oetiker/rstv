# Brief — Wire a real menu bar + status line into `Program` (Phase 4, FOUNDATION)

**Goal:** turn the standalone menu-bar + status-line views (already built, draw +
modal + data slices done) into a **drivable app**: real subviews inserted into the
root group, the `getEvent` status-line pre-routing wired, the desktop inset for the
bar/line rows, and `examples/hello.rs` grown into a runnable menu+status demo.

This is the integration payoff. Port FROM `magiblot-tvision`
`source/tvision/tprogram.cpp` (`getEvent`/`idle`/`initDeskTop`/`initMenuBar`/
`initStatusLine`) — already read for you and quoted below. **Faithful by default**
(CLAUDE.md methodology); the deviations are pre-decided here.

## What is IN scope (and what is explicitly OUT)

IN:
1. `Program` learns its menu-bar + status-line ids; **initial command regray** of both.
2. A faithful `getEvent` **status-line pre-routing** in `pump_once`.
3. The status line's **keyDown global-accelerator arm** (the last deferred arm).
4. `examples/hello.rs`: real menu bar + status line + desktop inset + a drivable loop.
5. Discriminating tests incl. a full-screen layout snapshot.

OUT (keep the existing breadcrumbs; do NOT build these):
- **`idle → statusLine->update()` help-ctx refresh.** With a single
  `TStatusDef(0,0xFFFF)` (the `All` def — what `initStatusLine` and our demo use),
  `find_items` is **invariant**: it selects the same def regardless of `TopView`'s
  help context, so `update()` is observably inert. No consumer ⇒ omit-until-consumer
  (the row-34 rule). **Do NOT add a `View::get_help_ctx` method or a TopView
  resolver this session.** Refine the existing idle breadcrumb in `program.rs` to
  record this rationale; leave the code path as-is.
- `cmTile`/`cmCascade`/`cmDosShell` + `Desktop::tile`/`cascade` geometry — a large
  self-contained follow-on; the demo menu must **not** include Tile/Cascade items.
- The status-line press-and-hold drag-highlight (`drawSelect(Some)` hover) — keep
  its `TODO(row 31, D9)`.

## C++ source of truth (already extracted)

```cpp
void TProgram::getEvent(TEvent& event) {
    // ... fetch event; if evNothing, idle() ...
    if( statusLine != 0 ) {
        if( (event.what & evKeyDown) != 0 ||
            ( (event.what & evMouseDown) != 0 &&
              firstThat( viewHasMouse, &event ) == statusLine ) )
            statusLine->handleEvent( event );    // <-- pre-routing, BEFORE normal dispatch
    }
    // ... cmScreenChanged (handled by our resize check) ...
}

void TStatusLine::handleEvent( TEvent& event ) {
    // ... evMouseDown arm: drag-highlight loop; on release post enabled cmd; clearEvent ...
    case evKeyDown:
        if( event.keyDown.keyCode != kbNoKey )
            for( TStatusItem *T = items; T != 0; T = T->next )
                if( TKey(event.keyDown) == T->keyCode && commandEnabled(T->command) ) {
                    event.what = evCommand;            // TRANSFORM IN PLACE
                    event.message.command = T->command;
                    return;                            // NO clearEvent, NO putEvent → it propagates
                }
        break;
}

TDeskTop   *initDeskTop(TRect r)    { r.a.y++; r.b.y--; return new TDeskTop(r); }
TMenuBar   *initMenuBar(TRect r)    { r.b.y = r.a.y + 1; return new TMenuBar(r, 0); }
TStatusLine*initStatusLine(TRect r) { r.a.y = r.b.y - 1; return new TStatusLine(r, /*defs*/); }
```

## Task 1 — `Program` ids + initial regray (`src/app/program.rs`)

- Add two fields beside `desktop: Option<ViewId>`:
  `menu_bar: Option<ViewId>`, `status_line: Option<ViewId>`. Capture them when
  `Program::new` inserts the factories (the inserts at ~line 289/292 currently
  discard the ids).
- **Initial regray (the carried gap).** Menus/status are born all-enabled and only
  regray on a `cmCommandSetChanged` broadcast, which does **not** fire at startup.
  In `Program::new`, build the command set as a **local** (`let command_set =
  default_command_set();`) *before* the struct literal, then — after the bar/line
  are inserted and you hold their ids — directly seed each:
  `if let Some(id) = menu_bar { if let Some(v) = group.find_mut(id) {
  v.update_menu_commands(&command_set); } }` (same for `status_line`). This is the
  established broker hook called immediately (we have `group` + `command_set` in
  hand — no need to defer; the deferred queue is not drained on an idle first pump
  anyway). Then move `command_set` into the struct literal.
- Keep the C++ insert ORDER (desktop, statusline, menubar) and the existing
  desktop-current logic untouched.

## Task 2 — `Group::topmost_child_at` (`src/view/group.rs`)

The `getEvent` mousedown guard is `firstThat(viewHasMouse, &event) == statusLine` —
faithful = the topmost **direct** child of the program (root group) under the point
(NOT recursive). The existing positional-routing in `Group::handle_event`
(group.rs:856) does this scan internally but exposes no id. Add a small pub(crate)
method mirroring it:

```rust
/// `firstThat(viewHasMouse)` over the group's direct children — the id of the
/// topmost visible child whose bounds contain `pos` (ports the program's
/// status-line mouse-route guard in `TProgram::getEvent`). Not recursive.
pub(crate) fn topmost_child_at(&self, pos: Point) -> Option<ViewId> {
    // children are stored back-to-front; topmost is last → iterate rev.
    // reuse the SAME visible + get_bounds().contains(pos) predicate as the
    // positional router at group.rs:856.
}
```
Return the child's `state().id()`. (Confirm the children vector + id accessor names
against the file; match the `(0..n).rev()` idiom already there.)

## Task 3 — `getEvent` status-line pre-routing in `pump_once` (`src/app/program.rs`)

Replace the breadcrumb at ~line 680 (`TODO(TStatusLine row): getEvent's status-line
pre-handling …`). At the **very top of the `Some(mut ev)` arm**, BEFORE the
`drop_disabled` computation and `captures.dispatch` — because C++ `getEvent`
pre-routes regardless of modal state, so accelerators must fire **while a modal
dialog is open**:

```rust
// getEvent status-line pre-routing (tprogram.cpp:153). keyDown always;
// mouseDown only when the status line is the topmost view under the cursor
// (firstThat(viewHasMouse) == statusLine) — else its unconditional clear would
// eat a click meant for the desktop/a dialog.
if let Some(sl) = *status_line {
    let pre = match &ev {
        Event::KeyDown(_) => true,
        Event::MouseDown(m) => group.topmost_child_at(m.position) == Some(sl),
        _ => false,
    };
    if pre {
        let mut ctx = Context::new(out_events, timers, now, deferred);
        if let Some(v) = group.find_mut(sl) {
            v.handle_event(&mut ev, &mut ctx);
        }
    }
}
```
(`status_line` is the destructured `&mut Option<ViewId>` field — deref it. Check the
`MouseDown` field name: the mouse arm in `status_line.rs` uses `m.position`; match
that. The `ctx` borrow ends before the existing dispatch block opens its own.)

Semantics that fall out (verify, do not re-port):
- **keyDown** → the status-line arm transforms `ev` into `Event::Command(cmd)`
  in place (Task 4) and does NOT clear. The same live `ev` flows on into
  `drop_disabled` (the command is enabled — `commandEnabled` already gated it — so
  it survives) → `captures.dispatch` / `program_handle_event` and routes. This is
  why F10 enters menus (→ cmMenu) and Alt-X quits (→ cmQuit) even mid-modal.
- **mouseDown over the bar** → the existing mouse arm posts the enabled command to
  `out_events` and clears `ev`; the cleared event makes the rest of the pump a
  no-op, and the posted command routes next pump (faithful putEvent + clearEvent).

## Task 4 — status-line keyDown accelerator arm (`src/status/status_line.rs`)

Replace the `_ => {}` breadcrumb (the `TODO(status keyDown global accelerator …)`
block, ~line 380) with the faithful keyDown arm:

```rust
Event::KeyDown(k) => {
    // tstatusl.cpp keyDown arm: match the keyCode against EVERY item
    // (incl. text == None hidden global hotkeys) and, if enabled, TRANSFORM
    // the event into evCommand IN PLACE — no clear, no post. The pre-routing
    // in TProgram::getEvent then lets the transformed command propagate to
    // normal dispatch (porting it as ctx.post + clear would double-handle).
    for item in self.items() {
        if item.key_code == Some(*k) && self.command_enabled(item.command) {
            *ev = Event::Command(item.command);
            break;
        }
    }
}
```
Check the real field/accessor names: `items()` returns the active `&[StatusItem]`
(used by the mouse arm); `item.key_code: Option<KeyEvent>`; `item.command:
Command`; `command_enabled(cmd)` is the cached-`CommandSet` check the mouse arm
already calls. Match `KeyDown`'s payload type so `Some(*k)` lines up with
`key_code`. Do **not** clear `ev`. Match over ALL items including textless ones
(do not skip `text == None`). Update the module-doc deferral note for the keyDown
arm to "DONE (transform-in-place; pre-routed by TProgram::getEvent)".

## Task 5 — `examples/hello.rs`: a drivable menu + status demo

Make it a real app. The menu can only wire commands that **already route** (menu→
dialog needs the unbuilt D9 `OpenModal` path, row 63):

- **`init_desktop`**: shrink for the bar/line rows — `let mut r = r; r.a.y += 1;
  r.b.y -= 1;` then build the `Desktop` over `r`. **Insert 2–3 demo `Window`s** into
  the desktop so the window commands do something visible (mirror the
  `program_with_windows` test harness at program.rs:1220 — `Window::new(Rect, Some
  title, number)` + `desktop.insert_view`).
- **`init_menu_bar`**: `let mut r = r; r.b.y = r.a.y + 1;` → `MenuBar::new(r, menu)`
  with a `MenuBuilder` menu, e.g. `~≡~`/File→`Exit` (cmQuit) and `~W~indow`→`~N~ext`
  (cmNext) / `~Z~oom` (cmZoom) / `~C~lose` (cmClose). **No Tile/Cascade.** (Use the
  builder API in `src/menu/mod.rs`: `.submenu(name, key, |m| …)`, `.command(name,
  cmd)`, `.command_key(...)`, `.separator()`.)
- **`init_status_line`**: `let mut r = r; r.a.y = r.b.y - 1;` → the standard line
  (`StatusLine::new(r, defs)` via the `StatusDef::list().def_all(|b| …).build()`
  builder in `src/status/mod.rs`): `Alt-X Exit`→cmQuit, `F10 Menu`→cmMenu (textless
  ok or labelled), plus textless `AltF3`→cmClose, `F5`→cmZoom — match
  `initStatusLine`'s shape. Use `.item(text, key, cmd)` for labelled and
  `.key_item(key, cmd)` for textless.
- **`run()`**: replace the `exec_view(About)` demo with `self.program.run()` (the
  real loop; returns on cmQuit). Keep `AboutDialog` in the file if it still
  compiles unused-free, else remove it — your call, keep the file warning-clean.

**Known limitation to STATE in a doc comment (advisor-flagged):** menu items cannot
open dialogs yet (no D9 `OpenModal`); the demo wires only routing commands.
Alt-shortcuts reach the bar via `ofPreProcess` (the menu bar sets `pre_process` and
`Group::handle_event` runs the preProcess phase) and F10 enters menus via the
status-line accelerator → both nav paths work.

## Task 6 — Tests (discriminating + bite-checked; Appendix B step 4 for snapshots)

Add to `program.rs` `#[cfg(test)]` (reuse the `program_with_*` harness; you will
need a variant that supplies a real menu bar + status line — extend the harness or
add a focused one). `cargo-insta` is NOT installed → generate `.snap` with
`INSTA_UPDATE=always cargo test <name>`, hand-verify, re-run plain, commit.

1. **Full-screen layout snapshot** — menu bar pinned at row 0, status line at row
   `h-1`, desktop between. Proves the inset.
2. **F10 → menu opens** — inject F10; pump; assert a menu box/active session
   appears (or `cmMenu` routed to the bar). Proves the status-line keyDown
   accelerator → propagation.
3. **Alt-X → quit** — inject Alt-X; pump (twice — transform this pump, posted/routed
   command next); assert `end_state == Some(cmQuit)`.
4. **THE placement crux: accelerator fires during a modal.** Open a dialog via
   `exec_view`, then inject Alt-X (or F10); assert it still reaches the status line
   (end_state set / menu activates). **Bite-check:** moving the pre-route to *after*
   `captures.dispatch` must fail this. Document the bite.
5. **mouseDown pre-route gating** — a mousedown on row `h-1` posts the hit item's
   command; a mousedown in the desktop area is NOT pre-routed (no spurious clear —
   it still reaches the desktop). If feasible, a mousedown on a modal dialog that
   overlaps row `h-1` is NOT pre-routed (topmost_child_at ≠ status line).
6. **Initial regray** — construct a program whose `default_command_set` has some
   menu/status command disabled (or disable one pre-construction path), assert the
   bar item is greyed (`disabled`) and the status line's `cmd_set` cache reflects it
   immediately after `Program::new` (no pump needed).

## Verify before handing back
```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
cargo test  --workspace
cargo clippy --workspace --all-targets -- -D warnings   # force a fresh lint
cargo fmt --all --check
cargo build --example hello
```
Report the test count delta and any deviation you had to make from this brief
(the brief can be wrong — the C++ + guide win).

## Build incrementally
Write ONE file's changes, verify it compiles, then the next (the subagent
incremental-write rule). Order: Task 4 (status_line arm) → Task 2 (group helper) →
Task 1 + 3 (program) → Task 5 (demo) → Task 6 (tests).
