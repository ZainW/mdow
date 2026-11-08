//! `MDOW_PERF=1` logs how long opening and switching documents take, from the request to the
//! frame that shows the result, as the Swift app's variable of the same name logs its steps.

use std::cell::RefCell;
use std::time::Instant;

thread_local! {
    static PENDING: RefCell<Option<(String, Instant)>> = const { RefCell::new(None) };
}

pub fn enabled() -> bool {
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("MDOW_PERF").is_some())
}

/// Starts timing `label` until the next frame reaches the screen.
pub fn mark(label: impl Into<String>) {
    if enabled() {
        PENDING.with_borrow_mut(|p| *p = Some((label.into(), Instant::now())));
    }
}

/// Logs a step that finished now.
pub fn note(label: &str, since: Instant) {
    if enabled() {
        eprintln!("MDOWPERF {label} {:.1}ms", since.elapsed().as_secs_f64() * 1000.0);
    }
}

/// Called after each drawn frame.
pub fn frame_drawn() {
    if let Some((label, at)) = PENDING.with_borrow_mut(|p| p.take()) {
        eprintln!("MDOWPERF {label} on screen {:.1}ms", at.elapsed().as_secs_f64() * 1000.0);
    }
}

thread_local! {
    static BENCH: RefCell<Option<(Instant, f64, Vec<Instant>)>> = const { RefCell::new(None) };
}

/// Records the frames drawn over the next `seconds`, then logs their pacing as the Swift app's
/// scroll benchmark does: hitches are frames that took over one and a half refresh intervals.
pub fn bench(seconds: f64) {
    BENCH.with_borrow_mut(|b| *b = Some((Instant::now(), seconds, Vec::new())));
}

pub fn bench_frame() {
    let done = BENCH.with_borrow_mut(|b| {
        let (start, seconds, stamps) = b.as_mut()?;
        stamps.push(Instant::now());
        (start.elapsed().as_secs_f64() > *seconds).then(|| b.take()).flatten()
    });
    let Some((_, _, stamps)) = done else { return };
    let mut gaps: Vec<f64> =
        stamps.windows(2).map(|w| (w[1] - w[0]).as_secs_f64() * 1000.0).collect();
    if gaps.is_empty() {
        return;
    }
    let budget = {
        let mut sorted = gaps.clone();
        sorted.sort_by(f64::total_cmp);
        sorted[sorted.len() / 2]
    };
    let hitches = gaps.iter().filter(|g| **g > budget * 1.5).count();
    let lost: f64 = gaps.iter().filter(|g| **g > budget * 1.5).map(|g| g - budget).sum();
    let mean = gaps.iter().sum::<f64>() / gaps.len() as f64;
    gaps.sort_by(f64::total_cmp);
    let pct = |p: f64| gaps[((gaps.len() as f64 - 1.0) * p) as usize];
    eprintln!(
        "MDOWBENCH frames={} budget={budget:.1}ms mean={mean:.2}ms p50={:.2} p95={:.2} p99={:.2} max={:.2} hitches={hitches} lost={lost:.0}ms",
        gaps.len() + 1,
        pct(0.5),
        pct(0.95),
        pct(0.99),
        gaps[gaps.len() - 1],
    );
}
