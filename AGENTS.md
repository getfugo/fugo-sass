# AGENTS.md

Guidance for coding agents (and people) working on fugo-sass, a Sass compiler in Rust: a fork of
grass that targets `dart-sass` 1.105.1. [CLAUDE.md](CLAUDE.md) points here.

## Files of at most 500 lines

No file of code may be longer than 500 lines: Rust sources and tests (`*.rs`) and scripts
(`*.sh`). CI fails if one is. Data and generated files (`crates/spec/failures.txt`,
`crates/spec/bootstrap/*.diff`, `Cargo.lock`) and the `sass-spec` submodule don't count.

Check with:

```sh
git ls-files '*.rs' '*.sh' | xargs wc -l | awk '$2 != "total" && $1 > 500'
```

When a file would grow past 500 lines, split it by responsibility before adding to it:

- Methods of a type: move a group of them to a child module, in its own `impl` block
  (`evaluate/visitor.rs` and `evaluate/visitor/*.rs`). A child module sees its parent's private
  items through `use super::*;`; methods it moves that siblings call become `pub(super)`.
- Default methods of a trait (`StylesheetParser`, `BaseParser`): keep in the trait the methods
  that an implementor overrides or must provide, and move the others to an extension trait in a
  child module, with a blanket impl (`parse/stylesheet/*.rs`):

  ```rust
  pub(crate) trait SupportsParser<'a>: StylesheetParser<'a> {
      // default methods
  }

  impl<'a, T: StylesheetParser<'a>> SupportsParser<'a> for T {}
  ```

  Callers import the extension traits they use.
- Tests: a test file becomes a directory, `crates/lib/tests/<name>/main.rs` (its imports, and
  `#[path = "../macros.rs"] mod macros;`) with the tests in `part1.rs`, `part2.rs`, ...

Name a new module after what it holds, give it a `//!` comment, and keep each file well under the
limit, so the next change doesn't need a split.

## Toolchain

- No `unsafe` code: every crate root has `#![forbid(unsafe_code)]`.
- The root `Cargo.toml` holds what the crates share, in `[workspace.package]` (edition 2024,
  `rust-version` 1.99, the version) and `[workspace.dependencies]` (every dependency's version);
  a crate inherits them with `<key>.workspace = true`. Change versions there, not in a crate.
- Rust 1.99, the latest stable, is both the minimum the crates support and what CI tests, lints,
  formats and builds the WebAssembly with (`RUST_TOOLCHAIN` in `.github/workflows/tests.yml` and
  `build_wasm.yml`: change both with `rust-version`). `rustfmt.toml` only sets the edition, for
  rustfmt run on its own (`cargo fmt` reads Cargo.toml's): keep the two equal.
- Before pushing:

  ```sh
  cargo +1.99.0 fmt --all
  cargo +1.99.0 clippy --workspace --all-targets --features=macro,wasm-exports -- -D warnings
  cargo +1.99.0 test --workspace --features=macro
  cargo spec --baseline crates/spec/failures.txt
  ```

## Following dart-sass

- Port behaviour from `dart-sass` 1.105.1's source, and check outputs and error messages with its
  release binary. Comments may name the `dart-sass` function a port follows.
- `cargo spec` runs sass-spec (pinned to `dart-sass` 1.105.1, `crates/spec`). The failing specs
  must be exactly those of `crates/spec/failures.txt`: when a change makes specs pass, update it
  with `cargo spec --write-baseline crates/spec/failures.txt`; a spec that newly fails needs a
  reason.
- CI compares Bootstrap's CSS with `dart-sass`'s (`crates/spec/bootstrap.sh`); the differences
  must stay those of `crates/spec/bootstrap/<version>.diff`.
- The tests in `crates/lib/tests` came from grass and encode `dart-sass` 1.54.3; change an
  expectation only to what `dart-sass` 1.105.1 outputs.
- User-visible changes get an entry in the `Unreleased` section of `CHANGELOG.md`.
