mod common;

use common::{run_in, stdout, success};

#[test]
fn finds_text_in_fixture() {
    let out = run_in("references", &["search", "origin"]);
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    assert!(stdout(&out).contains("origin"), "{}", stdout(&out));
}

#[test]
fn json_format_serializes_hits() {
    let out = run_in("references", &["search", "origin", "--format", "json"]);
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    assert!(stdout(&out).contains("\"hits\""), "{}", stdout(&out));
    assert!(stdout(&out).contains("origin"), "{}", stdout(&out));
}

#[test]
fn missing_query_is_an_error() {
    let out = run_in("basic", &["search", "definitely-not-present"]);
    assert!(!success(&out));
}
