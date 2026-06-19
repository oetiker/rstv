# Unified typed data-movement substrate (design)

**Date:** 2026-06-18
**Status:** design v5 (three reviews + extensibility pass: `FieldValue::Custom` open seam for third-party components, §3.5) — awaiting owner review → writing-plans
**Origin:** Porting the `tcv` example faithfully (its `Desktop^.ExecView(InfoBox)`)
surfaced consumer-API gap #2 (no generic view-launched modal). Investigating
*why* it wasn't ported revealed a deeper issue: C++'s one loose data-movement
mechanism (`void*`/`infoPtr`, `getData`/`setData`, `message`, `TStreamable`) was
ported into **several different typed solutions at different sites**. `program.rs`
carries on the order of ~40 `as_any` downcasts, but they are **not** all
data-movement: roughly the cluster-B sync brokers, the cluster-D modal-result
reads, and the cluster-E `Rc` sinks are the un-Rusty data-movement residue this
design removes — the rest (~half: structural parent→child pushes, `desktop_insert`,
FileDialog readback, and test scaffolding) are a different category and stay (§4.4,
§6.6). Objective (owner): **clean up the data-movement residue** — a single typed *currency* and one
*right mechanism per kind of data movement*, applied **uniformly** (the framework
is pre-release; retrofit cost is immaterial), faithful to Turbo Vision and
consistent with the port's lingo. The generic `ExecView` capability is built on it.

**Dual objective (owner):** the unified mechanism must serve **both** the
framework's own widgets **and** user-written components — the open, typed
interchange C++ achieved with `void*` but *without* erasing type safety across the
board. So the design must stay **open to third-party components and data structures
they invent**, not just tidy the framework's known widgets. This is the §3.5 open
story (three typed paths; `FieldValue::Custom` for user-invented payloads); it is a
first-class goal, not an afterthought.

> **v2 note.** v1 proposed a `BrokerMsg` god-enum + `Deferred::Broker` +
> `apply_broker`. An adversarial TV-porting/Rust-framework review rejected it:
> (a) it was a god-enum forcing every widget into `match { _ => {} }`, conserving
> (not reducing) concept count; (b) it created a *second* structured-payload
> vocabulary overlapping the widened `FieldValue`; (c) the `exec_view_with`
> closure as written was a use-after-free-shaped bug; (d) "zero `dyn Any` / ~40
> downcasts gone" was dishonest (multi-view dialogs don't fit a single-`self`
> hook). v2 drops `BrokerMsg`, centers on `FieldValue` as the data currency,
> reuses the existing defaulted-`View`-method pattern for sync signals, fixes the
> closure, and states honest scope.

---

## 1. Problem: one C++ idiom, many Rust answers

C++ moves data across view/loop boundaries loosely: `TEvent.infoPtr` (`void*`),
`getData`/`setData`/`dataSize` over raw record memory, `message(target, …,
infoPtr)`, `TStreamable`. Rust forbids that looseness, so the port solved the
problem in several places, several ways. Inventory (file:line in the audit
transcript), six clusters:

| Cluster | Problem | Current mechanism | Typing today |
|---|---|---|---|
| **A** | read/write a dialog field | `FieldValue` + `value`/`set_value`; `Group::gather_data`/`scatter_data` | typed enum (`Text`/`Int` only) |
| **B** | keep two siblings in sync (push a signal to a view by id) | ~10 `Deferred::Sync*`/`Set*` variants; ~half via `as_any` downcast, ~half already via a defaulted `View` method (`apply_list_scroll`, `set_menu_current`, `update_menu_commands`) | mixed: downcast + trait method |
| **C** | decide which sibling a broadcast is about | `Event::Broadcast { source: ViewId }` + `source == self.x` filter | `ViewId` filter (addressing, not payload) |
| **D** | deliver a modal's result to its launcher **view** | `ModalCompletion::{HistoryPick, RouteModalAnswer, SaveAsPick, FindPick, ReplacePick, ThemeColorPick}` + `set_modal_answer` | per-consumer enum; multi-child `as_any` reads |
| **E** | deliver a modal's result to a **`Program` method** | `ColorPick`/`ThemeEdit` via `Rc<Cell>`/`Rc<RefCell>` sinks | shared cell + `as_any` |
| **F** | pure loop-control effects | `Deferred::{PushCapture, EnableCommand, Close, EndModal, …}` | already unified, correct |

**The laissez-faire residue:** the `as_any`/`downcast_mut` reads in clusters B, D,
E. They are the typed port's nearest thing to C++ `void*`. Cleaning them is the goal.

**Key insight from the review:** there is **not one** unifying mechanism — there
are **two distinct kinds** of data movement that must stay distinct (folding sync
signals into the field-data currency is the over-unification v1 committed):

- **Field/record data** (cluster A; the *read side* of D) — small, named,
  marshalled values that cross into a view that can't receive a native type.
- **Sync signals** (cluster B) — "this sibling changed; recompute" — not field
  data; a behavioral poke at a known view.

DRY means *one way to do each kind*, not one mechanism for both.

---

## 2. Goals & non-goals

**Goals**
- **`FieldValue` is the single typed data currency** (widened) — the `getData`/
  `setData` successor. Every data-bearing view implements `value()`/`set_value()`.
- **Sync signals deliver through defaulted per-capability `View` methods**
  (extend the `apply_list_scroll` pattern), so **no sync site downcasts in the
  pump**.
- **`exec_view_with<R>`** returns top-level modal results **by native value**
  (deletes the `Rc` sinks).
- **Generic `ExecView`** (`request_exec_view` + `Deferred::OpenModal`) for gap #2,
  validated by `tcv`'s real Info box.
- Applied **broadly but with judgment** (pre-release, so retrofit cost is no
  excuse to skip a genuine improvement) — see §2.1.
- **No *framework-internal* `dyn Any`.** The pump never downcasts to its own known
  widget types. A *deliberate, typed-at-the-edges* `dyn Any` escape exists for
  user-invented payloads (`FieldValue::Custom`, §3.1) — that is the framework being
  correctly agnostic about types only the user owns, not a leak. (Honest scope below;
  the open/extensible story is §3.5.)
- **Open to third-party components.** A user's own widgets must have a unified,
  *typed* means to interchange data — both with framework widgets (via the
  well-known scalars) and with each other (via `Custom`) — without reopening C++'s
  anonymous-`void*` wound. See §3.5.

### 2.1 Guiding principle: apply where it helps, not where it burdens

Churn is cheap pre-release, so "it's a lot of sites" is **not** a reason to skip a
real cleanup. But "apply it everywhere" is **not** a licence to force the concept
into places it fits badly. Convert a site **only when** doing so (a) removes a real
downcast or duplication **and** (b) the result reads as naturally as (or better
than) what it replaces. **Do not** force-fit where it becomes a burden:

- don't route a single-field control's value through a record `FieldValue::List` —
  a plain `Text`/`Int` is the natural shape;
- don't mint a defaulted `View` method for a genuinely one-off, single-caller sync
  poke if a method makes the trait noisier than the downcast it removes — weigh
  trait-surface cost against the downcast removed;
- don't contort a genuinely multi-view, concrete-type orchestration into a
  `FieldValue` shape just to claim a downcast was removed;
- don't pull non-data structural pushes (parent→child display state) into the data
  path (§6.6).

The test is *"is this site clearer after?"*, not *"is this site converted?"* Any
site deliberately **left as-is records the one-line reason** (the house
"ported-or-deliberately-not-with-reason" rule), so a reader sees the boundary was a
decision, not an omission.

**Non-goals (explicitly NOT unified)**
- `Event::Broadcast.source` stays a subject *filter* (cluster C), never a payload.
- `gather_data`/`scatter_data` stays an ordered synchronous group-walk producing/
  consuming a `FieldValue` record (faithful to `getData` summing `dataSize`).
- Cluster-F loop-control `Deferred` variants stay effect variants.
- The two modal-result paths (view-launched vs `Program`-launched) stay distinct —
  forced by ownership (§4.2).
- **Non-data structural `as_any` is out of scope** and stays: parent→child display
  pushes (`Frame::set_flags`/`set_zoomed`, `Window`→frame), `desktop_insert`,
  FileDialog readback. These are "a parent reaches a *known* child to push display
  state," a different category from data movement. We do **not** claim to remove them.
- No serialization/`serde` (in-process; `TStreamable`→serde stays parked for persistence).
- **No startup wiring-declaration registry** (a DI-container that statically
  validates "producer A emits what consumer B expects"). It would force every
  consumer to declare its data dependencies up front — ceremony TV and idiomatic
  Rust both avoid — and for a statically-linked program a `cargo test` catches the
  same mismatch *earlier* than any runtime startup gate. We instead make a mismatch
  *loud* (the `value_as::<T>()` accessor) and offer an *optional* per-component
  self-test (§3.5), which is the realistic slice of the "surface problems early"
  idea. (Rationale: §3.5, §6.7.)

---

## 3. The design

### 3.1 Widen `FieldValue` (the one data currency)

`src/data.rs` — keep the name ("the typed unit of data transfer"); widen so it can
carry every field kind and a whole record:

```rust
pub enum FieldValue {
    Text(String),
    Int(i32),
    Bool(bool),
    Bits(u32),                       // cluster controls (checkbox/radio bit words)
    List(Vec<FieldValue>),           // an ORDERED record == C++ getData's offset walk
    Custom(Rc<dyn Any>),             // user-invented payloads; downcast at the edge (§3.5)
}
```

(`Rc<dyn Any>`, not `Box`, because `FieldValue` is `Clone` — `List` needs it — and
`Rc` gives cheap clone + `downcast_ref`. The `Custom` escape is the open
extensibility seam; see §3.5 for why it is *not* the framework-internal `dyn Any`
smell this design removes.)

**No keyed/`Map` variant.** C++ `getData(void *rec)` is *positional and anonymous*
— it walks children in order writing each child's `dataSize()` bytes at a running
offset (`tgroup.cpp:303-311`); there are no field names anywhere in TV's data
protocol, and the port already mirrors this (`Group::gather_data` →
`Vec<Option<FieldValue>>`, `Child { id, view }` with no name). So the faithful
record is the **ordered `List`**, not a keyed map (a `Map` would invent a naming
dimension that exists in neither C++ nor the model, and could not be constructed
from the positional `gather_data`). If a keyed view is ever genuinely wanted, it
must be justified as a deliberate *deviation* (like a D-rule), not as "the image of
`getData`."

Every data-bearing view implements `value()`/`set_value()` honestly:
`CheckBoxes`/`RadioButtons` → `Bits` (the bit word), and a
`Dialog`/`Group` gathers its children into an ordered `List` via `gather_data` (and
scatters a `List` back via `scatter_data`, in child order). `List` is the typed
image of C++ `getData(void *rec)`'s offset-addressed walk — so a whole dialog's
result is one `FieldValue::List`, **read without downcasting any child**.

**`Color` is NOT a `FieldValue`.** `Color` is a deliberate 4-variant enum
(`Default`/`Bios`/`Indexed`/`Rgb`, `color.rs:32`) — faithful to `TColorDesired`
*minus the bit-packing* — so it does **not** pack into `Bits(u32)` (that would
lose three of the four cases). This is not a new constraint: `ModalCompletion::ColorPick`'s
own doc (`program.rs:392`) already records "NOT a `FieldValue` … `FieldValue::Color`
is forbidden; the `color()` accessor is the contract." `Color` therefore rides the
**by-value path** (`exec_view_with<R>`, §3.3), alongside a whole `Theme` — the two
values too rich/structured to pack into `FieldValue`. `ColorPicker` exposes its
native `color()`; it is not a `value()`/`FieldValue` implementor.

### 3.2 Sync signals → defaulted per-capability `View` methods (no god-enum)

Cluster B is **not** field data; it keeps its per-kind `Deferred` variants (the
house "add-a-variant" rule), but each **delivers through a defaulted `View`
method** instead of a pump downcast — extending the pattern the port already uses
for `apply_list_scroll`/`set_menu_current`/`update_menu_commands`:

```rust
// src/view/view.rs — defaulted, object-safe (no Self params). Cluster B is NOT
// one method per Deferred variant — the scroll-sync FAMILY shares ONE hook.
//
// The five "a sibling scrolled, apply the delta and resync" brokers today —
// SyncScrollerDelta→Scroller, SyncEditorDelta→Editor, SyncOutlineViewerDelta→
// Outline, IndicatorSetValue→Indicator, PageStackSync→PageStack — collapse into
// a single scroll-sync hook, joining the existing `apply_list_scroll`:
fn apply_scroll_sync(&mut self, _delta: ScrollDelta, _ctx: &mut Context) {}
// Genuinely distinct, non-scroll sync kinds keep their own defaulted method
// (e.g. menu state) — only where the signature truly differs:
fn set_scroll_params(&mut self, _p: ScrollParams) {}
```

The pump's deferred-drain resolves `group.find_mut(target)` and calls the typed
method — **virtual dispatch to the concrete widget, never a downcast**. Each method
is defaulted, so widgets implement only the one(s) they answer (the compiler names
it; no `match { _ => {} }` arm-dropping). The five scroll-family brokers above are a
**five-into-one** collapse (a `ScrollDelta` carrying the per-widget delta shape),
not five parallel methods — the real DRY win, beyond merely de-downcasting each in
place. The exact `ScrollDelta` shape and which (if any) sync genuinely resists the
shared hook is settled per-widget in Phase 3 under the §2.1 test (a sync stays
separate only if folding it in makes the hook murkier than the downcast it removes,
reason recorded). **Each new `View` method needs a forwarder in
`tvision-rs-macros/src/specs.rs` AND an entry in `tests/delegate_view.rs`** (the
spy test) — mechanical, mirroring the existing `apply_list_scroll` forwarder.

*(Rejected: a single `apply_broker(&BrokerMsg)` god-enum — it conserves variant
count, forces arm-dropping, and duplicates the `FieldValue` payload vocabulary.)*

### 3.3 Modal results: `FieldValue` for views, `exec_view_with<R>` for `Program` methods

**View-launched modals (cluster D)** read the finished modal's result as a
`FieldValue` (a field via `value()`, or the whole record via `gather_data` →
ordered `List`) and deliver it to the requester by id — **no multi-child downcast**.
`set_modal_answer(Command)` stays for the command-only/decision case;
`set_value(FieldValue)` (or a `set_modal_data(FieldValue)` sibling) delivers the
typed result. Find/Replace stop downcasting `CheckBoxes`/`InputLine` children and
instead `gather_data` the modal into an ordered `List` the editor consumes. The multi-view
*routing* (which editor to write) is by-id and stays; only the *reads* go
downcast-free. `ThemeColorPick` is the one cluster-D result that is **not** a
`FieldValue` (its payload is a `Color`, §3.1): the theme-editor view recomposes its
style from its *own* embedded picker child via the native `color()` accessor (the
"second view" it reads is itself), so it never crosses a `FieldValue` boundary — a
deliberate exception, recorded, not a downcast we claim to delete.

**`Program`-launched modals (cluster E)** use a generic by-value return:

```rust
// src/app/program.rs — the extract closure is threaded INTO exec_view_with_completion
// and invoked at the pre-drop window (program.rs:~1442), NOT after it returns
// (the modal is removed+dropped at ~1463). It receives the modal's OWN &mut dyn View
// only; the existing end_state save/restore (re-entrancy) is inherited.
pub fn exec_view_with<R>(
    &mut self,
    view: Box<dyn View>,
    extract: impl FnOnce(&mut dyn View, Command) -> R,
) -> R
```

`R` is caller-named and returned by value — the framework never names it (no
`dyn Any`, no `Rc` cell). `color_dialog`/`theme_editor` switch to it; the
`ColorPick`/`ThemeEdit` `ModalCompletion` variants and both `Rc<Cell>`/
`Rc<RefCell>` sinks are **deleted**. No `Rc<RefCell>` "escape hatch" is documented
or kept: there is no view today wanting a big native result the `FieldValue` path
can't carry, and enshrining one would re-bless the shared-cell pattern this phase
removes. If such a need ever arises, it is a new design question, not a standing
recommendation.

### 3.4 Generic `ExecView` (gap #2) — the first consumer

- **View-launched:** `Context::request_exec_view(view: Box<dyn View>, requester:
  ViewId, then_command: Option<Command>)` queues `Deferred::OpenModal(view, …)`
  (the PORTING-GUIDE D9 pre-named plan). The pump stashes it in `pending_modal`
  (already `Box<dyn View>`), runs it via the existing single-loop machinery, and
  on close delivers the result to `requester` (command via `set_modal_answer`,
  data via `FieldValue`) and re-injects `then_command`.
- **`Program`-launched:** `exec_view_with<R>` (§3.3).

`tcv`'s Info box (read-only, OK = `cmCancel`) exercises only the command path; the
`FieldValue` path serves input dialogs.

### 3.5 Extensibility: third-party components (the open story)

The original objective is a *unified, typed* interchange for **both** framework
widgets **and** user-written components — what C++ achieved with `void*`/`getData`
but at the price of erasing *everything*, including its own well-known fields. The
port keeps the safety and stays open via **three paths**, each typed at the layer
that owns the type:

1. **Modal result** → `exec_view_with<R>` (§3.3). Generic over `R`: a user's
   component, launched modally, returns *any* native type by value. Already open;
   no framework change.
2. **Field data** → `FieldValue`. Well-known shapes (`Text/Int/Bool/Bits/List`)
   for interop with framework widgets and generic consumers; `Custom(Rc<dyn Any>)`
   for user-invented payloads.
3. **Notification** → `Event::Broadcast { command, source }`. `Command` is an open
   newtype (users mint their own namespaced commands), so a user component can
   notify siblings; the data it then needs is read via path 2.

**Why `Custom` is not the smell we removed.** The downcasts this design deletes are
the *framework* downcasting to its *own* concrete widgets in the pump — a smell,
because the framework knew the closed set. `Custom(Rc<dyn Any>)` is the *user*
round-tripping the *user's own* type through a framework that legitimately cannot
know it: the type knowledge lives at both edges (producer and consumer are user
code), and the framework moves the payload **opaquely, never inspecting it**. That
is precisely what `std::any` is for — the correct boundary, not a leak. The result
is *more* honest than C++: C++ erased even its own fields; we erase **only** the
genuinely-unknowable user payload, and keep everything else statically typed.

**Typed-at-the-edges, validated at runtime, fail-safe.** `Custom` is read with a
`downcast` — a **runtime** check (one `TypeId` integer comparison; not reflection,
not a string/hash lookup — effectively free, and only on the `Custom` path; the
scalars are a plain enum `match` with zero `Any` cost). It is type-**safe** (a
mismatch yields `None`, never a misread or UB) but not compile-**checked** across
the `value()` boundary, because the value crosses the object-safe `dyn View`
boundary, which cannot carry a generic `value<T>()` without giving up the
heterogeneous view tree. So the recommended accessor is **loud, not silent**:

```rust
// src/data.rs — fail LOUD with a descriptive error, not a silent None
impl FieldValue {
    pub fn custom<T: Any>(v: T) -> Self { FieldValue::Custom(Rc::new(v)) }
    pub fn as_custom<T: Any>(&self) -> Option<Rc<T>>; // raw, for match arms
    pub fn value_as<T: Any>(&self) -> Result<Rc<T>, FieldTypeError>; // RECOMMENDED
}
```

`value_as::<T>()` turns a contract mismatch into a clear error (`"expected T, found
<other>"`) **at first execution**, so a stale producer/consumer wiring announces
itself instead of going quietly wrong.

**Latent-mismatch risk, bounded.** A producer/consumer type disagreement is not
caught at build time and surfaces when its path first runs — the classic erasure
tradeoff. It is small and bounded: both ends share *one named, exported type* (the
compiler does check that type exists and is spelled right at both ends), so a
mismatch is *contract drift*, not a silent type slip; the scalar path has **zero**
latent risk (exhaustive `match`); `value_as` makes the failure loud; and **one
test exercising the actual A→B exchange** converts "random future time" into
"`cargo test`."

**Distributable components choose their openness.** A component shipped in a crate
exports its payload type publicly — the exported type *is* the contract. It may
expose data as `Custom(MyType)` (rich, typed; consumers depend on the crate) and/or
as scalars/`List` (dependency-free, any generic consumer can read it). C++'s
`void*` could only ever do the first, unsafely. Caveat to document: `TypeId` is
per-type-per-version, so a diamond dependency pulling the crate at two
semver-incompatible versions yields distinct types — `Custom` fails closed to
`None`/error across that boundary (inherent to any `Any`-based plugin; handled by
dependency discipline, not by this design).

**Tightly-coupled own components can skip erasure entirely.** Two of *your own*
components willing to be coupled can share a typed `Rc<RefCell<MyState>>` directly
and get full compile-time checking — `FieldValue::Custom` is only the price of
routing data through the framework's *generic* plumbing (gather/scatter,
broker-by-id), not a mandate.

**Optional per-component self-test (the realistic slice of "register & exercise at
startup").** A component author may ship a round-trip self-check proving its own
data machinery (`value()`/`set_value()`/`Custom`) is internally sound:

```rust
// what a component author writes; collected via `inventory` so an app can run all
fn data_self_check() -> Result<(), String>;  // produce → consume → assert round-trip
```

The framework offers an *optional* `inventory`-collected `Program::self_check() ->
Result<(), Vec<String>>` an app can assert in an integration test (or run behind a
debug flag at startup). This catches a component's **self-consistency** early.
Cross-component **wiring** is deliberately *not* validated here (it would require
the rejected dependency-declaration registry, §6.7); that stays covered by
`value_as` + the one A→B test. Self-test is opt-in: simple apps ignore it entirely.

---

## 4. Why this is right

### 4.1 DRY, honestly
One currency for field data (`FieldValue`), one mechanism for sync (defaulted
`View` methods), one by-value path for top-level results — *one way per kind*, not
one mechanism for all (which was the v1 god-enum mistake) and not two vocabularies
for the same kind (the v1 `FieldValue`-vs-`BrokerMsg` overlap). `FieldValue::List`
genuinely removes the cluster-D multi-child read downcasts; the trait-method sync
genuinely removes the cluster-B downcasts. The `Custom` escape does **not** add a
second currency: it is the *same* `FieldValue` channel carrying a payload the
framework declines to interpret — one currency, with one open slot for user types
(§3.5).

### 4.2 The ownership split is forced
A `Program` method *can* hold the `exec_view_with` closure; a downward-borrowed
view *cannot* (no `&mut Program`). So the two modal-result paths stay distinct by
necessity — closure-by-value for top-level, `FieldValue`/command for view-launched.

### 4.3 Faithful to TV
`FieldValue::List` is the ordered image of `getData(void *rec)`'s offset-addressed
child walk; `value`/`set_value`
are `getData`/`setData`; `gather_data`/`scatter_data` are `TGroup::getData/setData`
(child-order walk). Sync via a defaulted method is the **deferred, return-less
successor to `message(view, …, infoPtr)`** (the D3/D9 deviation — C++ `message`
is synchronous and returns `void*`; ours defers and returns nothing). `exec_view_with`
is C++ `execView` returning a value to a method caller. `Event::Broadcast.source`
is `infoPtr`-as-subject, unchanged.

### 4.4 Honest scope (no overclaim)
Removes the **framework-internal data-movement** downcasts (clusters B and
D-reads). Does **not** remove non-data structural `as_any` (parent→child display
pushes, `desktop_insert`, FileDialog readback) — a different category, kept and
documented. The remaining `dyn Any` is *user-owned*, by design: `FieldValue::Custom`
(the user round-tripping their own type, §3.5). So "no `dyn Any`" means *the
framework never downcasts to its own types* — not that erasure is banned for the
user's open extensibility seam.

---

## 5. Migration plan (uniform cleanup, staged; behavior-preserving per phase)

Reordered per the review — **the cleanest, highest-value phase lands first as the
proof of value.** Each phase is subagent-driven + two-stage-reviewed, snapshot-verified.

- **Phase 1 — `exec_view_with<R>` (proof of value, cleanest).** Add it (closure
  threaded into `exec_view_with_completion`, invoked pre-drop). Migrate
  `color_dialog`/`theme_editor`; delete the two `Rc` sinks + `ColorPick`/`ThemeEdit`
  variants. Single-view result, sound borrow. Highest reduction-per-risk.
- **Phase 2 — widen `FieldValue` + honest `value()` + the open seam.** Add
  `Bool`/`Bits`/`List` and `Custom(Rc<dyn Any>)` with the `custom`/`as_custom`/
  loud `value_as::<T>() -> Result<_, FieldTypeError>` accessors (§3.5); implement
  `value()`/`set_value()` on `CheckBoxes`/`RadioButtons` (not `ColorPicker` — Color
  is by-value, C-1); make `gather_data`/`scatter_data` produce/consume an ordered
  `List`. Snapshot dialogs unchanged. (The optional `inventory`-collected
  `Program::self_check()` + the `data_self_check` convention, §3.5, can land here or
  as a small follow-on — it is opt-in and additive.)
- **Phase 3 — sync signals → trait methods.** Widget-by-widget: add the defaulted
  `View` method (+ `specs.rs` forwarder + `delegate_view` entry), move the pump's
  downcast call to the method, verify, repeat. Retire each cluster-B downcast as
  its widget migrates. Genuinely incremental.
- **Phase 4 — modal-result reads via `FieldValue`.** Convert `FindPick`/
  `ReplacePick`/`ThemeColorPick` to read the modal via `value()`/`gather_data`
  (ordered `List`) + deliver by id; drop the multi-child downcasts. Honest: the *routing*
  stays; the *reads* go downcast-free. (Not a "free melt into a generic arm" — a
  real per-consumer conversion to the `FieldValue` read path.)
- **Phase 5 — generic `ExecView`.** `request_exec_view` + `Deferred::OpenModal`;
  make `tcv`'s Info box a real custom Dialog launched from the list. Snapshot.

Phases 1–4 are behavior-preserving refactors; Phase 5 adds the capability. Docs
land WITH each phase (§9), never trailing.

---

## 6. What stays separate (resisting over-unification)

1. **`Event::Broadcast.source`** — subject filter, never payload (re-adding payload
   re-creates the `infoPtr` polymorphism the port killed, D4).
2. **Sync signals vs field data** — kept as two mechanisms (trait methods vs
   `FieldValue`); folding sync into `FieldValue` was the v1 over-unification.
3. **`gather_data`/`scatter_data`** — ordered synchronous group-walk (faithful
   `getData`), not id-addressed delivery.
4. **Cluster-F loop-control `Deferred` variants** — effect variants, not data.
5. **The two modal-result paths** — distinct by ownership (§4.2).
6. **Non-data structural `as_any`** — parent→child display pushes, `desktop_insert`,
   FileDialog readback: a different category, kept (§4.4).
7. **Startup wiring-declaration registry** — rejected (§2 non-goals). Validating
   "producer A emits what consumer B expects" at startup needs every consumer to
   declare its data dependencies up front; a `cargo test` catches the same mismatch
   earlier for a statically-linked program. We keep the *optional, opt-in*
   per-component **self-test** (self-consistency only) and the loud `value_as`
   accessor (§3.5) — not a wiring graph.

---

## 7. Risks

- **Breadth (Phases 2–4):** every data control + every sync broker + the modal
  completions. Mitigation: strictly widget-by-widget, behavior-preserving,
  snapshot-verified after each; the `delegate_view` spy test guards each new trait
  method's forwarder.
- **`FieldValue` for `Color`/`Theme`:** neither packs into `FieldValue` — `Color`
  is a 4-variant enum (`color.rs:32`, not a `u32`) and `Theme` is far larger. Both
  are intentionally returned/read by value (`exec_view_with<R>` for the
  `Program`-launched color/theme dialogs; the native `color()` accessor for the
  theme editor's own picker child). Keep that boundary — `FieldValue::Color` is an
  explicit non-goal already enforced at `program.rs:392`.
- **Trait-surface growth:** far fewer than one-method-per-variant — the scroll
  family is one shared `apply_scroll_sync` hook (§3.2), so the net add is a small
  handful of defaulted methods, not ~10. Accepted: defaulted (widgets implement
  only what they answer), compiler-guided, and the honest idiomatic shape (vs a
  god-enum). Each needs a macro forwarder + spy entry.
- **`List` ordering contract:** `gather_data`/`scatter_data` are positional/ordered
  (faithful `getData`'s offset walk) and `List` is inherently positional, matching
  the existing `Vec<Option<FieldValue>>` — keep child order stable across
  gather/scatter (no keyed `Map`, deliberately; see §3.1).
- **`Custom` latent mismatch:** a producer/consumer type disagreement is
  runtime-checked, not compile-checked, so it can surface late (§3.5). Bounded by:
  the shared exported type (mismatch = contract drift, not a silent slip), the
  loud `value_as::<T>() -> Result` accessor, zero latent risk on the scalar path,
  and one A→B test. `TypeId` is per-version: a diamond dependency at incompatible
  versions fails closed (`None`/error) — document for component authors.

---

## 8. Faithfulness map (C++ → this design)

| C++ | This design |
|---|---|
| `getData`/`setData(void *rec)` / `dataSize` | widened `FieldValue` (ordered `List` = the positional record) + `value`/`set_value`; `gather_data`/`scatter_data` |
| `void*` payload of a *user* type | `FieldValue::Custom(Rc<dyn Any>)` — but typed-at-the-edges: producer & consumer share a named type, mismatch fails closed (§3.5), not C++'s silent reinterpret |
| `message(target, evBroadcast/evCommand, cmX, infoPtr)` | per-capability defaulted `View` method delivered via `Deferred::Sync*` (deferred, return-less successor — D3/D9) |
| `infoPtr` as subject | `Event::Broadcast.source` (unchanged) |
| `execView(p)` returns to a method caller | `exec_view_with<R>` (by value) |
| `execView(p)` from within a view | `request_exec_view` → result via `FieldValue`/command |
| `TStreamable` | out of scope (persistence only; stays parked) |

---

## 9. Documentation (part of "done", per phase)

New **public** API (`request_exec_view`, `exec_view_with`, the widened
`FieldValue` incl. `Custom`/`value_as`, the new sync `View` methods for widget
authors, the optional `Program::self_check`) and a new **conceptual model**
(field-data-currency vs sync-signal vs by-value-result; the three-channel
modal-result rule; the **three open extensibility paths** for third-party
components, §3.5). The mdBook guide (`docs/book/src/`) and rustdoc update **as part
of the phase that lands each piece**. Conventions: rustdoc is
user-facing (strip porting bookkeeping; quarantine C++ lineage into a `# Turbo
Vision heritage` section); new guide ```` ```rust ```` blocks need a hidden
`# use tvision_rs as tv;` and compile under `cargo xtask test`.

**Guide chapters to update (extend existing pages):**
- **`internals/brokering.md`** ("Cross-view brokering & ViewId") — the conceptual
  home: the sync-signal-via-defaulted-`View`-method model (no pump downcasts), and
  why sync is separate from field data. (Phase 3)
- **`apps/dialogs.md`** ("Dialogs & data") — the widened `FieldValue` as the data
  currency, `gather`/`scatter` ordered records (`List`), and the consumer recipe "build a
  custom modal, exec it, read its result," with the modal-result decision rule
  (view→`FieldValue`; top-level native/`Color`/`Theme`→`exec_view_with<R>`). (Phases 2, 5)
- **`port/modal.md`** ("Modal execView → one loop") — `request_exec_view` +
  `exec_view_with<R>`, with `tcv`'s Info box as the worked example via
  `{{#rustdoc_include}}`. (Phases 1 & 5)
- **`internals/custom-view.md`** ("Writing your own View") — widget authors
  implement the sync `View` methods + `value()`/`set_value()` instead of relying
  on a framework downcast. (Phases 2, 3)
- **`apps/extensibility.md`** (NEW — "Third-party components & data interchange") —
  the §3.5 open story: the three open paths; `FieldValue::Custom` with the
  `value_as` loud accessor; typed-at-the-edges/runtime-checked/fail-safe; the
  typed-vs-generic exposure choice for distributable crates and the `TypeId`-version
  caveat; share-typed-state-directly for compile-time coupling; the optional
  per-component `data_self_check` + `Program::self_check()`. (Phase 2)
- **`port/handles.md`** ("Pointers & infoPtr → handles") — refine: subject stays
  `Event::Broadcast.source`; field data is `FieldValue`; sync is a typed method. (Phase 3)
- **`port/deferred.md`** + **`internals/deferred.md`** — the `Deferred::Sync*`
  variants now deliver via trait methods; new `Deferred::OpenModal`. (Phases 3, 5)
- **`reference/symbol-map.md`** / **`reference/deviations.md`** — add new symbols
  (`request_exec_view`, `exec_view_with`, the sync methods, the `FieldValue`
  variants) and note the consolidation against D3/D4/D9/D10.

**Rustdoc** on every new public item, with the heritage-section convention.

**Verification (each phase):** `cargo xtask test` (guide doctests compile),
`cargo xtask docs` (regenerate + build + link-check), and
`grep -rl rustdoc_include docs/book/book` empty after any `{{#rustdoc_include}}` edit.
