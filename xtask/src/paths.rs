//! Filesystem locations the doc build needs, resolved relative to the
//! workspace root and honoring `CARGO_TARGET_DIR`.

// Functions are stubs consumed by later tasks (build, screens, serve).
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// Workspace root = the directory two levels up from this file's crate
/// (`xtask/`), i.e. the repo root. Resolved from `CARGO_MANIFEST_DIR`.
pub fn workspace_root() -> PathBuf {
    let xtask_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    xtask_dir
        .parent()
        .expect("xtask/ has a parent")
        .to_path_buf()
}

/// The mdBook root: `docs/book`.
pub fn book_root() -> PathBuf {
    workspace_root().join("docs").join("book")
}

/// Built book output: `docs/book/book` (mdBook `build-dir` default).
pub fn book_out() -> PathBuf {
    book_root().join("book")
}

/// Cargo target dir: `$CARGO_TARGET_DIR` if set, else `<root>/target`.
pub fn target_dir() -> PathBuf {
    match std::env::var_os("CARGO_TARGET_DIR") {
        Some(v) => PathBuf::from(v),
        None => workspace_root().join("target"),
    }
}

/// rustdoc output: `<target>/doc`.
pub fn rustdoc_out() -> PathBuf {
    target_dir().join("doc")
}

/// Where generated screenshots are written: `docs/book/src/screens`.
pub fn screens_dir() -> PathBuf {
    book_root().join("src").join("screens")
}
