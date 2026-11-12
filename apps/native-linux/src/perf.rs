//! `MDOW_PERF=1` logs how long starting up, opening documents and switching between them take,
//! from the request to the frame that shows the result, as the Mac apps' variable of the same
//! name does. `MDOW_PERF_RUN=<scenarios>` also drives the window through them and quits; see
//! `scripts/perf.sh`.

use std::cell::{Cell, RefCell};
use std::sync::OnceLock;
use std::time::Instant;

static START: OnceLock<Instant> = OnceLock::new();

thread_local! {
    static LAST_STEP: Cell<Option<Instant>> = const { Cell::new(None) };
    static PENDING: RefCell<Vec<(String, Instant, f64)>> = const { RefCell::new(Vec::new()) };
    static FRAMES: RefCell<Option<Frames>> = const { RefCell::new(None) };
    static FRAME_BEGAN: Cell<Option<Instant>> = const { Cell::new(None) };
    static DRAWN: Cell<u32> = const { Cell::new(0) };
}

/// Frames recorded for a benchmark: when each ended, and how long the main thread spent on it.
#[derive(Default)]
struct Frames {
    stamps: Vec<Instant>,
    work: Vec<f64>,
}

pub fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("MDOW_PERF").is_some() || bench().is_some())
}

/// The scenarios to run, from `MDOW_PERF_RUN`.
pub fn bench() -> Option<String> {
    std::env::var("MDOW_PERF_RUN").ok().filter(|s| !s.is_empty())
}

fn ms(from: Instant, to: Instant) -> f64 {
    (to - from).as_secs_f64() * 1000.0
}

/// The CPU time this thread has used, in milliseconds. Unlike the wall clock, it leaves out
/// waiting for the display, so it holds on any backend.
fn thread_cpu() -> f64 {
    let mut time = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: `time` is a valid timespec for the call to fill in.
    unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut time) };
    time.tv_sec as f64 * 1e3 + time.tv_nsec as f64 / 1e6
}

/// Starts the clock. Call first in `main`. `MDOW_PERF_EPOCH` (seconds since the Unix epoch when
/// the process was launched) adds the time spent before `main`: loading and linking.
pub fn init() {
    let start = *START.get_or_init(Instant::now);
    if !enabled() {
        return;
    }
    let launched = std::env::var("MDOW_PERF_EPOCH").ok().and_then(|v| v.parse::<f64>().ok());
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH);
    if let (Some(launched), Ok(now)) = (launched, now) {
        eprintln!("MDOWPERF startup: exec to main {:.1}ms", (now.as_secs_f64() - launched) * 1e3);
    }
    LAST_STEP.set(Some(start));
}

/// Logs a startup step that finished now: how long it took, and the time since `main` began.
pub fn step(label: &str) {
    if !enabled() {
        return;
    }
    let now = Instant::now();
    let start = *START.get_or_init(Instant::now);
    let last = LAST_STEP.replace(Some(now)).unwrap_or(start);
    eprintln!("MDOWPERF startup: {label} {:.1}ms (at {:.1}ms)", ms(last, now), ms(start, now));
}

/// Logs a step that finished now.
pub fn note(label: &str, since: Instant) {
    if enabled() {
        eprintln!("MDOWPERF {label} {:.1}ms", ms(since, Instant::now()));
    }
}

/// Times `label` from `since` until the next frame reaches the screen, with the main thread's
/// CPU time from this call to that frame.
pub fn mark(label: impl Into<String>, since: Instant) {
    if enabled() {
        PENDING.with_borrow_mut(|p| p.push((label.into(), since, thread_cpu())));
    }
}

/// How many frames have been drawn.
pub fn frames_drawn() -> u32 {
    DRAWN.get()
}

/// Called as a frame starts.
pub fn frame_began() {
    FRAME_BEGAN.set(Some(Instant::now()));
}

/// Called after each drawn frame.
pub fn frame_drawn() {
    let now = Instant::now();
    let first = DRAWN.get() == 0;
    DRAWN.set(DRAWN.get() + 1);
    if first {
        step("first frame");
        eprintln!("MDOWPERF startup: main thread {:.1}ms", thread_cpu());
    }
    let start = *START.get_or_init(Instant::now);
    for (label, since, cpu) in PENDING.with_borrow_mut(std::mem::take) {
        eprintln!(
            "MDOWPERF {label} on screen {:.1}ms, main thread {:.1}ms (at {:.1}ms)",
            ms(since, now),
            thread_cpu() - cpu,
            ms(start, now)
        );
    }
    let began = FRAME_BEGAN.take();
    FRAMES.with_borrow_mut(|frames| {
        if let (Some(frames), Some(began)) = (frames.as_mut(), began) {
            frames.stamps.push(now);
            frames.work.push(ms(began, now));
        }
    });
}

/// Logs the memory in use: the resident set, and how much of it is the process's own rather
/// than shared libraries and mapped files.
pub fn memory() {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else { return };
    let kb = |key: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(key))
            .and_then(|rest| rest.split_whitespace().next()?.parse::<f64>().ok())
            .unwrap_or(0.0)
    };
    eprintln!(
        "MDOWPERF memory {:.0}MB resident, {:.0}MB private",
        kb("VmRSS:") / 1024.0,
        kb("RssAnon:") / 1024.0
    );
}

/// Starts recording frames.
pub fn frames_begin() {
    FRAMES.with_borrow_mut(|frames| *frames = Some(Frames::default()));
}

/// Stops recording and logs the frames' pacing and cost. `work` is the main thread's time in a
/// frame (layout, building rows and drawing); `gap` is the time between frames, and hitches are
/// gaps over one and a half times the usual one, as in the Mac apps' scroll benchmark.
pub fn frames_report(label: &str) {
    let Some(frames) = FRAMES.with_borrow_mut(Option::take) else { return };
    let mut gaps: Vec<f64> = frames.stamps.windows(2).map(|w| ms(w[0], w[1])).collect();
    let mut work = frames.work;
    if gaps.is_empty() {
        eprintln!("MDOWBENCH {label} frames={}", work.len());
        return;
    }
    gaps.sort_by(f64::total_cmp);
    work.sort_by(f64::total_cmp);
    let pct = |sorted: &[f64], p: f64| sorted[((sorted.len() as f64 - 1.0) * p) as usize];
    let budget = pct(&gaps, 0.5);
    let hitches = gaps.iter().filter(|g| **g > budget * 1.5).count();
    eprintln!(
        "MDOWBENCH {label} frames={} work p50={:.2}ms p95={:.2}ms p99={:.2}ms max={:.2}ms \
         gap p50={budget:.2}ms p95={:.2}ms max={:.2}ms hitches={hitches}",
        work.len(),
        pct(&work, 0.5),
        pct(&work, 0.95),
        pct(&work, 0.99),
        work[work.len() - 1],
        pct(&gaps, 0.95),
        gaps[gaps.len() - 1],
    );
}
