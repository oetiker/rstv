# TMemo  (guide pp. 475–477)

Rust module(s): src/widgets/editor.rs (`struct Memo`)   |   magiblot: include/tvision/editors.h / source/tvision/tmemo.cpp

> TMemo is a subclass of TEditor that adds dialog-data exchange (`getData`/`setData`/`dataSize`) and
> palette (`getPalette`), and overrides `handleEvent` to swallow the Tab key.
> No TMemo-specific fields beyond what TEditor provides.

| Guide entry | Pg | Bucket | Corr | Rust symbol / mapping | Doc | Notes |
|---|---|---|---|---|---|---|
| `Init` (constructor) | 475 | PORTED | OK | `tv::Memo::new(bounds, h_scroll_bar, v_scroll_bar, indicator, buf_size)` | 2 | C++ `TMemo::TMemo` simply delegates to `TEditor`. Rust `Memo::new` does the same: calls `Editor::new(...)` and wraps it. Bounds, scroll-bar ids (as `Option<ViewId>`), indicator id, and buf_size all forwarded. Guide says fixed-size buffer; Rust `Editor::new` path (non-file-editor) is also fixed. Matches. "how/when to construct" not in doc. |
| `getData` (method) | 475 | EQUIVALENT | OK | `tv::Memo::value() -> Option<FieldValue>` (D10 value protocol) | 2 | C++: copies `bufLen` bytes from the gap buffer into a `TMemoData` record (length field + flat byte buffer). Rust: `Memo::value()` returns `FieldValue::Text(String)` by reconstructing the logical text via `Editor::text()`. Same semantic result — whole buffer content — expressed as the D10 typed value protocol instead of the untyped `getData`. Known idiomatic mapping. Doc explains what, not when/how to use. |
| `setData` (method) | 476 | EQUIVALENT | OK | `tv::Memo::set_value(FieldValue)` (D10 value protocol) | 2 | C++: calls `setBufSize(data->length)` then `memcpy` + `setBufLen`. Rust: `Memo::set_value` checks the variant is `FieldValue::Text`, then calls `Editor::set_text(bytes)` which grows-or-fits and calls `set_buf_len`. Functionally identical: whole-buffer replace. Non-`Text` variants are silently ignored (type mismatch), consistent with the D10 protocol for all controls. |
| `dataSize` (method) | 476 | EQUIVALENT | OK | size is implicit in `FieldValue::Text(String)` | N/A | C++: returns `bufSize + sizeof(ushort)` — the allocation for the record that `getData` fills. In D10 there is no separate size query; the `FieldValue` owns its allocation. No public counterpart needed. NOT public in Rust. |
| `getPalette` (method) | 476 | EQUIVALENT | OK | colors inherited from `tv::Editor` via `#[delegate(to = editor)]` → `Role::ScrollerNormal` / `Role::ScrollerSelected` | 2 | C++: `cpMemo = "\x1A\x1B"` (2 entries, same as editor normal/selected). Rust `Memo` has no `palette()` override; it delegates to `Editor`, which uses `Role::ScrollerNormal`/`ScrollerSelected` color lookup. The module doc says "reuses the editor's drawing and so its scroller colors; it carries no separate palette of its own." Functionally equivalent. Known idiomatic mapping: class Palette → `tv::Theme`. |
| `handleEvent` (method) | 476 | PORTED | OK | `tv::Memo::handle_event` (impl `View::handle_event` in `#[delegate]` block) | 3 | C++: `if (event.what != evKeyDown || event.keyDown.keyCode != kbTab) TEditor::handleEvent(event);` — swallows only plain Tab, forwarding all else. Rust: identical logic: returns early (without clearing) on an unmodified Tab `KeyDown`; all other events forwarded to `editor.handle_event`. Comment explains Shift/Ctrl/Alt+Tab ARE forwarded; test `memo_tab_swallowed_not_cleared` verifies the swallow-without-clear. Full match. |
| `TMemoData` type | 477 | EQUIVALENT | OK | `tv::data::FieldValue::Text(String)` | N/A | See dedicated TMemoData.md. Cross-reference only. |
| `Load` (stream constructor) | 477 | NOT-PORTED | — | — | — | `TStreamable` / stream machinery dropped project-wide (serde-if-revived). Known idiomatic mapping. |
| `Store` (stream method) | 477 | NOT-PORTED | — | — | — | Same: TStreamable dropped. Known idiomatic mapping. |

## Summary

- PORTED: 2   EQUIVALENT: 4   NOT-PORTED: 2   MISSING: 0   UNSURE: 0
- SUSPECT: 0   |   doc<3 (public): 4   |   → concept: 0
- Notable findings: No gaps or suspect items. The most important design point — `getData`/`setData`/`dataSize` collapsed into the D10 `value`/`set_value` typed-value protocol — is the single dominant mapping for this class. The `dataSize` method has no public counterpart (correct: allocation sizing is implicit in `FieldValue`), and this should be made explicit in the `Memo` rustdoc to help callers who come from the C++ API looking for it.
