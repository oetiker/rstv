//! Theme: a `Role` → [`Style`] map plus a glyph holder — deviation **D7**
//! (partial row 16).
//!
//! C++ Turbo Vision resolves colours by walking an owner chain of
//! length-prefixed palette strings (`getPalette`/`getColor`) and scatters drawing
//! glyphs (frame corners, scrollbar arrows, marks, shadows) as literals through
//! widget source. Per D7 a single [`Theme`] owns both: a view asks
//! `ctx.theme.style(Role::FrameActive)` and (later) reaches glyphs through
//! [`Glyphs`]. State → role resolution is centralized at each widget's
//! `getColor` → `Role` mapping, which lands when `TFrame`/`TButton` are ported.
//!
//! [`Role`] is a **first-party closed enum** (not a newtype): third parties do
//! not add roles. It **grows per-widget** — seeded here with exactly D7's
//! enumerated needs (active/passive/dragging frames; the
//! normal/focused/disabled/pressed quartet; the list-state matrix; the
//! error/warning/info/success family).

use crate::color::{Color, Style};

/// A semantic colour role. Faithful to D7's "resolve state → role in one
/// centralized mapper": each `getPalette`/`getColor` call site in the C++ maps
/// to one named `Role` here.
///
/// This enum is **closed and first-party** (not app-extensible) and grows as
/// later widgets are ported and need new roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Role {
    /// The desktop background fill.
    Background,
    /// An active (focused) window frame.
    FrameActive,
    /// A passive (unfocused) window frame.
    FramePassive,
    /// A frame being dragged/resized.
    FrameDragging,
    /// A frame icon (close/zoom/resize glyphs).
    FrameIcon,
    /// A scroll-bar page (trough) area.
    ScrollBarPage,
    /// Scroll-bar control glyphs (arrows / thumb).
    ScrollBarControls,
    /// Generic enabled control text.
    Normal,
    /// A focused control.
    Focused,
    /// A disabled (greyed-out) control.
    Disabled,
    /// A pressed control (e.g. a button mid-click).
    Pressed,
    /// A normal (unselected, unfocused) list item.
    ListNormal,
    /// A focused list (its cursor item, list not selected).
    ListFocused,
    /// A selected list item in an unfocused list.
    ListSelected,
    /// The selected item in a focused list.
    ListSelectedFocused,
    /// Error feedback.
    Error,
    /// Warning feedback.
    Warning,
    /// Informational feedback.
    Info,
    /// Success feedback.
    Success,
}

/// Number of [`Role`] variants — the fixed length of [`Theme`]'s style array.
const ROLE_COUNT: usize = 19;

impl Role {
    /// Total mapping of each variant to its index into the style array.
    ///
    /// A `match` (rather than `#[repr(usize)]` games) keeps this explicit and
    /// total; the compiler enforces exhaustiveness when new roles are added.
    fn index(self) -> usize {
        match self {
            Role::Background => 0,
            Role::FrameActive => 1,
            Role::FramePassive => 2,
            Role::FrameDragging => 3,
            Role::FrameIcon => 4,
            Role::ScrollBarPage => 5,
            Role::ScrollBarControls => 6,
            Role::Normal => 7,
            Role::Focused => 8,
            Role::Disabled => 9,
            Role::Pressed => 10,
            Role::ListNormal => 11,
            Role::ListFocused => 12,
            Role::ListSelected => 13,
            Role::ListSelectedFocused => 14,
            Role::Error => 15,
            Role::Warning => 16,
            Role::Info => 17,
            Role::Success => 18,
        }
    }
}

/// Holder for the framework's drawing glyphs — frame corners/tee-connectors,
/// scrollbar arrows, check/radio marks, shadows, window decorations.
///
/// Currently an **empty stub**: the glyph tables land at row 9 / per-widget as
/// each control is ported (D7). It exists now only so [`Theme`] can expose a
/// stable `glyphs()` accessor that `DrawCtx` and later widgets can reach.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Glyphs;

/// A theme: a fixed `Role` → [`Style`] map plus a [`Glyphs`] holder (D7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Theme {
    styles: [Style; ROLE_COUNT],
    glyphs: Glyphs,
}

impl Theme {
    /// The default theme — the classic Turbo-Vision blue look.
    ///
    /// **Provisional colours.** These BIOS values reproduce a plausible classic
    /// blue palette, but real per-role fidelity lands later when `TFrame` /
    /// `TButton` etc. map their `getColor` indices onto [`Role`]s; do not treat
    /// the exact values here as authoritative.
    pub fn classic_blue() -> Self {
        // BIOS 4-bit palette reminder: 0=black 1=blue 2=green 3=cyan 4=red
        // 5=magenta 6=brown 7=lightgray 8=darkgray 9=lightblue ... F=white.
        let mut styles = [Style::default(); ROLE_COUNT];
        let set = |styles: &mut [Style; ROLE_COUNT], role: Role, fg: u8, bg: u8| {
            styles[role.index()] = Style::new(Color::Bios(fg), Color::Bios(bg));
        };

        // Desktop / frames.
        set(&mut styles, Role::Background, 0x7, 0x1); // lightgray on blue
        set(&mut styles, Role::FrameActive, 0xF, 0x1); // white on blue
        set(&mut styles, Role::FramePassive, 0x7, 0x1); // lightgray on blue
        set(&mut styles, Role::FrameDragging, 0xE, 0x1); // yellow on blue
        set(&mut styles, Role::FrameIcon, 0xA, 0x1); // light green on blue
        set(&mut styles, Role::ScrollBarPage, 0x1, 0x3); // blue on cyan
        set(&mut styles, Role::ScrollBarControls, 0x1, 0x3); // blue on cyan

        // Generic control states.
        set(&mut styles, Role::Normal, 0x0, 0x3); // black on cyan
        set(&mut styles, Role::Focused, 0xF, 0x2); // white on green
        set(&mut styles, Role::Disabled, 0x8, 0x1); // darkgray on blue
        set(&mut styles, Role::Pressed, 0xF, 0x2); // white on green

        // List matrix.
        set(&mut styles, Role::ListNormal, 0x7, 0x1); // lightgray on blue
        set(&mut styles, Role::ListFocused, 0xF, 0x1); // white on blue
        set(&mut styles, Role::ListSelected, 0x0, 0x3); // black on cyan
        set(&mut styles, Role::ListSelectedFocused, 0xF, 0x2); // white on green

        // Feedback family.
        set(&mut styles, Role::Error, 0xF, 0x4); // white on red
        set(&mut styles, Role::Warning, 0x0, 0x6); // black on brown
        set(&mut styles, Role::Info, 0xF, 0x1); // white on blue
        set(&mut styles, Role::Success, 0xF, 0x2); // white on green

        Theme {
            styles,
            glyphs: Glyphs,
        }
    }

    /// The [`Style`] for `role`. Total — never panics.
    pub fn style(&self, role: Role) -> Style {
        self.styles[role.index()]
    }

    /// The theme's glyph holder (an empty stub until row 9 / per-widget, D7).
    pub fn glyphs(&self) -> &Glyphs {
        &self.glyphs
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::classic_blue()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant, used to assert totality and to seed expected values.
    const ALL_ROLES: [Role; ROLE_COUNT] = [
        Role::Background,
        Role::FrameActive,
        Role::FramePassive,
        Role::FrameDragging,
        Role::FrameIcon,
        Role::ScrollBarPage,
        Role::ScrollBarControls,
        Role::Normal,
        Role::Focused,
        Role::Disabled,
        Role::Pressed,
        Role::ListNormal,
        Role::ListFocused,
        Role::ListSelected,
        Role::ListSelectedFocused,
        Role::Error,
        Role::Warning,
        Role::Info,
        Role::Success,
    ];

    #[test]
    fn index_is_total_and_distinct() {
        let mut seen = [false; ROLE_COUNT];
        for role in ALL_ROLES {
            let i = role.index();
            assert!(i < ROLE_COUNT);
            assert!(!seen[i], "duplicate index {i} for {role:?}");
            seen[i] = true;
        }
        assert!(seen.iter().all(|&b| b), "every index must be covered");
    }

    #[test]
    fn style_is_total_over_all_variants() {
        let t = Theme::classic_blue();
        // Must not panic for any variant.
        for role in ALL_ROLES {
            let _ = t.style(role);
        }
    }

    #[test]
    fn each_role_returns_its_seeded_style() {
        let t = Theme::classic_blue();
        assert_eq!(
            t.style(Role::Background),
            Style::new(Color::Bios(0x7), Color::Bios(0x1))
        );
        assert_eq!(
            t.style(Role::FrameActive),
            Style::new(Color::Bios(0xF), Color::Bios(0x1))
        );
        assert_eq!(
            t.style(Role::Disabled),
            Style::new(Color::Bios(0x8), Color::Bios(0x1))
        );
        assert_eq!(
            t.style(Role::ListSelected),
            Style::new(Color::Bios(0x0), Color::Bios(0x3))
        );
        assert_eq!(
            t.style(Role::Error),
            Style::new(Color::Bios(0xF), Color::Bios(0x4))
        );
        assert_eq!(
            t.style(Role::Success),
            Style::new(Color::Bios(0xF), Color::Bios(0x2))
        );
    }

    #[test]
    fn default_equals_classic_blue() {
        assert_eq!(Theme::default(), Theme::classic_blue());
    }

    #[test]
    fn glyphs_accessor_returns_stub() {
        let t = Theme::classic_blue();
        assert_eq!(*t.glyphs(), Glyphs);
    }
}
