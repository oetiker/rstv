# Gap report — actionable code backlog

Derived from `reference/*.md`. See [`README.md`](README.md) for the axes. Back to [coverage-matrix](coverage-matrix.md).

**Summary:** 1 missing · 4 wrong (suspect) · 461 deliberately not-ported (register below). Each missing/wrong item below is a candidate fix; the register is the do-not-re-flag list.

## 1. Missing — capability the guide documents that has no counterpart

### TMenuView — `GetHelpCtx` (method) (guide p. 484)
CANDIDATE GAP — needs human confirmation. Guide: a dropped menu's `getHelpCtx` walks the `parentMenu` chain to return the highlighted item's `helpCtx`, so the status line shows per-item help while navigating a menu. No `get_help_ctx` override on any menu type; `Group::get_help_ctx` bubbles to the *current child*, but an open menu is a `MenuSession` modal capture, not a focused menu-view subtree, so the highlighted **item's** help context is never surfaced. The `parentMenu` chain in `menu_session.rs` is used only for event routing, not help-context. Behavior appears not ported.

## 2. Wrong — present but diverges from the 1992 spec (`SUSPECT`)

### TApplication — `WriteShellMsg` (virtual method) (guide p. 381)
- Rust: `println!` inline in `program_handle_event` (`src/app/program.rs:3319`)
- Guide: virtual procedure; default prints "Type EXIT to return..." (DOS) or the SIGTSTP message (unix). Rust inlines the print statement directly in `program_handle_event` rather than exposing a virtual/overridable hook. The printed text matches the magiblot unix branch. However, the virtual override point is **not preserved** — user code cannot customize the shell message without forking the crate. This is an undocumented loss of extensibility. Not a behavior-correctness bug (message text is correct) but a deliberate API reduction that is not called out in any D-rule or comment. SUSPECT on the "intentional deviation not documented" axis.

### TListViewer — `handleEvent` (method) (guide p. 472)
- Rust: `tv::list_viewer::handle_event(this, ev, ctx)` free function
- C++ event loop: mouse hold runs as a do-while polling `mouseEvent` inside `evMouseDown`. Rust replaces the loop with a capture-based state machine (D3 broker): `MouseDown` arms a `MouseTrackCapture`; pump delivers `MouseMove`/`MouseAuto`/`MouseUp` events. Behavior matches. **SUSPECT**: C++ scrollbar-changed broadcast reads `hScrollBar->value` directly inline (line 347: `focusItemNum(vScrollBar->value)`); Rust defers to a `SyncListViewer` op (pump broker). This is documented (D3) — so NOT suspect on that point. **However**: C++ `TView::handleEvent(event)` is called first (line 221: `TView::handleEvent(event)`); Rust does NOT call any base `handle_event` at the start of its free function — the `View` trait has no base `handle_event` equivalent for this. This is undocumented. In practice the C++ `TView::handleEvent` only processes `cmReceivedFocus`/`cmReleasedFocus` broadcasts (to auto-select the view); tvision-rs handles focus selection in `Group` instead, so it is likely intentional — but not commented. Flag SUSPECT until confirmed.

### TListViewer — `setState` (method) (guide p. 473)
- Rust: `tv::list_viewer::set_state(this, flag, enable, ctx)` free function
- C++ checks `(aState & (sfSelected | sfActive | sfVisible)) != 0` to decide whether to show/hide scroll bars. Rust checks only `flag == Active || flag == Selected` — the `Visible` arm is **missing**. Consequence: if a list viewer is hidden/shown via `sfVisible` alone (without an accompanying Active/Selected change), its scroll bars will not track the visibility change. This deviation is **undocumented**. Doc score 2 because the existing doc describes what the function does without noting this gap.

### TSortedListBox — `setData` / `set_value_ctx` (scatter) (guide p. 535)
- Rust: `SortedListBox::value()` exists (gather); `set_value_ctx` is **not** implemented
- C++ inherits `setData` from `TListBox` (focuses the item via `focusItem(p->selection)`). Rust `SortedListBox` overrides `View::value` for gather but not `set_value_ctx`, so the default no-op runs — a dialog scatter to a sorted list box silently fails to focus the target item (`ListBox`, by contrast, implements it at `src/widgets/list_box.rs:164`). Undocumented; may be intentional (sorted selection is search-driven) but should be confirmed.

## 2b. Secondary observations (prose-flagged; not classified MISSING/SUSPECT)

These were noted by auditors in passing — undocumented idiomatic deviations or latent enhancements, not confirmed bugs. Listed for the follow-up fix pass to triage.

- **TProgram** — window-insert `CanMoveFocus`/`ValidView` guard not applied at `desktop_insert` (C++ disposes a window if the active one can't release focus on insert; Rust enforces the gate only on Alt-N selection / modal close).
- **idle-time / background processing** (Part 2 sweep) — no user-facing `Idle`/`on_idle` seam; an app cannot run periodic work each idle pass (guide clock/heap-display pattern). Closely related: no `override getEvent` seam to inject an event source.
- **TLabel / ShowMarkers / SpecialChars** — the monochrome column-0 focus marker glyph is never rendered (also surfaced in Globals-363-378).
- **Editor find/replace + flags** — per-instance in Rust vs C++ class-static (shared across editors); deliberate but undocumented.
- **TInputLine** — no post-construction `set_validator` (validator is constructor-only); deliberate ownership choice, undocumented.
- **TMenu/MenuBuilder** — `submenu()`/`command()` hardcode `HelpCtx::NO_CONTEXT` with no escape hatch.
- **TStringLookupValidator** — `lookup` is linear scan over an unsorted `Vec` vs C++ binary search over a sorted collection (O(n) vs O(log n)); `new_string_list(nil)` free-vs-replace semantics differ.
- **TMonoSelector** — no user-facing picker for the mono attributes (only mattered inside the superseded `TColorDialog`).

## 3. NOT-PORTED register — intentional omissions (do not re-flag)

All 461 entries carry a written reason in their per-section file. Grouped by theme; each section link in the matrix has the per-entry reasons.

### TStreamable / object streaming — 150 entries
Globals-347-362×2, Globals-363-378×8, Globals-582-586, TBackground, TBufStream×13, TButton×2, TChDirDialog×2, TCheckBoxes, TCluster×2, TCollection, TColorDialog×2, TDialog×2, TDirCollection, TDosStream×9, TEditWindow, TEditor, TEmsStream×12, TFileCollection, TFileDialog×2, TFileEditor, TFileInfoPane, TFileInputLine, TFilterValidator×2, TGroup×2, THistory, TIndicator, TInputLine, TLabel×2, TListBox, TListViewer, TMemo×2, TMenuBar, TMenuView×2, TMultiCheckBoxes×2, TOutline, TOutlineViewer, TPXPictureValidator×2, TParamText×2, TPoint, TRangeValidator×2, TRect, TResourceCollection, TResourceFile×2, TScrollBar×2, TScroller×2, TSortedCollection, TSortedListBox, TStaticText×2, TStrIndex×2, TStrListMaker, TStream×23, TStreamRec×5, TStringList×2, TStringLookupValidator×2, TTextDevice, TValidator×2, TView×4, TWindow×2

### DOS / EMS / memory manager — 95 entries
Globals-317-330×11, Globals-331-346×16, Globals-347-362×18, Globals-363-378×21, PrimitiveTypes×2, TCollection×2, TColorGroup, TDirListBox, TEditor, TEvent×2, TFileCollection, TFileEditor×2, TFileList×2, TInputLine, TMenu, TMenuStr, TPXPictureValidator, TPoint, TProgram×2, TResourceCollection, TSearchRec×2, TSysErrorFunc×4, TWindow

### Video / screen-mode / CGA hardware — 3 entries
Globals-582-586, TDesktop, TProgram

### Pascal language artifacts (VMT/PString/PtrRec/typecast) — 2 entries
PrimitiveTypes, TTerminalBuffer

### Superseded by a tvision-rs extension (color picker, etc.) — 32 entries
Globals-317-330×2, TColorDialog×10, TColorDisplay, TColorGroup×4, TColorGroupList×4, TColorIndex, TColorItem×3, TColorSelector×7

### Stream registration (Register*) — 5 entries
TResourceFile, TStream×4

### Other (RAII/Drop, idiom-absorbed, obsolete hooks) — 174 entries
Globals-317-330×8, Globals-331-346×7, Globals-347-362×8, Globals-363-378×14, Globals-582-586×2, PrimitiveTypes×2, TButton, TCluster, TCollection×7, TColorDialog×4, TColorGroup, TColorGroupList×4, TColorIndex×2, TColorItem×2, TColorItemList, TDesktop, TDirCollection×7, TDirEntry, TEditBuffer, TEditor×2, TEvent×6, TFileCollection, TFileList×4, TGroup×8, THistory, THistoryWindow, TInputLine, TLabel, TListViewer, TMemoData, TMonoSelector×7, TMultiCheckBoxes, TOutlineViewer×2, TParamText×4, TRect, TResourceCollection×7, TResourceFile×11, TScroller×4, TSearchRec, TSortedCollection, TStaticText, TStrIndex×4, TStrListMaker×7, TStringCollection×2, TStringList×5, TStringLookupValidator, TSysErrorFunc, TTerminal×4, TTextDevice×2, TVTransfer, TValidator×2, TView, TWindow×3
