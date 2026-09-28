use crate::document::is_supported_document;
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
    /// True when the scan stopped at `MAX_WORKSPACE_FILES`.
    pub truncated: bool,
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

    pub fn files(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        collect_files(&self.root, &mut files);
        files
    }
}

/// Mirrors the Electron folder scan caps so huge trees stay responsive.
pub const MAX_WORKSPACE_FILES: usize = 5000;
pub const MAX_WORKSPACE_DEPTH: usize = 8;

struct ScanState {
    visited: HashSet<PathBuf>,
    file_count: usize,
    truncated: bool,
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

    let mut state = ScanState {
        visited: HashSet::from([canonical_root.clone()]),
        file_count: 0,
        truncated: false,
    };
    let children = scan_directory(&canonical_root, &canonical_root, 0, &mut state)?;
    Ok(WorkspaceTree {
        root: WorkspaceEntry {
            name: display_name(&canonical_root),
            path: canonical_root,
            kind: WorkspaceEntryKind::Directory,
            children,
            expanded: true,
        },
        truncated: state.truncated,
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

/// Only a failure to list `directory` itself is an error; unreadable entries and nested
/// directories are skipped so one locked folder cannot hide the rest of the workspace.
fn scan_directory(
    workspace_root: &Path,
    directory: &Path,
    depth: usize,
    state: &mut ScanState,
) -> Result<Vec<WorkspaceEntry>, WorkspaceError> {
    let read_dir = fs::read_dir(directory).map_err(|error| WorkspaceError::Read {
        path: directory.to_owned(),
        message: error.to_string(),
    })?;
    let mut candidates = Vec::new();

    for entry in read_dir.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_ignored_name(&name) {
            continue;
        }
        let path = entry.path();
        let Ok(canonical_path) = path.canonicalize() else {
            continue;
        };
        if !canonical_path.starts_with(workspace_root) {
            continue;
        }
        let Ok(metadata) = fs::metadata(&canonical_path) else {
            continue;
        };
        if metadata.is_dir() {
            candidates.push((WorkspaceEntryKind::Directory, name, canonical_path));
        } else if metadata.is_file() && is_supported_document(&path) {
            candidates.push((WorkspaceEntryKind::File, name, canonical_path));
        }
    }
    // Visit in display order so the file cap keeps the entries a reader sees first.
    candidates.sort_by(|left, right| entry_order((left.0, &left.1), (right.0, &right.1)));

    let mut children = Vec::new();
    for (kind, name, path) in candidates {
        if state.truncated {
            break;
        }
        match kind {
            WorkspaceEntryKind::Directory => {
                if depth >= MAX_WORKSPACE_DEPTH || !state.visited.insert(path.clone()) {
                    continue;
                }
                let Ok(descendants) = scan_directory(workspace_root, &path, depth + 1, state)
                else {
                    continue;
                };
                if descendants.is_empty() {
                    continue;
                }
                children.push(WorkspaceEntry {
                    path,
                    name,
                    kind,
                    children: descendants,
                    expanded: false,
                });
            }
            WorkspaceEntryKind::File => {
                if state.file_count >= MAX_WORKSPACE_FILES {
                    state.truncated = true;
                    break;
                }
                state.file_count += 1;
                children.push(WorkspaceEntry {
                    path,
                    name,
                    kind,
                    children: Vec::new(),
                    expanded: false,
                });
            }
        }
    }
    Ok(children)
}

fn entry_order(
    (left_kind, left_name): (WorkspaceEntryKind, &str),
    (right_kind, right_name): (WorkspaceEntryKind, &str),
) -> std::cmp::Ordering {
    entry_kind_rank(left_kind)
        .cmp(&entry_kind_rank(right_kind))
        .then_with(|| left_name.to_lowercase().cmp(&right_name.to_lowercase()))
        .then_with(|| left_name.cmp(right_name))
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

fn collect_files(entry: &WorkspaceEntry, files: &mut Vec<PathBuf>) {
    if entry.kind == WorkspaceEntryKind::File {
        files.push(entry.path.clone());
    }
    for child in &entry.children {
        collect_files(child, files);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[cfg(unix)]
    struct PermissionRestore {
        path: PathBuf,
        mode: u32,
    }

    #[cfg(unix)]
    impl PermissionRestore {
        fn deny(path: &Path) -> Self {
            let mut permissions = fs::metadata(path).unwrap().permissions();
            let mode = permissions.mode();
            permissions.set_mode(0o000);
            fs::set_permissions(path, permissions).unwrap();
            Self {
                path: path.to_owned(),
                mode,
            }
        }
    }

    #[cfg(unix)]
    impl Drop for PermissionRestore {
        fn drop(&mut self) {
            let mut permissions = fs::metadata(&self.path).unwrap().permissions();
            permissions.set_mode(self.mode);
            fs::set_permissions(&self.path, permissions).unwrap();
        }
    }

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
        let files = tree.files();
        let mut file_names = files
            .iter()
            .filter_map(|path| path.file_name()?.to_str())
            .collect::<Vec<_>>();
        file_names.sort_unstable();
        assert_eq!(
            file_names,
            vec!["Alpha.md", "Guide.MARKDOWN", "start.md", "zeta.md"]
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

    #[cfg(unix)]
    #[test]
    fn skips_symlinks_that_escape_the_canonical_workspace_root() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("workspace");
        let outside = temp.path().join("outside");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(root.join("visible.md"), "# Visible").unwrap();
        fs::write(outside.join("secret.md"), "# Secret").unwrap();
        symlink(&outside, root.join("external-directory")).unwrap();
        symlink(outside.join("secret.md"), root.join("external-file.md")).unwrap();

        let tree = scan_workspace(&root).unwrap();
        let canonical_root = root.canonicalize().unwrap();

        assert_eq!(names(&tree.root.children), vec!["visible.md"]);
        assert!(
            tree.all_paths()
                .all(|path| path.starts_with(&canonical_root))
        );
    }

    #[cfg(unix)]
    #[test]
    fn skips_broken_symlinks_without_hiding_real_files() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join("visible.md"), "# Visible").unwrap();
        symlink(root.join("missing.md"), root.join("broken.md")).unwrap();

        let tree = scan_workspace(root).unwrap();

        assert_eq!(names(&tree.root.children), vec!["visible.md"]);
    }

    #[cfg(unix)]
    #[test]
    fn skips_unreadable_nested_directories_and_keeps_the_rest_of_the_tree() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let denied = root.join("denied");
        fs::create_dir(&denied).unwrap();
        fs::write(denied.join("hidden.md"), "# Hidden").unwrap();
        fs::write(root.join("visible.md"), "# Visible").unwrap();
        let _restore = PermissionRestore::deny(&denied);

        let tree = scan_workspace(root).unwrap();

        assert_eq!(names(&tree.root.children), vec!["visible.md"]);
        assert!(!tree.truncated);
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_root_is_still_an_error() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        fs::create_dir(&root).unwrap();
        let canonical_root = root.canonicalize().unwrap();
        let _restore = PermissionRestore::deny(&root);

        assert!(matches!(
            scan_workspace(&root),
            Err(WorkspaceError::Read { path, .. }) if path == canonical_root
        ));
    }

    #[test]
    fn stops_descending_past_the_depth_cap() {
        let temp = tempfile::tempdir().unwrap();
        let mut deepest_kept = temp.path().to_owned();
        for level in 0..MAX_WORKSPACE_DEPTH {
            deepest_kept.push(format!("level{level}"));
        }
        let too_deep = deepest_kept.join("one-more");
        fs::create_dir_all(&too_deep).unwrap();
        fs::write(deepest_kept.join("kept.md"), "# Kept").unwrap();
        fs::write(too_deep.join("pruned.md"), "# Pruned").unwrap();

        let tree = scan_workspace(temp.path()).unwrap();
        let files = tree.files();

        assert!(files.iter().any(|path| path.ends_with("kept.md")));
        assert!(!files.iter().any(|path| path.ends_with("pruned.md")));
    }

    #[test]
    fn marks_the_tree_truncated_once_the_file_cap_is_hit() {
        let temp = tempfile::tempdir().unwrap();
        for index in 0..MAX_WORKSPACE_FILES + 5 {
            fs::write(temp.path().join(format!("f{index:05}.md")), "").unwrap();
        }

        let tree = scan_workspace(temp.path()).unwrap();

        assert!(tree.truncated);
        assert_eq!(tree.files().len(), MAX_WORKSPACE_FILES);
        assert_eq!(tree.root.children[0].name, "f00000.md");
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
