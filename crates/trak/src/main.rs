mod cli;
mod commands;
mod interactive;

use clap::Parser;
use cli::Cli;

fn main() {
    sigpipe::reset();
    let cli = Cli::parse();

    if let Err(error) = commands::run(cli.command) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
