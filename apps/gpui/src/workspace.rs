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
    let children = scan_directory(&canonical_root, &canonical_root, &mut visited)?;
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
    workspace_root: &Path,
    directory: &Path,
    visited: &mut HashSet<PathBuf>,
) -> Result<Vec<WorkspaceEntry>, WorkspaceError> {
    let read_dir = fs::read_dir(directory).map_err(|error| WorkspaceError::Read {
        path: directory.to_owned(),
        message: error.to_string(),
    })?;
    let mut children = Vec::new();

    for entry in read_dir {
        let entry = entry.map_err(|error| WorkspaceError::Read {
            path: directory.to_owned(),
            message: error.to_string(),
        })?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if is_ignored_name(&name) {
            continue;
        }

        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| WorkspaceError::Read {
            path: path.clone(),
            message: error.to_string(),
        })?;
        let canonical_path = match path.canonicalize() {
            Ok(canonical_path) => canonical_path,
            Err(error)
                if file_type.is_symlink() && error.kind() == std::io::ErrorKind::NotFound =>
            {
                continue;
            }
            Err(error) => {
                return Err(WorkspaceError::Read {
                    path,
                    message: error.to_string(),
                });
            }
        };
        if !canonical_path.starts_with(workspace_root) {
            continue;
        }
        let metadata = match fs::metadata(&canonical_path) {
            Ok(metadata) => metadata,
            Err(error) => {
                return Err(WorkspaceError::Read {
                    path: canonical_path,
                    message: error.to_string(),
                });
            }
        };

        if metadata.is_dir() {
            if !visited.insert(canonical_path.clone()) {
                continue;
            }
            let descendants = scan_directory(workspace_root, &canonical_path, visited)?;
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
        } else if metadata.is_file() && is_supported_document(&path) {
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
    Ok(children)
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
    fn reports_the_failing_nested_directory_instead_of_returning_a_partial_tree() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let denied = root.join("denied");
        fs::create_dir(&denied).unwrap();
        fs::write(denied.join("hidden.md"), "# Hidden").unwrap();
        fs::write(root.join("visible.md"), "# Visible").unwrap();
        let canonical_denied = denied.canonicalize().unwrap();
        let _restore = PermissionRestore::deny(&denied);

        let error = scan_workspace(root).unwrap_err();

        assert!(matches!(
            error,
            WorkspaceError::Read { path, .. } if path == canonical_denied
        ));
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
