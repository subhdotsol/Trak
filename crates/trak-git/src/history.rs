use std::path::Path;

use trak_core::Result;

use crate::git;

/// Commits that touched a symbol or function, newest first.
pub fn history(repo: &Path, name: &str) -> Result<String> {
    git(repo, &["log", "--oneline", "-S", name])
}
