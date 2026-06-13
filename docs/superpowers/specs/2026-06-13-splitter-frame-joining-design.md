# Splitter Frame-Joining — design

**Date:** 2026-06-13
**Status:** Proposed (for review) — v2, after a Turbo-Vision-mindset review
**Builds on:** `Splitter` (rstv-original extension) — spec
[`2026-06-13-splitter-design.md`](2026-06-13-splitter-design.md), plan
[`2026-06-13-splitter.md`](../plans/2026-06-13-splitter.md). Implemented on branch
`feat/splitter`.

## Goal

Make a `Splitter` embedded in a window look like **one continuous piece of
linework**: its divider lines connect to the surrounding window frame and to each
other with proper box-drawing junctions, instead of floating as bare lines that
stop one cell short of the frame.

Target — a grid (`tree` sidebar column; the right side split into stacked
`list` / `form` rows) embedded in a passive window:

```
┌────────┬────────────┐
│ tree   │ list       │
│        │            │
│        ├────────────┤
│        │ form       │
│        │            │
└────────┴────────────┘
```

- `┬`/`┴` — the **outer** vertical divider meets the top/bottom frame.
- `┤` — the **inner** horizontal divider meets the **right** frame.
- `├` — the inner divider meets the **outer** vertical divider.

A focused window draws a **double** frame; a single-line divider still connects
cleanly through **mixed** junctions (`╤ ╧ ╞ ╡`), so a divider never has to change
weight:

```
╔════════╤════════════╗
║ tree   │ list       ║
║        ╞════════════╣
║        │ form       ║
╚════════╧════════════╝
```

## Background — what Turbo Vision actually does

Classic Turbo Vision **does** join framed linework, in `TFrame`. `TFrame::frameLine`
(`magiblot-tvision/source/tvision/framelin.cpp`) builds each frame row as a
**per-cell arm bitmask** (`FrameMask`), seeded from the base frame bits, then
**walks the owner's subviews** (`owner->last->next`) and, for every `ofFramed`
visible sibling abutting the line, ORs junction bits into the mask; finally it
maps `frameChars[mask]` → the box glyph (`┬ ┴ ├ ┤ ┼`). So TV composes connected
linework **from a bitmask the frame owns**; it never reads the screen back.

rstv **dropped this** behavior (documented at `src/frame.rs:50-55`) for a specific
reason: `frameLine` has the *frame reach sideways to read its siblings'
`origin`/`size`, which is exactly what deviation **D3 (owner-data-down — no
sideways pointers)** forbids. So today rstv draws plain corners and edges.

**Implications for this feature:**
- The **divider→frame** join is a **faithful revival** of TV's `frameLine`
  tee-walk — re-expressed in a D3-legal way (below). Not a new idea.
- The **divider→divider** interior cross (`├`/`┼`) is **rstv-original**: TV has no
  splitter, so there is no interior cross to be faithful to.

(An earlier v1 of this spec wrongly claimed TV does no joining and proposed a
whole-buffer read-back pass. The TV-mindset review corrected the premise and
rejected the read-back as re-introducing exactly the screen-inspection that D8
deleted. This v2 is the corrected design.)

## The D3 inversion (the key idea)

TV's `frameLine` is the frame **pulling** sibling geometry sideways. D3 forbids a
child reaching sideways, so we **invert the data flow**: the **owning window**
(the common parent of its frame and its splitter) reads the divider positions from
its splitter child (parent→child, allowed) and **pushes them down to the frame**
as data (owner-data-down, allowed). The frame then composes its line exactly like
`frameLine` — same algorithm, same visual result — but fed by pushed data instead
of a sideways walk. No child reaches sideways; nothing reads the screen back.

The one net-new piece, the interior `├`/`┼`, is composed by the **outer splitter
during its own draw**, because the outer splitter *owns* the inner sub-splitter as
a pane child and can read its divider positions as owner-data — again parent→child,
local, no read-back.

## Scope

**In scope:** a window may opt in to joining the linework of the splitter(s) it
hosts — divider→frame and divider→divider (including nested grids).

**Non-goals:**
- No change to any window that does not opt in. Plain windows, dialogs, the file
  dialog, message boxes — byte-for-byte unchanged.
- **No buffer read-back / screen inspection** (rejected by review as re-adding
  what D8 deleted).
- **No new `View` trait method.** Producers are concrete (`Splitter`, `Frame`);
  the window reaches them by the existing `as_any` downcast (the same mechanism
  Window already uses to push the zoom flag to its Frame).
- No coupling of divider line-*weight* to window focus. Dividers keep their
  natural weight; mixed junction glyphs bridge single↔double.
- No new "split window" constructor (YAGNI).
- No global / whole-screen auto-join (would merge unrelated overlapping frames).

## Design overview — two composition sites, both owner-data-down

```
Window::draw (only when joined_lines):
  1. marks = interior_splitter.frame_junction_marks(frame_bounds)   // parent→child
  2. frame_child.set_junction_marks(marks)                          // owner-data-down
  3. self.group.draw(ctx)        // Frame composes tees from marks (faithful
                                 // frameLine); Splitter draws panes + dividers;
                                 // outer Splitter overlays its own ├/┼ crossings
```

- **Site 1 — Frame (divider→frame):** the Frame gains the `frameLine`-style
  composition: when emitting an edge cell that carries a junction mark, it
  substitutes the matching tee glyph (chosen from the frame's own weight × the
  mark's stem weight). Marks are pushed by the window each draw, computed from the
  splitter's current layout (so they track drags/resizes).
- **Site 2 — Splitter (divider→divider):** while drawing its own dividers, the
  outer splitter inspects each adjacent pane; if a pane is itself a `Splitter`
  with perpendicular dividers, it overlays `├`/`┤`/`┼` on its own divider cell at
  those positions (weight-correct), instead of the plain `│`/`─`.

Nothing reads the buffer; nothing reaches sideways; no universal trait grows.

## Components

### 1. `Glyphs` — complete the junction set (`src/theme.rs`, D7)

Single-line junctions already exist (`frame_tee_l ├`, `frame_tee_r ┤`,
`frame_tee_t ┬`, `frame_tee_b ┴`, `frame_cross ┼`). Add, seeded into every theme's
`Glyphs` like the other frame glyphs:

- **Double:** `frame_tee_t_d ╦` (U+2566), `frame_tee_b_d ╩` (U+2569),
  `frame_tee_l_d ╠` (U+2560), `frame_tee_r_d ╣` (U+2563), `frame_cross_d ╬`
  (U+256C).
- **Mixed — double bar / single stem:** `frame_tee_t_dh ╤` (U+2564),
  `frame_tee_b_dh ╧` (U+2567), `frame_tee_l_dv ╞` (U+255E), `frame_tee_r_dv ╡`
  (U+2561), and crosses `frame_cross_dh ╪` (U+256A), `frame_cross_dv ╫` (U+256B).
- **Mixed — single bar / double stem** (only when a reconfig-double divider meets
  a passive single frame — an edge case; include for completeness):
  `frame_tee_t_sh ╥` (U+2565), `frame_tee_b_sh ╨` (U+2568), `frame_tee_l_sv ╟`
  (U+255F), `frame_tee_r_sv ╢` (U+2562).

Naming extends the existing `frame_*` convention (`_d` = both double; `_dh`/`_dv`
= double bar with single perpendicular stem; `_sh`/`_sv` = single bar with double
stem).

### 2. The pure junction-glyph selector (unit-testable, no view deps)

```rust
/// Pick the box-drawing junction for an edge cell. `edge` = which frame edge
/// (Top/Bottom/Left/Right); `bar` = the frame line's weight (Single/Double);
/// `stem` = the abutting divider's weight. Returns the matching `Glyphs` field.
fn frame_junction(edge: Edge, bar: Weight, stem: Weight, g: &Glyphs) -> char;

/// Pick the interior junction where two dividers meet. `through` = the weight of
/// the divider being drawn; `branch` = directions+weights of meeting dividers.
fn divider_junction(...) -> char;
```

`Weight = { Single, Double }`, `Edge = { Top, Bottom, Left, Right }`. These are
small finite maps to the `Glyphs` fields above — the rstv-local equivalent of TV's
`frameChars[mask]` table. They are the only "logic" and get exhaustive unit tests.

### 3. `Frame` junction marks + mark-aware draw (`src/frame.rs`)

```rust
pub struct JunctionMark {
    pub edge: Edge,    // which frame edge this lands on
    pub offset: i32,   // frame-local position along that edge
    pub stem: Weight,  // the abutting divider's weight
}

impl Frame {
    /// Owner-data-down: the owning window pushes the divider abutment marks the
    /// frame should join into its border. Replaced each draw; empty = today's
    /// plain frame (so non-joined windows are unchanged).
    pub(crate) fn set_junction_marks(&mut self, marks: Vec<JunctionMark>);
}
```

`Frame::draw` is extended: as it emits each border cell, if a mark matches that
edge+offset it writes `frame_junction(edge, self_weight, mark.stem, glyphs)`
instead of the plain edge/corner glyph (`self_weight` = Double when active, Single
otherwise — the frame already branches on this). With no marks, the output is
identical to today. This is the faithful `frameLine` composition, minus the
forbidden sideways walk (the data arrives pre-computed).

### 4. `Window` — opt-in flag + brokering draw override (`src/window/window.rs`)

- Add `joined_lines: bool` (default `false`) + builder `with_joined_lines(self)
  -> Self` / setter.
- Override `draw` (Window currently delegates it to its group): when
  `joined_lines`, (a) find the interior splitter child via `as_any` downcast,
  (b) ask it for `frame_junction_marks(frame_bounds)`, (c) push them to the Frame
  child via `set_junction_marks`, then (d) draw the group as usual. When the flag
  is off (or there is no splitter child), it is behaviorally identical to the
  delegated draw — existing window snapshots must not change.

```rust
fn draw(&mut self, ctx: &mut DrawCtx) {
    if self.joined_lines {
        let fb = self.frame_bounds();
        if let Some(marks) = self.interior_splitter()      // as_any downcast
                                   .map(|s| s.frame_junction_marks(fb)) {
            if let Some(frame) = self.frame_mut() {         // child[0], typed
                frame.set_junction_marks(marks);
            }
        }
    }
    self.group.draw(ctx);
}
```

Marks come from layout state (divider positions), which is valid before drawing —
so computing them at the top of `draw` is fine and always current. (The window
already reaches its Frame concretely to push the zoom flag; this reuses that
parent→child channel.)

### 5. `Splitter` — frame marks + interior crossings (`src/widgets/splitter/mod.rs`)

- `pub(crate) fn frame_junction_marks(&self, frame_bounds: Rect) -> Vec<JunctionMark>`:
  for each of this splitter's dividers, if its end abuts the given frame edge,
  emit a mark (edge, frame-local offset, this divider's weight). **Recurses into
  pane sub-splitters** (a nested splitter's divider that reaches the outer frame
  contributes its own mark), translating coordinates into frame-local space. A
  `Hidden`/`Locked` divider that drew nothing emits no mark. Pure function of
  layout — no drawing, unit-testable.
- **Interior crossings in `draw_dividers`:** after drawing a divider, for each
  adjacent pane that is a `Splitter` (via `as_any` downcast) with perpendicular
  dividers, overlay the correct tee/cross (`divider_junction`) on this divider's
  own cell at the meeting position (e.g. an inner `rows` splitter on the right of
  an outer vertical divider → `├` at the inner divider's row). Weight-correct;
  draws only on *this* splitter's own cells.

### 6. Example (`examples/splitter.rs`)

Rework the demo into the grid — `Splitter::cols([tree, Splitter::rows([list,
form])])` sized into the interior of a `Window::…with_joined_lines()`. Must build
and run; shows `┬ ┴ ┤` against the frame and the interior `├`.

## Data flow

```
Window::draw (joined_lines):
  interior_splitter.frame_junction_marks(frame_bounds)   // recursive, layout-only
      → [JunctionMark{edge, offset, stem}, …]
  frame_child.set_junction_marks(marks)                  // owner-data-down
  group.draw:
     Frame::draw   → composes border, substituting frame_junction(...) at marks
     Splitter::draw→ panes + dividers; outer splitter overlays divider_junction(...)
  renderer diff/flush (unchanged)
```

All coordinates are **owner-local** (frame-local marks; splitter-local crossings),
consistent with the downward `DrawCtx` convention — no absolute/screen coords, no
read-back.

## Weight handling

The junction glyph is a function of (frame weight, divider weight): passive single
frame + single divider → `┬`; active double frame + single divider → `╤`; double
frame + double (reconfig) divider → `╦`; the rare passive+double → `╥`. The window
passes each divider's current weight in the mark, and the frame knows its own
weight — so the correct (possibly mixed) glyph is chosen with no view needing the
other's focus state.

## Edge cases

- **Hidden/Locked divider:** draws no line, emits no mark → frame edge unchanged.
- **Reconfig mode** (divider drawn double): the mark carries `Double`, so the
  frame joins with `╦/╩` (active) or `╥/╨` (passive) — still correct.
- **Splitter inset from the frame** (a margin): the divider end does not abut the
  frame edge, so no mark is emitted → nothing joins (correct).
- **Window not containing a splitter:** `interior_splitter()` is `None` → no marks
  → unchanged.

## Testing (D11)

- **Pure unit tests** for `frame_junction` / `divider_junction`: every (edge, bar,
  stem) and crossing combination → expected `Glyphs` field.
- **`Splitter::frame_junction_marks`** unit tests: a 2-pane cols splitter abutting
  a frame yields the two correct top/bottom marks; a nested grid yields the
  expected set including the inner divider's right-edge mark.
- **Snapshot tests** (HeadlessBackend) on a small `Window::with_joined_lines`:
  passive single frame → `┬…┴`; active double frame → `╤…╧`; the grid → interior
  `├` + `┤` to the right frame.
- **Regression:** an existing window snapshot WITHOUT the flag is unchanged.
  Because **no `View` trait method is added**, `tvision-macros/src/specs.rs` and
  the `delegate_view` spy test need **no change** (a plus of this design over v1).

## Alternatives considered

- **Whole-buffer read-back pass (v1 of this spec).** A renderer/window post-pass
  that reads painted cells (`DrawCtx::get_char`) and upgrades line stubs. Rejected
  by the TV-mindset review: it re-introduces the screen inspection D8 deliberately
  removed (`drawUnderView`/per-view back buffers), widens `DrawCtx`'s write-only
  contract, and does cross-view work in `draw` with ad-hoc rules. The mask
  composition above is how `frameLine` already thinks and needs no read-back.
- **`View::line_join_cells()` on the universal trait.** Rejected: a presentation
  concern leaking into a structural/lifecycle trait; the producers are concrete
  types reachable by the existing `as_any` downcast (the window→frame precedent).
- **Global whole-screen auto-join.** Rejected: merges unrelated overlapping window
  frames.

## Future (not in this spec)

- Make dividers **track the frame weight** (single passive / double active) for a
  fully unified look — needs the window's active state to reach the splitter;
  deferred because mixed glyphs already read seamlessly.
- **Auto-enable** joining when a `Splitter` is detected as a window's body (drop
  the explicit flag) — only if the flag proves annoying.
- A `Role::Splitter*` theme entry set if dividers should be themable independently
  of the frame roles (today they reuse `FramePassive`/`FrameDragging`).
- Reviving the **full** `frameLine` `ofFramed`-sibling tee-walk generally (any
  framed sub-view, not just splitters) — a larger, separate effort; this spec
  deliberately scopes to splitter dividers.

## Methodology note

The divider→frame join **restores a genuine Turbo Vision behavior** (`frameLine`'s
tee-walk) that rstv shelved under D3, re-expressed D3-legally as owner-data-down —
so a tvision veteran recognizes it on sight. The divider→divider interior cross is
an rstv-original extension (TV has no splitter). The whole feature is **gated and
additive** — nothing changes unless a window opts in — so it carries no risk to the
faithful baseline.
