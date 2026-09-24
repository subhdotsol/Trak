mod common;

use common::{run_in, stdout, success};

#[test]
fn lists_callees() {
    let out = run_in("calls", &["calls", "main"]);
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("parse"), "{text}");
}

#[test]
fn mermaid_format_is_emitted() {
    let out = run_in("calls", &["calls", "main", "--format", "mermaid"]);
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("graph TD"), "{text}");
    assert!(text.contains("main --> parse"), "{text}");
}
