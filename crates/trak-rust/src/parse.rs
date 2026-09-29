use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

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

/// Where the scanner sits at a line boundary. Strings, raw strings and block
/// comments all span lines, so this has to be carried between them; a plain
/// `bool` could express neither a raw string's hash count nor a comment's depth.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Scan {
    #[default]
    Code,
    /// Inside a `"..."` that has not closed yet.
    Str,
    /// Inside a raw string, holding the hash count its close requires.
    RawStr(usize),
    /// Inside `/* */`, holding the nesting depth.
    Block(u32),
}

/// Blank out everything on `line` that is not code: string, raw string and char
/// literal contents, line comments, and block comments. Replacement preserves
/// byte length, so callers keep using offsets into the original line.
///
/// `scan` carries multi-line state and is updated in place.
fn strip_noncode(line: &str, scan: &mut Scan) -> String {
    let src = line.as_bytes();
    let n = src.len();
    let mut out = src.to_vec();
    let mut i = 0;

    while i < n {
        match *scan {
            Scan::Block(depth) => match block_end(src, i, depth) {
                Ok(end) => {
                    blank(&mut out, i, end);
                    *scan = Scan::Code;
                    i = end;
                }
                Err(deeper) => {
                    blank(&mut out, i, n);
                    *scan = Scan::Block(deeper);
                    break;
                }
            },
            Scan::Str => match string_end(src, i) {
                Some(end) => {
                    blank(&mut out, i, end);
                    *scan = Scan::Code;
                    i = end;
                }
                None => {
                    blank(&mut out, i, n);
                    break;
                }
            },
            Scan::RawStr(hashes) => match raw_string_end(src, i, hashes) {
                Some(end) => {
                    blank(&mut out, i, end);
                    *scan = Scan::Code;
                    i = end;
                }
                None => {
                    blank(&mut out, i, n);
                    break;
                }
            },
            Scan::Code => match src[i] {
                // Nothing after a line comment is code.
                b'/' if i + 1 < n && src[i + 1] == b'/' => {
                    blank(&mut out, i, n);
                    break;
                }
                b'/' if i + 1 < n && src[i + 1] == b'*' => {
                    blank(&mut out, i, i + 2);
                    *scan = Scan::Block(1);
                    i += 2;
                }
                // `'x'` is a literal but `'a` in `&'a str` is a lifetime, and
                // `'"'` must not be read as opening a string.
                b'\'' => match char_literal_end(src, i) {
                    Some(end) => {
                        blank(&mut out, i, end);
                        i = end;
                    }
                    None => i += 1,
                },
                b'"' => match string_end(src, i + 1) {
                    Some(end) => {
                        blank(&mut out, i, end);
                        i = end;
                    }
                    None => {
                        blank(&mut out, i, n);
                        *scan = Scan::Str;
                        break;
                    }
                },
                _ => match raw_prefix(src, i) {
                    Some((body, hashes)) => match raw_string_end(src, body, hashes) {
                        Some(end) => {
                            blank(&mut out, i, end);
                            i = end;
                        }
                        None => {
                            blank(&mut out, i, n);
                            *scan = Scan::RawStr(hashes);
                            break;
                        }
                    },
                    None => i += 1,
                },
            },
        }
    }

    String::from_utf8(out).unwrap_or_else(|_| line.to_string())
}

/// Overwrite `out[from..to]` with spaces.
fn blank(out: &mut [u8], from: usize, to: usize) {
    let to = to.min(out.len());
    for byte in &mut out[from..to] {
        *byte = b' ';
    }
}

fn is_ident_byte(byte: Option<u8>) -> bool {
    byte.is_some_and(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// The body offset and hash count of a raw string opening at `from`, covering
/// `r"`, `r#"`, `br"` and `br#"`. `None` when no raw string starts here.
fn raw_prefix(src: &[u8], from: usize) -> Option<(usize, usize)> {
    if is_ident_byte(from.checked_sub(1).map(|prev| src[prev])) {
        return None;
    }
    let mut i = from;
    if src.get(i) == Some(&b'b') {
        i += 1;
    }
    if src.get(i) != Some(&b'r') {
        return None;
    }
    i += 1;
    let first_hash = i;
    while src.get(i) == Some(&b'#') {
        i += 1;
    }
    if src.get(i) != Some(&b'"') {
        return None;
    }
    Some((i + 1, i - first_hash))
}

/// Offset just past a char literal opening at `from`, or `None` when the quote
/// opens a lifetime (`&'a`, `'static`) instead.
fn char_literal_end(src: &[u8], from: usize) -> Option<usize> {
    let n = src.len();
    let body = from + 1;
    if body >= n {
        return None;
    }
    if src[body] == b'\\' {
        // `'\n'`, `'\''`, `'\u{1F600}'`: the escaped byte cannot close it.
        let mut i = body + 2;
        while i < n {
            if src[i] == b'\'' {
                return Some(i + 1);
            }
            i += 1;
        }
        return None;
    }
    // One whole character, then the closing quote.
    let close = body + utf8_len(src[body]);
    (close < n && src[close] == b'\'').then_some(close + 1)
}

/// Byte length of the UTF-8 character beginning with `lead`.
fn utf8_len(lead: u8) -> usize {
    match lead {
        0x00..0x80 => 1,
        0x80..0xE0 => 2,
        0xE0..0xF0 => 3,
        0xF0.. => 4,
    }
}

/// Offset just past the closing quote of a string whose body starts at `from`,
/// or `None` when the string runs past the end of the line.
fn string_end(src: &[u8], from: usize) -> Option<usize> {
    let mut i = from;
    while i < src.len() {
        match src[i] {
            b'\\' => i += 2,
            b'"' => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

/// Offset just past the closing `"` plus `hashes` of a raw string whose body
/// starts at `from`, or `None` when it runs past the end of the line.
fn raw_string_end(src: &[u8], from: usize, hashes: usize) -> Option<usize> {
    let n = src.len();
    let mut i = from;
    while i < n {
        if src[i] == b'"' {
            let after = &src[i + 1..];
            if after.len() >= hashes && after[..hashes].iter().all(|byte| *byte == b'#') {
                return Some(i + 1 + hashes);
            }
        }
        i += 1;
    }
    None
}

/// Offset just past the `*/` that closes a block comment nested `depth` deep, or
/// the depth still open when the line ends. Rust block comments nest.
fn block_end(src: &[u8], from: usize, depth: u32) -> std::result::Result<usize, u32> {
    let n = src.len();
    let mut depth = depth;
    let mut i = from;
    while i + 1 < n {
        if src[i] == b'/' && src[i + 1] == b'*' {
            depth += 1;
            i += 2;
        } else if src[i] == b'*' && src[i + 1] == b'/' {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return Ok(i);
            }
        } else {
            i += 1;
        }
    }
    Err(depth)
}

/// Try to read an item declaration out of a source line.
fn parse_symbol_line(line: &str) -> Option<(SymbolKind, String)> {
    let line = &strip_noncode(line, &mut Scan::default());
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
    let mut scan = Scan::default();
    for (index, line) in contents.lines().enumerate() {
        let code = strip_noncode(line, &mut scan);
        if let Some((kind, name)) = parse_symbol_line(&code) {
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
    let line = &strip_noncode(line, &mut Scan::default());
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

/// What a call site is attached to. Together with the name itself this is the
/// call's path in the source, and it is what separates a repository function
/// from a std or third-party one that happens to share its name.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Base {
    /// `foo(..)` — a bare name, so it can only be a function in scope.
    Bare,
    /// `path::foo(..)`, holding the leftmost path segment (`Report` in
    /// `Report::new(..)`, `std` in `std::io::stderr(..)`).
    Path(String),
    /// `receiver.foo(..)`, holding the receiver's own base and the name of the
    /// call it came from, so a chain is followed back to its root — including
    /// `style(x).yellow(..)`, where `style` may be an external import.
    Method(Box<Base>, String),
    /// `receiver.foo(..)` where the receiver is a value rather than a call, so
    /// its type — and therefore the method's origin — is unknowable textually.
    Unknown,
}

/// A `name(...)` call site, the path it was written on, and the file it sits
/// in, which decides whether a type root such as `Report` in `Report::new(..)`
/// is this repository's own type.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CallSite {
    name: String,
    base: Base,
    file: usize,
}

/// Method names that belong to the standard library. They are only consulted
/// for a call on a value of unknown type (`input.trim()`), where the name alone
/// is the only evidence available.
const FOREIGN_METHODS: &[&str] = &[
    "and_then",
    "any",
    "as_bytes",
    "as_mut",
    "as_ref",
    "as_str",
    "await",
    "bytes",
    "capacity",
    "chain",
    "chars",
    "clear",
    "clone",
    "cmp",
    "collect",
    "contains",
    "count",
    "dedup",
    "ends_with",
    "entries",
    "enumerate",
    "err",
    "expect",
    "extend",
    "filter",
    "filter_map",
    "find",
    "first",
    "flat_map",
    "flatten",
    "fold",
    "for_each",
    "from_iter",
    "get",
    "get_mut",
    "insert",
    "is_empty",
    "is_err",
    "is_none",
    "is_ok",
    "is_some",
    "iter",
    "iter_mut",
    "join",
    "keys",
    "last",
    "len",
    "lines",
    "map",
    "map_err",
    "max",
    "min",
    "next",
    "nth",
    "ok",
    "ok_or",
    "or",
    "or_else",
    "peekable",
    "position",
    "push",
    "push_str",
    "read",
    "read_line",
    "read_to_string",
    "reduce",
    "retain",
    "rev",
    "reverse",
    "rfind",
    "skip",
    "sort",
    "sort_by",
    "split",
    "split_at",
    "split_once",
    "splitn",
    "starts_with",
    "strip_prefix",
    "strip_suffix",
    "sum",
    "take",
    "then",
    "to_lowercase",
    "to_owned",
    "to_string",
    "to_uppercase",
    "trim",
    "trim_end",
    "trim_start",
    "truncate",
    "values",
    "windows",
    "with_capacity",
    "write",
    "write_all",
    "writeln",
    "zip",
];

/// Names this repository declares, used to resolve a call's path.
#[derive(Default)]
struct Declared {
    functions: HashSet<String>,
    modules: HashSet<String>,
    /// Types keyed by the file that declares them: a type is only in scope
    /// unqualified in its own module, so `String::new(..)` stays foreign even
    /// when some other module declares a `type String`.
    types: HashSet<(String, usize)>,
    /// Crate names in this repository, so `use topcoat_ui::manage::Package` is
    /// a repository import while `use console::style` is an external one.
    crates: HashSet<String>,
    /// Names each file imports. A bare call to one of these resolves to
    /// whatever it was imported from, so `use console::style` is not a call to
    /// this repository's `style`.
    imports: HashMap<usize, HashMap<String, Imported>>,
}

/// A name a file imports: the path root it came from, and the name it was
/// declared under, which differ when the import renames it.
#[derive(Clone)]
struct Imported {
    root: String,
    origin: String,
}

impl Declared {
    /// Record a symbol seen while scanning, so later call sites can be matched
    /// against it. `file` identifies the file the symbol was declared in.
    fn insert(&mut self, kind: SymbolKind, name: &str, file: usize) {
        match kind {
            SymbolKind::Function => {
                self.functions.insert(name.to_string());
            }
            SymbolKind::Mod => {
                self.modules.insert(name.to_string());
            }
            SymbolKind::Struct
            | SymbolKind::Enum
            | SymbolKind::Trait
            | SymbolKind::Union
            | SymbolKind::Type => {
                self.types.insert((name.to_string(), file));
            }
            _ => {}
        }
    }

    /// Whether a path root names something this repository declares, making
    /// `Root::call(..)` a repository call and `std::io::stderr(..)` a foreign
    /// one. A type root must be declared in the calling file to count.
    fn owns_root(&self, root: &str, file: usize) -> bool {
        self.owns_crate_root(root) || self.types.contains(&(root.to_string(), file))
    }

    /// Whether `root` is a crate, module or keyword path into this repository.
    /// A type is not enough: an import always starts at a crate or module.
    fn owns_crate_root(&self, root: &str) -> bool {
        matches!(root, "crate" | "self" | "super")
            || self.modules.contains(root)
            || self.crates.contains(root)
    }

    /// The name a file imports as `name`, if it imports it.
    fn imported(&self, file: usize, name: &str) -> Option<&Imported> {
        self.imports
            .get(&file)
            .and_then(|imports| imports.get(name))
    }

    /// Record a `use` declaration's names, the root each was imported from, and
    /// the name each was declared under.
    fn insert_imports(&mut self, file: usize, declaration: &str) {
        let file_imports = self.imports.entry(file).or_default();
        for (name, imported) in parse_use(declaration) {
            file_imports.insert(name, imported);
        }
    }
}

/// The names a `use` declaration brings into scope, each with the leftmost path
/// segment it came from: `style` from `console` for `use console::style;`.
fn parse_use(declaration: &str) -> Vec<(String, Imported)> {
    let body = declaration.trim();
    let body = body.strip_prefix("pub ").unwrap_or(body).trim();
    let Some(body) = body.strip_prefix("use ") else {
        return Vec::new();
    };
    let body = body.trim().trim_end_matches(';');

    let mut out = Vec::new();
    for chunk in split_top_level(body, b',') {
        collect_import(chunk, &mut out);
    }
    out
}

fn collect_import(chunk: &str, out: &mut Vec<(String, Imported)>) {
    let chunk = chunk.trim();
    let Some(open) = chunk.find('{') else {
        // `use foo;` names no root, so there is nothing to attribute it to.
        if chunk.contains("::")
            && let Some((name, imported)) = import_name(chunk)
        {
            out.push((name, imported));
        }
        return;
    };

    let prefix = chunk[..open].trim_end_matches(':');
    let group_root = path_root_text(prefix);
    let inner = &chunk[open + 1..];
    let inner = inner.rsplit_once('}').map_or(inner, |(names, _)| names);
    for item in split_top_level(inner, b',') {
        // `use a::b::{self, c};` binds the module `b` itself, so `b::call(..)`
        // is a path into the repository just as `c(..)` is.
        if item.trim() == "self" {
            if let Some(bound) = prefix.rsplit("::").next().map(str::trim)
                && !bound.is_empty()
            {
                out.push((
                    bound.to_string(),
                    Imported {
                        root: group_root.clone(),
                        origin: bound.to_string(),
                    },
                ));
            }
            continue;
        }
        if let Some((name, mut imported)) = import_name(item) {
            imported.root = group_root.clone();
            out.push((name, imported));
        }
    }
}

/// The name one imported item binds, and what it was imported from: `Package`
/// from `topcoat_ui` for `manage::Package`, and `draw` from `render` for
/// `super::render as draw`.
fn import_name(item: &str) -> Option<(String, Imported)> {
    let item = item.trim();
    // The root comes from the whole item, so an alias keeps the module it was
    // imported from rather than the tail it was renamed after.
    let root = path_root_text(item);
    let (name, origin) = match item.split_once(" as ") {
        Some((path, alias)) => (alias.trim(), path.rsplit("::").next()?.trim()),
        None => (
            item.rsplit("::").next()?.trim(),
            item.rsplit("::").next()?.trim(),
        ),
    };
    if is_importable(name) {
        Some((
            name.to_string(),
            Imported {
                root,
                origin: origin.to_string(),
            },
        ))
    } else {
        None
    }
}

fn is_importable(name: &str) -> bool {
    !name.is_empty() && name != "*" && name != "self" && name != "as"
}

/// The leftmost `::` segment of a path, or the whole path when it has none.
fn path_root_text(path: &str) -> String {
    let path = path.trim();
    match path.split("::").next() {
        Some(root) if !root.trim().is_empty() => root.trim().to_string(),
        _ => path.to_string(),
    }
}

/// Whether the value a call is made on belongs to this repository. `name` is
/// the name of the call sitting on that value, which an import lookup needs.
fn base_is_local(base: &Base, name: &str, declared: &Declared, file: usize) -> bool {
    match base {
        // A bare name is a function in this module, unless the file imported it
        // from outside the repository.
        Base::Bare => match declared.imported(file, name) {
            Some(imported) => declared.owns_crate_root(&imported.root),
            None => true,
        },
        // A module this file imports counts as a path into the repository, so
        // `Package::locate(..)` is kept when `Package` came in through a `use`.
        Base::Path(root) => {
            declared.owns_root(root, file)
                || declared
                    .imported(file, root)
                    .is_some_and(|imported| declared.owns_crate_root(&imported.root))
        }
        Base::Method(inner, inner_name) => base_is_local(inner, inner_name, declared, file),
        Base::Unknown => false,
    }
}

/// The function a call site reaches by name. An import can rename what it
/// brings in, so `shout(..)` for `use vendor::highlight as shout` is the
/// repository's `highlight`, and the edge is recorded under that name.
fn resolved_name<'a>(site: &'a CallSite, declared: &'a Declared) -> &'a str {
    declared
        .imported(site.file, &site.name)
        .filter(|imported| declared.owns_crate_root(&imported.root))
        .map_or(site.name.as_str(), |imported| imported.origin.as_str())
}

/// Whether a call site reaches a function this repository defines.
fn is_repo_call(site: &CallSite, declared: &Declared) -> bool {
    let origin = resolved_name(site, declared);
    if !declared.functions.contains(origin) {
        return false;
    }
    match &site.base {
        // The receiver is a local value whose type cannot be read from the
        // text, so the name decides: a std method name is not a repository
        // call, anything else the repository declares is.
        Base::Method(inner, _) if **inner == Base::Unknown => !FOREIGN_METHODS.contains(&origin),
        _ => base_is_local(&site.base, &site.name, declared, site.file),
    }
}

/// Offset of the `)` that closes the `(` at `open`, skipping string literals so
/// a bracket inside a message does not throw off the depth.
fn matching_paren(line: &str, open: usize) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut i = open;
    while i < bytes.len() {
        let c = bytes[i];
        if in_string {
            match c {
                b'\\' => i += 1,
                b'"' => in_string = false,
                _ => {}
            }
        } else {
            match c {
                b'"' => in_string = true,
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        i += 1;
    }
    None
}

/// The leftmost segment of the `::` path whose final segment starts at `start`:
/// `std` for `std::io::stderr`, `crate` for `crate::ui::style`.
fn path_root(line: &str, start: usize) -> Option<String> {
    let bytes = line.as_bytes();
    let mut end = start;
    let mut root = None;
    loop {
        let mut p = end;
        while p > 0 && bytes[p - 1].is_ascii_whitespace() {
            p -= 1;
        }
        if p < 2 || bytes[p - 1] != b':' || bytes[p - 2] != b':' {
            break;
        }
        let segment_end = p - 2;
        let mut segment_start = segment_end;
        while segment_start > 0
            && (bytes[segment_start - 1].is_ascii_alphanumeric()
                || bytes[segment_start - 1] == b'_')
        {
            segment_start -= 1;
        }
        if segment_start == segment_end {
            // Not a plain segment (`::<T>`, `(expr)::`, …); stop before it.
            break;
        }
        root = Some(line[segment_start..segment_end].to_string());
        end = segment_start;
    }
    root
}

/// The base and name of the expression a method is called on, looking left from
/// the `.`.
fn receiver_base(
    line: &str,
    dot: usize,
    receivers: &HashMap<usize, (String, Base)>,
) -> Option<(String, Base)> {
    let bytes = line.as_bytes();
    let mut p = dot;
    while p > 0 && bytes[p - 1].is_ascii_whitespace() {
        p -= 1;
    }
    if p == 0 {
        return None;
    }
    if bytes[p - 1] == b')' {
        return receivers.get(&(p - 1)).cloned();
    }
    let mut start = p;
    while start > 0 && (bytes[start - 1].is_ascii_alphanumeric() || bytes[start - 1] == b'_') {
        start -= 1;
    }
    match &line[start..p] {
        "self" => Some(("self".to_string(), Base::Path("self".to_string()))),
        _ => None,
    }
}

/// The base a callee at `start` is written on.
fn callee_base(line: &str, start: usize, receivers: &HashMap<usize, (String, Base)>) -> Base {
    let bytes = line.as_bytes();
    let mut p = start;
    while p > 0 && bytes[p - 1].is_ascii_whitespace() {
        p -= 1;
    }
    if p >= 2 && bytes[p - 1] == b':' && bytes[p - 2] == b':' {
        return Base::Path(path_root(line, start).unwrap_or_default());
    }
    if p >= 1 && bytes[p - 1] == b'.' {
        let (name, base) =
            receiver_base(line, p - 1, receivers).unwrap_or_else(|| (String::new(), Base::Unknown));
        return Base::Method(Box::new(base), name);
    }
    Base::Bare
}

/// Collect `name(...)` call sites from a line, each with the path it sits on.
/// `file` identifies the file being scanned, used to resolve type roots.
fn find_calls(line: &str, file: usize) -> Vec<CallSite> {
    let bytes = line.as_bytes();
    let n = bytes.len();
    let mut sites = Vec::new();
    // Closing paren offset to the name and base of the call that ended there,
    // so `style(x).yellow(..)` can see that its receiver is `style(..)`.
    let mut receivers: HashMap<usize, (String, Base)> = HashMap::new();
    let mut i = 0;

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
            if is_macro
                || j >= n
                || bytes[j] != b'('
                || ident.starts_with("r#")
                || NON_CALL_KEYWORDS.contains(&ident)
            {
                continue;
            }
            let base = callee_base(line, start, &receivers);
            if let Some(close) = matching_paren(line, j) {
                receivers.insert(close, (ident.to_string(), base.clone()));
            }
            sites.push(CallSite {
                name: ident.to_string(),
                base,
                file,
            });
            continue;
        }
        i += 1;
    }
    sites
}

/// A function currently being filled while scanning a file.
struct ActiveFn {
    function: Function,
    sites: Vec<CallSite>,
    body_depth: i32,
    opened: bool,
}

/// A function together with the call sites found in its body, before the call
/// sites are matched against the repository's own declarations.
struct Collected {
    function: Function,
    sites: Vec<CallSite>,
}

/// Extract every function together with the calls detected in its body. Only
/// calls that resolve to a function this repository defines are kept, and the
/// name path of the call is what decides that: `style(prompt)` and
/// `crate::ui::style(prompt)` are repository calls, `std::io::stderr()` and
/// `String::new()` are not, however common their names are elsewhere.
pub fn functions(repo: &Repository) -> Result<Vec<Function>> {
    let mut files: Vec<PathBuf> = Vec::new();
    for file in SourceFiles::new(repo.root()) {
        if file.extension().and_then(|e| e.to_str()) == Some("rs") {
            files.push(file);
        }
    }

    let mut collected: Vec<Collected> = Vec::new();
    let mut declared = Declared {
        crates: crate_names(&files, repo.root()),
        ..Default::default()
    };
    for (id, file) in files.iter().enumerate() {
        if let Ok(contents) = fs::read_to_string(file) {
            collected.extend(functions_in(&contents, file, id, &mut declared));
        }
    }

    let mut out = Vec::with_capacity(collected.len());
    for item in collected {
        let mut calls: Vec<String> = Vec::new();
        for site in &item.sites {
            let name = resolved_name(site, &declared);
            if is_repo_call(site, &declared) && !calls.iter().any(|seen| seen == name) {
                calls.push(name.to_string());
            }
        }
        out.push(Function {
            calls,
            ..item.function
        });
    }
    Ok(out)
}

/// The crate names in this repository, read off the `src` component of every
/// source path. `crates/topcoat_ui/src/lib.rs` contributes `topcoat_ui`, which
/// is how a `use topcoat_ui::..` import is told apart from `use console::..`.
fn crate_names(files: &[PathBuf], root: &Path) -> HashSet<String> {
    let mut names = HashSet::new();
    for file in files {
        let parts: Vec<_> = file.components().collect();
        if let Some(position) = parts.iter().position(|part| part.as_os_str() == "src")
            && position > 0
        {
            names.insert(crate_name(
                &parts[position - 1].as_os_str().to_string_lossy(),
            ));
        }
    }
    let root_src = root.join("src");
    if files
        .iter()
        .any(|file| file.parent() == Some(root_src.as_path()))
        && let Some(name) = root.file_name()
    {
        names.insert(crate_name(&name.to_string_lossy()));
    }
    names
}

/// A crate name as Rust spells it: package dashes become underscores.
fn crate_name(package: &str) -> String {
    package.replace('-', "_")
}

fn functions_in(contents: &str, file: &Path, id: usize, declared: &mut Declared) -> Vec<Collected> {
    let mut finished: Vec<Collected> = Vec::new();
    let mut active: Vec<ActiveFn> = Vec::new();
    let mut depth = 0i32;
    // A `use` declaration can wrap over several lines; hold it until the `;`.
    let mut use_declaration = String::new();
    let mut scan = Scan::default();

    for (index, line) in contents.lines().enumerate() {
        // Literals and comments are blanked before anything reads the line, so a
        // `fn` in a doc comment or a `use` inside a string is not mistaken for
        // source. Byte offsets are preserved, so `find_calls` is unaffected.
        let code = strip_noncode(line, &mut scan);
        let delta = brace_delta(&code);
        if let Some((kind, name)) = parse_symbol_line(&code) {
            declared.insert(kind, &name, id);
        }

        let trimmed = code.trim_start();
        if use_declaration.is_empty() && trimmed.starts_with("use ") {
            use_declaration.push_str(trimmed);
        } else if !use_declaration.is_empty() {
            use_declaration.push(' ');
            use_declaration.push_str(trimmed);
        }
        if use_declaration.contains(';') {
            declared.insert_imports(id, &use_declaration);
            use_declaration.clear();
        }

        if let Some(found) = parse_fn(&code) {
            let function = Function {
                name: found.name,
                file: file.to_path_buf(),
                line: index + 1,
                end_line: index + 1,
                signature: line.trim().to_string(),
                params: split_params(&found.params),
                calls: Vec::new(),
            };
            let name = function.name.clone();
            let mut sites = find_calls(&code, id);
            // The declaration line `fn name(` must not count as a self-call.
            sites.retain(|site| site.name != name);
            active.push(ActiveFn {
                function,
                sites,
                // The body opens one brace level below the declaration's depth.
                body_depth: depth + 1,
                opened: code.contains('{'),
            });
        } else if let Some(top) = active.last_mut() {
            top.sites.extend(find_calls(&code, id));
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
                finished.push(Collected {
                    function: top.function,
                    sites: top.sites,
                });
            }
        }
    }

    let last_line = contents.lines().count();
    while let Some(mut top) = active.pop() {
        top.function.end_line = last_line;
        finished.push(Collected {
            function: top.function,
            sites: top.sites,
        });
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

    /// The names `find_calls` reports for `line`, given a repository that
    /// declares `declared`.
    fn repo_calls(line: &str, declared: &Declared) -> Vec<String> {
        repo_calls_in(line, declared, 0)
    }

    /// As `repo_calls`, but for a call site in file `file`.
    fn repo_calls_in(line: &str, declared: &Declared, file: usize) -> Vec<String> {
        find_calls(line, file)
            .into_iter()
            .filter(|site| is_repo_call(site, declared))
            .map(|site| resolved_name(&site, declared).to_string())
            .collect()
    }

    /// A repository that declares a `style` helper returning a `Style` with a
    /// `yellow` method, plus the std-shaped names (`trim`, `flush`, `ok`,
    /// `as_str`, `map_err`, `stderr`) that collide with local functions, and a
    /// `ui` module.
    fn topcoat_like() -> Declared {
        let mut declared = Declared::default();
        for name in [
            "style", "yellow", "render", "new", "trim", "flush", "ok", "as_str", "map_err",
            "stderr",
        ] {
            declared.insert(SymbolKind::Function, name, 0);
        }
        declared.insert(SymbolKind::Struct, "Style", 0);
        declared.insert(SymbolKind::Mod, "ui", 0);
        declared
    }

    #[test]
    fn call_sites() {
        let sites = find_calls("let out = render(model, &opts);", 0);
        assert!(sites.iter().any(|s| s.name == "render"));

        let sites = find_calls("let _ = vec![foo(1)];", 0);
        assert!(sites.iter().any(|s| s.name == "foo"));
    }

    #[test]
    fn bare_calls_are_repository_candidates() {
        let declared = topcoat_like();
        assert_eq!(
            repo_calls("let x = style(prompt).yellow();", &declared),
            ["style", "yellow"]
        );
        assert_eq!(
            repo_calls("crate::ui::style(prompt);", &declared),
            ["style"]
        );
        assert_eq!(repo_calls("self.render();", &declared), ["render"]);
    }

    #[test]
    fn foreign_paths_are_dropped_even_when_the_name_exists() {
        let declared = topcoat_like();
        assert!(repo_calls("std::io::stderr().flush().ok();", &declared).is_empty());
        assert!(repo_calls("let s = String::new();", &declared).is_empty());
        assert!(repo_calls("std::io::stdin().read_line(&mut s);", &declared).is_empty());
    }

    #[test]
    fn std_methods_on_unknown_receivers_are_dropped() {
        let declared = topcoat_like();
        // `trim`, `flush` and `ok` are all real functions in this repository,
        // but they are called on std values here.
        assert!(repo_calls("input.trim().to_ascii_lowercase();", &declared).is_empty());
        assert!(repo_calls("let n = input.len();", &declared).is_empty());
    }

    #[test]
    fn repository_methods_on_local_receivers_are_kept() {
        let declared = topcoat_like();
        // Not a std method name, so the receiver's unknown type does not
        // disqualify the call.
        assert_eq!(repo_calls("config.render();", &declared), ["render"]);
    }

    #[test]
    fn names_the_repo_never_declares_are_dropped() {
        let declared = topcoat_like();
        assert!(repo_calls("let n = input.unwrap();", &declared).is_empty());
        assert!(repo_calls("read_answer().expect(\"boom\");", &declared).is_empty());
    }

    #[test]
    fn associated_calls_on_repository_types_are_kept() {
        let declared = topcoat_like();
        assert_eq!(repo_calls("let s = Style::new();", &declared), ["new"]);
    }

    #[test]
    fn a_type_alias_elsewhere_does_not_capture_std_paths() {
        let mut declared = topcoat_like();
        // Another module aliases the std String; the call site is not in that
        // module, so `String::new(..)` is still the std constructor.
        declared.insert(SymbolKind::Type, "String", 7);
        assert!(repo_calls_in("let s = String::new();", &declared, 3).is_empty());
        // In the aliasing module itself the name is genuinely local.
        assert_eq!(
            repo_calls_in("let s = String::new();", &declared, 7),
            ["new"]
        );
    }

    /// `topcoat_like`, plus the imports a file that calls `style` would have.
    fn with_imports(imports: &str) -> Declared {
        let mut declared = topcoat_like();
        declared.crates.insert("topcoat_ui".to_string());
        declared.insert_imports(0, imports);
        declared
    }

    #[test]
    fn imports_from_outside_the_repository_are_dropped() {
        // `style` is declared in this repository, but the file imports it from
        // the `console` crate, so the call is not the repository's.
        let declared = with_imports("use console::style;");
        assert!(repo_calls("let s = style(prompt);", &declared).is_empty());
        // The chain on top of it is external too.
        assert!(repo_calls("let s = style(prompt).yellow();", &declared).is_empty());
    }

    #[test]
    fn imports_from_this_workspace_are_kept() {
        let declared = with_imports("use topcoat_ui::manage::render;");
        assert_eq!(repo_calls("render(model);", &declared), ["render"]);

        let declared = with_imports("use crate::ui::style;");
        assert_eq!(repo_calls("let s = style(prompt);", &declared), ["style"]);

        // The alias resolves to the function it renames, so the edge is
        // recorded under the name the repository declares.
        let declared = with_imports("use super::render as draw;");
        assert_eq!(repo_calls("draw(model);", &declared), ["render"]);
    }

    #[test]
    fn paths_through_an_imported_module_are_kept() {
        // `Package` is declared in another workspace crate and imported here,
        // so `Package::locate(..)` reaches repository code.
        let mut declared = with_imports("use topcoat_ui::manage::{self, AddOptions, Package};");
        declared.insert(SymbolKind::Function, "locate", 9);
        declared.insert(SymbolKind::Function, "add", 9);
        assert_eq!(
            repo_calls("let p = Package::locate(root)?;", &declared),
            ["locate"]
        );
        assert_eq!(
            repo_calls("match manage::add(&p, &o)?;", &declared),
            ["add"]
        );
        // The same path shape rooted in an external import is not.
        let mut declared = with_imports("use console::manage::{self, Package};");
        declared.insert(SymbolKind::Function, "locate", 9);
        assert!(repo_calls("let p = Package::locate(root)?;", &declared).is_empty());
    }

    #[test]
    fn a_name_the_file_does_not_import_stays_local() {
        // No `use` for `style`, so the call is a function in this module.
        let declared = with_imports("use console::Style;");
        assert_eq!(repo_calls("let s = style(prompt);", &declared), ["style"]);
    }

    /// A `use` declaration as its imports, for readable assertions: `name@root`,
    /// or `alias@root from origin` when the import renames what it brings in.
    fn imports(declaration: &str) -> String {
        parse_use(declaration)
            .iter()
            .map(|(name, imported)| {
                if name == &imported.origin {
                    format!("{name}@{}", imported.root)
                } else {
                    format!("{name}@{} from {}", imported.root, imported.origin)
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    #[test]
    fn parses_use_declarations() {
        assert_eq!(imports("use console::style;"), "style@console");
        assert_eq!(
            imports("use console::{style, Style, Term};"),
            "style@console, Style@console, Term@console"
        );
        assert_eq!(
            imports("pub use topcoat_ui::{manage::Package, AddOutcome};"),
            "Package@topcoat_ui, AddOutcome@topcoat_ui"
        );
        assert_eq!(imports("use std::io::Write as W;"), "W@std from Write");
        assert_eq!(
            imports("use super::render as draw;"),
            "draw@super from render"
        );
        // A glob binds no name. A plain module import is recorded: it may be a
        // re-exported function, and a module name can never match a call site.
        assert_eq!(imports("use console::*;"), "");
        assert_eq!(imports("use crate::ui;"), "ui@crate");
        // `self` binds the module itself, which is what makes `manage::add(..)`
        // a path into the repository.
        assert_eq!(
            imports("use topcoat_ui::manage::{self, AddOptions};"),
            "manage@topcoat_ui, AddOptions@topcoat_ui"
        );
    }

    #[test]
    fn reads_crate_names_off_source_paths() {
        let root = Path::new("/repo");
        let files = vec![
            PathBuf::from("/repo/crates/topcoat-cli/src/main.rs"),
            PathBuf::from("/repo/crates/topcoat_ui/src/lib.rs"),
            PathBuf::from("/repo/crates/topcoat-ui/tests/it.rs"),
        ];
        let names = crate_names(&files, root);
        assert!(names.contains("topcoat_cli"));
        assert!(names.contains("topcoat_ui"));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn bracket_inside_a_string_does_not_hide_the_receiver() {
        let declared = topcoat_like();
        let sites = find_calls("let n = render(\")\").count();", 0);
        let names: Vec<_> = sites.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"render"), "{names:?}");
        assert_eq!(
            sites.iter().find(|s| s.name == "count").map(|s| &s.base),
            Some(&Base::Method(Box::new(Base::Bare), "render".to_string())),
            "the receiver is the `render(..)` call, not a stray bracket"
        );
        let _ = declared;
    }
    #[test]
    fn a_keyword_table_declares_nothing() {
        let line = r#"    "else", "enum", "extern", "fn", "for", "if", "impl","#;
        assert!(parse_fn(line).is_none(), "phantom fn from a string literal");
        assert_eq!(parse_symbol_line(line), None);
    }

    #[test]
    fn a_doc_comment_declares_nothing() {
        let line = "/// Read a `fn` declaration, capturing the parameter list.";
        assert!(parse_fn(line).is_none(), "phantom fn from a doc comment");
    }

    #[test]
    fn a_trailing_comment_declares_nothing() {
        let line = "    depth += delta; // the `fn name(` line must not self-call";
        assert!(
            parse_fn(line).is_none(),
            "phantom fn from a trailing comment"
        );
        assert_eq!(parse_symbol_line(line), None);
    }

    #[test]
    fn a_signature_inside_a_string_declares_nothing() {
        let line = r#"            signature: "fn parse() {".to_string(),"#;
        assert!(parse_fn(line).is_none(), "phantom fn from a string literal");
    }

    #[test]
    fn a_struct_named_in_a_string_declares_nothing() {
        assert_eq!(
            parse_symbol_line(r#"    let src = "struct Point { x: u8 }";"#),
            None
        );
    }

    #[test]
    fn real_declarations_still_parse() {
        assert_eq!(
            parse_fn("pub fn discover() -> crate::Result<Self> {")
                .unwrap()
                .name,
            "discover"
        );
        // A string elsewhere on the line must not hide a real declaration.
        let found = parse_fn(r#"fn label(kind: &str) -> &str { "fn" }"#).unwrap();
        assert_eq!(found.name, "label");
        assert_eq!(found.params, "kind: &str");
    }

    #[test]
    fn raw_strings_declare_nothing() {
        // A raw string holding quotes is what breaks a plain-string scanner: it
        // closes on the inner quote and reads the rest as code. Written with
        // escapes so this line is not itself a raw string.
        assert!(
            parse_fn("let line = r#\"  \"a\", \"fn\", \"for\",  \"#;").is_none(),
            "quotes inside a raw string resynchronised the scanner"
        );
        assert_eq!(
            parse_symbol_line("let t = r#\"  \"x\", \"struct\", \"Point\",  \"#;"),
            None
        );
        // Hashes must match: `"#` inside an `r##"..."##` does not close it.
        assert!(parse_fn("let q = r##\"a \"# fn inside() {\"##;").is_none());
        // A bare `r` that is only part of an identifier must not start one.
        assert_eq!(
            parse_fn("fn user_repr(r: u8) -> String {").unwrap().name,
            "user_repr"
        );
    }

    #[test]
    fn a_char_literal_is_not_a_string() {
        // `'"'` must not open a string: `string_end` would find no close, blank
        // the rest of the line, and take the line's `{` with it.
        let stripped = strip_noncode("    if ch == '\"' { go(); }", &mut Scan::default());
        assert!(stripped.contains('{'), "brace lost: {stripped:?}");
        assert!(stripped.contains("go"), "call lost: {stripped:?}");
        // The byte-for-byte contract still holds.
        assert_eq!(stripped.len(), "    if ch == '\"' { go(); }".len());
    }

    #[test]
    fn a_byte_char_literal_is_not_a_string() {
        let stripped = strip_noncode("        b'\"' => open(),", &mut Scan::default());
        assert!(stripped.contains("open"), "call lost: {stripped:?}");
    }

    #[test]
    fn a_lifetime_is_not_a_char_literal() {
        let stripped = strip_noncode(
            "fn f<'a>(x: &'a str) -> &'static str { go(x) }",
            &mut Scan::default(),
        );
        assert!(stripped.contains("go"), "call lost: {stripped:?}");
        assert_eq!(parse_fn("fn f<'a>(x: &'a str) {").unwrap().name, "f");
    }

    #[test]
    fn block_comments_declare_nothing() {
        let src = "/*\nfn commented_out() {\n    helper();\n}\n*/\nfn real() {}\n";
        let mut declared = Declared::default();
        let found = functions_in(src, std::path::Path::new("x.rs"), 0, &mut declared);
        let names: Vec<&str> = found.iter().map(|c| c.function.name.as_str()).collect();
        assert_eq!(names, vec!["real"], "block comment leaked a function");
    }

    #[test]
    fn nested_block_comments_declare_nothing() {
        // Rust block comments nest, so the state cannot be a bool.
        let src = "/* outer\n/* inner */\nfn phantom() {\n    ghost();\n}\n*/\nfn real() {}\n";
        let mut declared = Declared::default();
        let found = functions_in(src, std::path::Path::new("x.rs"), 0, &mut declared);
        let names: Vec<&str> = found.iter().map(|c| c.function.name.as_str()).collect();
        assert_eq!(names, vec!["real"], "nested comment leaked a function");
    }

    #[test]
    fn multi_line_raw_strings_declare_nothing() {
        // String state must carry across lines the way block-comment state does.
        let src = "fn holder() {\n    let s = r#\"\nfn phantom() {\n    ghost();\n}\n\"#;\n}\n";
        let mut declared = Declared::default();
        let found = functions_in(src, std::path::Path::new("x.rs"), 0, &mut declared);
        let names: Vec<&str> = found.iter().map(|c| c.function.name.as_str()).collect();
        assert_eq!(names, vec!["holder"], "raw string leaked a function");
    }

    #[test]
    fn a_char_literal_does_not_truncate_its_function() {
        let src = "fn outer(ch: char) {\n    if ch == '\"' {\n        inner_a();\n    }\n    inner_b();\n}\n";
        let mut declared = Declared::default();
        let found = functions_in(src, std::path::Path::new("x.rs"), 0, &mut declared);
        let outer = found
            .iter()
            .find(|c| c.function.name == "outer")
            .expect("outer not parsed");
        let called: Vec<&str> = outer.sites.iter().map(|s| s.name.as_str()).collect();
        assert!(called.contains(&"inner_a"), "{called:?}");
        assert!(
            called.contains(&"inner_b"),
            "brace desync dropped the tail: {called:?}"
        );
        assert_eq!(outer.function.end_line, 6, "span closed early");
    }
}
