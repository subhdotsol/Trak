mod common;

use common::{run_in, stdout, success};

#[test]
fn impact_reaches_transitive_callers() {
    let out = run_in("calls", &["impact", "validate"]);
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("parse"), "{text}");
    assert!(text.contains("main"), "{text}");
}

#[test]
fn impact_of_leaf_reports_nothing_else() {
    let out = run_in("references", &["impact", "origin"]);
    assert!(success(&out));
    assert!(
        stdout(&out).contains("affects nothing else"),
        "{}",
        stdout(&out)
    );
}
