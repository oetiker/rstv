//! `snake` — a single custom [`View`] dropped straight onto the desktop (no
//! window, no frame), painting a truecolor background and a 🐍 you steer with
//! the arrow keys or the mouse. It mirrors the "minimal custom view" sketch a
//! magiblot/tvision (C++) newcomer meets first: subclass `TView`, override
//! `draw()` and `handleEvent()`, insert it into the desktop.
//!
//! The C++ this ports, verbatim in spirit:
//!
//! ```cpp
//! class SnakeView : public TView {
//!     TPoint pos;
//!     SnakeView(const TRect &bounds) : TView(bounds) {
//!         growMode = gfGrowHiX | gfGrowHiY;          // track the desktop
//!         options |= ofSelectable;                   // accept focus → events
//!         eventMask |= evKeyDown | evMouseDown | evMouseMove;
//!         pos = {size.x/2, size.y/2};
//!     }
//!     void handleEvent(TEvent &ev) override;         // arrows + mouse move pos
//!     void draw() override {                         // ░ background + 🐍
//!         TColorAttr cSnake {0xcc33ff, 0x33cc33};    // {fg, bg} in RGB
//!         TColorAttr cBgrnd {0xffcc00, 0xccff33};
//!         ...writeLine(...);
//!     }
//! };
//!
//! int main() { SnakeApp app; app.run(); app.shutDown(); }
//! ```
//!
//! Run it:  `cargo run --example snake`
//!   - Arrow keys move the snake (it is two columns wide, so it steps by two
//!     horizontally — exactly like the C++ original).
//!   - Click or drag with the mouse to teleport the snake under the cursor.
//!   - The snake leaves a growing amber **tail** behind it: it lengthens by a
//!     quarter of the distance travelled, and follows the path actually swept
//!     (a mouse teleport draws a continuous line, not a gap). This is an
//!     embellishment beyond the C++ `SnakeView`, which had no tail.
//!   - `Alt-X` (or File → Exit) quits; `F10` enters the menu.
//!
//! The port differences worth knowing, versus the C++:
//!   - There is no inheritance: `SnakeView` *owns* a [`ViewState`] and
//!     implements the [`View`] trait, rather than deriving `TView`.
//!   - The framework hands `handle_event` mouse coordinates already translated
//!     into the view's own local frame, so there is no `makeLocal` call — the
//!     C++ `makeLocal(event.mouse.where)` collapses to just `me.position`.
//!   - Drawing goes through [`DrawCtx`] (which clips and translates for you)
//!     instead of filling a `TDrawBuffer` and calling `writeLine`.

use std::io;

use tvision_rs::{
    Backend, Color, Command, CrosstermBackend, Desktop, Event, Key, KeyEvent, Menu, MenuBar, Point,
    Program, Rect, StatusDef, StatusLine, Style, SystemClock, Theme, View, ViewState, alt,
};

// ---------------------------------------------------------------------------
// SnakeView : public TView
// ---------------------------------------------------------------------------

/// The whole game: one view that remembers where the snake is and repaints
/// itself each time that changes.
struct SnakeView {
    /// In C++ a `TView` base subobject; here an owned `ViewState` the trait
    /// methods hand back via `state()` / `state_mut()`.
    st: ViewState,
    /// The snake's position (the head's lead column), in local coordinates.
    pos: Point,
    /// Every cell the head has swept, oldest → newest; the last entry is the
    /// head. A mouse teleport lays down a continuous line (see [`bresenham`]),
    /// so the body never has gaps. Trimmed each move to `tail_len + 1` cells.
    path: Vec<Point>,
    /// Cumulative cells travelled. The tail grows by a quarter of this:
    /// `tail_len = distance / 4`.
    distance: i32,
}

impl SnakeView {
    /// `SnakeView::SnakeView(const TRect &bounds)`.
    fn new(bounds: Rect) -> Self {
        let mut st = ViewState::new(bounds);

        // growMode = gfGrowHiX | gfGrowHiY — the bottom-right corner tracks the
        // desktop, so the snake's playing field follows a terminal resize.
        st.grow_mode.hi_x = true;
        st.grow_mode.hi_y = true;

        // options |= ofSelectable — otherwise it never gets focus, hence never
        // sees a key or mouse event.
        st.options.selectable = true;

        // eventMask |= evMouseMove — MouseMove and MouseAuto are opt-in (every
        // other event, including evKeyDown and evMouseDown, is always delivered).
        st.event_mask.mouse_move = true;

        // Start the snake at the centre of the field.
        let size = st.size;
        let pos = Point::new(size.x / 2, size.y / 2);
        SnakeView {
            st,
            pos,
            path: vec![pos],
            distance: 0,
        }
    }

    /// Move the head to `target`, laying down body cells along the line swept
    /// from the old position (Bresenham, so a mouse teleport stays continuous).
    /// Each stepped cell adds 1 to `distance`; the path is then trimmed to the
    /// current tail length (`distance / 4`) plus the head.
    fn advance_to(&mut self, target: Point) {
        if target.x == self.pos.x && target.y == self.pos.y {
            return;
        }
        // `bresenham` includes both endpoints; skip the first (the current head
        // cell, already the last entry in `path`).
        for cell in bresenham(self.pos, target).into_iter().skip(1) {
            self.path.push(cell);
            self.distance += 1;
        }
        self.pos = target;

        // The tail grows by a quarter of the distance travelled.
        let keep = (self.distance / 4) as usize + 1; // body cells + the head
        if self.path.len() > keep {
            self.path.drain(0..self.path.len() - keep);
        }
    }
}

/// Bresenham's line from `a` to `b`, inclusive of both endpoints.
fn bresenham(a: Point, b: Point) -> Vec<Point> {
    let (mut x, mut y) = (a.x, a.y);
    let dx = (b.x - x).abs();
    let dy = -(b.y - y).abs();
    let sx = if a.x < b.x { 1 } else { -1 };
    let sy = if a.y < b.y { 1 } else { -1 };
    let mut err = dx + dy;
    let mut pts = Vec::new();
    loop {
        pts.push(Point::new(x, y));
        if x == b.x && y == b.y {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
    pts
}

impl View for SnakeView {
    fn state(&self) -> &ViewState {
        &self.st
    }

    fn state_mut(&mut self) -> &mut ViewState {
        &mut self.st
    }

    /// `void SnakeView::draw()` — fill the background with ░ light-shade in one
    /// truecolor pair, then stamp the 🐍 in another. `{fg, bg}` are spelled out
    /// as literal 24-bit RGB, the whole reason this example exists.
    fn draw(&mut self, ctx: &mut tvision_rs::DrawCtx) {
        // {fg, bg} as literal 24-bit RGB. The C++ original uses
        //   cSnake {0xcc33ff, 0x33cc33}; cBgrnd {0xffcc00, 0xccff33};
        // but 🐍 is a *color emoji* — terminals paint it in their own palette and
        // ignore the cell's fg, so only the snake's background distinguishes it
        // from the field. The original's green-on-green-on-green leaves the snake
        // barely legible, so we pick higher-contrast values: a calm dark field of
        // dim dots, and a solid bright amber pad under the snake that makes its
        // two-cell length read crisply.
        let c_snake = Style::new(Color::Rgb(0x10, 0x18, 0x10), Color::Rgb(0xff, 0xd0, 0x40));
        let c_bgrnd = Style::new(Color::Rgb(0x33, 0x3a, 0x55), Color::Rgb(0x14, 0x16, 0x22));

        let extent = self.st.get_extent();
        // Draw the background (C++: moveStr ░ across a row, then writeLine every
        // row). DrawCtx::fill does the whole rectangle in one call.
        ctx.fill(extent, '░', c_bgrnd);

        // Draw the body: every recorded path cell except the last one (the
        // head). Each is a blank on the snake's amber background, so the tail
        // reads as a continuous amber trail flowing out of the head.
        if let Some((_head, body)) = self.path.split_last() {
            for seg in body {
                if extent.contains(*seg) {
                    ctx.put_char(seg.x, seg.y, ' ', c_snake);
                }
            }
        }

        // Draw the snake head (two columns wide), over the newest body cell.
        if extent.contains(self.pos) {
            ctx.put_str(self.pos.x, self.pos.y, "🐍", c_snake);
        }
    }

    /// `void SnakeView::handleEvent(TEvent &event)` — arrows nudge the snake,
    /// mouse press/drag teleports it. Consuming the event (`ev.clear()`) lets
    /// the framework know it was handled and triggers the repaint.
    fn handle_event(&mut self, ev: &mut Event, _ctx: &mut tvision_rs::Context) {
        let size = self.st.size;
        let mut target = self.pos;
        match ev {
            Event::KeyDown(ke) => {
                match ke.key {
                    Key::Up => target.y = (self.pos.y - 1).max(0),
                    Key::Down => target.y = (self.pos.y + 1).min(size.y - 1),
                    // The snake emoji is two columns wide, so it moves by two
                    // horizontal positions, matching the C++ original.
                    Key::Left => target.x = (self.pos.x - 2).max(0),
                    Key::Right => target.x = (self.pos.x + 2).min(size.x - 2),
                    // Leave any other key untouched (do NOT clear it).
                    _ => return,
                }
                ev.clear();
            }
            // Mouse press and drag also move the snake. The position is already
            // local to this view (the framework translated it), so there is no
            // makeLocal step.
            Event::MouseDown(me) | Event::MouseMove(me) => {
                target = me.position;
                ev.clear();
            }
            _ => return,
        }
        // Lay down the body along the swept line and grow the tail.
        self.advance_to(target);
    }
}

// ---------------------------------------------------------------------------
// SnakeApp : public TApplication
// ---------------------------------------------------------------------------

struct SnakeApp {
    program: Program,
}

impl SnakeApp {
    /// `SnakeApp::SnakeApp` → `TProgInit(initStatusLine, initMenuBar, initDeskTop)`.
    fn new(backend: Box<dyn Backend>) -> Self {
        let program = Program::new(
            backend,
            Box::new(SystemClock::new()),
            Theme::classic_blue(),
            Self::init_desktop,
            Self::init_status_line,
            Self::init_menu_bar,
        );
        SnakeApp { program }
    }

    /// `TApplication::initDeskTop` — inset the desktop a row below the menu and
    /// above the status line, then drop the snake straight onto it. No window,
    /// no frame: `deskTop->insert(snakeView)`.
    fn init_desktop(r: Rect) -> Option<Box<dyn View>> {
        let mut r = r;
        r.a.y += 1; // below the menu bar
        r.b.y -= 1; // above the status line
        let mut desktop = Desktop::new(r, |br| Some(Desktop::init_background(br)));

        // C++: TRect r = deskTop->getExtent().grow(-2, -1);
        let mut field = desktop.state().get_extent();
        field.grow(-2, -1);
        desktop.insert_view(Box::new(SnakeView::new(field)));

        Some(Box::new(desktop))
    }

    /// `TApplication::initStatusLine` — pin to the bottom row: `Alt-X Exit` plus
    /// a hidden `F10 Menu` binding.
    fn init_status_line(r: Rect) -> Option<Box<dyn View>> {
        let mut r = r;
        r.a.y = r.b.y - 1;
        let defs = StatusDef::list()
            .def_all(|d| {
                d.item("~Alt-X~ Exit", alt('x'), Command::QUIT)
                    .key_item(KeyEvent::from(Key::F(10)), Command::MENU)
            })
            .build();
        Some(Box::new(StatusLine::new(r, defs)))
    }

    /// `TApplication::initMenuBar` — pin to the top row: a File menu whose only
    /// item is Exit.
    fn init_menu_bar(r: Rect) -> Option<Box<dyn View>> {
        let mut r = r;
        r.b.y = r.a.y + 1;
        let menu = Menu::builder()
            .submenu("~F~ile", alt('f'), |m| {
                m.command_key("E~x~it", Command::QUIT, alt('x'), "Alt-X")
            })
            .build();
        Some(Box::new(MenuBar::new(r, menu)))
    }

    /// `app.run()` — spin the real event loop until `cmQuit`.
    fn run(&mut self) -> Command {
        self.program.run_app(|_prog, _cmd| {})
    }
}

// ---------------------------------------------------------------------------
// int main()
// ---------------------------------------------------------------------------

fn main() -> io::Result<()> {
    let mut app = SnakeApp::new(Box::new(CrosstermBackend::new()?));
    let _result: Command = app.run();
    Ok(())
}
