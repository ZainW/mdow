//! Images, Mermaid diagrams and display math, made off the main thread and shown as Rune images.
//!
//! Raster files decode on Rune's image worker. SVG (local, remote, every Mermaid diagram and
//! every formula) is rasterised here by resvg at twice its size, with the system fonts loaded,
//! so badges and diagram labels keep their text. Remote files are fetched with ureq.

use crate::doc::ImageSrc;
use crate::theme::Palette;
use resvg::{tiny_skia, usvg};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, OnceLock};

/// Pixels per point for everything this module rasterises.
pub const SCALE: f32 = 2.0;
const MAX_PIXELS: f32 = 8192.0;
const CACHE_LIMIT: usize = 600;

#[derive(Clone)]
pub enum Asset {
    Pending,
    /// The image and its size in points.
    Ready(rune::Image, [f32; 2]),
    Failed(#[allow(dead_code)] String),
}

enum Done {
    /// Straight-alpha RGBA, its pixel size and its size in points.
    Raster(String, u32, u32, Vec<u8>, [f32; 2]),
    /// Encoded bytes for Rune's decoder.
    Bytes(String, Vec<u8>),
    Failed(String, String),
}

pub struct Media {
    cache: Option<rune::ImageCache>,
    waker: Option<rune::Waker>,
    assets: HashMap<String, Asset>,
    /// Raster files handed to Rune's decoder, waiting for their size.
    decoding: Vec<String>,
    /// How far each inline formula reaches below the baseline, in points.
    depths: HashMap<String, f32>,
    depth_tx: Sender<(String, f32)>,
    depth_rx: Receiver<(String, f32)>,
    tx: Sender<Done>,
    rx: Receiver<Done>,
}

impl Default for Media {
    fn default() -> Self {
        let (tx, rx) = channel();
        let (depth_tx, depth_rx) = channel();
        Media {
            cache: None,
            waker: None,
            assets: HashMap::new(),
            decoding: Vec::new(),
            depths: HashMap::new(),
            depth_tx,
            depth_rx,
            tx,
            rx,
        }
    }
}

pub fn image_key(src: &ImageSrc) -> String {
    match src {
        ImageSrc::File(p) => format!("file:{}", p.display()),
        ImageSrc::Remote(u) => format!("url:{u}"),
        ImageSrc::Missing => "missing".into(),
    }
}

fn hash(parts: impl Hash) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    parts.hash(&mut h);
    h.finish()
}

pub fn mermaid_key(code: &str, dark: bool) -> String {
    format!("mermaid:{dark}:{:x}", hash(code))
}

pub fn math_key(tex: &str, dark: bool, size: f32) -> String {
    format!("math:{dark}:{}:{:x}", size.round() as i32, hash(tex))
}

/// An inline formula: text style, so its depth below the baseline is known (`Media::depth`).
pub fn inline_math_key(tex: &str, dark: bool, size: f32) -> String {
    format!("imath:{dark}:{}:{:x}", size.round() as i32, hash(tex))
}

impl Media {
    pub fn setup(&mut self, waker: rune::Waker) {
        self.cache = Some(rune::ImageCache::new(waker.clone()));
        self.waker = Some(waker);
    }

    pub fn get(&self, key: &str) -> Option<&Asset> {
        self.assets.get(key)
    }

    fn start(&mut self, key: &str) -> bool {
        if self.assets.contains_key(key) {
            return false;
        }
        if self.assets.len() > CACHE_LIMIT {
            self.assets.retain(|_, a| matches!(a, Asset::Pending));
        }
        self.assets.insert(key.to_string(), Asset::Pending);
        true
    }

    fn spawn(&self, work: impl FnOnce() -> Done + Send + 'static) {
        let tx = self.tx.clone();
        let waker = self.waker.clone();
        std::thread::spawn(move || {
            let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(work))
                .unwrap_or_else(|_| Done::Failed(String::new(), "The worker stopped.".into()));
            let _ = tx.send(done);
            if let Some(w) = waker {
                w();
            }
        });
    }

    pub fn request_image(&mut self, src: &ImageSrc) {
        let key = image_key(src);
        if !self.start(&key) {
            return;
        }
        match src.clone() {
            ImageSrc::Missing => {
                self.assets.insert(key, Asset::Failed("No image".into()));
            }
            ImageSrc::File(path) => {
                let dir = path.parent().map(|p| p.to_path_buf());
                self.spawn(move || match std::fs::read(&path) {
                    Ok(bytes) => decode(key, bytes, dir),
                    Err(e) => Done::Failed(key, e.to_string()),
                });
            }
            ImageSrc::Remote(url) => {
                self.spawn(move || match fetch(&url) {
                    Ok(bytes) => decode(key, bytes, None),
                    Err(e) => Done::Failed(key, e),
                });
            }
        }
    }

    pub fn request_mermaid(&mut self, code: &Arc<str>, palette: &Palette) {
        let key = mermaid_key(code, palette.dark);
        if !self.start(&key) {
            return;
        }
        let theme = mermaid_theme(palette);
        let code = code.clone();
        self.spawn(move || {
            let options = mermaid_rs_renderer::RenderOptions {
                theme,
                layout: mermaid_rs_renderer::LayoutConfig::default(),
            };
            match mermaid_rs_renderer::render_with_options(&code, options) {
                Ok(svg) => raster(key, svg.as_bytes(), None),
                Err(e) => Done::Failed(key, e.to_string()),
            }
        });
    }

    pub fn request_math(&mut self, tex: &str, palette: &Palette, size: f32) {
        let key = math_key(tex, palette.dark, size);
        if !self.start(&key) {
            return;
        }
        let tex = tex.to_string();
        let color = palette.text.hex();
        self.spawn(move || match math_svg(&tex, &color, size, true) {
            Ok((svg, _)) => raster(key, svg.as_bytes(), None),
            Err(e) => Done::Failed(key, e),
        });
    }

    /// An inline formula, typeset in text style at `size`.
    pub fn request_inline_math(&mut self, tex: &str, palette: &Palette, size: f32) {
        let key = inline_math_key(tex, palette.dark, size);
        if !self.start(&key) {
            return;
        }
        let tex = tex.to_string();
        let color = palette.text.hex();
        let depths = self.depth_tx.clone();
        // STIX Two's x-height is larger than the body faces', so formulas set a little smaller.
        let set = (size * 0.92).round();
        self.spawn(move || match math_svg(&tex, &color, set, false) {
            Ok((svg, depth)) => {
                let _ = depths.send((key.clone(), depth));
                raster(key, svg.as_bytes(), None)
            }
            Err(e) => Done::Failed(key, e),
        });
    }

    pub fn depth(&self, key: &str) -> f32 {
        self.depths.get(key).copied().unwrap_or(0.0)
    }

    /// Lands finished work; true when anything changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok((key, depth)) = self.depth_rx.try_recv() {
            self.depths.insert(key, depth);
        }
        while let Ok(done) = self.rx.try_recv() {
            changed = true;
            match done {
                Done::Raster(key, w, h, rgba, size) => match &self.cache {
                    Some(cache) => {
                        let image = cache.rgba(w, h, rgba);
                        self.assets.insert(key, Asset::Ready(image, size));
                    }
                    None => {
                        self.assets.insert(key, Asset::Failed("No image cache".into()));
                    }
                },
                Done::Bytes(key, bytes) => match &self.cache {
                    Some(cache) => {
                        let image = cache.load(bytes);
                        self.assets.insert(key.clone(), Asset::Ready(image, [0.0, 0.0]));
                        self.decoding.push(key);
                    }
                    None => {
                        self.assets.insert(key, Asset::Failed("No image cache".into()));
                    }
                },
                Done::Failed(key, err) => {
                    if !key.is_empty() {
                        self.assets.insert(key, Asset::Failed(err));
                    }
                }
            }
        }
        // Raster files report their size once Rune's decoder is done.
        let mut still = Vec::new();
        for key in std::mem::take(&mut self.decoding) {
            let Some(Asset::Ready(image, _)) = self.assets.get(&key).cloned() else { continue };
            if let Some(err) = image.error() {
                self.assets.insert(key, Asset::Failed(err.to_string()));
                changed = true;
            } else if let Some([w, h]) = image.size() {
                // Pictures are drawn at their pixel size, as a browser draws them at 1×.
                self.assets.insert(key, Asset::Ready(image, [w as f32, h as f32]));
                changed = true;
            } else {
                still.push(key);
            }
        }
        self.decoding = still;
        changed
    }
}

fn fetch(url: &str) -> Result<Vec<u8>, String> {
    let response = ureq::get(url).header("User-Agent", "Mdow").call().map_err(|e| e.to_string())?;
    response
        .into_body()
        .with_config()
        .limit(32 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| e.to_string())
}

fn looks_svg(bytes: &[u8]) -> bool {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]);
    head.contains("<svg") || (head.trim_start().starts_with("<?xml") && head.contains("svg"))
}

fn decode(key: String, bytes: Vec<u8>, dir: Option<std::path::PathBuf>) -> Done {
    if looks_svg(&bytes) { raster(key, &bytes, dir) } else { Done::Bytes(key, bytes) }
}

fn fontdb() -> Arc<usvg::fontdb::Database> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        for font in crate::theme::FONTS {
            db.load_font_data(font.to_vec());
        }
        db.set_monospace_family(crate::theme::MONO_FAMILY);
        db.set_sans_serif_family(crate::theme::UI_FAMILY);
        Arc::new(db)
    })
    .clone()
}

fn raster(key: String, data: &[u8], dir: Option<std::path::PathBuf>) -> Done {
    let options = usvg::Options {
        fontdb: fontdb(),
        resources_dir: dir,
        font_family: crate::theme::UI_FAMILY.to_string(),
        ..usvg::Options::default()
    };
    let tree = match usvg::Tree::from_data(data, &options) {
        Ok(t) => t,
        Err(e) => return Done::Failed(key, e.to_string()),
    };
    let size = tree.size();
    let (lw, lh) = (size.width(), size.height());
    let fit = (MAX_PIXELS / (lw * SCALE)).min(MAX_PIXELS / (lh * SCALE)).min(1.0);
    let scale = SCALE * fit;
    let width = (lw * scale).ceil().max(1.0) as u32;
    let height = (lh * scale).ceil().max(1.0) as u32;
    let Some(mut pixmap) = tiny_skia::Pixmap::new(width, height) else {
        return Done::Failed(key, "The image is empty.".into());
    };
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    // Rune takes straight alpha.
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for p in pixmap.pixels() {
        let c = p.demultiply();
        rgba.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    Done::Raster(key, width, height, rgba, [lw, lh])
}

/// The formula as SVG, and how far it reaches below the baseline in points.
pub fn math_svg(tex: &str, color: &str, size: f32, display: bool) -> Result<(String, f32), String> {
    use latex_rust::{Dim, MathFont, MathStyle, SvgOptions, layout, parse, render_svg};
    static FONT: OnceLock<Option<MathFont>> = OnceLock::new();
    let font =
        FONT.get_or_init(|| MathFont::stix_two_math().ok()).as_ref().ok_or("No math font")?;
    let hex = color.trim_start_matches('#');
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0);
    let mut options = SvgOptions::new();
    options.font_size_pt = Dim::from_i64(size.round() as i64);
    options.color = latex_rust::Color::rgb(byte(0), byte(2), byte(4));
    options.display = display;
    let ast = parse(tex.trim()).map_err(|e| e.to_string())?;
    let style = if display { MathStyle::Display } else { MathStyle::Text };
    let tree = layout(&ast, font, style).map_err(|e| e.to_string())?;
    let depth = f32::from_bits(tree.depth.to_ieee32_bits()) * size.round();
    let svg = render_svg(&tree, font, &options).map_err(|e| e.to_string())?;
    Ok((svg, depth))
}

/// Mermaid colours from the Mdow palette: nodes on the well of code blocks, lines in secondary
/// ink, labels in the text colour, clusters a step lighter.
pub fn mermaid_theme(p: &Palette) -> mermaid_rs_renderer::Theme {
    let mut t = mermaid_rs_renderer::Theme::modern();
    let hex = |c: rune::Color| c.hex()[..7].to_string();
    let bg = hex(p.paper);
    let fg = hex(p.text);
    let surface = hex(p.well);
    let surface_alt = hex(p.well.mix(p.paper, 0.5));
    let border = hex(p.strong_line.alpha(1.0).mix(p.paper, 0.75));
    let line = hex(p.secondary);
    t.font_family = crate::theme::UI_FAMILY.to_string();
    t.font_size = 13.5;
    t.primary_color = surface.clone();
    t.primary_text_color = fg.clone();
    t.primary_border_color = border.clone();
    t.line_color = line.clone();
    t.secondary_color = surface_alt.clone();
    t.tertiary_color = bg.clone();
    t.edge_label_background = bg.clone();
    t.cluster_background = surface_alt.clone();
    t.cluster_border = border.clone();
    t.background = bg.clone();
    t.sequence_actor_fill = surface.clone();
    t.sequence_actor_border = border.clone();
    t.sequence_actor_line = line.clone();
    t.sequence_note_fill = surface_alt.clone();
    t.sequence_note_border = border.clone();
    t.sequence_activation_fill = surface_alt;
    t.sequence_activation_border = border.clone();
    t.text_color = fg.clone();
    let series = [
        p.accent,
        p.syntax(crate::highlight::Tok::String).unwrap_or(p.accent),
        p.syntax(crate::highlight::Tok::Entity).unwrap_or(p.accent),
        p.syntax(crate::highlight::Tok::Constant).unwrap_or(p.accent),
        p.syntax(crate::highlight::Tok::Keyword).unwrap_or(p.accent),
    ];
    for (i, slot) in t.git_colors.iter_mut().enumerate() {
        *slot = hex(series[i % series.len()]);
    }
    for slot in t.git_branch_label_colors.iter_mut() {
        *slot = bg.clone();
    }
    for (i, slot) in t.pie_colors.iter_mut().enumerate() {
        *slot =
            hex(series[i % series.len()].mix(p.paper, if i < series.len() { 0.0 } else { 0.35 }));
    }
    t.pie_title_text_color = fg.clone();
    t.pie_section_text_color = if p.dark { bg.clone() } else { "#ffffff".into() };
    t.pie_legend_text_color = fg;
    t.pie_stroke_color = bg;
    t.pie_outer_stroke_color = border;
    t
}
