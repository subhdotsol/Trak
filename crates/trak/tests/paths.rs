mod common;

use common::{run_in, stdout, success};

#[test]
fn finds_shortest_path() {
    let out = run_in("calls", &["path", "main", "validate"]);
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("main"), "{text}");
    assert!(text.contains("parse"), "{text}");
    assert!(text.contains("validate"), "{text}");
    assert!(text.contains("2 hop(s)"), "{text}");
}

#[test]
fn unreachable_is_a_graceful_miss() {
    let out = run_in("calls", &["path", "validate", "main"]);
    assert!(success(&out));
    assert!(stdout(&out).contains("no call path"), "{}", stdout(&out));
}

#[test]
fn path_to_a_log_message() {
    let out = run_in(
        "messages",
        &["path", "main", "failed to read input: {error}"],
    );
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("main → setup → log_empty"), "{text}");
    assert!(text.contains("[message]"), "{text}");
}

#[test]
fn path_message_resolves_positional_interpolation() {
    // The fixture prints `failed to read input: {}`, so this only matches once
    // `{name}` placeholders are treated as wildcards.
    let out = run_in(
        "messages",
        &["path", "main", "failed to read input: {error}"],
    );
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    assert!(stdout(&out).contains("log_empty"), "{}", stdout(&out));
}

#[test]
fn path_chains_are_reported_in_order() {
    let out = run_in(
        "messages",
        &["path", "main", "read_name", "--format", "json"],
    );
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    let text = stdout(&out);
    assert!(
        text.contains(
            "\"chains\": [\n    [\n      \"main\",\n      \"setup\",\n      \"read_name\""
        ),
        "{text}"
    );
}

#[test]
fn unknown_message_is_an_error() {
    let out = run_in("messages", &["path", "main", "no such message anywhere"]);
    assert!(!success(&out));
    assert!(
        common::stderr(&out).contains("function or message"),
        "{}",
        common::stderr(&out)
    );
}

#[test]
fn message_outside_a_function_reports_its_location() {
    let out = run_in("messages", &["path", "main", "no such user on disk"]);
    assert!(!success(&out));
    let err = common::stderr(&out);
    assert!(err.contains("outside any function"), "{err}");
    assert!(err.contains("no such user on disk"), "{err}");
}
