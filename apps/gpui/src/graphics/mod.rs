//! Typesets TeX math and Mermaid diagrams into raster images the reader paints.
//!
//! Both engines emit SVG. [`rasterize_svg`] turns that into a GPUI [`RenderImage`] at twice the
//! logical size so it stays crisp on Retina displays, and [`GraphicCache`] keeps finished images
//! (and failures) so scrolling past a diagram never re-renders it.

mod cache;
mod math;
mod measure;
mod mermaid;

pub use cache::{GraphicCache, GraphicKey, GraphicState};
pub use math::{MATH_SCALE, math_width_em, render_math};
pub use mermaid::{DiagramPalette, render_mermaid};

use gpui::RenderImage;
use resvg::{tiny_skia, usvg};
use std::{
    fmt,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::Arc,
};

/// Device pixels per logical pixel for every rasterized graphic.
pub const RASTER_SCALE: f32 = 2.0;

/// Upper bound on one rasterized graphic, so a pathological diagram cannot allocate gigabytes.
const MAX_RASTER_PIXELS: f32 = 24_000_000.0;

/// A rendered image plus the logical geometry the reader lays it out with.
#[derive(Clone)]
pub struct Graphic {
    pub image: Arc<RenderImage>,
    /// Logical width in pixels.
    pub width: f32,
    /// Logical height in pixels.
    pub height: f32,
    /// Logical distance from the bottom edge up to the baseline. Zero for diagrams.
    pub depth: f32,
}

impl Graphic {
    /// Bytes held by the decoded image, used for the cache budget.
    pub fn byte_size(&self) -> usize {
        let size = self.image.size(0);
        (size.width.0.max(0) as usize) * (size.height.0.max(0) as usize) * 4
    }
}

impl fmt::Debug for Graphic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Graphic")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("depth", &self.depth)
            .finish_non_exhaustive()
    }
}

/// Rasterizes `svg` so it covers `logical_width` x `logical_height` logical pixels (defaulting to
/// the SVG's own size) at [`RASTER_SCALE`] times `scale`.
pub(crate) fn rasterize_svg(
    svg: &str,
    fontdb: Option<Arc<usvg::fontdb::Database>>,
    logical_size: Option<(f32, f32)>,
    scale: f32,
) -> Result<Graphic, String> {
    let mut options = usvg::Options::default();
    if let Some(fontdb) = fontdb {
        options.fontdb = fontdb;
    }
    let tree = usvg::Tree::from_str(svg, &options).map_err(|error| error.to_string())?;
    let tree_size = tree.size();
    let (width, height) = logical_size.unwrap_or((tree_size.width(), tree_size.height()));
    if !(width.is_finite() && height.is_finite()) || width <= 0.0 || height <= 0.0 {
        return Err("the graphic has no size".into());
    }

    let mut device_scale = RASTER_SCALE * scale.max(0.1);
    let pixels = width * height * device_scale * device_scale;
    if pixels > MAX_RASTER_PIXELS {
        device_scale *= (MAX_RASTER_PIXELS / pixels).sqrt();
    }
    let pixel_width = (width * device_scale).ceil().max(1.0) as u32;
    let pixel_height = (height * device_scale).ceil().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(pixel_width, pixel_height)
        .ok_or_else(|| "the graphic is too large to rasterize".to_owned())?;
    let transform = tiny_skia::Transform::from_scale(
        pixel_width as f32 / tree_size.width(),
        pixel_height as f32 / tree_size.height(),
    );
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let mut data = pixmap.take();
    for pixel in data.as_chunks_mut::<4>().0 {
        premultiplied_rgba_to_bgra(pixel);
    }
    let buffer = image::RgbaImage::from_raw(pixel_width, pixel_height, data)
        .ok_or_else(|| "the rasterized graphic has the wrong size".to_owned())?;
    Ok(Graphic {
        image: Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])),
        width,
        height,
        depth: 0.0,
    })
}

/// GPUI stores image frames as straight-alpha BGRA; tiny-skia produces premultiplied RGBA.
fn premultiplied_rgba_to_bgra(pixel: &mut [u8; 4]) {
    pixel.swap(0, 2);
    let alpha = pixel[3];
    if alpha > 0 && alpha < u8::MAX {
        let alpha = f32::from(alpha) / 255.0;
        for channel in &mut pixel[..3] {
            *channel = (f32::from(*channel) / alpha).round().min(255.0) as u8;
        }
    }
}

/// Runs a third-party renderer, turning a panic into an ordinary failure so one malformed
/// diagram or formula can never take the reader down.
pub(crate) fn guarded<T>(render: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    catch_unwind(AssertUnwindSafe(render))
        .unwrap_or_else(|_| Err("the renderer stopped unexpectedly".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasterizes_svg_at_twice_the_logical_size() {
        let graphic = rasterize_svg(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="10"><rect width="40" height="10" fill="#ff0000"/></svg>"##,
            None,
            None,
            1.0,
        )
        .unwrap();

        assert_eq!((graphic.width, graphic.height), (40.0, 10.0));
        let size = graphic.image.size(0);
        assert_eq!((size.width.0, size.height.0), (80, 20));
        assert_eq!(graphic.byte_size(), 80 * 20 * 4);
        let bytes = graphic.image.as_bytes(0).unwrap();
        assert_eq!(&bytes[..4], &[0, 0, 255, 255], "red must be stored as BGRA");
    }

    #[test]
    fn unpremultiplies_translucent_pixels() {
        let mut pixel = [0, 0, 128, 128];
        premultiplied_rgba_to_bgra(&mut pixel);
        assert_eq!(pixel, [255, 0, 0, 128]);
    }

    #[test]
    fn rejects_invalid_svg_and_contains_panics() {
        assert!(rasterize_svg("<svg", None, None, 1.0).is_err());
        let result: Result<(), String> = guarded(|| panic!("boom"));
        assert!(result.is_err());
    }
}
