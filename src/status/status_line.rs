//! `TStatusLine` — the bottom status line (`tstatusl.cpp`). Row 53 (FOUNDATION).
//!
//! A one-row [`View`] at the bottom of the screen presenting the items of the
//! help-context-selected [`StatusDef`]. This row ports the **draw/data slice**:
//! [`draw`](StatusLine::draw) (`drawSelect(0)`, `tstatusl.cpp:62`),
//! [`find_items`](StatusLine::find_items) (`findItems`, `tstatusl.cpp:119`),
//! [`item_mouse_is_in`](StatusLine::item_mouse_is_in) (`tstatusl.cpp:133`), the
//! single-shot mouse-down arm + the `cmCommandSetChanged` broadcast arm of
//! `handleEvent`, and the command-graying broker hook
//! ([`View::update_menu_commands`]).
//!
//! Structured like [`MenuBar`](crate::menu::MenuBar): a plain struct embedding a
//! [`ViewState`] with hand-written `View` methods (**not** a D2 `#[delegate]`
//! embed — it embeds `ViewState`, not a `View`).
//!
//! ## Themes only — NO palettes
//!
//! rstv has no runtime palette / `getPalette` / `getColor` indirection (a locked
//! deviation). Colors resolve **directly from the [`Theme`](crate::theme::Theme)
//! via [`Role`](crate::theme::Role)s** ([`StatusColors`]), exactly like
//! [`MenuColors`](crate::menu::MenuColors). The C++ `cpStatusLine` palette /
//! `getColor` / `getPalette` are **not ported**.
//!
//! ## Deferrals (faithful, breadcrumbed — NOT stubbed)
//!
//! - **`TProgram` integration** — the whole "wire a real status line into
//!   `Program`" step: `TProgram::getEvent` pre-routes `evKeyDown` (always) and
//!   `evMouseDown` (when the mouse is over the status line) to
//!   `statusLine->handleEvent` *before* normal dispatch; `TProgram::idle` calls
//!   `statusLine->update()`. **Out of scope this row** (matches how the menu draw
//!   layer landed before the menu was wired into `Program`).
//!   `TODO(status TProgram integration: getEvent pre-routing of keyDown/mouseDown
//!   + idle()->update())`.
//! - **The keyDown arm of `handleEvent`** — the global accelerator (match
//!   `event.keyDown == item.key_code && commandEnabled` over **all** items incl.
//!   `text == None`, then transform the event into `evCommand` **in place and
//!   return WITHOUT clearing** so it propagates). Deferred to the `Program`-wiring
//!   step because the in-place-transform-and-propagate semantics only make sense
//!   inside `getEvent`'s pre-routing; it must **not** be ported as `ctx.post` +
//!   `clear` (that double-handles).
//!   `TODO(status keyDown global accelerator — lands with TProgram getEvent
//!   pre-routing; transform-in-place, not ctx.post)`.
//! - **`update()` / help-ctx refresh from `TopView`** (`tstatusl.cpp:209`):
//!   `update()` reads the modal top view's `getHelpCtx()` and re-runs
//!   [`find_items`](StatusLine::find_items) + redraw. The `TopView` plumbing is
//!   `Program`-level → lands with the wiring step.
//!   [`find_items`](StatusLine::find_items) itself **is**
//!   ported (call [`set_help_ctx`](StatusLine::set_help_ctx) to drive it
//!   directly). `TODO(status update(): help-ctx refresh from TopView::getHelpCtx
//!   — lands with TProgram wiring)`.
//! - **`drawSelect(selected)` hover highlighting** + the press-and-hold
//!   drag-highlight loop — only `draw` (= `drawSelect(0)`) is needed; the
//!   `Some(item)` hover path is part of the deferred D9 press-and-hold loop.
//!   `TODO(row 31, D9: status-line press-and-hold drag-highlight + drawSelect
//!   hover)`.
//! - **Streaming** (`read`/`write`/`build`/`streamableName`) → **D12 dropped**.
//! - **`disposeItems`/destructor** → moot (owned `Vec`s, RAII via `Drop`).

use crate::color::Style;
use crate::command::CommandSet;
use crate::event::Event;
use crate::help::HelpCtx;
use crate::status::StatusDef;
use crate::theme::Role;
use crate::view::{Context, DrawCtx, Point, Rect, View, ViewState};

/// `cstrlen` — display width of a `~`-marked control string, **ignoring** the `~`
/// markers (not printed columns). A per-module copy mirroring
/// [`menu_bar`](crate::menu::menu_bar)'s, using the same `UnicodeWidthChar`
/// primitive so widths match the rest of the renderer.
fn cstrlen(s: &str) -> i32 {
    s.chars()
        .filter(|&c| c != '~')
        .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(1) as i32)
        .sum()
}

/// The hint separator drawn before the hint text — C++
/// `TStatusLine::hintSeparator = "\xB3 "` (`tvtext1.cpp:109`): a CP437 `\xB3`
/// vertical bar `│` (U+2502) followed by a space. Drawn plain (`moveStr`), not a
/// `~`-cstr.
const HINT_SEPARATOR: &str = "\u{2502} ";

/// The four `(lo, hi)` style pairs a status item is drawn in — the C++
/// `drawSelect` color matrix (`cNormal`/`cSelect`/`cNormDisabled`/`cSelDisabled`,
/// `tstatusl.cpp:72-75`), resolved once per `draw` from the
/// [`Theme`](crate::theme::Theme) via the `Status*` [`Role`]s.
///
/// Analogous to [`MenuColors`](crate::menu::MenuColors) but reads the distinct
/// `Status*` roles — deliberately a separate type (different roles), not a reuse.
///
/// All four pairs are resolved as a unit even though `select` / `sel_disabled`
/// (and the `(_, true)` arms of [`item`](StatusColors::item)) go unread this row:
/// C++ `drawSelect` resolves all four `TAttrPair`s together at the function top,
/// so resolving the full matrix is faithful, not premature. Their consumer is the
/// breadcrumbed `drawSelect(Some)` hover variant (the row-31 press-and-hold
/// deferral) — selecting a hovered item picks `cSelect`/`cSelDisabled`.
#[derive(Clone, Copy)]
pub struct StatusColors {
    /// `cNormal` → `(StatusNormal, StatusShortcut)`.
    pub normal: (Style, Style),
    /// `cSelect` → `(StatusSelect, StatusShortcutSelect)`.
    pub select: (Style, Style),
    /// `cNormDisabled` → `StatusDisabled` for both lo and hi.
    pub norm_disabled: (Style, Style),
    /// `cSelDisabled` → `StatusSelDisabled` for both lo and hi.
    pub sel_disabled: (Style, Style),
}

impl StatusColors {
    /// Resolve the four `Status*` role pairs from the draw context's theme.
    pub fn resolve(ctx: &DrawCtx) -> Self {
        let d = ctx.style(Role::StatusDisabled);
        let sd = ctx.style(Role::StatusSelDisabled);
        StatusColors {
            normal: (
                ctx.style(Role::StatusNormal),
                ctx.style(Role::StatusShortcut),
            ),
            select: (
                ctx.style(Role::StatusSelect),
                ctx.style(Role::StatusShortcutSelect),
            ),
            // Disabled rows: a single style for both lo and hi (no shortcut
            // highlight when greyed) — C++ `cNormDisabled`/`cSelDisabled` lo==hi.
            norm_disabled: (d, d),
            sel_disabled: (sd, sd),
        }
    }

    /// The `(lo, hi)` pair for an item given its `enabled`/`selected` state — the
    /// C++ `drawSelect` matrix (`commandEnabled ? (sel?cSelect:cNormal) :
    /// (sel?cSelDisabled:cNormDisabled)`).
    fn item(&self, enabled: bool, selected: bool) -> (Style, Style) {
        match (enabled, selected) {
            (true, true) => self.select,
            (true, false) => self.normal,
            (false, true) => self.sel_disabled,
            (false, false) => self.norm_disabled,
        }
    }
}

/// `TStatusLine` — the bottom status line. Owns its `defs`, caches the selected
/// def index + a command-set snapshot for graying.
pub struct StatusLine {
    /// The embedded [`ViewState`] (`TView` data members).
    state: ViewState,
    /// The status-line definitions (C++ `TStatusDef* defs`, owned).
    defs: Vec<StatusDef>,
    /// Index into [`defs`](Self::defs) of the currently-selected def (C++ `items`
    /// = `defs[items_def].items`), or `None` if none match the current help
    /// context (C++ `items == 0`). Resolved by [`find_items`](Self::find_items).
    items_def: Option<usize>,
    /// The view's current help context (C++ `helpCtx`).
    help_ctx: HelpCtx,
    /// The hint provider (C++ virtual `hint()`); default returns `None` (C++
    /// `hint()` returns `""`). Overridable via [`set_hint`](Self::set_hint).
    hint: Box<dyn Fn(HelpCtx) -> Option<String>>,
    /// Cached enabled-command snapshot for graying (refreshed by the
    /// [`update_menu_commands`](View::update_menu_commands) broker hook). `None`
    /// before the first refresh means **treat all as enabled** (the same startup
    /// gap menus have). **Not** a per-item `disabled` field — `TStatusItem` has
    /// none; C++ `drawSelect` calls `commandEnabled` live, which we snapshot here.
    cmd_set: Option<CommandSet>,
}

impl StatusLine {
    /// Construct a status line over `bounds` presenting `defs` — ports
    /// `TStatusLine::TStatusLine` (`tstatusl.cpp:31`).
    ///
    /// Faithful: `growMode = gfGrowLoY | gfGrowHiX | gfGrowHiY` (the line sticks
    /// to the bottom and stretches with the screen) and `options |= ofPreProcess`
    /// (it pre-processes accelerators before the focused view). `eventMask |=
    /// evBroadcast` is **moot** — our `Group::handle_event` fans broadcasts to
    /// every child unconditionally (see the identical note in
    /// [`menu_view::handle_event`](crate::menu::menu_view)); no mask is ported.
    pub fn new(bounds: Rect, defs: Vec<StatusDef>) -> Self {
        let mut state = ViewState::new(bounds);
        state.grow_mode.lo_y = true; // gfGrowLoY
        state.grow_mode.hi_x = true; // gfGrowHiX
        state.grow_mode.hi_y = true; // gfGrowHiY
        state.options.pre_process = true; // ofPreProcess
        let mut sl = StatusLine {
            state,
            defs,
            items_def: None,
            help_ctx: HelpCtx::NO_CONTEXT, // C++ helpCtx default
            hint: Box::new(|_| None),      // C++ default hint() -> ""
            cmd_set: None,
        };
        sl.find_items();
        sl
    }

    /// Override the hint provider (the idiomatic port of the C++ `virtual
    /// hint()`). The closure maps the current help context to an optional hint
    /// string.
    pub fn set_hint(&mut self, hint: impl Fn(HelpCtx) -> Option<String> + 'static) {
        self.hint = Box::new(hint);
    }

    /// Builder-style [`set_hint`](Self::set_hint).
    pub fn with_hint(mut self, hint: impl Fn(HelpCtx) -> Option<String> + 'static) -> Self {
        self.set_hint(hint);
        self
    }

    /// `TStatusLine::findItems` (`tstatusl.cpp:119`) — select the first def whose
    /// `range` matches the current help context; if none match, leave
    /// [`items_def`](Self::items_def) `None` (C++ `items = 0`).
    pub fn find_items(&mut self) {
        self.items_def = self
            .defs
            .iter()
            .position(|d| d.range.matches(self.help_ctx));
    }

    /// Set the view's help context and re-run [`find_items`](Self::find_items) —
    /// the unit-testable / future-wiring entry point. The *automatic* `update()`
    /// trigger (reading `TopView::getHelpCtx`) is deferred to the `Program`-wiring
    /// step (see the module docs).
    pub fn set_help_ctx(&mut self, ctx: HelpCtx) {
        self.help_ctx = ctx;
        self.find_items();
    }

    /// The items of the currently-selected def (C++ `items`), or an empty slice if
    /// none is selected (C++ `items == 0`).
    fn items(&self) -> &[crate::status::StatusItem] {
        match self.items_def {
            Some(i) => &self.defs[i].items,
            None => &[],
        }
    }

    /// Whether `command` is enabled, per the cached command-set snapshot. `None`
    /// (before the first broker refresh) means **treat all as enabled** (the
    /// startup gap). Ports the C++ `commandEnabled(T->command)` call, snapshotted.
    fn command_enabled(&self, command: crate::command::Command) -> bool {
        self.cmd_set.as_ref().is_none_or(|cs| cs.has(command))
    }

    /// `TStatusLine::itemMouseIsIn` (`tstatusl.cpp:133`) — the index of the item
    /// whose drawn span `[i, k)` contains the **view-local** `mouse`, or `None`.
    ///
    /// Faithful: `mouse.y != 0 → None`; else walk the selected def's items
    /// accumulating `i` / `k = i + cstrlen + 2` over `text != None` items (a
    /// `text == None` item is **skipped** in the accumulator — it consumes no
    /// width), and return the item whose `[i, k)` contains `mouse.x`.
    fn item_mouse_is_in(&self, mouse: Point) -> Option<usize> {
        if mouse.y != 0 {
            return None;
        }
        let mut i = 0i32;
        for (idx, item) in self.items().iter().enumerate() {
            if let Some(text) = &item.text {
                let k = i + cstrlen(text) + 2;
                if mouse.x >= i && mouse.x < k {
                    return Some(idx);
                }
                i = k;
            }
        }
        None
    }
}

impl View for StatusLine {
    fn state(&self) -> &ViewState {
        &self.state
    }

    fn state_mut(&mut self) -> &mut ViewState {
        &mut self.state
    }

    /// `TStatusLine::draw` → `drawSelect(0)` (`tstatusl.cpp:62-117`). Fill the row,
    /// then lay the selected def's visible items out left-to-right, each with a
    /// leading + trailing space in the per-item colour, then the hint tail.
    ///
    /// `selected` is always `None` here (the `drawSelect(Some)` hover variant is
    /// part of the deferred press-and-hold loop). The C++ `TDrawBuffer` +
    /// `writeLine` becomes direct view-local [`DrawCtx`] writes (D8).
    fn draw(&mut self, ctx: &mut DrawCtx) {
        let colors = StatusColors::resolve(ctx);
        let size = self.state.size;

        // b.moveChar(0, ' ', cNormal, size.x) — fill the whole row with cNormal.lo.
        ctx.fill(Rect::new(0, 0, size.x, 1), ' ', colors.normal.0);

        let mut i = 0i32;
        for item in self.items() {
            // C++: the `i += l + 2` advance is INSIDE `if (text != 0)`, so a
            // text==None item draws nothing AND consumes no width.
            if let Some(text) = &item.text {
                let l = cstrlen(text);
                if i + l < size.x {
                    let enabled = self.command_enabled(item.command);
                    // selected is always None this row (no hover highlighting).
                    let (lo, hi) = colors.item(enabled, false);
                    ctx.put_char(i, 0, ' ', lo);
                    ctx.put_cstr(i + 1, 0, text, lo, hi);
                    ctx.put_char(i + l + 1, 0, ' ', lo);
                }
                i += l + 2;
            }
        }

        // Hint tail: if there is room (C++ `i < size.x - 2`) and the hint provider
        // returns a non-empty string, draw the separator then the clipped hint.
        if i < size.x - 2
            && let Some(text) = (self.hint)(self.help_ctx)
            && !text.is_empty()
        {
            // moveStr(i, hintSeparator, cNormal) — plain, in cNormal.lo.
            ctx.put_str(i, 0, HINT_SEPARATOR, colors.normal.0);
            i += 2;
            // moveStr(i, hintText, cNormal, size.x - i) — clipped to the row.
            // put_str already truncates at the clip right edge (the row), so no
            // explicit width arg is needed.
            ctx.put_str(i, 0, &text, colors.normal.0);
        }
    }

    /// `TStatusLine::handleEvent` (`tstatusl.cpp:154`).
    ///
    /// Ported branches:
    ///
    /// - **`evBroadcast cmCommandSetChanged`** → request the §2 regray broker by
    ///   the view's own id ([`Context::request_update_menu`] — the exact menu
    ///   pattern; reuses
    ///   [`Deferred::UpdateMenu`](crate::view::Deferred::UpdateMenu) + the
    ///   [`update_menu_commands`](View::update_menu_commands) hook). C++ does
    ///   `drawView()` here; dropped under D8 whole-tree redraw.
    /// - **`evMouseDown`** → single-shot: hit-test the item under the (view-local)
    ///   mouse; if it exists and is enabled, post its command. Always clear the
    ///   mouse-down (C++ clears unconditionally after the drag loop).
    ///
    /// The keyDown global-accelerator arm is **deferred** (see the module docs);
    /// the C++ leading `TView::handleEvent(event)` call is a no-op for our
    /// purposes and is not ported (mirroring [`MenuBar`](crate::menu::MenuBar)).
    fn handle_event(&mut self, ev: &mut Event, ctx: &mut Context) {
        match ev {
            // C++ evBroadcast / cmCommandSetChanged: drawView() (dropped under D8).
            // The regray is the §2 broker — a child (D3) cannot read the command
            // set inline, so request UpdateMenu by our own id; the pump calls back
            // through View::update_menu_commands at apply time.
            //
            // NOTE (deviation): C++ sets `eventMask |= evBroadcast` to opt in.
            // Our Group::handle_event fans broadcasts to EVERY child
            // unconditionally, so no mask/gate is ported (same as menu_view).
            Event::Broadcast {
                command: crate::command::Command::COMMAND_SET_CHANGED,
                ..
            } => {
                if let Some(id) = self.state.id() {
                    ctx.request_update_menu(id);
                }
            }
            // C++ evMouseDown (tstatusl.cpp:160). The C++ runs a press-and-hold
            // `do { drawSelect(itemMouseIsIn) } while(mouseEvent(evMouseMove))`
            // drag-highlight loop, then on release fires the item under the mouse
            // if commandEnabled, then clears unconditionally.
            //
            // TODO(row 31, D9: status-line press-and-hold drag-highlight +
            // drawSelect hover) — single-shot port (the D9 press-and-hold
            // deferral, identical to scroller/menu/list-viewer): one hit-test per
            // mouse-down, no hover redraw. The group already delivers the mouse
            // position view-local (D3 — makeLocal is gone).
            Event::MouseDown(m) => {
                if let Some(idx) = self.item_mouse_is_in(m.position) {
                    let cmd = self.items()[idx].command;
                    if self.command_enabled(cmd) {
                        ctx.post(cmd); // C++ putEvent(evCommand)
                    }
                }
                ev.clear(); // C++ clears unconditionally after the loop.
            }
            // TODO(status keyDown global accelerator — lands with TProgram getEvent
            // pre-routing; transform-in-place, not ctx.post). The keyDown arm
            // matches event.keyDown against EVERY item's key_code (incl.
            // text==None) and transforms the event into evCommand in place WITHOUT
            // clearing, so it propagates — semantics that only make sense inside
            // getEvent's pre-routing. Deliberately NOT ported as ctx.post + clear
            // (that double-handles).
            _ => {}
        }
    }

    /// The §2 command-graying broker hook (row 49 mechanism, reused). The pump
    /// calls this at apply time with the live [`CommandSet`] in hand; we snapshot
    /// it into [`cmd_set`](StatusLine::cmd_set) so `draw` can gray disabled items
    /// (C++ `drawSelect` calls `commandEnabled` live; we cache it because
    /// `TStatusItem` has no `disabled` field to mutate).
    fn update_menu_commands(&mut self, cs: &CommandSet) {
        self.cmd_set = Some(cs.clone());
    }

    /// Expose the concrete line so the pump / tests can introspect it (the cached
    /// command-set snapshot the broker drives).
    fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> {
        Some(self)
    }
}

impl StatusLine {
    /// Read the cached command-set snapshot (test/inspection hook for the
    /// broker-driven graying cache).
    pub fn cmd_set(&self) -> Option<&CommandSet> {
        self.cmd_set.as_ref()
    }

    /// Read the currently-selected def index (test hook for `find_items`).
    pub fn selected_def(&self) -> Option<usize> {
        self.items_def
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{HeadlessBackend, Renderer};
    use crate::command::{Command, CommandSet};
    use crate::event::{Event, Key, KeyEvent, MouseButtons, MouseEvent};
    use crate::menu::alt;
    use crate::screen::Buffer;
    use crate::status::StatusDef;
    use crate::theme::Theme;

    fn f1() -> KeyEvent {
        KeyEvent::from(Key::F(1))
    }

    /// A canonical default status line: Help, Exit, and a hidden Cut binding.
    fn sample_defs() -> Vec<StatusDef> {
        StatusDef::list()
            .def_all(|d| {
                d.item("~F1~ Help", f1(), Command::HELP)
                    .item("~Alt-X~ Exit", alt('x'), Command::QUIT)
                    .key_item(KeyEvent::from(Key::F(10)), Command::MENU)
            })
            .build()
    }

    fn render(line: &mut StatusLine, w: u16, h: u16) -> String {
        let theme = Theme::classic_blue();
        let (backend, screen) = HeadlessBackend::new(w, h);
        let mut r = Renderer::new(Box::new(backend));
        r.render(|buf: &mut Buffer| {
            let bounds = line.state().get_bounds();
            let mut dc = DrawCtx::new(buf, &theme, bounds, bounds.a);
            line.draw(&mut dc);
        });
        screen.snapshot()
    }

    // -- ctor ---------------------------------------------------------------

    #[test]
    fn ctor_sets_grow_and_preprocess() {
        let line = StatusLine::new(Rect::new(0, 24, 40, 25), sample_defs());
        assert!(line.state.grow_mode.lo_y, "gfGrowLoY set");
        assert!(line.state.grow_mode.hi_x, "gfGrowHiX set");
        assert!(line.state.grow_mode.hi_y, "gfGrowHiY set");
        assert!(line.state.options.pre_process, "ofPreProcess set");
        // find_items ran in the ctor: the All def is selected.
        assert_eq!(line.selected_def(), Some(0));
        assert_eq!(line.help_ctx, HelpCtx::NO_CONTEXT);
        assert!(line.cmd_set.is_none(), "no command set snapshot yet");
    }

    // -- find_items ---------------------------------------------------------

    #[test]
    fn find_items_all_matches_any_ctx() {
        let mut line = StatusLine::new(Rect::new(0, 0, 40, 1), sample_defs());
        assert_eq!(line.selected_def(), Some(0));
        line.set_help_ctx(HelpCtx::custom("whatever.context"));
        assert_eq!(line.selected_def(), Some(0), "All matches any context");
    }

    #[test]
    fn find_items_first_match_wins_with_one_of_then_all() {
        // [OneOf([a]), All]: ctx a -> def 0; ctx b -> def 1 (the All fallback).
        let a = HelpCtx::custom("app.editor");
        let b = HelpCtx::custom("app.browser");
        let defs = StatusDef::list()
            .def_one_of([a], |d| d.item("~F2~ Save", None, Command::SAVE))
            .def_all(|d| d.item("~F1~ Help", f1(), Command::HELP))
            .build();
        let mut line = StatusLine::new(Rect::new(0, 0, 40, 1), defs);

        line.set_help_ctx(a);
        assert_eq!(line.selected_def(), Some(0), "ctx a selects the OneOf def");
        line.set_help_ctx(b);
        assert_eq!(
            line.selected_def(),
            Some(1),
            "ctx b falls through to the All def"
        );
    }

    #[test]
    fn find_items_bite_all_first_captures_everything() {
        // BITE: reorder so All is FIRST -> everything selects def 0 (the OneOf def
        // is never reached). Proves first_match_wins is order-sensitive.
        let a = HelpCtx::custom("app.editor");
        let defs = StatusDef::list()
            .def_all(|d| d.item("~F1~ Help", f1(), Command::HELP))
            .def_one_of([a], |d| d.item("~F2~ Save", None, Command::SAVE))
            .build();
        let mut line = StatusLine::new(Rect::new(0, 0, 40, 1), defs);
        line.set_help_ctx(a);
        assert_eq!(
            line.selected_def(),
            Some(0),
            "All first captures even ctx a"
        );
    }

    #[test]
    fn find_items_no_match_leaves_none() {
        // A line with only a OneOf def and a non-member ctx -> no selection.
        let a = HelpCtx::custom("app.editor");
        let b = HelpCtx::custom("app.browser");
        let defs = StatusDef::list()
            .def_one_of([a], |d| d.item("~F2~ Save", None, Command::SAVE))
            .build();
        let mut line = StatusLine::new(Rect::new(0, 0, 40, 1), defs);
        line.set_help_ctx(b);
        assert_eq!(line.selected_def(), None, "no def matches -> items == 0");
        // items() is then empty.
        assert!(line.items().is_empty());
    }

    // -- item_mouse_is_in ---------------------------------------------------

    #[test]
    fn item_mouse_is_in_off_row_is_none() {
        let line = StatusLine::new(Rect::new(0, 0, 40, 1), sample_defs());
        // mouse.y != 0 -> None (C++ guard).
        assert_eq!(line.item_mouse_is_in(Point::new(2, 1)), None);
    }

    #[test]
    fn item_mouse_is_in_hits_correct_item_and_skips_textless() {
        // Layout (cstrlen ignores ~):
        //   "F1 Help"  cstrlen 7 -> [0, 9)   (idx 0)
        //   "Alt-X Exit" cstrlen 10 -> [9, 21) (idx 1)
        //   hidden Cut (text None) -> consumes NOTHING, span untouched (idx 2)
        let line = StatusLine::new(Rect::new(0, 0, 40, 1), sample_defs());

        assert_eq!(line.item_mouse_is_in(Point::new(0, 0)), Some(0));
        assert_eq!(
            line.item_mouse_is_in(Point::new(8, 0)),
            Some(0),
            "last col of item 0"
        );
        assert_eq!(
            line.item_mouse_is_in(Point::new(9, 0)),
            Some(1),
            "first col of item 1"
        );
        assert_eq!(
            line.item_mouse_is_in(Point::new(20, 0)),
            Some(1),
            "last col of item 1"
        );
        // BITE: the trailing space col of item 1 (col 20) maps to item 1, and a
        // click past it (col 21) is in no item's span -> None (the hidden Cut
        // binding does NOT occupy columns).
        assert_eq!(
            line.item_mouse_is_in(Point::new(21, 0)),
            None,
            "past last visible item"
        );
    }

    #[test]
    fn item_mouse_is_in_textless_neighbour_unaffected() {
        // A textless item BETWEEN two visible items must not shift the second's
        // span (its columns are unaffected — it consumes no width).
        let defs = StatusDef::list()
            .def_all(|d| {
                d.item("AB", None, Command::HELP) // cstrlen 2 -> [0, 4)
                    .key_item(f1(), Command::CUT) // hidden -> no width
                    .item("CD", None, Command::QUIT) // cstrlen 2 -> [4, 8)
            })
            .build();
        let line = StatusLine::new(Rect::new(0, 0, 40, 1), defs);
        assert_eq!(line.item_mouse_is_in(Point::new(0, 0)), Some(0));
        // Index 1 is the hidden item; the click at col 4 lands on the visible
        // "CD" item (index 2), NOT the hidden index 1.
        assert_eq!(line.item_mouse_is_in(Point::new(4, 0)), Some(2));
        assert_eq!(line.item_mouse_is_in(Point::new(7, 0)), Some(2));
    }

    // -- command graying (broker hook) --------------------------------------

    #[test]
    fn update_menu_commands_snapshots_set() {
        // The broker hook caches the live command set; command_enabled reads it.
        let mut line = StatusLine::new(Rect::new(0, 0, 40, 1), sample_defs());
        // Before any refresh, cmd_set is None -> everything is enabled.
        assert!(line.cmd_set().is_none());
        assert!(
            line.command_enabled(Command::QUIT),
            "None set -> all enabled"
        );

        // Snapshot a set that has HELP but not QUIT.
        let mut cs = CommandSet::new();
        cs.enable_cmd(Command::HELP);
        cs.disable_cmd(Command::QUIT);
        line.update_menu_commands(&cs);

        assert!(line.cmd_set().is_some(), "broker hook cached the set");
        assert!(line.command_enabled(Command::HELP), "HELP enabled in set");
        assert!(
            !line.command_enabled(Command::QUIT),
            "QUIT disabled in set -> grayed"
        );
    }

    #[test]
    fn command_enabled_bite_without_refresh_stays_all_enabled() {
        // BITE for the broker: without update_menu_commands the snapshot stays
        // None and every command reads enabled (the startup gap menus share).
        let line = StatusLine::new(Rect::new(0, 0, 40, 1), sample_defs());
        assert!(line.cmd_set().is_none());
        assert!(line.command_enabled(Command::QUIT));
        assert!(line.command_enabled(Command::HELP));
    }

    // -- handle_event: broadcast arm ----------------------------------------

    #[test]
    fn broadcast_command_set_changed_requests_update_menu() {
        use crate::view::{Context, Deferred};
        use std::collections::VecDeque;

        let mut line = StatusLine::new(Rect::new(0, 0, 40, 1), sample_defs());
        // The view needs an id for request_update_menu to fire; stamp one as the
        // group would.
        let id = crate::view::ViewId::next();
        line.state.id = Some(id);

        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred = Vec::new();
        let mut ev = Event::Broadcast {
            command: Command::COMMAND_SET_CHANGED,
            source: None,
        };
        {
            let mut ctx = Context::new(&mut out, &mut timers, 0, &mut deferred);
            line.handle_event(&mut ev, &mut ctx);
        }

        assert!(
            deferred
                .iter()
                .any(|d| matches!(d, Deferred::UpdateMenu(uid) if *uid == id)),
            "cmCommandSetChanged requests UpdateMenu by the view's own id"
        );
    }

    // -- handle_event: mouse arm --------------------------------------------

    fn mouse_down(x: i32, y: i32) -> Event {
        Event::MouseDown(MouseEvent {
            position: Point::new(x, y),
            buttons: MouseButtons {
                left: true,
                ..Default::default()
            },
            ..Default::default()
        })
    }

    #[test]
    fn mouse_down_on_enabled_item_posts_command_and_clears() {
        use crate::view::Context;
        use std::collections::VecDeque;

        let mut line = StatusLine::new(Rect::new(0, 0, 40, 1), sample_defs());
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred = Vec::new();
        // Click inside "Alt-X Exit" (item 1, span [9, 21)) -> post QUIT.
        let mut ev = mouse_down(10, 0);
        {
            let mut ctx = Context::new(&mut out, &mut timers, 0, &mut deferred);
            line.handle_event(&mut ev, &mut ctx);
        }

        assert!(ev.is_nothing(), "mouse-down is always cleared");
        assert!(
            out.iter()
                .any(|e| matches!(e, Event::Command(Command::QUIT))),
            "an enabled item posts its command"
        );
    }

    #[test]
    fn mouse_down_on_disabled_item_clears_but_posts_nothing() {
        use crate::view::Context;
        use std::collections::VecDeque;

        let mut line = StatusLine::new(Rect::new(0, 0, 40, 1), sample_defs());
        // Disable QUIT via the broker snapshot.
        let mut cs = CommandSet::new();
        cs.enable_cmd(Command::HELP);
        cs.disable_cmd(Command::QUIT);
        line.update_menu_commands(&cs);

        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred = Vec::new();
        let mut ev = mouse_down(10, 0); // on the (disabled) Exit item
        {
            let mut ctx = Context::new(&mut out, &mut timers, 0, &mut deferred);
            line.handle_event(&mut ev, &mut ctx);
        }

        assert!(ev.is_nothing(), "mouse-down is cleared even when disabled");
        assert!(
            !out.iter().any(|e| matches!(e, Event::Command(_))),
            "a disabled item posts nothing (C++ commandEnabled guard)"
        );
    }

    #[test]
    fn mouse_down_off_row_clears_but_posts_nothing() {
        use crate::view::Context;
        use std::collections::VecDeque;

        let mut line = StatusLine::new(Rect::new(0, 0, 40, 1), sample_defs());
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred = Vec::new();
        let mut ev = mouse_down(2, 1); // y != 0 -> no item hit
        {
            let mut ctx = Context::new(&mut out, &mut timers, 0, &mut deferred);
            line.handle_event(&mut ev, &mut ctx);
        }

        assert!(ev.is_nothing());
        assert!(!out.iter().any(|e| matches!(e, Event::Command(_))));
    }

    // -- draw snapshots -----------------------------------------------------

    #[test]
    fn snapshot_normal_with_disabled_item() {
        // Two visible items + a hidden Cut binding; QUIT disabled (grayed) via the
        // broker snapshot. Proves the color matrix and the `i += l + 2` layout
        // (the hidden item adds nothing).
        let mut line = StatusLine::new(Rect::new(0, 0, 40, 1), sample_defs());
        let mut cs = CommandSet::new();
        cs.enable_cmd(Command::HELP);
        cs.disable_cmd(Command::QUIT);
        line.update_menu_commands(&cs);
        insta::assert_snapshot!(render(&mut line, 40, 1));
    }

    #[test]
    fn snapshot_with_hint_tail() {
        // A hint closure returning text -> proves the hint tail (separator +
        // clipped hint), with `i < size.x - 2` true.
        let defs = StatusDef::list()
            .def_all(|d| d.item("~F1~ Help", f1(), Command::HELP))
            .build();
        let mut line = StatusLine::new(Rect::new(0, 0, 40, 1), defs)
            .with_hint(|_| Some("Press F1 for help".to_string()));
        insta::assert_snapshot!(render(&mut line, 40, 1));
    }

    #[test]
    fn empty_hint_string_draws_no_separator() {
        // A hint closure returning Some("") must render NOTHING extra — C++
        // `if (hintText.size())` skips an empty hint, so no separator is drawn.
        // BITE: deleting the `!text.is_empty()` guard in `draw` would draw the
        // `│ ` separator for the empty hint, making this render differ from the
        // None-hint render. We assert they are byte-identical.
        let defs = StatusDef::list()
            .def_all(|d| d.item("~F1~ Help", f1(), Command::HELP))
            .build();
        let mut empty_hint = StatusLine::new(Rect::new(0, 0, 40, 1), defs.clone())
            .with_hint(|_| Some(String::new()));
        let mut none_hint = StatusLine::new(Rect::new(0, 0, 40, 1), defs).with_hint(|_| None);
        assert_eq!(
            render(&mut empty_hint, 40, 1),
            render(&mut none_hint, 40, 1),
            "an empty hint string draws no separator (C++ if(hintText.size()))"
        );
    }

    #[test]
    fn snapshot_narrow_drops_overflowing_item() {
        // Width 8: "F1 Help" cstrlen 7, i=0 -> 0+7 < 8 true (drawn). i becomes 9;
        // "Alt-X Exit" -> 9+10 < 8 FALSE, not drawn (the clip-skip branch), and
        // there is no room for a hint (i=9, size.x-2=6). Exercises the clipped
        // path the wide line never hits.
        let mut line = StatusLine::new(Rect::new(0, 0, 8, 1), sample_defs());
        insta::assert_snapshot!(render(&mut line, 8, 1));
    }

    #[test]
    fn snapshot_textless_item_draws_nothing() {
        // A line whose only items are a hidden binding then a visible one: the
        // hidden item must add no width, so the visible item starts at column 0.
        let defs = StatusDef::list()
            .def_all(|d| {
                d.key_item(f1(), Command::CUT) // hidden
                    .item("Visible", None, Command::HELP)
            })
            .build();
        let mut line = StatusLine::new(Rect::new(0, 0, 40, 1), defs);
        insta::assert_snapshot!(render(&mut line, 40, 1));
    }
}
