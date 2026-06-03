//! `TMenuView::execute()` — the **modal layer**, flattened onto the single event
//! loop (D9) as one [`MenuSession`] capture handler (rows 50–52, Step-2 stage 1:
//! keyboard navigation).
//!
//! ## The architecture (settled in `docs/briefs/row50-52-menu-modal.md`)
//!
//! C++ `TMenuView::execute()` (`tmnuview.cpp:179`) is a nested `getEvent` loop:
//! opening a submenu **recurses** via `owner->execView(target)` (one nested modal
//! loop per open box level). We have a single event loop, so **one** `MenuSession`
//! capture handler owns the WHOLE open stack (bar + every open box) and runs the
//! flattened `execute()` for the entire interaction. While the session is on the
//! capture stack it **consumes every menu-directed event** (Clean Architecture A):
//! the bar and boxes are pure `draw`/`get_item_rect` — never focused, no event
//! logic of their own runs.
//!
//! ## State
//!
//! The session holds a **stack of levels** ([`MenuLevel`]), one per open
//! bar/box, each carrying its `view_id`, a **clone** of its `menu` (clone-at-open
//! is faithful — `execute()` has no `evBroadcast` case, so `disabled` is frozen
//! for the menu's lifetime; the session swallows `evBroadcast` while active), its
//! `current` highlight index, its `bounds` (cached at open — a box never moves),
//! and `is_bar` (the C++ `size.y == 1` discriminator that splits nearly every
//! `execute()` switch arm). The top of the stack is the **active** level (the one
//! C++ `execute()` frame currently running its `do/while`); a parent level is
//! "suspended" in its loop exactly as a C++ frame is across the nested
//! `execView`. The [`bounds`](MenuLevel::bounds) is shaped for stage-2 mouse from
//! day one (cross-level mouse gates against this cached per-level set).
//!
//! ## What is implemented (Step-2 stage 1 — keyboard)
//!
//! The **keyboard** arms of `execute()`'s switch: `kbUp`/`kbDown`/`kbLeft`/
//! `kbRight`/`kbHome`/`kbEnd`/`kbEnter`/`kbEsc` plus the `default:` char /
//! alt-shortcut / hotKey arm, with `trackKey`/`nextItem`/`prevItem`/`findItem`/
//! `findAltShortcut`. Submenu recursion pushes a level; command selection ends the
//! session and posts the command; Esc/left close levels.
//!
//! ## What is deferred to stage 2 (mouse) — breadcrumbed, NOT implemented
//!
//! The `evMouseDown`/`evMouseUp`/`evMouseMove` arms, `trackMouse`/`mouseInOwner`/
//! `mouseInMenus`/`autoSelect`/`lastTargetItem`/`putClickEventOnExit`, and the
//! `evMouseDown`-activation branch in [`menu_view::handle_event`]. The structs
//! already hold per-level `bounds` so the mouse gate drops in without a rewrite.
//! Stage 3 is `TMenuPopup` (row 52).

use crate::capture::{CaptureFlow, CaptureHandler};
use crate::command::Command;
use crate::event::{Event, Key, KeyEvent};
use crate::menu::menu_box::menu_box_rect;
use crate::menu::menu_view::hot_key;
use crate::menu::{Menu, MenuItem};
use crate::view::{Context, Point, Rect, ViewId};

/// One open bar/box level of the menu stack — the per-frame state of a C++
/// `execute()` invocation (`tmnuview.cpp:179`), made explicit so the single loop
/// can own all frames at once.
struct MenuLevel {
    /// The bar/box view's id in the root group (resolves to a
    /// [`MenuBar`](crate::menu::MenuBar)/[`MenuBox`](crate::menu::MenuBox) for the
    /// `SetMenuCurrent`/`Close` brokers). The bar's id is real; each box id is
    /// **pre-minted** by the session before [`Deferred::OpenMenuBox`].
    view_id: ViewId,
    /// A clone of the level's menu (`TMenuView::menu`). Clone-at-open is faithful:
    /// `execute()` ignores `evBroadcast`, so `disabled` is frozen for the menu's
    /// lifetime.
    menu: Menu,
    /// `TMenuView::current` — the highlighted item index, or `None` (C++
    /// `current == 0`).
    current: Option<usize>,
    /// The level's bounds in the root group's frame, cached at open (a box never
    /// moves). The bar's bounds; each box's computed bounds. Used to compute a
    /// child box's geometry (`getItemRect` + origin), and — stage 2 — to gate
    /// mouse.
    bounds: Rect,
    /// `size.y == 1` (C++ the bar/box discriminator). The bar is a one-row
    /// horizontal strip; a box is a vertical column.
    is_bar: bool,
    /// `execute()`'s `autoSelect` loop-local — **per level** (C++ inits it
    /// `False` at every `execute()` frame entry, `tmnuview.cpp:181`, so it never
    /// leaks from the bar's frame into a box's navigation). When `True`, a
    /// `doNothing` step whose `current` names a submenu opens that submenu (the
    /// open-gate `(doSelect || (doNothing && autoSelect))`, `tmnuview.cpp:368`).
    /// Set `True` on this level's bar kbDown / kbEnter / alt-char match; reset to
    /// `False` only by `cmMenu` (`tmnuview.cpp:346`). It is what makes a Left/Right
    /// walk along the bar **re-open** the adjacent title's box (Blocker 3).
    auto_select: bool,
}

impl MenuLevel {
    /// `getItemRect(index)` for this level, in **view-local** coordinates — the
    /// same contract as [`MenuBar::get_item_rect`](crate::menu::MenuView::get_item_rect)
    /// / [`MenuBox::get_item_rect`](crate::menu::MenuView::get_item_rect), but
    /// computed from the cached `menu` + `bounds` (the session has no view
    /// reference, D3). Must agree cell-for-cell with the draw layer.
    fn item_rect_local(&self, index: usize) -> Rect {
        if self.is_bar {
            // TMenuBar::getItemRect (tmenubar.cpp:94): horizontal accumulator.
            let mut r = Rect::new(1, 0, 1, 1);
            for (i, item) in self.menu.items.iter().enumerate() {
                r.a.x = r.b.x;
                if !matches!(item, MenuItem::Separator) {
                    r.b.x += cstrlen(item_name(item)) + 2;
                }
                if i == index {
                    return r;
                }
            }
            r
        } else {
            // TMenuBox::getItemRect (tmenubox.cpp:125): rows from y = 1.
            let y = 1 + index as i32;
            let size_x = self.bounds.b.x - self.bounds.a.x;
            Rect::new(2, y, size_x - 2, y + 1)
        }
    }
}

/// `cstrlen` — display width ignoring `~` markers (per-module copy, as in
/// `menu_bar.rs`/`menu_box.rs`).
fn cstrlen(s: &str) -> i32 {
    s.chars()
        .filter(|&c| c != '~')
        .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(1) as i32)
        .sum()
}

/// The display label of a named item (empty for a [`Separator`](MenuItem::Separator)).
fn item_name(item: &MenuItem) -> &str {
    match item {
        MenuItem::Command { name, .. } | MenuItem::SubMenu { name, .. } => name,
        MenuItem::Separator => "",
    }
}

/// `TMenuView::execute()` flattened onto the capture stack — the modal menu
/// interaction (rows 50–52, keyboard stage).
///
/// Pushed at activation (via [`Deferred::PushCapture`](crate::view::Deferred::PushCapture))
/// alongside the first [`OpenMenuBox`](crate::view::Deferred::OpenMenuBox) — or,
/// for a bar activation, with only the bar level on the stack and no box yet (the
/// first key opens a box). Consumes every event it is offered; pops itself
/// ([`CaptureFlow::ConsumedPop`]) when the last level closes, restoring the
/// pre-menu focus.
pub struct MenuSession {
    /// The open levels, bottom (bar) → top (deepest box). The top is the active
    /// level (the running C++ `execute()` frame).
    levels: Vec<MenuLevel>,
    /// The owner (root group) size — C++ `owner->size`, used as the bounds-hint
    /// `b` corner when sizing a submenu box (`tmnuview.cpp:379`). Captured at
    /// activation.
    owner_size: Point,
}

/// What a single `execute()` step decided — the C++ `menuAction` enum
/// (`tmnuview.cpp:177`), used internally to drive the post-switch logic.
#[derive(PartialEq)]
enum MenuAction {
    /// `doNothing` — stay open, redraw if `current` changed.
    Nothing,
    /// `doSelect` — try to open the current submenu, or select its command.
    Select,
    /// `doReturn` — this level returns (close it; if it was the bar, end session).
    Return,
}

impl MenuSession {
    /// Build a session over an initial level stack. `owner_size` is the root group
    /// size (`owner->size`). Use [`activate`] rather than calling this directly —
    /// it assembles the level + the first deferred batch.
    ///
    /// There is no `save_focus`: under Clean Architecture A the bar and boxes are
    /// **never focused** (the session consumes every event on the capture stack,
    /// before view-tree routing), so the pre-menu `current` is never disturbed and
    /// there is nothing to restore on close (the C++ `execView` focus save/restore
    /// is moot here).
    fn new(levels: Vec<MenuLevel>, owner_size: Point) -> Self {
        MenuSession { levels, owner_size }
    }

    /// The active (top) level — the running `execute()` frame. The session is never
    /// empty while on the stack (popping the last level returns `ConsumedPop`).
    fn top(&self) -> &MenuLevel {
        self.levels.last().expect("session has at least one level")
    }

    fn top_mut(&mut self) -> &mut MenuLevel {
        self.levels
            .last_mut()
            .expect("session has at least one level")
    }

    // -- nav primitives (tmnuview.cpp:111-146) ------------------------------

    /// `TMenuView::nextItem` (`tmnuview.cpp:111`) on the active level — advance
    /// `current`, wrapping to the head at the end. `current == None` (C++ `0`)
    /// bootstraps to the head; an empty menu stays `None`.
    fn next_item(&mut self) {
        let n = self.top().menu.items.len();
        if n == 0 {
            return;
        }
        let cur = self.top().current;
        let next = match cur {
            None => 0,
            Some(i) if i + 1 >= n => 0, // (current = current->next) == 0 → head
            Some(i) => i + 1,
        };
        self.top_mut().current = Some(next);
    }

    /// `TMenuView::prevItem` (`tmnuview.cpp:117`) on the active level. C++
    /// implements it *via* `nextItem` (walk forward until the next wraps to the
    /// old position); we match the **result** (the predecessor, wrapping the head
    /// to the tail) directly. `current == None` → tail (C++ `p = 0` makes the
    /// `do/while` run until `current->next == 0`, i.e. `current` is the last item).
    fn prev_item(&mut self) {
        let n = self.top().menu.items.len();
        if n == 0 {
            return;
        }
        let cur = self.top().current;
        let prev = match cur {
            None => n - 1,
            Some(0) => n - 1, // head → wrap to tail
            Some(i) => i - 1,
        };
        self.top_mut().current = Some(prev);
    }

    /// `TMenuView::trackKey(findNext)` (`tmnuview.cpp:129`) on the active level —
    /// move to the next/previous **non-separator** item.
    ///
    /// Faithful: the `current == 0` bootstrap (head, then `prevItem` if going
    /// backward, returning immediately if the landed item is named), then the
    /// `do { next/prev } while name == 0` separator skip.
    fn track_key(&mut self, find_next: bool) {
        if self.top().current.is_none() {
            self.top_mut().current = Some(0);
            if !find_next {
                self.prev_item();
            }
            // if current->name != 0 return (a named head/tail needs no skip).
            if !self.current_is_separator() {
                return;
            }
        }
        // do { next/prev } while( current->name == 0 ).
        loop {
            if find_next {
                self.next_item();
            } else {
                self.prev_item();
            }
            if !self.current_is_separator() {
                break;
            }
        }
    }

    /// Whether the active level's `current` points at a separator (C++
    /// `current->name == 0`). An out-of-range / `None` current is treated as
    /// non-separator so the loops terminate (the menu is assumed non-empty when a
    /// nav key arrives, faithful to C++ which never tracks an empty menu).
    fn current_is_separator(&self) -> bool {
        match self.top().current {
            Some(i) => matches!(self.top().menu.items.get(i), Some(MenuItem::Separator)),
            None => false,
        }
    }

    /// `TMenuView::findItem(ch)` (`tmnuview.cpp:420`) on the active level — the
    /// first **enabled, named** item whose hotkey letter matches a plain (no-alt)
    /// `ke`. Delegates to the shared [`menu_view::matching_item`] walk.
    fn find_item(&self, ke: &KeyEvent) -> Option<usize> {
        crate::menu::menu_view::matching_item(&self.top().menu, ke, false)
    }

    // -- the per-event step = one iteration of execute()'s do/while ---------

    /// One `execute()` switch pass on the active level (keyboard arms). Returns the
    /// `(action, cleared)` pair: `cleared` is the C++ `clearEvent(e)` bit — when
    /// `false` and `action == Return`, the re-apply loop re-delivers the SAME event
    /// to the parent level (the flattening of `execute()`'s
    /// `putEvent(e)`→parent-`getEvent` tail, `tmnuview.cpp:401-405`). The arm may
    /// mutate the level's `current`/`auto_select`. `pending_command` carries a
    /// hotKey accelerator result.
    fn step_keyboard(
        &mut self,
        k: KeyEvent,
        pending_command: &mut Option<Command>,
    ) -> (MenuAction, bool) {
        let is_bar = self.top().is_bar;
        match k.key {
            // kbUp / kbDown (tmnuview.cpp:280): box navigates; bar's kbDown sets
            // autoSelect = True (the open-gate then opens the current submenu —
            // action stays doNothing so the flag PERSISTS for a later Left/Right
            // walk, Blocker 3). A non-named key consumes (cleared) the event.
            Key::Up | Key::Down => {
                if !is_bar {
                    self.track_key(k.key == Key::Down);
                } else if k.key == Key::Down {
                    self.top_mut().auto_select = true;
                }
                (MenuAction::Nothing, true)
            }
            // kbLeft / kbRight (tmnuview.cpp:287): bar trackKeys to the adjacent
            // title; a box (parentMenu != 0) returns WITHOUT clearEvent → the
            // re-apply loop unwinds every open box to the bar, which then walks +
            // re-opens the neighbour (Blocker 3).
            Key::Left | Key::Right => {
                if is_bar {
                    self.track_key(k.key == Key::Right);
                    (MenuAction::Nothing, true)
                } else {
                    // parentMenu != 0 (always, a box has the bar/another box above)
                    // → doReturn, NOT cleared.
                    (MenuAction::Return, false)
                }
            }
            // kbHome / kbEnd (tmnuview.cpp:294): box only.
            Key::Home | Key::End => {
                if !is_bar {
                    self.top_mut().current = Some(0);
                    if k.key == Key::End {
                        self.track_key(false);
                    }
                }
                (MenuAction::Nothing, true)
            }
            // kbEnter (tmnuview.cpp:303): doSelect; the bar also sets autoSelect.
            Key::Enter => {
                if is_bar {
                    self.top_mut().auto_select = true;
                }
                (MenuAction::Select, true)
            }
            // kbEsc (tmnuview.cpp:308-312): doReturn. clearEvent runs iff
            // `parentMenu == 0 || parentMenu->size.y != 1` — i.e. cleared at the
            // bar OR at a 2nd+-level box (parent is a box), but NOT at a 1st-level
            // box (parent is the bar, size.y == 1). When not cleared the re-apply
            // loop carries the Esc up to the bar, closing the whole menu on one
            // press (Blocker 2). The asymmetry IS this guard, not a mouse concern.
            Key::Esc => {
                let cleared = self.esc_clear_event();
                (MenuAction::Return, cleared)
            }
            // default (tmnuview.cpp:313): alt-shortcut on the TOP menu, else a
            // plain char findItem, else a hotKey accelerator.
            _ => self.step_default_key(k, pending_command),
        }
    }

    /// The C++ kbEsc `clearEvent` guard (`tmnuview.cpp:310`):
    /// `parentMenu == 0 || parentMenu->size.y != 1`. The active level's parent is
    /// the level below it; `parentMenu == 0` is the bar (no level below).
    fn esc_clear_event(&self) -> bool {
        let depth = self.levels.len();
        if depth <= 1 {
            // The bar: parentMenu == 0 → cleared.
            true
        } else {
            // A box: parent is levels[depth-2]. Cleared iff the parent is NOT the
            // bar (a 2nd+-level box), i.e. parent.is_bar == false.
            !self.levels[depth - 2].is_bar
        }
    }

    /// The `default:` arm of `execute()`'s evKeyDown switch (`tmnuview.cpp:313`),
    /// keyboard subset. Returns `(action, cleared)`.
    fn step_default_key(
        &mut self,
        k: KeyEvent,
        pending_command: &mut Option<Command>,
    ) -> (MenuAction, bool) {
        // C++: target = this; if Alt-char, target = topMenu(), p = findAltShortcut
        // on the bar; else p = findItem on THIS level.
        if k.modifiers.alt {
            // Alt-shortcut dispatches against the TOP menu (the bar, level 0).
            if let Some(idx) = self.find_alt_shortcut_bar(&k) {
                // C++ `tmnuview.cpp:331-340`: if target == this (the active level IS
                // the bar) → if size.y==1 autoSelect=True; doSelect; current=p.
                // Otherwise (a box is active) → doReturn (not cleared) so the
                // re-apply loop unwinds toward the bar, which re-resolves.
                if self.top().is_bar {
                    self.top_mut().current = Some(idx);
                    self.top_mut().auto_select = true;
                    return (MenuAction::Select, true);
                } else {
                    return (MenuAction::Return, false);
                }
            }
        } else if let Some(idx) = self.find_item(&k) {
            // findItem matched on THIS (active) level → target == this → select it.
            // (size.y==1 → autoSelect=True, harmless on a box where it is unused.)
            self.top_mut().current = Some(idx);
            if self.top().is_bar {
                self.top_mut().auto_select = true;
            }
            return (MenuAction::Select, true);
        }
        // No item match: try the bar's hotKey accelerator (topMenu()->hotKey).
        if let Some(cmd) = hot_key(&self.levels[0].menu, k) {
            // commandEnabled is backstopped by the pump's drop_disabled filter
            // (row 49); hot_key already skips cached-disabled items. The result
            // path ends the session (clearEvent runs, tmnuview.cpp:395).
            *pending_command = Some(cmd);
            return (MenuAction::Return, true);
        }
        // No match at all: consume (a stray key in a modal menu does nothing).
        (MenuAction::Nothing, true)
    }

    /// `findAltShortcut` against the **bar** (`topMenu()`, `tmnuview.cpp:436`) — the
    /// matched top-level item index, if any (alt-char path). Delegates to the
    /// shared [`menu_view::matching_item`] walk.
    fn find_alt_shortcut_bar(&self, ke: &KeyEvent) -> Option<usize> {
        crate::menu::menu_view::matching_item(&self.levels[0].menu, ke, true)
    }

    /// The flattened keyboard event loop — the heart of the fix. Steps the active
    /// level, runs the post-switch open-gate, and on a non-cleared `doReturn` pops
    /// the level and **re-applies the SAME event** to the new top level, looping
    /// until a level produces a non-Return action (or a cleared Return), or the bar
    /// ends the whole session. This is the faithful flattening of C++ `execute()`'s
    /// nested `execView` re-post (`tmnuview.cpp:401-405`: `putEvent(e)` →
    /// parent-`getEvent`).
    fn handle_key(&mut self, k: KeyEvent, ctx: &mut Context) -> CaptureFlow {
        loop {
            let mut pending_command = None;
            let (action, cleared) = self.step_keyboard(k, &mut pending_command);

            // Post the (possibly changed) highlight of the active level to its view
            // (execute()'s `if itemShown != current drawView`, tmnuview.cpp:362).
            let top_id = self.top().view_id;
            let top_current = self.top().current;
            ctx.request_set_menu_current(top_id, top_current);

            // Post-switch open-gate (tmnuview.cpp:368-390):
            //   (doSelect || (doNothing && autoSelect)) && current names a NAMED
            //   item → open its submenu (any of the two), or select its command
            //   (doSelect only).
            let auto = self.top().auto_select;
            let gate = action == MenuAction::Select || (action == MenuAction::Nothing && auto);
            if gate && let Some(idx) = self.top().current {
                match self.top().menu.items.get(idx) {
                    // A submenu, not disabled → open a child box (recurse).
                    Some(MenuItem::SubMenu { menu, disabled, .. }) if !*disabled => {
                        let submenu = menu.clone();
                        self.open_submenu(idx, submenu, ctx);
                        return CaptureFlow::Consumed;
                    }
                    // A command item, not disabled → select it ONLY on doSelect
                    // (the autoSelect branch never selects a command,
                    // tmnuview.cpp:388). Post + end the whole session.
                    Some(MenuItem::Command {
                        command, disabled, ..
                    }) if !*disabled && action == MenuAction::Select => {
                        let cmd = *command;
                        return self.end_session_with(Some(cmd), ctx);
                    }
                    _ => {}
                }
            }

            // A hotKey accelerator (`topMenu()->hotKey`) is a COMMAND RESULT: it
            // propagates up through every nested execView and closes the WHOLE
            // menu, posting the command, regardless of depth (`tmnuview.cpp:392`).
            // Check it BEFORE the per-level Return-pop, else a deep hotKey would be
            // dropped (the box-level pop returns Consumed without posting).
            // Esc/Left/Right carry no pending_command, so they fall through.
            if let Some(cmd) = pending_command {
                return self.end_session_with(Some(cmd), ctx);
            }

            // doReturn — close the active level; re-apply upward unless cleared.
            if action == MenuAction::Return {
                if self.levels.len() > 1 {
                    // Pop + close the top box; the parent becomes active.
                    let top = self.levels.pop().expect("len > 1");
                    ctx.request_close(top.view_id);
                    if cleared {
                        // clearEvent → stop; the parent stays open.
                        return CaptureFlow::Consumed;
                    }
                    // Not cleared → re-apply the SAME event to the new top level.
                    continue;
                } else {
                    // The bar returned (Esc at the bar) → end the session.
                    return self.end_session_with(None, ctx);
                }
            }

            // doNothing with no open → consume; the active level stays open.
            return CaptureFlow::Consumed;
        }
    }

    /// Open the submenu at `index` of the active level as a new child box level
    /// (the C++ `execute()` submenu-open block, `tmnuview.cpp:368-387`, +
    /// `newSubView`/`execView` recursion). Pre-mints the box id, computes its
    /// geometry, and queues [`OpenMenuBox`](crate::view::Deferred::OpenMenuBox).
    fn open_submenu(&mut self, index: usize, submenu: Menu, ctx: &mut Context) {
        // Geometry block (tmnuview.cpp:376-381):
        //   r = getItemRect(current);          // view-local
        //   r.a.x = r.a.x + origin.x;
        //   r.a.y = r.b.y + origin.y;          // BELOW the item
        //   r.b = owner->size;
        //   if (size.y == 1) r.a.x--;          // bar shift
        let parent = self.top();
        let origin = parent.bounds.a;
        let r = parent.item_rect_local(index);
        let mut hint = Rect::new(
            r.a.x + origin.x,
            r.b.y + origin.y,
            self.owner_size.x,
            self.owner_size.y,
        );
        if parent.is_bar {
            hint.a.x -= 1;
        }
        // The box sizes itself inside this hint (menu_box_rect clamps).
        let bounds = menu_box_rect(hint, &submenu);

        // Pre-mint the box id so the session knows it with no callback.
        let id = ViewId::next();
        ctx.request_open_menu_box(id, submenu.clone(), bounds);

        // The new box level starts with current = menu->deflt (execute()'s
        // prologue runs on the freshly entered frame: `current = menu->deflt`) and
        // its OWN autoSelect = False (per-level; C++ inits it False each frame).
        let current = submenu.default;
        self.levels.push(MenuLevel {
            view_id: id,
            menu: submenu,
            current,
            bounds,
            is_bar: false,
            auto_select: false,
        });
        // Push the new level's initial highlight to its (about-to-exist) box.
        ctx.request_set_menu_current(id, current);
    }

    /// End the whole session: close every open box, clear the bar's highlight,
    /// restore focus, optionally post `cmd`, and pop the capture handler.
    fn end_session_with(&mut self, cmd: Option<Command>, ctx: &mut Context) -> CaptureFlow {
        // Close every open box level (the bar is NOT a session-owned box — it is a
        // permanent child, so it is only un-highlighted, not removed).
        for level in self.levels.iter().skip(1) {
            ctx.request_close(level.view_id);
        }
        // Clear the bar's highlight (execute()'s tail: current = 0; drawView()).
        if let Some(bar) = self.levels.first() {
            ctx.request_set_menu_current(bar.view_id, None);
        }
        // Post the selected command, if any (the pump's drop_disabled filter is
        // the backstop for a stale-enabled command).
        if let Some(cmd) = cmd {
            ctx.post(cmd);
        }
        // No focus restore: focus was never moved (Clean Architecture A — boxes
        // and the bar are never current), so the pre-menu current is intact.
        CaptureFlow::ConsumedPop
    }
}

impl CaptureHandler for MenuSession {
    /// The flattened `execute()` `do { getEvent; switch } while` body — one pass
    /// per offered event. Consumes every menu-directed event (Clean Architecture
    /// A). Keyboard navigation only (stage 1).
    fn handle(&mut self, ev: &mut Event, ctx: &mut Context) -> CaptureFlow {
        match *ev {
            Event::KeyDown(k) => self.handle_key(k, ctx),
            // evCommand cmMenu while active (tmnuview.cpp:343-350): autoSelect =
            // False, lastTargetItem = 0; if parentMenu != 0 → doReturn. On the bar
            // (the only active level reachable in stage 1) it just resets
            // autoSelect and stays open (no box re-opens on the next nav until a
            // fresh kbDown). We reset the active level's auto_select and consume.
            Event::Command(Command::MENU) => {
                self.top_mut().auto_select = false;
                CaptureFlow::Consumed
            }
            // A non-cmMenu command → doReturn (close the whole menu). C++
            // execute()'s tail re-posts the command (`putEvent(e)` when
            // `e.what == evCommand`, tmnuview.cpp:403-405) so it still reaches the
            // view after the menu closes — port that with put_event.
            Event::Command(cmd) => {
                let r = self.end_session_with(None, ctx);
                ctx.put_event(Event::Command(cmd));
                r
            }
            // evBroadcast: SWALLOWED while active (clone-at-open is faithful —
            // execute() has no evBroadcast case; a cmCommandSetChanged is fetched
            // and ignored, so disabled stays frozen and boxes never regray
            // mid-menu). Consume so it does not reach the (idle) menu broker.
            Event::Broadcast { .. } => CaptureFlow::Consumed,
            // Stage 2 (mouse): evMouseDown / evMouseUp / evMouseMove +
            // trackMouse / mouseInOwner / mouseInMenus / autoSelect /
            // lastTargetItem / putClickEventOnExit + click-outside-close. The
            // per-level `bounds` are already cached for the gate. For stage 1 we
            // consume mouse events (the session is modal: nothing beneath it sees
            // them) but take no action.
            Event::MouseDown(_) | Event::MouseUp(_) | Event::MouseMove(_) | Event::MouseAuto(_) => {
                CaptureFlow::Consumed
            }
            // Anything else (Timer, Nothing): consume to keep the session modal.
            _ => CaptureFlow::Consumed,
        }
    }

    fn view(&self) -> Option<ViewId> {
        // The session is associated with the bar (level 0). Bounds gating is
        // stage 2 and uses the per-level cache, not set_gate_bounds (boxes never
        // move), so this is informational only.
        self.levels.first().map(|l| l.view_id)
    }
}

// ---------------------------------------------------------------------------
// Activation — assemble a MenuSession + its first deferred batch
// ---------------------------------------------------------------------------

/// Open a menu session from the **bar**.
///
/// Two activation kinds, distinguished by `open_index`:
///
/// * **`cmMenu` / kbF10** (`open_index == None`, `tmnuview.cpp:343-350`): the C++
///   prologue sets `current = menu->deflt` and the re-posted `cmMenu` hits the
///   `evCommand cmMenu` arm → `autoSelect = False`, `parentMenu == 0` so `action`
///   stays `doNothing` → the open-gate is **false** → **no box opens**; F10 only
///   highlights the default title and waits (Blocker 1). So we set the bar's
///   `current = deflt`, `auto_select = false`, and open NO box.
/// * **Alt-shortcut** (`open_index == Some(idx)`, `tmnuview.cpp:331-340`): the
///   default-key arm sets `current = p`, `autoSelect = True`, `doSelect` → opens
///   the matched title's box in the SAME deferred batch (no dead first event), and
///   `auto_select` persists so a later Left/Right re-opens neighbours (Blocker 3).
///
/// `bar_menu` is a clone of the bar's `menu`; `bar_bounds` its bounds in the root
/// frame; `owner_size` the root group size.
pub fn activate(
    bar_id: ViewId,
    bar_menu: Menu,
    bar_bounds: Rect,
    owner_size: Point,
    open_index: Option<usize>,
    ctx: &mut Context,
) {
    // The bar's initial highlight: the matched item (Alt-shortcut) or the menu
    // default (cmMenu / kbF10).
    let initial = open_index.or(bar_menu.default);
    // autoSelect is True only for the Alt-shortcut path (it doSelects); cmMenu
    // resets it to False (Blocker 1 / 3).
    let auto_select = open_index.is_some();

    // Alt-shortcut to a top-level COMMAND item (`tmnuview.cpp:388`: doSelect on a
    // command → result = command): post it and open NO session (the menu never
    // appears, faithful to execView returning the command immediately).
    if let Some(idx) = open_index
        && let Some(MenuItem::Command {
            command, disabled, ..
        }) = bar_menu.items.get(idx)
        && !*disabled
    {
        ctx.post(*command);
        return;
    }

    let bar_level = MenuLevel {
        view_id: bar_id,
        menu: bar_menu.clone(),
        current: initial,
        bounds: bar_bounds,
        is_bar: true,
        auto_select,
    };
    let mut session = MenuSession::new(vec![bar_level], owner_size);

    // Push the bar's initial highlight for draw.
    ctx.request_set_menu_current(bar_id, initial);

    // Open the first box ONLY for the Alt-shortcut path (open_index is Some) — NOT
    // for cmMenu, which only highlights the default (Blocker 1). The matched item
    // must name a non-disabled submenu.
    if let Some(idx) = open_index
        && let Some(MenuItem::SubMenu { menu, disabled, .. }) = bar_menu.items.get(idx)
        && !*disabled
    {
        let submenu = menu.clone();
        session.open_submenu(idx, submenu, ctx);
    }

    ctx.push_capture(Box::new(session));
}
