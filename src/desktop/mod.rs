//! Desktop-layer views — `TDesktop` and `TBackground` (rows 28–29).
//!
//! Row 29 [`Background`] is the simplest concrete view: fills its owner's
//! bounds with a repeated pattern character. Row 28 (`TDesktop`) is not yet
//! ported; this module exists to house the desktop-layer group when it lands.

mod background;

pub use background::Background;
