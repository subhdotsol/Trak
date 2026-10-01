use clap::{Parser, Subcommand, ValueEnum};

/// Output formats that commands can render to.
#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Terminal,
    Json,
    Markdown,
    Mermaid,
    Dot,
}

impl From<OutputFormat> for trak_output::Format {
    fn from(value: OutputFormat) -> Self {
        match value {
            OutputFormat::Terminal => trak_output::Format::Terminal,
            OutputFormat::Json => trak_output::Format::Json,
            OutputFormat::Markdown => trak_output::Format::Markdown,
            OutputFormat::Mermaid => trak_output::Format::Mermaid,
            OutputFormat::Dot => trak_output::Format::Dot,
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "trak",
    version,
    about = "Navigate and trace unfamiliar codebases"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Search the repository
    Search {
        /// Text or pattern to search for
        query: String,

        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
    },
    /// Show information about a symbol
    Symbol {
        /// Symbol name
        name: String,

        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
    },
    /// Show information about a function
    Fn {
        /// Function name
        name: String,

        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
    },
    /// Find functions that call this function
    Callers {
        /// Function name
        name: String,

        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,

        /// Render the reverse call tree instead
        #[arg(long)]
        tree: bool,
    },
    /// Find functions called by this function
    Calls {
        /// Function name
        name: String,

        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,

        /// Render the forward call tree instead
        #[arg(long)]
        tree: bool,
    },
    /// Find a path between functions
    Path {
        /// Starting function
        from: String,

        /// Target function
        to: String,

        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
    },
    /// Find references to a symbol
    Refs {
        /// Symbol name
        name: String,

        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
    },
    /// Analyze the impact of changing a function
    Impact {
        /// Function name
        name: String,

        #[arg(long, value_enum, default_value = "terminal")]
        format: OutputFormat,
    },
    /// Show repository architecture
    Architecture,
    /// Show Git history
    History {
        /// Symbol or function
        name: String,
    },
    /// Show blame for a file location
    Blame {
        /// File, optionally with `:line` suffix
        loc: String,
    },
    /// Show the working tree diff
    Diff,
    /// Show code related to a search query
    Relevant {
        /// Text to search for
        query: String,
    },
    /// Find where a log message originates
    Log {
        /// Log message text
        message: String,
    },
    /// Find where an error message originates
    Error {
        /// Error message text
        message: String,
    },
    /// Find where a panic message originates
    Panic {
        /// Panic message text
        message: String,
    },
    /// Open a symbol or function in your editor
    Open {
        /// Symbol or function name
        name: String,
    },
}
