//! A bounded local subprocess adapter, not an OS sandbox.
use std::{
    io::{self, Read},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Copy)]
pub struct RunOptions {
    pub timeout: Duration,
    /// Maximum bytes retained independently for stdout and stderr.
    pub max_stream_bytes: usize,
}

#[derive(Debug)]
pub struct Capture {
    pub bytes: Vec<u8>,
    pub total_bytes: u64,
    pub truncated: bool,
}

#[derive(Debug)]
pub struct RunResult {
    pub stdout: Capture,
    pub stderr: Capture,
    pub exit_code: i32,
    pub timed_out: bool,
    pub elapsed: Duration,
}

fn capture(mut input: impl Read, maximum: usize) -> io::Result<Capture> {
    let mut bytes = Vec::new();
    let mut total_bytes = 0u64;
    let mut buffer = [0u8; 8192];
    loop {
        let amount = input.read(&mut buffer)?;
        if amount == 0 { break; }
        total_bytes = total_bytes.saturating_add(amount as u64);
        let retained = maximum.saturating_sub(bytes.len()).min(amount);
        bytes.extend_from_slice(&buffer[..retained]);
    }
    Ok(Capture { truncated: total_bytes > bytes.len() as u64, bytes, total_bytes })
}

/// Captures both streams independently and drains over-limit output rather
/// than deadlocking the child. On Unix, a timed-out child and members of its
/// process group are signaled; processes that leave the group can survive.
pub fn run(program: &str, args: &[String], options: RunOptions) -> io::Result<RunResult> {
    if options.timeout.is_zero() || options.max_stream_bytes == 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "run resource limits must be positive"));
    }
    let start = Instant::now();
    let mut command = Command::new(program);
    command.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn()?;
    let stdout = child.stdout.take().ok_or_else(|| io::Error::other("missing stdout pipe"))?;
    let stderr = child.stderr.take().ok_or_else(|| io::Error::other("missing stderr pipe"))?;
    let (tx, rx) = mpsc::channel();
    for (index, stream) in [
        (0usize, Box::new(stdout) as Box<dyn Read + Send>),
        (1usize, Box::new(stderr) as Box<dyn Read + Send>),
    ] {
        let tx = tx.clone();
        let cap = options.max_stream_bytes;
        thread::spawn(move || { let _ = tx.send((index, capture(stream, cap))); });
    }
    drop(tx);

    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                terminate(&mut child);
                let _ = child.wait();
                return Err(error);
            }
        }
        if start.elapsed() >= options.timeout {
            timed_out = true;
            terminate(&mut child);
            break child.wait()?;
        }
        thread::sleep(Duration::from_millis(20));
    };

    // A descendant may inherit a pipe and hold it open after the direct child
    // exits. Bound the join via a channel instead of waiting forever.
    let mut captured = [None, None];
    for _ in 0..2 {
        let (index, result) = rx.recv_timeout(Duration::from_secs(2))
            .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "output pipe remained open after child exit"))?;
        captured[index] = Some(result?);
    }
    Ok(RunResult {
        stdout: captured[0].take().expect("stdout captured"),
        stderr: captured[1].take().expect("stderr captured"),
        exit_code: status.code().unwrap_or(1),
        timed_out,
        elapsed: start.elapsed(),
    })
}

fn terminate(child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        if let Ok(pid) = i32::try_from(child.id()) {
            // The child was spawned as its own group leader and is not reaped yet.
            if unsafe { libc::killpg(pid, libc::SIGKILL) } == 0 { return; }
        }
    }
    let _ = child.kill();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_retention_but_drains_entire_stream() {
        let actual = capture(&b"abcdefgh"[..], 3).unwrap();
        assert_eq!(actual.bytes, b"abc");
        assert_eq!(actual.total_bytes, 8);
        assert!(actual.truncated);
    }

    #[test]
    fn retains_raw_bytes_including_invalid_utf8() {
        let input = [b'a', 0xff, 0x00, b'b'];
        let actual = capture(&input[..], 20).unwrap();
        assert_eq!(actual.bytes, input);
        assert!(!actual.truncated);
    }

    #[cfg(unix)]
    #[test]
    fn subprocess_preserves_stdout_stderr_and_nonzero_status() {
        let args = vec!["-c".to_owned(), "printf output; printf errors >&2; exit 7".to_owned()];
        let result = run("sh", &args, RunOptions {
            timeout: Duration::from_secs(5),
            max_stream_bytes: 1024,
        }).unwrap();
        assert_eq!(result.exit_code, 7);
        assert_eq!(result.stdout.bytes, b"output");
        assert_eq!(result.stderr.bytes, b"errors");
        assert!(!result.timed_out);
    }

    #[cfg(unix)]
    #[test]
    fn subprocess_overflow_retains_partial_data_and_reports_truncation() {
        let args = vec!["-c".to_owned(), "printf 1234567890; exit 0".to_owned()];
        let result = run("sh", &args, RunOptions {
            timeout: Duration::from_secs(5),
            max_stream_bytes: 3,
        }).unwrap();
        assert_eq!(result.stdout.bytes, b"123");
        assert_eq!(result.stdout.total_bytes, 10);
        assert!(result.stdout.truncated);
    }

    #[cfg(unix)]
    #[test]
    fn timeout_reaps_a_descendant_holding_output_pipes() {
        let args = vec!["-c".to_owned(), "sleep 30 & wait".to_owned()];
        let result = run("sh", &args, RunOptions {
            timeout: Duration::from_millis(100),
            max_stream_bytes: 1024,
        }).unwrap();
        assert!(result.timed_out);
        assert!(result.elapsed < Duration::from_secs(2));
    }

    #[test]
    fn rejects_zero_limits() {
        let options = RunOptions { timeout: Duration::ZERO, max_stream_bytes: 1 };
        assert_eq!(run("should-not-launch", &[], options).unwrap_err().kind(), io::ErrorKind::InvalidInput);
    }
}
