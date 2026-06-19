//! `tcv` — **Tobi's Catalog Vision**, a homage re-port of a real 1993 Turbo
//! Pascal / Turbo Vision program (TCV v2.2, a floppy-disk catalog browser by
//! Tobias Oetiker) into idiomatic tvision-rs.
//!
//! The original read a `PROGS.TFC` text file listing every file on a stack of
//! catalogued floppies; this port embeds a small period-appropriate mock
//! catalog instead. The soul of the program is its **search-as-you-type**
//! browser (`TDirBox`): type a word and the list jumps to the next entry whose
//! rendered line contains it (case-insensitive), highlighting the match.
//!
//! Run it:  `cargo run --example tcv`
//!   - Up/Down browse the catalog.
//!   - Start typing to search; the list jumps to the next matching entry and
//!     highlights the matched substring.
//!   - In search mode, Up/Down jump to the previous/next match.
//!   - Backspace shortens the search; Esc returns to browse mode.
//!   - Enter or double-click opens the Info box for the focused entry.
//!   - The `~I~nfo` / `~A~bout` / `E~x~it` buttons do the obvious things.
//!
//! The C++/Pascal classes this mirrors: `TTCV : TApplication`, the data window
//! (`TDialog`), `TDirBox : TListBox` (the search list), `TDiskCol`
//! (the catalog collection + `DirLine`), and `TTCVStatLine : TStatusLine` (the
//! context-sensitive hint line).

use std::io;

use tvision_rs::widgets::list_viewer;
use tvision_rs::{
    Backend, Button, ButtonFlags, Command, Context, CrosstermBackend, Desktop, Dialog, DrawCtx,
    Event, GrowMode, HelpCtx, Key, Label, ListViewer, ListViewerState, MessageBoxButtons,
    MessageBoxKind, Point, Program, Rect, Role, ScrollBar, StateFlag, StaticText, StatusDef,
    StatusLine, SystemClock, Theme, View, ViewId, ViewState, WindowFlags, alt, delegate,
};

// ---------------------------------------------------------------------------
// Commands & help contexts (port of the cm*/hc* constants in TCV.PAS)
// ---------------------------------------------------------------------------

const CMD_INFO: Command = Command::custom("tcv.info");
const CMD_ABOUT: Command = Command::custom("tcv.about");

const HC_BROWSE_MODE: HelpCtx = HelpCtx::custom("tcv.browse_mode");
const HC_SEARCH_MODE: HelpCtx = HelpCtx::custom("tcv.search_mode");

// ---------------------------------------------------------------------------
// Catalog data (replaces reading PROGS.TFC)
// ---------------------------------------------------------------------------

/// One catalogued file, mirroring the six `"..."`-delimited fields of a
/// `PROGS.TFC` line (`disk`, `date`, `file`, `size`, `description`, `scan`).
struct Entry {
    /// Disk (volume) label.
    disk: &'static str,
    /// File date (as stamped on the disk).
    date: &'static str,
    /// File name.
    file: &'static str,
    /// Size in bytes.
    size: u32,
    /// Description / comment.
    desc: &'static str,
    /// Date the disk was scanned into the catalog.
    scan: &'static str,
}

/// A charming, period-appropriate early-90s shareware/utility floppy catalog.
static CATALOG: &[Entry] = &[
    Entry {
        disk: "GAMES01",
        date: "10-12-93",
        file: "DOOM1_0.ZIP",
        size: 2_311_046,
        desc: "id Software shareware DOOM v1.0",
        scan: "11-03-93",
    },
    Entry {
        disk: "GAMES01",
        date: "09-30-93",
        file: "WOLF3D.ZIP",
        size: 716_800,
        desc: "Wolfenstein 3D shareware episode 1",
        scan: "11-03-93",
    },
    Entry {
        disk: "GAMES02",
        date: "06-15-92",
        file: "COMMANDR.ZIP",
        size: 458_240,
        desc: "Commander Keen 4 shareware",
        scan: "11-03-93",
    },
    Entry {
        disk: "GAMES02",
        date: "03-21-93",
        file: "JAZZJACK.ZIP",
        size: 1_204_224,
        desc: "Jazz Jackrabbit demo",
        scan: "11-03-93",
    },
    Entry {
        disk: "UTILS03",
        date: "01-15-93",
        file: "PKZIP204.EXE",
        size: 199_245,
        desc: "PKWARE PKZIP/PKUNZIP v2.04g",
        scan: "11-04-93",
    },
    Entry {
        disk: "UTILS03",
        date: "08-02-92",
        file: "ARJ241.EXE",
        size: 121_734,
        desc: "ARJ archiver v2.41 by Robert Jung",
        scan: "11-04-93",
    },
    Entry {
        disk: "UTILS03",
        date: "11-11-91",
        file: "LHA213.EXE",
        size: 50_018,
        desc: "LHA compression utility v2.13",
        scan: "11-04-93",
    },
    Entry {
        disk: "UTILS04",
        date: "04-04-93",
        file: "4DOS502.ZIP",
        size: 412_900,
        desc: "4DOS command interpreter v5.02",
        scan: "11-04-93",
    },
    Entry {
        disk: "UTILS04",
        date: "07-19-92",
        file: "LIST92.ZIP",
        size: 60_416,
        desc: "Vernon Buerg's LIST file viewer",
        scan: "11-04-93",
    },
    Entry {
        disk: "GRAPHICS",
        date: "05-23-93",
        file: "FRACTINT.ZIP",
        size: 893_120,
        desc: "Stone Soup fractal generator v18",
        scan: "11-05-93",
    },
    Entry {
        disk: "GRAPHICS",
        date: "02-14-93",
        file: "VPIC61.ZIP",
        size: 147_456,
        desc: "VPIC image viewer v6.1",
        scan: "11-05-93",
    },
    Entry {
        disk: "GRAPHICS",
        date: "12-01-92",
        file: "POVRAY10.ZIP",
        size: 655_360,
        desc: "Persistence of Vision raytracer 1.0",
        scan: "11-05-93",
    },
    Entry {
        disk: "SOUND01",
        date: "03-30-93",
        file: "MODPLAY.ZIP",
        size: 73_728,
        desc: "ModEdit / MOD music player",
        scan: "11-05-93",
    },
    Entry {
        disk: "SOUND01",
        date: "06-06-93",
        file: "SBOS.ZIP",
        size: 98_304,
        desc: "Sound Blaster OS drivers",
        scan: "11-05-93",
    },
    Entry {
        disk: "COMMS02",
        date: "09-09-93",
        file: "TELIX321.ZIP",
        size: 524_288,
        desc: "Telix terminal program v3.21",
        scan: "11-06-93",
    },
    Entry {
        disk: "COMMS02",
        date: "07-07-92",
        file: "QMODEM46.ZIP",
        size: 466_944,
        desc: "Qmodem modem terminal v4.6",
        scan: "11-06-93",
    },
    Entry {
        disk: "PROGRAM",
        date: "01-30-93",
        file: "TPASCAL7.ZIP",
        size: 1_310_720,
        desc: "Borland Turbo Pascal 7.0 patches",
        scan: "11-06-93",
    },
    Entry {
        disk: "PROGRAM",
        date: "10-10-92",
        file: "DJGPP.ZIP",
        size: 2_097_152,
        desc: "DJ Delorie's GCC port for DOS",
        scan: "11-06-93",
    },
    Entry {
        disk: "EDITORS",
        date: "08-18-93",
        file: "QEDIT21.ZIP",
        size: 184_320,
        desc: "QEdit text editor v2.1",
        scan: "11-07-93",
    },
    Entry {
        disk: "EDITORS",
        date: "11-25-92",
        file: "VDE166.ZIP",
        size: 110_592,
        desc: "VDE WordStar-style editor v1.66",
        scan: "11-07-93",
    },
    // --- Batch 1: More archivers / compression tools (PKZIP, ARJ, LHA clusters) ---
    Entry {
        disk: "UTILS03",
        date: "02-01-91",
        file: "PKZIP110.EXE",
        size: 56_312,
        desc: "PKWARE PKZIP/PKUNZIP v1.10",
        scan: "11-04-93",
    },
    Entry {
        disk: "UTILS03",
        date: "10-01-92",
        file: "PKZIP193.EXE",
        size: 88_455,
        desc: "PKWARE PKZIP/PKUNZIP v1.93",
        scan: "11-04-93",
    },
    Entry {
        disk: "UTILS05",
        date: "11-14-93",
        file: "PKLITE15.EXE",
        size: 34_276,
        desc: "PKLITE executable compressor v1.5",
        scan: "11-08-93",
    },
    Entry {
        disk: "UTILS05",
        date: "06-30-92",
        file: "PKLITE12.EXE",
        size: 29_184,
        desc: "PKLITE executable compressor v1.2",
        scan: "11-08-93",
    },
    Entry {
        disk: "UTILS05",
        date: "03-15-91",
        file: "ARJ230.EXE",
        size: 98_304,
        desc: "ARJ archiver v2.30 by Robert Jung",
        scan: "11-08-93",
    },
    Entry {
        disk: "UTILS05",
        date: "04-22-92",
        file: "LHA255.EXE",
        size: 52_736,
        desc: "LHA compression utility v2.55",
        scan: "11-08-93",
    },
    Entry {
        disk: "UTILS06",
        date: "09-10-93",
        file: "LHA265.EXE",
        size: 54_272,
        desc: "LHA compression utility v2.65",
        scan: "11-08-93",
    },
    Entry {
        disk: "UTILS06",
        date: "03-01-93",
        file: "ZOO210.EXE",
        size: 41_500,
        desc: "ZOO archive utility v2.10 by Rahul Dhesi",
        scan: "11-08-93",
    },
    Entry {
        disk: "UTILS06",
        date: "07-07-93",
        file: "STACKER3.ZIP",
        size: 812_032,
        desc: "Stacker disk compression v3.0 demo",
        scan: "11-08-93",
    },
    Entry {
        disk: "UTILS06",
        date: "10-18-93",
        file: "STACKER4.ZIP",
        size: 877_568,
        desc: "Stacker disk compression v4.0 demo",
        scan: "11-09-93",
    },
    // --- Batch 2: Games (DOOM/Wolf/Keen/Apogee/Epic cluster) ---
    Entry {
        disk: "GAMES01",
        date: "11-16-93",
        file: "DOOM1_2.ZIP",
        size: 2_389_114,
        desc: "id Software shareware DOOM v1.2",
        scan: "11-20-93",
    },
    Entry {
        disk: "GAMES03",
        date: "05-05-92",
        file: "WOLF3D14.ZIP",
        size: 741_376,
        desc: "Wolfenstein 3D shareware v1.4 episode 1",
        scan: "11-10-93",
    },
    Entry {
        disk: "GAMES03",
        date: "12-10-91",
        file: "KEEN4E.ZIP",
        size: 512_000,
        desc: "Commander Keen 4 episode shareware",
        scan: "11-10-93",
    },
    Entry {
        disk: "GAMES03",
        date: "04-01-93",
        file: "DUKE2.ZIP",
        size: 684_032,
        desc: "Duke Nukem 2 shareware episode 1",
        scan: "11-10-93",
    },
    Entry {
        disk: "GAMES04",
        date: "07-15-93",
        file: "DUKENUKEM.ZIP",
        size: 341_248,
        desc: "Duke Nukem shareware Apogee v1.0",
        scan: "11-10-93",
    },
    Entry {
        disk: "GAMES04",
        date: "09-01-93",
        file: "RAPTOR.ZIP",
        size: 1_024_000,
        desc: "Raptor: Call of the Shadows shareware demo",
        scan: "11-10-93",
    },
    Entry {
        disk: "GAMES04",
        date: "01-18-93",
        file: "EPIC_P.ZIP",
        size: 786_432,
        desc: "Epic Pinball shareware v1.0 demo",
        scan: "11-10-93",
    },
    Entry {
        disk: "GAMES04",
        date: "11-01-92",
        file: "ZONE66.ZIP",
        size: 632_320,
        desc: "Zone 66 shareware episode demo",
        scan: "11-10-93",
    },
    // --- Batch 3: More games + BBS/comms tools ---
    Entry {
        disk: "GAMES05",
        date: "08-20-93",
        file: "HERETIC.ZIP",
        size: 2_097_152,
        desc: "Heretic shareware episode 1 demo",
        scan: "11-21-93",
    },
    Entry {
        disk: "GAMES05",
        date: "06-03-93",
        file: "BLAKE2.ZIP",
        size: 918_528,
        desc: "Blake Stone shareware Apogee demo",
        scan: "11-11-93",
    },
    Entry {
        disk: "GAMES05",
        date: "02-28-93",
        file: "COSMO1.ZIP",
        size: 449_024,
        desc: "Cosmo's Cosmic Adventure episode 1 shareware",
        scan: "11-11-93",
    },
    Entry {
        disk: "DEMOS01",
        date: "10-15-93",
        file: "FUTURE.ZIP",
        size: 1_384_448,
        desc: "Future Crew Second Reality demo v1.0",
        scan: "11-11-93",
    },
    Entry {
        disk: "DEMOS01",
        date: "07-22-93",
        file: "UNREAL.ZIP",
        size: 1_245_184,
        desc: "Second Reality Unreal demo fractal engine",
        scan: "11-11-93",
    },
    Entry {
        disk: "BBS01",
        date: "09-01-93",
        file: "RBBS25.ZIP",
        size: 512_000,
        desc: "RBBS-PC BBS software v25.0",
        scan: "11-12-93",
    },
    Entry {
        disk: "BBS01",
        date: "08-15-93",
        file: "SPITFIRE.ZIP",
        size: 368_640,
        desc: "Spitfire BBS v3.5 bulletin board system",
        scan: "11-12-93",
    },
    Entry {
        disk: "BBS01",
        date: "05-10-93",
        file: "WILDCAT.ZIP",
        size: 614_400,
        desc: "Wildcat! BBS v4.01 multi-line bulletin board",
        scan: "11-12-93",
    },
    // --- Batch 4: Modem/comms, viewers, editors ---
    Entry {
        disk: "MODEM01",
        date: "04-19-93",
        file: "PROCOMM.ZIP",
        size: 438_272,
        desc: "ProComm Plus modem terminal demo",
        scan: "11-13-93",
    },
    Entry {
        disk: "MODEM01",
        date: "11-03-92",
        file: "XMODEM.ZIP",
        size: 22_528,
        desc: "XMODEM/YMODEM/ZMODEM file transfer util",
        scan: "11-13-93",
    },
    Entry {
        disk: "MODEM01",
        date: "02-20-93",
        file: "ZMODEM.ZIP",
        size: 31_744,
        desc: "ZMODEM fast modem file transfer protocol",
        scan: "11-13-93",
    },
    Entry {
        disk: "UTILS07",
        date: "06-17-93",
        file: "LIST91.ZIP",
        size: 59_392,
        desc: "Vernon Buerg's LIST file viewer v9.1",
        scan: "11-13-93",
    },
    Entry {
        disk: "UTILS07",
        date: "10-05-93",
        file: "LIST92B.ZIP",
        size: 61_440,
        desc: "Vernon Buerg's LIST file viewer v9.2b",
        scan: "11-13-93",
    },
    Entry {
        disk: "UTILS07",
        date: "03-10-93",
        file: "AVIEW10.ZIP",
        size: 88_064,
        desc: "ASCII art viewer v1.0 ANSI/RIP viewer",
        scan: "11-13-93",
    },
    Entry {
        disk: "EDITORS",
        date: "04-12-92",
        file: "QEDIT18.ZIP",
        size: 151_552,
        desc: "QEdit text editor v1.8 for DOS",
        scan: "11-14-93",
    },
    Entry {
        disk: "EDITORS",
        date: "09-21-93",
        file: "ELVIS2.ZIP",
        size: 209_920,
        desc: "Elvis vi-clone editor v2.0 for DOS",
        scan: "11-14-93",
    },
    // --- Batch 5: Graphics, sound, screen savers, benchmarks ---
    Entry {
        disk: "GRAPHICS",
        date: "07-30-93",
        file: "FRACTINT2.ZIP",
        size: 921_600,
        desc: "Stone Soup fractal generator v19.2",
        scan: "11-14-93",
    },
    Entry {
        disk: "GRAPHICS",
        date: "01-08-93",
        file: "VPIC52.ZIP",
        size: 128_000,
        desc: "VPIC image viewer v5.2 GIF/PCX/TIFF",
        scan: "11-14-93",
    },
    Entry {
        disk: "GRAPHICS",
        date: "11-10-92",
        file: "POVRAY20.ZIP",
        size: 713_728,
        desc: "Persistence of Vision raytracer 2.0",
        scan: "11-14-93",
    },
    Entry {
        disk: "SOUND01",
        date: "08-01-93",
        file: "MPPLAY.ZIP",
        size: 56_320,
        desc: "MPPlay MOD player for Sound Blaster",
        scan: "11-15-93",
    },
    Entry {
        disk: "SOUND02",
        date: "10-25-93",
        file: "ST3DEMO.ZIP",
        size: 1_572_864,
        desc: "Scream Tracker 3 MOD player demo",
        scan: "11-15-93",
    },
    Entry {
        disk: "SOUND02",
        date: "06-11-93",
        file: "SBPLAY.ZIP",
        size: 44_032,
        desc: "Sound Blaster SB player VOC/WAV util",
        scan: "11-15-93",
    },
    Entry {
        disk: "UTILS08",
        date: "05-15-93",
        file: "SCRNSAV3.ZIP",
        size: 67_584,
        desc: "Screen Saver 3 ANSI blanker with stars",
        scan: "11-15-93",
    },
    Entry {
        disk: "UTILS08",
        date: "03-28-93",
        file: "SCRNSAV2.ZIP",
        size: 61_440,
        desc: "Screen Saver 2 blank/protect DOS screen",
        scan: "11-15-93",
    },
    Entry {
        disk: "UTILS08",
        date: "08-30-93",
        file: "SPEEDIT.ZIP",
        size: 38_912,
        desc: "SpeedIT benchmark CPU/disk benchmark",
        scan: "11-15-93",
    },
    Entry {
        disk: "UTILS08",
        date: "11-01-93",
        file: "SYSCHK.ZIP",
        size: 47_104,
        desc: "SysCheck system benchmark v3.1",
        scan: "11-15-93",
    },
    // --- Batch 6: Programming tools, extra utils ---
    Entry {
        disk: "PROGRAM01",
        date: "06-22-93",
        file: "TASM30.ZIP",
        size: 524_288,
        desc: "Borland Turbo Assembler v3.0 patches",
        scan: "11-16-93",
    },
    Entry {
        disk: "PROGRAM01",
        date: "03-05-92",
        file: "NASM08.ZIP",
        size: 118_784,
        desc: "NASM x86 assembler v0.8 free",
        scan: "11-16-93",
    },
    Entry {
        disk: "PROGRAM01",
        date: "09-18-93",
        file: "RHIDE.ZIP",
        size: 346_112,
        desc: "RHIDE IDE for DJGPP GCC port DOS editor",
        scan: "11-16-93",
    },
    Entry {
        disk: "PROGRAM02",
        date: "01-12-94",
        file: "DJGPP2.ZIP",
        size: 2_228_224,
        desc: "DJ Delorie's GCC port for DOS v2.0",
        scan: "01-20-94",
    },
    Entry {
        disk: "UTILS09",
        date: "10-01-93",
        file: "4DOS55.ZIP",
        size: 421_888,
        desc: "4DOS command interpreter v5.5",
        scan: "11-16-93",
    },
    Entry {
        disk: "UTILS09",
        date: "07-04-93",
        file: "NORTON7.ZIP",
        size: 892_928,
        desc: "Norton Utilities v7.0 demo disk toolkit",
        scan: "11-17-93",
    },
    Entry {
        disk: "UTILS09",
        date: "04-20-93",
        file: "PCSHELL.ZIP",
        size: 789_504,
        desc: "PC Shell file manager v6.0 viewer",
        scan: "11-17-93",
    },
    Entry {
        disk: "UTILS10",
        date: "12-20-93",
        file: "PKZIP204C.EXE",
        size: 201_843,
        desc: "PKWARE PKZIP/PKUNZIP v2.04c beta",
        scan: "01-05-94",
    },
];

/// Format a catalog entry as one browser line — `TDiskCol.DirLine`: a leading
/// space, the disk label padded to 14, the file name padded to 15, then the
/// description.
fn dir_line(e: &Entry) -> String {
    format!(" {:<14}{:<15}{}", e.disk, e.file, e.desc)
}

/// The Info box body for entry `e` — the six labelled fields the original
/// `InfoBox` showed as static text lines (`TDirBox.HandleEvent.InfoBox`).
fn info_text(e: &Entry) -> String {
    format!(
        "Disk Label:  {}\nFile Name:   {}\nFile Date:   {}\n\
         Space Used:  {} Bytes\nDescription: {}\nScan Date:   {}",
        e.disk, e.file, e.date, e.size, e.desc, e.scan
    )
}

/// The `cmAbout` MessageBox body — the author's address homage, plus a re-port
/// note. `\x03` centers a line (the C++ `#3` center marker).
const ABOUT_TEXT: &str = "\x03CREATED in Nov '93 BY\n\n\
     \x03Tobias Oetiker\n\
     \x03Gallusstrasse 25\n\
     \x03CH-4600 Olten\n\
     \x03Switzerland\n\n\
     \x03eMail oetiker@stud.ee.ethz.ch\n\n\
     \x03USING Turbo Pascal 7.0 and Turbo Vision\n\n\
     \x03Re-ported to Rust with tvision-rs, 2026.";

/// Case-insensitive substring search — `NoCasePos`. Returns the byte position
/// (1-based, like Pascal's `Pos`) of `needle` in `haystack`, or 0 if absent. We
/// work in chars for the highlight math; here char index + 1.
fn no_case_pos(needle: &str, haystack: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    let h: Vec<char> = haystack.chars().map(|c| c.to_ascii_uppercase()).collect();
    let n: Vec<char> = needle.chars().map(|c| c.to_ascii_uppercase()).collect();
    if n.len() > h.len() {
        return 0;
    }
    for start in 0..=(h.len() - n.len()) {
        if h[start..start + n.len()] == n[..] {
            return start + 1;
        }
    }
    0
}

// ANCHOR: info_dialog
/// Build the Info dialog for entry `e` — the original `TDirBox.HandleEvent`
/// `InfoBox`, now a real custom `Dialog` launched via `request_exec_view`
/// (consumer-API gap #2 closed).
///
/// The dialog is titled "Information", sized 62 × 10, and shows the entry's
/// six fields as static text rows (one `StaticText` spanning all six lines).
/// An OK `Button` whose command is `Command::CANCEL` closes the modal
/// (read-only-info convention: `cmCancel` means "dismiss", nothing is read back).
///
/// Width is chosen so the longest description in the catalog
/// ("Description: Cosmo's Cosmic Adventure episode 1 shareware", 57 chars)
/// fits on one row without wrapping: content width = w - 4 = 58 >= 57.
fn build_info_dialog(e: &Entry) -> Dialog {
    // Inner content width: 62 - 2 (frame) - 2 (margin each side) = 58 chars.
    // Inner content height: 10 - 2 (top/bottom frame) = 8 usable rows.
    // Layout: rows 1-6 = static text, row 7 = spacer, row 8-9 = button (h=2).
    let w: i32 = 62;
    let h: i32 = 10;
    let mut dialog = Dialog::new(Rect::new(0, 0, w, h), Some("Information".to_string()));

    // Six-line static text block: all six labelled fields from `info_text`.
    dialog.insert_child(Box::new(StaticText::new(
        Rect::new(2, 1, w - 2, 7),
        info_text(e),
    )));

    // OK button centered in the bottom row (row 8, height 2).
    let btn_w: i32 = 8;
    let btn_x = (w - btn_w) / 2;
    dialog.insert_child(Box::new(Button::new(
        Rect::new(btn_x, 7, btn_x + btn_w, 9),
        "~O~K",
        Command::CANCEL, // read-only-info convention: OK = cmCancel (just dismiss)
        ButtonFlags::new(),
    )));

    dialog
}
// ANCHOR_END: info_dialog

// ---------------------------------------------------------------------------
// DirBox — port of `TDirBox : TListBox`, the search-as-you-type catalog list.
//
// A `TListViewer` subtype (here implemented over `ListViewerState` + the
// `list_viewer` free functions, the trait realisation of the C++ abstract
// base). It overrides `draw` (to highlight the search match) and `handle_event`
// (the incremental substring search), keeping the base list nav for browse
// mode.
// ---------------------------------------------------------------------------

struct DirBox {
    lv: ListViewerState,
    /// The accumulated search string (`TDirBox.Search`). Empty = browse mode.
    search: String,
    /// One-shot guard for the post-insert scrollbar wiring (see `ensure_inited`).
    inited: bool,
}

impl DirBox {
    fn new(bounds: Rect, h: Option<ViewId>, v: Option<ViewId>) -> Self {
        let mut lv = ListViewerState::new(bounds, 1, h, v);
        lv.range = CATALOG.len() as i32;
        lv.state.help_ctx = HC_BROWSE_MODE;
        DirBox {
            lv,
            search: String::new(),
            inited: false,
        }
    }

    /// Publish the vertical scrollbar's range + page/arrow steps.
    ///
    /// `ListViewerState::new` cannot reach the sibling scrollbar (no `Context`),
    /// so it leaves the bar at its default `max = 0` — a zero-range bar whose
    /// thumb is pinned and never moves. The scrollbar must be told the list
    /// length (`set_range` → bar `max = range - 1`) and its page/arrow steps
    /// (`update_steps`) *after insertion*, with a `Context`. `ListBox` does this
    /// via its `new_list` consumer; `DirBox` populates from a static `CATALOG`
    /// in the constructor, so it runs the same publish lazily on the first
    /// `handle_event` (the earliest point a `Context` and a resolvable bar id
    /// are both available). Idempotent via `self.inited`.
    fn ensure_inited(&mut self, ctx: &mut Context) {
        if self.inited {
            return;
        }
        self.inited = true;
        let range = self.lv.range;
        list_viewer::set_range(self, range, ctx);
        list_viewer::update_steps(self, ctx);
    }

    /// `TDiskCol.FindNext` — first index `>= start` whose line contains `key`
    /// (case-insensitive), or `start` if none.
    fn find_next(&self, start: i32, key: &str) -> i32 {
        let range = self.lv.range;
        if start >= 0 && start < range && !key.is_empty() {
            let mut i = start;
            while i < range {
                if no_case_pos(key, &self.get_text(i)) != 0 {
                    return i;
                }
                i += 1;
            }
            start
        } else {
            0
        }
    }

    /// `TDiskCol.FindPrev` — last index `< start` whose line contains `key`
    /// (case-insensitive), walking downwards; `start` if none / no key.
    fn find_prev(&self, start: i32, key: &str) -> i32 {
        if start >= 1 && !key.is_empty() {
            let mut i = start;
            while i >= 1 {
                i -= 1;
                if no_case_pos(key, &self.get_text(i)) != 0 {
                    return i;
                }
            }
            i
        } else {
            start
        }
    }

    /// Switch help context to match the current mode and refresh the status line
    /// (the window forwards our context via `get_help_ctx`).
    fn sync_mode(&mut self) {
        self.lv.state.help_ctx = if self.search.is_empty() {
            HC_BROWSE_MODE
        } else {
            HC_SEARCH_MODE
        };
    }

    /// Whether this view is the active/focused leaf (so search keys apply) —
    /// the port of the original's `Owner^.Phase = phFocused` guard.
    fn is_focused(&self) -> bool {
        self.lv.state.state.selected && self.lv.state.state.active
    }

    /// Open the Info dialog for the focused entry (the original `InfoBox`),
    /// built via `build_info_dialog` and launched as a real custom `Dialog`
    /// through `request_exec_view` (consumer-API gap #2 closed).
    fn open_info(&mut self, ctx: &mut Context) {
        if let Some(e) = CATALOG.get(self.lv.focused as usize) {
            let dialog = build_info_dialog(e);
            if let Some(id) = self.state().id() {
                ctx.request_exec_view(Box::new(dialog), id, None);
            }
        }
    }
}

impl ListViewer for DirBox {
    fn lv(&self) -> &ListViewerState {
        &self.lv
    }
    fn lv_mut(&mut self) -> &mut ListViewerState {
        &mut self.lv
    }
    fn get_text(&self, item: i32) -> String {
        CATALOG.get(item as usize).map(dir_line).unwrap_or_default()
    }
}

impl View for DirBox {
    fn state(&self) -> &ViewState {
        &self.lv.state
    }
    fn state_mut(&mut self) -> &mut ViewState {
        &mut self.lv.state
    }

    fn draw(&mut self, ctx: &mut DrawCtx) {
        // Without an active search this is just the base list draw.
        if self.search.is_empty() {
            list_viewer::draw(self, ctx);
            return;
        }

        // Search mode: base draw first (colors every row + the focused cell),
        // then overlay the matched substring of the focused row in a contrasting
        // style — the C++ `TDirBox.Draw` mark color (GetColor(5) == ListSelected).
        list_viewer::draw(self, ctx);

        let focused = self.lv.focused;
        let top = self.lv.top_item;
        let size = self.lv.state.size;
        let row = focused - top;
        if row < 0 || row >= size.y {
            return;
        }
        let line = self.get_text(focused);
        let pos = no_case_pos(&self.search, &line); // 1-based char index
        if pos == 0 {
            return;
        }
        let match_len = self.search.chars().count();
        let start = pos - 1;

        let active = self.is_focused();
        let base = ctx.style(if active {
            Role::ListFocused
        } else {
            Role::ListNormalInactive
        });
        let mark = ctx.style(Role::ListSelected);

        // The list draw renders text starting one column in (col 0 is the
        // cell's left pad); re-render the focused row in three styled spans so
        // the matched chars stand out.
        let pre: String = line.chars().take(start).collect();
        let hit: String = line.chars().skip(start).take(match_len).collect();
        let rest: String = line.chars().skip(start + match_len).collect();
        // `put_str` returns the *width* it drew (columns advanced), not the next
        // absolute column — so accumulate onto `x` to keep the three spans
        // contiguous (the base list draw starts text at column 1).
        let mut x = 1;
        x += ctx.put_str(x, row, &pre, base);
        x += ctx.put_str(x, row, &hit, mark);
        ctx.put_str(x, row, &rest, base);
        // The cursor lands just past the matched text via `cursor_request`.
    }

    fn handle_event(&mut self, ev: &mut Event, ctx: &mut Context) {
        // Publish the scrollbar range/steps on the first event we see (the
        // earliest point we hold a Context and the sibling bar id resolves).
        self.ensure_inited(ctx);

        // Info / About commands (from the buttons, broadcast) — open the
        // matching modal. The list is the natural handler for Info because it
        // owns the focused entry. (`TDirBox.HandleEvent` did the same via the
        // ofPostProcess command path.)
        if let Event::Broadcast { command, .. } = *ev {
            if command == CMD_INFO {
                self.open_info(ctx);
                ev.clear();
                return;
            } else if command == CMD_ABOUT {
                ctx.request_message_box(
                    ABOUT_TEXT.to_string(),
                    MessageBoxKind::Information,
                    MessageBoxButtons::ok(),
                    None,
                    None,
                );
                ev.clear();
                return;
            }
        }

        // Mouse: a double-click opens the Info box for the clicked entry.
        if let Event::MouseDown(me) = *ev
            && me.flags.double_click
        {
            let item = me.position.y + self.lv.top_item;
            if item >= 0 && item < self.lv.range {
                if item != self.lv.focused {
                    self.search.clear();
                    list_viewer::focus_item(self, item, ctx);
                }
                self.open_info(ctx);
                ev.clear();
                self.sync_mode();
                return;
            }
        }

        // Only run the search state machine when we are the focused leaf.
        if self.is_focused()
            && let Event::KeyDown(ke) = *ev
        {
            match ke.key {
                // Printable character: extend the search and jump to the next match.
                Key::Char(c) if !ke.modifiers.ctrl && !ke.modifiers.alt => {
                    let from = if self.search.is_empty() {
                        0
                    } else {
                        self.lv.focused
                    };
                    let mut probe = self.search.clone();
                    probe.push(c);
                    let found = self.find_next(from, &probe);
                    if no_case_pos(&probe, &self.get_text(found)) != 0 {
                        self.search = probe;
                        if found != self.lv.focused {
                            list_viewer::focus_item(self, found, ctx);
                        }
                    }
                    // No match: keep the old search and stay put (a gentle no-op,
                    // tidier than the original's error message box per keystroke).
                    ev.clear();
                    self.sync_mode();
                    return;
                }
                // Backspace: shorten the search, re-find from the top.
                Key::Backspace => {
                    if !self.search.is_empty() {
                        self.search.pop();
                        if !self.search.is_empty() {
                            let found = self.find_next(0, &self.search);
                            if found != self.lv.focused {
                                list_viewer::focus_item(self, found, ctx);
                            }
                        }
                    }
                    ev.clear();
                    self.sync_mode();
                    return;
                }
                // Enter: open the Info box for the focused entry.
                Key::Enter => {
                    self.open_info(ctx);
                    ev.clear();
                    return;
                }
                // Up/Down while searching: jump to the previous/next match.
                Key::Up if !self.search.is_empty() && self.lv.focused > 0 => {
                    let found = self.find_prev(self.lv.focused, &self.search.clone());
                    list_viewer::focus_item(self, found, ctx);
                    ev.clear();
                    self.sync_mode();
                    return;
                }
                Key::Down if !self.search.is_empty() && self.lv.focused < self.lv.range - 1 => {
                    let found = self.find_next(self.lv.focused + 1, &self.search.clone());
                    list_viewer::focus_item(self, found, ctx);
                    ev.clear();
                    self.sync_mode();
                    return;
                }
                // Esc, or any other navigation: leave search / browse mode.
                _ => {
                    if !self.search.is_empty() {
                        self.search.clear();
                        self.sync_mode();
                        if matches!(ke.key, Key::Esc) {
                            ev.clear();
                            return;
                        }
                    }
                }
            }
        }

        // Browse mode (or unconsumed keys): the base list nav + scrollbar sync.
        list_viewer::handle_event(self, ev, ctx);
    }

    fn set_state(&mut self, flag: StateFlag, enable: bool, ctx: &mut Context) {
        list_viewer::set_state(self, flag, enable, ctx);
    }

    fn cursor_request(&self) -> Option<Point> {
        let base = list_viewer::focused_cursor(self)?;
        if self.search.is_empty() {
            return Some(base);
        }
        // In search mode, place the cursor just past the matched substring.
        let line = self.get_text(self.lv.focused);
        let pos = no_case_pos(&self.search, &line);
        if pos == 0 {
            return Some(base);
        }
        let end = (pos - 1) + self.search.chars().count();
        Some(Point::new(base.x - 1 + end as i32, base.y))
    }

    fn apply_scroll_sync(&mut self, h: Option<i32>, v: Option<i32>, ctx: &mut Context) {
        list_viewer::apply_scroll(self, h, v, ctx);
    }

    fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> {
        Some(self)
    }
}

// ---------------------------------------------------------------------------
// DataWindow — port of `TDataWin : TDialog`, the full-desktop catalog window.
//
// Holds the header label, the search list, its scrollbar, and the three
// buttons. It forwards the list's help context (browse/search) up to the status
// line via `get_help_ctx`, and turns a `CMD_INFO` broadcast into an Info box.
//
// The original TCV.PAS set `Window^.Flags := $00; GrowMode := $00` to make
// this a fixed, icon-less panel (no close/zoom icons, not movable). This is
// now faithful via `with_flags(WindowFlags::default())` for the flags.
//
// DEVIATION: The original GrowMode := $00 was faithful to DOS's fixed 80×25
// screen where resizing was impossible. We deliberately deviate here by
// assigning grow-modes so the catalog and all its children follow modern
// terminal resizes: the window tracks both hi edges, children scale their
// respective edges to fill the new geometry.
// ---------------------------------------------------------------------------

struct DataWindow {
    dialog: Dialog,
}

impl DataWindow {
    fn new(bounds: Rect) -> Self {
        let mut dialog = Dialog::new(bounds, Some("Tobis Catalog Vision Version 2.2".to_string()))
            .with_flags(WindowFlags::default()) // TCV: Flags := $00 (fixed, no icons)
            // DEVIATION from TCV.PAS GrowMode := $00: track both hi edges so
            // the window stretches to fill the terminal on resize.
            .with_grow_mode(GrowMode {
                hi_x: true,
                hi_y: true,
                ..Default::default()
            });

        let inner = bounds; // dialog-local coords start at (0,0)
        let w = inner.b.x - inner.a.x;
        let h = inner.b.y - inner.a.y;

        // Buttons along the bottom row, mirroring the original layout.
        let btn_y = h - 3;
        let mut btn_info = Button::new(
            Rect::new(w - 45, btn_y, w - 33, btn_y + 2),
            "~I~nfo",
            CMD_INFO,
            ButtonFlags {
                broadcast: true,
                ..ButtonFlags::new()
            },
        );
        btn_info.state_mut().grow_mode = GrowMode::grow_all();
        dialog.insert_child(Box::new(btn_info));

        let mut btn_about = Button::new(
            Rect::new(w - 30, btn_y, w - 18, btn_y + 2),
            "~A~bout",
            CMD_ABOUT,
            ButtonFlags {
                broadcast: true,
                ..ButtonFlags::new()
            },
        );
        btn_about.state_mut().grow_mode = GrowMode::grow_all();
        dialog.insert_child(Box::new(btn_about));

        let mut btn_exit = Button::new(
            Rect::new(w - 15, btn_y, w - 3, btn_y + 2),
            "E~x~it",
            Command::QUIT,
            ButtonFlags::new(),
        );
        btn_exit.state_mut().grow_mode = GrowMode::grow_all();
        dialog.insert_child(Box::new(btn_exit));

        // The scrollbar lives on the right edge of the list area.
        let list_rect = Rect::new(2, 2, w - 2, h - 4);
        let mut sb = ScrollBar::new(Rect::new(w - 2, 2, w - 1, h - 4));
        sb.state_mut().grow_mode = GrowMode {
            lo_x: true,
            hi_x: true,
            hi_y: true,
            ..Default::default()
        };
        let sb_id = dialog.insert_child(Box::new(sb));

        let mut list = DirBox::new(list_rect, None, Some(sb_id));
        list.state_mut().grow_mode = GrowMode {
            hi_x: true,
            hi_y: true,
            ..Default::default()
        };
        let list_id = dialog.insert_child(Box::new(list));

        // Header label, linked to the list (Alt-D focuses it).
        let mut label = Label::new(
            Rect::new(3, 1, w - 2, 2),
            "~D~isk          File Name      Comment",
            Some(list_id),
        );
        label.state_mut().grow_mode = GrowMode {
            hi_x: true,
            ..Default::default()
        };
        dialog.insert_child(Box::new(label));

        DataWindow { dialog }
    }
}

#[delegate(to = dialog)]
impl View for DataWindow {
    fn as_any_mut(&mut self) -> Option<&mut dyn core::any::Any> {
        Some(self)
    }
    // handle_event is forwarded by #[delegate(to = dialog)].
    // Axis C.1 (Group::get_help_ctx bubble + program idle-path) means the
    // focused DirBox's help_ctx now reaches the status line automatically —
    // no manual cache into the dialog's state is needed.
}

// ---------------------------------------------------------------------------
// TcvApp — port of `TTCV : TApplication`.
// ---------------------------------------------------------------------------

struct TcvApp {
    program: Program,
}

impl TcvApp {
    fn new(backend: Box<dyn Backend>) -> Self {
        let mut program = Program::new(
            backend,
            Box::new(SystemClock::new()),
            Theme::classic_blue(),
            Self::init_desktop,
            Self::init_status_line,
            |r| {
                // No menu bar (the original's InitMenuBar is empty); pin a
                // zero-height bar so the desktop fills from the top.
                let mut r = r;
                r.b.y = r.a.y;
                let _ = r;
                None
            },
        );
        let r = program.desktop_rect();
        program.desktop_insert(Box::new(DataWindow::new(r)));
        TcvApp { program }
    }

    fn init_desktop(r: Rect) -> Option<Box<dyn View>> {
        let mut r = r;
        r.b.y -= 1; // above the status line
        Some(Box::new(Desktop::new(r, |br| {
            Some(Desktop::init_background(br))
        })))
    }

    /// `TTCVStatLine` — context-sensitive hints keyed on the browse/search mode.
    fn init_status_line(r: Rect) -> Option<Box<dyn View>> {
        let mut r = r;
        r.a.y = r.b.y - 1;
        let defs = StatusDef::list()
            .def_all(|d| d.item("~Alt-X~ Exit", alt('x'), Command::QUIT))
            .build();
        let line = StatusLine::new(r, defs).with_hint(|ctx| {
            if ctx == HC_SEARCH_MODE {
                Some(
                    "SEARCH MODE: [UP],[DOWN] for Next Match; Continue typing; [ESC] to Browse Mode"
                        .to_string(),
                )
            } else {
                Some(
                    "BROWSE MODE: Use [UP],[DOWN] to Browse or Enter a Word you are looking for."
                        .to_string(),
                )
            }
        });
        Some(Box::new(line))
    }

    fn run(&mut self) {
        // The Info / About modals and Exit are all handled within the view tree
        // (the buttons broadcast their commands; the list opens the boxes via
        // the async-modal-from-a-view seam, and Exit is the standard cmQuit), so
        // the application-level command hook is empty.
        self.program.run_app(|_prog, _cmd| {});
    }
}

fn main() -> io::Result<()> {
    let mut app = TcvApp::new(Box::new(CrosstermBackend::new()?));
    app.run();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tvision_rs::HeadlessBackend;
    use tvision_rs::KeyModifiers;

    /// Smoke test: the whole app constructs on a headless backend, renders a
    /// frame, accepts a search keystroke, and pumps several frames without
    /// panicking. Also checks the dir-line / search helpers directly.
    #[test]
    fn constructs_and_renders_without_panic() {
        let (backend, screen) = HeadlessBackend::new(80, 25);
        let mut app = TcvApp::new(Box::new(backend));

        // One frame: the catalog window draws.
        app.program.pump_once();
        let frame = screen.snapshot();
        assert!(
            frame.contains("Tobis Catalog Vision"),
            "title should render; got:\n{frame}"
        );

        // Type a search and pump it through — must not panic and should jump to
        // a matching entry.
        for c in "doom".chars() {
            screen.push_key(Key::Char(c), KeyModifiers::default());
            app.program.pump_once();
        }
        let frame = screen.snapshot();
        assert!(
            frame.to_lowercase().contains("doom"),
            "search should surface the DOOM entry; got:\n{frame}"
        );

        // Backspace + Esc must also pump cleanly.
        screen.push_key(Key::Backspace, KeyModifiers::default());
        app.program.pump_once();
        screen.push_key(Key::Esc, KeyModifiers::default());
        app.program.pump_once();
    }

    /// Regression guard for the dead scrollbar: `ListViewerState::new` leaves the
    /// sibling vertical bar at its default `max = 0`, so without the post-insert
    /// `set_range` + `update_steps` publish (`DirBox::ensure_inited`) the bar is a
    /// zero-range column (all `▓`, the `sb_page_no_range` glyph) whose thumb never
    /// moves. After the fix the bar shows a `■` thumb that walks down the column as
    /// the focus moves. We focus the list, page down, and assert the thumb glyph
    /// appears and is not pinned at the top arrow row.
    #[test]
    fn scrollbar_thumb_moves_with_focus() {
        use tvision_rs::{MouseButtons, MouseEvent};
        let (backend, screen) = HeadlessBackend::new(80, 25);
        let mut app = TcvApp::new(Box::new(backend));
        app.program.pump_once();

        // Glyphs: thumb '■' (U+25A0), no-range fill '▓' (U+2593).
        let thumb = '\u{25A0}';
        let no_range = '\u{2593}';

        // Focus the list (and run its lazy scrollbar init) by clicking a row,
        // releasing the mouse-track capture so subsequent keys reach the DirBox.
        screen.push_event(Event::MouseDown(MouseEvent {
            position: Point::new(10, 5),
            buttons: MouseButtons {
                left: true,
                ..Default::default()
            },
            ..Default::default()
        }));
        app.program.pump_once();
        screen.push_event(Event::MouseUp(MouseEvent {
            position: Point::new(10, 5),
            buttons: MouseButtons::default(),
            ..Default::default()
        }));
        app.program.pump_once();

        // The bar must not be a dead no-range column: a real thumb glyph exists.
        let before = screen.snapshot();
        assert!(
            before.contains(thumb),
            "scrollbar should show a movable thumb (■), not a dead no-range bar; got:\n{before}"
        );
        assert!(
            !before.lines().any(|l| l.matches(no_range).count() > 1),
            "scrollbar must not render as an all-▓ no-range column; got:\n{before}"
        );

        // Find the thumb's row in the rightmost-ish columns before scrolling.
        let row_of_thumb =
            |frame: &str| -> Option<usize> { frame.lines().position(|l| l.contains(thumb)) };
        let top_row = row_of_thumb(&before);

        // Drive the focus to the end of the catalog; the thumb must
        // travel downward.
        for _ in 0..CATALOG.len() {
            screen.push_key(Key::Down, KeyModifiers::default());
            app.program.pump_once();
        }
        let after = screen.snapshot();
        let bottom_row = row_of_thumb(&after);
        assert!(
            after.contains(thumb),
            "thumb should still render after scrolling to the end; got:\n{after}"
        );
        assert!(
            bottom_row > top_row,
            "scrollbar thumb should move DOWN as focus moves to the catalog end \
             (top_row={top_row:?}, bottom_row={bottom_row:?});\nbefore:\n{before}\nafter:\n{after}"
        );
    }

    /// Regression guard for the search-overlay column bug: `put_str` returns the
    /// *width* it drew (not the next absolute column), so the highlight overlay
    /// must accumulate `x` (`x += put_str(...)`). When it used `x = put_str(...)`
    /// the focused row was mangled (the `rest` span landed ~14 columns left). We
    /// focus the list, type a search, and assert the matched row still renders
    /// its full, contiguous text.
    #[test]
    fn search_does_not_corrupt_focused_row() {
        use tvision_rs::{MouseButtons, MouseEvent};
        let (backend, screen) = HeadlessBackend::new(80, 25);
        let mut app = TcvApp::new(Box::new(backend));
        app.program.pump_once();

        // The WOLF3D row, rendered verbatim (file padded to 15, then the comment).
        let intact = "WOLF3D.ZIP     Wolfenstein 3D shareware episode 1";
        assert!(
            screen.snapshot().contains(intact),
            "row should render intact in browse mode"
        );

        // Click a list row to focus the list.  Send MouseDown + MouseUp to
        // release the mouse-track capture before typing; while the capture is
        // live (between Down and Up) keyboard events are swallowed by the
        // hold handler.
        screen.push_event(Event::MouseDown(MouseEvent {
            position: Point::new(10, 5),
            buttons: MouseButtons {
                left: true,
                ..Default::default()
            },
            ..Default::default()
        }));
        app.program.pump_once();
        screen.push_event(Event::MouseUp(MouseEvent {
            position: Point::new(10, 5),
            buttons: MouseButtons::default(),
            ..Default::default()
        }));
        app.program.pump_once();

        // After focusing the list the DirBox is in browse mode; the bubble
        // (DirBox.help_ctx → Group::get_help_ctx → status-line idle path)
        // must surface "BROWSE MODE" on the status line.
        {
            let frame = screen.snapshot();
            assert!(
                frame.contains("BROWSE MODE"),
                "status line should show BROWSE MODE after focusing the list; got:\n{frame}"
            );
        }

        // Type a search for "wolf" — now that the hold is released, keys reach
        // the DirBox and activate search mode.
        for c in "wolf".chars() {
            screen.push_key(Key::Char(c), KeyModifiers::default());
            app.program.pump_once();
        }
        // Drain deferred broadcasts (RECEIVED_FOCUS etc.) then get one true
        // idle pump: the status-line idle arm reads group.get_help_ctx() only
        // when out_events is empty.  Each key pump may leave 1-2 broadcasts;
        // 8 extra pumps is conservative.
        for _ in 0..8 {
            app.program.pump_once();
        }

        // Search is active: status line must now show SEARCH MODE.
        {
            let frame = screen.snapshot();
            assert!(
                frame.contains("SEARCH MODE"),
                "status line should show SEARCH MODE while searching; got:\n{frame}"
            );
        }

        // The focused row's text is still contiguous — not shifted/duplicated.
        assert!(
            screen.snapshot().contains(intact),
            "search overlay must not corrupt the focused row; got:\n{}",
            screen.snapshot()
        );
    }

    #[test]
    fn dir_line_and_search_helpers() {
        let e = &CATALOG[0];
        let line = dir_line(e);
        assert!(line.starts_with(' '));
        assert!(line.contains(e.disk) && line.contains(e.file) && line.contains(e.desc));

        // Case-insensitive substring search (NoCasePos), 1-based.
        assert_eq!(no_case_pos("doom", "  DOOM1_0.ZIP"), 3);
        assert_eq!(no_case_pos("zzz", "abc"), 0);
        assert_eq!(no_case_pos("", "abc"), 0);
    }

    /// Builder test (content): `build_info_dialog` produces a dialog that renders
    /// the entry's six fields and an OK button.  Rendered standalone via
    /// `Renderer` on a `HeadlessBackend` — no app pump needed.
    #[test]
    fn build_info_dialog_renders_entry_fields_and_ok_button() {
        use tvision_rs::{Buffer, DrawCtx, Renderer};

        let entry = &CATALOG[0]; // DOOM1_0.ZIP on GAMES01
        let mut dialog = build_info_dialog(entry);

        let (backend, screen) = HeadlessBackend::new(52, 10);
        let mut r = Renderer::new(Box::new(backend));
        let theme = Theme::classic_blue();
        r.render(|buf: &mut Buffer| {
            let bounds = dialog.state().get_bounds();
            let mut dc = DrawCtx::new(buf, &theme, bounds, bounds.a);
            dialog.draw(&mut dc);
        });

        let frame = screen.snapshot();
        assert!(
            frame.contains(entry.disk),
            "frame should contain disk label '{}'; got:\n{frame}",
            entry.disk
        );
        assert!(
            frame.contains(entry.file),
            "frame should contain file name '{}'; got:\n{frame}",
            entry.file
        );
        assert!(
            frame.contains(entry.desc),
            "frame should contain description '{}'; got:\n{frame}",
            entry.desc
        );
        assert!(
            frame.contains("OK"),
            "frame should contain 'OK' button; got:\n{frame}"
        );
        assert!(
            frame.contains("Information"),
            "frame should contain the dialog title 'Information'; got:\n{frame}"
        );
    }

    /// Extended builder test: `build_info_dialog` for the entry with the LONGEST
    /// description renders all six labelled fields (disk, file, date, size,
    /// description, scan date) without clipping.  This is the regression guard
    /// for Fix 1 — the dialog must be wide enough that the "Description:" line
    /// does NOT wrap and push "Scan Date" off the bottom of the text box.
    ///
    /// Entry used: COSMO1.ZIP on GAMES05 — "Cosmo's Cosmic Adventure episode 1
    /// shareware" (57-char "Description: ..." line, the longest in the catalog).
    ///
    /// The test intentionally renders standalone on a [`HeadlessBackend`] sized
    /// to the dialog's own dimensions (no app pump needed), exactly like the
    /// existing `build_info_dialog_renders_entry_fields_and_ok_button` test.
    #[test]
    fn build_info_dialog_long_desc_renders_all_six_fields() {
        use tvision_rs::{Buffer, DrawCtx, Renderer};

        // CATALOG[40] = COSMO1.ZIP — the longest "Description: ..." line (57 chars).
        let entry = &CATALOG[40];
        assert_eq!(
            entry.file, "COSMO1.ZIP",
            "index 40 must be the COSMO1 entry"
        );

        let mut dialog = build_info_dialog(entry);
        let bounds = dialog.state().get_bounds();
        let w = bounds.b.x as u16;
        let h = bounds.b.y as u16;

        let (backend, screen) = HeadlessBackend::new(w, h);
        let mut r = Renderer::new(Box::new(backend));
        let theme = Theme::classic_blue();
        r.render(|buf: &mut Buffer| {
            let mut dc = DrawCtx::new(buf, &theme, bounds, bounds.a);
            dialog.draw(&mut dc);
        });

        let frame = screen.snapshot();

        // All six labelled fields must appear in the rendered frame.
        assert!(
            frame.contains(entry.disk),
            "frame should contain disk label '{}'; got:\n{frame}",
            entry.disk
        );
        assert!(
            frame.contains(entry.file),
            "frame should contain file name '{}'; got:\n{frame}",
            entry.file
        );
        assert!(
            frame.contains(entry.date),
            "frame should contain file date '{}'; got:\n{frame}",
            entry.date
        );
        assert!(
            frame.contains("449024"),
            "frame should contain size '449024'; got:\n{frame}"
        );
        assert!(
            frame.contains(entry.desc),
            "frame should contain description '{}'; got:\n{frame}",
            entry.desc
        );
        // Scan date is the sixth field — this assertion fails if the description
        // wraps and pushes scan date off the bottom of the text box.
        assert!(
            frame.contains(entry.scan),
            "frame should contain scan date '{}' (Fix 1 guard: clipped if dialog too narrow); got:\n{frame}",
            entry.scan
        );
        assert!(
            frame.contains("OK"),
            "frame should contain 'OK' button; got:\n{frame}"
        );
        assert!(
            frame.contains("Information"),
            "frame should contain the dialog title 'Information'; got:\n{frame}"
        );
    }

    /// Integration test (wiring): pressing Enter on a focused entry triggers
    /// `CMD_INFO` → `open_info` → `request_exec_view`.  We pre-queue the event
    /// sequence so `TcvApp::run()` drives the full pump-and-drive loop without
    /// hanging:
    ///   1. Enter  → DirBox queues `Deferred::OpenModal` (the Info dialog).
    ///   2. `Command::CANCEL`  → the modal's inner loop picks it up and closes
    ///      the Info dialog (the OK button emits `Command::CANCEL`).
    ///   3. `Command::QUIT`  → the outer app loop exits.
    /// The headless backend's `poll_event` queue is shared with the modal's
    /// inner loop, so the pre-queued events are consumed in the right order.
    #[test]
    fn enter_on_focused_entry_opens_and_closes_info_dialog_modal() {
        use tvision_rs::{MouseButtons, MouseEvent};

        let (backend, screen) = HeadlessBackend::new(80, 25);
        let mut app = TcvApp::new(Box::new(backend));

        // Initial render so the view tree is built.
        app.program.pump_once();

        // Click the list to focus the DirBox (so keyboard events reach it).
        // MouseDown + MouseUp to release capture before typing.
        screen.push_event(Event::MouseDown(MouseEvent {
            position: Point::new(10, 5),
            buttons: MouseButtons {
                left: true,
                ..Default::default()
            },
            ..Default::default()
        }));
        app.program.pump_once();
        screen.push_event(Event::MouseUp(MouseEvent {
            position: Point::new(10, 5),
            buttons: MouseButtons::default(),
            ..Default::default()
        }));
        app.program.pump_once();

        // Pre-queue the full sequence for TcvApp::run():
        //   Enter      — DirBox.handle_event calls open_info → request_exec_view
        //                → queues Deferred::OpenModal.
        //   CANCEL     — the Info dialog's modal loop picks this up; the OK
        //                button emits Command::CANCEL, so any Command::CANCEL
        //                closes the modal (it is the end-modal command).
        //   QUIT       — terminates the outer run_app loop so the test returns.
        screen.push_key(Key::Enter, KeyModifiers::default());
        screen.push_event(Event::Command(Command::CANCEL));
        screen.push_event(Event::Command(Command::QUIT));

        // Run the full app loop.  The pre-queued events drive it to completion
        // without blocking (headless backend never blocks on poll_event).
        app.run();

        // After run() returns the catalog window should still be renderable
        // (the desktop is intact after the modal closed cleanly).
        let frame = screen.snapshot();
        assert!(
            frame.contains("Tobis Catalog Vision"),
            "catalog window should still be visible after modal closes; got:\n{frame}"
        );
    }

    /// End-to-end resize test: after `HeadlessHandle::resize` + `pump_once` the
    /// catalog window and all its children grow to fill the new terminal size.
    ///
    /// Verifies:
    /// - Before resize: right frame border (`║`) at column 79 (last column of
    ///   80-wide terminal), NOT at column 99.
    /// - After resize to 100×30: right frame border at column 99, and the old
    ///   right-border column (79) is no longer `║` (it is interior content now).
    #[test]
    fn window_grows_on_terminal_resize() {
        let (backend, screen) = HeadlessBackend::new(80, 25);
        let mut app = TcvApp::new(Box::new(backend));

        // Initial layout pass.
        app.program.pump_once();

        // Box-drawing right-border glyph used by the Dialog/Window frame.
        let right_border = "\u{2551}"; // ║

        // At 80 wide: the dialog fills the desktop (0..80). The right edge of
        // the frame is at column 79 (0-indexed). Row 5 is well within the
        // interior rows so should be the `║` side border.
        let before_frame = screen.snapshot();
        {
            let buf = screen.buffer();
            let cell_at_79 = buf.get(79, 5).symbol();
            assert_eq!(
                cell_at_79, right_border,
                "before resize: expected '║' at col 79, row 5; got {:?}\nframe:\n{before_frame}",
                cell_at_79
            );
            // Column 99 doesn't exist on the 80-wide screen — we just verify
            // the right border is at 79, not wider.
        }

        // Simulate a terminal resize to 100×30.
        screen.resize(100, 30);
        app.program.pump_once();

        // After resize: the dialog should have grown to fill 100 columns ×
        // 29 desktop rows (status line still takes 1 row). Right border moves
        // to column 99, and column 79 is now interior (not `║`).
        let after_frame = screen.snapshot();
        {
            let buf = screen.buffer();

            let cell_at_99 = buf.get(99, 5).symbol();
            assert_eq!(
                cell_at_99, right_border,
                "after resize: expected '║' at col 99, row 5; got {:?}\nframe:\n{after_frame}",
                cell_at_99
            );

            let cell_at_79 = buf.get(79, 5).symbol();
            assert_ne!(
                cell_at_79, right_border,
                "after resize: col 79 should be interior (not '║'); got {:?}\nframe:\n{after_frame}",
                cell_at_79
            );
        }
    }
}
