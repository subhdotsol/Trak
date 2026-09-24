//! Call-graph analysis over the functions collected by `trak-rust`.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

use trak_core::{Function, Symbol};

/// Functions that call `name`, sorted by location.
pub fn callers<'a>(functions: &'a [Function], name: &str) -> Vec<&'a Function> {
    let mut out: Vec<_> = functions
        .iter()
        .filter(|f| f.calls.iter().any(|c| c == name))
        .collect();
    out.sort_by_key(|f| (f.file.clone(), f.line));
    out
}

/// The first function whose name matches.
pub fn function_by_name<'a>(functions: &'a [Function], name: &str) -> Option<&'a Function> {
    functions.iter().find(|f| f.name == name)
}

/// The innermost function whose span covers `(file, line)`.
pub fn containing_function<'a>(
    functions: &'a [Function],
    file: &std::path::Path,
    line: usize,
) -> Option<&'a Function> {
    functions
        .iter()
        .filter(|f| f.file == file && f.line <= line && line <= f.end_line)
        .min_by_key(|f| (f.end_line - f.line, f.line))
}

/// Shortest call path from `from` to `to` by function name, if one exists.
pub fn find_path(functions: &[Function], from: &str, to: &str) -> Option<Vec<String>> {
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    for f in functions {
        graph
            .entry(f.name.as_str())
            .or_default()
            .extend(f.calls.iter().map(String::as_str));
    }

    let mut queue = VecDeque::from([from]);
    let mut prev: HashMap<&str, &str> = HashMap::new();
    prev.insert(from, "");
    while let Some(node) = queue.pop_front() {
        if node == to {
            let mut path = vec![to.to_string()];
            let mut cur = to;
            while let Some(&parent) = prev.get(cur) {
                if parent.is_empty() {
                    break;
                }
                path.push(parent.to_string());
                cur = parent;
            }
            path.reverse();
            return Some(path);
        }
        if let Some(nexts) = graph.get(node) {
            for &next in nexts {
                if !prev.contains_key(next) {
                    prev.insert(next, node);
                    queue.push_back(next);
                }
            }
        }
    }
    None
}

/// Everything transitively affected when `name` changes (functions that call it
/// directly or indirectly), excluding itself.
pub fn impact(functions: &[Function], name: &str) -> Vec<String> {
    let mut affected: HashMap<&str, ()> = HashMap::new();
    let mut graph: HashMap<&str, Vec<&str>> = HashMap::new();
    for f in functions {
        for callee in &f.calls {
            graph
                .entry(callee.as_str())
                .or_default()
                .push(f.name.as_str());
        }
    }

    let mut stack = vec![name];
    affected.insert(name, ());
    while let Some(node) = stack.pop() {
        if let Some(callers) = graph.get(node) {
            for &caller in callers {
                if affected.insert(caller, ()).is_none() {
                    stack.push(caller);
                }
            }
        }
    }

    affected.remove(name);
    let mut names: Vec<String> = affected.into_keys().map(String::from).collect();
    names.sort();
    names
}

/// A textual reference to a name: a definition or a caller.
pub struct Reference {
    pub file: PathBuf,
    pub line: usize,
    pub text: String,
    pub kind: &'static str,
}

/// Collect every known reference to `name` across the codebase.
pub fn references(functions: &[Function], symbols: &[Symbol], name: &str) -> Vec<Reference> {
    let mut out = Vec::new();
    for s in symbols {
        if s.name == name {
            out.push(Reference {
                file: s.file.clone(),
                line: s.line,
                text: format!("{} declaration", s.kind),
                kind: "definition",
            });
        }
    }
    for f in functions {
        if f.name == name {
            out.push(Reference {
                file: f.file.clone(),
                line: f.line,
                text: f.signature.clone(),
                kind: "definition",
            });
        } else if f.calls.iter().any(|c| c == name) {
            out.push(Reference {
                file: f.file.clone(),
                line: f.line,
                text: f.signature.clone(),
                kind: "caller",
            });
        }
    }
    out.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
    out
}
