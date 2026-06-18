# TRadioButtons  (guide pp. 514–516)

Rust module(s): `src/widgets/cluster.rs`   |   magiblot: `include/tvision/dialogs.h` / `source/tvision/tradiobu.cpp`

> TRadioButtons is a concrete cluster where exactly one button is selected at
> any time. `Value` is the index of the selected (pressed) button; selecting a
> new button automatically deselects the previous one. The Rust port is a thin
> embed-and-delegate wrapper (`RadioButtons { cluster: Cluster }`) over the
> shared `Cluster` engine with `ClusterKind::RadioButtons` (deviation D2).

| Guide entry | Pg | Bucket | Corr | Rust symbol / mapping | Doc | Notes |
|---|---|---|---|---|---|---|
| `Draw` (method) | 515 | PORTED | OK | `Cluster::draw` (`ClusterKind::RadioButtons` arm: icon `" ( ) "`, marker `'•'`) | 2 | Guide: draws ` ( ) ` box. Rust: `kind.icon()` returns `" ( ) "`, `marker_char` returns `'\u{2022}'` (Unicode bullet, CP437 0x07 analog). Faithful. Doc at `Cluster::draw` level describes the behavior; `RadioButtons` struct doc is brief. |
| `Mark` (method) | 515 | PORTED | OK | `Cluster::mark` (`ClusterKind::RadioButtons` arm: `item == value as i32`) | 2 | Guide: returns `True` if `Item = Value`. Rust: identical (`item == self.value as i32`). |
| `MovedTo` (method) | 515 | PORTED | OK | `Cluster::moved_to` (`ClusterKind::RadioButtons` arm: `value = item as u32`) | 2 | Guide: assigns `Item` to `Value`. Rust: `self.value = item as u32` — identical. Called on arrow-key navigation so moving the selection also changes `value` (radio-button semantics). |
| `Press` (method) | 515 | PORTED | OK | `Cluster::press` (`ClusterKind::RadioButtons` arm: `value = item as u32`) | 2 | Guide: assigns `Item` to `Value`. Rust: identical. |
| `SetData` (method) | 515 | EQUIVALENT | OK | cluster opt-out of D10 value protocol (module doc) | 2 | Guide: calls `TCluster.SetData` to set `Value`, then sets `Sel = Value` (so the selection bar starts at the pressed button). Rust: clusters opt out of the `value()`/`set_value()` value protocol (deviation D10, documented in module doc). The `sel = value` initialization for RadioButtons on load is not explicitly present — callers who set `cluster.value` directly must also set `cluster.sel` manually. This is a minor undocumented usage constraint (not `SUSPECT` because clusters are expected to be used via constructors, not stream-loaded). |
| `CCluster` palette | 515 | EQUIVALENT | OK | `Role::ClusterNormal/ClusterSelected/ClusterNormalShortcut/ClusterSelectedShortcut/ClusterDisabled` | 2 | Guide: uses `CCluster`, same as TCluster/TCheckBoxes. Rust: same `Role`-keyed theme lookup. Known idiomatic mapping: class Palette → `tv::Theme`. |
| `Init` / `Load` / `Done` (constructors) | 514 | EQUIVALENT / NOT-PORTED | — | `RadioButtons::new(bounds, strings)` / NOT-PORTED | 2 / — | `Init` maps to `RadioButtons::new` (calls `Cluster::new(..., ClusterKind::RadioButtons)`). `Load`/`Done` not ported (`TStreamable` dropped). |

## Summary

- PORTED: 4   EQUIVALENT: 2   NOT-PORTED: 1   MISSING: 0   UNSURE: 0
- SUSPECT: 0   |   doc<3 (public): 5   |   → concept: 0
- Notable finding: The `SetData` semantics include `Sel = Value` initialization, which ensures the selection bar starts on the pressed button after a data load. The Rust port opts out of the value protocol entirely, meaning direct `cluster.value` mutation without matching `cluster.sel` mutation would leave the visual cursor misplaced. This usage constraint is not documented anywhere in the public API.
