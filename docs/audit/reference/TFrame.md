# TFrame  (guide pp. 443–445)

Rust module(s): src/frame.rs   |   magiblot: include/tvision/views.h / source/tvision/tframe.cpp

> TFrame has **no own fields** documented by the guide — it inherits all fields
> from TView. The guide documents only 5 methods and one palette entry.

| Guide entry | Pg | Bucket | Corr | Rust symbol / mapping | Doc | Notes |
|---|---|---|---|---|---|---|
| `Init` (constructor) | 444 | PORTED | OK | `tv::Frame::new(bounds: Rect) -> Frame` | 2 | Guide says sets `GrowMode` to `gfGrowHiX + gfGrowHiY` and `EventMask |= evBroadcast`. Rust `new` sets `grow_mode.hi_x / hi_y = true`. `evBroadcast` mask is implicit (frame receives broadcasts unconditionally per module doc). Matches. "how/when to construct" could be expanded. |
| `Draw` (method) | 444 | PORTED | OK | `tv::Frame::draw` (impl `View::draw`) | 3 | Guide: draws border with state-dependent colours and icons (active/inactive/dragging), title from owner. Rust: full draw impl; state → role family; title/flags/number pushed down (deviation D3, documented). Double-line active / single-line passive/dragging. All icon cases (close, zoom, unzoom, resize, drag-left) handled. Palette deviation to `Role`-keyed theme is D7, documented. |
| `GetPalette` (method) | 444 | EQUIVALENT | OK | `tv::Frame::palette() -> WindowPalette` + `tv::Theme` role mapping | 2 | C++ returns `CFrame` palette (5 entries, indices 1–5 map to first 3 window palette slots). Rust uses `WindowPalette` enum pushed down by owner + `Role::Frame*` / `Role::FrameGray*` / `Role::FrameCyan*` selected at draw time — same three colour families, different shape. Known idiomatic mapping: class Palette → `tv::Theme` (D7). Public getter documented (what), but "how the role families map to the three window colour schemes" could be clearer. |
| `HandleEvent` (method) | 444 | PORTED | OK | `tv::Frame::handle_event` (impl `View::handle_event`) | 3 | Guide: mouse events — close icon → `cmClose`, zoom icon or double-click top row → `cmZoom`, drag top row → move window, drag lower-right → resize. Rust handles close (with release-confirm via `MouseTrackCapture`, deviation D3 push-down), zoom click and double-click, title-bar drag and bottom-corner grow left/right unconsumed for Window. Middle-button interior move also unconsumed for Window. All cases covered; all deliberate deviations commented. |
| `SetState` (method) | 444 | PORTED | OK | `tv::view::Group::set_state` propagation (no override in `Frame`) | 2 | C++ `TFrame::setState` calls `TView::setState` then redraws if `sfActive` or `sfDragging` changed. Rust Frame does NOT override `set_state`; instead, `Group::set_state` propagates state flags to children (incl. the frame), and the redraw is triggered by the whole-tree redraw on every pump tick (deviation D9, documented in module doc: "active/dragging state arrives through normal `View::set_state` propagation that `Group` drives"). Functionally equivalent; the comment in the module doc is the documentation of this deliberate deviation. Doc score 2 — the how/why of not overriding could be made explicit. |
| `CFrame` palette (5 entries) | 445 | EQUIVALENT | OK | `tv::theme::Role::FramePassive`, `FrameActive`, `FrameDragging`, `FrameIcon` (+ Cyan/Gray families) | 2 | Guide: `CFrame` palette maps indices 1–5 to first three slots of window palette (passive frame, passive title, active frame, active title, icons). Rust expands this to three named families keyed by `WindowPalette` (Blue/Cyan/Gray), each with Active/Passive/Dragging/Icon roles. Known idiomatic mapping: class Palette → `tv::Theme`. Module doc describes the mapping but the `Role` enum items themselves score 2 (what they are, not how they compose). |

## Summary

- PORTED: 4   EQUIVALENT: 2   NOT-PORTED: 0   MISSING: 0   UNSURE: 0
- SUSPECT: 0   |   doc<3 (public): 3   |   → concept: 0
- Notable findings: No gaps or suspect items. The one non-obvious design point — Frame not overriding `set_state`, relying on Group propagation + whole-tree redraw instead — is documented in the module doc but the `SetState` mapping could carry a short explicit note in its own rustdoc to help readers who come looking for the C++ override.
