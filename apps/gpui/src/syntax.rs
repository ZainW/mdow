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
        assert_eq!(
            highlighted
                .light_runs
                .iter()
                .map(|run| run.len)
                .sum::<usize>(),
            code.len()
        );
        assert_eq!(
            highlighted
                .dark_runs
                .iter()
                .map(|run| run.len)
                .sum::<usize>(),
            code.len()
        );
    }

    #[test]
    fn unknown_language_falls_back_to_one_plain_run() {
        let code = "alpha < beta\n";
        let highlighted = highlight_code(Some("not-a-real-language"), code);

        assert_eq!(
            highlighted.normalized_language.as_deref(),
            Some("not-a-real-language")
        );
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

use crate::document::{DocumentBlock, ParsedDocument};
use std::{collections::HashMap, ops::Deref, sync::OnceLock};
use syntect::{
    easy::HighlightLines,
    highlighting::{
        Color, FontStyle, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSettings,
    },
    parsing::SyntaxSet,
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
        Self {
            parsed,
            code_blocks: HashMap::new(),
        }
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

const LIGHT_DEFAULT: SyntaxColor = SyntaxColor {
    red: 0x24,
    green: 0x29,
    blue: 0x2f,
};
const DARK_DEFAULT: SyntaxColor = SyntaxColor {
    red: 0xe6,
    green: 0xed,
    blue: 0xf3,
};

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
        scope: scope
            .parse::<ScopeSelectors>()
            .expect("valid syntax scope selector"),
        style: StyleModifier {
            foreground: Some(syntect_color(foreground)),
            background: None,
            font_style: italic.then_some(FontStyle::ITALIC),
        },
    }
}

fn github_theme(
    name: &str,
    default: u32,
    comment: u32,
    keyword: u32,
    string: u32,
    function: u32,
    constant: u32,
) -> Theme {
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
            theme_item(
                "entity.name.function, entity.name.type, support.type",
                function,
                false,
            ),
            theme_item("constant, constant.numeric", constant, false),
        ],
    }
}

fn github_themes() -> &'static (Theme, Theme) {
    static THEMES: OnceLock<(Theme, Theme)> = OnceLock::new();
    THEMES.get_or_init(|| {
        (
            github_theme(
                "Mdow GitHub Light",
                0x24292f,
                0x6e7781,
                0xcf222e,
                0x0a3069,
                0x8250df,
                0x0550ae,
            ),
            github_theme(
                "Mdow GitHub Dark",
                0xe6edf3,
                0x8b949e,
                0xff7b72,
                0xa5d6ff,
                0xd2a8ff,
                0x79c0ff,
            ),
        )
    })
}

fn plain_run(code: &str, color: SyntaxColor) -> Vec<HighlightedRun> {
    (!code.is_empty())
        .then_some(HighlightedRun {
            len: code.len(),
            color,
            italic: false,
        })
        .into_iter()
        .collect()
}

fn syntax_for<'a>(
    set: &'a SyntaxSet,
    language: &str,
) -> Option<&'a syntect::parsing::SyntaxReference> {
    set.find_syntax_by_token(language)
        .or_else(|| set.find_syntax_by_extension(language))
        .or_else(|| set.find_syntax_by_name(language))
}

fn runs_for(
    code: &str,
    syntax: &syntect::parsing::SyntaxReference,
    theme: &Theme,
) -> Vec<HighlightedRun> {
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
    PreparedDocument {
        parsed,
        code_blocks,
    }
}
