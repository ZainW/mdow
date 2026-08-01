use crate::document::is_supported_markdown;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceEntryKind {
    Directory,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceEntry {
    pub path: PathBuf,
    pub name: String,
    pub kind: WorkspaceEntryKind,
    pub children: Vec<WorkspaceEntry>,
    pub expanded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRow {
    pub path: PathBuf,
    pub name: String,
    pub kind: WorkspaceEntryKind,
    pub depth: usize,
    pub expanded: bool,
    pub has_children: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceTree {
    pub root: WorkspaceEntry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceError {
    Missing { path: PathBuf },
    NotDirectory { path: PathBuf },
    Read { path: PathBuf, message: String },
}

impl WorkspaceError {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Missing { .. } => "Folder not found",
            Self::NotDirectory { .. } => "This is not a folder",
            Self::Read { .. } => "Couldn't read folder",
        }
    }

    pub fn body(&self) -> &'static str {
        match self {
            Self::Missing { .. } => "This folder may have been moved or renamed.",
            Self::NotDirectory { .. } => "Choose a folder to show its Markdown files.",
            Self::Read { .. } => {
                "Mdow could not read this folder. Check that you have permission to access it."
            }
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            Self::Missing { path } | Self::NotDirectory { path } | Self::Read { path, .. } => path,
        }
    }
}

impl WorkspaceTree {
    pub fn toggle_directory(&mut self, path: &Path) -> bool {
        let path = path_identity(path);
        toggle_entry(&mut self.root, &path)
    }

    pub fn visible_rows(&self) -> Vec<WorkspaceRow> {
        let mut rows = Vec::new();
        collect_visible_rows(&self.root, 0, &mut rows);
        rows
    }

    pub fn all_paths(&self) -> impl Iterator<Item = &Path> {
        let mut entries = Vec::new();
        collect_entries(&self.root, &mut entries);
        entries.into_iter().map(|entry| entry.path.as_path())
    }
}

pub fn scan_workspace(root: &Path) -> Result<WorkspaceTree, WorkspaceError> {
    let canonical_root = canonicalize_root(root)?;
    let metadata = fs::metadata(&canonical_root).map_err(|error| WorkspaceError::Read {
        path: root.to_owned(),
        message: error.to_string(),
    })?;
    if !metadata.is_dir() {
        return Err(WorkspaceError::NotDirectory {
            path: root.to_owned(),
        });
    }

    let mut visited = HashSet::new();
    visited.insert(canonical_root.clone());
    let children = scan_directory(&canonical_root, &mut visited, true)?.unwrap_or_default();
    Ok(WorkspaceTree {
        root: WorkspaceEntry {
            name: display_name(&canonical_root),
            path: canonical_root,
            kind: WorkspaceEntryKind::Directory,
            children,
            expanded: true,
        },
    })
}

fn canonicalize_root(root: &Path) -> Result<PathBuf, WorkspaceError> {
    root.canonicalize().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            WorkspaceError::Missing {
                path: root.to_owned(),
            }
        } else {
            WorkspaceError::Read {
                path: root.to_owned(),
                message: error.to_string(),
            }
        }
    })
}

fn scan_directory(
    directory: &Path,
    visited: &mut HashSet<PathBuf>,
    is_root: bool,
) -> Result<Option<Vec<WorkspaceEntry>>, WorkspaceError> {
    let read_dir = match fs::read_dir(directory) {
        Ok(read_dir) => read_dir,
        Err(error) if is_root => {
            return Err(WorkspaceError::Read {
                path: directory.to_owned(),
                message: error.to_string(),
            });
        }
        Err(_) => return Ok(None),
    };
    let mut children = Vec::new();

    for entry in read_dir.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_ignored_name(&name) {
            continue;
        }

        let path = entry.path();
        let Ok(metadata) = fs::metadata(&path) else {
            continue;
        };
        let Ok(canonical_path) = path.canonicalize() else {
            continue;
        };

        if metadata.is_dir() {
            if !visited.insert(canonical_path.clone()) {
                continue;
            }
            let Some(descendants) = scan_directory(&canonical_path, visited, false)? else {
                continue;
            };
            if descendants.is_empty() {
                continue;
            }
            children.push(WorkspaceEntry {
                path: canonical_path,
                name,
                kind: WorkspaceEntryKind::Directory,
                children: descendants,
                expanded: false,
            });
        } else if metadata.is_file() && is_supported_markdown(&path) {
            children.push(WorkspaceEntry {
                path: canonical_path,
                name,
                kind: WorkspaceEntryKind::File,
                children: Vec::new(),
                expanded: false,
            });
        }
    }

    children.sort_by(|left, right| {
        entry_kind_rank(left.kind)
            .cmp(&entry_kind_rank(right.kind))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(Some(children))
}

fn is_ignored_name(name: &str) -> bool {
    name.starts_with('.') || matches!(name, ".git" | "node_modules" | "target" | "dist" | "build")
}

fn entry_kind_rank(kind: WorkspaceEntryKind) -> u8 {
    match kind {
        WorkspaceEntryKind::Directory => 0,
        WorkspaceEntryKind::File => 1,
    }
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn path_identity(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_owned())
}

fn toggle_entry(entry: &mut WorkspaceEntry, path: &Path) -> bool {
    if entry.path == path {
        if entry.kind == WorkspaceEntryKind::Directory {
            entry.expanded = !entry.expanded;
            return true;
        }
        return false;
    }
    entry
        .children
        .iter_mut()
        .any(|child| toggle_entry(child, path))
}

fn collect_visible_rows(entry: &WorkspaceEntry, depth: usize, rows: &mut Vec<WorkspaceRow>) {
    for child in &entry.children {
        rows.push(WorkspaceRow {
            path: child.path.clone(),
            name: child.name.clone(),
            kind: child.kind,
            depth,
            expanded: child.expanded,
            has_children: !child.children.is_empty(),
        });
        if child.kind == WorkspaceEntryKind::Directory && child.expanded {
            collect_visible_rows(child, depth + 1, rows);
        }
    }
}

fn collect_entries<'a>(entry: &'a WorkspaceEntry, entries: &mut Vec<&'a WorkspaceEntry>) {
    entries.push(entry);
    for child in &entry.children {
        collect_entries(child, entries);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn names(entries: &[WorkspaceEntry]) -> Vec<&str> {
        entries.iter().map(|entry| entry.name.as_str()).collect()
    }

    #[test]
    fn scans_supported_files_in_sorted_visible_directories() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir(root.join("guides")).unwrap();
        fs::create_dir(root.join("guides/nested")).unwrap();
        fs::create_dir(root.join(".git")).unwrap();
        fs::create_dir(root.join("node_modules")).unwrap();
        fs::create_dir(root.join("target")).unwrap();
        fs::write(root.join("Alpha.md"), "# Alpha").unwrap();
        fs::write(root.join("zeta.md"), "# Zeta").unwrap();
        fs::write(root.join("notes.txt"), "not markdown").unwrap();
        fs::write(root.join(".hidden.md"), "# Hidden").unwrap();
        fs::write(root.join("guides/start.md"), "# Start").unwrap();
        fs::write(root.join("guides/nested/Guide.MARKDOWN"), "# Guide").unwrap();
        fs::write(root.join(".git/hidden.md"), "# Hidden").unwrap();
        fs::write(root.join("node_modules/hidden.md"), "# Hidden").unwrap();
        fs::write(root.join("target/hidden.md"), "# Hidden").unwrap();

        let mut tree = scan_workspace(root).unwrap();

        assert_eq!(tree.root.path, root.canonicalize().unwrap());
        assert_eq!(
            names(&tree.root.children),
            vec!["guides", "Alpha.md", "zeta.md"]
        );
        assert!(
            !tree
                .all_paths()
                .any(|path| path.ends_with("node_modules/hidden.md"))
        );
        assert!(!tree.all_paths().any(|path| path.ends_with(".hidden.md")));
        assert!(!tree.visible_rows().iter().any(|row| row.name == "start.md"));

        assert!(tree.toggle_directory(&root.join("guides")));
        assert!(
            tree.visible_rows()
                .iter()
                .any(|row| row.name == "start.md" && row.depth == 1)
        );
    }

    #[cfg(unix)]
    #[test]
    fn skips_directory_symlink_cycles_without_losing_real_descendants() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir(root.join("guides")).unwrap();
        fs::write(root.join("guides/start.md"), "# Start").unwrap();
        symlink(root, root.join("guides/loop")).unwrap();

        let tree = scan_workspace(root).unwrap();

        assert!(
            tree.all_paths()
                .any(|path| path.ends_with("guides/start.md"))
        );
        assert!(!tree.all_paths().any(|path| path.ends_with("loop")));
    }

    #[test]
    fn rejects_missing_and_non_directory_roots() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("file.md");
        fs::write(&file, "# File").unwrap();

        assert!(matches!(
            scan_workspace(&temp.path().join("missing")),
            Err(WorkspaceError::Missing { .. })
        ));
        assert!(matches!(
            scan_workspace(&file),
            Err(WorkspaceError::NotDirectory { .. })
        ));
    }
}
