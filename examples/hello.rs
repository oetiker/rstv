//! `hello` — a minimal Turbo Vision application, written in the shape a
//! magiblot/tvision (C++) veteran will recognise on sight.
//!
//! The C++ skeleton this mirrors:
//!
//! ```cpp
//! class TAboutDialog : public TDialog {           // a custom dialog subclass
//! public:
//!     TAboutDialog(TRect);
//!     virtual void draw();                         // custom interior painting
//! };
//!
//! class HelloApp : public TApplication {
//!     static TDeskTop    *initDeskTop(TRect);
//!     static TStatusLine *initStatusLine(TRect);   // Phase 4 (stubbed below)
//!     static TMenuBar    *initMenuBar(TRect);      // Phase 4 (stubbed below)
//! };
//!
//! int main() {
//!     HelloApp app;
//!     app.run();                                   // opens dialogs from a menu
//! }
//! ```
//!
//! What is not yet ported (menus, status line, buttons, static text) is stubbed
//! and flagged. A finished app would spin `app.run()` and open this dialog from a
//! menu command; until menus land we open it directly with the *same* modal
//! primitive a menu would use — `execView` (`Program::exec_view`).
//!
//! Run it:  `cargo run --example hello`  (Esc or the close box to quit).

use std::io;

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::Signals;

use tvision::{
    Backend, Color, Command, Context, CrosstermBackend, Desktop, Dialog, DrawCtx, Event, Point,
    Program, Rect, StateFlag, Style, SystemClock, Theme, View, ViewId, ViewState,
};

// ---------------------------------------------------------------------------
// TAboutDialog : public TDialog
// ---------------------------------------------------------------------------

/// A dialog with custom interior drawing — the canonical TV "About box" shape.
///
/// It *is-a* [`Dialog`] (embed-and-delegate, the port's stand-in for C++
/// inheritance): every [`View`] method forwards to the inner dialog, except
/// [`draw`](View::draw), which first lets the dialog paint its frame/title/close
/// box, then fills the interior and overlays centred text.
struct AboutDialog {
    dialog: Dialog,
}

impl AboutDialog {
    const W: i32 = 46;
    const H: i32 = 14;

    /// `TAboutDialog::TAboutDialog` — build the dialog, centred in the desktop.
    ///
    /// C++ would set `options |= ofCentered` and let `insertView` centre it;
    /// ofCentered insertion is a later row, so we compute the centred bounds here.
    fn new(desktop_w: i32, desktop_h: i32) -> Self {
        let x = (desktop_w - Self::W) / 2;
        let y = (desktop_h - Self::H) / 2;
        let bounds = Rect::new(x, y, x + Self::W, y + Self::H);
        AboutDialog {
            dialog: Dialog::new(bounds, Some("About".to_string())),
        }
    }
}

impl View for AboutDialog {
    fn state(&self) -> &ViewState {
        self.dialog.state()
    }

    fn state_mut(&mut self) -> &mut ViewState {
        self.dialog.state_mut()
    }

    /// `TAboutDialog::draw` — `TDialog::draw()` first (frame, border, title, close
    /// box), then the interior fill + centred text. The `DrawCtx` is dialog-local
    /// (origin at the dialog's top-left), so coordinates are `0..W` / `0..H`.
    fn draw(&mut self, ctx: &mut DrawCtx) {
        self.dialog.draw(ctx);

        // Classic TV dialog body: black on light-gray. (Real gray-scheme theming
        // is a deferred row; we pick the colours directly here.)
        let body = Style::new(Color::Bios(0x0), Color::Bios(0x7));
        ctx.fill(Rect::new(1, 1, Self::W - 1, Self::H - 1), ' ', body);

        let lines = [
            "Turbo Vision for Rust",
            "an idiomatic port of magiblot/tvision",
            "",
            "Phase 2 — you are looking at a real",
            "desktop, window, dialog, frame and the",
            "modal event loop, all ported from C++.",
            "",
            "Drag the title bar to move me.",
            "Press Esc or click [\u{25A0}] to close.",
        ];
        for (i, line) in lines.iter().enumerate() {
            let len = line.chars().count() as i32;
            let x = 1 + (Self::W - 2 - len) / 2;
            ctx.put_str(x, 2 + i as i32, line, body);
        }
    }

    // -- everything else forwards to the inner dialog -----------------------

    fn handle_event(&mut self, ev: &mut Event, ctx: &mut Context) {
        self.dialog.handle_event(ev, ctx);
    }

    fn valid(&self, cmd: Command) -> bool {
        self.dialog.valid(cmd)
    }

    fn set_state(&mut self, flag: StateFlag, enable: bool, ctx: &mut Context) {
        self.dialog.set_state(flag, enable, ctx);
    }

    fn awaken(&mut self) {
        self.dialog.awaken();
    }

    fn size_limits(&self, owner_size: Point) -> (Point, Point) {
        self.dialog.size_limits(owner_size)
    }

    fn change_bounds(&mut self, bounds: Rect) {
        self.dialog.change_bounds(bounds);
    }

    fn cursor_request(&self) -> Option<Point> {
        self.dialog.cursor_request()
    }

    fn find_mut(&mut self, id: ViewId) -> Option<&mut dyn View> {
        self.dialog.find_mut(id)
    }

    fn remove_descendant(&mut self, id: ViewId, ctx: &mut Context) -> bool {
        self.dialog.remove_descendant(id, ctx)
    }

    fn number(&self) -> Option<i16> {
        self.dialog.number()
    }
}

// ---------------------------------------------------------------------------
// HelloApp : public TApplication
// ---------------------------------------------------------------------------

/// The application class. In C++ this would derive `TApplication` and override
/// the three `init*` factories; here it wraps the [`Program`] it builds from
/// them via [`Program::new`] (the port's `TProgInit` factory-mixin seam).
struct HelloApp {
    program: Program,
    /// The full screen extent, kept so `run` can centre the About box (C++ reads
    /// `deskTop->size`; we have no public desktop-size getter yet).
    extent: (i32, i32),
}

impl HelloApp {
    /// `HelloApp::HelloApp` → `TProgInit(initStatusLine, initMenuBar, initDeskTop)`.
    fn new(backend: Box<dyn Backend>) -> Self {
        let (w, h) = backend.size();
        let program = Program::new(
            backend,
            Box::new(SystemClock::new()),
            Theme::classic_blue(),
            Self::init_desktop,
            Self::init_status_line,
            Self::init_menu_bar,
        );
        HelloApp {
            program,
            extent: (w as i32, h as i32),
        }
    }

    /// `TApplication::initDeskTop` — the desktop with the default patterned
    /// background (`TDeskTop::initBackground`).
    fn init_desktop(r: Rect) -> Option<Box<dyn View>> {
        Some(Box::new(Desktop::new(r, |br| {
            Some(Desktop::init_background(br))
        })))
    }

    /// `TApplication::initStatusLine` — Phase 4 (no status line yet).
    fn init_status_line(_r: Rect) -> Option<Box<dyn View>> {
        None
    }

    /// `TApplication::initMenuBar` — Phase 4 (no menu bar yet).
    fn init_menu_bar(_r: Rect) -> Option<Box<dyn View>> {
        None
    }

    /// `TApplication::run` (the demo's flavour). A finished app would spin
    /// `program.run()` and open dialogs from a menu via `execView`; until menus
    /// land we open the About box directly with that same modal primitive. It
    /// blocks until the dialog ends (Esc / close box → `cmCancel`) and returns the
    /// end command.
    fn run(&mut self) -> Command {
        let (w, h) = self.extent;
        let about = AboutDialog::new(w, h);
        self.program.exec_view(Box::new(about))
    }
}

// ---------------------------------------------------------------------------
// Terminal setup (deferred out of CrosstermBackend until a later row)
// ---------------------------------------------------------------------------

/// Undo the terminal setup. Idempotent and safe to call more than once (Drop +
/// the signal thread may both run).
fn restore_terminal() {
    let _ = execute!(io::stdout(), DisableMouseCapture, LeaveAlternateScreen);
    let _ = disable_raw_mode();
}

/// RAII terminal guard: raw mode + alternate screen + mouse capture on entry,
/// restored on `Drop` — so a panic unwinding through `exec_view` still restores
/// the terminal. It also installs a signal thread so a `kill` (SIGTERM), a hangup
/// (SIGHUP), or SIGINT restores the terminal before exiting — without it the
/// shell is left in raw mode on the alternate screen. (SIGKILL is uncatchable; a
/// `kill -9` will still leave the terminal dirty — run `reset` to recover.)
struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;

        // Restore on fatal signals. We handle them on a dedicated thread (not in
        // an async-signal context), so calling into crossterm is sound. On the
        // first such signal we restore and exit (130 = 128 + SIGINT, the usual
        // shell convention); Drop does not run on `process::exit`, but we have
        // already restored.
        let mut signals = Signals::new([SIGINT, SIGTERM, SIGHUP])?;
        std::thread::spawn(move || {
            if signals.forever().next().is_some() {
                restore_terminal();
                std::process::exit(130);
            }
        });

        Ok(TerminalGuard)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

// ---------------------------------------------------------------------------
// int main()
// ---------------------------------------------------------------------------

fn main() -> io::Result<()> {
    let _guard = TerminalGuard::enter()?;

    let mut app = HelloApp::new(Box::new(CrosstermBackend::new()));
    let _result: Command = app.run();

    Ok(())
}
