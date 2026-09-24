use std::fmt;

/// Top-level error type shared across all trak crates.
#[derive(Debug)]
pub enum Error {
    /// A filesystem or I/O failure.
    Io(std::io::Error),
    /// Nothing matched a user-supplied name/query.
    NotFound(String),
    /// The repository could not be located or examined.
    Repo(String),
    /// An external process (e.g. git) failed.
    Command(String),
    /// Any other failure.
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(err) => write!(f, "{err}"),
            Error::NotFound(what) => write!(f, "nothing found for {what}"),
            Error::Repo(msg) => write!(f, "{msg}"),
            Error::Command(msg) => write!(f, "{msg}"),
            Error::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}

/// Convenience result alias.
pub type Result<T> = std::result::Result<T, Error>;
