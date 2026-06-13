# C++ Turbo Vision → `tv::` symbol map

A terse lookup for translating a C++ Turbo Vision symbol into its rstv
equivalent (the `rstv` crate, imported as `tv`). This is the *what*; the *why*
lives in [The Idiomatic Port](../port/faithful.md) and the summary in
[Differences from C++ Turbo Vision](deviations.md).

Two mechanical rules cover most of the table:

- **Drop the `T` prefix and namespace under `tv::`** — `TButton` → `tv::Button`.
- **`cmFoo` / `hcFoo` constant families** become associated consts on an open
  newtype — `cmOK` → [`tv::Command::OK`](../api/rstv/command/struct.Command.html),
  `hcNoContext` → [`tv::HelpCtx::NO_CONTEXT`](../api/rstv/help/struct.HelpCtx.html).

## Core types

| C++ Turbo Vision | rstv (Rust) | notes |
| ---------------- | -------------- | ----- |
| `TView` | [`View`](../api/rstv/view/trait.View.html) (trait) + [`ViewState`](../api/rstv/view/struct.ViewState.html) (data) | behaviour is a trait; data is composed in, not inherited |
| `TGroup` | [`Group`](../api/rstv/view/struct.Group.html) | owns `Vec<Box<dyn View>>` |
| `TFrame` | [`Frame`](../api/rstv/frame/struct.Frame.html) | |
| `TWindow` | [`Window`](../api/rstv/window/struct.Window.html) | |
| `TDialog` | [`Dialog`](../api/rstv/dialog/struct.Dialog.html) | |
| `TDeskTop` | [`Desktop`](../api/rstv/desktop/struct.Desktop.html) | |
| `TProgram` | [`Program`](../api/rstv/app/struct.Program.html) | the engine: tree + event loop + backend |
| `TApplication` | [`Application`](../api/rstv/app/struct.Application.html) | thin wrapper over `Program` |

See [The application skeleton](../getting-started/skeleton.md) for the
`Program` / `Application` split.

## Controls & widgets

| C++ Turbo Vision | rstv (Rust) |
| ---------------- | -------------- |
| `TButton` | [`Button`](../api/rstv/widgets/struct.Button.html) |
| `TStaticText` | [`StaticText`](../api/rstv/widgets/struct.StaticText.html) |
| `TParamText` | [`ParamText`](../api/rstv/widgets/struct.ParamText.html) |
| `TLabel` | [`Label`](../api/rstv/widgets/struct.Label.html) |
| `TInputLine` | [`InputLine`](../api/rstv/widgets/struct.InputLine.html) |
| `TCluster` | [`Cluster`](../api/rstv/widgets/struct.Cluster.html) |
| `TCheckBoxes` | [`CheckBoxes`](../api/rstv/widgets/struct.CheckBoxes.html) |
| `TRadioButtons` | [`RadioButtons`](../api/rstv/widgets/struct.RadioButtons.html) |
| `TScrollBar` | [`ScrollBar`](../api/rstv/widgets/struct.ScrollBar.html) |
| `TScroller` | [`Scroller`](../api/rstv/widgets/struct.Scroller.html) |
| `TListViewer` | [`ListViewer`](../api/rstv/widgets/list_viewer/trait.ListViewer.html) (trait) |
| `TListBox` | [`ListBox`](../api/rstv/widgets/struct.ListBox.html) |
| `TOutline` | [`Outline`](../api/rstv/widgets/outline/struct.Outline.html) / [`OutlineViewer`](../api/rstv/widgets/outline/trait.OutlineViewer.html) |
| `TEditor` | [`Editor`](../api/rstv/widgets/struct.Editor.html) |
| `TEditWindow` | [`EditWindow`](../api/rstv/widgets/struct.EditWindow.html) |
| `TMenuBar` | [`MenuBar`](../api/rstv/menu/menu_bar/struct.MenuBar.html) |
| `TMenu` / `TMenuItem` | [`Menu`](../api/rstv/menu/struct.Menu.html) / [`MenuItem`](../api/rstv/menu/enum.MenuItem.html) |
| `TStatusLine` / `TStatusItem` | [`StatusLine`](../api/rstv/status/status_line/struct.StatusLine.html) / [`StatusItem`](../api/rstv/status/struct.StatusItem.html) |
| `TValidator` family | [`Validator`](../api/rstv/validate/trait.Validator.html) trait + impls |

The full set is in [Controls](../apps/controls.md) and
[Menus, status line & help](../apps/menus.md).

## Events, keys & commands

| C++ Turbo Vision | rstv (Rust) | notes |
| ---------------- | -------------- | ----- |
| `TEvent` / `event.what == evX` | [`Event`](../api/rstv/event/enum.Event.html) enum, matched | `evKeyDown` → `Event::KeyDown(..)`, `evCommand` → `Event::Command(..)` |
| `KeyDownEvent` | [`KeyEvent`](../api/rstv/event/struct.KeyEvent.html) | |
| `MouseEventType` | [`MouseEvent`](../api/rstv/event/struct.MouseEvent.html) | |
| `kbEnter`, `kbF1`, … | [`Key`](../api/rstv/event/enum.Key.html) enum (`Key::Enter`, `Key::F(1)`) | combined codes decompose |
| `kbCtrlC`, `kbShiftTab` | base `Key` + [`KeyModifiers`](../api/rstv/event/struct.KeyModifiers.html) | `Key::Char('c')` + ctrl |
| `clearEvent(event)` | `*ev = Event::Nothing` | |
| `cmOK`, `cmCancel`, … | [`Command`](../api/rstv/command/struct.Command.html) assoc consts | open newtype, namespaced |
| `TCommandSet` (256-bit) | [`CommandSet`](../api/rstv/command/struct.CommandSet.html) | `Program` stores the *disabled* set (denylist) |
| `message(rcvr, evBroadcast, cmX, p)` | `ctx.broadcast(Command::X)` | |
| `message(...)` expecting a result | targeted query → `Option<T>` | |

How events route is covered in [Events → enum + match](../port/events.md) and
[Commands & events](../apps/commands.md).

## State, options & layout flags

The `ushort` flag words become named booleans, reached through the
[`Context`](../api/rstv/view/struct.Context.html) and a view's `ViewState`.

| C++ Turbo Vision | rstv (Rust) |
| ---------------- | -------------- |
| `state & sfFocused` | `self.state().focused` / [`StateFlag::Focused`](../api/rstv/view/enum.StateFlag.html) |
| `options & ofSelectable` | `self.state().options.selectable` ([`Options`](../api/rstv/view/struct.Options.html)) |
| `growMode` / `dragMode` | [`GrowMode`](../api/rstv/view/struct.GrowMode.html) / [`DragMode`](../api/rstv/view/struct.DragMode.html) |
| `helpCtx` / `hcNoContext` | `ViewState.help_ctx` / [`HelpCtx::NO_CONTEXT`](../api/rstv/help/struct.HelpCtx.html) |
| `owner` / `current` / `selected` | [`ViewId`](../api/rstv/view/struct.ViewId.html) handles |

The flag-word translation is detailed in
[Flag words → struct-of-bools](../port/flags.md), the handle model in
[Pointers & infoPtr → handles](../port/handles.md).

## Color, drawing & backend

| C++ Turbo Vision | rstv (Rust) | notes |
| ---------------- | -------------- | ----- |
| `getColor` / `getPalette` | `ctx.theme.style(Role::…)` | [`Role`](../api/rstv/theme/enum.Role.html) / [`Theme`](../api/rstv/theme/struct.Theme.html) |
| `TColorAttr` | [`Style`](../api/rstv/color/struct.Style.html) | |
| `TColorDesired` | [`Color`](../api/rstv/color/enum.Color.html) | 4-variant enum |
| hardcoded glyph tables | fields on `theme::Glyphs`, via `ctx.glyphs()` | |
| `TDrawBuffer` | [`DrawBuffer`](../api/rstv/screen/struct.DrawBuffer.html) | |
| `THardwareInfo` / `TScreen` | [`Backend`](../api/rstv/backend/trait.Backend.html) trait | [`CrosstermBackend`](../api/rstv/backend/struct.CrosstermBackend.html) / [`HeadlessBackend`](../api/rstv/backend/struct.HeadlessBackend.html) |

See [Palettes & glyphs → Theme/Role](../port/theme.md),
[The draw model](../port/draw.md), and [Drawing & backends](../internals/drawing.md).

## Modal flow & data

| C++ Turbo Vision | rstv (Rust) | notes |
| ---------------- | -------------- | ----- |
| `execView` | `exec_view` | result returned via a posted `Command` |
| `dragView` / press-tracking | capture-stack handlers | see [The event loop](../internals/event-loop.md) |
| `getData` / `setData` / `dataSize` | typed `value` / `set_value` | currency is [`FieldValue`](../api/rstv/data/enum.FieldValue.html) |

The modal model is in [Modal execView → one loop + capture](../port/modal.md),
the data protocol in [Dialogs & data](../apps/dialogs.md).

## Dropped or replaced

| C++ Turbo Vision | rstv (Rust) |
| ---------------- | -------------- |
| `drawHide` / `drawShow` / `drawUnder*` / buffered group | dropped — whole-tree redraw + diff |
| `TStreamable` / `TResourceFile` | dropped (serde if revived) |
| `forEach` / `firstThat` / `TSortedCollection` | iterators / `Vec<T: Ord>` |

Rationale for each removal is in
[Dropped & changed](../port/dropped.md).

---

> Anything not in this table ports verbatim — same name (minus the `T`), same
> method, same behaviour. For the differences that *do* change a symbol, see
> [Differences from C++ Turbo Vision](deviations.md).
