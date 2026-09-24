use trak_core::{Error, Repository};
use trak_editor::Editor;

pub fn run(name: &str) -> Result<(), Error> {
    let repo = Repository::discover()?;
    let all_symbols = trak_rust::symbols(&repo)?;
    let all = trak_rust::functions(&repo)?;

    let location = all_symbols
        .iter()
        .find(|s| s.name == name)
        .map(|s| (s.file.display().to_string(), s.line))
        .or_else(|| {
            all.iter()
                .find(|f| f.name == name)
                .map(|f| (f.file.display().to_string(), f.line))
        });

    let (file, line) = match location {
        Some(location) => location,
        None => return Err(Error::NotFound(format!("`{name}`"))),
    };

    let editor = Editor::detect().ok_or_else(|| {
        Error::Other(
            "no editor found (set TRAK_EDITOR, $EDITOR, or install code/zed/nvim/vim)".to_string(),
        )
    })?;
    editor
        .open(&file, line)
        .map_err(|err| Error::Other(format!("failed to open editor: {err}")))?;
    println!(
        "opened {}:{line} in {}",
        repo.display_path(std::path::Path::new(&file)),
        editor.name()
    );
    Ok(())
}
