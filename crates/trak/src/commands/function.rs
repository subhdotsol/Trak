use trak_core::{Error, Repository};
use trak_rust::functions;

pub fn run(name: &str, format: trak_output::Format) -> Result<(), Error> {
    let repo = Repository::discover()?;
    let all = functions(&repo)?;
    let matches: Vec<_> = all.iter().filter(|f| f.name == name).collect();
    if matches.is_empty() {
        return Err(Error::NotFound(format!("function `{name}`")));
    }

    let mut report = trak_output::Report::new(format!("fn `{name}`"));
    for f in &matches {
        report.hit(
            repo.display_path(&f.file),
            f.line,
            f.signature.clone(),
            Some("definition"),
            Some(f.file.display().to_string()),
        );
        report.node(f.name.clone(), f.signature.clone(), Some("fn"));
        for call in &f.calls {
            report.edge(f.name.clone(), call.clone());
        }
    }
    print!("{}", report.render(format));
    Ok(())
}
