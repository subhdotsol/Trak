use trak_core::{Error, Repository};
use trak_search::search;

pub fn run(query: &str, format: trak_output::Format) -> Result<(), Error> {
    let repo = Repository::discover()?;
    let hits = search(&repo, query)?;
    if hits.is_empty() {
        return Err(Error::NotFound(format!("`{query}`")));
    }

    let mut report = trak_output::Report::new(format!("search `{query}`"));
    report.summary(format!("{} hit(s)", hits.len()));
    for (file, line, text) in &hits {
        report.hit(
            repo.display_path(std::path::Path::new(file)),
            *line,
            text.trim(),
            None,
            Some(file.clone()),
        );
    }
    print!("{}", report.render(format));
    Ok(())
}
