# Docs — user-facing cleanup across both layers (rustdoc + guide) — Design

**Date:** 2026-06-12
**Status:** Proposed (pre-implementation)
**Topic:** Make the whole documentation product — the rustdoc `/api/` layer **and**
the mdBook guide — read for **library users**, not porting contributors, and
reflect the **finished** state of the port. Fix the information architecture so
the "faithful" intro and the deviations reference stop competing.

---

## 1. Problem

The guide (Parts I–V) and the rustdoc were both written *during* the port and
leak internal porting-process language into outward-facing docs:

- **Bookkeeping with no meaning to a user:** PORT-ORDER row numbers, bare
  deviation labels (`D2`, `D9`), `FOUNDATION`/`MECHANICAL`/`INFRA` tags,
  references to the internal porting docs (PORT-ORDER, HANDOVER,
  IMPLEMENTATION-LOG, PORTING-GUIDE), "breadcrumb" notes.
  - rustdoc: ~64 files. Guide: `Dn` labels in 23 pages; internal-doc names in
    `reference/deviations.md` + `reference/symbol-map.md`.
- **Stale "deferred" claims.** The port is **done**, so "deferred / not ported
  yet / lands when X" is now either false or should be a *deliberate* non-port
  with a reason. Proven: `src/app/application.rs` calls `tile`/`cascade`
  "deferred… lands when `Desktop::tile`/`cascade` exist" — they exist
  (`desktop.rs:314`/`354`) and are menu-wired. (rustdoc: 33 doc-comment + 19
  code-comment hits.)
- **Two pages compete.** `port/faithful.md` (Part II opener) enumerates the
  deviations as narrative; `reference/deviations.md` (Part V) lists them as
  reference. They overlap and duplicate.
- **A load-bearing concept is hidden.** The **capture stack** (`src/capture.rs`)
  is the unified mechanism behind modality, mouse tracking, window drag/resize,
  and press-and-hold across ~10 widgets — yet the guide files it under a "modal"
  sub-bullet. The linear D-catalog framing flattens exactly these cross-cutting
  ideas.

The C++/Turbo Vision heritage is **not** the problem — it is valuable to the
veteran audience. It just must be *quarantined*, not interleaved with bookkeeping.

## 2. Goals

1. Every item's/page's **primary prose** describes what it does *for the user
   today* — no row numbers, `Dn` labels, internal-doc names, or "breadcrumb"s.
2. No doc claims unfinished work: only **ported** (described as working) or
   **deliberately not ported** (stated plainly, with a real reason).
3. C++ heritage is preserved, quarantined (rustdoc: a standard section; guide:
   the veteran-facing Part II narrative).
4. The guide's "faithful" intro and the differences reference each have **one
   job** and cross-link; event **capture** is a first-class topic.
5. No behavior change — doc comments, `//`-comments, and Markdown only.

## 3. Rules for the rustdoc layer

### Rule A — Strip porting bookkeeping (mechanical)
Remove from **primary, Rust-API-explaining prose**: row numbers; bare `Dn`
labels; `FOUNDATION`/`MECHANICAL`/`INFRA`; names of internal porting docs;
"breadcrumb" phrasing. A deviation *concept* may stay if reworded plainly
("embed-and-delegate composition", not "D2").

**Exception — `Dn` citations belong in the heritage/C++ context.** A
`(deviation D8)` citation **with a link** to the canonical differences list is
*welcome* in the Turbo Vision / C++ heritage section (Rule C below, and the
guide's C++-origin discussion). The distinction is positional: a linked `Dn`
citation when relating the design to C++ = good; a bare `Dn` label while
explaining the Rust API = removed. This implies the differences reference carries
a stable per-deviation anchor (`#d8`) to link to.

### Rule B — "Deferred" is a bug to investigate, never to copy
For every `deferred` / `not ported yet` / `lands when…` / `TODO`, check the
implementation, then: **(a) implemented** → rewrite as working; **(b) genuinely
not ported** → state plainly as a deliberate non-port **with the real reason**
(often in HANDOVER "latent edge notes" or the surrounding code), never the word
"deferred"; **(c) no good reason found** → **FLAG for the user**, never invent or
silently delete.
- **Nuance:** the **Deferred channel / deferred effects** is a real runtime
  feature (the `Deferred` enum, effects routed to the loop owner). That sense is
  CORRECT and stays. Only porting-"deferred" is removed.

### Rule C — Quarantine heritage into a standard section
Major public items (modules + primary `struct`/`trait`/`enum`; not every method)
end with a fixed-heading section:

```rust
//! # Turbo Vision heritage
//! Ports `TProgram` / `TApplication` (`tprogram.cpp`, `tapplica.cpp`).
//! C++ `TApplication : TProgram` inheritance becomes embed-and-delegate
//! composition here — one type holds the other and forwards to it.
```
- Heading: **`# Turbo Vision heritage`** (verbatim). Always: C++ class/fn + its
  header/source file. Translation note: only where the mapping is non-obvious
  (inheritance→trait, pointers→`ViewId`, flags→bools, palette→`Role`).
- Guide back-links from rustdoc: **deferred to a separate pass** (link fragility).

## 4. Rules for the guide layer

Same A and B (with the same Deferred-channel nuance). Heritage in the guide is
already quarantined into Part II's veteran narrative — keep it. Plus the IA fixes:

### IA-1 — `Dn` labels out of prose
Drop `D2`/`D5`/… from the Part II narrative, the apps/, and the internals/ pages.
Describe each difference plainly. (Stable IDs, if wanted, survive **only** in the
canonical differences list — see IA-3.)

### IA-2 — `faithful.md` = philosophy + gateway, not a competing list
Rewrite `port/faithful.md` to cover *what "faithful" means* and *why*, and to
**introduce + link** the topic chapters and the canonical differences list. It
stops re-enumerating the deviations.

### IA-3 — `reference/deviations.md` → "Differences from C++ Turbo Vision"
The single canonical at-a-glance list. Each entry **links** to its Part II
narrative chapter and (where apt) the rustdoc. Drop "PORTING-GUIDE is the spec";
replace with at most a "contributors: see the repo" pointer. `symbol-map.md`
likewise stops treating internal docs as canonical.

### IA-4 — Event capture becomes first-class
Cover the **capture stack** as the one mechanism for modality + mouse-tracking +
drag/resize + press-and-hold:
- Part II: **its own page** — "Event capture — one mechanism for modal, drag/
  resize & press-and-hold" (new `SUMMARY.md` entry). `modal.md` narrows to
  "execView → one loop" and links to it.
- Part IV: the mechanics live with the event loop / brokering; ensure capture is
  named and explained, not buried.

## 4a. Naming convention (project = `rstv`)

To avoid confusion with the C++ `tvision` it was ported from, the **project /
product** is named **`rstv`** in all docs. This is **branding-only**:
- **`rstv`** — the project/product name. Guide title "rstv — Developer Guide";
  standalone references ("rstv is a faithful Rust port…", "rstv does X").
- **`tvision`** — the **crate** name, unchanged (`tv = { package = "tvision" }`).
- **`tv::`** — the import namespace, unchanged.
- **"Turbo Vision" / "magiblot/tvision"** — the C++ origin, unchanged.

Standard phrasing: *"rstv is a TUI framework for Rust; add the `tvision` crate,
imported as `tv`."* Applies to the already-authored Part I–V pages too (they
currently use "tvision" as the product name) — swept in this pass.

## 5. Scope

- **In:** `//!`/`///` doc comments + false-signal `//`-comments across `src/`
  (~64 files); all guide Markdown under `docs/book/src/`. Doc/markdown only — no
  behavior change.
- **Out:** rustdoc→guide hyperlinks (separate task); the internal `docs/*.md`
  porting files (they serve contributors; the "no deferred" principle applies to
  them later, not here); method-by-method heritage sections.

## 6. Execution & verification

Subagent-driven, grouped by module / guide-section (editor + reviewer per group;
Rule-B class-(c) items escalate to the orchestrator → the user). Gate on the
integrated tree: `cargo doc --no-deps -p tvision` clean; `cargo test --workspace`
+ `clippy --all-targets` + `fmt --check` green; `cargo xtask docs` link check
clean (catches any broken cross-links introduced by IA-2/IA-3). **Invariant
sweep** after the pass: `grep -rE '\b(deferred|row [0-9]|FOUNDATION|MECHANICAL|D[0-9]+|PORT-ORDER|HANDOVER|PORTING-GUIDE)\b'`
over `src/` and `docs/book/src/` returns only the Deferred-channel feature sense
and the canonical-list stable IDs — every other hit is gone or explained.

## 7. Success criteria

1. A reader of `/api/` or the guide never meets a row number, a `Dn` label in
   prose, or an internal-doc reference.
2. No doc says "deferred" except the real Deferred-channel feature. Every former
   deferral is described as working, stated as a reasoned non-port, or escalated.
3. rustdoc items carry a `# Turbo Vision heritage` section; heritage no longer
   pollutes primary prose.
4. `faithful.md` and the differences reference each have one job and cross-link;
   the capture stack is a first-class, discoverable topic.
5. Build + tests + link check stay green; zero behavior change.
