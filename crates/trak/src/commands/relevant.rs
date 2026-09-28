use std::collections::BTreeSet;
use std::path::PathBuf;

use trak_analysis::{callers, containing_function};
use trak_core::{Error, Function, Repository};
use trak_rust::functions;
use trak_search::{search, search_message};

/// A source line together with the function that emits it.
pub struct Emission {
    pub function: Function,
    pub file: PathBuf,
    pub line: usize,
    pub text: String,
}

/// Where a message was found: the functions that emit it, plus any match that
/// sits outside a function (a constant, a doc comment, a top-level statement)
/// and therefore has no call path leading to it.
pub struct MessageMatches {
    pub emissions: Vec<Emission>,
    pub outside: Vec<(PathBuf, usize, String)>,
}

/// Search for a log or error message, falling back to a placeholder-tolerant
/// pattern so a message typed as `failed to read input: {error}` still finds
/// the call site that interpolates it as `failed to read input: {}`.
fn locate(repo: &Repository, message: &str) -> Vec<(String, usize, String)> {
    match search(repo, message) {
        Ok(hits) if !hits.is_empty() => hits,
        _ => search_message(repo, message).unwrap_or_default(),
    }
}

/// Locate `message` in the source and split the matches by whether a function
/// contains them, deduplicating the functions by their definition site.
pub fn find_message(repo: &Repository, all: &[Function], message: &str) -> MessageMatches {
    let mut seen = BTreeSet::new();
    let mut matches = MessageMatches {
        emissions: Vec::new(),
        outside: Vec::new(),
    };
    for (file, line, text) in locate(repo, message) {
        let path = PathBuf::from(&file);
        let Some(function) = containing_function(all, &path, line) else {
            matches.outside.push((path, line, text));
            continue;
        };
        if !seen.insert((function.file.clone(), function.line)) {
            continue;
        }
        matches.emissions.push(Emission {
            function: function.clone(),
            file: path,
            line,
            text,
        });
    }
    matches
}

/// Locate `query` in the codebase and expand each match to its containing
/// function and its call relationships. `heading` renames the first line so
/// the same flow serves `relevant`, `log`, `error` and `panic`.
pub fn run_context(query: &str, heading: &str) -> Result<(), Error> {
    let repo = Repository::discover()?;
    let hits = locate(&repo, query);
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
