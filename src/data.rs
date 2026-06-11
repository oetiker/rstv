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
//! / [`Glyphs`](crate::theme::Glyphs) tables, it **grows per control**:
//! [`Text`](FieldValue::Text) lands with [`TInputLine`](crate::widgets::InputLine)
//! (row 39) and [`Int`](FieldValue::Int) with [`TScrollBar`](crate::widgets::ScrollBar)
//! (its first consumer is the row-27 `TScroller` read-broker, which reads a
//! scrollbar's `value` through [`View::value`](crate::view::View::value)). Future
//! variants land with their first consumer — e.g. a `Bits(u32)` for `TCluster`
//! arrives when that control wires its `value`/`set_value` (deferred — no consumer
//! reads it yet).
//!
//! ## Dialog gather/scatter group-walk (C4, landed)
//!
//! `Group::gather_data()` walks children in C++ `getData` order (see
//! `src/view/group.rs` for the ring-mapping rationale) and returns
//! `Vec<Option<FieldValue>>`. `Group::scatter_data(values, ctx)` distributes
//! values back in the same order via `View::set_value_ctx` (the context-aware
//! setter that `ListBox` overrides to republish its v-bar).

/// The typed unit of dialog data transfer (D10) — the successor to the untyped
/// `getData`/`setData` `void*` record.
///
/// **Grows per control.** [`Text`](FieldValue::Text) lands at row 39
/// ([`TInputLine`](crate::widgets::InputLine)); [`Int`](FieldValue::Int) at row 27
/// ([`TScrollBar`](crate::widgets::ScrollBar), read by the `TScroller` broker);
/// `Bits(u32)` for `TCluster` lands when that control wires its `value`/`set_value`
/// (deferred — no consumer yet).
#[derive(Clone, Debug, PartialEq)]
pub enum FieldValue {
    /// A text field's contents (`TInputLine`).
    Text(String),
    /// An integer value (`TScrollBar::value`; read by the row-27 `TScroller`
    /// read-broker via [`View::value`](crate::view::View::value)).
    Int(i32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_variant_round_trips() {
        let v = FieldValue::Text("hello".to_string());
        assert_eq!(v, FieldValue::Text("hello".to_string()));
        assert_ne!(v, FieldValue::Text("world".to_string()));
        let FieldValue::Text(s) = v else {
            panic!("expected Text");
        };
        assert_eq!(s, "hello");
    }

    #[test]
    fn int_variant_round_trips() {
        let v = FieldValue::Int(42);
        assert_eq!(v, FieldValue::Int(42));
        assert_ne!(v, FieldValue::Int(7));
        // Distinct from Text even when "equal-looking".
        assert_ne!(FieldValue::Int(0), FieldValue::Text("0".to_string()));
        let FieldValue::Int(n) = v else {
            panic!("expected Int");
        };
        assert_eq!(n, 42);
    }
}
