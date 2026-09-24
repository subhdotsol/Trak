use std::fs;
use std::path::Path;

use trak_core::{Function, Repository, Result, SourceFiles, Symbol, SymbolKind};

/// Keywords that never begin a call expression.
const NON_CALL_KEYWORDS: &[&str] = &[
    "as", "async", "await", "box", "break", "const", "continue", "copy", "crate", "default", "dyn",
    "else", "enum", "extern", "fn", "for", "if", "impl", "in", "let", "loop", "macro", "match",
    "mod", "move", "mut", "pub", "ref", "return", "self", "static", "struct", "super", "trait",
    "type", "union", "unsafe", "use", "where", "while", "yield",
];

/// Keywords that may precede an item name (e.g. `static mut NAME`).
const NAME_SKIP_KEYWORDS: &[&str] = &["mut", "ref", "dyn", "fn", "const", "unsafe", "extern"];

/// A word token with its byte offsets inside the source line.
#[derive(Debug, Clone)]
struct Token {
    word: String,
    start: usize,
    end: usize,
}

/// Carve an identifier-like token stream out of a line.
fn tokens(line: &str) -> Vec<Token> {
    let bytes = line.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_alphabetic() || c == b'_' {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            out.push(Token {
                word: line[start..i].to_string(),
                start,
                end: i,
            });
        } else {
            i += 1;
        }
    }
    out
}

fn is_name(word: &str) -> bool {
    !word.is_empty()
        && (word.chars().next().is_some_and(|c| c.is_alphabetic()) || word.starts_with('_'))
}

/// The name of the first token after `start`, skipping keywords and generic
/// brackets (`impl<T> Foo` selects `Foo`, not `T`).
fn name_after_impl(line: &str, toks: &[Token], start: usize) -> Option<String> {
    let mut prev_end = if start > 0 { toks[start - 1].end } else { 0 };
    for tk in &toks[start..] {
        let gap = &line[prev_end..tk.start];
        if !gap.contains('<') && !NAME_SKIP_KEYWORDS.contains(&tk.word.as_str()) {
            return Some(tk.word.clone());
        }
        prev_end = tk.end;
    }
    None
}

/// Try to read an item declaration out of a source line.
fn parse_symbol_line(line: &str) -> Option<(SymbolKind, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('#')
        || trimmed.starts_with("//")
        || trimmed.starts_with("/*")
        || trimmed.starts_with('*')
    {
        return None;
    }

    let toks = tokens(line);
    for (i, tk) in toks.iter().enumerate() {
        let name = match tk.word.as_str() {
            "fn" => return Some((SymbolKind::Function, toks.get(i + 1)?.word.clone())),
            "macro_rules" => {
                if line.get(tk.end..).is_some_and(|rest| rest.starts_with('!')) {
                    toks.get(i + 1)?.word.clone()
                } else {
                    continue;
                }
            }
            "struct" => toks.get(i + 1)?.word.clone(),
            "enum" => toks.get(i + 1)?.word.clone(),
            "trait" => toks.get(i + 1)?.word.clone(),
            "union" => toks.get(i + 1)?.word.clone(),
            "impl" => match name_after_impl(line, &toks, i + 1) {
                Some(name) => name,
                None => continue,
            },
            "type" => toks.get(i + 1)?.word.clone(),
            "mod" => toks.get(i + 1)?.word.clone(),
            "const" => {
                // `const fn` is a function, handled above.
                if toks.get(i + 1).is_some_and(|t| t.word == "fn") {
                    continue;
                }
                name_after_impl(line, &toks, i + 1)?
            }
            "static" => {
                if toks.get(i + 1).is_some_and(|t| t.word == "ref") {
                    continue;
                }
                name_after_impl(line, &toks, i + 1)?
            }
            _ => continue,
        };
        let kind = match tk.word.as_str() {
            "macro_rules" => SymbolKind::Macro,
            "struct" => SymbolKind::Struct,
            "enum" => SymbolKind::Enum,
            "trait" => SymbolKind::Trait,
            "union" => SymbolKind::Union,
            "impl" => SymbolKind::Impl,
            "type" => SymbolKind::Type,
            "mod" => SymbolKind::Mod,
            "const" => SymbolKind::Const,
            "static" => SymbolKind::Static,
            _ => continue,
        };
        return Some((kind, name));
    }
    None
}

/// Extract every symbol declared in the repository's Rust source.
pub fn symbols(repo: &Repository) -> Result<Vec<Symbol>> {
    let mut out = Vec::new();
    for file in SourceFiles::new(repo.root()) {
        if file.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if let Ok(contents) = fs::read_to_string(&file) {
            out.extend(symbols_in(&contents, &file));
        }
    }
    Ok(out)
}

fn symbols_in(contents: &str, file: &Path) -> Vec<Symbol> {
    let mut out = Vec::new();
    for (index, line) in contents.lines().enumerate() {
        if let Some((kind, name)) = parse_symbol_line(line) {
            out.push(Symbol::new(name, kind, file.to_path_buf(), index + 1));
        }
    }
    out
}

/// A declaration parsed from a single line.
struct FnFound {
    name: String,
    params: String,
}

/// Read a `fn` declaration, capturing the parameter list.
fn parse_fn(line: &str) -> Option<FnFound> {
    let toks = tokens(line);
    let bytes = line.as_bytes();
    for (i, tk) in toks.iter().enumerate() {
        if tk.word != "fn" {
            continue;
        }
        let fname = toks.get(i + 1)?;

        // Skip angle-bracket generics so `fn f<T: Fn() -> i32>()` lands on the
        // real parameter list rather than the `Fn()` trait bound.
        let mut p = fname.end;
        let mut angle = 0usize;
        while p < bytes.len() {
            match bytes[p] {
                b'<' => angle += 1,
                b'>' if angle > 0 => angle -= 1,
                b'(' if angle == 0 => break,
                _ => {}
            }
            p += 1;
        }
        if p >= bytes.len() {
            return Some(FnFound {
                name: fname.word.clone(),
                params: String::new(),
            });
        }

        let mut q = p + 1;
        let mut depth = 1usize;
        while q < bytes.len() {
            match bytes[q] {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            q += 1;
        }

        let params = if depth == 0 {
            line[p + 1..q].trim().to_string()
        } else {
            String::new()
        };
        return Some(FnFound {
            name: fname.word.clone(),
            params,
        });
    }
    None
}

/// Extract parameter names from a comma-separated parameter list.
fn split_params(params: &str) -> Vec<String> {
    let mut names = Vec::new();
    for chunk in split_top_level(params, b',') {
        let mut took = false;
        for tk in tokens(chunk) {
            if took {
                break;
            }
            let before = if tk.start > 0 {
                chunk.as_bytes()[tk.start - 1]
            } else {
                b' '
            };
            if before == b'\'' || before == b'#' {
                // lifetime or attribute, not a parameter name
                continue;
            }
            let word = tk.word.as_str();
            if word == "self" || (is_name(word) && !NON_CALL_KEYWORDS.contains(&word)) {
                names.push(word.to_string());
                took = true;
            }
        }
    }
    names
}

/// Split a string on a byte at the top nesting level only.
fn split_top_level(text: &str, sep: u8) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0u32;
    let mut start = 0usize;
    for (index, ch) in text.bytes().enumerate() {
        match ch {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth = depth.saturating_sub(1),
            _ => {}
        }
        if ch == sep && depth == 0 {
            out.push(text[start..index].trim());
            start = index + 1;
        }
    }
    out.push(text[start..].trim());
    out
}

/// Count the net brace delta of a line, ignoring strings and line comments.
fn brace_delta(line: &str) -> i32 {
    let bytes = line.as_bytes();
    let mut delta = 0i32;
    let mut in_string = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            if c == b'\\' {
                i += 2;
                continue;
            }
            if c == b'"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match c {
            b'"' => in_string = true,
            b'/' if i + 1 < bytes.len() && bytes[i + 1] == b'/' => break,
            b'{' => delta += 1,
            b'}' => delta -= 1,
            _ => {}
        }
        i += 1;
    }
    delta
}

/// Collect simple `name(...)` call sites from a line.
fn find_calls(line: &str, out: &mut Vec<String>) {
    let bytes = line.as_bytes();
    let mut i = 0;
    let n = bytes.len();
    while i < n {
        let c = bytes[i];
        if c.is_ascii_alphabetic() || c == b'_' {
            let start = i;
            while i < n && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let ident = &line[start..i];
            if bytes[start].is_ascii_uppercase() {
                // Type/const identifiers, not calls.
                continue;
            }
            let is_macro = i < n && bytes[i] == b'!';
            let mut j = i;
            while j < n && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if !is_macro
                && j < n
                && bytes[j] == b'('
                && !ident.starts_with("r#")
                && !ident.ends_with('!')
                && !NON_CALL_KEYWORDS.contains(&ident)
                && !out.contains(&ident.to_string())
            {
                out.push(ident.to_string());
            }
            continue;
        }
        i += 1;
    }
}

/// A function currently being filled while scanning a file.
struct ActiveFn {
    function: Function,
    body_depth: i32,
    opened: bool,
}

/// Extract every function together with the calls detected in its body.
pub fn functions(repo: &Repository) -> Result<Vec<Function>> {
    let mut out = Vec::new();
    for file in SourceFiles::new(repo.root()) {
        if file.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        if let Ok(contents) = fs::read_to_string(&file) {
            out.extend(functions_in(&contents, &file));
        }
    }
    Ok(out)
}

fn functions_in(contents: &str, file: &Path) -> Vec<Function> {
    let mut finished: Vec<Function> = Vec::new();
    let mut active: Vec<ActiveFn> = Vec::new();
    let mut depth = 0i32;

    for (index, line) in contents.lines().enumerate() {
        let delta = brace_delta(line);

        if let Some(found) = parse_fn(line) {
            let mut function = Function {
                name: found.name,
                file: file.to_path_buf(),
                line: index + 1,
                end_line: index + 1,
                signature: line.trim().to_string(),
                params: split_params(&found.params),
                calls: Vec::new(),
            };
            find_calls(line, &mut function.calls);
            // The declaration line `fn name(` must not count as a self-call.
            function.calls.retain(|call| call != &function.name);
            active.push(ActiveFn {
                function,
                // The body opens one brace level below the declaration's depth.
                body_depth: depth + 1,
                opened: line.contains('{'),
            });
        } else if let Some(top) = active.last_mut() {
            find_calls(line, &mut top.function.calls);
        }

        depth += delta;

        if let Some(top) = active.last_mut()
            && !top.opened
            && depth >= top.body_depth
        {
            top.opened = true;
        }

        loop {
            let should_close = active
                .last()
                .is_some_and(|top| top.opened && depth < top.body_depth);
            if !should_close {
                break;
            }
            if let Some(mut top) = active.pop() {
                top.function.end_line = index + 1;
                finished.push(top.function);
            }
        }
    }

    let last_line = contents.lines().count();
    while let Some(mut top) = active.pop() {
        top.function.end_line = last_line;
        finished.push(top.function);
    }

    finished
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes() {
        let toks = tokens("  let x = foo(&bar, |y| y + 1);");
        let words: Vec<_> = toks.iter().map(|t| t.word.as_str()).collect();
        assert!(words.contains(&"foo"));
        assert!(words.contains(&"bar"));
    }

    #[test]
    fn finds_symbols() {
        assert_eq!(
            parse_symbol_line("pub fn main()"),
            Some((SymbolKind::Function, "main".to_string()))
        );
        assert_eq!(
            parse_symbol_line("struct Point<T> {"),
            Some((SymbolKind::Struct, "Point".to_string()))
        );
        assert_eq!(
            parse_symbol_line("impl Point {"),
            Some((SymbolKind::Impl, "Point".to_string()))
        );
        assert_eq!(
            parse_symbol_line("impl<T> Wrapper<T> {"),
            Some((SymbolKind::Impl, "Wrapper".to_string()))
        );
        assert_eq!(
            parse_symbol_line("const MAX: usize = 100;"),
            Some((SymbolKind::Const, "MAX".to_string()))
        );
        assert_eq!(
            parse_symbol_line("static mut COUNTER: u64;"),
            Some((SymbolKind::Static, "COUNTER".to_string()))
        );
        assert_eq!(parse_symbol_line("let x = 5;"), None);
        assert_eq!(parse_symbol_line("// fn doc()"), None);
    }

    #[test]
    fn finds_fns() {
        let found = parse_fn("async fn handle<T: Fn() -> usize>(x: usize) -> T {").unwrap();
        assert_eq!(found.name, "handle");
        assert_eq!(found.params, "x: usize");
    }

    #[test]
    fn splits_params() {
        assert_eq!(
            split_params("x: usize, mut y: &str, z:(u8, u8)"),
            vec!["x", "y", "z"]
        );
        assert_eq!(split_params("&self"), vec!["self"]);
        assert_eq!(split_params("&'a self"), vec!["self"]);
    }

    #[test]
    fn call_sites() {
        let mut calls = Vec::new();
        find_calls("let out = render(model, &opts);", &mut calls);
        assert!(calls.contains(&"render".to_string()));

        let mut calls = Vec::new();
        find_calls("let _ = vec![foo(1)];", &mut calls);
        assert!(calls.contains(&"foo".to_string()));

        // The declaration line of a function must not self-call.
        let mut function = Function {
            name: "parse".to_string(),
            file: std::path::PathBuf::from("x.rs"),
            line: 1,
            end_line: 3,
            signature: "fn parse() {".to_string(),
            params: Vec::new(),
            calls: Vec::new(),
        };
        find_calls("fn parse() { validate(); }", &mut function.calls);
        function.calls.retain(|call| call != &function.name);
        assert!(function.calls.contains(&"validate".to_string()));
        assert!(!function.calls.contains(&"parse".to_string()));
    }
}
