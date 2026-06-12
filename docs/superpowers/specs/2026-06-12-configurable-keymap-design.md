# Configurable Keymap — design spec

> **Status:** design approved (brainstorm complete), pending implementation plan.
> **Date:** 2026-06-12. **Type:** rstv-original *extension* on a faithful base —
> the WordStar default is a 1:1 port of the C++ editor key tables; the data-driven
> engine, the CUA/Emacs presets, and the input-line unification are modern
> extensions *alongside* the port (precedent: `RegexValidator`, the color picker).

## Why this exists

The editor's keybindings are TV's native **WordStar / diamond** set (`Ctrl-S/D/E/X`
move, `Ctrl-K`/`Ctrl-Q` two-key prefixes, `Ctrl-G` delete, …). To a modern user
several of those collide hard with universal expectations (`Ctrl-C` = page-down,
`Ctrl-V` = insert-toggle, `Ctrl-X` = line-down, `Ctrl-S` = cursor-left,
`Ctrl-Y` = delete-line). Two concrete asks triggered this work:

1. **Plain Backspace does nothing in the editor** — a genuine faithfulness *bug*
   (root cause below), not a preference.
2. The port is modern; the user wants **fully configurable** keybindings with
   recognizable presets, applied **globally to all text input** — the multi-line
   `Editor` *and* the single-line `InputLine` (so every dialog field, Find/Replace,
   file-dialog, etc. follows the same scheme).

There is **no single industry standard** for keybindings. There are three live
families, and "modern" means different things by context:

| Family | What it is | Where it's the default |
|---|---|---|
| **WordStar / diamond** | `Ctrl-S/D/E/X` move, `Ctrl-K` block, `Ctrl-Q` quick, `Ctrl-G` delete | **What Turbo Vision ships** |
| **CUA / "Office"** | `Ctrl-C/X/V` copy/cut/paste, `Ctrl-Z/Y` undo/redo, `Ctrl-A` select-all, `Ctrl-F` find | **VS Code (Win/Linux default)**, GUI editors, Windows |
| **Emacs** | `Ctrl-A/E` line ends, `Ctrl-K` kill, `Ctrl-D` del-fwd, `Ctrl-F/B/N/P` move | **readline/bash, all CLIs, macOS text fields** |

The mature answer most editors land on is: ship a sensible default, make the
keymap **data-driven** so it can be swapped/rebound. We follow **VS Code's
`keybindings.json` model** for the *shape* of a binding because it fits our
existing machinery almost exactly (see §2).

## The Backspace bug (root cause)

In the C++ `scanKeyMap` (`teditor1.cpp`), a binding matches when
`mapLow == codeLow && (mapHi == 0 || mapHi == codeHi)`. The `firstKeys` entry
`kbCtrlH, cmBackSpace` has `kbCtrlH = 0x0008` → **`mapHi == 0`**, so it matches on
the low byte (`0x08`) alone. Plain Backspace `kbBack = 0x0e08` *also* has low byte
`0x08`, so in real Turbo Vision plain Backspace resolves to `cmBackSpace` via the
Ctrl-H entry.

Our Rust port turned that data table into `match` arms and (correctly) split
`Ctrl-H` (`Key::Char('h') + ctrl`) from `Key::Backspace` into distinct variants —
but only wired the **Ctrl/Shift** Backspace arms (→ `DEL_WORD_LEFT`). Plain
`Backspace` falls through to `None` and does nothing
(`src/widgets/editor.rs`, `scan_key_map`, ~line 2299). The fix: the WordStar preset
binds plain `Backspace → BACK_SPACE` explicitly. (`InputLine` is unaffected — its
plain Backspace already works, `input_line.rs:747`.)

## Design

### 1. One shared keymap primitive — `src/keymap.rs`

A **top-level framework module** (not under `editor/`), because two widget families
consume it.

```rust
pub struct KeyStroke { key: Key, ctrl: bool, alt: bool, shift: bool }  // one normalized keystroke
pub struct Chord(Vec<KeyStroke>);     // 1 stroke; 2 strokes for Ctrl-K / Ctrl-Q style prefixes
pub struct Keymap { /* ordered (Chord -> Command) bindings + the set of prefix strokes */ }
```

- **Resolution** reuses the editor's existing result enum
  `KeyMapResult { Command(Command), Prefix, None }`: combine the incoming
  `KeyStroke` with any pending prefix stroke; a full-chord match → `Command`, a
  stroke that begins a known 2-stroke chord → `Prefix`, otherwise → `None`.
  This **generalizes** the editor's current `key_state: i32` (0/1/2) prefix machine
  into a real chord lookup; `key_state` becomes `pending: Option<KeyStroke>`.
- **Normalization** removes the C++ zero-high-byte wildcard trick. Our `Key` model
  already distinguishes `Char('h')+ctrl` from `Backspace`, so presets bind **both
  explicitly**. Letter strokes normalize to **lowercase `Char` + `shift` flag**, so
  `ctrl+q a` and `ctrl+q A` resolve identically (matching C++'s uppercase-fold of
  the second prefix key).
- **Ordering / first-match:** the C++ tables are order-sensitive in one spot
  (`kbCtrlDel, cmDelWord` precedes the dead `kbCtrlDel, cmClear`). With distinct
  `Key` variants we have no wildcards, so each `(Chord → Command)` is unambiguous;
  the dead `cmClear` binding is simply omitted (documented), preserving behavior.

### 2. Authoring surface — VS Code-style chord strings (pure, no I/O)

VS Code is the standard we follow for the *shape* of a binding, for three reasons
that line up with our codebase:

1. **Chord syntax already matches our engine.** VS Code writes multi-stroke chords
   as `"ctrl+k ctrl+c"`; TV's editor already has a two-key prefix machine. Same
   concept.
2. **Command-by-name already matches `Command`.** `Command(&'static str)`
   (`src/command.rs:43`) is a namespaced string, exactly like VS Code binds a key
   to a command *string*.
3. **`when` contexts** (VS Code's third field) map to our focus/view model **if** we
   ever want context-sensitive bindings — explicitly **out of scope** here, noted
   only so the model can grow into it.

Bindings are authored with an **in-memory** parser (a builder convenience — **not**
a file loader; keeps the door open for one later):

```rust
let mut km = Keymap::word_star();        // start from a preset
km.bind("ctrl+c", Command::COPY);         // override one binding
km.bind("ctrl+k ctrl+c", Command::COPY);  // a chord
km.unbind("ctrl+v");
```

The parser handles `ctrl+ shift+ alt+`, named keys (`backspace delete home end
pageup pagedown left right up down insert enter tab esc fN`), single printable
chars, and space-separated chords — the VS Code subset we need. **No file I/O, no
serde, no runtime config file** in this spec (the chosen config surface is the
app-developer Rust API).

### 3. Global active keymap — the "all input" knob

```rust
tv::keymap::set_global(Keymap::cua());   // process-wide default for all text input
tv::keymap::global()                     // both Editor and InputLine read this
```

A single **process-global default** behind a setter is the simplest match for
"change once → applies to all input," and mirrors rstv's existing process-global
`ViewId` counter pattern. Threading the keymap through `Context` (the faithful DI
route) was considered and **rejected** as heavier with no benefit for a single
global knob. Concretely: a `OnceLock`-backed global initialized to
`Keymap::word_star()`, replaceable via `set_global`.

### 4. Both widgets resolve through it — with a pass-through rule

- **`Editor`** (`src/widgets/editor.rs`): replace the hardcoded `scan_key_map`
  `match` body with a `keymap::global()` lookup; `key_state: i32` → `pending:
  Option<KeyStroke>`. `convert_event` and the **printable-char insertion path**
  (the `[32,255)`/Tab gate, ~lines 2070–2098) are otherwise untouched — the keymap
  resolves **commands only**.
- **`InputLine`** (`src/widgets/input_line.rs`): replace the hardcoded
  `match ke.key` (~lines 725–790) with a `keymap::global()` lookup. **Invariant —
  the repertoire/pass-through rule:** a single-line field acts only on commands in
  its repertoire (`CHAR_LEFT/RIGHT`, `WORD_LEFT/RIGHT`, `LINE_START/END`,
  `BACK_SPACE`, `DEL_CHAR`, `DEL_WORD_LEFT`, `DEL_WORD`, `CUT`/`COPY`/`PASTE`,
  `SELECT_ALL`, `INS_MODE`, and the WordStar clear-line via `Ctrl-Y`). A resolved
  command **outside** that set (`NEW_LINE`, `LINE_UP/DOWN`, `PAGE_UP/DOWN`, block
  ops) is treated as **unhandled and bubbles** — so plain Enter still fires the
  dialog's default button, Tab still moves focus, Esc still cancels. **The printable
  insert path and the existing selection/shift-extend logic are preserved**
  unchanged.
- **Selection stays orthogonal.** Shift-extends-selection is driven by the `shift`
  flag on movement in both widgets' `handle_event` (e.g. `editor.rs:1814`,
  `input_line.rs:710`), **not** by the keymap, so CUA-style Shift+arrow selection
  already works under every preset.

### 5. Three shipped presets

- **`Keymap::word_star()` — the default.** Exact transcription of the editor's
  `firstKeys` / `quickKeys` / `blockKeys` tables **plus** the input-line bindings
  that exist today, so **every existing snapshot stays green**. The **only**
  intended behavior change is plain `Backspace → BACK_SPACE` (the bug fix).
- **`Keymap::cua()`.** `Ctrl-C/X/V` copy/cut/paste, `Ctrl-Z` undo (+ redo if the
  editor exposes one — verify during implementation), `Ctrl-A` select-all,
  `Ctrl-F` find, `Ctrl`-arrows word-move, `Ctrl-Backspace`/`Ctrl-Delete`
  word-delete, arrows + Home/End/PageUp/Dn navigation — uniform across editor and
  input fields.
- **`Keymap::emacs()`.** `Ctrl-A/E` line start/end, `Ctrl-F/B` char, `Ctrl-N/P`
  line, `Ctrl-D` del-forward, `Ctrl-K` kill-to-end, `Alt-F/B` word, `Ctrl-Y`
  paste/yank — and these now work **in input fields too** ("emacs everywhere").

> Preset key choices are the design's most negotiable surface; the table above is
> the starting point, refined during review against the actual `Command` set.

### 6. tvedit — Options ▸ Keyboard mapping submenu

`examples/tvedit.rs` gains a new menu-bar submenu (the example already drives
everything through `MenuBar` + command dispatch; rstv has **no right-click popup
primitive**, so a menu-bar submenu is the faithful, zero-new-primitive choice):

```
~O~ptions
  Keyboard mapping
    • WordStar      (KEYMAP_WORDSTAR)
      CUA           (KEYMAP_CUA)
      Emacs         (KEYMAP_EMACS)
```

Three check-marked radio items (active preset marked). Selecting one dispatches an
example-local command (`KEYMAP_WORDSTAR` / `KEYMAP_CUA` / `KEYMAP_EMACS`), handled
in the example's `run_app` closure (`tvedit.rs:135`) by calling
`tv::keymap::set_global(...)`. Because it sets the **global** keymap, the switch is
immediately visible in the editor **and** in any open input field (Find/Replace,
file dialogs) — the live demo of "global for all input."

## Out of scope (YAGNI)

- File/runtime config (`keybindings.json` loader, serde) — config surface is the
  Rust API only.
- `when`-context-sensitive bindings — the model leaves room; we don't build it.
- A right-click popup `ContextMenu` primitive — separate feature if ever wanted.
- Rebinding keys for non-text widgets (menus, buttons, lists) — those keep their
  current direct event handling; this spec covers **text input** only.

## Testing & verification (D11)

- **Unit** (`src/keymap.rs`): single-stroke resolve, chord prefix→command,
  miss-clears-pending, normalization (`ctrl+q a` == `ctrl+q A`), string parser
  round-trips for `ctrl+ shift+ alt+`, named keys, chords.
- **Editor behavior:** plain `Backspace` deletes a char (regression lock for the
  bug); under CUA `Ctrl-C` copies; under Emacs `Ctrl-A` → line start.
- **InputLine behavior:** under CUA `Ctrl-C` copies and `Ctrl-A` select-all; under
  Emacs `Ctrl-A` → field start, `Ctrl-E` → field end; **under every preset, Enter /
  Tab / Esc still bubble** (default-button / focus / cancel) — the pass-through
  invariant.
- **Snapshots:** all existing editor + input_line + dialog snapshots stay green
  under the default preset. No `View` trait method is added → **no
  `tvision-macros/src/specs.rs` forwarder change**.
- Gates: `cargo test --workspace`, `cargo clippy --workspace --all-targets -D
  warnings`, `cargo fmt --all --check`.

## Build sequence (each step independently verifiable)

1. **Keymap primitive + presets + parser** (`src/keymap.rs`, unit-tested in
   isolation). INFRA — Opus/main-thread design.
2. **Editor adoption** — swap `scan_key_map`/`key_state` to the keymap; lock the
   Backspace regression. Snapshots unchanged.
3. **InputLine adoption** — keymap lookup + repertoire/pass-through rule. **Highest
   risk** (backs every dialog field); guard with the Enter/Tab/Esc bubble tests.
4. **Global setter + tvedit Options menu** — `set_global` plumbing and the live
   selector.

## Methodology note

This is an **INFRA-grade** change to the input core touching two widget families.
Per CLAUDE.md it is driven subagent-style: the main thread owns the seam design and
the few shared-file edits (`lib.rs`/`mod.rs` wiring, re-exports); an implementer
subagent does each step; **two-stage review** (spec-compliance then code-quality,
fresh subagents) before integrating each step. Default-preset faithfulness keeps
the port's "faithful by default, extensions alongside" rule intact.
