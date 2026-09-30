//! Text search across a repository, powered by the ripgrep engine.

use std::io;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use grep_regex::RegexMatcherBuilder;
use grep_searcher::{MmapChoice, Searcher, SearcherBuilder, Sink, SinkMatch};
use ignore::{DirEntry, WalkBuilder, WalkState};
use regex::escape;
use trak_core::{Error, Repository, Result, SOURCE_EXTENSIONS, is_fixture_path};

/// Cap on returned hits, matching the previous line-scan implementation.
const MAX_HITS: usize = 200;

/// Case-insensitive literal search over every source file in the repo.
///
/// The `ignore` walker performs gitignore-aware, parallel directory traversal
/// and `grep-searcher` runs the same core matching loop that ripgrep uses, so
/// large repositories such as the Rust compiler index quickly.
pub fn search(repo: &Repository, query: &str) -> Result<Vec<(String, usize, String)>> {
    run(repo, &escape(query))
}

/// Case-insensitive search for a log or error message, where a `{name}`
/// placeholder matches any run of characters. Typing the message as it reads in
/// the source (`failed to read input: {error}`) still finds a call site that
/// interpolates it positionally (`failed to read input: {}`).
pub fn search_message(repo: &Repository, message: &str) -> Result<Vec<(String, usize, String)>> {
    run(repo, &message_pattern(message))
}

/// Escape `message` into a regex, widening each `{name}` placeholder to match
/// any text up to the next brace. Everything else is matched literally.
fn message_pattern(message: &str) -> String {
    let mut out = String::new();
    let mut rest = message;
    while let Some(open) = rest.find('{') {
        out.push_str(&escape(&rest[..open]));
        let Some(close) = rest[open..].find('}') else {
            out.push_str(&escape(&rest[open..]));
            return out;
        };
        out.push_str("[^{}\\n]*");
        rest = &rest[open + close + 1..];
    }
    out.push_str(&escape(rest));
    out
}

fn run(repo: &Repository, pattern: &str) -> Result<Vec<(String, usize, String)>> {
    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(true)
        .build(pattern)
        .map_err(|err| Error::Other(format!("invalid search pattern: {err}")))?;

    let shared = Shared::default();
    WalkBuilder::new(repo.root()).build_parallel().run(|| {
        Box::new(|result: std::result::Result<DirEntry, ignore::Error>| {
            let Ok(entry) = result else {
                return WalkState::Continue;
            };
            let is_file = entry
                .file_type()
                .map(|kind| kind.is_file())
                .unwrap_or(false);
            let path = entry.into_path();
            if !is_file || !is_source_path(&path) || is_fixture_path(&path) || shared.is_full() {
                return WalkState::Continue;
            }

            let mut local = Vec::new();
            let mut sink = HitSink {
                path: &path,
                hits: &mut local,
            };
            let mut searcher = SearcherBuilder::new()
                .memory_map(unsafe { MmapChoice::auto() })
                .build();
            let _ = searcher.search_path(&matcher, &path, &mut sink);
            shared.push(local);
            WalkState::Continue
        })
    });

    Ok(shared.into_hits())
}

/// Whether `path` carries a recognized source extension, without stat-ing it.
fn is_source_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| SOURCE_EXTENSIONS.contains(&ext))
}

/// Thread-safe result collection with a global hit cap. The cap is tracked on
/// an atomic so cap checks never contend on the lock during the walk.
#[derive(Default)]
struct Shared {
    hits: Mutex<Vec<(String, usize, String)>>,
    count: AtomicUsize,
}

impl Shared {
    /// Whether the result cap has been reached.
    fn is_full(&self) -> bool {
        self.count.load(Ordering::Relaxed) >= MAX_HITS
    }

    /// Append `hits`, stopping at the cap. Files may still be visited after
    /// the cap is reached but nothing beyond it is stored.
    fn push(&self, hits: Vec<(String, usize, String)>) {
        let mut all = self.hits.lock().unwrap();
        if all.len() >= MAX_HITS {
            return;
        }
        for hit in hits {
            if all.len() >= MAX_HITS {
                break;
            }
            all.push(hit);
        }
        self.count.store(all.len(), Ordering::Relaxed);
    }

    fn into_hits(self) -> Vec<(String, usize, String)> {
        self.hits.into_inner().unwrap()
    }
}

/// A `grep-searcher` sink that records one entry per matching line.
struct HitSink<'a> {
    path: &'a Path,
    hits: &'a mut Vec<(String, usize, String)>,
}

impl Sink for HitSink<'_> {
    type Error = io::Error;

    fn matched(
        &mut self,
        _searcher: &Searcher,
        mat: &SinkMatch<'_>,
    ) -> std::result::Result<bool, Self::Error> {
        if let Some(number) = mat.line_number() {
            let text = String::from_utf8_lossy(mat.bytes()).into_owned();
            self.hits
                .push((self.path.display().to_string(), number as usize, text));
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::{Repository, message_pattern, search};
    use std::fs;

    fn matches(message: &str, candidate: &str) -> bool {
        let pattern = regex::RegexBuilder::new(&message_pattern(message))
            .case_insensitive(true)
            .build()
            .expect("pattern compiles");
        pattern.is_match(candidate)
    }

    #[test]
    fn plain_messages_match_literally() {
        assert!(matches(
            "failed to read input",
            "eprintln!(\"failed to read input\")"
        ));
        assert!(!matches("failed to read input", "read the input file"));
    }

    #[test]
    fn placeholders_match_interpolations() {
        assert!(matches(
            "failed to read input: {error}",
            "failed to read input: {}"
        ));
        assert!(matches(
            "failed to read input: {error}",
            "failed to read input: {e}"
        ));
        assert!(!matches("failed to read input: {error}", "failed to read"));
    }

    #[test]
    fn unclosed_brace_stays_literal() {
        assert!(matches("cost is {x", "cost is {x"));
    }
    /// Search must skip the same fixture trees the symbol walker skips, or a hit
    /// inside one arrives with no containing function to report.
    #[test]
    fn fixture_trees_are_not_searched() {
        let root = std::env::temp_dir().join(format!("trak-search-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("tests/fixtures/rust/basic/src")).unwrap();
        fs::write(root.join("src/main.rs"), "let needle = 1;\n").unwrap();
        fs::write(
            root.join("tests/fixtures/rust/basic/src/main.rs"),
            "let needle = 2;\n",
        )
        .unwrap();

        let repo = Repository::from_dir(&root).unwrap();
        let hits = search(&repo, "needle").unwrap();
        let _ = fs::remove_dir_all(&root);

        assert_eq!(hits.len(), 1, "{hits:?}");
        assert!(!hits[0].0.contains("fixtures"), "{hits:?}");
    }
}
