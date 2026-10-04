//! Running specs in parallel, and summarizing the results.

use super::*;

/// The state the workers of [`run`] share.
pub(super) struct Shared {
    suite: Suite,
    picks: Vec<usize>,
    run_todo: bool,
    next: AtomicUsize,
    results: Mutex<Vec<Option<Outcome>>>,
    /// Per worker: the pick it runs and since when.
    running: Mutex<Vec<Option<(usize, Instant)>>>,
}

pub(super) fn worker(shared: &Arc<Shared>, slot: usize) {
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

pub(super) fn spawn_worker(shared: &Arc<Shared>, slot: usize) {
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
