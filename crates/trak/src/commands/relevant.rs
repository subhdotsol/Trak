use std::collections::BTreeSet;
use std::path::PathBuf;

use trak_analysis::{callers, containing_function};
use trak_core::{Error, Repository};
use trak_rust::functions;
use trak_search::search;

/// Locate `query` in the codebase and expand each match to its containing
/// function and its call relationships. `heading` renames the first line so
/// the same flow serves `relevant`, `log`, `error` and `panic`.
pub fn run_context(query: &str, heading: &str) -> Result<(), Error> {
    let repo = Repository::discover()?;
    let hits = search(&repo, query)?;
    if hits.is_empty() {
        return Err(Error::NotFound(format!("`{query}`")));
    }

    let all = functions(&repo)?;
    let mut seen = BTreeSet::new();
    println!("{heading}: `{query}`");

    for (file, line, text) in &hits {
        let path = PathBuf::from(file);
        let shown = repo.display_path(&path);
        match containing_function(&all, &path, *line) {
            Some(f) => {
                let key = format!("{}:{}", f.file.display(), f.line);
                if !seen.insert(key) {
                    continue;
                }
                println!("\n{}:{}", repo.display_path(&f.file), f.line);
                println!("  {}", f.signature);

                let callers = callers(&all, &f.name);
                if !callers.is_empty() {
                    let names: Vec<&str> = callers.iter().map(|c| c.name.as_str()).collect();
                    println!("  called from: {}", names.join(", "));
                }
                if !f.calls.is_empty() {
                    println!("  calls: {}", f.calls.join(", "));
                }
                println!("  match: {shown}:{line}: {}", text.trim());
            }
            None => println!("{shown}:{line}: {}", text.trim()),
        }
    }
    Ok(())
}

pub fn run(query: &str) -> Result<(), Error> {
    run_context(query, "relevant to")
}
