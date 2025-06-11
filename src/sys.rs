use std::collections::HashMap;
use std::env;
use std::ffi::OsString;
use std::process::{Command, Stdio};
use tokio::process::{Child, Command as TokioCommand};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub fn pid() -> u32 {
    std::process::id()
}

pub fn getenv(key: &str) -> Option<String> {
    env::var(key).ok()
}

pub fn setenv(key: &str, value: &str) -> crate::Result<()> {
    env::set_var(key, value);
    Ok(())
}

pub fn unsetenv(key: &str) -> crate::Result<()> {
    env::remove_var(key);
    Ok(())
}

pub fn environ() -> HashMap<String, String> {
    env::vars().collect()
}

pub fn hostname() -> crate::Result<String> {
    match hostname::get() {
        Ok(name) => Ok(name.to_string_lossy().to_string()),
        Err(e) => Err(format!("Failed to get hostname: {}", e).into()),
    }
}

#[derive(Debug)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub hostname: String,
    pub uptime: u64,
    pub load_average: (f64, f64, f64),
    pub memory: MemoryInfo,
}

#[derive(Debug)]
pub struct MemoryInfo {
    pub total: u64,
    pub available: u64,
    pub used: u64,
    pub free: u64,
}

impl SystemInfo {
    pub async fn get() -> crate::Result<Self> {
        Ok(Self {
            os: env::consts::OS.to_string(),
            arch: env::consts::ARCH.to_string(),
            hostname: hostname().unwrap_or_else(|_| "unknown".to_string()),
            uptime: get_uptime().await?,
            load_average: get_load_average().await?,
            memory: get_memory_info().await?,
        })
    }
}

async fn get_uptime() -> crate::Result<u64> {
    #[cfg(unix)]
    {
        use std::time::SystemTime;
        match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(duration) => Ok(duration.as_secs()),
            Err(_) => Ok(0),
        }
    }
    
    #[cfg(not(unix))]
    {
        Ok(0) // Placeholder for non-Unix systems
    }
}

async fn get_load_average() -> crate::Result<(f64, f64, f64)> {
    #[cfg(unix)]
    {
        // This is a simplified implementation
        // For more accurate load average, you'd use system-specific APIs
        Ok((0.0, 0.0, 0.0))
    }
    
    #[cfg(not(unix))]
    {
        Ok((0.0, 0.0, 0.0))
    }
}

async fn get_memory_info() -> crate::Result<MemoryInfo> {
    #[cfg(unix)]
    {
        // This is a simplified implementation
        // For accurate memory info, you'd parse /proc/meminfo on Linux
        // or use system-specific APIs on other Unix systems
        Ok(MemoryInfo {
            total: 0,
            available: 0,
            used: 0,
            free: 0,
        })
    }
    
    #[cfg(not(unix))]
    {
        Ok(MemoryInfo {
            total: 0,
            available: 0,
            used: 0,
            free: 0,
        })
    }
}

pub struct Process {
    child: Option<Child>,
}

impl Process {
    pub async fn spawn(program: &str, args: &[&str]) -> crate::Result<Self> {
        let mut cmd = TokioCommand::new(program);
        cmd.args(args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        cmd.stdin(Stdio::piped());
        
        let child = cmd.spawn()?;
        
        Ok(Self { child: Some(child) })
    }

    pub async fn spawn_shell(command: &str) -> crate::Result<Self> {
        #[cfg(unix)]
        let child = TokioCommand::new("sh")
            .arg("-c")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;
        
        #[cfg(windows)]
        let child = TokioCommand::new("cmd")
            .arg("/C")
            .arg(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;
        
        Ok(Self { child: Some(child) })
    }

    pub fn id(&self) -> Option<u32> {
        self.child.as_ref()?.id()
    }

    pub async fn wait(&mut self) -> crate::Result<ProcessOutput> {
        let child = self.child.take().ok_or("Process already waited on")?;
        let output = child.wait_with_output().await?;
        
        Ok(ProcessOutput {
            status: output.status.code().unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
        })
    }

    pub async fn kill(&mut self) -> crate::Result<()> {
        if let Some(ref mut child) = self.child {
            child.kill().await?;
        }
        Ok(())
    }

    pub async fn write_stdin(&mut self, data: &[u8]) -> crate::Result<()> {
        if let Some(ref mut child) = self.child {
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(data).await?;
                stdin.flush().await?;
            }
        }
        Ok(())
    }

    pub async fn read_stdout(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        if let Some(ref mut child) = self.child {
            if let Some(stdout) = child.stdout.as_mut() {
                let n = stdout.read(buf).await?;
                return Ok(n);
            }
        }
        Ok(0)
    }

    pub async fn read_stderr(&mut self, buf: &mut [u8]) -> crate::Result<usize> {
        if let Some(ref mut child) = self.child {
            if let Some(stderr) = child.stderr.as_mut() {
                let n = stderr.read(buf).await?;
                return Ok(n);
            }
        }
        Ok(0)
    }
}

#[derive(Debug)]
pub struct ProcessOutput {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl ProcessOutput {
    pub fn stdout_string(&self) -> String {
        String::from_utf8_lossy(&self.stdout).to_string()
    }

    pub fn stderr_string(&self) -> String {
        String::from_utf8_lossy(&self.stderr).to_string()
    }

    pub fn success(&self) -> bool {
        self.status == 0
    }
}

// Utility functions for common operations
pub async fn exec(program: &str, args: &[&str]) -> crate::Result<ProcessOutput> {
    let mut process = Process::spawn(program, args).await?;
    process.wait().await
}

pub async fn exec_shell(command: &str) -> crate::Result<ProcessOutput> {
    let mut process = Process::spawn_shell(command).await?;
    process.wait().await
}

pub async fn which(program: &str) -> crate::Result<String> {
    let output = exec_shell(&format!("which {}", program)).await?;
    if output.success() {
        Ok(output.stdout_string().trim().to_string())
    } else {
        Err(format!("Program '{}' not found", program).into())
    }
}

// Signal handling (Unix only)
#[cfg(unix)]
pub mod signal {
    use nix::sys::signal::{Signal, kill};
    use nix::unistd::Pid;
    
    pub fn send_signal(pid: u32, signal: i32) -> crate::Result<()> {
        let pid = Pid::from_raw(pid as i32);
        let signal = Signal::try_from(signal)
            .map_err(|e| format!("Invalid signal: {}", e))?;
        
        kill(pid, signal)
            .map_err(|e| format!("Failed to send signal: {}", e))?;
        
        Ok(())
    }
    
    pub fn terminate(pid: u32) -> crate::Result<()> {
        send_signal(pid, Signal::SIGTERM as i32)
    }
    
    pub fn kill_process(pid: u32) -> crate::Result<()> {
        send_signal(pid, Signal::SIGKILL as i32)
    }
    
    pub fn interrupt(pid: u32) -> crate::Result<()> {
        send_signal(pid, Signal::SIGINT as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid() {
        let pid = pid();
        assert!(pid > 0);
    }

    #[test]
    fn test_env_operations() {
        let key = "RUST_ECO_TEST_VAR";
        let value = "test_value";
        
        // Set environment variable
        setenv(key, value).unwrap();
        assert_eq!(getenv(key), Some(value.to_string()));
        
        // Unset environment variable
        unsetenv(key).unwrap();
        assert_eq!(getenv(key), None);
    }

    #[test]
    fn test_environ() {
        let env_vars = environ();
        assert!(!env_vars.is_empty());
        
        // PATH should exist on most systems
        assert!(env_vars.contains_key("PATH"));
    }

    #[tokio::test]
    async fn test_system_info() {
        let info = SystemInfo::get().await.unwrap();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
    }

    #[tokio::test]
    async fn test_exec() {
        // Test simple command
        let output = exec("echo", &["hello"]).await.unwrap();
        assert!(output.success());
        assert_eq!(output.stdout_string().trim(), "hello");
    }

    #[tokio::test]
    async fn test_exec_shell() {
        let output = exec_shell("echo hello world").await.unwrap();
        assert!(output.success());
        assert_eq!(output.stdout_string().trim(), "hello world");
    }

    #[tokio::test]
    async fn test_process_spawn() {
        let mut process = Process::spawn("echo", &["test"]).await.unwrap();
        let output = process.wait().await.unwrap();
        
        assert!(output.success());
        assert_eq!(output.stdout_string().trim(), "test");
    }

    #[tokio::test]
    async fn test_process_interactive() {
        let mut process = Process::spawn_shell("cat").await.unwrap();
        
        // Write to stdin
        process.write_stdin(b"hello\n").await.unwrap();
        
        // Read from stdout
        let mut buf = [0u8; 1024];
        let n = process.read_stdout(&mut buf).await.unwrap();
        let output = String::from_utf8_lossy(&buf[..n]);
        
        // Kill the process
        process.kill().await.unwrap();
        
        assert!(output.contains("hello"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_signal_operations() {
        use signal::*;
        
        // Start a long-running process
        let mut process = Process::spawn_shell("sleep 10").await.unwrap();
        let pid = process.id().unwrap();
        
        // Send interrupt signal
        interrupt(pid).unwrap();
        
        // Wait for process to exit
        let output = process.wait().await.unwrap();
        // Process should have been interrupted (non-zero exit code)
        assert!(!output.success());
    }

    #[tokio::test]
    async fn test_which() {
        // Test with a command that should exist on most systems
        match which("echo").await {
            Ok(path) => {
                assert!(path.contains("echo"));
            }
            Err(_) => {
                // Some minimal systems might not have 'which'
                println!("'which' command not available or 'echo' not found");
            }
        }
    }
}