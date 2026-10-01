mod cli;
mod commands;
mod interactive;

use clap::Parser;
use cli::Cli;

fn main() {
    sigpipe::reset();
    let cli = Cli::parse();

    let result = match cli.command {
        Some(command) => commands::run(command),
        None => interactive::run(),
    };

    if let Err(error) = result {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
