# Design note — OS clipboard by default (backlog A6)

> Status: **LANDED**. User directive: rstv copy/paste should hit the real OS
> clipboard out of the box, not just an internal string.

## Baseline (the C++)

`TClipboard` (`tclipbrd.cpp:26-44`) is **native-first, internal-buffer-only-on-
failure**: `setText` tries `THardwareInfo::setClipboardText` and writes
`localText` only when that fails; `requestText` mirrors it for paste. On Unix
the native rung itself is a chain (`unixcon.cpp:50-60`): subprocess tools
(xclip/wl-copy) → OSC 52 emit → internal. The OSC 52 copy
(`termio.cpp:896-917`) is *always emitted* once reached, but reports success
only when a startup capability probe detected full OSC 52 support; the OSC 52
*read* is gated behind those probes (`termio.cpp:314-330`: TERM allowlist,
XTGETTCAP/XTQALLOWED queries answered through the input parser).

## Deviation

`src/backend/clipboard.rs` — a `ClipboardChain` with the same shape, different
rungs:

- **Native** = `arboard` (feature `os-clipboard`, **in `default`**), not
  subprocess tools. Constructed once at backend construction; init failure →
  the chain just runs without the rung (never fails `with_color_depth`).
  `default-features = false` drops arboard's `image` dependency;
  `wayland-data-control` keeps native Wayland support.
- **OSC 52 emit** via crossterm's `osc52` feature (`CopyToClipboard`), queued
  on the backend's normal output handle, fire-and-forget, **only when the
  native rung did not take the text** (magiblot doesn't double-emit; a blind
  OSC sequence on a dumb terminal risks on-screen garbage). We cannot run
  magiblot's capability probes (crossterm owns the input parser), so reaching
  this rung always reports the internal fallback (`false`).
- **Internal buffer**, written only on the non-native path — no stale shadow
  of a live native clipboard (`tclipbrd.cpp:28-34`).
- **No OSC 52 read** — needs the `termio.cpp:314-330` probes plus a reply
  through the input parser we don't own; terminals gate reads behind security
  opt-ins anyway.

Nothing above the backend changed: the `Backend` trait contract
(`set_clipboard -> bool`, false = internal fallback), the
`Deferred::SetClipboard`/`EditorPaste` brokers, and the pump's apply code are
untouched. `HeadlessBackend` deliberately keeps plain internal-string
semantics (it is the D11 test fake); its clipboard string moved into the
shared handle state so tests can assert copies / seed pastes via
`HeadlessHandle::clipboard()` / `set_clipboard()`.

## Platform / SSH matrix

| Situation | Copy lands on | Paste reads from |
|---|---|---|
| Local X11/Wayland/macOS/Windows | OS clipboard (arboard) | OS clipboard |
| Wayland without data-control | OSC 52 → terminal's clipboard (per-op fallback) | internal buffer |
| SSH / no display (arboard init fails) | OSC 52 → the **local** terminal's clipboard | internal buffer (or the terminal's own bracketed paste — backlog C9) |
| `--no-default-features` build | OSC 52 → terminal's clipboard | internal buffer |
| Headless (tests) | shared internal string | shared internal string |

## Risks (accepted)

- **X11 exit loss**: arboard serves the selection from a thread inside the
  app; without a clipboard manager the contents vanish at exit. Magiblot's
  xclip subprocess rung (which outlives the app) would be a future
  `NativeClipboard` impl closing the gap.
- **Blind OSC 52**: on terminals that neither support OSC 52 nor swallow
  unknown OSC sequences, the emit could leak garbage. Magiblot redraws the
  screen after the attempt (`unixcon.cpp:55-58`); rstv's whole-tree
  redraw+diff makes the next pump self-healing, but a byte could flash.
  Mitigated by emitting only on the non-native path.
- **Size limits**: some terminals cap OSC 52 payloads (commonly ~100 KB
  base64). Oversized copies silently truncate or drop at the terminal; the
  internal mirror still holds the full text for in-app paste.

## Integration

Copy: editor `clipCopy` → `Deferred::SetClipboard` → pump applies →
`Backend::set_clipboard` → chain. Paste: `clipPaste` → `Deferred::EditorPaste`
→ pump reads `Backend::get_clipboard` → chain → `insert_text`. Verified by
chain unit tests (stub native rung + `Vec<u8>` OSC sink,
`backend/clipboard.rs`) and pump-level tests
(`app/program.rs::tests::clipboard_a6`). Bracketed paste stays off until the
`Event::Paste` arm produces an event (backlog C9) — enabling it earlier would
make terminal-paste a no-op instead of today's plain-keystroke arrival.
