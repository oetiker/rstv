# Data-Movement Phase 1 — `exec_view_with<R>` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a generic by-value modal-result mechanism (`exec_view_with<R>`) and migrate the two `Program`-launched dialogs (`color_dialog`, `theme_editor`) onto it, deleting the `Rc<Cell>`/`Rc<RefCell>` sinks and the `ColorPick`/`ThemeEdit` `ModalCompletion` variants.

**Architecture:** Extract the existing `exec_view_with_completion` body into one private generic core `exec_view_capture<R>` that threads a caller `extract: FnOnce(&mut dyn View, Command) -> R` closure, invoked at the **pre-drop window** (the modal is still in the tree by `id`, right after the existing completion+gather reads, before `Group::remove`). `exec_view_with_completion` becomes a thin wrapper passing a no-op `|_, _| ()` extract; a new public `exec_view_with<R>` wraps the core with no completion/gather. The two dialogs call the core directly (they need `initial_focus`) and return their result by value.

**Tech Stack:** Rust (`tvision-rs` workspace crate), `insta` snapshot tests, mdBook guide (`docs/book/`), `cargo xtask test`/`docs`.

**Spec:** `docs/superpowers/specs/2026-06-18-unified-data-movement-design.md` (§3.3 "Modal results", §5 "Phase 1", §9 docs). This is the **proof-of-value** phase: a single-view result with a sound borrow, the highest reduction-per-risk.

## Global Constraints

- **Build env:** `export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target` before every cargo command. Artifacts land there, not `./target`.
- **Parallelism cap:** never exceed 4 cores — use `-j2` and `-- --test-threads=2` on every cargo invocation.
- **Faithful to C++**; English identifiers and comments. User-facing strings may be localized; nothing here is.
- **Commit trailer:** every commit message ends with `Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>`.
- **Behavior-preserving phase:** Phases 1–4 are refactors. No snapshot may change and no existing public test may change its asserted behavior. The only test edited here is the one that constructed a now-deleted enum variant.
- **No new public item without rustdoc**, following the house convention: rustdoc is user-facing (strip porting bookkeeping); quarantine C++ lineage into a `# Turbo Vision heritage` section.
- **`Color` is NOT a `FieldValue`** (spec C-1): `Color` is a 4-variant enum (`src/color.rs:32`), and `FieldValue::Color` is an explicit non-goal. The color/theme results ride this by-value path, not `FieldValue`.
- **Acceptable residual downcast (spec §6.6):** the `extract` closures in `color_dialog`/`theme_editor` reach their *own* known child (`ColorPicker`/`ThemeEditorBody`) via `find_mut` + `as_any_mut` + `downcast_mut`. That is the "a parent reaches a known child" category — local to the helper, **not** framework-pump data-movement. The win is deleting the `ModalCompletion` variants + `Rc` sinks + `apply_modal_completion` arms from the generic pump.

---

### Task 1: Generic `exec_view_capture<R>` core + public `exec_view_with<R>`

**Files:**
- Modify: `src/app/program.rs` — the `exec_view_with_completion` fn (currently `fn exec_view_with_completion(&mut self, …) -> (Command, Option<crate::data::FieldValue>)`, starts at the `fn exec_view_with_completion(` line). Add the generic core, the public method, and convert the old fn to a wrapper.
- Test: `src/app/program.rs` `#[cfg(test)]` module (add one new test near the other `exec_view`/modal tests).

**Interfaces:**
- Consumes: existing `Group::insert`/`insert_with_id`/`find_mut`/`remove`/`set_current`, `Context::new`, `ModalFrame`, `validate_modal_close`, `pump_and_drive`, `apply_modal_completion` (all already in scope in this fn). `View::find_mut(&mut self, ViewId) -> Option<&mut dyn View>`, `View::as_any_mut(&mut self) -> Option<&mut dyn Any>` (both trait methods, already used in `apply_modal_completion`). `ViewId` is `Copy` (`src/view/id.rs:55`). `Command` is `Copy`.
- Produces:
  - `fn exec_view_capture<R>(&mut self, view: Box<dyn View>, completion: Option<ModalCompletion>, initial_focus: Option<ViewId>, gather: Option<ViewId>, gather_self: bool, extract: impl FnOnce(&mut dyn View, Command) -> R) -> (Command, Option<crate::data::FieldValue>, R)` — private to `impl Program`.
  - `pub fn exec_view_with<R>(&mut self, view: Box<dyn View>, extract: impl FnOnce(&mut dyn View, Command) -> R) -> R` — new public API.
  - `fn exec_view_with_completion(...) -> (Command, Option<crate::data::FieldValue>)` — unchanged signature, now a wrapper. All its existing callers are unaffected.

- [ ] **Step 1: Write the failing test**

Add to the `program.rs` test module (place it in the same `mod` as the existing `color_dialog`/modal tests; use the existing `program_with_desktop` helper and `Event`/`Command`/`Dialog` imports the module already has — add a local `use crate::dialog::Dialog;` inside the test if not in scope):

```rust
/// `exec_view_with` returns the closure's value BY VALUE, and the closure sees
/// the modal's end command. OK → the extracted value; Cancel → the cancel value.
/// Proves the by-value channel (no Rc sink, no dyn Any in the framework).
#[test]
fn exec_view_with_returns_extract_value_by_command() {
    use crate::dialog::Dialog;
    let (mut program, _handle, _clock) = program_with_desktop(80, 30);

    // OK path.
    let d_ok = Dialog::new(crate::view::Rect::new(0, 0, 20, 6), Some("t".to_string()));
    program.out_events.push_back(Event::Command(Command::OK));
    let ok: &str = program.exec_view_with(Box::new(d_ok), |_modal, cmd| {
        if cmd == Command::OK { "ok" } else { "other" }
    });
    assert_eq!(ok, "ok", "extract must see cmOK and its return is handed back by value");
    assert_eq!(program.capture_len(), 0, "ModalFrame popped on close");

    // Cancel path.
    let d_cancel = Dialog::new(crate::view::Rect::new(0, 0, 20, 6), Some("t".to_string()));
    program.out_events.push_back(Event::Command(Command::CANCEL));
    let cancelled: &str = program.exec_view_with(Box::new(d_cancel), |_modal, cmd| {
        if cmd == Command::OK { "ok" } else { "other" }
    });
    assert_eq!(cancelled, "other", "extract must see cmCancel");
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
cargo test -p tvision-rs --lib -j2 -- --test-threads=2 exec_view_with_returns_extract_value_by_command
```
Expected: FAIL to **compile** — `exec_view_with` does not exist yet (`no method named exec_view_with`).

- [ ] **Step 3: Add the generic core (move the body verbatim, make 3 changes)**

Rename the existing `fn exec_view_with_completion` to `fn exec_view_capture<R>`, with these exact changes and **nothing else** in the body:

1. New signature (add the `<R>` param, the `extract` param, and `R` in the return tuple):

```rust
    fn exec_view_capture<R>(
        &mut self,
        view: Box<dyn View>,
        completion: Option<ModalCompletion>,
        initial_focus: Option<ViewId>,
        gather: Option<ViewId>,
        gather_self: bool,
        extract: impl FnOnce(&mut dyn View, Command) -> R,
    ) -> (Command, Option<crate::data::FieldValue>, R) {
```

2. Invoke `extract` at the pre-drop window. Find the existing `gathered` block (the `let gathered = if retval != Command::CANCEL { … } else { None };` that ends just before the `// 8. Pop the frame` comment) and insert, immediately after it and before the `self.captures.pop();` line:

```rust
        // Extract the caller's typed result while the modal is STILL in the tree
        // by `id` — the same pre-drop window as the completion + gather above.
        // The modal is guaranteed present: inserted in step 2, removed only below.
        let extracted = {
            let modal = self
                .group
                .find_mut(id)
                .expect("modal is in the tree by id until remove() below");
            extract(modal, retval)
        };
```

3. Change the final return expression `(retval, gathered)` to `(retval, gathered, extracted)`.

- [ ] **Step 4: Add the wrapper + the public method**

Immediately above the new `exec_view_capture`, add the behavior-preserving wrapper (identical signature to the old fn):

```rust
    /// Execute `view` as a modal, applying an optional completion and gathering
    /// an optional field — the no-result-extraction entry point. See
    /// [`exec_view_capture`](Self::exec_view_capture) for the generic core.
    fn exec_view_with_completion(
        &mut self,
        view: Box<dyn View>,
        completion: Option<ModalCompletion>,
        initial_focus: Option<ViewId>,
        gather: Option<ViewId>,
        gather_self: bool,
    ) -> (Command, Option<crate::data::FieldValue>) {
        let (cmd, gathered, ()) =
            self.exec_view_capture(view, completion, initial_focus, gather, gather_self, |_, _| ());
        (cmd, gathered)
    }
```

And add the public method (place it near `exec_view`, the other public modal entry point):

```rust
    /// Execute `view` as a modal and return a caller-typed result extracted from
    /// the finished modal **by value** — no shared `Rc` cell and no `dyn Any` in
    /// the framework. `extract` runs once, at modal close, receiving the modal's
    /// own `&mut dyn View` and the end [`Command`] while the view is still in the
    /// tree; whatever it returns is handed straight back to the caller.
    ///
    /// `R` is named by the caller and never by the framework. A consumer that
    /// needs a single field can read it through [`View::value`]; a consumer that
    /// needs a richer native value (a [`Color`](crate::color::Color), a whole
    /// [`Theme`](crate::theme::Theme)) returns it directly from `extract`.
    ///
    /// # Turbo Vision heritage
    /// The value-returning twin of `TGroup::execView` (`tgroup.cpp:188`), which
    /// returns a `ushort` end command to its method caller. Where C++ then reads
    /// results out of the still-live dialog with `getData`, `extract` reads them
    /// by value here.
    pub fn exec_view_with<R>(
        &mut self,
        view: Box<dyn View>,
        extract: impl FnOnce(&mut dyn View, Command) -> R,
    ) -> R {
        self.exec_view_capture(view, None, None, None, false, extract).2
    }
```

- [ ] **Step 5: Run the new test + the refactor guard suite**

```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
cargo test -p tvision-rs --lib -j2 -- --test-threads=2 exec_view_with_returns_extract_value_by_command
```
Expected: PASS.

Then confirm the refactor preserved all existing modal behavior (these exercise the wrapper path through the new core):
```bash
cargo test -p tvision-rs --lib -j2 -- --test-threads=2 color_dialog theme_editor exec_view input_box message_box
```
Expected: all PASS (no behavior change).

- [ ] **Step 6: Commit**

```bash
git add src/app/program.rs
git commit -m "feat(program): exec_view_with<R> — by-value modal results via a generic core

Extract exec_view_with_completion's body into a private generic
exec_view_capture<R> that threads a caller extract closure, invoked at the
pre-drop window (modal still in the tree by id). exec_view_with_completion
becomes a thin wrapper with a no-op extract; add the public exec_view_with<R>.
No behavior change; the new mechanism is the by-value successor to the Rc-sink
ModalCompletion pattern (TGroup::execView returning to a method caller).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: Migrate `color_dialog`; delete `ModalCompletion::ColorPick`

**Files:**
- Modify: `src/app/program.rs` — `color_dialog` (the `pub fn color_dialog(&mut self, initial: crate::color::Color) -> Option<crate::color::Color>` body), the `ModalCompletion::ColorPick { … }` enum variant (with its doc comment), the `ModalCompletion::ColorPick { picker, sink } => { … }` arm in `apply_modal_completion`, and the doc comment of `color_dialog_ok_returns_initial_color` that mentions the sink.

**Interfaces:**
- Consumes: `exec_view_capture` (Task 1); `ColorPicker::color(&self) -> Color` (existing, used in the deleted arm); `View::find_mut`/`as_any_mut`.
- Produces: no signature change — `color_dialog` keeps `(&mut self, Color) -> Option<Color>`.

- [ ] **Step 1: Run the existing color_dialog tests to confirm the baseline is green**

```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
cargo test -p tvision-rs --lib -j2 -- --test-threads=2 color_dialog
```
Expected: `color_dialog_ok_returns_initial_color`, `color_dialog_cancel_returns_none`, `color_dialog_esc_returns_none` all PASS. These call the public `color_dialog` and assert its return — they must remain PASS unchanged after the migration.

- [ ] **Step 2: Rewrite `color_dialog` to use the by-value core**

Replace the tail of `color_dialog` — the three lines:

```rust
        let sink = std::rc::Rc::new(std::cell::Cell::new(None));
        let completion = ModalCompletion::ColorPick {
            picker: picker_id,
            sink: sink.clone(),
        };
        self.exec_view_with_completion(Box::new(d), Some(completion), Some(picker_id), None, false);
        sink.get()
```

with:

```rust
        // Read the chosen color out of the modal's own ColorPicker child by value
        // at close (spec §6.6: a helper reaching its own known child; Color is not
        // a FieldValue, spec C-1). No Rc sink, no ModalCompletion variant.
        self.exec_view_capture(
            Box::new(d),
            None,
            Some(picker_id),
            None,
            false,
            |modal, cmd| {
                if cmd == Command::OK {
                    modal
                        .find_mut(picker_id)
                        .and_then(|v| v.as_any_mut())
                        .and_then(|a| a.downcast_mut::<ColorPicker>())
                        .map(|p| p.color())
                } else {
                    None
                }
            },
        )
        .2
```

(`ColorPicker` and `Dialog` are already in scope via the `use crate::dialog::{ColorPicker, Dialog};` at the top of `color_dialog`. `picker_id` is `Copy` and is captured into the closure.)

- [ ] **Step 3: Update the `color_dialog` rustdoc**

Replace the doc paragraph that currently reads (the lines beginning `/// An tvision-rs-original extension. The result is read by downcasting …` through `/// No \`FieldValue::Color\` (spec non-goal).`) with:

```rust
    /// An tvision-rs-original extension. The chosen color is read out of the
    /// modal's own [`ColorPicker`](crate::dialog::ColorPicker) and returned **by
    /// value** via [`exec_view_with`](Self::exec_view_with)-style capture — no
    /// shared sink. `Color` is deliberately not a `FieldValue` (a 4-variant enum,
    /// not a packable scalar; spec non-goal).
```

- [ ] **Step 4: Delete the `ColorPick` enum variant and its `apply_modal_completion` arm**

Delete the whole `ColorPick { picker: ViewId, sink: std::rc::Rc<std::cell::Cell<Option<crate::color::Color>>> }` variant from `enum ModalCompletion` **including its doc comment** (the `/// … in-tree modal \`ColorPicker\` … forbids \`FieldValue::Color\`.` block directly above it).

Delete the arm in `apply_modal_completion`:

```rust
        // color_dialog result extraction: downcast the in-tree modal ColorPicker,
        // read color(), and write into the caller's sink on cmOK.
        ModalCompletion::ColorPick { picker, sink } => {
            if result == Command::OK {
                let c = group
                    .find_mut(picker)
                    .and_then(|v| v.as_any_mut())
                    .and_then(|a| a.downcast_mut::<crate::dialog::ColorPicker>())
                    .map(|p| p.color());
                sink.set(c);
            }
            None
        }
```

- [ ] **Step 5: Fix the stale test doc comment**

In `color_dialog_ok_returns_initial_color`, change the doc line that mentions the sink:

```rust
        /// OK returns `Some(color)` — the initial color is returned unchanged when
        /// nothing is edited (the ModalCompletion::ColorPick sink is written on cmOK).
```
to:
```rust
        /// OK returns `Some(color)` — the initial color is returned unchanged when
        /// nothing is edited (exec_view_with extracts the picker's color on cmOK).
```

- [ ] **Step 6: Build + run the color_dialog tests + clippy**

```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
cargo test -p tvision-rs --lib -j2 -- --test-threads=2 color_dialog
cargo clippy -p tvision-rs --all-targets -j2 -- -D warnings
```
Expected: the three `color_dialog_*` tests PASS unchanged; clippy clean (in particular, no `unused_variables`/`dead_code` from a half-deleted variant — confirms `ColorPick` has no remaining references).

- [ ] **Step 7: Commit**

```bash
git add src/app/program.rs
git commit -m "refactor(program): color_dialog returns Color by value; delete ModalCompletion::ColorPick

color_dialog reads its own ColorPicker child at modal close via exec_view_capture
and returns Option<Color> directly, deleting the Rc<Cell> sink, the ColorPick
ModalCompletion variant, and its apply_modal_completion arm. Behavior unchanged
(the three color_dialog_* tests pass as-is). Color stays a by-value result, not a
FieldValue (spec C-1).

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: Migrate `theme_editor`; delete `ModalCompletion::ThemeEdit`

**Files:**
- Modify: `src/app/program.rs` — `theme_editor` (the `pub fn theme_editor(&mut self)` body), the `ModalCompletion::ThemeEdit { … }` enum variant (with its doc comment), the `ModalCompletion::ThemeEdit { editor_id, sink } => { … }` arm in `apply_modal_completion`, and the test `theme_editor_ok_installs_new_theme` (which constructs the deleted variant).

**Interfaces:**
- Consumes: `exec_view_with` + `exec_view_capture` (Task 1); `ThemeEditorBody::working_theme(&self) -> &Theme` and `Dialog::insert_child` (existing); `View::find_mut`/`as_any_mut`.
- Produces: no signature change — `theme_editor` stays `(&mut self)`.
- **Do NOT touch** `ModalCompletion::ThemeColorPick` or its `apply_modal_completion` arm — that is the per-role color sub-modal (cluster D, deferred to a later phase). It stays.

- [ ] **Step 1: Run the existing theme_editor tests to confirm the baseline**

```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
cargo test -p tvision-rs --lib -j2 -- --test-threads=2 theme_editor
```
Expected: `theme_editor_cancel_leaves_theme_unchanged`, `theme_editor_ok_installs_equal_theme`, `theme_editor_ok_installs_new_theme` all PASS (baseline). The first two stay PASS unchanged; the third is rewritten in Step 4.

- [ ] **Step 2: Rewrite `theme_editor` to use the by-value core**

Replace the tail of `theme_editor` — the lines:

```rust
        let sink = std::rc::Rc::new(std::cell::RefCell::new(None::<crate::theme::Theme>));
        let completion = ModalCompletion::ThemeEdit {
            editor_id: te_id,
            sink: sink.clone(),
        };
        self.exec_view_with_completion(Box::new(d), Some(completion), Some(te_id), None, false);
        if let Some(new_theme) = sink.borrow_mut().take() {
            self.set_theme(new_theme);
        }
```

with:

```rust
        // Read the edited working theme out of the modal's own ThemeEditorBody by
        // value at close (spec §6.6: a helper reaching its own known child; a whole
        // Theme is too large to be a FieldValue, spec C-1). No Rc sink, no variant.
        let new_theme = self
            .exec_view_capture(
                Box::new(d),
                None,
                Some(te_id),
                None,
                false,
                |modal, cmd| {
                    if cmd == Command::OK {
                        modal
                            .find_mut(te_id)
                            .and_then(|v| v.as_any_mut())
                            .and_then(|a| a.downcast_mut::<ThemeEditorBody>())
                            .map(|te| te.working_theme().clone())
                    } else {
                        None
                    }
                },
            )
            .2;
        if let Some(new_theme) = new_theme {
            self.set_theme(new_theme);
        }
```

(`ThemeEditorBody` and `Dialog` are already in scope via the `use crate::dialog::{Dialog, ThemeEditorBody};` at the top of `theme_editor`. `te_id` is `Copy`.)

- [ ] **Step 3: Delete the `ThemeEdit` enum variant and its `apply_modal_completion` arm**

Delete the `ThemeEdit { editor_id: ViewId, sink: std::rc::Rc<std::cell::RefCell<Option<crate::theme::Theme>>> }` variant from `enum ModalCompletion` **including its doc comment** (the `/// … read the \`ThemeEditorBody\`'s working theme and write it into the sink …` block above it).

Delete the arm in `apply_modal_completion`:

```rust
        // theme editor dialog result — read the ThemeEditorBody's working
        // theme and write into the sink on OK.
        ModalCompletion::ThemeEdit { editor_id, sink } => {
            if result == Command::OK {
                let theme = group
                    .find_mut(editor_id)
                    .and_then(|v| v.as_any_mut())
                    .and_then(|a| a.downcast_mut::<crate::dialog::ThemeEditorBody>())
                    .map(|te| te.working_theme().clone());
                *sink.borrow_mut() = theme;
            }
            None
        }
```

- [ ] **Step 4: Rewrite `theme_editor_ok_installs_new_theme` onto the by-value path**

Replace the entire body of `theme_editor_ok_installs_new_theme` (it currently builds a `ModalCompletion::ThemeEdit` and calls `apply_modal_completion` directly — that variant no longer exists) with a test that drives the real public `exec_view_with` mechanism against a theme-editor-shaped modal whose `ThemeEditorBody` already holds the modified working theme:

```rust
        /// OK extracts the ThemeEditorBody's modified working theme BY VALUE — the
        /// path Program::theme_editor uses (exec_view_with), replacing the deleted
        /// ModalCompletion::ThemeEdit sink.
        #[test]
        fn theme_editor_ok_installs_new_theme() {
            use crate::color::{Color, Style};
            use crate::dialog::{Dialog, ThemeEditorBody};
            use crate::theme::{Role, Theme};

            let original = Theme::classic_blue();
            let mut modified = original.clone();
            modified.set_style(
                Role::Background,
                Style::new(Color::Bios(0xF), Color::Bios(0x0)),
            );
            assert_ne!(modified, original, "test setup: modified theme must differ");

            let (mut program, _handle, _clock) = program_with_desktop(80, 30);

            // A theme-editor-shaped modal pre-seeded with the modified working theme.
            let mut d = Dialog::new(
                crate::view::Rect::new(0, 0, 64, 24),
                Some("Theme Editor".to_string()),
            );
            let te_id = d.insert_child(Box::new(ThemeEditorBody::new(
                crate::view::Rect::new(1, 1, 63, 19),
                modified.clone(),
            )));

            program.out_events.push_back(Event::Command(Command::OK));
            let extracted = program.exec_view_with(Box::new(d), |modal, cmd| {
                if cmd == Command::OK {
                    modal
                        .find_mut(te_id)
                        .and_then(|v| v.as_any_mut())
                        .and_then(|a| a.downcast_mut::<ThemeEditorBody>())
                        .map(|te| te.working_theme().clone())
                } else {
                    None
                }
            });
            assert_eq!(
                extracted,
                Some(modified),
                "OK must extract the modified working theme by value"
            );
        }
```

- [ ] **Step 5: Build + run theme_editor tests + clippy**

```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
cargo test -p tvision-rs --lib -j2 -- --test-threads=2 theme_editor
cargo clippy -p tvision-rs --all-targets -j2 -- -D warnings
```
Expected: all three `theme_editor_*` tests PASS (first two unchanged, third rewritten); clippy clean (no `dead_code` for a half-removed `ThemeEdit` — confirms no remaining references). The untouched `ThemeColorPick` arm still compiles.

- [ ] **Step 6: Commit**

```bash
git add src/app/program.rs
git commit -m "refactor(program): theme_editor returns Theme by value; delete ModalCompletion::ThemeEdit

theme_editor reads its own ThemeEditorBody's working theme at modal close via
exec_view_capture and installs it directly, deleting the Rc<RefCell> sink, the
ThemeEdit ModalCompletion variant, and its apply_modal_completion arm. The
manual-construction test is rewritten onto the public exec_view_with path.
ThemeColorPick (the per-role color sub-modal, cluster D) is untouched.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 4: Documentation (lands WITH the phase, per spec §9)

**Files:**
- Modify: `docs/book/src/port/modal.md` — extend with an `exec_view_with` subsection under the existing `## exec_view steers the one loop` section.
- Verify rustdoc: the `exec_view_with` rustdoc added in Task 1 and the `color_dialog`/`theme_editor` rustdoc edits in Tasks 2–3 already satisfy the per-item rustdoc requirement; this task only adds the conceptual guide prose and runs the doc gates.

**Interfaces:** none (documentation only).

- [ ] **Step 1: Add the guide subsection**

Append to `docs/book/src/port/modal.md`, after the `## exec_view steers the one loop` section, this subsection (a fenced `rust` block needs the hidden `# use tvision_rs as tv;` preamble so `cargo xtask test` can compile it; if `exec_view_with` is not callable without a live `Program`, mark the block `rust,ignore` and prefer linking the rustdoc instead):

```markdown
## Getting a result back: `exec_view_with`

C++ `execView` returns a `ushort` end command; the caller then reads results out
of the still-live dialog with `getData` before it is destroyed. tvision-rs keeps
that shape with
[`Program::exec_view_with`](../api/tvision-rs/app/struct.Program.html#method.exec_view_with):
it runs the modal, then — at the **pre-drop window**, while the view is still in
the tree — hands your `extract` closure the modal's `&mut dyn View` and the end
command. Whatever the closure returns is handed straight back, **by value**:

```rust,ignore
let chosen: Option<Color> = program.exec_view_with(Box::new(dialog), |modal, cmd| {
    (cmd == Command::OK)
        .then(|| read_the_color_out_of(modal))
        .flatten()
});
```

There is no shared `Rc<Cell>` sink and no `dyn Any` in the framework: the result
type `R` is named by the caller, never by the framework. This is the by-value
successor to the old per-dialog `ModalCompletion` "sink" variants. A single field
crosses as a [`FieldValue`](../api/tvision-rs/data/enum.FieldValue.html) via
`View::value`; a richer native value (a `Color`, a whole `Theme`) is returned
directly from `extract` — `Color`/`Theme` are deliberately not `FieldValue`s.
```

- [ ] **Step 2: Run the doc gates**

```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
cargo xtask test    # guide rust blocks compile
cargo xtask docs    # regenerate + build + link-check the integrated site
```
Expected: both succeed. If `cargo xtask docs` reports broken intra-doc links to `struct.Program.html#method.exec_view_with`, confirm the method is `pub` (Task 1) and re-run.

- [ ] **Step 3: Commit**

```bash
git add docs/book/src/port/modal.md
git commit -m "docs(guide): document exec_view_with — by-value modal results

Extend port/modal.md with the exec_view_with subsection (pre-drop extract
window, by-value R, no Rc sink / no dyn Any), per the data-movement spec's
docs-per-phase rule.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

## Final verification (whole-phase, before the broad review)

```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
cargo test --workspace -j2 -- --test-threads=2
cargo clippy --workspace --all-targets -j2 -- -D warnings
cargo fmt --all --check    # if it flags files you did not touch, only ensure src/app/program.rs + the guide are formatted
```
Expected: green. No snapshot changed (Phase 1 is behavior-preserving). `grep -rn "ColorPick\|ThemeEdit" src/app/program.rs` should show **only** `ThemeColorPick` (the untouched cluster-D variant) and `FindPick`/`ReplacePick` — no bare `ColorPick`/`ThemeEdit`.

## Self-Review notes (author)

- **Spec coverage (Phase 1):** `exec_view_with<R>` added (Task 1) ✓; closure threaded into the core and invoked pre-drop (Task 1, Step 3) ✓; `color_dialog`/`theme_editor` migrated (Tasks 2–3) ✓; `ColorPick`/`ThemeEdit` variants + both `Rc` sinks deleted (Tasks 2–3) ✓; docs land with the phase (Task 4) ✓; `Color` stays by-value not `FieldValue` (C-1, Global Constraints + Tasks 2–3) ✓.
- **Type consistency:** `exec_view_capture<R>` returns `(Command, Option<FieldValue>, R)` and is `.2`-projected identically in the public wrapper, `color_dialog`, and `theme_editor`; the wrapper destructures `(cmd, gathered, ())`. `extract: impl FnOnce(&mut dyn View, Command) -> R` is identical at every call site.
- **Out of scope (recorded):** `ThemeColorPick` (cluster D) is explicitly untouched here — it is a per-role sub-modal whose by-value/`FieldValue` treatment is a later phase. The residual `find_mut`+`downcast_mut` inside the two `extract` closures is the spec §6.6 "parent reaches its own known child" category, kept and documented, not framework-pump data-movement.
