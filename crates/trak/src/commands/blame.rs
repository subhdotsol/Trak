use trak_core::{Error, Repository};
use trak_git::blame;

pub fn run(loc: &str) -> Result<(), Error> {
    let repo = Repository::discover()?;
    let (file, line) = loc
        .rsplit_once(':')
        .and_then(|(file, rest)| rest.parse::<usize>().ok().map(|n| (file.to_string(), n)))
        .unwrap_or((loc.to_string(), 0));

    let relative = std::path::Path::new(&file);
    let output = if line > 0 {
        blame::blame_line(repo.root(), relative, line)?
    } else {
        blame::blame_file(repo.root(), relative)?
    };
    print!("{output}");
    Ok(())
}
