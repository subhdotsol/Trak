use std::path::Path;

use trak_core::{Error, Repository};
use trak_editor::Editor;

pub fn run(name: &str) -> Result<(), Error> {
    let repo = Repository::discover()?;
    let all_symbols = trak_rust::symbols(&repo)?;
    let all = trak_rust::functions(&repo)?;

    // A Rust identifier cannot contain `/`, `\`, or `.`, so a value with any of
    // those is a file path rather than a symbol name. Open it directly.
    let looks_like_path = name.contains('/') || name.contains('\\') || name.contains('.');

    let mut location = if looks_like_path {
        resolve_file(&repo, name)
    } else {
        None
    };

    if location.is_none() {
        location = all_symbols
            .iter()
            .find(|s| s.name == name)
            .map(|s| (s.file.display().to_string(), s.line, None))
            .or_else(|| {
                all.iter()
                    .find(|f| f.name == name)
                    .map(|f| (f.file.display().to_string(), f.line, None))
            });
    }

    // A name without path markers, like `Makefile`, is still a file.
    if location.is_none() && !looks_like_path {
        location = resolve_file(&repo, name);
    }

    let Some((file, line, col)) = location else {
        return Err(Error::NotFound(format!("`{name}`")));
    };

    let editor = Editor::detect().ok_or_else(|| {
        Error::Other(
            "no editor found (set TRAK_EDITOR, $EDITOR, or install code/zed/nvim/vim)".to_string(),
        )
    })?;
    editor
        .open_at(&file, line, col)
        .map_err(|err| Error::Other(format!("failed to open editor: {err}")))?;
    println!(
        "opened {}:{line} in {}",
        repo.display_path(Path::new(&file)),
        editor.name()
    );
    Ok(())
}

/// Resolve a `path`, `path:line`, or `path:line:col` argument to an existing
/// file, relative to the current directory or the repository root.
fn resolve_file(repo: &Repository, input: &str) -> Option<(String, usize, Option<usize>)> {
    let (path, line, col) = split_location(input);
    let path = Path::new(path);
    let mut candidates: Vec<std::path::PathBuf> = Vec::new();
    if path.is_absolute() {
        candidates.push(path.to_path_buf());
    } else {
        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(cwd.join(path));
        }
        candidates.push(repo.root().join(path));
    }
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .map(|candidate| (candidate.display().to_string(), line, col))
}

/// Split `path:line` or `path:line:col` into its parts. Trailing segments that
/// are entirely digits are the location; everything else stays in the path.
fn split_location(input: &str) -> (&str, usize, Option<usize>) {
    fn rsplit_digits(s: &str) -> Option<(&str, usize)> {
        let (head, tail) = s.rsplit_once(':')?;
        if tail.is_empty() || !tail.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        Some((head, tail.parse().unwrap_or(1)))
    }

    if let Some((head, last)) = rsplit_digits(input) {
        if let Some((path, line)) = rsplit_digits(head) {
            return (path, line, Some(last));
        }
        return (head, last, None);
    }
    (input, 1, None)
}

#[cfg(test)]
mod tests {
    use super::split_location;

    #[test]
    fn a_plain_path_has_no_location() {
        assert_eq!(split_location("src/blame.rs"), ("src/blame.rs", 1, None));
    }

    #[test]
    fn a_path_with_a_line_keeps_the_line() {
        assert_eq!(
            split_location("src/blame.rs:42"),
            ("src/blame.rs", 42, None)
        );
    }

    #[test]
    fn a_path_with_line_and_column_orders_them() {
        assert_eq!(
            split_location("src/blame.rs:20:3"),
            ("src/blame.rs", 20, Some(3))
        );
    }

    #[test]
    fn a_colon_that_is_not_a_location_stays_in_the_path() {
        assert_eq!(
            split_location("src/blame.rs:note"),
            ("src/blame.rs:note", 1, None)
        );
    }
}
