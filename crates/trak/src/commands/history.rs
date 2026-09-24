use trak_core::{Error, Repository};
use trak_git::history as git_history;

pub fn run(name: &str) -> Result<(), Error> {
    let repo = Repository::discover()?;
    let output = git_history::history(repo.root(), name)?;
    if output.trim().is_empty() {
        println!("no git history mentions `{name}`");
        return Ok(());
    }
    print!("{output}");
    Ok(())
}
