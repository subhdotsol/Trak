// A repository that declares functions whose names collide with std helpers,
// so filtering by name alone would keep every std call below.
use std::io::{IsTerminal, Write};

use console::style;
use sibling::highlight as shout;

mod support;

fn trim(input: &str) -> String {
    input.to_string()
}

fn ok() -> bool {
    true
}

fn as_str() -> &'static str {
    "collisions"
}

fn main() {
    let input = support::read_input();

    // Repository calls, including one through a renamed sibling-crate import.
    let _ = shout(&input);
    let _ = trim(&input);
    let _ = ok();
    let _ = as_str();
    support::record(&input);

    // std and external calls that share those names.
    let _ = input.trim();
    let _ = String::new();
    let _ = input.map_err(|error| error);
    let _ = std::io::stderr().flush().ok();
    let _ = style("ready").yellow();
    let _ = style("ready");
    let _ = sibling::paint(&input);
    let _ = console::paint(&input);
    let _ = std::fs::read_to_string(&input).map_err(|error| error);
}
