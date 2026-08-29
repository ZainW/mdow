use anyhow::Context;
use gpui::{AssetSource, Result, SharedString};
use std::{
    borrow::Cow,
    ffi::OsStr,
    fs,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};

pub const REQUIRED_ASSETS: &[&str] = &[
    "fonts/InterVariable.ttf",
    "fonts/GeistMono-Variable.ttf",
    "icons/alert-circle.svg",
    "icons/check.svg",
    "icons/chevron-down.svg",
    "icons/clock.svg",
    "icons/chevron-right.svg",
    "icons/chevron-up.svg",
    "icons/command.svg",
    "icons/copy.svg",
    "icons/expand.svg",
    "icons/file.svg",
    "icons/folder-open.svg",
    "icons/folder.svg",
    "icons/list.svg",
    "icons/mdow-logo.svg",
    "icons/search.svg",
    "icons/settings.svg",
    "icons/sidebar.svg",
    "icons/x.svg",
];

pub fn discover_asset_root(
    executable: impl AsRef<Path>,
    development_assets: impl AsRef<Path>,
) -> Result<PathBuf> {
    let executable = fs::canonicalize(executable.as_ref()).context("canonicalizing executable")?;

    if let Some(contents) = bundled_contents(&executable) {
        let resources = fs::canonicalize(contents.join("Resources"))
            .context("canonicalizing bundled Contents/Resources")?;
        if !resources.starts_with(contents) {
            anyhow::bail!("bundled resources must stay inside Contents");
        }
        let assets = fs::canonicalize(resources.join("assets"))
            .context("canonicalizing bundled Contents/Resources/assets")?;
        if !assets.starts_with(&resources) {
            anyhow::bail!("bundled assets must stay inside Contents/Resources");
        }
        return Ok(assets);
    }

    if executable
        .ancestors()
        .any(|ancestor| ancestor.extension() == Some(OsStr::new("app")))
    {
        anyhow::bail!("executable is inside a malformed app bundle");
    }

    fs::canonicalize(development_assets.as_ref()).context("canonicalizing development assets")
}

pub fn validate_required_assets(root: impl AsRef<Path>) -> Result<()> {
    let source = MdowAssets::new(root.as_ref().to_owned());
    let mut missing = Vec::new();

    for asset in REQUIRED_ASSETS {
        match source
            .resolve(asset)
            .with_context(|| format!("validating required asset {asset}"))?
        {
            Some(path) if path.is_file() => {}
            _ => missing.push(*asset),
        }
    }

    if missing.is_empty() {
        Ok(())
    } else {
        anyhow::bail!("missing required Mdow assets: {}", missing.join(", "));
    }
}

fn bundled_contents(executable: &Path) -> Option<&Path> {
    let macos = executable.parent()?;
    let contents = macos.parent()?;
    let bundle = contents.parent()?;

    (macos.file_name() == Some(OsStr::new("MacOS"))
        && contents.file_name() == Some(OsStr::new("Contents"))
        && bundle.extension() == Some(OsStr::new("app")))
    .then_some(contents)
}

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
    fn discovers_assets_next_to_a_bundled_executable() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("Mdow Native.app/Contents/MacOS/MdowNative");
        let bundled_assets = dir.path().join("Mdow Native.app/Contents/Resources/assets");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::create_dir_all(&bundled_assets).unwrap();
        fs::write(&executable, b"fixture").unwrap();

        assert_eq!(
            discover_asset_root(&executable, dir.path().join("development-assets")).unwrap(),
            bundled_assets.canonicalize().unwrap(),
        );
    }

    #[test]
    fn falls_back_to_development_assets_outside_an_app_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("target/debug/mdow-gpui");
        let development_assets = dir.path().join("assets");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::create_dir_all(&development_assets).unwrap();
        fs::write(&executable, b"fixture").unwrap();

        assert_eq!(
            discover_asset_root(&executable, &development_assets).unwrap(),
            development_assets.canonicalize().unwrap(),
        );
    }

    #[test]
    fn rejects_a_bundled_executable_without_resources_assets() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("Mdow Native.app/Contents/MacOS/MdowNative");
        let development_assets = dir.path().join("development-assets");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::create_dir(&development_assets).unwrap();
        fs::write(&executable, b"fixture").unwrap();

        assert!(discover_asset_root(&executable, &development_assets).is_err());
    }

    #[test]
    fn rejects_a_malformed_app_bundle_without_falling_back_to_development_assets() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("Mdow Native.app/Contents/bin/MdowNative");
        let development_assets = dir.path().join("development-assets");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::create_dir(&development_assets).unwrap();
        fs::write(&executable, b"fixture").unwrap();

        assert!(discover_asset_root(&executable, &development_assets).is_err());
    }

    #[test]
    fn rejects_a_missing_development_asset_root() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("target/debug/mdow-gpui");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::write(&executable, b"fixture").unwrap();

        assert!(discover_asset_root(&executable, dir.path().join("missing-assets")).is_err());
    }

    #[test]
    fn reports_every_missing_required_asset() {
        let dir = tempfile::tempdir().unwrap();
        let error = validate_required_assets(dir.path())
            .unwrap_err()
            .to_string();

        for asset in REQUIRED_ASSETS {
            assert!(error.contains(asset), "missing {asset} from {error}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn rejects_bundled_assets_symlinked_outside_resources() {
        use std::os::unix::fs::symlink;

        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("Mdow Native.app/Contents/MacOS/MdowNative");
        let resources = dir.path().join("Mdow Native.app/Contents/Resources");
        let outside = dir.path().join("outside-assets");
        fs::create_dir_all(executable.parent().unwrap()).unwrap();
        fs::create_dir_all(&resources).unwrap();
        fs::create_dir(&outside).unwrap();
        fs::write(&executable, b"fixture").unwrap();
        symlink(&outside, resources.join("assets")).unwrap();

        assert!(discover_asset_root(&executable, dir.path().join("development-assets")).is_err());
    }

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
