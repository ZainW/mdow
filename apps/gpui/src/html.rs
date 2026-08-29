//! Converts HTML documents into the reader's `DocumentBlock` model.
//!
//! Sanitizer contract: `script`, `style`, `iframe`, `object`, and `embed` never reach the
//! output, and no attribute other than `alt`, `src`, `href`, `class`, and `start` is ever
//! read, so `on*` handlers are dropped by construction. Relative `src`/`href` targets are
//! rewritten against the document's parent directory.

use std::path::Path;

use crate::document::{
    DocumentBlock, InlineSpan, ListKind, TableBlock, has_uri_scheme, normalize_lexically,
};

pub fn html_to_blocks(source: &str, document_parent: &Path) -> Vec<DocumentBlock> {
    let nodes = parse_nodes(source);
    let converter = Converter { document_parent };
    let mut blocks = Vec::new();
    converter.collect_blocks(&nodes, 0, &mut blocks);
    blocks
}

#[derive(Debug)]
enum Node {
    Element {
        name: String,
        attrs: Vec<(String, String)>,
        children: Vec<Node>,
    },
    Text(String),
}

/// Elements whose entire subtree is discarded. `script`/`style` are also raw-text elements,
/// so their contents never tokenize as markup.
const STRIPPED_ELEMENTS: &[&str] = &[
    "script", "style", "iframe", "object", "embed", "head", "noscript", "template", "title",
    "meta", "link", "base",
];

const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

const INLINE_ELEMENTS: &[&str] = &[
    "a", "abbr", "b", "br", "cite", "code", "del", "em", "i", "kbd", "mark", "q", "s", "samp",
    "small", "span", "strike", "strong", "sub", "sup", "time", "u", "var",
];

// ---------------------------------------------------------------------------
// Lenient tree construction
// ---------------------------------------------------------------------------

struct OpenElement {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
}

fn parse_nodes(source: &str) -> Vec<Node> {
    let mut root = Vec::new();
    let mut stack = Vec::<OpenElement>::new();
    let mut rest = source;

    while !rest.is_empty() {
        let Some(open_at) = rest.find('<') else {
            append_text(rest, &mut root, &mut stack);
            break;
        };
        let (text, tail) = rest.split_at(open_at);
        append_text(text, &mut root, &mut stack);
        rest = tail;

        if rest.starts_with("<!--") {
            rest = rest
                .find("-->")
                .map_or("", |comment_end| &rest[comment_end + 3..]);
        } else if rest.starts_with("<!") || rest.starts_with("<?") {
            rest = rest.find('>').map_or("", |tag_end| &rest[tag_end + 1..]);
        } else if let Some(close) = rest.strip_prefix("</") {
            let Some(tag_end) = close.find('>') else {
                break;
            };
            let name = close[..tag_end]
                .trim()
                .trim_end_matches('/')
                .to_ascii_lowercase();
            close_element(&name, &mut root, &mut stack);
            rest = &close[tag_end + 1..];
        } else if let Some((name, attrs, self_closing, consumed)) = scan_open_tag(rest) {
            rest = &rest[consumed..];
            apply_implied_closes(&name, &mut root, &mut stack);
            if is_raw_text_element(&name) {
                let close_tag = format!("</{name}");
                let content_end = find_case_insensitive(rest, &close_tag).unwrap_or(rest.len());
                rest = &rest[content_end..];
                rest = rest.find('>').map_or("", |tag_end| &rest[tag_end + 1..]);
                attach(
                    Node::Element {
                        name,
                        attrs,
                        children: Vec::new(),
                    },
                    &mut root,
                    &mut stack,
                );
            } else if self_closing || VOID_ELEMENTS.contains(&name.as_str()) {
                attach(
                    Node::Element {
                        name,
                        attrs,
                        children: Vec::new(),
                    },
                    &mut root,
                    &mut stack,
                );
            } else {
                stack.push(OpenElement {
                    name,
                    attrs,
                    children: Vec::new(),
                });
            }
        } else {
            // Not actually a tag; keep the '<' as text.
            append_text("<", &mut root, &mut stack);
            rest = &rest[1..];
        }
    }

    while let Some(open) = stack.pop() {
        attach(
            Node::Element {
                name: open.name,
                attrs: open.attrs,
                children: open.children,
            },
            &mut root,
            &mut stack,
        );
    }
    root
}

fn is_raw_text_element(name: &str) -> bool {
    matches!(name, "script" | "style")
}

fn find_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .to_ascii_lowercase()
        .find(&needle.to_ascii_lowercase())
}

fn append_text(text: &str, root: &mut Vec<Node>, stack: &mut [OpenElement]) {
    if text.is_empty() {
        return;
    }
    let node = Node::Text(decode_entities(text));
    if let Some(open) = stack.last_mut() {
        open.children.push(node);
    } else {
        root.push(node);
    }
}

fn attach(node: Node, root: &mut Vec<Node>, stack: &mut [OpenElement]) {
    if let Some(open) = stack.last_mut() {
        open.children.push(node);
    } else {
        root.push(node);
    }
}

fn close_element(name: &str, root: &mut Vec<Node>, stack: &mut Vec<OpenElement>) {
    let Some(position) = stack.iter().rposition(|open| open.name == name) else {
        return;
    };
    while stack.len() > position {
        let open = stack.pop().expect("stack has at least `position` entries");
        attach(
            Node::Element {
                name: open.name,
                attrs: open.attrs,
                children: open.children,
            },
            root,
            stack,
        );
    }
}

/// A subset of the HTML5 implied-end-tag rules, enough for typical hand-written documents.
fn apply_implied_closes(name: &str, root: &mut Vec<Node>, stack: &mut Vec<OpenElement>) {
    let closes_top: &[&str] = match name {
        "li" => &["li"],
        "td" | "th" => &["td", "th"],
        "tr" => &["td", "th", "tr"],
        "p" | "div" | "ul" | "ol" | "table" | "blockquote" | "pre" | "hr" | "section"
        | "article" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => &["p"],
        _ => &[],
    };
    while stack
        .last()
        .is_some_and(|open| closes_top.contains(&open.name.as_str()))
    {
        let top = stack.last().expect("just checked").name.clone();
        close_element(&top, root, stack);
    }
}

type OpenTagScan = (String, Vec<(String, String)>, bool, usize);

fn scan_open_tag(rest: &str) -> Option<OpenTagScan> {
    let bytes = rest.as_bytes();
    debug_assert_eq!(bytes.first(), Some(&b'<'));
    let mut index = 1;
    let name_start = index;
    while index < bytes.len()
        && (bytes[index].is_ascii_alphanumeric() || matches!(bytes[index], b'-' | b':'))
    {
        index += 1;
    }
    if index == name_start || !bytes[name_start].is_ascii_alphabetic() {
        return None;
    }
    let name = rest[name_start..index].to_ascii_lowercase();

    let mut attrs = Vec::new();
    let mut self_closing = false;
    loop {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index >= bytes.len() {
            return None;
        }
        match bytes[index] {
            b'>' => {
                index += 1;
                break;
            }
            b'/' => {
                if bytes.get(index + 1) == Some(&b'>') {
                    self_closing = true;
                    index += 2;
                    break;
                }
                index += 1;
            }
            _ => {
                let attr_start = index;
                while index < bytes.len()
                    && !bytes[index].is_ascii_whitespace()
                    && !matches!(bytes[index], b'=' | b'>' | b'/')
                {
                    index += 1;
                }
                let attr_name = rest[attr_start..index].to_ascii_lowercase();
                while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                    index += 1;
                }
                let mut value = String::new();
                if bytes.get(index) == Some(&b'=') {
                    index += 1;
                    while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                        index += 1;
                    }
                    match bytes.get(index) {
                        Some(&quote @ (b'"' | b'\'')) => {
                            index += 1;
                            let value_start = index;
                            while index < bytes.len() && bytes[index] != quote {
                                index += 1;
                            }
                            value = decode_entities(&rest[value_start..index]);
                            index = (index + 1).min(bytes.len());
                        }
                        _ => {
                            let value_start = index;
                            while index < bytes.len()
                                && !bytes[index].is_ascii_whitespace()
                                && bytes[index] != b'>'
                            {
                                index += 1;
                            }
                            value = decode_entities(&rest[value_start..index]);
                        }
                    }
                }
                if !attr_name.is_empty() {
                    attrs.push((attr_name, value));
                }
            }
        }
    }
    Some((name, attrs, self_closing, index))
}

fn decode_entities(text: &str) -> String {
    if !text.contains('&') {
        return text.to_owned();
    }
    let mut decoded = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        decoded.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let semicolon = rest[..rest.len().min(32)].find(';');
        let Some(semicolon) = semicolon else {
            decoded.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..semicolon];
        let replacement = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            _ => entity
                .strip_prefix('#')
                .and_then(|digits| {
                    digits.strip_prefix(['x', 'X']).map_or_else(
                        || digits.parse::<u32>().ok(),
                        |hex| u32::from_str_radix(hex, 16).ok(),
                    )
                })
                .and_then(char::from_u32),
        };
        if let Some(character) = replacement {
            decoded.push(character);
            rest = &rest[semicolon + 1..];
        } else {
            decoded.push('&');
            rest = &rest[1..];
        }
    }
    decoded.push_str(rest);
    decoded
}

// ---------------------------------------------------------------------------
// Block conversion
// ---------------------------------------------------------------------------

struct Converter<'a> {
    document_parent: &'a Path,
}

impl Converter<'_> {
    fn collect_blocks(&self, nodes: &[Node], list_depth: usize, out: &mut Vec<DocumentBlock>) {
        let mut pending = Vec::<InlineSpan>::new();
        for node in nodes {
            let Node::Element {
                name,
                attrs,
                children,
            } = node
            else {
                if let Node::Text(text) = node {
                    self.append_collapsed_text(text, &mut pending);
                }
                continue;
            };
            if STRIPPED_ELEMENTS.contains(&name.as_str()) {
                continue;
            }
            if INLINE_ELEMENTS.contains(&name.as_str()) {
                self.append_inline_node(node, &mut pending);
                continue;
            }
            match name.as_str() {
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    flush_paragraph(&mut pending, out);
                    let level = name.as_bytes()[1] - b'0';
                    out.push(DocumentBlock::Heading {
                        level,
                        content: self.inline_spans(children),
                    });
                }
                "p" => {
                    flush_paragraph(&mut pending, out);
                    if let Some(image) = self.standalone_image(children) {
                        out.push(image);
                    } else {
                        let mut content = self.inline_spans(children);
                        trim_inline_edges(&mut content);
                        if !content.is_empty() {
                            out.push(DocumentBlock::Paragraph(content));
                        }
                    }
                }
                "hr" => {
                    flush_paragraph(&mut pending, out);
                    out.push(DocumentBlock::ThematicBreak);
                }
                "pre" => {
                    flush_paragraph(&mut pending, out);
                    out.push(self.code_block(children));
                }
                "ul" => {
                    flush_paragraph(&mut pending, out);
                    self.push_list_items(children, None, list_depth, out);
                }
                "ol" => {
                    flush_paragraph(&mut pending, out);
                    let start = attr(attrs, "start")
                        .and_then(|value| value.trim().parse::<u64>().ok())
                        .unwrap_or(1);
                    self.push_list_items(children, Some(start), list_depth, out);
                }
                "li" => {
                    flush_paragraph(&mut pending, out);
                    out.push(self.list_item(children, ListKind::Unordered, list_depth));
                }
                "blockquote" => {
                    flush_paragraph(&mut pending, out);
                    out.push(DocumentBlock::Blockquote(self.quote_spans(children)));
                }
                "table" => {
                    flush_paragraph(&mut pending, out);
                    out.push(DocumentBlock::Table(self.table_block(children)));
                }
                "img" => {
                    flush_paragraph(&mut pending, out);
                    out.push(DocumentBlock::Image {
                        alt: attr(attrs, "alt").unwrap_or_default().to_owned(),
                        source: self.rewrite_target(attr(attrs, "src").unwrap_or_default()),
                    });
                }
                // Structural containers and unknown tags are transparent: their text
                // survives, their markup does not.
                _ => {
                    flush_paragraph(&mut pending, out);
                    self.collect_blocks(children, list_depth, out);
                }
            }
        }
        flush_paragraph(&mut pending, out);
    }

    fn push_list_items(
        &self,
        children: &[Node],
        ordered_start: Option<u64>,
        depth: usize,
        out: &mut Vec<DocumentBlock>,
    ) {
        let mut number = ordered_start.unwrap_or(1);
        for child in children {
            let Node::Element { name, children, .. } = child else {
                continue;
            };
            match name.as_str() {
                "li" => {
                    let kind = match ordered_start {
                        Some(_) => {
                            let item_number = number;
                            number += 1;
                            ListKind::Ordered {
                                number: item_number,
                            }
                        }
                        None => ListKind::Unordered,
                    };
                    out.push(self.list_item(children, kind, depth));
                }
                // Malformed nesting: a list directly inside a list.
                "ul" => self.push_list_items(children, None, depth + 1, out),
                "ol" => self.push_list_items(children, Some(1), depth + 1, out),
                _ => {}
            }
        }
    }

    fn list_item(&self, children: &[Node], kind: ListKind, depth: usize) -> DocumentBlock {
        let mut item_children = Vec::new();
        self.collect_blocks(children, depth + 1, &mut item_children);
        DocumentBlock::ListItem {
            kind,
            depth,
            children: item_children,
        }
    }

    fn code_block(&self, children: &[Node]) -> DocumentBlock {
        let language = find_code_language(children);
        let code = raw_text(children);
        let code = code.strip_prefix('\n').unwrap_or(&code).to_owned();
        if language
            .as_deref()
            .is_some_and(|language| language.eq_ignore_ascii_case("mermaid"))
        {
            DocumentBlock::MermaidCard { source: code }
        } else {
            DocumentBlock::CodeBlock { language, code }
        }
    }

    /// Untyped Markdown quotes flatten to inline text; HTML quotes match that shape.
    fn quote_spans(&self, children: &[Node]) -> Vec<InlineSpan> {
        let mut inner = Vec::new();
        self.collect_blocks(children, 0, &mut inner);
        let mut spans = Vec::new();
        for block in inner {
            let content = match block {
                DocumentBlock::Paragraph(content) => content,
                other => {
                    let text = other.plain_text();
                    if text.is_empty() {
                        continue;
                    }
                    vec![InlineSpan::Text(text)]
                }
            };
            if !spans.is_empty() {
                spans.push(InlineSpan::SoftBreak);
            }
            spans.extend(content);
        }
        spans
    }

    fn table_block(&self, children: &[Node]) -> TableBlock {
        let mut headers = Vec::new();
        let mut rows = Vec::new();
        for (row, in_head) in table_rows(children) {
            let Node::Element { children, .. } = row else {
                continue;
            };
            let mut cells = Vec::new();
            let mut all_header_cells = true;
            for cell in children {
                let Node::Element { name, children, .. } = cell else {
                    continue;
                };
                match name.as_str() {
                    "th" => cells.push(self.inline_spans(children)),
                    "td" => {
                        all_header_cells = false;
                        cells.push(self.inline_spans(children));
                    }
                    _ => {}
                }
            }
            if cells.is_empty() {
                continue;
            }
            if headers.is_empty() && rows.is_empty() && (in_head || all_header_cells) {
                headers = cells;
            } else {
                rows.push(cells);
            }
        }
        TableBlock { headers, rows }
    }

    fn standalone_image(&self, children: &[Node]) -> Option<DocumentBlock> {
        let mut image = None;
        for child in children {
            match child {
                Node::Text(text) if text.trim().is_empty() => {}
                Node::Element { name, attrs, .. } if name == "img" && image.is_none() => {
                    image = Some(DocumentBlock::Image {
                        alt: attr(attrs, "alt").unwrap_or_default().to_owned(),
                        source: self.rewrite_target(attr(attrs, "src").unwrap_or_default()),
                    });
                }
                _ => return None,
            }
        }
        image
    }

    // -- inline conversion --

    fn inline_spans(&self, nodes: &[Node]) -> Vec<InlineSpan> {
        let mut spans = Vec::new();
        for node in nodes {
            self.append_inline_node(node, &mut spans);
        }
        trim_inline_edges(&mut spans);
        spans
    }

    fn append_inline_node(&self, node: &Node, spans: &mut Vec<InlineSpan>) {
        let Node::Element {
            name,
            attrs,
            children,
        } = node
        else {
            if let Node::Text(text) = node {
                self.append_collapsed_text(text, spans);
            }
            return;
        };
        if STRIPPED_ELEMENTS.contains(&name.as_str()) {
            return;
        }
        match name.as_str() {
            "strong" | "b" => spans.push(InlineSpan::Strong(self.inline_spans(children))),
            "em" | "i" => spans.push(InlineSpan::Emphasis(self.inline_spans(children))),
            "s" | "del" | "strike" => {
                spans.push(InlineSpan::Strikethrough(self.inline_spans(children)))
            }
            "code" | "kbd" | "samp" => spans.push(InlineSpan::Code(collapse_whitespace(
                &raw_text(children),
                false,
            ))),
            "a" => match attr(attrs, "href").filter(|href| !href.trim().is_empty()) {
                Some(href) => spans.push(InlineSpan::Link {
                    label: self.inline_spans(children),
                    target: self.rewrite_target(href),
                }),
                None => {
                    for child in children {
                        self.append_inline_node(child, spans);
                    }
                }
            },
            "br" => spans.push(InlineSpan::HardBreak),
            "img" => {
                let alt = attr(attrs, "alt").unwrap_or_default();
                if !alt.is_empty() {
                    spans.push(InlineSpan::Text(alt.to_owned()));
                }
            }
            _ => {
                for child in children {
                    self.append_inline_node(child, spans);
                }
            }
        }
    }

    fn append_collapsed_text(&self, text: &str, spans: &mut Vec<InlineSpan>) {
        let at_start = spans.is_empty() || matches!(spans.last(), Some(InlineSpan::HardBreak));
        let collapsed = collapse_whitespace(text, at_start);
        if collapsed.is_empty() {
            return;
        }
        if let Some(InlineSpan::Text(existing)) = spans.last_mut() {
            existing.push_str(&collapsed);
        } else {
            spans.push(InlineSpan::Text(collapsed));
        }
    }

    fn rewrite_target(&self, target: &str) -> String {
        let target = target.trim();
        if target.is_empty()
            || target.starts_with('#')
            || target.starts_with('/')
            || has_uri_scheme(target)
        {
            return target.to_owned();
        }
        normalize_lexically(&self.document_parent.join(target))
            .to_string_lossy()
            .into_owned()
    }
}

fn attr<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(attr_name, _)| attr_name == name)
        .map(|(_, value)| value.as_str())
}

fn flush_paragraph(pending: &mut Vec<InlineSpan>, out: &mut Vec<DocumentBlock>) {
    let mut content = std::mem::take(pending);
    trim_inline_edges(&mut content);
    if !content.is_empty() {
        out.push(DocumentBlock::Paragraph(content));
    }
}

fn trim_inline_edges(spans: &mut Vec<InlineSpan>) {
    if let Some(InlineSpan::Text(text)) = spans.first_mut() {
        let trimmed = text.trim_start().to_owned();
        *text = trimmed;
    }
    if let Some(InlineSpan::Text(text)) = spans.last_mut() {
        let trimmed = text.trim_end().to_owned();
        *text = trimmed;
    }
    spans.retain(|span| !matches!(span, InlineSpan::Text(text) if text.is_empty()));
}

fn collapse_whitespace(text: &str, trim_start: bool) -> String {
    let mut collapsed = String::with_capacity(text.len());
    let mut in_whitespace = trim_start;
    for character in text.chars() {
        if character.is_whitespace() {
            if !in_whitespace {
                collapsed.push(' ');
            }
            in_whitespace = true;
        } else {
            collapsed.push(character);
            in_whitespace = false;
        }
    }
    collapsed
}

/// Text content with `<br>` as newlines and whitespace preserved, for `<pre>` bodies.
fn raw_text(nodes: &[Node]) -> String {
    let mut text = String::new();
    for node in nodes {
        match node {
            Node::Text(content) => text.push_str(content),
            Node::Element { name, children, .. } => {
                if name == "br" {
                    text.push('\n');
                } else if !STRIPPED_ELEMENTS.contains(&name.as_str()) {
                    text.push_str(&raw_text(children));
                }
            }
        }
    }
    text
}

fn find_code_language(nodes: &[Node]) -> Option<String> {
    for node in nodes {
        let Node::Element {
            name,
            attrs,
            children,
        } = node
        else {
            continue;
        };
        if name == "code"
            && let Some(class) = attr(attrs, "class")
        {
            for class_name in class.split_ascii_whitespace() {
                if let Some(language) = class_name
                    .strip_prefix("language-")
                    .or_else(|| class_name.strip_prefix("lang-"))
                    && !language.is_empty()
                {
                    return Some(language.to_owned());
                }
            }
        }
        if let Some(language) = find_code_language(children) {
            return Some(language);
        }
    }
    None
}

fn table_rows(children: &[Node]) -> Vec<(&Node, bool)> {
    let mut rows = Vec::new();
    for child in children {
        let Node::Element { name, children, .. } = child else {
            continue;
        };
        match name.as_str() {
            "tr" => rows.push((child, false)),
            "thead" => rows.extend(direct_rows(children).into_iter().map(|row| (row, true))),
            "tbody" | "tfoot" => {
                rows.extend(direct_rows(children).into_iter().map(|row| (row, false)))
            }
            _ => {}
        }
    }
    rows
}

fn direct_rows(children: &[Node]) -> Vec<&Node> {
    children
        .iter()
        .filter(|child| matches!(child, Node::Element { name, .. } if name == "tr"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn blocks(source: &str) -> Vec<DocumentBlock> {
        html_to_blocks(source, Path::new("/site/docs"))
    }

    #[test]
    fn strips_script_and_keeps_heading_and_paragraph() {
        let parsed = blocks(
            "<html><head><title>Skip</title><script>if (1 < 2) alert('x');</script></head>\
             <body><h1>Title</h1><p>Body text</p><iframe src=\"evil\"></iframe></body></html>",
        );

        assert_eq!(
            parsed,
            vec![
                DocumentBlock::Heading {
                    level: 1,
                    content: vec![InlineSpan::Text("Title".into())],
                },
                DocumentBlock::Paragraph(vec![InlineSpan::Text("Body text".into())]),
            ]
        );
    }

    #[test]
    fn event_handler_attributes_never_reach_the_model() {
        let parsed =
            blocks("<p onclick=\"steal()\">safe <a href=\"a.md\" onmouseover=\"x()\">link</a></p>");

        assert_eq!(
            parsed,
            vec![DocumentBlock::Paragraph(vec![
                InlineSpan::Text("safe ".into()),
                InlineSpan::Link {
                    label: vec![InlineSpan::Text("link".into())],
                    target: "/site/docs/a.md".into(),
                },
            ])]
        );
    }

    #[test]
    fn rewrites_relative_targets_and_leaves_absolute_ones() {
        let parsed = blocks(
            "<p><a href=\"../guide.md\">guide</a> <a href=\"https://mdow.dev\">web</a></p>\
             <img src=\"images/hero.png\" alt=\"Hero\">",
        );

        assert_eq!(
            parsed,
            vec![
                DocumentBlock::Paragraph(vec![
                    InlineSpan::Link {
                        label: vec![InlineSpan::Text("guide".into())],
                        target: "/site/guide.md".into(),
                    },
                    InlineSpan::Text(" ".into()),
                    InlineSpan::Link {
                        label: vec![InlineSpan::Text("web".into())],
                        target: "https://mdow.dev".into(),
                    },
                ]),
                DocumentBlock::Image {
                    alt: "Hero".into(),
                    source: "/site/docs/images/hero.png".into(),
                },
            ]
        );
    }

    #[test]
    fn converts_lists_code_quotes_tables_and_inline_styles() {
        let parsed = blocks(
            "<ol start=\"3\"><li>Third</li><li>Fourth<ul><li>Nested</li></ul></li></ol>\
             <pre><code class=\"language-rust\">let n = 1;\n</code></pre>\
             <blockquote><p>Quoted</p></blockquote>\
             <table><thead><tr><th>Name</th></tr></thead><tbody><tr><td>one</td></tr></tbody></table>\
             <p><strong>bold</strong> <em>italic</em> <s>gone</s> <code>x &amp; y</code><br>next</p><hr>",
        );

        assert_eq!(
            parsed,
            vec![
                DocumentBlock::ListItem {
                    kind: ListKind::Ordered { number: 3 },
                    depth: 0,
                    children: vec![DocumentBlock::Paragraph(vec![InlineSpan::Text(
                        "Third".into()
                    )])],
                },
                DocumentBlock::ListItem {
                    kind: ListKind::Ordered { number: 4 },
                    depth: 0,
                    children: vec![
                        DocumentBlock::Paragraph(vec![InlineSpan::Text("Fourth".into())]),
                        DocumentBlock::ListItem {
                            kind: ListKind::Unordered,
                            depth: 1,
                            children: vec![DocumentBlock::Paragraph(vec![InlineSpan::Text(
                                "Nested".into()
                            )])],
                        },
                    ],
                },
                DocumentBlock::CodeBlock {
                    language: Some("rust".into()),
                    code: "let n = 1;\n".into(),
                },
                DocumentBlock::Blockquote(vec![InlineSpan::Text("Quoted".into())]),
                DocumentBlock::Table(TableBlock {
                    headers: vec![vec![InlineSpan::Text("Name".into())]],
                    rows: vec![vec![vec![InlineSpan::Text("one".into())]]],
                }),
                DocumentBlock::Paragraph(vec![
                    InlineSpan::Strong(vec![InlineSpan::Text("bold".into())]),
                    InlineSpan::Text(" ".into()),
                    InlineSpan::Emphasis(vec![InlineSpan::Text("italic".into())]),
                    InlineSpan::Text(" ".into()),
                    InlineSpan::Strikethrough(vec![InlineSpan::Text("gone".into())]),
                    InlineSpan::Text(" ".into()),
                    InlineSpan::Code("x & y".into()),
                    InlineSpan::HardBreak,
                    InlineSpan::Text("next".into()),
                ]),
                DocumentBlock::ThematicBreak,
            ]
        );
    }

    #[test]
    fn mermaid_pre_blocks_become_mermaid_cards() {
        let parsed = blocks("<pre><code class=\"language-mermaid\">graph TD;</code></pre>");

        assert_eq!(
            parsed,
            vec![DocumentBlock::MermaidCard {
                source: "graph TD;".into(),
            }]
        );
    }

    #[test]
    fn unknown_tags_flatten_to_their_text() {
        let parsed = blocks("<custom-widget data-x=\"1\"><p>Inside</p>tail</custom-widget>");

        assert_eq!(
            parsed,
            vec![
                DocumentBlock::Paragraph(vec![InlineSpan::Text("Inside".into())]),
                DocumentBlock::Paragraph(vec![InlineSpan::Text("tail".into())]),
            ]
        );
    }

    #[test]
    fn html_to_blocks_uses_the_provided_parent_for_relative_paths() {
        let parsed = html_to_blocks(
            "<img src=\"a.png\" alt=\"a\">",
            &PathBuf::from("/somewhere/else"),
        );

        assert_eq!(
            parsed,
            vec![DocumentBlock::Image {
                alt: "a".into(),
                source: "/somewhere/else/a.png".into(),
            }]
        );
    }
}
