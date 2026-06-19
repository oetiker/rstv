# TParamText  (guide pp. 499–501)

Rust module(s): src/widgets/static_text.rs   |   magiblot: include/tvision/dialogs.h / source/tvision/tparamte.cpp

> TParamText adds runtime-settable text to TStaticText. The 1992 guide describes
> a Pascal `FormatStr`-based API (`ParamCount` + `ParamList` pointer). The
> magiblot C++ port replaced this with a `vsnprintf`-style `setText(fmt, ...)`
> and a 256-byte `str` buffer. The Rust port replaces that with `set_text(String)`
> (format at call site) and an unbounded `String`. Both the guide's Pascal fields
> and the magiblot C++ field (`str`) are covered below.

| Guide entry | Pg | Bucket | Corr | Rust symbol / mapping | Doc | Notes |
|---|---|---|---|---|---|---|
| `ParamCount` (field) | 499 | NOT-PORTED | — | — | — | Pascal-era `FormatStr` parameter count. The magiblot C++ port already removed this field, replacing `FormatStr` with `vsnprintf`; the Rust port follows magiblot, not the 1992 guide, on this point. Intentional: magiblot is the source of truth. |
| `ParamList` (field) | 499 | NOT-PORTED | — | — | — | Pascal-era untyped pointer to formatted-parameter array. Same rationale as `ParamCount`; eliminated by the `vsnprintf` replacement in magiblot. |
| `str` (field, magiblot) | — | EQUIVALENT | OK | `ParamText.inner: StaticText` (whose `text: String` holds the formatted content) | N/A | magiblot's 256-byte `char* str` maps to the `String` inside the embedded `StaticText`. No 256-byte cap (documented in struct doc). Private. |
| `Init` (constructor) | 499 | PORTED | OK | `tv::ParamText::new(bounds: Rect) -> ParamText` | 2 | Guide: calls `TStaticText::Init` with `AText` (a format template), stores `AParamCount`. magiblot: `TStaticText(bounds, 0)` then allocates `str[256]`. Rust: `ParamText::new(bounds)` with empty inner text. The format-template argument is gone (format at call site); documented in struct doc. Missing "how/when to use" detail in `new` rustdoc. |
| `Load` (constructor) | 500 | NOT-PORTED | — | — | — | `TStreamable` / stream persistence dropped project-wide. |
| `DataSize` (method) | 500 | NOT-PORTED | — | — | — | C++: returns `ParamCount * SizeOf(Longint)` — the size of the `ParamList` block for `getData`/`setData`. The D10 value protocol replaces the raw `getData`/`setData` buffer approach; `ParamText` has no `FieldValue` to expose, so `DataSize` has no analog. |
| `GetText` (method) | 500 | EQUIVALENT | OK | formatting is caller's responsibility via `format!(…)` + `set_text` | N/A | C++ `getText(var S)` runs `FormatStr(S, Text^, ParamList^)` to merge parameters into the format template. magiblot does the same via `vsnprintf`. Rust: the caller formats with `format!(…)` and calls `set_text`; no `getText` needed. Documented in struct doc and `set_text` doc. Not a public symbol in its own right; N/A rustdoc. |
| `SetData` (method) | 500 | NOT-PORTED | — | — | — | Reads `DataSize` bytes into `ParamList` from a raw record. Replaced by `set_text(String)` at the call site; the raw-buffer D10 path is not needed for a text widget. Intentional. |
| `Store` (method) | 500 | NOT-PORTED | — | — | — | `TStreamable` / stream persistence dropped project-wide. |
| `setText` (method, magiblot) | — | EQUIVALENT | OK | `tv::ParamText::set_text(text: impl Into<String>)` | 2 | magiblot's `setText(fmt, ...)` calls `vsnprintf(str, 256, fmt, ap)` then `drawView()`. Rust's `set_text` stores the already-formatted `String`; the next pump cycle redraws. 256-byte cap gone (documented). Missing explicit note in rustdoc that the caller is responsible for formatting (only hinted via `format!(…)` example in struct doc). |
| `getTextLen` (method, magiblot) | — | PORTED | OK | `tv::ParamText::text_len() -> usize` | 2 | magiblot: `strlen(str)`. Rust: `self.inner.text().len()` (byte count). The struct doc explicitly notes byte vs. display-column semantics for multi-byte UTF-8 — a documented deviation. Missing "when to use" in the method rustdoc itself. |
| `CStaticText` palette (1 entry) | 500 | EQUIVALENT | OK | `tv::theme::Role::StaticText` (inherited from `StaticText` draw) | N/A | Guide: TParamText uses `CStaticText`, same as TStaticText (entry 6 in dialog palette). Rust inherits `StaticText::draw` via delegate, which uses `Role::StaticText`. Correct. N/A (not a public symbol). |

## Summary

- PORTED: 2   EQUIVALENT: 4   NOT-PORTED: 6   MISSING: 0   UNSURE: 0
- SUSPECT: 0   |   doc<3 (public): 3   |   → concept: 0
- Notable findings: The high NOT-PORTED count is expected — five entries are the Pascal `FormatStr`/`ParamList` machinery and stream persistence, all eliminated by design following magiblot. No gaps. The most actionable doc nudge: `set_text` and `text_len` lack a "when to use" paragraph; `set_text` should explicitly state that the caller formats via `format!(…)` rather than passing a format string.
