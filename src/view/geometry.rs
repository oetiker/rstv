//! Geometry primitives — faithful port of `TPoint` and `TRect` (`objects.h`).
//!
//! magiblot widened the historical `short` coordinates to `int`; we follow the
//! source of truth and use `i32`. Signed is required regardless: view origins go
//! negative when scrolled offscreen, and `move`/`grow` take negative deltas.
//! Conversion to the unsigned buffer index space happens at the screen boundary.
//!
//! ### Keyword-collision renames
//! `TRect::move` and `TRect::Union` collide with Rust keywords. We keep the
//! faithful names via raw identifiers: [`Rect::r#move`] and [`Rect::r#union`]
//! (call sites read `rect.r#move(1, 2)` / `rect.r#union(&other)`). C++ already
//! capitalized `Union` for the very same reason.

use std::ops::{Add, AddAssign, Sub, SubAssign};

/// A point on the screen. Faithful port of `TPoint` (`objects.h`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    /// Construct a point. (C++ used aggregate initialization `{x, y}`.)
    pub const fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }
}

impl Add for Point {
    type Output = Point;
    fn add(self, rhs: Point) -> Point {
        Point::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl Sub for Point {
    type Output = Point;
    fn sub(self, rhs: Point) -> Point {
        Point::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl AddAssign for Point {
    fn add_assign(&mut self, rhs: Point) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

impl SubAssign for Point {
    fn sub_assign(&mut self, rhs: Point) {
        self.x -= rhs.x;
        self.y -= rhs.y;
    }
}

/// A rectangle defined by its top-left corner `a` (inclusive) and bottom-right
/// corner `b` (exclusive). Faithful port of `TRect` (`objects.h`).
///
/// The mutating methods take `&mut self` and return `&mut Self`, mirroring C++'s
/// `TRect&` return so the chaining/in-place idioms (`r.grow(-1, -1)`) port
/// straight across.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rect {
    pub a: Point,
    pub b: Point,
}

impl Rect {
    /// `TRect(ax, ay, bx, by)`.
    pub const fn new(ax: i32, ay: i32, bx: i32, by: i32) -> Self {
        Rect {
            a: Point::new(ax, ay),
            b: Point::new(bx, by),
        }
    }

    /// `TRect(TPoint p1, TPoint p2)`.
    pub const fn from_points(p1: Point, p2: Point) -> Self {
        Rect { a: p1, b: p2 }
    }

    /// `TRect::move` — translate both corners by `(dx, dy)`.
    pub fn r#move(&mut self, dx: i32, dy: i32) -> &mut Self {
        self.a.x += dx;
        self.a.y += dy;
        self.b.x += dx;
        self.b.y += dy;
        self
    }

    /// `TRect::grow` — inflate (or deflate, for negative args) about the centre:
    /// pull `a` out by `(dx, dy)` and push `b` out by `(dx, dy)`.
    pub fn grow(&mut self, dx: i32, dy: i32) -> &mut Self {
        self.a.x -= dx;
        self.a.y -= dy;
        self.b.x += dx;
        self.b.y += dy;
        self
    }

    /// `TRect::intersect` — clip to the overlap with `r`.
    pub fn intersect(&mut self, r: &Rect) -> &mut Self {
        self.a.x = self.a.x.max(r.a.x);
        self.a.y = self.a.y.max(r.a.y);
        self.b.x = self.b.x.min(r.b.x);
        self.b.y = self.b.y.min(r.b.y);
        self
    }

    /// `TRect::Union` — expand to the bounding box of `self` and `r`.
    pub fn r#union(&mut self, r: &Rect) -> &mut Self {
        self.a.x = self.a.x.min(r.a.x);
        self.a.y = self.a.y.min(r.a.y);
        self.b.x = self.b.x.max(r.b.x);
        self.b.y = self.b.y.max(r.b.y);
        self
    }

    /// `TRect::contains` — half-open test: the right/bottom edges are *excluded*
    /// (`p.x < b.x`, `p.y < b.y`).
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.a.x && p.x < self.b.x && p.y >= self.a.y && p.y < self.b.y
    }

    /// `TRect::isEmpty` — true when the rect has no area.
    pub fn is_empty(&self) -> bool {
        self.a.x >= self.b.x || self.a.y >= self.b.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_arithmetic() {
        let p = Point::new(3, 4) + Point::new(1, 2);
        assert_eq!(p, Point::new(4, 6));
        assert_eq!(Point::new(3, 4) - Point::new(1, 2), Point::new(2, 2));

        let mut q = Point::new(10, 10);
        q += Point::new(5, -5);
        assert_eq!(q, Point::new(15, 5));
        q -= Point::new(5, 5);
        assert_eq!(q, Point::new(10, 0));
    }

    #[test]
    fn rect_constructors_equivalent() {
        let a = Rect::new(1, 2, 3, 4);
        let b = Rect::from_points(Point::new(1, 2), Point::new(3, 4));
        assert_eq!(a, b);
        assert_eq!(Rect::default(), Rect::new(0, 0, 0, 0));
    }

    #[test]
    fn rect_move_and_grow() {
        let mut r = Rect::new(0, 0, 10, 10);
        r.r#move(2, 3);
        assert_eq!(r, Rect::new(2, 3, 12, 13));

        r.grow(1, 1);
        assert_eq!(r, Rect::new(1, 2, 13, 14));

        // negative grow shrinks
        r.grow(-1, -1);
        assert_eq!(r, Rect::new(2, 3, 12, 13));

        // chaining returns &mut Self
        let mut c = Rect::new(0, 0, 4, 4);
        c.r#move(1, 1).grow(1, 1);
        assert_eq!(c, Rect::new(0, 0, 6, 6));
    }

    #[test]
    fn rect_intersect_and_union() {
        let mut r = Rect::new(0, 0, 10, 10);
        r.intersect(&Rect::new(5, 5, 20, 8));
        assert_eq!(r, Rect::new(5, 5, 10, 8));

        let mut u = Rect::new(0, 0, 4, 4);
        u.r#union(&Rect::new(2, 2, 10, 6));
        assert_eq!(u, Rect::new(0, 0, 10, 6));
    }

    #[test]
    fn rect_contains_is_half_open() {
        let r = Rect::new(0, 0, 10, 5);
        // inside
        assert!(r.contains(Point::new(0, 0)));
        assert!(r.contains(Point::new(9, 4)));
        // right/bottom edges are EXCLUDED (classic off-by-one trap)
        assert!(!r.contains(Point::new(10, 0)));
        assert!(!r.contains(Point::new(0, 5)));
        assert!(!r.contains(Point::new(10, 5)));
        // left/top edges are INCLUDED
        assert!(r.contains(Point::new(0, 4)));
    }

    #[test]
    fn rect_is_empty() {
        assert!(Rect::new(0, 0, 0, 0).is_empty());
        assert!(Rect::new(5, 0, 5, 10).is_empty()); // zero width
        assert!(Rect::new(0, 5, 10, 5).is_empty()); // zero height
        assert!(Rect::new(3, 4, 2, 1).is_empty()); // inverted
        assert!(!Rect::new(0, 0, 1, 1).is_empty());
    }
}
