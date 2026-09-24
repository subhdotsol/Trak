//! Output rendering for `trak` results: terminal, JSON, Markdown, Mermaid, DOT.

pub mod dot;
pub mod json;
pub mod markdown;
pub mod mermaid;
pub mod terminal;

use serde::Serialize;

/// Output format selectable via the CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Terminal,
    Json,
    Markdown,
    Mermaid,
    Dot,
}

/// A result report that any renderer can consume. Commands build this from
/// analysis output; renderers turn it into terminal text, JSON, etc.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Report {
    pub title: String,
    pub summary: Option<String>,
    pub hits: Vec<Hit>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

/// A plain `file:line` item with optional text.
#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub file: String,
    pub line: usize,
    pub text: String,
    /// E.g. "definition", "caller" — omitted in JSON when unknown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// A graph node (used for the call/architecture graph).
#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// A graph edge from one node id to another.
#[derive(Debug, Clone, Serialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl Report {
    pub fn new(title: impl Into<String>) -> Self {
        Report {
            title: title.into(),
            ..Report::default()
        }
    }

    pub fn summary(&mut self, summary: impl Into<String>) -> &mut Self {
        self.summary = Some(summary.into());
        self
    }

    pub fn hit(
        &mut self,
        file: impl Into<String>,
        line: usize,
        text: impl Into<String>,
        kind: Option<&str>,
    ) -> &mut Self {
        self.hits.push(Hit {
            file: file.into(),
            line,
            text: text.into(),
            kind: kind.map(str::to_string),
        });
        self
    }

    /// Add a node, dropping duplicates with the same id.
    pub fn node(
        &mut self,
        id: impl Into<String>,
        label: impl Into<String>,
        kind: Option<&str>,
    ) -> &mut Self {
        let id = id.into();
        if !self.nodes.iter().any(|n| n.id == id) {
            self.nodes.push(Node {
                id,
                label: label.into(),
                kind: kind.map(str::to_string),
            });
        }
        self
    }

    pub fn edge(&mut self, from: impl Into<String>, to: impl Into<String>) -> &mut Self {
        self.edges.push(Edge {
            from: from.into(),
            to: to.into(),
            label: None,
        });
        self
    }

    pub fn render(&self, format: Format) -> String {
        match format {
            Format::Terminal => terminal::render(self),
            Format::Json => json::render(self),
            Format::Markdown => markdown::render(self),
            Format::Mermaid => mermaid::render(self),
            Format::Dot => dot::render(self),
        }
    }
}
