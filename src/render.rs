use std::collections::BTreeMap;

use anyhow::Result;

use crate::model::{Analysis, Call, Function};

/// Render an analysis graph in Graphviz DOT format.
pub fn render_dot(analysis: &Analysis) -> String {
    let mut functions = analysis.functions.clone();
    functions.sort_by(|left, right| {
        left.file
            .cmp(&right.file)
            .then(left.name.cmp(&right.name))
            .then(left.line.cmp(&right.line))
            .then(left.id.cmp(&right.id))
    });

    let mut calls = analysis.calls.clone();
    calls.sort_by(|left, right| {
        left.caller
            .cmp(&right.caller)
            .then(left.callee.cmp(&right.callee))
            .then(left.file.cmp(&right.file))
            .then(left.line.cmp(&right.line))
    });

    let mut functions_by_file: BTreeMap<&str, Vec<&Function>> = BTreeMap::new();
    for function in &functions {
        functions_by_file
            .entry(function.file.as_str())
            .or_default()
            .push(function);
    }

    let mut output = String::from(
        "digraph coviz {\n  graph [rankdir=LR, bgcolor=\"#dde5f4\", pad=\"0.35\", nodesep=\"0.6\", ranksep=\"1.0\", splines=true, overlap=false, fontname=\"Helvetica\"];\n  node [shape=ellipse, style=\"filled\", fillcolor=\"#b9e1ea\", color=\"#111111\", penwidth=1.6, fontname=\"Helvetica\", fontsize=14, margin=\"0.12,0.08\"];\n  edge [color=\"#934f12\", arrowsize=0.75, penwidth=1.2, fontname=\"Helvetica\", fontsize=10];\n",
    );

    for (file, file_functions) in functions_by_file {
        output.push_str(&format!(
            "  subgraph \"cluster_{}\" {{\n    graph [label=\"{}\", style=\"filled\", fillcolor=\"{}\", color=\"#333333\", penwidth=1.1, fontname=\"Helvetica-Bold\", fontsize=18, margin=14];\n",
            escape_dot(&cluster_id(file)),
            escape_dot(&cluster_label(file)),
            cluster_color(file),
        ));

        for function in file_functions {
            output.push_str(&format!(
                "    \"{}\" [label=\"{}\"];\n",
                escape_dot(&function.id),
                escape_dot(&function_label(function)),
            ));
        }

        output.push_str("  }\n");
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

fn cluster_label(file: &str) -> String {
    let trimmed = file.trim_matches('/');
    if trimmed.is_empty() {
        return "source".to_string();
    }

    let mut parts = trimmed.rsplit('/');
    let file_name = parts.next().unwrap_or(trimmed);
    let parent = parts.next();

    match parent {
        Some(parent) => format!("{parent}/{file_name}"),
        None => file_name.to_string(),
    }
}

fn cluster_color(file: &str) -> &'static str {
    let colors = ["#ffffdd", "#d9f7d8", "#d8ecff", "#eadcff", "#ffe5c7"];
    let hash = file
        .bytes()
        .fold(0_usize, |state, byte| state.wrapping_add(byte as usize));
    colors[hash % colors.len()]
}

fn cluster_id(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '_'
            }
        })
        .collect()
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
      --bg: #d8deeb;
      --ink: #10131a;
      --muted: #536070;
      --panel: #eef3ff;
      --panel-strong: #fff8d4;
      --line: #8b95a7;
      --accent: #934f12;
    }

    * {
      box-sizing: border-box;
    }

    body {
      margin: 0;
      min-height: 100vh;
      color: var(--ink);
      background: var(--bg);
      font-family: Helvetica, Arial, sans-serif;
    }

    header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 1rem;
      padding: 0.9rem 1.25rem;
      border-bottom: 1px solid var(--line);
      background: #cfd8e8;
    }

    h1 {
      margin: 0;
      font-size: 1.45rem;
      font-weight: 700;
      letter-spacing: -0.02em;
    }

    .summary {
      color: var(--muted);
      font-size: 0.95rem;
    }

    main {
      padding: 1rem;
    }

    .toolbar {
      display: flex;
      flex-wrap: wrap;
      gap: 0.75rem;
      align-items: center;
      margin-bottom: 1rem;
    }

    input {
      width: min(34rem, 100%);
      border: 1px solid var(--line);
      border-radius: 0.35rem;
      background: white;
      color: var(--ink);
      padding: 0.65rem 0.8rem;
      font: inherit;
    }

    a {
      color: var(--accent);
      font-weight: 700;
      text-decoration: none;
    }

    a:hover {
      text-decoration: underline;
    }

    #canvas {
      position: relative;
      min-height: calc(100vh - 8rem);
      overflow: hidden;
      border: 1px solid #6f7888;
      background: #dde5f4;
      box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.55);
      cursor: grab;
      touch-action: none;
      user-select: none;
    }

    #canvas.is-panning {
      cursor: grabbing;
    }

    .graph-viewport {
      display: inline-block;
      min-width: 100%;
      min-height: calc(100vh - 8rem);
      transform-origin: 0 0;
      will-change: transform;
    }

    #canvas svg {
      display: block;
      width: max-content;
      min-width: 100%;
      height: auto;
      min-height: calc(100vh - 8rem);
    }

    .fallback {
      position: relative;
      min-width: 980px;
      min-height: 680px;
    }

    .fallback svg {
      position: absolute;
      inset: 0;
      pointer-events: none;
    }

    .node {
      position: absolute;
      width: 10.8rem;
      min-height: 3.8rem;
      display: grid;
      place-items: center;
      padding: 0.6rem;
      border: 2px solid #111;
      border-radius: 999px;
      background: #b9e1ea;
      text-align: center;
      transition: opacity 120ms ease;
    }

    .node.hidden {
      opacity: 0.12;
    }

    .name {
      font-weight: 700;
      overflow-wrap: anywhere;
    }

    .meta {
      color: var(--muted);
      font-size: 0.76rem;
      overflow-wrap: anywhere;
    }

    .empty {
      padding: 2rem;
      color: var(--muted);
    }

    .hint {
      color: var(--muted);
      font-size: 0.9rem;
    }

    @media (max-width: 720px) {
      header {
        align-items: start;
        flex-direction: column;
      }
    }
  </style>
</head>
<body>
  <header>
    <h1>coviz quick</h1>
    <div class="summary">__FUNCTION_COUNT__ functions / __CALL_COUNT__ calls</div>
  </header>
  <main>
    <div class="toolbar">
      <input id="filter" type="search" placeholder="Filter function or file" autocomplete="off">
      <a href="/graph.svg">graph.svg</a>
      <a href="/graph.dot">graph.dot</a>
      <a href="/graph.json">graph.json</a>
      <span class="hint">Wheel zoom / left-drag pan</span>
    </div>
    <section id="canvas" aria-label="Call graph">
      <div class="empty">Loading graph...</div>
    </section>
  </main>
  <script>
    const canvas = document.querySelector("#canvas");
    const filter = document.querySelector("#filter");
    const view = {
      scale: 1,
      offsetX: 0,
      offsetY: 0,
      isPanning: false,
      startX: 0,
      startY: 0,
      startOffsetX: 0,
      startOffsetY: 0
    };

    function escapeHtml(value) {
      return String(value).replace(/[&<>"']/g, (char) => ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        "\"": "&quot;",
        "'": "&#39;"
      }[char]));
    }

    function viewport() {
      return document.querySelector("#viewport");
    }

    function clamp(value, min, max) {
      return Math.min(max, Math.max(min, value));
    }

    function applyViewTransform() {
      const target = viewport();
      if (!target) {
        return;
      }

      target.style.transform = `translate(${view.offsetX}px, ${view.offsetY}px) scale(${view.scale})`;
    }

    function resetView() {
      view.scale = 1;
      view.offsetX = 0;
      view.offsetY = 0;
      applyViewTransform();
    }

    function setGraphContent(html) {
      canvas.innerHTML = `<div id="viewport" class="graph-viewport">${html}</div>`;
      resetView();
    }

    canvas.addEventListener("wheel", (event) => {
      if (!viewport()) {
        return;
      }

      event.preventDefault();
      const rect = canvas.getBoundingClientRect();
      const pointerX = event.clientX - rect.left;
      const pointerY = event.clientY - rect.top;
      const nextScale = clamp(view.scale * (event.deltaY < 0 ? 1.12 : 0.88), 0.2, 6);
      const graphX = (pointerX - view.offsetX) / view.scale;
      const graphY = (pointerY - view.offsetY) / view.scale;

      view.scale = nextScale;
      view.offsetX = pointerX - graphX * nextScale;
      view.offsetY = pointerY - graphY * nextScale;
      applyViewTransform();
    }, { passive: false });

    canvas.addEventListener("pointerdown", (event) => {
      if (event.button !== 0 || !viewport()) {
        return;
      }

      event.preventDefault();
      view.isPanning = true;
      view.startX = event.clientX;
      view.startY = event.clientY;
      view.startOffsetX = view.offsetX;
      view.startOffsetY = view.offsetY;
      canvas.classList.add("is-panning");
      canvas.setPointerCapture(event.pointerId);
    });

    canvas.addEventListener("pointermove", (event) => {
      if (!view.isPanning) {
        return;
      }

      view.offsetX = view.startOffsetX + event.clientX - view.startX;
      view.offsetY = view.startOffsetY + event.clientY - view.startY;
      applyViewTransform();
    });

    function stopPanning(event) {
      if (!view.isPanning) {
        return;
      }

      view.isPanning = false;
      canvas.classList.remove("is-panning");
      if (canvas.hasPointerCapture(event.pointerId)) {
        canvas.releasePointerCapture(event.pointerId);
      }
    }

    canvas.addEventListener("pointerup", stopPanning);
    canvas.addEventListener("pointercancel", stopPanning);

    function applySvgFilter() {
      const query = filter.value.trim().toLowerCase();
      const nodes = canvas.querySelectorAll(".node");
      if (!nodes.length) {
        return;
      }

      nodes.forEach((node) => {
        const label = node.textContent.toLowerCase();
        node.style.opacity = query && !label.includes(query) ? "0.12" : "1";
      });
    }

    function renderSvg() {
      return fetch("/graph.svg")
        .then((response) => {
          if (!response.ok) {
            throw new Error("graph.svg is unavailable");
          }
          return response.text();
        })
        .then((svg) => {
          setGraphContent(svg);
          filter.addEventListener("input", applySvgFilter);
        });
    }

    function fallbackDraw(data) {
      const nodes = data.functions;
      const calls = data.calls;

      if (!nodes.length) {
        canvas.innerHTML = '<div class="empty">No supported functions found.</div>';
        return;
      }

      const incoming = new Map(nodes.map((node) => [node.id, 0]));
      const outgoing = new Map(nodes.map((node) => [node.id, []]));
      calls.forEach((call) => {
        incoming.set(call.callee, (incoming.get(call.callee) || 0) + 1);
        outgoing.get(call.caller)?.push(call.callee);
      });

      const depth = new Map();
      const queue = nodes.filter((node) => (incoming.get(node.id) || 0) === 0).map((node) => node.id);
      nodes.forEach((node) => depth.set(node.id, 0));

      for (const id of queue) {
        const nextDepth = (depth.get(id) || 0) + 1;
        (outgoing.get(id) || []).forEach((callee) => {
          if (nextDepth > (depth.get(callee) || 0)) {
            depth.set(callee, nextDepth);
            queue.push(callee);
          }
        });
      }

      const layers = new Map();
      nodes.forEach((node) => {
        const layer = depth.get(node.id) || 0;
        if (!layers.has(layer)) {
          layers.set(layer, []);
        }
        layers.get(layer).push(node);
      });

      const layerEntries = [...layers.entries()].sort((left, right) => left[0] - right[0]);
      const width = Math.max(980, layerEntries.length * 260 + 160);
      const height = Math.max(680, Math.max(...layerEntries.map(([, layer]) => layer.length)) * 120 + 160);
      const positions = new Map();

      setGraphContent(`<div class="fallback" style="width:${width}px;min-height:${height}px"><svg viewBox="0 0 ${width} ${height}" width="${width}" height="${height}"><defs><marker id="arrow" markerWidth="10" markerHeight="10" refX="8" refY="3" orient="auto"><path d="M0,0 L0,6 L9,3 z" fill="#934f12"></path></marker></defs><g id="edges"></g></svg></div>`);
      const fallback = canvas.querySelector(".fallback");

      layerEntries.forEach(([layerIndex, layer]) => {
        const x = 90 + layerIndex * 260;
        layer.forEach((node, row) => {
          const y = 70 + row * 120;
          positions.set(node.id, { x: x + 86, y: y + 34, node });
          const element = document.createElement("article");
          element.className = "node";
          element.dataset.search = `${node.name} ${node.file}`.toLowerCase();
          element.style.left = `${x}px`;
          element.style.top = `${y}px`;
          element.innerHTML = `<div><div class="name">${escapeHtml(node.name)}</div><div class="meta">${escapeHtml(node.file)}:${node.line}</div></div>`;
          fallback.appendChild(element);
        });
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
        line.setAttribute("stroke", "#934f12");
        line.setAttribute("stroke-width", "1.4");
        line.setAttribute("marker-end", "url(#arrow)");
        edgeLayer.appendChild(line);
      });

      filter.addEventListener("input", () => {
        const query = filter.value.trim().toLowerCase();
        document.querySelectorAll(".node").forEach((node) => {
          node.classList.toggle("hidden", query && !node.dataset.search.includes(query));
        });
      });
    }

    renderSvg().catch(() => {
      fetch("/graph.json")
        .then((response) => response.json())
        .then(fallbackDraw)
        .catch((error) => {
          canvas.innerHTML = `<div class="empty">Failed to load graph: ${escapeHtml(error.message)}</div>`;
        });
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
        assert!(dot.contains("subgraph \"cluster_main_go\""));
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
        assert!(html.contains("graph.svg"));
        assert!(html.contains("Wheel zoom / left-drag pan"));
        assert!(html.contains("graph-viewport"));
    }
}
