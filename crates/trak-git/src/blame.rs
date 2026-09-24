use std::path::Path;

use trak_core::Result;

use crate::git;

/// Blame a single line of a file.
pub fn blame_line(repo: &Path, file: &Path, line: usize) -> Result<String> {
    let spec = format!("{line},{line}");
    git(
        repo,
        &[
            "blame",
            "-L",
            &spec,
            "--",
            file.to_str().unwrap_or_default(),
        ],
    )
}

/// Blame an entire file.
pub fn blame_file(repo: &Path, file: &Path) -> Result<String> {
    git(repo, &["blame", "--", file.to_str().unwrap_or_default()])
}
