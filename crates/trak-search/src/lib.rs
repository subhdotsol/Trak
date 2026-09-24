//! Text search across a repository.

use std::fs;

use trak_core::{Repository, Result, SourceFiles};

/// Case-insensitive substring search over every source file in the repo.
pub fn search(repo: &Repository, query: &str) -> Result<Vec<(String, usize, String)>> {
    let needle = query.to_lowercase();
    let mut hits = Vec::new();
    for file in SourceFiles::new(repo.root()) {
        let Ok(contents) = fs::read_to_string(&file) else {
            continue;
        };
        for (index, text) in contents.lines().enumerate() {
            if text.to_lowercase().contains(&needle) && hits.len() < 200 {
                hits.push((file.display().to_string(), index + 1, text.to_string()));
            }
        }
    }
    Ok(hits)
}
