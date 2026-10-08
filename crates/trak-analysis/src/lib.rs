//! Call-graph analysis over the functions collected by `trak-rust`.

use std::collections::{HashMap, HashSet, VecDeque};
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

/// The tree of functions that reach `name`, as indented text. The root is not
/// included; each line ends in a newline. Empty when nothing calls `name`.
pub fn caller_tree(functions: &[Function], name: &str) -> String {
    let next = |of: &str| -> Vec<String> {
        callers(functions, of)
            .iter()
            .map(|f| f.name.clone())
            .collect()
    };
    render_tree(name, &next)
}

/// The tree of functions `name` reaches, as indented text. The root is not
/// included; each line ends in a newline. Empty when `name` calls nothing in
/// this repository.
pub fn callee_tree(functions: &[Function], name: &str) -> String {
    let next = |of: &str| -> Vec<String> {
        function_by_name(functions, of)
            .map(|f| f.calls.clone())
            .unwrap_or_default()
    };
    render_tree(name, &next)
}

/// Render the tree below `root`, taking each level from `next`. A name already
/// placed in the tree is not expanded again, so a cycle terminates.
///
/// `seen` starts empty rather than holding `root`: several functions in a
/// workspace can share a name, and seeding the root would drop a real edge from
/// a different function that happens to be called the same thing.
fn render_tree<F: Fn(&str) -> Vec<String>>(root: &str, next: &F) -> String {
    let mut out = String::new();
    let mut seen = HashSet::new();
    walk_tree(&mut out, root, "", &mut seen, next);
    out
}

fn walk_tree<F: Fn(&str) -> Vec<String>>(
    out: &mut String,
    name: &str,
    prefix: &str,
    seen: &mut HashSet<String>,
    next: &F,
) {
    let level: Vec<String> = next(name)
        .into_iter()
        .filter(|child| seen.insert(child.clone()))
        .collect();
    let total = level.len();
    for (index, child) in level.iter().enumerate() {
        let last = index + 1 == total;
        out.push_str(prefix);
        out.push_str(if last { "└─ " } else { "├─ " });
        out.push_str(child);
        out.push('\n');
        let deeper = format!("{prefix}{}", if last { "   " } else { "│  " });
        walk_tree(out, child, &deeper, seen, next);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tree_fn(name: &str, calls: &[&str]) -> Function {
        Function {
            name: name.to_string(),
            file: PathBuf::from("x.rs"),
            line: 1,
            end_line: 2,
            signature: format!("fn {name}()"),
            params: Vec::new(),
            calls: calls.iter().map(|c| c.to_string()).collect(),
        }
    }

    #[test]
    fn caller_tree_walks_up_to_the_entry_point() {
        let fns = vec![
            tree_fn("main", &["run"]),
            tree_fn("run", &["parse"]),
            tree_fn("parse", &[]),
        ];
        assert_eq!(caller_tree(&fns, "parse"), "└─ run\n   └─ main\n");
    }

    #[test]
    fn callee_tree_walks_down() {
        let fns = vec![
            tree_fn("main", &["run"]),
            tree_fn("run", &["parse"]),
            tree_fn("parse", &[]),
        ];
        assert_eq!(callee_tree(&fns, "main"), "└─ run\n   └─ parse\n");
    }

    #[test]
    fn a_tree_branches() {
        let fns = vec![
            tree_fn("top", &["a", "b"]),
            tree_fn("a", &[]),
            tree_fn("b", &[]),
        ];
        assert_eq!(callee_tree(&fns, "top"), "├─ a\n└─ b\n");
    }

    #[test]
    fn a_leaf_has_an_empty_tree() {
        let fns = vec![tree_fn("alone", &[])];
        assert_eq!(caller_tree(&fns, "alone"), "");
        assert_eq!(callee_tree(&fns, "alone"), "");
    }

    #[test]
    fn a_cycle_terminates() {
        let fns = vec![tree_fn("a", &["b"]), tree_fn("b", &["a"])];
        assert_eq!(callee_tree(&fns, "a"), "└─ b\n   └─ a\n");
    }

    #[test]
    fn direct_recursion_terminates() {
        let fns = vec![tree_fn("loops", &["loops"])];
        assert_eq!(callee_tree(&fns, "loops"), "└─ loops\n");
    }

    #[test]
    fn a_caller_sharing_the_root_name_is_kept() {
        // Two different functions named `run`: the outer one calls the inner.
        // Filtering the root out of its own tree would drop this real edge, and
        // would make `callers --tree` disagree with the flat `callers` listing.
        let mut outer = tree_fn("run", &["target"]);
        outer.file = PathBuf::from("a.rs");
        let mut inner = tree_fn("target", &[]);
        inner.file = PathBuf::from("b.rs");
        let mut top = tree_fn("run", &["run"]);
        top.file = PathBuf::from("c.rs");
        let fns = vec![outer, inner, top];
        // Seeding `seen` with the root would filter this edge out entirely and
        // leave the tree empty, which is what the flat `callers` listing would
        // then contradict.
        assert_eq!(caller_tree(&fns, "target"), "└─ run\n");
    }
}
