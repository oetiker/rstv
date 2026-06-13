//! End-to-end render pipeline test (D11, row 19).
//!
//! Proves the full pipeline:
//!   Buffer paint → Renderer diff → HeadlessBackend.draw → snapshot
//!
//! These tests are the verification backbone: every widget test from Phase 1
//! onward follows the same pattern.

use rstv::Theme;
use rstv::backend::{HeadlessBackend, HeadlessHandle, Renderer};
use rstv::color::{Color, Style};
use rstv::event::{Event, Key, KeyModifiers};
use rstv::screen::Buffer;
use rstv::view::{DrawCtx, Point, Rect};

/// Full end-to-end pipeline: paint cells, render, snapshot.
///
/// Paints "Hello" in bright-white-on-blue (BIOS 0xF/0x1) at row 0, leaves
/// the remaining 7 columns and rows 1–2 as default cells.  The cursor is
/// placed at (5, 0).
#[test]
fn renders_text_to_headless_snapshot() {
    let (backend, screen) = HeadlessBackend::new(12, 3);
    let mut r = Renderer::new(Box::new(backend));
    r.set_cursor(Some((5, 0)));
    r.render(|buf: &mut Buffer| {
        let s = Style::new(Color::Bios(0xF), Color::Bios(0x1));
        for (i, ch) in "Hello".chars().enumerate() {
            let c = buf.get_mut(i as u16, 0);
            c.set_char(ch);
            c.set_style(s);
        }
    });
    insta::assert_snapshot!(screen.snapshot());
}

/// The D8 core, end-to-end: the **real widget path** (paint through `DrawCtx`)
/// across **multiple frames**, proving the diff *clears* cells that revert and
/// that a reused back buffer is reset between frames.
///
/// Frame 1 paints "Hello", frame 2 the shorter "Hi", frame 3 "AB". After frame 2
/// the headless screen must read "Hi   " (NOT "Hillo" — the trailing l,l,o must be
/// cleared by the diff). After frame 3 it must read "AB   " (NOT "ABllo" — proving
/// `Renderer::render` reset the *reused* back buffer, which held frame 1's content).
#[test]
fn drawctx_multi_frame_diff_clears_reverted_cells() {
    let theme = Theme::classic_blue();
    let (backend, screen) = HeadlessBackend::new(8, 1);
    let mut r = Renderer::new(Box::new(backend));

    // The current headless screen row, as a String (trailing blanks included).
    let row_text = |screen: &HeadlessHandle| -> String {
        let buf = screen.buffer();
        (0..buf.width())
            .map(|x| buf.get(x, 0).symbol().to_string())
            .collect()
    };

    // Each frame paints through a DrawCtx over the back buffer — exactly the path
    // a real widget's draw() takes (not direct Buffer writes).
    r.render(|buf: &mut Buffer| {
        let mut dc = DrawCtx::new(buf, &theme, Rect::new(0, 0, 8, 1), Point::new(0, 0));
        dc.put_str(0, 0, "Hello", Style::default());
    });
    assert_eq!(row_text(&screen), "Hello   ");

    // Frame 2: shorter string — the diff must CLEAR the trailing "llo".
    r.render(|buf: &mut Buffer| {
        let mut dc = DrawCtx::new(buf, &theme, Rect::new(0, 0, 8, 1), Point::new(0, 0));
        dc.put_str(0, 0, "Hi", Style::default());
    });
    assert_eq!(
        row_text(&screen),
        "Hi      ",
        "diff must clear cells that reverted to blank (D8)"
    );

    // Frame 3: reuses the back buffer that held frame 1's "Hello"; render() must
    // reset() it, else stale "llo" leaks through.
    r.render(|buf: &mut Buffer| {
        let mut dc = DrawCtx::new(buf, &theme, Rect::new(0, 0, 8, 1), Point::new(0, 0));
        dc.put_str(0, 0, "AB", Style::default());
    });
    assert_eq!(
        row_text(&screen),
        "AB      ",
        "reused back buffer must be reset between frames (no stale content)"
    );
}

/// Proves the headless event queue: injected events come back from poll_event.
#[test]
fn headless_event_queue_roundtrip() {
    let (backend, screen) = HeadlessBackend::new(10, 3);
    let mut r = Renderer::new(Box::new(backend));

    // Inject a Ctrl+C key event.
    screen.push_key(
        Key::Char('c'),
        KeyModifiers {
            ctrl: true,
            ..Default::default()
        },
    );
    // Inject a plain Enter.
    screen.push_event(Event::KeyDown(rstv::event::KeyEvent::new(
        Key::Enter,
        KeyModifiers::default(),
    )));

    // Poll should return the first injected event.
    let ev0 = r.backend_mut().poll_event(None);
    match ev0 {
        Some(Event::KeyDown(k)) => {
            assert_eq!(k.key, Key::Char('c'));
            assert!(k.modifiers.ctrl);
            assert!(!k.modifiers.shift);
            assert!(!k.modifiers.alt);
        }
        other => panic!("expected KeyDown(Char('c')+ctrl), got {other:?}"),
    }

    // Second event: Enter.
    let ev1 = r.backend_mut().poll_event(None);
    match ev1 {
        Some(Event::KeyDown(k)) => {
            assert_eq!(k.key, Key::Enter);
            assert_eq!(k.modifiers, KeyModifiers::default());
        }
        other => panic!("expected KeyDown(Enter), got {other:?}"),
    }

    // Queue is now empty.
    let ev2 = r.backend_mut().poll_event(None);
    assert!(ev2.is_none(), "expected empty queue, got {ev2:?}");
}
