//! Every open Mdow window, so menu commands reach the one in front.
//!
//! Native menu items belong to the first window's app and run with its state, so they don't act
//! directly: they queue a [`Cmd`] for the focused window and wake it, and that window runs it in
//! its next `poll`. Files opened from Finder or the Dock arrive the same way.

use crate::app::Cmd;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Mutex;

struct Entry {
    key: String,
    waker: Option<rune::Waker>,
    focused: bool,
    /// Bumped each time the window takes focus, so the most recent one wins.
    order: u64,
}

static WINDOWS: Mutex<Vec<Entry>> = Mutex::new(Vec::new());
static QUEUE: Mutex<Vec<(String, Cmd)>> = Mutex::new(Vec::new());
/// Files for windows that haven't started yet, oldest first.
static PENDING: Mutex<VecDeque<(String, Vec<PathBuf>)>> = Mutex::new(VecDeque::new());
static ORDER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
static NEXT_WINDOW: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub const MAIN: &str = "main";

pub fn register(key: &str, waker: rune::Waker) {
    let mut w = WINDOWS.lock().unwrap();
    w.retain(|e| e.key != key);
    let order = ORDER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    w.push(Entry { key: key.to_string(), waker: Some(waker), focused: false, order });
}

pub fn unregister(key: &str) {
    WINDOWS.lock().unwrap().retain(|e| e.key != key);
}

pub fn set_focused(key: &str, focused: bool) {
    let order = ORDER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    for e in WINDOWS.lock().unwrap().iter_mut() {
        if e.key == key {
            e.focused = focused;
            if focused {
                e.order = order;
            }
        } else if focused {
            e.focused = false;
        }
    }
}

/// The window commands go to: the focused one, else the one focused last.
pub fn target() -> String {
    let w = WINDOWS.lock().unwrap();
    w.iter()
        .find(|e| e.focused)
        .or_else(|| w.iter().max_by_key(|e| e.order))
        .map(|e| e.key.clone())
        .unwrap_or_else(|| MAIN.to_string())
}

pub fn send(key: &str, cmd: Cmd) {
    QUEUE.lock().unwrap().push((key.to_string(), cmd));
    wake(key);
}

#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn send_to_front(cmd: Cmd) {
    send(&target(), cmd);
}

pub fn take(key: &str) -> Vec<Cmd> {
    let mut q = QUEUE.lock().unwrap();
    let mut mine = Vec::new();
    q.retain(|(k, c)| {
        if k == key {
            mine.push(c.clone());
            false
        } else {
            true
        }
    });
    mine
}

fn wake(key: &str) {
    let waker = WINDOWS.lock().unwrap().iter().find(|e| e.key == key).and_then(|e| e.waker.clone());
    if let Some(w) = waker {
        w();
    }
}

pub fn wake_all() {
    let wakers: Vec<rune::Waker> =
        WINDOWS.lock().unwrap().iter().filter_map(|e| e.waker.clone()).collect();
    for w in wakers {
        w();
    }
}

pub fn keys() -> Vec<String> {
    WINDOWS.lock().unwrap().iter().map(|e| e.key.clone()).collect()
}

pub fn push_pending(key: String, paths: Vec<PathBuf>) {
    PENDING.lock().unwrap().push_back((key, paths));
}

/// The key and files of the next window to start.
pub fn take_pending() -> Option<(String, Vec<PathBuf>)> {
    PENDING.lock().unwrap().pop_front()
}

pub fn new_key() -> String {
    format!("window-{}", NEXT_WINDOW.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
}

/// The document window commands from a Settings window go to: the last one focused.
pub fn document_target() -> String {
    let w = WINDOWS.lock().unwrap();
    w.iter()
        .filter(|e| e.key != "settings" && e.key != "shortcuts")
        .max_by_key(|e| e.order)
        .map(|e| e.key.clone())
        .unwrap_or_else(|| MAIN.to_string())
}
