//! The pure parts of the companion service, ported from the Swift app's `CompanionLogic.swift`
//! and the reply-text splitting in `CompanionPartViews.swift`.

use super::client::{ModelInfo, PermissionRule, ProviderInfo};
use super::types::*;
use std::path::Path;

pub const INSTRUCTIONS: &str = "You are the writing companion inside Mdow, a calm reader for Markdown and HTML documents. The user is reading the documents in this folder and wants help understanding and improving them.

- Answer questions about the documents. Read them with your tools instead of guessing, and mention documents by their path relative to the folder.
- When the user asks for a change, make it with your edit or write tool right away. Mdow shows the user every change as a diff to approve or reject before it is written, so do not ask for confirmation first.
- Keep edits focused. Preserve each document's voice, structure and formatting, whether it is Markdown or HTML.
- Do not run shell commands, reach outside this folder, or create new files unless the user asks for one.
- Reply concisely in Markdown.";

/// Shell commands and other folders are off limits; edits always come back for review.
pub fn session_permissions() -> Vec<PermissionRule> {
    vec![
        PermissionRule::new("shell", "*", "deny"),
        PermissionRule::new("bash", "*", "deny"),
        PermissionRule::new("edit", "*", "ask"),
    ]
}

/// Mdow talks to the v2 HTTP API (`opencode serve`), which shipped in OpenCode 2.0.
pub const MINIMUM_MAJOR_VERSION: u32 = 2;

pub fn home() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/".into())
}

/// The open folder, else the document's folder, else home.
pub fn session_directory(document: Option<&Path>, folder: Option<&Path>, home: &str) -> String {
    if let Some(folder) = folder {
        return folder.to_string_lossy().into_owned();
    }
    if let Some(parent) = document.and_then(Path::parent) {
        return parent.to_string_lossy().into_owned();
    }
    home.to_string()
}

pub fn display_path(path: &str, directory: &str) -> String {
    let base =
        if directory.ends_with('/') { directory.to_string() } else { format!("{directory}/") };
    match path.strip_prefix(&base) {
        Some(rest) if !rest.is_empty() => rest.to_string(),
        _ => path.to_string(),
    }
}

/// Prefixes the user's message with what they are looking at in Mdow.
pub fn compose_prompt(
    text: &str,
    active_path: Option<&str>,
    tags: &[ContextTag],
    directory: &str,
) -> String {
    let mut lines: Vec<String> = Vec::new();
    if let Some(active) = active_path {
        lines.push(format!("Viewing: {}", display_path(active, directory)));
    }
    let attached: Vec<String> = tags.iter().map(|t| display_path(&t.path, directory)).collect();
    if !attached.is_empty() {
        lines.push(format!("Attached: {}", attached.join(", ")));
    }
    if lines.is_empty() {
        return text.to_string();
    }
    format!("<mdow-context>\n{}\n</mdow-context>\n\n{text}", lines.join("\n"))
}

/// Model options without deprecated models, sorted by provider then name.
pub fn model_options(models: &[ModelInfo], providers: &[ProviderInfo]) -> Vec<ModelOption> {
    let name_of = |id: &str| {
        providers
            .iter()
            .find(|p| p.id == id)
            .map(|p| p.name.clone().unwrap_or_else(|| p.id.clone()))
            .unwrap_or_else(|| id.to_string())
    };
    let mut options: Vec<ModelOption> = models
        .iter()
        .filter(|m| m.status.as_deref() != Some("deprecated"))
        .map(|m| ModelOption {
            value: format!("{}/{}", m.provider_id, m.id),
            name: m.name.clone().unwrap_or_else(|| m.id.clone()),
            provider_id: m.provider_id.clone(),
            provider_name: name_of(&m.provider_id),
        })
        .collect();
    options.sort_by(|a, b| {
        localized_cmp(&a.provider_name, &b.provider_name)
            .then_with(|| localized_cmp(&a.name, &b.name))
    });
    options
}

/// Case-insensitive first, as `localizedCompare` sorts.
fn localized_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b))
}

// MARK: Binary and server

/// Apps launched from Finder get a minimal PATH, so the usual install locations are searched
/// as well as whatever PATH the app inherited (and the login shell's, when known).
pub fn search_directories(
    path: Option<&str>,
    home: &str,
    login_shell_path: Option<&str>,
) -> Vec<String> {
    let well_known = [
        format!("{home}/.opencode/bin"),
        format!("{home}/.local/bin"),
        format!("{home}/.bun/bin"),
        format!("{home}/.npm-global/bin"),
        "/opt/homebrew/bin".to_string(),
        "/usr/local/bin".to_string(),
        "/usr/bin".to_string(),
    ];
    let mut out: Vec<String> = Vec::new();
    let all = path
        .unwrap_or("")
        .split(':')
        .chain(login_shell_path.unwrap_or("").split(':'))
        .map(str::to_string)
        .chain(well_known);
    for dir in all {
        if !dir.is_empty() && !out.contains(&dir) {
            out.push(dir);
        }
    }
    out
}

/// The first `X.Y.Z` in `opencode --version`'s output.
pub fn parse_version(output: &str) -> Option<String> {
    let bytes = output.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            let mut groups = 0;
            let mut j = i;
            loop {
                let group = j;
                while j < bytes.len() && bytes[j].is_ascii_digit() {
                    j += 1;
                }
                if j == group {
                    break;
                }
                groups += 1;
                if groups == 3 {
                    return Some(output[start..j].to_string());
                }
                if j < bytes.len()
                    && bytes[j] == b'.'
                    && j + 1 < bytes.len()
                    && bytes[j + 1].is_ascii_digit()
                {
                    j += 1;
                } else {
                    break;
                }
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    None
}

pub fn is_supported_version(version: &str) -> bool {
    version
        .split('.')
        .next()
        .and_then(|m| m.parse::<u32>().ok())
        .is_some_and(|m| m >= MINIMUM_MAJOR_VERSION)
}

/// The server's address in `opencode serve`'s "listening on http://…" line.
pub fn parse_listening_url(output: &str) -> Option<String> {
    let lower = output.to_lowercase();
    let at = lower.find("listening on ")? + "listening on ".len();
    let rest = &output[at..];
    if !(rest.starts_with("http://") || rest.starts_with("https://")) {
        return None;
    }
    let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

/// A login shell prints its PATH between these markers, so rc-file output can't confuse it.
pub const PATH_MARKER: &str = "__MDOW_PATH__";

pub fn parse_shell_path(output: &str) -> Option<String> {
    let parts: Vec<&str> = output.split(PATH_MARKER).collect();
    if parts.len() < 3 {
        return None;
    }
    let path = parts[1].trim();
    (!path.is_empty()).then(|| path.to_string())
}

/// A fresh per-launch password for the private server, never written to disk.
pub fn make_password() -> String {
    use std::io::Read;
    let mut bytes = [0u8; 24];
    let filled =
        std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut bytes)).is_ok();
    if !filled {
        // Not expected on macOS; fall back on the hasher's per-process random keys.
        use std::hash::{BuildHasher, Hasher};
        for (i, chunk) in bytes.chunks_mut(8).enumerate() {
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_usize(i);
            h.write_u128(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_nanos()),
            );
            chunk.copy_from_slice(&h.finish().to_le_bytes()[..chunk.len()]);
        }
    }
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// MARK: Unified diff

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffKind {
    Context,
    Add,
    Remove,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub text: String,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

/// `@@ -a[,b] +c[,d] @@ rest`: the two start lines and the rest.
fn hunk_header(line: &str) -> Option<(u32, u32, String)> {
    let rest = line.strip_prefix("@@ -")?;
    let (old, rest) = rest.split_once(' ')?;
    let rest = rest.strip_prefix('+')?;
    let (new, rest) = rest.split_once(" @@")?;
    let start = |s: &str| s.split(',').next().and_then(|n| n.parse::<u32>().ok());
    Some((start(old)?, start(new)?, rest.trim().to_string()))
}

/// Parses the hunks of a unified diff, skipping file headers and "no newline" markers.
pub fn parse_diff(patch: &str) -> Vec<DiffHunk> {
    let mut hunks: Vec<DiffHunk> = Vec::new();
    let (mut old, mut new) = (0u32, 0u32);
    for raw in patch.replace("\r\n", "\n").split('\n') {
        if let Some((o, n, header)) = hunk_header(raw) {
            old = o;
            new = n;
            hunks.push(DiffHunk { header, lines: Vec::new() });
            continue;
        }
        let Some(hunk) = hunks.last_mut() else { continue };
        if raw.starts_with('\\') {
            continue;
        }
        let Some(marker) = raw.chars().next() else { continue };
        let text = raw[marker.len_utf8()..].to_string();
        match marker {
            '+' => {
                hunk.lines.push(DiffLine {
                    kind: DiffKind::Add,
                    text,
                    old_line: None,
                    new_line: Some(new),
                });
                new += 1;
            }
            '-' => {
                hunk.lines.push(DiffLine {
                    kind: DiffKind::Remove,
                    text,
                    old_line: Some(old),
                    new_line: None,
                });
                old += 1;
            }
            ' ' => {
                hunk.lines.push(DiffLine {
                    kind: DiffKind::Context,
                    text,
                    old_line: Some(old),
                    new_line: Some(new),
                });
                old += 1;
                new += 1;
            }
            _ => {}
        }
    }
    hunks.retain(|h| !h.lines.is_empty());
    hunks
}

// MARK: Assistant markdown

/// A block of an assistant reply, for simple native rendering.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplyBlock {
    Paragraph(String),
    Heading { level: usize, text: String },
    Code { language: String, text: String },
}

/// Splits markdown into paragraphs (blank-line separated, line breaks kept), ATX headings and
/// fenced code. Inline markdown is left for [`inline`] to style.
pub fn split_reply(markdown: &str) -> Vec<ReplyBlock> {
    let mut blocks: Vec<ReplyBlock> = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    // (marker, language, lines)
    let mut fence: Option<(String, String, Vec<&str>)> = None;

    fn flush(paragraph: &mut Vec<&str>, blocks: &mut Vec<ReplyBlock>) {
        let text = paragraph.join("\n");
        let text = text.trim_matches(|c| c == '\n' || c == '\r');
        if !text.is_empty() {
            blocks.push(ReplyBlock::Paragraph(text.to_string()));
        }
        paragraph.clear();
    }

    for line in markdown.split('\n') {
        let trimmed = line.trim_start_matches(' ');
        if let Some((marker, language, lines)) = &mut fence {
            let first = marker.chars().next().unwrap_or('`');
            if trimmed.starts_with(marker.as_str()) && trimmed.trim().chars().all(|c| c == first) {
                blocks
                    .push(ReplyBlock::Code { language: language.clone(), text: lines.join("\n") });
                fence = None;
            } else {
                lines.push(line);
            }
            continue;
        }
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            flush(&mut paragraph, &mut blocks);
            let first = trimmed.chars().next().unwrap();
            let marker: String = trimmed.chars().take_while(|c| *c == first).collect();
            let language = trimmed[marker.len()..].trim().to_string();
            fence = Some((marker, language, Vec::new()));
            continue;
        }
        if trimmed.trim().is_empty() {
            flush(&mut paragraph, &mut blocks);
            continue;
        }
        if trimmed.starts_with('#') {
            let hashes = trimmed.chars().take_while(|c| *c == '#').count();
            let rest = &trimmed[hashes..];
            if hashes <= 6 && (rest.is_empty() || rest.starts_with(' ')) {
                flush(&mut paragraph, &mut blocks);
                blocks.push(ReplyBlock::Heading { level: hashes, text: rest.trim().to_string() });
                continue;
            }
        }
        paragraph.push(line);
    }
    if let Some((_, language, lines)) = fence {
        // Still streaming: show the unfinished block as code.
        blocks.push(ReplyBlock::Code { language, text: lines.join("\n") });
    }
    flush(&mut paragraph, &mut blocks);
    blocks
}

/// A paragraph's lines, with list items pulled out so they can hang their wrapped lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplyLine {
    Text(String),
    Item { marker: String, depth: usize, text: String },
}

/// `-`, `*`, `+`, `1.` or `1)` followed by whitespace: the marker and the byte where text starts.
fn list_marker(rest: &str) -> Option<(String, usize)> {
    let b = rest.as_bytes();
    let end = if matches!(b.first(), Some(b'-' | b'*' | b'+')) {
        1
    } else {
        let digits = b.iter().take_while(|c| c.is_ascii_digit()).count();
        if digits == 0 || !matches!(b.get(digits), Some(b'.' | b')')) {
            return None;
        }
        digits + 1
    };
    let spaces = rest[end..].chars().take_while(|c| c.is_whitespace()).count();
    if spaces == 0 {
        return None;
    }
    Some((rest[..end].to_string(), end + spaces))
}

pub fn split_lines(paragraph: &str) -> Vec<ReplyLine> {
    let mut out: Vec<ReplyLine> = Vec::new();
    for line in paragraph.split('\n') {
        let indent = line.chars().take_while(|c| *c == ' ').count();
        let rest = &line[indent..];
        if let Some((marker, at)) = list_marker(rest) {
            let bullet = matches!(marker.as_str(), "-" | "*" | "+");
            out.push(ReplyLine::Item {
                marker: if bullet { "•".into() } else { marker },
                depth: indent / 2,
                text: rest[at..].to_string(),
            });
        } else if indent > 0
            && let Some(ReplyLine::Item { text, .. }) = out.last_mut()
        {
            // An indented line under an item continues it.
            text.push(' ');
            text.push_str(rest.trim());
        } else if let Some(ReplyLine::Text(text)) = out.last_mut() {
            text.push('\n');
            text.push_str(line);
        } else {
            out.push(ReplyLine::Text(line.to_string()));
        }
    }
    out
}

/// Inline styles in a reply line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlineStyle {
    Strong,
    Emphasis,
    Code,
    Strike,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Inline {
    pub text: String,
    pub styles: Vec<(std::ops::Range<usize>, InlineStyle)>,
    pub links: Vec<(std::ops::Range<usize>, String)>,
}

/// Inline markdown (`**strong**`, `*emphasis*`, `` `code` ``, `~~strike~~` and
/// `[links](url)`) as plain text with styled byte ranges. Markers without a partner stay as
/// typed, as streaming text often has them.
pub fn inline(source: &str) -> Inline {
    let mut out = Inline::default();
    parse_inline(source, &mut out, 0);
    out
}

fn parse_inline(src: &str, out: &mut Inline, depth: usize) {
    let mut i = 0;
    while i < src.len() {
        let rest = &src[i..];
        let c = rest.chars().next().unwrap();
        // Escapes.
        if c == '\\'
            && let Some(next) = rest[1..].chars().next()
            && next.is_ascii_punctuation()
        {
            out.text.push(next);
            i += 1 + next.len_utf8();
            continue;
        }
        // Code: a run of backticks to the same run.
        if c == '`' {
            let ticks = rest.bytes().take_while(|b| *b == b'`').count();
            let fence = &rest[..ticks];
            if let Some(close) = rest[ticks..].find(fence) {
                let code = &rest[ticks..ticks + close];
                let code = if code.len() > 2 && code.starts_with(' ') && code.ends_with(' ') {
                    &code[1..code.len() - 1]
                } else {
                    code
                };
                let start = out.text.len();
                out.text.push_str(code);
                out.styles.push((start..out.text.len(), InlineStyle::Code));
                i += ticks + close + ticks;
                continue;
            }
            out.text.push_str(fence);
            i += ticks;
            continue;
        }
        // Links.
        if c == '['
            && depth < 4
            && let Some(close) = find_closing_bracket(rest)
            && rest[close + 1..].starts_with('(')
            && let Some(paren) = rest[close + 2..].find(')')
        {
            let label = &rest[1..close];
            let url = rest[close + 2..close + 2 + paren].trim();
            let start = out.text.len();
            parse_inline(label, out, depth + 1);
            out.links.push((start..out.text.len(), url.to_string()));
            i += close + 2 + paren + 1;
            continue;
        }
        // Emphasis, strong and strike.
        if matches!(c, '*' | '_' | '~') && depth < 4 {
            let run = rest.bytes().take_while(|b| *b == c as u8).count();
            let (marker_len, style) = match (c, run) {
                ('~', n) if n >= 2 => (2, InlineStyle::Strike),
                ('~', _) => (0, InlineStyle::Strike),
                (_, n) if n >= 2 => (2, InlineStyle::Strong),
                _ => (1, InlineStyle::Emphasis),
            };
            let before = if i == 0 { None } else { src[..i].chars().next_back() };
            let opens = marker_len > 0
                && rest[marker_len..].chars().next().is_some_and(|n| !n.is_whitespace())
                // `snake_case` words keep their underscores.
                && !(c == '_' && before.is_some_and(char::is_alphanumeric));
            if opens {
                let marker = &rest[..marker_len];
                if let Some(close) = find_closer(&rest[marker_len..], marker, c) {
                    let inner = &rest[marker_len..marker_len + close];
                    let start = out.text.len();
                    parse_inline(inner, out, depth + 1);
                    out.styles.push((start..out.text.len(), style));
                    i += marker_len + close + marker_len;
                    continue;
                }
            }
            let n = run.max(1);
            out.text.push_str(&rest[..n]);
            i += n;
            continue;
        }
        out.text.push(c);
        i += c.len_utf8();
    }
}

/// The `]` that closes the `[` at the start of `s`.
fn find_closing_bracket(s: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, c) in s.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Where `marker` closes in `s`: not after whitespace, and (for `_`) not inside a word.
fn find_closer(s: &str, marker: &str, c: char) -> Option<usize> {
    let mut from = 0;
    while let Some(at) = s[from..].find(marker) {
        let at = from + at;
        let before = s[..at].chars().next_back();
        let after = s[at + marker.len()..].chars().next();
        let single = marker.len() == 1;
        // A single `*` that is half of a `**` belongs to the strong marker.
        let doubled = single && (after == Some(c) || (at > 0 && s[..at].ends_with(c)));
        let ok = at > 0
            && before.is_some_and(|b| !b.is_whitespace())
            && !doubled
            && !(c == '_' && after.is_some_and(char::is_alphanumeric));
        if ok {
            // `***both***`: a strong marker closes at the end of its run.
            let mut end = at + marker.len();
            while !single && s[end..].starts_with(c) {
                end += c.len_utf8();
            }
            return Some(end - marker.len());
        }
        from = at + marker.len();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn uses_the_open_folder_else_the_document_folder() {
        let doc = PathBuf::from("/docs/guide/intro.md");
        assert_eq!(session_directory(Some(&doc), Some(Path::new("/docs")), "/h"), "/docs");
        assert_eq!(session_directory(Some(&doc), None, "/h"), "/docs/guide");
        assert_eq!(session_directory(None, None, "/Users/me"), "/Users/me");
    }

    #[test]
    fn prefixes_the_prompt_with_what_the_user_is_viewing() {
        assert_eq!(
            compose_prompt(
                "Tighten the intro",
                Some("/docs/guide/intro.md"),
                &[ContextTag::new("/docs/notes.md")],
                "/docs"
            ),
            "<mdow-context>\nViewing: guide/intro.md\nAttached: notes.md\n</mdow-context>\n\nTighten the intro"
        );
        assert_eq!(compose_prompt("Hi", None, &[], "/docs"), "Hi");
        assert!(
            compose_prompt("Hi", Some("/elsewhere/a.md"), &[], "/docs")
                .contains("Viewing: /elsewhere/a.md")
        );
    }

    #[test]
    fn lists_models_grouped_by_provider_without_deprecated_ones() {
        let m = |id: &str, provider: &str, name: &str, status: Option<&str>| ModelInfo {
            id: id.into(),
            provider_id: provider.into(),
            name: Some(name.into()),
            status: status.map(Into::into),
        };
        let options = model_options(
            &[
                m("inclusionai/ling", "openrouter", "Ling", None),
                m("claude-sonnet-5-5", "opencode", "Claude Sonnet 5.5", None),
                m("old", "opencode", "Old", Some("deprecated")),
            ],
            &[
                ProviderInfo { id: "opencode".into(), name: Some("OpenCode Zen".into()) },
                ProviderInfo { id: "openrouter".into(), name: Some("OpenRouter".into()) },
            ],
        );
        let values: Vec<&str> = options.iter().map(|o| o.value.as_str()).collect();
        assert_eq!(values, ["opencode/claude-sonnet-5-5", "openrouter/inclusionai/ling"]);
        assert_eq!(options[0].provider_name, "OpenCode Zen");
    }

    #[test]
    fn requires_opencode_2_or_newer() {
        assert_eq!(parse_version("opencode v2.0.16\n").as_deref(), Some("2.0.16"));
        assert_eq!(parse_version("v1.2 then 3.4.5").as_deref(), Some("3.4.5"));
        assert_eq!(parse_version("none"), None);
        assert!(is_supported_version("2.0.16"));
        assert!(!is_supported_version("1.18.34"));
    }

    #[test]
    fn searches_the_install_folder_even_when_path_is_minimal() {
        let dirs = search_directories(Some("/usr/bin"), "/Users/me", Some("/opt/x/bin:/usr/bin"));
        assert_eq!(dirs[0], "/usr/bin");
        assert_eq!(dirs[1], "/opt/x/bin");
        assert!(dirs.contains(&"/Users/me/.opencode/bin".to_string()));
        assert!(dirs.contains(&"/opt/homebrew/bin".to_string()));
        let mut unique = dirs.clone();
        unique.dedup();
        assert_eq!(unique.len(), dirs.len());
    }

    #[test]
    fn finds_the_listening_url_and_the_shell_path() {
        assert_eq!(
            parse_listening_url("boot\nServer listening on http://127.0.0.1:4096\n").as_deref(),
            Some("http://127.0.0.1:4096")
        );
        assert_eq!(parse_listening_url("starting…"), None);
        assert_eq!(
            parse_shell_path("motd\n__MDOW_PATH__/a:/b__MDOW_PATH__").as_deref(),
            Some("/a:/b")
        );
        assert_eq!(parse_shell_path("nothing"), None);
    }

    #[test]
    fn passwords_are_long_and_fresh() {
        let password = make_password();
        assert_eq!(password.len(), 48);
        assert!(password.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(password, make_password());
    }

    #[test]
    fn parses_unified_diffs() {
        let hunks = parse_diff(
            "Index: x\n===\n--- a\n+++ b\n@@ -1,3 +1,3 @@ Title\n # Notes\n \n-Teh fox.\n+The fox.\n\\ No newline at end of file\n",
        );
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].header, "Title");
        let kinds: Vec<DiffKind> = hunks[0].lines.iter().map(|l| l.kind).collect();
        assert_eq!(kinds, [DiffKind::Context, DiffKind::Context, DiffKind::Remove, DiffKind::Add]);
        assert_eq!(
            hunks[0].lines[2],
            DiffLine {
                kind: DiffKind::Remove,
                text: "Teh fox.".into(),
                old_line: Some(3),
                new_line: None
            }
        );
        assert_eq!(
            hunks[0].lines[3],
            DiffLine {
                kind: DiffKind::Add,
                text: "The fox.".into(),
                old_line: None,
                new_line: Some(3)
            }
        );
        assert!(parse_diff("").is_empty());
    }

    #[test]
    fn splits_replies_into_blocks() {
        assert_eq!(
            split_reply(
                "## Title\nSome *text*\nmore\n\n```swift\nlet a = 1\n\nlet b = 2\n```\n- one\n- two"
            ),
            vec![
                ReplyBlock::Heading { level: 2, text: "Title".into() },
                ReplyBlock::Paragraph("Some *text*\nmore".into()),
                ReplyBlock::Code {
                    language: "swift".into(),
                    text: "let a = 1\n\nlet b = 2".into()
                },
                ReplyBlock::Paragraph("- one\n- two".into()),
            ]
        );
        assert_eq!(
            split_reply("```\nunfinished"),
            vec![ReplyBlock::Code { language: String::new(), text: "unfinished".into() }]
        );
        assert_eq!(split_reply("#hashtag"), vec![ReplyBlock::Paragraph("#hashtag".into())]);
    }

    #[test]
    fn splits_paragraphs_into_lines_and_list_items() {
        assert_eq!(
            split_lines("Intro\nstill intro\n- one\n  wrapped\n  1. nested\n2) two"),
            vec![
                ReplyLine::Text("Intro\nstill intro".into()),
                ReplyLine::Item { marker: "•".into(), depth: 0, text: "one wrapped".into() },
                ReplyLine::Item { marker: "1.".into(), depth: 1, text: "nested".into() },
                ReplyLine::Item { marker: "2)".into(), depth: 0, text: "two".into() },
            ]
        );
    }

    #[test]
    fn styles_inline_markdown() {
        let i = inline("A **bold** and *soft* `code` [link](https://x.y) snake_case_name 2 * 3");
        assert_eq!(i.text, "A bold and soft code link snake_case_name 2 * 3");
        let range = |s: &str| {
            let at = i.text.find(s).unwrap();
            at..at + s.len()
        };
        assert!(i.styles.contains(&(range("bold"), InlineStyle::Strong)));
        assert!(i.styles.contains(&(range("soft"), InlineStyle::Emphasis)));
        assert!(i.styles.contains(&(range("code"), InlineStyle::Code)));
        assert_eq!(i.links, vec![(range("link"), "https://x.y".to_string())]);
        // Unclosed markers stay as typed while a reply streams in.
        assert_eq!(inline("**half").text, "**half");
        assert_eq!(inline("`open").text, "`open");
        assert_eq!(inline("***both***").text, "both");
    }
}
