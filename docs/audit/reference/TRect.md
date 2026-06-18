# TRect  (guide pp. 518–519)

Rust module(s): `src/view/geometry.rs` (re-exported as `tv::Rect`)   |   magiblot: `include/tvision/objects.h`

> TRect is a rectangle defined by two TPoint corners.  The 1992 guide documents
> 2 fields and 8 methods; magiblot's C++ header adds `operator==`, `operator!=`,
> `isEmpty`, two-point and coordinate constructors, and stream operators
> (ipstream/opstream).  The guide's `Assign` and `Copy` are Pascal-era helpers
> that have direct Rust structural equivalents.

| Guide entry | Pg | Bucket | Corr | Rust symbol / mapping | Doc | Notes |
|---|---|---|---|---|---|---|
| `A` (field, TPoint) | 518 | PORTED | OK | `tv::Rect::a: Point` | 2 | Guide: top-left corner. Rust: `pub a: Point`. Score 2: field is documented as "top-left corner (inclusive)" in struct doc. Missing explicit "when is `a` > `b` valid?" guidance. |
| `B` (field, TPoint) | 518 | PORTED | OK | `tv::Rect::b: Point` | 2 | Guide: bottom-right corner. Rust: `pub b: Point`. Struct doc notes half-open (b exclusive). Score 2. |
| `Assign` (procedure, sets all four coordinates) | 518 | EQUIVALENT | OK | `tv::Rect::new(ax, ay, bx, by)` (const fn) | 2 | Guide: `Assign(XA, YA, XB, YB: Integer)` sets the four corner coordinates in place. Rust: `Rect::new(ax, ay, bx, by)` constructs a new Rect; in-place coordinate assignment is done by a plain struct literal or field assignment (`r.a.x = …`). Functionally equivalent: caller creates or reassigns. Score 2. |
| `Contains` (function, Boolean) | 518 | PORTED | OK | `tv::Rect::contains(&self, p: Point) -> bool` | 2 | Guide: returns True if rectangle contains point P. Rust: `contains` with half-open semantics (`p.x < b.x`, `p.y < b.y`). magiblot uses the same half-open test. Score 2: what it does is documented; the half-open edge exclusion is noted in a test comment but not in the method doc itself. → could reach 3 with one sentence on half-open semantics. |
| `Copy` (procedure, copy from R: TRect) | 518 | EQUIVALENT | OK | `#[derive(Clone, Copy)]` + struct assignment `r1 = r2` | N/A | Guide: `Copy(R: TRect)` sets all fields from R. Rust: `Rect` is `Copy`, so `r1 = r2` copies all fields. No named method needed. N/A: not a public method. |
| `Empty` (function, Boolean) | 518 | PORTED | OK | `tv::Rect::is_empty(&self) -> bool` | 2 | Guide: True if rectangle contains no character spaces (A == B essentially). Rust: `is_empty` returns true when `a.x >= b.x || a.y >= b.y` — matches magiblot exactly. Renamed `isEmpty` → `is_empty` per Rust convention. Score 2: what it does; missing note that an inverted rect (b < a) also returns true. |
| `Equals` (function, Boolean) | 519 | EQUIVALENT | OK | `#[derive(PartialEq, Eq)]` (`r1 == r2`) | N/A | Guide: `Equals(R: TRect): Boolean`. C++ `operator==` in magiblot. Rust: derived `PartialEq`; `r1 == r2` works. N/A: derived, not a named public function. |
| `Grow` (procedure, ADX ADY: Integer) | 518 | PORTED | OK | `tv::Rect::grow(&mut self, dx: i32, dy: i32) -> &mut Self` | 2 | Guide: changes size by ±ADX/ADY symmetrically. Rust: `grow` with identical `a.x -= dx / b.x += dx` semantics; returns `&mut Self` for chaining. Score 2: what it does; missing note that negative args deflate. |
| `Intersect` (procedure, R: TRect) | 519 | PORTED | OK | `tv::Rect::intersect(&mut self, r: &Rect) -> &mut Self` | 2 | Guide: clips to the intersection with R. Rust: identical `max/min` logic; returns `&mut Self`. Score 2. |
| `Move` (procedure, ADX ADY: Integer) | 519 | PORTED | OK | `tv::Rect::r#move(&mut self, dx: i32, dy: i32) -> &mut Self` | 2 | Guide: translates both corners by (ADX, ADY). Rust: `r#move` (raw identifier to avoid keyword clash — documented in module doc). Identical logic. Score 2: what + rename rationale is in the module doc but not in the method doc itself. |
| `Union` (procedure, R: TRect) | 519 | PORTED | OK | `tv::Rect::r#union(&mut self, r: &Rect) -> &mut Self` | 2 | Guide: expands to the bounding box of self and R. Rust: `r#union` (raw identifier — keyword clash). Same `min/max` logic. Score 2: module doc explains rename. |
| `operator==` (C++ header) | — | EQUIVALENT | OK | `#[derive(PartialEq, Eq)]` | N/A | See `Equals` above. N/A. |
| `operator!=` (C++ header) | — | EQUIVALENT | OK | `#[derive(PartialEq)]` (provides `!=`) | N/A | Derived automatically. N/A. |
| `isEmpty` (C++ header, not in guide) | — | PORTED | OK | `tv::Rect::is_empty` | 2 | Present in magiblot header. Covered under guide `Empty` entry above. |
| Two-arg constructor `TRect(TPoint, TPoint)` (C++ header) | — | EQUIVALENT | OK | `tv::Rect::from_points(p1: Point, p2: Point)` (const fn) | 2 | C++ `TRect(TPoint p1, TPoint p2)`. Rust: `from_points`. Score 2. |
| Default constructor `TRect()` → zeros (C++ header) | — | EQUIVALENT | OK | `#[derive(Default)]` → `Rect { a: (0,0), b: (0,0) }` | N/A | C++ `TRect()` zero-initializes. Rust: `Default::default()` yields the same. Verified by test `rect_constructors_equivalent`. N/A: derived. |
| stream `operator>>` / `operator<<` (C++ header) | — | NOT-PORTED | — | — | — | Borland TStreamable serialization layer. Known idiomatic mapping: `TStreamable`/streams → dropped (serde-if-revived). Intentional. |
| `Hash` / `Debug` (Rust extras) | — | NOT-PORTED | — | `#[derive(Debug, Hash)]` | — | Rust additions for ergonomics; not guide entries. Out-of-scope additions, not gaps. |

## Summary

- PORTED: 7   EQUIVALENT: 7   NOT-PORTED: 2   MISSING: 0   UNSURE: 0
- SUSPECT: 0   |   doc<3 (public): 7 (`a`, `b`, `new`, `from_points`, `contains`, `grow`, `intersect`, `r#move`, `r#union`, `is_empty`)   |   → concept: 0
- Notable findings: No gaps or suspect items — every guide method has a direct counterpart, and the two raw-identifier renames (`r#move`, `r#union`) are correctly explained in the module doc. The most actionable doc gap: `contains` scores 2 but the half-open edge-exclusion rule (right/bottom edges excluded) is critical for correct hit-testing and deserves a sentence in the method doc itself, not just in a test comment. Seven public symbols currently score 2; all could reach 3 with a "when/how to use" phrase.
