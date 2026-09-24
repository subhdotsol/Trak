//! Text search across a repository, powered by the ripgrep engine.

use std::io;
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use grep_regex::RegexMatcherBuilder;
use grep_searcher::{MmapChoice, Searcher, SearcherBuilder, Sink, SinkMatch};
use ignore::{DirEntry, WalkBuilder, WalkState};
use regex::escape;
use trak_core::{Error, Repository, Result, SOURCE_EXTENSIONS};

/// Cap on returned hits, matching the previous line-scan implementation.
const MAX_HITS: usize = 200;

/// Case-insensitive literal search over every source file in the repo.
///
/// The `ignore` walker performs gitignore-aware, parallel directory traversal
/// and `grep-searcher` runs the same core matching loop that ripgrep uses, so
/// large repositories such as the Rust compiler index quickly.
pub fn search(repo: &Repository, query: &str) -> Result<Vec<(String, usize, String)>> {
    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(true)
        .build(&escape(query))
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
            if !is_file || !is_source_path(&path) || shared.is_full() {
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
