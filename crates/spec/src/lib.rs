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

/// The implementation whose expectations and options apply (sass-spec's `--impl`).
pub const IMPL: &str = "dart-sass";

/// The files and directories of an HRX archive (<https://github.com/google/hrx>).
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Hrx {
    /// Each file's path in the archive and its contents.
    pub files: Vec<(String, String)>,
    /// The directories the archive declares (`<===> dir/`), without the `/`.
    pub dirs: Vec<String>,
}

/// Parses an HRX archive. The boundary is the archive's first line's `<`, `=`s and `>`; a body
/// ends before the newline that precedes the next boundary, or at the end of the archive.
///
/// # Errors
/// An archive that does not start with a boundary, or an entry with no path after it.
pub fn parse_hrx(text: &str) -> Result<Hrx, String> {
    let bytes = text.as_bytes();
    let blen = match bytes.first() {
        Some(b'<') => {
            let eqs = bytes[1..].iter().take_while(|&&b| b == b'=').count();
            if eqs == 0 || bytes.get(1 + eqs) != Some(&b'>') {
                return Err("the archive does not start with a boundary".to_owned());
            }
            eqs + 2
        }
        _ => return Err("the archive does not start with a boundary".to_owned()),
    };
    let boundary = &text[..blen];
    let is_boundary = |at: usize| {
        text[at..].starts_with(boundary)
            && matches!(bytes.get(at + blen), None | Some(b' ' | b'\n'))
    };
    let mut starts = vec![0];
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'\n' && i + 1 < bytes.len() && is_boundary(i + 1) {
            starts.push(i + 1);
        }
    }
    let mut hrx = Hrx::default();
    for (n, &start) in starts.iter().enumerate() {
        let header_end = text[start..].find('\n').map_or(text.len(), |i| start + i);
        let header = &text[start + blen..header_end];
        let body_start = (header_end + 1).min(text.len());
        let body = match starts.get(n + 1) {
            Some(&next) => &text[body_start..(next - 1).max(body_start)],
            None => &text[body_start..],
        };
        if header.is_empty() {
            continue; // a comment
        }
        let Some(path) = header.strip_prefix(' ') else {
            return Err(format!("expected a path after {boundary:?}"));
        };
        if let Some(dir) = path.strip_suffix('/') {
            hrx.dirs.push(dir.to_owned());
        } else {
            hrx.files.push((path.to_owned(), body.to_owned()));
        }
    }
    Ok(hrx)
}

/// The options of `options.yml` files that matter to a runner.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SpecOptions {
    pub todo: Vec<String>,
    pub ignore_for: Vec<String>,
    pub warning_todo: Vec<String>,
}

impl SpecOptions {
    /// Parses the YAML subset sass-spec's `options.yml` files use: `:key:` lines followed by
    /// `- item` lines; other keys (`:precision:`) are ignored.
    #[must_use]
    pub fn parse(text: &str) -> Self {
        let mut o = Self::default();
        let mut list: Option<&mut Vec<String>> = None;
        for line in text.lines() {
            let t = line.trim();
            if let Some(item) = t.strip_prefix("- ") {
                if let Some(l) = list.as_mut() {
                    l.push(item.trim().to_owned());
                }
            } else if t.starts_with(':') {
                list = match t.trim_end_matches(':') {
                    ":todo" => Some(&mut o.todo),
                    ":ignore_for" => Some(&mut o.ignore_for),
                    ":warning_todo" => Some(&mut o.warning_todo),
                    _ => None,
                };
            }
        }
        o
    }

    /// These options with `inner`'s (a nested `options.yml`) added.
    #[must_use]
    pub fn merge(&self, inner: &Self) -> Self {
        let cat = |a: &[String], b: &[String]| a.iter().chain(b).cloned().collect();
        Self {
            todo: cat(&self.todo, &inner.todo),
            ignore_for: cat(&self.ignore_for, &inner.ignore_for),
            warning_todo: cat(&self.warning_todo, &inner.warning_todo),
        }
    }

    /// sass-spec's test: an entry names the implementation when it contains its name
    /// (`dart-sass`, `sass/dart-sass#123`).
    fn names(list: &[String]) -> bool {
        list.iter().any(|item| item.contains(IMPL))
    }

    #[must_use]
    pub fn mode(&self) -> Mode {
        if Self::names(&self.ignore_for) {
            Mode::Ignore
        } else if Self::names(&self.todo) {
            Mode::Todo
        } else {
            Mode::Run
        }
    }
}

/// Whether a spec runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Run,
    /// dart-sass does not pass it yet.
    Todo,
    /// Not meant for dart-sass.
    Ignore,
}

/// What a spec expects.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Expect {
    /// The CSS, and the warnings (`warning`), if any.
    Output {
        css: String,
        warning: Option<String>,
    },
    /// The error message (`error`).
    Error(String),
    /// Neither `output.css` nor `error`.
    Missing,
}

impl Expect {
    /// The expectation of a spec directory, given its files by name.
    fn of(file: impl Fn(&str) -> Option<String>) -> Self {
        let warning = || file(&format!("warning-{IMPL}")).or_else(|| file("warning"));
        if let Some(css) = file(&format!("output-{IMPL}.css")) {
            return Self::Output {
                css,
                warning: warning(),
            };
        }
        if let Some(e) = file(&format!("error-{IMPL}")) {
            return Self::Error(e);
        }
        if let Some(css) = file("output.css") {
            return Self::Output {
                css,
                warning: warning(),
            };
        }
        file("error").map_or(Self::Missing, Self::Error)
    }
}

/// One spec.
#[derive(Clone, Debug)]
pub struct Case {
    /// Its path below the spec root, `/`-separated (`core_functions/color/rgb/…`).
    pub name: String,
    /// Its directory (virtual for an archive's specs).
    pub dir: PathBuf,
    /// `input.scss` or `input.sass` in `dir`.
    pub input: PathBuf,
    pub expect: Expect,
    pub mode: Mode,
    pub warning_todo: bool,
}

/// The specs below a root, with the files of their archives.
#[derive(Debug, Default)]
pub struct Suite {
    /// The canonical spec root.
    pub root: PathBuf,
    /// The files of every archive, at their virtual paths below `root`.
    files: HashMap<PathBuf, Vec<u8>>,
    /// The directories of every archive (the archives themselves included).
    dirs: HashSet<PathBuf>,
    pub cases: Vec<Case>,
}

const INPUTS: [&str; 2] = ["input.scss", "input.sass"];

impl Suite {
    /// Reads every spec below `root` (sass-spec's `spec` directory).
    ///
    /// # Errors
    /// An unreadable directory or file, or an archive that is not HRX.
    pub fn load(root: &Path) -> io::Result<Self> {
        let mut suite = Self {
            root: fs::canonicalize(root)?,
            ..Self::default()
        };
        let root = suite.root.clone();
        suite.walk(&root, "", &SpecOptions::default())?;
        suite.cases.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(suite)
    }

    fn walk(&mut self, dir: &Path, rel: &str, outer: &SpecOptions) -> io::Result<()> {
        let options = match fs::read_to_string(dir.join("options.yml")) {
            Ok(text) => outer.merge(&SpecOptions::parse(&text)),
            Err(_) => outer.clone(),
        };
        if let Some(input) = INPUTS.iter().map(|i| dir.join(i)).find(|p| p.is_file()) {
            self.cases.push(Case {
                name: rel.to_owned(),
                dir: dir.to_path_buf(),
                input,
                expect: Expect::of(|name| fs::read_to_string(dir.join(name)).ok()),
                mode: options.mode(),
                warning_todo: SpecOptions::names(&options.warning_todo),
            });
        }
        let mut entries: Vec<_> = fs::read_dir(dir)?.collect::<io::Result<_>>()?;
        entries.sort_by_key(fs::DirEntry::file_name);
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let child = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            if entry.file_type()?.is_dir() {
                self.walk(&path, &child, &options)?;
            } else if let Some(stem) = child.strip_suffix(".hrx") {
                let text = fs::read_to_string(&path)?;
                let hrx = parse_hrx(&text).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("{}: {e}", path.display()),
                    )
                })?;
                self.add_archive(&dir.join(&name[..name.len() - 4]), stem, &hrx, &options);
            }
        }
        Ok(())
    }

    /// Adds an archive's files at `base` and its specs below `rel`.
    fn add_archive(&mut self, base: &Path, rel: &str, hrx: &Hrx, options: &SpecOptions) {
        let files: BTreeMap<&str, &str> = hrx
            .files
            .iter()
            .map(|(p, c)| (p.as_str(), c.as_str()))
            .collect();
        self.dirs.insert(base.to_path_buf());
        for path in files
            .keys()
            .copied()
            .chain(hrx.dirs.iter().map(|d| d.as_str()))
        {
            let mut p = base.to_path_buf();
            let parts: Vec<&str> = path.split('/').collect();
            let (last, parents) = parts.split_last().expect("split yields one part");
            for part in parents {
                p.push(part);
                self.dirs.insert(p.clone());
            }
            if files.contains_key(path) {
                self.files
                    .insert(p.join(last), files[path].as_bytes().to_vec());
            } else {
                self.dirs.insert(p.join(last));
            }
        }
        for &path in files.keys() {
            let (dir, file) = path.rsplit_once('/').unwrap_or(("", path));
            if !INPUTS.contains(&file) {
                continue;
            }
            let prefix = |name: &str| {
                if dir.is_empty() {
                    name.to_owned()
                } else {
                    format!("{dir}/{name}")
                }
            };
            // The archive's `options.yml` files on the way down to the spec.
            let mut opts = options.clone();
            let mut at = String::new();
            for part in std::iter::once("").chain(dir.split('/').filter(|s| !s.is_empty())) {
                if !part.is_empty() {
                    if !at.is_empty() {
                        at.push('/');
                    }
                    at.push_str(part);
                }
                let yml = if at.is_empty() {
                    "options.yml".to_owned()
                } else {
                    format!("{at}/options.yml")
                };
                if let Some(text) = files.get(yml.as_str()) {
                    opts = opts.merge(&SpecOptions::parse(text));
                }
            }
            let case_dir = if dir.is_empty() {
                base.to_path_buf()
            } else {
                base.join(dir)
            };
            self.cases.push(Case {
                name: if dir.is_empty() {
                    rel.to_owned()
                } else {
                    format!("{rel}/{dir}")
                },
                input: case_dir.join(file),
                dir: case_dir,
                expect: Expect::of(|name| {
                    files.get(prefix(name).as_str()).map(|s| (*s).to_owned())
                }),
                mode: opts.mode(),
                warning_todo: SpecOptions::names(&opts.warning_todo),
            });
        }
    }
}

/// `p` with `.` and `..` resolved.
fn lexical(p: &Path) -> PathBuf {
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
struct SpecFs<'a>(&'a Suite);

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
struct Captured(RefCell<Vec<String>>);

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
fn reduce_input_paths(s: &str) -> String {
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
fn error_line(s: &str) -> Option<&str> {
    s.lines()
        .map(str::trim_end)
        .find(|l| l.starts_with("Error:"))
}

/// Compiles a spec's input as sass-spec runs dart-sass: `--load-path=<spec root> --no-unicode`.
fn compile(suite: &Suite, case: &Case) -> (Result<String, String>, Vec<String>) {
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

/// The state the workers of [`run`] share.
struct Shared {
    suite: Suite,
    picks: Vec<usize>,
    run_todo: bool,
    next: AtomicUsize,
    results: Mutex<Vec<Option<Outcome>>>,
    /// Per worker: the pick it runs and since when.
    running: Mutex<Vec<Option<(usize, Instant)>>>,
}

fn worker(shared: &Arc<Shared>, slot: usize) {
    loop {
        let i = shared.next.fetch_add(1, Ordering::SeqCst);
        let Some(&case) = shared.picks.get(i) else {
            return;
        };
        shared.running.lock().unwrap()[slot] = Some((i, Instant::now()));
        let outcome = run_case(&shared.suite, &shared.suite.cases[case], shared.run_todo);
        let mut running = shared.running.lock().unwrap();
        if running[slot].is_none() {
            return; // timed out: another worker took this slot
        }
        running[slot] = None;
        shared.results.lock().unwrap()[i].get_or_insert(outcome);
    }
}

fn spawn_worker(shared: &Arc<Shared>, slot: usize) {
    let shared = Arc::clone(shared);
    thread::Builder::new()
        .name(format!("spec-{slot}"))
        // Deeply nested stylesheets recurse deeply; the main thread's 8 MiB is not always enough.
        .stack_size(256 << 20)
        .spawn(move || worker(&shared, slot))
        .expect("spawn a spec worker");
}

/// Runs the specs `picks` (indexes into `suite.cases`) on all cores; a spec still running after
/// `timeout` fails, its thread abandoned.
#[must_use]
pub fn run(
    suite: Suite,
    picks: Vec<usize>,
    run_todo: bool,
    timeout: Duration,
) -> (Suite, Vec<Outcome>) {
    let n = picks.len();
    let workers = thread::available_parallelism().map_or(4, |n| n.get());
    let shared = Arc::new(Shared {
        suite,
        picks,
        run_todo,
        next: AtomicUsize::new(0),
        results: Mutex::new(vec![None; n]),
        running: Mutex::new(vec![None; workers]),
    });
    // Panics are outcomes here; keep the default hook for other threads.
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        if !thread::current()
            .name()
            .is_some_and(|t| t.starts_with("spec-"))
        {
            default_hook(info);
        }
    }));
    for slot in 0..workers {
        spawn_worker(&shared, slot);
    }
    loop {
        thread::sleep(Duration::from_millis(50));
        let mut stuck = Vec::new();
        {
            let mut running = shared.running.lock().unwrap();
            for run in running.iter_mut() {
                if let Some((i, since)) = *run
                    && since.elapsed() > timeout
                {
                    // Its worker sees the empty slot when (if) it returns, and stops.
                    *run = None;
                    stuck.push(i);
                }
            }
        }
        for i in stuck {
            shared.results.lock().unwrap()[i].get_or_insert(Outcome::Fail(Failure::Timeout));
            let new_slot = {
                let mut running = shared.running.lock().unwrap();
                running.push(None);
                running.len() - 1
            };
            spawn_worker(&shared, new_slot);
        }
        if shared.results.lock().unwrap().iter().all(Option::is_some) {
            break;
        }
    }
    let _ = panic::take_hook(); // back to the default hook
    let outcomes = shared
        .results
        .lock()
        .unwrap()
        .iter()
        .map(|o| o.clone().expect("every spec ran"))
        .collect();
    // Workers still stuck in a timed-out spec hold the suite; give them a copy-free exit path.
    let suite = match Arc::try_unwrap(shared) {
        Ok(shared) => shared.suite,
        Err(shared) => Suite {
            root: shared.suite.root.clone(),
            cases: shared.suite.cases.clone(),
            ..Suite::default()
        },
    };
    (suite, outcomes)
}

/// Counts of a group of specs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub passed: usize,
    pub failed: usize,
    pub todo: usize,
    pub ignored: usize,
    /// Error specs that passed with the same first `Error:` line.
    pub error_message_matched: usize,
    /// Error specs that passed.
    pub error_specs_passed: usize,
}

impl Tally {
    pub fn add(&mut self, outcome: &Outcome) {
        match outcome {
            Outcome::Pass { error_message } => {
                self.passed += 1;
                if let Some(matched) = error_message {
                    self.error_specs_passed += 1;
                    self.error_message_matched += usize::from(*matched);
                }
            }
            Outcome::Fail(_) => self.failed += 1,
            Outcome::Todo => self.todo += 1,
            Outcome::Ignored => self.ignored += 1,
        }
    }

    /// The share of the specs that ran that passed, in percent.
    #[must_use]
    pub fn percent(&self) -> f64 {
        let ran = self.passed + self.failed;
        if ran == 0 {
            100.0
        } else {
            #[allow(clippy::cast_precision_loss)]
            let p = self.passed as f64 * 100.0 / ran as f64;
            p
        }
    }
}

/// A table of the tallies per group of the first `depth` path segments of the specs' names.
#[must_use]
pub fn table(cases: &[&Case], outcomes: &[Outcome], depth: usize) -> String {
    let mut groups: BTreeMap<String, Tally> = BTreeMap::new();
    let mut total = Tally::default();
    for (case, outcome) in cases.iter().zip(outcomes) {
        let group = case
            .name
            .split('/')
            .take(depth)
            .collect::<Vec<_>>()
            .join("/");
        groups.entry(group).or_default().add(outcome);
        total.add(outcome);
    }
    let width = groups.keys().map(String::len).max().unwrap_or(5).max(5);
    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:width$}  {:>6}  {:>6}  {:>6}  {:>5}  {:>7}",
        "", "passed", "failed", "todo", "ign.", "%"
    );
    for (name, t) in groups
        .iter()
        .chain(std::iter::once((&"TOTAL".to_owned(), &total)))
    {
        let _ = writeln!(
            out,
            "{name:width$}  {:>6}  {:>6}  {:>6}  {:>5}  {:>6.2}%",
            t.passed,
            t.failed,
            t.todo,
            t.ignored,
            t.percent()
        );
    }
    let _ = writeln!(
        out,
        "error specs: {} of {} passed with the same first `Error:` line",
        total.error_message_matched, total.error_specs_passed
    );
    out
}

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
