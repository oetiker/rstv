# Brief — Row 46 `TMenuItem`/`TSubMenu`/`TMenu` (FOUNDATION, the menu *data tree*)

**Scope: the menu DATA TREE ONLY.** No `View` impl, no drawing, no event
handling, no `execute`/`findItem`/`hotKey`/`getItemRect`. Those are rows 49–52
(`TMenuView`/`TMenuBar`/`TMenuBox`/`TMenuPopup`) and must NOT appear here. This
row is pure data + a builder API + an equivalence test.

## C++ source of truth
`include/tvision/menus.h` (the `TMenuItem`/`TSubMenu`/`TMenu` class decls +
`newLine()` + the `operator+` builder signatures) and `source/tvision/menu.cpp`
(the `operator+` bodies that chain the linked lists). Port FROM
`/home/oetiker/scratch/tvision-spec/magiblot-tvision/`.

### What the C++ models
`TMenuItem` is a linked-list node (a discriminated union over a C `union { const
char *param; TMenu *subMenu; }`). The consumers (rows 49–52, e.g.
`tmnuview.cpp`/`tmenubox.cpp`/`tmenubar.cpp`) discriminate **two-level**:
- `item->name == 0`  ⇒ **separator** (`newLine()`: name=0, command=0, subMenu=0).
- else `item->command == 0` ⇒ **submenu** (union holds `subMenu`).
- else ⇒ **command item** (union holds `param`, the shortcut display text like `"Alt-X"`).

`TMenuItem` fields: `next` (linked list), `name`, `command`, `disabled`,
`keyCode` (a `TKey` accelerator), `helpCtx`, and the `param`/`subMenu` union.
`TSubMenu : TMenuItem` is just a `TMenuItem` whose `subMenu` is a fresh `TMenu`
and whose `command` is 0. `TMenu` holds `items` (list head) + `deflt` (default
item pointer; `TMenu(itemList)` sets `deflt = items` — literally the **head**,
index 0, NO separator-skip). `disabled` is mutated at runtime by
`TMenuView::updateMenu` (command-graying), but ONLY on command items.

## Target: `src/menu/mod.rs` (new module `menu`), wired into `lib.rs`

### Data model — a 3-variant enum (advisor-confirmed)
Translate the C `union` to a Rust enum. This is house style (Key/Event are
enums) and makes param-xor-subMenu type-safe (no illegal states). Shared fields
(`name`, `disabled`, `help_ctx`, `key_code`) are accessed uniformly by consumers
via **or-patterns** (`Command { disabled, .. } | SubMenu { disabled, .. } => …`),
so the enum costs nothing on uniform access. **Do NOT** factor shared fields into
a speculative common sub-struct — if rows 49–52 want it, that's a cheap later
refactor.

```rust
pub enum MenuItem {
    /// C++ newLine(): name==0. A horizontal divider line.
    Separator,
    /// C++ command != 0: union holds `param`.
    Command {
        name: String,                 // ~-marked label (the ~ marks the hotkey)
        command: Command,
        key_code: Option<KeyEvent>,   // TKey accelerator; None == kbNoKey (our model: absence)
        param: Option<String>,        // shortcut display text ("Alt-X"); C++ param, may be empty/0
        help_ctx: HelpCtx,
        disabled: bool,
    },
    /// C++ TSubMenu (command == 0): union holds `subMenu`.
    SubMenu {
        name: String,
        key_code: Option<KeyEvent>,
        help_ctx: HelpCtx,
        disabled: bool,
        menu: Menu,                    // owned (C++ owns subMenu, frees in ~TMenuItem)
    },
}

pub struct Menu {
    pub items: Vec<MenuItem>,         // C++ linked list → Vec
    pub default: Option<usize>,       // C++ deflt as an INDEX into items (None iff empty)
}
```

- `key_code: Option<KeyEvent>` — faithful: our key model represents `kbNoKey` as
  the *absence* of a key event. Most submenus/separators have no accelerator.
- `Menu::default` is a Vec **index** (not a pointer) so row 49's `current` can be
  an index. Init = **0** when `items` is non-empty (C++ `TMenu(itemList)` head),
  `None` when empty (C++ default ctor `deflt=0`).
- Provide a small helper if convenient for row 49's graying loop, e.g.
  `MenuItem::disabled_mut(&mut self) -> Option<&mut bool>` (returns `None` for
  `Separator`). Optional — the or-pattern works too; add only if it reads clean.

### Builder API (replaces C++ `operator+`)
The PORT-ORDER deviation for this row: `operator+` chains → a Rust builder. C++
usage to reproduce:
```cpp
new TMenu(
  *new TSubMenu("~F~ile", kbAltF) +
    *new TMenuItem("~O~pen", cmOpen, kbF3, hcNoContext, "F3") +
    *new TMenuItem("~N~ew",  cmNew,  kbF4) +
    newLine() +
    *new TMenuItem("E~x~it", cmQuit, kbAltX, hcNoContext, "Alt-X") +
  *new TSubMenu("~W~indow", kbAltW) +
    *new TMenuItem("~N~ext", cmNext, kbF6) )
```
Design a fluent builder (`Menu::builder()` / a `MenuBuilder`) with closures for
nesting, e.g.:
```rust
Menu::builder()
    .submenu("~F~ile", alt('f'), |m| m
        .command_key("~O~pen", Command::OPEN, KeyEvent::from(Key::F(3)), "F3")
        .command("~N~ew", Command::NEW)            // no accelerator/param
        .separator()
        .command_key("E~x~it", Command::QUIT, alt('x'), "Alt-X"))
    .submenu("~W~indow", alt('w'), |m| m
        .command_key("~N~ext", Command::NEXT, KeyEvent::from(Key::F(6)), ""))
    .build()
```
Method set is your call, but cover at least: `separator()`, a no-accel command,
a command-with-accelerator+param, `submenu(name, key, closure)`, and a raw
`item(MenuItem)` escape hatch for full control (help_ctx, disabled). Keep it
**lean** — common cases ergonomic, full control via `item()`. `help_ctx`
defaults to `HelpCtx::NO_CONTEXT`; `disabled` defaults to `false`. The builder
sets `default = Some(0)` once the first item is pushed (faithful to C++ head).
- Do **NOT** add helpers to `src/event/key.rs` (out of scope — FOUNDATION file).
  If you want an `alt(char)` shorthand for tests/builder ergonomics, make it a
  small local `fn` in the menu module or just construct
  `KeyEvent::new(Key::Char('f'), KeyModifiers { alt: true, ..Default::default() })`
  inline.

### Types you build on (already exist)
- `crate::command::Command` (opaque newtype; consts like `Command::OPEN`,
  `Command::QUIT`, `Command::NEW`, `Command::NEXT`).
- `crate::help::HelpCtx` (`HelpCtx::NO_CONTEXT` default).
- `crate::event::{Key, KeyEvent, KeyModifiers}` — `KeyEvent { key, modifiers }`,
  `KeyEvent::from(Key::F(3))`, `KeyEvent::new(key, mods)`. No `.alt()` helper
  exists; build modifiers via the struct literal.

## VERIFICATION — NOT a snapshot test
This row is pure data: **nothing renders**, so the standard "add a snapshot test
(Appendix B step 4)" boilerplate does NOT apply — do not fabricate one. The
correctness claim is: **the builder reproduces the exact tree the C++ `operator+`
chains produce.** Write unit tests in `src/menu/mod.rs` that:
1. Build a representative two-submenu menu (the File/Window example above) with
   the builder, then assert the resulting `Menu` **node-for-node**: item count
   and order, separator in the right position, `name`/`command`/`key_code`/
   `param`/`help_ctx` on each command item, submenu nesting, and
   `default == Some(0)`. This equivalence IS the row.
2. Edge cases: empty `Menu::builder().build()` ⇒ `items` empty, `default == None`.
   A separator's variant is `MenuItem::Separator`. A command with no accelerator
   ⇒ `key_code == None`. (Make at least one assertion *discriminating* — verify a
   wrong tree would fail it.)
3. If you add `disabled_mut`, test it returns `None` for `Separator` and a live
   `&mut` for command/submenu items.
Derive `PartialEq`/`Debug` on the types so the tree assertion can be a single
`assert_eq!` against a hand-built expected `Menu` (cleanest equivalence proof).

## Conventions / gates
- English for all code/comments/identifiers. Doc-comment each public type with
  the C++ symbol it ports (match the house style in `command.rs`/`help.rs`).
- No dead stubs / omit-until-consumer: build ONLY the data tree + builder + the
  fields the row-49–52 access patterns above consume. No `View`, no draw helpers.
- Run, from the repo root with `export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target`:
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo fmt --all --check`
  All three must pass clean. Report the final test counts.
- Wire the module: `pub mod menu;` in `src/lib.rs` + re-export the public types
  (`pub use menu::{Menu, MenuItem, MenuBuilder};` — match the existing re-export
  block style around `lib.rs:91-115`).
```
