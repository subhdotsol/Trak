use trak_core::{Function, Repository, Result, SourceFiles};

use crate::commands::relevant::Emission;

/// One interactive session: the repository being explored, its call graph once
/// something has needed it, and the most recent result set.
pub struct Session {
    repo: Repository,
    files: usize,
    functions: Option<Vec<Function>>,
    /// The query behind `matches`, kept so the match list can be redrawn.
    pub query: String,
    /// The emissions the last query found, kept so `b` can go back to them.
    pub matches: Vec<Emission>,
    /// Occurrences of `query` that sit outside every function. They have no
    /// call path, so they are counted rather than listed — but counted, not
    /// dropped, or the total contradicts `trak search`.
    pub outside: usize,
}

impl Session {
    /// Open a session on the repository containing the working directory.
    pub fn new() -> Result<Session> {
        Session::from_repo(Repository::discover()?)
    }

    /// Open a session rooted at `dir`. Used by the tests, which must not depend
    /// on the process's working directory.
    #[cfg(test)]
    pub fn in_dir(dir: &std::path::Path) -> Result<Session> {
        Session::from_repo(Repository::from_dir(dir)?)
    }

    fn from_repo(repo: Repository) -> Result<Session> {
        let files = SourceFiles::new(repo.root())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("rs"))
            .count();
        Ok(Session {
            repo,
            files,
            functions: None,
            query: String::new(),
            matches: Vec::new(),
            outside: 0,
        })
    }

    /// Forget the last result set, so going back to the query screen cannot
    /// later return to a list the user has abandoned.
    pub fn clear_matches(&mut self) {
        self.query.clear();
        self.matches.clear();
        self.outside = 0;
    }

    /// The repository's directory name, for the banner.
    pub fn name(&self) -> String {
        self.repo
            .root()
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.repo.root().display().to_string())
    }

    /// How many Rust files the walk found. Costs no parse.
    pub fn files(&self) -> usize {
        self.files
    }

    /// The repository and its call graph. The graph is parsed on the first call
    /// and kept for the rest of the session.
    ///
    /// Both halves are returned together because a caller usually needs the
    /// repository to render a path while holding the graph; taking them from one
    /// `&mut self` call is what keeps that from being two conflicting borrows.
    pub fn parsed(&mut self) -> Result<(&Repository, &[Function])> {
        if self.functions.is_none() {
            self.functions = Some(trak_rust::functions(&self.repo)?);
        }
        Ok((&self.repo, self.functions.as_deref().unwrap_or(&[])))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::interactive::testing::fixture_copy;

    #[test]
    fn counts_rust_files_without_parsing() {
        let session = Session::in_dir(&fixture_copy("messages")).unwrap();
        assert_eq!(session.files(), 1);
        assert_eq!(session.name(), "messages");
    }

    #[test]
    fn parses_once_and_caches() {
        let mut session = Session::in_dir(&fixture_copy("messages")).unwrap();
        let first = session.parsed().unwrap().1.len();
        assert!(first > 0, "fixture should define functions");
        let second = session.parsed().unwrap().1.len();
        assert_eq!(first, second);
    }

    #[test]
    fn parsed_hands_back_the_repository_too() {
        let mut session = Session::in_dir(&fixture_copy("messages")).unwrap();
        let (repo, functions) = session.parsed().unwrap();
        assert!(repo.root().ends_with("messages"));
        assert!(functions.iter().any(|f| f.name == "read_name"));
    }

    #[test]
    fn starts_with_no_result_set() {
        let session = Session::in_dir(&fixture_copy("messages")).unwrap();
        assert!(session.query.is_empty());
        assert!(session.matches.is_empty());
    }
}
