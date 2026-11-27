//! Running external tools (plugin commands, page captures) with a time limit, never through a shell.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub struct ProcOutput {
    pub status: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

/// What a tool may print before it is stopped; a reply that large is a fault, not a result.
pub const MAX_OUTPUT: usize = 64 << 20;
const MAX_STDERR: usize = 1 << 20;
const TIMED_OUT: &str = " timed out after ";

/// The error `run` and its kin give for a tool stopped at its time limit, as against one that failed.
pub(crate) fn timed_out(err: &str) -> bool {
    err.contains(TIMED_OUT)
}

pub fn run(program: &str, args: &[String], stdin: Option<&[u8]>, timeout: Duration) -> Result<ProcOutput, String> {
    run_limited(program, args, stdin, timeout, None, &[], MAX_OUTPUT)
}

/// In `dir`, with `env` added to the environment it inherits.
pub fn run_in(program: &str, args: &[String], stdin: Option<&[u8]>, timeout: Duration, dir: Option<&std::path::Path>, env: &[(String, String)]) -> Result<ProcOutput, String> {
    run_limited(program, args, stdin, timeout, dir, env, MAX_OUTPUT)
}

/// Runs the program and everything it starts as one group: a timeout stops them all, and output still held open by a leftover child doesn't hold up the caller past the deadline.
pub fn run_limited(program: &str, args: &[String], stdin: Option<&[u8]>, timeout: Duration, dir: Option<&std::path::Path>, env: &[(String, String)], max_output: usize) -> Result<ProcOutput, String> {
    let (mut child, group) = start(program, args, stdin, dir, env)?;
    let out_rx = drain(child.stdout.take(), max_output);
    let err_rx = drain(child.stderr.take(), MAX_STDERR);
    let deadline = Instant::now() + timeout;
    let timed_out = || format!("{program}{TIMED_OUT}{}s", timeout.as_secs());
    let status = wait_until(&mut child, &group, deadline).map_err(|e| e.unwrap_or_else(timed_out))?;
    let Ok((stdout, over)) = out_rx.recv_deadline(deadline) else {
        group.kill(&mut child);
        return Err(timed_out());
    };
    let stderr = err_rx.recv_deadline(deadline).map(|(b, _)| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
    if over {
        return Err(format!("{program} printed more than {} MB", max_output >> 20));
    }
    Ok(ProcOutput { status, stdout, stderr })
}

/// Like `run`, but the output goes to `each` as it is printed and is not kept.
pub fn run_streamed(program: &str, args: &[String], timeout: Duration, each: &mut dyn FnMut(&[u8])) -> Result<ProcOutput, String> {
    let (mut child, group) = start(program, args, None, None, &[])?;
    let err_rx = drain(child.stderr.take(), MAX_STDERR);
    let (tx, rx) = crossbeam_channel::bounded::<Vec<u8>>(8);
    if let Some(mut out) = child.stdout.take() {
        std::thread::spawn(move || {
            let mut buf = vec![0u8; 64 << 10];
            while let Ok(n) = out.read(&mut buf) {
                if n == 0 || tx.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        });
    }
    let deadline = Instant::now() + timeout;
    let timed_out = || format!("{program}{TIMED_OUT}{}s", timeout.as_secs());
    loop {
        match rx.recv_deadline(deadline) {
            Ok(chunk) => each(&chunk),
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break,
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                group.kill(&mut child);
                return Err(timed_out());
            }
        }
    }
    let status = wait_until(&mut child, &group, deadline).map_err(|e| e.unwrap_or_else(timed_out))?;
    let stderr = err_rx.recv_deadline(deadline).map(|(b, _)| String::from_utf8_lossy(&b).to_string()).unwrap_or_default();
    Ok(ProcOutput { status, stdout: Vec::new(), stderr })
}

/// The pipe a line came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pipe {
    Out,
    Err,
}

/// How a program run line by line ended, with the last of what it printed.
#[derive(Debug, Default)]
pub struct Lines {
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    /// `stop` was set and the program and everything it started were stopped.
    pub stopped: bool,
}

/// Runs the program, handing `line` each line it prints on either pipe as it comes, until it ends, `stop` is set or the deadline passes; keeps the last `keep` bytes of each pipe.
/// A carriage return ends a line too, as progress bars redraw their line with one.
pub fn run_lines(program: &str, args: &[String], stdin: Option<&[u8]>, dir: Option<&std::path::Path>, env: &[(String, String)], timeout: Duration, stop: &std::sync::atomic::AtomicBool, line: &mut dyn FnMut(Pipe, &str), keep: usize) -> Result<Lines, String> {
    let (mut child, group) = start(program, args, stdin, dir, env)?;
    let (tx, rx) = crossbeam_channel::unbounded::<(Pipe, String)>();
    let read_lines = |pipe: Option<Box<dyn Read + Send>>, which: Pipe, tx: crossbeam_channel::Sender<(Pipe, String)>| {
        std::thread::spawn(move || {
            let Some(mut p) = pipe else { return };
            let mut buf = vec![0u8; 16 << 10];
            let mut held: Vec<u8> = Vec::new();
            while let Ok(n) = p.read(&mut buf) {
                if n == 0 {
                    break;
                }
                for &b in &buf[..n] {
                    if b == b'\n' || b == b'\r' {
                        if !held.is_empty() && tx.send((which, String::from_utf8_lossy(&held).to_string())).is_err() {
                            return;
                        }
                        held.clear();
                    } else if held.len() < 64 << 10 {
                        held.push(b);
                    }
                }
            }
            if !held.is_empty() {
                let _ = tx.send((which, String::from_utf8_lossy(&held).to_string()));
            }
        })
    };
    read_lines(child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>), Pipe::Out, tx.clone());
    read_lines(child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>), Pipe::Err, tx);
    let mut out = Lines::default();
    let (mut tail_out, mut tail_err) = (std::collections::VecDeque::<String>::new(), std::collections::VecDeque::<String>::new());
    let mut take = |which: Pipe, s: String, line: &mut dyn FnMut(Pipe, &str)| {
        line(which, &s);
        let tail = if which == Pipe::Out { &mut tail_out } else { &mut tail_err };
        tail.push_back(s);
        while tail.iter().map(|l| l.len() + 1).sum::<usize>() > keep && tail.len() > 1 {
            tail.pop_front();
        }
    };
    let deadline = Instant::now() + timeout;
    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok((which, s)) => take(which, s, line),
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {}
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
        }
        if stop.load(std::sync::atomic::Ordering::Relaxed) {
            group.kill(&mut child);
            out.stopped = true;
            break;
        }
        if Instant::now() > deadline {
            group.kill(&mut child);
            return Err(format!("{program} was stopped after {}s", timeout.as_secs()));
        }
        match child.try_wait() {
            Ok(Some(s)) => {
                out.status = s.code();
                // what it printed last, while the pipes close
                let end = Instant::now() + Duration::from_secs(2);
                while let Ok((which, s)) = rx.recv_deadline(end) {
                    take(which, s, line);
                }
                break;
            }
            Ok(None) => {}
            Err(e) => {
                group.kill(&mut child);
                return Err(e.to_string());
            }
        }
    }
    out.stdout = tail_out.into_iter().collect::<Vec<_>>().join("\n");
    out.stderr = tail_err.into_iter().collect::<Vec<_>>().join("\n");
    Ok(out)
}

fn start(program: &str, args: &[String], stdin: Option<&[u8]>, dir: Option<&std::path::Path>, env: &[(String, String)]) -> Result<(std::process::Child, Group), String> {
    let mut cmd = Command::new(program);
    if let Some(d) = dir {
        cmd.current_dir(d);
    }
    cmd.envs(env.iter().map(|(k, v)| (k, v)));
    cmd.args(args).stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() }).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let mut child = cmd.spawn().map_err(|e| format!("could not start {program}: {e}"))?;
    let group = Group::of(&child);
    if let Some(data) = stdin {
        if let Some(mut si) = child.stdin.take() {
            let data = data.to_vec();
            std::thread::spawn(move || {
                let _ = si.write_all(&data);
            });
        }
    }
    Ok((child, group))
}

/// The exit code once the program ends; past the deadline the group is stopped and `Err(None)` returned.
fn wait_until(child: &mut std::process::Child, group: &Group, deadline: Instant) -> Result<Option<i32>, Option<String>> {
    loop {
        match child.try_wait() {
            Ok(Some(s)) => return Ok(s.code()),
            Ok(None) if Instant::now() > deadline => {
                group.kill(child);
                return Err(None);
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => {
                group.kill(child);
                return Err(Some(e.to_string()));
            }
        }
    }
}

/// Reads a pipe on its own thread, keeping at most `max` bytes; the flag says more was printed.
fn drain<R: Read + Send + 'static>(pipe: Option<R>, max: usize) -> crossbeam_channel::Receiver<(Vec<u8>, bool)> {
    let (tx, rx) = crossbeam_channel::bounded(1);
    std::thread::spawn(move || {
        let mut b = Vec::new();
        let mut over = false;
        if let Some(p) = pipe {
            let _ = p.take(max as u64 + 1).read_to_end(&mut b);
            if b.len() > max {
                b.truncate(max);
                over = true;
            }
        }
        let _ = tx.send((b, over));
    });
    rx
}

/// The program's process tree: a Job object on Windows, a process group on Unix.
struct Group {
    #[cfg(windows)]
    job: Option<windows::Win32::Foundation::HANDLE>,
    #[cfg(unix)]
    pid: i32,
}

impl Group {
    #[cfg(windows)]
    fn of(child: &std::process::Child) -> Group {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::System::JobObjects::{AssignProcessToJobObject, CreateJobObjectW};
        let job = unsafe { CreateJobObjectW(None, windows::core::PCWSTR::null()) }.ok().filter(|job| {
            let ok = unsafe { AssignProcessToJobObject(*job, HANDLE(child.as_raw_handle())) }.is_ok();
            if !ok {
                let _ = unsafe { CloseHandle(*job) };
            }
            ok
        });
        Group { job }
    }

    #[cfg(unix)]
    fn of(child: &std::process::Child) -> Group {
        Group { pid: child.id() as i32 }
    }

    #[cfg(not(any(windows, unix)))]
    fn of(_: &std::process::Child) -> Group {
        Group {}
    }

    fn kill(&self, child: &mut std::process::Child) {
        #[cfg(windows)]
        if let Some(job) = self.job {
            let _ = unsafe { windows::Win32::System::JobObjects::TerminateJobObject(job, 1) };
        }
        #[cfg(unix)]
        unsafe {
            libc::killpg(self.pid, libc::SIGKILL);
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}

impl Drop for Group {
    fn drop(&mut self) {
        #[cfg(windows)]
        if let Some(job) = self.job {
            let _ = unsafe { windows::Win32::Foundation::CloseHandle(job) };
        }
    }
}

/// Replace `{input}`, `{output}`, `{size}` placeholders in a command template.
pub fn expand(template: &[String], vars: &[(&str, String)]) -> Vec<String> {
    template
        .iter()
        .map(|a| {
            let mut s = a.clone();
            for (k, v) in vars {
                s = s.replace(&format!("{{{k}}}"), v);
            }
            s
        })
        .collect()
}
