//! Finding the `opencode` binary, and the private `opencode serve` Mdow runs on loopback. Mdow
//! owns this server instead of borrowing a background service, so it controls the credentials
//! and the server's lifetime matches the app's.

use super::logic;
use super::types::*;
use gtk::glib;
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::mpsc;
use std::sync::{Once, OnceLock};
use std::time::Duration;

pub struct Resolved {
    pub path: String,
    pub version: String,
}

/// Puts the child in its own session, away from any terminal, so an interactive login shell can't
/// stop on terminal I/O and so the server's helpers can be signalled as a group.
fn detach(command: &mut Command) {
    // SAFETY: `setsid` is async-signal-safe and touches no memory of the parent.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

/// Runs a short command and returns its standard output, or `None` if it failed or timed out.
fn run(
    executable: &str,
    arguments: &[&str],
    timeout: Duration,
    path: Option<&str>,
) -> Option<String> {
    let mut command = Command::new(executable);
    command.args(arguments).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    if let Some(path) = path {
        command.env("PATH", path);
    }
    detach(&mut command);
    let mut child = command.spawn().ok()?;
    let pid = child.id() as i32;
    let mut stdout = child.stdout.take()?;
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut text = String::new();
        let read = stdout.read_to_string(&mut text);
        let succeeded = child.wait().is_ok_and(|status| status.success());
        let _ = sender.send((read.is_ok() && succeeded).then_some(text));
    });
    match receiver.recv_timeout(timeout) {
        Ok(output) => output,
        Err(_) => {
            // SAFETY: signalling the process group the child leads.
            unsafe { libc::kill(-pid, libc::SIGKILL) };
            None
        }
    }
}

/// The user's login shell PATH, read once and cached.
fn login_shell_path() -> Option<String> {
    static PATH: OnceLock<Option<String>> = OnceLock::new();
    PATH.get_or_init(|| {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
        let marker = logic::PATH_MARKER;
        let script = format!("printf '{marker}%s{marker}' \"$PATH\"");
        run(&shell, &["-l", "-i", "-c", &script], Duration::from_secs(4), None)
            .and_then(|output| logic::parse_shell_path(&output))
    })
    .clone()
}

fn search_directories() -> Vec<String> {
    logic::search_directories(
        std::env::var("PATH").ok().as_deref(),
        &glib::home_dir().to_string_lossy(),
        login_shell_path().as_deref(),
    )
}

fn is_executable(path: &Path) -> bool {
    path.metadata().is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

fn find() -> Option<String> {
    search_directories().into_iter().find_map(|directory| {
        let candidate = Path::new(&directory).join("opencode");
        is_executable(&candidate).then(|| candidate.to_string_lossy().into_owned())
    })
}

pub fn detect() -> (RuntimeStatus, Option<Resolved>) {
    let status = |availability, version, detail: Option<String>| RuntimeStatus {
        availability,
        version,
        detail,
    };
    let Some(path) = find() else {
        return (
            status(
                RuntimeAvailability::Missing,
                None,
                Some(
                    "Install OpenCode 2 from opencode.ai, sign in to a provider, then try again."
                        .to_string(),
                ),
            ),
            None,
        );
    };
    let environment = search_directories().join(":");
    let output = run(&path, &["--version"], Duration::from_secs(5), Some(&environment));
    let Some(version) = output.as_deref().and_then(logic::parse_version) else {
        return (
            status(
                RuntimeAvailability::Failed,
                None,
                Some(format!("Could not run {path} --version.")),
            ),
            None,
        );
    };
    if !logic::is_supported_version(&version) {
        return (
            status(
                RuntimeAvailability::Outdated,
                Some(version),
                Some(format!(
                    "Mdow needs OpenCode {} or newer. Run `opencode upgrade`.",
                    logic::MINIMUM_MAJOR_VERSION
                )),
            ),
            None,
        );
    }
    (
        status(RuntimeAvailability::Available, Some(version.clone()), None),
        Some(Resolved { path, version }),
    )
}

/// A fresh per-launch password for the private server, never written to disk.
pub fn make_password() -> Result<String, String> {
    let mut bytes = [0u8; 24];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut random| random.read_exact(&mut bytes))
        .map_err(|err| format!("Could not generate a password: {err}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn pid_file() -> PathBuf {
    glib::user_cache_dir().join("mdow-linux").join("opencode-server.pid")
}

fn recorded_pid() -> Option<i32> {
    std::fs::read_to_string(pid_file()).ok()?.trim().parse().ok()
}

fn record_pid(pid: i32) {
    let file = pid_file();
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(mut out) = std::fs::File::create(file) {
        let _ = write!(out, "{pid}");
    }
}

fn clear_pid(pid: i32) {
    if recorded_pid() == Some(pid) {
        let _ = std::fs::remove_file(pid_file());
    }
}

fn parent_of(pid: i32) -> Option<i32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // The command name can hold spaces and brackets, so fields count from the last `)`.
    stat.rsplit_once(')')?.1.split_whitespace().nth(1)?.parse().ok()
}

/// Whether a command line is the server Mdow starts, with exactly the flags it passes.
fn is_mdow_server(command_line: &str) -> bool {
    command_line.contains("opencode")
        && command_line.contains("serve")
        && command_line.contains("--hostname=127.0.0.1")
        && command_line.contains("--port=0")
}

/// Stops a server recorded by an earlier Mdow that crashed or was killed. It has to still be
/// that server, and whatever Mdow started it has to be gone.
fn stop_orphan() {
    let Some(pid) = recorded_pid().filter(|pid| *pid > 1) else { return };
    let _ = std::fs::remove_file(pid_file());
    let Ok(raw) = std::fs::read(format!("/proc/{pid}/cmdline")) else { return };
    let command_line = String::from_utf8_lossy(&raw).replace('\0', " ");
    if !is_mdow_server(&command_line) {
        return;
    }
    let parent = parent_of(pid)
        .and_then(|parent| std::fs::read_to_string(format!("/proc/{parent}/comm")).ok())
        .unwrap_or_default();
    if parent.trim() == "mdow-linux" {
        return;
    }
    signal_group(pid);
}

fn signal_group(pid: i32) {
    // SAFETY: plain signal calls; the server was started as its own process group's leader.
    unsafe {
        if libc::getpgid(pid) == pid {
            libc::kill(-pid, libc::SIGTERM);
        } else {
            libc::kill(pid, libc::SIGTERM);
        }
    }
}

/// The running server, for the signal handler below.
static SERVER_PID: AtomicI32 = AtomicI32::new(0);

extern "C" fn stop_server_on_signal(number: libc::c_int) {
    let pid = SERVER_PID.load(Ordering::SeqCst);
    // SAFETY: `kill`, `signal` and `raise` are async-signal-safe.
    unsafe {
        if pid > 1 {
            libc::kill(-pid, libc::SIGTERM);
        }
        libc::signal(number, libc::SIG_DFL);
        libc::raise(number);
    }
}

/// On SIGTERM, SIGINT or SIGHUP, stops the server, then lets the signal end Mdow as before. After
/// a crash or SIGKILL, the next start stops the orphan instead.
fn install_signal_handlers() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        for number in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
            // SAFETY: the handler only makes async-signal-safe calls.
            unsafe {
                libc::signal(number, stop_server_on_signal as *const () as libc::sighandler_t);
            }
        }
    });
}

pub struct Server {
    pub url: String,
    pub password: String,
    pid: i32,
}

impl Server {
    pub fn stop(&self) {
        signal_group(self.pid);
    }
}

/// Starts the server and waits for it to say where it's listening. `exited` is called once when
/// the process ends, whether stopped or crashed. Blocks; call it off the main thread.
pub fn start(binary: &str, exited: impl FnOnce() + Send + 'static) -> Result<Server, String> {
    stop_orphan();
    install_signal_handlers();
    let password = make_password()?;
    let mut command = Command::new(binary);
    command
        .args(["serve", "--hostname=127.0.0.1", "--port=0"])
        .env("PATH", search_directories().join(":"))
        .env("OPENCODE_SERVER_PASSWORD", &password)
        .current_dir(glib::home_dir())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    detach(&mut command);
    let mut child = command.spawn().map_err(|err| err.to_string())?;
    let pid = child.id() as i32;
    SERVER_PID.store(pid, Ordering::SeqCst);
    record_pid(pid);

    // Keep draining both pipes after start-up so the server never blocks on a full buffer.
    let (lines, incoming) = mpsc::channel::<String>();
    let pipes: Vec<Box<dyn Read + Send>> = vec![
        Box::new(child.stdout.take().ok_or("OpenCode's output is not available.")?),
        Box::new(child.stderr.take().ok_or("OpenCode's output is not available.")?),
    ];
    for pipe in pipes {
        let lines = lines.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(pipe).lines().map_while(Result::ok) {
                let _ = lines.send(line);
            }
        });
    }
    drop(lines);
    let (exit_sender, exit_receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let code = child.wait().ok().and_then(|status| status.code());
        clear_pid(pid);
        let _ = SERVER_PID.compare_exchange(pid, 0, Ordering::SeqCst, Ordering::SeqCst);
        let _ = exit_sender.send(code);
        exited();
    });

    let mut output = String::new();
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        match incoming.recv_timeout(Duration::from_millis(100)) {
            Ok(line) => {
                output.push_str(&line);
                output.push('\n');
                if let Some(url) = logic::parse_listening_url(&output) {
                    std::thread::spawn(move || for _ in incoming {});
                    return Ok(Server { url, password, pid });
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                std::thread::sleep(Duration::from_millis(100));
            }
        }
        if let Ok(code) = exit_receiver.try_recv() {
            let code = code.unwrap_or(-1);
            let lines: Vec<&str> = output.trim().lines().collect();
            let detail = lines[lines.len().saturating_sub(3)..].join("\n");
            return Err(if detail.is_empty() {
                format!("OpenCode exited ({code})")
            } else {
                format!("OpenCode exited ({code}): {detail}")
            });
        }
        if std::time::Instant::now() > deadline {
            signal_group(pid);
            return Err("OpenCode did not start within 20 seconds.".to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passwords_are_long_and_fresh() {
        let password = make_password().unwrap();
        assert_eq!(password.len(), 48);
        assert!(password.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(password, make_password().unwrap());
    }

    #[test]
    fn only_recognises_mdows_own_server() {
        assert!(is_mdow_server("/x/opencode serve --hostname=127.0.0.1 --port=0"));
        assert!(!is_mdow_server("/x/opencode serve --port=4096"));
        assert!(!is_mdow_server("vim opencode-notes.md"));
    }

    #[test]
    fn runs_commands_and_gives_up_on_slow_ones() {
        assert_eq!(
            run("/bin/sh", &["-c", "printf hi"], Duration::from_secs(5), None).as_deref(),
            Some("hi")
        );
        assert_eq!(run("/bin/sh", &["-c", "exit 3"], Duration::from_secs(5), None), None);
        assert_eq!(run("/bin/sh", &["-c", "sleep 30"], Duration::from_millis(100), None), None);
    }
}
