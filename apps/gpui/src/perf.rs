//! Opt-in timing probe for the real app, used by `script/bench_gpui_open.sh`.
//!
//! Set `MDOW_PERF_LOG=/path/report.json` and launch with a document: the app records when the
//! open started, when the reader first rendered a loading line or real content, when the full
//! document landed, and the longest main-thread stall (a 4ms heartbeat that measures how late it
//! wakes). With `MDOW_PERF_RELOAD=1` it then edits the file near its top to time a live reload,
//! writes the report, and quits. Without the variable every call is a cheap no-op.

use gpui::{App, Timer};
use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

struct Probe {
    log: PathBuf,
    started: Instant,
    events: Vec<(String, f64)>,
    max_stall_ms: f64,
    stalls_over_50ms: usize,
    stall_samples: Vec<(f64, f64)>,
}

fn probe() -> Option<&'static Mutex<Probe>> {
    static PROBE: OnceLock<Option<Mutex<Probe>>> = OnceLock::new();
    PROBE
        .get_or_init(|| {
            std::env::var_os("MDOW_PERF_LOG").map(|log| {
                Mutex::new(Probe {
                    log: PathBuf::from(log),
                    started: Instant::now(),
                    events: Vec::new(),
                    max_stall_ms: 0.0,
                    stalls_over_50ms: 0,
                    stall_samples: Vec::new(),
                })
            })
        })
        .as_ref()
}

pub fn enabled() -> bool {
    probe().is_some()
}

/// Records `event` at the current time (milliseconds since the probe started).
pub fn mark(event: &str) {
    if let Some(probe) = probe()
        && let Ok(mut probe) = probe.lock()
    {
        let at = probe.started.elapsed().as_secs_f64() * 1000.0;
        probe.events.push((event.to_owned(), at));
    }
}

/// Records `event` only the first time it happens since `after` was last marked.
pub fn mark_once_after(event: &str, after: &str) {
    if let Some(probe) = probe()
        && let Ok(mut probe) = probe.lock()
    {
        let last_after = probe.events.iter().rposition(|(name, _)| name == after);
        let last_event = probe.events.iter().rposition(|(name, _)| name == event);
        if last_after.is_some() && last_event.is_none_or(|event| event < last_after.unwrap()) {
            let at = probe.started.elapsed().as_secs_f64() * 1000.0;
            probe.events.push((event.to_owned(), at));
        }
    }
}

fn has_event(event: &str) -> bool {
    probe()
        .and_then(|probe| probe.lock().ok())
        .is_some_and(|probe| probe.events.iter().any(|(name, _)| name == event))
}

fn event_count(event: &str) -> usize {
    probe()
        .and_then(|probe| probe.lock().ok())
        .map_or(0, |probe| {
            probe
                .events
                .iter()
                .filter(|(name, _)| name == event)
                .count()
        })
}

fn record_stall(stall_ms: f64) {
    if let Some(probe) = probe()
        && let Ok(mut probe) = probe.lock()
    {
        probe.max_stall_ms = probe.max_stall_ms.max(stall_ms);
        if stall_ms > 50.0 {
            probe.stalls_over_50ms += 1;
            let at = probe.started.elapsed().as_secs_f64() * 1000.0;
            probe.stall_samples.push((at, stall_ms));
        }
    }
}

fn write_report() {
    let Some(probe) = probe() else {
        return;
    };
    let Ok(probe) = probe.lock() else {
        return;
    };
    let report = serde_json::json!({
        "events": probe
            .events
            .iter()
            .map(|(name, at)| serde_json::json!({ "event": name, "ms": at }))
            .collect::<Vec<_>>(),
        "max_main_thread_stall_ms": probe.max_stall_ms,
        "stalls_over_50ms": probe.stalls_over_50ms,
        "stall_samples_ms": probe
            .stall_samples
            .iter()
            .map(|(at, ms)| serde_json::json!({ "ended_at": at, "ms": ms }))
            .collect::<Vec<_>>(),
    });
    let _ = std::fs::write(
        &probe.log,
        serde_json::to_string_pretty(&report).unwrap_or_default(),
    );
}

const HEARTBEAT: Duration = Duration::from_millis(4);

/// Starts the stall heartbeat and, when a document was launched, the scripted scenario.
pub fn start(document: Option<PathBuf>, cx: &mut App) {
    if !enabled() {
        return;
    }
    mark("probe_start");
    cx.spawn(async move |_| {
        loop {
            let before = Instant::now();
            Timer::after(HEARTBEAT).await;
            let late = before.elapsed().saturating_sub(HEARTBEAT);
            record_stall(late.as_secs_f64() * 1000.0);
        }
    })
    .detach();
    let reload = std::env::var_os("MDOW_PERF_RELOAD").is_some();
    cx.spawn(async move |cx| {
        let deadline = Instant::now() + Duration::from_secs(60);
        while !has_event("full_ready") && Instant::now() < deadline {
            Timer::after(Duration::from_millis(20)).await;
        }
        Timer::after(Duration::from_millis(1500)).await;
        if reload && let Some(path) = document {
            for round in 0..3 {
                let applied_before = event_count("reload_painted");
                if let Ok(source) = std::fs::read_to_string(&path) {
                    // An edit near the top, like typing into the document's opening section.
                    let cut = source
                        .char_indices()
                        .nth(source.len().min(4096))
                        .map_or(source.len(), |(index, _)| index);
                    let cut = source[..cut].rfind("\n\n").map_or(cut, |index| index + 2);
                    let edited = format!(
                        "{}Inserted paragraph {round} while reading.\n\n{}",
                        &source[..cut],
                        &source[cut..]
                    );
                    mark("reload_write");
                    let _ = std::fs::write(&path, edited);
                }
                let deadline = Instant::now() + Duration::from_secs(30);
                while event_count("reload_painted") <= applied_before && Instant::now() < deadline {
                    Timer::after(Duration::from_millis(10)).await;
                }
                Timer::after(Duration::from_millis(700)).await;
            }
        }
        // A final idle window lets the stall monitor see the settled app.
        Timer::after(Duration::from_millis(500)).await;
        write_report();
        let _ = cx.update(|cx| cx.quit());
    })
    .detach();
}
