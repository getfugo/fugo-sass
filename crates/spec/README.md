# fugo-sass-spec

Runs [sass-spec](https://github.com/sass/sass-spec), the official Sass test suite, against
fugo-sass, with the expectations sass-spec holds for `dart-sass`. It needs only Rust: the specs'
HRX archives are read in memory and compiled with `fugo-sass-compiler` on all cores.

```bash
git submodule update --init sass-spec   # once
cargo spec                              # every spec: a table per top-level directory
cargo spec core_functions/color --depth 3 --show 5
cargo spec --baseline crates/spec/failures.txt
```

`cargo spec` is an alias (`.cargo/config.toml`) for `cargo run --profile spec -p fugo-sass-spec --`.
The `spec` profile is the release profile with panics unwinding, so a spec that panics fails on
its own instead of ending the run.

## What is compared

sass-spec runs the compiler in each spec's directory as `sass --load-path=<spec root>
--no-unicode input.scss`; this runner does the same through the library. A spec passes when:

- it expects CSS (`output.css`, or `output-dart-sass.css`) and the CSS matches after collapsing
  runs of newlines and trimming trailing whitespace, as sass-spec normalizes it;
- it expects an error (`error`, or `error-dart-sass`) and compilation fails. The message is not
  compared, but the table counts how many first `Error:` lines match.

Warnings are not compared yet. Specs whose `options.yml` lists dart-sass under `:todo:` (dart-sass
does not pass them either) or `:ignore_for:` are skipped unless `--run-todo` is given. A spec
still compiling after `--timeout` seconds (default 20) fails, and the others go on.

## The baseline

`failures.txt` lists every spec fugo-sass fails at the pinned sass-spec commit (the `sass-spec`
submodule, at the specs of dart-sass 1.105.1). CI runs `cargo spec --baseline
crates/spec/failures.txt`, which fails on any difference: a spec that starts failing is a
regression, and a spec that starts passing must leave the list, so the count only goes down.
After a change that fixes specs, update the list:

```bash
cargo spec --write-baseline crates/spec/failures.txt
```

## Options

```
--spec DIR              sass-spec's spec directory (default: the submodule's)
--baseline FILE         fail if the failing specs differ from FILE's list
--write-baseline FILE   write the failing specs to FILE
--run-todo              also run the specs dart-sass does not pass (:todo:)
--depth N               group the table by N name segments (default 1)
--show N                print N failures with their expected and actual output
--list                  print every failing spec and why
--timeout SECS          fail a spec still compiling after SECS (default 20)
PREFIX...               run only the specs whose names start with a PREFIX
```
