use gpui::{AssetSource, Result, SharedString};
use std::{
    borrow::Cow,
    fs,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};

pub struct MdowAssets {
    root: PathBuf,
}

impl MdowAssets {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn resolve(&self, path: &str) -> Result<Option<PathBuf>> {
        let path = Path::new(path);
        if path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            anyhow::bail!("asset paths must stay inside the asset root");
        }

        let root = match fs::canonicalize(&self.root) {
            Ok(root) => root,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let resolved = match fs::canonicalize(root.join(path)) {
            Ok(resolved) => resolved,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };

        if !resolved.starts_with(&root) {
            anyhow::bail!("asset paths must stay inside the asset root");
        }

        Ok(Some(resolved))
    }
}

impl AssetSource for MdowAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        let Some(path) = self.resolve(path)? else {
            return Ok(None);
        };
        match fs::read(path) {
            Ok(bytes) => Ok(Some(Cow::Owned(bytes))),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let Some(directory) = self.resolve(path)? else {
            return Ok(Vec::new());
        };
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error.into()),
        };
        let prefix = path.trim_end_matches('/');
        let mut paths = entries
            .map(|entry| {
                let entry = entry?;
                let name = entry.file_name().to_string_lossy().into_owned();
                Ok(SharedString::from(if prefix.is_empty() {
                    name
                } else {
                    format!("{prefix}/{name}")
                }))
            })
            .collect::<std::io::Result<Vec<_>>>()?;
        paths.sort_unstable();
        Ok(paths)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::AssetSource;
    use std::fs;

    #[test]
    fn loads_assets_relative_to_the_configured_root() {
        let dir = tempfile::tempdir().unwrap();
        let icons = dir.path().join("icons");
        fs::create_dir(&icons).unwrap();
        fs::write(icons.join("file.svg"), b"<svg />").unwrap();
        let source = MdowAssets::new(dir.path().to_owned());

        assert_eq!(
            source.load("icons/file.svg").unwrap().unwrap().as_ref(),
            b"<svg />"
        );
        assert_eq!(source.load("icons/missing.svg").unwrap(), None);
        assert_eq!(source.list("icons").unwrap(), vec!["icons/file.svg"]);
    }

    #[test]
    fn rejects_parent_directory_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let source = MdowAssets::new(dir.path().to_owned());

        assert!(source.load("../secret.svg").is_err());
        assert!(source.list("../icons").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_that_escape_the_asset_root() {
        use std::os::unix::fs::symlink;

        let parent = tempfile::tempdir().unwrap();
        let root = parent.path().join("assets");
        let outside = parent.path().join("outside");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(outside.join("secret.svg"), b"not an asset").unwrap();
        symlink(&outside, root.join("escaped")).unwrap();
        let source = MdowAssets::new(root);

        assert!(source.load("escaped/secret.svg").is_err());
        assert!(source.list("escaped").is_err());
    }
}
