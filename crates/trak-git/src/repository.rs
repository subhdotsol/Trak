use std::path::Path;

use trak_core::Result;

use crate::git;

/// Short repository status (`git status --short`).
pub fn status(repo: &Path) -> Result<String> {
    git(repo, &["status", "--short"])
}

/// The current branch name.
pub fn branch(repo: &Path) -> Result<String> {
    git(repo, &["branch", "--show-current"])
}

/// The latest commit hash (short form).
pub fn head(repo: &Path) -> Result<String> {
    git(repo, &["rev-parse", "--short", "HEAD"])
}
