//! Inline markdown flattened into one string with Rune spans: bold, italic, code, marks, links,
//! footnote references and inline math, so a paragraph is a single selectable text element.

use crate::doc::Inline;
use crate::theme::Palette;
use rune::Span;
use std::ops::Range;

/// A paragraph's text and how to draw it.
#[derive(Default, Clone, Debug)]
pub struct Flat {
    pub text: String,
    pub spans: Vec<Span>,
    /// Byte range and target of each link.
    pub links: Vec<(Range<usize>, String)>,
    pub strikes: Vec<Range<usize>>,
    /// Inline code's padding spaces, drawn but not copied.
    pub uncopied: Vec<Range<usize>>,
    /// Inline formulas: the range their readable text takes, and their TeX.
    pub maths: Vec<(Range<usize>, String)>,
}

#[derive(Clone, Copy, Default)]
struct Fmt {
    bold: bool,
    italic: bool,
    underline: bool,
    mark: bool,
    small: bool,
    link: bool,
}

pub struct InlineStyle<'a> {
    pub p: &'a Palette,
    pub size: f32,
    pub code_family_mono: bool,
    /// The weight plain text has (headings are bold).
    pub base_weight: f32,
}

pub fn flatten(inlines: &[Inline], st: &InlineStyle) -> Flat {
    let mut out = Flat::default();
    walk(inlines, st, Fmt::default(), &mut out);
    // Trailing breaks (a `<br>` before the end) would draw an empty last line.
    while out.text.ends_with('\n') {
        out.text.pop();
    }
    let len = out.text.len();
    for s in &mut out.spans {
        s.end = s.end.min(len as u32);
        s.start = s.start.min(s.end);
    }
    out.links.retain(|(r, _)| r.start < len);
    for (r, _) in &mut out.links {
        r.end = r.end.min(len);
    }
    out
}

fn styled(fmt: Fmt, st: &InlineStyle, start: usize, end: usize) -> Option<Span> {
    if !(fmt.bold || fmt.italic || fmt.underline || fmt.mark || fmt.small || fmt.link) {
        return None;
    }
    let mut s = Span::new(start, end);
    if fmt.bold {
        s = s.weight(if st.base_weight >= 600.0 { 800.0 } else { 700.0 });
    }
    if fmt.italic {
        s = s.italic();
    }
    if fmt.link {
        s = s.color(st.p.accent);
    }
    if fmt.underline {
        s = s.underline(st.p.text.alpha(0.6));
    }
    if fmt.mark {
        s = s.bg(st.p.mark, 1.0);
    }
    if fmt.small {
        s = s.size((st.size * 0.72).round());
    }
    Some(s)
}

fn push_text(text: &str, fmt: Fmt, st: &InlineStyle, out: &mut Flat) {
    if text.is_empty() {
        return;
    }
    let start = out.text.len();
    out.text.push_str(text);
    if let Some(s) = styled(fmt, st, start, out.text.len()) {
        out.spans.push(s);
    }
}

fn walk(inlines: &[Inline], st: &InlineStyle, fmt: Fmt, out: &mut Flat) {
    for inline in inlines {
        match inline {
            Inline::Text(t) => push_text(t, fmt, st, out),
            Inline::Break => out.text.push('\n'),
            Inline::Code(code) => {
                // A thin space either side gives the rounded background room; it isn't copied.
                let pad_start = out.text.len();
                out.text.push('\u{2009}');
                out.uncopied.push(pad_start..out.text.len());
                let start = out.text.len();
                out.text.push_str(&code.replace('\n', " "));
                let end = out.text.len();
                out.text.push('\u{2009}');
                out.uncopied.push(end..out.text.len());
                let mut s = Span::new(start, end).mono().size((st.size * 0.86).round());
                if !st.code_family_mono {
                    s.mono = true;
                }
                s = s.bg(st.p.inline_code_bg, 2.5);
                if fmt.link {
                    s = s.color(st.p.accent);
                }
                if fmt.bold {
                    s = s.weight(600.0);
                }
                out.spans.push(s);
            }
            Inline::Strong(c) => walk(c, st, Fmt { bold: true, ..fmt }, out),
            Inline::Emph(c) => walk(c, st, Fmt { italic: true, ..fmt }, out),
            Inline::Underline(c) => walk(c, st, Fmt { underline: true, ..fmt }, out),
            Inline::Mark(c) => walk(c, st, Fmt { mark: true, ..fmt }, out),
            Inline::Sup(c) | Inline::Sub(c) => walk(c, st, Fmt { small: true, ..fmt }, out),
            Inline::Strike(c) => {
                let start = out.text.len();
                walk(c, st, fmt, out);
                out.strikes.push(start..out.text.len());
            }
            Inline::Link { url, children } => {
                let start = out.text.len();
                walk(children, st, Fmt { link: true, ..fmt }, out);
                if out.text.len() == start {
                    push_text(url, Fmt { link: true, ..fmt }, st, out);
                }
                out.links.push((start..out.text.len(), url.clone()));
            }
            Inline::Image { alt, .. } => {
                let label = if alt.trim().is_empty() { "image".to_string() } else { alt.clone() };
                let start = out.text.len();
                out.text.push_str(&label);
                out.spans.push(Span::new(start, out.text.len()).color(st.p.secondary).italic());
            }
            Inline::Math(tex) => {
                let shown = tex_to_unicode(tex);
                let start = out.text.len();
                out.text.push_str(&shown);
                let mut s = Span::new(start, out.text.len()).italic();
                if fmt.bold {
                    s = s.bold();
                }
                out.spans.push(s);
                out.maths.push((start..out.text.len(), tex.clone()));
            }
            Inline::FootnoteRef(label) => {
                // A superscript number, as the Swift app sets it.
                const SUP: [char; 10] = ['⁰', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', '⁸', '⁹'];
                let sup: Option<String> =
                    label.chars().map(|c| c.to_digit(10).map(|d| SUP[d as usize])).collect();
                let start = out.text.len();
                let mut span = Span::new(start, start).color(st.p.accent).weight(600.0);
                match sup {
                    Some(s) => out.text.push_str(&s),
                    None => {
                        out.text.push_str(label);
                        span = span.size((st.size * 0.72).round());
                    }
                }
                span.end = out.text.len() as u32;
                out.spans.push(span);
                out.links.push((start..out.text.len(), format!("#fn-{label}")));
            }
        }
    }
}

/// Inline TeX as readable text: Greek letters, common operators, and super- and subscript
/// digits. Display math is typeset properly; this keeps inline formulas legible in a line.
pub fn tex_to_unicode(tex: &str) -> String {
    const COMMANDS: &[(&str, &str)] = &[
        ("alpha", "α"),
        ("beta", "β"),
        ("gamma", "γ"),
        ("delta", "δ"),
        ("epsilon", "ε"),
        ("varepsilon", "ε"),
        ("zeta", "ζ"),
        ("eta", "η"),
        ("theta", "θ"),
        ("vartheta", "ϑ"),
        ("iota", "ι"),
        ("kappa", "κ"),
        ("lambda", "λ"),
        ("mu", "μ"),
        ("nu", "ν"),
        ("xi", "ξ"),
        ("pi", "π"),
        ("rho", "ρ"),
        ("sigma", "σ"),
        ("tau", "τ"),
        ("upsilon", "υ"),
        ("phi", "φ"),
        ("varphi", "φ"),
        ("chi", "χ"),
        ("psi", "ψ"),
        ("omega", "ω"),
        ("Gamma", "Γ"),
        ("Delta", "Δ"),
        ("Theta", "Θ"),
        ("Lambda", "Λ"),
        ("Xi", "Ξ"),
        ("Pi", "Π"),
        ("Sigma", "Σ"),
        ("Phi", "Φ"),
        ("Psi", "Ψ"),
        ("Omega", "Ω"),
        ("times", "×"),
        ("cdot", "·"),
        ("div", "÷"),
        ("pm", "±"),
        ("mp", "∓"),
        ("leq", "≤"),
        ("le", "≤"),
        ("geq", "≥"),
        ("ge", "≥"),
        ("neq", "≠"),
        ("ne", "≠"),
        ("approx", "≈"),
        ("equiv", "≡"),
        ("sim", "∼"),
        ("propto", "∝"),
        ("infty", "∞"),
        ("partial", "∂"),
        ("nabla", "∇"),
        ("sum", "∑"),
        ("prod", "∏"),
        ("int", "∫"),
        ("oint", "∮"),
        ("sqrt", "√"),
        ("in", "∈"),
        ("notin", "∉"),
        ("subset", "⊂"),
        ("subseteq", "⊆"),
        ("supset", "⊃"),
        ("cup", "∪"),
        ("cap", "∩"),
        ("emptyset", "∅"),
        ("forall", "∀"),
        ("exists", "∃"),
        ("neg", "¬"),
        ("land", "∧"),
        ("lor", "∨"),
        ("to", "→"),
        ("rightarrow", "→"),
        ("leftarrow", "←"),
        ("Rightarrow", "⇒"),
        ("Leftarrow", "⇐"),
        ("leftrightarrow", "↔"),
        ("Leftrightarrow", "⇔"),
        ("mapsto", "↦"),
        ("ldots", "…"),
        ("cdots", "⋯"),
        ("dots", "…"),
        ("circ", "∘"),
        ("degree", "°"),
        ("prime", "′"),
        ("hbar", "ℏ"),
        ("ell", "ℓ"),
        ("Re", "ℜ"),
        ("Im", "ℑ"),
        ("aleph", "ℵ"),
        ("langle", "⟨"),
        ("rangle", "⟩"),
        ("lfloor", "⌊"),
        ("rfloor", "⌋"),
        ("lceil", "⌈"),
        ("rceil", "⌉"),
        ("quad", " "),
        ("qquad", "  "),
        ("left", ""),
        ("right", ""),
        ("mathrm", ""),
        ("mathbf", ""),
        ("mathit", ""),
        ("text", ""),
        ("operatorname", ""),
        ("displaystyle", ""),
        ("frac", ""),
        ("log", "log"),
        ("ln", "ln"),
        ("sin", "sin"),
        ("cos", "cos"),
        ("tan", "tan"),
        ("exp", "exp"),
        ("lim", "lim"),
        ("max", "max"),
        ("min", "min"),
        ("det", "det"),
    ];
    fn script(c: char, sup: bool) -> Option<char> {
        let (from, to) = if sup {
            ("0123456789+-=()ni", "⁰¹²³⁴⁵⁶⁷⁸⁹⁺⁻⁼⁽⁾ⁿⁱ")
        } else {
            ("0123456789+-=()aeoxij", "₀₁₂₃₄₅₆₇₈₉₊₋₌₍₎ₐₑₒₓᵢⱼ")
        };
        from.chars().position(|f| f == c).and_then(|i| to.chars().nth(i))
    }
    let chars: Vec<char> = tex.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\\' {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_alphabetic() {
                j += 1;
            }
            if j == i + 1 {
                // An escaped symbol: \{ \} \, \; \!
                if let Some(&n) = chars.get(j) {
                    match n {
                        ',' | ';' | ':' => out.push(' '),
                        '!' => {}
                        other => out.push(other),
                    }
                }
                i = j + 1;
                continue;
            }
            let name: String = chars[i + 1..j].iter().collect();
            match COMMANDS.iter().find(|(n, _)| *n == name) {
                Some((_, s)) => out.push_str(s),
                None => out.push_str(&name),
            }
            i = j;
            continue;
        }
        if (c == '^' || c == '_') && i + 1 < chars.len() {
            let sup = c == '^';
            let (group, next): (Vec<char>, usize) = if chars[i + 1] == '{' {
                let mut depth = 0;
                let mut j = i + 1;
                while j < chars.len() {
                    match chars[j] {
                        '{' => depth += 1,
                        '}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                (chars[i + 2..j.min(chars.len())].to_vec(), j + 1)
            } else {
                (vec![chars[i + 1]], i + 2)
            };
            let inner: String = group.iter().collect();
            let inner = tex_to_unicode(&inner);
            let mapped: Option<String> = inner.chars().map(|c| script(c, sup)).collect();
            match mapped {
                Some(m) => out.push_str(&m),
                None => {
                    out.push(c);
                    out.push_str(&inner);
                }
            }
            i = next;
            continue;
        }
        if c == '{' || c == '}' {
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_tex_reads_as_text() {
        assert_eq!(tex_to_unicode(r"\alpha^2 + \beta_{10}"), "α² + β₁₀");
        assert_eq!(tex_to_unicode(r"x \leq \infty"), "x ≤ ∞");
    }

    #[test]
    fn links_and_code_keep_their_ranges() {
        let p = Palette::get(false);
        let st = InlineStyle { p: &p, size: 16.0, code_family_mono: true, base_weight: 400.0 };
        let flat = flatten(
            &[
                Inline::Text("see ".into()),
                Inline::Link {
                    url: "https://x".into(),
                    children: vec![Inline::Text("docs".into())],
                },
                Inline::Text(" and ".into()),
                Inline::Code("run".into()),
            ],
            &st,
        );
        let (range, url) = &flat.links[0];
        assert_eq!(&flat.text[range.clone()], "docs");
        assert_eq!(url, "https://x");
        assert!(flat.text.contains("run"));
        assert_eq!(flat.uncopied.len(), 2);
    }
}
