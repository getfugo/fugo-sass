//! `fugo-sass-spec`: runs sass-spec against fugo-sass (see the crate's README).

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use fugo_sass_spec::{Failure, Mode, Outcome, Suite, table};

const USAGE: &str = "\
usage: fugo-sass-spec [options] [PREFIX...]

Runs the specs whose names start with a PREFIX (default: all).

options:
  --spec DIR              sass-spec's spec directory (default: the sass-spec submodule's)
  --baseline FILE         fail if the failing specs differ from FILE's list
  --write-baseline FILE   write the failing specs to FILE
  --run-todo              also run the specs dart-sass does not pass (:todo:)
  --depth N               group the table by N name segments (default 1)
  --show N                print N failures with their expected and actual output
  --list                  print every failing spec and why
  --timeout SECS          fail a spec still compiling after SECS (default 20)
";

struct Args {
    spec: PathBuf,
    baseline: Option<PathBuf>,
    write_baseline: Option<PathBuf>,
    run_todo: bool,
    depth: usize,
    show: usize,
    list: bool,
    timeout: Duration,
    prefixes: Vec<String>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        spec: Path::new(env!("CARGO_MANIFEST_DIR")).join("../../sass-spec/spec"),
        baseline: None,
        write_baseline: None,
        run_todo: false,
        depth: 1,
        show: 0,
        list: false,
        timeout: Duration::from_secs(20),
        prefixes: Vec::new(),
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        let mut value = |name: &str| it.next().ok_or_else(|| format!("{name} needs a value"));
        match a.as_str() {
            "--spec" => args.spec = value("--spec")?.into(),
            "--baseline" => args.baseline = Some(value("--baseline")?.into()),
            "--write-baseline" => args.write_baseline = Some(value("--write-baseline")?.into()),
            "--run-todo" => args.run_todo = true,
            "--depth" => {
                args.depth = value("--depth")?
                    .parse()
                    .map_err(|e| format!("--depth: {e}"))?
            }
            "--show" => {
                args.show = value("--show")?
                    .parse()
                    .map_err(|e| format!("--show: {e}"))?
            }
            "--list" => args.list = true,
            "--timeout" => {
                let s: u64 = value("--timeout")?
                    .parse()
                    .map_err(|e| format!("--timeout: {e}"))?;
                args.timeout = Duration::from_secs(s);
            }
            "-h" | "--help" => return Err(String::new()),
            s if s.starts_with('-') => return Err(format!("unknown option {s}")),
            s => args.prefixes.push(s.trim_end_matches('/').to_owned()),
        }
    }
    Ok(args)
}

fn matches(name: &str, prefixes: &[String]) -> bool {
    prefixes.is_empty()
        || prefixes.iter().any(|p| {
            name == p
                || name
                    .strip_prefix(p.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
        })
}

fn describe(f: &Failure) -> String {
    match f {
        Failure::Output { expected, actual } => {
            format!("--- expected\n{expected}\n--- actual\n{actual}")
        }
        Failure::UnexpectedError(e) => format!("unexpected error:\n{}", e.trim_end()),
        Failure::UnexpectedSuccess(css) => format!("expected an error, got:\n{}", css.trim_end()),
        Failure::Panic(m) => format!("panic: {m}"),
        Failure::Timeout => "timeout".to_owned(),
        Failure::NoExpectation => "no output.css or error".to_owned(),
    }
}

fn read_list(path: &Path) -> Result<BTreeSet<String>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect())
}

fn main() -> ExitCode {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            if !e.is_empty() {
                eprintln!("{e}\n");
            }
            eprint!("{USAGE}");
            return ExitCode::from(2);
        }
    };
    let started = Instant::now();
    let suite = match Suite::load(&args.spec) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "cannot read the specs at {}: {e}\n(git submodule update --init sass-spec)",
                args.spec.display()
            );
            return ExitCode::from(2);
        }
    };
    let picks: Vec<usize> = (0..suite.cases.len())
        .filter(|&i| matches(&suite.cases[i].name, &args.prefixes))
        .collect();
    let (suite, outcomes) = fugo_sass_spec::run(suite, picks.clone(), args.run_todo, args.timeout);
    let cases: Vec<_> = picks.iter().map(|&i| &suite.cases[i]).collect();
    print!("{}", table(&cases, &outcomes, args.depth));
    println!(
        "{} specs in {:.1}s",
        cases.len(),
        started.elapsed().as_secs_f64()
    );

    let failing: BTreeSet<String> = cases
        .iter()
        .zip(&outcomes)
        .filter(|(_, o)| matches!(o, Outcome::Fail(_)))
        .map(|(c, _)| c.name.clone())
        .collect();
    let mut shown = 0;
    for (case, outcome) in cases.iter().zip(&outcomes) {
        let Outcome::Fail(f) = outcome else { continue };
        if args.list {
            println!("FAIL {} ({})", case.name, f.kind());
        }
        if shown < args.show {
            shown += 1;
            println!("\n=== {} ({})\n{}", case.name, f.kind(), describe(f));
        }
    }

    if let Some(path) = &args.write_baseline {
        let mut text = String::from(
            "# sass-spec specs fugo-sass fails (fugo-sass-spec --write-baseline); one per line.\n",
        );
        for name in &failing {
            text.push_str(name);
            text.push('\n');
        }
        if let Err(e) = fs::write(path, text) {
            eprintln!("{}: {e}", path.display());
            return ExitCode::from(2);
        }
        println!(
            "wrote {} failing specs to {}",
            failing.len(),
            path.display()
        );
    }
    if let Some(path) = &args.baseline {
        let baseline = match read_list(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        };
        // Only the specs this run covers (prefixes, todo) count.
        let ran: BTreeSet<&str> = cases
            .iter()
            .filter(|c| args.run_todo || c.mode == Mode::Run)
            .map(|c| c.name.as_str())
            .collect();
        let new: Vec<_> = failing.iter().filter(|n| !baseline.contains(*n)).collect();
        let fixed: Vec<_> = baseline
            .iter()
            .filter(|n| ran.contains(n.as_str()) && !failing.contains(*n))
            .collect();
        for n in &new {
            println!("NEW FAILURE {n}");
        }
        for n in &fixed {
            println!("NOW PASSES {n}");
        }
        if !new.is_empty() || !fixed.is_empty() {
            println!(
                "{} new failures, {} specs now pass: fix the failures, then update {} with --write-baseline",
                new.len(),
                fixed.len(),
                path.display()
            );
            return ExitCode::FAILURE;
        }
        println!("the failing specs match {}", path.display());
    }
    ExitCode::SUCCESS
}
