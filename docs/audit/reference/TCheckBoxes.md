# TCheckBoxes  (guide pp. 393–395)

Rust module(s): `src/widgets/cluster.rs`   |   magiblot: `include/tvision/dialogs.h` / `source/tvision/tcheckbo.cpp`

> TCheckBoxes is a concrete cluster for independent on/off toggles. `Value` is
> a bitmask: bit `i` set ⇔ item `i` is checked. The Rust port is a thin
> embed-and-delegate wrapper (`CheckBoxes { cluster: Cluster }`) over the shared
> `Cluster` engine with `ClusterKind::CheckBoxes` (deviation D2).

| Guide entry | Pg | Bucket | Corr | Rust symbol / mapping | Doc | Notes |
|---|---|---|---|---|---|---|
| `Draw` (method) | 394 | PORTED | OK | `Cluster::draw` (dispatched via `#[delegate]`) | 2 | Guide: draws ` [ ] ` box, `X` marker when checked. Rust: `kind.icon()` returns `" [ ] "`, `marker_char` returns `'X'` for state 1. Guide notes TCheckBoxes does NOT override the TCluster constructor/destructor/event handler. Rust: confirmed — `CheckBoxes` has no own `draw` impl; it delegates to `Cluster::draw`. Doc score 2: what the draw does is described in the `Cluster::draw` rustdoc; TCheckBoxes itself has only the struct-level comment. |
| `Mark` (method) | 394 | PORTED | OK | `Cluster::mark` (`ClusterKind::CheckBoxes` arm: `value & (1 << item) != 0`) | 2 | Guide: returns `True` if item-th bit of `Value` is set; items 0–15. Rust: supports items 0–31 (bit-mask is `u32`; `item >= 32` returns `false`). Rust extends the guide's 16-item cap to 32-item faithfully matching magiblot's `uint32_t` field. |
| `Press` (method) | 394 | PORTED | OK | `Cluster::press` (`ClusterKind::CheckBoxes` arm: `value ^= 1 << item`) | 2 | Guide: toggles item-th bit of `Value`; items 0–15. Rust: XOR toggle, `item >= 32` is a no-op (overflow guard). Same semantic. |
| `CCluster` palette | 395 | EQUIVALENT | OK | `Role::ClusterNormal/ClusterSelected/ClusterNormalShortcut/ClusterSelectedShortcut/ClusterDisabled` via `DrawCtx` | 2 | Guide: TCheckBoxes uses `CCluster` (4 entries, same as TCluster). Rust: same `Role`-keyed theme lookup in `Cluster::draw`. Known idiomatic mapping: class Palette → `tv::Theme`. |
| `Init` (constructor) | 393 | EQUIVALENT | OK | `CheckBoxes::new(bounds: Rect, strings: Vec<String>)` | 2 | Guide: `Init(var Bounds; AStrings: PSItem)` — inherits TCluster constructor. Rust: `CheckBoxes::new` calls `Cluster::new(..., ClusterKind::CheckBoxes)`. Takes `Vec<String>` instead of linked `TSItem` list (idiomatic replacement). Doc explains what it builds. |
| `Load` / `Done` / stream methods | 393 | NOT-PORTED | — | — | — | `TStreamable` dropped crate-wide. Note: guide states "TCheckBoxes does not override TCluster constructors, destructor, or event handler" — consistent with Rust delegation. |

## Summary

- PORTED: 3   EQUIVALENT: 2   NOT-PORTED: 1   MISSING: 0   UNSURE: 0
- SUSPECT: 0   |   doc<3 (public): 4   |   → concept: 0
- Notable finding: The guide says items 0–15; magiblot extended to 32-bit mask; Rust faithfully follows magiblot (not the original guide limit). No gaps or suspect items. The `CheckBoxes` struct-level rustdoc is brief (what it is, TV heritage); it could carry a short usage example showing `value` as a bitmask.
