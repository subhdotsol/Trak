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
