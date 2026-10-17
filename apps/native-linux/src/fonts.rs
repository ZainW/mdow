//! The desktop app's fonts (Inter, Charter, Geist Mono, JetBrains Mono), compiled into the binary
//! and registered with fontconfig for this process only. Nothing is installed into the user's home.

use std::ffi::{CString, c_char, c_int, c_void};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::sync::OnceLock;

macro_rules! font {
    ($name:literal) => {
        ($name, include_bytes!(concat!("../../native/assets/fonts/", $name)) as &[u8])
    };
}

const FONTS: &[(&str, &[u8])] = &[
    font!("Inter-400.ttf"),
    font!("Inter-500.ttf"),
    font!("Inter-600.ttf"),
    font!("Inter-700.ttf"),
    font!("Inter-Italic-400.ttf"),
    font!("Inter-Italic-500.ttf"),
    font!("Inter-Italic-600.ttf"),
    font!("Inter-Italic-700.ttf"),
    font!("Charter-Regular.ttf"),
    font!("Charter-Italic.ttf"),
    font!("Charter-Bold.ttf"),
    font!("Charter-BoldItalic.ttf"),
    font!("GeistMono-400.ttf"),
    font!("GeistMono-500.ttf"),
    font!("GeistMono-700.ttf"),
    font!("GeistMono-Italic-400.ttf"),
    font!("GeistMono-Italic-700.ttf"),
    font!("JetBrainsMono-400.ttf"),
    font!("JetBrainsMono-500.ttf"),
    font!("JetBrainsMono-700.ttf"),
    font!("JetBrainsMono-Italic-400.ttf"),
    font!("JetBrainsMono-Italic-700.ttf"),
];

static DIR: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Copy the fonts into the cache directory (unchanged files are left alone): Pango and fontdb
/// load fonts from files.
fn extract() -> Option<PathBuf> {
    DIR.get_or_init(|| {
        let dir = gtk::glib::user_cache_dir().join("mdow-linux").join("fonts");
        std::fs::create_dir_all(&dir).ok()?;
        for (name, bytes) in FONTS {
            let dest = dir.join(name);
            let same =
                std::fs::metadata(&dest).map(|m| m.len() == bytes.len() as u64).unwrap_or(false);
            if !same {
                std::fs::write(&dest, bytes).ok()?;
            }
        }
        Some(dir)
    })
    .clone()
}

pub fn dir() -> Option<PathBuf> {
    extract()
}

#[link(name = "fontconfig")]
unsafe extern "C" {
    fn FcConfigGetCurrent() -> *mut c_void;
    fn FcConfigAppFontAddFile(config: *mut c_void, file: *const c_char) -> c_int;
}

/// Add the bundled fonts to fontconfig's application font set. Call before GTK starts, so the
/// font map Pango builds already includes them. (Pango's own `add_font_file` needs Pango 1.56.)
pub fn register() {
    let Some(dir) = extract() else { return };
    // SAFETY: fontconfig's current configuration is process-global and outlives this call; the
    // path is a valid NUL-terminated string for the duration of the call.
    unsafe {
        let config = FcConfigGetCurrent();
        if config.is_null() {
            return;
        }
        for (name, _) in FONTS {
            let Ok(path) = CString::new(dir.join(name).as_os_str().as_bytes()) else { continue };
            if FcConfigAppFontAddFile(config, path.as_ptr()) == 0 {
                eprintln!("mdow: could not load font {name}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_font_is_bundled() {
        assert!(FONTS.iter().all(|(_, bytes)| bytes.len() > 10_000));
    }
}
