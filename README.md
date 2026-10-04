# fugo-sass

A [Sass](https://sass-lang.com/documentation/) compiler written purely in Rust: a library
with a very small API (two functions) and a binary intended as a replacement for the Sass
command-line executable.

fugo-sass is a fork of [grass](https://github.com/connorskees/grass) by Connor Skees,
maintained by the [fugo](https://github.com/getfugo/fugo) project, which compiles its sites'
Sass with it. grass has had no release since 0.13.4 (August 2024). fugo-sass carries on from
there: 0.14.0 is grass 0.13.4 with the fixes listed in the [CHANGELOG](CHANGELOG.md), and
later releases follow newer versions of `dart-sass`.

The goal is complete feature parity with the `dart-sass` reference implementation. A deviation
from `dart-sass` is a bug, except in the case of error messages and error spans.

[Documentation](https://docs.rs/fugo-sass/)  
[crates.io](https://crates.io/crates/fugo-sass)

## Usage

```rust
fn main() -> Result<(), Box<fugo_sass::Error>> {
    let css = fugo_sass::from_string(
        "a { b { color: &; } }".to_owned(),
        &fugo_sass::Options::default(),
    )?;
    assert_eq!(css, "a b {\n  color: a b;\n}\n");
    Ok(())
}
```

```bash
cargo install fugo-sass
fugo-sass input.scss
```

## Migrating from grass

The API is grass's. Renaming the dependency keeps every `grass::` path working:

```toml
[dependencies]
grass = { package = "fugo-sass", version = "0.14" }
```

The crates were renamed: `grass` is `fugo-sass` (binary `fugo-sass`), `grass_compiler` is
`fugo-sass-compiler`, and `include_sass` is `fugo-sass-macro`.

## Status

One can be quite confident in fugo-sass's output. For the average user there should not be
perceptible differences from `dart-sass`. Every commit compiles Bootstrap 5.0.2 and 5.3.3, and CI
checks the differences from `dart-sass` 1.105.1's CSS against a list that may only shrink
([crates/spec](crates/spec/README.md)).

fugo-sass is moving from `dart-sass` 1.54.3, which grass 0.13.4 targeted, to 1.105.1: the
[CHANGELOG](CHANGELOG.md) says what already follows 1.105.1, and the failing specs
([`crates/spec/failures.txt`](crates/spec/failures.txt)) what does not yet.

There are a number of known missing features and bugs. The rough edges largely include
`@forward` and more complex uses of `@use`. Basic usage of these rules is supported, but more
advanced features such as `@import`ing modules containing `@forward` with prefixes may not
behave as expected. grass tracked its known gaps in
[connorskees/grass#19](https://github.com/connorskees/grass/issues/19).

fugo-sass is not a drop-in replacement for `libsass` and does not intend to be. If you are
upgrading from `libsass`, you may have to modify your stylesheets, though these changes should
not differ from those you would have to make if upgrading to `dart-sass`.

## Performance

grass was benchmarked against `dart-sass` and `sassc` (`libsass`)
[here](https://github.com/connorskees/sass-perf), where it appeared to be ~2x faster than
`dart-sass` and ~1.7x faster than `sassc`.

## Cargo Features

### commandline

(enabled by default): build a binary using clap

### random

(enabled by default): enable the builtin functions [`random([$limit])`](https://sass-lang.com/documentation/modules/math/#random) and [`unique-id()`](https://sass-lang.com/documentation/modules/string/#unique-id)

### macro

(disabled by default): enable the macro `fugo_sass::include!` for compiling Sass to
CSS at compile time

### nightly

(disabled by default): currently only used by `fugo_sass::include!` to enable
[proc_macro::tracked_path](https://github.com/rust-lang/rust/issues/99515)

## Testing

As much as possible this library attempts to follow the same [philosophy for testing as
`rust-analyzer`](https://internals.rust-lang.org/t/experience-report-contributing-to-rust-lang-rust/12012/17).
Namely, all one should have to do is run `cargo test` to run all its tests.
This library maintains a test suite distinct from the `sass-spec`, though it
does include some spec tests verbatim. This has the benefit of allowing tests
to be run without ruby as well as allowing the tests more granular than they
are in the official spec.

Having said that, the official test suite runs with `cargo spec`, which needs only Rust
([crates/spec](crates/spec/README.md)):

```bash
git clone https://github.com/getfugo/fugo-sass --recursive
cd fugo-sass && cargo spec
```

The `sass-spec` submodule is pinned to the specs of `dart-sass` 1.105.1, the release fugo-sass
works towards. There fugo-sass passes 8,612 of the 14,347 specs that apply to `dart-sass` (60%):
most of the 5,735 failures are the color functions, mostly for the CSS Color 4 color spaces
(5,194). CI checks that the failing specs are exactly those of
[`crates/spec/failures.txt`](crates/spec/failures.txt).

## Versioning

The crates use the 2024 edition. The minimum supported Rust version (MSRV) is `1.96.0`, that of
[fugo](https://github.com/getfugo/fugo), and CI builds every crate with it; CI tests and lints with
the latest stable Rust. An increase to the MSRV will correspond with a minor version bump.

An increase to the targeted `dart-sass` version will correspond to either a minor or bugfix
version bump, depending on the changes.

## Licence

MIT ([LICENSE](LICENSE)): Copyright (c) 2020 Connor Skees, and the fugo-sass authors for the
changes since the fork.
