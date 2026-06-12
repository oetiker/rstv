# rstv Developer Documentation — Design

**Date:** 2026-06-12
**Status:** Approved design (pre-implementation)
**Topic:** A developer-facing documentation system for the `tvision` crate (rstv).

---

## 1. Problem & goal

rstv's `docs/` today are **internal porting artifacts** — `HANDOVER.md`,
`IMPLEMENTATION-LOG.md`, `PORTING-GUIDE.md`, `PORT-ORDER.md`, `briefs/`,
`design/`. They serve the porting effort, not a developer who arrives wanting to
*use* the crate.

**Goal:** an outward-facing documentation product for developers who build TUI
apps with `tvision`, deep enough that a reader can also implement their own
`View`. It must feel like **one integrated experience**, not a book in one place
and an API reference in another.

### Audience (in priority order)

1. **Library users** — Rust developers building TUI apps with `tvision`. They may
   or may not know C++ Turbo Vision. This is the primary audience.
2. **Custom-view authors** — users who need to extend the framework with new
   views; served by a dedicated architecture/internals part.

A C++ Turbo Vision veteran is a **cross-cutting lens**, not a third audience: a
dedicated part (Part II) explains the ideas behind the idiomatic port for readers
arriving with that background.

**Not** a primary audience: porting contributors. The existing `docs/` already
serve them; the new docs only *point* there (Part V).

---

## 2. Architecture: two layers, single-sourced

Rust gives a library a better documentation shape than a standalone book. We use
**two layers**, with **one source of truth per fact**.

### Layer 1 — In-source reference (rustdoc)

Per-component reference docs live **in the source** as rustdoc (`//!` module docs,
`///` item docs). This already exists at ~90% coverage: every widget module has
`//!` docs and there are ~2,600 `///` item docs in `src/widgets/` alone (e.g.
`src/widgets/button.rs` already carries a C++ cross-reference and a `# Model`
section). This layer is the **per-component reference**.

Work in this layer:
- **Complete the gaps** — notably `src/theme/` (currently zero module/item docs).
- **Promote examples to doctests** — code in `///` blocks is compiled by
  `cargo test --doc`, so reference examples cannot rot.
- **Add outbound guide links** — a `See the [Dialogs guide](…)` line in the
  relevant module `//!` so the reference points back into the narrative.
- **Publish** via `cargo doc`.

### Layer 2 — The Guide (mdBook)

The narrative a developer reads to *learn*. It **never restates API signatures**.
Instead it:
- **Pulls code from real `examples/*.rs`** via `{{#rustdoc_include
  examples/hello.rs:anchor}}` (anchored with `// ANCHOR:`/`// ANCHOR_END:`
  comments in the example) — so guide code is compiled and never drifts.
- **Links into the rustdoc** for exact API (`[Button](../api/tvision/struct.Button.html)`).
- **Carries colored screenshots** of the running result (see §4).

### The bridge — zero drift, enforced in CI

- `cargo test --doc` compiles every rustdoc doctest.
- `mdbook test` compiles every Rust snippet in the guide.
- `mdbook-linkcheck` fails the build on a broken cross-link.
- Where *identical prose* is wanted in both layers, `#[doc =
  include_str!("…")]` pulls one shared `.md` fragment into rustdoc. Used
  sparingly; most pages are guide-prose XOR reference-prose, not both.

---

## 3. Integrated delivery — one unified site

Single GitHub Pages deploy that reads as one product:

- **Guide at the site root** (`/rstv/`), **rustdoc at `/rstv/api/`** — the
  `cargo doc` output is copied into the deployed tree under `api/`.
- **Bidirectional deep links:** guide → `../api/tvision/struct.*.html`; rustdoc
  modules → guide chapters (outbound links in `//!`).
- **Shared identity:** same logo/favicon/palette in both. mdBook via its theme
  overrides (`docs/book/theme/`); rustdoc via `#![doc(html_logo_url = …,
  html_favicon_url = …)]` plus a small injected header
  (`RUSTDOCFLAGS="--html-in-header …"`) carrying a "← Guide" link and matching
  colors.
- **Top-bar "Guide ⇄ API" toggle** present in both surfaces, so they read as two
  tabs of one site.
- **One CI deploy job** builds the guide, builds the rustdoc, assembles the
  combined tree, runs the link check, and publishes.

Out of scope: a custom mdBook preprocessor that auto-links every type mention or
inlines API summaries ("deeply embedded"). Diminishing returns + fragile against
rustdoc HTML changes.

---

## 4. Screenshots — colored, selectable, from tmux

rstv is a TUI. Screens are captured as **colored, selectable HTML** (not PNGs):

```
run example in fixed-size tmux pane
  → tmux send-keys  (drive it into the desired state)
  → tmux capture-pane -e -p   (colored ANSI — verified working)
  → repo-owned Rust ANSI→HTML converter  (themed <pre>)
  → embedded in the mdBook page
```

`tmux capture-pane -e -p` preserves the full SGR color/attribute escapes and the
UTF-8 box-drawing — **verified** during design. This beats PNGs: selectable,
copy-pasteable, scalable, tiny, diffable, and it supports **interactive** shots
(send keystrokes to open a menu/dialog, then capture).

The **ANSI→HTML converter is repo-owned Rust** (no external `aha` dependency),
living inside the xtask crate (§5) so it shares the build. It maps SGR codes to
the same theme colors the crate uses and emits a self-contained `<pre>` (inline
styles or a small shared stylesheet) that mdBook embeds directly.

Driving discipline: each example launch + `send-keys` + `capture-pane` runs as a
**single** orchestration step (the established rstv tmux pattern).

---

## 5. Tooling — pure `cargo`, no mise, no Makefile

A **`cargo xtask`** workspace crate (`xtask/`) is the single entry point. No
external task runner and no tool manager.

- `cargo xtask docs` — build the unified site (guide + rustdoc + screenshots +
  link check + assemble `api/`).
- `cargo xtask docs --serve` — live-reload dev server.
- `cargo xtask screens` — regenerate the tmux screenshots only.

**mdBook is a library dependency** of the xtask crate: it drives the build
in-process via `mdbook`'s `MDBook::load(path)?.build()`, with the
`mdbook-mermaid` and `mdbook-linkcheck` preprocessors/backends wired
programmatically. This is fully reproducible from the workspace `Cargo.lock` with
**zero global installs**. (Trade-off accepted: couples to mdbook's library API,
which can churn across major versions; pinned by `Cargo.lock`.)

The **ANSI→HTML converter and tmux orchestration live in the same xtask crate**,
so the converter, screenshot capture, and doc build are one Rust program.

CI invokes the same `cargo xtask docs` to build + link-check + deploy.

---

## 6. Book outline

### Front
- **Introduction** — what tvision is, the C++ Turbo Vision heritage, the `tv::`
  house style, and how to read these docs (Guide vs API).

### Part I · Getting Started
- **Installation & the `tv::` alias** — Cargo setup.
- **Your first app** — desktop + menu bar + status line (the `hello` example),
  with a colored screenshot; code pulled from `examples/hello.rs`.
- **The application skeleton** — `Application`/`Program` and the run loop at a
  glance (high level; depth is Part IV).

### Part II · The Idiomatic Port — for Turbo Vision veterans
*Optional for newcomers; if you know C++ Turbo Vision, read this next. This is the
**narrative behind the deviations** — the *ideas* that turned the C++ class
library into idiomatic Rust. The formal deviation list is D1–D13 in
`docs/PORTING-GUIDE.md`; the terse C++→Rust symbol lookup is in Part V. Each topic
links forward to Part IV (How It Works) for the runtime mechanics and to the
rustdoc.*
- **What "faithful" means** — behavior is ported verbatim; the *only* intentional
  departures are the pre-decided D1–D13. Why a faithful port at all.
- **Inheritance → trait + composition** — the `TView` class hierarchy becomes the
  `View` trait + `ViewState` composition; `#[delegate]` embed-and-delegate (D2).
- **Pointers & `infoPtr` → handles** — raw `TView*` becomes `ViewId` handles
  resolved through a downward-borrowed `Context` (D3).
- **Events → `enum Event` + match** (D4), including `Event::Broadcast`.
- **Flag words → struct-of-bools** — `ofXxx`/`sfXxx`/`bfXxx` bitmasks become named
  boolean fields (D5).
- **Constant families → open newtypes** — `cmXxx`/`hcXxx` become namespaced
  `&'static str` consts (`Command::OK`); newtype-vs-enum chosen by extensibility.
- **Palettes & glyph tables → `Theme`/`Role`** — the index-into-palette scheme
  becomes a typed theme.
- **The draw model → whole-tree redraw + diff** — no damage tracking; a
  back-buffer diff replaces it.
- **Modal `execView` → one loop + capture stack** — nested modal loops collapse
  into a single event loop with a capture stack (D9).
- **The `Deferred` channel** — effects a downward-borrowed `&mut View` can't
  perform inline are deferred to the loop owner.
- **Dropped/changed** — `TStreamable` dropped (serde if revived); coordinates stay
  `i32`, faithful to magiblot's `int`.

### Part III · Building Apps
*Task recipes. Each topic = a screenshot + example-sourced code + a link into the
rustdoc reference. Not a method-by-method dump.*
- Windows & the desktop (z-order, tile/cascade)
- Dialogs & data (modal `exec_view`, gather/scatter)
- Controls — buttons · checkboxes · radio · input lines + validators · labels ·
  list boxes · scrollbars
- Menus, status line & context-sensitive help (`HelpCtx`)
- Commands & the event model (enable/disable, broadcasts)
- Keyboard & configurable key mapping (`Keymap`)
- Theming & colors (`Theme`, `Role`)
- Text editing (`Memo`, `FileEditor`, `Terminal`)

### Part IV · How It Works
*Architecture, ending at the path to custom views.*
- The view tree: `View` trait + `ViewState`
- The event loop in depth (`pump_once`, the capture stack)
- Deferred effects (why a leaf can't mutate loop state; adding an effect)
- Cross-view brokering & `ViewId`
- Drawing: `DrawBuffer`/`Cell`, back-buffer diff, the `Backend` trait
- **Writing your own View** (capstone) + delegation (`#[delegate]`)

### Part V · Reference
- How the rustdoc API is organized (where per-component reference lives)
- C++ Turbo Vision → tvision symbol map — terse lookup table (the *narrative* is
  Part II)
- Deviations D1–D13 (summary + link to `docs/PORTING-GUIDE.md`)
- The screenshot tooling (how to add/regenerate a screen)
- Pointer to the internal porting docs (for contributors)

---

## 7. Placement & repository layout

- The mdBook lives in **`docs/book/`** — its own mdBook root (`docs/book/book.toml`,
  `docs/book/src/`, `docs/book/theme/`). Existing `docs/*.md` porting docs are
  **untouched**.
- The **`xtask/`** crate is a new workspace member (`Cargo.toml` `[workspace]`
  members updated).
- Generated screenshots land under `docs/book/src/screens/` (committed, so a
  plain `mdbook build` works without tmux available).
- Anchored example code stays in `examples/*.rs` (adding `// ANCHOR:` comments as
  needed — comments only, no behavior change).

---

## 8. Scope boundaries (YAGNI)

**In scope:** the two-layer architecture; completing rustdoc gaps (theme) +
doctests; the mdBook guide (outline above, including the Part II "idiomatic port"
narrative for C++ veterans); the unified-site assembly + CI deploy; the
tmux→Rust screenshot pipeline; the xtask tooling.

**Out of scope:**
- A separate porting-contributor track (existing `docs/` serve it; Part V points
  there).
- A deeply-embedded auto-linking mdBook preprocessor.
- PNG rasterization of screens.
- Retaining mise / Makefile for docs (replaced by `cargo xtask`).

---

## 9. Success criteria

1. `cargo xtask docs` produces a single site tree: guide at root, rustdoc at
   `api/`, screenshots embedded, link check clean — with no global tool installs.
2. The guide's getting-started path takes a reader from `cargo add` to a running
   `hello`-style app, with a colored screenshot of the result.
3. Every code snippet in the guide and every rustdoc doctest compiles in CI
   (`mdbook test` + `cargo test --doc`).
4. `src/theme/` reaches parity with other modules' rustdoc coverage.
5. A reader can follow Part IV to implement a trivial custom `View` end to end.
6. A C++ Turbo Vision veteran can read Part II and understand the rationale for
   each major deviation (inheritance→trait+composition, pointers→`ViewId`, flag
   words→struct-of-bools, modal loops→single loop+capture, …).
7. Guide ⇄ API cross-links resolve in both directions on the deployed site.
