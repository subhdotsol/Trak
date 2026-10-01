//! Test helpers shared by the interactive module's unit tests.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

/// Copy the named fixture out of the repository and return the copy's path.
///
/// Two reasons the fixture cannot be used where it sits: `Repository::from_dir`
/// walks up looking for `.git` and would root at trak's own tree, and
/// `is_fixture_path` skips anything under `tests/fixtures`, so the fixture's own
/// functions would never enter the graph. The copy gets its own `.git` and a
/// path with no `tests/fixtures` in it, which fixes both.
pub fn fixture_copy(name: &str) -> PathBuf {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/rust")
        .join(name);
    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("trak-session-{}-{id}", std::process::id()));
    let work = root.join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(work.join(".git")).expect("create the fixture copy");
    copy_dir(&source, &work).expect("copy the fixture");
    work
}

fn copy_dir(source: &Path, target: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let into = target.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &into)?;
        } else {
            std::fs::copy(&path, &into)?;
        }
    }
    Ok(())
}
