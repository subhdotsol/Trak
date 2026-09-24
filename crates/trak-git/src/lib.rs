//! Git-backed history, blame and diff analysis for `trak`.

pub mod blame;
pub mod diff;
pub mod history;
pub mod repository;

use std::path::Path;

use trak_core::Result;

/// Run `git` against `repo` and return the cleaned stdout, surfacing stderr
/// on failure.
fn git(repo: &Path, args: &[&str]) -> Result<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|err| trak_core::Error::Other(format!("failed to run git: {err}")))?;

    if !output.status.success() {
        return Err(trak_core::Error::Command(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
