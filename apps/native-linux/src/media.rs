//! Images and Mermaid diagrams: decoded or rendered off the main thread, cached, and shown as
//! paintables. SVG (including every Mermaid diagram) is rasterised by resvg at the display's
//! scale, so diagrams stay crisp on HiDPI screens.

use crate::doc::ImageSrc;
use crate::theme::Palette;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib, graphene, gsk, prelude::*};
use resvg::{tiny_skia, usvg};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, OnceLock};

mod imp {
    use super::*;
    use std::cell::Cell;

    #[derive(Default)]
    pub struct ScaledTexture {
        pub texture: RefCell<Option<gdk::Texture>>,
        pub width: Cell<f64>,
        pub height: Cell<f64>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ScaledTexture {
        const NAME: &'static str = "MdowScaledTexture";
        type Type = super::ScaledTexture;
        type Interfaces = (gdk::Paintable,);
    }

    impl ObjectImpl for ScaledTexture {}

    impl PaintableImpl for ScaledTexture {
        fn intrinsic_width(&self) -> i32 {
            self.width.get().round() as i32
        }
        fn intrinsic_height(&self) -> i32 {
            self.height.get().round() as i32
        }
        fn intrinsic_aspect_ratio(&self) -> f64 {
            let h = self.height.get();
            if h > 0.0 { self.width.get() / h } else { 0.0 }
        }
        fn flags(&self) -> gdk::PaintableFlags {
            gdk::PaintableFlags::STATIC_SIZE | gdk::PaintableFlags::STATIC_CONTENTS
        }
        fn snapshot(&self, snapshot: &gdk::Snapshot, width: f64, height: f64) {
            let Some(texture) = self.texture.borrow().clone() else { return };
            if let Some(snapshot) = snapshot.downcast_ref::<gtk::Snapshot>() {
                snapshot.append_scaled_texture(
                    &texture,
                    gsk::ScalingFilter::Trilinear,
                    &graphene::Rect::new(0.0, 0.0, width as f32, height as f32),
                );
            } else {
                texture.snapshot(snapshot, width, height);
            }
        }
    }
}

glib::wrapper! {
    /// A texture drawn at a logical size smaller than its pixel size (HiDPI rasters).
    pub struct ScaledTexture(ObjectSubclass<imp::ScaledTexture>) @implements gdk::Paintable;
}

impl ScaledTexture {
    pub fn new(texture: gdk::Texture, width: f64, height: f64) -> Self {
        let obj: Self = glib::Object::new();
        obj.imp().texture.replace(Some(texture));
        obj.imp().width.set(width);
        obj.imp().height.set(height);
        obj
    }
}

/// Premultiplied RGBA pixels, made on a worker thread.
struct Raster {
    width: u32,
    height: u32,
    logical_width: f64,
    logical_height: f64,
    pixels: Vec<u8>,
}

impl Raster {
    fn into_paintable(self) -> gdk::Paintable {
        let bytes = glib::Bytes::from_owned(self.pixels);
        let texture = gdk::MemoryTexture::new(
            self.width as i32,
            self.height as i32,
            gdk::MemoryFormat::R8g8b8a8Premultiplied,
            &bytes,
            self.width as usize * 4,
        );
        ScaledTexture::new(texture.upcast(), self.logical_width, self.logical_height).upcast()
    }
}

fn fontdb() -> Arc<usvg::fontdb::Database> {
    static DB: OnceLock<Arc<usvg::fontdb::Database>> = OnceLock::new();
    DB.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        if let Some(dir) = crate::fonts::dir() {
            db.load_fonts_dir(dir);
        }
        db.set_monospace_family("Geist Mono");
        db.set_sans_serif_family("Inter Variable");
        Arc::new(db)
    })
    .clone()
}

const MAX_PIXELS: f64 = 8192.0;

fn rasterize_svg(
    data: &[u8],
    scale: f64,
    resources: Option<std::path::PathBuf>,
) -> Result<Raster, String> {
    let options = usvg::Options {
        fontdb: fontdb(),
        resources_dir: resources,
        font_family: "Inter Variable".to_string(),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(data, &options).map_err(|e| e.to_string())?;
    let size = tree.size();
    let (lw, lh) = (size.width() as f64, size.height() as f64);
    let fit = (MAX_PIXELS / (lw * scale)).min(MAX_PIXELS / (lh * scale)).min(1.0);
    let scale = scale * fit;
    let width = (lw * scale).ceil().max(1.0) as u32;
    let height = (lh * scale).ceil().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or("empty image")?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale as f32, scale as f32),
        &mut pixmap.as_mut(),
    );
    Ok(Raster { width, height, logical_width: lw, logical_height: lh, pixels: pixmap.take() })
}

type Waiter = Box<dyn FnOnce(Result<gdk::Paintable, String>)>;

enum Entry {
    Pending(Vec<Waiter>),
    Ready(gdk::Paintable),
    Failed(String),
}

thread_local! {
    static CACHE: RefCell<HashMap<String, Entry>> = RefCell::new(HashMap::new());
}

const CACHE_LIMIT: usize = 400;

/// Run `cb` with the cached result for `key`, or start `work` on a worker thread.
fn cached<W>(key: String, cb: Waiter, work: W)
where
    W: FnOnce() -> Result<WorkOutput, String> + Send + 'static,
{
    let start = CACHE.with_borrow_mut(|cache| {
        if cache.len() > CACHE_LIMIT {
            cache.retain(|_, entry| matches!(entry, Entry::Pending(_)));
        }
        match cache.get_mut(&key) {
            Some(Entry::Ready(paintable)) => {
                let paintable = paintable.clone();
                Some(Err((cb, Ok(paintable))))
            }
            Some(Entry::Failed(err)) => Some(Err((cb, Err(err.clone())))),
            Some(Entry::Pending(waiters)) => {
                waiters.push(cb);
                None
            }
            None => {
                cache.insert(key.clone(), Entry::Pending(vec![cb]));
                Some(Ok(()))
            }
        }
    });
    match start {
        Some(Err((cb, result))) => cb(result),
        Some(Ok(())) => {
            glib::spawn_future_local(async move {
                let result = gtk::gio::spawn_blocking(work)
                    .await
                    .unwrap_or_else(|_| Err("worker panicked".into()));
                let result = result.map(|out| match out {
                    WorkOutput::Raster(raster) => raster.into_paintable(),
                    WorkOutput::Texture(texture) => texture.upcast(),
                });
                let waiters = CACHE.with_borrow_mut(|cache| {
                    let entry = match &result {
                        Ok(p) => Entry::Ready(p.clone()),
                        Err(e) => Entry::Failed(e.clone()),
                    };
                    match cache.insert(key, entry) {
                        Some(Entry::Pending(waiters)) => waiters,
                        _ => Vec::new(),
                    }
                });
                for waiter in waiters {
                    waiter(result.clone());
                }
            });
        }
        None => {}
    }
}

enum WorkOutput {
    Raster(Raster),
    Texture(gdk::Texture),
}

fn decode(
    bytes: Vec<u8>,
    is_svg: bool,
    scale: f64,
    dir: Option<std::path::PathBuf>,
) -> Result<WorkOutput, String> {
    let looks_svg = is_svg || {
        let head = &bytes[..bytes.len().min(512)];
        let head = String::from_utf8_lossy(head);
        head.contains("<svg") || (head.trim_start().starts_with("<?xml") && head.contains("svg"))
    };
    if looks_svg {
        return rasterize_svg(&bytes, scale, dir).map(WorkOutput::Raster);
    }
    gdk::Texture::from_bytes(&glib::Bytes::from_owned(bytes))
        .map(WorkOutput::Texture)
        .map_err(|e| e.to_string())
}

pub fn load_image(
    src: &ImageSrc,
    scale: i32,
    cb: impl FnOnce(Result<gdk::Paintable, String>) + 'static,
) {
    let scale = scale.max(1) as f64;
    match src.clone() {
        ImageSrc::Missing => cb(Err("missing".into())),
        ImageSrc::File(path) => {
            let key = format!("file:{}@{scale}", path.display());
            cached(key, Box::new(cb), move || {
                let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
                let is_svg = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("svg"));
                decode(bytes, is_svg, scale, path.parent().map(|p| p.to_path_buf()))
            });
        }
        ImageSrc::Remote(url) => {
            let key = format!("url:{url}@{scale}");
            // Fetched with GIO (gvfs handles http), then decoded on a worker thread.
            let has_entry = CACHE.with_borrow(|c| c.contains_key(&key));
            if has_entry {
                cached(key, Box::new(cb), || Err("unreachable".into()));
                return;
            }
            CACHE.with_borrow_mut(|c| c.insert(key.clone(), Entry::Pending(vec![Box::new(cb)])));
            glib::spawn_future_local(async move {
                let file = gtk::gio::File::for_uri(&url);
                let fetched = file.load_bytes_future().await.map(|(b, _)| b.to_vec());
                let is_svg = url
                    .split(['?', '#'])
                    .next()
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .ends_with(".svg");
                let result = match fetched {
                    Ok(bytes) => {
                        gtk::gio::spawn_blocking(move || decode(bytes, is_svg, scale, None))
                            .await
                            .unwrap_or_else(|_| Err("worker panicked".into()))
                    }
                    Err(err) => Err(err.to_string()),
                };
                let result = result.map(|out| match out {
                    WorkOutput::Raster(raster) => raster.into_paintable(),
                    WorkOutput::Texture(texture) => texture.upcast::<gdk::Paintable>(),
                });
                let waiters = CACHE.with_borrow_mut(|cache| {
                    let entry = match &result {
                        Ok(p) => Entry::Ready(p.clone()),
                        Err(e) => Entry::Failed(e.clone()),
                    };
                    match cache.insert(key, entry) {
                        Some(Entry::Pending(w)) => w,
                        _ => Vec::new(),
                    }
                });
                for waiter in waiters {
                    waiter(result.clone());
                }
            });
        }
    }
}

/// Mermaid colours from the Mdow palette, as the desktop maps them onto Pele's variables: nodes
/// on the muted surface of code blocks and tables, lines in muted ink, charts in the chart series.
fn mermaid_theme(p: &Palette) -> mermaid_rs_renderer::Theme {
    let mut t = mermaid_rs_renderer::Theme::modern();
    let bg = p.background.hex();
    let fg = p.foreground.hex();
    let surface = p.muted.hex();
    let surface_alt = p.muted.mix(p.background, 0.5).hex();
    let border = p.border.hex();
    let line = p.muted_fg.hex();
    t.font_family = "Geist Mono, monospace".into();
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
    let series: Vec<String> = p.chart.iter().map(|c| c.hex()).collect();
    for (i, slot) in t.git_colors.iter_mut().enumerate() {
        *slot = series[i % series.len()].clone();
    }
    for slot in t.git_branch_label_colors.iter_mut() {
        *slot = bg.clone();
    }
    for (i, slot) in t.pie_colors.iter_mut().enumerate() {
        let base = p.chart[i % p.chart.len()];
        *slot = if i < p.chart.len() { base.hex() } else { base.mix(p.background, 0.35).hex() };
    }
    t.pie_title_text_color = fg.clone();
    t.pie_section_text_color = if p.dark { bg.clone() } else { "#ffffff".into() };
    t.pie_legend_text_color = fg;
    t.pie_stroke_color = bg;
    t.pie_outer_stroke_color = border;
    t
}

pub fn render_mermaid(
    code: Arc<str>,
    palette: &Palette,
    scale: i32,
    cb: impl FnOnce(Result<gdk::Paintable, String>) + 'static,
) {
    let scale = scale.max(1) as f64;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    code.hash(&mut hasher);
    let key = format!("mermaid:{}:{scale}:{:x}", palette.dark, hasher.finish());
    let theme = mermaid_theme(palette);
    cached(key, Box::new(cb), move || {
        let options = mermaid_rs_renderer::RenderOptions {
            theme,
            layout: mermaid_rs_renderer::LayoutConfig::default(),
        };
        let svg =
            std::panic::catch_unwind(|| mermaid_rs_renderer::render_with_options(&code, options))
                .map_err(|_| "The diagram could not be laid out.".to_string())?
                .map_err(|e| e.to_string())?;
        rasterize_svg(svg.as_bytes(), scale, None).map(WorkOutput::Raster)
    });
}

/// The app logo, rasterised for the welcome screen.
pub fn logo(size: f64, scale: i32) -> Option<gdk::Paintable> {
    const LOGO: &[u8] = include_bytes!("../assets/icons/mdow-logo.svg");
    let scale = scale.max(1) as f64;
    let raster = rasterize_svg(LOGO, scale * size / 1024.0, None).ok()?;
    let paintable = raster.into_paintable();
    let texture = paintable.downcast_ref::<ScaledTexture>()?.imp().texture.borrow().clone()?;
    Some(ScaledTexture::new(texture, size, size).upcast())
}

pub const LOGO_SVG: &[u8] = include_bytes!("../assets/icons/mdow-logo.svg");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterizes_svg_at_scale() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20"><rect width="40" height="20" fill="red"/></svg>"#;
        let raster = rasterize_svg(svg, 2.0, None).unwrap();
        assert_eq!((raster.width, raster.height), (80, 40));
        assert_eq!((raster.logical_width, raster.logical_height), (40.0, 20.0));
        assert_eq!(&raster.pixels[..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn renders_mermaid_with_the_palette() {
        let options = mermaid_rs_renderer::RenderOptions {
            theme: mermaid_theme(&Palette::dark()),
            layout: mermaid_rs_renderer::LayoutConfig::default(),
        };
        let svg = mermaid_rs_renderer::render_with_options("graph TD; A-->B", options).unwrap();
        assert!(svg.contains(&Palette::dark().muted.hex()));
        assert!(rasterize_svg(svg.as_bytes(), 1.0, None).is_ok());
    }
}
