use std::collections::BTreeMap;
use std::io::IsTerminal;

use crate::Report;

const RESET: &str = "\x1b[0m";
const BLUE: &str = "\x1b[34m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";

/// Enable ANSI colors on TTYs; honor the standard `NO_COLOR` opt-out.
fn colors_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}

/// Percent-encode a path so it is a valid `file://` URI for OSC 8 hyperlinks.
fn encode_path(path: &str) -> String {
    let mut out = String::new();
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                out.push(byte as char);
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Render a path: blue, clickable via OSC 8, when the terminal supports it.
fn render_path(path: &str, href: Option<&str>) -> String {
    let color = colors_enabled();
    if color {
        let colored = format!("{BLUE}{path}{RESET}");
        match href {
            Some(target) => {
                let uri = format!("file://localhost{}", encode_path(target));
                format!("\x1b]8;;{uri}\x1b\\{colored}\x1b]8;;\x1b\\")
            }
            None => colored,
        }
    } else {
        path.to_string()
    }
}

/// Render the line number in green.
fn render_line(line: usize) -> String {
    if colors_enabled() {
        format!("{GREEN}{line}{RESET}:")
    } else {
        format!("{line}:")
    }
}

/// Highlight case-insensitive occurrences of `needle` in `text` in red.
fn render_text(text: &str, needle: Option<&str>) -> String {
    let text = text.trim();
    let Some(needle) = needle
        .filter(|n| !n.is_empty())
        .filter(|_| colors_enabled())
    else {
        return text.to_string();
    };
    let lower = text.to_lowercase();
    let needle_lower = needle.to_lowercase();
    if !lower.contains(&needle_lower) {
        return text.to_string();
    }
    let mut out = String::new();
    let mut search_from = 0;
    let mut matched = 0;
    while let Some(offset) = lower[search_from..].find(&needle_lower) {
        let start = search_from + offset;
        let end = start + needle.len();
        out.push_str(&text[search_from..start]);
        out.push_str(&format!("{RED}{}{RESET}", &text[start..end]));
        search_from = end;
        matched += 1;
    }
    out.push_str(&text[search_from..]);
    if matched > 0 { out } else { text.to_string() }
}

pub fn render(report: &Report) -> String {
    let mut out = String::new();
    out.push_str(&report.title);
    out.push('\n');
    if let Some(summary) = &report.summary {
        out.push_str(summary);
        out.push('\n');
    }

    if !report.hits.is_empty() {
        let mut groups: BTreeMap<&str, Vec<&crate::Hit>> = BTreeMap::new();
        for hit in &report.hits {
            groups.entry(hit.file.as_str()).or_default().push(hit);
        }
        for (file, hits) in &groups {
            out.push('\n');
            let href = hits.iter().find_map(|h| h.href.as_deref());
            out.push_str(&render_path(file, href));
            out.push('\n');
            for hit in hits {
                let mut entry = render_line(hit.line);
                entry.push(' ');
                let (text, kind) = match &hit.kind {
                    Some(kind) => (
                        render_text(&hit.text, report.highlight.as_deref()),
                        Some(kind.as_str()),
                    ),
                    None => (render_text(&hit.text, report.highlight.as_deref()), None),
                };
                entry.push_str(&text);
                if let Some(kind) = kind {
                    entry.push_str(&format!("  [{kind}]"));
                }
                out.push_str(&entry);
                out.push('\n');
            }
        }
    }

    if !report.nodes.is_empty() {
        out.push_str("\nnodes:\n");
        for node in &report.nodes {
            out.push_str(&format!("  {}\n", node.label));
        }
    }

    if !report.edges.is_empty() {
        out.push('\n');
        for edge in &report.edges {
            out.push_str(&format!("{} → {}\n", edge.from, edge.to));
        }
    }
    out
}
