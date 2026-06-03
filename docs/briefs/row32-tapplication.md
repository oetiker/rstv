# Brief — Row 32 `TApplication` (MECHANICAL, thin)

Port `TApplication` (C++ `tapplica.cpp`) into a new module `src/app/application.rs`.
This is a **genuinely thin** D2 embed-and-delegate wrapper over the existing
`Program` (row 31). Dependency order makes it thin: its substantive behavior
(tile/cascade geometry, dosShell terminal-suspend) is **deferred** because the
prerequisites do not exist yet. Do NOT pad it. Faithfully scope, drop, and
breadcrumb.

## What `TApplication` is in C++ (the whole class)
```cpp
TAppInit::TAppInit()      // static subsystem init (hw/mouse/screen/eventq/syserr)
TApplication::TApplication() : TProgInit(initStatusLine, initMenuBar, initDeskTop)
                          { initHistory(); }
TApplication::~TApplication() { doneHistory(); }
void suspend()  { TSystemError::suspend(); TEventQueue::suspend(); TScreen::suspend(); }
void resume()   { TScreen::resume(); TEventQueue::resume(); TSystemError::resume(); }
void cascade()  { if(deskTop) deskTop->cascade(getTileRect()); }
void tile()     { if(deskTop) deskTop->tile(getTileRect()); }
TRect getTileRect() { return deskTop->getExtent(); }
void dosShell() { suspend(); writeShellMsg(); raise(SIGTSTP); resume(); redraw(); }
void handleEvent(TEvent& e) {                 // overrides TProgram::handleEvent
    TProgram::handleEvent(e);
    if (e.what==evCommand) switch(e.message.command) {
        case cmDosShell: dosShell(); break;
        case cmCascade:  cascade();  break;
        case cmTile:     tile();     break;
        default: return;
    }
    clearEvent(e);
}
```

## Our mapping (what to build)

`Application { program: Program }` in `src/app/application.rs`. This is the type a
real app constructs (today `examples/hello.rs` hand-rolls a `HelloApp` embedding
`Program` directly — leave it alone, see "Do NOT" below).

### Constructor
`Application::new(backend, clock, theme, create_desktop, create_status_line,
create_menu_bar)` forwards **verbatim** to `Program::new(...)` and wraps the
result. Add `// TODO(history): TApplication ctor calls initHistory() — the history
list subsystem is not ported yet.` No `TAppInit` (subsystem init = our
backend/Renderer's job, dropped).

### `Drop`
No `Drop` impl needed; add a one-line `// TODO(history): ~TApplication calls
doneHistory().` comment on the type.

### `get_tile_rect` — the ONE real body
Add a **public method on `Program`** (it has the private `group` + `desktop`;
`Application` cannot reach them, and the future cmTile/cmCascade handling in
`program_handle_event` — same module — will reuse it):
```rust
/// `TApplication::getTileRect` — the rectangle tile/cascade lay windows into:
/// the **desktop child's extent** (NOT the screen rect), so it stays correct once
/// Phase 4 insets the desktop under a menu/status bar. `None` if no desktop.
pub fn get_tile_rect(&self) -> Option<Rect>
```
Body: resolve `self.desktop` via the group and return `v.state().get_extent()`.
(`get_extent()` exists on `ViewState`; returns `(0,0,size)`.) You may need a
`&self` resolver — `find_mut` is `&mut`; either add a tiny private `&self` lookup
or make `get_tile_rect(&mut self)`. Prefer keeping it `&self`: add a minimal
`group: &Group` read path (there is `find_mut`; check if a `&self` `find`/`view`
accessor exists, else `&mut self` is acceptable — note your choice).
`Application::get_tile_rect(&self)` delegates to `self.program.get_tile_rect()`.

### `tile` / `cascade` / `dos_shell` / `suspend` / `resume` — breadcrumbed stubs
Provide them as `Application` methods so the API matches C++, but they are
**blocked**, so implement as documented no-ops with TODOs (no fake bodies):
- `tile(&mut self)` / `cascade(&mut self)`:
  `// TODO(Phase 4): TDeskTop::tile/cascade geometry (mostEqualDivisors/`
  `// calcTileRect/doCascade, tdesktop.cpp) is deferred — needs real tileable`
  `// windows + a menu emitting cmTile/cmCascade to trigger + test it. When built,`
  `// the cmTile/cmCascade cases in program_handle_event call`
  `// deskTop->tile/cascade(get_tile_rect()). Body deferred with the geometry.`
- `dos_shell(&mut self)`:
  `// TODO(dosShell): needs a backend terminal suspend/resume seam (CrosstermBackend`
  `// owns no terminal setup today) + SIGTSTP. Deferred.`
- `suspend`/`resume`: same — `// TODO(dosShell): backend suspend/resume seam not surfaced.`
  (You may fold these into `dos_shell`'s breadcrumb and omit separate methods —
  your call; note it.)

### Command handling — breadcrumb in `program_handle_event`
`TApplication::handleEvent` is **program-level** command handling — its faithful
home is the existing `program_handle_event` free fn in `src/app/program.rs`,
right beside the QUIT and Alt-N blocks (which it already mirrors). The three
command cases (cmDosShell/cmCascade/cmTile) are **blocked on the deferred bodies
above**, so do NOT wire them live. Add a breadcrumb comment block near the QUIT
catch:
```rust
// TODO(Phase 4: TApplication command handling): cmTile/cmCascade/cmDosShell are
// program-level commands (TApplication::handleEvent, tapplica.cpp). They belong
// HERE beside the QUIT/Alt-N blocks (group.find_mut(desktop) -> desktop.tile/
// cascade(get_tile_rect()); dosShell -> backend suspend). Deferred: tile/cascade
// need Desktop::tile/cascade geometry + a menu to emit the commands; dosShell
// needs a backend suspend seam. No source emits these commands yet (no menus).
```
Do NOT add `cmTile`/`cmCascade`/`cmDosShell` `Command` consts unless they already
exist — check `src/command.rs`; if absent, mention it (the breadcrumb references
them by name only). Do NOT invent them speculatively.

### Delegations on `Application`
Provide ergonomic delegations so apps use `Application` like C++ apps use the
`TApplication` they derive (all one-liners to `self.program` / `&mut self.program`):
`run`, `pump_once`, `exec_view`, `desktop`, `end_modal`, `end_state`,
`enable_command`, `disable_command`, `command_enabled`. Plus escape hatches
`program(&self) -> &Program` and `program_mut(&mut self) -> &mut Program`.
Keep each a trivial forward — **do NOT introduce a delegation macro** (see Do NOT).

## Wiring
- `src/app/mod.rs`: `mod application;` + `pub use application::Application;`
- `src/lib.rs`: add `Application` to the `pub use app::{...}` line.
The orchestrator may also do this; do it yourself and the orchestrator will reconcile.

## Tests (add to `application.rs` `#[cfg(test)]`)
- `get_tile_rect` returns the desktop extent (build an `Application` on a
  `HeadlessBackend` with a real `Desktop` factory; assert the rect == desktop
  size). Use the `Program` test patterns already in `program.rs` for harness shape
  (HeadlessBackend + ManualClock + Theme + `Desktop::new` factory).
- A delegation smoke test: e.g. `command_enabled`/`enable_command` round-trips
  through `Application`, or `desktop()` returns `Some`.
- (No tile/cascade tests — bodies are deferred.)

## Verify before returning
Run with `CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target`:
- `cargo test` (all green)
- `cargo clippy --all-targets -- -D warnings` (clean)
- `cargo fmt --check` (clean)
Report the exact test counts.

## Do NOT (scope guards — these have bitten this project before)
- Do NOT build `Desktop::tile`/`cascade` geometry. Deferred to Phase 4.
- Do NOT add a `delegate_view_rest!` (or any) delegation macro. A prior implementer
  added one as out-of-scope creep and it was reverted. Write the few delegations by
  hand.
- Do NOT refactor `examples/hello.rs`'s `HelloApp` to embed `Application`. Out of
  scope; leave the example untouched.
- Do NOT add fields/callbacks/hooks to `Program` beyond the single `get_tile_rect`
  method (no app-command callback — the seam is `program_handle_event`).
- Do NOT invent `Command` consts that don't exist.
- Touch `src/app/program.rs` ONLY for: the `get_tile_rect` method + the breadcrumb
  comment. Nothing else in that FOUNDATION file.
```
