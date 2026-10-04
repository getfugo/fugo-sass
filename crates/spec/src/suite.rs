//! The specs: options, expectations, and loading them from sass-spec.

use super::*;

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
    pub(crate) files: HashMap<PathBuf, Vec<u8>>,
    /// The directories of every archive (the archives themselves included).
    pub(crate) dirs: HashSet<PathBuf>,
    pub cases: Vec<Case>,
}

pub(super) const INPUTS: [&str; 2] = ["input.scss", "input.sass"];

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
