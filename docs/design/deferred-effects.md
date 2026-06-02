# Design note — unify the post-dispatch deferred channels

> Status: **LANDED** (commit below, pre-33d-2). The channel unification shipped;
> the companion geometry tidy was **dropped** after implementation found its
> premise false — the constants are single-site (close/zoom zones only in
> `frame.rs`, grow corners only in `window.rs`), not duplicated, so a shared module
> would relocate single-site consts behind indirection and invert the frame↔window
> layering. Left as a non-goal.

## The concept the codebase already has but hasn't named

During event dispatch the view tree is a live **`&mut` borrow stack** (root →
desktop → window → frame). A view being handled cannot reach *up* or *sideways*
through that tree — every ancestor is already `&mut`-borrowed above it on the
stack, and a fresh `root.find_mut(id)` would alias that borrow. So any action a
view wants that touches **loop-owned state it can't borrow inline** must be
**deferred**: the view *requests* it through `Context`, and the loop *applies* it
after the dispatch unwinds and the root is free again.

That single idea — **"an effect on loop-owned state that a downward-borrowed view
can't perform itself"** — is what `pending_captures`, `command_changes`, and
`pending_tree_ops` all are. We just grew them one row at a time as three parallel
structures instead of one.

## Current state (three parallel channels)

`Context` carries three separate deferred Vecs (plus `out_events`, `timers`,
`now_ms`, `owner_size` — see *Boundary* below for why those are different):

```rust
pub fn new(
    out_events: &'a mut VecDeque<Event>,
    timers: &'a mut TimerQueue,
    now_ms: u64,
    pending_captures: &'a mut Vec<Box<dyn CaptureHandler>>,   // deferred
    command_changes:  &'a mut Vec<(Command, bool)>,           // deferred
    pending_tree_ops: &'a mut Vec<TreeOp>,                    // deferred
) -> Self
```

The loop drains them in three loops after dispatch (`program.rs` ~517/525/542):

```rust
for h in pending_captures.drain(..) { captures.push(h); }
for (cmd, enable) in pending_command_changes.drain(..) { /* enable/disable + flag */ }
let ops = std::mem::take(pending_tree_ops); /* find_mut/change_bounds, set_state, remove_descendant */
```

**The cost is accretion, not correctness.** Every new deferred capability =
a new field + a new `Context::new` parameter + a new drain loop + threading a new
local through **every** `Context::new` call site (≈17, almost all test harnesses).
33d-1 paid that tax for `pending_tree_ops`; row 34's `message()`/modal-pop will
pay it again; the scrollbar pass again. Each `Context::new` call site already looks
like this in tests:

```rust
let mut pending: Vec<Box<dyn CaptureHandler>> = Vec::new();
let mut cmd_changes: Vec<(Command, bool)> = Vec::new();
let mut tree_ops: Vec<TreeOp> = Vec::new();
let mut ctx = Context::new(&mut out, &mut timers, 0, &mut pending, &mut cmd_changes, &mut tree_ops);
```

## Proposal — one `Deferred` enum, one queue, one drain

```rust
/// An effect on loop-owned state that a downward-borrowed view/handler cannot
/// perform inline (the tree is a live `&mut` borrow stack during dispatch, D3/D9).
/// The view requests it via `Context`; the loop applies it after dispatch.
pub enum Deferred {
    /// Push a capture handler (sees the NEXT event, never the current).
    PushCapture(Box<dyn CaptureHandler>),
    /// Enable a command in the program's command set (`enableCommand`).
    EnableCommand(Command),
    /// Disable a command (`disableCommand`).
    DisableCommand(Command),
    /// Apply new bounds to a view by id (drag move/grow).
    ChangeBounds(ViewId, Rect),
    /// Flip a propagating state flag on a view by id (drag end → sfDragging off).
    SetState(ViewId, StateFlag, bool),
    /// Remove a view from whichever group owns it (`cmClose`).
    Close(ViewId),
}
```

`Context` holds one field, `deferred: &'a mut Vec<Deferred>`, and the existing
methods (`push_capture`, `enable_command`, `disable_command`, `request_bounds`,
`request_set_state`, `request_close`) all push a variant. The signature slims to
**four** params (the deferred trio collapses to one; `owner_size` already isn't a
ctor param — it's a defaulted field + setter):

```rust
pub fn new(
    out_events: &'a mut VecDeque<Event>,
    timers: &'a mut TimerQueue,
    now_ms: u64,
    deferred: &'a mut Vec<Deferred>,
) -> Self
```

Call sites stop churning — a future capability **adds a variant, touches no
signature**:

```rust
let mut deferred: Vec<Deferred> = Vec::new();
let mut ctx = Context::new(&mut out, &mut timers, 0, &mut deferred);
```

And the loop drains **once**, with the borrow story told a single time (cleaner
than today's three, not just fewer):

```rust
let ops = std::mem::take(deferred);          // drain to local FIRST (re-queues land next pump)
if !ops.is_empty() {
    let mut ctx = Context::new(out_events, timers, now, deferred); // deferred now empty
    for op in ops {
        match op {
            Deferred::PushCapture(h)       => captures.push(h),
            Deferred::EnableCommand(c)     => if !command_set.has(c) { command_set.enable_cmd(c);  *command_set_changed = true },
            Deferred::DisableCommand(c)    => if  command_set.has(c) { command_set.disable_cmd(c); *command_set_changed = true },
            Deferred::ChangeBounds(id, r)  => { if let Some(v) = group.find_mut(id) { v.change_bounds(r); } }
            Deferred::SetState(id, f, e)   => { if let Some(v) = group.find_mut(id) { v.set_state(f, e, &mut ctx); } }
            Deferred::Close(id)            => { group.remove_descendant(id, &mut ctx); }
        }
    }
}
```

`captures`, `command_set`, `group`, `command_set_changed` are all disjoint from
the ctx-backing fields (`out_events`/`timers`/`deferred`), so this composes with
the existing `pump_once` destructure exactly as the three loops do today.

## The boundary — what stays *out* of `Deferred` (so it stays principled)

- **`out_events` (post / broadcast) stays separate.** It is not a post-dispatch
  *effect* on loop/tree state — it *feeds the input stream*. It is drained at the
  **top** of the next pump and re-enters as an `Event` through normal routing;
  `Deferred` is drained at the **bottom** of the current pump and applied to
  state. Different lifecycle, different direction. Folding them would muddy a
  clean line: **`Deferred` = "mutate loop/tree state after this dispatch";
  `out_events` = "produce an event to route later."**
- **`timers` / `now_ms` / `owner_size` stay as they are.** `timers` is a queue
  mutated directly (not a deferred effect); `now_ms` is a per-pass clock sample;
  `owner_size` is transient routing state set/restored by `Group::handle_event`,
  not a loop-owned channel.

## The one thing to verify (not assume): cross-kind ordering

Today the loop applies **all** captures, then **all** command-changes, then
**all** tree-ops. A unified queue applies in **insertion order**, interleaving
kinds. This is *more* causally faithful, and I believe it's behaviourally
identical in every current case (a single dispatch pushes 0–1 deferred items;
nothing relies on "captures before command-changes"). But it is the one invariant
the implementer must check by audit, not take on faith — call it out in the brief.
The capture-push-sees-next-event invariant (`compose_full_protocol`) is preserved:
`PushCapture` still applies after dispatch.

## Companion tidy — shared window-chrome geometry

Separate, smaller: the frame and the window each encode the same hit-test
constants — title row (`y == 0`), close zone (`x ∈ 2..=4`), zoom zone
(`x ∈ w-5..=w-3`), grow corners (bottom row, `x <= 1` / `x >= w-2`). 33d-1 left
them duplicated in `frame.rs` (icon clicks) and `window.rs` (drag detection).
Extract them into small shared helpers (e.g. a `window::chrome` module with
`is_close_zone(x)`, `is_zoom_zone(x, w)`, `is_grow_corner(p, size)` + a
`title_row` const) so both sites read from one source of truth. This is cosmetic —
the window remains the correct drag *initiator* (it's the only node that knows
its own `ViewId` *and* sees the desktop extent via `ctx.owner_size()` at routing
time; the limits live at that grandparent level). We're de-duplicating *constants*,
not relocating *logic*.

## Scope / migration (all mechanical, churn shrinks net)

Touched: `src/view/context.rs` (the enum + field + method bodies), `src/app/
program.rs` (field + single drain), and every `Context::new` call site (collapse
three locals → one — a *net reduction* in the test harnesses). `TreeOp` is
subsumed by `Deferred` (rename/fold). No behavioural change; the existing 282
tests are the regression net. Best landed as its own stage **before 33d-2**, so
33d-2 and row 34 add variants instead of re-paying the channel tax.

## Recommendation

Do it before 33d-2. It is the highest elegance-per-effort change available: it
names a concept the code already embodies, removes the per-row accretion that
prompted this whole question, and makes the inherent (borrow-stack-forced) cost
*cheap* rather than trying to escape a cost that can't be escaped. The companion
geometry tidy can ride along or be a follow-up nit.

**Open question for review:** keep `Deferred` in `context.rs` (co-located with
`Context`, fewest new files) or give it `src/view/deferred.rs`? Leaning
`context.rs`.
