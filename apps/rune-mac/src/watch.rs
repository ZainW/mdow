//! Live reload: watches the open documents and each tab's folder with FSEvents (notify), and
//! reports changed paths to the window's `poll`.

use notify::{RecommendedWatcher, RecursiveMode, Watcher as _};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
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
        let dirs = |files: &HashSet<PathBuf>| -> HashSet<PathBuf> {
            files.iter().filter_map(|f| f.parent().map(Path::to_path_buf)).collect()
        };
        let (old_dirs, new_dirs) = (dirs(&self.files), dirs(&files));
        for d in old_dirs.difference(&new_dirs) {
            if !self.folders.contains(d) {
                let _ = w.unwatch(d);
            }
        }
        for f in self.folders.difference(&folders) {
            let _ = w.unwatch(f);
        }
        for d in new_dirs.difference(&old_dirs) {
            let _ = w.watch(d, RecursiveMode::NonRecursive);
        }
        for f in folders.difference(&self.folders) {
            let _ = w.watch(f, RecursiveMode::Recursive);
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
