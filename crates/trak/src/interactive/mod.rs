//! The interactive session: `trak` with no subcommand.

mod screen;
mod session;
#[cfg(test)]
mod testing;

use std::io::{self, BufRead, Write};

use trak_core::Result;

use screen::{Screen, Step, advance, render};
use session::Session;

/// The prompt. Every screen uses the same one, so it takes no argument.
const PROMPT: &str = "> ";

/// Run the interactive session until the user leaves or stdin ends.
pub fn run() -> Result<()> {
    let mut session = Session::new()?;
    let mut screen = Screen::Query;
    let mut redraw = true;

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    loop {
        if redraw {
            print!("{}", render(&mut session, &screen)?);
        }
        print!("{PROMPT}");
        // The prompt has no newline, so it has to be pushed explicitly.
        io::stdout().flush().ok();

        let Some(line) = lines.next() else { break };
        // A message pasted out of a latin-1 log is not valid UTF-8. That is a
        // bad line, not a reason to end the session and throw away the parse.
        let line = match line {
            Ok(line) => line,
            Err(_) => {
                println!("that input was not valid UTF-8");
                redraw = false;
                continue;
            }
        };

        match advance(&mut session, &screen, line.trim())? {
            Step::Render(next) => {
                screen = next;
                redraw = true;
            }
            Step::Note(message) => {
                if !message.is_empty() {
                    println!("{message}");
                }
                redraw = false;
            }
            Step::Exit => break,
        }
    }

    println!();
    Ok(())
}
