//! Finds locally installed ACP agent CLIs. Mdow never installs anything itself.
//!
//! Apps launched from Finder inherit a minimal `PATH`, so detection also searches the login
//! shell's `PATH` and the usual per-user install directories. The same search path is handed to
//! the agent process, so Node-based adapters can find `node`.

use super::acp::McpServer;
use super::types::{Availability, ProviderId, ProviderStatus};
use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
    sync::OnceLock,
    time::Duration,
};

/// The program and arguments Mdow runs for one provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderCommand {
    pub program: PathBuf,
    pub args: Vec<String>,
    pub display: String,
}

/// Where executables are looked up: an ordered list of directories.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchPath {
    dirs: Vec<PathBuf>,
}

impl SearchPath {
    pub fn new(dirs: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut unique = Vec::new();
        for dir in dirs {
            if !dir.as_os_str().is_empty() && !unique.contains(&dir) {
                unique.push(dir);
            }
        }
        Self { dirs: unique }
    }

    pub fn from_path_var(value: &OsStr) -> Self {
        Self::new(std::env::split_paths(value))
    }

    /// The process `PATH`, the login shell's `PATH`, then common install locations.
    pub fn system() -> Self {
        static SYSTEM: OnceLock<SearchPath> = OnceLock::new();
        SYSTEM
            .get_or_init(|| {
                let mut dirs = Vec::new();
                if let Some(path) = std::env::var_os("PATH") {
                    dirs.extend(std::env::split_paths(&path));
                }
                if let Some(path) = login_shell_path() {
                    dirs.extend(std::env::split_paths(&path));
                }
                let home = std::env::var_os("HOME").map(PathBuf::from);
                dirs.extend(well_known_dirs(home.as_deref()));
                Self::new(dirs)
            })
            .clone()
    }

    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }

    pub fn find(&self, command: &str) -> Option<PathBuf> {
        self.dirs
            .iter()
            .map(|dir| dir.join(command))
            .find(|candidate| is_executable(candidate))
    }

    /// A `PATH` value for the agent process.
    pub fn to_path_var(&self) -> OsString {
        std::env::join_paths(&self.dirs).unwrap_or_default()
    }
}

pub fn well_known_dirs(home: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = home {
        for relative in [
            ".local/bin",
            ".opencode/bin",
            ".bun/bin",
            ".npm-global/bin",
            ".cargo/bin",
            ".volta/bin",
            "Library/pnpm",
        ] {
            dirs.push(home.join(relative));
        }
    }
    dirs.extend(
        ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"]
            .into_iter()
            .map(PathBuf::from),
    );
    dirs
}

/// Asks the login shell for its `PATH`, giving up after two seconds.
fn login_shell_path() -> Option<OsString> {
    let shell = std::env::var_os("SHELL")?;
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let output = std::process::Command::new(shell)
            .args(["-l", "-c", "printf %s \"$PATH\""])
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output();
        let _ = sender.send(output);
    });
    let output = receiver.recv_timeout(Duration::from_secs(2)).ok()?.ok()?;
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim();
    (output.status.success() && !text.is_empty()).then(|| OsString::from(text))
}

pub fn is_executable(path: &Path) -> bool {
    let Ok(metadata) = std::fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// Executable names tried for each built-in provider, in order.
fn candidates(id: ProviderId) -> &'static [&'static str] {
    match id {
        ProviderId::OpenCode => &["opencode"],
        ProviderId::CodexAcp => &["codex-acp"],
        ProviderId::ClaudeCode => &["claude-code-acp", "claude-agent-acp"],
        ProviderId::Gemini => &["gemini"],
        ProviderId::Custom => &[],
    }
}

fn args_for(id: ProviderId) -> Vec<String> {
    match id {
        ProviderId::OpenCode => vec!["acp".into()],
        ProviderId::Gemini => vec!["--experimental-acp".into()],
        _ => Vec::new(),
    }
}

fn display_for(id: ProviderId, custom: Option<&Path>) -> String {
    match id {
        ProviderId::OpenCode => "opencode acp".into(),
        ProviderId::CodexAcp => "codex-acp".into(),
        ProviderId::ClaudeCode => "claude-code-acp".into(),
        ProviderId::Gemini => "gemini --experimental-acp".into(),
        ProviderId::Custom => custom
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| "(none)".into()),
    }
}

/// Resolves the command to run. Custom executables must be absolute paths with no arguments,
/// as in Electron: Mdow never runs shell command lines.
pub fn resolve_command(
    id: ProviderId,
    custom: Option<&Path>,
    search: &SearchPath,
) -> Option<ProviderCommand> {
    let program = match id {
        ProviderId::Custom => {
            let path = custom?;
            if !path.is_absolute() || path.to_string_lossy().trim().is_empty() {
                return None;
            }
            path.to_owned()
        }
        _ => candidates(id)
            .iter()
            .find_map(|name| search.find(name))
            .unwrap_or_else(|| PathBuf::from(candidates(id)[0])),
    };
    Some(ProviderCommand {
        program,
        args: args_for(id),
        display: display_for(id, custom),
    })
}

pub fn detect_providers(custom: Option<&Path>, search: &SearchPath) -> Vec<ProviderStatus> {
    ProviderId::ALL
        .into_iter()
        .map(|id| {
            let executable = match id {
                ProviderId::Custom => custom
                    .filter(|path| path.is_absolute() && is_executable(path))
                    .map(Path::to_owned),
                _ => candidates(id).iter().find_map(|name| search.find(name)),
            };
            let availability = if executable.is_some() {
                Availability::Available
            } else {
                Availability::Missing
            };
            let detail = match (id, availability) {
                (ProviderId::Custom, Availability::Available) => {
                    Some("Runs the executable you selected as a local ACP subprocess.".into())
                }
                (_, Availability::Available) => None,
                (_, Availability::Missing) => Some(id.install_hint().into()),
            };
            ProviderStatus {
                id,
                label: id.label(),
                command_display: display_for(id, custom),
                executable,
                availability,
                detail,
            }
        })
        .collect()
}

/// The optional read-only FFF search server, offered to OpenCode sessions.
pub fn resolve_fff_mcp(search: &SearchPath) -> Option<McpServer> {
    let command = search.find("fff-mcp")?;
    Some(McpServer {
        name: "fff".into(),
        command: command.to_string_lossy().into_owned(),
        args: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn install(dir: &Path, name: &str) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = dir.join(name);
        std::fs::write(&path, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        path
    }

    #[test]
    fn resolves_built_in_and_custom_commands() {
        let empty = SearchPath::default();
        let opencode = resolve_command(ProviderId::OpenCode, None, &empty).unwrap();
        assert_eq!(opencode.program, PathBuf::from("opencode"));
        assert_eq!(opencode.args, vec!["acp".to_string()]);
        assert_eq!(opencode.display, "opencode acp");
        let codex = resolve_command(ProviderId::CodexAcp, None, &empty).unwrap();
        assert_eq!(codex.display, "codex-acp");
        assert!(codex.args.is_empty());
        let gemini = resolve_command(ProviderId::Gemini, None, &empty).unwrap();
        assert_eq!(gemini.args, vec!["--experimental-acp".to_string()]);

        let custom_path = Path::new("/Applications/My Agent.app/Contents/MacOS/agent");
        let custom = resolve_command(ProviderId::Custom, Some(custom_path), &empty).unwrap();
        assert_eq!(custom.program, custom_path);
        assert!(custom.args.is_empty());
        assert_eq!(custom.display, custom_path.to_string_lossy());
        assert!(
            resolve_command(
                ProviderId::Custom,
                Some(Path::new("my-agent --acp")),
                &empty
            )
            .is_none()
        );
        assert!(
            resolve_command(
                ProviderId::Custom,
                Some(Path::new("relative/agent")),
                &empty
            )
            .is_none()
        );
        assert!(resolve_command(ProviderId::Custom, None, &empty).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn detects_installed_clis_on_a_fake_path() {
        let bin = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        let opencode = install(bin.path(), "opencode");
        let claude = install(other.path(), "claude-agent-acp");
        // Present but not executable: not an installed CLI.
        std::fs::write(bin.path().join("codex-acp"), "").unwrap();
        let search = SearchPath::new([bin.path().to_owned(), other.path().to_owned()]);

        let statuses = detect_providers(None, &search);
        let status = |id| statuses.iter().find(|status| status.id == id).unwrap();
        assert!(status(ProviderId::OpenCode).is_available());
        assert_eq!(
            status(ProviderId::OpenCode).executable.as_ref(),
            Some(&opencode)
        );
        assert!(!status(ProviderId::CodexAcp).is_available());
        assert!(
            status(ProviderId::CodexAcp)
                .detail
                .as_ref()
                .unwrap()
                .contains("npx")
        );
        assert_eq!(
            status(ProviderId::ClaudeCode).executable.as_ref(),
            Some(&claude)
        );
        assert!(!status(ProviderId::Gemini).is_available());
        assert!(!status(ProviderId::Custom).is_available());
        assert_eq!(
            resolve_command(ProviderId::ClaudeCode, None, &search)
                .unwrap()
                .program,
            claude
        );
    }

    #[cfg(unix)]
    #[test]
    fn custom_executables_must_exist_and_be_absolute() {
        let dir = tempfile::tempdir().unwrap();
        let agent = install(dir.path(), "agent");
        let search = SearchPath::default();
        let statuses = detect_providers(Some(&agent), &search);
        let custom = statuses
            .iter()
            .find(|s| s.id == ProviderId::Custom)
            .unwrap();
        assert!(custom.is_available());
        assert_eq!(custom.command_display, agent.to_string_lossy());

        let missing = detect_providers(Some(Path::new("/nope/agent")), &search);
        assert!(
            !missing
                .iter()
                .find(|s| s.id == ProviderId::Custom)
                .unwrap()
                .is_available()
        );
    }

    #[cfg(unix)]
    #[test]
    fn fff_mcp_is_optional_and_read_only() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(
            resolve_fff_mcp(&SearchPath::new([dir.path().to_owned()])),
            None
        );
        let fff = install(dir.path(), "fff-mcp");
        assert_eq!(
            resolve_fff_mcp(&SearchPath::new([dir.path().to_owned()])),
            Some(McpServer {
                name: "fff".into(),
                command: fff.to_string_lossy().into_owned(),
                args: Vec::new(),
            })
        );
    }

    #[test]
    fn search_path_dedupes_and_round_trips_through_a_path_variable() {
        let search = SearchPath::new([
            PathBuf::from("/a"),
            PathBuf::from("/b"),
            PathBuf::from("/a"),
            PathBuf::new(),
        ]);
        assert_eq!(search.dirs(), &[PathBuf::from("/a"), PathBuf::from("/b")]);
        assert_eq!(SearchPath::from_path_var(&search.to_path_var()), search);
        let dirs = well_known_dirs(Some(Path::new("/Users/me")));
        assert!(dirs.contains(&PathBuf::from("/Users/me/.local/bin")));
        assert!(dirs.contains(&PathBuf::from("/opt/homebrew/bin")));
    }
}
