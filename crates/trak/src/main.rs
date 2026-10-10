mod cli;
mod commands;
mod interactive;

use std::io::IsTerminal;

use clap::{CommandFactory, Parser};
use cli::Cli;

fn main() {
    sigpipe::reset();
    let cli = Cli::parse();

    let result = match cli.command {
        Some(command) => commands::run(command),
        // Bare `trak` opens a session for a person at a terminal. Off a
        // terminal it keeps the usage error it has always printed, so a build
        // step that runs `trak` with no arguments still fails fast instead of
        // blocking on a prompt nobody will answer. `trak interactive` is the
        // explicit entry point for a script.
        None if std::io::stdin().is_terminal() => interactive::run(),
        None => {
            let _ = Cli::command().print_help();
            println!();
            std::process::exit(2);
        }
    };

    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
