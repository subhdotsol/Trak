mod common;

use common::{run_in, stdout, success};

#[test]
fn finds_struct() {
    let out = run_in("references", &["symbol", "Point"]);
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    assert!(stdout(&out).contains("struct"), "{}", stdout(&out));
}

#[test]
fn finds_assoc_fn() {
    let out = run_in("references", &["symbol", "origin"]);
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    assert!(stdout(&out).contains("fn"), "{}", stdout(&out));
}

#[test]
fn unknown_symbol_is_an_error() {
    let out = run_in("references", &["symbol", "Mystery"]);
    assert!(!success(&out));
}
