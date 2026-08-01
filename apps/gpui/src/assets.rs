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

    fn resolve(&self, path: &str) -> Result<PathBuf> {
        let path = Path::new(path);
        if path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            anyhow::bail!("asset paths must stay inside the asset root");
        }
        Ok(self.root.join(path))
    }
}

impl AssetSource for MdowAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match fs::read(self.resolve(path)?) {
            Ok(bytes) => Ok(Some(Cow::Owned(bytes))),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let directory = self.resolve(path)?;
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
}
