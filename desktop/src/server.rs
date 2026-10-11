//! Runs `dadrock preview` for one project, the same way the VS Code extension
//! does: the desktop app is a window around the CLI's own web server, so the
//! two can never disagree about what a project contains.

use std::{
    collections::VecDeque,
    env,
    ffi::OsString,
    io::{BufRead, BufReader, Read},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex, OnceLock,
    },
    thread,
    time::Duration,
};

use tauri::Url;

/// `dadrock preview` prints this to stderr once it accepts connections.
const READY_PREFIX: &str = "dadrock-preview: listening on ";

/// Ports are derived from the project path, so a project gets the same origin
/// every time it is opened and the web UI's localStorage (theme, resolved
/// diagnostics) survives a restart. A busy port falls through to the next one.
const PORT_BASE: u16 = 41000;
const PORT_SPAN: u16 = 2000;
const PORT_ATTEMPTS: u16 = 20;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(20);
const STDERR_TAIL_LINES: usize = 40;

pub struct PreviewServer {
    child: Child,
    url: Url,
    stopping: Arc<AtomicBool>,
}

impl PreviewServer {
    /// Starts the server and blocks until it is listening. `on_unexpected_exit`
    /// runs if the process later ends without [`PreviewServer`] being dropped.
    pub fn start(
        root: &Path,
        on_unexpected_exit: impl FnOnce() + Send + 'static,
    ) -> Result<Self, String> {
        let dadrock = locate_dadrock()?;
        let port = free_port(root)?;

        let mut command = Command::new(&dadrock);
        command
            .arg("preview")
            .args(["--host", "127.0.0.1", "--port", &port.to_string()])
            .arg(root)
            .current_dir(root)
            .env("PATH", search_path())
            // dadrock exits when this pipe closes, which happens even if the
            // app is killed before it can stop the server itself.
            .env("DADROCK_PREVIEW_EXIT_ON_STDIN_EOF", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = command
            .spawn()
            .map_err(|err| format!("Could not start {}: {err}", dadrock.display()))?;
        let stderr = child.stderr.take().expect("stderr is piped");

        let stopping = Arc::new(AtomicBool::new(false));
        let tail = Arc::new(Mutex::new(VecDeque::with_capacity(STDERR_TAIL_LINES)));
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Url>(1);
        {
            let stopping = Arc::clone(&stopping);
            let tail = Arc::clone(&tail);
            thread::spawn(move || {
                let mut ready_tx = Some(ready_tx);
                for line in BufReader::new(stderr).lines() {
                    let Ok(line) = line else { break };
                    eprintln!("[dadrock] {line}");
                    if let Some(url) = line.strip_prefix(READY_PREFIX) {
                        if let (Some(tx), Ok(url)) = (ready_tx.take(), Url::parse(url.trim())) {
                            let _ = tx.send(url);
                        }
                    }
                    let mut tail = tail.lock().unwrap();
                    if tail.len() == STDERR_TAIL_LINES {
                        tail.pop_front();
                    }
                    tail.push_back(line);
                }
                // Dropping an unused sender tells `start` that the process
                // ended before it was ready; past that point an EOF means the
                // server died while its window was still open.
                if ready_tx.is_none() && !stopping.load(Ordering::SeqCst) {
                    on_unexpected_exit();
                }
            });
        }

        match ready_rx.recv_timeout(STARTUP_TIMEOUT) {
            Ok(url) => Ok(Self {
                child,
                url,
                stopping,
            }),
            Err(err) => {
                stopping.store(true, Ordering::SeqCst);
                let _ = child.kill();
                let status = child.wait().ok();
                let tail = tail.lock().unwrap();
                let output = tail.iter().cloned().collect::<Vec<_>>().join("\n");
                let reason = match (err, status) {
                    (mpsc::RecvTimeoutError::Timeout, _) => format!(
                        "dadrock preview did not start within {} seconds.",
                        STARTUP_TIMEOUT.as_secs()
                    ),
                    (_, Some(status)) => format!("dadrock preview exited ({status})."),
                    (_, None) => "dadrock preview exited.".to_owned(),
                };
                Err(if output.is_empty() {
                    reason
                } else {
                    format!("{reason}\n\n{output}")
                })
            }
        }
    }

    pub fn url(&self) -> &Url {
        &self.url
    }
}

impl Drop for PreviewServer {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::SeqCst);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The `dadrock` the app runs: `$DADROCK_BIN`, then a copy next to this
/// executable (bundled into the macOS and Windows apps; `/usr/bin/dadrock`
/// from the `dadrock` package on Linux), then the first one on PATH.
pub fn locate_dadrock() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("DADROCK_BIN").filter(|path| !path.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    let name = format!("dadrock{}", env::consts::EXE_SUFFIX);
    if let Some(sibling) = env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(&name)))
        .filter(|path| path.is_file())
    {
        return Ok(sibling);
    }
    env::split_paths(&search_path())
        .map(|dir| dir.join(&name))
        .find(|path| path.is_file())
        .ok_or_else(|| {
            "Could not find the dadrock command. Install it \
             (https://vincentarelbundock.github.io/dadr/dadrock/) or set DADROCK_BIN \
             to its path."
                .to_owned()
        })
}

fn free_port(root: &Path) -> Result<u16, String> {
    let preferred =
        PORT_BASE + (fnv1a(root.as_os_str().as_encoded_bytes()) % PORT_SPAN as u64) as u16;
    (0..PORT_ATTEMPTS)
        .map(|offset| PORT_BASE + (preferred - PORT_BASE + offset) % PORT_SPAN)
        .find(|port| TcpListener::bind(("127.0.0.1", *port)).is_ok())
        .ok_or_else(|| format!("No free port near {preferred} for dadrock preview."))
}

/// FNV-1a, because `DefaultHasher` makes no promise to hash the same way in
/// the next Rust release, and a moved port would forget the UI's settings.
fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// The PATH handed to `dadrock`, which in turn launches agents such as
/// `claude-agent-acp` or `codex-acp`. An app started from the Dock or a desktop
/// launcher inherits a minimal PATH rather than the one the user's shell sets
/// up, so on macOS and Linux the login shell's PATH is put in front of it.
pub fn search_path() -> OsString {
    static PATH: OnceLock<OsString> = OnceLock::new();
    PATH.get_or_init(|| {
        let inherited = env::var_os("PATH").unwrap_or_default();
        let Some(login) = login_shell_path() else {
            return inherited;
        };
        let mut dirs: Vec<PathBuf> = Vec::new();
        for dir in env::split_paths(&login).chain(env::split_paths(&inherited)) {
            if !dir.as_os_str().is_empty() && !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
        env::join_paths(dirs).unwrap_or(inherited)
    })
    .clone()
}

#[cfg(windows)]
fn login_shell_path() -> Option<OsString> {
    None
}

#[cfg(not(windows))]
fn login_shell_path() -> Option<OsString> {
    const MARKER: &str = "__DADROCK_PATH__";
    let shell = env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into());
    let mut child = Command::new(shell)
        .args(["-l", "-i", "-c"])
        .arg(format!("printf '{MARKER}%s{MARKER}' \"$PATH\""))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut output = String::new();
        let _ = stdout.read_to_string(&mut output);
        let _ = tx.send(output);
    });
    // A shell startup file that waits for input must not hang the app.
    let output = rx.recv_timeout(Duration::from_secs(5));
    let _ = child.kill();
    let _ = child.wait();
    let output = output.ok()?;
    let (_, rest) = output.split_once(MARKER)?;
    let (path, _) = rest.split_once(MARKER)?;
    (!path.is_empty()).then(|| path.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_is_stable_and_in_range() {
        let root = Path::new("/home/someone/analysis");
        let first = free_port(root).unwrap();
        assert!((PORT_BASE..PORT_BASE + PORT_SPAN).contains(&first));
        assert_eq!(fnv1a(b"abc"), 0xe71f_a219_0541_574b);
    }
}
