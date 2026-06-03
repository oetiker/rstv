//! Typed dialog-data transfer — deviation **D10** (the `getData`/`setData`
//! currency).
//!
//! C++ Turbo Vision moves dialog data through `TView::getData`/`setData`/
//! `dataSize`: each control `memcpy`s its raw value into/out of an untyped
//! `void*` record at an offset the caller tracks by hand, and a dialog gathers
//! the whole record by walking its children in order (`TGroup::getData`/
//! `setData`). Per **D10** that untyped `memcpy` protocol becomes a **typed
//! value currency**: a control exposes its value as a [`FieldValue`] via the
//! [`value`](crate::view::View::value)/[`set_value`](crate::view::View::set_value)
//! pair on the [`View`](crate::view::View) trait.
//!
//! [`FieldValue`] is the unit transferred. Like the [`Role`](crate::theme::Role)
//! / [`Glyphs`](crate::theme::Glyphs) tables, it **grows per control**: this row
//! ([`TInputLine`](crate::widgets::InputLine)) needs only [`FieldValue::Text`].
//! Future variants land with their first consumer — e.g. a `Bits(u32)` for
//! `TCluster` and an `Int` for `TRangeValidator` arrive when those controls wire
//! their `value`/`set_value` (deferred — no consumer reads them yet).
//!
//! ## Deferred: the dialog gather/scatter group-walk
//!
//! C++ `TGroup::getData`/`setData` walk the children in order, concatenating
//! each control's bytes into one record. No data dialog consumes that ordered
//! walk yet (the first will be `inputBox` / the Batch-E data dialogs), so it is
//! **not built here**: the ordered child-walk over `value`/`set_value` lands
//! with its first consumer (inputBox / Batch E). Until then a control's value is
//! read/written individually through the trait pair.

/// The typed unit of dialog data transfer (D10) — the successor to the untyped
/// `getData`/`setData` `void*` record.
///
/// **Grows per control.** Only [`Text`](FieldValue::Text) exists at row 39
/// ([`TInputLine`](crate::widgets::InputLine)); `Bits(u32)` for `TCluster` and
/// `Int` for `TRangeValidator` land when those controls wire their
/// `value`/`set_value` (deferred — no consumer yet).
#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    /// A text field's contents (`TInputLine`).
    Text(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_variant_round_trips() {
        let v = FieldValue::Text("hello".to_string());
        assert_eq!(v, FieldValue::Text("hello".to_string()));
        assert_ne!(v, FieldValue::Text("world".to_string()));
        let FieldValue::Text(s) = v;
        assert_eq!(s, "hello");
    }
}
