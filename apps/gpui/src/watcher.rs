use anyhow::{Context as _, anyhow};
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, RwLock,
        mpsc::{self, Receiver, RecvTimeoutError, Sender},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const DEBOUNCE: Duration = Duration::from_millis(150);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatchMessage {
    Reload(PathBuf),
}

enum RawWatchMessage {
    Paths(Vec<PathBuf>),
    Shutdown,
}

struct PathCoalescer {
    delay: Duration,
    deadlines: HashMap<PathBuf, Instant>,
}

impl PathCoalescer {
    fn new(delay: Duration) -> Self {
        Self {
            delay,
            deadlines: HashMap::new(),
        }
    }

    fn push(&mut self, path: PathBuf, now: Instant) {
        self.deadlines.insert(path, now + self.delay);
    }

    fn next_wait(&self, now: Instant) -> Option<Duration> {
        self.deadlines
            .values()
            .min()
            .map(|deadline| deadline.saturating_duration_since(now))
    }

    fn drain_due(&mut self, now: Instant) -> Vec<PathBuf> {
        let mut due = self
            .deadlines
            .iter()
            .filter_map(|(path, deadline)| (*deadline <= now).then_some(path.clone()))
            .collect::<Vec<_>>();
        due.sort();
        for path in &due {
            self.deadlines.remove(path);
        }
        due
    }
}

pub struct FileWatcher {
    watcher: RecommendedWatcher,
    watched_parents: HashSet<PathBuf>,
    watched_files: Arc<RwLock<HashSet<PathBuf>>>,
    messages: Arc<Mutex<Receiver<WatchMessage>>>,
    raw_sender: Sender<RawWatchMessage>,
    worker: Option<JoinHandle<()>>,
}

impl FileWatcher {
    pub fn new() -> anyhow::Result<Self> {
        let watched_files = Arc::new(RwLock::new(HashSet::new()));
        let (raw_sender, raw_receiver) = mpsc::channel();
        let (message_sender, message_receiver) = mpsc::channel();
        let callback_files = watched_files.clone();
        let callback_sender = raw_sender.clone();
        let watcher = notify::recommended_watcher(move |result: notify::Result<notify::Event>| {
            let Ok(event) = result else {
                return;
            };
            if matches!(event.kind, EventKind::Access(_)) {
                return;
            }
            let matching = matching_watched_paths(&event.paths, &callback_files);
            if !matching.is_empty() {
                let _ = callback_sender.send(RawWatchMessage::Paths(matching));
            }
        })
        .context("create file watcher")?;
        let worker = thread::Builder::new()
            .name("mdow-file-watch-debounce".into())
            .spawn(move || run_debounce_worker(raw_receiver, message_sender))
            .context("start file watcher debounce worker")?;

        Ok(Self {
            watcher,
            watched_parents: HashSet::new(),
            watched_files,
            messages: Arc::new(Mutex::new(message_receiver)),
            raw_sender,
            worker: Some(worker),
        })
    }

    pub fn watch(&mut self, path: &Path) -> anyhow::Result<()> {
        let canonical = path
            .canonicalize()
            .with_context(|| format!("resolve watched file {}", path.display()))?;
        let parent = canonical
            .parent()
            .ok_or_else(|| anyhow!("watched file has no parent: {}", canonical.display()))?
            .to_owned();

        if !self.watched_parents.contains(&parent) {
            self.watcher
                .watch(&parent, RecursiveMode::NonRecursive)
                .with_context(|| format!("watch directory {}", parent.display()))?;
            self.watched_parents.insert(parent);
        }
        self.watched_files.write().unwrap().insert(canonical);
        Ok(())
    }

    pub fn messages(&self) -> Arc<Mutex<Receiver<WatchMessage>>> {
        self.messages.clone()
    }
}

impl Drop for FileWatcher {
    fn drop(&mut self) {
        let _ = self.raw_sender.send(RawWatchMessage::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn matching_watched_paths(
    event_paths: &[PathBuf],
    watched_files: &RwLock<HashSet<PathBuf>>,
) -> Vec<PathBuf> {
    let watched_files = watched_files.read().unwrap();
    let mut matching = HashSet::new();
    for event_path in event_paths {
        let event_path = logical_file_identity(event_path);
        if let Some(watched) = watched_files.get(&event_path) {
            matching.insert(watched.clone());
        }
    }
    matching.into_iter().collect()
}

fn logical_file_identity(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| {
        path.parent()
            .and_then(|parent| parent.canonicalize().ok())
            .and_then(|parent| path.file_name().map(|name| parent.join(name)))
            .unwrap_or_else(|| path.to_owned())
    })
}

fn run_debounce_worker(
    raw_receiver: Receiver<RawWatchMessage>,
    message_sender: Sender<WatchMessage>,
) {
    let mut coalescer = PathCoalescer::new(DEBOUNCE);
    loop {
        let next = match coalescer.next_wait(Instant::now()) {
            Some(wait) => raw_receiver.recv_timeout(wait),
            None => raw_receiver
                .recv()
                .map_err(|_| RecvTimeoutError::Disconnected),
        };
        match next {
            Ok(RawWatchMessage::Paths(paths)) => {
                let now = Instant::now();
                for path in paths {
                    coalescer.push(path, now);
                }
            }
            Ok(RawWatchMessage::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
        for path in coalescer.drain_due(Instant::now()) {
            if message_sender.send(WatchMessage::Reload(path)).is_err() {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        path::PathBuf,
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    };

    fn watcher_tempdir() -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix("mdow-watcher-")
            .tempdir_in("/private/tmp")
            .unwrap()
    }

    fn receive_path(
        receiver: &Arc<Mutex<std::sync::mpsc::Receiver<WatchMessage>>>,
        timeout: Duration,
    ) -> Option<PathBuf> {
        receiver
            .lock()
            .unwrap()
            .recv_timeout(timeout)
            .ok()
            .map(|message| match message {
                WatchMessage::Reload(path) => path,
            })
    }

    fn wait_for_watcher_to_settle(receiver: &Arc<Mutex<std::sync::mpsc::Receiver<WatchMessage>>>) {
        while receive_path(receiver, DEBOUNCE * 2).is_some() {}
    }

    #[test]
    fn coalescer_debounces_each_path_independently() {
        let start = Instant::now();
        let first = PathBuf::from("/tmp/first.md");
        let second = PathBuf::from("/tmp/second.md");
        let mut coalescer = PathCoalescer::new(Duration::from_millis(150));

        coalescer.push(first.clone(), start);
        coalescer.push(second.clone(), start + Duration::from_millis(40));
        coalescer.push(first.clone(), start + Duration::from_millis(80));

        assert!(
            coalescer
                .drain_due(start + Duration::from_millis(149))
                .is_empty()
        );
        assert_eq!(
            coalescer.drain_due(start + Duration::from_millis(190)),
            vec![second]
        );
        assert_eq!(
            coalescer.drain_due(start + Duration::from_millis(230)),
            vec![first]
        );
    }

    #[test]
    fn watching_two_files_in_one_directory_registers_one_parent_watch() {
        let dir = watcher_tempdir();
        let first = dir.path().join("first.md");
        let second = dir.path().join("second.md");
        fs::write(&first, "# First").unwrap();
        fs::write(&second, "# Second").unwrap();
        let mut watcher = FileWatcher::new().unwrap();

        watcher.watch(&first).unwrap();
        watcher.watch(&second).unwrap();
        watcher.watch(&first).unwrap();

        assert_eq!(watcher.watched_parents.len(), 1);
        assert_eq!(watcher.watched_files.read().unwrap().len(), 2);
    }

    #[test]
    fn matching_watched_paths_ignores_unwatched_siblings() {
        let dir = watcher_tempdir();
        let logical_watched = dir.path().join("watched.md");
        let logical_sibling = dir.path().join("sibling.md");
        fs::write(&logical_watched, "# Watched").unwrap();
        fs::write(&logical_sibling, "# Sibling").unwrap();
        let watched = logical_watched.canonicalize().unwrap();
        let sibling = logical_sibling.canonicalize().unwrap();
        let watched_files = RwLock::new(HashSet::from([watched]));

        assert!(matching_watched_paths(&[sibling], &watched_files).is_empty());
    }

    #[test]
    fn repeated_write_events_produce_one_reload_after_the_trailing_debounce() {
        let dir = watcher_tempdir();
        let logical_path = dir.path().join("guide.md");
        fs::write(&logical_path, "# Before").unwrap();
        let path = logical_path.canonicalize().unwrap();
        let mut watcher = FileWatcher::new().unwrap();
        watcher.watch(&path).unwrap();
        let messages = watcher.messages();
        wait_for_watcher_to_settle(&messages);
        let started = Instant::now();

        fs::write(&path, "# First").unwrap();
        std::thread::sleep(Duration::from_millis(50));
        fs::write(&path, "# Second").unwrap();

        assert_eq!(
            receive_path(&messages, Duration::from_secs(2)),
            Some(path.canonicalize().unwrap())
        );
        assert!(started.elapsed() >= Duration::from_millis(140));
        assert_eq!(receive_path(&messages, Duration::from_millis(300)), None);
    }

    #[test]
    fn atomic_replacement_of_a_watched_file_produces_a_reload() {
        let dir = watcher_tempdir();
        let logical_path = dir.path().join("guide.md");
        fs::write(&logical_path, "# Before").unwrap();
        let path = logical_path.canonicalize().unwrap();
        let replacement = path.parent().unwrap().join(".guide.md.swp");
        let canonical = path.clone();
        let mut watcher = FileWatcher::new().unwrap();
        watcher.watch(&path).unwrap();
        let messages = watcher.messages();
        wait_for_watcher_to_settle(&messages);

        fs::write(&replacement, "# After").unwrap();
        fs::rename(&replacement, &path).unwrap();

        assert_eq!(
            receive_path(&messages, Duration::from_secs(2)),
            Some(canonical)
        );
    }
}
