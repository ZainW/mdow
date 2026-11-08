//! A document as a standalone HTML page in Mdow's light look, for printing and PDF export
//! through WebKit. Diagrams and formulas go in as SVG, so they stay vector in the PDF.

use crate::doc::{Align, Block, Doc, ImageSrc, Inline, List, Table};
use crate::highlight;
use crate::theme::Palette;
use std::fmt::Write;
use std::path::{Path, PathBuf};

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn hex(c: rune::Color) -> String {
    c.hex()[..7].to_string()
}

struct Out<'a> {
    p: &'a Palette,
    html: String,
}

impl Out<'_> {
    fn inlines(&mut self, list: &[Inline]) {
        for i in list {
            match i {
                Inline::Text(t) => self.html.push_str(&esc(t)),
                Inline::Code(c) => {
                    let _ = write!(self.html, "<code>{}</code>", esc(c));
                }
                Inline::Strong(c) => self.wrap("strong", c),
                Inline::Emph(c) => self.wrap("em", c),
                Inline::Strike(c) => self.wrap("del", c),
                Inline::Underline(c) => self.wrap("u", c),
                Inline::Mark(c) => self.wrap("mark", c),
                Inline::Sup(c) => self.wrap("sup", c),
                Inline::Sub(c) => self.wrap("sub", c),
                Inline::Link { url, children } => {
                    let _ = write!(self.html, "<a href=\"{}\">", esc(url));
                    self.inlines(children);
                    self.html.push_str("</a>");
                }
                Inline::Image { src, alt } => {
                    let _ = write!(self.html, "<img src=\"{}\" alt=\"{}\">", esc(src), esc(alt));
                }
                Inline::Break => self.html.push_str("<br>"),
                Inline::Math(tex) => self.math(tex, false),
                Inline::FootnoteRef(label) => {
                    let _ = write!(
                        self.html,
                        "<sup class=\"fn\"><a href=\"#fn-{0}\">{0}</a></sup>",
                        esc(label)
                    );
                }
            }
        }
    }

    fn wrap(&mut self, tag: &str, children: &[Inline]) {
        let _ = write!(self.html, "<{tag}>");
        self.inlines(children);
        let _ = write!(self.html, "</{tag}>");
    }

    fn math(&mut self, tex: &str, display: bool) {
        let size = if display { 15.0 } else { 12.0 };
        match crate::media::math_svg(tex, &hex(self.p.text), size, display) {
            Ok((svg, depth)) if display => {
                let _ = write!(self.html, "<div class=\"math\">{svg}</div>");
                let _ = depth;
            }
            Ok((svg, depth)) => {
                let _ = write!(
                    self.html,
                    "<span class=\"imath\" style=\"vertical-align:-{depth:.2}pt\">{svg}</span>"
                );
            }
            Err(_) => {
                let _ = write!(self.html, "<code>{}</code>", esc(tex));
            }
        }
    }

    fn code(&mut self, lang: &str, code: &str) {
        let spans = highlight::highlight(lang, code);
        let mut at = 0usize;
        self.html.push_str("<pre><code>");
        for s in spans.iter() {
            let (start, end) = (s.start as usize, s.end as usize);
            if start > at {
                self.html.push_str(&esc(&code[at..start]));
            }
            match self.p.syntax(s.tok) {
                Some(c) => {
                    let _ = write!(
                        self.html,
                        "<span style=\"color:{}\">{}</span>",
                        hex(c),
                        esc(&code[start..end])
                    );
                }
                None => self.html.push_str(&esc(&code[start..end])),
            }
            at = end;
        }
        self.html.push_str(&esc(&code[at.min(code.len())..]));
        self.html.push_str("</code></pre>");
    }

    fn list(&mut self, list: &List) {
        let tag = if list.ordered { "ol" } else { "ul" };
        if list.ordered && list.start != 1 {
            let _ = write!(self.html, "<ol start=\"{}\">", list.start);
        } else {
            let _ = write!(self.html, "<{tag}>");
        }
        for item in &list.items {
            match item.task {
                Some(done) => {
                    let _ = write!(
                        self.html,
                        "<li class=\"task\"><span class=\"box\">{}</span>",
                        if done { "☑" } else { "☐" }
                    );
                }
                None => self.html.push_str("<li>"),
            }
            self.blocks(&item.blocks);
            self.html.push_str("</li>");
        }
        let _ = write!(self.html, "</{tag}>");
    }

    fn table(&mut self, t: &Table) {
        let align = |i: usize| match t.aligns.get(i) {
            Some(Align::Center) => " style=\"text-align:center\"",
            Some(Align::Right) => " style=\"text-align:right\"",
            _ => "",
        };
        self.html.push_str("<table>");
        if !t.head.is_empty() {
            self.html.push_str("<thead><tr>");
            for (i, cell) in t.head.iter().enumerate() {
                let _ = write!(self.html, "<th{}>", align(i));
                self.inlines(cell);
                self.html.push_str("</th>");
            }
            self.html.push_str("</tr></thead>");
        }
        self.html.push_str("<tbody>");
        for row in &t.rows {
            self.html.push_str("<tr>");
            for (i, cell) in row.iter().enumerate() {
                let _ = write!(self.html, "<td{}>", align(i));
                self.inlines(cell);
                self.html.push_str("</td>");
            }
            self.html.push_str("</tr>");
        }
        self.html.push_str("</tbody></table>");
    }

    fn blocks(&mut self, blocks: &[Block]) {
        let mut i = 0;
        while i < blocks.len() {
            match &blocks[i] {
                Block::Heading { level, inlines, slug } => {
                    let _ = write!(self.html, "<h{level} id=\"{}\">", esc(slug));
                    self.inlines(inlines);
                    let _ = write!(self.html, "</h{level}>");
                }
                Block::Paragraph(inlines) => {
                    self.html.push_str("<p>");
                    self.inlines(inlines);
                    self.html.push_str("</p>");
                }
                Block::Code(chunk) => {
                    // Long blocks are split into rows; print each block once, whole.
                    if matches!(chunk.part, crate::doc::Part::Whole | crate::doc::Part::First) {
                        self.code(&chunk.source.lang, &chunk.source.text);
                    }
                }
                Block::Mermaid(code) => {
                    let options = mermaid_rs_renderer::RenderOptions {
                        theme: crate::media::mermaid_theme(self.p),
                        layout: mermaid_rs_renderer::LayoutConfig::default(),
                    };
                    match mermaid_rs_renderer::render_with_options(code, options) {
                        Ok(svg) => {
                            let _ = write!(self.html, "<div class=\"diagram\">{svg}</div>");
                        }
                        Err(_) => self.code("mermaid", code),
                    }
                }
                Block::Math(tex) => self.math(tex, true),
                Block::Quote(c) => {
                    self.html.push_str("<blockquote>");
                    self.blocks(c);
                    self.html.push_str("</blockquote>");
                }
                Block::Alert(kind, c) => {
                    let (color, label) = self.p.alert(*kind);
                    let _ = write!(
                        self.html,
                        "<div class=\"alert\" style=\"border-color:{0}\"><div class=\"alert-title\" style=\"color:{0}\">{1}</div>",
                        hex(color),
                        label
                    );
                    self.blocks(c);
                    self.html.push_str("</div>");
                }
                Block::List(list) => self.list(list),
                Block::Table(t) => self.table(t),
                Block::Images(images) => {
                    self.html.push_str("<p class=\"images\">");
                    for image in images {
                        let src = match &image.src {
                            ImageSrc::File(p) => format!("file://{}", p.display()),
                            ImageSrc::Remote(u) => u.clone(),
                            ImageSrc::Missing => continue,
                        };
                        let _ = write!(
                            self.html,
                            "<img src=\"{}\" alt=\"{}\">",
                            esc(&src),
                            esc(&image.alt)
                        );
                    }
                    self.html.push_str("</p>");
                }
                Block::Rule => self.html.push_str("<hr>"),
                Block::Footnotes(notes) => {
                    self.html.push_str("<section class=\"footnotes\"><hr><ol>");
                    for n in notes {
                        let _ = write!(self.html, "<li id=\"fn-{}\">", esc(&n.label));
                        self.blocks(&n.blocks);
                        self.html.push_str("</li>");
                    }
                    self.html.push_str("</ol></section>");
                }
            }
            i += 1;
        }
    }
}

/// The page for `doc`, set in `body_family`.
pub fn html(doc: &Doc, title: &str, body_family: &str) -> String {
    let p = Palette::get(false);
    let mut out = Out { p: &p, html: String::new() };
    out.blocks(&doc.blocks);
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><title>{title}</title><style>
body {{ font: 11.5pt/1.55 {body}, Charter, Georgia, serif; color: {text}; background: #fff; margin: 0; }}
h1, h2, h3, h4, h5, h6 {{ line-height: 1.2; margin: 1.4em 0 .5em; break-after: avoid; }}
h1 {{ font-size: 2em }} h2 {{ font-size: 1.45em }} h3 {{ font-size: 1.2em }} h4 {{ font-size: 1.06em }} h5, h6 {{ font-size: 1em }}
h6 {{ color: {secondary} }}
p, ul, ol, blockquote, table, pre, .alert, .diagram, .math {{ margin: 0 0 .85em }}
a {{ color: {accent}; text-decoration: none }}
code {{ font: .86em ui-monospace, Menlo, monospace; background: #f1eee8; border-radius: 4px; padding: 0 .2em }}
pre {{ background: #f5f3ed; border: 1px solid #e4e0d8; border-radius: 8px; padding: 10px 12px; white-space: pre-wrap; break-inside: avoid }}
pre code {{ background: none; padding: 0; font-size: .82em }}
blockquote {{ border-left: 3px solid #d9d3c9; padding-left: 14px; margin-left: 0; color: {secondary} }}
.alert {{ border-left: 3px solid; padding: 8px 14px; background: #f6f5f2; border-radius: 6px; break-inside: avoid }}
.alert-title {{ font: 600 .75em ui-monospace, Menlo, monospace; letter-spacing: .05em; text-transform: uppercase; margin-bottom: 4px }}
table {{ border-collapse: collapse; width: 100%; font-size: .95em }}
th, td {{ padding: 6px 10px; border-bottom: .5px solid #e4e0d8; text-align: left }}
th {{ border-bottom: 1px solid #cfc8bc }}
li.task {{ list-style: none; margin-left: -1.2em }} .box {{ margin-right: .4em }} li.task > p:first-of-type {{ display: inline }}
img {{ max-width: 100% }} .images img {{ margin-right: 6px; vertical-align: bottom }}
.diagram, .math {{ text-align: center; break-inside: avoid }} .diagram svg {{ max-width: 100%; height: auto }}
.imath svg {{ display: inline }}
hr {{ border: 0; border-top: 1px solid #d9d3c9; margin: 1.6em 0 }}
.footnotes {{ font-size: .86em; color: {secondary} }} sup.fn a {{ font-size: .8em }}
mark {{ background: #ffe79a }}
</style></head><body>{body_html}</body></html>"#,
        title = esc(title),
        body = body_family,
        text = hex(p.text),
        secondary = hex(p.secondary),
        accent = hex(p.accent),
        body_html = out.html,
    )
}

/// Writes the page for `doc` to a temporary file and returns its path.
pub fn write_temp(doc: &Doc, source: &Path, body_family: &str) -> Option<PathBuf> {
    let title = doc.title.clone().unwrap_or_else(|| crate::files::display_name(source));
    let page = html(doc, &title, body_family);
    let dir = std::env::temp_dir().join("mdow-rune-print");
    std::fs::create_dir_all(&dir).ok()?;
    let path = dir.join(format!("{}.html", std::process::id()));
    std::fs::write(&path, page).ok()?;
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn documents_become_pages() {
        let doc = crate::doc::parse_markdown(
            "# Title\n\nSome **bold** and `code` and $x^2$.\n\n```rust\nfn main() {}\n```\n\n| a | b |\n|---|--:|\n| 1 | 2 |\n",
            Path::new("/tmp/a.md"),
        );
        let page = html(&doc, "Title", "Charter");
        assert!(page.contains("<h1 id=\"title\">Title</h1>"));
        assert!(page.contains("<strong>bold</strong>"));
        assert!(page.contains("<pre><code>"));
        assert!(page.contains("<span class=\"imath\""));
        assert!(page.contains("text-align:right"));
    }
}
