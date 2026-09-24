use std::fs;

use trak_core::{Repository, Result, SourceFiles, is_source_file};
use trak_rust::symbols;

const MAX_DEPTH: usize = 3;

pub fn run() -> Result<()> {
    let repo = Repository::discover()?;
    let root = repo.root();

    let files: Vec<_> = SourceFiles::new(root).collect();
    let all = symbols(&repo)?;
    let functions = trak_rust::functions(&repo)?;

    let root_name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string());

    println!("{root_name}");
    println!(
        "{} source files · {} symbols · {} functions",
        files.len(),
        all.len(),
        functions.len()
    );
    println!();

    render(root, "", 0);
    Ok(())
}

fn render(dir: &std::path::Path, prefix: &str, depth: usize) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .into_iter()
        .flat_map(|rd| rd.flatten())
        .map(|e| e.path())
        .filter(|p| {
            let name = p
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default();
            !trak_core::IGNORED_DIRS.contains(&name.as_ref())
        })
        .collect();
    entries.sort();

    for (index, entry) in entries.iter().enumerate() {
        let last = index + 1 == entries.len();
        let branch = if last { "└── " } else { "├── " };
        let child_prefix = format!("{prefix}{}", if last { "    " } else { "│   " });

        if entry.is_dir() {
            let count = SourceFiles::new(entry).count();
            let name = entry
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default();
            println!("{prefix}{branch}{name}/ ({count} files)");
            if depth < MAX_DEPTH {
                render(entry, &child_prefix, depth + 1);
            }
        } else if is_source_file(entry) {
            let name = entry
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default();
            println!("{prefix}{branch}{name}");
        }
    }
}
