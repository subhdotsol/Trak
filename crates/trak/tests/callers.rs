mod common;

use common::{run_in, stdout, success};

#[test]
fn finds_direct_callers() {
    let out = run_in("calls", &["callers", "parse"]);
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    assert!(stdout(&out).contains("main"), "{}", stdout(&out));
}

#[test]
fn tree_lists_ancestors() {
    let out = run_in("calls", &["callers", "validate", "--tree"]);
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("parse"), "{text}");
    assert!(text.contains("main"), "{text}");
}
