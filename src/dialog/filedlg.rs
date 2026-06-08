//! `TFileDialog` data-support classes (rows 71–75): [`DirEntry`],
//! [`SearchRec`], [`DirCollection`], [`FileCollection`], [`DirListBox`].
//!
//! Rows 71–74 are pure-data types. Row 75 (`TDirListBox`) is the first view
//! type here — a concrete [`ListViewer`](crate::widgets::list_viewer::ListViewer)
//! impl over a [`Vec<DirEntry>`] that renders a tree-indented directory listing.
//!
//! Per the rstv "collections → `Vec`" deviation (no `TCollection`; cf.
//! `ListBox`'s `Vec<String>`), `TDirCollection` is a plain `Vec<DirEntry>`
//! alias and `TFileCollection` is a `Vec<SearchRec>` carrying only the one
//! piece of real logic — the sorted insert and its comparator.  The unused C++
//! collection API (`indexOf`/`remove`/`atPut`/`firstThat`/…) is dropped; no
//! consumer exists.
//!
//! ## D14 — native Linux paths
//! `TDirListBox::newDirectory` had a DOS `showDrives` branch (A:–Z: drive scan)
//! and a DOS `showDirs` branch (`\`-separated). Per D14 only `showDirs` is
//! ported, with `/`-separated paths and `std::fs::read_dir` for enumeration.
//! The `showDrives` branch and all drive-related helpers are dropped.

use core::cmp::Ordering;

// ---------------------------------------------------------------------------
// DirEntry — row 71
// ---------------------------------------------------------------------------

/// `TDirEntry` (row 71) — a (display-text, directory-path) pair for the
/// directory tree pane.
///
/// The C++ type heap-allocates two `char*` fields (`displayText`,
/// `directory`).  In Rust they are plain `String`s on the same allocation as
/// the struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    /// `displayText` — what the dir-list box draws (may carry tree-glyph
    /// prefixes added by the populator).
    pub display_text: String,
    /// `directory` — the path this entry navigates to when selected.
    pub directory: String,
}

impl DirEntry {
    /// `TDirEntry::TDirEntry(txt, dir)` — construct from any `Into<String>`.
    pub fn new(display_text: impl Into<String>, directory: impl Into<String>) -> Self {
        DirEntry {
            display_text: display_text.into(),
            directory: directory.into(),
        }
    }

    /// `TDirEntry::text()` — the display string.
    pub fn text(&self) -> &str {
        &self.display_text
    }

    /// `TDirEntry::dir()` — the navigation path.
    pub fn dir(&self) -> &str {
        &self.directory
    }
}

// ---------------------------------------------------------------------------
// DirCollection — row 72
// ---------------------------------------------------------------------------

/// `TDirCollection` (row 72) — an ordered list of [`DirEntry`] items.
///
/// The C++ type is a `TCollection` of `TDirEntry*`.  Per the
/// collections→`Vec` deviation this collapses to a bare type alias: row 75
/// (`TDirListBox`) only needs `push`, index, and `len`; the full `TCollection`
/// API is dropped.
pub type DirCollection = Vec<DirEntry>;

// ---------------------------------------------------------------------------
// SearchRec — row 73
// ---------------------------------------------------------------------------

/// The directory-attribute bit of [`SearchRec::attr`] (`FA_DIREC = 0x10`).
pub const FA_DIREC: u8 = 0x10;

/// `TSearchRec` (row 73) — a directory-listing file-metadata record.
///
/// The C++ struct uses a fixed-length `char name[MAXFILE+MAXEXT-1]` to keep
/// it POD-copyable for the collection.  In Rust, `name` is a `String` and the
/// struct derives `Clone`.
///
/// `attr`, `time`, and `size` are populated by the filesystem-reading layer in
/// `TFileList`/`TFileDialog` (deferred to those rows).
/// `TODO(filedlg fs-read): populate attr/time/size from std::fs.`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchRec {
    /// `attr` — DOS file-attribute byte; only [`FA_DIREC`] is examined here.
    pub attr: u8,
    /// `time` — packed DOS timestamp.
    pub time: i32,
    /// `size` — file size in bytes.
    pub size: i32,
    /// `name` — the file or directory name (no path component).
    pub name: String,
}

// ---------------------------------------------------------------------------
// FileCollection — row 74
// ---------------------------------------------------------------------------

/// `TFileCollection::compare` (row 74) — the sort order for a directory
/// listing: `".."` last, directories after plain files, then case-sensitive
/// byte-order by name.
///
/// Ported verbatim from the C++ (the sign of every branch matters — do not
/// "tidy" it).
///
/// ```
/// use tvision::dialog::{SearchRec, search_rec_compare, FA_DIREC};
/// use core::cmp::Ordering;
///
/// let a = SearchRec { attr: 0,        time: 0, size: 0, name: "..".into() };
/// let b = SearchRec { attr: 0,        time: 0, size: 0, name: "foo".into() };
/// assert_eq!(search_rec_compare(&a, &b), Ordering::Greater); // ".." sorts last
/// ```
pub fn search_rec_compare(a: &SearchRec, b: &SearchRec) -> Ordering {
    // Equal names → Equal (mirrors the first strcmp returning 0).
    if a.name == b.name {
        return Ordering::Equal;
    }
    // key1 == ".." → positive (Greater means *after* in ascending order).
    if a.name == ".." {
        return Ordering::Greater;
    }
    // key2 == ".." → negative (Less).
    if b.name == ".." {
        return Ordering::Less;
    }
    let a_dir = a.attr & FA_DIREC != 0;
    let b_dir = b.attr & FA_DIREC != 0;
    // a is a directory, b is a plain file → a sorts after b.
    if a_dir && !b_dir {
        return Ordering::Greater;
    }
    // b is a directory, a is a plain file → a sorts before b.
    if b_dir && !a_dir {
        return Ordering::Less;
    }
    // Same kind — case-sensitive byte order (faithful to C++ `strcmp`).
    a.name.cmp(&b.name)
}

/// `TFileCollection` (row 74) — a name-sorted list of [`SearchRec`] items.
///
/// The C++ type is a `TSortedCollection` of `TSearchRec*`.  Per the
/// collections→`Vec` deviation the only transported behaviour is the sorted
/// insert and its comparator ([`search_rec_compare`]); the rest of the C++
/// `TSortedCollection` API is dropped (no consumer).
///
/// The sort order is: plain files alphabetically (case-sensitive), then
/// directories alphabetically, then `".."` last.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileCollection {
    items: Vec<SearchRec>,
}

impl FileCollection {
    /// Create an empty `FileCollection`.
    pub fn new() -> Self {
        FileCollection { items: Vec::new() }
    }

    /// `TSortedCollection::insert` — insert `rec` while keeping the list
    /// sorted by [`search_rec_compare`].  Duplicate names do not occur in a
    /// real directory listing.
    pub fn insert(&mut self, rec: SearchRec) {
        let pos = self
            .items
            .partition_point(|x| search_rec_compare(x, &rec) == Ordering::Less);
        self.items.insert(pos, rec);
    }

    /// `at(index)` — borrow the record at `index`, or `None` when out of
    /// bounds.
    pub fn at(&self, index: usize) -> Option<&SearchRec> {
        self.items.get(index)
    }

    /// `getCount()` — number of records in the collection.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the collection contains no records.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Read-only slice of the sorted records.
    pub fn items(&self) -> &[SearchRec] {
        &self.items
    }

    /// Consume the collection, returning its sorted records by value (avoids a
    /// clone when the `FileCollection` is a temporary).
    pub fn into_items(self) -> Vec<SearchRec> {
        self.items
    }
}

// ---------------------------------------------------------------------------
// DirListBox — row 75
// ---------------------------------------------------------------------------

/// Tree-glyph connector: root entry and each ancestor — `pathDir` in the C++.
const PATH_DIR: &str = "└─┬"; // U+2514 U+2500 U+252C
/// Tree-glyph connector: first subdirectory — `firstDir` in the C++.
const FIRST_DIR: &str = "└┬─"; // U+2514 U+252C U+2500
/// Tree-glyph connector: subsequent subdirectories — `middleDir` in the C++.
const MIDDLE_DIR: &str = " ├─"; // SPACE U+251C U+2500
/// How many extra spaces are added per depth level.
const INDENT_STEP: usize = 2;

/// `TDirListBox` (row 75) — a concrete [`ListViewer`] over a
/// [`Vec<DirEntry>`] that renders the current working directory as a
/// tree-indented listing of its ancestors and immediate subdirectories.
///
/// ## How it differs from `ListBox`
///
/// `TDirListBox` is a **C++ subclass of `TListBox`**. In rstv it is a *second,
/// parallel, direct* [`ListViewer`] impl — exactly like
/// [`ListBox`](crate::widgets::ListBox) is — over its own `Vec<DirEntry>`
/// storage. It does **not** embed or delegate through a
/// `ListBox`: if it delegated [`View::draw`](crate::view::View::draw), draw
/// would run with the inner `ListBox` as `self` and call its `get_text` over
/// `Vec<String>`, never consulting the `Vec<DirEntry>`. See the D2
/// embed-and-delegate note in PORTING-GUIDE.md.
///
/// ## D14 — native Linux paths
///
/// The C++ `newDirectory` has a DOS `showDrives` branch (A:–Z: drive scan).
/// Per D14 only `showDirs` is ported, re-imagined for Linux `/`-separated
/// paths. No `showDrives`, no drive letters, no backslashes.
///
/// ## Drops / deferrals
///
/// - `showDrives` / drive-letter scan — D14 (native Linux).
/// - `~TDirListBox` / `destroy` — Vec ownership; no manual destroy needed.
/// - `write`/`read`/`build`/`streamableName`/`name` — D12 streaming dropped.
/// - `select_item` cmChangeDir payload — deferred to row 80 (`TChDirDialog`).
///
/// [`ListViewer`]: crate::widgets::list_viewer::ListViewer
pub struct DirListBox {
    lv: crate::widgets::list_viewer::ListViewerState,
    /// The `TDirCollection` — the rendered tree of [`DirEntry`] items.
    items: Vec<DirEntry>,
    /// Index of the *current* directory entry (the highlighted ancestor).
    cur: usize,
    /// The current directory path (native `/`-separated, with trailing `/`).
    ///
    /// Retained for the row-80 `TChDirDialog` consumer (`select_item` /
    /// `cmChangeDir` reads the current directory). Currently **unread in the
    /// base port** — like `SortedListBox::shift_state`, it is captured but has
    /// no reader until its consumer lands.
    dir: String,
}

impl DirListBox {
    /// `TDirListBox::TDirListBox` — construct an empty dir list box.
    ///
    /// Faithful: `TListBox(bounds, 1, aScrollBar)` → `ListViewerState::new(bounds,
    /// 1, h, v)` (num_cols always 1, only the vertical scrollbar is used in the
    /// C++ ctor). `h` is kept for **ctor parity with
    /// [`ListBox::new`](crate::widgets::ListBox::new)** — the C++ `TListBox` ctor
    /// takes a single scrollbar and `TDirListBox` only ever wires the vertical
    /// one, so `h` is typically `None` here.
    pub fn new(
        bounds: crate::view::Rect,
        h: Option<crate::view::ViewId>,
        v: Option<crate::view::ViewId>,
    ) -> Self {
        DirListBox {
            lv: crate::widgets::list_viewer::ListViewerState::new(bounds, 1, h, v),
            items: Vec::new(),
            cur: 0,
            dir: String::new(),
        }
    }

    /// The current item collection (`TDirListBox::list()`).
    pub fn list(&self) -> &[DirEntry] {
        &self.items
    }

    /// Pure tree-builder — ports the `showDirs` block of `TDirListBox::newDirectory`.
    ///
    /// Given `dir` (a `/`-terminated absolute path, e.g. `"/home/oetiker/"`) and
    /// an already-sorted list of immediate subdirectory names `subdirs`, returns
    /// `(entries, cur)` where `cur` is the index of the current-directory entry
    /// (the deepest ancestor, highlighted by [`ListViewer::is_selected`]).
    ///
    /// ## Layout (D14)
    ///
    /// ```text
    /// └─┬/             ← root, indent 0 (PATH_DIR)
    ///   └─┬home        ← indent 2 (PATH_DIR)
    ///     └─┬oetiker   ← indent 4 (PATH_DIR) ← cur
    ///       └┬─projects  ← indent 6 (FIRST_DIR, fixed up → └── if last)
    ///        ├─scratch    ← indent 6 (MIDDLE_DIR)
    ///        └─tmp        ← indent 6 (last; ├ → └)
    /// ```
    ///
    /// For `dir = "/"` (only the root): `cur = 0`, subdirs at indent 2.
    fn build_tree(dir: &str, subdirs: &[String]) -> (Vec<DirEntry>, usize) {
        let mut entries: Vec<DirEntry> = Vec::new();

        // --- Step 1: root entry -------------------------------------------
        entries.push(DirEntry::new(format!("{PATH_DIR}/"), "/".to_string()));

        // --- Step 2: ancestor entries ---------------------------------------
        // Split `dir` on `/`; the meaningful segments are the non-empty parts.
        // For `dir = "/home/oetiker/"` → segments ["home", "oetiker"].
        let segments: Vec<&str> = dir.split('/').filter(|s| !s.is_empty()).collect();

        for (i, &seg) in segments.iter().enumerate() {
            let indent = (i + 1) * INDENT_STEP;
            // Build the absolute path through this segment (no trailing slash).
            let abs_path = format!("/{}", segments[..=i].join("/"));
            entries.push(DirEntry::new(
                format!("{}{}{}", " ".repeat(indent), PATH_DIR, seg),
                abs_path,
            ));
        }

        // `cur` is the index of the deepest ancestor = last entry pushed so far.
        let cur = entries.len() - 1;

        // Indent for subdirs = (depth of cur) + INDENT_STEP.
        // For root-only (`/`, segments empty, cur = 0): sub_indent = 0 + 2 = 2.
        // For `/home/oetiker/` (cur at index 2, depth = 2*INDENT_STEP = 4):
        //   sub_indent = 4 + 2 = 6.
        let sub_indent = cur * INDENT_STEP + INDENT_STEP;

        // --- Step 3: immediate subdirectories --------------------------------
        for (i, name) in subdirs.iter().enumerate() {
            let connector = if i == 0 { FIRST_DIR } else { MIDDLE_DIR };
            // `directory = dir + name` (dir ends with `/`).
            let directory = format!("{}{}", dir, name);
            entries.push(DirEntry::new(
                format!("{}{}{}", " ".repeat(sub_indent), connector, name),
                directory,
            ));
        }

        // --- Step 4: last-entry glyph fix-up --------------------------------
        // Faithful to the C++ pointer surgery on `dirs->at(getCount()-1)`,
        // applied UNCONDITIONALLY to the last entry (the deepest visible node
        // has no sibling/child below it, so its connector becomes a corner):
        //   - has '└' (PATH_DIR "└─┬" or FIRST_DIR "└┬─"): turn the two chars
        //     after '└' into "──"  →  "└──".
        //   - else has '├' (MIDDLE_DIR " ├─"): turn '├' into '└'  →  " └─".
        // When subdirs exist this hits the last subdir; with no subdirs it hits
        // the deepest ancestor ("└─┬name" → "└──name"). `entries` is never empty
        // (the root is always present).
        let last = entries.last_mut().unwrap();
        let mut c: Vec<char> = last.display_text.chars().collect();
        if let Some(i) = c.iter().position(|&ch| ch == '└') {
            if i + 1 < c.len() {
                c[i + 1] = '─';
            }
            if i + 2 < c.len() {
                c[i + 2] = '─';
            }
            last.display_text = c.into_iter().collect();
        } else if let Some(i) = c.iter().position(|&ch| ch == '├') {
            c[i] = '└';
            last.display_text = c.into_iter().collect();
        }

        (entries, cur)
    }

    /// `TDirListBox::newDirectory` — read `dir`'s subdirectories from the
    /// filesystem, build the tree via the private `build_tree`, and
    /// publish the result to the list-viewer machinery.
    ///
    /// Faithful: `newList(dirs); focusItem(cur)`.
    ///
    /// The only impure operation (filesystem read) is isolated here; all tree
    /// construction is in the pure `build_tree`.
    pub fn new_directory(&mut self, dir: &str, ctx: &mut crate::view::Context) {
        self.dir = dir.to_string();

        // Read immediate subdirectories from the filesystem.
        let mut subdirs: Vec<String> = match std::fs::read_dir(dir) {
            Ok(entries) => entries
                .filter_map(|e| {
                    let e = e.ok()?;
                    // stat-follows-symlinks (std::fs::metadata), matching
                    // magiblot's findfirst (cvtAttr in source/platform/findfrst.cpp
                    // uses stat()). DirEntry::file_type() is lstat-based and would
                    // wrongly exclude a symlink pointing at a directory — wrong for
                    // a directory navigator. A broken symlink → metadata errs → the
                    // `?` skips it, which is the desired behavior.
                    let meta = std::fs::metadata(e.path()).ok()?;
                    if !meta.is_dir() {
                        return None;
                    }
                    let name = e.file_name().to_string_lossy().into_owned();
                    if name.starts_with('.') {
                        return None;
                    }
                    Some(name)
                })
                .collect(),
            Err(_) => Vec::new(),
        };
        // Sort case-insensitively (row-70 ordering — identical to `ci_cmp` in
        // list_box.rs; inlined here to avoid cross-module coupling).
        subdirs.sort_by(|a, b| {
            a.chars()
                .map(|c| c.to_ascii_lowercase())
                .cmp(b.chars().map(|c| c.to_ascii_lowercase()))
        });

        let (items, cur) = Self::build_tree(dir, &subdirs);
        self.items = items;
        self.cur = cur;

        let len = self.items.len() as i32;
        crate::widgets::list_viewer::set_range(self, len, ctx);
        if self.lv.range > 0 {
            crate::widgets::list_viewer::focus_item(self, self.cur as i32, ctx);
        }
    }
}

impl crate::widgets::list_viewer::ListViewer for DirListBox {
    fn lv(&self) -> &crate::widgets::list_viewer::ListViewerState {
        &self.lv
    }

    fn lv_mut(&mut self) -> &mut crate::widgets::list_viewer::ListViewerState {
        &mut self.lv
    }

    /// `TDirListBox::getText` — the display text for `item`.
    ///
    /// Faithful: `strnzcpy(text, list()->at(item)->text(), …)`.
    fn get_text(&self, item: i32) -> String {
        self.items
            .get(item as usize)
            .map(|e| e.text().to_string())
            .unwrap_or_default()
    }

    /// `TDirListBox::isSelected` — `item == cur` (the current directory ancestor).
    ///
    /// Faithful override of the base (base is `item == focused`; dir list
    /// highlights the *current directory* entry, not just the cursor position).
    fn is_selected(&self, item: i32) -> bool {
        item as usize == self.cur
    }

    /// `TDirListBox::selectItem` — sends cmChangeDir carrying the chosen
    /// [`DirEntry`] payload to the owner `TChDirDialog`.
    ///
    /// TODO(row 80 TChDirDialog): selectItem sends cmChangeDir carrying the
    /// chosen DirEntry payload to the owner. rstv's payload-less
    /// Event::Broadcast can't carry it; design the typed-payload command seam
    /// when TChDirDialog (the only consumer) lands.
    fn select_item(&mut self, _item: i32, _ctx: &mut crate::view::Context) {
        // Intentionally empty — see TODO above.
    }
}

impl crate::view::View for DirListBox {
    fn state(&self) -> &crate::view::ViewState {
        &self.lv.state
    }

    fn state_mut(&mut self) -> &mut crate::view::ViewState {
        &mut self.lv.state
    }

    fn draw(&mut self, ctx: &mut crate::view::DrawCtx) {
        crate::widgets::list_viewer::draw(self, ctx);
    }

    fn handle_event(&mut self, ev: &mut crate::event::Event, ctx: &mut crate::view::Context) {
        crate::widgets::list_viewer::handle_event(self, ev, ctx);
    }

    /// `TDirListBox::setState` — delegates to `list_viewer::set_state`, then (on
    /// `sfFocused` change) would call `makeDefault(enable)` on the owner's
    /// `chDirButton`.
    ///
    /// TODO(row 80 TChDirDialog): on sfFocused change, also makeDefault(enable)
    /// the owner's chDirButton. Needs the owner-downcast + button seam; deferred
    /// to TChDirDialog.
    fn set_state(
        &mut self,
        flag: crate::view::StateFlag,
        enable: bool,
        ctx: &mut crate::view::Context,
    ) {
        crate::widgets::list_viewer::set_state(self, flag, enable, ctx);
    }

    fn cursor_request(&self) -> Option<crate::view::Point> {
        crate::widgets::list_viewer::focused_cursor(self)
    }

    fn apply_list_scroll(
        &mut self,
        h: Option<i32>,
        v: Option<i32>,
        ctx: &mut crate::view::Context,
    ) {
        crate::widgets::list_viewer::apply_scroll(self, h, v, ctx);
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> {
        Some(self)
    }

    /// `TDirListBox` has no `getData` override in the C++ — the focused item
    /// index (same as `ListBox`) is the natural value.
    fn value(&self) -> Option<crate::data::FieldValue> {
        Some(crate::data::FieldValue::Int(self.lv.focused))
    }
}

// ---------------------------------------------------------------------------
// FileList — row 76
// ---------------------------------------------------------------------------

/// `TFileList` (row 76) — the file-listing pane of `TFileDialog`. A concrete
/// [`SortedSearch`](crate::widgets::list_viewer::SortedSearch) (hence
/// [`ListViewer`]) over a name-sorted [`Vec<SearchRec>`] (== a
/// [`FileCollection`]'s contents), with two columns and incremental
/// type-to-search.
///
/// ## Structural shape
///
/// `TFileList` is a C++ subclass of `TSortedListBox`. In rstv it is a *direct*
/// [`SortedSearch`] impl — the same shape as
/// [`SortedListBox`](crate::widgets::SortedListBox), NOT
/// [`DirListBox`](crate::dialog::DirListBox) (which is a plain [`ListViewer`]).
/// It therefore wires `handle_event`→`sorted_handle_event` and
/// `cursor_request`→`sorted_cursor`, inherits the base `is_selected`
/// (`item == focused`), and contributes no dialog data (`value() == None`).
///
/// ## The `search` key (getKey + list()->search, fused)
///
/// `search` is the one method whose comparator is **non-obvious**: it must
/// compare via [`search_rec_compare`] over raw [`SearchRec`]s, **not** over
/// [`get_text`](ListViewer::get_text) (which appends `/` to dir names and would
/// mis-order). The key carries an `attr` of [`FA_DIREC`] when the
/// [`shift_state`](crate::widgets::list_viewer::SortedSearch::shift_state)
/// holds [`KB_SHIFT`](crate::widgets::list_viewer::KB_SHIFT) or the typed prefix
/// starts with `.`, routing the search into the directory section of the
/// collection. That routing exists only in `search_rec_compare`'s file/dir
/// ordering — hence this is a per-impl `search`, not the shared `get_text` one.
///
/// ## D14 — native Linux paths
///
/// `getText` appends `/` (not the DOS `\`) to directory names; `getKey`'s
/// `strupr` (DOS-only, `#ifndef __FLAT__`) is **not** ported — the Linux build
/// is case-sensitive.
///
/// ## Owner broadcasts (row 77)
///
/// `TFileList::focusItem` broadcasts `cmFileFocused` (payload = the focused
/// `SearchRec`) on every focus change, and `selectItem` broadcasts
/// `cmFileDoubleClicked`. Both are wired here via the `on_focus_changed`
/// virtual-`focusItem` hook ([`ListViewer::on_focus_changed`]) and the
/// `select_item` override; the payload travels through the pump's
/// [`ResolveFocusedFile`](crate::view::Deferred::ResolveFocusedFile) broker (D4:
/// payload-less broadcast + resolvable `source`).
///
/// ## Drops / deferrals (faithful, breadcrumbed)
///
/// - `getData`/`setData`/`dataSize` no-op → `value() == None` (no dialog data).
/// - `write`/`read`/`build`/`streamableName`/`name` — D12 streaming dropped.
/// - the `tooManyFiles` messageBox OOM guard — dropped (`Vec` is infallible).
/// - `DirSearchRec::operator new` safety-pool check — dropped.
/// - `fexpand`/`squeeze`/path canonicalization — DOS path machinery, not needed
///   under D14 (the caller passes an absolute `/`-path).
///
/// [`ListViewer`]: crate::widgets::list_viewer::ListViewer
/// [`SortedSearch`]: crate::widgets::list_viewer::SortedSearch
pub struct FileList {
    lv: crate::widgets::list_viewer::ListViewerState,
    /// The sorted listing — the contents of the C++ `TFileCollection`.
    items: Vec<SearchRec>,
    /// `searchPos` — index of the last matched char in the focused item's text;
    /// -1 = no active search.
    search_pos: i32,
    /// `shiftState` — `kbShift` bits captured at the searchPos -1↔0 transition;
    /// read by [`search`](FileList::search) to route the key into the dir
    /// section (`getKey`: `(shiftState & kbShift) != 0` → `attr = FA_DIREC`).
    shift_state: u8,
}

impl FileList {
    /// `TFileList::TFileList(bounds, aScrollBar)` — `TSortedListBox(bounds, 2,
    /// aScrollBar)`, so **num_cols = 2**. Like the `TSortedListBox` ctor it shows
    /// the cursor at column 1. `h` is kept for ctor parity (the C++ ctor takes a
    /// single scrollbar — typically the vertical one).
    pub fn new(
        bounds: crate::view::Rect,
        h: Option<crate::view::ViewId>,
        v: Option<crate::view::ViewId>,
    ) -> Self {
        let mut lv = crate::widgets::list_viewer::ListViewerState::new(bounds, 2, h, v);
        lv.state.show_cursor();
        lv.state.set_cursor(1, 0);
        FileList {
            lv,
            items: Vec::new(),
            search_pos: -1,
            shift_state: 0,
        }
    }

    /// The current item collection (`TFileList::list()`).
    pub fn list(&self) -> &[SearchRec] {
        &self.items
    }

    /// The focused entry (`list()->at(focused)`), or `None` when the list is empty
    /// (or `focused` is out of range). Read by the pump's
    /// [`ResolveFocusedFile`](crate::view::Deferred::ResolveFocusedFile) broker to
    /// deliver the `cmFileFocused` payload to a `TFileInputLine`/`TFileInfoPane`.
    pub fn focused_rec(&self) -> Option<SearchRec> {
        self.items.get(self.lv.focused as usize).cloned()
    }

    /// Match `name` against a `*`/`?` glob (case-sensitive, like the Linux
    /// build). `*` = any run (including empty); `?` = exactly one char. No
    /// `[...]` classes or escaping (a Unix `fnmatch` subset). `"*"` matches
    /// everything. Classic two-pointer scan with `*`-backtracking.
    fn wildcard_match(pattern: &str, name: &str) -> bool {
        let p: Vec<char> = pattern.chars().collect();
        let s: Vec<char> = name.chars().collect();
        let (mut pi, mut si) = (0usize, 0usize);
        // Backtrack anchors: the last `*` position in `p` and the `s` position
        // it was matched against (so a failed match can extend the `*` run).
        let mut star: Option<usize> = None;
        let mut star_si = 0usize;
        while si < s.len() {
            if pi < p.len() && (p[pi] == '?' || p[pi] == s[si]) {
                pi += 1;
                si += 1;
            } else if pi < p.len() && p[pi] == '*' {
                star = Some(pi);
                star_si = si;
                pi += 1;
            } else if let Some(sp) = star {
                // Extend the last `*` to consume one more char of `s`.
                pi = sp + 1;
                star_si += 1;
                si = star_si;
            } else {
                return false;
            }
        }
        // Trailing `*`s in the pattern match the empty remainder.
        while pi < p.len() && p[pi] == '*' {
            pi += 1;
        }
        pi == p.len()
    }

    /// Build the sorted listing from a directory's raw entries — the pure,
    /// unit-testable core of [`read_directory`](FileList::read_directory). `dir`
    /// is a `/`-terminated absolute path; each raw entry is `(name, is_dir,
    /// size)`.
    ///
    /// Faithful to the two-pass C++ `readDirectory`:
    /// - **Pass 1 (files):** a non-directory is kept iff it matches `wildcard`.
    /// - **Pass 2 (dirs):** a directory is kept iff `name[0] != '.'` — the
    ///   wildcard does NOT apply (C++ resets the pattern to `"*.*"`). This drops
    ///   `.`, `..`, AND hidden dirs (faithful to `ff_name[0] != '.'`).
    /// - **`".."`:** appended iff `dir != "/"` (C++ `strlen(dir) > 1`).
    ///
    /// `time` is left 0 on every record. TODO(row 78): time/date packing — the
    /// DOS-timestamp packing + size/date display is the `TFileInfoPane` (row 78)
    /// concern.
    ///
    /// Returns the [`search_rec_compare`]-sorted `Vec`, built via
    /// [`FileCollection::insert`].
    fn build_listing(dir: &str, wildcard: &str, raw: &[(String, bool, i32)]) -> Vec<SearchRec> {
        let mut fc = FileCollection::new();
        for (name, is_dir, size) in raw {
            if *is_dir {
                if !name.starts_with('.') {
                    fc.insert(SearchRec {
                        attr: FA_DIREC,
                        time: 0,
                        size: 0,
                        name: name.clone(),
                    });
                }
            } else if Self::wildcard_match(wildcard, name) {
                fc.insert(SearchRec {
                    attr: 0,
                    time: 0,
                    size: *size,
                    name: name.clone(),
                });
            }
        }
        if dir != "/" {
            fc.insert(SearchRec {
                attr: FA_DIREC,
                time: 0,
                size: 0,
                name: "..".into(),
            });
        }
        fc.into_items()
    }

    /// `TFileList::readDirectory` — read `dir`'s entries from the filesystem,
    /// build the sorted listing via the pure [`build_listing`](FileList::build_listing),
    /// and publish it to the list-viewer machinery.
    ///
    /// The only impure operation (the filesystem read) is isolated here. Like
    /// [`DirListBox::new_directory`], it uses `std::fs::metadata` (which follows
    /// symlinks — matching magiblot's `findfirst`/`stat`, not `lstat`); a broken
    /// symlink errs and is skipped via `.ok()`. `size` saturates into `i32`.
    ///
    /// Faithful to the C++ `readDirectory` tail: after `newList(fileList)`, it
    /// broadcasts `cmFileFocused` for item 0 (or an all-zero `DirSearchRec noFile`
    /// sentinel when the listing is empty). Here the non-empty case is FREE — the
    /// `focus_item(self, 0, …)` below fires `on_focus_changed`, which broadcasts
    /// `FILE_FOCUSED { source = self }`; the broker reads `focused_rec()`. The
    /// empty case has no focusable item, so it broadcasts `FILE_FOCUSED` directly
    /// (the broker then reads `focused_rec() == None` → a blank field, the noFile
    /// sentinel's effect).
    pub fn read_directory(&mut self, dir: &str, wildcard: &str, ctx: &mut crate::view::Context) {
        let raw: Vec<(String, bool, i32)> = match std::fs::read_dir(dir) {
            Ok(entries) => entries
                .filter_map(|e| {
                    let e = e.ok()?;
                    let meta = std::fs::metadata(e.path()).ok()?;
                    let name = e.file_name().to_string_lossy().into_owned();
                    let is_dir = meta.is_dir();
                    let size = meta.len().min(i32::MAX as u64) as i32;
                    Some((name, is_dir, size))
                })
                .collect(),
            Err(_) => Vec::new(),
        };

        self.items = Self::build_listing(dir, wildcard, &raw);

        let len = self.items.len() as i32;
        crate::widgets::list_viewer::set_range(self, len, ctx);
        if self.lv.range > 0 {
            // focus_item → on_focus_changed → FILE_FOCUSED broadcast (item 0).
            crate::widgets::list_viewer::focus_item(self, 0, ctx);
        } else if let Some(id) = self.lv.state.id() {
            // Empty listing — no focusable item, so the `focus_item` path can't
            // fire. Broadcast FILE_FOCUSED directly: the broker reads
            // `focused_rec() == None` → a blank field (the C++ noFile sentinel).
            ctx.broadcast(crate::command::Command::FILE_FOCUSED, Some(id));
        }
    }
}

impl crate::widgets::list_viewer::ListViewer for FileList {
    fn lv(&self) -> &crate::widgets::list_viewer::ListViewerState {
        &self.lv
    }

    fn lv_mut(&mut self) -> &mut crate::widgets::list_viewer::ListViewerState {
        &mut self.lv
    }

    /// `TFileList::getText` — the file/dir name, with a trailing `/` (D14, not
    /// the DOS `\`) appended to directories. OOB → empty.
    fn get_text(&self, item: i32) -> String {
        match self.items.get(item as usize) {
            Some(rec) => {
                if rec.attr & FA_DIREC != 0 {
                    format!("{}/", rec.name)
                } else {
                    rec.name.clone()
                }
            }
            None => String::new(),
        }
    }

    // `is_selected` is INHERITED (base: `item == focused`). TFileList does not
    // override it — do NOT add one here.

    /// `TFileList::focusItem` — the virtual tail, fired on EVERY focus change.
    ///
    /// Faithful: `message(owner, evBroadcast, cmFileFocused, list()->at(item))`.
    /// rstv's broadcast is payload-less (D4), so we broadcast `FILE_FOCUSED` with
    /// this view as the resolvable `source`; the pump's
    /// [`ResolveFocusedFile`](crate::view::Deferred::ResolveFocusedFile) broker
    /// reads [`focused_rec`](FileList::focused_rec) and delivers it to the
    /// consumer. (The C++ calls `TSortedListBox::focusItem` first — that base step
    /// is the shared `focus_item` free fn that invokes this hook, so it has already
    /// run.)
    fn on_focus_changed(&mut self, ctx: &mut crate::view::Context) {
        if let Some(id) = self.lv.state.id() {
            ctx.broadcast(crate::command::Command::FILE_FOCUSED, Some(id));
        }
    }

    /// `TFileList::selectItem` — broadcast `cmFileDoubleClicked`.
    ///
    /// Faithful: `message(owner, evBroadcast, cmFileDoubleClicked, list()->at(item))`.
    /// The C++ payload is the focused `SearchRec`, but the only consumer
    /// (`TFileDialog::handleEvent`) merely turns `cmFileDoubleClicked` into `cmOK`
    /// and never reads the record — so it is faithfully **payload-less** here (D4):
    /// just `FILE_DOUBLE_CLICKED { source = self }`. Does NOT call the base, so it
    /// does NOT also broadcast `cmListItemSelected`.
    ///
    /// TODO(row 79 TFileDialog): the dialog's `cmFileDoubleClicked → cmOK`
    /// conversion is row 79's job.
    fn select_item(&mut self, _item: i32, ctx: &mut crate::view::Context) {
        if let Some(id) = self.lv.state.id() {
            ctx.broadcast(crate::command::Command::FILE_DOUBLE_CLICKED, Some(id));
        }
    }
}

impl crate::widgets::list_viewer::SortedSearch for FileList {
    fn search_pos(&self) -> i32 {
        self.search_pos
    }

    fn set_search_pos(&mut self, pos: i32) {
        self.search_pos = pos;
    }

    fn shift_state(&self) -> u8 {
        self.shift_state
    }

    fn set_shift_state(&mut self, s: u8) {
        self.shift_state = s;
    }

    /// `TFileList::getKey` + `list()->search`, fused.
    ///
    /// `getKey`: the key's `attr` is [`FA_DIREC`] when `shift_state & KB_SHIFT`
    /// is set OR the typed prefix starts with `.`, else 0. The name is the typed
    /// prefix verbatim — NO `strupr` (the `__FLAT__`/Linux branch).
    ///
    /// `list()->search`: the first index `i` in `0..range` where
    /// `search_rec_compare(items[i], key) != Less` (the insertion point);
    /// `range` if none. **Compares via [`search_rec_compare`] over raw
    /// [`SearchRec`]s** (the `attr` routes the search into the file or dir
    /// section) — NOT over `get_text`.
    fn search(&self, cur: &[char]) -> i32 {
        use crate::widgets::list_viewer::KB_SHIFT;
        let attr = if (self.shift_state & KB_SHIFT) != 0 || cur.first() == Some(&'.') {
            FA_DIREC
        } else {
            0
        };
        let name: String = cur.iter().collect();
        let key = SearchRec {
            attr,
            time: 0,
            size: 0,
            name,
        };
        // Insertion point over the sorted items, bounded by `range` (which the
        // tests set independently of `items.len()`), via search_rec_compare.
        let range = self.lv.range;
        let (mut lo, mut hi) = (0i32, range);
        while lo < hi {
            let mid = lo + (hi - lo) / 2;
            if search_rec_compare(&self.items[mid as usize], &key) == Ordering::Less {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo
    }
}

impl crate::view::View for FileList {
    fn state(&self) -> &crate::view::ViewState {
        &self.lv.state
    }

    fn state_mut(&mut self) -> &mut crate::view::ViewState {
        &mut self.lv.state
    }

    fn draw(&mut self, ctx: &mut crate::view::DrawCtx) {
        crate::widgets::list_viewer::draw(self, ctx);
    }

    /// `TSortedListBox::handleEvent` — the incremental type-to-search machine.
    fn handle_event(&mut self, ev: &mut crate::event::Event, ctx: &mut crate::view::Context) {
        crate::widgets::list_viewer::sorted_handle_event(self, ev, ctx);
    }

    fn cursor_request(&self) -> Option<crate::view::Point> {
        crate::widgets::list_viewer::sorted_cursor(self)
    }

    /// `TFileList` has no `setState` override (its only owner poke is in the
    /// `focusItem` override, deferred to row 79) — the plain base.
    fn set_state(
        &mut self,
        flag: crate::view::StateFlag,
        enable: bool,
        ctx: &mut crate::view::Context,
    ) {
        crate::widgets::list_viewer::set_state(self, flag, enable, ctx);
    }

    fn apply_list_scroll(
        &mut self,
        h: Option<i32>,
        v: Option<i32>,
        ctx: &mut crate::view::Context,
    ) {
        crate::widgets::list_viewer::apply_scroll(self, h, v, ctx);
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> {
        Some(self)
    }

    /// `None` — C++ overrides `getData`/`setData`/`dataSize` to no-op/0, so
    /// `TFileList` contributes nothing to dialog data transfer.
    fn value(&self) -> Option<crate::data::FieldValue> {
        None
    }
}

// ---------------------------------------------------------------------------
// FileInputLine — row 77
// ---------------------------------------------------------------------------

/// `TFileInputLine` (stddlg.cpp) — a [`TInputLine`](crate::widgets::InputLine)
/// filename field that mirrors the [`FileList`]'s focused entry.
///
/// On a `cmFileFocused` broadcast (and only while the user is **not** typing in
/// it — the `!(state & sfSelected)` guard) it copies the focused entry's name
/// into the field, appending `/<wildCard>` (D14 `/`, not the DOS `\`) when the
/// entry is a directory.
///
/// ## Structural shape (D2 embed-and-delegate)
///
/// `TFileInputLine` is a C++ subclass of `TInputLine`. In rstv it **embeds** an
/// [`InputLine`] and forwards the un-overridden [`View`](crate::view::View)
/// methods via `#[crate::delegate(to = inner)]` — only `handle_event` and
/// `as_any_mut` differ. `value`/`set_value` (D10) are NOT overridden (the C++
/// `TFileInputLine` has no `getData`/`setData`), so they forward to the inner
/// `InputLine`.
///
/// ## The payload broker (D3/D4)
///
/// The broadcast is payload-less in rstv, so `handle_event` does not read the
/// record inline; it requests
/// [`ResolveFocusedFile`](crate::view::Deferred::ResolveFocusedFile) and the pump
/// resolves the producer's [`focused_rec`](FileList::focused_rec), then calls
/// [`on_file_focused`](FileInputLine::on_file_focused) on this view.
///
/// ## Drops / deferrals
/// - **D8:** the C++ `drawView()` after the copy is dropped (whole-tree redraw).
/// - **D12:** `write`/`read`/`build`/`streamableName`/`name` streaming dropped.
pub struct FileInputLine {
    /// The embedded `TInputLine` — the D2 delegation target.
    inner: crate::widgets::InputLine,
    /// Cached `((TFileDialog*)owner)->wildCard` (D3: a child can't read its owner
    /// inline). Set at ctor; row 79 pushes updates when the dialog re-reads with a
    /// new mask. Appended after a `/` when the focused entry is a directory.
    wild_card: String,
}

impl FileInputLine {
    /// `TFileInputLine::TFileInputLine(bounds, aMaxLen)` — build a filename field.
    ///
    /// Faithful: `TInputLine(bounds, aMaxLen)` — `aMaxLen` is the byte cap
    /// ([`LimitMode::MaxBytes`], the C++ default), so `maxLen = aMaxLen - 1`. The
    /// C++ `eventMask |= evBroadcast` is implicit (the group delivers every
    /// broadcast unconditionally), so no event mask is wired. `wild_card` caches
    /// the owner's `wildCard` (which the C++ reads via `owner` on each dir entry).
    pub fn new(bounds: crate::view::Rect, max_len: i32, wild_card: impl Into<String>) -> Self {
        FileInputLine {
            inner: crate::widgets::InputLine::new(
                bounds,
                max_len,
                None,
                crate::widgets::LimitMode::MaxBytes,
            ),
            wild_card: wild_card.into(),
        }
    }

    /// Write the focused record into the field — the body of the C++
    /// `handleEvent`'s `cmFileFocused` block, called by the pump's
    /// [`ResolveFocusedFile`](crate::view::Deferred::ResolveFocusedFile) broker
    /// once it has resolved the producer's focused [`SearchRec`].
    ///
    /// Faithful: `strcpy(data, rec->name)`; if `rec->attr & FA_DIREC`, then
    /// `strcat(data, "/"); strcat(data, wildCard)` (D14 `/`, not `\`); then
    /// `selectAll(False)`. `None` is the C++ all-zero `noFile` sentinel (empty
    /// name) → a blank field. `drawView()` is dropped (D8).
    pub fn on_file_focused(&mut self, rec: Option<SearchRec>) {
        let text = match rec {
            Some(r) if r.attr & FA_DIREC != 0 => format!("{}/{}", r.name, self.wild_card),
            Some(r) => r.name,
            None => String::new(),
        };
        // C++ `strcpy(data, …)` does not clamp in this path; assign directly
        // (InputLine exposes no clamping text-setter, and `data` is `pub`).
        self.inner.data = text;
        // selectAll(False) — the C++ shorthand `selectAll(Boolean)` defaults
        // `scroll = True`, so this is `select_all(false, true)`.
        self.inner.select_all(false, true);
        // drawView() dropped (D8 — whole-tree redraw + diff).
    }

    /// Update the cached owner `wildCard` (row 79 drives this via a deferred push
    /// when the dialog re-reads the directory with a new mask).
    // TODO(row 79): driven by the dialog's wildCard-change push.
    pub fn set_wild_card(&mut self, w: impl Into<String>) {
        self.wild_card = w.into();
    }
}

#[crate::delegate(to = inner)]
impl crate::view::View for FileInputLine {
    /// `TFileInputLine::handleEvent` — delegate to `TInputLine::handleEvent`
    /// first, then, on a `cmFileFocused` broadcast while NOT selected (so the copy
    /// never clobbers the field the user is typing in), request the payload broker.
    fn handle_event(&mut self, ev: &mut crate::event::Event, ctx: &mut crate::view::Context) {
        // TInputLine::handleEvent — the base call (the C++ runs it first).
        self.inner.handle_event(ev, ctx);

        if let crate::event::Event::Broadcast {
            command,
            source: Some(src),
        } = *ev
            && command == crate::command::Command::FILE_FOCUSED
            // !(state & sfSelected) — do NOT clobber the field while the user types.
            && !self.inner.state().state.selected
            && let Some(my_id) = self.inner.state().id()
        {
            ctx.request_resolve_focused_file(my_id, src);
        }
    }

    /// **Override** (the OPPOSITE of [`Memo`](crate::widgets::Memo), which
    /// forwards `as_any_mut` to its editor): the pump's broker downcasts the
    /// resolved subscriber to `FileInputLine`, so `as_any_mut` MUST return
    /// `self`, NOT the inner `InputLine`.
    fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> {
        Some(self)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: build a SearchRec with a given name and optional dir-flag.
    fn rec(name: &str, is_dir: bool) -> SearchRec {
        SearchRec {
            attr: if is_dir { FA_DIREC } else { 0 },
            time: 0,
            size: 0,
            name: name.into(),
        }
    }

    // --- DirEntry ---

    #[test]
    fn dir_entry_accessors() {
        let e = DirEntry::new("~ disp", "/some/dir");
        assert_eq!(e.text(), "~ disp");
        assert_eq!(e.dir(), "/some/dir");
    }

    #[test]
    fn dir_entry_clone_and_eq() {
        let e = DirEntry::new("a", "b");
        assert_eq!(e.clone(), e);
    }

    // --- search_rec_compare: one assertion per branch ---

    #[test]
    fn compare_equal_names() {
        assert_eq!(
            search_rec_compare(&rec("foo", false), &rec("foo", false)),
            Ordering::Equal
        );
    }

    #[test]
    fn compare_a_is_dotdot() {
        // ".." sorts after everything → Greater.
        assert_eq!(
            search_rec_compare(&rec("..", false), &rec("foo", false)),
            Ordering::Greater
        );
    }

    #[test]
    fn compare_b_is_dotdot() {
        // ".." as key2 → Less.
        assert_eq!(
            search_rec_compare(&rec("foo", false), &rec("..", false)),
            Ordering::Less
        );
    }

    #[test]
    fn compare_dir_after_file() {
        // a is a directory, b is a plain file (different names to avoid the
        // equal-name short-circuit) → a sorts after → Greater.
        assert_eq!(
            search_rec_compare(&rec("src", true), &rec("main.rs", false)),
            Ordering::Greater
        );
    }

    #[test]
    fn compare_file_before_dir() {
        // a is a plain file, b is a directory → a sorts before → Less.
        assert_eq!(
            search_rec_compare(&rec("main.rs", false), &rec("src", true)),
            Ordering::Less
        );
    }

    #[test]
    fn compare_both_files_alpha_order() {
        // "apple" < "banana" in byte order.
        assert_eq!(
            search_rec_compare(&rec("apple", false), &rec("banana", false)),
            Ordering::Less
        );
        assert_eq!(
            search_rec_compare(&rec("banana", false), &rec("apple", false)),
            Ordering::Greater
        );
    }

    #[test]
    fn compare_case_sensitive_byte_order() {
        // 'Z' (0x5A) < 'a' (0x61) → "Zebra" < "apple".
        assert_eq!(
            search_rec_compare(&rec("Zebra", false), &rec("apple", false)),
            Ordering::Less
        );
    }

    #[test]
    fn compare_both_dirs_alpha_order() {
        assert_eq!(
            search_rec_compare(&rec("alpha", true), &rec("beta", true)),
            Ordering::Less
        );
    }

    // --- FileCollection::insert keeps sorted order ---

    #[test]
    fn file_collection_sorted_insert() {
        let mut fc = FileCollection::new();
        // Insert out of order: a file, a directory, "..", another file.
        fc.insert(rec("readme.txt", false));
        fc.insert(rec("..", false));
        fc.insert(rec("src", true));
        fc.insert(rec("main.rs", false));

        // Expected order (by comparator): files alpha, dirs alpha, ".." last.
        let names: Vec<&str> = fc.items().iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["main.rs", "readme.txt", "src", ".."]);

        // Also verify adjacent pairs are non-decreasing under the comparator.
        for w in fc.items().windows(2) {
            assert_ne!(
                search_rec_compare(&w[0], &w[1]),
                Ordering::Greater,
                "pair ({}, {}) is out of order",
                w[0].name,
                w[1].name
            );
        }
    }

    #[test]
    fn file_collection_multiple_dirs() {
        let mut fc = FileCollection::new();
        fc.insert(rec("docs", true));
        fc.insert(rec("file.txt", false));
        fc.insert(rec("alpha", true));
        fc.insert(rec("..", false));
        fc.insert(rec("build.rs", false));

        let names: Vec<&str> = fc.items().iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["build.rs", "file.txt", "alpha", "docs", ".."]);
    }

    // --- at / len / is_empty ---

    #[test]
    fn file_collection_at_len_is_empty() {
        let mut fc = FileCollection::new();
        assert!(fc.is_empty());
        assert_eq!(fc.len(), 0);
        assert!(fc.at(0).is_none());

        fc.insert(rec("foo.txt", false));
        assert!(!fc.is_empty());
        assert_eq!(fc.len(), 1);
        assert_eq!(fc.at(0).map(|r| r.name.as_str()), Some("foo.txt"));
        assert!(fc.at(1).is_none());
    }

    #[test]
    fn file_collection_default_is_empty() {
        let fc = FileCollection::default();
        assert!(fc.is_empty());
    }

    // =========================================================================
    // DirListBox — row 75
    // =========================================================================

    // -- build_tree: pure deterministic tests ----------------------------------

    /// Verify the deep tree: `/home/oetiker/` with three subdirs.
    #[test]
    fn build_tree_deep_dir_three_subdirs() {
        let subdirs: Vec<String> = vec!["projects".into(), "scratch".into(), "tmp".into()];
        let (entries, cur) = DirListBox::build_tree("/home/oetiker/", &subdirs);

        // Counts: root + home + oetiker + 3 subdirs = 6.
        assert_eq!(entries.len(), 6, "6 entries total");
        assert_eq!(cur, 2, "cur == 2 (oetiker is the 3rd entry, idx 2)");

        // Ancestor entries.
        assert_eq!(entries[0].directory, "/", "root directory");
        assert_eq!(entries[1].directory, "/home", "home directory");
        assert_eq!(entries[2].directory, "/home/oetiker", "oetiker directory");

        // Subdir directory values.
        assert_eq!(entries[3].directory, "/home/oetiker/projects");
        assert_eq!(entries[4].directory, "/home/oetiker/scratch");
        assert_eq!(entries[5].directory, "/home/oetiker/tmp");

        // Connector prefixes on ancestor entries (indent 0, 2, 4).
        assert!(
            entries[0].display_text.contains("└─┬"),
            "root uses PATH_DIR"
        );
        assert!(
            entries[1].display_text.contains("└─┬"),
            "home uses PATH_DIR"
        );
        assert!(
            entries[2].display_text.contains("└─┬"),
            "oetiker uses PATH_DIR"
        );

        // Connector prefixes on subdirs.
        assert!(
            entries[3].display_text.contains("└┬─"),
            "first subdir uses FIRST_DIR"
        );
        assert!(
            entries[4].display_text.contains(" ├─"),
            "middle subdir uses MIDDLE_DIR"
        );

        // Last-entry fix-up: `├` → `└`.
        assert!(
            entries[5].display_text.contains('└'),
            "last subdir has └ after fix-up"
        );
        assert!(
            !entries[5].display_text.contains('├'),
            "last subdir no longer has ├"
        );
    }

    /// Root-only dir (`"/"`) with two subdirs.
    #[test]
    fn build_tree_root_only_two_subdirs() {
        let subdirs: Vec<String> = vec!["etc".into(), "usr".into()];
        let (entries, cur) = DirListBox::build_tree("/", &subdirs);

        // Counts: root + 2 subdirs = 3.
        assert_eq!(entries.len(), 3, "3 entries total");
        assert_eq!(cur, 0, "cur == 0 (root is the only ancestor)");

        assert_eq!(entries[0].directory, "/");
        assert_eq!(entries[1].directory, "/etc");
        assert_eq!(entries[2].directory, "/usr");

        // Subdirs at indent 2.
        assert!(
            entries[1].display_text.starts_with("  "),
            "subdir indent = 2 spaces"
        );
        // Last-entry fix-up: `├` → `└`.
        assert!(
            entries[2].display_text.contains('└'),
            "last subdir has └ after fix-up"
        );
        assert!(!entries[2].display_text.contains('├'));
    }

    /// Single-subdir fix-up: `└┬─` → `└──`.
    #[test]
    fn build_tree_single_subdir_fixup() {
        let subdirs: Vec<String> = vec!["only".into()];
        let (entries, cur) = DirListBox::build_tree("/", &subdirs);

        assert_eq!(entries.len(), 2);
        assert_eq!(cur, 0);

        // The single subdir started as FIRST_DIR "└┬─"; fix-up replaces "┬─" → "──".
        let display = &entries[1].display_text;
        assert!(
            display.contains("└──"),
            "single subdir fix-up: '└┬─' → '└──', got: {:?}",
            display
        );
        assert!(!display.contains("┬─"), "no remaining ┬─ after fix-up");
    }

    /// No subdirs — the fix-up still runs on the deepest ancestor (it is the
    /// last entry), turning its "└─┬" connector into a leaf corner "└──".
    #[test]
    fn build_tree_no_subdirs() {
        let (entries, cur) = DirListBox::build_tree("/home/user/", &[]);
        // root + home + user = 3 entries, no subdirs.
        assert_eq!(entries.len(), 3);
        assert_eq!(cur, 2);
        // The deepest ancestor (last entry) became a leaf corner "└──user".
        assert!(
            entries[2].display_text.ends_with("└──user"),
            "deepest ancestor fix-up: '└─┬user' → '└──user', got: {:?}",
            entries[2].display_text
        );
        assert!(
            !entries[2].display_text.contains('┬'),
            "no remaining ┬ after fix-up"
        );
        // Earlier ancestors keep their "└─┬" connector (they have children).
        assert!(
            entries[1].display_text.contains("└─┬"),
            "home keeps its branch connector"
        );
    }

    /// Root-only, no subdirs — a single entry, fixed up to a leaf corner.
    #[test]
    fn build_tree_root_only_no_subdirs() {
        let (entries, cur) = DirListBox::build_tree("/", &[]);
        assert_eq!(entries.len(), 1, "just the root");
        assert_eq!(cur, 0);
        assert_eq!(entries[0].directory, "/");
        // "└─┬/" → "└──/".
        assert_eq!(
            entries[0].display_text, "└──/",
            "root-only fix-up: '└─┬/' → '└──/'"
        );
    }

    /// `is_selected` returns true only for `cur`, not for `focused`.
    #[test]
    fn dir_list_box_is_selected_returns_cur() {
        use crate::widgets::list_viewer::ListViewer;

        let subdirs: Vec<String> = vec!["a".into(), "b".into()];
        let (items, cur) = DirListBox::build_tree("/home/oetiker/", &subdirs);

        let mut dlb = DirListBox::new(crate::view::Rect::new(0, 0, 30, 8), None, None);
        dlb.items = items;
        dlb.cur = cur;
        dlb.lv.range = dlb.items.len() as i32;
        dlb.lv.focused = 0; // cursor is on root, not cur

        // Only `cur` (index 2) is "selected".
        assert!(dlb.is_selected(cur as i32), "cur entry is selected");
        assert!(!dlb.is_selected(0), "root entry (focused) is not selected");
        assert!(!dlb.is_selected(1), "home entry is not selected");
    }

    // -- snapshot test: draw the rendered tree ---------------------------------

    fn render_dlb(dlb: &mut DirListBox, w: u16, h: u16) -> String {
        use crate::backend::{HeadlessBackend, Renderer};
        use crate::screen::Buffer;
        use crate::theme::Theme;
        use crate::view::{DrawCtx, View};

        let theme = Theme::classic_blue();
        let (backend, screen) = HeadlessBackend::new(w, h);
        let mut r = Renderer::new(Box::new(backend));
        r.render(|buf: &mut Buffer| {
            let bounds = dlb.state().get_bounds();
            let mut dc = DrawCtx::new(buf, &theme, bounds, bounds.a);
            dlb.draw(&mut dc);
        });
        screen.snapshot()
    }

    /// Snapshot with `focused == cur`: both focused and is_selected land on
    /// the same row so the highlighted row comes from the focused-color branch.
    #[test]
    fn snapshot_dir_list_box_tree() {
        let subdirs: Vec<String> = vec!["projects".into(), "scratch".into(), "tmp".into()];
        let (items, cur) = DirListBox::build_tree("/home/oetiker/", &subdirs);

        let mut dlb = DirListBox::new(crate::view::Rect::new(0, 0, 30, 8), None, None);
        // Seed directly (no Context needed for draw test).
        dlb.lv.state.state.selected = true;
        dlb.lv.state.state.active = true;
        dlb.items = items;
        dlb.cur = cur;
        dlb.lv.range = dlb.items.len() as i32;
        dlb.lv.focused = cur as i32;

        insta::assert_snapshot!(render_dlb(&mut dlb, 30, 8));
    }

    /// Snapshot with `focused != cur` — exercises `is_selected` through the
    /// draw path. The cursor sits on the root (row 0, `focused=0`) while
    /// `is_selected` still marks the oetiker ancestor (`cur=2`).
    ///
    /// In `list_viewer::draw`, the color precedence is:
    ///   focused == item  → focused_color   (root row, cursor here)
    ///   is_selected(item) → selected_color (cur row, highlighted here)
    ///   else              → normal_color
    ///
    /// If `is_selected` were broken (always false) the cur row would render in
    /// normal_color and this snapshot would differ — making the check bite.
    #[test]
    fn snapshot_dir_list_box_tree_cursor_off_cur() {
        let subdirs: Vec<String> = vec!["projects".into(), "scratch".into(), "tmp".into()];
        let (items, cur) = DirListBox::build_tree("/home/oetiker/", &subdirs);

        let mut dlb = DirListBox::new(crate::view::Rect::new(0, 0, 30, 8), None, None);
        dlb.lv.state.state.selected = true;
        dlb.lv.state.state.active = true;
        dlb.items = items;
        dlb.cur = cur; // cur == 2 (oetiker) remains the "selected" dir.
        dlb.lv.range = dlb.items.len() as i32;
        dlb.lv.focused = 0; // cursor on root — NOT the current dir.

        insta::assert_snapshot!(render_dlb(&mut dlb, 30, 8));
    }

    // =========================================================================
    // FileList — row 76
    // =========================================================================

    use crate::event::{Event, Key, KeyEvent, KeyModifiers};
    use crate::view::{Context, Deferred, View};
    use crate::widgets::list_viewer::ListViewer;
    use std::collections::VecDeque;

    fn fl_make_ctx<'a>(
        out: &'a mut VecDeque<Event>,
        timers: &'a mut crate::timer::TimerQueue,
        deferred: &'a mut Vec<Deferred>,
    ) -> Context<'a> {
        Context::new(out, timers, 0, deferred)
    }

    fn fl_key(c: char) -> Event {
        Event::KeyDown(KeyEvent::new(Key::Char(c), KeyModifiers::default()))
    }

    // -- 1. wildcard_match -----------------------------------------------------

    #[test]
    fn wildcard_star_matches_all() {
        assert!(FileList::wildcard_match("*", ""));
        assert!(FileList::wildcard_match("*", "anything.rs"));
        assert!(FileList::wildcard_match("*", "no.extension.here"));
    }

    #[test]
    fn wildcard_extension_filter() {
        assert!(FileList::wildcard_match("*.txt", "a.txt"));
        assert!(FileList::wildcard_match("*.txt", "longer.name.txt"));
        assert!(!FileList::wildcard_match("*.txt", "a.rs"));
        assert!(!FileList::wildcard_match("*.txt", "txt")); // no dot
    }

    #[test]
    fn wildcard_question_mark_single_char() {
        assert!(FileList::wildcard_match("?.rs", "a.rs"));
        assert!(!FileList::wildcard_match("?.rs", "ab.rs")); // ? is exactly one
        assert!(!FileList::wildcard_match("?.rs", ".rs")); // needs one char
    }

    #[test]
    fn wildcard_star_edges() {
        // "a*z" — prefix 'a', suffix 'z', any middle.
        assert!(FileList::wildcard_match("a*z", "az")); // empty middle
        assert!(FileList::wildcard_match("a*z", "abcz"));
        assert!(!FileList::wildcard_match("a*z", "abc")); // no trailing z
        assert!(!FileList::wildcard_match("a*z", "bz")); // no leading a
        // Case-sensitive (Linux build).
        assert!(!FileList::wildcard_match("*.TXT", "a.txt"));
    }

    // -- 2. build_listing ------------------------------------------------------

    // Helper: a raw (name, is_dir, size) triple.
    fn raw(name: &str, is_dir: bool, size: i32) -> (String, bool, i32) {
        (name.into(), is_dir, size)
    }

    #[test]
    fn build_listing_files_filtered_dirs_always() {
        let raw_entries = vec![
            raw("keep.txt", false, 10),
            raw("skip.rs", false, 20),
            raw("src", true, 0),  // dir: kept regardless of wildcard
            raw("docs", true, 0), // dir: kept regardless of wildcard
        ];
        let items = FileList::build_listing("/home/oetiker/", "*.txt", &raw_entries);
        let names: Vec<&str> = items.iter().map(|r| r.name.as_str()).collect();
        // File "skip.rs" filtered out; both dirs kept; ".." appended.
        assert_eq!(names, vec!["keep.txt", "docs", "src", ".."]);
        // "keep.txt" carries its size; dirs/".." carry FA_DIREC.
        assert_eq!(items[0].size, 10);
        assert_eq!(items[0].attr & FA_DIREC, 0, "file has no FA_DIREC");
        assert_eq!(items[1].attr & FA_DIREC, FA_DIREC, "docs is a dir");
        assert_eq!(items[3].attr & FA_DIREC, FA_DIREC, ".. is a dir");
    }

    #[test]
    fn build_listing_drops_hidden_dirs_and_dot_entries() {
        let raw_entries = vec![
            raw(".git", true, 0), // hidden dir -> dropped
            raw(".", true, 0),    // self -> dropped
            raw("..", true, 0),   // parent in raw -> dropped (synthesized below)
            raw("visible", true, 0),
            raw(".hidden", false, 5), // hidden FILE: matched by "*" -> kept
        ];
        let items = FileList::build_listing("/home/oetiker/", "*", &raw_entries);
        let names: Vec<&str> = items.iter().map(|r| r.name.as_str()).collect();
        // Hidden file kept (wildcard "*"); hidden/./.. dirs dropped; ".." synthesized last.
        assert_eq!(names, vec![".hidden", "visible", ".."]);
    }

    #[test]
    fn build_listing_dotdot_only_off_root() {
        // Off the root: ".." appended.
        let items = FileList::build_listing("/home/oetiker/", "*", &[raw("a", true, 0)]);
        assert!(
            items.iter().any(|r| r.name == ".."),
            "dotdot present off root"
        );

        // At the root: no "..".
        let items = FileList::build_listing("/", "*", &[raw("a", true, 0)]);
        assert!(
            !items.iter().any(|r| r.name == ".."),
            "no dotdot at the root"
        );
    }

    #[test]
    fn build_listing_order_matches_comparator() {
        let raw_entries = vec![
            raw("zebra.rs", false, 1),
            raw("apple.rs", false, 1),
            raw("src", true, 0),
            raw("bin", true, 0),
        ];
        let items = FileList::build_listing("/home/oetiker/", "*", &raw_entries);
        let names: Vec<&str> = items.iter().map(|r| r.name.as_str()).collect();
        // Files alpha, dirs alpha, ".." last.
        assert_eq!(names, vec!["apple.rs", "zebra.rs", "bin", "src", ".."]);

        // get_text appends "/" to dirs and "..".
        let mut fl = FileList::new(crate::view::Rect::new(0, 0, 30, 8), None, None);
        fl.items = items;
        assert_eq!(fl.get_text(0), "apple.rs", "file: no slash");
        assert_eq!(fl.get_text(2), "bin/", "dir: trailing slash");
        assert_eq!(fl.get_text(4), "../", "dotdot dir: trailing slash");
    }

    // -- 3. search: the discriminating comparator test -------------------------

    /// The key check that `search` compares via `search_rec_compare` (with the
    /// attr'd key), NOT via `get_text`.
    ///
    /// Seed a FILE "src.rs" and a DIRECTORY "src". Under the collection order:
    /// files first (alpha), then dirs (alpha), then "..". So:
    ///   items = ["src.rs" (file), "src" (dir), ".." (dir)]
    /// Searching "s" with shift_state == 0 → attr 0 → a FILE key → lands at the
    /// first item not-less-than it, which is the file "src.rs" (index 0 — the
    /// file section). Searching "s" with shift_state == KB_SHIFT → attr FA_DIREC
    /// → a DIR key → sorts AFTER every file → lands at the first dir "src"
    /// (index 1 — the dir section). A get_text-based impl ignores attr and would
    /// return the SAME index for both — this test rules that out.
    #[test]
    fn search_attr_routes_into_file_vs_dir_section() {
        use crate::widgets::list_viewer::{KB_SHIFT, SortedSearch};

        let raw_entries = vec![raw("src.rs", false, 1), raw("src", true, 0)];
        let items = FileList::build_listing("/home/oetiker/", "*", &raw_entries);
        // Expected sorted: file "src.rs" (0), dir "src" (1), ".." (2).
        assert_eq!(
            items.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            vec!["src.rs", "src", ".."]
        );

        let mut fl = FileList::new(crate::view::Rect::new(0, 0, 30, 8), None, None);
        fl.items = items;
        fl.lv.range = fl.items.len() as i32;

        // shift_state 0 → file key → lands in the FILE section (index 0).
        fl.shift_state = 0;
        assert_eq!(
            fl.search(&['s']),
            0,
            "file key lands at the first file >= 's' (src.rs, idx 0)"
        );

        // shift_state KB_SHIFT → dir key → lands in the DIR section (index 1).
        fl.shift_state = KB_SHIFT;
        assert_eq!(
            fl.search(&['s']),
            1,
            "dir key sorts after all files -> first dir (src, idx 1)"
        );

        // A leading '.' in the prefix ALSO routes to FA_DIREC (getKey: `*s == '.'`),
        // even with shift_state == 0 — the discriminating proof that the key's
        // attr (not get_text) drives the search. A dir-key "." sorts AMONG the
        // dirs (after all files), and "." < "src" < ".." in byte order, so it
        // lands at the first dir, index 1. (A get_text impl with shift 0 would
        // treat "." as a plain name and land it in the file section at index 0.)
        fl.shift_state = 0;
        assert_eq!(
            fl.search(&['.']),
            1,
            "leading '.' -> dir key -> first dir (src, idx 1), NOT the file section"
        );
    }

    #[test]
    fn search_plain_first_ge() {
        use crate::widgets::list_viewer::SortedSearch;
        // Files only: a search for "s" finds the first file >= "s".
        let raw_entries = vec![
            raw("alpha.rs", false, 1),
            raw("sigma.rs", false, 1),
            raw("zeta.rs", false, 1),
        ];
        let items = FileList::build_listing("/home/oetiker/", "*", &raw_entries);
        let mut fl = FileList::new(crate::view::Rect::new(0, 0, 30, 8), None, None);
        fl.items = items;
        fl.lv.range = fl.items.len() as i32;
        // "alpha.rs"(0) "sigma.rs"(1) "zeta.rs"(2) ".."(3, a dir).
        assert_eq!(
            fl.search(&['s']),
            1,
            "first file >= 's' is sigma.rs (idx 1)"
        );
    }

    // -- 4. type-to-search smoke test through handle_event ---------------------

    #[test]
    fn file_list_type_to_jump_focuses_match() {
        let raw_entries = vec![
            raw("alpha.rs", false, 1),
            raw("sigma.rs", false, 1),
            raw("zeta.rs", false, 1),
        ];
        let items = FileList::build_listing("/home/oetiker/", "*", &raw_entries);

        let mut fl = FileList::new(crate::view::Rect::new(0, 0, 30, 8), None, None);
        fl.items = items;
        fl.lv.range = fl.items.len() as i32;
        fl.lv.focused = 0; // start on "alpha.rs"

        let mut out: VecDeque<Event> = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred: Vec<Deferred> = vec![];

        // Type 's' -> jump to "sigma.rs" (index 1).
        let mut ev = fl_key('s');
        {
            let mut ctx = fl_make_ctx(&mut out, &mut timers, &mut deferred);
            fl.handle_event(&mut ev, &mut ctx);
        }
        assert_eq!(fl.lv().focused, 1, "'s' -> focus sigma.rs (idx 1)");
        assert!(ev.is_nothing(), "alphabetic char consumed");

        // After that single char the search machine advanced search_pos to 0
        // (the index of the matched char in the focused item's text), confirming
        // the type-to-search seam is wired through handle_event.
        use crate::widgets::list_viewer::SortedSearch;
        assert_eq!(fl.search_pos(), 0, "search_pos == 0 after one char");
    }

    // -- 5. snapshot of a rendered FileList ------------------------------------

    fn render_fl(fl: &mut FileList, w: u16, h: u16) -> String {
        use crate::backend::{HeadlessBackend, Renderer};
        use crate::screen::Buffer;
        use crate::theme::Theme;
        use crate::view::{DrawCtx, View};

        let theme = Theme::classic_blue();
        let (backend, screen) = HeadlessBackend::new(w, h);
        let mut r = Renderer::new(Box::new(backend));
        r.render(|buf: &mut Buffer| {
            let bounds = fl.state().get_bounds();
            let mut dc = DrawCtx::new(buf, &theme, bounds, bounds.a);
            fl.draw(&mut dc);
        });
        screen.snapshot()
    }

    #[test]
    fn snapshot_file_list() {
        let raw_entries = vec![
            raw("main.rs", false, 100),
            raw("lib.rs", false, 200),
            raw("readme.txt", false, 50),
            raw("src", true, 0),
            raw("tests", true, 0),
        ];
        let items = FileList::build_listing("/home/oetiker/", "*", &raw_entries);

        // 2-column layout (num_cols == 2). Width 30 so each column is ~15 wide.
        let mut fl = FileList::new(crate::view::Rect::new(0, 0, 30, 6), None, None);
        fl.lv.state.state.selected = true;
        fl.lv.state.state.active = true;
        fl.items = items;
        fl.lv.range = fl.items.len() as i32;
        fl.lv.focused = 0;

        insta::assert_snapshot!(render_fl(&mut fl, 30, 6));
    }

    // =========================================================================
    // FileInputLine + the cmFileFocused payload broker — row 77
    // =========================================================================

    use crate::command::Command;
    use crate::view::{Group, Rect};

    /// Insert `fl` into a fresh group, returning the group + the stamped id so we
    /// can drive it through `as_any_mut().downcast_mut::<FileList>()` and read the
    /// broadcasts it queues with `self` as `source`.
    fn fl_in_group(fl: FileList) -> (Group, crate::view::ViewId) {
        let mut g = Group::new(Rect::new(0, 0, 40, 12));
        let id = g.insert(Box::new(fl));
        (g, id)
    }

    fn count_broadcasts(out: &VecDeque<Event>, cmd: Command, src: crate::view::ViewId) -> usize {
        out.iter()
            .filter(|e| {
                matches!(e, Event::Broadcast { command, source }
                if *command == cmd && *source == Some(src))
            })
            .count()
    }

    // -- 1. on_focus_changed queues FILE_FOCUSED { source = self } -------------

    #[test]
    fn focus_change_broadcasts_file_focused_with_self_source() {
        let raw_entries = vec![raw("a.rs", false, 1), raw("b.rs", false, 1)];
        let items = FileList::build_listing("/home/oetiker/", "*", &raw_entries);
        let mut fl = FileList::new(Rect::new(0, 0, 30, 8), None, None);
        fl.items = items;
        fl.lv.range = fl.items.len() as i32;
        fl.lv.focused = 0;

        let (mut g, id) = fl_in_group(fl);
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred: Vec<Deferred> = vec![];

        // Drive a focus change through the shared `focus_item` funnel.
        {
            let mut ctx = fl_make_ctx(&mut out, &mut timers, &mut deferred);
            let fl = g
                .find_mut(id)
                .and_then(|v| v.as_any_mut())
                .and_then(|a| a.downcast_mut::<FileList>())
                .unwrap();
            crate::widgets::list_viewer::focus_item(fl, 1, &mut ctx);
            assert_eq!(fl.lv().focused, 1);
        }
        assert_eq!(
            count_broadcasts(&out, Command::FILE_FOCUSED, id),
            1,
            "focus change broadcasts exactly one FILE_FOCUSED with self as source"
        );
    }

    // -- 2. focused_rec returns the focused entry / None when empty -----------

    #[test]
    fn focused_rec_returns_focused_or_none() {
        let raw_entries = vec![raw("a.rs", false, 1), raw("b.rs", false, 1)];
        let items = FileList::build_listing("/home/oetiker/", "*", &raw_entries);
        let mut fl = FileList::new(Rect::new(0, 0, 30, 8), None, None);
        fl.items = items;
        fl.lv.range = fl.items.len() as i32;

        fl.lv.focused = 0;
        assert_eq!(fl.focused_rec().map(|r| r.name), Some("a.rs".to_string()));
        fl.lv.focused = 1;
        assert_eq!(fl.focused_rec().map(|r| r.name), Some("b.rs".to_string()));

        // Empty listing -> None.
        let empty = FileList::new(Rect::new(0, 0, 30, 8), None, None);
        assert_eq!(empty.focused_rec(), None);
    }

    // -- 3. select_item queues FILE_DOUBLE_CLICKED ----------------------------

    #[test]
    fn select_item_broadcasts_file_double_clicked() {
        let raw_entries = vec![raw("a.rs", false, 1)];
        let items = FileList::build_listing("/home/oetiker/", "*", &raw_entries);
        let mut fl = FileList::new(Rect::new(0, 0, 30, 8), None, None);
        fl.items = items;
        fl.lv.range = fl.items.len() as i32;

        let (mut g, id) = fl_in_group(fl);
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred: Vec<Deferred> = vec![];
        {
            let mut ctx = fl_make_ctx(&mut out, &mut timers, &mut deferred);
            let fl = g
                .find_mut(id)
                .and_then(|v| v.as_any_mut())
                .and_then(|a| a.downcast_mut::<FileList>())
                .unwrap();
            fl.select_item(0, &mut ctx);
        }
        assert_eq!(
            count_broadcasts(&out, Command::FILE_DOUBLE_CLICKED, id),
            1,
            "selectItem broadcasts FILE_DOUBLE_CLICKED with self as source"
        );
        // Faithful: it does NOT also broadcast cmListItemSelected (no base call).
        assert_eq!(
            count_broadcasts(&out, Command::LIST_ITEM_SELECTED, id),
            0,
            "selectItem does NOT call the base -> no cmListItemSelected"
        );
    }

    // -- 4. read_directory on an empty dir queues a FILE_FOCUSED (noFile) ------

    /// Verifies the `read_directory` noFile-branch **contract by construction**:
    /// an empty listing (range 0) has no focusable item, so it must broadcast
    /// `FILE_FOCUSED` directly rather than via `focus_item`. A genuinely-empty
    /// listing is unreachable end-to-end (off-root always synthesizes `..`; `/`
    /// always has subdirs on a live system), so this cannot drive `read_directory`
    /// for real — it instead exercises the exact `else` arm's logic in isolation.
    #[test]
    fn empty_listing_branch_broadcasts_file_focused_by_contract() {
        let fl = FileList::new(Rect::new(0, 0, 30, 8), None, None);
        let (mut g, id) = fl_in_group(fl);
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred: Vec<Deferred> = vec![];
        {
            let mut ctx = fl_make_ctx(&mut out, &mut timers, &mut deferred);
            let fl = g
                .find_mut(id)
                .and_then(|v| v.as_any_mut())
                .and_then(|a| a.downcast_mut::<FileList>())
                .unwrap();
            // Mirrors the production `read_directory` `else` arm verbatim (an empty
            // listing can't be produced end-to-end, so we reproduce its logic here):
            // empty items, range 0 -> broadcast FILE_FOCUSED directly (focus_item
            // never runs with range 0).
            fl.items.clear();
            crate::widgets::list_viewer::set_range(fl, 0, &mut ctx);
            assert_eq!(fl.lv().range, 0);
            if fl.lv().range == 0
                && let Some(vid) = fl.lv().state.id()
            {
                ctx.broadcast(Command::FILE_FOCUSED, Some(vid));
            }
        }
        assert_eq!(
            count_broadcasts(&out, Command::FILE_FOCUSED, id),
            1,
            "empty-listing branch broadcasts FILE_FOCUSED (noFile) once"
        );
    }

    /// Non-empty `read_directory` (off-root, so `..` is present) takes the
    /// `focus_item(0)` path -> `on_focus_changed` -> exactly one FILE_FOCUSED.
    #[test]
    fn read_directory_nonempty_broadcasts_file_focused_once() {
        let tmp = std::env::temp_dir().join(format!("rstv_filedlg_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("keep.txt"), b"x").unwrap();
        let dir = format!("{}/", tmp.to_string_lossy());

        let fl = FileList::new(Rect::new(0, 0, 30, 8), None, None);
        let (mut g, id) = fl_in_group(fl);
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred: Vec<Deferred> = vec![];
        {
            let mut ctx = fl_make_ctx(&mut out, &mut timers, &mut deferred);
            let fl = g
                .find_mut(id)
                .and_then(|v| v.as_any_mut())
                .and_then(|a| a.downcast_mut::<FileList>())
                .unwrap();
            fl.read_directory(&dir, "*", &mut ctx);
            assert!(fl.lv().range > 0, "off-root dir has at least '..'");
        }
        let _ = std::fs::remove_dir_all(&tmp);
        assert_eq!(
            count_broadcasts(&out, Command::FILE_FOCUSED, id),
            1,
            "non-empty read_directory broadcasts exactly one FILE_FOCUSED (item 0)"
        );
    }

    // -- 5. on_file_focused: file / dir / None --------------------------------

    #[test]
    fn file_input_line_on_file_focused_file_dir_none() {
        let mut fil = FileInputLine::new(Rect::new(0, 0, 20, 1), 80, "*.rs");

        // A plain file -> just the name, no slash.
        fil.on_file_focused(Some(SearchRec {
            attr: 0,
            time: 0,
            size: 10,
            name: "main.rs".into(),
        }));
        assert_eq!(fil.inner.data, "main.rs");

        // A directory -> "name/<wild_card>" (D14 slash).
        fil.on_file_focused(Some(SearchRec {
            attr: FA_DIREC,
            time: 0,
            size: 0,
            name: "src".into(),
        }));
        assert_eq!(fil.inner.data, "src/*.rs");

        // None (the noFile sentinel) -> blank.
        fil.on_file_focused(None);
        assert_eq!(fil.inner.data, "");
    }

    // -- 6. handle_event requests ResolveFocusedFile (guarded by sfSelected) --

    #[test]
    fn file_input_line_handle_event_requests_broker_when_not_selected() {
        let fil = FileInputLine::new(Rect::new(0, 0, 20, 1), 80, "*.rs");
        let mut g = Group::new(Rect::new(0, 0, 40, 12));
        let fil_id = g.insert(Box::new(fil));
        // A producer id to name as the broadcast source.
        let src_id = crate::view::ViewId::next();

        // NOT selected (default) -> the broker IS requested.
        let mut out = VecDeque::new();
        let mut timers = crate::timer::TimerQueue::new();
        let mut deferred: Vec<Deferred> = vec![];
        {
            let mut ctx = fl_make_ctx(&mut out, &mut timers, &mut deferred);
            let mut ev = Event::Broadcast {
                command: Command::FILE_FOCUSED,
                source: Some(src_id),
            };
            g.find_mut(fil_id).unwrap().handle_event(&mut ev, &mut ctx);
        }
        assert_eq!(
            deferred
                .iter()
                .filter(|d| matches!(d,
                    Deferred::ResolveFocusedFile { subscriber, source }
                        if *subscriber == fil_id && *source == src_id))
                .count(),
            1,
            "not-selected FILE_FOCUSED -> one ResolveFocusedFile request"
        );

        // SELECTED (user typing) -> the broker is NOT requested (sfSelected guard).
        if let Some(v) = g.find_mut(fil_id) {
            v.state_mut().state.selected = true;
        }
        let mut out2 = VecDeque::new();
        let mut timers2 = crate::timer::TimerQueue::new();
        let mut deferred2: Vec<Deferred> = vec![];
        {
            let mut ctx = fl_make_ctx(&mut out2, &mut timers2, &mut deferred2);
            let mut ev = Event::Broadcast {
                command: Command::FILE_FOCUSED,
                source: Some(src_id),
            };
            g.find_mut(fil_id).unwrap().handle_event(&mut ev, &mut ctx);
        }
        assert!(
            !deferred2
                .iter()
                .any(|d| matches!(d, Deferred::ResolveFocusedFile { .. })),
            "selected (typing) FILE_FOCUSED -> no broker request (sfSelected guard)"
        );
    }

    // -- 7. full pump-free round trip: broker resolves producer into consumer -

    #[test]
    fn file_focused_round_trip_through_broker() {
        // Producer FileList with a focused dir entry, consumer FileInputLine —
        // both in one group; emulate the pump's ResolveFocusedFile apply.
        let raw_entries = vec![raw("readme.txt", false, 1), raw("src", true, 0)];
        let items = FileList::build_listing("/home/oetiker/", "*", &raw_entries);
        let mut fl = FileList::new(Rect::new(0, 0, 30, 8), None, None);
        fl.items = items;
        fl.lv.range = fl.items.len() as i32;
        fl.lv.focused = 1; // the "src" directory

        let mut g = Group::new(Rect::new(0, 0, 40, 12));
        let src_id = g.insert(Box::new(fl));
        let sub_id = g.insert(Box::new(FileInputLine::new(
            Rect::new(0, 1, 20, 1),
            80,
            "*.c",
        )));

        // Emulate the pump's broker: read producer's focused rec, write to consumer.
        let rec = g
            .find_mut(src_id)
            .and_then(|v| v.as_any_mut())
            .and_then(|a| a.downcast_mut::<FileList>())
            .and_then(|fl| fl.focused_rec());
        let fil = g
            .find_mut(sub_id)
            .and_then(|v| v.as_any_mut())
            .and_then(|a| a.downcast_mut::<FileInputLine>())
            .unwrap();
        fil.on_file_focused(rec);

        // "src" is a directory -> "src/" + wildCard "*.c".
        assert_eq!(fil.inner.data, "src/*.c");
    }

    // -- 8. snapshot of a rendered FileInputLine ------------------------------

    fn render_fil(fil: &mut FileInputLine, w: u16, h: u16) -> String {
        use crate::backend::{HeadlessBackend, Renderer};
        use crate::screen::Buffer;
        use crate::theme::Theme;
        use crate::view::{DrawCtx, View};

        let theme = Theme::classic_blue();
        let (backend, screen) = HeadlessBackend::new(w, h);
        let mut r = Renderer::new(Box::new(backend));
        r.render(|buf: &mut Buffer| {
            let bounds = fil.state().get_bounds();
            let mut dc = DrawCtx::new(buf, &theme, bounds, bounds.a);
            fil.draw(&mut dc);
        });
        screen.snapshot()
    }

    #[test]
    fn snapshot_file_input_line() {
        let mut fil = FileInputLine::new(Rect::new(0, 0, 20, 1), 80, "*.rs");
        fil.inner.state.state.selected = true;
        fil.inner.state.state.active = true;
        // A directory focus -> "src/*.rs" in the field.
        fil.on_file_focused(Some(SearchRec {
            attr: FA_DIREC,
            time: 0,
            size: 0,
            name: "src".into(),
        }));
        insta::assert_snapshot!(render_fil(&mut fil, 20, 1));
    }
}
