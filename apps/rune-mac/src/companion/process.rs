//! Finding `opencode` and running Mdow's private `opencode serve`, ported from the Swift app's
//! `OpencodeProcess.swift`.
//!
//! A Mac app launched from Finder gets a minimal PATH, so the login shell's PATH is asked for as
//! well as the usual install folders. Mdow owns its server instead of borrowing the user's
//! background service, so it controls the credentials and the server's lifetime matches the
//! app's: it is stopped on quit (`atexit`), on SIGTERM, SIGINT and SIGHUP, and an orphan left by
//! a crash is stopped the next time one starts.

use super::logic;
use super::types::{Availability, RuntimeStatus};
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, Once, OnceLock};
use std::time::{Duration, Instant};

unsafe extern "C" {
    fn kill(pid: i32, sig: i32) -> i32;
    fn signal(sig: i32, handler: usize) -> usize;
    fn raise(sig: i32) -> i32;
    fn atexit(cb: extern "C" fn()) -> i32;
    fn setsid() -> i32;
}

const SIGHUP: i32 = 1;
const SIGINT: i32 = 2;
const SIGTERM: i32 = 15;
const SIG_DFL: usize = 0;

pub struct Resolved {
    pub path: String,
    pub version: String,
}

/// The user's login shell PATH, read once (in a few hundred milliseconds) and cached.
pub fn login_shell_path() -> Option<String> {
    static PATH: OnceLock<Option<String>> = OnceLock::new();
    PATH.get_or_init(|| {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
        let marker = logic::PATH_MARKER;
        let script = format!("printf '{marker}%s{marker}' \"$PATH\"");
        run(&shell, &["-l", "-i", "-c", &script], Duration::from_secs(4), None)
            .and_then(|out| logic::parse_shell_path(&out))
    })
    .clone()
}

pub fn search_directories() -> Vec<String> {
    logic::search_directories(
        std::env::var("PATH").ok().as_deref(),
        &logic::home(),
        login_shell_path().as_deref(),
    )
}

/// PATH for the server process, so OpenCode can find git and its own helpers.
pub fn environment_path() -> String {
    search_directories().join(":")
}

fn is_executable(path: &Path) -> bool {
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

pub fn find() -> Option<String> {
    search_directories()
        .into_iter()
        .map(|d| Path::new(&d).join("opencode"))
        .find(|p| is_executable(p))
        .map(|p| p.to_string_lossy().into_owned())
}

pub fn detect() -> (RuntimeStatus, Option<Resolved>) {
    let Some(path) = find() else {
        return (
            RuntimeStatus::new(
                Availability::Missing,
                None,
                Some(
                    "Install OpenCode 2 from opencode.ai, sign in to a provider, then try again."
                        .into(),
                ),
            ),
            None,
        );
    };
    let output = run(&path, &["--version"], Duration::from_secs(5), Some(&environment_path()));
    let Some(version) = output.and_then(|o| logic::parse_version(&o)) else {
        return (
            RuntimeStatus::new(
                Availability::Failed,
                None,
                Some(format!("Could not run {path} --version.")),
            ),
            None,
        );
    };
    if !logic::is_supported_version(&version) {
        let detail = format!(
            "Mdow needs OpenCode {} or newer. Run `opencode upgrade`.",
            logic::MINIMUM_MAJOR_VERSION
        );
        return (RuntimeStatus::new(Availability::Outdated, Some(version), Some(detail)), None);
    }
    (
        RuntimeStatus::new(Availability::Available, Some(version.clone()), None),
        Some(Resolved { path, version }),
    )
}

/// Runs a short command and returns its standard output, or None if it failed or timed out.
pub fn run(
    executable: &str,
    args: &[&str],
    timeout: Duration,
    path: Option<&str>,
) -> Option<String> {
    let mut command = Command::new(executable);
    command.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    if let Some(path) = path {
        command.env("PATH", path);
    }
    // A session of its own, so an interactive login shell can't take over Mdow's terminal.
    unsafe {
        command.pre_exec(|| {
            setsid();
            Ok(())
        });
    }
    let mut child = command.spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    // The output arrives on a channel rather than through `join`: a background process the
    // shell started can hold the pipe open long after the shell is gone.
    let (sent, output) = mpsc::channel();
    std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stdout.read_to_end(&mut out);
        let _ = sent.send(out);
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                // The child leads its own session (setsid), so this stops what it started too.
                unsafe { kill(-(child.id() as i32), 9) };
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let status = status.filter(|s| s.success())?;
    let out = output.recv_timeout(Duration::from_millis(500)).ok()?;
    let _ = status;
    Some(String::from_utf8_lossy(&out).into_owned())
}

/// The running server's pid, for stopping it from a signal handler or at exit.
static SERVER_PID: AtomicI32 = AtomicI32::new(0);
static NEXT_SERVER: AtomicU64 = AtomicU64::new(1);

/// A private `opencode serve` bound to loopback on a free port.
pub struct Server {
    pub id: u64,
    pub url: String,
    /// Basic auth password, generated per launch and never written to disk.
    pub password: String,
    pid: i32,
    exited: Arc<AtomicBool>,
}

impl Server {
    pub fn is_running(&self) -> bool {
        !self.exited.load(Ordering::SeqCst)
    }

    pub fn stop(&self) {
        if self.is_running() {
            unsafe { kill(self.pid, SIGTERM) };
        }
    }

    /// Starts the server and waits (up to 20 seconds) for it to say where it listens.
    /// `on_exit` runs once with the server's id when the process ends, whether stopped or
    /// crashed.
    pub fn start(
        binary: &str,
        on_exit: impl FnOnce(u64) + Send + 'static,
    ) -> Result<Server, String> {
        stop_orphan();
        let password = logic::make_password();
        let mut child = Command::new(binary)
            .args(["serve", "--hostname=127.0.0.1", "--port=0"])
            .env("PATH", environment_path())
            .env("OPENCODE_SERVER_PASSWORD", &password)
            .current_dir(logic::home())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Could not start OpenCode: {e}"))?;
        let pid = child.id() as i32;
        SERVER_PID.store(pid, Ordering::SeqCst);
        record(pid);

        let output = Arc::new(Mutex::new(String::new()));
        let (tx, rx) = mpsc::channel::<Result<String, String>>();
        // Keep draining both pipes after start-up so the server never blocks on a full buffer.
        let mut drains = Vec::new();
        for pipe in [
            child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>),
            child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>),
        ]
        .into_iter()
        .flatten()
        {
            let output = output.clone();
            let tx = tx.clone();
            drains.push(std::thread::spawn(move || drain(pipe, &output, &tx)));
        }
        let exited = Arc::new(AtomicBool::new(false));
        let id = NEXT_SERVER.fetch_add(1, Ordering::Relaxed);
        {
            let exited = exited.clone();
            let output = output.clone();
            std::thread::spawn(move || {
                let status = child.wait();
                exited.store(true, Ordering::SeqCst);
                let _ = SERVER_PID.compare_exchange(pid, 0, Ordering::SeqCst, Ordering::SeqCst);
                clear(pid);
                // Let the pipes deliver the last words.
                let until = Instant::now() + Duration::from_millis(300);
                while Instant::now() < until && drains.iter().any(|d| !d.is_finished()) {
                    std::thread::sleep(Duration::from_millis(10));
                }
                let code = status.ok().and_then(|s| s.code()).unwrap_or(-1);
                let text = output.lock().unwrap().clone();
                let lines: Vec<&str> = text.trim().lines().collect();
                let detail = lines[lines.len().saturating_sub(3)..].join("\n");
                let _ = tx.send(Err(if detail.is_empty() {
                    format!("OpenCode exited ({code})")
                } else {
                    format!("OpenCode exited ({code}): {detail}")
                }));
                on_exit(id);
            });
        }
        match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(Ok(url)) => Ok(Server { id, url, password, pid, exited }),
            Ok(Err(e)) => Err(e),
            Err(_) => {
                if !exited.load(Ordering::SeqCst) {
                    unsafe { kill(pid, SIGTERM) };
                }
                Err("OpenCode did not start within 20 seconds.".into())
            }
        }
    }
}

fn drain(
    mut pipe: Box<dyn Read + Send>,
    output: &Mutex<String>,
    tx: &mpsc::Sender<Result<String, String>>,
) {
    let mut buf = [0u8; 4096];
    let mut found = false;
    loop {
        match pipe.read(&mut buf) {
            Ok(0) | Err(_) => return,
            Ok(n) => {
                let mut out = output.lock().unwrap();
                out.push_str(&String::from_utf8_lossy(&buf[..n]));
                if out.len() > 8_000 {
                    let mut cut = out.len() - 8_000;
                    while !out.is_char_boundary(cut) {
                        cut += 1;
                    }
                    out.drain(..cut);
                }
                if !found && let Some(url) = logic::parse_listening_url(&out) {
                    found = true;
                    let _ = tx.send(Ok(url));
                }
            }
        }
    }
}

// MARK: Cleanup

fn pid_file() -> PathBuf {
    PathBuf::from(logic::home()).join("Library/Caches/com.zain.mdow.Rune/opencode-server.pid")
}

fn recorded_pid() -> Option<i32> {
    std::fs::read_to_string(pid_file()).ok()?.trim().parse().ok()
}

fn record(pid: i32) {
    let file = pid_file();
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(file, pid.to_string());
}

fn clear(pid: i32) {
    if recorded_pid() == Some(pid) {
        let _ = std::fs::remove_file(pid_file());
    }
}

/// Stops a server recorded by an earlier Mdow that is still running without it: its parent is
/// now launchd and it is still `opencode`, so it can't be anything else.
fn stop_orphan() {
    let Some(pid) = recorded_pid().filter(|p| *p > 1) else { return };
    let _ = std::fs::remove_file(pid_file());
    let Some(out) = run(
        "/bin/ps",
        &["-o", "ppid=,comm=", "-p", &pid.to_string()],
        Duration::from_secs(2),
        None,
    ) else {
        return;
    };
    let mut fields = out.split_whitespace();
    let ppid = fields.next().and_then(|p| p.parse::<i32>().ok());
    let comm = fields.collect::<Vec<_>>().join(" ");
    if ppid == Some(1) && Path::new(&comm).file_name().is_some_and(|n| n == "opencode") {
        unsafe { kill(pid, SIGTERM) };
    }
}

extern "C" fn stop_at_exit() {
    let pid = SERVER_PID.load(Ordering::SeqCst);
    if pid > 0 {
        unsafe { kill(pid, SIGTERM) };
    }
}

extern "C" fn stop_on_signal(sig: i32) {
    stop_at_exit();
    unsafe {
        signal(sig, SIG_DFL);
        raise(sig);
    }
}

/// Stops the server when Mdow exits or is told to (SIGTERM, SIGINT, SIGHUP), then lets the
/// signal end Mdow as before.
pub fn install_cleanup() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe {
        atexit(stop_at_exit);
        for sig in [SIGTERM, SIGINT, SIGHUP] {
            signal(sig, stop_on_signal as extern "C" fn(i32) as usize);
        }
    });
}
