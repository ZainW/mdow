//! Snapshot of what was open. Produced from live state or disk, never synced field by field.

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SavedWindowBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionTabs {
    pub before: Vec<PathBuf>,
    pub active: PathBuf,
    pub after: Vec<PathBuf>,
}

impl SessionTabs {
    pub fn new(paths: Vec<PathBuf>, active: Option<PathBuf>) -> Option<Self> {
        if paths.is_empty() {
            return None;
        }
        let active_idx = active
            .as_ref()
            .and_then(|active| paths.iter().position(|path| path == active))
            .unwrap_or(paths.len() - 1);
        let mut paths = paths;
        let after = paths.split_off(active_idx + 1);
        let active = paths.pop()?;
        Some(Self {
            before: paths,
            active,
            after,
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &Path> {
        self.before
            .iter()
            .map(PathBuf::as_path)
            .chain(std::iter::once(self.active.as_path()))
            .chain(self.after.iter().map(PathBuf::as_path))
    }

    pub fn active(&self) -> &Path {
        &self.active
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Recents(Vec<PathBuf>);

impl Recents {
    pub const CAP: usize = 20;

    pub fn note(&mut self, path: &Path) -> bool {
        if self.0.first().is_some_and(|first| first == path) {
            return false;
        }
        self.0.retain(|existing| existing != path);
        self.0.insert(0, path.to_owned());
        self.0.truncate(Self::CAP);
        true
    }

    pub fn iter(&self) -> impl Iterator<Item = &Path> {
        self.0.iter().map(PathBuf::as_path)
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn from_paths(paths: Vec<PathBuf>) -> Self {
        let mut recents = Vec::new();
        for path in paths {
            if recents.iter().any(|existing| existing == &path) {
                continue;
            }
            recents.push(path);
            if recents.len() == Self::CAP {
                break;
            }
        }
        Self(recents)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Session {
    pub tabs: Option<SessionTabs>,
    pub last_folder: Option<PathBuf>,
    pub recents: Recents,
    pub window: Option<SavedWindowBounds>,
}

impl Session {
    pub fn from_parts(
        tab_paths: impl IntoIterator<Item = PathBuf>,
        active: Option<PathBuf>,
        last_folder: Option<PathBuf>,
        recents: Recents,
        window: Option<SavedWindowBounds>,
    ) -> Self {
        Self {
            tabs: SessionTabs::new(tab_paths.into_iter().collect(), active),
            last_folder,
            recents,
            window,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_active_path_falls_back_to_the_last_tab() {
        let tabs = SessionTabs::new(
            vec![
                PathBuf::from("/a.md"),
                PathBuf::from("/b.md"),
                PathBuf::from("/c.md"),
            ],
            Some(PathBuf::from("/missing.md")),
        )
        .unwrap();

        assert_eq!(tabs.active(), Path::new("/c.md"));
        assert_eq!(
            tabs.iter().collect::<Vec<_>>(),
            vec![Path::new("/a.md"), Path::new("/b.md"), Path::new("/c.md")]
        );
    }

    #[test]
    fn recents_treat_macos_var_and_private_var_as_one_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("showcase.md");
        std::fs::write(&file, "# Showcase").unwrap();
        let canonical = file.canonicalize().unwrap();
        if file == canonical {
            return;
        }

        let mut recents = Recents::from_paths(vec![file.clone()]);
        recents.note(&canonical);
        let listed: Vec<_> = recents.iter().collect();
        assert_eq!(listed, vec![canonical.as_path()]);
    }

    #[test]
    fn recents_treat_symlink_and_canonical_path_as_one_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("showcase.md");
        std::fs::write(&file, "# Showcase").unwrap();
        let link = dir.path().join("also-showcase.md");
        std::os::unix::fs::symlink(&file, &link).unwrap();
        let canonical = file.canonicalize().unwrap();
        assert_ne!(link.as_path(), canonical.as_path());

        let mut recents = Recents::from_paths(vec![link.clone()]);
        recents.note(&canonical);
        let listed: Vec<_> = recents.iter().collect();
        assert_eq!(listed, vec![canonical.as_path()]);
    }

    #[test]
    fn recents_move_to_front_dedupe_and_cap() {
        let mut recents = Recents::default();
        assert!(recents.note(Path::new("/a.md")));
        assert!(recents.note(Path::new("/b.md")));
        assert!(!recents.note(Path::new("/b.md")));
        assert!(recents.note(Path::new("/a.md")));
        assert_eq!(
            recents.iter().collect::<Vec<_>>(),
            vec![Path::new("/a.md"), Path::new("/b.md")]
        );

        let overflow: Vec<PathBuf> = (0..30).map(|i| PathBuf::from(format!("/{i}.md"))).collect();
        let capped = Recents::from_paths(overflow);
        assert_eq!(capped.iter().count(), Recents::CAP);
        assert_eq!(capped.iter().next(), Some(Path::new("/0.md")));
    }

    #[test]
    fn empty_tab_list_is_no_session_tabs() {
        assert!(SessionTabs::new(Vec::new(), None).is_none());
    }
}
