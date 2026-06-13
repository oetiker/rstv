# Splitter Frame-Joining Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a window opt in to making an embedded `Splitter`'s divider lines connect to the surrounding frame and to each other with proper box-drawing junctions (`┬ ┴ ├ ┤ ┼` and the double/mixed variants), instead of floating as bare lines.

**Architecture:** Faithful D3-legal revival of Turbo Vision's `TFrame::frameLine` tee-walk, **inverted** so data flows owner→child: the owning `Window` reads divider positions from its `Splitter` child (parent→child, allowed) and pushes them down to its `Frame` child as `JunctionMark`s (owner-data-down, allowed). The `Frame` substitutes tee glyphs at marked edge cells; the outer `Splitter` overlays interior `├`/`┼` crossings where a sub-splitter's perpendicular dividers meet its own. Nothing reads the screen back; no child reaches sideways; no new `View` trait method.

**Tech Stack:** Rust (workspace `tvision` + `tvision-macros`), `insta` snapshot tests on `HeadlessBackend`, the existing `#[delegate(to = …)]` proc-macro.

**Source spec:** [`docs/superpowers/specs/2026-06-13-splitter-frame-joining-design.md`](../specs/2026-06-13-splitter-frame-joining-design.md) (v4).

---

## Conventions for every task

- **Cargo target dir:** artifacts land in `/home/oetiker/scratch/cargo-target`, NOT `./target`. Every command below assumes `export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target` is set.
- **Parallelism cap:** never use more than 4 cores. Run cargo with `-j4` (or `--test-threads` for `insta`). The machine is shared.
- **Workspace:** use `--workspace` where it matters; clippy with `--all-targets -- -D warnings`.
- **Snapshot review:** new `insta::assert_snapshot!` calls write `.snap.new` files. Inspect with `cargo insta review` (or read the `.snap.new` content directly) and accept with `cargo insta accept` once the rendered text matches the spec's target diagrams. **Do not blind-accept.**
- **Commit trailer:** every commit message ends with the project's `Co-Authored-By` trailer (see `git log`).
- This work lives on branch `feat/splitter` in the worktree `/scratch/oetiker/claude-worktrees/rstv-splitter`.

---

## File Structure

New and modified files, each with one clear responsibility:

| File | Responsibility | New/Modify |
|------|----------------|------------|
| `src/junction.rs` | **New module.** The view-independent junction vocabulary: `Edge`, `Weight`, `JunctionMark` data types + the two pure glyph-selector functions `frame_junction` and `divider_junction`. Depends only on `crate::theme::Glyphs`. Exhaustively unit-tested. | Create |
| `src/lib.rs` | Add `pub mod junction;` and re-export `Edge`, `Weight`, `JunctionMark`. | Modify |
| `src/theme.rs` | Add the missing junction glyphs (double + mixed tees and crosses) to the `Glyphs` struct and its single `Default` impl. | Modify |
| `src/frame.rs` | `Frame` gains a `junction_marks: Vec<JunctionMark>` field, a `set_junction_marks` setter, and mark-aware border drawing. | Modify |
| `src/widgets/splitter/mod.rs` | `Splitter` gains the `as_any_mut` downcast keystone, `frame_junction_marks` (recursive, produces marks), and `draw_interior_crossings` (Site 2). | Modify |
| `src/window/window.rs` | `Window` gains `joined_lines` flag + `with_joined_lines` builder/setter, `interior_splitter_mut`/`frame_mut` helpers, and a `draw` override that brokers marks frame-ward. | Modify |
| `examples/splitter.rs` | Rework the demo into the grid (`cols([tree, rows([list, form])])`) inside a `Window::…with_joined_lines()`. | Modify |
| `docs/IMPLEMENTATION-LOG.md`, `docs/PORT-ORDER.md` | Changelog + tracking entry for the feature. | Modify |

---

## Task 1: Junction vocabulary + glyph table

Create the pure data types and add the missing glyphs. No view code yet.

**Files:**
- Create: `src/junction.rs`
- Modify: `src/lib.rs` (module decl + re-exports)
- Modify: `src/theme.rs` (`Glyphs` struct fields + `Default` impl)
- Test: inline `#[cfg(test)]` in `src/junction.rs` and `src/theme.rs`

- [ ] **Step 1: Add the new glyph fields to the `Glyphs` struct.**

In `src/theme.rs`, the `Glyphs` struct currently ends its frame-join section at `pub frame_cross: char,` (around line 552). Replace that single-line tee/cross block's trailing field group by inserting the new fields immediately after `pub frame_cross: char,`:

```rust
    // --- Frame glyphs — double-line tee/cross joins ---
    /// Double-line left tee `╠` (U+2560).
    pub frame_tee_l_d: char,
    /// Double-line right tee `╣` (U+2563).
    pub frame_tee_r_d: char,
    /// Double-line top tee `╦` (U+2566).
    pub frame_tee_t_d: char,
    /// Double-line bottom tee `╩` (U+2569).
    pub frame_tee_b_d: char,
    /// Double-line cross `╬` (U+256C).
    pub frame_cross_d: char,

    // --- Frame glyphs — mixed: double BAR, single perpendicular STEM ---
    /// Double-bar top tee, single stem `╤` (U+2564).
    pub frame_tee_t_dh: char,
    /// Double-bar bottom tee, single stem `╧` (U+2567).
    pub frame_tee_b_dh: char,
    /// Double-bar left tee, single stem `╞` (U+255E).
    pub frame_tee_l_dv: char,
    /// Double-bar right tee, single stem `╡` (U+2561).
    pub frame_tee_r_dv: char,
    /// Double-bar cross, single horizontal stem `╪` (U+256A).
    pub frame_cross_dh: char,
    /// Double-bar cross, single vertical stem `╫` (U+256B).
    pub frame_cross_dv: char,

    // --- Frame glyphs — mixed: single BAR, double perpendicular STEM ---
    /// Single-bar top tee, double stem `╥` (U+2565).
    pub frame_tee_t_sh: char,
    /// Single-bar bottom tee, double stem `╨` (U+2568).
    pub frame_tee_b_sh: char,
    /// Single-bar left tee, double stem `╟` (U+255F).
    pub frame_tee_l_sv: char,
    /// Single-bar right tee, double stem `╢` (U+2562).
    pub frame_tee_r_sv: char,
```

- [ ] **Step 2: Seed the new fields in the `Default for Glyphs` impl.**

In `src/theme.rs`, the `Default` impl seeds frame glyphs ending with `frame_cross: '\u{253C}',` (around line 631). Insert immediately after that line:

```rust
            // Frame double-line tee/cross joins: ╠ ╣ ╦ ╩ ╬
            frame_tee_l_d: '\u{2560}',
            frame_tee_r_d: '\u{2563}',
            frame_tee_t_d: '\u{2566}',
            frame_tee_b_d: '\u{2569}',
            frame_cross_d: '\u{256C}',

            // Mixed: double bar / single stem: ╤ ╧ ╞ ╡ ╪ ╫
            frame_tee_t_dh: '\u{2564}',
            frame_tee_b_dh: '\u{2567}',
            frame_tee_l_dv: '\u{255E}',
            frame_tee_r_dv: '\u{2561}',
            frame_cross_dh: '\u{256A}',
            frame_cross_dv: '\u{256B}',

            // Mixed: single bar / double stem: ╥ ╨ ╟ ╢
            frame_tee_t_sh: '\u{2565}',
            frame_tee_b_sh: '\u{2568}',
            frame_tee_l_sv: '\u{255F}',
            frame_tee_r_sv: '\u{2562}',
```

- [ ] **Step 3: Add a glyph-default unit test in `src/theme.rs`.**

There is an existing glyph test near line 950 (asserting `frame_tl` etc.). Add a new test in that same `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn junction_glyphs_seeded() {
        let g = Theme::classic_blue();
        let g = g.glyphs();
        // double
        assert_eq!(g.frame_tee_t_d, '╦');
        assert_eq!(g.frame_tee_b_d, '╩');
        assert_eq!(g.frame_tee_l_d, '╠');
        assert_eq!(g.frame_tee_r_d, '╣');
        assert_eq!(g.frame_cross_d, '╬');
        // mixed: double bar / single stem
        assert_eq!(g.frame_tee_t_dh, '╤');
        assert_eq!(g.frame_tee_b_dh, '╧');
        assert_eq!(g.frame_tee_l_dv, '╞');
        assert_eq!(g.frame_tee_r_dv, '╡');
        // mixed: single bar / double stem
        assert_eq!(g.frame_tee_t_sh, '╥');
        assert_eq!(g.frame_tee_b_sh, '╨');
    }
```

- [ ] **Step 4: Create `src/junction.rs` with the data types.**

```rust
//! View-independent vocabulary for joining box-drawing linework: which frame
//! edge a divider abuts, the weight (single/double) of each line, and the pure
//! glyph-selector functions that map an (edge, bar-weight, stem-weight) tuple to
//! the matching box character in [`Glyphs`].
//!
//! This is the rstv-local equivalent of Turbo Vision's `frameChars[mask]` table
//! (`framelin.cpp`): a small finite map with no view dependencies, so it is
//! exhaustively unit-testable. The owning [`Window`](crate::window::Window)
//! pushes [`JunctionMark`]s down to its [`Frame`](crate::frame::Frame), which
//! calls [`frame_junction`] per marked edge cell; the outer
//! [`Splitter`](crate::widgets::Splitter) calls [`divider_junction`] for its
//! interior crossings. See the design spec
//! `docs/superpowers/specs/2026-06-13-splitter-frame-joining-design.md`.

use crate::theme::Glyphs;

/// Which frame edge a divider abutment (or junction cell) lands on.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// The drawn weight of a line (a frame border or a divider).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Weight {
    Single,
    Double,
}

/// A divider abutment the owning window pushes down to its frame: the divider's
/// line meets the frame `edge` at frame-local `offset` along that edge, drawn at
/// the divider's `stem` weight. The frame substitutes the matching tee glyph
/// (chosen from its own border weight × this `stem`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct JunctionMark {
    /// Which frame edge this lands on.
    pub edge: Edge,
    /// Frame-local position along that edge (x for Top/Bottom, y for Left/Right).
    pub offset: i32,
    /// The abutting divider's drawn weight.
    pub stem: Weight,
}

/// Where two dividers meet in the interior (Site 2). `TeeRight` = a vertical
/// line with a branch going right (`├`); `TeeDown` = a horizontal line with a
/// branch going down (`┬`); `Cross` = both perpendicular branches (`┼`). Named
/// by the visual branch direction, matching the existing `frame_tee_*` glyph
/// names in [`Glyphs`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Junction {
    TeeRight,
    TeeLeft,
    TeeUp,
    TeeDown,
    Cross,
}
```

Run nothing yet — the selector functions and tests come in Step 5/6. Continue editing the same file.

- [ ] **Step 5: Add the `frame_junction` selector to `src/junction.rs`.**

Append to `src/junction.rs`:

```rust
/// Pick the box-drawing junction for a frame edge cell that a divider abuts.
///
/// * `edge` — which frame edge the cell is on.
/// * `bar` — the frame border's own weight (Double when the window is active).
/// * `stem` — the abutting divider's weight.
///
/// The naming key in [`Glyphs`]: `_d` = both double; `_dh`/`_dv` = double bar
/// with a single perpendicular stem; `_sh`/`_sv` = single bar with a double stem.
/// For Top/Bottom the bar is horizontal (`_dh`/`_sh`); for Left/Right the bar is
/// vertical (`_dv`/`_sv`).
pub fn frame_junction(edge: Edge, bar: Weight, stem: Weight, g: &Glyphs) -> char {
    use Edge::*;
    use Weight::*;
    match (edge, bar, stem) {
        // Top edge → tee pointing down (┬ family).
        (Top, Single, Single) => g.frame_tee_t,
        (Top, Double, Single) => g.frame_tee_t_dh,
        (Top, Double, Double) => g.frame_tee_t_d,
        (Top, Single, Double) => g.frame_tee_t_sh,
        // Bottom edge → tee pointing up (┴ family).
        (Bottom, Single, Single) => g.frame_tee_b,
        (Bottom, Double, Single) => g.frame_tee_b_dh,
        (Bottom, Double, Double) => g.frame_tee_b_d,
        (Bottom, Single, Double) => g.frame_tee_b_sh,
        // Left edge → tee pointing right (├ family).
        (Left, Single, Single) => g.frame_tee_l,
        (Left, Double, Single) => g.frame_tee_l_dv,
        (Left, Double, Double) => g.frame_tee_l_d,
        (Left, Single, Double) => g.frame_tee_l_sv,
        // Right edge → tee pointing left (┤ family).
        (Right, Single, Single) => g.frame_tee_r,
        (Right, Double, Single) => g.frame_tee_r_dv,
        (Right, Double, Double) => g.frame_tee_r_d,
        (Right, Single, Double) => g.frame_tee_r_sv,
    }
}

/// Pick the box-drawing junction where two dividers meet in the interior.
///
/// `dir` is the visual shape (which way the branch points); `weight` is the
/// shared divider weight. For this feature both the through-divider and the
/// branching divider carry the same weight at draw time (a divider never changes
/// weight), so a single `weight` parameter suffices — mixed interior crossings
/// are out of scope (the spec's non-goals).
pub fn divider_junction(dir: Junction, weight: Weight, g: &Glyphs) -> char {
    use Junction::*;
    use Weight::*;
    match (dir, weight) {
        (TeeRight, Single) => g.frame_tee_l,   // ├
        (TeeRight, Double) => g.frame_tee_l_d, // ╠
        (TeeLeft, Single) => g.frame_tee_r,    // ┤
        (TeeLeft, Double) => g.frame_tee_r_d,  // ╣
        (TeeUp, Single) => g.frame_tee_b,      // ┴
        (TeeUp, Double) => g.frame_tee_b_d,    // ╩
        (TeeDown, Single) => g.frame_tee_t,    // ┬
        (TeeDown, Double) => g.frame_tee_t_d,  // ╦
        (Cross, Single) => g.frame_cross,      // ┼
        (Cross, Double) => g.frame_cross_d,    // ╬
    }
}
```

- [ ] **Step 6: Add exhaustive unit tests to `src/junction.rs`.**

Append:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Glyphs;

    fn g() -> Glyphs {
        Glyphs::default()
    }

    #[test]
    fn frame_junction_single_bar_single_stem() {
        let g = g();
        assert_eq!(frame_junction(Edge::Top, Weight::Single, Weight::Single, &g), '┬');
        assert_eq!(frame_junction(Edge::Bottom, Weight::Single, Weight::Single, &g), '┴');
        assert_eq!(frame_junction(Edge::Left, Weight::Single, Weight::Single, &g), '├');
        assert_eq!(frame_junction(Edge::Right, Weight::Single, Weight::Single, &g), '┤');
    }

    #[test]
    fn frame_junction_double_bar_single_stem_is_mixed() {
        let g = g();
        assert_eq!(frame_junction(Edge::Top, Weight::Double, Weight::Single, &g), '╤');
        assert_eq!(frame_junction(Edge::Bottom, Weight::Double, Weight::Single, &g), '╧');
        assert_eq!(frame_junction(Edge::Left, Weight::Double, Weight::Single, &g), '╞');
        assert_eq!(frame_junction(Edge::Right, Weight::Double, Weight::Single, &g), '╡');
    }

    #[test]
    fn frame_junction_double_bar_double_stem() {
        let g = g();
        assert_eq!(frame_junction(Edge::Top, Weight::Double, Weight::Double, &g), '╦');
        assert_eq!(frame_junction(Edge::Bottom, Weight::Double, Weight::Double, &g), '╩');
        assert_eq!(frame_junction(Edge::Left, Weight::Double, Weight::Double, &g), '╠');
        assert_eq!(frame_junction(Edge::Right, Weight::Double, Weight::Double, &g), '╣');
    }

    #[test]
    fn frame_junction_single_bar_double_stem_is_rare_mixed() {
        let g = g();
        assert_eq!(frame_junction(Edge::Top, Weight::Single, Weight::Double, &g), '╥');
        assert_eq!(frame_junction(Edge::Bottom, Weight::Single, Weight::Double, &g), '╨');
        assert_eq!(frame_junction(Edge::Left, Weight::Single, Weight::Double, &g), '╟');
        assert_eq!(frame_junction(Edge::Right, Weight::Single, Weight::Double, &g), '╢');
    }

    #[test]
    fn divider_junction_all_directions() {
        let g = g();
        assert_eq!(divider_junction(Junction::TeeRight, Weight::Single, &g), '├');
        assert_eq!(divider_junction(Junction::TeeLeft, Weight::Single, &g), '┤');
        assert_eq!(divider_junction(Junction::TeeUp, Weight::Single, &g), '┴');
        assert_eq!(divider_junction(Junction::TeeDown, Weight::Single, &g), '┬');
        assert_eq!(divider_junction(Junction::Cross, Weight::Single, &g), '┼');
        assert_eq!(divider_junction(Junction::Cross, Weight::Double, &g), '╬');
    }
}
```

- [ ] **Step 7: Wire the module into `src/lib.rs`.**

Add the module declaration in the `pub mod` block (alphabetical-ish; it sits between `help` and `keymap` or anywhere in the list — place after `pub mod help;`):

```rust
pub mod junction;
```

And add a re-export near the existing `pub use frame::Frame;` (line ~102):

```rust
pub use junction::{Edge, JunctionMark, Weight};
```

(`Junction`, `frame_junction`, `divider_junction` stay crate-internal — consumers are `Frame`/`Splitter`/`Window`, all in-crate. Do not re-export them.)

- [ ] **Step 8: Run the tests + clippy + fmt.**

Run:
```bash
cargo test -j4 -p tvision junction:: theme::tests::junction_glyphs_seeded
cargo clippy -j4 --workspace --all-targets -- -D warnings
cargo fmt --all --check
```
Expected: all PASS; clippy clean; fmt clean.

- [ ] **Step 9: Commit.**

```bash
git add src/junction.rs src/lib.rs src/theme.rs
git commit -m "feat(junction): junction glyph vocabulary + pure selectors

Add Edge/Weight/JunctionMark + frame_junction/divider_junction (the
rstv-local frameChars[mask] equivalent) and seed the double + mixed
tee/cross glyphs into Glyphs. View-independent, exhaustively unit-tested.

Co-Authored-By: ..."
```

---

## Task 2: Frame — store + draw junction marks

`Frame` accepts marks pushed down by its owner and substitutes tee glyphs at the matching interior edge cells. With no marks it draws exactly as today.

**Files:**
- Modify: `src/frame.rs` (field, setter, draw loops, module docs)
- Test: inline `#[cfg(test)]` in `src/frame.rs`

- [ ] **Step 1: Write the failing test (marked edge cells get tees; corners + unmarked cells unchanged).**

Add to `src/frame.rs`'s `#[cfg(test)] mod tests`:

```rust
    #[test]
    fn junction_marks_substitute_tees_on_interior_edges() {
        use crate::junction::{Edge, JunctionMark, Weight};
        // Passive (single-line) frame so the bar weight is Single.
        let mut f = Frame::new(Rect::new(0, 0, 20, 6));
        f.set_junction_marks(vec![
            JunctionMark { edge: Edge::Top, offset: 8, stem: Weight::Single },
            JunctionMark { edge: Edge::Bottom, offset: 8, stem: Weight::Single },
            JunctionMark { edge: Edge::Right, offset: 3, stem: Weight::Single },
        ]);
        let buf = render_frame(&mut f, 20, 6);
        assert_eq!(buf.get(8, 0).symbol(), "┬", "top-edge mark → ┬");
        assert_eq!(buf.get(8, 5).symbol(), "┴", "bottom-edge mark → ┴");
        assert_eq!(buf.get(19, 3).symbol(), "┤", "right-edge mark → ┤");
        // An unmarked edge cell keeps the plain edge glyph.
        assert_eq!(buf.get(5, 0).symbol(), "─", "unmarked top stays ─");
        // Corners untouched.
        assert_eq!(buf.get(0, 0).symbol(), "┌");
        assert_eq!(buf.get(19, 0).symbol(), "┐");
    }

    #[test]
    fn no_marks_is_byte_for_byte_unchanged() {
        // A frame with an empty mark list draws identically to default.
        let mut a = Frame::new(Rect::new(0, 0, 20, 6));
        a.st.state.active = true;
        let mut b = Frame::new(Rect::new(0, 0, 20, 6));
        b.st.state.active = true;
        b.set_junction_marks(vec![]);
        let ba = render_frame(&mut a, 20, 6);
        let bb = render_frame(&mut b, 20, 6);
        for y in 0..6 {
            assert_eq!(row_text(&ba, y), row_text(&bb, y), "row {y} identical");
        }
    }

    #[test]
    fn active_double_frame_uses_mixed_tee_for_single_stem() {
        use crate::junction::{Edge, JunctionMark, Weight};
        let mut f = Frame::new(Rect::new(0, 0, 20, 6));
        f.st.state.active = true; // double-line bar
        f.set_junction_marks(vec![JunctionMark {
            edge: Edge::Top,
            offset: 8,
            stem: Weight::Single,
        }]);
        let buf = render_frame(&mut f, 20, 6);
        assert_eq!(buf.get(8, 0).symbol(), "╤", "double bar + single stem → ╤");
    }
```

- [ ] **Step 2: Run to verify failure.**

Run:
```bash
cargo test -j4 -p tvision frame::tests::junction_marks_substitute_tees_on_interior_edges
```
Expected: FAIL — `set_junction_marks` does not exist (compile error).

- [ ] **Step 3: Add the field + setter.**

In `src/frame.rs`, add the import near the top (`use crate::theme::Role;` line ~60):
```rust
use crate::junction::{Edge, JunctionMark, Weight, frame_junction};
```

Add the field to `struct Frame` (after `close_pressed: bool,`):
```rust
    /// Divider abutment marks pushed down by the owning window each draw
    /// (owner-data-down). Empty = today's plain frame, so non-joined windows are
    /// byte-for-byte unchanged. See [`set_junction_marks`](Frame::set_junction_marks).
    junction_marks: Vec<JunctionMark>,
```

Initialize it in `Frame::new` (in the struct literal, after `close_pressed: false,`):
```rust
            junction_marks: Vec::new(),
```

Add the setter in the `impl Frame` block (after `set_palette`/`palette`):
```rust
    /// Owner-data-down: the owning window pushes the divider abutment marks the
    /// frame should join into its border. Replaced each draw; empty = a plain
    /// frame (non-joined windows unchanged). Faithful re-expression of TV's
    /// `frameLine` tee-walk, fed by pushed data instead of a sideways sibling walk.
    pub(crate) fn set_junction_marks(&mut self, marks: Vec<JunctionMark>) {
        self.junction_marks = marks;
    }

    /// The junction glyph to substitute at an **interior** border cell on `edge`
    /// at `offset`, if a mark lands there; `None` to keep the plain edge glyph.
    /// `bar` is the frame's own weight. Callers only invoke this on interior edge
    /// cells, so corner offsets never reach here — but the corner guard is also
    /// structural (the draw loops skip the corners).
    fn junction_at(&self, edge: Edge, offset: i32, bar: Weight, g: &crate::theme::Glyphs) -> Option<char> {
        self.junction_marks
            .iter()
            .find(|m| m.edge == edge && m.offset == offset)
            .map(|m| frame_junction(edge, bar, m.stem, g))
    }
```

- [ ] **Step 4: Substitute tees in the border draw loops.**

In `Frame::draw`, after the `(border_role, double)` is computed (line ~252), introduce the bar weight:
```rust
        let bar = if double { Weight::Double } else { Weight::Single };
```

Then replace the three edge loops in "**1. The box**" so marks substitute on interior cells. The **top row** loop becomes:
```rust
        // Top row: tl, ─ (or a tee at a marked cell) across the interior, tr.
        ctx.put_char(0, 0, tl, border);
        for x in 1..w - 1 {
            let ch = self.junction_at(Edge::Top, x, bar, &glyphs).unwrap_or(h_edge);
            ctx.put_char(x, 0, ch, border);
        }
        if w >= 2 {
            ctx.put_char(w - 1, 0, tr, border);
        }
```

The **middle rows** loop becomes:
```rust
        // Middle rows: │ (or a tee at a marked cell), spaces, │ (or a tee).
        for y in 1..h - 1 {
            let lch = self.junction_at(Edge::Left, y, bar, &glyphs).unwrap_or(v_edge);
            ctx.put_char(0, y, lch, border);
            for x in 1..w - 1 {
                ctx.put_char(x, y, ' ', border);
            }
            if w >= 2 {
                let rch = self.junction_at(Edge::Right, y, bar, &glyphs).unwrap_or(v_edge);
                ctx.put_char(w - 1, y, rch, border);
            }
        }
```

The **bottom row** loop becomes:
```rust
        // Bottom row: bl, ─ (or a tee at a marked cell) across, br.
        if h >= 2 {
            ctx.put_char(0, h - 1, bl, border);
            for x in 1..w - 1 {
                let ch = self.junction_at(Edge::Bottom, x, bar, &glyphs).unwrap_or(h_edge);
                ctx.put_char(x, h - 1, ch, border);
            }
            if w >= 2 {
                ctx.put_char(w - 1, h - 1, br, border);
            }
        }
```

(The marks are applied here in step 1, **before** the number/title/icon overlays in steps 3–6, so an overlay always wins over a junction — matching the spec's ordering requirement. No reordering of steps 3–6 is needed.)

- [ ] **Step 5: Run to verify the new tests pass + the existing frame tests + snapshots still pass.**

Run:
```bash
cargo test -j4 -p tvision frame::
```
Expected: all PASS, including `snapshot_active_frame` / `snapshot_passive_frame` (those frames have no marks → unchanged). If a snapshot unexpectedly changed, STOP — the no-marks path must be byte-for-byte identical.

- [ ] **Step 6: Update the frame module docs.**

In `src/frame.rs`, the module-level doc comment (lines ~50–55) currently says the sibling tee-walk "is not reproduced … The tee/cross glyphs remain seeded in `Glyphs` but unused." Replace that paragraph with:

```rust
//! The classic sibling tee-walk (`TFrame::frameLine` reaching sideways to read
//! its siblings' bounds) is **not** reproduced as a sideways walk — deviation D3
//! forbids a child reaching its siblings. Instead, a window that opts into
//! line-joining (`Window::with_joined_lines`) computes the divider abutments from
//! its `Splitter` child and pushes them **down** to this frame as
//! [`JunctionMark`](crate::junction::JunctionMark)s via
//! [`set_junction_marks`](Frame::set_junction_marks); the frame then substitutes
//! the matching tee glyph at each marked edge cell — the same visual result as
//! `frameLine`, fed by pushed data. A frame with no marks (every non-joined
//! window) draws plain corners and edges, exactly as before.
```

Also fix the same claim in the inner `draw` doc (the line "The sibling tee-walk is not reproduced — we draw plain corners/edges; see the module docs.") to:
```rust
    /// Marked interior edge cells get a junction tee (owner-data-down via
    /// `set_junction_marks`); with no marks the corners/edges are plain. See the
    /// module docs.
```

And in `src/theme.rs`, the `Glyphs` doc comment says the tee/cross glyphs are "seeded for completeness but unused". Update that sentence to note they now feed `crate::junction::frame_junction` when a window opts into line-joining. (One-line edit; keep it brief.)

- [ ] **Step 7: clippy + fmt + commit.**

Run:
```bash
cargo clippy -j4 --workspace --all-targets -- -D warnings
cargo fmt --all --check
git add src/frame.rs src/theme.rs
git commit -m "feat(frame): mark-aware border draw (frameLine revival, owner-data-down)

Frame stores JunctionMarks pushed down by its owner and substitutes the
matching tee glyph at marked interior edge cells. Empty marks = plain
frame, byte-for-byte unchanged. Faithful frameLine composition minus the
D3-forbidden sideways walk.

Co-Authored-By: ..."
```

---

## Task 3: Splitter — the `as_any_mut` downcast keystone

Without this, no parent can downcast a pane (or the window's interior child) to `Splitter`, and every downcast in this design fails. This is a one-line override **body**; `as_any_mut` already exists in `tvision-macros/src/specs.rs`, so the `delegate_view` spy test stays green with no `specs.rs` edit.

**Files:**
- Modify: `src/widgets/splitter/mod.rs`
- Test: inline `#[cfg(test)]` in `src/widgets/splitter/mod.rs`

- [ ] **Step 1: Write the failing keystone test.**

Add to the `view_tests` module in `src/widgets/splitter/mod.rs`:

```rust
    #[test]
    fn splitter_downcasts_through_as_any_mut() {
        let mut sp = Splitter::cols();
        sp.change_bounds(Rect::new(0, 0, 13, 3));
        sp.insert(Fill::boxed('A'), Constraints::flex());
        sp.insert(Fill::boxed('B'), Constraints::flex());
        let resolved = (&mut sp as &mut dyn View)
            .as_any_mut()
            .and_then(|a| a.downcast_mut::<Splitter>())
            .is_some();
        assert!(resolved, "Splitter must override as_any_mut → Some(self)");
    }
```

- [ ] **Step 2: Run to verify failure.**

Run:
```bash
cargo test -j4 -p tvision splitter::view_tests::splitter_downcasts_through_as_any_mut
```
Expected: FAIL — the assertion fails because `as_any_mut` is currently forwarded to the inner `Group` (returns `None`).

- [ ] **Step 3: Add the override body.**

In `src/widgets/splitter/mod.rs`, inside the `#[crate::delegate(to = group)] impl View for Splitter { … }` block, add the override (anywhere among the existing overrides, e.g. after `change_bounds`). **Add a body — do NOT add `skip(as_any_mut)`** (the macro auto-excludes any method written in the impl from forwarding, exactly as it already does for `draw`/`handle_event`/`change_bounds`):

```rust
    /// Downcast seam: a parent (the owning window, or an outer splitter reaching a
    /// pane sub-splitter) reaches this `Splitter` concretely via `child_mut` +
    /// `as_any_mut` + `downcast_mut::<Splitter>()` — the same mechanism a window
    /// uses to push data to its `Frame`. The `#[delegate(to = group)]` macro would
    /// otherwise forward this to the inner `Group` (which returns `None`), so the
    /// override body here is required; the macro auto-excludes it from forwarding.
    fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> {
        Some(self)
    }
```

- [ ] **Step 4: Run to verify the keystone test passes AND the delegate spy test stays green.**

Run:
```bash
cargo test -j4 -p tvision splitter::view_tests::splitter_downcasts_through_as_any_mut
cargo test -j4 -p tvision --test delegate_view
```
Expected: both PASS (the spy test confirms no forwarder regressed, and `as_any_mut` is already in `specs.rs` so no edit is needed).

- [ ] **Step 5: clippy + fmt + commit.**

```bash
cargo clippy -j4 --workspace --all-targets -- -D warnings
cargo fmt --all --check
git add src/widgets/splitter/mod.rs
git commit -m "feat(splitter): as_any_mut downcast keystone

Override as_any_mut → Some(self) so a parent can downcast a Splitter pane
(or a window's interior child) concretely. Required by frame-joining;
delegate_view spy test stays green (as_any_mut already in specs.rs).

Co-Authored-By: ..."
```

---

## Task 4: Splitter — produce frame junction marks

`Splitter::frame_junction_marks` walks its dividers, emits a mark for each end that abuts a frame edge, and recurses into pane sub-splitters. It is `&mut self` because reaching a pane child to recurse requires `Group::child_mut` (the only child accessor `Group` exposes is `&mut`). It is otherwise a pure function of layout.

**Coordinate contract (from the spec's "Data flow"):** a divider at splitter-local 0-based axis position `dx` maps to a frame-edge offset of `(splitter.bounds.a − frame_bounds.a) + dx` along the abutted edge. The frame and the (top-level) splitter are **siblings in the window's group**, so their bounds share that group's coordinate space; the subtraction yields the frame-local offset directly. **This off-by-one-prone mapping is verified empirically by the snapshot tests in Task 5** — treat those snapshots as the ground truth and adjust the origin term if a snapshot reveals the divider tee is one cell off.

**Files:**
- Modify: `src/widgets/splitter/mod.rs`
- Test: inline `#[cfg(test)]` in `src/widgets/splitter/mod.rs`

- [ ] **Step 1: Write the failing unit test (a 2-pane cols splitter filling a frame interior yields a top + bottom mark).**

Add to the `view_tests` module:

```rust
    #[test]
    fn frame_marks_two_pane_cols_abut_top_and_bottom() {
        use crate::junction::{Edge, Weight};
        // Frame fills a 13×5 window: frame_bounds = (0,0,13,5). Splitter fills the
        // interior (1,1,12,4): 2 panes, 1 divider, 10 content => 5/5; the divider
        // sits at splitter-local x = 5, i.e. frame-local x = 1 + 5 = 6.
        let frame_bounds = Rect::new(0, 0, 13, 5);
        let mut sp = Splitter::cols();
        sp.change_bounds(Rect::new(1, 1, 12, 4));
        sp.insert(Fill::boxed('A'), Constraints::flex());
        sp.insert(Fill::boxed('B'), Constraints::flex());
        let marks = sp.frame_junction_marks(frame_bounds);
        assert_eq!(marks.len(), 2, "one top + one bottom mark");
        assert!(marks.contains(&crate::junction::JunctionMark {
            edge: Edge::Top,
            offset: 6,
            stem: Weight::Single
        }));
        assert!(marks.contains(&crate::junction::JunctionMark {
            edge: Edge::Bottom,
            offset: 6,
            stem: Weight::Single
        }));
    }

    #[test]
    fn frame_marks_handle_divider_emits_nothing() {
        // A Handle divider draws only a midpoint nub — its line does not reach the
        // frame edge, so it must emit no mark.
        let frame_bounds = Rect::new(0, 0, 13, 5);
        let mut sp = Splitter::cols().default_divider(DividerStyle::Handle);
        sp.change_bounds(Rect::new(1, 1, 12, 4));
        sp.insert(Fill::boxed('A'), Constraints::flex());
        sp.insert(Fill::boxed('B'), Constraints::flex());
        assert!(sp.frame_junction_marks(frame_bounds).is_empty());
    }

    #[test]
    fn frame_marks_inset_splitter_emits_nothing() {
        // A splitter inset from the frame (a 2-cell margin) does not abut → no marks.
        let frame_bounds = Rect::new(0, 0, 13, 7);
        let mut sp = Splitter::cols();
        sp.change_bounds(Rect::new(2, 2, 11, 5)); // not adjacent to any frame edge
        sp.insert(Fill::boxed('A'), Constraints::flex());
        sp.insert(Fill::boxed('B'), Constraints::flex());
        assert!(sp.frame_junction_marks(frame_bounds).is_empty());
    }

    #[test]
    fn frame_marks_nested_grid_inner_divider_hits_right_frame() {
        use crate::junction::Edge;
        // Outer cols [tree | inner-rows]; inner-rows has one horizontal divider
        // whose right end abuts the right frame edge → a Right mark.
        let frame_bounds = Rect::new(0, 0, 22, 7);
        let inner = Splitter::rows()
            .pane(Fill::boxed('L'), Constraints::flex())
            .pane(Fill::boxed('F'), Constraints::flex());
        let mut outer = Splitter::cols();
        outer.change_bounds(Rect::new(1, 1, 21, 6));
        outer.insert(Fill::boxed('T'), Constraints::fixed(8));
        outer.insert(Box::new(inner), Constraints::flex());
        let marks = outer.frame_junction_marks(frame_bounds);
        // The outer vertical divider abuts top + bottom; the inner horizontal
        // divider abuts the right edge. Assert at least the Right mark is present.
        assert!(
            marks.iter().any(|m| m.edge == Edge::Right),
            "inner horizontal divider must abut the right frame edge, got {marks:?}"
        );
        assert!(
            marks.iter().filter(|m| m.edge == Edge::Top).count() == 1,
            "outer vertical divider abuts the top edge once"
        );
    }
```

- [ ] **Step 2: Run to verify failure.**

Run:
```bash
cargo test -j4 -p tvision splitter::view_tests::frame_marks_two_pane_cols_abut_top_and_bottom
```
Expected: FAIL — `frame_junction_marks` does not exist.

- [ ] **Step 3: Implement `frame_junction_marks` + the private recursive collector.**

In `src/widgets/splitter/mod.rs`, add the import near the top (the `use crate::theme::Role;` line):
```rust
use crate::junction::{Edge, JunctionMark, Weight};
```

Add to the `impl Splitter { … }` block (e.g. after `divider_axis_pos`):

```rust
    /// Owner-data-down producer: for each divider whose drawn line abuts a frame
    /// edge, emit a [`JunctionMark`] in `frame_bounds`-local coordinates; recurses
    /// into pane sub-splitters. A pure function of layout (no drawing), but `&mut
    /// self` because reaching a pane child to recurse needs `Group::child_mut`
    /// (the only child accessor `Group` exposes is `&mut`). The owning window
    /// already holds the `&mut Splitter`, so this is free there.
    pub(crate) fn frame_junction_marks(&mut self, frame_bounds: Rect) -> Vec<JunctionMark> {
        let mut out = Vec::new();
        self.collect_frame_marks(frame_bounds, &mut out);
        out
    }

    /// Recursive worker for [`frame_junction_marks`]. `frame_bounds` stays in the
    /// window group's coordinate space across the recursion (the frame and every
    /// (sub-)splitter share that space).
    fn collect_frame_marks(&mut self, fb: Rect, out: &mut Vec<JunctionMark>) {
        let b = self.group.state().get_bounds();
        let sizes = solve(&self.slots, self.content_len());
        // Every divider draws at the same weight: double only in reconfig mode.
        let stem = if self.reconfig.is_some() {
            Weight::Double
        } else {
            Weight::Single
        };
        let fw = fb.b.x - fb.a.x; // frame width
        let fh = fb.b.y - fb.a.y; // frame height

        let mut cursor = 0i32; // splitter-local 0-based axis position
        for i in 0..self.slots.len().saturating_sub(1) {
            cursor += sizes.get(i).copied().unwrap_or(0);
            let local = cursor; // this divider's local axis position
            // A divider emits a mark only if its line is actually drawn at its end
            // cells — i.e. a full line: a `Line` divider, or any divider in
            // reconfig mode. A `Handle` (midpoint nub) / `Hidden` / `Locked`
            // divider in normal mode draws nothing at the ends → no mark.
            let draws_full = matches!(self.style_of(i), DividerStyle::Line) || self.reconfig.is_some();
            if draws_full {
                match self.orientation {
                    Orientation::Cols => {
                        // Vertical divider; window-space x = b.a.x + local.
                        let off = (b.a.x - fb.a.x) + local;
                        let interior = off > 0 && off < fw - 1;
                        // Top end abuts the frame top edge if the splitter's top is
                        // one cell inside the frame's top.
                        if interior && b.a.y == fb.a.y + 1 {
                            out.push(JunctionMark { edge: Edge::Top, offset: off, stem });
                        }
                        if interior && b.b.y == fb.b.y - 1 {
                            out.push(JunctionMark { edge: Edge::Bottom, offset: off, stem });
                        }
                    }
                    Orientation::Rows => {
                        // Horizontal divider; window-space y = b.a.y + local.
                        let off = (b.a.y - fb.a.y) + local;
                        let interior = off > 0 && off < fh - 1;
                        if interior && b.a.x == fb.a.x + 1 {
                            out.push(JunctionMark { edge: Edge::Left, offset: off, stem });
                        }
                        if interior && b.b.x == fb.b.x - 1 {
                            out.push(JunctionMark { edge: Edge::Right, offset: off, stem });
                        }
                    }
                }
            }
            cursor += 1; // step over the divider cell
        }

        // Recurse into pane sub-splitters (child_mut → as_any_mut → downcast).
        let ids = self.group.child_ids_in_order();
        for id in ids {
            if let Some(sp) = self
                .group
                .child_mut(id)
                .and_then(|v| v.as_any_mut())
                .and_then(|a| a.downcast_mut::<Splitter>())
            {
                sp.collect_frame_marks(fb, out);
            }
        }
    }
```

- [ ] **Step 4: Run to verify the unit tests pass.**

Run:
```bash
cargo test -j4 -p tvision splitter::view_tests::frame_marks
```
Expected: all four `frame_marks_*` tests PASS. If `frame_marks_nested_grid_inner_divider_hits_right_frame` fails on the Right mark, the inner splitter's bounds origin term is the suspect — inspect what `b` the inner splitter reports inside `collect_frame_marks` (add a temporary `eprintln!`), and confirm the inner bounds share the window-group space. The window-level snapshot in Task 5 is the authoritative check.

- [ ] **Step 5: clippy + fmt + commit.**

```bash
cargo clippy -j4 --workspace --all-targets -- -D warnings
cargo fmt --all --check
git add src/widgets/splitter/mod.rs
git commit -m "feat(splitter): frame_junction_marks producer (recursive, layout-only)

Walk dividers, emit a JunctionMark for each end abutting a frame edge (in
frame-local coords), recurse into pane sub-splitters. Marks emitted only
when the divider draws a full line (Line, or any in reconfig); Handle/
Hidden/Locked + inset splitters emit nothing.

Co-Authored-By: ..."
```

---

## Task 5: Window — opt-in flag + brokering draw

`Window` gains the opt-in flag and a `draw` override that reads marks from its splitter child and pushes them to its frame child. Both child borrows are sequential (marks cloned out between them), so only one `&mut` child is live at a time.

**Files:**
- Modify: `src/window/window.rs` (field, builder/setter, helpers, `draw` override)
- Test: inline `#[cfg(test)]` in `src/window/window.rs`

- [ ] **Step 1: Write the failing snapshot tests (passive `┬…┴`; active `╤…╧`).**

Add to `src/window/window.rs`'s `#[cfg(test)] mod tests`. (Use the existing `HeadlessBackend`/`Renderer`/`Buffer` snapshot pattern already used by `selected_window_with_scrollbar_snapshot` and `zoom_restored_vs_filled_snapshot` in this file — copy that harness shape.)

```rust
    /// A window with `with_joined_lines()` hosting a 2-column splitter that fills
    /// the interior: the vertical divider must tee into the top + bottom frame
    /// (passive single frame → ┬ / ┴).
    #[test]
    fn joined_lines_passive_frame_tees_top_and_bottom() {
        use crate::widgets::{Constraints, Splitter};
        let theme = Theme::classic_blue();
        let win_rect = Rect::new(0, 0, 16, 6);
        let mut win = Window::new(win_rect, Some("Grid".to_string()), 0).with_joined_lines();

        let ext = win.state().get_extent();
        let interior = Rect::new(1, 1, ext.b.x - 1, ext.b.y - 1);
        let split = Splitter::cols()
            .pane(plain_fill('A'), Constraints::flex())
            .pane(plain_fill('B'), Constraints::flex());
        let sid = win.insert_child(Box::new(split));
        if let Some(v) = win.child_mut(sid) {
            v.change_bounds(interior);
        }

        let (backend, screen) = HeadlessBackend::new(16, 6);
        let mut r = Renderer::new(Box::new(backend));
        r.render(|buf: &mut Buffer| {
            let bounds = win.state().get_bounds();
            let mut dc = DrawCtx::new(buf, &theme, bounds, bounds.a);
            win.draw(&mut dc);
        });
        insta::assert_snapshot!(screen.snapshot());
    }

    /// Same window but active → double frame; the single divider joins through
    /// the mixed ╤ / ╧ junctions (the divider weight stays single).
    #[test]
    fn joined_lines_active_frame_uses_mixed_tees() {
        use crate::widgets::{Constraints, Splitter};
        let theme = Theme::classic_blue();
        let mut win = Window::new(Rect::new(0, 0, 16, 6), Some("Grid".to_string()), 0)
            .with_joined_lines();
        win.state_mut().state.active = true;

        let ext = win.state().get_extent();
        let interior = Rect::new(1, 1, ext.b.x - 1, ext.b.y - 1);
        let split = Splitter::cols()
            .pane(plain_fill('A'), Constraints::flex())
            .pane(plain_fill('B'), Constraints::flex());
        let sid = win.insert_child(Box::new(split));
        if let Some(v) = win.child_mut(sid) {
            v.change_bounds(interior);
        }

        let (backend, screen) = HeadlessBackend::new(16, 6);
        let mut r = Renderer::new(Box::new(backend));
        r.render(|buf: &mut Buffer| {
            let bounds = win.state().get_bounds();
            let mut dc = DrawCtx::new(buf, &theme, bounds, bounds.a);
            win.draw(&mut dc);
        });
        insta::assert_snapshot!(screen.snapshot());
    }

    /// A window WITHOUT the flag, hosting the same splitter, must NOT join — the
    /// frame stays plain (regression guard for the off-by-default path).
    #[test]
    fn without_flag_frame_is_plain() {
        use crate::widgets::{Constraints, Splitter};
        let theme = Theme::classic_blue();
        let mut win = Window::new(Rect::new(0, 0, 16, 6), Some("Grid".to_string()), 0);
        let ext = win.state().get_extent();
        let interior = Rect::new(1, 1, ext.b.x - 1, ext.b.y - 1);
        let split = Splitter::cols()
            .pane(plain_fill('A'), Constraints::flex())
            .pane(plain_fill('B'), Constraints::flex());
        let sid = win.insert_child(Box::new(split));
        if let Some(v) = win.child_mut(sid) {
            v.change_bounds(interior);
        }
        let (backend, screen) = HeadlessBackend::new(16, 6);
        let mut r = Renderer::new(Box::new(backend));
        r.render(|buf: &mut Buffer| {
            let bounds = win.state().get_bounds();
            let mut dc = DrawCtx::new(buf, &theme, bounds, bounds.a);
            win.draw(&mut dc);
        });
        // Top edge interior is plain single-line ─ everywhere (no tee).
        let row0: String = (1..15)
            .map(|x| screen.snapshot_cell_symbol(x, 0))
            .collect::<String>();
        assert!(!row0.contains('┬'), "no tee without the flag: {row0:?}");
    }
```

If the test helpers `plain_fill` and `snapshot_cell_symbol` do not already exist in this module, define `plain_fill` as a small local helper near the top of the test module:

```rust
    /// A minimal fill view for splitter-pane snapshot tests (fills its local
    /// rect with a character — same shape as the splitter module's `Fill`).
    fn plain_fill(ch: char) -> Box<dyn View> {
        struct F(char, ViewState);
        impl View for F {
            fn state(&self) -> &ViewState {
                &self.1
            }
            fn state_mut(&mut self) -> &mut ViewState {
                &mut self.1
            }
            fn draw(&mut self, ctx: &mut DrawCtx) {
                let b = self.1.get_bounds();
                let (w, h) = (b.b.x - b.a.x, b.b.y - b.a.y);
                ctx.fill(Rect::new(0, 0, w, h), self.0, ctx.style(crate::theme::Role::Normal));
            }
        }
        Box::new(F(ch, ViewState::new(Rect::new(0, 0, 1, 1))))
    }
```

For the `without_flag_frame_is_plain` check, instead of `snapshot_cell_symbol` (which may not exist), assert against the multi-line `screen.snapshot()` string: split it into lines and check the first interior row contains no `┬`. Adjust to whatever read accessor the sibling snapshot tests already use — prefer reading the `screen.snapshot()` String and inspecting it, to avoid inventing an accessor.

- [ ] **Step 2: Run to verify failure.**

Run:
```bash
cargo test -j4 -p tvision window::tests::joined_lines_passive_frame_tees_top_and_bottom
```
Expected: FAIL — `with_joined_lines` does not exist (compile error).

- [ ] **Step 3: Add the field + builder/setter.**

In `src/window/window.rs`, add the field to `struct Window` (after `title: Option<String>,`):
```rust
    /// Opt-in: join the linework of an embedded splitter to this window's frame
    /// (and the splitter's own dividers to each other). Default `false` — a plain
    /// window is byte-for-byte unchanged. See [`with_joined_lines`](Window::with_joined_lines).
    joined_lines: bool,
```

Initialize in `Window::new`'s struct literal (after `title,`):
```rust
            joined_lines: false,
```

Add the builder + setter in the `impl Window` accessors region (e.g. after `title()`):
```rust
    /// Opt this window into divider↔frame and divider↔divider line-joining for an
    /// embedded [`Splitter`](crate::widgets::Splitter). Builder form (chains on
    /// `Window::new`).
    pub fn with_joined_lines(mut self) -> Self {
        self.joined_lines = true;
        self
    }

    /// Setter form of [`with_joined_lines`](Window::with_joined_lines).
    pub fn set_joined_lines(&mut self, on: bool) {
        self.joined_lines = on;
    }
```

- [ ] **Step 4: Add the `interior_splitter_mut` + `frame_mut` helpers.**

Add to the `impl Window` block (near `child_mut`):
```rust
    /// The first non-frame child that downcasts to a [`Splitter`](crate::widgets::Splitter),
    /// or `None`. Used by the joined-lines `draw` to read divider abutments; no
    /// stored id needed — the splitter identifies itself by its concrete type.
    fn interior_splitter_mut(&mut self) -> Option<&mut crate::widgets::Splitter> {
        let frame_id = self.frame_id;
        for id in self.group.child_ids_in_order() {
            if id == frame_id {
                continue;
            }
            // Re-borrow per id so the &mut child does not outlive the iteration.
            if self
                .group
                .child_mut(id)
                .and_then(|v| v.as_any_mut())
                .and_then(|a| a.downcast_mut::<crate::widgets::Splitter>())
                .is_some()
            {
                return self
                    .group
                    .child_mut(id)
                    .and_then(|v| v.as_any_mut())
                    .and_then(|a| a.downcast_mut::<crate::widgets::Splitter>());
            }
        }
        None
    }

    /// The frame child, downcast to [`Frame`] — the same seam `zoom`/`set_flags`
    /// already use.
    fn frame_mut(&mut self) -> Option<&mut Frame> {
        self.group
            .child_mut(self.frame_id)
            .and_then(|v| v.as_any_mut())
            .and_then(|a| a.downcast_mut::<Frame>())
    }
```

> **Borrow-checker note for the implementer:** the two-pass `interior_splitter_mut` above (probe, then re-borrow to return) sidesteps a borrow that would otherwise be held across the loop. If the borrow checker is satisfied with a single-pass `find`-style return, simplify it — but the probe-then-return form is known to compile. The key correctness property is that the returned `&mut Splitter` borrow is **dropped before** `frame_mut` is called in `draw` (Step 5 clones the marks `Vec` out between the two).

- [ ] **Step 5: Add the `draw` override.**

In the `#[crate::delegate(to = group, skip(...))] impl View for Window` block, add a `draw` method (the macro auto-excludes a provided body from forwarding — same rule that lets `handle_event` stand; do **not** add `draw` to the `skip(...)` list):

```rust
    /// Default draw delegates to the embedded group. When `joined_lines` is set,
    /// first read the divider abutment marks from the interior splitter child
    /// (parent→child) and push them down to the frame child (owner-data-down), so
    /// the frame composes connected tees. Both child borrows are sequential — the
    /// marks `Vec` is cloned out between them, so only one `&mut` child is live at
    /// a time (D3-safe). With the flag off, or with no splitter child, this is
    /// behaviorally identical to the delegated draw.
    fn draw(&mut self, ctx: &mut DrawCtx) {
        if self.joined_lines {
            // The frame fills the window extent, so frame-local bounds = extent.
            let fb = self.group.state().get_extent();
            let marks = self.interior_splitter_mut().map(|s| s.frame_junction_marks(fb));
            if let (Some(marks), Some(frame)) = (marks, self.frame_mut()) {
                frame.set_junction_marks(marks);
            }
        }
        self.group.draw(ctx);
    }
```

- [ ] **Step 6: Run + review + accept the snapshots.**

Run:
```bash
cargo test -j4 -p tvision window::tests::joined_lines
cargo test -j4 -p tvision window::tests::without_flag_frame_is_plain
```
The two `joined_lines_*` tests will produce `.snap.new` files. Inspect them:
```bash
cargo insta review   # or read the .snap.new files directly
```
Verify against the spec's target diagrams:
- passive: the top row reads `┌────┬─────┐`-style (a `┬` where the divider column meets the top edge) and the bottom `└────┴─────┘` (a `┴`);
- active: `╔════╤═════╗` (a `╤`) and `╚════╧═════╝` (a `╧`).

If correct:
```bash
cargo insta accept
```
If the tee is one column off, the offset term in `collect_frame_marks` (Task 4) is wrong — fix it there, re-run, re-review. **This snapshot is the ground truth for the coordinate mapping.**

- [ ] **Step 7: Run the full window + frame + splitter suites (regression).**

Run:
```bash
cargo test -j4 -p tvision window:: frame:: splitter::
```
Expected: all PASS — every existing window snapshot (no `joined_lines`) is unchanged because the new `draw` only diverges when the flag is set.

- [ ] **Step 8: clippy + fmt + commit.**

```bash
cargo clippy -j4 --workspace --all-targets -- -D warnings
cargo fmt --all --check
git add src/window/window.rs src/window/snapshots/ 2>/dev/null || git add src/window/window.rs
git add -A   # picks up the accepted .snap files
git commit -m "feat(window): with_joined_lines — broker splitter marks to the frame

Window opts into line-joining: draw reads divider abutment marks from the
interior splitter child and pushes them to the frame child (sequential
borrows, marks cloned between). Off by default — plain windows unchanged.

Co-Authored-By: ..."
```

---

## Task 6: Splitter — interior `├`/`┼` crossings (Site 2)

The outer splitter overlays the correct tee/cross on its own divider cells where a perpendicular pane sub-splitter's divider meets it. This is the one rstv-original piece (TV has no splitter). It runs in `draw(&mut self)` (not the `&self draw_dividers`) because reading a pane child needs `&mut` access.

**Files:**
- Modify: `src/widgets/splitter/mod.rs`
- Test: inline `#[cfg(test)]` in `src/widgets/splitter/mod.rs`

- [ ] **Step 1: Write the failing snapshot test (the grid: outer cols + inner rows → `├` on the outer divider at the inner divider's row).**

Add to the `view_tests` module:

```rust
    #[test]
    fn interior_crossing_grid_renders_left_tee() {
        // Outer cols: [tree(fixed 6) | inner-rows(flex)]. The inner rows splitter
        // has a horizontal divider; where it meets the outer vertical divider, the
        // outer divider cell must show ├ (a vertical line branching right into the
        // inner pane), not a plain │.
        let inner = Splitter::rows()
            .pane(Fill::boxed('L'), Constraints::flex())
            .pane(Fill::boxed('F'), Constraints::flex());
        let mut outer = Splitter::cols();
        outer.change_bounds(Rect::new(0, 0, 20, 7));
        outer.insert(Fill::boxed('T'), Constraints::fixed(6));
        outer.insert(Box::new(inner), Constraints::flex());
        insta::assert_snapshot!(render(&mut outer, 20, 7));
    }
```

- [ ] **Step 2: Run to verify failure (no crossing yet).**

Run:
```bash
cargo test -j4 -p tvision splitter::view_tests::interior_crossing_grid_renders_left_tee
```
Expected: produces a `.snap.new` showing a plain `│` at the crossing (no `├`). Do **not** accept it — it is the wrong (pre-feature) output; it exists only to confirm the test runs. After Step 3 the snapshot will change; review it then.

- [ ] **Step 3: Implement `draw_interior_crossings` and call it from `draw`.**

In `src/widgets/splitter/mod.rs`, add the import for the junction selector to the existing `use crate::junction::…` line from Task 4:
```rust
use crate::junction::{Edge, Junction, JunctionMark, Weight, divider_junction};
```

Add the method to `impl Splitter`:
```rust
    /// Site 2 (rstv-original): overlay `├`/`┤`/`┴`/`┬`/`┼` on this splitter's own
    /// divider cells where a perpendicular pane sub-splitter's divider meets them.
    /// `&mut self` because reaching a pane child to read its divider positions
    /// needs `Group::child_mut` (the `&self draw_dividers` cannot do this). Reads
    /// the child's positions into an owned `Vec` (borrow released) before drawing
    /// on this splitter's own cells via `ctx`.
    fn draw_interior_crossings(&mut self, ctx: &mut DrawCtx) {
        // Only meaningful when this splitter has at least one divider of its own.
        if self.slots.len() < 2 {
            return;
        }
        let b = self.group.state().get_bounds();
        let weight = if self.reconfig.is_some() {
            Weight::Double
        } else {
            Weight::Single
        };

        // For each pane, gather (pane-local origin, perpendicular divider local
        // positions) from any sub-splitter, into owned data, then overlay.
        // A pane index p sits between divider (p-1) on its low side and divider p
        // on its high side. We branch toward the pane that holds the sub-splitter.
        let ids = self.group.child_ids_in_order();
        // Pane axis spans (low edge .. high edge), splitter-local, parallel to ids.
        let sizes = solve(&self.slots, self.content_len());

        for (p, id) in ids.iter().enumerate() {
            // The sub-splitter's bounds + perpendicular divider positions (owned).
            let info = self.group.child_mut(*id).and_then(|v| {
                v.as_any_mut()
                    .and_then(|a| a.downcast_mut::<Splitter>())
                    .filter(|sub| sub.orientation != self.orientation)
                    .map(|sub| {
                        let cb = sub.group.state().get_bounds();
                        let csizes = solve(&sub.slots, sub.content_len());
                        // Perpendicular divider local positions within the sub.
                        let mut pos = Vec::new();
                        let mut c = 0i32;
                        for i in 0..sub.slots.len().saturating_sub(1) {
                            c += csizes.get(i).copied().unwrap_or(0);
                            let full = matches!(sub.style_of(i), DividerStyle::Line)
                                || sub.reconfig.is_some();
                            if full {
                                pos.push(c);
                            }
                            c += 1;
                        }
                        (cb, pos)
                    })
            });
            let Some((cb, perp)) = info else { continue };

            // The outer dividers bordering this pane (in this splitter's local
            // axis coords): low side = divider (p-1), high side = divider p.
            let low = if p > 0 { self.divider_axis_pos(p - 1) } else { None };
            let high = if p < self.slots.len() - 1 {
                self.divider_axis_pos(p)
            } else {
                None
            };

            for d in &perp {
                // Cross-axis position of the sub-splitter's divider, in THIS
                // splitter's local coords. The sub's bounds share this splitter's
                // coordinate space, so subtract this splitter's origin.
                let (cross_local, branch_low, branch_high) = match self.orientation {
                    Orientation::Cols => {
                        // outer dividers are vertical (columns); sub dividers are
                        // horizontal (rows). cross-axis = the row.
                        let row = (cb.a.y - b.a.y) + d;
                        (row, Junction::TeeLeft, Junction::TeeRight)
                    }
                    Orientation::Rows => {
                        // outer dividers are horizontal (rows); sub dividers are
                        // vertical (cols). cross-axis = the column.
                        let col = (cb.a.x - b.a.x) + d;
                        (col, Junction::TeeUp, Junction::TeeDown)
                    }
                };
                // The pane is on the HIGH side of `low` and the LOW side of `high`.
                // A sub on the high side of the low divider → that divider branches
                // toward high (TeeRight / TeeDown). On the low side of the high
                // divider → branches toward low (TeeLeft / TeeUp).
                if let Some(ld) = low {
                    let glyph = divider_junction(branch_high, weight, ctx.glyphs());
                    self.put_crossing(ctx, ld, cross_local, glyph, weight);
                }
                if let Some(hd) = high {
                    let glyph = divider_junction(branch_low, weight, ctx.glyphs());
                    self.put_crossing(ctx, hd, cross_local, glyph, weight);
                }
            }
        }
    }

    /// Overlay one crossing glyph at (outer-divider axis pos, cross-axis pos) in
    /// this splitter's local coords, mapped to (x, y) by orientation.
    fn put_crossing(&self, ctx: &mut DrawCtx, axis: i32, cross: i32, glyph: char, _w: Weight) {
        let role = if self.reconfig.is_some() {
            Role::FrameDragging
        } else {
            Role::FramePassive
        };
        let st = ctx.style(role);
        let (x, y) = match self.orientation {
            Orientation::Cols => (axis, cross), // vertical divider at column `axis`, row `cross`
            Orientation::Rows => (cross, axis), // horizontal divider at row `axis`, col `cross`
        };
        ctx.put_char(x, y, glyph, st);
    }
```

Then call it from `draw` (in the `impl View for Splitter` block), after `draw_dividers`:
```rust
    fn draw(&mut self, ctx: &mut DrawCtx) {
        self.abs_origin = ctx.origin();
        self.group.draw(ctx);
        self.draw_dividers(ctx);
        self.draw_interior_crossings(ctx);
    }
```

> **Implementer note:** the `branch_low`/`branch_high` naming maps the side the sub-splitter sits on to which way the outer divider branches. For the canonical grid (outer cols, inner-rows pane on the *high* side of divider 0), the inner sits to the right of divider 0, so divider 0 (the pane's `low` divider) branches *toward high* = `TeeRight` = `├`. Verify the snapshot shows `├` (not `┤`); if mirrored, swap `branch_low`/`branch_high`. This is exactly what the Step 4 snapshot review locks down.

- [ ] **Step 4: Run + review + accept the snapshot.**

Run:
```bash
cargo test -j4 -p tvision splitter::view_tests::interior_crossing_grid_renders_left_tee
cargo insta review   # inspect the .snap.new
```
Confirm the outer vertical divider column shows `├` at the inner horizontal divider's row (a vertical line branching right into the inner pane). Then:
```bash
cargo insta accept
```
If the glyph is `┤` (branching the wrong way) or off by a row, fix per the implementer note / coordinate term and re-review.

- [ ] **Step 5: Run the full splitter suite (regression — no plain splitter changed).**

Run:
```bash
cargo test -j4 -p tvision splitter::
```
Expected: all PASS. The existing single-splitter snapshots have no perpendicular sub-splitter pane, so `draw_interior_crossings` is a no-op for them (the `< 2` guard or the `orientation != self.orientation` filter / empty `perp` skips everything) — those snapshots must be unchanged.

- [ ] **Step 6: clippy + fmt + commit.**

```bash
cargo clippy -j4 --workspace --all-targets -- -D warnings
cargo fmt --all --check
git add -A
git commit -m "feat(splitter): interior ├/┼ crossings where sub-splitter dividers meet

Outer splitter overlays the correct tee/cross on its own divider cells at
the positions a perpendicular pane sub-splitter's dividers meet it. Runs
in draw(&mut self) since reading a pane child needs &mut access; no-op for
a plain splitter.

Co-Authored-By: ..."
```

---

## Task 7: Example + docs

Rework the demo into the grid and record the feature.

**Files:**
- Modify: `examples/splitter.rs`
- Modify: `docs/IMPLEMENTATION-LOG.md`, `docs/PORT-ORDER.md`

- [ ] **Step 1: Rework the example into the nested grid inside a joined-lines window.**

In `examples/splitter.rs`, change `init_desktop` so the window opts into joined lines and the layout is the grid (`tree` column beside a stacked `list`/`form`). Replace the splitter-construction block (currently lines ~187–212) with:

```rust
        let mut win = tvision::Window::new(
            win_rect,
            Some("Multi-pane Splitter".to_string()),
            1,
        )
        .with_joined_lines();

        let ext = win.state().get_extent();
        let interior = Rect::new(1, 1, ext.b.x - 1, ext.b.y - 1);

        let tree = build_tree(interior);
        let list = build_list(interior);
        let form = build_form(interior);

        // Right side: list stacked over form (horizontal divider between them).
        let right = Splitter::rows()
            .pane(list, Constraints::flex().min(3))
            .pane(form, Constraints::flex().min(6));

        // Outer: fixed tree sidebar column beside the right grid.
        let split = Splitter::cols()
            .pane(tree, Constraints::fixed(22))
            .pane(Box::new(right), Constraints::flex())
            .divider(0, DividerStyle::Line);

        let split_id = win.insert_child(Box::new(split));
        if let Some(v) = win.child_mut(split_id) {
            v.change_bounds(interior);
        }
```

Update the module-doc controls/intro comment at the top of the file to describe the grid (a tree sidebar beside a stacked list/form) and that the divider lines now join the window frame.

- [ ] **Step 2: Build the example (compile-check; it is a TUI so do not run it interactively here).**

Run:
```bash
cargo build -j4 --example splitter
```
Expected: compiles cleanly. (Per the project's tmux-sandbox gotcha, do not attempt an interactive run from a non-tmux shell; a smoke-run, if desired, must be a single tmux launch+capture invocation.)

- [ ] **Step 3: Record the feature in the docs.**

Prepend a dated entry to `docs/IMPLEMENTATION-LOG.md` (newest first) summarizing: the splitter frame-joining feature (junction vocabulary + glyphs, frame mark-aware draw, splitter mark producer + interior crossings, window opt-in broker), and that it is gated/additive (D3-legal `frameLine` revival + rstv-original interior cross). Reference this plan and the spec.

In `docs/PORT-ORDER.md`, add a note under the existing Splitter extension row (or the rstv-original extensions section) that frame-joining is implemented on `feat/splitter`.

- [ ] **Step 4: Final full-workspace gate.**

Run:
```bash
cargo test -j4 --workspace
cargo clippy -j4 --workspace --all-targets -- -D warnings
cargo fmt --all --check
```
Expected: all PASS, clippy clean, fmt clean. If the repo has a doctest/example gate (`cargo xtask test`), run it too:
```bash
cargo xtask test 2>/dev/null || true
```

- [ ] **Step 5: Commit.**

```bash
git add examples/splitter.rs docs/IMPLEMENTATION-LOG.md docs/PORT-ORDER.md
git commit -m "feat(example,docs): grid splitter demo with joined frame lines

Rework examples/splitter.rs into the nested grid (cols[tree, rows[list,
form]]) inside a Window::with_joined_lines(); log the feature in
IMPLEMENTATION-LOG + PORT-ORDER.

Co-Authored-By: ..."
```

---

## Self-Review (run before declaring complete)

**Spec coverage (each spec section → task):**
- Component 1 (glyph set) → Task 1 Steps 1–3. ✓
- Component 2 (pure selectors) → Task 1 Steps 4–6. ✓
- Component 3 (Frame marks + mark-aware draw) → Task 2. ✓
- Component 4 (Window flag + brokering draw) → Task 5. ✓
- Component 5 keystone (`as_any_mut`) → Task 3. ✓
- Component 5 marks (`frame_junction_marks`, recursive, `&mut self`) → Task 4. ✓
- Component 5 interior crossings (`draw_interior_crossings` in `draw`) → Task 6. ✓
- Component 6 (example) → Task 7. ✓
- Weight handling (mixed glyph selection) → Task 1 `frame_junction` + Task 2 `active_double_frame_uses_mixed_tee_for_single_stem`. ✓
- Edge cases (Hidden/Locked/Handle no mark; inset no mark; no-splitter window) → Task 4 tests + Task 5 `without_flag` + `interior_splitter_mut` returning `None`. ✓
- Testing (pure unit, marks unit, snapshots, regression, downcast keystone assertion) → Tasks 1/3/4/5/6. ✓
- Non-goals (no new trait method; no read-back; off-by-default unchanged) → enforced by Task 3 (`delegate_view` green), the design (no `get_char` used), and Task 5 regression. ✓

**Type consistency:** `Edge`/`Weight`/`JunctionMark`/`Junction` defined once in Task 1 and used unchanged in Tasks 2/4/6. `frame_junction(edge, bar, stem, g)` and `divider_junction(dir, weight, g)` signatures are stable across all call sites. `set_junction_marks(Vec<JunctionMark>)` (Task 2) ↔ `frame_junction_marks(...) -> Vec<JunctionMark>` (Task 4) ↔ `frame.set_junction_marks(marks)` (Task 5) line up.

**Known risk to watch (flagged, not a placeholder):** the frame-local **offset mapping** in `collect_frame_marks` (Task 4) and the **cross-axis mapping** in `draw_interior_crossings` (Task 6) are off-by-one-prone because of the splitter's mixed local/owner coordinate handling (`draw_dividers` is 0-based; `resolve_layout_local` seeds from `b.a.x`). Both are pinned by **snapshot tests** (Task 5 Step 6, Task 6 Step 4) which are the ground truth — the plan explicitly routes verification through them and says exactly which term to adjust if a snapshot is one cell off.

---

## Execution Handoff

This plan implements one cohesive subsystem (splitter↔frame line-joining) — it is a single plan, not multiple.

Suggested execution: **subagent-driven** (`superpowers:subagent-driven-development`) — one fresh implementer subagent per task, two-stage review (spec-compliance, then code-quality) between tasks, integrating on the shared `feat/splitter` worktree. Tasks are sequential (each builds on the prior: glyphs → frame draw → keystone → marks → window broker → interior crossings → example), so do **not** parallelize them. Commit each task before dispatching the next (worktree subagents branch from the last commit).
