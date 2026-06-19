# Data-Movement Phase 4 — modal-result reads via `FieldValue` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Retire the cluster-D *modal-result* framework-internal downcasts in the Find/Replace dialog completions — the `CheckBoxes` option-read downcast and the `Editor` search-state-write downcast — by reading each dialog field through the existing `View::value()` trait and delivering the assembled result to the editor through one new defaulted `View::set_modal_data(FieldValue)` trait method (virtual dispatch).

**Architecture:** Cluster D (deliver a modal's result to its launcher *view*) keeps its per-consumer `ModalCompletion` variants and its by-id *routing* (which editor to write); only the *reads* and the *delivery* go downcast-free. The Find/Replace completions read each dialog child via `View::value()` (`InputLine`→`Text`, `CheckBoxes`→`Bits`, the latter from Phase 2) and assemble the editor's search record as a single ordered `FieldValue::List`, then deliver it via `editor.set_modal_data(record)` — a new defaulted `View` method the `Editor` overrides to unpack the list into `find_str`/`replace_str`/`editor_flags`. `ThemeColorPick` is the recorded exception (its payload is a `Color`, not a `FieldValue`, §3.1) and stays downcasting, reason recorded. The dialog-*open* pre-fill reads (the pump reading editor search state to seed a freshly-built dialog) are a different category — "build UI from a known widget's state," the §4.4 kept structural read — and stay, reason recorded.

**Tech Stack:** Rust (workspace `tvision-rs` + `tvision-rs-macros`); the `#[delegate(to = field)]` proc-macro; `insta` snapshot tests; the single-event-loop pump + `apply_modal_completion` in `src/app/program.rs`.

## Global Constraints

- **Spec authority:** `docs/superpowers/specs/2026-06-18-unified-data-movement-design.md` — read §3.3 (modal results: `FieldValue` for views, the `set_modal_data(FieldValue)` sibling, the `ThemeColorPick` recorded exception), §2.1 (the apply-with-judgment guard, *reason recorded*), §4.4 (honest scope — non-data structural reads stay), and §5 Phase 4.
- **DELIBERATE DEVIATION FROM THE SPEC'S LITERAL WORDING (recorded, §2.1):** the spec sketches "`gather_data` the modal into an ordered `List`." This plan instead reads each dialog field through `View::value()` by id (which the same sentence explicitly permits — "a field via `value()`") and assembles the `List` in the completion. Rationale: `gather_list` is an **inherent** method on `Dialog`/`Group`, *unreachable through `&mut dyn View`* without a `downcast::<Dialog>` — which would trade one framework-internal downcast for another. Making it trait-reachable would require adding `Dialog::value()`→`gather_list()` (a broader semantic change — every `Dialog`'s `value()` becomes a non-`None` gathered record, interacting with `Group::gather_data` if a `Dialog` is ever nested) **and** would couple the editor's positional parse to the dialog's child-insertion order (a silent break on reorder). The per-field `value()` read removes the identical set of downcasts, is surgical and robust to reordering, and keeps this a low-risk behavior-preserving phase. The delivery still uses a single ordered `FieldValue::List` via `set_modal_data`, so "the editor consumes an ordered `List`" (the spec's core intent) is honored — only the *read* mechanism differs. This deviation is approved by the orchestrator and recorded here + in the IMPLEMENTATION-LOG.
- **Behavior-preserving.** Every task is a refactor: no user-visible behavior changes. The safety net is the **existing test suite staying green**, one **new** focused unit test for `Editor::set_modal_data`, plus per-task grep-proofs that each downcast is gone. Snapshots must be **byte-identical** (do NOT accept `insta` changes — a changed snapshot means a real regression).
- **No *framework-internal* `dyn Any`.** After this phase, the Find/Replace completions must not `downcast_mut::<CheckBoxes>()` or `downcast_mut::<Editor>()`. (The deliberate typed-at-the-edges `FieldValue::Custom` escape is unrelated and out of scope. `ThemeColorPick`'s `ColorPicker`/`ThemeEditorBody` downcasts stay — recorded exception, Task 5.)
- **Each new `View` trait method needs BOTH:** (1) a forwarder entry in `tvision-rs-macros/src/specs.rs`, and (2) an entry in the `tests/delegate_view.rs` spy test (a spy impl that `mark`s, a call in the call-list, and the name in the asserted name list + the method-count comment bump). The spy test does NOT auto-catch a forgotten forwarder for a *brand-new* defaulted method, so adding it is a required step.
- **Out of scope (deliberately NOT migrated this phase; record as such — Task 5):** `ThemeColorPick` (Color is not a `FieldValue`); the dialog-*open* pre-fill editor reads at `program.rs:2558-2563` (find) and `:2644-2655` (replace); `SaveAsPick`'s `downcast_mut::<FileEditor>()` write (its result *read* already uses `value()`; the FileEditor write is the same routing-write category and is not in the Find/Replace scope — leave it untouched, do not record, it is a separate consumer not named by Phase 4).
- **Coordinates / types:** `FieldValue` is `crate::data::FieldValue`; its `Bits` variant is `Bits(u32)`. `EF_DO_REPLACE: u16 = 0x0010` (`editor.rs:96`).
- **Commands:** workspace build. `export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target`. Use `cargo test --workspace -j2 -- --test-threads=2`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`. Commit messages end with the project Co-Authored-By trailer.

---

## File Structure

The work centers on three files plus the macro/spy and docs:

- `src/view/view.rs` — the `View` trait: add the defaulted `set_modal_data(&mut self, _data: FieldValue)` method (near `value`/`set_value`).
- `tvision-rs-macros/src/specs.rs` — add the `set_modal_data` forwarder.
- `tests/delegate_view.rs` — add the `set_modal_data` spy entry (impl + call + name + count bump).
- `src/widgets/editor.rs` — add the `Editor::set_modal_data` override (parse the `List`); remove the now-dead inherent `set_find_str`/`set_replace_str`/`set_editor_flags` (Task 4).
- `src/widgets/cluster.rs` — remove the now-dead `CheckBoxes::as_any_mut` override + its `skip(...)` entry (Task 4).
- `src/app/program.rs` — add the `field_bits` helper (beside `field_int`/`field_text`); convert the `FindPick` (Task 2) and `ReplacePick` (Task 3) arms of `apply_modal_completion`; record the open-side kept-read reason (Task 5).
- Docs: `docs/book/src/apps/dialogs.md` (the modal-result rule), `docs/book/src/internals/custom-view.md` (widget authors override `set_modal_data`), `docs/IMPLEMENTATION-LOG.md`, `docs/HANDOVER.md`, and the worktree SDD ledger. (Everything under `docs/book/book/` is **generated** — do not hand-edit.)

---

## Task 1: Add `View::set_modal_data` + `field_bits` + the `Editor` override (with a unit test)

**Files:**
- Modify: `src/view/view.rs` (add the defaulted trait method near `value`/`set_value`)
- Modify: `tvision-rs-macros/src/specs.rs` (add the forwarder)
- Modify: `tests/delegate_view.rs` (spy impl + call + name + count comment)
- Modify: `src/widgets/editor.rs` (add `Editor::set_modal_data` override + a `#[cfg(test)]` unit test)
- Modify: `src/app/program.rs` (add the `field_bits` helper)

**Interfaces:**
- Produces: `View::set_modal_data(&mut self, _data: crate::data::FieldValue)` — defaulted no-op; the editor overrides it to load a search record. Consumed by Tasks 2–3.
- Produces: `Editor::set_modal_data` override — accepts a `FieldValue::List`: a 2-element `[Text(find), Bits(flags)]` sets `find_str` + `editor_flags` (Find shape, leaving `replace_str` untouched); a 3-element `[Text(find), Text(replace), Bits(flags)]` sets all three (Replace shape). `flags` is stored as `editor_flags = (flags as u16)` — the completion has already masked it and set/cleared `EF_DO_REPLACE`. Any other variant/shape is ignored (typed-model drop, like `set_value`).
- Produces: `fn field_bits(v: FieldValue) -> Option<u32>` in `program.rs` (the `Bits` sibling of `field_int`/`field_text`). Consumed by Tasks 2–3.

- [ ] **Step 1: Add the failing unit test for the `Editor` override**

In `src/widgets/editor.rs`, inside the existing `#[cfg(test)] mod tests` block (find it with `grep -n 'mod tests' src/widgets/editor.rs`), add:
```rust
    #[test]
    fn set_modal_data_loads_find_shape() {
        use crate::data::FieldValue;
        let mut e = Editor::new(crate::view::Rect::new(0, 0, 20, 10), None, None, None, 0);
        e.replace_str = "keep".to_string();
        e.set_modal_data(FieldValue::List(vec![
            FieldValue::Text("needle".into()),
            FieldValue::Bits(0x0003),
        ]));
        assert_eq!(e.find_str(), "needle");
        assert_eq!(e.editor_flags(), 0x0003);
        // Find shape must NOT touch replace_str.
        assert_eq!(e.replace_str(), "keep");
    }

    #[test]
    fn set_modal_data_loads_replace_shape() {
        use crate::data::FieldValue;
        let mut e = Editor::new(crate::view::Rect::new(0, 0, 20, 10), None, None, None, 0);
        e.set_modal_data(FieldValue::List(vec![
            FieldValue::Text("needle".into()),
            FieldValue::Text("thread".into()),
            FieldValue::Bits((0x000F | EF_DO_REPLACE as u32)),
        ]));
        assert_eq!(e.find_str(), "needle");
        assert_eq!(e.replace_str(), "thread");
        assert_eq!(e.editor_flags(), 0x000F | EF_DO_REPLACE);
    }
```
**Before writing**, confirm the `Editor::new` signature with `grep -n 'pub fn new' src/widgets/editor.rs` (the first `impl Editor` constructor) and the `find_str()`/`replace_str()`/`editor_flags()` getter names with `grep -n 'fn find_str\|fn replace_str\|fn editor_flags' src/widgets/editor.rs` — adjust the constructor args/getters to match exactly. If `find_str`/`replace_str`/`editor_flags` fields are private to the module (they are — same file), the direct `e.replace_str = ...` and the assertions through getters both work because the test is in the same module.

- [ ] **Step 2: Run the test to confirm it fails to compile (method missing)**

Run:
```bash
export CARGO_TARGET_DIR=/home/oetiker/scratch/cargo-target
cargo test --workspace -j2 -- --test-threads=2 set_modal_data 2>&1 | tail -20
```
Expected: compile error — `set_modal_data` is not yet a method (the trait method does not exist).

- [ ] **Step 3: Add the defaulted `View::set_modal_data` trait method**

In `src/view/view.rs`, near the `value`/`set_value`/`set_value_ctx` methods (find with `grep -n 'fn set_value_ctx\|fn value' src/view/view.rs`), add:
```rust
    /// Deliver a finished modal's **typed result record** to the view that
    /// launched it. Defaulted no-op; a launcher view overrides it to load the
    /// result the pump read out of the modal's fields (each via [`View::value`])
    /// and packed into an ordered [`FieldValue::List`](crate::data::FieldValue::List).
    ///
    /// Distinct from [`set_value`](View::set_value): `set_value` carries a view's
    /// **own** field/document data (e.g. the editor's text), whereas
    /// `set_modal_data` carries a **separate modal-result record** addressed to the
    /// launcher by id — the two channels must not collide. Driven from
    /// `apply_modal_completion` (the cluster-D modal-result path); the pump resolves
    /// the launcher with `group.find_mut(id)` and calls this method by **virtual
    /// dispatch**, never a downcast.
    ///
    /// # Turbo Vision heritage
    /// The return-less successor to delivering a dialog's gathered record back to
    /// the requester (`getData` read at the modal's close, handed to the owner).
    fn set_modal_data(&mut self, _data: crate::data::FieldValue) {}
```

- [ ] **Step 4: Add the macro forwarder**

In `tvision-rs-macros/src/specs.rs`, add an entry mirroring the shape of the existing `value`/`set_value` forwarders (find them with `grep -n '"set_value"\|"value"' tvision-rs-macros/src/specs.rs`):
```rust
        ("set_modal_data",
         quote! { fn set_modal_data(&mut self, data: #k::data::FieldValue) { self.#f.set_modal_data(data) } }),
```
**Verify** the `FieldValue` path matches how other `specs.rs` entries reference it (e.g. the `set_value` entry — copy its exact `#k::...::FieldValue` path). If `set_value` uses `#k::data::FieldValue`, match it; if it uses a re-export, match that.

- [ ] **Step 5: Add the `delegate_view` spy entry**

In `tests/delegate_view.rs`:
1. Add the spy impl method (near the `apply_page_sync` spy at ~line 149):
```rust
    fn set_modal_data(&mut self, _data: crate::data::FieldValue) {
        self.mark("set_modal_data");
    }
```
2. Add a call in the call-list section (near `d.apply_page_sync(0, &mut ctx);` ~line 318):
```rust
    d.set_modal_data(crate::data::FieldValue::Int(0));
```
3. Add `"set_modal_data"` to the asserted method-name list (near `"apply_page_sync"` ~line 371).
4. Bump the method-count comment (`grep -n 'Method count' tests/delegate_view.rs`) from 29 to **30** and note `+1 set_modal_data from Phase 4`.

- [ ] **Step 6: Add the `Editor::set_modal_data` override**

In `src/widgets/editor.rs`, in the `impl View for Editor` block (starts at `impl View for Editor` ~line 1761), add:
```rust
    /// Load a finished Find/Replace dialog's search record (the pump packs the
    /// dialog fields, read via [`View::value`], into an ordered
    /// [`FieldValue::List`](crate::data::FieldValue::List)). A 2-element
    /// `[Text(find), Bits(flags)]` is the Find shape (leaves `replace_str`
    /// untouched); a 3-element `[Text(find), Text(replace), Bits(flags)]` is the
    /// Replace shape. `flags` is pre-masked by the completion (and already carries
    /// or clears `EF_DO_REPLACE`); stored as the low 16 bits. Other shapes are
    /// ignored (typed-model drop, like [`set_value`](View::set_value)).
    fn set_modal_data(&mut self, data: crate::data::FieldValue) {
        use crate::data::FieldValue;
        if let FieldValue::List(items) = data {
            match items.as_slice() {
                [FieldValue::Text(find), FieldValue::Bits(flags)] => {
                    self.find_str = find.clone();
                    self.editor_flags = *flags as u16;
                }
                [
                    FieldValue::Text(find),
                    FieldValue::Text(replace),
                    FieldValue::Bits(flags),
                ] => {
                    self.find_str = find.clone();
                    self.replace_str = replace.clone();
                    self.editor_flags = *flags as u16;
                }
                _ => {}
            }
        }
    }
```
(`find_str`/`replace_str`/`editor_flags` are private fields on `Editor` in this file — direct assignment is in-module and valid; this inlines exactly what the old `set_find_str`/`set_replace_str`/`set_editor_flags` did, removed in Task 4.)

- [ ] **Step 7: Add the `field_bits` helper**

In `src/app/program.rs`, immediately after `field_text` (ends ~line 2886), add:
```rust
/// Extract the `u32` bit word out of a [`FieldValue::Bits`](crate::data::FieldValue::Bits),
/// or `None` for any other variant. The `Bits` sibling of [`field_int`]/[`field_text`],
/// used by the Find/Replace modal-result reads to pull a `CheckBoxes` options word
/// through [`View::value`].
fn field_bits(v: crate::data::FieldValue) -> Option<u32> {
    match v {
        crate::data::FieldValue::Bits(b) => Some(b),
        _ => None,
    }
}
```

- [ ] **Step 8: Run the unit tests + the spy test**

Run:
```bash
cargo test --workspace -j2 -- --test-threads=2 set_modal_data delegate_view 2>&1 | tail -25
```
Expected: `set_modal_data_loads_find_shape`, `set_modal_data_loads_replace_shape`, and the `delegate_view` spy test all PASS.

- [ ] **Step 9: Full build, test, lint**

Run:
```bash
cargo test --workspace -j2 -- --test-threads=2
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```
Expected: all green. `field_bits` is not yet called (Tasks 2–3) — clippy will flag `dead_code`. If so, add `#[allow(dead_code)]` on `field_bits` **with a comment** `// used by FindPick/ReplacePick in the next tasks` (removed implicitly once Task 2 calls it — re-check clippy clean at the end of Task 2). Alternatively land Tasks 1+2 together if the dead-code gate blocks the commit; prefer the `#[allow]` + comment to keep tasks reviewable.

- [ ] **Step 10: Commit**

```bash
git add -A
git commit
# message: "feat(view): add View::set_modal_data + Editor override + field_bits helper (Phase 4 substrate)"
```

---

## Task 2: Convert the `FindPick` completion to downcast-free reads + `set_modal_data` delivery

**Files:**
- Modify: `src/app/program.rs` — the `ModalCompletion::FindPick` arm of `apply_modal_completion` (~lines 3000-3033).

**Interfaces:**
- Consumes: `field_bits` + `View::set_modal_data` (Task 1); `CheckBoxes::value()`→`Bits` (Phase 2); `InputLine::value()`→`Text`; `field_text` (existing).
- Produces: nothing new (rewrites an existing arm). The `FindPick { editor_id, find_id, opts_id }` variant shape is **unchanged** — the by-id routing/reads stay; only the *mechanism* (value() vs downcast) changes.

- [ ] **Step 1: Rewrite the `FindPick` arm**

In `src/app/program.rs`, replace the body of the `ModalCompletion::FindPick { editor_id, find_id, opts_id }` arm (the block from `if result == Command::CANCEL` through `Some(Event::Command(Command::SEARCH_AGAIN))`, ~lines 3005-3032) with:
```rust
            if result == Command::CANCEL {
                return None;
            }
            // Read the find string from the InputLine via the value() trait.
            let find_str = group
                .find_mut(find_id)
                .and_then(|v| v.value())
                .and_then(field_text)
                .unwrap_or_default();
            // Read the options bit word from the CheckBoxes via value() (Bits) —
            // no downcast. Mask to bits 0-1 (case sensitive, whole words).
            let opts = group
                .find_mut(opts_id)
                .and_then(|v| v.value())
                .and_then(field_bits)
                .unwrap_or(0)
                & 0x0003;
            // Deliver the search record to the editor via set_modal_data (the
            // 2-element Find shape: find + flags, no EF_DO_REPLACE) — virtual
            // dispatch, never a downcast. The editor leaves replace_str untouched.
            if let Some(ed) = group.find_mut(editor_id) {
                ed.set_modal_data(crate::data::FieldValue::List(vec![
                    crate::data::FieldValue::Text(find_str),
                    crate::data::FieldValue::Bits(opts),
                ]));
            }
            // Re-inject cmSearchAgain to run do_search_replace on the editor.
            Some(Event::Command(Command::SEARCH_AGAIN))
```
Note `opts` is now `u32` (from `field_bits`) — `& 0x0003` and `Bits(opts)` are `u32`, matching the variant. The editor casts to `u16` in `set_modal_data`.

- [ ] **Step 2: Grep-proof the FindPick downcasts are gone**

Run:
```bash
sed -n '/ModalCompletion::FindPick/,/^        }/p' src/app/program.rs | grep -n 'downcast_mut\|as_any_mut'
```
Expected: no matches inside the `FindPick` arm.

- [ ] **Step 3: Build, test, lint**

Run:
```bash
cargo test --workspace -j2 -- --test-threads=2
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```
Expected: all green; any editor find/replace snapshot tests **byte-identical**. `field_bits` is now used (the Task-1 `#[allow(dead_code)]`, if added, is now removable — remove it and re-run clippy). If any snapshot changed, STOP — it is a real regression.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit
# message: "refactor(program): FindPick reads via value()/field_bits + delivers via set_modal_data (no downcast)"
```

---

## Task 3: Convert the `ReplacePick` completion to downcast-free reads + `set_modal_data` delivery

**Files:**
- Modify: `src/app/program.rs` — the `ModalCompletion::ReplacePick` arm of `apply_modal_completion` (~lines 3037-3074).

**Interfaces:**
- Consumes: same as Task 2, plus `EF_DO_REPLACE` (`crate::widgets::EF_DO_REPLACE`, a `u16`).
- Produces: nothing new. The `ReplacePick { editor_id, find_id, replace_id, opts_id }` variant is **unchanged**.

- [ ] **Step 1: Rewrite the `ReplacePick` arm**

In `src/app/program.rs`, replace the body of the `ModalCompletion::ReplacePick { editor_id, find_id, replace_id, opts_id }` arm (~lines 3043-3073) with:
```rust
            if result == Command::CANCEL {
                return None;
            }
            let find_str = group
                .find_mut(find_id)
                .and_then(|v| v.value())
                .and_then(field_text)
                .unwrap_or_default();
            let replace_str = group
                .find_mut(replace_id)
                .and_then(|v| v.value())
                .and_then(field_text)
                .unwrap_or_default();
            // Options via value() (Bits) — no downcast. Mask to bits 0-3
            // (case, whole-words, prompt, replace-all), then set EF_DO_REPLACE
            // unconditionally for the replace flow.
            let opts = (group
                .find_mut(opts_id)
                .and_then(|v| v.value())
                .and_then(field_bits)
                .unwrap_or(0)
                & 0x000F)
                | crate::widgets::EF_DO_REPLACE as u32;
            // Deliver via set_modal_data (the 3-element Replace shape: find +
            // replace + flags) — virtual dispatch, never a downcast.
            if let Some(ed) = group.find_mut(editor_id) {
                ed.set_modal_data(crate::data::FieldValue::List(vec![
                    crate::data::FieldValue::Text(find_str),
                    crate::data::FieldValue::Text(replace_str),
                    crate::data::FieldValue::Bits(opts),
                ]));
            }
            Some(Event::Command(Command::SEARCH_AGAIN))
```
This preserves the old semantics exactly: `find_str`/`replace_str` set, `editor_flags = (opts & 0x000F) | EF_DO_REPLACE` (now folded into `opts` before delivery; the editor stores `flags as u16`).

- [ ] **Step 2: Grep-proof the ReplacePick downcasts are gone**

Run:
```bash
sed -n '/ModalCompletion::ReplacePick/,/^        }/p' src/app/program.rs | grep -n 'downcast_mut\|as_any_mut'
```
Expected: no matches inside the `ReplacePick` arm.

- [ ] **Step 3: Build, test, lint**

Run:
```bash
cargo test --workspace -j2 -- --test-threads=2
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```
Expected: all green; find/replace snapshots byte-identical.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit
# message: "refactor(program): ReplacePick reads via value()/field_bits + delivers via set_modal_data (no downcast)"
```

---

## Task 4: Remove the now-dead setters + the dead `CheckBoxes::as_any_mut` hook

**Files:**
- Modify: `src/widgets/editor.rs` — remove `set_find_str`, `set_replace_str`, `set_editor_flags` (~lines 1574-1587).
- Modify: `src/widgets/cluster.rs` — remove the `CheckBoxes::as_any_mut` override (~lines 740-744) + drop `as_any_mut` from the `#[crate::delegate(to = cluster, skip(...))]` list (line 738).

**Interfaces:** none (pure dead-code removal). Both removals are gated on grep-proofs that nothing references them.

- [ ] **Step 1: Prove the three editor setters are dead**

Run:
```bash
grep -rn 'set_find_str\|set_replace_str\|set_editor_flags' src/ tests/
```
Expected: only the **definitions** in `src/widgets/editor.rs` (and possibly their own doc-comment lines). NO call sites (Tasks 2–3 removed the only callers — the completions now use `set_modal_data`; the dialog-*open* seams use the `find_str()`/`replace_str()`/`editor_flags()` **getters**, which stay). If any call site remains, STOP and investigate — do not remove a live method.

- [ ] **Step 2: Remove the three setters**

In `src/widgets/editor.rs`, delete the `set_find_str`, `set_replace_str`, and `set_editor_flags` methods (the three `pub(crate) fn` at ~1574-1587, including their doc comments). Leave the `find_str()`/`replace_str()`/`editor_flags()` getters and `editor_flags` field intact.

- [ ] **Step 3: Prove `CheckBoxes` is no longer downcast anywhere**

Run:
```bash
grep -rn 'downcast_mut::<CheckBoxes>\|downcast_ref::<CheckBoxes>\|downcast::<CheckBoxes>' src/ tests/
```
Expected: no matches (Tasks 2–3 removed the only two downcasts — the `FindPick`/`ReplacePick` option reads).

- [ ] **Step 4: Remove the dead `CheckBoxes::as_any_mut` override + skip entry**

In `src/widgets/cluster.rs`:
1. Delete the `fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> { Some(self) }` override on `impl View for CheckBoxes` (~lines 740-744, including its `/// Downcast hook: ...` doc comment).
2. On the `#[crate::delegate(to = cluster, skip(apply_scroll_sync, focus_descendant, grabs_focus_on_click, set_value, value))]` attribute (line 738), confirm `as_any_mut` is **already absent** from the skip list — looking at the current source, `CheckBoxes`'s skip list is `skip(apply_scroll_sync, focus_descendant, grabs_focus_on_click, set_value, value)` and does **not** contain `as_any_mut` (the override existed *outside* the skip list as an extra method). So **no skip-list edit is needed** — just delete the method. After deletion, the macro generates the default-forwarding `as_any_mut` (→ `self.cluster.as_any_mut()`), which is `None` by default. **Verify** by re-reading the attribute after editing: `grep -n 'delegate(to = cluster' src/widgets/cluster.rs`.

   (If, contrary to the above, your working copy DOES list `as_any_mut` in the `CheckBoxes` skip set, remove it from the skip set as well so the forwarder is generated.)

- [ ] **Step 5: Build, test, lint**

Run:
```bash
cargo test --workspace -j2 -- --test-threads=2
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```
Expected: all green; the `delegate_view` spy test still passes (it asserts the macro's generated method set — `CheckBoxes` now forwards `as_any_mut` by default); snapshots byte-identical.

- [ ] **Step 6: Commit**

```bash
git add -A
git commit
# message: "refactor(editor,cluster): drop dead set_find_str/set_editor_flags + CheckBoxes::as_any_mut (Phase 4)"
```

---

## Task 5: Record the deliberate exceptions (ThemeColorPick + the open-side pre-fill reads)

**Files:**
- Modify: `src/app/program.rs` — append the §2.1 reason to the `ThemeColorPick` variant doc (~line 427) and a kept-read comment on the dialog-*open* editor reads (`OpenFindDialog` ~line 2558, `OpenReplaceDialog` ~line 2644).

**Interfaces:** none (doc/comment only — no behavior or signature change).

**Rationale to record:**
- **`ThemeColorPick`** stays downcasting (`ColorPicker` → `color()`, write into `ThemeEditorBody`) because its payload is a `Color`, which is deliberately **not** a `FieldValue` (a 4-variant enum that cannot pack into `Bits`; `FieldValue::Color` is an explicit non-goal — see `program.rs` `ColorPick` doc + spec §3.1). It never crosses a `FieldValue` boundary, so the `set_modal_data` path does not apply. Recorded exception, not a downcast Phase 4 claims to delete (§3.3).
- **The dialog-*open* pre-fill reads** (`group.find_mut(editor_id).downcast_mut::<Editor>()` to read `find_str()`/`editor_flags()` when *building* the Find/Replace dialog) are a different category from modal-result reads: the pump reads a **known widget's current state to construct UI** — the same kept category as FileDialog readback / parent→child display reads (spec §4.4/§6.6). Phase 4 ("modal-result reads") does not target them; recorded as deliberately kept.

- [ ] **Step 1: Append the §2.1 reason to the `ThemeColorPick` variant doc**

In `src/app/program.rs`, on the `ThemeColorPick` variant doc (~line 427-430), append a sentence:
```rust
    /// Result from the per-role color picker opened from `ThemeEditorBody`.
    /// On [`Command::OK`], read the `ColorPicker`'s color() and update the
    /// `ThemeEditorBody`'s working theme for the given role/fg. On cancel,
    /// nothing.
    ///
    /// **Deliberate cluster-D exception (spec §3.3/§2.1):** unlike `FindPick`/
    /// `ReplacePick` (which deliver their result downcast-free via
    /// [`View::set_modal_data`]), this completion stays downcasting because its
    /// payload is a [`Color`](crate::color::Color), which is deliberately **not** a
    /// [`FieldValue`](crate::data::FieldValue) (`FieldValue::Color` is an explicit
    /// non-goal). The result never crosses a `FieldValue` boundary, so the
    /// `set_modal_data` path does not apply — recorded, not a downcast we claim to
    /// delete.
```

- [ ] **Step 2: Add the kept-read comment on the two open seams**

In `src/app/program.rs`, on the `Deferred::OpenFindDialog` arm just above the `// Read current editor search state.` line (~line 2557), insert:
```rust
                                // NOTE (Phase 4, spec §4.4): this is a dialog-OPEN
                                // pre-fill read — the pump reads the editor's current
                                // search state to seed the freshly-built dialog. That
                                // is "build UI from a known widget's state," the kept
                                // structural-read category (like FileDialog readback),
                                // NOT a cluster-D modal-result read. It stays a
                                // downcast deliberately; only the completion reads
                                // (FindPick/ReplacePick) went downcast-free.
```
Add the equivalent comment on the `Deferred::OpenReplaceDialog` arm above its `let (find_str, replace_str, editor_flags) = ...` read (~line 2644).

- [ ] **Step 3: Build, fmt (doc/comment-only — no test change expected)**

Run:
```bash
cargo build --workspace -j2
cargo fmt --all --check
```
Expected: clean build; fmt clean.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit
# message: "docs(program): record Phase 4 deliberate exceptions (ThemeColorPick Color, open-side pre-fill reads)"
```

---

## Task 6: Docs + final whole-phase verification

**Files:**
- Modify: `docs/book/src/apps/dialogs.md` (the modal-result decision rule + `set_modal_data`).
- Modify: `docs/book/src/internals/custom-view.md` (widget authors override `set_modal_data`).
- Modify: `docs/IMPLEMENTATION-LOG.md` (prepend a Phase 4 section).
- Modify: `docs/HANDOVER.md` (mark Phase 4 done; point "Next" at Phase 5).
- Modify: the worktree SDD ledger (append a "PHASE 4 COMPLETE" block — find it with the path noted in HANDOVER, e.g. `sdd/progress.md`).
- Verify only (no edit): generated `docs/book/book/**`.

**Interfaces:** none (docs + verification).

- [ ] **Step 1: Update `apps/dialogs.md` with the modal-result rule**

In `docs/book/src/apps/dialogs.md`, find the section that discusses reading a dialog's data / `FieldValue` (`grep -n 'FieldValue\|gather\|modal' docs/book/src/apps/dialogs.md`). Add a short paragraph stating the modal-result decision rule: a **view-launched** modal's result is read field-by-field through `View::value()` and delivered to the launcher by id via `View::set_modal_data(FieldValue)` (no downcast); a **`Program`-launched** modal's result returns natively via `exec_view_with<R>` (the by-value path for rich types like `Color`/`Theme`). Any ```` ```rust ```` block must follow the doctest convention: a hidden `# use tvision_rs as tv;` (and a hidden `# fn _demo(...) {…}` wrapper for method calls). Prefer prose-only to avoid adding doctest surface unless an example genuinely clarifies.

- [ ] **Step 2: Note `set_modal_data` in `internals/custom-view.md`**

In `docs/book/src/internals/custom-view.md`, find where the data/sync `View` methods for widget authors are listed (`grep -n 'value\|set_value\|apply_scroll_sync\|set_indicator_value' docs/book/src/internals/custom-view.md`). Add one sentence: a view that launches a modal and needs its typed result overrides `set_modal_data(&mut self, data: FieldValue)` to load the ordered `List` the pump read out of the modal's fields — virtual dispatch, no framework downcast.

- [ ] **Step 3: Verify no stale broker names linger in the hand-edited guide**

Run:
```bash
grep -rn 'set_find_str\|set_editor_flags\|downcast.*CheckBoxes\|downcast.*Editor' docs/book/src/
```
Expected: no matches that describe the *removed* mechanism as current (any historical mention must read as past/heritage, not present behavior).

- [ ] **Step 4: Prepend the IMPLEMENTATION-LOG section**

Add a newest-first section to `docs/IMPLEMENTATION-LOG.md` summarizing Phase 4: the `FindPick`/`ReplacePick` completions now read each dialog field via `View::value()` (`CheckBoxes`→`Bits`, the new `field_bits` helper) and deliver the editor's search record as one ordered `FieldValue::List` via the new defaulted `View::set_modal_data` (Editor override); the dead `set_find_str`/`set_replace_str`/`set_editor_flags` + `CheckBoxes::as_any_mut` removed; **the recorded §2.1 deviation** (per-field `value()` reads instead of the spec's literal `gather_list`-on-the-modal — `gather_list` is inherent/not trait-reachable, and per-field reads avoid the broader `Dialog::value()` semantic change + child-order coupling); and the recorded exceptions (`ThemeColorPick` = `Color` not `FieldValue`; open-side pre-fill reads = kept structural reads).

- [ ] **Step 5: Final grep-proof — no Find/Replace modal-result downcast remains**

Run:
```bash
sed -n '/ModalCompletion::FindPick/,/Some(Event::Command(Command::SEARCH_AGAIN))/p' src/app/program.rs | grep -c 'downcast_mut\|as_any_mut'
sed -n '/ModalCompletion::ReplacePick/,/Some(Event::Command(Command::SEARCH_AGAIN))/p' src/app/program.rs | grep -c 'downcast_mut\|as_any_mut'
grep -rn 'downcast_mut::<CheckBoxes>' src/
```
Expected: `0`, `0`, and no matches respectively. (The `ThemeColorPick` `ColorPicker`/`ThemeEditorBody` downcasts and the open-side `Editor` reads remain by design — Task 5.)

- [ ] **Step 6: Full integrated-tree gate**

Run:
```bash
cargo test --workspace -j2 -- --test-threads=2
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo build --examples
```
Expected: all green; **no snapshot changes across the whole phase** (every task was behavior-preserving except the two new `set_modal_data` unit tests).

- [ ] **Step 7: Docs build (guide doctests + link check)**

Run:
```bash
cargo xtask test
cargo xtask docs
grep -rl rustdoc_include docs/book/book/ || echo "no leftover include directives"
```
Expected: `cargo xtask test` green (any new guide rust block compiles), `cargo xtask docs` builds + link-checks clean (modulo the ~20+ pre-existing unresolved-link warnings noted in HANDOVER — confirm you added none new), and no leftover `rustdoc_include` directives.

- [ ] **Step 8: Update the SDD ledger + HANDOVER**

Append a "PHASE 4 COMPLETE" block to the worktree's SDD ledger (commits + per-task review verdicts + any deferred Minors). Update `docs/HANDOVER.md`'s 2026-06-19 section: move Phase 4 into the "stacked on the branch" list, mark it done, and re-point "Next" at **Phase 5 (generic `ExecView`: `Context::request_exec_view` + `Deferred::OpenModal`; make `tcv`'s Info box a real custom dialog)**. Commit:
```bash
git add -A
git commit
# message: "docs(handover,log): Phase 4 modal-result-via-FieldValue complete; next is Phase 5"
```

---

## Self-Review

**Spec coverage (§3.3 / §5 Phase 4):**
- "Convert `FindPick`/`ReplacePick` to read the modal via `value()` … + deliver by id; drop the multi-child downcasts" → Tasks 2–3 read each field via `View::value()` (`field_text`/`field_bits`) and deliver via `set_modal_data` by id; Task 4 removes the now-dead `CheckBoxes::as_any_mut` + editor setters; the CheckBoxes-read and Editor-write downcasts are grep-proven gone (Tasks 2, 3, 6).
- "the *routing* stays; the *reads* go downcast-free" → the `FindPick`/`ReplacePick` variants keep `editor_id`/`find_id`/`replace_id`/`opts_id` (by-id routing unchanged); only the read/delivery mechanism changes.
- "`set_value(FieldValue)` (or a `set_modal_data(FieldValue)` sibling) delivers the typed result" → Task 1 adds the `set_modal_data` sibling (distinct from `set_value`, which carries the editor's own document text) + forwarder + spy entry.
- "`ThemeColorPick` is the recorded exception (its payload is a `Color`)" → Task 5 records it on the variant doc; it stays downcasting by design.
- The **deliberate deviation** from the spec's literal "`gather_data` the modal" (per-field `value()` reads instead) is recorded in Global Constraints + IMPLEMENTATION-LOG (Task 6), with the §2.1 rationale (inherent `gather_list` not trait-reachable; avoids the broader `Dialog::value()` change + child-order coupling). The spec's own sentence permits "a field via `value()`," so this is within-spec, not against it.
- "Docs land WITH the phase" → Task 6 updates `apps/dialogs.md` + `internals/custom-view.md` + IMPLEMENTATION-LOG, and runs `cargo xtask test`/`docs`.

**Placeholder scan:** every code step shows the actual code; every command shows expected output. The Editor constructor args + getter names are explicitly cross-checked against the source in Task 1 Step 1 (the one place exact local signatures must be confirmed by grep) rather than guessed.

**Type consistency:** `set_modal_data(&mut self, data: FieldValue)` is identical across the trait default (Task 1 Step 3), forwarder (Step 4), spy (Step 5), and `Editor` override (Step 6). `field_bits(v: FieldValue) -> Option<u32>` is consistent across its def (Task 1 Step 7) and call sites (Tasks 2–3). The `FieldValue::List` shapes are consistent: Find = `[Text, Bits]` (Task 2 produces, Task 1 Editor override + unit test consume); Replace = `[Text, Text, Bits]` (Task 3 produces, Task 1 consumes). `opts` is `u32` (from `field_bits`) end-to-end; the editor casts `flags as u16` once, in `set_modal_data`.

**Out-of-scope guard:** `ThemeColorPick`, the open-side pre-fill reads, and `SaveAsPick`'s `FileEditor` write are explicitly excluded (Global Constraints) and recorded where they remain (Task 5); Task 6 Step 5 confirms only the in-scope downcasts were removed.
