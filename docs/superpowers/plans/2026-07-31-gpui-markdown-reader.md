# Mdow GPUI Markdown Reader Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a locally runnable macOS Mdow prototype in Rust + GPUI that opens files and folders, renders core Markdown, supports tabs and live reload, faithfully reproduces the established compact UI, and contains no AI chat.

**Architecture:** Add an independent Cargo package at `apps/gpui/`. Keep file loading, Markdown parsing, folder scanning, tab behavior, and path resolution independent from GPUI; make one `MdowApp` GPUI entity compose a custom shell and reader from those tested models. GPUI 0.2.2 supplies the window, native path prompts, system appearance, input actions, assets, clipboard, drag/drop, and rendering.

**Tech Stack:** Stable Rust 1.93+, edition 2024, GPUI 0.2.2, pulldown-cmark 0.13.4, notify 8.2.0, open 5.4.0, anyhow 1.0, tempfile 3.27.0 for tests, macOS 14+.

## Global Constraints

- Target macOS 14 or newer on Apple Silicon for this local prototype.
- Use stable Rust 1.93.0 or newer and commit `apps/gpui/Cargo.lock`.
- Use `gpui = "=0.2.2"`; do not add `gpui-component`, a web view, or a browser renderer.
- Treat `apps/desktop/src/renderer/src/assets/styles/index.css` and `markdown.css` as the visual source of truth.
- Reuse Inter for application/document text and Geist Mono for code; register bundled native font files at startup.
- Keep all application work under `apps/gpui/` and `script/build_and_run_gpui.sh`.
- Do not modify the Electron or Swift application behavior.
- Do not add AI chat, editing, split view, search, command palette, settings, recents persistence, Mermaid, HTML execution, math rendering, syntax highlighting, CI, packaging, signing, telemetry, updates, or distribution work.
- Required final checks are `cargo test --manifest-path apps/gpui/Cargo.toml`, `cargo build --manifest-path apps/gpui/Cargo.toml`, local launch verification, and visual screenshot review.

---

## File Structure

- `apps/gpui/Cargo.toml` — pinned package metadata and dependencies.
- `apps/gpui/Cargo.lock` — resolved dependency graph.
- `apps/gpui/src/lib.rs` — exports the testable domain modules.
- `apps/gpui/src/main.rs` — GPUI startup, font registration, menus, window creation, and optional launch path.
- `apps/gpui/src/document.rs` — supported paths, UTF-8 loading, Markdown block/inline model, parsing, and local-link resolution.
- `apps/gpui/src/workspace.rs` — recursive folder scan, filtering, sorting, and expandable tree state.
- `apps/gpui/src/tabs.rs` — tab deduplication, activation, replacement, reload errors, and close selection.
- `apps/gpui/src/watcher.rs` — notify watcher, debounce, and GPUI-safe reload messages.
- `apps/gpui/src/assets.rs` — package-relative `AssetSource` for SVGs and raster images.
- `apps/gpui/src/theme.rs` — light/dark colors and exact compact layout/typography tokens.
- `apps/gpui/src/actions.rs` — GPUI actions for file/folder open, sidebar, tabs, reading width, and quit.
- `apps/gpui/src/app.rs` — `MdowApp` state, file/folder operations, dialogs, drop handling, actions, and root render.
- `apps/gpui/src/ui/mod.rs` — UI module exports.
- `apps/gpui/src/ui/primitives.rs` — buttons, icon buttons, SVG icons, separators, hover/pressed/focus styling.
- `apps/gpui/src/ui/welcome.rs` — polished empty/welcome state and drop affordance.
- `apps/gpui/src/ui/chrome.rs` — titlebar inset, sidebar tree, tab bar, breadcrumb, and error banner.
- `apps/gpui/src/ui/reader.rs` — scrollable Markdown block and inline renderer.
- `apps/gpui/assets/fonts/InterVariable.ttf` — native Inter variable font converted from the existing WOFF2 asset.
- `apps/gpui/assets/fonts/GeistMono-Variable.ttf` — native Geist Mono variable font converted from the existing WOFF2 asset.
- `apps/gpui/assets/icons/*.svg` — Mdow logo and the exact Lucide file/folder/navigation/control icons used by the prototype.
- `apps/gpui/tests/fixtures/showcase.md` — deterministic Markdown for parser, runtime, and visual checks.
- `script/build_and_run_gpui.sh` — build and foreground-launch helper with optional path and verify modes.

## Task 1: Cargo Package and File Loading Core

**Files:**
- Create: `apps/gpui/Cargo.toml`
- Create: `apps/gpui/src/lib.rs`
- Create: `apps/gpui/src/document.rs`
- Create: `apps/gpui/tests/fixtures/showcase.md`

**Interfaces:**
- Produces: `is_supported_markdown(path: &Path) -> bool`
- Produces: `load_source(path: &Path) -> Result<LoadedSource, DocumentError>`
- Produces: `LoadedSource { canonical_path: PathBuf, source: String }`
- Produces: `DocumentError::{Unsupported, Missing, InvalidUtf8, Read}` with `title()`, `body()`, and `path()`

- [ ] **Step 1: Create the package manifest and empty library surface**

Create `apps/gpui/Cargo.toml`:

```toml
[package]
name = "mdow-gpui"
version = "0.1.0"
edition = "2024"
publish = false

[dependencies]
anyhow = "1.0"
gpui = "=0.2.2"
notify = "=8.2.0"
open = "=5.4.0"
pulldown-cmark = { version = "=0.13.4", default-features = false }

[dev-dependencies]
tempfile = "=3.27.0"
```

Create `src/lib.rs` with `pub mod document;` and create an empty `src/document.rs`.

- [ ] **Step 2: Write failing file-loading tests**

Add these tests at the bottom of `document.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn recognizes_supported_extensions_case_insensitively() {
        assert!(is_supported_markdown(Path::new("README.md")));
        assert!(is_supported_markdown(Path::new("notes.MARKDOWN")));
        assert!(is_supported_markdown(Path::new("component.MdX")));
        assert!(!is_supported_markdown(Path::new("notes.txt")));
    }

    #[test]
    fn loads_utf8_and_returns_a_canonical_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.md");
        fs::write(&path, "# Hello\n").unwrap();

        let loaded = load_source(&path).unwrap();

        assert_eq!(loaded.canonical_path, path.canonicalize().unwrap());
        assert_eq!(loaded.source, "# Hello\n");
    }

    #[test]
    fn reports_invalid_utf8_without_debug_copy() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.md");
        fs::write(&path, [0xff, 0xfe]).unwrap();

        let error = load_source(&path).unwrap_err();

        assert!(matches!(error, DocumentError::InvalidUtf8 { .. }));
        assert_eq!(error.title(), "This file is not UTF-8");
    }
}
```

- [ ] **Step 3: Run the focused tests and confirm they fail**

Run: `cargo test --manifest-path apps/gpui/Cargo.toml document::tests -- --nocapture`

Expected: compilation fails because the loader types and functions do not exist.

- [ ] **Step 4: Implement extension validation, loading, and readable errors**

Implement these exact public types and signatures:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSource {
    pub canonical_path: PathBuf,
    pub source: String,
}

#[derive(Debug)]
pub enum DocumentError {
    Unsupported { path: PathBuf },
    Missing { path: PathBuf },
    InvalidUtf8 { path: PathBuf },
    Read { path: PathBuf, message: String },
}

pub fn is_supported_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "md" | "markdown" | "mdx"))
}

pub fn load_source(path: &Path) -> Result<LoadedSource, DocumentError> {
    if !is_supported_markdown(path) {
        return Err(DocumentError::Unsupported { path: path.to_owned() });
    }
    if !path.exists() {
        return Err(DocumentError::Missing { path: path.to_owned() });
    }
    let bytes = std::fs::read(path).map_err(|error| DocumentError::Read {
        path: path.to_owned(),
        message: error.to_string(),
    })?;
    let source = String::from_utf8(bytes)
        .map_err(|_| DocumentError::InvalidUtf8 { path: path.to_owned() })?;
    let canonical_path = path.canonicalize().map_err(|error| DocumentError::Read {
        path: path.to_owned(),
        message: error.to_string(),
    })?;
    Ok(LoadedSource { canonical_path, source })
}
```

Implement `title`, `body`, and `path` using concise product copy from the design spec; `body` must not contain `Debug` formatting.

- [ ] **Step 5: Add the deterministic showcase fixture**

Create `apps/gpui/tests/fixtures/showcase.md` with H1–H6, emphasis, strong text, a local Markdown link, inline code, ordered/unordered/task lists, a blockquote, a fenced Rust block, a three-column table, a thematic break, raw HTML text, and an image reference.

- [ ] **Step 6: Run tests and commit**

Run: `cargo test --manifest-path apps/gpui/Cargo.toml document::tests`

Expected: all Task 1 tests pass and Cargo creates `apps/gpui/Cargo.lock`.

Commit:

```bash
git add apps/gpui/Cargo.toml apps/gpui/Cargo.lock apps/gpui/src/lib.rs apps/gpui/src/document.rs apps/gpui/tests/fixtures/showcase.md
git commit -m "feat(gpui): add markdown file loading core"
```

## Task 2: Owned Markdown Model and Parser

**Files:**
- Modify: `apps/gpui/src/document.rs`

**Interfaces:**
- Consumes: `LoadedSource`
- Produces: `parse_document(path: PathBuf, source: String) -> ParsedDocument`
- Produces: `ParsedDocument { path, title, source, blocks, headings }`
- Produces: `DocumentBlock`, `InlineSpan`, `ListKind`, `TableBlock`, and `Heading`
- Produces: `resolve_local_target(document_path: &Path, target: &str) -> Option<PathBuf>`

- [ ] **Step 1: Write failing parser and path-resolution tests**

Add tests that assert the complete owned model rather than parser events:

```rust
#[test]
fn parses_reader_blocks_and_inline_styles() {
    let parsed = parse_document(
        PathBuf::from("/tmp/guide.md"),
        "# Guide\n\nHello *quiet* **reader** with `code`.\n\n- [x] Done\n\n```rust\nlet n = 1;\n```\n".into(),
    );

    assert_eq!(parsed.title, "Guide");
    assert_eq!(parsed.headings, vec![Heading { level: 1, text: "Guide".into() }]);
    assert_eq!(parsed.blocks[0], DocumentBlock::Heading {
        level: 1,
        content: vec![InlineSpan::Text("Guide".into())],
    });
    assert!(matches!(parsed.blocks[1], DocumentBlock::Paragraph(_)));
    assert_eq!(parsed.blocks[2], DocumentBlock::TaskItem {
        checked: true,
        depth: 0,
        content: vec![InlineSpan::Text("Done".into())],
    });
    assert_eq!(parsed.blocks[3], DocumentBlock::CodeBlock {
        language: Some("rust".into()),
        code: "let n = 1;\n".into(),
    });
}

#[test]
fn keeps_raw_html_and_mdx_inert_and_readable() {
    let parsed = parse_document(
        PathBuf::from("/tmp/component.mdx"),
        "<aside>Note</aside>\n\n<Component value={1} />".into(),
    );
    let visible = parsed.plain_text();
    assert!(visible.contains("<aside>Note</aside>"));
    assert!(visible.contains("<Component value={1} />"));
}

#[test]
fn resolves_relative_targets_against_the_document() {
    assert_eq!(
        resolve_local_target(Path::new("/vault/guides/start.md"), "../images/hero.png"),
        Some(PathBuf::from("/vault/images/hero.png")),
    );
    assert_eq!(resolve_local_target(Path::new("/vault/start.md"), "https://mdow.dev"), None);
}
```

- [ ] **Step 2: Run the focused tests and confirm they fail**

Run: `cargo test --manifest-path apps/gpui/Cargo.toml parses_reader_blocks_and_inline_styles`

Expected: compilation fails because the Markdown model does not exist.

- [ ] **Step 3: Define the owned model**

Use these model shapes:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedDocument {
    pub path: PathBuf,
    pub title: String,
    pub source: String,
    pub blocks: Vec<DocumentBlock>,
    pub headings: Vec<Heading>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineSpan {
    Text(String),
    Emphasis(Vec<InlineSpan>),
    Strong(Vec<InlineSpan>),
    Code(String),
    Link { label: Vec<InlineSpan>, target: String },
    SoftBreak,
    HardBreak,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DocumentBlock {
    Heading { level: u8, content: Vec<InlineSpan> },
    Paragraph(Vec<InlineSpan>),
    ListItem { kind: ListKind, depth: usize, content: Vec<InlineSpan> },
    TaskItem { checked: bool, depth: usize, content: Vec<InlineSpan> },
    Blockquote(Vec<InlineSpan>),
    ThematicBreak,
    CodeBlock { language: Option<String>, code: String },
    Table(TableBlock),
    Image { alt: String, source: String },
    RawText(String),
}
```

`ListKind` is `Unordered` or `Ordered { number: u64 }`. `TableBlock` owns `headers: Vec<Vec<InlineSpan>>` and `rows: Vec<Vec<Vec<InlineSpan>>>`. `Heading` owns `level: u8` and `text: String`.

- [ ] **Step 4: Implement the pulldown-cmark event reducer**

Enable `Options::ENABLE_TABLES`, `ENABLE_TASKLISTS`, `ENABLE_STRIKETHROUGH`, `ENABLE_FOOTNOTES`, and `ENABLE_GFM`. Reduce start/end events with explicit stacks for inline spans, list depth, blockquote content, fenced code, and table cells. Convert `Event::Html` and `Event::InlineHtml` to `DocumentBlock::RawText` or `InlineSpan::Text`; never pass them to an HTML renderer.

Title priority is the first H1's plain text, then `path.file_name()`, then `Untitled`.

- [ ] **Step 5: Implement plain text and local target helpers**

Add `InlineSpan::plain_text`, `ParsedDocument::plain_text`, and lexical path normalization that removes `.` and resolves `..` without requiring the target to exist. Return `None` for `http:`, `https:`, `mailto:`, fragment-only, and other absolute URI schemes.

- [ ] **Step 6: Run the document tests and commit**

Run: `cargo test --manifest-path apps/gpui/Cargo.toml document::tests`

Expected: all loader, parser, inert-content, and path tests pass.

Commit:

```bash
git add apps/gpui/src/document.rs
git commit -m "feat(gpui): parse markdown into reader blocks"
```

## Task 3: Workspace Tree and Tab State

**Files:**
- Modify: `apps/gpui/src/lib.rs`
- Create: `apps/gpui/src/workspace.rs`
- Create: `apps/gpui/src/tabs.rs`

**Interfaces:**
- Consumes: `is_supported_markdown`, `ParsedDocument`
- Produces: `scan_workspace(root: &Path) -> Result<WorkspaceTree, WorkspaceError>`
- Produces: `WorkspaceEntry { path, name, kind, children, expanded }`
- Produces: `WorkspaceTree::toggle_directory(path: &Path)` and `visible_rows() -> Vec<WorkspaceRow>`
- Produces: `TabSet::{open, replace_document, activate, close, active, get, len, is_empty, paths, set_reload_error}`

- [ ] **Step 1: Write failing workspace-tree tests**

Create a temporary tree containing `.git`, `node_modules`, `target`, hidden files, Markdown files, a text file, and nested directories. Assert directories precede files, sorting is case-insensitive, ignored entries are absent, and toggling a directory changes `visible_rows()`.

```rust
assert_eq!(names(&tree.root.children), vec!["guides", "Alpha.md", "zeta.md"]);
assert!(!tree.all_paths().any(|path| path.ends_with("node_modules/hidden.md")));
tree.toggle_directory(&root.join("guides"));
assert!(tree.visible_rows().iter().any(|row| row.name == "start.md" && row.depth == 1));
```

- [ ] **Step 2: Write failing tab behavior tests**

```rust
#[test]
fn opening_an_existing_path_focuses_without_duplication() {
    let mut tabs = TabSet::default();
    tabs.open(document("/tmp/a.md", "A"));
    tabs.open(document("/tmp/b.md", "B"));
    tabs.open(document("/tmp/a.md", "A changed"));
    assert_eq!(tabs.len(), 2);
    assert_eq!(tabs.active().unwrap().document.title, "A changed");
}

#[test]
fn closing_active_prefers_the_tab_to_its_right() {
    let mut tabs = three_tabs();
    tabs.activate(Path::new("/tmp/b.md"));
    tabs.close(Path::new("/tmp/b.md"));
    assert_eq!(tabs.active().unwrap().path(), Path::new("/tmp/c.md"));
}
```

- [ ] **Step 3: Run both focused modules and confirm failure**

Run: `cargo test --manifest-path apps/gpui/Cargo.toml workspace::tests tabs::tests`

Expected: compilation fails because both modules are missing.

- [ ] **Step 4: Implement workspace scanning and presentation state**

Use `std::fs::read_dir` recursively. Ignore an entry if its file name begins with `.` or exactly matches `.git`, `node_modules`, `target`, `dist`, or `build`. Keep a directory only when it contains a supported descendant. Sort with `(kind_rank, name.to_lowercase(), name)` where directories have rank zero.

- [ ] **Step 5: Implement `TabSet`**

Use canonical `PathBuf` as tab identity. `DocumentTab` owns `document: Arc<ParsedDocument>`, `last_source: Arc<str>`, and `reload_error: Option<String>`. `replace_document` changes only the matching tab's document/source/error; it must preserve ordering and active selection.

- [ ] **Step 6: Run tests and commit**

Run: `cargo test --manifest-path apps/gpui/Cargo.toml workspace::tests tabs::tests`

Expected: all Task 3 tests pass.

Commit:

```bash
git add apps/gpui/src/lib.rs apps/gpui/src/workspace.rs apps/gpui/src/tabs.rs
git commit -m "feat(gpui): add workspace tree and document tabs"
```

## Task 4: Assets, Exact Theme Tokens, and GPUI Window Bootstrap

**Files:**
- Create: `apps/gpui/src/main.rs`
- Create: `apps/gpui/src/assets.rs`
- Create: `apps/gpui/src/theme.rs`
- Create: `apps/gpui/src/actions.rs`
- Create: `apps/gpui/src/app.rs`
- Create: `apps/gpui/src/ui/mod.rs`
- Create: `apps/gpui/src/ui/primitives.rs`
- Create: `apps/gpui/src/ui/welcome.rs`
- Create: `apps/gpui/assets/fonts/InterVariable.ttf`
- Create: `apps/gpui/assets/fonts/GeistMono-Variable.ttf`
- Create: `apps/gpui/assets/icons/mdow-logo.svg`
- Create: `apps/gpui/assets/icons/{file,folder,folder-open,chevron-right,x,sidebar,expand,copy,check,alert-circle}.svg`

**Interfaces:**
- Consumes: `TabSet`, `WorkspaceTree`, `DocumentError`
- Produces: `MdowAssets: AssetSource`
- Produces: `Theme::for_appearance(WindowAppearance) -> Theme`
- Produces: `Metrics` constants used by every UI module
- Produces: `MdowApp: Render`
- Produces: `actions::{OpenFile, OpenFolder, ToggleSidebar, CloseTab, ToggleWideMode, Quit}`

- [ ] **Step 1: Write failing theme-contract tests**

In `theme.rs`, add tests for the non-negotiable measurements:

```rust
#[test]
fn compact_metrics_match_the_established_ui() {
    assert_eq!(Metrics::SIDEBAR_WIDTH, 244.0);
    assert_eq!(Metrics::TAB_BAR_HEIGHT, 36.0);
    assert_eq!(Metrics::TAB_HEIGHT, 28.0);
    assert_eq!(Metrics::BREADCRUMB_HEIGHT, 28.0);
    assert_eq!(Metrics::READING_WIDTH, 768.0);
    assert_eq!(Metrics::READING_PADDING_X, 48.0);
    assert_eq!(Metrics::BODY_FONT_SIZE, 15.5);
    assert_eq!(Metrics::BODY_LINE_HEIGHT, 1.65);
}
```

- [ ] **Step 2: Run the theme test and confirm failure**

Run: `cargo test --manifest-path apps/gpui/Cargo.toml compact_metrics_match_the_established_ui`

Expected: compilation fails because `theme.rs` and `Metrics` do not exist.

- [ ] **Step 3: Prepare native font and SVG assets**

Convert the two existing WOFF2 assets into native TTF variable fonts using a temporary FontTools environment; do not replace or modify the Electron assets:

```bash
python3 -m venv /tmp/mdow-fonttools
/tmp/mdow-fonttools/bin/pip install fonttools brotli
/tmp/mdow-fonttools/bin/fonttools ttLib.woff2 decompress apps/desktop/src/renderer/src/assets/fonts/InterVariable.woff2 -o apps/gpui/assets/fonts/InterVariable.ttf
/tmp/mdow-fonttools/bin/fonttools ttLib.woff2 decompress apps/desktop/src/renderer/src/assets/fonts/GeistMono-Variable.woff2 -o apps/gpui/assets/fonts/GeistMono-Variable.ttf
```

Copy the Mdow logo SVG and add only the listed Lucide icons as 24×24 `currentColor` SVG assets with stroke width 2, linecap round, and linejoin round.

- [ ] **Step 4: Implement assets and font registration**

`MdowAssets` loads from `PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")`. Startup calls:

```rust
cx.text_system().add_fonts(vec![
    Cow::Owned(std::fs::read(asset_root.join("fonts/InterVariable.ttf"))?),
    Cow::Owned(std::fs::read(asset_root.join("fonts/GeistMono-Variable.ttf"))?),
])?;
```

Use `Application::new().with_assets(MdowAssets::new(asset_root))` so `svg().path("icons/file.svg")` resolves consistently.

- [ ] **Step 5: Implement exact light/dark tokens**

Convert the Electron OKLCH tokens to fixed GPUI `Hsla` values once and define `Theme { background, foreground, card, muted, muted_foreground, primary, accent, destructive, border, border_subtle, sidebar, sidebar_accent }`. Select light for `Light | VibrantLight` and dark for `Dark | VibrantDark`. Do not use GPUI's default component colors.

- [ ] **Step 6: Bootstrap the custom window and welcome state**

Create a centered 1120×760 window with a transparent titlebar:

```rust
WindowOptions {
    titlebar: Some(TitlebarOptions {
        title: Some("Mdow".into()),
        appears_transparent: true,
        traffic_light_position: Some(point(px(14.), px(14.))),
        ..Default::default()
    }),
    window_bounds: Some(WindowBounds::Windowed(bounds)),
    ..Default::default()
}
```

`MdowApp` initially owns an empty `TabSet`, no workspace, `sidebar_open: true`, `wide_mode: false`, and `drop_active: false`. Render a 28 px titlebar inset and the approved welcome state with 48 px logo, 24 px/600 Mdow title, 14 px relaxed copy, compact outline buttons, and dashed drop surface.

- [ ] **Step 7: Bind actions and system appearance observation**

Use `actions!` and `cx.bind_keys` for `cmd-o`, `cmd-shift-o`, `cmd-b`, `cmd-w`, `cmd-shift-w`, and `cmd-q`. Retain the `observe_window_appearance` subscription in `MdowApp` and call `cx.notify()` on appearance changes.

- [ ] **Step 8: Run tests, build the first window, and commit**

Run:

```bash
cargo test --manifest-path apps/gpui/Cargo.toml compact_metrics_match_the_established_ui
cargo build --manifest-path apps/gpui/Cargo.toml
```

Expected: the metric test passes and the GPUI executable builds.

Commit:

```bash
git add apps/gpui
git commit -m "feat(gpui): bootstrap polished Mdow window"
```

## Task 5: File/Folder Operations, Sidebar, Tabs, and Breadcrumb

**Files:**
- Modify: `apps/gpui/src/app.rs`
- Create: `apps/gpui/src/ui/chrome.rs`
- Modify: `apps/gpui/src/ui/mod.rs`
- Modify: `apps/gpui/src/ui/primitives.rs`
- Modify: `apps/gpui/src/ui/welcome.rs`

**Interfaces:**
- Consumes: loader/parser, `WorkspaceTree`, `TabSet`, actions, theme primitives
- Produces: `MdowApp::{open_path, open_file_prompt, open_folder_prompt, close_active_tab}`
- Produces: `render_sidebar`, `render_tab_bar`, `render_breadcrumb`, `render_error_state`

- [ ] **Step 1: Write failing orchestration tests around a pure state helper**

Extract `AppModel` from the GPUI entity so tests can verify:

```rust
#[test]
fn opening_a_file_populates_a_tab_and_selects_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("guide.md");
    std::fs::write(&path, "# Guide").unwrap();
    let mut model = AppModel::default();

    model.open_path(&path).unwrap();

    assert_eq!(model.tabs.len(), 1);
    assert_eq!(model.tabs.active().unwrap().document.title, "Guide");
}

#[test]
fn opening_a_folder_populates_the_tree_without_opening_a_tab() {
    let root = markdown_workspace();
    let mut model = AppModel::default();
    model.open_path(root.path()).unwrap();
    assert!(model.workspace.is_some());
    assert!(model.tabs.is_empty());
}
```

- [ ] **Step 2: Run the orchestration tests and confirm failure**

Run: `cargo test --manifest-path apps/gpui/Cargo.toml app::tests`

Expected: compilation fails because `AppModel` does not exist.

- [ ] **Step 3: Implement pure open-path orchestration**

`AppModel::open_path` dispatches directories to `scan_workspace` and files to `load_source` + `parse_document` + `TabSet::open`. Return `AppOpenError` with a user-facing error model and keep previous successful state intact on failure.

- [ ] **Step 4: Implement GPUI native prompts**

`OpenFile` calls `cx.prompt_for_paths(PathPromptOptions { files: true, directories: false, multiple: true, prompt: Some("Open".into()) })`; `OpenFolder` uses `{ files: false, directories: true, multiple: false, prompt: Some("Open Folder".into()) }`. Await receivers with `cx.spawn`, then update the `MdowApp` entity and notify.

- [ ] **Step 5: Render the sidebar exactly**

Render a 244 px sidebar with 1 px subtle right border, 8 px header padding, a single Folder label/action row, and scrollable flattened `visible_rows()`. Rows use 12 px Inter, 4 px vertical/6 px horizontal padding, 10 px per nesting level, 14 px icons, stable font weight, hover surface, and the active row's 2 px orange bar. Disclosure clicks call `toggle_directory`; file clicks call `open_path`.

- [ ] **Step 6: Render tabs and breadcrumb exactly**

Tab bar height is 36 px. Tabs are 28 px high, max 200 px, with 14 px file icon, 6 px gap, 10 px horizontal text padding, and 24 px close target. Active tabs use card background, 8 px radius, 1 px subtle ring, and restrained shadow. Breadcrumb height is 28 px with 11 px Inter, up to the final three parent segments, 10 px chevrons, bold-enough current title, and a 20 px full-width icon button.

- [ ] **Step 7: Wire external file/folder drops**

On the root element, use `.drag_over::<ExternalPaths>(...)` for the primary-tint state and `.on_drop(cx.listener(|this, paths: &ExternalPaths, _, cx| ...))`. Iterate `paths.paths()` through `AppModel::open_path`; display the first failure without discarding successful opens.

- [ ] **Step 8: Run tests, build, and commit**

Run:

```bash
cargo test --manifest-path apps/gpui/Cargo.toml app::tests tabs::tests workspace::tests
cargo build --manifest-path apps/gpui/Cargo.toml
```

Expected: orchestration/model tests pass and the complete chrome builds.

Commit:

```bash
git add apps/gpui/src/app.rs apps/gpui/src/ui
git commit -m "feat(gpui): add native workspace chrome"
```

## Task 6: Faithful Markdown Reader

**Files:**
- Create: `apps/gpui/src/ui/reader.rs`
- Modify: `apps/gpui/src/ui/mod.rs`
- Modify: `apps/gpui/src/app.rs`
- Modify: `apps/gpui/src/ui/primitives.rs`

**Interfaces:**
- Consumes: `ParsedDocument`, `DocumentBlock`, `InlineSpan`, `Theme`, `Metrics`
- Produces: `render_document(document: Arc<ParsedDocument>, wide_mode: bool, ...) -> AnyElement`
- Produces: `render_inline(spans: &[InlineSpan], ...) -> AnyElement`

- [ ] **Step 1: Add render-contract tests for deterministic style mapping**

Define `BlockStyle::for_block(&DocumentBlock) -> BlockStyle` outside GPUI rendering and test exact values:

```rust
#[test]
fn reader_styles_match_markdown_css() {
    assert_eq!(BlockStyle::heading(1).font_size, 15.5 * 1.875);
    assert_eq!(BlockStyle::heading(1).line_height, 1.2);
    assert_eq!(BlockStyle::heading(2).font_weight, 650);
    assert_eq!(BlockStyle::code_block().radius, 10.0);
    assert_eq!(BlockStyle::code_block().padding, [14.0, 18.0]);
    assert_eq!(BlockStyle::table_cell().padding, [10.0, 14.0]);
}
```

- [ ] **Step 2: Run the style tests and confirm failure**

Run: `cargo test --manifest-path apps/gpui/Cargo.toml reader_styles_match_markdown_css`

Expected: compilation fails because `reader.rs` does not exist.

- [ ] **Step 3: Implement the reading column and scroll container**

Render one vertical `.overflow_y_scroll()` surface. In constrained mode, center a `.max_w(px(768.))` column; in wide mode, remove the cap and align left. Both modes use 48 px horizontal, 22 px top, and 40 px bottom padding. Set Inter, 15.5 px, 1.65 line-height, foreground color at the column root.

- [ ] **Step 4: Implement inline spans**

Render text runs with GPUI `StyledText`/`TextRun` so emphasis, strong, code, and links do not break wrapping into separate flex boxes. Code uses Geist Mono, 0.875 em, muted background, 4 px radius, and 0.1 em/0.35 em inset. Links use primary color, underline on hover, pointer cursor, and click routing: supported local Markdown calls `open_path`, HTTP/HTTPS calls `open::that`, and other resolved local paths call `open::that`.

- [ ] **Step 5: Implement all in-scope block views**

Map heading sizes/weights/line-heights/tracking exactly from the spec. Lists use 8 px marker gap and indentation derived from depth. Blockquotes use a 3 px border, 16 px inset, and muted copy. Thematic breaks use one border pixel and 31 px vertical breathing room. Task boxes are 14 px rounded-square SVG states.

Code blocks use a muted surface, 1 px border, 10 px radius, 14×18 px padding, 1.6 line-height, Geist Mono, 11 px language label, and 28 px copy control. Copy writes `ClipboardItem::new_string(code.clone())`; store `copied_code: Option<(usize, Instant)>` and clear it after two seconds with `Timer::after`.

Tables render a horizontally scrollable grid with 1 px borders, 8 px outer radius, 10×14 px cells, 0.925 em body copy, 0.8 em uppercase headers, and tabular numeric features when available. Images use `img(Arc::<Path>::from(resolved_path))`, max width 100%, preserved aspect ratio, and 8 px radius; failed images render alt text in the muted surface.

- [ ] **Step 6: Add centered initial errors and reload banners**

Initial file errors render the 48 px muted circular icon, 16 px/500 title, 14 px relaxed body, muted monospaced path, and Open File action. A tab reload error renders a compact destructive-tint banner between breadcrumb and reader without replacing the last successful document.

- [ ] **Step 7: Run tests, build, and commit**

Run:

```bash
cargo test --manifest-path apps/gpui/Cargo.toml reader_styles_match_markdown_css document::tests
cargo build --manifest-path apps/gpui/Cargo.toml
```

Expected: style/parser tests pass and the full reader compiles.

Commit:

```bash
git add apps/gpui/src/app.rs apps/gpui/src/ui
git commit -m "feat(gpui): render polished markdown documents"
```

## Task 7: Live Reload and Complete Shortcut Behavior

**Files:**
- Create: `apps/gpui/src/watcher.rs`
- Modify: `apps/gpui/src/lib.rs`
- Modify: `apps/gpui/src/app.rs`
- Modify: `apps/gpui/src/main.rs`

**Interfaces:**
- Consumes: open tab paths and GPUI foreground updates
- Produces: `FileWatcher::watch(path: &Path) -> Result<()>`
- Produces: `WatchMessage::Reload(PathBuf)` after 150 ms debounce
- Produces: `AppModel::reload_path(path: &Path) -> Result<(), AppOpenError>`

- [ ] **Step 1: Write failing reload tests**

```rust
#[test]
fn successful_reload_replaces_content_without_reordering_tabs() {
    let fixture = open_two_files();
    let before = fixture.model.tabs.paths();
    std::fs::write(&fixture.first, "# Changed").unwrap();
    fixture.model.reload_path(&fixture.first).unwrap();
    assert_eq!(fixture.model.tabs.paths(), before);
    assert_eq!(fixture.model.tabs.get(&fixture.first).unwrap().document.title, "Changed");
}

#[test]
fn failed_reload_preserves_the_last_document() {
    let fixture = open_one_file();
    let before = fixture.model.tabs.active().unwrap().document.clone();
    std::fs::remove_file(&fixture.path).unwrap();
    assert!(fixture.model.reload_path(&fixture.path).is_err());
    let tab = fixture.model.tabs.active().unwrap();
    assert!(Arc::ptr_eq(&tab.document, &before));
    assert!(tab.reload_error.is_some());
}
```

- [ ] **Step 2: Run reload tests and confirm failure**

Run: `cargo test --manifest-path apps/gpui/Cargo.toml reload_`

Expected: compilation fails because `reload_path` is missing.

- [ ] **Step 3: Implement reload semantics**

Load and parse into temporary values first. On success call `TabSet::replace_document`; on failure call `TabSet::set_reload_error` and return the readable error. Never clear or replace the previous `Arc<ParsedDocument>` on failure.

- [ ] **Step 4: Implement notify watcher and debounce**

Create one `RecommendedWatcher` whose callback sends changed paths over `std::sync::mpsc`. A background thread coalesces paths for 150 ms and forwards `WatchMessage::Reload`. Watch each canonical parent directory once with `RecursiveMode::NonRecursive`; file paths remain the filter boundary.

Store the debounced receiver behind `Arc<Mutex<Receiver<WatchMessage>>>`. In `MdowApp`, run a
detached GPUI task that waits 100 ms with `Timer::after`, drains `try_recv`, updates the entity on
the foreground executor, and repeats while the weak entity can still upgrade. Retain the watcher,
receiver, and detached task for the entity lifetime; never block GPUI's foreground executor on
`recv`.

- [ ] **Step 5: Complete action behavior and menu bar**

Bind root `.on_action` handlers to `OpenFile`, `OpenFolder`, `ToggleSidebar`, `CloseTab`, and `ToggleWideMode`. `ToggleSidebar` changes only width/presentation state; `ToggleWideMode` retains 48 px padding. Add File menu Open File, Open Folder, Close Tab, and Quit items with registered actions.

- [ ] **Step 6: Accept an optional launch path**

Read only the first non-flag CLI argument. After constructing `MdowApp`, call the same `open_path` method for that file or directory. An invalid path opens the window and shows the normal error state instead of terminating the process.

- [ ] **Step 7: Run tests, build, and commit**

Run:

```bash
cargo test --manifest-path apps/gpui/Cargo.toml
cargo build --manifest-path apps/gpui/Cargo.toml
```

Expected: the entire Rust suite passes and the executable builds.

Commit:

```bash
git add apps/gpui/src
git commit -m "feat(gpui): add live reload and native actions"
```

## Task 8: Local Run Script and Visual Verification

**Files:**
- Create: `script/build_and_run_gpui.sh`
- Modify as required by visual findings: `apps/gpui/src/theme.rs`
- Modify as required by visual findings: `apps/gpui/src/ui/*.rs`

**Interfaces:**
- Produces: `./script/build_and_run_gpui.sh [--verify] [path]`

- [ ] **Step 1: Create the local run script**

Implement an executable zsh script that resolves the repository root from its own location, runs `cargo build --manifest-path "$repo_root/apps/gpui/Cargo.toml"`, locates `apps/gpui/target/debug/mdow-gpui`, terminates only an earlier process whose executable path exactly matches that binary, then launches the binary in the foreground with the optional path.

`--verify` starts the showcase fixture in the background, polls for that exact PID for up to ten seconds, prints `Mdow GPUI verification passed`, terminates only that PID, and exits zero. It must never use a broad process-name kill.

- [ ] **Step 2: Run automated and launch verification**

Run:

```bash
cargo test --manifest-path apps/gpui/Cargo.toml
cargo build --manifest-path apps/gpui/Cargo.toml
./script/build_and_run_gpui.sh --verify apps/gpui/tests/fixtures/showcase.md
```

Expected: tests/build exit zero and verify prints the success line.

- [ ] **Step 3: Capture the GPUI states at fixed dimensions**

Launch and capture 1120×760 screenshots for:

1. Welcome state in light appearance.
2. Showcase document plus folder sidebar in light appearance.
3. Showcase document plus folder sidebar in dark appearance.
4. Two tabs with the second tab active.

Use macOS screenshot tooling on the Mdow GPUI window only. Record the screenshot paths in the task notes; do not commit temporary screenshots.

- [ ] **Step 4: Compare against authoritative measurements**

Verify with screenshots and source inspection:

- Inter and Geist Mono are visibly loaded rather than falling back.
- Sidebar is 244 px; titlebar inset 28 px; tab bar 36 px; tabs 28 px; breadcrumb 28 px.
- Constrained content width is 768 px and centered with 48 px horizontal padding.
- Body is 15.5 px/1.65; H1/H2/H3 ratios and margins match the CSS reference.
- Warm light and neutral dark palette relationships match established Mdow.
- Active row accent, active tab lift, borders, table grid, blockquote rule, code card, and empty-state hierarchy are present.
- No chat icon, panel, AI copy, split-view icon, or reserved companion space exists.

Correct any mismatch in the token or responsible UI module, rebuild, and recapture the affected state.

- [ ] **Step 5: Exercise runtime interactions**

Manually verify Open File, Open Folder, drag/drop, folder disclosure, file selection, tab switching, tab close, sidebar toggle, wide mode, code copy feedback, external HTTP link, local Markdown link, local image, live file reload, missing-file reload banner, and every specified shortcut.

- [ ] **Step 6: Run the final completion audit**

Run:

```bash
cargo test --manifest-path apps/gpui/Cargo.toml
cargo build --manifest-path apps/gpui/Cargo.toml
./script/build_and_run_gpui.sh --verify apps/gpui/tests/fixtures/showcase.md
rg -n "companion|chat|ai[-_ ]|openai|anthropic|ACP" apps/gpui script/build_and_run_gpui.sh
git diff --check
```

Expected: tests, build, and launch verification pass; the AI search has no matches; diff check is clean.

- [ ] **Step 7: Commit the run helper and visual corrections**

```bash
git add script/build_and_run_gpui.sh apps/gpui
git commit -m "feat(gpui): finish local Mdow prototype"
```

## Final Evidence Required Before Completion

- `apps/gpui/Cargo.toml` pins GPUI 0.2.2 and the lockfile is present.
- Automated tests directly cover every pure domain requirement listed in the design spec.
- A fresh debug build exits zero.
- The local verify command proves the real binary launches with the showcase fixture.
- Fixed-size light/dark screenshots prove the visual contract rather than relying on source values alone.
- Manual interaction results cover the dialogs, drop path, sidebar, tabs, links, copy feedback, live reload, and shortcuts.
- Search evidence proves no AI chat surface or runtime exists in the GPUI package.
