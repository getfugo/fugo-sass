//! Runs [sass-spec](https://github.com/sass/sass-spec), the official Sass test suite, against
//! fugo-sass, with the expectations sass-spec holds for dart-sass.
//!
//! A spec is a directory with an `input.scss` or `input.sass` and either an `output.css`
//! (the compiled CSS) or an `error` (compilation fails); `-dart-sass` variants of those files
//! take precedence. Most specs live in HRX archives, each a virtual directory named after the
//! file. `options.yml` files apply to their directory and below: `:todo:` and `:ignore_for:`
//! entries naming dart-sass skip a spec (dart-sass does not pass it either), `:warning_todo:`
//! skips its warnings.
//!
//! Outputs are compared as sass-spec does, after collapsing runs of newlines (and, here, trailing
//! whitespace). An error spec passes when compilation fails: error messages are not compared,
//! but whether the first `Error:` line matches is counted. Warnings are not compared yet.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use fugo_sass_compiler::codemap::SpanLoc;
use fugo_sass_compiler::{Fs, Logger, Options, OutputStyle};

mod compile;
mod hrx;
mod run;
mod suite;

pub use compile::*;
pub use hrx::*;
pub use run::*;
pub use suite::*;

/// The implementation whose expectations and options apply (sass-spec's `--impl`).
pub const IMPL: &str = "dart-sass";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hrx_entries() {
        let hrx = parse_hrx(
            "<===> a/input.scss\na {b: c}\n\n<===> a/output.css\na {\n  b: c;\n}\n\n<===>\n\
             ==========\n<===> d/\n<===> e.txt\n<====> not a boundary\nlast\n",
        )
        .unwrap();
        assert_eq!(
            hrx.files,
            vec![
                ("a/input.scss".to_owned(), "a {b: c}\n".to_owned()),
                ("a/output.css".to_owned(), "a {\n  b: c;\n}\n".to_owned()),
                (
                    "e.txt".to_owned(),
                    "<====> not a boundary\nlast\n".to_owned()
                ),
            ]
        );
        assert_eq!(hrx.dirs, vec!["d".to_owned()]);
        assert!(parse_hrx("no boundary").is_err());
        assert_eq!(
            parse_hrx("<===> empty\n<===> x\ny").unwrap().files,
            vec![
                ("empty".to_owned(), String::new()),
                ("x".to_owned(), "y".to_owned())
            ]
        );
    }

    #[test]
    fn options() {
        let o = SpecOptions::parse("---\n:todo:\n- sass/dart-sass#1234\n:precision: 5\n");
        assert_eq!(o.mode(), Mode::Todo);
        let o = SpecOptions::parse(":ignore_for:\n  - libsass\n");
        assert_eq!(o.mode(), Mode::Run);
        let o = o.merge(&SpecOptions::parse(":ignore_for:\n- dart-sass\n"));
        assert_eq!(o.mode(), Mode::Ignore);
    }

    #[test]
    fn normalization() {
        assert_eq!(normalize("a {\n\n\n  b: c;\r\n}\n\n"), "a {\n  b: c;\n}");
        assert_eq!(
            normalize("Error: x\n  /tmp/spec/a-b_c/input.scss 1:1  root stylesheet"),
            "Error: x\n  input.scss 1:1  root stylesheet"
        );
    }
}
