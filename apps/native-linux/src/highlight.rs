//! Syntax highlighting with syntect and bat's grammar collection (two-face).
//!
//! Tokens are classified into the six roles of the desktop's GitHub themes rather than coloured
//! directly, so one highlight pass serves both the light and the dark palette.

use std::str::FromStr;
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{
    Color, ScopeSelectors, StyleModifier, Theme, ThemeItem, ThemeSettings,
};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Tok {
    Plain = 0,
    Comment = 1,
    Keyword = 2,
    String = 3,
    Entity = 4,
    Constant = 5,
}

/// A highlighted byte range of a code block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
    pub tok: Tok,
}

/// Larger blocks stay plain: highlighting them would take seconds.
const MAX_HIGHLIGHT_BYTES: usize = 512 * 1024;

fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(two_face::syntax::extra_newlines)
}

fn classifier() -> &'static Theme {
    static THEME: OnceLock<Theme> = OnceLock::new();
    THEME.get_or_init(|| {
        let rules: &[(&str, Tok)] = &[
            ("comment, punctuation.definition.comment", Tok::Comment),
            (
                "keyword, storage, storage.type, storage.modifier, keyword.operator, \
                 entity.name.tag, support.type.primitive, support.type.builtin, \
                 storage.type.primitive, constant.language.import-export-all",
                Tok::Keyword,
            ),
            (
                "string, punctuation.definition.string, string.regexp, \
                 markup.inline.raw, string.quoted",
                Tok::String,
            ),
            (
                "entity.name.function, support.function, entity.name.type, entity.name.class, \
                 support.class, entity.other.inherited-class, entity.name.struct, \
                 entity.name.enum, entity.name.trait, entity.name.interface, variable.function, \
                 entity.name.label, entity.name.macro, support.macro, meta.function-call.generic, \
                 entity.name.section, markup.heading",
                Tok::Entity,
            ),
            (
                "constant, variable.language, support.constant, constant.character.escape, \
                 entity.other.attribute-name, meta.property-name, support.type.property-name, \
                 variable.other.property, variable.other.constant, variable.other.enummember, \
                 meta.object-literal.key, entity.name.constant, variable.other.member, \
                 meta.attribute, markup.underline.link",
                Tok::Constant,
            ),
        ];
        Theme {
            name: Some("mdow-roles".into()),
            author: None,
            settings: ThemeSettings {
                foreground: Some(Color { r: Tok::Plain as u8, g: 0, b: 0, a: 255 }),
                ..ThemeSettings::default()
            },
            scopes: rules
                .iter()
                .map(|(selector, tok)| ThemeItem {
                    scope: ScopeSelectors::from_str(selector).expect("valid scope selector"),
                    style: StyleModifier {
                        foreground: Some(Color { r: *tok as u8, g: 0, b: 0, a: 255 }),
                        background: None,
                        font_style: None,
                    },
                })
                .collect(),
        }
    })
}

/// Fence info strings the grammars don't know by name, as in the desktop's `normalizeLanguage`.
fn alias(lang: &str) -> &str {
    match lang {
        "shell" | "console" | "shellscript" | "sh-session" | "zsh" | "fish" => "bash",
        "js" | "mjs" | "cjs" | "node" => "javascript",
        "ts" | "mts" | "cts" => "typescript",
        "jsx" => "javascript",
        "tsx" => "tsx",
        "py" | "python3" => "python",
        "rb" => "ruby",
        "rs" => "rust",
        "golang" => "go",
        "yml" => "yaml",
        "jsonc" | "json5" => "json",
        "c++" | "cxx" | "hpp" => "cpp",
        "cs" | "csharp" => "c#",
        "kt" => "kotlin",
        "objc" | "objective-c" => "objective-c",
        "ps" | "ps1" | "pwsh" => "powershell",
        "dockerfile" | "docker" => "dockerfile",
        "md" | "mdx" => "markdown",
        "htm" | "xhtml" | "vue" | "svelte" => "html",
        "tf" | "hcl" => "terraform",
        "conf" | "ini" | "cfg" => "ini",
        other => other,
    }
}

fn find_syntax(lang: &str) -> Option<&'static SyntaxReference> {
    let set = syntaxes();
    let lang = lang.trim().to_ascii_lowercase();
    if lang.is_empty() || matches!(lang.as_str(), "text" | "txt" | "plain" | "plaintext") {
        return None;
    }
    let name = alias(&lang);
    set.find_syntax_by_token(name)
        .or_else(|| set.find_syntax_by_name(name))
        .or_else(|| set.syntaxes().iter().find(|s| s.name.eq_ignore_ascii_case(name)))
}

/// The language label for a fence's info string: its first word.
pub fn fence_language(info: &str) -> String {
    info.split(|c: char| c.is_whitespace() || c == '{' || c == ',')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase()
}

pub fn highlight(lang: &str, code: &str) -> Vec<Span> {
    if code.len() > MAX_HIGHLIGHT_BYTES {
        return Vec::new();
    }
    let Some(syntax) = find_syntax(lang) else { return Vec::new() };
    let set = syntaxes();
    let mut lines = HighlightLines::new(syntax, classifier());
    let mut spans: Vec<Span> = Vec::new();
    let mut offset = 0u32;
    for line in LinesWithEndings::from(code) {
        let Ok(regions) = lines.highlight_line(line, set) else { return Vec::new() };
        for (style, text) in regions {
            let len = text.len() as u32;
            let tok = match style.foreground.r {
                1 => Tok::Comment,
                2 => Tok::Keyword,
                3 => Tok::String,
                4 => Tok::Entity,
                5 => Tok::Constant,
                _ => Tok::Plain,
            };
            if tok != Tok::Plain && len > 0 {
                match spans.last_mut() {
                    Some(last) if last.tok == tok && last.end == offset => last.end += len,
                    _ => spans.push(Span { start: offset, end: offset + len, tok }),
                }
            }
            offset += len;
        }
    }
    spans
}

/// Warm the grammar set off the main thread so the first code block doesn't stall.
pub fn preload() {
    std::thread::spawn(|| {
        let _ = highlight("rust", "fn main() {}\n");
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_rust_tokens() {
        let code = "// hi\nfn main() { let x = \"s\"; 42 }\n";
        let spans = highlight("rust", code);
        let tok_of = |needle: &str| {
            let start = code.find(needle).unwrap() as u32;
            spans.iter().find(|s| s.start <= start && start < s.end).map(|s| s.tok)
        };
        assert_eq!(tok_of("// hi"), Some(Tok::Comment));
        assert_eq!(tok_of("fn"), Some(Tok::Keyword));
        assert_eq!(tok_of("\"s\""), Some(Tok::String));
        assert_eq!(tok_of("main"), Some(Tok::Entity));
        assert_eq!(tok_of("42"), Some(Tok::Constant));
    }

    #[test]
    fn knows_common_aliases() {
        for lang in ["ts", "tsx", "js", "sh", "shell", "py", "yml", "json", "go", "toml"] {
            assert!(find_syntax(lang).is_some(), "{lang}");
        }
        assert!(find_syntax("text").is_none());
        assert!(highlight("nope-lang", "x").is_empty());
    }

    #[test]
    fn fence_language_takes_the_first_word() {
        assert_eq!(fence_language("TypeScript title=\"a.ts\""), "typescript");
        assert_eq!(fence_language("js{1,3}"), "js");
        assert_eq!(fence_language(""), "");
    }
}
