//! Reading documents, scanning folders, and resolving links — the desktop's file and folder
//! services (`apps/desktop/src/main/file-service.ts`, `folder-service.ts`).

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

pub const MD_EXTENSIONS: [&str; 3] = ["md", "markdown", "mdx"];
pub const HTML_EXTENSIONS: [&str; 2] = ["html", "htm"];

fn extension(path: &Path) -> Option<String> {
    path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase())
}

pub fn is_markdown(path: &Path) -> bool {
    extension(path).is_some_and(|e| MD_EXTENSIONS.contains(&e.as_str()))
}

pub fn is_html(path: &Path) -> bool {
    extension(path).is_some_and(|e| HTML_EXTENSIONS.contains(&e.as_str()))
}

pub fn is_document(path: &Path) -> bool {
    is_markdown(path) || is_html(path)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    NotFound,
    PermissionDenied,
    Deleted,
    ReadError,
}

impl ErrorKind {
    pub fn title(self) -> &'static str {
        match self {
            ErrorKind::NotFound => "File not found",
            ErrorKind::Deleted => "File moved or deleted",
            ErrorKind::PermissionDenied => "Access denied",
            ErrorKind::ReadError => "Couldn't read file",
        }
    }
    pub fn body(self) -> &'static str {
        match self {
            ErrorKind::NotFound => {
                "This file seems to have wandered off. It may have been moved or renamed."
            }
            ErrorKind::Deleted => {
                "This file was here a moment ago. Someone (or something) must have moved it."
            }
            ErrorKind::PermissionDenied => {
                "You don't have permission to read this file. Check the file permissions and try again."
            }
            ErrorKind::ReadError => {
                "Something went wrong trying to read this file. It might be corrupted or locked by another process."
            }
        }
    }
    pub fn icon(self) -> &'static str {
        match self {
            ErrorKind::NotFound => "action-unavailable-symbolic",
            ErrorKind::Deleted => "user-trash-symbolic",
            ErrorKind::PermissionDenied => "dialog-password-symbolic",
            ErrorKind::ReadError => "dialog-error-symbolic",
        }
    }
}

pub fn read_document(path: &Path) -> Result<String, ErrorKind> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(err) => String::from_utf8_lossy(err.as_bytes()).into_owned(),
        }),
        Err(err) => Err(match err.kind() {
            std::io::ErrorKind::NotFound => ErrorKind::NotFound,
            std::io::ErrorKind::PermissionDenied => ErrorKind::PermissionDenied,
            _ if path.is_dir() => ErrorKind::ReadError,
            _ => ErrorKind::ReadError,
        }),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeNode {
    pub name: String,
    pub path: PathBuf,
    pub children: Option<Vec<TreeNode>>,
}

impl TreeNode {
    pub fn is_dir(&self) -> bool {
        self.children.is_some()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scan {
    pub tree: Vec<TreeNode>,
    pub truncated: bool,
    /// Every directory visited, for change monitoring.
    pub dirs: Vec<PathBuf>,
}

const IGNORED_DIRS: [&str; 8] =
    ["node_modules", "dist", "out", "build", "target", ".next", ".turbo", "coverage"];
pub const MAX_FILES: usize = 5000;
const MAX_DEPTH: usize = 8;

fn skip(name: &str) -> bool {
    name.starts_with('.') || IGNORED_DIRS.contains(&name)
}

/// Natural-ish, case-insensitive name order (like `localeCompare`).
pub fn compare_names(a: &str, b: &str) -> Ordering {
    let la = a.to_lowercase();
    let lb = b.to_lowercase();
    la.cmp(&lb).then_with(|| a.cmp(b))
}

/// Documents under `root`, folders first, empty folders dropped, capped at `MAX_FILES`.
pub fn scan_folder(root: &Path) -> Scan {
    let mut scan = Scan::default();
    let mut count = 0;
    scan.tree = scan_into(root, 0, &mut count, &mut scan.truncated, &mut scan.dirs);
    scan
}

fn scan_into(
    dir: &Path,
    depth: usize,
    count: &mut usize,
    truncated: &mut bool,
    dirs: &mut Vec<PathBuf>,
) -> Vec<TreeNode> {
    if *truncated {
        return Vec::new();
    }
    dirs.push(dir.to_path_buf());
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut entries: Vec<(String, PathBuf, bool)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if skip(&name) {
                return None;
            }
            let path = entry.path();
            // Follow symlinks for the kind, as readdir+stat would.
            let is_dir = std::fs::metadata(&path).map(|m| m.is_dir()).unwrap_or(false);
            Some((name, path, is_dir))
        })
        .collect();
    entries.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| compare_names(&a.0, &b.0)));
    let mut nodes = Vec::new();
    for (name, path, is_dir) in entries {
        if *truncated {
            break;
        }
        if is_dir {
            if depth >= MAX_DEPTH {
                continue;
            }
            let children = scan_into(&path, depth + 1, count, truncated, dirs);
            if !children.is_empty() {
                nodes.push(TreeNode { name, path, children: Some(children) });
            }
        } else if is_document(&path) {
            if *count >= MAX_FILES {
                *truncated = true;
                break;
            }
            *count += 1;
            nodes.push(TreeNode { name, path, children: None });
        }
    }
    nodes
}

pub fn flatten_files(nodes: &[TreeNode], out: &mut Vec<PathBuf>) {
    for node in nodes {
        match &node.children {
            Some(children) => flatten_files(children, out),
            None => out.push(node.path.clone()),
        }
    }
}

/// Keep files whose path relative to the root, or name, contains the query; their folders stay.
pub fn filter_tree(nodes: &[TreeNode], root: &Path, query: &str) -> Vec<TreeNode> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return nodes.to_vec();
    }
    fn walk(nodes: &[TreeNode], root: &Path, q: &str) -> Vec<TreeNode> {
        nodes
            .iter()
            .filter_map(|node| match &node.children {
                Some(children) => {
                    let kept = walk(children, root, q);
                    (!kept.is_empty()).then(|| TreeNode { children: Some(kept), ..node.clone() })
                }
                None => {
                    let rel = node.path.strip_prefix(root).unwrap_or(&node.path);
                    let hit = rel.to_string_lossy().to_lowercase().contains(q)
                        || node.name.to_lowercase().contains(q);
                    hit.then(|| node.clone())
                }
            })
            .collect()
    }
    walk(nodes, root, &q)
}

pub fn count_files(nodes: &[TreeNode]) -> usize {
    nodes
        .iter()
        .map(|n| match &n.children {
            Some(c) => count_files(c),
            None => 1,
        })
        .sum()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkTarget {
    /// `#heading` in the same document.
    Anchor(String),
    /// A web (or other scheme) URL for the system handler.
    External(String),
    /// Another markdown or HTML document, opened in a tab; with an optional `#anchor`.
    Document(PathBuf, Option<String>),
    /// Any other local file, opened with its default application.
    File(PathBuf),
}

fn has_scheme(href: &str) -> bool {
    let mut chars = href.chars();
    let Some(first) = chars.next() else { return false };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    for c in chars {
        if c == ':' {
            return true;
        }
        if !(c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
            return false;
        }
    }
    false
}

/// What clicking `href` inside the document at `doc_path` should do.
pub fn resolve_link(href: &str, doc_path: &Path) -> LinkTarget {
    let href = href.trim();
    if let Some(anchor) = href.strip_prefix('#') {
        return LinkTarget::Anchor(crate::doc::percent_decode(anchor));
    }
    let local = if let Some(rest) = href.strip_prefix("file://") {
        rest.to_string()
    } else if has_scheme(href) {
        return LinkTarget::External(href.to_string());
    } else {
        href.to_string()
    };
    let (path_part, anchor) = match local.split_once('#') {
        Some((p, a)) => (p.to_string(), Some(crate::doc::percent_decode(a))),
        None => (local.clone(), None),
    };
    let path_part = path_part.split('?').next().unwrap_or("").to_string();
    let decoded = crate::doc::percent_decode(&path_part);
    let base = doc_path.parent().unwrap_or(Path::new("/"));
    let joined = if Path::new(&decoded).is_absolute() {
        PathBuf::from(&decoded)
    } else {
        base.join(&decoded)
    };
    let path = normalize(&joined);
    if is_document(&path) {
        LinkTarget::Document(path, anchor.filter(|a| !a.is_empty()))
    } else {
        LinkTarget::File(path)
    }
}

/// Lexically resolve `.` and `..`.
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

pub fn display_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// The parent folder for display, with the home directory as `~`.
pub fn display_dir(path: &Path) -> String {
    let Some(parent) = path.parent() else { return String::new() };
    let home = gtk::glib::home_dir();
    match parent.strip_prefix(&home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => parent.display().to_string(),
    }
}

/// Shorten the middle of a long path, as the desktop's `truncatePathMiddle`.
pub fn truncate_middle(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_string();
    }
    let keep = max.saturating_sub(1);
    let head = keep / 2;
    let tail = keep - head;
    let mut out: String = chars[..head].iter().collect();
    out.push('…');
    out.extend(&chars[chars.len() - tail..]);
    out
}

/// Breadcrumb segments: the folders above a file, from the open folder if it contains the file,
/// otherwise the last three.
pub fn breadcrumb_segments(file: &Path, root: Option<&Path>) -> Vec<(String, PathBuf)> {
    let Some(parent) = file.parent() else { return Vec::new() };
    let dirs: Vec<PathBuf> =
        parent.ancestors().map(Path::to_path_buf).collect::<Vec<_>>().into_iter().rev().collect();
    // `dirs` runs from `/` down to the parent; drop the bare root.
    let dirs: Vec<PathBuf> = dirs.into_iter().filter(|d| d.file_name().is_some()).collect();
    let mut start = dirs.len().saturating_sub(3);
    if let Some(root_name) = root.and_then(|r| r.file_name())
        && let Some(idx) = dirs.iter().rposition(|d| d.file_name() == Some(root_name))
    {
        start = idx;
    }
    dirs[start..].iter().map(|d| (display_name(d), d.clone())).collect()
}

/// The desktop palette's file search: subsequence score with bonuses for runs, word starts and a
/// matching first character (`apps/desktop/src/renderer/src/lib/fuzzy-search.ts`).
pub fn fuzzy_score(query: &str, target: &str) -> f64 {
    let q: Vec<char> = query.to_lowercase().chars().collect();
    let t: Vec<char> = target.to_lowercase().chars().collect();
    let mut score = 0.0;
    let mut qi = 0;
    let mut consecutive = 0.0;
    for i in 0..t.len() {
        if qi >= q.len() {
            break;
        }
        if t[i] == q[qi] {
            score += 1.0;
            if consecutive > 0.0 {
                score += consecutive * 2.0;
            }
            consecutive += 1.0;
            if i == 0 || matches!(t[i - 1], '/' | '-' | '_' | '.') {
                score += 5.0;
            }
            if qi == 0 && i == 0 {
                score += 3.0;
            }
            qi += 1;
        } else {
            consecutive = 0.0;
        }
    }
    if qi < q.len() { 0.0 } else { score }
}

/// Rank `(path, name)` items for a query; an empty query keeps the original order.
pub fn fuzzy_search<'a>(
    query: &str,
    items: &'a [(String, String)],
    max: usize,
) -> Vec<&'a (String, String)> {
    if query.trim().is_empty() {
        return items.iter().take(max).collect();
    }
    let mut scored: Vec<(f64, &(String, String))> = items
        .iter()
        .filter_map(|item| {
            let score = (fuzzy_score(query, &item.1) * 1.5).max(fuzzy_score(query, &item.0));
            (score > 0.0).then_some((score, item))
        })
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
    scored.into_iter().take(max).map(|(_, item)| item).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_types() {
        assert!(is_document(Path::new("/a/B.MD")));
        assert!(is_document(Path::new("x.mdx")));
        assert!(is_html(Path::new("index.htm")));
        assert!(!is_document(Path::new("notes.txt")));
    }

    #[test]
    fn resolves_links_like_the_desktop() {
        let doc = Path::new("/docs/guide/intro.md");
        assert_eq!(resolve_link("#set-up", doc), LinkTarget::Anchor("set-up".into()));
        assert_eq!(
            resolve_link("https://example.com", doc),
            LinkTarget::External("https://example.com".into())
        );
        assert_eq!(resolve_link("mailto:a@b.c", doc), LinkTarget::External("mailto:a@b.c".into()));
        assert_eq!(
            resolve_link("../api/My%20Doc.md#usage", doc),
            LinkTarget::Document(PathBuf::from("/docs/api/My Doc.md"), Some("usage".into()))
        );
        assert_eq!(
            resolve_link("./img.png", doc),
            LinkTarget::File(PathBuf::from("/docs/guide/img.png"))
        );
        assert_eq!(
            resolve_link("file:///tmp/x.markdown", doc),
            LinkTarget::Document(PathBuf::from("/tmp/x.markdown"), None)
        );
    }

    #[test]
    fn scans_documents_only() {
        let root = std::env::temp_dir().join(format!("mdow-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("b/deep")).unwrap();
        std::fs::create_dir_all(root.join("empty")).unwrap();
        std::fs::create_dir_all(root.join("node_modules/x")).unwrap();
        std::fs::write(root.join("Zed.md"), "").unwrap();
        std::fs::write(root.join("alpha.md"), "").unwrap();
        std::fs::write(root.join("skip.txt"), "").unwrap();
        std::fs::write(root.join("b/deep/page.html"), "").unwrap();
        std::fs::write(root.join("node_modules/x/readme.md"), "").unwrap();
        let scan = scan_folder(&root);
        let names: Vec<_> = scan.tree.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, ["b", "alpha.md", "Zed.md"]);
        assert_eq!(count_files(&scan.tree), 3);
        let filtered = filter_tree(&scan.tree, &root, "page");
        assert_eq!(count_files(&filtered), 1);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn breadcrumbs_anchor_at_the_open_folder() {
        let file = Path::new("/home/me/notes/proj/docs/a.md");
        let names = |root: Option<&Path>| -> Vec<String> {
            breadcrumb_segments(file, root).into_iter().map(|s| s.0).collect()
        };
        assert_eq!(names(None), ["notes", "proj", "docs"]);
        assert_eq!(names(Some(Path::new("/home/me/notes/proj"))), ["proj", "docs"]);
        assert_eq!(names(Some(Path::new("/home/me"))), ["me", "notes", "proj", "docs"]);
    }

    #[test]
    fn fuzzy_prefers_names_and_runs() {
        let items = vec![
            ("/x/readme.md".to_string(), "readme.md".to_string()),
            ("/x/road/map.md".to_string(), "map.md".to_string()),
            ("/x/other.md".to_string(), "other.md".to_string()),
        ];
        let hits = fuzzy_search("rdm", &items, 10);
        assert_eq!(hits[0].1, "readme.md");
        assert!(hits.iter().all(|h| h.1 != "other.md"));
        assert_eq!(fuzzy_search("", &items, 2).len(), 2);
    }

    #[test]
    fn truncates_the_middle() {
        assert_eq!(truncate_middle("abcdefghij", 7), "abc…hij");
        assert_eq!(truncate_middle("short", 10), "short");
    }
}
