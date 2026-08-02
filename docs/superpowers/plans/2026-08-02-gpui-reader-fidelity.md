# Mdow GPUI Reader Fidelity Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the local Rust + GPUI Mdow reader reliably scrollable, syntax highlighted, and visually faithful to the established Electron reader without adding AI chat or secondary product areas.

**Architecture:** Keep Markdown parsing, syntax preparation, and GPUI rendering separate. Syntect prepares light and dark token runs once per loaded document; tabs retain that prepared presentation, while the reader renders a bounded native scroll viewport with one stable `ScrollHandle` per canonical path. Chrome and prose use measured constants copied from the shipping Electron components.

**Tech Stack:** Rust 1.93+, GPUI 0.2.2, pulldown-cmark 0.13.4, Syntect 5.3.0, native macOS test support.

## Global Constraints

- Target macOS 14 or newer on Apple Silicon for this local prototype.
- Keep GPUI pinned to `=0.2.2`, pulldown-cmark pinned to `=0.13.4`, and add Syntect pinned to `=5.3.0`.
- Use bundled Inter for UI/prose and bundled Geist Mono for code; do not substitute system glyphs or emoji for icons.
- The Electron app is the visual source of truth; compare at the same 1120 × 760 viewport, appearance, open-tab state, and document position.
- Preserve the minimal product scope: no AI chat, companion UI, search, recents, outline, settings, editing, split view, Mermaid, math, or raw HTML/MDX execution.
- Keep changes inside `apps/gpui/`, `script/build_and_run_gpui.sh`, and the GPUI specification/plan files.
- Do not modify Electron or Swift behavior and do not add pnpm tasks, CI, packaging, signing, notarization, updates, telemetry, or distribution work.
- Follow red-green-refactor: every behavior change starts with a focused failing test and ends with the narrow and full relevant suites passing.

## File Map

- Create `apps/gpui/src/syntax.rs`: language normalization, Syntect initialization, GitHub-inspired themes, owned token-run types, plain-text fallback, and document preparation.
- Modify `apps/gpui/Cargo.toml`: add the exact Syntect dependency.
- Modify `apps/gpui/Cargo.lock`: lock Syntect and its transitive dependencies.
- Modify `apps/gpui/src/lib.rs`: export the syntax module.
- Modify `apps/gpui/src/tabs.rs`: store prepared documents while preserving the current tab API for parser-only tests.
- Modify `apps/gpui/src/app.rs`: prepare documents on open/reload, keep stable scroll handles, support reader keyboard scrolling, and add GPUI behavior tests.
- Modify `apps/gpui/src/document.rs`: extract a small frontmatter title without rendering the frontmatter as Markdown.
- Modify `apps/gpui/src/theme.rs`: add measured tab/reader constants and explicit light/dark scheme metadata.
- Modify `apps/gpui/src/ui/reader.rs`: establish the real vertical scroll layout, render highlighted code runs, and align Markdown surfaces with `markdown.css`.
- Modify `apps/gpui/src/ui/chrome.rs`: match tab, breadcrumb, separator, and sidebar-empty-state geometry.
- Modify `apps/gpui/tests/fixtures/showcase.md`: make the deterministic fixture long and comprehensive enough for scrolling, highlighting, and Markdown-fidelity QA.

---

### Task 1: Syntect Highlighting Foundation

**Files:**
- Create: `apps/gpui/src/syntax.rs`
- Modify: `apps/gpui/Cargo.toml:7-13`
- Modify: `apps/gpui/Cargo.lock`
- Modify: `apps/gpui/src/lib.rs:1-9`
- Test: `apps/gpui/src/syntax.rs` unit-test module

**Interfaces:**
- Consumes: `document::ParsedDocument` and `document::DocumentBlock`.
- Produces: `SyntaxColor`, `HighlightedRun`, `HighlightedCode`, `PreparedDocument`, `normalize_language(&str) -> String`, `highlight_code(Option<&str>, &str) -> HighlightedCode`, and `prepare_document(ParsedDocument) -> PreparedDocument`.

- [ ] **Step 1: Add failing tests for language normalization, tokenization, palette selection, and fallback**

Add this test module at the bottom of the new file before defining the implementation:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parse_document;
    use std::path::PathBuf;

    #[test]
    fn normalizes_electron_language_aliases() {
        assert_eq!(normalize_language(" language-TS "), "typescript");
        assert_eq!(normalize_language("js title=app.js"), "javascript");
        assert_eq!(normalize_language("rs"), "rust");
        assert_eq!(normalize_language("zsh"), "bash");
        assert_eq!(normalize_language("yml"), "yaml");
        assert_eq!(normalize_language("c++"), "cpp");
    }

    #[test]
    fn rust_highlighting_preserves_text_and_emits_multiple_colors() {
        let code = "fn main() { println!(\"hello\"); }\n";
        let highlighted = highlight_code(Some("rust"), code);

        assert_eq!(highlighted.text, code);
        assert!(highlighted.light_runs.len() > 1);
        assert!(highlighted.dark_runs.len() > 1);
        assert_ne!(highlighted.light_runs, highlighted.dark_runs);
        assert_eq!(highlighted.light_runs.iter().map(|run| run.len).sum::<usize>(), code.len());
        assert_eq!(highlighted.dark_runs.iter().map(|run| run.len).sum::<usize>(), code.len());
    }

    #[test]
    fn unknown_language_falls_back_to_one_plain_run() {
        let code = "alpha < beta\n";
        let highlighted = highlight_code(Some("not-a-real-language"), code);

        assert_eq!(highlighted.normalized_language.as_deref(), Some("not-a-real-language"));
        assert_eq!(highlighted.text, code);
        assert_eq!(highlighted.light_runs.len(), 1);
        assert_eq!(highlighted.dark_runs.len(), 1);
        assert_eq!(highlighted.light_runs[0].len, code.len());
    }

    #[test]
    fn prepares_highlights_by_markdown_block_index() {
        let document = parse_document(
            PathBuf::from("/tmp/code.md"),
            "before\n\n```rust\nlet answer = 42;\n```\n".into(),
        );
        let prepared = prepare_document(document);

        assert!(prepared.code_block(1).is_some());
        assert!(prepared.code_block(0).is_none());
    }
}
```

- [ ] **Step 2: Run the new test target and confirm the red state**

Run:

```sh
cargo test --manifest-path apps/gpui/Cargo.toml syntax::tests -- --nocapture
```

Expected: compilation fails because `syntax.rs` and the highlighted types/functions do not exist.

- [ ] **Step 3: Add Syntect and export the module**

Add the dependency and module export:

```toml
# apps/gpui/Cargo.toml
syntect = { version = "=5.3.0", default-features = false, features = ["default-fancy"] }
```

```rust
// apps/gpui/src/lib.rs
pub mod syntax;
```

Run `cargo check --manifest-path apps/gpui/Cargo.toml` once so Cargo updates
`apps/gpui/Cargo.lock` with the pinned graph.

- [ ] **Step 4: Implement owned highlighted types and language normalization**

Use UI-independent RGB bytes and byte lengths so the module does not depend on GPUI:

```rust
use crate::document::{DocumentBlock, ParsedDocument};
use std::{collections::HashMap, ops::Deref, sync::OnceLock};
use syntect::{
    easy::HighlightLines,
    highlighting::{Color, FontStyle, StyleModifier, Theme, ThemeItem, ThemeSettings},
    parsing::{ScopeSelectors, SyntaxSet},
    util::LinesWithEndings,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightedRun {
    pub len: usize,
    pub color: SyntaxColor,
    pub italic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightedCode {
    pub normalized_language: Option<String>,
    pub text: String,
    pub light_runs: Vec<HighlightedRun>,
    pub dark_runs: Vec<HighlightedRun>,
}

#[derive(Debug, Clone)]
pub struct PreparedDocument {
    parsed: ParsedDocument,
    code_blocks: HashMap<usize, HighlightedCode>,
}

impl Deref for PreparedDocument {
    type Target = ParsedDocument;

    fn deref(&self) -> &Self::Target {
        &self.parsed
    }
}

impl PreparedDocument {
    pub fn plain(parsed: ParsedDocument) -> Self {
        Self { parsed, code_blocks: HashMap::new() }
    }

    pub fn code_block(&self, block_index: usize) -> Option<&HighlightedCode> {
        self.code_blocks.get(&block_index)
    }

    pub(crate) fn set_path(&mut self, path: std::path::PathBuf) {
        self.parsed.path = path;
    }
}

pub fn normalize_language(info: &str) -> String {
    let raw = info
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let raw = raw.strip_prefix("language-").unwrap_or(&raw);
    match raw {
        "js" => "javascript",
        "ts" => "typescript",
        "py" => "python",
        "rs" => "rust",
        "sh" | "shell" | "zsh" => "bash",
        "yml" => "yaml",
        "md" => "markdown",
        "rb" => "ruby",
        "cs" => "csharp",
        "c++" => "cpp",
        other => other,
    }
    .to_owned()
}
```

- [ ] **Step 5: Implement one-time syntax/theme loading and exact fallback behavior**

Create `OnceLock<SyntaxSet>` and `OnceLock<(Theme, Theme)>`. Build the light and dark themes with
the specification's exact anchor colors and Syntect scope selectors:

```rust
const LIGHT_DEFAULT: SyntaxColor = SyntaxColor { red: 0x24, green: 0x29, blue: 0x2f };
const DARK_DEFAULT: SyntaxColor = SyntaxColor { red: 0xe6, green: 0xed, blue: 0xf3 };

fn syntax_set() -> &'static SyntaxSet {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn syntect_color(hex: u32) -> Color {
    let [_, r, g, b] = hex.to_be_bytes();
    Color { r, g, b, a: 0xff }
}

fn theme_item(scope: &str, foreground: u32, italic: bool) -> ThemeItem {
    ThemeItem {
        scope: scope.parse::<ScopeSelectors>().expect("valid syntax scope selector"),
        style: StyleModifier {
            foreground: Some(syntect_color(foreground)),
            background: None,
            font_style: italic.then_some(FontStyle::ITALIC),
        },
    }
}

fn github_theme(name: &str, default: u32, comment: u32, keyword: u32, string: u32, function: u32, constant: u32) -> Theme {
    Theme {
        name: Some(name.into()),
        author: Some("Mdow".into()),
        settings: ThemeSettings {
            foreground: Some(syntect_color(default)),
            ..ThemeSettings::default()
        },
        scopes: vec![
            theme_item("comment", comment, true),
            theme_item("keyword, storage", keyword, false),
            theme_item("string", string, false),
            theme_item("entity.name.function, entity.name.type, support.type", function, false),
            theme_item("constant, constant.numeric", constant, false),
        ],
    }
}

fn github_themes() -> &'static (Theme, Theme) {
    static THEMES: OnceLock<(Theme, Theme)> = OnceLock::new();
    THEMES.get_or_init(|| {
        (
            github_theme("Mdow GitHub Light", 0x24292f, 0x6e7781, 0xcf222e, 0x0a3069, 0x8250df, 0x0550ae),
            github_theme("Mdow GitHub Dark", 0xe6edf3, 0x8b949e, 0xff7b72, 0xa5d6ff, 0xd2a8ff, 0x79c0ff),
        )
    })
}

fn plain_run(code: &str, color: SyntaxColor) -> Vec<HighlightedRun> {
    (!code.is_empty())
        .then_some(HighlightedRun { len: code.len(), color, italic: false })
        .into_iter()
        .collect()
}

fn syntax_for<'a>(set: &'a SyntaxSet, language: &str) -> Option<&'a syntect::parsing::SyntaxReference> {
    set.find_syntax_by_token(language)
        .or_else(|| set.find_syntax_by_extension(language))
        .or_else(|| set.find_syntax_by_name(language))
}

fn runs_for(code: &str, syntax: &syntect::parsing::SyntaxReference, theme: &Theme) -> Vec<HighlightedRun> {
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut runs = Vec::new();
    for line in LinesWithEndings::from(code) {
        let Ok(parts) = highlighter.highlight_line(line, syntax_set()) else {
            return Vec::new();
        };
        runs.extend(parts.into_iter().map(|(style, text)| HighlightedRun {
            len: text.len(),
            color: SyntaxColor {
                red: style.foreground.r,
                green: style.foreground.g,
                blue: style.foreground.b,
            },
            italic: style.font_style.contains(FontStyle::ITALIC),
        }));
    }
    runs
}

pub fn highlight_code(language: Option<&str>, code: &str) -> HighlightedCode {
    let normalized_language = language
        .map(normalize_language)
        .filter(|language| !language.is_empty());
    let fallback = || HighlightedCode {
        normalized_language: normalized_language.clone(),
        text: code.to_owned(),
        light_runs: plain_run(code, LIGHT_DEFAULT),
        dark_runs: plain_run(code, DARK_DEFAULT),
    };
    let Some(language) = normalized_language.as_deref() else {
        return fallback();
    };
    let Some(syntax) = syntax_for(syntax_set(), language) else {
        return fallback();
    };
    let (light_theme, dark_theme) = github_themes();
    let light_runs = runs_for(code, syntax, light_theme);
    let dark_runs = runs_for(code, syntax, dark_theme);
    if (!code.is_empty() && light_runs.is_empty()) || (!code.is_empty() && dark_runs.is_empty()) {
        return fallback();
    }
    HighlightedCode {
        normalized_language,
        text: code.to_owned(),
        light_runs,
        dark_runs,
    }
}
```

Keep the explicit mapping shown above: comments `#6e7781/#8b949e`, keyword/storage
`#cf222e/#ff7b72`, strings `#0a3069/#a5d6ff`, entity/type/function `#8250df/#d2a8ff`, and
constant/number `#0550ae/#79c0ff`. Do not silently substitute a bundled Syntect theme.

Implement document preparation by enumerating Markdown blocks:

```rust
pub fn prepare_document(parsed: ParsedDocument) -> PreparedDocument {
    let code_blocks = parsed
        .blocks
        .iter()
        .enumerate()
        .filter_map(|(index, block)| match block {
            DocumentBlock::CodeBlock { language, code } => {
                Some((index, highlight_code(language.as_deref(), code)))
            }
            _ => None,
        })
        .collect();
    PreparedDocument { parsed, code_blocks }
}
```

- [ ] **Step 6: Run syntax tests and the full library suite**

Run:

```sh
cargo test --manifest-path apps/gpui/Cargo.toml syntax::tests -- --nocapture
cargo test --manifest-path apps/gpui/Cargo.toml --lib
```

Expected: all syntax tests pass; all existing library tests remain green.

- [ ] **Step 7: Commit the syntax foundation**

```sh
git add apps/gpui/Cargo.toml apps/gpui/Cargo.lock apps/gpui/src/lib.rs apps/gpui/src/syntax.rs
git commit -m "feat(gpui): prepare syntax-highlighted code"
```

---

### Task 2: Prepared Documents in Tabs and Reloads

**Files:**
- Modify: `apps/gpui/src/tabs.rs:1-115`
- Modify: `apps/gpui/src/app.rs:1-145`
- Test: `apps/gpui/src/tabs.rs` unit-test module
- Test: `apps/gpui/src/app.rs:906-965`

**Interfaces:**
- Consumes: `syntax::PreparedDocument`, `syntax::prepare_document(ParsedDocument)`.
- Produces: `DocumentTab::document: Arc<PreparedDocument>`, `TabSet::open_prepared(PreparedDocument)`, and `TabSet::replace_prepared(PreparedDocument) -> bool`.

- [ ] **Step 1: Write failing tests for prepared open and prepared reload**

Add focused tests that leave the existing parser-only helpers intact:

```rust
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
    assert!(tabs.get(Path::new("/tmp/a.md")).unwrap().document.code_block(0).is_some());
}
```

- [ ] **Step 2: Run the focused tests and confirm the missing-method failures**

```sh
cargo test --manifest-path apps/gpui/Cargo.toml tabs::tests::prepared -- --nocapture
```

Expected: compilation fails because `open_prepared` and `replace_prepared` do not exist.

- [ ] **Step 3: Store `PreparedDocument` while preserving parser-only entry points**

Change `DocumentTab` and add paired methods:

```rust
use crate::{
    document::ParsedDocument,
    syntax::{PreparedDocument, prepare_document},
};

pub struct DocumentTab {
    pub document: Arc<PreparedDocument>,
    pub last_source: Arc<str>,
    pub reload_error: Option<String>,
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
}

fn canonical_prepared_document(mut document: PreparedDocument) -> PreparedDocument {
    let path = path_identity(&document.path);
    document.set_path(path);
    document
}
```

- [ ] **Step 4: Prepare production documents during open and reload**

Update the model flow:

```rust
use crate::syntax::prepare_document;

pub fn open_document(&mut self, path: &Path) -> Result<(), AppOpenError> {
    let loaded = load_source(path)?;
    let parsed = parse_document(loaded.canonical_path, loaded.source);
    self.tabs.open_prepared(prepare_document(parsed));
    Ok(())
}

pub fn reload_path(&mut self, path: &Path) -> Result<(), AppOpenError> {
    let tab_path = canonical_file_identity(path);
    let loaded = match load_source(path) {
        Ok(loaded) => loaded,
        Err(error) => {
            let error = AppOpenError::from(error);
            self.tabs
                .set_reload_error(&tab_path, error.view().body.clone());
            return Err(error);
        }
    };
    let parsed = parse_document(loaded.canonical_path, loaded.source);
    self.tabs.replace_prepared(prepare_document(parsed));
    Ok(())
}
```

- [ ] **Step 5: Run tab, model, reload, and full library tests**

```sh
cargo test --manifest-path apps/gpui/Cargo.toml tabs::tests -- --nocapture
cargo test --manifest-path apps/gpui/Cargo.toml app::tests::successful_reload -- --nocapture
cargo test --manifest-path apps/gpui/Cargo.toml --lib
```

Expected: prepared documents survive open/reload; existing tab identity, last-good content, and
reload-error behavior remain green.

- [ ] **Step 6: Commit prepared-document integration**

```sh
git add apps/gpui/src/tabs.rs apps/gpui/src/app.rs
git commit -m "refactor(gpui): retain prepared reader documents"
```

---

### Task 3: Native Highlighted Code Rendering

**Files:**
- Modify: `apps/gpui/src/theme.rs:1-130`
- Modify: `apps/gpui/src/ui/reader.rs:1-20,693-760,860-890,1330-1425`
- Modify: `apps/gpui/src/app.rs:686-742,1515-1545`
- Test: `apps/gpui/src/ui/reader.rs` unit-test module
- Test: `apps/gpui/src/app.rs` GPUI-test module

**Interfaces:**
- Consumes: `PreparedDocument::code_block`, `HighlightedCode`, `HighlightedRun`, and the active `Theme`.
- Produces: `highlighted_text_runs(&HighlightedCode, bool) -> Vec<TextRun>` and highlighted fenced-code output without changing copy semantics.

- [ ] **Step 1: Write failing unit tests for GPUI token conversion**

Add a small conversion test in `reader.rs`:

```rust
#[test]
fn highlighted_runs_keep_lengths_fonts_and_theme_colors() {
    let highlighted = highlight_code(Some("rust"), "fn main() {}\n");
    let light = highlighted_text_runs(&highlighted, false);
    let dark = highlighted_text_runs(&highlighted, true);

    assert_eq!(light.iter().map(|run| run.len).sum::<usize>(), highlighted.text.len());
    assert_eq!(dark.iter().map(|run| run.len).sum::<usize>(), highlighted.text.len());
    assert!(light.iter().all(|run| run.font.family.as_ref() == Metrics::FONT_MONO));
    assert_ne!(light[0].color, dark[0].color);
}
```

Add a GPUI test that opens a prepared Rust block and asserts both the code surface and exact copy
result remain available:

```rust
#[gpui::test]
fn prepared_code_renders_and_copy_keeps_original_source(cx: &mut TestAppContext) {
    let code = "fn main() {\n    println!(\"Hello\");\n}\n";
    let document = prepare_document(parse_document(
        PathBuf::from("/tmp/highlighted.md"),
        format!("```rust\n{code}```\n"),
    ));
    let window = cx.update(|cx| {
        cx.open_window(Default::default(), |window, cx| {
            cx.new(|cx| {
                let mut app = MdowApp::new(window, cx);
                app.model.tabs.open_prepared(document);
                app.open_error = None;
                app
            })
        })
        .unwrap()
    });
    let mut visual = VisualTestContext::from_window((*window).into(), cx);
    visual.update(|window, cx| window.draw(cx).clear());

    assert!(visual.debug_bounds("reader-code-0").is_some());
    click_debug(&mut visual, "copy-code-0");
    visual.update(|window, cx| window.draw(cx).clear());
    assert_eq!(
        cx.read_from_clipboard().and_then(|item| item.text()),
        Some(code.to_owned()),
    );
}
```

- [ ] **Step 2: Run the focused tests and confirm the red state**

```sh
cargo test --manifest-path apps/gpui/Cargo.toml highlighted_runs_keep -- --nocapture
cargo test --manifest-path apps/gpui/Cargo.toml prepared_code_renders -- --nocapture
```

Expected: compilation fails because highlighted run conversion and prepared-code rendering are not
wired into the reader.

- [ ] **Step 3: Expose explicit appearance metadata in `Theme`**

Add a field that avoids guessing from background color:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorScheme {
    Light,
    Dark,
}

pub struct Theme {
    pub color_scheme: ColorScheme,
}
```

Insert `color_scheme` before the current color fields rather than deleting those fields. Set
`ColorScheme::Light` in `Theme::light()` and `ColorScheme::Dark` in `Theme::dark()`. Update theme
equality tests to include the field.

- [ ] **Step 4: Convert syntax RGB bytes into GPUI `TextRun` values**

Implement the conversion with the existing Geist Mono metrics:

```rust
fn highlighted_text_runs(highlighted: &HighlightedCode, dark: bool) -> Vec<TextRun> {
    let source = if dark { &highlighted.dark_runs } else { &highlighted.light_runs };
    source
        .iter()
        .map(|run| {
            let hex = ((run.color.red as u32) << 16)
                | ((run.color.green as u32) << 8)
                | run.color.blue as u32;
            let mut run_font = font(Metrics::FONT_MONO);
            run_font.weight = FontWeight::NORMAL;
            run_font.style = if run.italic { FontStyle::Italic } else { FontStyle::Normal };
            TextRun {
                len: run.len,
                font: run_font,
                color: gpui::Hsla::from(gpui::rgb(hex)),
                background_color: None,
                underline: None,
                strikethrough: None,
            }
        })
        .collect()
}
```

- [ ] **Step 5: Pass prepared block data to the renderer and render `StyledText`**

Change `render_document` to accept `Arc<PreparedDocument>`. During block enumeration, pass
`document.code_block(block_index)` into `render_block`, then into `render_code_block`.

Replace the plain `.child(code.to_owned())` with:

```rust
let highlighted_text = highlighted
    .map(|value| {
        StyledText::new(value.text.clone()).with_runs(highlighted_text_runs(
            value,
            theme.color_scheme == ColorScheme::Dark,
        ))
    })
    .unwrap_or_else(|| StyledText::new(code.to_owned()));

restrict_scroll_to_axis(div())
    .id(("code-scroll", document_scoped_element_id(document_path, "code-scroll", block_index)))
    .debug_selector(move || format!("reader-code-{block_index}"))
    .w_full()
    .overflow_x_scroll()
    .scrollbar_width(px(6.0))
    .px(px(18.0))
    .py(px(14.0))
    .font_family(Metrics::FONT_MONO)
    .font_weight(FontWeight::NORMAL)
    .text_size(px(15.5 * 0.875))
    .line_height(px(15.5 * 0.875 * 1.6))
    .whitespace_nowrap()
    .child(highlighted_text)
```

Use `highlighted.normalized_language` for the lowercase badge when prepared data exists; retain the
parser label for the plain fallback. Keep `code_to_copy = code.to_owned()` unchanged.

- [ ] **Step 6: Run focused rendering tests and full GPUI tests**

```sh
cargo test --manifest-path apps/gpui/Cargo.toml highlighted -- --nocapture
cargo test --manifest-path apps/gpui/Cargo.toml code_copy -- --nocapture
cargo test --manifest-path apps/gpui/Cargo.toml
```

Expected: known code produces multiple theme-aware runs, unknown code remains plain, and exact
clipboard behavior stays green.

- [ ] **Step 7: Commit highlighted rendering**

```sh
git add apps/gpui/src/theme.rs apps/gpui/src/ui/reader.rs apps/gpui/src/app.rs
git commit -m "feat(gpui): render syntax-highlighted code"
```

---

### Task 4: Functional Document Scrolling and Position Restoration

**Files:**
- Modify: `apps/gpui/src/ui/reader.rs:693-760`
- Modify: `apps/gpui/src/app.rs:230-245,560-600,723-760,1481-1513,1882-1930`
- Test: `apps/gpui/src/app.rs` GPUI-test module

**Interfaces:**
- Consumes: `MdowApp::reader_scroll_handles: HashMap<PathBuf, ScrollHandle>`.
- Produces: a content-sized `reader-column`, a bounded `reader-scroll` viewport, `scroll_active_reader(&str, &mut Context<Self>)`, and verified native wheel/keyboard behavior.

- [ ] **Step 1: Replace the structural-only test with a failing wheel-scroll test**

Extend the existing long-reader GPUI test after the first draw:

```rust
let bounds = visual.debug_bounds("reader-scroll").expect("reader viewport");
let handle = window
    .update(cx, |app, _, _| app.reader_scroll_handles.values().next().unwrap().clone())
    .unwrap();

assert!(handle.max_offset().height > px(0.0));
visual.simulate_event(ScrollWheelEvent {
    position: bounds.center(),
    delta: ScrollDelta::Pixels(point(px(0.0), px(-180.0))),
    ..Default::default()
});
visual.update(|window, cx| window.draw(cx).clear());

assert!(handle.offset().y < px(0.0));
```

Also retain the current assertions that there is exactly one reader scroll surface and wrapped
paragraph height exceeds 40 px.

- [ ] **Step 2: Run the wheel test and confirm it fails for lack of scroll extent**

```sh
cargo test --manifest-path apps/gpui/Cargo.toml active_document_renders_one_scroll -- --nocapture
```

Expected: FAIL because `max_offset().height` is zero or the wheel leaves the offset unchanged.

- [ ] **Step 3: Fix the flex ownership of the viewport and document column**

Make the content column non-shrinking and the viewport explicitly vertical:

```rust
let mut column = div()
    .id("reader-column")
    .flex()
    .flex_col()
    .flex_none()
    .w_full()
    .min_w_0()
    .px(px(Metrics::READER_INSET))
    .pt(px(Metrics::READER_TOP_PADDING))
    .pb(px(Metrics::READER_BOTTOM_PADDING))
    .font_family(Metrics::FONT_SANS)
    .font_weight(FontWeight::NORMAL)
    .text_size(px(15.5))
    .line_height(px(15.5 * 1.65))
    .text_color(theme.foreground)
    .when(!wide_mode, |column| {
        column.max_w(px(Metrics::READER_MAX_WIDTH)).mx_auto()
    });

div()
    .id("reader-scroll")
    .flex()
    .flex_col()
    .flex_grow()
    .min_w_0()
    .min_h_0()
    .overflow_y_scroll()
    .scrollbar_width(px(6.0))
    .track_scroll(scroll_handle)
    .child(column)
```

Do not add vertical overflow to code or table wrappers. Keep their existing axis restriction so a
vertical wheel delta bubbles to `reader-scroll`.

- [ ] **Step 4: Add failing tests for Page Down, Home, End, and per-tab retention**

Add one pure helper test around a new scroll-target calculation and one GPUI state test:

```rust
#[test]
fn reader_key_targets_are_clamped_to_scroll_extent() {
    assert_eq!(reader_key_target("home", -240.0, 600.0, 1600.0), Some(0.0));
    assert_eq!(reader_key_target("end", -240.0, 600.0, 1600.0), Some(-1600.0));
    assert_eq!(reader_key_target("pagedown", -240.0, 600.0, 1600.0), Some(-780.0));
    assert_eq!(reader_key_target("pageup", -240.0, 600.0, 1600.0), Some(0.0));
}
```

In the GPUI test, set path A to `-120`, path B to `-260`, activate A/B/A, and assert both handles
retain their values. Retain the existing close/reopen test asserting the new handle starts at zero.

Use this state sequence after drawing both documents once:

```rust
window.update(cx, |app, _, cx| {
    app.activate_tab(&first, cx);
    app.reader_scroll_handles[&first].set_offset(point(px(0.0), px(-120.0)));
    app.activate_tab(&second, cx);
    app.reader_scroll_handles[&second].set_offset(point(px(0.0), px(-260.0)));
    app.activate_tab(&first, cx);
}).unwrap();

window.update(cx, |app, _, _| {
    assert_eq!(app.reader_scroll_handles[&first].offset().y, px(-120.0));
    assert_eq!(app.reader_scroll_handles[&second].offset().y, px(-260.0));
    assert_eq!(app.model.tabs.active().unwrap().path(), first);
}).unwrap();
```

- [ ] **Step 5: Implement keyboard targets using the active path's stable handle**

Add:

```rust
fn reader_key_target(key: &str, current: f32, viewport: f32, max: f32) -> Option<f32> {
    let page = viewport * 0.9;
    match key {
        "home" => Some(0.0),
        "end" => Some(-max),
        "pageup" => Some((current + page).min(0.0)),
        "pagedown" => Some((current - page).max(-max)),
        _ => None,
    }
}

fn scroll_active_reader(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
    let Some(path) = self.model.tabs.active().map(|tab| tab.path().to_owned()) else {
        return false;
    };
    let Some(handle) = self.reader_scroll_handles.get(&path) else {
        return false;
    };
    let Some(target) = reader_key_target(
        key,
        f32::from(handle.offset().y),
        f32::from(handle.bounds().size.height),
        f32::from(handle.max_offset().height),
    ) else {
        return false;
    };
    handle.set_offset(point(px(0.0), px(target)));
    cx.notify();
    true
}
```

Call this from the existing root `capture_key_down` before tab-focus handling when no platform,
control, alt, or function modifier is held. Stop propagation only when a reader key was handled.

- [ ] **Step 6: Run all scrolling, reload, tab-close, and GPUI tests**

```sh
cargo test --manifest-path apps/gpui/Cargo.toml scroll -- --nocapture
cargo test --manifest-path apps/gpui/Cargo.toml reload -- --nocapture
cargo test --manifest-path apps/gpui/Cargo.toml active_document_renders -- --nocapture
cargo test --manifest-path apps/gpui/Cargo.toml
```

Expected: wheel input moves the long document, keyboard targets clamp correctly, each tab retains
its own handle, reload preserves it, and close/reopen creates a fresh handle.

- [ ] **Step 7: Commit functional scrolling**

```sh
git add apps/gpui/src/ui/reader.rs apps/gpui/src/app.rs
git commit -m "fix(gpui): restore functional reader scrolling"
```

---

### Task 5: Measured Tab, Breadcrumb, and Metadata Fidelity

**Files:**
- Modify: `apps/gpui/src/document.rs:92-101,362-675`
- Modify: `apps/gpui/src/theme.rs:3-24,60-90`
- Modify: `apps/gpui/src/ui/chrome.rs:297-510`
- Modify: `apps/gpui/src/ui/reader.rs:1918-1975` (test fixtures gain `frontmatter_title: None`)
- Modify: `apps/gpui/src/app.rs` chrome GPUI tests
- Test: `apps/gpui/src/document.rs` unit-test module
- Test: `apps/gpui/src/theme.rs` unit-test module
- Test: `apps/gpui/src/ui/chrome.rs` unit-test module

**Interfaces:**
- Produces: `ParsedDocument::frontmatter_title: Option<String>`, `breadcrumb_display(&DocumentTab) -> BreadcrumbDisplay`, and measured `Metrics` constants used by chrome and reader.

- [ ] **Step 1: Add failing frontmatter and breadcrumb-display tests**

```rust
fn document_tab(path: &str, source: &str) -> DocumentTab {
    let parsed = parse_document(PathBuf::from(path), source.to_owned());
    let last_source = Arc::from(parsed.source.clone());
    DocumentTab {
        document: Arc::new(PreparedDocument::plain(parsed)),
        last_source,
        reload_error: None,
    }
}

#[test]
fn frontmatter_title_is_extracted_and_not_rendered_as_markdown() {
    let parsed = parse_document(
        PathBuf::from("/tmp/showcase.md"),
        "---\ntitle: Reader title\n---\n# Visible heading\n".into(),
    );

    assert_eq!(parsed.frontmatter_title.as_deref(), Some("Reader title"));
    assert_eq!(parsed.blocks.len(), 1);
    assert_eq!(parsed.title, "Visible heading");
}

#[test]
fn breadcrumb_uses_filename_until_frontmatter_supplies_a_title() {
    let plain = document_tab("/tmp/showcase.md", "# Heading\n");
    assert_eq!(breadcrumb_display(&plain), BreadcrumbDisplay {
        primary: "showcase.md".into(),
        secondary: None,
    });

    let titled = document_tab(
        "/tmp/showcase.md",
        "---\ntitle: Reader title\n---\n# Heading\n",
    );
    assert_eq!(breadcrumb_display(&titled), BreadcrumbDisplay {
        primary: "Reader title".into(),
        secondary: Some("showcase.md".into()),
    });
}
```

- [ ] **Step 2: Run focused metadata tests and confirm the red state**

```sh
cargo test --manifest-path apps/gpui/Cargo.toml frontmatter_title -- --nocapture
cargo test --manifest-path apps/gpui/Cargo.toml breadcrumb_uses_filename -- --nocapture
```

Expected: compilation fails because the frontmatter field and breadcrumb display helper are absent.

- [ ] **Step 3: Extract only a leading YAML title and parse the Markdown body**

Add `frontmatter_title` to `ParsedDocument`. Implement a bounded helper that recognizes only a
leading `---` block and a scalar `title:` line:

```rust
fn split_frontmatter(source: &str) -> (Option<String>, &str) {
    let Some(rest) = source.strip_prefix("---\n").or_else(|| source.strip_prefix("---\r\n")) else {
        return (None, source);
    };
    let mut consumed = source.len() - rest.len();
    let mut title = None;
    for line in rest.split_inclusive('\n') {
        consumed += line.len();
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" {
            return (title, &source[consumed..]);
        }
        if title.is_none()
            && let Some(value) = trimmed.strip_prefix("title:")
        {
            let value = value.trim().trim_matches(['\'', '"']);
            if !value.is_empty() {
                title = Some(value.to_owned());
            }
        }
    }
    (None, source)
}
```

Call `Parser::new_ext(markdown_body, options)` while retaining the full original `source` in the
model. Store `frontmatter_title`; keep the existing H1-or-filename `title` field for current model
behavior outside the breadcrumb.

- [ ] **Step 4: Add measured constants and update metric tests**

```rust
impl Metrics {
    pub const TAB_LIST_INSET: f32 = 6.0;
    pub const TAB_GAP: f32 = 1.0;
    pub const TAB_RADIUS: f32 = 6.0;
    pub const TAB_CONTENT_INSET: f32 = 10.0;
    pub const TAB_CONTENT_GAP: f32 = 6.0;
    pub const TAB_ICON_SIZE: f32 = 14.0;
    pub const TAB_CLOSE_SIZE: f32 = 24.0;
    pub const TAB_CLOSE_END_MARGIN: f32 = 4.0;
    pub const TAB_TOGGLE_SLOT: f32 = 36.0;
    pub const READER_TOP_PADDING: f32 = 32.0;
}
```

Extend `compact_shell_allocates_established_chrome_and_reader_regions` to assert that
`(TAB_BAR_HEIGHT - TAB_HEIGHT) / 2.0 == 4.0`, `TAB_RADIUS == 6.0`, and
`READER_TOP_PADDING == 32.0`.

- [ ] **Step 5: Apply the constants to the tab rail and isolate the sidebar toggle**

Update `render_tab_bar` so the toggle lives in a fixed 36 px slot with a trailing subtle divider,
while `tabs-scroll` retains its independent 6 px padding. Replace inline values with the constants:

```rust
let toggle_slot = div()
    .flex()
    .items_center()
    .justify_center()
    .w(px(Metrics::TAB_TOGGLE_SLOT))
    .h_full()
    .flex_none()
    .border_r_1()
    .border_color(theme.border_subtle)
    .child(icon_button(
        "toggle-sidebar",
        "icons/sidebar.svg",
        theme,
        |_, _, cx| cx.dispatch_action(&ToggleSidebar),
    ));

let tabs = div()
    .gap(px(Metrics::TAB_GAP))
    .px(px(Metrics::TAB_LIST_INSET));

let tab = div()
    .h(px(Metrics::TAB_HEIGHT))
    .rounded(px(Metrics::TAB_RADIUS));
```

Use `TAB_CONTENT_INSET`, `TAB_CONTENT_GAP`, `TAB_ICON_SIZE`, `TAB_CLOSE_SIZE`, and
`TAB_CLOSE_END_MARGIN` for the tab's children. Keep the existing active ring/card and muted inactive
behavior; verify hover/focus never changes dimensions.

- [ ] **Step 6: Render filename/frontmatter breadcrumb semantics**

Add:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreadcrumbDisplay {
    pub primary: String,
    pub secondary: Option<String>,
}

pub fn breadcrumb_display(tab: &DocumentTab) -> BreadcrumbDisplay {
    let filename = tab
        .path()
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Untitled")
        .to_owned();
    match tab.document.frontmatter_title.as_ref() {
        Some(title) => BreadcrumbDisplay {
            primary: title.clone(),
            secondary: Some(filename),
        },
        None => BreadcrumbDisplay { primary: filename, secondary: None },
    }
}
```

Render the primary label at 11 px medium foreground/85%; when present, render the secondary filename
at 10 px muted/60% with 4 px leading separation. Keep the 28 px row, 12 px outer inset, 8 px control
gap, and 2 px internal path gaps.

- [ ] **Step 7: Run metadata, chrome, theme, and full tests**

```sh
cargo test --manifest-path apps/gpui/Cargo.toml document::tests -- --nocapture
cargo test --manifest-path apps/gpui/Cargo.toml theme::tests -- --nocapture
cargo test --manifest-path apps/gpui/Cargo.toml breadcrumb -- --nocapture
cargo test --manifest-path apps/gpui/Cargo.toml
```

Expected: frontmatter is hidden from rendered blocks, breadcrumb labels match Electron semantics,
and all measured chrome tests pass.

- [ ] **Step 8: Commit measured chrome and metadata**

```sh
git add apps/gpui/src/document.rs apps/gpui/src/theme.rs apps/gpui/src/ui/chrome.rs apps/gpui/src/ui/reader.rs apps/gpui/src/app.rs
git commit -m "fix(gpui): match measured reader chrome"
```

---

### Task 6: Markdown Surface Polish, Comprehensive Fixture, and Final QA

**Files:**
- Modify: `apps/gpui/src/ui/reader.rs:20-130,600-690,693-760,1185-1435`
- Modify: `apps/gpui/src/ui/chrome.rs:48-110`
- Modify: `apps/gpui/tests/fixtures/showcase.md`
- Test: `apps/gpui/src/ui/reader.rs` unit-test module
- Test: `apps/gpui/src/app.rs` GPUI-test module

**Interfaces:**
- Consumes: measured `Metrics`, functional scroll viewport, prepared code runs, and existing block-spacing helpers.
- Produces: final Electron-parity prose/code/table/list/sidebar presentation and the deterministic QA fixture.

- [ ] **Step 1: Add failing metric and composition assertions for the remaining reader surfaces**

Extend reader unit tests with exact source-of-truth values:

```rust
#[test]
fn reader_surface_metrics_match_markdown_css() {
    assert_eq!(BlockStyle::body().font_size, 15.5);
    assert_eq!(BlockStyle::body().line_height, 1.65);
    assert_eq!(BlockStyle::heading(1).font_size, 15.5 * 1.875);
    assert_eq!(BlockStyle::heading(2).margin_top_em, 1.8);
    assert_eq!(BlockStyle::blockquote().padding, [6.2, 16.0]);
    assert_eq!(BlockStyle::code_block().radius, 10.0);
    assert_eq!(BlockStyle::code_block().padding, [14.0, 18.0]);
    assert_eq!(BlockStyle::table_cell().padding, [10.0, 14.0]);
}
```

Add GPUI bounds assertions that the reader column begins 32 px below `reader-scroll`, the tab is
4 px from the rail's top, and a wide table/code line stays inside its own horizontal surface rather
than increasing the main viewport width.

Add `debug_selector(|| "tab-bar".into())` to the tab-bar root, then use these assertions after a
draw of the showcase document:

```rust
let scroll = visual.debug_bounds("reader-scroll").expect("reader viewport");
let column = visual.debug_bounds("reader-column").expect("reader column");
let first_block = visual.debug_bounds("reader-block-0").expect("first reader block");
let tab_bar = visual.debug_bounds("tab-bar").expect("tab bar");
let tab = visual.debug_bounds("document-tab-0").expect("active tab");
let code = visual.debug_bounds("reader-code-27").expect("wide code surface");

assert_eq!(first_block.top() - scroll.top(), px(32.0));
assert_eq!(tab.top() - tab_bar.top(), px(4.0));
assert!(code.left() >= column.left());
assert!(code.right() <= column.right());
```

- [ ] **Step 2: Run the focused reader tests and record every failing metric/bounds assertion**

```sh
cargo test --manifest-path apps/gpui/Cargo.toml reader_surface_metrics -- --nocapture
cargo test --manifest-path apps/gpui/Cargo.toml reader_bounds -- --nocapture
```

Expected: at least the old 22 px reader top inset and any remaining chrome/surface drift fail.

- [ ] **Step 3: Align every supported Markdown surface with `markdown.css`**

Audit and set these exact relationships in `reader.rs`:

```rust
// Reader column
.px(px(48.0))
.pt(px(32.0))
.pb(px(40.0))
.text_size(px(15.5))
.line_height(px(15.5 * 1.65))

// Inline code
.rounded(px(4.0))
.px(px(15.5 * 0.35))
.py(px(15.5 * 0.1))

// Fenced code
.rounded(px(10.0))
.px(px(18.0))
.py(px(14.0))
.line_height(px(15.5 * 0.875 * 1.6))

// Blockquote and table
.border_l(px(3.0))
.rounded(px(8.0))
.overflow_hidden()
.border_1()

// Each table header/cell
.px(px(14.0))
.py(px(10.0))
```

Use the existing `block_sequence_spacing` rather than introducing duplicate margin constants.
Check H1–H6, paragraph, nested list, task item, link, inline code, blockquote, thematic rule, code,
table, raw text, and local image branches one by one against the approved spec. Preserve all
existing link focus/click and copy behavior.

- [ ] **Step 4: Refine the minimal sidebar empty state without adding utilities**

Replace the two-line placeholder with the established hierarchy:

```rust
div()
    .flex()
    .flex_col()
    .items_center()
    .pt(px(36.0))
    .px(px(20.0))
    .child(icon("icons/folder.svg", theme.muted_foreground.opacity(0.55), 22.0))
    .child(
        div()
            .mt(px(10.0))
            .font_weight(FontWeight::MEDIUM)
            .text_size(px(13.0))
            .text_color(theme.foreground)
            .child("No folder open"),
    )
    .child(
        div()
            .mt(px(6.0))
            .max_w(px(190.0))
            .text_center()
            .text_size(px(12.0))
            .line_height(px(18.0))
            .text_color(theme.muted_foreground)
            .child("Open or drop a folder to browse its Markdown files."),
    )
```

If a folder is open but contains no Markdown, retain “No Markdown files in this folder.” as the
single concise message. Do not add settings, recents, outline, or footer controls.

- [ ] **Step 5: Expand the deterministic fixture to cover the complete reader path**

Keep the existing fixture content and add:

````markdown
### Nested content

- Parent item
  - Nested unordered item
  - [ ] Nested task

```javascript
export function greet(name) {
  return `Hello, ${name}`
}
```

```json
{"name":"Mdow","native":true,"features":["scrolling","highlighting"]}
```

```shell
cargo test --manifest-path apps/gpui/Cargo.toml
```

```unknown-language
this stays readable <without> a grammar
```

```text
this_is_a_deliberately_long_code_line_that_must_scroll_horizontally_without_widening_the_reader_or_consuming_vertical_wheel_input = "0123456789abcdefghijklmnopqrstuvwxyz0123456789abcdefghijklmnopqrstuvwxyz"
```

| A deliberately wide column | Another deliberately wide column | Final column |
| --- | --- | --- |
| Horizontal table scrolling remains local | The document still scrolls vertically | Ready |

Reader verification paragraph 01 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 02 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 03 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 04 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 05 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 06 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 07 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 08 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 09 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 10 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 11 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 12 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 13 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 14 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 15 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 16 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 17 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 18 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 19 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 20 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 21 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 22 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 23 confirms that long documents maintain the established measure and vertical rhythm.

Reader verification paragraph 24 confirms that long documents maintain the established measure and vertical rhythm.
````

- [ ] **Step 6: Run formatting, focused tests, the full suite, Clippy, and build**

```sh
cargo fmt --manifest-path apps/gpui/Cargo.toml -- --check
cargo test --manifest-path apps/gpui/Cargo.toml
cargo clippy --manifest-path apps/gpui/Cargo.toml --all-targets -- -D warnings
cargo build --manifest-path apps/gpui/Cargo.toml
```

Expected: every command exits zero with no warnings promoted by Clippy.

- [ ] **Step 7: Launch and perform the complete native interaction pass**

```sh
./script/build_and_run_gpui.sh apps/gpui/tests/fixtures/showcase.md
```

At 1120 × 760 in dark and light appearance, verify:

- Trackpad, mouse wheel, scrollbar thumb, Page Up, Page Down, Home, and End reach the full document.
- Switching two tabs restores independent positions; resize, width toggle, theme change, and live
  reload do not reset them.
- Vertical wheel movement still works while the pointer is over horizontally scrollable code/table
  content.
- Rust, JavaScript, JSON, and shell blocks use multiple token colors; the unknown block remains
  plain and readable; copy returns exact source and displays two-second feedback.
- Tab rail is 36 px, tab is 28 px with 4 px vertical breathing room, active radius is 6 px, tab-list
  inset is 6 px, tab content inset is 10 px, and the toggle has a separate fixed slot.
- Breadcrumb uses `showcase.md` unless frontmatter supplies a title.
- Reader starts 32 px below the breadcrumb, maintains a 768 px constrained measure, and the final
  block clears the bottom by 40 px.
- H1–H6, paragraphs, nested lists/tasks, blockquote, rule, table, raw HTML text, and image remain
  correctly styled and unclipped.

- [ ] **Step 8: Capture a state-matched visual comparison and correct visible drift**

Capture Electron and GPUI in the same one-tab showcase state, dark appearance, 1120 × 760 viewport,
and top-of-document position. Put both captures into one comparison view and inspect tab spacing,
separator alignment, breadcrumb, reader start, content measure, typography, code surface, and
scrollbar. Repeat in light appearance. Correct any unexplained padding, margin, font weight, radius,
border, clipping, or alignment difference, then rerun Step 6.

- [ ] **Step 9: Commit the polished reader and fixture**

```sh
git add apps/gpui/src/ui/reader.rs apps/gpui/src/ui/chrome.rs apps/gpui/tests/fixtures/showcase.md
git commit -m "fix(gpui): finish reader fidelity pass"
```

---

## Completion Gate

Before reporting completion, confirm all six task commits exist, the worktree is clean, every Step 6
command passes from fresh output, the launcher opens the exact built binary, and the state-matched
light/dark captures show no unexplained reader-path drift. Do not claim scrolling from a static
screenshot: preserve evidence from the wheel/keyboard tests and the live interaction pass.
