# Modal `execView` → one loop

In C++ Turbo Vision, modality is recursion. `TGroup::execView` spins a **nested,
blocking `getEvent` loop** inside the already-running one; the outer loop is
suspended on the call stack while the inner one runs, and the modal view ends it
by calling `endModal`. The same trick drives `dragView` and a pressed button's
press-and-hold tracking.

Rust will not let you do that. A nested loop would have to re-borrow the view
tree that the outer loop already holds `&mut` to — the borrow checker refuses,
and there is no `&mut self`-reentrancy to lean on. So the nested loops collapse
into **one** non-recursive event loop, and modality becomes a handler on the
capture stack rather than a new loop. [Event capture](capture.md) is the general
mechanism; modality is one use of it.

## Modality as a handler

The modal handler is `ModalFrame`. While it sits on the capture stack it lets
keyboard, command, and broadcast events pass through to normal routing — which
reaches the modal view because the group focuses it — while positional (mouse)
events are gated by the modal view's bounds: inside, they pass; outside, they are
consumed and swallowed, so views beneath the dialog never see the click. That
gate is exactly what "modal" means.

## `exec_view` steers the one loop

[`Program::exec_view`](../api/tvision-rs/app/struct.Program.html#method.exec_view) is
the blocking wrapper that replaces `execView`. It inserts the view, makes it
current, pushes a `ModalFrame`, then runs the *same*
[`pump_once`](../api/tvision-rs/app/struct.Program.html#method.pump_once) loop until
the view calls [`end_modal`](../api/tvision-rs/view/struct.Context.html#method.end_modal),
setting the end state. Then it pops the frame, removes the view, restores the
previous focus and command set, and returns the chosen
[`Command`](../api/tvision-rs/command/struct.Command.html). No new loop is spun —
`exec_view` just steers the one loop that was already running.

See [Dialogs & data](../apps/dialogs.md) for the user-facing recipe, [Event
capture](capture.md) for the mechanism modality shares with drag/resize and
press-and-hold, and [the event loop in depth](../internals/event-loop.md) for
what each `pump_once` turn does.

## Ending a modal (execView)

The full lifecycle of `exec_view` maps each C++ step to a tvision-rs equivalent:

| C++ `execView` step | tvision-rs |
| -------------------- | ---------- |
| Save and clear command set | Save `disabled_commands`; the modal starts with its own restrictions |
| `saveOwner`; insert into desktop | Root-insert into the program's group (not the desktop); own the view |
| `setState(sfModal, True)` | `state.state.modal = true` (direct field write) |
| `setCurrent(p, enterSelect)` | `group.set_current(id, SelectMode::Enter, &mut ctx)` |
| Push capture — none in C++ | Push a `ModalFrame` onto the capture stack |
| `p->execute()` — a nested loop | Steer the **same** pump loop; no new loop |
| `ClearEvent` on every unhandled event | The `ModalFrame` swallows outside-bounds mouse events |
| Pop capture — none in C++ | Pop the `ModalFrame` |
| `remove(p)`; restore focus | `group.remove(id, &mut ctx)`; restore previous current |
| Restore command set | Restore `disabled_commands` |
| Return `endState` | Return the `Command` that was passed to `end_modal` |

The capture-stack push and pop are the key addition over the C++ design: they
replace the nested loop's ability to intercept all events while the dialog runs.
Because the push is immediate (not deferred — `exec_view` holds `&mut self`
directly), the `ModalFrame` is live from the very first pump pass inside the
modal.

**Sources:** `exec_view_with_completion` in `src/app/program.rs`;
`ModalFrame` in `src/app/program.rs`; `CaptureStack` in `src/capture.rs`.

> **Turbo Vision heritage:** ports `TGroup::execView` (`tgroup.cpp`). The C++
> version spun a nested `getEvent` loop; in tvision-rs the nested loop is
> replaced by `ModalFrame` on the capture stack plus the shared `pump_once`
> loop (deviation D9).

## The modal loop (Execute)

tvision-rs has **one** event loop, period. The `Program::run` skeleton is:

```rust,ignore
// src/app/program.rs — Program::run (simplified)
loop {
    self.end_state = None;
    while self.end_state.is_none() {
        self.pump_and_drive();    // one pump pass: event → dispatch → redraw
    }
    let es = self.end_state.unwrap();
    if self.valid_end(es) {
        return es;
    }
}
```

`exec_view` runs the **same** `pump_and_drive` loop in a fresh `while` block
with its own `end_state`:

```rust,ignore
// src/app/program.rs — exec_view_with_completion (simplified inner loop)
loop {
    self.end_state = None;
    while self.end_state.is_none() {
        self.pump_and_drive();  // exactly the same pump
    }
    let es = self.end_state.unwrap();
    if self.validate_modal_close(id, es) {
        break es;
    }
}
```

There is no new thread, no async runtime, no re-borrow. The difference between
the outer `run` loop and the inner `exec_view` loop is only *which end-state
terminates them*: the outer loop ends when the whole application quits; the
inner loop ends when a view inside the dialog calls `end_modal`. When the
inner loop exits it restores `end_state` to its pre-modal value, so the outer
loop does not spuriously see the modal's end command.

A modal that opens another modal (e.g. a file dialog opening a history popup)
adds a second `exec_view` frame on top; this is safe because each frame owns
its own `end_state` snapshot and its own `ModalFrame` on the capture stack.

**Sources:** `Program::run` and `exec_view_with_completion` in
`src/app/program.rs`.

> **Turbo Vision heritage:** ports `TGroup::execute` (`tgroup.cpp`). C++ had
> one `execute` per active modal, each with its own `getEvent` loop. tvision-rs
> has one `pump_and_drive` loop shared by all levels; modality is a stack of
> `ModalFrame` capture handlers and `end_state` save/restore frames.

## endModal

A view signals "close this modal and return result `cmd`" by calling
[`ctx.end_modal(cmd)`](../api/tvision-rs/view/struct.Context.html#method.end_modal)
on its [`Context`](../api/tvision-rs/view/struct.Context.html). This **queues**
[`Deferred::EndModal(cmd)`](../api/tvision-rs/view/enum.Deferred.html) rather than
acting immediately, because a view's `handle_event` runs inside the dispatch
borrow and cannot reach the loop-owned `end_state` directly:

```rust
# use tvision_rs as tv;
# use tv::event::Event;
# use tv::view::{View, ViewState, Context, DrawCtx};
# struct OkButton { state: ViewState }
# impl View for OkButton {
#     fn state(&self) -> &ViewState { &self.state }
#     fn state_mut(&mut self) -> &mut ViewState { &mut self.state }
#     fn draw(&mut self, _ctx: &mut DrawCtx) {}
fn handle_event(&mut self, ev: &mut Event, ctx: &mut Context) {
    if let Event::Command(cmd) = ev {
        if *cmd == tv::command::Command::OK {
            ctx.end_modal(tv::command::Command::OK);
            ev.clear();
        }
    }
}
# }
```

The deferred drain — which runs every pump pass after dispatch — picks up the
`EndModal` effect and writes `end_state = Some(cmd)`. On the very next
iteration of `exec_view`'s `while end_state.is_none()` loop, the condition
becomes false and the loop exits.

From **top-level code** (outside a view, holding `&mut Program`) you can call
[`Program::end_modal`](../api/tvision-rs/app/struct.Program.html#method.end_modal)
directly — it sets `end_state` without the deferred queue, which is useful in
tests where you want to terminate a headless modal after pre-queuing events.

Rule of thumb: view code → `ctx.end_modal`; program-level code → `Program::end_modal`.

**Sources:** `Context::end_modal` (queues `Deferred::EndModal`) in
`src/view/context.rs`; `Deferred::EndModal` application in `src/app/program.rs`.

> **Turbo Vision heritage:** ports `TView::endModal` (`tview.cpp`). The C++
> version wrote `endState` directly from inside the nested loop's stack frame;
> in tvision-rs the write is deferred via `Deferred::EndModal` because the view
> cannot reach the loop-owned `end_state` during dispatch.
