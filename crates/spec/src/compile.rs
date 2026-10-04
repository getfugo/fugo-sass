//! Compiling one spec with fugo-sass, and comparing the result with its expectation.

use super::*;

/// `p` with `.` and `..` resolved.
pub(super) fn lexical(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

/// The compiler's file system: the archives' files, then the real ones.
#[derive(Debug)]
pub(super) struct SpecFs<'a>(&'a Suite);

impl Fs for SpecFs<'_> {
    fn is_dir(&self, path: &Path) -> bool {
        self.0.dirs.contains(&lexical(path)) || path.is_dir()
    }

    fn is_file(&self, path: &Path) -> bool {
        self.0.files.contains_key(&lexical(path)) || path.is_file()
    }

    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        match self.0.files.get(&lexical(path)) {
            Some(bytes) => Ok(bytes.clone()),
            None => fs::read(path),
        }
    }

    fn canonicalize(&self, path: &Path) -> io::Result<PathBuf> {
        let p = lexical(path);
        if self.0.files.contains_key(&p) || self.0.dirs.contains(&p) {
            Ok(p)
        } else {
            fs::canonicalize(path)
        }
    }
}

/// The messages of `@warn`, `@debug` and the compiler's own warnings, as dart-sass's command
/// line prints their first lines.
#[derive(Debug, Default)]
pub(super) struct Captured(RefCell<Vec<String>>);

impl Logger for Captured {
    fn debug(&self, location: SpanLoc, message: &str) {
        let file = Path::new(location.file.name())
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let line = location.begin.line + 1;
        self.0
            .borrow_mut()
            .push(format!("{file}:{line} DEBUG: {message}"));
    }

    fn warn(&self, _location: SpanLoc, message: &str) {
        self.0.borrow_mut().push(format!("WARNING: {message}"));
    }
}

/// How a spec that ran failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Failure {
    /// The CSS differs (both normalized).
    Output { expected: String, actual: String },
    /// Compilation failed where it should not have.
    UnexpectedError(String),
    /// Compilation succeeded where it should have failed (the CSS).
    UnexpectedSuccess(String),
    /// The compiler panicked (the message).
    Panic(String),
    /// The compiler ran longer than the timeout.
    Timeout,
    /// The spec has neither `output.css` nor `error`.
    NoExpectation,
}

impl Failure {
    /// A short name of the kind.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Output { .. } => "output",
            Self::UnexpectedError(_) => "unexpected error",
            Self::UnexpectedSuccess(_) => "unexpected success",
            Self::Panic(_) => "panic",
            Self::Timeout => "timeout",
            Self::NoExpectation => "no expectation",
        }
    }
}

/// The result of one spec.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// It passed; for an error spec, whether the first `Error:` line matched too.
    Pass {
        error_message: Option<bool>,
    },
    Fail(Failure),
    Todo,
    Ignored,
}

/// sass-spec's normalization: runs of newlines collapse, paths of the input file are reduced to
/// its name; trailing whitespace is dropped too.
#[must_use]
pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut after_newline = false;
    for c in s.replace("\r\n", "\n").chars() {
        if c == '\n' {
            if !after_newline {
                out.push('\n');
            }
            after_newline = true;
        } else {
            after_newline = false;
            out.push(c);
        }
    }
    reduce_input_paths(out.trim_end())
}

/// `…/dir/input.scss` → `input.scss`, as sass-spec's `[-_/a-zA-Z0-9]+(input\.s[ca]ss)`.
pub(super) fn reduce_input_paths(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find("input.s") {
        let tail = &rest[i..];
        if !(tail.starts_with("input.scss") || tail.starts_with("input.sass")) {
            out.push_str(&rest[..i + 1]);
            rest = &rest[i + 1..];
            continue;
        }
        let head = &rest[..i];
        let keep = head
            .trim_end_matches(|c: char| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/'))
            .len();
        out.push_str(&head[..keep]);
        out.push_str(&tail[..10]);
        rest = &tail[10..];
    }
    out.push_str(rest);
    out
}

/// The first line of `s` that starts with `Error:`.
pub(super) fn error_line(s: &str) -> Option<&str> {
    s.lines()
        .map(str::trim_end)
        .find(|l| l.starts_with("Error:"))
}

/// Compiles a spec's input as sass-spec runs dart-sass: `--load-path=<spec root> --no-unicode`.
pub(super) fn compile(suite: &Suite, case: &Case) -> (Result<String, String>, Vec<String>) {
    let fs = SpecFs(suite);
    let logger = Captured::default();
    let options = Options::default()
        .fs(&fs)
        .logger(&logger)
        .style(OutputStyle::Expanded)
        .load_path(&suite.root)
        .unicode_error_messages(false);
    let result = fugo_sass_compiler::from_path(&case.input, &options).map_err(|e| e.to_string());
    (result, logger.0.into_inner())
}

/// Runs one spec (not `Todo` or `Ignore` ones unless `run_todo`).
#[must_use]
pub fn run_case(suite: &Suite, case: &Case, run_todo: bool) -> Outcome {
    match case.mode {
        Mode::Ignore => return Outcome::Ignored,
        Mode::Todo if !run_todo => return Outcome::Todo,
        _ => {}
    }
    if case.expect == Expect::Missing {
        return Outcome::Fail(Failure::NoExpectation);
    }
    let result = panic::catch_unwind(AssertUnwindSafe(|| compile(suite, case)));
    let (result, _warnings) = match result {
        Ok(r) => r,
        Err(payload) => {
            let message = payload
                .downcast_ref::<String>()
                .cloned()
                .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                .unwrap_or_default();
            return Outcome::Fail(Failure::Panic(message));
        }
    };
    match (&case.expect, result) {
        (Expect::Output { css, .. }, Ok(actual)) => {
            let (expected, actual) = (normalize(css), normalize(&actual));
            if expected == actual {
                Outcome::Pass {
                    error_message: None,
                }
            } else {
                Outcome::Fail(Failure::Output { expected, actual })
            }
        }
        (Expect::Output { .. }, Err(e)) => Outcome::Fail(Failure::UnexpectedError(e)),
        (Expect::Error(_), Ok(css)) => Outcome::Fail(Failure::UnexpectedSuccess(css)),
        (Expect::Error(expected), Err(e)) => Outcome::Pass {
            error_message: Some(error_line(expected) == error_line(&e)),
        },
        (Expect::Missing, _) => unreachable!("handled above"),
    }
}
