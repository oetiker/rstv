# Cross-view brokering & `ViewId`

A scroller needs to know how far its scrollbars have moved; a list box needs to
push its position back into them. In tvision-rs, ownership is a downward tree of
`Box<dyn View>`, and every up- or sideways link is a lightweight handle — a
[`ViewId`](../api/tvision-rs/view/struct.ViewId.html) — rather than a raw pointer.
The event loop resolves handles and performs the actual cross-view reads and
writes at a safe point when no other borrow is active. See
[Pointers & `infoPtr` → handles](../port/handles.md) for the broader handle
design.

> **Turbo Vision heritage:** in C++ the two views simply held raw `TView*`
> pointers at each other and called across directly. Rust forbids that aliased
> mutable access — the broker pattern is the replacement.

## `ViewId`: identity, not a pointer

A `ViewId` is a `Copy`, globally-unique identity — internally one
`NonZeroU64`, minted from a single monotonic process counter when a view is
inserted into its group (`Group::insert`), and stamped into the view's own
`ViewState`. It is **not** an index into any store. You can stash it freely (in
a sibling link, a focus stack, a capture handler) because it borrows nothing.
Use `Option<ViewId>` for "no link" — the `NonZeroU64` gives that a free niche,
so it costs no extra size.

Resolving a handle back to the live view is a tree-walk:
`View::find_mut(id)` recurses through the groups and hands back a
`&mut dyn View`, or `None`. A handle whose view has since been removed simply
resolves to `None` — there is no dangling slot to alias, so no generational
validation is ever needed.

## Why a leaf can't reach its sibling

During event handling a leaf view holds only `&mut Context` (the downward
borrow). It cannot reach back up to its parent group, let alone sideways to a
sibling — the borrow checker would be holding two `&mut` into the same tree. So
a scroller that wants to read its scrollbars, or write new parameters into them,
has no inline way to do it.

## The pump is the broker

The resolution: the leaf does not act directly. It *requests* the cross-view
read or write as a [deferred effect](deferred.md), and the
[event loop](event-loop.md) — which owns the whole tree — performs it at
deferred-apply time, when the entire tree is reachable through the root group.
The pump is the broker.

The pattern is always the same: resolve each participant in its **own**
`find_mut`, so only one `&mut` is live at a time, then either call through the
`View` trait (virtual dispatch) or downcast to the concrete type via
`as_any_mut()` depending on what the effect needs.

```rust,ignore
// Illustrative sketch — not a standalone program.
// At deferred-apply, inside the pump, `group` is the whole tree:
let hv = h_bar_id
    .and_then(|id| group.find_mut(id))   // resolve one bar
    .and_then(|view| view.value())       // read its value via the value protocol
    .and_then(field_int);                // -> Option<i32>
// ...read the v-bar the same way, then call the target through the View trait:
if let Some(view) = group.find_mut(target) {
    view.apply_scroll_sync(hv, vv, &mut ctx);  // virtual dispatch — no downcast
}
```

Reads cross the seam through the value protocol (`View::value()` →
`FieldValue::Int`), so the broker never needs to know the sibling's concrete
type to read a number. Writes use a **defaulted `View` trait method** when the
effect can be expressed generically — the callee overrides exactly what it needs
without the pump knowing the concrete type.

The **scroll-family read-syncs** — Scroller, all list-viewer variants, Outline
viewer, and Editor — share a single hook `View::apply_scroll_sync(h, v, ctx)`
and a single deferred variant `Deferred::ScrollSync { target, h, v }`. Each
widget overrides `apply_scroll_sync` to interpret the `Option<i32>` deltas per
its own semantics (Scroller and Outline use `unwrap_or(0)`; Editor preserves
`None` to skip an axis). For composite widgets such as the text editor, the
`#[delegate(to = field)]` macro forwards `apply_scroll_sync` to the inner
view automatically, so the pump calls it without knowing the wrapper.

Two further syncs — `Deferred::IndicatorSetValue` and `Deferred::PageStackSync`
— are also downcast-free but keep their own hooks (`View::set_indicator_value`
and `View::apply_page_sync`) because their payloads are not scroll deltas and
the shared `(h, v)` hook would not fit (§2.1).

A downcast (`as_any_mut()` + `downcast_mut`) is now used only where the effect
genuinely requires the concrete struct and no trait hook fits — `ScrollBarSetParams`
(the *write* direction, scroller → `ScrollBar`) and `SplitterDivider`
(`Splitter`'s divider-move op). These remain as downcast sites by design.

Each cross-view interaction is its own deferred variant — `ScrollSync`,
`ScrollBarSetParams`, `IndicatorSetValue`, `PageStackSync`, and so on — so
adding a new brokered relationship means [adding a variant](deferred.md), not
threading a new pointer.

## Avoiding feedback loops

A read-sync that writes back (the list viewer pushes its new position into the
v-bar) could re-enter forever. It does not, because the scrollbar's parameter
setter — `ScrollBar::set_params` — is change-guarded: it re-broadcasts
`cmScrollBarChanged` only when the value actually changes. Writing back the value
the bar already holds is a silent no-op, and the cycle goes quiet on the next
pump.
