//! What the companion sends with each question, ported from Electron's `context-selection`,
//! `context-ledger`, `context-builder`, `retrieval` and the service's `CitationStream`.
//!
//! The focused document stays lean: small documents go whole, large ones are cut down to a
//! heading map plus the sections that match the question, and a document already sent in this
//! session is replaced by its hash and outline. Other files are only searched when the question
//! is structurally cross-document.

use super::types::{ContextTag, ContextTrace, RetrievalMode, TagKind, TraceItem, TraceReason};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

const MAX_MAP_BYTES: usize = 4_096;
const MAX_LINK_MANIFEST_BYTES: usize = 2_048;
const MAX_INITIAL_SOURCE_BYTES: usize = 16_384;
const MAX_INITIAL_TOTAL_BYTES: usize = 16_384;
const MAX_TURN_SOURCE_BYTES: usize = 32_768;
const MAX_FILES_SCANNED: usize = 200;
const MAX_RANGES: usize = 3;
const MAX_RANGE_BYTES: usize = 4_096;

const SELECTION_STOP_WORDS: &[&str] = &[
    "about", "also", "does", "from", "have", "into", "that", "their", "this", "what", "when",
    "where", "which", "with", "would",
];
const RETRIEVAL_STOP_WORDS: &[&str] = &[
    "about", "does", "from", "have", "into", "that", "this", "what", "when", "where", "which",
    "with",
];

/// Cuts `text` to at most `max_bytes` bytes on a character boundary.
pub fn truncate_utf8(text: &str, max_bytes: usize) -> &str {
    if text.len() <= max_bytes {
        return text;
    }
    let mut end = max_bytes;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownHeading {
    pub depth: usize,
    pub text: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownLink {
    pub label: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedMarkdown {
    pub excerpt: String,
    pub bytes: usize,
    pub whole_document: bool,
    pub headings: Vec<MarkdownHeading>,
    pub links: Vec<MarkdownLink>,
}

fn split_lines(content: &str) -> Vec<&str> {
    content
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect()
}

/// `^(#{1,6})\s+(.+?)\s*#*\s*$`
fn parse_heading(line: &str) -> Option<(usize, String)> {
    let depth = line.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&depth) {
        return None;
    }
    let rest = &line[depth..];
    if !rest.starts_with(|ch: char| ch.is_whitespace()) {
        return None;
    }
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    let stripped = rest.trim_end_matches('#').trim_end();
    let text = if stripped.is_empty() { rest } else { stripped };
    Some((depth, text.to_owned()))
}

fn parse_headings(lines: &[&str]) -> Vec<(MarkdownHeading, usize)> {
    lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| {
            parse_heading(line).map(|(depth, text)| {
                (
                    MarkdownHeading {
                        depth,
                        text,
                        line: index + 1,
                    },
                    index,
                )
            })
        })
        .collect()
}

fn is_local_link(target: &str) -> bool {
    let normalized = target.trim().to_lowercase();
    !normalized.is_empty()
        && !normalized.starts_with('#')
        && !["http:", "https:", "mailto:", "data:"]
            .iter()
            .any(|scheme| normalized.starts_with(scheme))
}

fn strip_angle(target: &str) -> String {
    let target = target.strip_prefix('<').unwrap_or(target);
    target.strip_suffix('>').unwrap_or(target).to_owned()
}

/// Local markdown links as metadata only (targets are never resolved or read).
pub fn extract_links(content: &str) -> Vec<MarkdownLink> {
    let mut references = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix('[')
            && let Some(close) = rest.find(']')
            && close > 0
            && let Some(after) = rest[close + 1..].strip_prefix(':')
            && let Some(target) = after.split_whitespace().next()
        {
            references.insert(rest[..close].trim().to_lowercase(), strip_angle(target));
        }
    }

    let mut links = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |label: String, target: String| {
        if is_local_link(&target) && seen.insert((label.clone(), target.clone())) {
            links.push(MarkdownLink { label, target });
        }
    };

    let bytes = content.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'[' || (index > 0 && bytes[index - 1] == b'!') {
            index += 1;
            continue;
        }
        let Some(close) = content[index + 1..]
            .find(']')
            .map(|offset| index + 1 + offset)
        else {
            break;
        };
        let label = &content[index + 1..close];
        if label.is_empty() || label.contains('[') {
            index += 1;
            continue;
        }
        let after = &content[close + 1..];
        if let Some(inner) = after.strip_prefix('(') {
            let target_end = inner
                .find(|ch: char| ch == ')' || ch.is_whitespace())
                .unwrap_or(inner.len());
            let target = &inner[..target_end];
            let tail = &inner[target_end..];
            let closes = tail.starts_with(')') || {
                let tail = tail.trim_start();
                let quoted = tail
                    .strip_prefix(['"', '\''])
                    .and_then(|rest| rest.find(['"', '\'']).map(|end| &rest[end + 1..]));
                quoted.is_some_and(|rest| rest.starts_with(')'))
            };
            if !target.is_empty() && closes {
                add(label.trim().to_owned(), strip_angle(target));
            }
        } else if let Some(inner) = after.strip_prefix('[')
            && let Some(end) = inner.find(']')
        {
            let reference = inner[..end].trim();
            let key = if reference.is_empty() {
                label.trim().to_lowercase()
            } else {
                reference.to_lowercase()
            };
            if let Some(target) = references.get(&key) {
                add(label.trim().to_owned(), target.clone());
            }
        }
        index = close + 1;
    }
    links
}

/// `[\p{L}\p{N}_-]{3,}` tokens, lower-cased, minus stop words, deduplicated in order.
fn word_terms(text: &str, stop_words: &[&str]) -> Vec<String> {
    let lowered = text.to_lowercase();
    let mut seen = HashSet::new();
    lowered
        .split(|ch: char| !(ch.is_alphanumeric() || ch == '_' || ch == '-'))
        .filter(|term| term.chars().count() >= 3 && !stop_words.contains(term))
        .filter(|term| seen.insert(term.to_string()))
        .map(str::to_owned)
        .collect()
}

fn question_terms(question: &str) -> Vec<String> {
    word_terms(question, SELECTION_STOP_WORDS)
}

struct Section {
    start: usize,
    text: String,
    score: usize,
}

fn build_sections(
    lines: &[&str],
    headings: &[(MarkdownHeading, usize)],
    terms: &[String],
) -> Vec<Section> {
    headings
        .iter()
        .enumerate()
        .map(|(index, (heading, line_index))| {
            let end = headings[index + 1..]
                .iter()
                .find(|(next, _)| next.depth <= heading.depth)
                .map(|(_, next_line)| *next_line)
                .unwrap_or(lines.len());
            let text = lines[*line_index..end].join("\n").trim().to_owned();
            let heading_text = heading.text.to_lowercase();
            let body = text.to_lowercase();
            let score = terms
                .iter()
                .map(|term| {
                    (if heading_text.contains(term.as_str()) {
                        8
                    } else {
                        0
                    }) + body.matches(term.as_str()).count().min(4)
                })
                .sum();
            Section {
                start: *line_index,
                text,
                score,
            }
        })
        .collect()
}

fn heading_rows(headings: &[MarkdownHeading]) -> Vec<String> {
    headings
        .iter()
        .map(|heading| {
            format!(
                "{}- {} (line {})",
                "  ".repeat(heading.depth.saturating_sub(1)),
                heading.text,
                heading.line
            )
        })
        .collect()
}

fn link_rows(links: &[MarkdownLink]) -> Vec<String> {
    links
        .iter()
        .map(|link| format!("- {}: {}", link.label, link.target))
        .collect()
}

pub fn select_initial_markdown(
    content: &str,
    question: &str,
    max_bytes: usize,
) -> SelectedMarkdown {
    let lines = split_lines(content);
    let positioned = parse_headings(&lines);
    let headings = positioned
        .iter()
        .map(|(heading, _)| heading.clone())
        .collect::<Vec<_>>();
    let links = extract_links(content);

    if content.len() <= max_bytes {
        return SelectedMarkdown {
            excerpt: content.to_owned(),
            bytes: content.len(),
            whole_document: true,
            headings,
            links,
        };
    }

    let terms = question_terms(question);
    let sections = build_sections(&lines, &positioned, &terms);
    let candidates: Vec<&Section> = if sections.len() > 1 {
        sections.iter().skip(1).collect()
    } else {
        sections.iter().collect()
    };
    let relevant = candidates
        .iter()
        .copied()
        .filter(|section| section.score > 0)
        .collect::<Vec<_>>();
    let mut ranked = if relevant.is_empty() {
        candidates
    } else {
        relevant
    };
    ranked.sort_by(|a, b| b.score.cmp(&a.score).then(a.start.cmp(&b.start)));
    ranked.truncate(3);
    ranked.sort_by_key(|section| section.start);

    let map = if headings.is_empty() {
        String::new()
    } else {
        truncate_utf8(
            &format!("## Document map\n{}", heading_rows(&headings).join("\n")),
            MAX_MAP_BYTES,
        )
        .to_owned()
    };
    let manifest = if links.is_empty() {
        String::new()
    } else {
        truncate_utf8(
            &format!(
                "## Linked document manifest\n{}",
                link_rows(&links).join("\n")
            ),
            MAX_LINK_MANIFEST_BYTES,
        )
        .to_owned()
    };
    let blocks = [map, manifest, "## Selected sections".to_owned()]
        .into_iter()
        .filter(|block| !block.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    let mut excerpt = truncate_utf8(&blocks, max_bytes).to_owned();
    for section in ranked {
        let separator = if excerpt.is_empty() { "" } else { "\n\n" };
        let remaining = max_bytes.saturating_sub(excerpt.len() + separator.len());
        if remaining == 0 {
            break;
        }
        let selected = truncate_utf8(&section.text, remaining);
        if selected.is_empty() {
            break;
        }
        excerpt.push_str(separator);
        excerpt.push_str(selected);
    }
    SelectedMarkdown {
        bytes: excerpt.len(),
        excerpt,
        whole_document: false,
        headings,
        links,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerRecord {
    pub hash: String,
    pub already_sent: bool,
}

/// Remembers which document contents this agent session has already seen.
#[derive(Debug, Default)]
pub struct ContextLedger {
    entries: HashMap<PathBuf, String>,
}

pub fn hash_content(content: &str) -> String {
    Sha256::digest(content.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

impl ContextLedger {
    pub fn record(&mut self, path: &Path, content: &str) -> LedgerRecord {
        let hash = hash_content(content);
        let already_sent = self.entries.get(path) == Some(&hash);
        self.entries.insert(path.to_owned(), hash.clone());
        LedgerRecord { hash, already_sent }
    }

    pub fn has(&self, path: &Path, hash: &str) -> bool {
        self.entries.get(path).is_some_and(|stored| stored == hash)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextSource {
    pub source_id: String,
    pub path: PathBuf,
    pub heading_id: Option<String>,
    pub excerpt: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPacket {
    pub sources: Vec<ContextSource>,
    pub warnings: Vec<String>,
    pub summary: String,
    pub trace: ContextTrace,
}

pub fn source_id_for(path: &Path, heading_id: Option<&str>) -> String {
    match heading_id {
        Some(heading) => format!("src:{}#{heading}", path.display()),
        None => format!("src:{}", path.display()),
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

fn format_unchanged_source(
    path: &Path,
    hash: &str,
    headings: &[MarkdownHeading],
    links: &[MarkdownLink],
) -> String {
    let heading_map = heading_rows(headings);
    let link_map = link_rows(links);
    [
        "Content unchanged from earlier in this session.".to_owned(),
        format!("Path: {}", path.display()),
        format!("SHA-256: {hash}"),
        if heading_map.is_empty() {
            String::new()
        } else {
            format!("Headings:\n{}", heading_map.join("\n"))
        },
        if link_map.is_empty() {
            String::new()
        } else {
            format!("Linked documents:\n{}", link_map.join("\n"))
        },
    ]
    .into_iter()
    .filter(|part| !part.is_empty())
    .collect::<Vec<_>>()
    .join("\n\n")
}

pub struct BuildContextInput<'a> {
    pub active_path: Option<&'a Path>,
    pub tags: &'a [ContextTag],
    pub question: &'a str,
}

fn validate_markdown_path(path: &Path) -> Option<PathBuf> {
    if !crate::document::is_supported_markdown(path) {
        return None;
    }
    Some(std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned()))
}

struct Builder<'a> {
    sources: Vec<ContextSource>,
    warnings: Vec<String>,
    items: Vec<TraceItem>,
    seen: HashSet<PathBuf>,
    used: usize,
    question: &'a str,
}

impl Builder<'_> {
    fn add(&mut self, path: &Path, reason: TraceReason, ledger: &mut ContextLedger) {
        let Some(resolved) = validate_markdown_path(path) else {
            self.warnings.push(format!(
                "Skipped non-markdown or invalid path: {}",
                path.display()
            ));
            return;
        };
        if self.seen.contains(&resolved) {
            return;
        }
        if self.used >= MAX_INITIAL_TOTAL_BYTES {
            self.warnings
                .push("Context budget reached; additional files omitted".into());
            return;
        }
        let Ok(raw) = std::fs::read_to_string(&resolved) else {
            self.warnings
                .push(format!("Could not read {}", path.display()));
            return;
        };
        let remaining = MAX_INITIAL_SOURCE_BYTES.min(MAX_INITIAL_TOTAL_BYTES - self.used);
        let selected = select_initial_markdown(&raw, self.question, remaining);
        let record = ledger.record(&resolved, &raw);
        let excerpt = if record.already_sent {
            truncate_utf8(
                &format_unchanged_source(
                    &resolved,
                    &record.hash,
                    &selected.headings,
                    &selected.links,
                ),
                remaining,
            )
            .to_owned()
        } else {
            selected.excerpt.clone()
        };
        let bytes = excerpt.len();
        self.used += bytes;
        self.seen.insert(resolved.clone());
        self.sources.push(ContextSource {
            source_id: source_id_for(&resolved, None),
            path: resolved.clone(),
            heading_id: None,
            excerpt,
            bytes,
        });
        self.items.push(TraceItem {
            path: resolved.clone(),
            reason,
            bytes,
        });
        if !record.already_sent && !selected.whole_document {
            self.warnings.push(format!(
                "Selected relevant sections from {}",
                file_name(&resolved)
            ));
        }
    }
}

/// The focused document first, then attached files. Folder tags only scope retrieval.
pub fn build_context(input: BuildContextInput<'_>, ledger: &mut ContextLedger) -> ContextPacket {
    let mut builder = Builder {
        sources: Vec::new(),
        warnings: Vec::new(),
        items: Vec::new(),
        seen: HashSet::new(),
        used: 0,
        question: input.question,
    };
    if let Some(active) = input.active_path {
        builder.add(active, TraceReason::Focused, ledger);
    }
    for tag in input.tags.iter().filter(|tag| tag.kind == TagKind::File) {
        builder.add(&tag.path, TraceReason::Attached, ledger);
    }

    let names = builder
        .sources
        .iter()
        .map(|source| file_name(&source.path))
        .collect::<Vec<_>>();
    let summary = if names.is_empty() {
        "No docs in context".to_owned()
    } else {
        let more = if names.len() > 3 {
            format!(" + {} more", names.len() - 3)
        } else {
            String::new()
        };
        format!("Using {}{more}", names[..names.len().min(3)].join(", "))
    };
    let injected_bytes = builder.items.iter().map(|item| item.bytes).sum::<usize>();
    let count = |reason| {
        builder
            .items
            .iter()
            .filter(|item| item.reason == reason)
            .count()
    };
    let trace = ContextTrace {
        focused_count: count(TraceReason::Focused),
        attached_count: count(TraceReason::Attached),
        searched_count: 0,
        read_range_count: 0,
        injected_bytes,
        estimated_tokens: injected_bytes.div_ceil(4),
        retrieval_mode: RetrievalMode::FocusedOnly,
        items: builder.items,
    };
    ContextPacket {
        sources: builder.sources,
        warnings: builder.warnings,
        summary,
        trace,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetrievedRange {
    pub path: PathBuf,
    pub excerpt: String,
    pub start_line: usize,
    pub end_line: usize,
    pub bytes: usize,
    pub score: usize,
}

pub fn append_retrieved_context(
    packet: ContextPacket,
    ranges: &[RetrievedRange],
    mode: RetrievalMode,
) -> ContextPacket {
    let ContextPacket {
        mut sources,
        warnings,
        mut trace,
        ..
    } = packet;
    let mut injected = trace.injected_bytes;
    let mut read = 0;
    for range in ranges.iter().take(3) {
        let remaining = MAX_TURN_SOURCE_BYTES.saturating_sub(injected);
        if remaining == 0 {
            break;
        }
        let excerpt = truncate_utf8(&range.excerpt, remaining.min(4_096)).to_owned();
        if excerpt.is_empty() {
            continue;
        }
        let bytes = excerpt.len();
        let heading_id = format!("L{}-L{}", range.start_line, range.end_line);
        sources.push(ContextSource {
            source_id: source_id_for(&range.path, Some(&heading_id)),
            path: range.path.clone(),
            heading_id: Some(heading_id),
            excerpt,
            bytes,
        });
        trace.items.push(TraceItem {
            path: range.path.clone(),
            reason: TraceReason::Retrieved,
            bytes,
        });
        injected += bytes;
        read += 1;
    }
    let summary = format!("{} focused · searched 1 · read {read}", trace.focused_count);
    trace.searched_count = 1;
    trace.read_range_count = read;
    trace.injected_bytes = injected;
    trace.estimated_tokens = injected.div_ceil(4);
    trace.retrieval_mode = mode;
    ContextPacket {
        sources,
        warnings,
        summary,
        trace,
    }
}

pub fn format_context_prompt(packet: &ContextPacket, question: &str) -> String {
    let blocks = packet
        .sources
        .iter()
        .map(|source| {
            format!(
                "### Source {}\nPath: {}\n\n```markdown\n{}\n```",
                source.source_id,
                source.path.display(),
                source.excerpt
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    [
        "You are Mdow Companion, a read-only docs assistant.",
        "Answer using the provided markdown sources.",
        "Cite source IDs like src:/absolute/path.md when making doc-specific claims.",
        "If the docs do not contain enough information, say so.",
        "Search linked files or attached folders only when the question requires more context.",
        "Use only read-only context or search tools made available by Mdow.",
        "Do not edit files, use write tools, run terminal commands, or grant permissions.",
        "",
        "## Docs context",
        if blocks.is_empty() {
            "(no sources)"
        } else {
            &blocks
        },
        "",
        "## User question",
        question,
    ]
    .join("\n")
}

fn stem(term: &str) -> &str {
    const SUFFIXES: &[&str] = &[
        "ations", "ation", "ments", "ment", "ingly", "edly", "ing", "ed", "es", "s",
    ];
    SUFFIXES
        .iter()
        .filter(|suffix| term.ends_with(*suffix))
        .map(|suffix| &term[..term.len() - suffix.len()])
        .min_by_key(|stemmed| stemmed.len())
        .unwrap_or(term)
}

fn retrieval_terms(question: &str) -> Vec<String> {
    let mut terms = Vec::new();
    for term in word_terms(question, RETRIEVAL_STOP_WORDS) {
        let stemmed = stem(&term).to_owned();
        let push = |value: String, terms: &mut Vec<String>| {
            if !terms.contains(&value) {
                terms.push(value);
            }
        };
        let stem_differs = stemmed.chars().count() >= 3 && stemmed != term;
        push(term, &mut terms);
        if stem_differs {
            push(stemmed, &mut terms);
        }
    }
    terms
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .filter(|word| !word.is_empty())
}

/// Cross-document questions only: naming another markdown file, comparison words, or asking
/// about "all the docs" with a folder attached.
pub fn should_retrieve(question: &str, active_path: Option<&Path>, tags: &[ContextTag]) -> bool {
    let normalized = question.to_lowercase();
    let active_name = active_path
        .map(file_name)
        .unwrap_or_default()
        .to_lowercase();
    let names_other_markdown = normalized
        .split(|ch: char| !(ch.is_alphanumeric() || matches!(ch, '_' | '.' | '/' | '-')))
        .map(|token| token.trim_end_matches('.'))
        .filter(|token| {
            [".md", ".markdown", ".mdx"]
                .iter()
                .any(|extension| token.len() > extension.len() && token.ends_with(extension))
        })
        .any(|token| file_name(Path::new(token)) != active_name);
    if names_other_markdown {
        return true;
    }
    const CROSS: &[&str] = &[
        "compare",
        "contrast",
        "difference",
        "across",
        "other",
        "related",
        "reference",
        "references",
    ];
    if words(&normalized).any(|word| CROSS.contains(&word)) {
        return true;
    }
    const COLLECTION: &[&str] = &[
        "all",
        "doc",
        "docs",
        "document",
        "documents",
        "file",
        "files",
        "folder",
        "collection",
    ];
    tags.iter().any(|tag| tag.kind == TagKind::Folder)
        && words(&normalized).any(|word| COLLECTION.contains(&word))
}

fn collect_markdown(dir: &Path, output: &mut Vec<PathBuf>) {
    if output.len() >= MAX_FILES_SCANNED {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        if output.len() >= MAX_FILES_SCANNED {
            return;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "node_modules" {
            continue;
        }
        let path = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => collect_markdown(&path, output),
            Ok(_) if crate::document::is_supported_markdown(&path) => output.push(path),
            _ => {}
        }
    }
}

fn is_heading_line(line: &str) -> bool {
    let depth = line.bytes().take_while(|byte| *byte == b'#').count();
    (1..=6).contains(&depth) && line[depth..].starts_with(|ch: char| ch.is_whitespace())
}

fn range_for_content(path: &Path, content: &str, terms: &[String]) -> Option<RetrievedRange> {
    let lines = split_lines(content);
    let mut best_line = 0;
    let mut best_line_score = 0;
    for (index, line) in lines.iter().enumerate() {
        let normalized = line.to_lowercase();
        let score = terms
            .iter()
            .filter(|term| normalized.contains(term.as_str()))
            .count();
        if score > best_line_score {
            best_line = index;
            best_line_score = score;
        }
    }
    let name = file_name(path).to_lowercase();
    let path_score = terms
        .iter()
        .filter(|term| name.contains(term.as_str()))
        .count()
        * 6;
    let lowered = content.to_lowercase();
    let content_score = terms
        .iter()
        .map(|term| lowered.matches(term.as_str()).count().min(6))
        .sum::<usize>();
    let score = path_score + content_score + best_line_score * 3;
    if score == 0 {
        return None;
    }

    let mut start = best_line;
    for index in (0..=best_line).rev() {
        if is_heading_line(lines[index]) {
            start = index;
            break;
        }
        if best_line - index >= 8 {
            break;
        }
        start = index;
    }

    let mut selected = Vec::new();
    let mut bytes = 0;
    let mut end = start;
    for index in start..lines.len() {
        let line = lines[index];
        let separator = usize::from(!selected.is_empty());
        if bytes + separator + line.len() > MAX_RANGE_BYTES {
            break;
        }
        selected.push(line);
        bytes += separator + line.len();
        end = index;
        if index > best_line + 24
            && lines
                .get(index + 1)
                .is_some_and(|next| is_heading_line(next))
        {
            break;
        }
    }
    Some(RetrievedRange {
        path: path.to_owned(),
        excerpt: selected.join("\n"),
        start_line: start + 1,
        end_line: end + 1,
        bytes,
        score,
    })
}

/// At most three bounded ranges from markdown files under `roots`, best first.
pub fn retrieve_markdown_ranges(
    question: &str,
    roots: &[PathBuf],
    excluded: &[PathBuf],
) -> Vec<RetrievedRange> {
    let canonical = |path: &Path| std::fs::canonicalize(path).unwrap_or_else(|_| path.to_owned());
    let excluded = excluded
        .iter()
        .map(|path| canonical(path))
        .collect::<HashSet<_>>();
    let mut paths = Vec::new();
    let mut seen_roots = HashSet::new();
    for root in roots {
        if !seen_roots.insert(root.clone()) {
            continue;
        }
        collect_markdown(root, &mut paths);
        if paths.len() >= MAX_FILES_SCANNED {
            break;
        }
    }
    let mut unique = HashSet::new();
    let terms = retrieval_terms(question);
    let mut ranges = paths
        .into_iter()
        .filter(|path| unique.insert(path.clone()) && !excluded.contains(&canonical(path)))
        .filter_map(|path| {
            let content = std::fs::read_to_string(&path).ok()?;
            range_for_content(&path, &content, &terms)
        })
        .collect::<Vec<_>>();
    ranges.sort_by(|a, b| b.score.cmp(&a.score).then(a.path.cmp(&b.path)));
    ranges.truncate(MAX_RANGES);
    ranges
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CitationChunk {
    pub text: String,
    pub citation_ids: Vec<String>,
}

/// Strips known source ids out of streamed text, even when an id is split across chunks, and
/// reports each cited source once.
#[derive(Debug, Default)]
pub struct CitationStream {
    buffer: String,
    source_ids: Vec<String>,
    emitted: HashSet<String>,
}

impl CitationStream {
    pub fn new(source_ids: impl IntoIterator<Item = String>) -> Self {
        let mut ids = source_ids
            .into_iter()
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        ids.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
        Self {
            buffer: String::new(),
            source_ids: ids,
            emitted: HashSet::new(),
        }
    }

    pub fn consume(&mut self, text: &str) -> CitationChunk {
        self.buffer.push_str(text);
        let citation_ids = self.remove_known_ids();
        self.buffer = remove_cited_wrappers(&self.buffer);
        let carry = self.carry_length();
        let rest = self.buffer.split_off(self.buffer.len() - carry);
        let visible = std::mem::replace(&mut self.buffer, rest);
        CitationChunk {
            text: visible.replace(CITE, ""),
            citation_ids,
        }
    }

    pub fn flush(&mut self) -> CitationChunk {
        let citation_ids = self.remove_known_ids();
        let text = remove_cited_wrappers(&std::mem::take(&mut self.buffer)).replace(CITE, "");
        CitationChunk { text, citation_ids }
    }

    /// Swaps each known id for a marker, so only wrappers that held a citation are removed.
    fn remove_known_ids(&mut self) -> Vec<String> {
        let mut ids = Vec::new();
        for id in &self.source_ids {
            if !self.buffer.contains(id.as_str()) {
                continue;
            }
            self.buffer = self.buffer.replace(id.as_str(), CITE_STR);
            if self.emitted.insert(id.clone()) {
                ids.push(id.clone());
            }
        }
        ids
    }

    fn carry_length(&self) -> usize {
        let mut carry = 0;
        for id in &self.source_ids {
            let max = (id.len().saturating_sub(1)).min(self.buffer.len());
            for length in (carry + 1..=max).rev() {
                if id.is_char_boundary(length) && self.buffer.ends_with(&id[..length]) {
                    carry = length;
                    break;
                }
            }
        }
        let carry_start = self.buffer.len() - carry;
        let opener_before = self.buffer[..carry_start]
            .chars()
            .next_back()
            .is_some_and(|ch| ch == '(' || ch == '[');
        if carry > 0 && opener_before {
            carry += 1;
        } else if carry == 0 && self.buffer.ends_with(['(', '[']) {
            carry = 1;
        }
        // "(src:a" complete but its ")" not streamed yet: hold the whole wrapper back.
        if let Some(opener) = self.buffer.rfind(['(', '['])
            && self.buffer[opener + 1..].contains(CITE)
            && self.buffer[opener + 1..].chars().all(is_citation_filler)
        {
            carry = carry.max(self.buffer.len() - opener);
        }
        carry
    }
}

const CITE: char = '\u{E000}';
const CITE_STR: &str = "\u{E000}";

fn is_citation_filler(ch: char) -> bool {
    ch == CITE || ch.is_whitespace() || ch == ',' || ch == ';'
}

/// Drops `(...)` and `[...]` that held only citations (Electron removed every empty pair,
/// which also ate `()` from code), with the space before them.
fn remove_cited_wrappers(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars = text.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if ch == '(' || ch == '[' {
            let close = if ch == '(' { ')' } else { ']' };
            let mut next = index + 1;
            let mut cited = false;
            while next < chars.len() && is_citation_filler(chars[next]) {
                cited |= chars[next] == CITE;
                next += 1;
            }
            if cited && next < chars.len() && chars[next] == close {
                if out.ends_with(' ') {
                    out.pop();
                }
                index = next + 1;
                continue;
            }
        }
        out.push(ch);
        index += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_MAX_BYTES: usize = 16_384;

    fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();
        std::fs::canonicalize(path).unwrap()
    }

    #[test]
    fn returns_a_small_markdown_document_whole() {
        let selected =
            select_initial_markdown("# Intro\nComplete body", "summarize", DEFAULT_MAX_BYTES);
        assert!(selected.whole_document);
        assert!(selected.excerpt.contains("Complete body"));
    }

    #[test]
    fn selects_question_relevant_sections_from_a_large_document() {
        let large = [
            "# Handbook".to_owned(),
            String::new(),
            "## Background".into(),
            "x".repeat(20_000),
            String::new(),
            "## Authentication".into(),
            "Use short-lived access tokens and rotate refresh tokens.".into(),
            String::new(),
            "## Appendix".into(),
            "y".repeat(20_000),
        ]
        .join("\n");
        let selected =
            select_initial_markdown(&large, "How does authentication work?", DEFAULT_MAX_BYTES);
        assert!(!selected.whole_document);
        assert!(selected.excerpt.contains("## Authentication"));
        assert!(selected.excerpt.contains("short-lived access tokens"));
        assert!(!selected.excerpt.contains(&"x".repeat(20_000)));
        assert!(selected.bytes <= 16_384);
        assert!(selected.excerpt.starts_with("## Document map"));
    }

    #[test]
    fn extracts_local_markdown_links_without_resolving_targets() {
        let selected = select_initial_markdown(
            "[API guide](./api.md) and [Setup][setup].\n\n[setup]: ../setup.md",
            "summarize",
            DEFAULT_MAX_BYTES,
        );
        assert_eq!(
            selected.links,
            vec![
                MarkdownLink {
                    label: "API guide".into(),
                    target: "./api.md".into()
                },
                MarkdownLink {
                    label: "Setup".into(),
                    target: "../setup.md".into()
                },
            ]
        );
    }

    #[test]
    fn excludes_remote_mail_image_and_in_document_links() {
        let selected = select_initial_markdown(
            "[Web](https://example.com) [Mail](mailto:test@example.com) [Heading](#intro) ![img](a.png)",
            "summarize",
            DEFAULT_MAX_BYTES,
        );
        assert!(selected.links.is_empty());
        assert_eq!(
            extract_links("[a](one.md \"Title\")[b](two.md)"),
            vec![
                MarkdownLink {
                    label: "a".into(),
                    target: "one.md".into()
                },
                MarkdownLink {
                    label: "b".into(),
                    target: "two.md".into()
                },
            ]
        );
    }

    #[test]
    fn headings_strip_closing_hashes() {
        assert_eq!(parse_heading("## Title ##"), Some((2, "Title".into())));
        assert_eq!(parse_heading("#NoSpace"), None);
        assert_eq!(parse_heading("####### Seven"), None);
    }

    #[test]
    fn ledger_marks_unchanged_content_and_keeps_paths_apart() {
        let mut ledger = ContextLedger::default();
        let a = Path::new("/docs/a.md");
        assert!(!ledger.record(a, "one").already_sent);
        assert!(ledger.record(a, "one").already_sent);
        assert!(!ledger.record(a, "two").already_sent);
        ledger.clear();
        assert!(!ledger.record(a, "two").already_sent);

        let mut ledger = ContextLedger::default();
        let first = ledger.record(a, "same");
        let second = ledger.record(Path::new("/docs/b.md"), "same");
        assert_eq!(first.hash, second.hash);
        assert!(!second.already_sent);
        assert!(ledger.has(a, &first.hash));
        assert_eq!(
            hash_content("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn prioritizes_the_active_file_then_tags_and_assigns_source_ids() {
        let dir = tempfile::tempdir().unwrap();
        let active = write(dir.path(), "active.md", "# Active\nactive body");
        let tagged = write(dir.path(), "tagged.md", "# Tagged\ntagged body");
        write(dir.path(), "other.md", "# Other\nother body");
        let packet = build_context(
            BuildContextInput {
                active_path: Some(&active),
                tags: &[ContextTag::file(&tagged)],
                question: "",
            },
            &mut ContextLedger::default(),
        );
        assert_eq!(packet.sources[0].path, active);
        assert!(packet.sources.iter().any(|source| source.path == tagged));
        assert!(
            packet
                .sources
                .iter()
                .all(|source| source.source_id.starts_with("src:"))
        );
        assert!(packet.summary.to_lowercase().contains("using"));
        assert_eq!(packet.trace.focused_count, 1);
        assert_eq!(packet.trace.attached_count, 1);
    }

    #[test]
    fn does_not_inject_unrelated_files_or_expand_folder_tags() {
        let dir = tempfile::tempdir().unwrap();
        let active = write(dir.path(), "active.md", "# Active\nactive body");
        let tagged = write(dir.path(), "tagged.md", "# Tagged");
        write(dir.path(), "unrelated.md", "# Unrelated\nshould stay out");
        let packet = build_context(
            BuildContextInput {
                active_path: Some(&active),
                tags: &[ContextTag::file(&tagged), ContextTag::folder(dir.path())],
                question: "Compare the focused and attached file",
            },
            &mut ContextLedger::default(),
        );
        assert_eq!(
            packet
                .sources
                .iter()
                .map(|s| s.path.clone())
                .collect::<Vec<_>>(),
            vec![active, tagged]
        );
        assert_eq!(packet.trace.searched_count, 0);
        assert_eq!(packet.trace.read_range_count, 0);
    }

    #[test]
    fn keeps_the_active_file_when_tags_consume_the_budget() {
        let dir = tempfile::tempdir().unwrap();
        let active = write(
            dir.path(),
            "active.md",
            "# Focused document\nmust be included",
        );
        let tags = (0..6)
            .map(|index| {
                ContextTag::file(write(
                    dir.path(),
                    &format!("tag-{index}.md"),
                    &"x".repeat(24_000),
                ))
            })
            .collect::<Vec<_>>();
        let packet = build_context(
            BuildContextInput {
                active_path: Some(&active),
                tags: &tags,
                question: "",
            },
            &mut ContextLedger::default(),
        );
        assert_eq!(packet.sources[0].path, active);
        assert!(packet.sources[0].excerpt.contains("must be included"));
        assert!(packet.trace.injected_bytes <= MAX_INITIAL_TOTAL_BYTES);
    }

    #[test]
    fn selects_a_relevant_section_instead_of_truncating_from_the_top() {
        let dir = tempfile::tempdir().unwrap();
        let active = write(
            dir.path(),
            "handbook.md",
            &format!(
                "# Handbook\n\n## Background\n{}\n\n## Authentication\nRotate tokens daily.",
                "x".repeat(20_000)
            ),
        );
        let packet = build_context(
            BuildContextInput {
                active_path: Some(&active),
                tags: &[],
                question: "How does authentication work?",
            },
            &mut ContextLedger::default(),
        );
        assert!(packet.sources[0].excerpt.contains("## Authentication"));
        assert!(packet.sources[0].excerpt.contains("Rotate tokens daily"));
        assert!(packet.sources[0].bytes <= 16_384);
    }

    #[test]
    fn sends_identity_metadata_for_unchanged_documents() {
        let dir = tempfile::tempdir().unwrap();
        let active = write(
            dir.path(),
            "active.md",
            &format!(
                "# Active\nprivate body that should not be repeated\n{}",
                "detail ".repeat(400)
            ),
        );
        let mut ledger = ContextLedger::default();
        let input = || BuildContextInput {
            active_path: Some(&active),
            tags: &[],
            question: "Summarize this",
        };
        let first = build_context(input(), &mut ledger);
        let second = build_context(input(), &mut ledger);
        assert!(first.sources[0].excerpt.contains("private body"));
        assert!(
            second.sources[0]
                .excerpt
                .contains("Content unchanged from earlier in this session")
        );
        assert!(!second.sources[0].excerpt.contains("private body"));
        assert!(second.trace.injected_bytes < first.trace.injected_bytes);
    }

    #[test]
    fn appends_bounded_retrieved_ranges_with_an_honest_trace() {
        let dir = tempfile::tempdir().unwrap();
        let active = write(dir.path(), "active.md", "# Active\nFocused content");
        let related = dir.path().join("related.md");
        let packet = build_context(
            BuildContextInput {
                active_path: Some(&active),
                tags: &[],
                question: "Compare related.md",
            },
            &mut ContextLedger::default(),
        );
        let expanded = append_retrieved_context(
            packet,
            &[RetrievedRange {
                path: related.clone(),
                excerpt: "## Related\nRetrieved content".into(),
                start_line: 8,
                end_line: 9,
                bytes: 28,
                score: 10,
            }],
            RetrievalMode::AdaptiveLocal,
        );
        assert_eq!(expanded.sources[1].path, related);
        assert_eq!(expanded.sources[1].heading_id.as_deref(), Some("L8-L9"));
        assert_eq!(
            expanded.sources[1].source_id,
            format!("src:{}#L8-L9", related.display())
        );
        assert_eq!(expanded.trace.searched_count, 1);
        assert_eq!(expanded.trace.read_range_count, 1);
        assert_eq!(expanded.trace.retrieval_mode, RetrievalMode::AdaptiveLocal);
        assert_eq!(
            expanded.trace.items.last().unwrap().reason,
            TraceReason::Retrieved
        );
        assert_eq!(expanded.summary, "1 focused · searched 1 · read 1");
    }

    #[test]
    fn skips_non_markdown_paths_with_a_warning() {
        let dir = tempfile::tempdir().unwrap();
        let html = write(dir.path(), "page.html", "<p>hi</p>");
        let packet = build_context(
            BuildContextInput {
                active_path: Some(&html),
                tags: &[],
                question: "",
            },
            &mut ContextLedger::default(),
        );
        assert!(packet.sources.is_empty());
        assert!(
            packet
                .warnings
                .iter()
                .any(|w| w.to_lowercase().contains("skipped"))
        );
        assert_eq!(packet.summary, "No docs in context");
    }

    #[test]
    fn formats_a_read_only_prompt_with_source_ids() {
        let packet = ContextPacket {
            sources: vec![ContextSource {
                source_id: "src:/docs/a.md".into(),
                path: "/docs/a.md".into(),
                heading_id: None,
                excerpt: "hello".into(),
                bytes: 5,
            }],
            warnings: Vec::new(),
            summary: "Using a.md".into(),
            trace: ContextTrace {
                focused_count: 1,
                attached_count: 0,
                searched_count: 0,
                read_range_count: 0,
                injected_bytes: 5,
                estimated_tokens: 2,
                retrieval_mode: RetrievalMode::FocusedOnly,
                items: Vec::new(),
            },
        };
        let prompt = format_context_prompt(&packet, "Summarize this");
        assert!(prompt.contains("src:/docs/a.md"));
        assert!(prompt.contains("Summarize this"));
        assert!(prompt.contains("read-only"));
    }

    #[test]
    fn retrieval_only_triggers_for_cross_document_questions() {
        let active = Path::new("/docs/current.md");
        let folder = [ContextTag::folder("/docs")];
        assert!(!should_retrieve("Summarize this", Some(active), &[]));
        assert!(!should_retrieve("Summarize current.md", Some(active), &[]));
        assert!(should_retrieve(
            "Compare this with architecture.md",
            Some(active),
            &[]
        ));
        assert!(should_retrieve(
            "What do the docs in this folder say about caching?",
            Some(active),
            &folder
        ));
        assert!(!should_retrieve("What do the docs say?", Some(active), &[]));
        assert!(should_retrieve("Find related notes", Some(active), &[]));
    }

    #[test]
    fn retrieves_at_most_three_bounded_ranges() {
        let dir = tempfile::tempdir().unwrap();
        let active = write(dir.path(), "current.md", "# Current\nNo answer here");
        let cache = write(
            dir.path(),
            "notes/cache.md",
            &format!(
                "# Cache\n\n## Invalidation\nCache entries are invalidated by version keys.\n{}",
                "detail ".repeat(900)
            ),
        );
        write(
            dir.path(),
            "notes/other.md",
            "# Other\nUnrelated release notes",
        );
        write(dir.path(), "notes/page.html", "<p>cache invalidation</p>");
        let ranges = retrieve_markdown_ranges(
            "How is caching invalidated?",
            &[dir.path().to_owned()],
            &[active],
        );
        assert!(ranges.len() <= 3);
        assert!(ranges.iter().all(|range| range.bytes <= 4_096));
        assert_eq!(std::fs::canonicalize(&ranges[0].path).unwrap(), cache);
        assert!(ranges[0].excerpt.contains("invalidated by version keys"));
        assert_eq!(stem("caching"), "cach");
        assert_eq!(stem("invalidations"), "invalid");
    }

    #[test]
    fn citation_split_across_deltas_is_extracted_without_exposing_the_path() {
        let mut stream = CitationStream::new(["src:/docs/overview.md".to_owned()]);
        let first = stream.consume("The launch date is October 14 (src:/docs/over");
        let second = stream.consume("view.md).");
        let last = stream.flush();
        assert_eq!(
            first,
            CitationChunk {
                text: "The launch date is October 14 ".into(),
                citation_ids: vec![]
            }
        );
        assert_eq!(
            second,
            CitationChunk {
                text: ".".into(),
                citation_ids: vec!["src:/docs/overview.md".into()]
            }
        );
        assert_eq!(last, CitationChunk::default());
    }

    #[test]
    fn repeated_citations_are_deduplicated() {
        let mut stream = CitationStream::new(["src:/docs/overview.md".to_owned()]);
        let result = stream
            .consume("See src:/docs/overview.md and src:/docs/overview.md for launch details.");
        assert_eq!(result.text, "See  and  for launch details.");
        assert_eq!(
            result.citation_ids,
            vec!["src:/docs/overview.md".to_string()]
        );
        let mut code = CitationStream::new(["src:/a.md".to_owned()]);
        let mut text = code.consume("fn main() {} is covered (src:/a.md).").text;
        text.push_str(&code.flush().text);
        assert_eq!(text, "fn main() {} is covered.");
        let mut late = CitationStream::new(["src:/a.md".to_owned()]);
        assert_eq!(late.consume("Done [src:/a.md").text, "Done ");
        assert_eq!(late.consume("] now").text, " now");
        let mut plain = CitationStream::new(Vec::new());
        assert_eq!(plain.consume("tail (").text, "tail ");
        assert_eq!(plain.flush().text, "(");
    }
}
