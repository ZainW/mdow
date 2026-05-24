use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::{AppHandle, Emitter};

use crate::fs_tree;
use crate::models::FileChangedPayload;

const FILE_DEBOUNCE_MS: u64 = 300;
const FOLDER_DEBOUNCE_MS: u64 = 1000;

const IGNORED_DIRS: &[&str] = &[
    "node_modules",
    "dist",
    "out",
    "build",
    "target",
    ".next",
    ".turbo",
    "coverage",
];

pub struct WatcherHub {
    file_watchers: Mutex<HashMap<String, FileWatchHandle>>,
    folder_watcher: Mutex<Option<FolderWatchHandle>>,
}

impl Default for WatcherHub {
    fn default() -> Self {
        Self {
            file_watchers: Mutex::new(HashMap::new()),
            folder_watcher: Mutex::new(None),
        }
    }
}

struct FileWatchHandle {
    _watcher: RecommendedWatcher,
    _generation: Arc<AtomicU64>,
}

struct FolderWatchHandle {
    _watcher: RecommendedWatcher,
    _generation: Arc<AtomicU64>,
}

impl WatcherHub {
    pub fn watch_file(&self, app: AppHandle, path: PathBuf) {
        let path_key = path.to_string_lossy().into_owned();
        self.unwatch_file(&path_key);

        let generation = Arc::new(AtomicU64::new(0));
        let event_generation = Arc::clone(&generation);
        let app_for_events = app.clone();
        let watched_path = path.clone();

        let delete_path = path_key.clone();
        let watcher = RecommendedWatcher::new(
            move |result: notify::Result<notify::Event>| {
                let Ok(event) = result else {
                    return;
                };

                match event.kind {
                    EventKind::Modify(_) | EventKind::Create(_) => {
                        schedule_file_change(
                            Arc::clone(&event_generation),
                            app_for_events.clone(),
                            watched_path.clone(),
                        );
                    }
                    EventKind::Remove(_) => {
                        let _ = app_for_events.emit("file:deleted", delete_path.clone());
                    }
                    _ => {}
                }
            },
            Config::default(),
        );

        let mut watcher = watcher.unwrap_or_else(|err| {
            panic!("failed to create file watcher for {path_key}: {err}");
        });

        if watcher.watch(&path, RecursiveMode::NonRecursive).is_err() {
            return;
        }

        self.file_watchers
            .lock()
            .expect("file watchers mutex poisoned")
            .insert(
                path_key,
                FileWatchHandle {
                    _watcher: watcher,
                    _generation: generation,
                },
            );
    }

    pub fn unwatch_file(&self, path: &str) {
        self.file_watchers
            .lock()
            .expect("file watchers mutex poisoned")
            .remove(path);
    }

    pub fn watch_folder(&self, app: AppHandle, path: PathBuf) {
        self.unwatch_folder();

        let generation = Arc::new(AtomicU64::new(0));
        let event_generation = Arc::clone(&generation);
        let app_for_events = app.clone();
        let watched_path = path.clone();

        let watcher = RecommendedWatcher::new(
            move |result: notify::Result<notify::Event>| {
                let Ok(event) = result else {
                    return;
                };

                if !folder_event_is_relevant(&event) {
                    return;
                }

                schedule_folder_rescan(
                    Arc::clone(&event_generation),
                    app_for_events.clone(),
                    watched_path.clone(),
                );
            },
            Config::default(),
        );

        let mut watcher = watcher.unwrap_or_else(|err| {
            panic!(
                "failed to create folder watcher for {}: {err}",
                path.display()
            );
        });

        if watcher.watch(&path, RecursiveMode::Recursive).is_err() {
            return;
        }

        *self
            .folder_watcher
            .lock()
            .expect("folder watcher mutex poisoned") = Some(FolderWatchHandle {
            _watcher: watcher,
            _generation: generation,
        });
    }

    pub fn unwatch_folder(&self) {
        *self
            .folder_watcher
            .lock()
            .expect("folder watcher mutex poisoned") = None;
    }
}

fn schedule_file_change(generation: Arc<AtomicU64>, app: AppHandle, path: PathBuf) {
    let token = generation.fetch_add(1, Ordering::SeqCst) + 1;
    let path_string = path.to_string_lossy().into_owned();

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(FILE_DEBOUNCE_MS));
        if generation.load(Ordering::SeqCst) != token {
            return;
        }

        match fs::read_to_string(&path) {
            Ok(content) => {
                let payload = FileChangedPayload {
                    path: path_string,
                    content,
                };
                let _ = app.emit("file:changed", payload);
            }
            Err(_) => {
                // File might be temporarily unavailable during save.
            }
        }
    });
}

fn schedule_folder_rescan(generation: Arc<AtomicU64>, app: AppHandle, folder_path: PathBuf) {
    let token = generation.fetch_add(1, Ordering::SeqCst) + 1;

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(FOLDER_DEBOUNCE_MS));
        if generation.load(Ordering::SeqCst) != token {
            return;
        }

        match fs_tree::scan_folder(&folder_path) {
            Ok(scan) => {
                let _ = app.emit("folder:changed", scan);
            }
            Err(_) => {
                // Folder might have been deleted.
            }
        }
    });
}

fn folder_event_is_relevant(event: &notify::Event) -> bool {
    event.paths.iter().any(|path| {
        if is_markdown_path(path) {
            return true;
        }

        if !path.is_dir() {
            return false;
        }

        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| !name.starts_with('.') && !IGNORED_DIRS.contains(&name))
            .unwrap_or(false)
    })
}

fn is_markdown_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            lower.ends_with(".md") || lower.ends_with(".markdown") || lower.ends_with(".mdx")
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_path_detection_is_case_insensitive() {
        assert!(is_markdown_path(Path::new("/tmp/readme.MD")));
        assert!(is_markdown_path(Path::new("/tmp/notes.Markdown")));
        assert!(!is_markdown_path(Path::new("/tmp/script.ts")));
    }
}
