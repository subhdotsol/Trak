use trak_analysis::{caller_tree, callers};
use trak_core::{Error, Repository};
use trak_rust::functions;

pub fn run(name: &str, format: trak_output::Format, tree: bool) -> Result<(), Error> {
    let repo = Repository::discover()?;
    let all = functions(&repo)?;
    let found = callers(&all, name);

    if tree {
        println!("{name}");
        print!("{}", caller_tree(&all, name));
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
