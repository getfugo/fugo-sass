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
perceptible differences from `dart-sass`. Every commit is tested against Bootstrap v5.0.2,
whose output must match `dart-sass`'s byte for byte.

fugo-sass currently targets `dart-sass` version `1.54.3`, as grass 0.13.4 did. Work is under
way to follow the current `dart-sass` release (1.105.1).

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

Having said that, to run the official test suite,

```bash
# This script expects node >=v14.14.0. Check version with `node --version`
git clone https://github.com/getfugo/fugo-sass --recursive
cd fugo-sass && cargo b --release
cd sass-spec && npm install
npm run sass-spec -- --impl=dart-sass --command '../target/release/fugo-sass'
```

The spec runner does not work on Windows.

## Versioning

The minimum supported rust version (MSRV) is `1.70.0`. An increase to the MSRV will correspond
with a minor version bump. The current MSRV is not a hard minimum, but future bugfix versions are
not guaranteed to work on versions prior to this.

An increase to the targeted `dart-sass` version will correspond to either a minor or bugfix
version bump, depending on the changes.

## Licence

MIT ([LICENSE](LICENSE)): Copyright (c) 2020 Connor Skees, and the fugo-sass authors for the
changes since the fork.
