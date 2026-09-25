use std::collections::HashSet;

use trak_analysis::callers;
use trak_core::{Error, Repository};
use trak_rust::functions;

pub fn run(name: &str, format: trak_output::Format, tree: bool) -> Result<(), Error> {
    let repo = Repository::discover()?;
    let all = functions(&repo)?;
    let found = callers(&all, name);

    if tree {
        println!("{name}");
        let mut visited = HashSet::new();
        print_tree(&all, name, "", &mut visited);
        return Ok(());
    }

    if found.is_empty() {
        println!("no callers of `{name}`");
        return Ok(());
    }

    let mut report = trak_output::Report::new(format!("callers of `{name}`"));
    report.summary(format!("{} caller(s)", found.len()));
    for f in &found {
        report.hit(
            repo.display_path(&f.file),
            f.line,
            f.signature.clone(),
            Some("caller"),
            Some(f.file.display().to_string()),
        );
        report.node(f.name.clone(), f.signature.clone(), Some("fn"));
        report.edge(f.name.clone(), name.to_string());
    }
    print!("{}", report.render(format));
    Ok(())
}

fn print_tree(
    functions: &[trak_core::Function],
    name: &str,
    prefix: &str,
    visited: &mut HashSet<String>,
) {
    let callers = callers(functions, name);
    let pending: Vec<_> = callers
        .iter()
        .filter(|c| visited.insert(c.name.clone()))
        .collect();
    let total = pending.len();
    for (i, caller) in pending.into_iter().enumerate() {
        let last = i + 1 == total;
        let branch = if last { "└─ " } else { "├─ " };
        println!("{prefix}{branch}{}", caller.name);
        let child_prefix = format!("{prefix}{}", if last { "   " } else { "│  " });
        print_tree(functions, &caller.name, &child_prefix, visited);
    }
}
