# TStatusItem  (guide pp. 538–539)

Rust module(s): src/status/mod.rs   |   magiblot: include/tvision/menus.h (TStatusItem)

> TStatusItem is a plain record (no inheritance). The guide documents four record
> fields and one constructor (`NewStatusKey`). The magiblot C++ header also
> exposes an `operator+` builder and a `nullptr`-text convention for hidden
> bindings. All are listed below.

| Guide entry | Pg | Bucket | Corr | Rust symbol / mapping | Doc | Notes |
|---|---|---|---|---|---|---|
| `next` (field) | 538 | EQUIVALENT | OK | `Vec<StatusItem>` owned by `StatusDef` | N/A | C++: `TStatusItem *next` — singly-linked list node pointer. Rust: the `Vec<StatusItem>` in `StatusDef.items` replaces the linked list. Known idiomatic mapping: linked list → Vec. No public field. |
| `text` (field) | 538 | PORTED | OK | `StatusItem.text: Option<String>` | 3 | C++: `char *text` — `nullptr` text is the hidden-binding convention (draws nothing, consumes no width). Rust: `Option<String>` — `None` is the hidden-binding sentinel; `Some(s)` is a visible label. The convention and its draw / hit-test effects are documented in the field and module docs. |
| `keyCode` (field) | 538 | PORTED | OK | `StatusItem.key_code: Option<KeyEvent>` | 2 | C++: `TKey keyCode` — a key combination (may be `kbNoKey` for a label-only item). Rust: `Option<KeyEvent>`; `None` means no accelerator. The `Option` replaces the sentinel value. Public field. Doc score 2 (what it is; "how to use" note on accelerator matching could be added). |
| `command` (field) | 538 | PORTED | OK | `StatusItem.command: Command` | 2 | C++: `ushort command`. Rust: `tv::Command` newtype. Public field. Doc score 2. |
| `NewStatusKey` (constructor) | 538 | EQUIVALENT | OK | `StatusItem::new(text, key_code, command)` + `StatusItem::key(key_code, command)` + `StatusItemsBuilder::item` / `key_item` | 3 | C++: `NewStatusKey(aText, aKey, aCommand)` — a macro/function that allocates a `TStatusItem` with the given fields. Rust: `StatusItem::new` for visible items, `StatusItem::key` for hidden bindings (the `nullptr`-text case), plus the fluent builder methods `item()` / `key_item()` on `StatusItemsBuilder`. All three paths are documented. The `key` constructor's no-text semantics are explicitly called out. |
| `operator+` (C++ builder chain) | — | EQUIVALENT | OK | `StatusItemsBuilder` chaining (`.item`, `.key_item`, `.raw`) | 2 | C++ `operator+(TStatusDef&, TStatusItem&)` appends an item to a def. Rust: `StatusItemsBuilder` does this via chained method calls. Known idiomatic mapping. |

## Summary

- PORTED: 3   EQUIVALENT: 3   NOT-PORTED: 0   MISSING: 0   UNSURE: 0
- SUSPECT: 0   |   doc<3 (public): 2   |   → concept: 0
- Notable findings: No gaps or suspect items. The most notable mapping is `char* text = nullptr` → `Option<String>` (`None` = hidden binding): the Rust encoding is cleaner and the convention is well-documented at the field level. `key_code: Option<KeyEvent>` similarly improves on the `kbNoKey` sentinel; the field doc could add a note about `kbNoKey` heritage to reach score 3.
