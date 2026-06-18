# TMenu type  (guide pp. 477–478)

Rust module(s): `src/menu/mod.rs`   |   magiblot: `include/tvision/menus.h` / `source/tvision/menu.cpp`

| Guide entry | Pg | Bucket | Corr | Rust symbol / mapping | Doc | Notes |
|---|---|---|---|---|---|---|
| `Items` (field, `PMenuItem`) | 477 | EQUIVALENT | OK | `Menu::items: Vec<MenuItem>` | 2 | C++ linked list via pointer becomes an owned `Vec`. Known idiomatic mapping (D1). `items` is `pub`, explained in struct doc ("what"), but "how to iterate / modify" not covered. |
| `Default` (field, `PMenuItem`) | 477 | EQUIVALENT | OK | `Menu::default: Option<usize>` | 2 | C++ pointer to the default item becomes an index into `items` (or `None`). D1. Public, documented what/why for `None` but not "how the session uses it at open". |
| `NewMenu` (constructor function) | 477 | EQUIVALENT | OK | `Menu::builder() -> MenuBuilder` + `MenuBuilder::build()` | 2 | C++ `NewMenu(items)` heap-allocates a `TMenu` with `items = deflt = &itemList`. Rust uses a fluent `MenuBuilder`; first appended item auto-sets `default = Some(0)`. Module doc calls this out. `MenuBuilder` itself has a doc explaining what/how; `Menu::builder()` entry-point scores 2 (what, not when-vs-struct-literal). |
| `DisposeMenu` (destructor procedure) | 477 | NOT-PORTED | — | — | — | DOS Pascal manual memory management; Rust ownership + `Drop` makes an explicit `DisposeMenu` unnecessary. |
| default `TMenu()` (empty constructor) | 477 | EQUIVALENT | OK | `Menu::default()` (`#[derive(Default)]`) | 1 | C++ `TMenu()` sets `items = deflt = 0`. Rust `Default` gives `items: vec![], default: None`. No doc on the derived impl beyond the struct-level note. |
| `TMenu(itemList)` (single-arg constructor) | 477 | EQUIVALENT | OK | `MenuBuilder` first `.item()` / `.command()` / `.submenu()` call | 2 | Sets `items = deflt = &itemList` (first item is the default). `MenuBuilder::item()` sets `default = Some(0)` on first push — same semantics. Covered by builder doc. |
| `TMenu(itemList, theDefault)` (two-arg constructor) | 477 | EQUIVALENT | OK | `Menu { items: ..., default: Some(n) }` struct literal (escape hatch) | 1 | C++ lets caller pass a separate default item. Rust: the struct fields are `pub`; the builder always uses `Some(0)`. A custom default requires a struct literal. Not called out in builder doc. |

## Summary

- PORTED: 0   EQUIVALENT: 5   NOT-PORTED: 1   MISSING: 0   UNSURE: 0
- SUSPECT: 0   |   doc<3 (public): 5   |   → concept: 0
- Notable finding: The two-arg `TMenu(itemList, theDefault)` constructor — which lets the caller designate any item as the default, not just the first — has no builder equivalent and is only accessible via a struct literal; this is not called out in the `MenuBuilder` documentation.
