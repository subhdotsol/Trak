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
