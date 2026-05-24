use std::fs;
use std::path::Path;

use thiserror::Error;

use crate::models::{ScanResult, TreeNode};

const MD_EXTENSIONS: &[&str] = &[".md", ".markdown", ".mdx"];
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
pub const MAX_FILES: usize = 5000;
pub const MAX_DEPTH: u32 = 8;

#[derive(Debug, Error)]
pub enum FsError {
    #[error("failed to read directory: {0}")]
    ReadDir(#[from] std::io::Error),
}

struct ScanState {
    file_count: usize,
    truncated: bool,
}

fn is_md_file(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    MD_EXTENSIONS
        .iter()
        .any(|extension| name.ends_with(extension))
}

fn should_skip_entry(name: &str) -> bool {
    name.starts_with('.') || IGNORED_DIRS.contains(&name)
}

fn append_entry(
    nodes: &mut Vec<TreeNode>,
    entry_path: &Path,
    entry_name: &str,
    is_directory: bool,
    depth: u32,
    state: &mut ScanState,
) -> Result<(), FsError> {
    if state.truncated || should_skip_entry(entry_name) {
        return Ok(());
    }

    if is_directory {
        if depth >= MAX_DEPTH {
            return Ok(());
        }
        let children = scan_into(entry_path, depth + 1, state)?;
        if !children.is_empty() {
            nodes.push(TreeNode {
                name: entry_name.to_string(),
                path: entry_path.to_string_lossy().into_owned(),
                is_directory: true,
                children: Some(children),
            });
        }
        return Ok(());
    }

    if is_md_file(entry_name) {
        if state.file_count >= MAX_FILES {
            state.truncated = true;
            return Ok(());
        }
        state.file_count += 1;
        nodes.push(TreeNode {
            name: entry_name.to_string(),
            path: entry_path.to_string_lossy().into_owned(),
            is_directory: false,
            children: None,
        });
    }

    Ok(())
}

fn scan_into(
    folder_path: &Path,
    depth: u32,
    state: &mut ScanState,
) -> Result<Vec<TreeNode>, FsError> {
    if state.truncated {
        return Ok(vec![]);
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(folder_path)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        entries.push((
            entry.file_name().to_string_lossy().into_owned(),
            entry.path(),
            file_type.is_dir(),
        ));
    }

    entries.sort_by(
        |(left_name, _, left_is_dir), (right_name, _, right_is_dir)| match (
            left_is_dir,
            right_is_dir,
        ) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => left_name.cmp(right_name),
        },
    );

    let mut nodes = Vec::new();
    for (name, path, is_directory) in entries {
        append_entry(&mut nodes, &path, &name, is_directory, depth, state)?;
        if state.truncated {
            break;
        }
    }

    Ok(nodes)
}

pub fn scan_folder(folder_path: &Path) -> Result<ScanResult, FsError> {
    let mut state = ScanState {
        file_count: 0,
        truncated: false,
    };
    let tree = scan_into(folder_path, 0, &mut state)?;
    Ok(ScanResult {
        tree,
        truncated: state.truncated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static TEMP_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    fn temp_dir() -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let pid = std::process::id();
        let counter = TEMP_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("mdow-tree-test-{pid}-{counter}-{stamp}"));
        if path.exists() {
            fs::remove_dir_all(&path).expect("cleanup");
        }
        fs::create_dir_all(&path).expect("mkdir");
        path
    }

    fn write_file(path: &Path, content: &str) {
        let mut file = File::create(path).expect("create");
        file.write_all(content.as_bytes()).expect("write");
    }

    #[test]
    fn returns_empty_tree_for_empty_directory() {
        let dir = temp_dir();
        let result = scan_folder(&dir).expect("scan");
        assert!(result.tree.is_empty());
        assert!(!result.truncated);
    }

    #[test]
    fn finds_markdown_files() {
        let dir = temp_dir();
        write_file(&dir.join("readme.md"), "# Hello");
        write_file(&dir.join("notes.markdown"), "notes");
        write_file(&dir.join("doc.mdx"), "mdx content");

        let result = scan_folder(&dir).expect("scan");
        assert_eq!(result.tree.len(), 3);
        let mut names: Vec<_> = result.tree.iter().map(|node| node.name.clone()).collect();
        names.sort();
        assert_eq!(
            names,
            vec!["doc.mdx", "notes.markdown", "readme.md"]
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn ignores_non_markdown_files() {
        let dir = temp_dir();
        write_file(&dir.join("readme.md"), "# Hello");
        write_file(&dir.join("script.ts"), "code");
        write_file(&dir.join("style.css"), "css");

        let result = scan_folder(&dir).expect("scan");
        assert_eq!(result.tree.len(), 1);
        assert_eq!(result.tree[0].name, "readme.md");
    }

    #[test]
    fn ignores_hidden_files_and_directories() {
        let dir = temp_dir();
        write_file(&dir.join(".hidden.md"), "hidden");
        let hidden_dir = dir.join(".git");
        fs::create_dir_all(&hidden_dir).expect("mkdir");
        write_file(&hidden_dir.join("config.md"), "git config");
        write_file(&dir.join("visible.md"), "visible");

        let result = scan_folder(&dir).expect("scan");
        assert_eq!(result.tree.len(), 1);
        assert_eq!(result.tree[0].name, "visible.md");
    }

    #[test]
    fn skips_well_known_ignored_directories() {
        let dir = temp_dir();
        let node_modules = dir.join("node_modules").join("pkg");
        fs::create_dir_all(&node_modules).expect("mkdir");
        write_file(&node_modules.join("readme.md"), "pkg readme");
        let dist = dir.join("dist");
        fs::create_dir_all(&dist).expect("mkdir");
        write_file(&dist.join("out.md"), "built");
        write_file(&dir.join("visible.md"), "visible");

        let result = scan_folder(&dir).expect("scan");
        assert_eq!(result.tree.len(), 1);
        assert_eq!(result.tree[0].name, "visible.md");
    }

    #[test]
    fn scans_subdirectories_recursively() {
        let dir = temp_dir();
        let docs = dir.join("docs");
        fs::create_dir_all(&docs).expect("mkdir");
        write_file(&docs.join("guide.md"), "guide");
        write_file(&dir.join("readme.md"), "readme");

        let result = scan_folder(&dir).expect("scan");
        assert_eq!(result.tree.len(), 2);
        assert_eq!(result.tree[0].name, "docs");
        assert!(result.tree[0].is_directory);
        assert_eq!(result.tree[0].children.as_ref().unwrap().len(), 1);
        assert_eq!(
            result.tree[0].children.as_ref().unwrap()[0].name,
            "guide.md"
        );
        assert_eq!(result.tree[1].name, "readme.md");
        assert!(!result.tree[1].is_directory);
    }

    #[test]
    fn sorts_directories_before_files() {
        let dir = temp_dir();
        write_file(&dir.join("zebra.md"), "z");
        let alpha = dir.join("alpha");
        fs::create_dir_all(&alpha).expect("mkdir");
        write_file(&alpha.join("file.md"), "a");

        let result = scan_folder(&dir).expect("scan");
        assert!(result.tree[0].is_directory);
        assert_eq!(result.tree[0].name, "alpha");
        assert!(!result.tree[1].is_directory);
        assert_eq!(result.tree[1].name, "zebra.md");
    }

    #[test]
    fn sorts_alphabetically_within_same_type() {
        let dir = temp_dir();
        write_file(&dir.join("charlie.md"), "c");
        write_file(&dir.join("alpha.md"), "a");
        write_file(&dir.join("bravo.md"), "b");

        let result = scan_folder(&dir).expect("scan");
        assert_eq!(
            result
                .tree
                .iter()
                .map(|node| node.name.as_str())
                .collect::<Vec<_>>(),
            vec!["alpha.md", "bravo.md", "charlie.md"]
        );
    }

    #[test]
    fn excludes_empty_directories_without_markdown() {
        let dir = temp_dir();
        fs::create_dir_all(dir.join("empty")).expect("mkdir");
        let has_code = dir.join("has-code");
        fs::create_dir_all(&has_code).expect("mkdir");
        write_file(&has_code.join("script.ts"), "code");

        let result = scan_folder(&dir).expect("scan");
        assert!(result.tree.is_empty());
    }

    #[test]
    fn includes_directories_with_nested_markdown() {
        let dir = temp_dir();
        let outer = dir.join("outer").join("inner");
        fs::create_dir_all(&outer).expect("mkdir");
        write_file(&outer.join("deep.md"), "deep");

        let result = scan_folder(&dir).expect("scan");
        assert_eq!(result.tree.len(), 1);
        assert_eq!(result.tree[0].name, "outer");
        assert_eq!(result.tree[0].children.as_ref().unwrap()[0].name, "inner");
        assert_eq!(
            result.tree[0].children.as_ref().unwrap()[0]
                .children
                .as_ref()
                .unwrap()[0]
                .name,
            "deep.md"
        );
    }

    #[test]
    fn file_nodes_have_no_children_property() {
        let dir = temp_dir();
        write_file(&dir.join("file.md"), "content");

        let result = scan_folder(&dir).expect("scan");
        assert!(result.tree[0].children.is_none());
    }

    #[test]
    fn handles_case_insensitive_extensions() {
        let dir = temp_dir();
        write_file(&dir.join("upper.MD"), "upper");
        write_file(&dir.join("mixed.Markdown"), "mixed");

        let result = scan_folder(&dir).expect("scan");
        assert_eq!(result.tree.len(), 2);
    }

    #[test]
    fn does_not_recurse_past_max_depth() {
        let dir = temp_dir();
        let mut path = dir.clone();
        for index in 0..12 {
            path = path.join(format!("level{index}"));
            fs::create_dir_all(&path).expect("mkdir");
        }
        write_file(&path.join("deep.md"), "too deep");

        let result = scan_folder(&dir).expect("scan");
        assert!(result.tree.is_empty());
    }

    #[test]
    fn marks_result_truncated_at_file_cap() {
        let dir = temp_dir();
        for index in 0..5005 {
            write_file(&dir.join(format!("f{index}.md")), "");
        }

        let result = scan_folder(&dir).expect("scan");
        assert!(result.truncated);
        assert!(!result.tree.is_empty());
        assert!(result.tree.len() <= MAX_FILES);
    }
}
