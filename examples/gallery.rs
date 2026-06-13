//! `gallery` — renders a single widget per run, for the documentation's widget
//! gallery. Each `// ANCHOR: <name>`-marked builder below is included verbatim
//! into the guide, so every documented widget snippet is real, compiling code,
//! and the same builder drives the captured screenshot.
//!
//! Usage:
//! ```text
//! cargo run --example gallery -- <name>   # show one widget
//! cargo run --example gallery             # list the available names
//! ```
//!
//! Adding a widget = write a builder, add a `specimen()` arm + a `NAMES` entry,
//! and register a `Screen` in `xtask/src/screens.rs`.

use std::{env, io};

use tvision::{
    Button, ButtonFlags, Command, CrosstermBackend, Desktop, Dialog, Key, KeyEvent, Menu, MenuBar,
    Program, Rect, StatusDef, StatusLine, SystemClock, Theme, View, alt,
};

/// How a specimen is shown. Most widgets are leaf controls hosted in a dialog on
/// the desktop; the `Menu` / `Status` variants replace the corresponding chrome
/// so those two can be showcased in place.
#[derive(Clone, Copy)]
enum Specimen {
    /// A view (a dialog or a window) placed on the desktop.
    OnDesktop(fn() -> Box<dyn View>),
    /// A rich menu bar (the screenshot opens it with a keystroke).
    Menu(fn() -> Menu),
    /// A rich status line.
    Status(fn() -> Vec<StatusDef>),
}

/// Map a CLI name to its specimen. Keep `NAMES` in sync.
fn specimen(name: &str) -> Option<Specimen> {
    use Specimen::*;
    Some(match name {
        "button" => OnDesktop(button),
        "menubar" => Menu(menubar),
        "statusline" => Status(statusline),
        _ => return None,
    })
}

/// Every registered widget name (for the no-arg listing and the xtask registry).
const NAMES: &[&str] = &["button", "menubar", "statusline"];

// ===========================================================================
// Specimens — one `// ANCHOR: <name>` builder per widget.
// ===========================================================================

// ANCHOR: button
/// A dialog with a default `OK` button and a `Cancel` button. The `~` marks the
/// hot-letter; `default: true` makes `Enter` press `OK`.
fn button() -> Box<dyn View> {
    let mut dlg = Dialog::new(Rect::new(2, 1, 36, 9), Some("Buttons".to_string()));
    dlg.insert_child(Box::new(Button::new(
        Rect::new(4, 4, 16, 6),
        "~O~K",
        Command::OK,
        ButtonFlags {
            default: true,
            ..Default::default()
        },
    )));
    dlg.insert_child(Box::new(Button::new(
        Rect::new(19, 4, 31, 6),
        "~C~ancel",
        Command::CANCEL,
        ButtonFlags::default(),
    )));
    Box::new(dlg)
}
// ANCHOR_END: button

// ANCHOR: menubar
/// A menu bar with `File`, `Edit`, and `Window` pull-downs. Each `~`-marked
/// letter is the hot-key; `command_key` adds the accelerator shown at the right.
fn menubar() -> Menu {
    Menu::builder()
        .submenu("~F~ile", alt('f'), |m| {
            m.command_key(
                "~O~pen…",
                Command::custom("gallery.open"),
                KeyEvent::from(Key::F(3)),
                "F3",
            )
            .command_key(
                "~N~ew",
                Command::custom("gallery.new"),
                KeyEvent::from(Key::F(4)),
                "F4",
            )
            .separator()
            .command_key("E~x~it", Command::QUIT, alt('x'), "Alt-X")
        })
        .submenu("~E~dit", alt('e'), |m| {
            m.command("Cu~t~", Command::CUT)
                .command("~C~opy", Command::COPY)
                .command("~P~aste", Command::PASTE)
        })
        .submenu("~W~indow", alt('w'), |m| {
            m.command("~T~ile", Command::TILE)
                .command("C~a~scade", Command::CASCADE)
        })
        .build()
}
// ANCHOR_END: menubar

// ANCHOR: statusline
/// A status line of labelled hot-key items. Clicking a label or pressing its key
/// fires the command; `~`-marked text is highlighted.
fn statusline() -> Vec<StatusDef> {
    StatusDef::list()
        .def_all(|d| {
            d.item("~F2~ Save", KeyEvent::from(Key::F(2)), Command::SAVE)
                .item(
                    "~F3~ Open",
                    KeyEvent::from(Key::F(3)),
                    Command::custom("gallery.open"),
                )
                .item("~F10~ Menu", KeyEvent::from(Key::F(10)), Command::MENU)
                .item("~Alt-X~ Exit", alt('x'), Command::QUIT)
        })
        .build()
}
// ANCHOR_END: statusline

// ===========================================================================
// Default chrome (used whenever a specimen does not replace it).
// ===========================================================================

/// A representative File / Edit / Window menu bar.
fn default_menu() -> Menu {
    Menu::builder()
        .submenu("~F~ile", alt('f'), |m| {
            m.command_key(
                "~O~pen…",
                Command::custom("gallery.open"),
                KeyEvent::from(Key::F(3)),
                "F3",
            )
            .separator()
            .command_key("E~x~it", Command::QUIT, alt('x'), "Alt-X")
        })
        .submenu("~E~dit", alt('e'), |m| {
            m.command("Cu~t~", Command::CUT)
                .command("~C~opy", Command::COPY)
                .command("~P~aste", Command::PASTE)
        })
        .submenu("~W~indow", alt('w'), |m| {
            m.command("~T~ile", Command::TILE)
                .command("C~a~scade", Command::CASCADE)
        })
        .build()
}

/// A representative status line.
fn default_status() -> Vec<StatusDef> {
    StatusDef::list()
        .def_all(|d| {
            d.item("~F10~ Menu", KeyEvent::from(Key::F(10)), Command::MENU)
                .item("~Alt-X~ Exit", alt('x'), Command::QUIT)
        })
        .build()
}

// ===========================================================================
// Program assembly: the three factories consult the selected specimen.
// ===========================================================================

fn make_desktop(extent: Rect, spec: Specimen) -> Box<dyn View> {
    let mut r = extent;
    r.a.y += 1; // below the menu bar
    r.b.y -= 1; // above the status line
    let mut desktop = Desktop::new(r, |br| Some(Desktop::init_background(br)));
    if let Specimen::OnDesktop(build) = spec {
        desktop.insert_view(build());
    }
    Box::new(desktop)
}

fn make_status(extent: Rect, spec: Specimen) -> Box<dyn View> {
    let mut r = extent;
    r.a.y = r.b.y - 1;
    let defs = match spec {
        Specimen::Status(build) => build(),
        _ => default_status(),
    };
    Box::new(StatusLine::new(r, defs))
}

fn make_menu(extent: Rect, spec: Specimen) -> Box<dyn View> {
    let mut r = extent;
    r.b.y = r.a.y + 1;
    let menu = match spec {
        Specimen::Menu(build) => build(),
        _ => default_menu(),
    };
    Box::new(MenuBar::new(r, menu))
}

fn main() -> io::Result<()> {
    let name = env::args().nth(1).unwrap_or_default();
    let Some(spec) = specimen(&name) else {
        eprintln!("usage: cargo run --example gallery -- <name>");
        eprintln!("widgets: {}", NAMES.join(", "));
        return Ok(());
    };

    let mut program = Program::new(
        Box::new(CrosstermBackend::new()?),
        Box::new(SystemClock::new()),
        Theme::classic_blue(),
        |r| Some(make_desktop(r, spec)),
        |r| Some(make_status(r, spec)),
        |r| Some(make_menu(r, spec)),
    );
    // run() idles after painting; the screenshot tooling captures the static
    // frame and then kills the session (no quit command is ever sent).
    let _ = program.run();
    Ok(())
}
