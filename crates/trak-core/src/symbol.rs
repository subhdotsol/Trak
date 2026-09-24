use std::fmt;
use std::path::PathBuf;

/// The kind of a named item found in the codebase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Trait,
    Type,
    Const,
    Static,
    Mod,
    Macro,
    Union,
    Impl,
}

impl SymbolKind {
    /// Short source-keyword label for the kind.
    pub fn label(self) -> &'static str {
        match self {
            SymbolKind::Function => "fn",
            SymbolKind::Struct => "struct",
            SymbolKind::Enum => "enum",
            SymbolKind::Trait => "trait",
            SymbolKind::Type => "type",
            SymbolKind::Const => "const",
            SymbolKind::Static => "static",
            SymbolKind::Mod => "mod",
            SymbolKind::Macro => "macro",
            SymbolKind::Union => "union",
            SymbolKind::Impl => "impl",
        }
    }
}

impl fmt::Display for SymbolKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.label())
    }
}

/// A named item located at a file and line.
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file: PathBuf,
    pub line: usize,
}

impl Symbol {
    pub fn new(name: impl Into<String>, kind: SymbolKind, file: PathBuf, line: usize) -> Self {
        Symbol {
            name: name.into(),
            kind,
            file,
            line,
        }
    }
}

/// A function together with the call sites detected in its body.
#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub file: PathBuf,
    /// First line of the declaration.
    pub line: usize,
    /// Last line of the body (inclusive), for span queries.
    pub end_line: usize,
    /// The trimmed declaration line.
    pub signature: String,
    /// Parameter names extracted from the signature.
    pub params: Vec<String>,
    /// Smallest unique set of function names called inside the body.
    pub calls: Vec<String>,
}
