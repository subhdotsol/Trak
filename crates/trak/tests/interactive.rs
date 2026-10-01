mod common;

use common::{run_with_input, stdout, success};

#[test]
fn a_scripted_session_reaches_the_caller() {
    let out = run_with_input("messages", &[], "failed to read input\n1\n2\nq\n");
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("Repository: "), "{text}");
    assert!(
        text.contains("2 matches for `failed to read input`"),
        "{text}"
    );
    assert!(text.contains("read_name()"), "{text}");
    assert!(text.contains("CALLERS"), "{text}");
    assert!(text.contains("setup"), "{text}");
}

#[test]
fn a_script_that_ends_mid_session_exits_cleanly() {
    // No `q`: stdin simply runs out while the detail screen is up.
    let out = run_with_input("messages", &[], "failed to read input\n1\n");
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
    let out = run_with_input("messages", &[], "setup\nq\n");
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
