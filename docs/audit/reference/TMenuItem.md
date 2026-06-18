# TMenuItem type  (guide p. 481)

Rust module(s): `src/menu/mod.rs`   |   magiblot: `include/tvision/menus.h` / `source/tvision/menu.cpp`

> The guide documents `TMenuItem` as a Pascal record with 7 named fields plus a
> variant part (`case Integer of`), and three constructor functions (`NewItem`,
> `NewLine`, `NewSubMenu`). `TSubMenu` (a subclass of `TMenuItem` in the C++ port)
> is also noted; in the Rust port all three are folded into one `MenuItem` enum.

| Guide entry | Pg | Bucket | Corr | Rust symbol / mapping | Doc | Notes |
|---|---|---|---|---|---|---|
| `Next` (field, `PMenuItem`) | 481 | EQUIVALENT | OK | implicit `Vec` membership in `Menu::items` | N/A | C++ singly-linked list threading. Rust: items live in `Menu::items: Vec<MenuItem>`; "next" is array succession. Known idiomatic mapping (D1). No public `next` field needed. |
| `Name` (field, `PString`) | 481 | EQUIVALENT | OK | `MenuItem::Command { name: String, .. }` / `MenuItem::SubMenu { name: String, .. }` | 2 | C++ `nil` name signals a divider line; Rust uses `MenuItem::Separator` variant instead — more type-safe. Module doc explains the three-variant model. Per-field docs score 2 (what). |
| `Command` (field, `Word`) | 481 | EQUIVALENT | OK | `MenuItem::Command { command: Command, .. }` | 2 | C++ `0` means submenu; Rust uses the enum split to make the command-vs-submenu distinction explicit (D1). Doc explains what. |
| `Disabled` (field, `Boolean`) | 481 | PORTED | OK | `MenuItem::Command { disabled: bool, .. }` / `MenuItem::SubMenu { disabled: bool, .. }` | 2 | Present in both named variants; `MenuItem::disabled_mut()` gives a `&mut bool` handle, with `None` for a `Separator`. Doc explains what + the graying use case. "how/when the pump toggles it" not in rustdoc (lives in session doc). |
| `KeyCode` (field, `Word`) | 481 | EQUIVALENT | OK | `MenuItem::Command { key_code: Option<KeyEvent>, .. }` / `MenuItem::SubMenu { key_code: Option<KeyEvent>, .. }` | 2 | C++ `0` = no hot key; Rust uses `Option<KeyEvent>`. Doc says what; "how to construct an accelerator" not here (the `alt()` free function is the helper, not linked from the field doc). |
| `HelpCtx` (field, `Word`) | 481 | PORTED | OK | `MenuItem::Command { help_ctx: HelpCtx, .. }` / `MenuItem::SubMenu { help_ctx: HelpCtx, .. }` | 2 | `HelpCtx::NO_CONTEXT` replaces `hcNoContext`. Doc states what. |
| `Param` (field, variant case 0, `PString`) | 481 | EQUIVALENT | OK | `MenuItem::Command { param: Option<String>, .. }` | 2 | C++ `nil` = no param text; Rust `None`. Empty string coerced to `None` by `MenuBuilder::command_key` (documented in method). |
| `SubMenu` (field, variant case 1, `PMenu`) | 481 | EQUIVALENT | OK | `MenuItem::SubMenu { menu: Menu, .. }` | 2 | C++ raw pointer to a heap-allocated `TMenu`; Rust owned `Menu` (no pointer, no manual disposal). Deviation D1. Doc explains what. |
| `NewItem` (constructor function) | 481 | EQUIVALENT | OK | `MenuBuilder::command_key` / `MenuBuilder::command` | 2 | C++ `NewItem(Name, Param, Key, Cmd, HelpCtx, Next)` heap-allocates and chains. Rust uses builder methods; `command` for no-key items, `command_key` for items with an accelerator. Builder doc covers both. "what the builder produces" explained; "migrating NewItem calls" not explicit. |
| `NewLine` (constructor function) | 481 | EQUIVALENT | OK | `MenuBuilder::separator` | 2 | C++ `newLine()` (also spelled `NewLine` in magiblot) creates a nil-name item. Rust: `separator()` appends `MenuItem::Separator`. Documented. |
| `NewSubMenu` (constructor function) | 481 | EQUIVALENT | OK | `MenuBuilder::submenu` | 2 | C++ `NewSubMenu(Name, Key, SubMenu, HelpCtx, Next)`. Rust: `submenu(name, key_code, |b| b...)` with a closure. Builder doc covers it. `help_ctx` always `HelpCtx::NO_CONTEXT` from the builder — no escape hatch in `submenu()` itself (requires the raw `MenuItem::SubMenu` struct literal). |
| `TSubMenu` (subclass, magiblot C++) | 481 | EQUIVALENT | OK | `MenuItem::SubMenu { .. }` variant | 2 | magiblot refactored `TSubMenu` as a `TMenuItem` subclass; the Rust port folds it into the same enum variant. Module heritage note covers this. |
| `operator+` overloads (magiblot C++) | 481 | EQUIVALENT | OK | `MenuBuilder` chaining (`.command().submenu()...`) | 2 | magiblot uses `operator+(TSubMenu&, TMenuItem&)` etc. to chain items. Rust replaces these with builder method chaining. Builder heritage note covers this. |

## Summary

- PORTED: 2   EQUIVALENT: 11   NOT-PORTED: 0   MISSING: 0   UNSURE: 0
- SUSPECT: 0   |   doc<3 (public): 12   |   → concept: 0
- Notable finding: `MenuBuilder::submenu()` has no way to set a non-`NO_CONTEXT` `help_ctx` for the submenu item itself — only a struct literal escape hatch works — and this is not documented on `submenu()`.
