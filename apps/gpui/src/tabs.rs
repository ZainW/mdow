use crate::{document::ParsedDocument, syntax::PreparedDocument};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct DocumentTab {
    pub document: Arc<PreparedDocument>,
    pub last_source: Arc<str>,
    pub reload_error: Option<String>,
}

impl DocumentTab {
    pub fn path(&self) -> &Path {
        &self.document.path
    }
}

#[derive(Debug, Default)]
pub struct TabSet {
    tabs: Vec<DocumentTab>,
    active_path: Option<PathBuf>,
}

impl TabSet {
    pub fn open(&mut self, document: ParsedDocument) {
        self.open_prepared(PreparedDocument::plain(document));
    }

    pub fn open_prepared(&mut self, document: PreparedDocument) {
        let document = canonical_prepared_document(document);
        let path = document.path.clone();
        if let Some(tab) = self.tabs.iter_mut().find(|tab| tab.path() == path) {
            tab.last_source = Arc::from(document.source.clone());
            tab.document = Arc::new(document);
            tab.reload_error = None;
            self.active_path = Some(path);
            return;
        }

        self.active_path = Some(path);
        self.tabs.push(DocumentTab {
            last_source: Arc::from(document.source.clone()),
            document: Arc::new(document),
            reload_error: None,
        });
    }

    pub fn replace_document(&mut self, document: ParsedDocument) -> bool {
        self.replace_prepared(PreparedDocument::plain(document))
    }

    pub fn replace_prepared(&mut self, document: PreparedDocument) -> bool {
        let document = canonical_prepared_document(document);
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.path() == document.path) else {
            return false;
        };
        tab.last_source = Arc::from(document.source.clone());
        tab.document = Arc::new(document);
        tab.reload_error = None;
        true
    }

    pub fn activate(&mut self, path: &Path) -> bool {
        let path = path_identity(path);
        if self.tabs.iter().any(|tab| tab.path() == path) {
            self.active_path = Some(path);
            true
        } else {
            false
        }
    }

    pub fn close(&mut self, path: &Path) -> Option<DocumentTab> {
        let path = path_identity(path);
        let index = self.tabs.iter().position(|tab| tab.path() == path)?;
        let closing_active = self.active_path.as_deref() == Some(path.as_path());
        let removed = self.tabs.remove(index);

        if closing_active {
            self.active_path = self
                .tabs
                .get(index)
                .or_else(|| index.checked_sub(1).and_then(|left| self.tabs.get(left)))
                .map(|tab| tab.path().to_owned());
        }
        Some(removed)
    }

    pub fn active(&self) -> Option<&DocumentTab> {
        self.active_path.as_deref().and_then(|path| self.get(path))
    }

    pub fn get(&self, path: &Path) -> Option<&DocumentTab> {
        let path = path_identity(path);
        self.tabs.iter().find(|tab| tab.path() == path)
    }

    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    pub fn paths(&self) -> impl Iterator<Item = &Path> {
        self.tabs.iter().map(DocumentTab::path)
    }

    pub fn set_reload_error(&mut self, path: &Path, error: String) -> bool {
        let path = path_identity(path);
        let Some(tab) = self.tabs.iter_mut().find(|tab| tab.path() == path) else {
            return false;
        };
        tab.reload_error = Some(error);
        true
    }
}

fn canonical_prepared_document(mut document: PreparedDocument) -> PreparedDocument {
    let path = path_identity(&document.path);
    document.set_path(path);
    document
}

fn path_identity(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{ParsedDocument, parse_document};
    use crate::syntax::prepare_document;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    fn document(path: &str, title: &str) -> ParsedDocument {
        parse_document(PathBuf::from(path), format!("# {title}\n"))
    }

    fn three_tabs() -> TabSet {
        let mut tabs = TabSet::default();
        tabs.open(document("/tmp/a.md", "A"));
        tabs.open(document("/tmp/b.md", "B"));
        tabs.open(document("/tmp/c.md", "C"));
        tabs
    }

    #[test]
    fn prepared_open_keeps_highlights_on_the_tab() {
        let prepared = prepare_document(parse_document(
            PathBuf::from("/tmp/a.md"),
            "```rust\nlet n = 1;\n```\n".into(),
        ));
        let mut tabs = TabSet::default();

        tabs.open_prepared(prepared);

        assert!(tabs.active().unwrap().document.code_block(0).is_some());
    }

    #[test]
    fn prepared_reload_replaces_highlights_without_changing_selection() {
        let mut tabs = three_tabs();
        tabs.activate(Path::new("/tmp/b.md"));
        let replacement = prepare_document(parse_document(
            PathBuf::from("/tmp/a.md"),
            "```javascript\nconst n = 2;\n```\n".into(),
        ));

        assert!(tabs.replace_prepared(replacement));
        assert_eq!(tabs.active().unwrap().path(), Path::new("/tmp/b.md"));
        assert!(
            tabs.get(Path::new("/tmp/a.md"))
                .unwrap()
                .document
                .code_block(0)
                .is_some()
        );
    }

    #[test]
    fn opening_an_existing_path_focuses_without_duplication() {
        let mut tabs = TabSet::default();
        tabs.open(document("/tmp/a.md", "A"));
        tabs.open(document("/tmp/b.md", "B"));
        tabs.open(document("/tmp/a.md", "A changed"));

        assert_eq!(tabs.len(), 2);
        assert_eq!(tabs.active().unwrap().document.title, "A changed");
        assert_eq!(
            tabs.paths().collect::<Vec<_>>(),
            vec![Path::new("/tmp/a.md"), Path::new("/tmp/b.md")]
        );
    }

    #[test]
    fn opening_a_symlinked_file_uses_the_canonical_tab_identity() {
        #[cfg(unix)]
        {
            use std::fs;
            use std::os::unix::fs::symlink;

            let temp = tempfile::tempdir().unwrap();
            let target = temp.path().join("original.md");
            let alias = temp.path().join("alias.md");
            fs::write(&target, "# Original").unwrap();
            symlink(&target, &alias).unwrap();

            let mut tabs = TabSet::default();
            tabs.open(document(target.to_str().unwrap(), "Original"));
            tabs.open(document(alias.to_str().unwrap(), "Alias"));

            assert_eq!(tabs.len(), 1);
            assert_eq!(
                tabs.active().unwrap().path(),
                target.canonicalize().unwrap()
            );
            assert_eq!(tabs.active().unwrap().document.title, "Alias");
        }
    }

    #[test]
    fn replacing_a_document_preserves_order_and_active_selection() {
        let mut tabs = three_tabs();
        tabs.activate(Path::new("/tmp/b.md"));

        assert!(tabs.replace_document(document("/tmp/a.md", "A refreshed")));

        assert_eq!(
            tabs.paths().collect::<Vec<_>>(),
            vec![
                Path::new("/tmp/a.md"),
                Path::new("/tmp/b.md"),
                Path::new("/tmp/c.md"),
            ]
        );
        assert_eq!(tabs.active().unwrap().path(), Path::new("/tmp/b.md"));
        assert_eq!(
            tabs.get(Path::new("/tmp/a.md")).unwrap().document.title,
            "A refreshed"
        );
    }

    #[test]
    fn reload_error_keeps_last_successful_document_and_source() {
        let mut tabs = TabSet::default();
        tabs.open(document("/tmp/a.md", "A"));
        let before = tabs.active().unwrap().document.clone();
        let source = tabs.active().unwrap().last_source.clone();

        assert!(tabs.set_reload_error(Path::new("/tmp/a.md"), "File not found".into()));

        let tab = tabs.active().unwrap();
        assert!(Arc::ptr_eq(&tab.document, &before));
        assert!(Arc::ptr_eq(&tab.last_source, &source));
        assert_eq!(tab.reload_error.as_deref(), Some("File not found"));
    }

    #[test]
    fn replacing_a_document_clears_its_reload_error() {
        let mut tabs = TabSet::default();
        tabs.open(document("/tmp/a.md", "A"));
        tabs.set_reload_error(Path::new("/tmp/a.md"), "Invalid UTF-8".into());

        assert!(tabs.replace_document(document("/tmp/a.md", "A refreshed")));

        let tab = tabs.active().unwrap();
        assert_eq!(tab.document.title, "A refreshed");
        assert!(tab.reload_error.is_none());
    }

    #[test]
    fn closing_active_prefers_the_tab_to_its_right() {
        let mut tabs = three_tabs();
        tabs.activate(Path::new("/tmp/b.md"));

        assert!(tabs.close(Path::new("/tmp/b.md")).is_some());

        assert_eq!(tabs.active().unwrap().path(), Path::new("/tmp/c.md"));
    }

    #[test]
    fn closing_the_last_active_tab_prefers_its_left_neighbor() {
        let mut tabs = three_tabs();
        tabs.activate(Path::new("/tmp/c.md"));

        assert!(tabs.close(Path::new("/tmp/c.md")).is_some());

        assert_eq!(tabs.active().unwrap().path(), Path::new("/tmp/b.md"));
    }

    #[test]
    fn closing_an_inactive_tab_keeps_the_current_selection() {
        let mut tabs = three_tabs();
        tabs.activate(Path::new("/tmp/c.md"));

        assert!(tabs.close(Path::new("/tmp/a.md")).is_some());

        assert_eq!(tabs.active().unwrap().path(), Path::new("/tmp/c.md"));
    }
}
