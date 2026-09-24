use crate::Report;

pub fn render(report: &Report) -> String {
    let mut out = String::new();
    out.push_str(&report.title);
    out.push('\n');
    if let Some(summary) = &report.summary {
        out.push_str(summary);
        out.push('\n');
    }

    for hit in &report.hits {
        let location = format!("{}:{}", hit.file, hit.line);
        match &hit.kind {
            Some(kind) => out.push_str(&format!("{location}  [{kind}]  {}\n", hit.text.trim())),
            None => out.push_str(&format!("{location}: {}\n", hit.text.trim())),
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
