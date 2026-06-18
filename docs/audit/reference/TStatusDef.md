# TStatusDef  (guide pp. 537–538)

Rust module(s): src/status/mod.rs   |   magiblot: include/tvision/menus.h (TStatusDef) / source/tvision/tstatusl.cpp

> TStatusDef is a plain record (no inheritance). The guide documents four record
> fields and one constructor (`NewStatusDef`). The magiblot C++ header also
> exposes the operator+ builder syntax. All are listed below.

| Guide entry | Pg | Bucket | Corr | Rust symbol / mapping | Doc | Notes |
|---|---|---|---|---|---|---|
| `next` (field) | 537 | EQUIVALENT | OK | `Vec<StatusDef>` owned by `StatusLine` | N/A | C++: `TStatusDef *next` — singly-linked list node; ownership threaded through the `TStatusLine` destructor. Rust: the `Vec<StatusDef>` owned by `StatusLine` replaces the entire linked list. Known idiomatic mapping: linked list → Vec. Private implementation detail; no public field. |
| `min` (field) | 537 | EQUIVALENT | OK | `StatusDef.range: HelpCtxRange` (`HelpCtxRange::All` or `HelpCtxRange::OneOf`) | 2 | C++: `ushort min` — lower bound of the integer help-context range `[min, max]`. Rust: the numeric range is replaced by `HelpCtxRange`, which is either `All` (matches any context) or `OneOf(Vec<HelpCtx>)` (matches named membership set). This is deviation D1 (HelpCtx is now a `&'static str` key, not an integer), documented in the module doc. The `range` field is public. |
| `max` (field) | 537 | EQUIVALENT | OK | `StatusDef.range: HelpCtxRange` (same field, upper bound subsumed) | 2 | C++: `ushort max` — upper bound of the `[min, max]` range. Rust: absorbed into `HelpCtxRange` alongside `min`; no separate field. See the `min` row. |
| `items` (field) | 537 | EQUIVALENT | OK | `StatusDef.items: Vec<StatusItem>` | 2 | C++: `TStatusItem *items` — pointer to the first item in a singly-linked list. Rust: `Vec<StatusItem>`. Known idiomatic mapping: linked list → Vec. Public field. Doc score 2 (what it is; usage context is in module and `StatusLine` docs). |
| `NewStatusDef` (constructor) | 538 | EQUIVALENT | OK | `StatusDef::list() -> StatusDefListBuilder` + `.def_all()` / `.def_one_of()` / `.build()` | 3 | C++: `NewStatusDef(AMin, AMax)` — a macro/function that creates a `TStatusDef` node setting `min` and `max`, returning a reference for chaining via `operator+`. Rust: replaced by the fluent `StatusDefListBuilder` — `StatusDef::list().def_all(|d| …).build()`. The builder pattern is the idiomatic Rust analog of the C++ operator+ chain. Fully documented including the "escape hatch" `.def()` method. |
| `operator+` (C++ builder chain) | — | EQUIVALENT | OK | `StatusDefListBuilder` chaining (`.def_all`, `.def_one_of`, `.def`) | 2 | C++ `operator+(TStatusDef&, TStatusItem&)` and `operator+(TStatusDef&, TStatusDef&)` compose the definition chain. Rust: all composition is via the builder; the operators have no direct Rust counterpart. Known idiomatic mapping. |

## Summary

- PORTED: 0   EQUIVALENT: 6   NOT-PORTED: 0   MISSING: 0   UNSURE: 0
- SUSPECT: 0   |   doc<3 (public): 3   |   → concept: 0
- Notable findings: No gaps or suspect items. The most significant design change is `min`/`max` integer range → `HelpCtxRange` enum (deviation D1): because `HelpCtx` is now a namespaced `&'static str` rather than an integer, a numeric `[min, max]` interval has no meaning; the replacement is `All` (universal) or `OneOf` (explicit set). This is documented in the module doc but the `range` field rustdoc could explicitly reference the C++ `min`/`max` heritage for readers coming from the guide.
