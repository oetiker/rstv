//! Input validators — `TValidator` ported per deviation **D2** (row 35).
//!
//! In C++ Turbo Vision, `TValidator` is an *abstract* base class whose concrete
//! subclasses (`TPXPictureValidator`, `TFilterValidator`, `TRangeValidator`,
//! `TLookupValidator`, …) gate what a `TInputLine` accepts. Per D2 inheritance
//! becomes a **trait**: [`Validator`] is the trait an input line holds as
//! `Option<Box<dyn Validator>>`. The concrete validators are later rows
//! (`TFilterValidator`/`TRangeValidator` etc.); only the abstract base lands
//! here, so every default method simply accepts.
//!
//! ## Object safety
//!
//! A `TInputLine` stores its validator as a boxed trait object
//! (`Option<Box<dyn Validator>>`), so [`Validator`] must be **object-safe**:
//! every method takes `&self`, no generics, no `Self` return. `validate` is
//! non-virtual in C++ (it calls the virtual `isValid`/`error`), so it stays a
//! provided method here.
//!
//! ## Deviations from PORT-ORDER row 35's note
//!
//! PORT-ORDER lists a `transfer(void*, TVTransfer)` hook. It has **no overrider
//! until `TRangeValidator` (row 59)** and **no caller** until then (its only
//! callers are `TInputLine::dataSize`/`getData`/`setData`, which under D10 become
//! the typed [`value`](crate::view::View::value)/[`set_value`](crate::view::View::set_value)
//! protocol — see `src/data.rs`). Building `transfer` now would be a dead stub,
//! so it is **deliberately omitted**; it lands with its first overrider/consumer
//! (row 59). The slot-in point is breadcrumbed in `InputLine::value`/`set_value`.

/// An input validator — `TValidator` (D2: abstract base → trait).
///
/// A [`TInputLine`](crate::widgets::InputLine) holds an
/// `Option<Box<dyn Validator>>`; with no validator every input is accepted. The
/// default methods all accept (faithful to the abstract base, which returns
/// `True`/`vsOk`); concrete validators (later rows) override them.
pub trait Validator {
    /// `TValidator::isValidInput` — check (and optionally auto-fill/modify) `s`
    /// *as it is being typed*. May mutate `s` in place (e.g. a picture validator
    /// inserting literal characters). `suppress_fill` (`noAutoFill` /
    /// `voFill`-suppression) asks it not to auto-fill. Default: accept, no
    /// change. Object-safe: `&self`, `s: &mut String`.
    fn is_valid_input(&self, _s: &mut String, _suppress_fill: bool) -> bool {
        true
    }

    /// `TValidator::isValid` — the final-form check, run when the field must be
    /// fully valid (the modal-OK / focus-release path). Default: accept.
    fn is_valid(&self, _s: &str) -> bool {
        true
    }

    /// `TValidator::error` — report an invalid final value. Concrete validators
    /// pop up a message box; the abstract base is a no-op.
    ///
    /// TODO(msgbox row 63): wire this to a real message box. No-op until then;
    /// `validate` still returns `false`, so the failure is observable.
    fn error(&self) {}

    /// `TValidator::validate` — **non-virtual in C++**: report the error and fail
    /// iff [`is_valid`](Validator::is_valid) is false, else succeed. Kept as a
    /// provided method (it dispatches through the overridable `is_valid`/`error`).
    fn validate(&self, s: &str) -> bool {
        if self.is_valid(s) {
            true
        } else {
            self.error();
            false
        }
    }

    /// Whether the validator's status is `vsOk` (`TValidator::status == vsOk`) —
    /// consulted by `TInputLine::valid(cmValid)`. The abstract base never sets a
    /// non-OK status, so the default is `true`; `TPXPictureValidator` (row 62)
    /// overrides to report a syntax error (`vsSyntax`).
    fn is_status_ok(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The abstract base accepts everything and reports OK.
    struct AcceptAll;
    impl Validator for AcceptAll {}

    /// A validator that rejects anything not equal to its target — exercises the
    /// non-default path through `validate`/`is_valid`.
    struct OnlyExact(&'static str);
    impl Validator for OnlyExact {
        fn is_valid(&self, s: &str) -> bool {
            s == self.0
        }
    }

    #[test]
    fn default_methods_accept() {
        let v = AcceptAll;
        let mut s = String::from("anything");
        assert!(v.is_valid_input(&mut s, false));
        assert_eq!(s, "anything", "default is_valid_input does not modify");
        assert!(v.is_valid("x"));
        assert!(v.validate("x"));
        assert!(v.is_status_ok());
    }

    #[test]
    fn validate_fails_when_is_valid_false() {
        let v = OnlyExact("ok");
        assert!(v.validate("ok"));
        assert!(!v.validate("nope"));
        assert!(v.is_valid("ok"));
        assert!(!v.is_valid("nope"));
    }

    #[test]
    fn is_object_safe() {
        // Compiles only if Validator is object-safe (the InputLine storage form).
        let v: Box<dyn Validator> = Box::new(OnlyExact("ok"));
        assert!(v.validate("ok"));
        assert!(!v.validate("no"));
    }
}
