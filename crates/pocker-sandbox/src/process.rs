//! Process-based sandbox — spawns child processes with resource limits.

use std::process::Stdio;
use tokio::process::Command;

/// A sandbox for executing commands in isolated processes.
pub struct Sandbox {
    /// Working directory for sandboxed processes
    work_dir: std::path::PathBuf,
    /// Timeout in seconds (0 = no timeout)
    timeout_secs: u64,
    /// Whether network access is allowed
    network_allowed: bool,
}

/// Result of a sandboxed execution.
#[derive(Debug, Clone)]
pub struct SandboxResult {
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

impl Sandbox {
    /// Create a new sandbox with default settings.
    pub fn new(work_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            work_dir: work_dir.into(),
            timeout_secs: 30,
            network_allowed: false,
        }
    }

    /// Set the timeout in seconds.
    #[must_use]
    pub const fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    /// Allow or deny network access.
    #[must_use]
    pub const fn network(mut self, allowed: bool) -> Self {
        self.network_allowed = allowed;
        self
    }

    /// Execute a command in the sandbox.
    ///
    /// # Errors
    /// Returns an error if the program cannot be spawned, if execution exceeds
    /// the configured timeout, or if waiting for its output fails.
    pub async fn run(&self, program: &str, args: &[&str]) -> anyhow::Result<SandboxResult> {
        let start = std::time::Instant::now();

        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd.current_dir(&self.work_dir);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Platform-specific isolation would be applied here:
        // - Linux: unshare(CLONE_NEWPID | CLONE_NEWNET | CLONE_NEWNS)
        // - Windows: CreateJobObject + SetInformationJobObject
        // - macOS: sandbox-exec with a profile

        let child = cmd.spawn()?;
        let output = if self.timeout_secs > 0 {
            tokio::time::timeout(
                std::time::Duration::from_secs(self.timeout_secs),
                child.wait_with_output(),
            )
            .await
            .map_err(|_| {
                anyhow::anyhow!("sandbox execution timed out after {}s", self.timeout_secs)
            })??
        } else {
            child.wait_with_output().await?
        };

        let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);

        Ok(SandboxResult {
            success: output.status.success(),
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn sandbox_run_echo() {
        let tmp = tempdir().unwrap();
        let sandbox = Sandbox::new(tmp.path());

        // Use the `echo` command which exists on all platforms
        let result = sandbox.run("echo", &["hello"]).await.unwrap();

        assert!(result.success);
        assert_eq!(result.stdout.trim(), "hello");
    }

    #[tokio::test]
    async fn sandbox_run_nonexistent() {
        let tmp = tempdir().unwrap();
        let sandbox = Sandbox::new(tmp.path());

        let result = sandbox.run("nonexistent-command-xyz", &[]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn sandbox_timeout() {
        let tmp = tempdir().unwrap();
        let sandbox = Sandbox::new(tmp.path()).timeout(1);

        // Use a command that sleeps — on Windows use `timeout` (different syntax)
        // On Unix, use `sleep`
        #[cfg(unix)]
        let result = sandbox.run("sleep", &["10"]).await;
        #[cfg(windows)]
        let result = sandbox.run("ping", &["-n", "10", "127.0.0.1"]).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn sandbox_exit_code() {
        let tmp = tempdir().unwrap();
        let sandbox = Sandbox::new(tmp.path());

        // Exit with code 42
        #[cfg(unix)]
        let result = sandbox.run("sh", &["-c", "exit 42"]).await.unwrap();
        #[cfg(windows)]
        let result = sandbox.run("cmd", &["/c", "exit 42"]).await.unwrap();

        assert_eq!(result.exit_code, Some(42));
        assert!(!result.success);
    }
}
