use std::path::Path;
use std::process::Command;
use std::time::Duration;

/// Run a `GitNexus` CLI command and return stdout.
pub fn run_gitnexus_command(
    project_root: &Path,
    args: &[&str],
    timeout: Duration,
) -> Result<String, GitNexusCliError> {
    let mut cmd = Command::new("npx");
    cmd.arg("gitnexus");
    cmd.args(args);
    cmd.current_dir(project_root);
    cmd.env("NO_COLOR", "1");

    let output = run_with_timeout(&mut cmd, timeout)?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(GitNexusCliError::CommandFailed { stderr })
    }
}

/// Check whether `GitNexus` CLI is available.
#[must_use]
pub fn is_gitnexus_available(project_root: &Path) -> bool {
    Command::new("npx")
        .args(["gitnexus", "--version"])
        .current_dir(project_root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn run_with_timeout(
    cmd: &mut Command,
    timeout: Duration,
) -> Result<std::process::Output, GitNexusCliError> {
    let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| GitNexusCliError::NotInstalled {
            reason: e.to_string(),
        })?;

    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return child.wait_with_output().map_err(GitNexusCliError::Io),
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    return Err(GitNexusCliError::Timeout);
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(e) => return Err(GitNexusCliError::Io(e)),
        }
    }
}

#[derive(Debug)]
pub enum GitNexusCliError {
    NotInstalled { reason: String },
    CommandFailed { stderr: String },
    Timeout,
    Io(std::io::Error),
}

impl std::fmt::Display for GitNexusCliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInstalled { reason } => write!(f, "`GitNexus` not installed: {reason}"),
            Self::CommandFailed { stderr } => write!(f, "`GitNexus` command failed: {stderr}"),
            Self::Timeout => write!(f, "`GitNexus` command timed out (10s)"),
            Self::Io(e) => write!(f, "`GitNexus` I/O error: {e}"),
        }
    }
}

impl std::error::Error for GitNexusCliError {}
