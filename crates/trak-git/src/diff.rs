use std::path::Path;

use trak_core::Result;

use crate::git;

/// Working-tree diff (unstaged changes).
pub fn diff(repo: &Path) -> Result<String> {
    git(repo, &["diff"])
}

/// Working-tree diff including staged changes.
pub fn diff_cached(repo: &Path) -> Result<String> {
    git(repo, &["diff", "--cached"])
}
