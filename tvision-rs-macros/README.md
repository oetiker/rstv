# tvision-rs-macros

Procedural macros for [`tvision-rs`](https://crates.io/crates/tvision-rs), an
idiomatic Rust port of Turbo Vision (magiblot/tvision).

This is an **internal companion crate**: it has no standalone use and its API is
driven by the needs of `tvision-rs`. You normally depend on `tvision-rs`, which
re-exports what it needs from here.

## `#[delegate]`

The crate provides one attribute macro, `#[delegate(to = <field>, skip(...))]`,
the boilerplate half of the port's **embed-and-delegate** pattern (the stand-in
for C++ implementation inheritance). On an `impl Trait for Type` block it injects
forwarders for every trait method the block does not already provide, each
forwarding to `self.<field>.<method>(<args>)`:

```rust,ignore
use tvision_rs::delegate;

#[delegate(to = inner)]
impl View for MyWidget {
    // Write only the methods that differ; everything else forwards to `inner`.
    fn handle_event(&mut self, event: &Event, ctx: &mut Context) { /* ... */ }
}
```

- `to = <field>` (required): the field to forward un-provided methods to.
- `skip(method1, method2, ...)` (optional): methods to leave at the trait's own
  default instead of forwarding.

Only the `View` trait is currently supported.

## License

MIT — see the [repository](https://github.com/oetiker/tvision-rs).
