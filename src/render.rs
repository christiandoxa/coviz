use anyhow::Result;

use crate::model::{Analysis, Call, Function};

/// Render an analysis graph in Graphviz DOT format.
pub fn render_dot(analysis: &Analysis) -> String {
    let mut functions = analysis.functions.clone();
    functions.sort_by(|left, right| left.id.cmp(&right.id));

    let mut calls = analysis.calls.clone();
    calls.sort_by(|left, right| {
        left.caller
            .cmp(&right.caller)
            .then(left.callee.cmp(&right.callee))
            .then(left.file.cmp(&right.file))
            .then(left.line.cmp(&right.line))
    });

    let mut output = String::from("digraph coviz {\n  rankdir=LR;\n");
    for function in &functions {
        output.push_str(&format!(
            "  \"{}\" [label=\"{}\"];\n",
            escape_dot(&function.id),
            escape_dot(&function_label(function)),
        ));
    }

    for call in &calls {
        output.push_str(&format!(
            "  \"{}\" -> \"{}\" [label=\"{}\"];\n",
            escape_dot(&call.caller),
            escape_dot(&call.callee),
            escape_dot(&call_label(call)),
        ));
    }

    output.push_str("}\n");
    output
}

/// Render an analysis graph as pretty JSON.
pub fn render_json(analysis: &Analysis) -> Result<String> {
    Ok(serde_json::to_string_pretty(analysis)?)
}

/// Render the browser viewer used by `coviz quick`.
pub fn render_html(analysis: &Analysis) -> String {
    QUICK_VIEWER_TEMPLATE
        .replace("__FUNCTION_COUNT__", &analysis.functions.len().to_string())
        .replace("__CALL_COUNT__", &analysis.calls.len().to_string())
}

fn function_label(function: &Function) -> String {
    format!("{}\n{}:{}", function.name, function.file, function.line)
}

fn call_label(call: &Call) -> String {
    format!("{}:{}", call.file, call.line)
}

fn escape_dot(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

const QUICK_VIEWER_TEMPLATE: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>coviz quick</title>
  <style>
    :root {
      --bg: #f5f1e8;
      --ink: #1c1b17;
      --muted: #676056;
      --panel: #fffaf0;
      --line: #d6c7ad;
      --accent: #0f766e;
      --edge: #9a6a2f;
    }

    * {
      box-sizing: border-box;
    }

    body {
      margin: 0;
      min-height: 100vh;
      color: var(--ink);
      background:
        radial-gradient(circle at top left, rgba(15, 118, 110, 0.18), transparent 34rem),
        linear-gradient(135deg, #fbf4df 0%, var(--bg) 45%, #e8dcc8 100%);
      font-family: ui-serif, Georgia, Cambria, "Times New Roman", serif;
    }

    header {
      padding: 2rem clamp(1rem, 4vw, 4rem) 1rem;
      display: flex;
      align-items: end;
      justify-content: space-between;
      gap: 1rem;
      border-bottom: 1px solid var(--line);
    }

    h1 {
      margin: 0;
      font-size: clamp(2.5rem, 7vw, 6rem);
      letter-spacing: -0.08em;
      line-height: 0.9;
    }

    .summary {
      color: var(--muted);
      font-size: 1rem;
      text-align: right;
    }

    main {
      padding: 1rem clamp(1rem, 4vw, 4rem) 3rem;
    }

    .toolbar {
      display: flex;
      flex-wrap: wrap;
      gap: 0.75rem;
      align-items: center;
      margin: 1rem 0;
    }

    input {
      width: min(32rem, 100%);
      border: 1px solid var(--line);
      border-radius: 999px;
      background: rgba(255, 250, 240, 0.78);
      color: var(--ink);
      padding: 0.8rem 1rem;
      font: inherit;
    }

    a {
      color: var(--accent);
      font-weight: 700;
    }

    #canvas {
      position: relative;
      min-height: 42rem;
      overflow: auto;
      border: 1px solid var(--line);
      border-radius: 1.25rem;
      background: rgba(255, 250, 240, 0.6);
      box-shadow: 0 1.5rem 5rem rgba(28, 27, 23, 0.08);
    }

    svg {
      position: absolute;
      inset: 0;
      min-width: 100%;
      min-height: 100%;
      pointer-events: none;
    }

    .node {
      position: absolute;
      width: 12rem;
      min-height: 5rem;
      padding: 0.9rem;
      border: 1px solid var(--line);
      border-radius: 1rem;
      background: var(--panel);
      box-shadow: 0 1rem 2.4rem rgba(28, 27, 23, 0.12);
      transition: opacity 120ms ease, transform 120ms ease;
    }

    .node:hover {
      transform: translateY(-0.2rem);
    }

    .node.hidden {
      opacity: 0.12;
    }

    .name {
      font: 700 1rem ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      overflow-wrap: anywhere;
    }

    .meta {
      margin-top: 0.5rem;
      color: var(--muted);
      font-size: 0.85rem;
      overflow-wrap: anywhere;
    }

    .empty {
      padding: 2rem;
      color: var(--muted);
    }

    @media (max-width: 720px) {
      header {
        align-items: start;
        flex-direction: column;
      }

      .summary {
        text-align: left;
      }

      #canvas {
        min-height: 32rem;
      }
    }
  </style>
</head>
<body>
  <header>
    <h1>coviz</h1>
    <div class="summary">__FUNCTION_COUNT__ functions / __CALL_COUNT__ calls</div>
  </header>
  <main>
    <div class="toolbar">
      <input id="filter" type="search" placeholder="Filter by function or file" autocomplete="off">
      <a href="/graph.json">graph.json</a>
      <a href="/graph.dot">graph.dot</a>
    </div>
    <section id="canvas" aria-label="Call graph">
      <div class="empty">Loading graph...</div>
    </section>
  </main>
  <script>
    const canvas = document.querySelector("#canvas");
    const filter = document.querySelector("#filter");

    function escapeHtml(value) {
      return String(value).replace(/[&<>"']/g, (char) => ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        "\"": "&quot;",
        "'": "&#39;"
      }[char]));
    }

    function draw(data) {
      const nodes = data.functions;
      const calls = data.calls;

      if (!nodes.length) {
        canvas.innerHTML = '<div class="empty">No supported functions found.</div>';
        return;
      }

      const width = Math.max(960, nodes.length * 180);
      const height = Math.max(620, Math.ceil(nodes.length / 8) * 520);
      const centerX = width / 2;
      const centerY = height / 2;
      const radiusX = Math.max(260, width * 0.34);
      const radiusY = Math.max(190, height * 0.28);
      const positions = new Map();

      canvas.style.minWidth = `${width}px`;
      canvas.style.minHeight = `${height}px`;
      canvas.innerHTML = `
        <svg viewBox="0 0 ${width} ${height}" width="${width}" height="${height}" role="img" aria-label="Call graph edges">
          <defs>
            <marker id="arrow" markerWidth="10" markerHeight="10" refX="8" refY="3" orient="auto" markerUnits="strokeWidth">
              <path d="M0,0 L0,6 L9,3 z" fill="var(--edge)"></path>
            </marker>
          </defs>
          <g id="edges"></g>
        </svg>
      `;

      nodes.forEach((node, index) => {
        const angle = (-Math.PI / 2) + (Math.PI * 2 * index / nodes.length);
        const x = centerX + Math.cos(angle) * radiusX;
        const y = centerY + Math.sin(angle) * radiusY;
        positions.set(node.id, { x, y, node });

        const element = document.createElement("article");
        element.className = "node";
        element.dataset.search = `${node.name} ${node.file}`.toLowerCase();
        element.style.left = `${x - 96}px`;
        element.style.top = `${y - 40}px`;
        element.innerHTML = `
          <div class="name">${escapeHtml(node.name)}</div>
          <div class="meta">${escapeHtml(node.file)}:${node.line}</div>
        `;
        canvas.appendChild(element);
      });

      const edgeLayer = canvas.querySelector("#edges");
      calls.forEach((call) => {
        const caller = positions.get(call.caller);
        const callee = positions.get(call.callee);
        if (!caller || !callee) {
          return;
        }

        const line = document.createElementNS("http://www.w3.org/2000/svg", "line");
        line.setAttribute("x1", caller.x);
        line.setAttribute("y1", caller.y);
        line.setAttribute("x2", callee.x);
        line.setAttribute("y2", callee.y);
        line.setAttribute("stroke", "var(--edge)");
        line.setAttribute("stroke-width", "2");
        line.setAttribute("stroke-opacity", "0.72");
        line.setAttribute("marker-end", "url(#arrow)");
        edgeLayer.appendChild(line);
      });
    }

    filter.addEventListener("input", () => {
      const query = filter.value.trim().toLowerCase();
      document.querySelectorAll(".node").forEach((node) => {
        node.classList.toggle("hidden", query && !node.dataset.search.includes(query));
      });
    });

    fetch("/graph.json")
      .then((response) => response.json())
      .then(draw)
      .catch((error) => {
        canvas.innerHTML = `<div class="empty">Failed to load graph: ${escapeHtml(error.message)}</div>`;
      });
  </script>
</body>
</html>
"##;

#[cfg(test)]
mod tests {
    use super::{render_dot, render_html, render_json};
    use crate::model::{Analysis, Function};

    #[test]
    fn renders_isolated_function_as_dot_node() {
        let analysis = Analysis {
            functions: vec![Function {
                id: "f0".to_string(),
                name: "main".to_string(),
                file: "main.go".to_string(),
                line: 1,
            }],
            calls: vec![],
        };

        let dot = render_dot(&analysis);
        assert!(dot.contains("\"f0\" [label=\"main\\nmain.go:1\"]"));
    }

    #[test]
    fn renders_json() {
        let json = render_json(&Analysis::default()).unwrap();
        assert!(json.contains("\"functions\""));
        assert!(json.contains("\"calls\""));
    }

    #[test]
    fn renders_quick_html() {
        let html = render_html(&Analysis::default());
        assert!(html.contains("<title>coviz quick</title>"));
        assert!(html.contains("0 functions / 0 calls"));
        assert!(html.contains("graph.json"));
    }
}
