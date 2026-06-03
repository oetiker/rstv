//! `TListViewer` — faithful Rust port of `tlstview.cpp` (row 28, FOUNDATION).
//!
//! `TListViewer` is the **abstract base** for every list widget (`TListBox`
//! row 48, history viewers, color/file lists). It lays out `range` items in
//! `num_cols` columns, tracks a `focused` item and a `top_item` scroll offset,
//! and drives **two sibling `TScrollBar`s** that live on the window frame.
//!
//! ## D-A: a TRAIT, not a concrete struct (≠ the row-27 `Scroller` shape)
//!
//! `TListBox` reuses `TListViewer::draw` (it does **not** override it) and
//! overrides the virtuals `getText`/`isSelected`. A concrete-struct-embedded base
//! (the [`Scroller`](crate::widgets::Scroller) D2 shape) physically cannot
//! dispatch from the base's own `draw` back into the embedder's `getText`. So the
//! abstract base is modeled as a **trait** (the [`Validator`](crate::validate::Validator)
//! pattern): [`ListViewer`] carries the overridable virtuals, [`ListViewerState`]
//! carries the non-virtual data members, and the shared draw/event/nav logic
//! lives as **free functions generic over `<L: ListViewer + ?Sized>`** so a
//! concrete widget's `View` impl reuses them verbatim while they call back into
//! `get_text`/`is_selected`/`select_item`.
//!
//! [`ListViewer`] is intentionally **not object-safe** (`get_text -> String`);
//! that is fine — concrete widgets are stored as `Box<dyn View>`, and
//! `ListViewer` is only ever a generic bound behind a concrete type.
//!
//! ## D-B/D-C: the cross-view scrollbar read-sync (D3)
//!
//! Like the scroller, a list viewer holds only `&mut Context` during dispatch
//! (D3) and so can neither **read** nor **mutate** its window-frame sibling
//! scrollbars. The pump is the broker: the list stores its bars as
//! [`Option<ViewId>`] handles and the cached [`indent`](ListViewerState::indent)
//! (the live h-bar `value` the draw needs, refreshed by the read-sync). On a
//! `cmScrollBarChanged` broadcast naming one of its bars as `source`, the list
//! requests [`Deferred::SyncListViewer`](crate::view::Deferred::SyncListViewer);
//! the pump reads both bars' `value`s and calls back through
//! [`View::apply_list_scroll`](crate::view::View::apply_list_scroll) →
//! [`apply_scroll`].
//!
//! ## D-D: TERMINATION (the centerpiece property)
//!
//! Unlike the scroller, this read-sync **writes back**: `apply_scroll`'s v-bar
//! branch runs `focus_item_num` → [`focus_item`] → a deferred v-bar
//! `setValue(focused)`. That would re-broadcast `cmScrollBarChanged` and re-enter
//! the sync — except [`ScrollBar::set_params`](crate::widgets::ScrollBar::set_params)
//! is **change-guarded** (re-broadcasts only on an actual value change), so the
//! write-back of the already-current value is a silent no-op. Steady state
//! (vbar == focused): quiescent. After a clamp: one extra round, then quiescent.
//! (If the change-guard were removed, the cycle would spin forever — see the
//! termination test in `program.rs`.)
//!
//! ## Drops / deferrals (faithful, breadcrumbed)
//!
//! - **D12/D2:** `shutDown`/`write`/`read`/`build`/`streamableName`/`name` dropped.
//! - **getPalette → Theme roles** (D7): `cpListViewer` → [`Role::ListNormalActive`]
//!   / [`Role::ListNormalInactive`] / [`Role::ListFocused`] / [`Role::ListSelected`]
//!   / [`Role::ListDivider`].
//! - **mouse press-and-hold / auto-scroll loop** → `TODO(row 31, D9)` (single-shot
//!   positioning only, like the scrollbar/cluster/input-line).
//! - **`change_bounds` step republish** → `TODO(resize)` (no consumer yet). NOTE:
//!   a resize consumer must NOT call [`update_steps`] — that reproduces the C++
//!   **ctor** `setStep` formula, but `TListViewer::changeBounds` (tlstview.cpp:71-74)
//!   uses a **distinct** formula: vbar `setStep(size.y, <preserve arStep>)` (plain
//!   `size.y`, NOT `size.y-1` / `size.y*numCols`) and hbar `setStep(size.x/numCols,
//!   <preserve arStep>)` — **both bars preserve the existing arStep**. The resize
//!   consumer must apply that `changeBounds` formula directly.
//! - **`showMarkers` block** dropped (removed framework-wide at row 23).
//! - scroller/list-viewer read-sync unification → optional later, out of scope.

use crate::command::Command;
use crate::event::{Event, Key, ctrl_to_arrow};
use crate::theme::Role;
use crate::view::{Context, DrawCtx, StateFlag, View, ViewId, ViewState};

/// The empty-list placeholder text (`TListViewer::emptyText`, `tlstview.cpp`).
const EMPTY_TEXT: &str = "<empty>";

// ---------------------------------------------------------------------------
// ListViewerState — the non-virtual data members (TListViewer's fields)
// ---------------------------------------------------------------------------

/// The shared state of every list-viewer — `TListViewer`'s non-virtual data
/// members. A concrete list widget embeds one and exposes it via
/// [`ListViewer::lv`]/[`ListViewer::lv_mut`].
pub struct ListViewerState {
    /// View state (geometry, flags, …) — the D2 `View` composition target.
    pub state: ViewState,
    /// `numCols` (`>= 1`): the number of columns the items are laid out in.
    pub num_cols: i32,
    /// `topItem`: the item index drawn at the top-left cell.
    pub top_item: i32,
    /// `focused`: the currently focused (cursor) item index.
    pub focused: i32,
    /// `range`: the number of items (the list length).
    pub range: i32,
    /// **Cached** `hScrollBar->value` — `draw` reads the h-bar `value` live in
    /// C++, but under D3 the draw (a [`DrawCtx`]) cannot reach the sibling bar, so
    /// the value is cached here and refreshed by the read-sync ([`apply_scroll`]).
    pub indent: i32,
    /// The horizontal scrollbar, by id (`None` if absent). TV's `hScrollBar`.
    pub h_scroll_bar: Option<ViewId>,
    /// The vertical scrollbar, by id (`None` if absent). TV's `vScrollBar`.
    pub v_scroll_bar: Option<ViewId>,
}

impl ListViewerState {
    /// Construct list-viewer state — ports `TListViewer::TListViewer`'s field
    /// initialization (the bar `setStep` calls cannot run here without a `Context`;
    /// the consumer calls [`update_steps`] after insertion — see its docs).
    ///
    /// Faithful: `options |= ofFirstClick | ofSelectable`; `topItem = focused =
    /// range = 0`; `indent = 0`; `numCols = aNumCols`. The C++ `eventMask |=
    /// evBroadcast` has **no analogue** — under D4 broadcasts are delivered
    /// unconditionally (same note as the scroller).
    pub fn new(
        bounds: crate::view::Rect,
        num_cols: i32,
        h_scroll_bar: Option<ViewId>,
        v_scroll_bar: Option<ViewId>,
    ) -> Self {
        debug_assert!(
            num_cols >= 1,
            "ListViewer num_cols must be >= 1 (size.x / num_cols and col_width math \
             divide by it); clamping to 1"
        );
        let num_cols = num_cols.max(1);
        let mut state = ViewState::new(bounds);
        state.options.first_click = true;
        state.options.selectable = true;
        ListViewerState {
            state,
            num_cols,
            top_item: 0,
            focused: 0,
            range: 0,
            indent: 0,
            h_scroll_bar,
            v_scroll_bar,
        }
    }
}

// ---------------------------------------------------------------------------
// ListViewer — the overridable virtuals (a trait, D-A)
// ---------------------------------------------------------------------------

/// The abstract list-viewer base — `TListViewer`'s overridable virtuals (D-A).
///
/// Concrete list widgets implement [`lv`](Self::lv)/[`lv_mut`](Self::lv_mut)
/// (the data accessors) and override [`get_text`](Self::get_text)/
/// [`is_selected`](Self::is_selected)/[`select_item`](Self::select_item) as
/// needed; the shared draw/event/nav logic (the free functions in this module)
/// is generic over `L: ListViewer` and calls back into these.
///
/// **Wiring caveat (no compile-time enforcement):** a concrete list widget MUST
/// delegate ALL of these `View` methods to this module's free functions:
/// [`draw`], [`handle_event`], [`set_state`], [`View::cursor_request`](crate::view::View::cursor_request)
/// → [`focused_cursor`], [`View::apply_list_scroll`](crate::view::View::apply_list_scroll)
/// → [`apply_scroll`], and [`View::as_any_mut`](crate::view::View::as_any_mut)
/// (the cross-view broker downcasts through it). In particular, forgetting to
/// override `apply_list_scroll` is **silent** — its base default is a no-op, so the
/// widget compiles but loses all scrollbar read-sync with no error. (See `FakeList`
/// in this module's tests for the full delegation template.)
pub trait ListViewer: View {
    /// Borrow the embedded [`ListViewerState`].
    fn lv(&self) -> &ListViewerState;
    /// Mutably borrow the embedded [`ListViewerState`].
    fn lv_mut(&mut self) -> &mut ListViewerState;

    /// `TListViewer::getText` — the text for `item`. Base returns empty (C++
    /// `*dest = EOS`); `TListBox` & friends override.
    fn get_text(&self, _item: i32) -> String {
        String::new()
    }

    /// `TListViewer::isSelected` — whether `item` is "selected" (drawn in the
    /// selected color). Base: `item == focused`; multi-select subclasses override.
    fn is_selected(&self, item: i32) -> bool {
        item == self.lv().focused
    }

    /// `TListViewer::selectItem` — the user committed to `item` (double-click /
    /// Space / Enter). Base broadcasts `cmListItemSelected` with this view as the
    /// subject (`message(owner, evBroadcast, cmListItemSelected, this)` → the
    /// `infoPtr`→`source` successor, D4). Subclasses override to act.
    fn select_item(&mut self, _item: i32, ctx: &mut Context) {
        let source = self.lv().state.id();
        ctx.broadcast(Command::LIST_ITEM_SELECTED, source);
    }
}

// ---------------------------------------------------------------------------
// Shared logic — free functions generic over <L: ListViewer + ?Sized>
// ---------------------------------------------------------------------------

/// `TListViewer::focusItemNum` — clamp `item` into the valid range, then focus it
/// (only when the list is non-empty).
///
/// Faithful: `item < 0 → 0`; `item >= range && range > 0 → range - 1`; then
/// `if range != 0 { focus_item(item) }`.
pub fn focus_item_num<L: ListViewer + ?Sized>(this: &mut L, mut item: i32, ctx: &mut Context) {
    if item < 0 {
        item = 0;
    } else if item >= this.lv().range && this.lv().range > 0 {
        item = this.lv().range - 1;
    }
    if this.lv().range != 0 {
        focus_item(this, item, ctx);
    }
}

/// `TListViewer::focusItem` — set `focused = item`, push the new value to the
/// v-bar, and adjust `top_item` so the focused item is visible.
///
/// Faithful: `focused = item`; if a v-bar exists, request `setValue(item)`
/// (deferred — D3); the C++ `else drawView()` is **dropped** (D8, whole-tree
/// redraw). Then the `top_item` adjust block (verbatim, guarded by `size.y > 0`,
/// the `numCols == 1` vs multi-col cases).
pub fn focus_item<L: ListViewer + ?Sized>(this: &mut L, item: i32, ctx: &mut Context) {
    this.lv_mut().focused = item;
    if let Some(v) = this.lv().v_scroll_bar {
        // vScrollBar->setValue(item) — the write-back the termination property
        // relies on (no-op when the bar's value already == item; ScrollBar::
        // set_params is change-guarded).
        ctx.request_scroll_bar_params(v, Some(item), None, None, None, None);
    }
    // else drawView() dropped (D8).

    let size_y = this.lv().state.size.y;
    let num_cols = this.lv().num_cols;
    let top_item = this.lv().top_item;
    if size_y > 0 {
        if item < top_item {
            this.lv_mut().top_item = if num_cols == 1 {
                item
            } else {
                item - item % size_y
            };
        } else if item >= top_item + size_y * num_cols {
            this.lv_mut().top_item = if num_cols == 1 {
                item - size_y + 1
            } else {
                item - item % size_y - (size_y * (num_cols - 1))
            };
        }
    }
}

/// `TListViewer::setRange` — set the list length, resetting `focused` if it now
/// falls past the end, and (re)publish the v-bar's range.
///
/// Faithful: `range = aRange`; if `focused >= aRange` → `focused = 0`; if a v-bar
/// exists, request `setParams(focused, 0, aRange - 1, <preserve pg>, <preserve
/// ar>)` (deferred — D3). The C++ `else drawView()` is dropped (D8).
pub fn set_range<L: ListViewer + ?Sized>(this: &mut L, a_range: i32, ctx: &mut Context) {
    this.lv_mut().range = a_range;
    if this.lv().focused >= a_range {
        this.lv_mut().focused = 0;
    }
    let focused = this.lv().focused;
    if let Some(v) = this.lv().v_scroll_bar {
        ctx.request_scroll_bar_params(v, Some(focused), Some(0), Some(a_range - 1), None, None);
    }
    // else drawView() dropped (D8).
}

/// The body of the `cmScrollBarChanged` read-sync — ports both branches of
/// `TListViewer::handleEvent`'s `cmScrollBarChanged` case, called by the pump
/// (the read broker) after it resolves both bars and reads their `value`s.
///
/// Faithful merge (D-C): the h-bar branch (C++ just `drawView`) refreshes the
/// cached [`indent`](ListViewerState::indent); the v-bar branch runs
/// `focusItemNum(vScrollBar->value)`. Reading both each sync is harmless — the
/// v-bar write-back is a no-op in steady state (D-D).
pub fn apply_scroll<L: ListViewer + ?Sized>(
    this: &mut L,
    h: Option<i32>,
    v: Option<i32>,
    ctx: &mut Context,
) {
    if let Some(hv) = h {
        this.lv_mut().indent = hv;
    }
    if let Some(vv) = v {
        focus_item_num(this, vv, ctx);
    }
}

/// The list-viewer **ctor** `setStep` — (re)publish each bar's page/arrow step.
/// Exposed as a `Context`-taking entry the consumer/test calls **after insertion**
/// (the no-`Context` ctor cannot reach the bars — the same constraint the scroller
/// hit).
///
/// Faithful to the **ctor** (`TListViewer::TListViewer`, tlstview.cpp):
/// - v-bar: `numCols == 1` → `pgStep = size.y - 1`, `arStep = 1`; else
///   `pgStep = size.y * numCols`, `arStep = size.y`. `setStep(pgStep, arStep)`.
/// - h-bar: `setStep(size.x / numCols, 1)`.
///
/// **This is the CTOR formula, NOT the resize formula.** `TListViewer::changeBounds`
/// (tlstview.cpp:71-74) uses a **different** `setStep`: vbar `setStep(size.y,
/// <preserve arStep>)` (plain `size.y`) and hbar `setStep(size.x/numCols, <preserve
/// arStep>)`, both **preserving the live arStep**. A future resize consumer must
/// apply that `changeBounds` formula directly — do **NOT** call `update_steps` for a
/// resize. (No resize consumer exists yet — `TODO(resize)`.)
pub fn update_steps<L: ListViewer + ?Sized>(this: &L, ctx: &mut Context) {
    let size = this.lv().state.size;
    let num_cols = this.lv().num_cols;
    if let Some(v) = this.lv().v_scroll_bar {
        let (pg_step, ar_step) = if num_cols == 1 {
            (size.y - 1, 1)
        } else {
            (size.y * num_cols, size.y)
        };
        ctx.request_scroll_bar_params(v, None, None, None, Some(pg_step), Some(ar_step));
    }
    if let Some(h) = this.lv().h_scroll_bar {
        ctx.request_scroll_bar_params(h, None, None, None, Some(size.x / num_cols), Some(1));
    }
}

/// `TListViewer::setState` — flip the flag (+ the Focused broadcast), then on
/// `Active`/`Selected` show/hide BOTH bars.
///
/// Faithful: the base flip + the `sfFocused` broadcast (copied from the
/// scroller's `set_state`). C++ triggers the show/hide on `(sfSelected | sfActive
/// | sfVisible)`; we have no `StateFlag::Visible` (D8 dropped its propagation), so
/// the `sfVisible` arm is moot and we trigger on `{Active, Selected}`. Visibility
/// is `getState(sfActive) && getState(sfVisible)` — **both** (NOT the scroller's
/// `active || selected`!). `drawView` dropped (D8).
pub fn set_state<L: ListViewer + ?Sized>(
    this: &mut L,
    flag: StateFlag,
    enable: bool,
    ctx: &mut Context,
) {
    this.lv_mut().state.set_flag(flag, enable);
    if flag == StateFlag::Focused {
        let source = this.lv().state.id();
        ctx.broadcast(
            if enable {
                Command::RECEIVED_FOCUS
            } else {
                Command::RELEASED_FOCUS
            },
            source,
        );
    }
    // sfVisible arm is moot (D8 dropped StateFlag::Visible) — trigger on
    // Active/Selected only.
    if flag == StateFlag::Active || flag == StateFlag::Selected {
        // C++ show iff getState(sfActive) && getState(sfVisible) — BOTH, not the
        // scroller's active||selected.
        let visible = this.lv().state.state.active && this.lv().state.state.visible;
        if let Some(h) = this.lv().h_scroll_bar {
            ctx.request_set_visible(h, visible);
        }
        if let Some(v) = this.lv().v_scroll_bar {
            ctx.request_set_visible(v, visible);
        }
    }
}

/// `TListViewer::handleEvent` — mouse (single-shot) + keyboard nav + the
/// scrollbar broadcast filter. Reusable verbatim by `TListBox` (D-A).
///
/// The C++ `TView::handleEvent(event)` super-call is the relocated mouse-down
/// auto-select (now `Group`'s job, D4), so it is omitted.
pub fn handle_event<L: ListViewer + ?Sized>(this: &mut L, ev: &mut Event, ctx: &mut Context) {
    match *ev {
        // -------------------------------------------------------------------
        // evMouseDown — single-shot positioning + double-click select.
        //
        // TODO(row 31, D9): the C++ runs a `do { … } while(mouseEvent(event,
        // evMouseMove | evMouseAuto))` press-and-hold / edge auto-scroll loop.
        // That synchronous inner pump needs the live event loop (a capture
        // handler on MouseMove/MouseAuto/MouseUp). Until then we do exactly one
        // positioning per mouse-down (and the double-click select).
        // -------------------------------------------------------------------
        Event::MouseDown(me) => {
            let size = this.lv().state.size;
            let num_cols = this.lv().num_cols;
            let col_width = size.x / num_cols + 1;
            let top_item = this.lv().top_item;
            // mouse is view-local already (D3 — makeLocal/mouseInView are gone;
            // the group delivers view-local coords).
            let mouse = me.position;
            let new_item = mouse.y + size.y * (mouse.x / col_width) + top_item;
            focus_item_num(this, new_item, ctx);
            // drawView() dropped (D8).
            if me.flags.double_click && this.lv().range > new_item {
                this.select_item(new_item, ctx);
            }
            ev.clear();
        }

        // -------------------------------------------------------------------
        // evKeyDown — Space → select, else the nav switch (via ctrlToArrow).
        // -------------------------------------------------------------------
        Event::KeyDown(ke) => {
            let focused = this.lv().focused;
            let range = this.lv().range;
            let size_y = this.lv().state.size.y;
            let num_cols = this.lv().num_cols;

            let new_item: i32;
            // charCode == ' ' && focused < range -> selectItem(focused).
            if matches!(ke.key, Key::Char(' '))
                && !ke.modifiers.ctrl
                && !ke.modifiers.alt
                && focused < range
            {
                this.select_item(focused, ctx);
                new_item = focused;
            } else if matches!(ke.key, Key::PageDown) && ke.modifiers.ctrl {
                // kbCtrlPgDn -> last item. Matched on the DECOMPOSED key (PageDown +
                // ctrl, D5) BEFORE ctrl_to_arrow, which would otherwise see no
                // Char to remap and pass PageDown through as a plain page jump.
                new_item = range - 1;
            } else if matches!(ke.key, Key::PageUp) && ke.modifiers.ctrl {
                // kbCtrlPgUp -> first item.
                new_item = 0;
            } else {
                // ctrlToArrow(keyCode) — the WordStar Ctrl-letter nav aliases.
                let mapped = ctrl_to_arrow(ke);
                new_item = match mapped.key {
                    Key::Up => focused - 1,
                    Key::Down => focused + 1,
                    // Left/Right only navigate when there is more than one column;
                    // with numCols == 1 the C++ `return`s (event left uncleared) —
                    // realized here by the guard falling through to `_ => return`.
                    Key::Right if num_cols > 1 => focused + size_y,
                    Key::Left if num_cols > 1 => focused - size_y,
                    Key::PageDown => focused + size_y * num_cols,
                    Key::PageUp => focused - size_y * num_cols,
                    Key::Home => this.lv().top_item,
                    Key::End => this.lv().top_item + (size_y * num_cols) - 1,
                    _ => return, // default (incl. single-col Left/Right): return.
                };
            }
            focus_item_num(this, new_item, ctx);
            // drawView() dropped (D8).
            ev.clear();
        }

        // -------------------------------------------------------------------
        // evBroadcast — own-bar cmScrollBarClicked → select; cmScrollBarChanged
        // → request a read-sync (the source filter, like the scroller).
        // -------------------------------------------------------------------
        Event::Broadcast { command, source } => {
            // (options & ofSelectable) guard, faithful to the C++.
            if !this.lv().state.options.selectable {
                return;
            }
            let h = this.lv().h_scroll_bar;
            let v = this.lv().v_scroll_bar;
            let from_own_bar = source.is_some() && (source == h || source == v);
            if command == Command::SCROLL_BAR_CLICKED && from_own_bar {
                // select() — focus this view within its owning group (the row-41
                // FocusById seam). Requires this view be inserted (have an id).
                if let Some(id) = this.lv().state.id() {
                    ctx.request_focus(id);
                }
            } else if command == Command::SCROLL_BAR_CHANGED && from_own_bar {
                // The pump brokers the read (resolve the bars, read value, call
                // back through apply_list_scroll). Requires this view have an id.
                if let Some(id) = this.lv().state.id() {
                    ctx.request_sync_list_viewer(id, h, v);
                }
            }
        }

        _ => {}
    }
}

/// `TListViewer::draw` — render the `range` items in `num_cols` columns. Reusable
/// verbatim by `TListBox` (D-A); calls back into [`get_text`](ListViewer::get_text)
/// / [`is_selected`](ListViewer::is_selected).
///
/// Ports the C++ draw loop (D-F): the active/inactive color matrix, the per-cell
/// item/column layout, the `indent` column-skip (the cached h-bar value), the
/// `<empty>` placeholder, the `│` divider, and the focused-cell cursor. The
/// `showMarkers` block is dropped (D8/row 23). `writeLine`/`DrawBuffer` becomes
/// direct [`DrawCtx`] writes (no `DrawBuffer` in our model).
pub fn draw<L: ListViewer + ?Sized>(this: &L, ctx: &mut DrawCtx) {
    let lv = this.lv();
    let st = &lv.state.state;
    let active = st.selected && st.active;

    // Color matrix (cpListViewer idx 1..5 via Theme roles, D7).
    let (normal, selected, focused_color) = if active {
        (
            ctx.style(Role::ListNormalActive),  // getColor(1)
            ctx.style(Role::ListSelected),      // getColor(4)
            Some(ctx.style(Role::ListFocused)), // getColor(3)
        )
    } else {
        (
            ctx.style(Role::ListNormalInactive), // getColor(2)
            ctx.style(Role::ListSelected),       // getColor(4)
            None,                                // focusedColor unused
        )
    };
    let divider_color = ctx.style(Role::ListDivider); // getColor(5)
    let empty_color = ctx.style(Role::ListNormalActive); // getColor(1)

    let size = lv.state.size;
    let indent = lv.indent; // the CACHE (not a live h-bar read).
    let num_cols = lv.num_cols;
    let top_item = lv.top_item;
    let range = lv.range;
    let focused = lv.focused;

    let col_width = size.x / num_cols + 1;

    for i in 0..size.y {
        for j in 0..num_cols {
            let item = j * size.y + i + top_item;
            let cur_col = j * col_width;

            let color = if active && focused == item && range > 0 {
                // focused cell: drawn in the focused color; the hardware cursor for
                // this cell is surfaced (&self) via `focused_cursor`, not set here
                // (C++ does `setCursor(curCol+1, i)` inline — our &self draw +
                // top-down cursor walk derives it on demand).
                focused_color.unwrap_or(normal)
            } else if item < range && this.is_selected(item) {
                selected
            } else {
                normal
            };

            // b.moveChar(curCol, ' ', color, colWidth).
            ctx.fill(
                crate::view::Rect::new(cur_col, i, cur_col + col_width, i + 1),
                ' ',
                color,
            );

            if item < range {
                // b.moveStr(curCol+1, text, color, colWidth, indent) — the
                // moveStr `begin` offset is the indent column-skip (put_str_part).
                // The C++ `if (indent < 255)` guard caps a 255-wide text buffer;
                // our get_text returns an owned String, so the guard is moot.
                let text = this.get_text(item);
                ctx.put_str_part(cur_col + 1, i, &text, indent, color);
            } else if i == 0 && j == 0 {
                // b.moveStr(curCol+1, emptyText, getColor(1)).
                ctx.put_str(cur_col + 1, i, EMPTY_TEXT, empty_color);
            }

            // b.moveChar(curCol+colWidth-1, '\xB3', getColor(5), 1) — the divider.
            let vbar = ctx.glyphs().frame_v;
            ctx.put_char(cur_col + col_width - 1, i, vbar, divider_color);
        }
    }
    // The C++ `if (!focusedVis) setCursor(-1,-1)` (hide the cursor when no focused
    // cell is visible) is realized by `focused_cursor` returning `None`, which a
    // concrete widget surfaces via `cursor_request` — not a mutation here (draw is
    // &self under our top-down cursor walk).
}

/// The view-local cursor position the focused cell sits at, or `None` if no
/// focused cell is visible — the `setCursor(curCol+1, i)` / `setCursor(-1,-1)`
/// outcome of [`draw`], computed `&self`-only so a concrete widget can surface it
/// via [`View::cursor_request`](crate::view::View::cursor_request).
///
/// (C++ `draw` calls `setCursor` as a side effect; under our `&self` draw +
/// top-down cursor walk the position is derived on demand instead.)
pub fn focused_cursor<L: ListViewer + ?Sized>(this: &L) -> Option<crate::view::Point> {
    let lv = this.lv();
    let st = &lv.state.state;
    if !(st.selected && st.active) || lv.range <= 0 {
        return None;
    }
    let size = lv.state.size;
    let num_cols = lv.num_cols;
    let col_width = size.x / num_cols + 1;
    let top_item = lv.top_item;
    let focused = lv.focused;
    for i in 0..size.y {
        for j in 0..num_cols {
            let item = j * size.y + i + top_item;
            if focused == item {
                return Some(crate::view::Point::new(j * col_width + 1, i));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::{HeadlessBackend, Renderer};
    use crate::screen::Buffer;
    use crate::theme::Theme;
    use crate::view::{Deferred, Group, Point, Rect};
    use std::collections::HashSet;
    use std::collections::VecDeque;

    // -- FakeList: the first (test-only) concrete ListViewer (D-A) ------------

    /// A concrete list viewer over a `Vec<String>` with a `HashSet<i32>` of
    /// selected items — the first real consumer of the trait (NOT a dead stub; it
    /// drives the draw/nav/sync tests). `TListBox` (row 48) is the production one.
    struct FakeList {
        lv: ListViewerState,
        items: Vec<String>,
        selected: HashSet<i32>,
    }

    impl FakeList {
        fn new(
            bounds: Rect,
            num_cols: i32,
            items: Vec<String>,
            h: Option<ViewId>,
            v: Option<ViewId>,
        ) -> Self {
            let mut lv = ListViewerState::new(bounds, num_cols, h, v);
            lv.range = items.len() as i32;
            FakeList {
                lv,
                items,
                selected: HashSet::new(),
            }
        }
    }

    impl View for FakeList {
        fn state(&self) -> &ViewState {
            &self.lv.state
        }
        fn state_mut(&mut self) -> &mut ViewState {
            &mut self.lv.state
        }
        fn draw(&mut self, ctx: &mut DrawCtx) {
            draw(self, ctx);
        }
        fn handle_event(&mut self, ev: &mut Event, ctx: &mut Context) {
            handle_event(self, ev, ctx);
        }
        fn set_state(&mut self, flag: StateFlag, enable: bool, ctx: &mut Context) {
            set_state(self, flag, enable, ctx);
        }
        fn cursor_request(&self) -> Option<Point> {
            focused_cursor(self)
        }
        fn apply_list_scroll(&mut self, h: Option<i32>, v: Option<i32>, ctx: &mut Context) {
            apply_scroll(self, h, v, ctx);
        }
        fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> {
            Some(self)
        }
    }

    impl ListViewer for FakeList {
        fn lv(&self) -> &ListViewerState {
            &self.lv
        }
        fn lv_mut(&mut self) -> &mut ListViewerState {
            &mut self.lv
        }
        fn get_text(&self, item: i32) -> String {
            self.items.get(item as usize).cloned().unwrap_or_default()
        }
        fn is_selected(&self, item: i32) -> bool {
            // Honor the explicit selected-set; fall back to the base (== focused).
            self.selected.contains(&item) || item == self.lv.focused
        }
    }

    fn items(n: i32) -> Vec<String> {
        (0..n).map(|i| format!("item{i}")).collect()
    }

    fn make_ctx<'a>(
        out: &'a mut VecDeque<Event>,
        timers: &'a mut crate::timer::TimerQueue,
        deferred: &'a mut Vec<Deferred>,
    ) -> Context<'a> {
        Context::new(out, timers, 0, deferred)
    }

    /// Mint a real `ViewId` by inserting a throwaway view into a group.
    fn mint_id() -> (Group, ViewId) {
        let mut g = Group::new(Rect::new(0, 0, 4, 4));
        let id = g.insert(Box::new(FakeList::new(
            Rect::new(0, 0, 1, 1),
            1,
            vec![],
            None,
            None,
        )));
        (g, id)
    }

    // -- 1. ctor defaults ----------------------------------------------------

    #[test]
    fn ctor_sets_options_and_zeroes_fields() {
        let l = FakeList::new(Rect::new(0, 0, 10, 5), 1, vec![], None, None);
        assert!(l.lv.state.options.first_click, "ofFirstClick set");
        assert!(l.lv.state.options.selectable, "ofSelectable set");
        assert_eq!(l.lv.top_item, 0);
        assert_eq!(l.lv.focused, 0);
        assert_eq!(l.lv.indent, 0);
        assert_eq!(l.lv.num_cols, 1);
        // range is set by the FakeList ctor from items (empty -> 0).
        assert_eq!(l.lv.range, 0);
        // No evBroadcast mask analogue (D4).
        assert_eq!(l.lv.state.event_mask, crate::event::EventMask::default());
    }

    // -- 2. focus_item_num clamp matrix --------------------------------------

    #[test]
    fn focus_item_num_clamps_negative_and_over_range_and_skips_empty() {
        let (_g, v) = mint_id();
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred = vec![];

        // range 5: clamp -3 -> 0.
        let mut l = FakeList::new(Rect::new(0, 0, 10, 5), 1, items(5), None, Some(v));
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            focus_item_num(&mut l, -3, &mut ctx);
        }
        assert_eq!(l.lv.focused, 0, "negative clamps to 0");

        // clamp 99 -> range-1 = 4.
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            focus_item_num(&mut l, 99, &mut ctx);
        }
        assert_eq!(l.lv.focused, 4, ">= range clamps to range-1");

        // range == 0: focus_item is NOT called (focused stays whatever it was).
        let mut empty = FakeList::new(Rect::new(0, 0, 10, 5), 1, vec![], None, None);
        empty.lv.focused = 7; // a sentinel
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            focus_item_num(&mut empty, 3, &mut ctx);
        }
        assert_eq!(empty.lv.focused, 7, "range==0 -> focus_item skipped");
    }

    // -- 3. focus_item topItem adjust ----------------------------------------

    #[test]
    fn focus_item_single_col_scrolls_top_item_both_directions() {
        // size.y = 5, numCols = 1, 20 items. Scroll down past the bottom, then up.
        let mut l = FakeList::new(Rect::new(0, 0, 10, 5), 1, items(20), None, None);
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred = vec![];

        // Focus item 7: 7 >= topItem(0) + size.y(5)*1 -> topItem = 7 - 5 + 1 = 3.
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            focus_item(&mut l, 7, &mut ctx);
        }
        assert_eq!(l.lv.top_item, 3, "scroll down: topItem = item - size.y + 1");

        // Focus item 1: 1 < topItem(3) -> topItem = item = 1.
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            focus_item(&mut l, 1, &mut ctx);
        }
        assert_eq!(l.lv.top_item, 1, "scroll up: topItem = item");
    }

    #[test]
    fn focus_item_multi_col_scrolls_top_item() {
        // size.y = 3, numCols = 2 -> a page is size.y*numCols = 6 items.
        let mut l = FakeList::new(Rect::new(0, 0, 10, 3), 2, items(40), None, None);
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred = vec![];

        // Focus item 10: 10 >= topItem(0) + 6 -> multi-col:
        //   topItem = item - item%size.y - size.y*(numCols-1)
        //           = 10 - 10%3 - 3*1 = 10 - 1 - 3 = 6.
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            focus_item(&mut l, 10, &mut ctx);
        }
        assert_eq!(l.lv.top_item, 6, "multi-col scroll down");

        // Now focus item 2: 2 < topItem(6) -> multi-col: topItem = item - item%size.y
        //   = 2 - 2%3 = 2 - 2 = 0.
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            focus_item(&mut l, 2, &mut ctx);
        }
        assert_eq!(l.lv.top_item, 0, "multi-col scroll up");
    }

    #[test]
    fn focus_item_queues_v_bar_set_value() {
        let (_g, v) = mint_id();
        let mut l = FakeList::new(Rect::new(0, 0, 10, 5), 1, items(20), None, Some(v));
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred: Vec<Deferred> = vec![];
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            focus_item(&mut l, 7, &mut ctx);
        }
        assert_eq!(deferred.len(), 1, "one setValue op");
        assert!(matches!(
            deferred[0],
            Deferred::ScrollBarSetParams { id, value: Some(7), min: None, max: None, page_step: None, arrow_step: None }
                if id == v
        ));
    }

    // -- 4. set_range --------------------------------------------------------

    #[test]
    fn set_range_resets_focused_past_end_and_queues_v_params() {
        let (_g, v) = mint_id();
        let mut l = FakeList::new(Rect::new(0, 0, 10, 5), 1, items(20), None, Some(v));
        l.lv.focused = 15;
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred: Vec<Deferred> = vec![];
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            set_range(&mut l, 10, &mut ctx); // focused 15 >= 10 -> reset to 0
        }
        assert_eq!(l.lv.range, 10);
        assert_eq!(l.lv.focused, 0, "focused reset when >= new range");
        assert_eq!(deferred.len(), 1);
        // setParams(focused=0, 0, aRange-1=9, preserve pg, preserve ar).
        assert!(matches!(
            deferred[0],
            Deferred::ScrollBarSetParams { id, value: Some(0), min: Some(0), max: Some(9), page_step: None, arrow_step: None }
                if id == v
        ));
    }

    #[test]
    fn set_range_keeps_focused_when_in_range() {
        let mut l = FakeList::new(Rect::new(0, 0, 10, 5), 1, items(20), None, None);
        l.lv.focused = 3;
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred: Vec<Deferred> = vec![];
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            set_range(&mut l, 10, &mut ctx);
        }
        assert_eq!(l.lv.focused, 3, "focused kept (still in range)");
        assert!(deferred.is_empty(), "no v-bar -> no params op");
    }

    // -- 5. update_steps -----------------------------------------------------

    #[test]
    fn update_steps_single_col_vbar_and_hbar() {
        let (_g, h) = mint_id();
        let (_g2, v) = mint_id();
        // size 12×5, numCols 1.
        let l = FakeList::new(Rect::new(0, 0, 12, 5), 1, items(5), Some(h), Some(v));
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred: Vec<Deferred> = vec![];
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            update_steps(&l, &mut ctx);
        }
        assert_eq!(deferred.len(), 2);
        // v-bar: numCols==1 -> pgStep = size.y-1 = 4, arStep = 1.
        assert!(matches!(
            deferred[0],
            Deferred::ScrollBarSetParams { id, value: None, min: None, max: None, page_step: Some(4), arrow_step: Some(1) }
                if id == v
        ));
        // h-bar: setStep(size.x/numCols = 12, 1).
        assert!(matches!(
            deferred[1],
            Deferred::ScrollBarSetParams { id, page_step: Some(12), arrow_step: Some(1), .. }
                if id == h
        ));
    }

    #[test]
    fn update_steps_multi_col_vbar() {
        let (_g2, v) = mint_id();
        // size 12×4, numCols 3.
        let l = FakeList::new(Rect::new(0, 0, 12, 4), 3, items(5), None, Some(v));
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred: Vec<Deferred> = vec![];
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            update_steps(&l, &mut ctx);
        }
        // v-bar: multi -> pgStep = size.y*numCols = 12, arStep = size.y = 4.
        assert!(matches!(
            deferred[0],
            Deferred::ScrollBarSetParams { id, page_step: Some(12), arrow_step: Some(4), .. }
                if id == v
        ));
    }

    // -- 7. handle_event nav / select / scrollbar filter ---------------------

    fn key_ev(k: Key) -> Event {
        Event::KeyDown(crate::event::KeyEvent::new(
            k,
            crate::event::KeyModifiers::default(),
        ))
    }

    #[test]
    fn key_down_and_up_move_focus_and_clear() {
        let mut l = FakeList::new(Rect::new(0, 0, 10, 5), 1, items(20), None, None);
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred = vec![];

        let mut ev = key_ev(Key::Down);
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            l.handle_event(&mut ev, &mut ctx);
        }
        assert_eq!(l.lv.focused, 1, "Down -> focused+1");
        assert!(ev.is_nothing(), "Down consumed");

        let mut ev = key_ev(Key::Up);
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            l.handle_event(&mut ev, &mut ctx);
        }
        assert_eq!(l.lv.focused, 0, "Up -> focused-1");
    }

    #[test]
    fn key_home_end_pgdn_pgup() {
        let mut l = FakeList::new(Rect::new(0, 0, 10, 5), 1, items(50), None, None);
        l.lv.top_item = 10;
        l.lv.focused = 12;
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred = vec![];

        // Home -> topItem (10).
        let mut ev = key_ev(Key::Home);
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            l.handle_event(&mut ev, &mut ctx);
        }
        assert_eq!(l.lv.focused, 10, "Home -> topItem");

        // End -> topItem + size.y*numCols - 1 = 10 + 5 - 1 = 14.
        let mut ev = key_ev(Key::End);
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            l.handle_event(&mut ev, &mut ctx);
        }
        assert_eq!(l.lv.focused, 14, "End -> topItem + page - 1");

        // PgDn -> focused + size.y*numCols = 14 + 5 = 19.
        let mut ev = key_ev(Key::PageDown);
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            l.handle_event(&mut ev, &mut ctx);
        }
        assert_eq!(l.lv.focused, 19, "PgDn -> +page");

        // PgUp -> focused - page = 19 - 5 = 14.
        let mut ev = key_ev(Key::PageUp);
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            l.handle_event(&mut ev, &mut ctx);
        }
        assert_eq!(l.lv.focused, 14, "PgUp -> -page");
    }

    #[test]
    fn key_ctrl_pgdn_pgup_jump_to_ends() {
        let mut l = FakeList::new(Rect::new(0, 0, 10, 5), 1, items(50), None, None);
        l.lv.focused = 20;
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred = vec![];

        let ctrl = crate::event::KeyModifiers {
            ctrl: true,
            ..Default::default()
        };
        // Ctrl+PgDn -> range-1 = 49.
        let mut ev = Event::KeyDown(crate::event::KeyEvent::new(Key::PageDown, ctrl));
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            l.handle_event(&mut ev, &mut ctx);
        }
        assert_eq!(l.lv.focused, 49, "Ctrl+PgDn -> range-1");

        // Ctrl+PgUp -> 0.
        let mut ev = Event::KeyDown(crate::event::KeyEvent::new(Key::PageUp, ctrl));
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            l.handle_event(&mut ev, &mut ctx);
        }
        assert_eq!(l.lv.focused, 0, "Ctrl+PgUp -> 0");
    }

    #[test]
    fn left_right_no_op_single_col_leaves_event_live() {
        let mut l = FakeList::new(Rect::new(0, 0, 10, 5), 1, items(20), None, None);
        l.lv.focused = 3;
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred = vec![];

        for k in [Key::Left, Key::Right] {
            let mut ev = key_ev(k);
            {
                let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
                l.handle_event(&mut ev, &mut ctx);
            }
            assert_eq!(l.lv.focused, 3, "{k:?} is a no-op when numCols==1");
            assert!(
                !ev.is_nothing(),
                "{k:?} leaves the event LIVE (C++ return, no clearEvent)"
            );
        }
    }

    #[test]
    fn left_right_move_by_size_y_multi_col() {
        let mut l = FakeList::new(Rect::new(0, 0, 12, 3), 2, items(40), None, None);
        l.lv.focused = 5;
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred = vec![];

        // Right -> focused + size.y = 5 + 3 = 8.
        let mut ev = key_ev(Key::Right);
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            l.handle_event(&mut ev, &mut ctx);
        }
        assert_eq!(l.lv.focused, 8, "Right -> +size.y (multi-col)");
        assert!(ev.is_nothing(), "Right consumed (multi-col)");

        // Left -> focused - size.y = 8 - 3 = 5.
        let mut ev = key_ev(Key::Left);
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            l.handle_event(&mut ev, &mut ctx);
        }
        assert_eq!(l.lv.focused, 5, "Left -> -size.y (multi-col)");
    }

    #[test]
    fn space_selects_focused_and_broadcasts() {
        // The list must have an id for the broadcast source; insert it.
        let mut group = Group::new(Rect::new(0, 0, 20, 10));
        let id = group.insert(Box::new(FakeList::new(
            Rect::new(0, 0, 10, 5),
            1,
            items(20),
            None,
            None,
        )));
        if let Some(v) = group.find_mut(id) {
            v.state_mut().state.focused = true;
        }
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred = vec![];

        let mut ev = key_ev(Key::Char(' '));
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            group.find_mut(id).unwrap().handle_event(&mut ev, &mut ctx);
        }
        // selectItem broadcasts cmListItemSelected sourced by the list.
        assert!(
            out.iter().any(|e| matches!(
                e,
                Event::Broadcast { command, source }
                    if *command == Command::LIST_ITEM_SELECTED && *source == Some(id)
            )),
            "Space broadcasts cmListItemSelected with self as source"
        );
        assert!(ev.is_nothing(), "Space consumed");
    }

    #[test]
    fn double_click_selects_item_under_cursor() {
        let mut group = Group::new(Rect::new(0, 0, 20, 10));
        let id = group.insert(Box::new(FakeList::new(
            Rect::new(0, 0, 10, 5),
            1,
            items(20),
            None,
            None,
        )));
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred = vec![];

        // Double-click at view-local (3, 2): newItem = 2 + 5*(3/11) + 0 = 2.
        let me = crate::event::MouseEvent {
            position: Point::new(3, 2),
            flags: crate::event::MouseEventFlags {
                double_click: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut ev = Event::MouseDown(me);
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            group.find_mut(id).unwrap().handle_event(&mut ev, &mut ctx);
        }
        // focusItemNum(2) -> focused 2; double-click + range > 2 -> selectItem(2).
        let focused = group
            .find_mut(id)
            .and_then(|v| v.as_any_mut())
            .and_then(|a| a.downcast_mut::<FakeList>())
            .map(|l| l.lv.focused)
            .unwrap();
        assert_eq!(focused, 2, "click positioned focus to item 2");
        assert!(
            out.iter().any(|e| matches!(
                e,
                Event::Broadcast { command, .. } if *command == Command::LIST_ITEM_SELECTED
            )),
            "double-click selects -> cmListItemSelected"
        );
        assert!(ev.is_nothing(), "mouse-down consumed");
    }

    #[test]
    fn scrollbar_changed_filter_requests_sync_only_for_own_bars() {
        let mut group = Group::new(Rect::new(0, 0, 30, 20));
        let (_gh, h) = mint_id();
        let (_gv, v) = mint_id();
        let id = group.insert(Box::new(FakeList::new(
            Rect::new(0, 0, 10, 5),
            1,
            items(20),
            Some(h),
            Some(v),
        )));
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred: Vec<Deferred> = vec![];

        // (a) CHANGED from own h-bar -> SyncListViewer queued.
        let mut ev = Event::Broadcast {
            command: Command::SCROLL_BAR_CHANGED,
            source: Some(h),
        };
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            group.find_mut(id).unwrap().handle_event(&mut ev, &mut ctx);
        }
        assert_eq!(deferred.len(), 1);
        assert!(matches!(
            deferred[0],
            Deferred::SyncListViewer { list, h: rh, v: rv }
                if list == id && rh == Some(h) && rv == Some(v)
        ));

        // (b) CHANGED from a foreign source -> nothing.
        deferred.clear();
        let mut ev = Event::Broadcast {
            command: Command::SCROLL_BAR_CHANGED,
            source: Some(id),
        };
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            group.find_mut(id).unwrap().handle_event(&mut ev, &mut ctx);
        }
        assert!(deferred.is_empty(), "foreign source ignored (filter bites)");

        // (c) CLICKED from own v-bar -> FocusById queued (select()).
        let mut ev = Event::Broadcast {
            command: Command::SCROLL_BAR_CLICKED,
            source: Some(v),
        };
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            group.find_mut(id).unwrap().handle_event(&mut ev, &mut ctx);
        }
        assert_eq!(deferred.len(), 1);
        assert!(matches!(deferred[0], Deferred::FocusById(fid) if fid == id));
    }

    // -- apply_scroll body ---------------------------------------------------

    #[test]
    fn apply_scroll_h_updates_indent_v_focuses() {
        let (_g, v) = mint_id();
        let mut l = FakeList::new(Rect::new(0, 0, 10, 5), 1, items(20), None, Some(v));
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred: Vec<Deferred> = vec![];
        {
            let mut ctx = make_ctx(&mut out, &mut timers, &mut deferred);
            apply_scroll(&mut l, Some(4), Some(8), &mut ctx);
        }
        assert_eq!(l.lv.indent, 4, "h branch refreshes the cached indent");
        assert_eq!(l.lv.focused, 8, "v branch focusItemNum(8)");
    }

    // -- focused_cursor (the &self setCursor successor) -----------------------

    #[test]
    fn focused_cursor_visible_and_offscreen() {
        // size 16×3, numCols 2 -> colWidth = 16/2 + 1 = 9. Items lay column-major:
        //   item = j*size.y + i + top_item.
        // With top_item = 0: col0 = items 0,1,2 at rows 0,1,2; col1 = items 3,4,5.
        let mut l = FakeList::new(Rect::new(0, 0, 16, 3), 2, items(20), None, None);
        l.lv.state.state.selected = true;
        l.lv.state.state.active = true;

        // (a) focused item 4 is in col j=1, row i=1 (4 = 1*3 + 1 + 0). Visible.
        //   x = j*col_width + 1 = 1*9 + 1 = 10; y = i = 1.
        // A column-major bug (e.g. row-major item = i*numCols + j) would put item 4
        // at a different cell, so the (10, 1) assertion bites.
        l.lv.focused = 4;
        assert_eq!(
            focused_cursor(&l),
            Some(Point::new(10, 1)),
            "focused item 4 -> col1 row1 -> view-local (10, 1)"
        );

        // (b) focused item 2 is col0 row2 -> (1, 2).
        l.lv.focused = 2;
        assert_eq!(
            focused_cursor(&l),
            Some(Point::new(1, 2)),
            "focused item 2 -> col0 row2 -> view-local (1, 2)"
        );

        // (c) focused item scrolled BELOW the visible page (a page = size.y*numCols
        //   = 6 items; with top_item 0 the visible items are 0..=5). Item 9 is off.
        l.lv.focused = 9;
        assert_eq!(focused_cursor(&l), None, "focused below page -> None");

        // (d) focused item scrolled ABOVE top_item.
        l.lv.top_item = 6; // visible items now 6..=11
        l.lv.focused = 3; // 3 < top_item -> not in the visible grid
        assert_eq!(focused_cursor(&l), None, "focused above top_item -> None");
    }

    // -- 6. draw snapshots ---------------------------------------------------

    fn render(l: &mut FakeList, w: u16, h: u16) -> String {
        let theme = Theme::classic_blue();
        let (backend, screen) = HeadlessBackend::new(w, h);
        let mut r = Renderer::new(Box::new(backend));
        r.render(|buf: &mut Buffer| {
            let bounds = l.state().get_bounds();
            let mut dc = DrawCtx::new(buf, &theme, bounds, bounds.a);
            l.draw(&mut dc);
        });
        screen.snapshot()
    }

    #[test]
    fn snapshot_single_col_active_focused_and_selected() {
        let mut l = FakeList::new(Rect::new(0, 0, 12, 4), 1, items(3), None, None);
        // Active list (selected + active) so focused/selected colors show.
        l.lv.state.state.selected = true;
        l.lv.state.state.active = true;
        l.lv.focused = 1;
        l.selected.insert(2); // item 2 explicitly selected
        insta::assert_snapshot!(render(&mut l, 12, 4));
    }

    #[test]
    fn snapshot_multi_col() {
        // size 16×3, numCols 2 -> colWidth = 16/2 + 1 = 9. 8 items laid column-
        // major: col0 = items 0,1,2; col1 = items 3,4,5.
        let mut l = FakeList::new(Rect::new(0, 0, 16, 3), 2, items(8), None, None);
        l.lv.state.state.selected = true;
        l.lv.state.state.active = true;
        insta::assert_snapshot!(render(&mut l, 16, 3));
    }

    #[test]
    fn snapshot_empty_shows_placeholder() {
        let mut l = FakeList::new(Rect::new(0, 0, 12, 3), 1, vec![], None, None);
        l.lv.state.state.selected = true;
        l.lv.state.state.active = true;
        insta::assert_snapshot!(render(&mut l, 12, 3));
    }
}
