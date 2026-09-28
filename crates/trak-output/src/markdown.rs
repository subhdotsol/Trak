use crate::Report;

pub fn render(report: &Report) -> String {
    let mut out = String::new();
    out.push_str(&format!("## {}\n\n", report.title));
    if let Some(summary) = &report.summary {
        out.push_str(&format!("> {summary}\n\n"));
    }

    if !report.hits.is_empty() {
        out.push_str("### Results\n\n");
        for hit in &report.hits {
            let kind = hit
                .kind
                .as_deref()
                .map(|k| format!(" _{k}_"))
                .unwrap_or_default();
            out.push_str(&format!(
                "- `{}:{}`{kind}: {}\n",
                hit.file,
                hit.line,
                hit.text.trim()
            ));
        }
        out.push('\n');
    }

    if !report.chains.is_empty() {
        out.push_str("### Path\n\n");
        for chain in &report.chains {
            out.push_str(&format!("- `{}`\n", chain.join("` → `")));
        }
        out.push('\n');
    }

    if !report.edges.is_empty() {
        out.push_str("### Graph\n\n");
        for edge in &report.edges {
            out.push_str(&format!("- `{}` → `{}`\n", edge.from, edge.to));
        }
    }
    out
}
