# Docs Phase 3 — verified docs (doctest / build gates)

> Goal: make the guide's code self-verifying so it cannot silently drift.
> Scope (confirmed with user 2026-06-13): **FULL** — cheap example gate + src
> doctests on + triage all inline fragments (convert the self-contained ones to
> real **hidden-line doctests**, label the rest illustrative) + a `cargo xtask
> test` (mdbook test) subcommand + turn every gate on in `docs.yml`.

Verify everything on the integrated tree with
`CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target`. Max 4 cores
(`-j2 --test-threads=2`).

## Conversion technique (the rubric every conversion task follows)

mdbook/rustdoc wraps each non-`ignore` ```rust block in an implicit `fn main`
and **compiles + runs** it (unless `no_run`). To convert a `rust,ignore`
fragment into a real, compiling ```rust doctest:

1. **Hidden lines** start with `# ` and are compiled but not displayed.
2. Alias the crate once per block with a hidden `# use tvision as tv;` (the book
   is an *external* consumer, so the crate is `tvision`, not `tv`). Add any other
   needed `# use tvision::…;` hidden lines.
3. **Receiver-as-parameter trick:** when the shown code calls a method on a
   runtime-only object you cannot cheaply construct (a live `Program`, `Context`,
   `Desktop`, a `&mut self` view), wrap the shown code in a hidden, *uncalled*
   function that takes that receiver as a parameter:
   ```
   # use tvision as tv;
   # fn _demo(ctx: &mut tv::Context) {
   ctx.post(REFRESH);          // ← the only line shown to the reader
   # }
   ```
   rustdoc nests `_demo` inside `main` and never calls it, so the body
   **type-checks** (proving the API call is real) without needing a terminal or
   constructing the object. This is the workhorse — prefer it over `no_run`.
4. Hidden `# let bounds = tv::Rect::new(0,0,10,3);`-style bindings for undeclared
   locals the snippet references.
5. Only fall back to `rust,ignore` when the snippet references genuinely
   **private / pump-local / fictional** items (a `Deferred` variant match, the
   pump's local `group`/`captures`, a sketch method like `pump_and_drive`, or a
   type that lives outside the `tvision` crate such as xtask's `Screen`). When you
   keep `ignore`, prepend ONE label line inside the block:
   `// Illustrative sketch — not a standalone program.`

**Display discipline:** the reader must see ONLY the intended teaching lines.
Every scaffolding line is hidden with `# `. After converting, build the book and
read the rendered page — no `#`, no wrapper `fn _demo`, no `use tvision as tv`
may be visible.

**Verify each converted block** by running `cargo xtask test` (Task 1 delivers
it) until green, then `cargo xtask docs` (link + build) stays green.

## Task 1 — `cargo xtask test` subcommand (FOUNDATION; build the gate first)

`xtask/src/test.rs` + wire into `main.rs`. Mirror `build::docs` structure.

- `pub fn run() -> Result<()>`:
  1. `cargo build -p tvision` (lib only) so the rlib + dep rlibs exist in
     `$CARGO_TARGET_DIR/debug/deps`. Resolve the target dir from
     `CARGO_TARGET_DIR` env (fallback to the workspace `target`); compute the
     `debug/deps` path.
  2. `let mut book = MDBook::load(paths::book_root())?;`
     `book.test(vec!["-L", &deps_path])` (mdbook's library API — runs every
     non-`ignore` rust block as a doctest with the rlib on the search path).
     Map the error like `build_book` does.
- `main.rs`: add `Some("test") => test::run(),` arm + a usage line
  `  test            run mdbook doctests (compile the guide's rust blocks)`.
- **Acceptance:** on the *current* tree (all blocks still `,ignore`) `cargo xtask
  test` exits 0 (0 blocks compiled — proves the harness + `-L` path work). Add a
  smoke check if practical. `cargo clippy -p xtask --all-targets -D warnings` +
  `cargo fmt --check` clean.

## Task 2 — turn on the 3 ignored `src/` rustdoc doctests

Convert these three `///`/`//!` ```ignore blocks to compiling doctests (they are
*internal* crate doctests, so `use crate::…;` / the crate's own items work; no
`tv::` alias needed). Hidden-line them so the shown text is unchanged or minimally
changed:

- `src/lib.rs:21` — `let r = tv::Rect::new(0,0,80,25);`. Keep the `tv::` flavour
  via a hidden `# use tvision as tv;` (lib.rs doctests are external-style for the
  crate root). Show the one line.
- `src/backend/headless.rs:35` — the `HeadlessBackend::new` + `Renderer` snippet.
  `insta` is a dev-dep **not available to doctests** → drop/replace the
  `insta::assert_snapshot!` line with `# let _ = screen.snapshot();` (hidden) or a
  shown `let snap = screen.snapshot();`. Hidden `use` lines for the types.
- `src/desktop/background.rs:18` — `Background::new(Rect::new(0,0,80,25), '▒')`.
  Hidden `# use` for `Background`/`Rect`.

**Acceptance:** `cargo test --doc -p tvision` shows these now **pass** (not
ignored); count moves from `4 passed; 3 ignored` toward `7 passed; 0 ignored`.
clippy/fmt clean.

## Task 3 — convert `apps/` fragments

Per the triage table below. Self-verify with `cargo xtask test`.

| File | Block | Action |
|---|---|---|
| commands.md | `Command::custom` const | CONVERT (const + `# use tvision as tv;`) |
| commands.md | `ctx.post(REFRESH)` | CONVERT (`fn _demo(ctx:&mut tv::Context)`; hidden REFRESH const) |
| commands.md | `app.disable_command/enable_command` | CONVERT (`fn _demo(app:&mut tv::Program)`) |
| commands.md | `ctx.broadcast(...)` | CONVERT (`fn _demo(ctx:&mut tv::Context, my_id: tv::ViewId)`) |
| controls.md | `Button::new` | CONVERT (hidden `# let bounds = tv::Rect::new(…);` + `# use` Button/ButtonFlags/Command) |
| dialogs.md | `Dialog::new`+`insert_child`+`InputLine`+`Button` | CONVERT (self-contained; hidden uses) |
| dialogs.md | `match program.exec_view(…)` | CONVERT (`fn _demo(program:&mut tv::Program, dialog: Box<dyn tv::View>)`) |
| keyboard.md | `Keymap::new/bind/unbind` | CONVERT (self-contained) |
| keyboard.md | `set_global(Keymap::cua())` | CONVERT |
| text-editing.md | `EditWindow::new`+`desktop.insert` | CONVERT (`fn _demo(desktop:&mut tv::Desktop)`; hidden bounds/path/num) |
| theming.md | `fn draw` w/ `ctx.style`/`put_str` | CONVERT — adapt to hidden `struct`+`impl` or a free `fn _demo(ctx:&mut tv::DrawCtx)`; if it can't read cleanly, label illustrative |
| theming.md | `Theme::classic_blue`+`set_style`+`set_theme` | CONVERT (`fn _demo(program:&mut tv::Program)`) |
| windows.md | `{{#rustdoc_include hello.rs:setup}}` | **LEAVE** (example-backed; stays `rust,ignore`) |
| windows.md | `desktop_rect`+`Window::new`+`desktop_insert` | CONVERT (`fn _demo(prog:&mut tv::Program)`; hidden next_num) |
| windows.md | `win.state_mut().options.tileable = true` | CONVERT (`fn _demo(win:&mut tv::window::Window)`) |

Find exact public paths (e.g. is it `tv::Window` or `tv::window::Window`,
`tv::widgets::Button` or `tv::Button`) by grepping `src/lib.rs` re-exports. If a
CONVERT row can't be made to compile cleanly within reason, downgrade it to a
labeled illustrative block and note why.

## Task 4 — convert `port/` + `getting-started/installation.md`

| File | Block | Action |
|---|---|---|
| constants.md | `Command::OK/CANCEL/QUIT` lets | CONVERT |
| constants.md | `Command::custom` const | CONVERT |
| deferred.md | `ctx.request_close(self.id())` | CONVERT (`fn _demo(ctx:&mut tv::Context, this:&dyn tv::View)` → `ctx.request_close(this.id())`; if `View::id()` isn't pub, label illustrative) |
| events.md | `match event {…}` | CONVERT (`fn _demo(event: tv::Event)`) |
| flags.md | `if view.state().state.focused` | CONVERT (`fn _demo(view:&dyn tv::View)`) |
| inheritance.md | `fn state/state_mut` pair | **ILLUSTRATIVE** (bare trait-method bodies) — label |
| inheritance.md | `struct AboutDialog { dialog: Dialog }` | CONVERT (struct def; hidden `# use`) |
| inheritance.md | `#[delegate(to=dialog)] impl View …` | CONVERT-attempt (needs the `AboutDialog`+`dialog: Dialog` field & `use tvision::delegate`; the macro fills the rest). If the macro needs more surface, label illustrative |
| theme.md | `ctx.theme.style(…)`+`glyphs().frame_tl` | CONVERT-attempt (`fn _demo(ctx:&tv::DrawCtx)`; only if `ctx.theme` is a pub field — else illustrative) |
| installation.md | `use tv::{Program, Desktop, …}` | CONVERT (hidden `# use tvision as tv;` then the shown `use tv::{…};`; add `# fn main(){}`) |

## Task 5 — `internals/` + `getting-started/skeleton.md` + `reference/` + custom-view

| File | Block | Action |
|---|---|---|
| custom-view.md | `Banner` full `impl View` | CONVERT (complete program; hidden `# fn main(){}`; add any hidden required View methods Banner omits) — high value |
| custom-view.md | `MyTerminal` delegate | CONVERT-attempt (hidden `# struct/use`; macro fills rest) — else illustrative |
| internals/brokering.md | group/find_mut/field_int/downcast | **ILLUSTRATIVE** (pump-local `group`, likely-private `field_int`) — label |
| internals/deferred.md | `match effect { Deferred::… }` | **ILLUSTRATIVE** (`Deferred` variants are `pub(crate)`, pump-local) — label |
| internals/drawing.md | `DrawBuffer::new`+`move_str` | CONVERT-attempt (only if `DrawBuffer`/`move_str` are pub) — else illustrative |
| internals/event-loop.md | `loop { … self.pump_and_drive() }` | **ILLUSTRATIVE** (private fields + sketch method) — label |
| skeleton.md (`Program::new(...)`) | ctor-shape sketch | **ILLUSTRATIVE** (undeclared backend/clock/factories) — label |
| skeleton.md (hello.rs:run) | example-backed | **LEAVE** (`rust,ignore`) |
| reference/screenshots.md | `Screen { … }` literal | **ILLUSTRATIVE** (`Screen` lives in `xtask`, not `tvision`) — label |

## Task 6 — wire gates into `docs.yml` + final integrated verification

Add to `.github/workflows/docs.yml` `build` job, **before** `cargo xtask docs`:
```yaml
      - name: Compile examples (gallery + hello)
        run: cargo build --examples
      - name: Doctests (src)
        run: cargo test --doc -p tvision
      - name: Guide doctests (mdbook)
        run: cargo xtask test
```
(`cargo xtask test` must run after a build so the rlib exists; `cargo build
--examples` upstream covers that, but Task 1's `run()` also builds defensively.)

**Final integrated verification (orchestrator, canonical target dir):**
- `cargo build --examples` ✓
- `cargo test --doc -p tvision` → 0 ignored among the 3 targeted ✓
- `cargo xtask test` → all converted book blocks compile ✓
- `cargo xtask docs` → build + link check clean; **grep built HTML for leftover
  `rustdoc_include`** (`grep -rc rustdoc_include docs/book/book` ⇒ 0) ✓
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -D warnings`,
  `cargo fmt --all --check` ✓
- Update `docs/IMPLEMENTATION-LOG.md` (new section, newest-first) + `docs/HANDOVER.md`
  (Phase 3 landed; next steps from the "Smaller follow-ups" list).
