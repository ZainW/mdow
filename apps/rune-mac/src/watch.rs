//! Live reload: watches the open documents and each tab's folder with FSEvents (notify), and
//! reports changed paths to the window's `poll`.

use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, channel};

#[derive(Default)]
pub struct Watcher {
    inner: Option<RecommendedWatcher>,
    rx: Option<Receiver<PathBuf>>,
    files: HashSet<PathBuf>,
    folders: HashSet<PathBuf>,
}

impl Watcher {
    pub fn setup(&mut self, waker: rune::Waker) {
        let (tx, rx) = channel();
        let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            let Ok(event) = res else { return };
            if matches!(event.kind, notify::EventKind::Access(_)) {
                return;
            }
            for path in event.paths {
                let _ = tx.send(path);
            }
            waker();
        });
        if let Ok(w) = watcher {
            self.inner = Some(w);
            self.rx = Some(rx);
        }
    }

    /// Watch exactly these files (by their folders) and folders (recursively).
    pub fn set(&mut self, files: HashSet<PathBuf>, folders: HashSet<PathBuf>) {
        let Some(w) = self.inner.as_mut() else { return };
        // What each path should be watched as. A folder that is also an open file's folder is
        // watched recursively, which covers the file.
        let wanted = |files: &HashSet<PathBuf>,
                      folders: &HashSet<PathBuf>|
         -> HashMap<PathBuf, RecursiveMode> {
            let mut out: HashMap<PathBuf, RecursiveMode> = files
                .iter()
                .filter_map(|f| f.parent().map(|d| (d.to_path_buf(), RecursiveMode::NonRecursive)))
                .collect();
            for f in folders {
                out.insert(f.clone(), RecursiveMode::Recursive);
            }
            out
        };
        let before = wanted(&self.files, &self.folders);
        let after = wanted(&files, &folders);
        for (path, mode) in &before {
            if after.get(path) != Some(mode) {
                let _ = w.unwatch(path);
            }
        }
        for (path, mode) in &after {
            if before.get(path) != Some(mode) {
                let _ = w.watch(path, *mode);
            }
        }
        self.files = files;
        self.folders = folders;
    }

    /// Paths that changed since the last call.
    pub fn changes(&mut self) -> Vec<PathBuf> {
        let Some(rx) = self.rx.as_ref() else { return Vec::new() };
        let mut out: Vec<PathBuf> = Vec::new();
        while let Ok(p) = rx.try_recv() {
            if !out.contains(&p) {
                out.push(p);
            }
        }
        out
    }
}
