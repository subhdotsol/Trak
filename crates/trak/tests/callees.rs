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

#[test]
fn std_and_library_calls_are_pruned() {
    let out = run_in("messages", &["calls", "setup"]);
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("setup → read_name"), "{text}");
    assert!(text.contains("setup → log_empty"), "{text}");
    for std_call in ["unwrap", "is_empty", "to_string", "trim"] {
        assert!(!text.contains(std_call), "{std_call} leaked into {text}");
    }
}

#[test]
fn fn_shows_only_repo_defined_calls() {
    let out = run_in("messages", &["fn", "read_name"]);
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    let text = stdout(&out);
    assert!(!text.contains("→ trim"), "{text}");
    assert!(!text.contains("→ to_string"), "{text}");
}

/// A repository that declares functions named after std helpers: only the
/// calls that resolve into this repository should survive.
#[test]
fn name_collisions_with_std_do_not_leak_into_calls() {
    let out = run_in("collisions", &["fn", "main"]);
    assert!(success(&out), "stderr: {}", common::stderr(&out));
    let text = stdout(&out);

    // Calls into this repository, including the module-qualified one and the
    // renamed sibling-crate import, which is recorded under the name it
    // renames.
    for expected in ["read_input", "record", "paint", "highlight"] {
        assert!(
            text.contains(&format!("main → {expected}")),
            "{expected} missing in {text}"
        );
    }
    for same_name in ["trim", "ok", "as_str"] {
        assert!(
            text.contains(&format!("main → {same_name}")),
            "{same_name} missing in {text}"
        );
    }

    // The std and external calls that share those names must not appear.
    for foreign in [
        "new",
        "read_to_string",
        "map_err",
        "flush",
        "yellow",
        "style",
        "read_line",
    ] {
        assert!(
            !text.contains(&format!("main → {foreign}")),
            "{foreign} leaked into {text}"
        );
    }
}
