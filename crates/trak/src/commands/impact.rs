use std::collections::HashMap;

use trak_analysis::impact;
use trak_core::{Error, Repository};
use trak_rust::functions;

pub fn run(name: &str, format: trak_output::Format) -> Result<(), Error> {
    let repo = Repository::discover()?;
    let all = functions(&repo)?;
    let affected = impact(&all, name);
    if affected.is_empty() {
        println!("changing `{name}` affects nothing else");
        return Ok(());
    }

    let map: HashMap<&str, &_> = all.iter().map(|f| (f.name.as_str(), f)).collect();
    let mut report = trak_output::Report::new(format!("impact of changing `{name}`"));
    report.summary(format!("{} affected function(s)", affected.len()));
    for item in &affected {
        if let Some(f) = map.get(item.as_str()) {
            report.hit(
                repo.display_path(&f.file),
                f.line,
                f.signature.clone(),
                Some("affected"),
                Some(f.file.display().to_string()),
            );
        } else {
            report.hit("", 0, item.clone(), Some("affected"), None);
        }
    }
    print!("{}", report.render(format));
    Ok(())
}
