use crate::Report;

fn quote(label: &str) -> String {
    label.replace('"', "'")
}

pub fn render(report: &Report) -> String {
    let mut out = String::from("```mermaid\ngraph TD\n");
    for node in &report.nodes {
        let label = quote(&node.label);
        out.push_str(&format!("  {}[\"{label}\"]\n", node.id));
    }
    for edge in &report.edges {
        out.push_str(&format!("  {} --> {}\n", edge.from, edge.to));
    }
    out.push_str("```\n");
    out
}
