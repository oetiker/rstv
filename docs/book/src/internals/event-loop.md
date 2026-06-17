# The event loop in depth

tvision-rs runs the entire application on **one** non-recursive event loop in
[`Program`](../api/tvision-rs/app/struct.Program.html). Every event — keystrokes,
mouse motion, modal dialogs, window drags, mouse hold-tracking — routes through a
single pass called `pump_once`. Modality and press-and-hold are not separate
blocking loops; they are *capture handlers* stacked on a LIFO capture stack (see
[Cross-view brokering & ViewId](./brokering.md) and the
[capture section](#the-capture-stack) below). Because the whole tree is owned
behind a single `&mut`, exactly one thing at a time may borrow it: the single
loop enforces this structurally.

> **Turbo Vision heritage:** the C++ library had *many* loops. `execView` spun a
> nested blocking `getEvent` loop for every modal dialog; `dragView` spun another
> while you dragged a window; a pressed button spun its own while you held the
> mouse. Each re-entered the framework and re-borrowed the view tree — which Rust
> forbids. Every one of those nested loop bodies is now a capture handler.

## `run` is the only outer loop

[`Program::run`](../api/tvision-rs/app/struct.Program.html#method.run) is the whole
application loop. It pumps until something sets an end command, then asks the
*tree* to validate that command; if it validates, return, otherwise clear it and
keep pumping:

```rust,ignore
// Illustrative sketch — not a standalone program.
loop {
    self.end_state = None;
    while self.end_state.is_none() {
        self.pump_and_drive();        // one event, fully processed
    }
    let es = self.end_state.unwrap();
    if self.valid_end(es) {           // tree-wide valid() walk
        return es;
    }
}
```

> **Turbo Vision heritage:** this mirrors `TGroup::execute`'s
> `while (!valid(endState))` pattern.

[`run_app`](../api/tvision-rs/app/struct.Program.html#method.run_app) is the same
loop with one addition: any [`Command`](../api/tvision-rs/command/struct.Command.html)
that survives all view routing is handed to your callback. That is where menu
commands like "open the color picker" get serviced. You almost always call one of
these two and never touch the machinery below.

## One pass: `pump_once`

`pump_once` is the heart of the single loop. Each call does exactly one trip through these
phases, in order:

| Phase | What happens |
| ----- | ------------ |
| **Resize** | Query the terminal size; if it changed, relayout the whole tree. There is no `Event::Resize` — the backend is polled live. |
| **Settle currency** | Apply any pending insert-time focus cascades so the event about to be dispatched sees a fully settled focus state. |
| **Pick an event** | Drain the internal queue first, else poll the backend with the frame-tick timeout; an idle pick may synthesize a mouse auto-repeat. |
| **Idle** | No event: fire expired timers as [`Event::Timer`](../api/tvision-rs/event/enum.Event.html), refresh the status line's help context. |
| **Pre-route** | A `KeyDown` (always) or a `MouseDown` on the status line is offered to the status line first, so accelerators like F10/Alt-X fire even under a modal. |
| **The dispatch gate** | Drop the event if it is a disabled command; otherwise offer it to the capture stack, then to normal view routing. |
| **Deferred drain** | Apply every queued effect once, in insertion order. |
| **Cursor + redraw** | Set the hardware cursor, then redraw the whole tree and diff it to the screen. |

### The dispatch gate

Before an event reaches a view it passes a small gate. A command that is
currently **disabled** is dropped here — tvision-rs uses a denylist, so unknown custom
commands flow through untouched (see [Commands & events](../apps/commands.md)).
What survives is offered to the [capture stack](#the-capture-stack) first; only if
no handler consumes it does it go to the normal view-tree walk
(`program_handle_event`). A modal handler that consumes every otherwise-unhandled
event *is* the modal loop.

### The deferred drain

A view is borrowed *downward* during dispatch as `&mut dyn View` plus a
[`Context`](../api/tvision-rs/view/struct.Context.html); it cannot reach back up to
the loop-owned capture stack, command set, or sibling views. So instead of acting
inline it **queues** the effect, and the pump applies the whole queue in one pass
*after* dispatch — capture pushes, command enable/disable, bounds changes, modal
close, focus moves, and the cross-view broker syncs. This is the
[`Deferred`](../api/tvision-rs/view/enum.Deferred.html) channel; it has its own page,
[Deferred effects](./deferred.md). Two rules matter here: the drain runs even when
the pre-route consumed the event, and it runs **once** — anything an effect
re-queues waits for the next pump (a loop-until-empty would risk spinning).

Because capture pushes are deferred, a freshly pushed handler sees the *next*
event, not the one that pushed it — the push and the first handled event are
always separated by at least one pump boundary.

## The capture stack

The [`CaptureStack`](../api/tvision-rs/capture/struct.CaptureStack.html) is the LIFO
list of [`CaptureHandler`](../api/tvision-rs/capture/trait.CaptureHandler.html)s that
implements modality, dragging, press-and-hold, and menu sessions — anything that
needs to intercept events globally before normal routing. Each handler is offered
every event and returns a
[`CaptureFlow`](../api/tvision-rs/capture/enum.CaptureFlow.html):

- `Pass` — not mine; offer it to the next lower handler, then to the view tree.
- `Consumed` — handled; stop routing, stay on the stack.
- `ConsumedPop` — handled, and remove *myself* (e.g. a modal closing).

The return value is authoritative — handlers do **not** signal "consumed" by
clearing the event. A handler holds a [`ViewId`](./brokering.md), never a view
reference. Concrete handlers include a bounds-gating *modal frame*, window
dragging and keyboard resize, mouse hold-tracking, and the menu session. Before
every dispatch the pump re-syncs each bounds-gating handler from the live tree
(`sync_gate_bounds`), so a dialog you have just dragged stays clickable in its new
position.

## Where to go next

- [Deferred effects](./deferred.md) — the full effect catalogue and why each one
  is queued rather than applied inline.
- [Cross-view brokering & ViewId](./brokering.md) — how the pump brokers reads and
  writes between sibling views during the drain.
- [Modal execView → one loop + capture](../port/modal.md) — the veteran's view of
  how `execView` became a capture handler.
