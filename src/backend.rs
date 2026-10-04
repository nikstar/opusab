use crate::error::{cancelled, fail};
use anyhow::{Context, Result, ensure};
use serde::Serialize;
use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

static CANCEL: AtomicBool = AtomicBool::new(false);
pub fn cancel() {
    CANCEL.store(true, Ordering::SeqCst);
}
pub fn reset_cancel() {
    CANCEL.store(false, Ordering::SeqCst);
}
pub fn is_cancelled() -> bool {
    CANCEL.load(Ordering::SeqCst)
}
pub fn install_signals() -> Result<()> {
    ctrlc::set_handler(cancel)?;
    Ok(())
}

pub fn find(name: &str) -> Result<PathBuf> {
    let env_key = format!("OPUSAB_{}", name.to_ascii_uppercase());
    if let Some(p) = env::var_os(&env_key) {
        let p = PathBuf::from(p);
        ensure!(p.is_file(), "{env_key} does not point to a file");
        return Ok(p);
    }
    let mut roots = Vec::new();
    if let Some(p) = env::var_os("OPUSAB_RUNTIME") {
        roots.push(PathBuf::from(p));
    }
    if let Ok(p) = env::current_exe().and_then(|p| p.canonicalize())
        && let Some(dir) = p.parent()
    {
        roots.push(dir.join("runtime"));
        roots.push(dir.join("../lib/opusab/runtime"));
    }
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    for root in &roots {
        for p in [
            root.join(format!("bin/{name}{suffix}")),
            root.join(format!("{name}{suffix}")),
            root.join(format!("freac.app/Contents/MacOS/{name}")),
            root.join(format!("freac/{name}{suffix}")),
        ] {
            if p.is_file() {
                return Ok(p);
            }
        }
    }
    for dir in env::split_paths(&env::var_os("PATH").unwrap_or_default()) {
        let p = dir.join(format!("{name}{suffix}"));
        if p.is_file() {
            return Ok(p);
        }
    }
    if cfg!(target_os = "macos") {
        for p in [
            PathBuf::from(format!("/Applications/freac.app/Contents/MacOS/{name}")),
            PathBuf::from(format!("/opt/homebrew/bin/{name}")),
            PathBuf::from(format!("/usr/local/bin/{name}")),
        ] {
            if p.is_file() {
                return Ok(p);
            }
        }
    }
    Err(fail(
        "backend_missing",
        format!(
            "Cannot find {name}. Use the bundled distribution, set {env_key}, or add it to PATH."
        ),
    ))
}
#[derive(Clone, Debug, Serialize)]
pub struct Progress {
    pub schema_version: u32,
    pub event: String,
    pub stage: String,
    pub message: String,
    pub elapsed_seconds: f64,
}
pub fn event(stage: &str, message: impl Into<String>, elapsed: f64) -> Progress {
    Progress {
        schema_version: 1,
        event: "progress".into(),
        stage: stage.into(),
        message: message.into(),
        elapsed_seconds: elapsed,
    }
}

/// Drain both pipes concurrently, bound retained diagnostics, and kill the whole child group on cancellation.
pub fn run(command: &mut Command, stage: &str, progress: &mut dyn FnMut(Progress)) -> Result<()> {
    if is_cancelled() {
        return Err(cancelled());
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().context("Could not start audio backend")?;
    let (tx, rx) = mpsc::channel();
    let readers: Vec<Box<dyn Read + Send>> = vec![
        Box::new(child.stdout.take().unwrap()),
        Box::new(child.stderr.take().unwrap()),
    ];
    let threads: Vec<_> = readers
        .into_iter()
        .map(|mut stream| {
            let tx = tx.clone();
            thread::spawn(move || {
                let mut b = [0; 2048];
                loop {
                    match stream.read(&mut b) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if tx
                                .send(String::from_utf8_lossy(&b[..n]).to_string())
                                .is_err()
                            {
                                break;
                            }
                        }
                    }
                }
            })
        })
        .collect();
    drop(tx);
    let start = Instant::now();
    let mut last = Instant::now();
    let mut log = String::new();
    let status = loop {
        for data in rx.try_iter() {
            log.push_str(&data);
            if log.len() > 32768 {
                let mut n = log.len() - 16384;
                while !log.is_char_boundary(n) {
                    n += 1;
                }
                log.drain(..n);
            }
        }
        if is_cancelled() {
            #[cfg(unix)]
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGTERM);
            }
            #[cfg(not(unix))]
            {
                let _ = child.kill();
            }
            let deadline = Instant::now() + Duration::from_secs(2);
            while Instant::now() < deadline {
                if child.try_wait()?.is_some() {
                    break;
                }
                thread::sleep(Duration::from_millis(50));
            }
            #[cfg(unix)]
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.kill();
            let _ = child.wait();
            for t in threads {
                let _ = t.join();
            }
            return Err(cancelled());
        }
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if last.elapsed() >= Duration::from_secs(1) {
            progress(event(
                stage,
                "Encoding audiobook",
                start.elapsed().as_secs_f64(),
            ));
            last = Instant::now();
        }
        thread::sleep(Duration::from_millis(100));
    };
    for t in threads {
        let _ = t.join();
    }
    for data in rx.try_iter() {
        log.push_str(&data);
    }
    if !status.success() {
        return Err(fail(
            "backend_failed",
            format!("Audio backend exited with {status}: {}", log.trim()),
        ));
    }
    Ok(())
}
pub fn encode(
    sources: &[PathBuf],
    output: &Path,
    bitrate: u32,
    jobs: usize,
    progress: &mut dyn FnMut(Progress),
) -> Result<()> {
    let mut cmd = Command::new(find("freaccmd")?);
    cmd.args(["-e", "opus"]);
    if jobs > 1 {
        cmd.arg("--superfast");
    }
    cmd.arg(format!("--threads={jobs}"))
        .arg("-o")
        .arg(output)
        .args([
            "--bitrate",
            &bitrate.to_string(),
            "--comp",
            "10",
            "--framesize",
            "20",
        ]);
    cmd.args(sources);
    run(&mut cmd, "encoding", progress)?;
    ensure!(
        fs::metadata(output).is_ok_and(|m| m.len() > 0),
        "Encoder did not produce an output file"
    );
    Ok(())
}
pub fn prepare_channels(
    source: &Path,
    output: &Path,
    channels: u32,
    progress: &mut dyn FnMut(Progress),
) -> Result<()> {
    let mut cmd = Command::new(find("ffmpeg")?);
    cmd.args(["-v", "error", "-nostdin", "-i"])
        .arg(source)
        .args([
            "-map",
            "0:a:0",
            "-map_metadata",
            "-1",
            "-map_chapters",
            "-1",
            "-ac",
            &channels.to_string(),
            "-c:a",
            "flac",
            "-compression_level",
            "0",
        ])
        .arg(output);
    run(&mut cmd, "preparing_channels", progress)
}

#[derive(Serialize)]
pub struct ToolStatus {
    pub name: String,
    pub path: Option<PathBuf>,
    pub available: bool,
}
pub fn doctor() -> Vec<ToolStatus> {
    ["freaccmd", "ffprobe", "ffmpeg"]
        .iter()
        .map(|name| {
            let path = find(name).ok();
            ToolStatus {
                name: (*name).into(),
                available: path.is_some(),
                path,
            }
        })
        .collect()
}
