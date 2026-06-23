# Changelog

All notable changes to tvision-rs will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

The `Unreleased` section accumulates changes on `main`; the release workflow
moves it into a dated, versioned section when a release is cut.

## Unreleased

### New

### Changed

- CI: bump `actions/checkout` to v5 and pass the crates.io token via the
  `CARGO_REGISTRY_TOKEN` env var instead of the deprecated `cargo publish
  --token` flag, clearing the Node-20 and cargo deprecation warnings in the
  release workflow.

### Fixed

## 0.1.0 - 2026-06-22

### New

- Initial public release of `tvision-rs` — an idiomatic Rust port of Turbo Vision
  (magiblot/tvision): the `View` trait + `ViewState` composition, the single
  event loop and deferred-effects channel, the core widget set (windows,
  dialogs, menus, buttons, input lines, list/scroll views, validators, color
  picker, …), the `Theme` palette system, and the `crossterm`-backed terminal
  `Backend` with a `HeadlessBackend` for snapshot testing.
### Changed

### Fixed
