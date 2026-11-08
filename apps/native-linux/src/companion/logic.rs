//! The pure parts of the companion: session setup, prompts, model lists, finding the server's
//! address, unified diffs and splitting replies for display.

use super::client::{ModelInfo, PermissionRule, ProviderInfo, RuleEffect};
use super::types::*;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub const INSTRUCTIONS: &str = "You are the writing companion inside Mdow, a calm reader for Markdown and HTML documents. The user is reading the documents in this folder and wants help understanding and improving them.

- Answer questions about the documents. Read them with your tools instead of guessing, and mention documents by their path relative to the folder.
- When the user asks for a change, make it with your edit or write tool right away. Mdow shows the user every change as a diff to approve or reject before it is written, so do not ask for confirmation first.
- Keep edits focused. Preserve each document's voice, structure and formatting, whether it is Markdown or HTML.
- Do not run shell commands, reach outside this folder, or create new files unless the user asks for one.
- Reply concisely in Markdown.";

/// Shell commands and other folders are off limits; edits always come back for review.
pub fn session_permissions() -> Vec<PermissionRule> {
    let rule = |action: &str, effect| PermissionRule {
        action: action.to_string(),
        resource: "*".to_string(),
        effect,
    };
    vec![
        rule("shell", RuleEffect::Deny),
        rule("bash", RuleEffect::Deny),
        rule("edit", RuleEffect::Ask),
    ]
}

/// Mdow talks to the v2 HTTP API (`opencode serve`), which shipped in OpenCode 2.0.
pub const MINIMUM_MAJOR_VERSION: u32 = 2;

/// The open folder, else the document's folder, else home.
pub fn session_directory(document: Option<&Path>, folder: Option<&Path>, home: &Path) -> String {
    let directory = folder.or_else(|| document.and_then(Path::parent)).unwrap_or(home);
    directory.to_string_lossy().into_owned()
}

fn display_path<'a>(path: &'a str, directory: &str) -> &'a str {
    let base =
        if directory.ends_with('/') { directory.to_string() } else { format!("{directory}/") };
    match path.strip_prefix(&base) {
        Some(rest) if !rest.is_empty() => rest,
        _ => path,
    }
}

/// Prefixes the user's message with what they are looking at in Mdow.
pub fn compose_prompt(
    text: &str,
    active_path: Option<&str>,
    tags: &[ContextTag],
    directory: &str,
) -> String {
    let mut lines = Vec::new();
    if let Some(active) = active_path {
        lines.push(format!("Viewing: {}", display_path(active, directory)));
    }
    let attached: Vec<&str> = tags.iter().map(|t| display_path(&t.path, directory)).collect();
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
    let names: HashMap<&str, &str> = providers
        .iter()
        .map(|p| (p.id.as_str(), p.name.as_deref().unwrap_or(p.id.as_str())))
        .collect();
    let mut options: Vec<ModelOption> = models
        .iter()
        .filter(|m| m.status.as_deref() != Some("deprecated"))
        .map(|m| ModelOption {
            value: format!("{}/{}", m.provider_id, m.id),
            name: m.name.clone().unwrap_or_else(|| m.id.clone()),
            provider_id: m.provider_id.clone(),
            provider_name: names
                .get(m.provider_id.as_str())
                .unwrap_or(&m.provider_id.as_str())
                .to_string(),
        })
        .collect();
    options.sort_by_cached_key(|o| (o.provider_name.to_lowercase(), o.name.to_lowercase()));
    options
}

/// Desktop launchers get a minimal PATH, so the usual install locations are searched as well as
/// whatever PATH the app inherited (and the login shell's, when known).
pub fn search_directories(
    path: Option<&str>,
    home: &str,
    login_shell_path: Option<&str>,
) -> Vec<String> {
    let well_known = [
        format!("{home}/.opencode/bin"),
        format!("{home}/.local/bin"),
        format!("{home}/.local/share/mise/shims"),
        format!("{home}/.bun/bin"),
        format!("{home}/.npm-global/bin"),
        "/usr/local/bin".to_string(),
        "/usr/bin".to_string(),
    ];
    let mut seen = HashSet::new();
    path.unwrap_or_default()
        .split(':')
        .chain(login_shell_path.unwrap_or_default().split(':'))
        .map(str::to_string)
        .chain(well_known)
        .filter(|dir| !dir.is_empty() && seen.insert(dir.clone()))
        .collect()
}

/// The first `major.minor.patch` in `opencode --version` output.
pub fn parse_version(output: &str) -> Option<String> {
    output.split(|c: char| !c.is_ascii_digit() && c != '.').find_map(|word| {
        let parts: Vec<&str> = word.split('.').collect();
        (parts.len() >= 3 && parts[..3].iter().all(|p| !p.is_empty())).then(|| parts[..3].join("."))
    })
}

pub fn is_supported_version(version: &str) -> bool {
    version
        .split('.')
        .next()
        .and_then(|major| major.parse::<u32>().ok())
        .is_some_and(|major| major >= MINIMUM_MAJOR_VERSION)
}

pub fn parse_listening_url(output: &str) -> Option<String> {
    let lower = output.to_lowercase();
    let start = lower.find("listening on ")? + "listening on ".len();
    let rest = output.get(start..)?;
    let url = rest.split_whitespace().next()?;
    (url.starts_with("http://") || url.starts_with("https://")).then(|| url.to_string())
}

/// A login shell prints its PATH between these markers, so rc-file output can't confuse it.
pub const PATH_MARKER: &str = "__MDOW_PATH__";

pub fn parse_shell_path(output: &str) -> Option<String> {
    let mut parts = output.split(PATH_MARKER);
    parts.next();
    let path = parts.next()?.trim();
    parts.next()?;
    (!path.is_empty()).then(|| path.to_string())
}

/// The text after a trailing `@` in the composer, if the user is typing a mention.
pub fn mention_query(text: &str) -> Option<&str> {
    let at = text.rfind('@')?;
    let (before, query) = (&text[..at], &text[at + 1..]);
    let starts_word = before.chars().next_back().is_none_or(char::is_whitespace);
    (starts_word && !query.chars().any(char::is_whitespace)).then_some(query)
}

/// The composer's text without its trailing mention.
pub fn without_mention(text: &str) -> &str {
    match mention_query(text) {
        Some(query) => &text[..text.len() - query.len() - 1],
        None => text,
    }
}

// ── Unified diff ─────────────────────────────────────────────────────────────

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

fn parse_hunk_header(line: &str) -> Option<(u32, u32, String)> {
    let rest = line.strip_prefix("@@ -")?;
    let (ranges, header) = rest.split_once(" @@")?;
    let (old, new) = ranges.split_once(" +")?;
    let start = |range: &str| range.split(',').next()?.parse::<u32>().ok();
    Some((start(old)?, start(new)?, header.trim().to_string()))
}

/// Parses the hunks of a unified diff, skipping file headers and "no newline" markers.
pub fn parse_diff(patch: &str) -> Vec<DiffHunk> {
    let mut hunks: Vec<DiffHunk> = Vec::new();
    let (mut old_line, mut new_line) = (0, 0);
    for raw in patch.replace("\r\n", "\n").split('\n') {
        if let Some((old, new, header)) = parse_hunk_header(raw) {
            old_line = old;
            new_line = new;
            hunks.push(DiffHunk { header, lines: Vec::new() });
            continue;
        }
        let Some(hunk) = hunks.last_mut() else { continue };
        let mut chars = raw.chars();
        let Some(marker) = chars.next() else { continue };
        let text = chars.as_str().to_string();
        match marker {
            '+' => {
                hunk.lines.push(DiffLine {
                    kind: DiffKind::Add,
                    text,
                    old_line: None,
                    new_line: Some(new_line),
                });
                new_line += 1;
            }
            '-' => {
                hunk.lines.push(DiffLine {
                    kind: DiffKind::Remove,
                    text,
                    old_line: Some(old_line),
                    new_line: None,
                });
                old_line += 1;
            }
            ' ' => {
                hunk.lines.push(DiffLine {
                    kind: DiffKind::Context,
                    text,
                    old_line: Some(old_line),
                    new_line: Some(new_line),
                });
                old_line += 1;
                new_line += 1;
            }
            _ => {}
        }
    }
    hunks.retain(|h| !h.lines.is_empty());
    hunks
}

// ── Assistant markdown ───────────────────────────────────────────────────────

/// A block of an assistant reply.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplyBlock {
    Paragraph(String),
    Heading { level: usize, text: String },
    Code { language: String, text: String },
}

/// Splits markdown into paragraphs (blank-line separated, line breaks kept), ATX headings and
/// fenced code. Inline markdown is left for the caller to style.
pub fn split_reply(markdown: &str) -> Vec<ReplyBlock> {
    let mut blocks = Vec::new();
    let mut paragraph: Vec<&str> = Vec::new();
    let mut fence: Option<(String, String, Vec<&str>)> = None;

    fn flush(paragraph: &mut Vec<&str>, blocks: &mut Vec<ReplyBlock>) {
        let joined = paragraph.join("\n");
        let text = joined.trim_matches('\n');
        if !text.is_empty() {
            blocks.push(ReplyBlock::Paragraph(text.to_string()));
        }
        paragraph.clear();
    }

    for line in markdown.split('\n') {
        let trimmed = line.trim_start_matches(' ');
        if let Some((marker, language, lines)) = &mut fence {
            let closing = trimmed.starts_with(marker.as_str())
                && trimmed.trim().chars().all(|c| marker.starts_with(c));
            if closing {
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
            let first = trimmed.chars().next().unwrap_or('`');
            let length = trimmed.chars().take_while(|c| *c == first).count();
            let marker: String = trimmed.chars().take(length).collect();
            let language = trimmed[marker.len()..].trim().to_string();
            fence = Some((marker, language, Vec::new()));
            continue;
        }
        if trimmed.is_empty() {
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

fn list_marker(line: &str) -> Option<(&str, &str)> {
    let end = match line.chars().next()? {
        '-' | '*' | '+' => 1,
        c if c.is_ascii_digit() => {
            let digits = line.chars().take_while(char::is_ascii_digit).count();
            if !matches!(line[digits..].chars().next(), Some('.' | ')')) {
                return None;
            }
            digits + 1
        }
        _ => return None,
    };
    let (marker, rest) = line.split_at(end);
    let text = rest.trim_start();
    (text.len() < rest.len()).then_some((marker, text))
}

pub fn split_reply_lines(paragraph: &str) -> Vec<ReplyLine> {
    let mut result: Vec<ReplyLine> = Vec::new();
    for line in paragraph.split('\n') {
        let rest = line.trim_start_matches(' ');
        let indent = line.len() - rest.len();
        if let Some((marker, text)) = list_marker(rest) {
            let bullet = matches!(marker, "-" | "*" | "+");
            result.push(ReplyLine::Item {
                marker: if bullet { "•".to_string() } else { marker.to_string() },
                depth: indent / 2,
                text: text.to_string(),
            });
            continue;
        }
        match result.last_mut() {
            // An indented line under an item continues it.
            Some(ReplyLine::Item { text, .. }) if indent > 0 => {
                text.push(' ');
                text.push_str(rest.trim());
            }
            Some(ReplyLine::Text(text)) => {
                text.push('\n');
                text.push_str(line);
            }
            _ => result.push(ReplyLine::Text(line.to_string())),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(id: &str, provider: &str, name: &str, status: Option<&str>) -> ModelInfo {
        ModelInfo {
            id: id.to_string(),
            provider_id: provider.to_string(),
            name: Some(name.to_string()),
            status: status.map(str::to_string),
        }
    }

    #[test]
    fn uses_the_open_folder_else_the_document_folder() {
        let doc = Path::new("/docs/guide/intro.md");
        let home = Path::new("/home/me");
        assert_eq!(session_directory(Some(doc), Some(Path::new("/docs")), home), "/docs");
        assert_eq!(session_directory(Some(doc), None, home), "/docs/guide");
        assert_eq!(session_directory(None, None, home), "/home/me");
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
        let options = model_options(
            &[
                model("inclusionai/ling", "openrouter", "Ling", None),
                model("claude-sonnet-5-5", "opencode", "Claude Sonnet 5.5", None),
                model("old", "opencode", "Old", Some("deprecated")),
            ],
            &[
                ProviderInfo { id: "opencode".to_string(), name: Some("OpenCode Zen".to_string()) },
                ProviderInfo { id: "openrouter".to_string(), name: Some("OpenRouter".to_string()) },
            ],
        );
        let values: Vec<&str> = options.iter().map(|o| o.value.as_str()).collect();
        assert_eq!(values, ["opencode/claude-sonnet-5-5", "openrouter/inclusionai/ling"]);
        assert_eq!(options[0].provider_name, "OpenCode Zen");
    }

    #[test]
    fn requires_opencode_2_or_newer() {
        assert_eq!(parse_version("opencode v2.0.16\n").as_deref(), Some("2.0.16"));
        assert!(is_supported_version("2.0.16"));
        assert!(!is_supported_version("1.18.34"));
        assert_eq!(parse_version("nothing here"), None);
    }

    #[test]
    fn searches_the_install_folder_even_when_path_is_minimal() {
        let dirs = search_directories(Some("/usr/bin"), "/home/me", Some("/opt/x/bin:/usr/bin"));
        assert_eq!(dirs[0], "/usr/bin");
        assert_eq!(dirs[1], "/opt/x/bin");
        assert!(dirs.iter().any(|d| d == "/home/me/.opencode/bin"));
        assert!(dirs.iter().any(|d| d == "/home/me/.local/share/mise/shims"));
        let unique: HashSet<&String> = dirs.iter().collect();
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
                text: "Teh fox.".to_string(),
                old_line: Some(3),
                new_line: None
            }
        );
        assert_eq!(
            hunks[0].lines[3],
            DiffLine {
                kind: DiffKind::Add,
                text: "The fox.".to_string(),
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
            [
                ReplyBlock::Heading { level: 2, text: "Title".to_string() },
                ReplyBlock::Paragraph("Some *text*\nmore".to_string()),
                ReplyBlock::Code {
                    language: "swift".to_string(),
                    text: "let a = 1\n\nlet b = 2".to_string()
                },
                ReplyBlock::Paragraph("- one\n- two".to_string()),
            ]
        );
        assert_eq!(
            split_reply("```\nunfinished"),
            [ReplyBlock::Code { language: String::new(), text: "unfinished".to_string() }]
        );
        assert_eq!(split_reply("#hashtag"), [ReplyBlock::Paragraph("#hashtag".to_string())]);
    }

    #[test]
    fn finds_the_mention_being_typed() {
        assert_eq!(mention_query("@"), Some(""));
        assert_eq!(mention_query("look at @no"), Some("no"));
        assert_eq!(mention_query("me@example.com"), None);
        assert_eq!(mention_query("@notes and more"), None);
        assert_eq!(mention_query("plain"), None);
        assert_eq!(without_mention("look at @no"), "look at ");
        assert_eq!(without_mention("plain"), "plain");
    }

    #[test]
    fn list_items_hang_and_text_stays_together() {
        assert_eq!(
            split_reply_lines("Intro\nmore\n- one\n  continued\n  - nested\n2. two"),
            [
                ReplyLine::Text("Intro\nmore".to_string()),
                ReplyLine::Item {
                    marker: "•".to_string(),
                    depth: 0,
                    text: "one continued".to_string()
                },
                ReplyLine::Item { marker: "•".to_string(), depth: 1, text: "nested".to_string() },
                ReplyLine::Item { marker: "2.".to_string(), depth: 0, text: "two".to_string() },
            ]
        );
        assert_eq!(split_reply_lines("*bold* text"), [ReplyLine::Text("*bold* text".to_string())]);
    }
}
