//! Shared harness for running the `trak` binary against fixture repos.
//!
//! Each run copies the fixture into a fresh temp directory so traversal
//! cannot escape into the workspace's own `.git`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

/// Absolute path of a named fixture under `tests/fixtures/rust/`.
pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/rust")
        .join(name)
}

/// Run `trak args...` against a fresh copy of the named fixture.
pub fn run_in(dir: &str, args: &[&str]) -> Output {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let work = std::env::temp_dir().join(format!("trak-test-{}-{}-{id}", std::process::id(), dir));
    let _ = std::fs::remove_dir_all(&work);
    copy_dir(&fixture(dir), &work).expect("failed to copy fixture into temp dir");

    Command::new(env!("CARGO_BIN_EXE_trak"))
        .args(args)
        .current_dir(&work)
        .output()
        .expect("failed to run trak binary")
}

/// Run `trak args...` against a fresh copy of the named fixture, feeding
/// `input` on stdin. `wait_with_output` closes stdin first, so the child sees
/// EOF after the script.
///
/// This module is compiled into every integration test binary, and only the
/// interactive tests drive stdin, so it is dead code in the others.
#[allow(dead_code)]
pub fn run_with_input(dir: &str, args: &[&str], input: &str) -> Output {
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let work = std::env::temp_dir().join(format!("trak-test-{}-{}-{id}", std::process::id(), dir));
    let _ = std::fs::remove_dir_all(&work);
    copy_dir(&fixture(dir), &work).expect("failed to copy fixture into temp dir");

    let mut child = Command::new(env!("CARGO_BIN_EXE_trak"))
        .args(args)
        .current_dir(&work)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn trak binary");

    child
        .stdin
        .as_mut()
        .expect("stdin was piped")
        .write_all(input.as_bytes())
        .expect("failed to write the script to stdin");

    child.wait_with_output().expect("failed to run trak binary")
}

fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let target = dst.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

pub fn success(output: &Output) -> bool {
    output.status.success()
}

pub fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}
