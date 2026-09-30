use std::fs;
use std::path::{Path, PathBuf};

/// A walkable repository root with helpers shared by all trak crates.
#[derive(Debug, Clone)]
pub struct Repository {
    root: PathBuf,
}

impl Repository {
    /// Discover the repository from the current working directory by walking
    /// up until a `.git` entry is found.
    pub fn discover() -> crate::Result<Self> {
        let start = std::env::current_dir()?;
        Self::from_dir(&start)
    }

    /// Walk upward from `dir` looking for a `.git` entry. Falls back to `dir`
    /// itself when no repository is found so loose folders still work.
    pub fn from_dir(dir: &Path) -> crate::Result<Self> {
        let mut current = dir;
        loop {
            if current.join(".git").is_dir() || current.join(".git").is_file() {
                return Ok(Repository {
                    root: current.to_path_buf(),
                });
            }
            match current.parent() {
                Some(parent) => current = parent,
                None => {
                    return Ok(Repository {
                        root: dir.to_path_buf(),
                    });
                }
            }
        }
    }

    /// The absolute path of the repository root.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Render `path` relative to the repository root for display. Paths
    /// inside the repo become `./relative`; anything else keeps its absolute
    /// form.
    pub fn display_path(&self, path: &Path) -> String {
        match path.strip_prefix(&self.root) {
            Ok(relative) => {
                let shown = relative.display().to_string();
                if shown.is_empty() {
                    "./".to_string()
                } else {
                    format!("./{shown}")
                }
            }
            Err(_) => path.display().to_string(),
        }
    }
}

/// Directories that are never traversed.
pub const IGNORED_DIRS: &[&str] = &[
    ".git",
    ".git-rewrite",
    ".idea",
    ".vscode",
    ".venv",
    "__pycache__",
    "build",
    "dist",
    "node_modules",
    "target",
    "vendor",
];

/// File extensions considered source code / text.
pub const SOURCE_EXTENSIONS: &[&str] = &[
    "c", "cc", "cpp", "cs", "css", "cxx", "dart", "ex", "go", "h", "hh", "hpp", "hxx", "html",
    "java", "js", "jsx", "json", "kt", "lua", "md", "php", "pl", "py", "rb", "rs", "scala", "scss",
    "sh", "sql", "swift", "toml", "ts", "tsx", "txt", "vue", "yaml", "yml", "zig",
];

/// Deterministically list all source files under a directory, skipping
/// ignored directories.
pub struct SourceFiles {
    stack: Vec<PathBuf>,
}

impl SourceFiles {
    /// Build a new walker rooted at `root`.
    pub fn new(root: &Path) -> Self {
        let mut stack = Vec::new();
        if let Ok(entries) = fs::read_dir(root) {
            let mut dirs = entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .collect::<Vec<_>>();
            dirs.sort();
            stack.extend(dirs.into_iter().rev());
        }
        SourceFiles { stack }
    }
}

impl Iterator for SourceFiles {
    type Item = PathBuf;

    fn next(&mut self) -> Option<PathBuf> {
        while let Some(path) = self.stack.pop() {
            let file_name = path
                .file_name()
                .map(|name| name.to_string_lossy())
                .unwrap_or_default();

            if is_fixture_path(&path) {
                continue;
            }

            if path.is_dir() {
                if IGNORED_DIRS.contains(&file_name.as_ref()) {
                    continue;
                }
                if let Ok(entries) = fs::read_dir(&path) {
                    let mut dirs = entries
                        .filter_map(|entry| entry.ok())
                        .map(|entry| entry.path())
                        .collect::<Vec<_>>();
                    dirs.sort();
                    self.stack.extend(dirs.into_iter().rev());
                }
                continue;
            }

            if !is_source_file(&path) {
                continue;
            }
            return Some(path);
        }
        None
    }
}

/// Whether `path` is a file with a recognized source extension.
pub fn is_source_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| SOURCE_EXTENSIONS.contains(&ext))
        .unwrap_or(false)
}

/// Whether `path` lies inside a `tests/fixtures` tree.
///
/// Fixture trees are self-contained sample programs, so folding them into the
/// analysis merges unrelated codebases into one call graph. Matching the
/// `tests/fixtures` pair rather than a bare `fixtures` directory keeps a
/// top-level `fixtures/` of real source visible.
pub fn is_fixture_path(path: &Path) -> bool {
    let mut after_tests = false;
    for component in path.components() {
        let name = component.as_os_str().to_string_lossy();
        if after_tests && name == "fixtures" {
            return true;
        }
        after_tests = name == "tests";
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_trees_are_excluded() {
        assert!(is_fixture_path(Path::new(
            "crates/trak/tests/fixtures/rust/basic/src/main.rs"
        )));
        assert!(is_fixture_path(Path::new("tests/fixtures/a.rs")));
    }

    #[test]
    fn ordinary_paths_are_kept() {
        assert!(!is_fixture_path(Path::new("crates/trak/src/main.rs")));
        assert!(!is_fixture_path(Path::new(
            "crates/trak/tests/common/mod.rs"
        )));
    }

    #[test]
    fn a_fixtures_dir_outside_tests_is_kept() {
        assert!(!is_fixture_path(Path::new("fixtures/data/main.rs")));
        assert!(!is_fixture_path(Path::new("src/fixtures/loader.rs")));
    }

    #[test]
    fn a_copied_fixture_is_kept() {
        // The integration harness copies a fixture to a temp dir before running
        // trak, so the copy must stay visible or every integration test breaks.
        assert!(!is_fixture_path(Path::new(
            "/tmp/trak-test-1-calls-0/src/lib.rs"
        )));
    }

    /// Build a throwaway tree and return its root.
    fn scratch(name: &str, files: &[&str]) -> PathBuf {
        let root = std::env::temp_dir().join(format!("trak-core-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        for rel in files {
            let path = root.join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "fn placeholder() {}\n").unwrap();
        }
        root
    }

    #[test]
    fn the_walker_skips_fixture_trees() {
        let root = scratch(
            "walk",
            &[
                "src/main.rs",
                "tests/common/mod.rs",
                "tests/fixtures/rust/basic/src/main.rs",
            ],
        );
        let found: Vec<String> = SourceFiles::new(&root)
            .map(|p| {
                p.strip_prefix(&root)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect();
        let _ = fs::remove_dir_all(&root);

        assert!(found.contains(&"src/main.rs".to_string()), "{found:?}");
        assert!(
            found.contains(&"tests/common/mod.rs".to_string()),
            "{found:?}"
        );
        assert!(
            !found.iter().any(|f| f.contains("fixtures")),
            "fixture tree leaked into the walk: {found:?}"
        );
    }
}
