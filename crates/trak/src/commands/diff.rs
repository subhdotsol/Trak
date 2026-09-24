use trak_core::{Error, Repository};
use trak_git::diff as git_diff;

pub fn run() -> Result<(), Error> {
    let repo = Repository::discover()?;
    let output = git_diff::diff(repo.root())?;
    if output.trim().is_empty() {
        println!("no diff (clean working tree)");
    } else {
        print!("{output}");
    }
    Ok(())
}
