//! Syntax highlighting for fenced code blocks.
//!
//! Mirrors Electron's `lib/highlight.ts`: the same language aliases, GitHub light/dark colors,
//! and lazy highlighting. `prepare_document` does no highlighting; the reader asks the shared
//! [`HighlightCache`] for a block when the block is laid out (GPUI's list only lays out items in
//! or near the viewport, like Electron's `onceNearViewport`), paints plain monospaced text on a
//! miss, and highlights that block for the active color scheme on a background thread.

use crate::{document::ParsedDocument, theme::ColorScheme};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::{Hash, Hasher},
    ops::Deref,
    sync::{Arc, Mutex, OnceLock},
};
use syntect::{
    easy::HighlightLines,
    highlighting::{
        Color, FontStyle, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSettings,
    },
    parsing::{SyntaxReference, SyntaxSet},
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

/// One code block highlighted for one color scheme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HighlightedCode {
    pub normalized_language: Option<String>,
    pub scheme: ColorScheme,
    pub text: String,
    pub runs: Vec<HighlightedRun>,
}

#[derive(Debug, Clone)]
pub struct PreparedDocument {
    parsed: ParsedDocument,
}

impl Deref for PreparedDocument {
    type Target = ParsedDocument;

    fn deref(&self) -> &Self::Target {
        &self.parsed
    }
}

impl PreparedDocument {
    pub fn plain(parsed: ParsedDocument) -> Self {
        Self { parsed }
    }

    pub(crate) fn set_path(&mut self, path: std::path::PathBuf) {
        self.parsed.path = path;
    }
}

/// Wraps a parsed document for the reader. Highlighting is deferred to render time.
pub fn prepare_document(parsed: ParsedDocument) -> PreparedDocument {
    PreparedDocument::plain(parsed)
}

/// Electron's `normalizeLanguage`: trim, lowercase, drop a `language-` prefix, first word.
fn fence_language(info: &str) -> String {
    let lower = info.trim().to_lowercase();
    let lower = lower.strip_prefix("language-").unwrap_or(&lower);
    lower
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_owned()
}

/// The fence aliases Electron's Shiki loader table accepts, mapped to one canonical name and the
/// syntect grammar that highlights it.
const LANGUAGES: &[(&[&str], &str, &str)] = &[
    (
        &["javascript", "js", "mjs", "cjs"],
        "javascript",
        "JavaScript",
    ),
    (
        &["typescript", "ts", "mts", "cts"],
        "typescript",
        "TypeScript",
    ),
    (&["python", "py"], "python", "Python"),
    (&["rust", "rs"], "rust", "Rust"),
    (&["go", "golang"], "go", "Go"),
    (&["java"], "java", "Java"),
    (&["c", "h"], "c", "C"),
    (&["cpp", "cxx", "c++", "hpp"], "cpp", "C++"),
    (&["csharp", "cs", "c#"], "csharp", "C#"),
    (&["ruby", "rb"], "ruby", "Ruby"),
    (&["swift"], "swift", "Swift"),
    (&["kotlin", "kt"], "kotlin", "Kotlin"),
    (&["html", "htm"], "html", "HTML"),
    (&["css"], "css", "CSS"),
    (&["json", "jsonc"], "json", "JSON"),
    (&["yaml", "yml"], "yaml", "YAML"),
    (&["toml"], "toml", "TOML"),
    (&["xml", "svg"], "xml", "XML"),
    (&["markdown", "md", "mdx"], "markdown", "Markdown"),
    (&["sql"], "sql", "SQL"),
    (
        &["bash", "sh", "shell", "shellscript", "zsh", "console"],
        "bash",
        "Bourne Again Shell (bash)",
    ),
    (&["diff", "patch"], "diff", "Diff"),
    (&["graphql", "gql"], "graphql", "GraphQL"),
    (&["dockerfile", "docker"], "dockerfile", "Dockerfile"),
    (&["lua"], "lua", "Lua"),
    (&["zig"], "zig", "Zig"),
    (&["elixir", "ex", "exs"], "elixir", "Elixir"),
    (&["haskell", "hs"], "haskell", "Haskell"),
    (&["ocaml", "ml"], "ocaml", "OCaml"),
    // No JSX grammar ships with syntect/two-face; the TSX grammar is a JSX superset.
    (&["jsx"], "jsx", "TypeScriptReact"),
    (&["tsx"], "tsx", "TypeScriptReact"),
    (&["php"], "php", "PHP Source"),
];

fn known_language(fence: &str) -> Option<(&'static str, &'static str)> {
    LANGUAGES
        .iter()
        .find(|(aliases, _, _)| aliases.contains(&fence))
        .map(|(_, canonical, grammar)| (*canonical, *grammar))
}

/// Normalizes a fence info string to the canonical language name (`ts` -> `typescript`).
/// Unknown languages pass through normalized but otherwise unchanged.
pub fn normalize_language(info: &str) -> String {
    let fence = fence_language(info);
    known_language(&fence).map_or(fence, |(canonical, _)| canonical.to_owned())
}

fn syntax_set() -> &'static SyntaxSet {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAXES.get_or_init(two_face::syntax::extra_newlines)
}

fn syntax_for(info: &str) -> Option<&'static SyntaxReference> {
    let fence = fence_language(info);
    if fence.is_empty() {
        return None;
    }
    let set = syntax_set();
    match known_language(&fence) {
        Some((_, grammar)) => set.find_syntax_by_name(grammar),
        // Beyond Electron's list, still try the extended grammar set by token.
        None => set.find_syntax_by_token(&fence),
    }
}

/// Whether a fence language has a grammar, so the reader should request highlighting at all.
pub fn is_highlightable_language(info: &str) -> bool {
    syntax_for(info).is_some()
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

pub fn default_code_color(scheme: ColorScheme) -> SyntaxColor {
    match scheme {
        ColorScheme::Light => LIGHT_DEFAULT,
        ColorScheme::Dark => DARK_DEFAULT,
    }
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

struct GithubPalette {
    name: &'static str,
    default: u32,
    comment: u32,
    keyword: u32,
    string: u32,
    function: u32,
    constant: u32,
    tag: u32,
    variable: u32,
}

fn github_theme(palette: GithubPalette) -> Theme {
    Theme {
        name: Some(palette.name.into()),
        author: Some("Mdow".into()),
        settings: ThemeSettings {
            foreground: Some(syntect_color(palette.default)),
            ..ThemeSettings::default()
        },
        scopes: vec![
            theme_item(
                "comment, punctuation.definition.comment",
                palette.comment,
                true,
            ),
            theme_item(
                "keyword, storage, keyword.operator.new, variable.language",
                palette.keyword,
                false,
            ),
            theme_item(
                "string, punctuation.definition.string",
                palette.string,
                false,
            ),
            theme_item(
                "entity.name.function, support.function, entity.name.type, support.type, \
                 entity.name.class, entity.other.inherited-class, entity.other.attribute-name",
                palette.function,
                false,
            ),
            theme_item(
                "constant, constant.numeric, constant.language, support.constant, \
                 variable.other.constant, entity.name.constant, markup.heading",
                palette.constant,
                false,
            ),
            theme_item("entity.name.tag, markup.inserted", palette.tag, false),
            theme_item(
                "variable.parameter, variable.other.readwrite",
                palette.variable,
                false,
            ),
            theme_item("markup.deleted, invalid", palette.keyword, false),
        ],
    }
}

fn github_theme_for(scheme: ColorScheme) -> &'static Theme {
    static LIGHT: OnceLock<Theme> = OnceLock::new();
    static DARK: OnceLock<Theme> = OnceLock::new();
    match scheme {
        ColorScheme::Light => LIGHT.get_or_init(|| {
            github_theme(GithubPalette {
                name: "Mdow GitHub Light",
                default: 0x24292f,
                comment: 0x6e7781,
                keyword: 0xcf222e,
                string: 0x0a3069,
                function: 0x8250df,
                constant: 0x0550ae,
                tag: 0x116329,
                variable: 0x953800,
            })
        }),
        ColorScheme::Dark => DARK.get_or_init(|| {
            github_theme(GithubPalette {
                name: "Mdow GitHub Dark",
                default: 0xe6edf3,
                comment: 0x8b949e,
                keyword: 0xff7b72,
                string: 0xa5d6ff,
                function: 0xd2a8ff,
                constant: 0x79c0ff,
                tag: 0x7ee787,
                variable: 0xffa657,
            })
        }),
    }
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

fn runs_for(code: &str, syntax: &SyntaxReference, theme: &Theme) -> Vec<HighlightedRun> {
    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut runs: Vec<HighlightedRun> = Vec::new();
    for line in LinesWithEndings::from(code) {
        let Ok(parts) = highlighter.highlight_line(line, syntax_set()) else {
            return Vec::new();
        };
        for (style, text) in parts {
            let run = HighlightedRun {
                len: text.len(),
                color: SyntaxColor {
                    red: style.foreground.r,
                    green: style.foreground.g,
                    blue: style.foreground.b,
                },
                italic: style.font_style.contains(FontStyle::ITALIC),
            };
            // Fold adjacent same-styled pieces, as Electron folds whitespace tokens.
            match runs.last_mut() {
                Some(last) if last.color == run.color && last.italic == run.italic => {
                    last.len += run.len;
                }
                _ => runs.push(run),
            }
        }
    }
    runs
}

/// Highlights one block for one color scheme. Unknown languages get one plain run.
pub fn highlight_code(language: Option<&str>, code: &str, scheme: ColorScheme) -> HighlightedCode {
    let normalized_language = language
        .map(normalize_language)
        .filter(|language| !language.is_empty());
    let fallback = || HighlightedCode {
        normalized_language: normalized_language.clone(),
        scheme,
        text: code.to_owned(),
        runs: plain_run(code, default_code_color(scheme)),
    };
    let Some(syntax) = language.and_then(syntax_for) else {
        return fallback();
    };
    let runs = runs_for(code, syntax, github_theme_for(scheme));
    if !code.is_empty() && runs.is_empty() {
        return fallback();
    }
    HighlightedCode {
        normalized_language,
        scheme,
        text: code.to_owned(),
        runs,
    }
}

/// Identity of one highlight job: canonical language, color scheme, and the code's content.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HighlightKey {
    language: String,
    scheme: ColorScheme,
    code_hash: u64,
    code_len: usize,
}

impl HighlightKey {
    pub fn new(language: &str, code: &str, scheme: ColorScheme) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        code.hash(&mut hasher);
        Self {
            language: normalize_language(language),
            scheme,
            code_hash: hasher.finish(),
            code_len: code.len(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum HighlightLookup {
    /// Tokens for this exact code and scheme are cached.
    Ready(Arc<HighlightedCode>),
    /// The caller claimed the job and must highlight, then [`HighlightCache::fulfill`] it.
    Claimed(HighlightKey),
    /// Another render already claimed this job.
    Pending,
    /// No grammar: keep the plain text.
    Unsupported,
}

/// Electron keeps the 400 most recent blocks.
pub const MAX_CACHED_BLOCKS: usize = 400;

#[derive(Default)]
struct CacheInner {
    entries: HashMap<HighlightKey, Arc<HighlightedCode>>,
    order: VecDeque<HighlightKey>,
    pending: HashSet<HighlightKey>,
}

#[derive(Default)]
pub struct HighlightCache {
    inner: Mutex<CacheInner>,
}

impl HighlightCache {
    pub fn global() -> &'static Self {
        static CACHE: OnceLock<HighlightCache> = OnceLock::new();
        CACHE.get_or_init(Self::default)
    }

    pub fn lookup(
        &self,
        language: Option<&str>,
        code: &str,
        scheme: ColorScheme,
    ) -> HighlightLookup {
        let Some(language) = language.filter(|language| is_highlightable_language(language)) else {
            return HighlightLookup::Unsupported;
        };
        let key = HighlightKey::new(language, code, scheme);
        let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(entry) = inner.entries.get(&key)
            && entry.text == code
        {
            return HighlightLookup::Ready(entry.clone());
        }
        if inner.pending.insert(key.clone()) {
            HighlightLookup::Claimed(key)
        } else {
            HighlightLookup::Pending
        }
    }

    pub fn fulfill(&self, key: HighlightKey, value: HighlightedCode) -> Arc<HighlightedCode> {
        let value = Arc::new(value);
        let mut inner = self.inner.lock().unwrap_or_else(|error| error.into_inner());
        inner.pending.remove(&key);
        if inner.entries.insert(key.clone(), value.clone()).is_none() {
            inner.order.push_back(key);
        }
        while inner.order.len() > MAX_CACHED_BLOCKS {
            if let Some(oldest) = inner.order.pop_front() {
                inner.entries.remove(&oldest);
            }
        }
        value
    }

    /// Runs a claimed job synchronously. The reader does this on a background thread.
    pub fn highlight_claimed(
        &self,
        key: HighlightKey,
        language: &str,
        code: &str,
    ) -> Arc<HighlightedCode> {
        let scheme = key.scheme;
        self.fulfill(key, highlight_code(Some(language), code, scheme))
    }

    pub fn len(&self) -> usize {
        self.inner
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .entries
            .len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

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
        assert_eq!(normalize_language("console"), "bash");
        assert_eq!(normalize_language("yml"), "yaml");
        assert_eq!(normalize_language("c++"), "cpp");
        assert_eq!(normalize_language("c#"), "csharp");
        assert_eq!(normalize_language("kt"), "kotlin");
        assert_eq!(normalize_language("gql"), "graphql");
        assert_eq!(normalize_language("exs"), "elixir");
        assert_eq!(normalize_language("not-a-language"), "not-a-language");
    }

    #[test]
    fn every_electron_shiki_alias_has_a_grammar() {
        for (aliases, _, grammar) in LANGUAGES {
            assert!(
                syntax_set().find_syntax_by_name(grammar).is_some(),
                "missing grammar {grammar}"
            );
            for alias in *aliases {
                assert!(is_highlightable_language(alias), "{alias}");
            }
        }
    }

    #[test]
    fn extended_languages_emit_syntax_colored_runs() {
        for (language, code) in [
            ("rust", "fn main() { println!(\"hello\"); }\n"),
            ("ts", "const answer: number = 42;\n"),
            ("tsx", "const el = <div className=\"x\">hi</div>;\n"),
            ("jsx", "const el = <div className=\"x\">hi</div>;\n"),
            ("toml", "[package]\nname = \"mdow\"\n"),
            ("swift", "let answer = \"forty-two\"\n"),
            ("kotlin", "val answer = \"forty-two\"\n"),
            ("javascript", "const answer = \"forty-two\";\n"),
            ("json", "{\"answer\": 42}\n"),
            ("shell", "if true; then echo \"hello\"; fi\n"),
            ("php", "$answer = \"forty-two\";\n"),
            ("graphql", "query { answer }\n"),
            ("zig", "const x: u8 = 42;\n"),
        ] {
            for scheme in [ColorScheme::Light, ColorScheme::Dark] {
                let highlighted = highlight_code(Some(language), code, scheme);
                assert_eq!(highlighted.text, code, "{language} source");
                assert_eq!(
                    highlighted.runs.iter().map(|run| run.len).sum::<usize>(),
                    code.len(),
                    "{language} run lengths"
                );
                assert!(highlighted.runs.len() > 1, "{language} {scheme:?} runs");
            }
        }
    }

    #[test]
    fn light_and_dark_highlights_differ() {
        let code = "fn main() {}\n";
        assert_ne!(
            highlight_code(Some("rust"), code, ColorScheme::Light).runs,
            highlight_code(Some("rust"), code, ColorScheme::Dark).runs
        );
    }

    #[test]
    fn unknown_language_falls_back_to_one_plain_run() {
        let code = "alpha < beta\n";
        let highlighted = highlight_code(Some("not-a-real-language"), code, ColorScheme::Dark);

        assert_eq!(
            highlighted.normalized_language.as_deref(),
            Some("not-a-real-language")
        );
        assert_eq!(highlighted.text, code);
        assert_eq!(highlighted.runs.len(), 1);
        assert_eq!(highlighted.runs[0].color, DARK_DEFAULT);
        assert!(!is_highlightable_language("not-a-real-language"));
    }

    #[test]
    fn preparing_a_document_highlights_nothing_eagerly() {
        let cache = HighlightCache::default();
        let document = prepare_document(parse_document(
            PathBuf::from("/tmp/code.md"),
            "```rust\nlet answer = 42;\n```\n".into(),
        ));
        assert_eq!(document.blocks.len(), 1);
        assert!(cache.is_empty());
    }

    #[test]
    fn lazy_highlighting_claims_once_then_serves_the_cache_per_scheme() {
        let cache = HighlightCache::default();
        let code = "let answer = 42;\n";

        let HighlightLookup::Claimed(key) = cache.lookup(Some("rs"), code, ColorScheme::Dark)
        else {
            panic!("first sight of a block claims the job");
        };
        assert_eq!(
            cache.lookup(Some("rust"), code, ColorScheme::Dark),
            HighlightLookup::Pending,
            "aliases share one job"
        );
        cache.highlight_claimed(key, "rs", code);

        let HighlightLookup::Ready(dark) = cache.lookup(Some("rust"), code, ColorScheme::Dark)
        else {
            panic!("fulfilled jobs are served from the cache");
        };
        assert_eq!(dark.scheme, ColorScheme::Dark);
        assert!(dark.runs.len() > 1);
        assert!(matches!(
            cache.lookup(Some("rust"), code, ColorScheme::Light),
            HighlightLookup::Claimed(_)
        ));
        assert!(matches!(
            cache.lookup(Some("rust"), "let other = 1;\n", ColorScheme::Dark),
            HighlightLookup::Claimed(_)
        ));
        assert_eq!(
            cache.lookup(Some("nope"), code, ColorScheme::Dark),
            HighlightLookup::Unsupported
        );
        assert_eq!(
            cache.lookup(None, code, ColorScheme::Dark),
            HighlightLookup::Unsupported
        );
    }

    #[test]
    fn cache_evicts_the_oldest_blocks_past_the_electron_cap() {
        let cache = HighlightCache::default();
        for index in 0..MAX_CACHED_BLOCKS + 5 {
            let code = format!("let n = {index};\n");
            let key = HighlightKey::new("rust", &code, ColorScheme::Light);
            cache.fulfill(key, highlight_code(Some("rust"), &code, ColorScheme::Light));
        }
        assert_eq!(cache.len(), MAX_CACHED_BLOCKS);
        assert!(matches!(
            cache.lookup(Some("rust"), "let n = 0;\n", ColorScheme::Light),
            HighlightLookup::Claimed(_)
        ));
    }
}
