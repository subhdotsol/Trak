use trak_analysis::references;
use trak_core::{Error, Repository};
use trak_rust::functions;

pub fn run(name: &str, format: trak_output::Format) -> Result<(), Error> {
    let repo = Repository::discover()?;
    let all_symbols = trak_rust::symbols(&repo)?;
    let all = functions(&repo)?;
    let found = references(&all, &all_symbols, name);
    if found.is_empty() {
        return Err(Error::NotFound(format!("references to `{name}`")));
    }

    let mut report = trak_output::Report::new(format!("references to `{name}`"));
    report.summary(format!("{} reference(s)", found.len()));
    for r in &found {
        report.hit(
            repo.display_path(&r.file),
            r.line,
            r.text.clone(),
            Some(r.kind),
            Some(r.file.display().to_string()),
        );
    }
    print!("{}", report.render(format));
    Ok(())
}
