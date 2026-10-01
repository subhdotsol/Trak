use trak_analysis::{callee_tree, caller_tree, callers, function_by_name};
use trak_core::{Function, Result};
use trak_editor::Editor;

use super::session::Session;
use crate::commands::relevant::find_message;

/// How many matches the list shows before it stops.
const MAX_SHOWN: usize = 20;

/// How many lines of a function body `Show source` prints.
const MAX_SOURCE_LINES: usize = 60;

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
        Screen::Detail(function) => render_detail(session, function),
    }
}

/// Decide what `input` means on `screen`.
pub fn advance(session: &mut Session, screen: &Screen, input: &str) -> Result<Step> {
    match screen {
        Screen::Query => query_input(session, input),
        Screen::Matches => matches_input(session, input),
        Screen::Detail(function) => detail_input(session, function, input),
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

fn render_detail(session: &mut Session, function: &Function) -> Result<String> {
    let (repo, functions) = session.parsed()?;
    let shown = repo.display_path(&function.file);
    let calling: Vec<String> = callers(functions, &function.name)
        .iter()
        .map(|caller| caller.name.clone())
        .collect();

    Ok(format!(
        "\n{}()\n\nLocation\n    {shown}:{}-{}\n\nCALLERS\n{}\nCALLEES\n{}\n\
         [1] Show source     [4] Trace upward\n\
         [2] Show callers    [5] Trace downward\n\
         [3] Show callees    [6] Open in editor\n\
         [b] Back            [q] Quit\n",
        function.name,
        function.line,
        function.end_line,
        level(&calling),
        level(&function.calls),
    ))
}

/// One level of a call graph as box-drawing lines. Empty renders as `(none)` so
/// the screen keeps its shape between functions.
fn level(names: &[String]) -> String {
    if names.is_empty() {
        return "    └─ (none)\n".to_string();
    }
    let mut out = String::new();
    let total = names.len();
    for (index, name) in names.iter().enumerate() {
        let branch = if index + 1 == total {
            "└─"
        } else {
            "├─"
        };
        out.push_str(&format!("    {branch} {name}\n"));
    }
    out
}

fn detail_input(session: &mut Session, function: &Function, input: &str) -> Result<Step> {
    match input {
        "q" | "quit" => Ok(Step::Exit),
        "b" => Ok(Step::Render(if session.matches.is_empty() {
            Screen::Query
        } else {
            Screen::Matches
        })),
        "1" => Ok(Step::Note(source_of(session, function)?)),
        "2" => {
            let (_, functions) = session.parsed()?;
            let names: Vec<String> = callers(functions, &function.name)
                .iter()
                .map(|caller| caller.name.clone())
                .collect();
            Ok(Step::Note(if names.is_empty() {
                format!("nothing calls {}", function.name)
            } else {
                names.join("\n")
            }))
        }
        "3" => Ok(Step::Note(if function.calls.is_empty() {
            format!("{} calls nothing in this repository", function.name)
        } else {
            function.calls.join("\n")
        })),
        "4" => {
            let (_, functions) = session.parsed()?;
            let tree = caller_tree(functions, &function.name);
            Ok(Step::Note(if tree.is_empty() {
                format!("nothing calls {}", function.name)
            } else {
                format!("{}\n{tree}", function.name)
            }))
        }
        "5" => {
            let (_, functions) = session.parsed()?;
            let tree = callee_tree(functions, &function.name);
            Ok(Step::Note(if tree.is_empty() {
                format!("{} calls nothing in this repository", function.name)
            } else {
                format!("{}\n{tree}", function.name)
            }))
        }
        "6" => Ok(Step::Note(open_in_editor(session, function)?)),
        _ => Ok(Step::Note(
            "pick 1-6, b to go back, or q to quit".to_string(),
        )),
    }
}

/// The function's own lines, numbered, capped so a large body cannot flood the
/// terminal.
fn source_of(session: &mut Session, function: &Function) -> Result<String> {
    let (repo, _) = session.parsed()?;
    let shown = repo.display_path(&function.file);
    let contents = match std::fs::read_to_string(&function.file) {
        Ok(contents) => contents,
        Err(err) => return Ok(format!("cannot read {shown}: {err}")),
    };
    let lines: Vec<&str> = contents.lines().collect();
    let start = function.line.saturating_sub(1);
    let end = function.end_line.min(lines.len());
    if start >= end {
        return Ok(format!("{} has no source to show", function.name));
    }

    let mut out = String::new();
    for (offset, text) in lines[start..end].iter().take(MAX_SOURCE_LINES).enumerate() {
        out.push_str(&format!("{:>5}  {text}\n", function.line + offset));
    }
    let span = end - start;
    if span > MAX_SOURCE_LINES {
        out.push_str(&format!("… {} more lines\n", span - MAX_SOURCE_LINES));
    }
    Ok(out)
}

fn open_in_editor(session: &mut Session, function: &Function) -> Result<String> {
    let Some(editor) = Editor::detect() else {
        return Ok(
            "no editor found (set TRAK_EDITOR, $EDITOR, or install code/zed/nvim/vim)".to_string(),
        );
    };
    let (repo, _) = session.parsed()?;
    let shown = repo.display_path(&function.file);
    let target = function.file.display().to_string();
    Ok(match editor.open(&target, function.line) {
        Ok(()) => format!("opened {shown}:{} in {}", function.line, editor.name()),
        Err(err) => format!("failed to open editor: {err}"),
    })
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
    fn detail_of(s: &mut Session, name: &str) -> Screen {
        match advance(s, &Screen::Query, name).unwrap() {
            Step::Render(screen @ Screen::Detail(_)) => screen,
            other => panic!("expected Detail({name}), got {other:?}"),
        }
    }

    #[test]
    fn the_detail_screen_shows_the_span_and_both_directions() {
        let mut s = session();
        let screen = detail_of(&mut s, "read_name");
        let text = render(&mut s, &screen).unwrap();
        assert!(text.contains("read_name()"), "{text}");
        assert!(text.contains("17-23"), "{text}");
        assert!(text.contains("CALLERS"), "{text}");
        assert!(text.contains("setup"), "{text}");
        assert!(text.contains("CALLEES"), "{text}");
    }

    #[test]
    fn a_function_with_no_repository_callees_shows_none() {
        let mut s = session();
        let screen = detail_of(&mut s, "read_name");
        let text = render(&mut s, &screen).unwrap();
        let callees = text.split("CALLEES").nth(1).expect("a CALLEES section");
        assert!(callees.contains("(none)"), "{text}");
    }

    #[test]
    fn the_detail_screen_lists_the_menu() {
        let mut s = session();
        let screen = detail_of(&mut s, "setup");
        let text = render(&mut s, &screen).unwrap();
        for item in [
            "[1] Show source",
            "[2] Show callers",
            "[3] Show callees",
            "[4] Trace upward",
            "[5] Trace downward",
            "[6] Open in editor",
            "[b] Back",
            "[q] Quit",
        ] {
            assert!(text.contains(item), "missing {item:?} in {text}");
        }
    }
    fn note(s: &mut Session, screen: &Screen, input: &str) -> String {
        match advance(s, screen, input).unwrap() {
            Step::Note(msg) => msg,
            other => panic!("expected a note, got {other:?}"),
        }
    }

    #[test]
    fn show_source_prints_numbered_lines_of_the_function() {
        let mut s = session();
        let screen = detail_of(&mut s, "read_name");
        let text = note(&mut s, &screen, "1");
        assert!(text.contains("fn read_name"), "{text}");
        assert!(
            text.contains("   17"),
            "numbering should start at 17: {text}"
        );
        assert!(!text.contains("fn log_empty"), "ran past the span: {text}");
    }

    #[test]
    fn show_callers_names_the_caller() {
        let mut s = session();
        let screen = detail_of(&mut s, "read_name");
        assert!(note(&mut s, &screen, "2").contains("setup"));
    }

    #[test]
    fn show_callees_reports_an_empty_set_in_words() {
        let mut s = session();
        let screen = detail_of(&mut s, "read_name");
        let text = note(&mut s, &screen, "3");
        assert!(text.contains("nothing in this repository"), "{text}");
    }

    #[test]
    fn show_callees_lists_them_when_there_are_some() {
        let mut s = session();
        let screen = detail_of(&mut s, "setup");
        let text = note(&mut s, &screen, "3");
        assert!(text.contains("read_name"), "{text}");
        assert!(text.contains("log_empty"), "{text}");
    }

    #[test]
    fn trace_upward_reaches_the_entry_point() {
        let mut s = session();
        let screen = detail_of(&mut s, "read_name");
        let text = note(&mut s, &screen, "4");
        assert!(text.contains("setup"), "{text}");
        assert!(text.contains("main"), "{text}");
    }

    #[test]
    fn trace_downward_walks_the_callees() {
        let mut s = session();
        let screen = detail_of(&mut s, "main");
        let text = note(&mut s, &screen, "5");
        assert!(text.contains("setup"), "{text}");
        assert!(text.contains("read_name"), "{text}");
    }

    #[test]
    fn an_unknown_key_explains_the_menu() {
        let mut s = session();
        let screen = detail_of(&mut s, "setup");
        assert!(note(&mut s, &screen, "99").contains("pick 1-6"));
    }

    #[test]
    fn b_goes_back_to_the_match_list_when_there_is_one() {
        let mut s = session();
        advance(&mut s, &Screen::Query, "failed to read input").unwrap();
        let screen = match advance(&mut s, &Screen::Matches, "1").unwrap() {
            Step::Render(screen) => screen,
            other => panic!("{other:?}"),
        };
        assert!(matches!(
            advance(&mut s, &screen, "b").unwrap(),
            Step::Render(Screen::Matches)
        ));
    }

    #[test]
    fn b_goes_back_to_the_query_when_there_is_no_list() {
        let mut s = session();
        let screen = detail_of(&mut s, "setup");
        assert!(matches!(
            advance(&mut s, &screen, "b").unwrap(),
            Step::Render(Screen::Query)
        ));
    }

    #[test]
    fn q_exits_from_the_detail_screen() {
        let mut s = session();
        let screen = detail_of(&mut s, "setup");
        assert!(matches!(advance(&mut s, &screen, "q").unwrap(), Step::Exit));
    }

    #[test]
    fn unreadable_source_is_a_note() {
        let mut s = session();
        let mut function = match detail_of(&mut s, "setup") {
            Screen::Detail(function) => *function,
            other => panic!("{other:?}"),
        };
        function.file = std::path::PathBuf::from("/nonexistent/gone.rs");
        let screen = Screen::Detail(Box::new(function));
        let text = note(&mut s, &screen, "1");
        assert!(text.contains("cannot read"), "{text}");
    }
}
