use std::path::PathBuf;

use trak_analysis::{find_path, function_by_name};
use trak_core::{Error, Function, Repository};
use trak_rust::functions;

use super::relevant::find_message;

/// One end of a path query: a function, plus the message that resolved it when
/// the endpoint was given as log or error text rather than a function name.
struct Endpoint {
    name: String,
    origin: Option<(PathBuf, usize, String)>,
}

/// Resolve an endpoint to the functions a path may start or end at. A name the
/// repository defines wins; anything else is read as a message and mapped to
/// the functions that emit it, so `trak path main "failed to read input"` finds
/// the route to the log call site.
fn endpoints(repo: &Repository, all: &[Function], query: &str) -> Result<Vec<Endpoint>, Error> {
    if function_by_name(all, query).is_some() {
        return Ok(vec![Endpoint {
            name: query.to_string(),
            origin: None,
        }]);
    }

    let matches = find_message(repo, all, query);
    if let Some((file, line, _)) = matches.outside.first() {
        return Err(Error::Other(format!(
            "`{query}` is at {}:{}, outside any function",
            repo.display_path(file),
            line
        )));
    }
    if matches.emissions.is_empty() {
        return Err(Error::NotFound(format!("function or message `{query}`")));
    }
    Ok(matches
        .emissions
        .into_iter()
        .map(|hit| Endpoint {
            name: hit.function.name,
            origin: Some((hit.file, hit.line, hit.text)),
        })
        .collect())
}

pub fn run(from: &str, to: &str, format: trak_output::Format) -> Result<(), Error> {
    let repo = Repository::discover()?;
    let all = functions(&repo)?;

    let starts = endpoints(&repo, &all, from)?;
    let ends = endpoints(&repo, &all, to)?;

    let mut chains: Vec<Vec<String>> = Vec::new();
    for start in &starts {
        for end in &ends {
            if let Some(path) = find_path(&all, &start.name, &end.name)
                && !chains.contains(&path)
            {
                chains.push(path);
            }
        }
    }
    if chains.is_empty() {
        println!("no call path from `{from}` to `{to}`");
        return Ok(());
    }
    chains.sort_by(|a, b| a.len().cmp(&b.len()).then(a.cmp(b)));

    let mut report = trak_output::Report::new(format!("path from `{from}` to `{to}`"));
    report.summary(match chains.as_slice() {
        [chain] => format!("{} hop(s)", chain.len() - 1),
        many => format!(
            "{} path(s), longest {} hop(s)",
            many.len(),
            many.iter().map(Vec::len).max().unwrap_or(1) - 1
        ),
    });
    for end in &ends {
        if let Some((file, line, text)) = &end.origin {
            report.hit(
                repo.display_path(file),
                *line,
                text.clone(),
                Some("message"),
                Some(file.display().to_string()),
            );
        }
    }
    for chain in chains {
        report.chain(chain);
    }
    print!("{}", report.render(format));
    Ok(())
}
