use trak_core::{Function, Result};

use super::session::Session;

/// Where the session currently is.
pub enum Screen {
    /// Asking what to look for.
    Query,
    /// Showing the numbered result set in `Session::matches`.
    Matches,
    /// Showing one function in detail.
    Detail(Box<Function>),
}

/// What the loop should do with the input it just read.
#[derive(Debug)]
pub enum Step {
    /// Move to this screen and redraw it.
    Render(Screen),
    /// Print this and stay put. An empty string just re-prompts.
    Note(String),
    /// Leave the session.
    Exit,
}

impl std::fmt::Debug for Screen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Screen::Query => f.write_str("Query"),
            Screen::Matches => f.write_str("Matches"),
            Screen::Detail(function) => write!(f, "Detail({})", function.name),
        }
    }
}

/// Draw `screen`.
pub fn render(session: &mut Session, screen: &Screen) -> Result<String> {
    match screen {
        Screen::Query => Ok(format!(
            "\nRepository: {} ({} files)\nWhat are you looking for?\n",
            session.name(),
            session.files()
        )),
        Screen::Matches => Ok(String::new()),
        Screen::Detail(_) => Ok(String::new()),
    }
}

/// Decide what `input` means on `screen`.
pub fn advance(session: &mut Session, screen: &Screen, input: &str) -> Result<Step> {
    match screen {
        Screen::Query => query_input(session, input),
        Screen::Matches => Ok(Step::Note(String::new())),
        Screen::Detail(_) => Ok(Step::Note(String::new())),
    }
}

fn query_input(_session: &mut Session, input: &str) -> Result<Step> {
    if input == "q" || input == "quit" {
        return Ok(Step::Exit);
    }
    if input.is_empty() {
        return Ok(Step::Note(String::new()));
    }
    Ok(Step::Note(String::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::interactive::testing::fixture_copy;

    fn session() -> Session {
        Session::in_dir(&fixture_copy("messages")).unwrap()
    }

    #[test]
    fn the_query_screen_names_the_repository_and_file_count() {
        let mut s = session();
        let text = render(&mut s, &Screen::Query).unwrap();
        assert!(text.contains("Repository: messages (1 files)"), "{text}");
        assert!(text.contains("What are you looking for?"), "{text}");
    }

    #[test]
    fn q_exits_from_the_query_screen() {
        let mut s = session();
        assert!(matches!(
            advance(&mut s, &Screen::Query, "q").unwrap(),
            Step::Exit
        ));
        assert!(matches!(
            advance(&mut s, &Screen::Query, "quit").unwrap(),
            Step::Exit
        ));
    }

    #[test]
    fn empty_input_just_reprompts() {
        let mut s = session();
        match advance(&mut s, &Screen::Query, "").unwrap() {
            Step::Note(msg) => assert!(msg.is_empty(), "got {msg:?}"),
            other => panic!("expected a silent note, got {other:?}"),
        }
    }
}
