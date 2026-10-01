mod architecture;
mod blame;
mod callers;
mod calls;
mod diff;
mod error;
mod function;
mod history;
mod impact;
mod log;
mod open;
mod panic;
mod path;
mod refs;
pub(crate) mod relevant;
mod search;
mod symbol;

use crate::cli::Commands;
use trak_core::Result;

/// Dispatch a parsed CLI command to its handler.
pub fn run(command: Commands) -> Result<()> {
    match command {
        Commands::Search { query, format } => search::run(&query, format.into()),
        Commands::Symbol { name, format } => symbol::run(&name, format.into()),
        Commands::Fn { name, format } => function::run(&name, format.into()),
        Commands::Callers { name, format, tree } => callers::run(&name, format.into(), tree),
        Commands::Calls { name, format, tree } => calls::run(&name, format.into(), tree),
        Commands::Path { from, to, format } => path::run(&from, &to, format.into()),
        Commands::Refs { name, format } => refs::run(&name, format.into()),
        Commands::Impact { name, format } => impact::run(&name, format.into()),
        Commands::Architecture => architecture::run(),
        Commands::History { name } => history::run(&name),
        Commands::Blame { loc } => blame::run(&loc),
        Commands::Diff => diff::run(),
        Commands::Relevant { query } => relevant::run(&query),
        Commands::Log { message } => log::run(&message),
        Commands::Error { message } => error::run(&message),
        Commands::Panic { message } => panic::run(&message),
        Commands::Open { name } => open::run(&name),
    }
}
