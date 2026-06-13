//! Run the guide's doctests via the mdBook library API.
//!
//! Compiles every non-`ignore` ```rust block in the mdBook guide
//! (`docs/book/`) as a doctest, with the freshly-built `tvision` rlib (and its
//! dependency rlibs) on the linker search path.

use crate::paths;
use anyhow::{Context, Result};
use std::process::Command;

/// `cargo xtask test`: build the `tvision` lib, then run the guide's doctests
/// against the produced rlibs.
pub fn run() -> Result<()> {
    // 1. Build the `tvision` lib so its rlib and dependency rlibs exist on disk.
    //    `-j2` respects the shared-machine core cap.
    let status = Command::new("cargo")
        .args(["build", "--package", "tvision", "--lib", "-j2"])
        .current_dir(paths::workspace_root())
        .status()
        .context("spawn cargo build -p tvision")?;
    anyhow::ensure!(status.success(), "cargo build -p tvision failed");

    // 2. The deps dir holds the rlibs the doctests link against.
    let deps_dir = paths::target_dir().join("debug").join("deps");
    let deps_dir_str = deps_dir.to_str().context("deps dir path is not UTF-8")?;

    // 3. Load the book and run its doctests, pointing rustdoc at the deps dir.
    let mut book =
        mdbook::MDBook::load(paths::book_root()).map_err(|e| anyhow::anyhow!("load book: {e}"))?;
    book.test(vec!["-L", deps_dir_str])
        .map_err(|e| anyhow::anyhow!("mdbook test: {e}"))?;

    eprintln!("OK: guide doctests passed");
    Ok(())
}
