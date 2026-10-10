mod common;

use common::{run_with_bytes, run_with_input, stdout, success};

#[test]
fn a_scripted_session_reaches_the_caller() {
    let out = run_with_input(
        "messages",
        &["interactive"],
        "failed to read input\n1\n2\nq\n",
    );
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("Repository: "), "{text}");
    assert!(
        text.contains("2 functions for `failed to read input`"),
        "{text}"
    );
    assert!(text.contains("read_name()"), "{text}");
    assert!(text.contains("CALLERS"), "{text}");
    assert!(text.contains("setup"), "{text}");
}

#[test]
fn a_script_that_ends_mid_session_exits_cleanly() {
    // No `q`: stdin simply runs out while the detail screen is up.
    let out = run_with_input("messages", &["interactive"], "failed to read input\n1\n");
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    assert!(stdout(&out).contains("read_name()"), "{}", stdout(&out));
    assert!(
        common::stderr(&out).is_empty(),
        "stderr: {}",
        common::stderr(&out)
    );
}

#[test]
fn a_function_name_goes_straight_to_its_detail() {
    let out = run_with_input("messages", &["interactive"], "setup\nq\n");
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("setup()"), "{text}");
    assert!(text.contains("Location"), "{text}");
    assert!(
        !text.contains("matches for"),
        "should not have listed: {text}"
    );
}

#[test]
fn an_existing_subcommand_is_unaffected() {
    let out = common::run_in("messages", &["fn", "read_name"]);
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    assert!(stdout(&out).contains("read_name"), "{}", stdout(&out));
    assert!(!stdout(&out).contains("What are you looking for?"));
}

#[test]
fn invalid_utf8_on_stdin_does_not_kill_the_session() {
    // An error message pasted out of a latin-1 log is exactly the workflow this
    // feature is for, and it must not end the session.
    let script = b"caf\xe9 not utf8\nsetup\nq\n";
    let out = run_with_bytes("messages", &["interactive"], script);
    assert!(
        out.status.success(),
        "exit {:?}, stderr: {}",
        out.status.code(),
        common::stderr(&out)
    );
    let text = stdout(&out);
    assert!(text.contains("not valid UTF-8"), "{text}");
    assert!(
        text.contains("setup()"),
        "the session stopped early: {text}"
    );
}

#[test]
fn bare_trak_on_a_pipe_does_not_hang() {
    // Before interactive mode, bare `trak` exited non-zero at once. A build
    // step that still runs it must not block forever waiting on a prompt.
    let out = common::run_in("messages", &[]);
    assert!(
        !out.status.success(),
        "bare trak on a pipe should not succeed"
    );
    let combined = format!("{}{}", stdout(&out), common::stderr(&out));
    assert!(combined.contains("Usage"), "{combined}");
}

#[test]
fn the_interactive_subcommand_is_the_scripted_entry_point() {
    let out = run_with_input("messages", &["interactive"], "setup\nq\n");
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    assert!(stdout(&out).contains("setup()"), "{}", stdout(&out));
}
