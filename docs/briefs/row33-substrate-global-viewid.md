# Brief — Substrate realignment: global `ViewId` + self-id + tree-walk resolvers

**Type:** FOUNDATION (substrate). **Gates:** row 33d (drag + close) and all later
cross-tree view addressing. **Model:** Opus.

## Why (read first)

The row-17 `ViewId` substrate diverged from the guide's own intent. `PORTING-GUIDE.md`
D3 promises *one* id space with resolution as "a tree-walk performed by the downward
context" and a `ctx.query(id, …)`. What got built instead: each `Group` embeds its own
generational `ViewArena`, so ids are **group-local** and a bare `ViewId` is meaningless
outside its group. Two consequences:

1. The promised global-resolution / `ctx.query` mechanism was never built — every new
   cross-tree need (drag, close) is forced to invent a bespoke downward channel.
2. The generational reuse-safety is **dead code**: `ViewArena::is_valid` is called
   nowhere outside its own unit tests. Resolution is always `Group::index_of` (scan
   children by full `ViewId` equality), which already returns `None` for a removed
   child — so the use-after-free / ABA hazard the generations guard against cannot occur.

The fix (already recorded in `PORTING-GUIDE.md` D3, "Resolution substrate — corrected"):
a **single process-global, monotonic `ViewId`**, **stamped into each view's own
`ViewState.id`** at insert, resolved by a **`find_mut(id)` tree-walk**. This deletes the
unused arena and makes "the loop acts on a view by id" the obvious primitive.

**Do NOT in this stage:** build drag, close, or `ctx.query`. This stage delivers only
the substrate + resolvers. (`ctx.query` is now *unblocked* but built later, when a
consumer needs it.)

## Verification gate (every step)

- `cargo test` — all green (baseline is **269 unit + 3 integration + doctests**).
- `cargo clippy --all-targets -- -D warnings` — clean.
- `cargo fmt --check` — clean.
English for all code/comments. Faithful, minimal, no speculative API.

---

## STEP 1 — behaviour-preserving swap (must leave ALL existing tests green)

Land this first and confirm the full suite is green **before** Step 2. If a test reddens
here, it's the swap, not the resolver.

### `src/view/id.rs` — replace the arena with a global monotonic minter

Delete `ViewArena`, `Slot`, and every method on them (`alloc`/`free`/`is_valid`/`len`/
`is_empty`/`Default`) and the `index()`/`generation()` accessors (no live callers).
Replace with:

```rust
use std::num::NonZeroU64;
use std::sync::atomic::{AtomicU64, Ordering};

/// A lightweight, globally-unique view identity (D3). `Copy`, carries no
/// reference into the tree, so it can be stored freely (sibling links, focus
/// stacks, capture handlers). Identity is `ViewId` equality.
///
/// Ids are **process-global and monotonic** — minted once at `Group::insert`,
/// never reused. A stale handle (its view removed) therefore matches nothing and
/// simply fails to resolve via [`View::find_mut`]; there is no slot to alias, so
/// no generational validation is needed. The `NonZeroU64` gives `Option<ViewId>`
/// a niche (no discriminant word). The `u64` space never realistically exhausts
/// (mirrors `TimerId`, D9).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ViewId(NonZeroU64);

/// The global id counter. Starts at 1 so the first id is non-zero.
static NEXT_VIEW_ID: AtomicU64 = AtomicU64::new(1);

impl ViewId {
    /// Mint a fresh, globally-unique id. Called by [`Group::insert`].
    pub fn next() -> ViewId {
        let n = NEXT_VIEW_ID.fetch_add(1, Ordering::Relaxed);
        // fetch_add starts at 1 and only increases; n is never 0 in practice.
        ViewId(NonZeroU64::new(n).expect("view id counter starts at 1 and increases"))
    }
}
```

Rewrite the unit tests for the new model (drop arena/generation/reuse tests, which no
longer apply). Cover: `next()` returns distinct, strictly-increasing ids; the
`Option<ViewId>` niche (`size_of::<Option<ViewId>>() == size_of::<ViewId>()`). Do **not**
assert literal id *values* (the global counter is shared across tests; only relative
distinctness/ordering within a test is meaningful).

### `src/view/mod.rs`

Drop `ViewArena` from the re-export; keep `pub use id::ViewId;`.

### `src/view/view.rs` — `ViewState` knows its own id

Add a field `id: Option<ViewId>` to `ViewState` (default `None` in `ViewState::new`).
Add a public accessor:

```rust
/// This view's global identity, set by [`Group::insert`] when the view enters a
/// group; `None` before insertion. NOT an up-pointer — it is the view's own
/// handle (like an ECS entity id), which lets a handler/loop address it by id.
pub fn id(&self) -> Option<ViewId> { self.id }
```

(`Group::insert` will set it — see below. Add `use crate::view::id::ViewId;` to view.rs
if not already in scope.)

### `src/view/group.rs` — mint global ids, stamp `ViewState.id`, drop the arena

- Change the import `use crate::view::id::{ViewArena, ViewId};` → `use crate::view::id::ViewId;`.
- Remove the `arena: ViewArena` field and its `ViewArena::new()` initialisation.
- In `insert`, replace `let id = self.arena.alloc();` with:
  ```rust
  let id = ViewId::next();
  view.state_mut().id = Some(id);   // stamp the view's own handle (self-id)
  ```
  (keep the rest: push `Child { id, view }`, return `id`).
- In `remove`, delete the `self.arena.free(id);` line. Everything else (remove from the
  `Vec`, `reset_current` when the removed child was current) is unchanged.
- Update the `group.rs` module doc that says "group-local `ViewArena`" to reflect global
  ids + self-id (one sentence).
- `index_of` is unchanged (it compares full `ViewId` equality — dropping generations
  changes nothing it relied on).

**After Step 1: run the full gate. All 269 tests must be green.** Commit boundary:
`feat: substrate — global monotonic ViewId + self-id (drop unused generational arena)`.

---

## STEP 2 — the tree-walk resolvers

### `src/view/view.rs` — two defaulted `View` trait methods

```rust
/// Resolve `id` to a **descendant** of this view (never self — the *parent*
/// identifies a view by id). A leaf has no descendants, so the base returns
/// `None`; a `Group` overrides to search its children and recurse; a
/// `Group`-embedding view delegates to its inner group. This is the "tree-walk
/// via Context" promised by D3 — the uniform way the event loop / a capture
/// handler acts on a view it holds only by id (move a window's bounds, flip
/// `sfDragging`, …).
fn find_mut(&mut self, id: ViewId) -> Option<&mut dyn View> {
    let _ = id;
    None
}

/// Remove the descendant named by `id` from whichever group owns it (faithful
/// `destroy`/self-removal). Returns `true` if it was found+removed. Distinct
/// from `find_mut` because removal happens in the *owner's* child `Vec` (a view
/// cannot remove itself — it doesn't know its owner, D3) and must run the
/// owning group's `reset_current`. Base: `false` (a leaf owns nothing).
fn remove_descendant(&mut self, id: ViewId, ctx: &mut Context) -> bool {
    let _ = (id, ctx);
    false
}
```

### `src/view/group.rs` — `Group`'s overrides (in its `impl View`)

```rust
fn find_mut(&mut self, id: ViewId) -> Option<&mut dyn View> {
    // Direct children first (the common case), then recurse. Two passes keep the
    // borrows trivial — do NOT fold into one pass (it tangles NLL needlessly).
    for child in self.children.iter_mut() {
        if child.view.state().id == Some(id) {
            return Some(child.view.as_mut());
        }
    }
    for child in self.children.iter_mut() {
        if let Some(v) = child.view.find_mut(id) {
            return Some(v);
        }
    }
    None
}

fn remove_descendant(&mut self, id: ViewId, ctx: &mut Context) -> bool {
    if self.index_of(id).is_some() {
        self.remove(id, ctx); // direct child: faithful removal + reset_current
        return true;
    }
    for child in self.children.iter_mut() {
        if child.view.remove_descendant(id, ctx) {
            return true;
        }
    }
    false
}
```

### `src/window/window.rs` and `src/desktop/desktop.rs` — delegate

Each embeds a `Group`; add the two delegating overrides to their `impl View` so the walk
descends through them (exactly as `draw`/`handle_event` already delegate):

```rust
fn find_mut(&mut self, id: ViewId) -> Option<&mut dyn View> {
    self.group.find_mut(id)
}
fn remove_descendant(&mut self, id: ViewId, ctx: &mut Context) -> bool {
    self.group.remove_descendant(id, ctx)
}
```

(Add `ViewId` to the `use` list where needed. `Frame` is a leaf — no override.)

### Tests (Step 2)

Add unit tests proving the walk works *through the embedders* (no `Program` needed):

- **Nested resolve:** a root `Group` → insert a `Desktop` → insert a `Window` → insert a
  probe child into the window. `root.find_mut(probe_id)` returns the probe (mutate a field
  through it and observe). `root.find_mut(window_id)` returns the window. `root.find_mut`
  of a never-minted id (e.g. `ViewId::next()` you don't insert) returns `None`.
- **Nested remove:** `root.remove_descendant(window_id, ctx)` returns `true`, the desktop
  no longer contains the window (`find_mut` now `None`), and the desktop's `current`
  updated (reset_current ran). `remove_descendant` of a bogus id returns `false` and
  changes nothing.
- **Direct-child resolve/remove** on a plain `Group` (the non-delegating path).

**After Step 2: full gate green.** Commit boundary:
`feat: substrate — find_mut/remove_descendant tree-walk resolvers`.

---

## Borrow / gotcha notes for the implementer

- `Child.view` is `Box<dyn View>`; `child.view.as_mut()` yields `&mut dyn View`. The
  returned reference's lifetime is tied to `&mut self` (correct for
  `find_mut(&mut self) -> Option<&mut dyn View>`).
- The `View` trait already has `Context` in scope (used by `handle_event`); no new import
  needed for `remove_descendant`'s signature beyond `ViewId`.
- Process-global ids mean ids are **not** reset between tests. Never `assert_eq!(id, <literal>)`;
  compare ids to each other within a single test only. Snapshots don't carry ids.
- Keep `Group::remove` (inherent) as the single removal implementation;
  `remove_descendant` just routes to it for the owning group.

## Out of scope (explicit)

Drag, close/`cmClose`, `cmNext`/`cmPrev`, `setState` enable-set, `ctx.query`,
scrollbar auto-repeat — all are **row 33d / later**. This stage is substrate only.
