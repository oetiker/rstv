# TMultiCheckBoxes  (guide pp. 486–488)

Rust module(s): `src/widgets/cluster.rs`   |   magiblot: `include/tvision/dialogs.h` / `source/tvision/tmulchkb.cpp`

> TMultiCheckBoxes is a concrete cluster where each item cycles through
> `SelRange` distinct states (not just on/off). `Value` packs multiple n-bit
> state fields: `Flags` (a `cfXXXX` constant) specifies how many bits each
> item occupies; `States` is a string of marker characters, one per state.
> The Rust port is a thin embed-and-delegate wrapper (`MultiCheckBoxes {
> cluster: Cluster }`) over the shared `Cluster` engine with
> `ClusterKind::MultiCheckBoxes { sel_range, flags, states }` (deviation D2).

| Guide entry | Pg | Bucket | Corr | Rust symbol / mapping | Doc | Notes |
|---|---|---|---|---|---|---|
| `Flags` (field) | 486 | PORTED | OK | `ClusterKind::MultiCheckBoxes { flags: u16, .. }` | 2 | Guide: `Flags: Word` — one of the `cfXXXX` constants, encodes bits-per-item. Rust: `flags: u16` inside the enum variant. The packing interpretation (`lo = flags & 0xff` = per-item mask, `hi = flags >> 8` = per-item bit-shift multiplier) is documented in the field comment. |
| `SelRange` (field) | 486 | PORTED | OK | `ClusterKind::MultiCheckBoxes { sel_range: u8, .. }` | 2 | Guide: `SelRange: Byte` — number of states each item can assume. Rust: `sel_range: u8`. Identical semantics. |
| `States` (field) | 486 | PORTED | OK | `ClusterKind::MultiCheckBoxes { states: String, .. }` | 2 | Guide: `States: PString` — pointer to string of marker characters, one per state. Rust: `states: String` (owned). Doc explains "marker glyph for each state value". |
| `Init` (constructor) | 486 | PORTED | OK | `MultiCheckBoxes::new(bounds, strings, sel_range, flags, states)` | 2 | Guide: `Init(var Bounds; AStrings; ASelRange; AFlags; const AStates)` — sets `SelRange`, `Flags`, copies `AStates`. Rust: `MultiCheckBoxes::new` passes all five parameters to `Cluster::new(..., ClusterKind::MultiCheckBoxes{..})`. Takes `Vec<String>` instead of `PSItem` list. Faithful. |
| `Load` (constructor) | 487 | NOT-PORTED | — | — | — | `TStreamable` dropped crate-wide. |
| `Done` (destructor) | 487 | NOT-PORTED | — | — | — | Rust `Drop` handles memory automatically; no explicit destructor. |
| `DataSize` (method) | 487 | EQUIVALENT | OK | cluster opt-out of D10 value protocol | 2 | Guide: returns `SizeOf(Longint)`. Rust: clusters opt out of the value protocol entirely (D10, documented in module doc). |
| `Draw` (method) | 487 | PORTED | OK | `Cluster::draw` (multi branch via `multi_mark` + `states` string) | 2 | Guide: draws ` [ ] ` box, uses characters in `States` as markers. Rust: `marker_char` calls `multi_mark`, indexes `states` string. Overflow guard (`checked_shl`) for items ≥ 16 with 2-bit packing documented and tested. |
| `GetData` (method) | 487 | EQUIVALENT | OK | cluster opt-out of D10 value protocol | 2 | Guide: typecasts `Rec` as `Longint`, copies `Value` into it. Rust: excluded from value protocol (module doc). Known idiomatic mapping. |
| `MultiMark` (method) | 487 | PORTED | OK | `Cluster::multi_mark(item: i32) -> usize` | 2 | Guide: returns state of item-th check box as a `Byte`. Rust: returns `usize`. Same packed-state read (`(value & (flo << fhi)) >> fhi`). Overflow guard documented. |
| `Press` (method) | 487 | PORTED | OK | `Cluster::press` (`ClusterKind::MultiCheckBoxes` arm: cycles `0 → 1 → … → sel_range-1 → 0`) | 2 | Guide: "changes the state of the Item-th check box … cycles through all available states." Rust: identical cycle logic with `checked_shl` overflow guard for items beyond the 32-bit cap. |
| `SetData` (method) | 488 | EQUIVALENT | OK | cluster opt-out of D10 value protocol | 2 | Guide: typecasts `Rec` as `Longint`, copies its value into `Value`, calls `DrawView`. Rust: excluded from value protocol (module doc). |
| `Store` (method) | 488 | NOT-PORTED | — | — | — | `TStreamable` dropped crate-wide. |

## Summary

- PORTED: 7   EQUIVALENT: 3   NOT-PORTED: 3   MISSING: 0   UNSURE: 0
- SUSPECT: 0   |   doc<3 (public): 8   |   → concept: 0
- Notable finding: The `cfXXXX` constants (`cfOneBit`, `cfTwoBits`, etc.) that the guide references as valid `Flags` values are not ported — callers must construct the raw `flags: u16` word manually. This is not flagged anywhere in the `MultiCheckBoxes::new` rustdoc, which only describes `flags` as "packed item layout" without listing the named constants. A usage example or list of standard values would raise the doc score to 3.
