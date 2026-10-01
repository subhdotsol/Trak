use trak_analysis::function_by_name;
use trak_core::{Function, Result};

use super::session::Session;
use crate::commands::relevant::find_message;

/// How many matches the list shows before it stops.
const MAX_SHOWN: usize = 20;

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
        Screen::Matches => Ok(render_matches(session)),
        Screen::Detail(_) => Ok(String::new()),
    }
}

/// Decide what `input` means on `screen`.
pub fn advance(session: &mut Session, screen: &Screen, input: &str) -> Result<Step> {
    match screen {
        Screen::Query => query_input(session, input),
        Screen::Matches => matches_input(session, input),
        Screen::Detail(_) => Ok(Step::Note(String::new())),
    }
}

fn query_input(session: &mut Session, input: &str) -> Result<Step> {
    if input == "q" || input == "quit" {
        return Ok(Step::Exit);
    }
    if input.is_empty() {
        return Ok(Step::Note(String::new()));
    }

    // A name the repository defines wins, so typing a function goes straight to
    // it. This is the order `commands/path.rs` already resolves endpoints in.
    let direct = {
        let (_, functions) = session.parsed()?;
        function_by_name(functions, input).cloned()
    };
    if let Some(function) = direct {
        return Ok(Step::Render(Screen::Detail(Box::new(function))));
    }

    let found = {
        let (repo, functions) = session.parsed()?;
        find_message(repo, functions, input)
    };

    if !found.emissions.is_empty() {
        session.query = input.to_string();
        session.matches = found.emissions;
        return Ok(Step::Render(Screen::Matches));
    }

    if let Some((file, line, _)) = found.outside.first() {
        let line = *line;
        let (repo, _) = session.parsed()?;
        return Ok(Step::Note(format!(
            "`{input}` is at {}:{line}, outside any function, so nothing calls it",
            repo.display_path(file)
        )));
    }

    Ok(Step::Note(format!("nothing found for `{input}`")))
}

fn matches_input(session: &mut Session, input: &str) -> Result<Step> {
    let shown = session.matches.len().min(MAX_SHOWN);
    match input {
        "q" | "quit" => Ok(Step::Exit),
        "b" => Ok(Step::Render(Screen::Query)),
        _ => match input.parse::<usize>() {
            Ok(index) if index >= 1 && index <= shown => {
                let function = session.matches[index - 1].function.clone();
                Ok(Step::Render(Screen::Detail(Box::new(function))))
            }
            _ => Ok(Step::Note(format!(
                "pick 1-{shown}, b to go back, or q to quit"
            ))),
        },
    }
}

fn render_matches(session: &Session) -> String {
    let total = session.matches.len();
    let plural = if total == 1 { "match" } else { "matches" };
    let mut out = format!("\n{total} {plural} for `{}`\n", session.query);
    for (index, hit) in session.matches.iter().take(MAX_SHOWN).enumerate() {
        let base = hit
            .file
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| hit.file.display().to_string());
        out.push_str(&format!(
            "  [{}] {}:{}   {}()\n",
            index + 1,
            base,
            hit.line,
            hit.function.name
        ));
    }
    if total > MAX_SHOWN {
        out.push_str(&format!("  … and {} more\n", total - MAX_SHOWN));
    }
    out
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
    fn names(s: &Session) -> Vec<String> {
        s.matches.iter().map(|m| m.function.name.clone()).collect()
    }

    #[test]
    fn a_message_builds_the_match_list() {
        let mut s = session();
        let step = advance(&mut s, &Screen::Query, "failed to read input").unwrap();
        assert!(matches!(step, Step::Render(Screen::Matches)), "{step:?}");
        assert_eq!(s.matches.len(), 2, "{:?}", names(&s));
        assert_eq!(s.query, "failed to read input");
        let found = names(&s);
        assert!(found.contains(&"read_name".to_string()), "{found:?}");
        assert!(found.contains(&"log_empty".to_string()), "{found:?}");
    }

    #[test]
    fn the_match_list_is_numbered_with_file_and_line() {
        let mut s = session();
        advance(&mut s, &Screen::Query, "failed to read input").unwrap();
        let text = render(&mut s, &Screen::Matches).unwrap();
        assert!(
            text.contains("2 matches for `failed to read input`"),
            "{text}"
        );
        assert!(text.contains("[1] lib.rs:20"), "{text}");
        assert!(text.contains("read_name()"), "{text}");
    }

    #[test]
    fn a_name_wins_over_a_message_match() {
        let mut s = session();
        let step = advance(&mut s, &Screen::Query, "setup").unwrap();
        match step {
            Step::Render(Screen::Detail(function)) => assert_eq!(function.name, "setup"),
            other => panic!("expected Detail(setup), got {other:?}"),
        }
    }

    #[test]
    fn nothing_found_is_a_note() {
        let mut s = session();
        match advance(&mut s, &Screen::Query, "zzz no such text zzz").unwrap() {
            Step::Note(msg) => assert!(msg.contains("nothing found"), "{msg}"),
            other => panic!("expected a note, got {other:?}"),
        }
    }

    #[test]
    fn a_match_outside_any_function_explains_itself() {
        let mut s = session();
        match advance(&mut s, &Screen::Query, "no such user on disk").unwrap() {
            Step::Note(msg) => {
                assert!(msg.contains("outside any function"), "{msg}");
                assert!(msg.contains("lib.rs"), "{msg}");
            }
            other => panic!("expected a note, got {other:?}"),
        }
    }

    #[test]
    fn a_query_with_regex_characters_is_a_note() {
        let mut s = session();
        for pasted in ["unwrap(", "a)b", "*", "[", "\\"] {
            let step = advance(&mut s, &Screen::Query, pasted).unwrap();
            assert!(
                matches!(step, Step::Note(_) | Step::Render(_)),
                "{pasted:?} gave {step:?}"
            );
        }
    }

    #[test]
    fn selecting_an_index_opens_that_function() {
        let mut s = session();
        advance(&mut s, &Screen::Query, "failed to read input").unwrap();
        let first = s.matches[0].function.name.clone();
        match advance(&mut s, &Screen::Matches, "1").unwrap() {
            Step::Render(Screen::Detail(function)) => assert_eq!(function.name, first),
            other => panic!("expected Detail, got {other:?}"),
        }
    }

    #[test]
    fn an_index_past_the_end_is_a_note() {
        let mut s = session();
        advance(&mut s, &Screen::Query, "failed to read input").unwrap();
        match advance(&mut s, &Screen::Matches, "9").unwrap() {
            Step::Note(msg) => assert!(msg.contains("pick 1-2"), "{msg}"),
            other => panic!("expected a note, got {other:?}"),
        }
    }

    #[test]
    fn non_numeric_input_at_the_match_list_is_a_note() {
        let mut s = session();
        advance(&mut s, &Screen::Query, "failed to read input").unwrap();
        match advance(&mut s, &Screen::Matches, "banana").unwrap() {
            Step::Note(msg) => assert!(msg.contains("pick 1-2"), "{msg}"),
            other => panic!("expected a note, got {other:?}"),
        }
    }

    #[test]
    fn b_goes_back_from_the_match_list_to_the_query() {
        let mut s = session();
        advance(&mut s, &Screen::Query, "failed to read input").unwrap();
        assert!(matches!(
            advance(&mut s, &Screen::Matches, "b").unwrap(),
            Step::Render(Screen::Query)
        ));
    }

    #[test]
    fn q_exits_from_the_match_list() {
        let mut s = session();
        assert!(matches!(
            advance(&mut s, &Screen::Matches, "q").unwrap(),
            Step::Exit
        ));
    }
}
