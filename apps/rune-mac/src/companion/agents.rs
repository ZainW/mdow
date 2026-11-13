//! The agents the companion can run, ported from Mrow's `agents.rs`: OpenCode through its own
//! server (`opencode serve`, as before), Claude and Codex through their own interfaces (Claude
//! Code's stream-json mode, `claude.rs`; Codex's app server, `codex.rs`), and Cursor over the
//! Agent Client Protocol (`acp`). Each is found the same way: the command set in Settings, else
//! its program on the usual PATH (the login shell's included). None needs Node.
//!
//! What an ACP agent may do is decided by tool kind: reading, searching and thinking run; edits
//! come back as diffs to accept or decline; anything else (commands, fetches, deletes) is
//! declined and shown in the chat.

use std::path::Path;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Agent {
    #[default]
    OpenCode,
    Claude,
    Codex,
    Cursor,
}

/// Whether an ACP tool call of this kind runs without asking.
pub fn allows(kind: &str) -> bool {
    matches!(kind, "read" | "search" | "think")
}

impl Agent {
    pub const ALL: [Agent; 4] = [Agent::OpenCode, Agent::Claude, Agent::Codex, Agent::Cursor];

    pub fn raw(self) -> &'static str {
        match self {
            Agent::OpenCode => "opencode",
            Agent::Claude => "claude",
            Agent::Codex => "codex",
            Agent::Cursor => "cursor",
        }
    }

    pub fn from_raw(raw: &str) -> Option<Agent> {
        Agent::ALL.into_iter().find(|a| a.raw() == raw)
    }

    /// "OpenCode", "Codex": for the header and the menus.
    pub fn name(self) -> &'static str {
        match self {
            Agent::OpenCode => "OpenCode",
            Agent::Claude => "Claude",
            Agent::Codex => "Codex",
            Agent::Cursor => "Cursor",
        }
    }

    /// The line beside its name in the agent menu.
    pub fn blurb(self) -> &'static str {
        match self {
            Agent::OpenCode => "the providers you use in OpenCode",
            Agent::Claude => "Claude Code, with your Claude plan",
            Agent::Codex => "OpenAI’s Codex, with your ChatGPT plan",
            Agent::Cursor => "Cursor’s agent, read-only",
        }
    }

    /// The process, for errors: "Codex stopped."
    pub fn process(self) -> &'static str {
        match self {
            Agent::OpenCode => "OpenCode",
            Agent::Claude => "Claude Code",
            Agent::Codex => "Codex",
            Agent::Cursor => "Cursor’s agent",
        }
    }

    /// Whether it runs through the ACP-shaped path (every agent but OpenCode).
    pub fn is_acp(self) -> bool {
        self != Agent::OpenCode
    }

    /// Whether Mdow drives it through its maker's own interface, in-process, rather than ACP.
    pub fn native(self) -> bool {
        matches!(self, Agent::Claude | Agent::Codex)
    }

    /// Whether its edits come back for review. Cursor stays in its read-only ask mode: its own
    /// settings can run commands without asking.
    pub fn edits(self) -> bool {
        self != Agent::Cursor
    }

    /// What it can do, after its name, for the greeting: "Codex reads…".
    pub fn reach(self) -> &'static str {
        if self.edits() {
            "reads the documents in this folder and can suggest edits. Every change comes back as a diff for you to approve."
        } else {
            "reads the documents in this folder. It’s read-only here: it can’t edit them or run commands."
        }
    }

    /// The mode a session starts in, where the agent has modes that limit it.
    pub fn mode(self) -> Option<&'static str> {
        match self {
            Agent::Cursor => Some("ask"),
            _ => None,
        }
    }

    pub fn sign_in(self) -> &'static str {
        match self {
            Agent::OpenCode => "Run `opencode auth login` to connect a provider.",
            Agent::Claude => {
                "Claude isn’t signed in. Run `claude` in a terminal and sign in with /login, then ask again."
            }
            Agent::Codex => {
                "Codex isn’t signed in. Run `codex login` in a terminal, then ask again."
            }
            Agent::Cursor => {
                "Cursor isn’t signed in. Run `cursor-agent login` in a terminal, then ask again."
            }
        }
    }

    /// How to install it and sign in, for the setup screen.
    pub fn setup_commands(self) -> &'static [&'static str] {
        match self {
            Agent::OpenCode => {
                &["curl -fsSL https://opencode.ai/install | bash", "opencode auth login"]
            }
            Agent::Claude => &["curl -fsSL https://claude.ai/install.sh | bash", "claude"],
            Agent::Codex => &["brew install --cask codex", "codex login"],
            Agent::Cursor => &["curl https://cursor.com/install -fsS | bash", "cursor-agent login"],
        }
    }

    pub fn docs_url(self) -> &'static str {
        match self {
            Agent::OpenCode => "https://opencode.ai/docs",
            Agent::Claude => "https://docs.anthropic.com/en/docs/claude-code/setup",
            Agent::Codex => "https://developers.openai.com/codex/cli",
            Agent::Cursor => "https://cursor.com/cli",
        }
    }

    /// The program and its arguments for an ACP agent.
    pub fn command(self) -> Result<Vec<String>, String> {
        let custom = crate::prefs::get()
            .companion_agent_commands
            .get(self.raw())
            .map(|c| c.trim().to_string())
            .unwrap_or_default();
        if !custom.is_empty() {
            let words: Vec<String> = custom.split_whitespace().map(str::to_string).collect();
            if find(&words[0]).is_none() {
                return Err(format!(
                    "{} isn’t at {}. Fix its command in Settings.",
                    self.process(),
                    words[0]
                ));
            }
            return Ok(words);
        }
        let with = |program: String, args: &[&str]| {
            std::iter::once(program).chain(args.iter().map(|a| a.to_string())).collect::<Vec<_>>()
        };
        match self {
            Agent::OpenCode => Err("OpenCode runs as a server, not over ACP.".into()),
            Agent::Cursor => {
                let local = format!("{}/.local/bin/agent", super::logic::home());
                find(&local).or_else(|| find("cursor-agent")).map(|p| with(p, &["acp"])).ok_or_else(
                    || {
                        "Couldn’t find Cursor’s agent. Install it, or set its command in Settings."
                            .into()
                    },
                )
            }
            Agent::Claude => find("claude").map(|p| vec![p]).ok_or_else(|| {
                "Couldn’t find Claude Code. Install it and sign in, or set its command in Settings."
                    .into()
            }),
            Agent::Codex => find("codex").map(|p| with(p, &["app-server"])).ok_or_else(|| {
                "Couldn’t find Codex. Install it and sign in, or set its command in Settings."
                    .into()
            }),
        }
    }
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

/// A program by name on the companion's PATH, or a path as it is.
pub fn find(name: &str) -> Option<String> {
    if name.contains('/') {
        return is_executable(Path::new(name)).then(|| name.to_string());
    }
    super::process::search_directories()
        .into_iter()
        .map(|d| Path::new(&d).join(name))
        .find(|p| is_executable(p))
        .map(|p| p.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agents_round_trip_and_limit_what_runs() {
        for a in Agent::ALL {
            assert_eq!(Agent::from_raw(a.raw()), Some(a));
        }
        assert!(allows("read") && allows("search") && !allows("execute") && !allows("edit"));
        assert!(!Agent::Cursor.edits() && Agent::Codex.edits());
    }
}
