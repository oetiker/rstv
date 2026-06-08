//! Color-dialog data classes (row 81): [`ColorItem`], [`ColorGroup`],
//! [`ColorIndex`].
//!
//! These are **pure data** types — no rendering, no event handling.  They back
//! the color-selector dialog family (rows 85–87).
//!
//! ## Collections → `Vec` deviation
//! The C++ types use `next`-pointer singly-linked lists to form chains of items
//! and groups.  Per the rstv "collections → `Vec`" deviation the linked-list
//! machinery is replaced by owned `Vec`s:
//! * `TColorItem::next` → membership in a `Vec<ColorItem>` inside `ColorGroup`.
//! * `TColorGroup::next` → membership in a `Vec<ColorGroup>` at the call site.
//! * The C++ `operator+` chaining helpers are **not** ported; `vec![]` and the
//!   [`ColorGroup::with_item`] builder are the Rust-idiomatic equivalents.
//!
//! ## `char*` → `String`
//! Both `TColorItem::name` and `TColorGroup::name` were C-string pointers
//! heap-allocated via `newStr`.  In Rust they are plain `String` fields owned
//! by the struct.

// ---------------------------------------------------------------------------
// ColorItem — row 81
// ---------------------------------------------------------------------------

/// `TColorItem` (row 81) — a (name, palette-index) pair in a color group.
///
/// The C++ type heap-allocates a `const char* name` via `newStr` and stores
/// a `uchar index` that is a **palette index** — immutable once constructed.
/// The `next` linked-list pointer is replaced by membership in
/// [`ColorGroup::items`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorItem {
    /// `TColorItem::name` — display name of this color entry.
    name: String,
    /// `TColorItem::index` — palette index; set at construction, never mutated.
    index: u8,
}

impl ColorItem {
    /// `TColorItem::TColorItem(nm, idx)` — construct from any `Into<String>`.
    pub fn new(name: impl Into<String>, index: u8) -> Self {
        ColorItem {
            name: name.into(),
            index,
        }
    }

    /// `TColorItem::name` — the display name of this color entry.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// `TColorItem::index` — the palette index (immutable after construction).
    pub fn index(&self) -> u8 {
        self.index
    }
}

// ---------------------------------------------------------------------------
// ColorGroup — row 81
// ---------------------------------------------------------------------------

/// `TColorGroup` (row 81) — a named group of [`ColorItem`]s with a mutable
/// focus position.
///
/// **Naming trap:** `TColorGroup::index` and `TColorItem::index` share the
/// C++ field name `index` but are completely different concepts:
/// * [`ColorItem::index`] is a **palette index**, set at construction,
///   immutable.
/// * [`ColorGroup::index`] is the **focused-item position within the group**,
///   defaulting to `0` and mutated later by `setGroupIndex` (row 85).  It is
///   *not* a constructor parameter (the C++ constructor leaves it
///   uninitialized; it is always written before it is read).
///
/// The `TColorItem* items` linked list is replaced by an owned
/// `Vec<ColorItem>`.  The `next` pointer is replaced by membership in a
/// `Vec<ColorGroup>` at the call site.  The C++ `operator+` chaining helpers
/// are **not** ported; use `vec![]` + [`ColorGroup::with_item`] instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorGroup {
    /// `TColorGroup::name` — display name of this group.
    name: String,
    /// `TColorGroup::index` — focused-item *position* within the group;
    /// defaults to `0`, mutated by rows 85–87.
    index: u8,
    /// `TColorGroup::items` — owned collection replacing the C++ linked list.
    items: Vec<ColorItem>,
}

impl ColorGroup {
    /// `TColorGroup::TColorGroup(nm, itm)` — construct with an initial item
    /// list.  `index` defaults to `0` (not a parameter; the C++ constructor
    /// leaves it uninitialized, always written before read).
    pub fn new(name: impl Into<String>, items: Vec<ColorItem>) -> Self {
        ColorGroup {
            name: name.into(),
            index: 0,
            items,
        }
    }

    /// Fluent builder: push one [`ColorItem`] and return `self`.
    ///
    /// This is the idiomatic replacement for the C++ `group + item` `operator+`
    /// chaining.  Example:
    /// ```rust
    /// use tvision::dialog::ColorGroup;
    ///
    /// let g = ColorGroup::new("Desktop", vec![])
    ///     .with_item("Color", 1)
    ///     .with_item("Mono", 2);
    /// assert_eq!(g.items().len(), 2);
    /// ```
    pub fn with_item(mut self, name: impl Into<String>, index: u8) -> Self {
        self.items.push(ColorItem::new(name, index));
        self
    }

    /// `TColorGroup::name` — the display name of this group.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// `TColorGroup::index` — focused-item position (mutable focus state).
    pub fn index(&self) -> u8 {
        self.index
    }

    /// Set the focused-item position.  Called by `setGroupIndex` (row 85).
    pub fn set_index(&mut self, index: u8) {
        self.index = index;
    }

    /// `TColorGroup::items` — the items in this group.
    pub fn items(&self) -> &[ColorItem] {
        &self.items
    }
}

// ---------------------------------------------------------------------------
// ColorIndex — row 81
// ---------------------------------------------------------------------------

/// `TColorIndex` (row 81) — the per-group focused-item index table used by the
/// color-selector dialog.
///
/// The C++ struct is a true variable-length allocation (`new uchar[numGroups +
/// 2]`); the `colorIndex[256]` in the header is only a sentinel declaration,
/// never the real size.  The runtime layout is:
/// ```c
/// // runtime layout — allocated as `new uchar[numGroups + 2]`:
/// uchar groupIndex;    // which group is focused
/// uchar colorSize;     // == numGroups
/// uchar colorIndex[];  // numGroups entries (C99 flexible array member)
/// ```
/// In Rust the flexible array member is a `Vec<u8>` sized to `numGroups`, and
/// `colorSize` is implicit as `color_index.len()` (the `Vec` carries its own
/// length), so there is no separate `color_size` field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorIndex {
    /// `TColorIndex::groupIndex` — which color group is currently focused.
    group_index: u8,
    /// `TColorIndex::colorIndex[]` — per-group focused-item positions;
    /// `colorSize` == `self.color_index.len()`.
    color_index: Vec<u8>,
}

impl ColorIndex {
    /// Construct with an initial `group_index` and `color_index` table.
    pub fn new(group_index: u8, color_index: Vec<u8>) -> Self {
        ColorIndex {
            group_index,
            color_index,
        }
    }

    /// `TColorIndex::groupIndex` — the currently focused group.
    pub fn group_index(&self) -> u8 {
        self.group_index
    }

    /// Set the focused group.  Called by `setGroupIndex` (row 85).
    pub fn set_group_index(&mut self, group_index: u8) {
        self.group_index = group_index;
    }

    /// `TColorIndex::colorIndex[]` — per-group focused-item index table.
    pub fn color_index(&self) -> &[u8] {
        &self.color_index
    }

    /// `TColorIndex::colorSize` — number of entries in the index table
    /// (derived from `Vec::len`; no separate field needed).
    pub fn color_size(&self) -> usize {
        self.color_index.len()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- ColorItem ---

    #[test]
    fn color_item_construction_and_accessors() {
        let item = ColorItem::new("Desktop", 42);
        assert_eq!(item.name(), "Desktop");
        assert_eq!(item.index(), 42);
    }

    #[test]
    fn color_item_from_string_types() {
        // Accepts &str, String, or any Into<String>.
        let a = ColorItem::new(String::from("A"), 1);
        let b = ColorItem::new("B", 2);
        assert_eq!(a.name(), "A");
        assert_eq!(b.name(), "B");
    }

    // --- ColorGroup ---

    #[test]
    fn color_group_defaults_index_to_zero() {
        let g = ColorGroup::new("Desktop", vec![]);
        assert_eq!(g.index(), 0, "index must default to 0 (not a ctor param)");
    }

    #[test]
    fn color_group_stores_items() {
        let items = vec![ColorItem::new("Color", 1), ColorItem::new("Mono", 2)];
        let g = ColorGroup::new("Desktop", items);
        assert_eq!(g.items().len(), 2);
        assert_eq!(g.items()[0].name(), "Color");
        assert_eq!(g.items()[1].name(), "Mono");
    }

    #[test]
    fn color_group_with_item_builder_chains() {
        let g = ColorGroup::new("Desktop", vec![])
            .with_item("Color", 1)
            .with_item("Mono", 2)
            .with_item("Cyan", 3);
        assert_eq!(g.items().len(), 3);
        assert_eq!(g.items()[2].name(), "Cyan");
        assert_eq!(g.items()[2].index(), 3);
    }

    #[test]
    fn color_group_set_index_mutates() {
        let mut g = ColorGroup::new("Desktop", vec![]);
        assert_eq!(g.index(), 0);
        g.set_index(5);
        assert_eq!(g.index(), 5);
    }

    // --- ColorIndex ---

    #[test]
    fn color_index_construction_and_accessors() {
        let ci = ColorIndex::new(2, vec![0, 1, 3]);
        assert_eq!(ci.group_index(), 2);
        assert_eq!(ci.color_index(), &[0u8, 1, 3]);
    }

    #[test]
    fn color_index_color_size_equals_vec_len() {
        let ci = ColorIndex::new(0, vec![5, 6, 7, 8]);
        assert_eq!(ci.color_size(), 4);
        assert_eq!(ci.color_size(), ci.color_index().len());
    }

    #[test]
    fn color_index_set_group_index_mutates() {
        let mut ci = ColorIndex::new(0, vec![1, 2]);
        ci.set_group_index(3);
        assert_eq!(ci.group_index(), 3);
    }

    #[test]
    fn color_index_empty_table() {
        let ci = ColorIndex::new(0, vec![]);
        assert_eq!(ci.color_size(), 0);
    }
}
