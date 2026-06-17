# Design note — command enablement: allowlist → denylist (backlog A1)

> Status: **LANDED** (branch `a1-commandset`). Flips `Program`'s command-enable
> storage from an enumerated *enabled* set to the faithful *disabled* set, and
> adds the `Context::command_enabled` snapshot query (unblocks backlog B1,
> button/inputline self-graying).

## Baseline (C++)

`TView::curCommandSet` is a 256-bit array seeded by `initCommands()`
(`tview.cpp`): **enable all 256 bits, then disable five** — `cmZoom`,
`cmClose`, `cmResize`, `cmNext`, `cmPrev` (a window grants them on selection).

```cpp
Boolean TView::commandEnabled( ushort command ) noexcept
{ return Boolean((command > 255) || curCommandSet.has(command)); }
```

So the C++ model is **enabled-by-default**, with two components:

1. every command in the trackable 0..255 range starts enabled except the five
   window commands;
2. commands `> 255` are *always* enabled and *never maskable* — not a design
   feature, just a capacity artifact of the 256-bit array (`stddlg.h`'s
   `cmFileOpen = 1001` etc. live out there deliberately, to be filter-proof).

`enableCommand`/`disableCommand` set `commandSetChanged` only on a **real
transition** (`has`-guarded); `TProgram::idle` then broadcasts
`cmCommandSetChanged` once and clears the flag. The set variants use
intersection: `disableCommands(s)` flags if `(cur & s)` is nonempty,
`enableCommands(s)` if `(cur & s) != s`.

## The old tvision-rs mistake (the allowlist)

D1 made commands open strings, so "all 256 bits on" had no direct translation,
and the original port enumerated an *enabled* allowlist (~35 framework
commands) in `default_command_set()`. That inverted the C++ model's default:
any command **not** on the list — including every app-minted
`Command::custom` and the file-dialog result commands (`cmFileOpen`, …, the
C++ `> 255` family) — was silently dropped by the pump's boundary filter.
Symptom: the FileDialog's **"OK does nothing"** bug (the `cmFileOpen` result
never reached the dialog, the modal never ended), bandaided by appending the
six file-dialog commands to the allowlist. Every new command was a latent
repeat of that bug.

## Deviation (the denylist)

`Program` stores `curCommandSet` as its **complement** — `disabled_commands:
CommandSet`:

- `command_enabled(cmd) == !disabled_commands.has(cmd)`;
- startup seed = exactly `{ZOOM, CLOSE, RESIZE, NEXT, PREV}`
  (`initial_disabled_commands`, the faithful `initCommands` image);
- `enable_command` removes / `disable_command` inserts, each flipping
  `command_set_changed` only on a real membership transition (the C++
  `has`-guards, mirrored in the pump's `Deferred::EnableCommand`/
  `DisableCommand` apply arms);
- the pump's `Event::Command` boundary filter drops a command **iff**
  `disabled_commands.has(cmd)`.

**The `> 255` rule is subsumed, not ported.** Over an open string space every
command is enabled by default *and* maskable; the C++ "never maskable above
255" half was a capacity artifact, so dropping it is strictly more capable
while reproducing all observable C++ behavior (nothing in the ported code
disables an out-of-range command, because in C++ it couldn't).

`CommandSet` itself stays a polarity-neutral `HashSet<Command>` wrapper (D1);
only `Program`'s storage interpretation flips.

## Integration

- **Menu/status graying** — `View::update_menu_commands(&CommandSet)` (the §2
  broker) now receives the **disabled set**: a menu item grays iff
  `disabled.has(item.command)` (`menu_view::update_menu_commands`), and
  `StatusLine` caches the disabled set (`disabled_cmds`), reading
  `command_enabled = !cached.has(cmd)`. The `cmCommandSetChanged` plumbing
  (changed flag → idle broadcast → `Deferred::UpdateMenu` → regray) is
  unchanged; only the membership predicate flipped.
- **`Context::command_enabled(cmd)`** — the view-side `TView::commandEnabled`.
  `Context` carries an owned **snapshot** of the disabled set, refreshed by the
  pump once per `pump_once` (`set_disabled_commands`; a clone — the set holds
  ≤ a dozen entries). Owned, not `&CommandSet`, because the deferred-apply
  `Context` is alive while the `EnableCommand`/`DisableCommand` arms mutate the
  live set `&mut` (the aliasing rationale on `Deferred::UpdateMenu`). Snapshot
  semantics: an enable/disable deferred in pump N is visible to the query in
  pump N+1. Contexts built outside the pump default to the empty set = all
  enabled.
- **Modal save/restore** (`exec_view`) — `getCommands`/`setCommands` clone and
  restore `disabled_commands` unchanged in shape; the pre-existing deviation
  (no `commandSetChanged` on restore) is untouched.
- **Apps need no registration** — `examples/hello.rs` dropped its
  allowlist-era `enable_command(CMD_*)` calls; a custom command works the
  moment it is minted.
