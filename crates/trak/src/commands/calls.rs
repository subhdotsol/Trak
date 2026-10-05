use trak_analysis::{callee_tree, function_by_name};
use trak_core::{Error, Repository};
use trak_rust::functions;

pub fn run(name: &str, format: trak_output::Format, tree: bool) -> Result<(), Error> {
    let repo = Repository::discover()?;
    let all = functions(&repo)?;
    let f = function_by_name(&all, name)
        .ok_or_else(|| Error::NotFound(format!("function `{name}`")))?;

    if tree {
        println!("{name}");
        print!("{}", callee_tree(&all, name));
        return Ok(());
    }

    let mut report = trak_output::Report::new(format!("calls of `{name}`"));
    report.summary(format!("{} call(s)", f.calls.len()));
    report.hit(
        repo.display_path(&f.file),
        f.line,
        f.signature.clone(),
        Some("definition"),
        Some(f.file.display().to_string()),
    );
    report.node(f.name.clone(), f.signature.clone(), Some("fn"));
    for call in &f.calls {
        report.node(call.clone(), call.clone(), Some("fn"));
        report.edge(f.name.clone(), call.clone());
    }
    print!("{}", report.render(format));
    Ok(())
}
