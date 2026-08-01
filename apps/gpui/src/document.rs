use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedSource {
    pub canonical_path: PathBuf,
    pub source: String,
}

#[derive(Debug)]
pub enum DocumentError {
    Unsupported { path: PathBuf },
    Missing { path: PathBuf },
    InvalidUtf8 { path: PathBuf },
    Read { path: PathBuf, message: String },
}

impl DocumentError {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Unsupported { .. } => "Unsupported file type",
            Self::Missing { .. } => "File not found",
            Self::InvalidUtf8 { .. } => "This file is not UTF-8",
            Self::Read { .. } => "Couldn't read file",
        }
    }

    pub fn body(&self) -> &'static str {
        match self {
            Self::Unsupported { .. } => {
                "Mdow opens .md, .markdown, and .mdx files. Choose a Markdown file or drop a folder."
            }
            Self::Missing { .. } => "This file may have been moved or renamed.",
            Self::InvalidUtf8 { .. } => "Mdow can only open files encoded as UTF-8.",
            Self::Read { .. } => {
                "Something went wrong trying to read this file. It might be corrupted or locked by another process."
            }
        }
    }

    pub fn path(&self) -> &Path {
        match self {
            Self::Unsupported { path }
            | Self::Missing { path }
            | Self::InvalidUtf8 { path }
            | Self::Read { path, .. } => path,
        }
    }
}

pub fn is_supported_markdown(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()).is_some_and(|extension| {
        matches!(extension.to_ascii_lowercase().as_str(), "md" | "markdown" | "mdx")
    })
}

pub fn load_source(path: &Path) -> Result<LoadedSource, DocumentError> {
    if !is_supported_markdown(path) {
        return Err(DocumentError::Unsupported {
            path: path.to_owned(),
        });
    }
    if !path.exists() {
        return Err(DocumentError::Missing {
            path: path.to_owned(),
        });
    }
    let bytes = std::fs::read(path).map_err(|error| DocumentError::Read {
        path: path.to_owned(),
        message: error.to_string(),
    })?;
    let source = String::from_utf8(bytes).map_err(|_| DocumentError::InvalidUtf8 {
        path: path.to_owned(),
    })?;
    let canonical_path = path.canonicalize().map_err(|error| DocumentError::Read {
        path: path.to_owned(),
        message: error.to_string(),
    })?;
    Ok(LoadedSource {
        canonical_path,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn recognizes_supported_extensions_case_insensitively() {
        assert!(is_supported_markdown(Path::new("README.md")));
        assert!(is_supported_markdown(Path::new("notes.MARKDOWN")));
        assert!(is_supported_markdown(Path::new("component.MdX")));
        assert!(!is_supported_markdown(Path::new("notes.txt")));
    }

    #[test]
    fn loads_utf8_and_returns_a_canonical_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.md");
        fs::write(&path, "# Hello\n").unwrap();

        let loaded = load_source(&path).unwrap();

        assert_eq!(loaded.canonical_path, path.canonicalize().unwrap());
        assert_eq!(loaded.source, "# Hello\n");
    }

    #[test]
    fn reports_invalid_utf8_without_debug_copy() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.md");
        fs::write(&path, [0xff, 0xfe]).unwrap();

        let error = load_source(&path).unwrap_err();

        assert!(matches!(error, DocumentError::InvalidUtf8 { .. }));
        assert_eq!(error.title(), "This file is not UTF-8");
    }
}
