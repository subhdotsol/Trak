use trak_analysis::find_path;
use trak_core::{Error, Repository};
use trak_rust::functions;

pub fn run(from: &str, to: &str, format: trak_output::Format) -> Result<(), Error> {
    let repo = Repository::discover()?;
    let all = functions(&repo)?;
    let path = find_path(&all, from, to);
    let Some(path) = path else {
        println!("no call path from `{from}` to `{to}`");
        return Ok(());
    };

    let mut report = trak_output::Report::new(format!("path from `{from}` to `{to}`"));
    report.summary(format!("{} hop(s)", path.len() - 1));
    for node in &path {
        report.node(node.clone(), node.clone(), Some("fn"));
    }
    for pair in path.windows(2) {
        report.edge(pair[0].clone(), pair[1].clone());
    }
    print!("{}", report.render(format));
    Ok(())
}
