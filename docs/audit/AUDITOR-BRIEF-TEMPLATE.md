# Auditor brief template

Fill `<...>` and dispatch a fresh read-only auditor. The auditor reads the PDF
pages + the Rust source, writes `docs/audit/reference/<Section>.md`, and returns
only status + counts.

---

You are a **READ-ONLY auditor** for the `tvision-rs` project. Do **not** edit any
`src/`, rustdoc, or mdBook file. Your only write is the one reference file named
below.

**Section(s):** `<names>`
**Guide PDF:** `/home/oetiker/checkouts/rstv/Turbo_Vision_Version_2.0_Programming_Guide_1992.pdf`
**Guide pages:** `<range>` — Read in ≤20-page slices (the Read tool's `pages` param).
**Rust module(s) to read:** `<paths>` — also `rg`/glob to confirm; the hint may be incomplete.
**magiblot reference (original C++ semantics):** `/home/oetiker/scratch/tvision-spec/magiblot-tvision/include/tvision/<header>` (+ `source/tvision/`).
**Write your output to:** `docs/audit/reference/<Section>.md`

## What to do

1. Read the guide pages for the section. Enumerate **every** Field, Method, and
   Palette entry it documents (for a type/global/constant section: every
   variable / procedure / function / constant family / record field).
2. Read the Rust source. For **each** guide entry, emit one table row using the
   schema below. **Never omit an entry.**
3. Classify on all three axes (full definitions in `docs/audit/README.md`):
   - **Bucket:** `PORTED` (name the `tv::` symbol) · `EQUIVALENT` (analog + one-line
     mapping) · `NOT-PORTED` (written reason) · `MISSING` (you searched `src/`
     and found no counterpart) · `UNSURE` (write your question; never omit).
   - **Corr** (PORTED/EQUIVALENT only): `OK` · `SUSPECT` (undocumented divergence —
     cite the specific divergence + the guide page). A deliberate, commented
     deviation is `OK`.
   - **Doc** (public PORTED/EQUIVALENT only; else `N/A`): `0`/`1`/`2`/`3` per the
     rubric; add `→ concept` if the gap belongs in the mdBook, not the symbol.
4. **Known idiomatic mappings — treat as `EQUIVALENT`, not `MISSING`/`SUSPECT`:**
   flag word → struct-of-bools; `getData`/`setData` → value protocol
   (`src/data.rs`); class Palette → `tv::Theme` (`src/theme.rs`); `infoPtr`/
   pointers → `ViewId`; `TStreamable`/streams → dropped; DOS/EMS/memory-manager →
   no analog; `TCollection` family → Rust `Vec`.

## Output file format

```markdown
# <Section>  (guide pp. <start>–<end>)

Rust module(s): <paths>   |   magiblot: <header>

| Guide entry | Pg | Bucket | Corr | Rust symbol / mapping | Doc | Notes |
|---|---|---|---|---|---|---|
| ... one row per guide entry ... |

## Summary
- PORTED: n   EQUIVALENT: n   NOT-PORTED: n   MISSING: n   UNSURE: n
- SUSPECT: n   |   doc<3 (public): n   |   → concept: n
- Notable findings: <one or two lines on the most important MISSING/SUSPECT, if any>
```

## Return to the controller (ONLY this — do not paste the table)

`STATUS: DONE` · the section file path(s) written · the Summary counts line(s) ·
one line on the single most important finding (or "no gaps").
