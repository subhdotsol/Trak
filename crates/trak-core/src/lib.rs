//! Shared foundation for the `trak` codebase navigator.

pub mod error;
pub mod repo;
pub mod symbol;

pub use error::{Error, Result};
pub use repo::{
    IGNORED_DIRS, Repository, SOURCE_EXTENSIONS, SourceFiles, is_fixture_path, is_source_file,
};
pub use symbol::{Function, Symbol, SymbolKind};
