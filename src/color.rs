//! Typed colour and style — deviation **D6**.
//!
//! magiblot packs fg/bg/style into a 64-bit `TColorAttr` whose fg/bg are each a
//! tagged-union `TColorDesired` (`colors.h`). We keep that four-variant design
//! but drop the bit-packing: [`Color`] is a plain enum and [`Style`] holds two
//! `Color`s plus a [`Modifiers`] struct-of-bools (D5).
//!
//! The RGB→256→16→BIOS quantization ladder (`mapcolor.cpp`) is *not* here — per
//! D6 it lives in the `Backend` (row 5), since it only matters when colours are
//! flushed to a real terminal.

/// A desired foreground *or* background colour. Faithful to `TColorDesired`'s
/// four-variant design (`colors.h`), minus the packing.
///
/// In a terminal, [`Color::Default`] is the colour of text with no display
/// attributes set — so a default/default cell still produces visible text.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Color {
    /// Terminal default (`ctDefault`).
    #[default]
    Default,
    /// 4-bit BIOS colour (`ctBIOS` / `TColorBIOS`). Invariant: 0..=15. C++
    /// masks `bios & 0xF` on construction; we store the raw `u8`, so callers and
    /// the quantization ladder (row 5) must mask to keep a 16-entry palette
    /// lookup in range.
    Bios(u8),
    /// Index into the xterm-256 palette, 0..=255 (`ctXTerm` / `TColorXTerm`).
    Indexed(u8),
    /// 24-bit true colour (`ctRGB` / `TColorRGB`).
    Rgb(u8, u8, u8),
}

impl Color {
    pub fn is_default(self) -> bool {
        matches!(self, Color::Default)
    }
    pub fn is_bios(self) -> bool {
        matches!(self, Color::Bios(_))
    }
    pub fn is_indexed(self) -> bool {
        matches!(self, Color::Indexed(_))
    }
    pub fn is_rgb(self) -> bool {
        matches!(self, Color::Rgb(..))
    }
}

/// Text-style flags. Faithful to the `sl*` masks of `TColorAttr`'s 10-bit style
/// word (`colors.h`), as a struct-of-bools (D5).
///
/// `no_shadow` is TV's private `slNoShadow`: a per-cell marker that window
/// shadows must not be cast over this cell (used by the D8 shadow pass).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Modifiers {
    pub bold: bool,      // slBold
    pub italic: bool,    // slItalic
    pub underline: bool, // slUnderline
    pub blink: bool,     // slBlink
    pub reverse: bool,   // slReverse  (prefer Style::reversed())
    pub strike: bool,    // slStrike
    pub no_shadow: bool, // slNoShadow (private)
}

/// The colour attributes of a screen cell. Faithful to `TColorAttr` (`colors.h`):
/// a foreground colour, a background colour, and a set of style modifiers.
///
/// A zero-initialized `Style` (via [`Default`]) has both colours `Default` and no
/// modifiers — matching TV's "zero-initialized `TColorAttr` produces visible
/// text" guarantee.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub modifiers: Modifiers,
}

impl Style {
    /// A style with the given colours and no modifiers.
    pub fn new(fg: Color, bg: Color) -> Self {
        Style {
            fg,
            bg,
            modifiers: Modifiers::default(),
        }
    }

    /// A style with explicit colours and modifiers.
    pub fn with_modifiers(fg: Color, bg: Color, modifiers: Modifiers) -> Self {
        Style { fg, bg, modifiers }
    }

    /// Port of the free function `reverseAttribute` (`colors.h`).
    ///
    /// The `slReverse` attribute is rendered inconsistently across terminals, so
    /// TV swaps the colours manually — *unless* either colour is `Default`, in
    /// which case there is nothing meaningful to swap and it falls back to
    /// toggling the reverse flag.
    pub fn reversed(self) -> Style {
        let mut out = self;
        if self.fg.is_default() || self.bg.is_default() {
            out.modifiers.reverse = !out.modifiers.reverse;
        } else {
            out.fg = self.bg;
            out.bg = self.fg;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_predicates() {
        assert!(Color::Default.is_default());
        assert!(Color::Bios(7).is_bios());
        assert!(Color::Indexed(200).is_indexed());
        assert!(Color::Rgb(1, 2, 3).is_rgb());
        assert!(!Color::Rgb(0, 0, 0).is_default());
    }

    #[test]
    fn default_style_is_visible_text() {
        let s = Style::default();
        assert_eq!(s.fg, Color::Default);
        assert_eq!(s.bg, Color::Default);
        assert_eq!(s.modifiers, Modifiers::default());
    }

    #[test]
    fn reversed_swaps_concrete_colors() {
        let s = Style::new(Color::Bios(0x7), Color::Bios(0x1));
        let r = s.reversed();
        assert_eq!(r.fg, Color::Bios(0x1));
        assert_eq!(r.bg, Color::Bios(0x7));
        assert!(!r.modifiers.reverse); // flag untouched when colours swapped
    }

    #[test]
    fn reversed_toggles_flag_when_a_color_is_default() {
        // Default foreground -> swap is meaningless, toggle the flag instead.
        let s = Style::new(Color::Default, Color::Bios(0x1));
        let r = s.reversed();
        assert_eq!(r.fg, Color::Default);
        assert_eq!(r.bg, Color::Bios(0x1));
        assert!(r.modifiers.reverse);

        // toggling twice returns to the original
        assert!(!r.reversed().modifiers.reverse);
    }
}
