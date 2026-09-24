use trak_core::{Error, Repository};
use trak_rust::symbols;

pub fn run(name: &str, format: trak_output::Format) -> Result<(), Error> {
    let repo = Repository::discover()?;
    let all = symbols(&repo)?;
    let matches: Vec<_> = all.iter().filter(|s| s.name == name).collect();
    if matches.is_empty() {
        return Err(Error::NotFound(format!("symbol `{name}`")));
    }

    let mut report = trak_output::Report::new(format!("symbol `{name}`"));
    for s in &matches {
        report.hit(
            repo.display_path(&s.file),
            s.line,
            format!("{} {}", s.kind, s.name),
            Some(s.kind.label()),
            Some(s.file.display().to_string()),
        );
    }
    print!("{}", report.render(format));
    Ok(())
}
