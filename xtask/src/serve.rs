//! `cargo xtask docs --serve`: build once, serve the assembled tree, and
//! rebuild the book on source changes. Minimal by design.

use crate::{build, paths};
use anyhow::{Context, Result};
use notify::{RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;

pub fn run() -> Result<()> {
    build::docs().context("initial build")?;

    let root = paths::book_out();
    let addr = "127.0.0.1:3000";
    let server = tiny_http::Server::http(addr).map_err(|e| anyhow::anyhow!("bind {addr}: {e}"))?;
    eprintln!("serving {} at http://{addr}/", root.display());

    // Watch sources; rebuild the book (not rustdoc) on change for fast loops.
    let (tx, rx) = channel();
    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = tx.send(res);
    })?;
    watcher.watch(&paths::book_root().join("src"), RecursiveMode::Recursive)?;
    watcher.watch(&paths::book_root().join("theme"), RecursiveMode::Recursive)?;

    std::thread::spawn(move || {
        for ev in rx {
            if ev.is_ok() {
                eprintln!("change detected — rebuilding book…");
                if let Err(e) = build::build_book() {
                    eprintln!("rebuild error: {e:#}");
                }
            }
        }
    });

    for request in server.incoming_requests() {
        serve_one(&root, request);
    }
    Ok(())
}

fn serve_one(root: &Path, request: tiny_http::Request) {
    let url = request.url().split('?').next().unwrap_or("/");
    let rel = url.trim_start_matches('/');
    let mut path = root.join(rel);
    if path.is_dir() || rel.is_empty() {
        path = path.join("index.html");
    }
    match std::fs::read(&path) {
        Ok(bytes) => {
            let mime = match path.extension().and_then(|e| e.to_str()) {
                Some("html") => "text/html; charset=utf-8",
                Some("css") => "text/css",
                Some("js") => "application/javascript",
                Some("svg") => "image/svg+xml",
                _ => "application/octet-stream",
            };
            let header =
                tiny_http::Header::from_bytes(&b"Content-Type"[..], mime.as_bytes()).unwrap();
            let _ = request.respond(tiny_http::Response::from_data(bytes).with_header(header));
        }
        Err(_) => {
            let _ = request.respond(tiny_http::Response::from_string("404").with_status_code(404));
        }
    }
}
