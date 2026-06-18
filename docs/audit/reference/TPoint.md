# TPoint  (guide p. 501)

Rust module(s): `src/view/geometry.rs` (re-exported as `tv::Point`)   |   magiblot: `include/tvision/objects.h`

> TPoint is a simple struct with two fields and six arithmetic/comparison
> operators.  The guide documents only the two fields; all operators are in the
> C++ header.  The Borland 1992 guide shows no methods — just `X` and `Y`.
> magiblot adds `+=`, `-=`, `+`, `-`, `==`, `!=` plus stream operators (ipstream/opstream).

| Guide entry | Pg | Bucket | Corr | Rust symbol / mapping | Doc | Notes |
|---|---|---|---|---|---|---|
| `X` (field, Integer) | 501 | PORTED | OK | `tv::Point::x: i32` | 2 | Guide: screen column. Rust: `pub x: i32`. Uses `i32` rather than Pascal `Integer` (16-bit) — faithful to magiblot which uses `int`. Coordinate type rationale documented in module doc. Score 2: says what it is, not when signed values arise (scrolled-offscreen). |
| `Y` (field, Integer) | 501 | PORTED | OK | `tv::Point::y: i32` | 2 | Guide: screen row. Same i32 rationale as X. Score 2. |
| `operator+=` (C++ header) | — | EQUIVALENT | OK | `impl AddAssign for Point` | 2 | C++: `TPoint& operator+=(const TPoint&)`. Rust: `AddAssign` trait, `p += q`. Idiomatic operator overloading — same semantics. Score 2: trait impl present, no usage guidance. |
| `operator-=` (C++ header) | — | EQUIVALENT | OK | `impl SubAssign for Point` | 2 | C++: `TPoint& operator-=(const TPoint&)`. Rust: `SubAssign` trait, `p -= q`. Score 2. |
| `operator+` (C++ header) | — | EQUIVALENT | OK | `impl Add for Point` | 2 | C++: `friend TPoint operator+(const TPoint&, const TPoint&)`. Rust: `Add` trait, returns new `Point`. Score 2. |
| `operator-` (C++ header) | — | EQUIVALENT | OK | `impl Sub for Point` | 2 | C++: `friend TPoint operator-(const TPoint&, const TPoint&)`. Rust: `Sub` trait. Score 2. |
| `operator==` (C++ header) | — | EQUIVALENT | OK | `#[derive(PartialEq, Eq)]` | N/A | C++: `friend int operator==(const TPoint&, const TPoint&)`. Rust: derived `PartialEq` / `Eq`. `p == q` works identically. N/A: derived, not a named public symbol to score. |
| `operator!=` (C++ header) | — | EQUIVALENT | OK | `#[derive(PartialEq)]` (provides `!=`) | N/A | C++ has an explicit `operator!=`. Rust's derived `PartialEq` provides `!=` automatically. N/A for same reason. |
| stream `operator>>` / `operator<<` (C++ header) | — | NOT-PORTED | — | — | — | ipstream/opstream are the Borland TStreamable serialisation layer. Per known idiomatic mapping: `TStreamable`/streams → dropped (serde-if-revived). No Rust analog exists; intentional. |
| `new` / default constructor (C++ TRect has one; TPoint relies on POD zero-init) | — | EQUIVALENT | OK | `tv::Point::new(x, y)` (const fn) + `#[derive(Default)]` | 2 | C++ TPoint is POD; zero-init is implicit. Rust: `Point::new(x, y)` for explicit construction; `Default::default()` → `(0, 0)`. Docs explain the constructor. Score 2: present, not linked to zero-init convention. |
| `Hash` / `Copy` / `Clone` / `Debug` (Rust extras) | — | NOT-PORTED | — | `#[derive(Clone, Copy, Debug, Hash)]` | — | C++ TPoint has none of these. Rust derives them as quality-of-life additions consistent with the project's Rust-idiomatic approach. These are additions, not missing items. NOT-PORTED is N/A here — these are Rust extras, not guide entries. Classified as out-of-scope additions. |

## Summary

- PORTED: 2   EQUIVALENT: 7   NOT-PORTED: 1   MISSING: 0   UNSURE: 0
- SUSPECT: 0   |   doc<3 (public): 4 (`x`, `y`, `Point::new`, operator impls)   |   → concept: 0
- Notable findings: No gaps or suspect items. All arithmetic operators are faithfully present as idiomatic Rust trait impls. The one NOT-PORTED entry (stream operators) is correct and expected. The four public symbols scoring 2 could each be nudged to 3 by adding a sentence on when negative coordinates arise (scrolled-offscreen views) — the module doc explains this but the field-level docs do not.
