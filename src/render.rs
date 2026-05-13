use std::collections::BTreeMap;

use anyhow::Result;

use crate::model::{Analysis, Call, CallKind, Function};

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
            "  \"{}\" -> \"{}\" [label=\"{}\", color=\"{}\", style=\"{}\"];\n",
            escape_dot(&call.caller),
            escape_dot(&call.callee),
            escape_dot(&call_label(call)),
            call_color(call),
            call_style(call),
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

fn call_color(call: &Call) -> &'static str {
    match call.kind {
        CallKind::Direct => "#934f12",
        CallKind::Method => "#1d6f8f",
        CallKind::Associated => "#6d4aa1",
        CallKind::Unknown => "#6f7888",
    }
}

fn call_style(call: &Call) -> &'static str {
    match call.kind {
        CallKind::Direct | CallKind::Associated => "solid",
        CallKind::Method => "dashed",
        CallKind::Unknown => "dotted",
    }
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
      --accent-2: #1d6f8f;
      --accent-3: #6d4aa1;
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
      overflow: hidden;
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
      --inspector-width: 24rem;
      display: grid;
      grid-template-columns: minmax(0, 1fr) 0.7rem var(--inspector-width);
      gap: 0.65rem;
      height: calc(100vh - 4.1rem);
      padding: 1rem;
    }

    .toolbar {
      display: flex;
      flex-wrap: wrap;
      gap: 0.75rem;
      align-items: center;
      margin-bottom: 1rem;
    }

    input,
    select {
      width: min(34rem, 100%);
      border: 1px solid var(--line);
      border-radius: 0.35rem;
      background: white;
      color: var(--ink);
      padding: 0.65rem 0.8rem;
      font: inherit;
    }

    select {
      width: auto;
      max-width: 13rem;
    }

    a {
      color: var(--accent);
      font-weight: 700;
      text-decoration: none;
    }

    a:hover {
      text-decoration: underline;
    }

    button {
      border: 1px solid var(--line);
      border-radius: 0.35rem;
      background: #fff8d4;
      color: var(--ink);
      cursor: pointer;
      font: inherit;
      font-weight: 700;
      padding: 0.62rem 0.75rem;
    }

    button:hover {
      border-color: var(--accent);
    }

    button.active {
      background: #f7c873;
      border-color: var(--accent);
    }

    .toolbar-group {
      display: flex;
      flex-wrap: wrap;
      gap: 0.45rem;
      align-items: center;
    }

    .search-tools {
      display: flex;
      min-width: min(36rem, 100%);
      flex: 1 1 28rem;
      gap: 0.35rem;
      align-items: center;
    }

    .search-tools input {
      flex: 1 1 18rem;
      min-width: 12rem;
    }

    .icon-button {
      min-width: 2.45rem;
      padding-inline: 0.65rem;
    }

    .count-pill {
      min-width: 4.2rem;
      color: var(--muted);
      font-size: 0.86rem;
      text-align: center;
    }

    .chips {
      display: flex;
      flex-wrap: wrap;
      gap: 0.35rem;
      margin: -0.4rem 0 0.75rem;
    }

    .chip {
      border: 1px solid #9aa4b5;
      border-radius: 999px;
      background: #eef3ff;
      color: var(--muted);
      cursor: pointer;
      font: inherit;
      font-size: 0.82rem;
      font-weight: 700;
      padding: 0.35rem 0.58rem;
    }

    .chip.active {
      background: #fff8d4;
      color: var(--ink);
      border-color: var(--accent);
    }

    #canvas {
      position: relative;
      height: calc(100vh - 9rem);
      min-height: 30rem;
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
      min-height: calc(100vh - 9rem);
      transform-origin: 0 0;
      will-change: transform;
    }

    #canvas svg {
      display: block;
      width: max-content;
      min-width: 100%;
      height: auto;
      min-height: calc(100vh - 9rem);
    }

    #canvas svg .node,
    #canvas svg .edge {
      cursor: pointer;
    }

    #canvas svg .node.selected ellipse,
    #canvas svg .node.selected polygon,
    #canvas svg .node.selected path {
      stroke: #d12f1f;
      stroke-width: 3px;
    }

    #canvas svg .edge.selected path,
    #canvas svg .edge.selected polygon {
      stroke: #d12f1f;
      stroke-width: 2.6px;
      fill: #d12f1f;
    }

    #canvas line.edge.selected {
      stroke: #d12f1f;
      stroke-width: 2.6px;
    }

    #canvas svg .edge.kind-direct path,
    #canvas svg .edge.kind-direct polygon,
    #canvas line.edge.kind-direct {
      stroke: var(--accent);
    }

    #canvas svg .edge.kind-method path,
    #canvas svg .edge.kind-method polygon,
    #canvas line.edge.kind-method {
      stroke: var(--accent-2);
      stroke-dasharray: 6 4;
    }

    #canvas svg .edge.kind-associated path,
    #canvas svg .edge.kind-associated polygon,
    #canvas line.edge.kind-associated {
      stroke: var(--accent-3);
    }

    #canvas svg .edge.kind-unknown path,
    #canvas svg .edge.kind-unknown polygon,
    #canvas line.edge.kind-unknown {
      stroke: #6f7888;
      stroke-dasharray: 2 4;
    }

    #canvas svg .edge.selected path,
    #canvas svg .edge.selected polygon {
      stroke: #d12f1f;
      fill: #d12f1f;
    }

    #canvas line.edge.selected {
      stroke: #d12f1f;
    }

    #canvas svg .dimmed,
    #canvas svg .edge.dimmed {
      opacity: 0.12;
    }

    #canvas .filtered-out {
      display: none;
    }

    .minimap {
      position: absolute;
      right: 0.85rem;
      bottom: 0.85rem;
      z-index: 3;
      width: 12rem;
      height: 8rem;
      overflow: hidden;
      border: 1px solid #6f7888;
      background: rgba(238, 243, 255, 0.9);
      box-shadow: 0 0.35rem 1rem rgba(16, 19, 26, 0.18);
      cursor: pointer;
      user-select: none;
    }

    .minimap-graph {
      position: absolute;
      inset: 0;
      transform-origin: 0 0;
      opacity: 0.72;
      pointer-events: none;
    }

    .minimap-graph svg {
      display: block;
      width: auto;
      height: auto;
      min-width: 0;
      min-height: 0;
    }

    .minimap-window {
      position: absolute;
      border: 2px solid #d12f1f;
      background: rgba(209, 47, 31, 0.12);
      pointer-events: none;
    }

    .fallback {
      position: relative;
      min-width: 980px;
      min-height: 680px;
    }

    .fallback svg {
      position: absolute;
      inset: 0;
    }

    .fallback .edge {
      pointer-events: stroke;
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

    .node.dimmed {
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

    .workspace {
      min-width: 0;
    }

    .splitter {
      align-self: stretch;
      border: 1px solid #6f7888;
      background:
        linear-gradient(90deg, transparent 0 35%, #6f7888 35% 45%, transparent 45% 55%, #6f7888 55% 65%, transparent 65% 100%),
        #c8d1e1;
      cursor: col-resize;
      touch-action: none;
      user-select: none;
    }

    .splitter:hover,
    body.resizing .splitter {
      background:
        linear-gradient(90deg, transparent 0 35%, #934f12 35% 45%, transparent 45% 55%, #934f12 55% 65%, transparent 65% 100%),
        #f7c873;
    }

    body.resizing {
      cursor: col-resize;
      user-select: none;
    }

    .inspector {
      height: calc(100vh - 6.1rem);
      overflow: auto;
      border: 1px solid #6f7888;
      background: #f6f1e2;
      box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.55);
      padding: 1rem;
    }

    .inspector h2,
    .inspector h3 {
      margin: 0 0 0.75rem;
    }

    .inspector .muted {
      color: var(--muted);
      font-size: 0.9rem;
    }

    .stat-grid {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 0.5rem;
      margin: 0.75rem 0 1rem;
    }

    .stat {
      background: white;
      border: 1px solid var(--line);
      border-radius: 0.45rem;
      padding: 0.65rem;
    }

    .stat strong {
      display: block;
      font-size: 1.25rem;
    }

    .call-list {
      list-style: none;
      margin: 0 0 1rem;
      padding: 0;
    }

    .call-list li {
      border-bottom: 1px solid var(--line);
      cursor: pointer;
      padding: 0.45rem 0;
    }

    .call-list li:hover {
      color: var(--accent);
    }

    .breadcrumb {
      display: flex;
      flex-wrap: wrap;
      gap: 0.35rem;
      align-items: center;
      margin: 0.5rem 0 0.85rem;
      color: var(--muted);
      font-size: 0.9rem;
    }

    .breadcrumb button {
      max-width: 100%;
      overflow: hidden;
      padding: 0.32rem 0.5rem;
      text-overflow: ellipsis;
      white-space: nowrap;
    }

    .source-actions,
    .source-modes {
      display: flex;
      flex-wrap: wrap;
      gap: 0.4rem;
      margin: 0.55rem 0;
    }

    .source-actions button,
    .source-actions a,
    .source-modes button {
      border: 1px solid var(--line);
      border-radius: 0.35rem;
      background: #fff8d4;
      color: var(--ink);
      display: inline-flex;
      align-items: center;
      font-size: 0.84rem;
      font-weight: 700;
      min-height: 2rem;
      padding: 0.38rem 0.55rem;
    }

    .source-modes button.active {
      background: #f7c873;
      border-color: var(--accent);
    }

    pre.source {
      background: #111827;
      border-radius: 0.45rem;
      color: #dbeafe;
      font: 0.82rem/1.45 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
      overflow: auto;
      padding: 0.8rem;
      white-space: pre;
    }

    .source-line.focus {
      display: block;
      color: #facc15;
      font-weight: 700;
    }

    .source-line {
      display: block;
    }

    .line-number {
      color: #94a3b8;
      user-select: none;
    }

    .source-line .kw {
      color: #facc15;
      font-weight: 700;
    }

    .source-line .str {
      color: #86efac;
    }

    .source-line .com {
      color: #9ca3af;
      font-style: italic;
    }

    @media (max-width: 720px) {
      body {
        overflow: auto;
      }

      main {
        display: block;
        height: auto;
      }

      .splitter {
        display: none;
      }

      header {
        align-items: start;
        flex-direction: column;
      }

      .inspector {
        height: auto;
        margin-top: 1rem;
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
    <section class="workspace">
      <div class="toolbar">
        <div class="search-tools">
          <input id="filter" type="search" placeholder="Filter function or file" autocomplete="off">
          <button id="search-prev" class="icon-button" type="button" title="Previous match">Prev</button>
          <button id="search-next" class="icon-button" type="button" title="Next match">Next</button>
          <span id="search-count" class="count-pill">0 / 0</span>
        </div>
        <div class="toolbar-group">
          <select id="layout-preset" aria-label="Layout preset">
            <option value="all">All calls</option>
            <option value="by-file">Group by file</option>
            <option value="fan-in">Fan-in only</option>
            <option value="fan-out">Fan-out only</option>
            <option value="cycles">Cycles</option>
          </select>
          <button id="reset-view" type="button">Reset view</button>
          <button id="isolate" type="button">Isolate selected</button>
          <button id="hide-isolated" type="button">Hide isolated</button>
        </div>
        <div class="toolbar-group">
          <a href="/graph.svg">graph.svg</a>
          <a href="/graph.dot">graph.dot</a>
          <a href="/graph.json">graph.json</a>
          <a href="/source.json">source.json</a>
        </div>
        <span class="hint">Wheel zoom / left-drag pan / click inspect / press ? shortcuts</span>
      </div>
      <div id="file-filters" class="chips" aria-label="File filters"></div>
      <section id="canvas" aria-label="Call graph">
        <div class="empty">Loading graph...</div>
      </section>
    </section>
    <div id="splitter" class="splitter" role="separator" aria-label="Resize inspector" aria-orientation="vertical" title="Drag to resize inspector. Double-click to reset."></div>
    <aside id="inspector" class="inspector" aria-label="Inspector">
      <h2>Inspector</h2>
      <p class="muted">Click a node or edge to inspect calls and source context.</p>
    </aside>
  </main>
  <script>
    const canvas = document.querySelector("#canvas");
    const filter = document.querySelector("#filter");
    const searchPrevButton = document.querySelector("#search-prev");
    const searchNextButton = document.querySelector("#search-next");
    const searchCount = document.querySelector("#search-count");
    const fileFilters = document.querySelector("#file-filters");
    const inspector = document.querySelector("#inspector");
    const splitter = document.querySelector("#splitter");
    const resetButton = document.querySelector("#reset-view");
    const isolateButton = document.querySelector("#isolate");
    const hideIsolatedButton = document.querySelector("#hide-isolated");
    const layoutPreset = document.querySelector("#layout-preset");
    const state = {
      graph: { functions: [], calls: [] },
      functions: new Map(),
      sources: new Map(),
      files: new Map(),
      allFiles: [],
      activeFiles: new Set(),
      incoming: new Map(),
      outgoing: new Map(),
      callsByEdge: new Map(),
      selectedNode: null,
      selectedEdge: null,
      isolate: false,
      hideIsolated: false,
      preset: "all",
      searchMatches: [],
      searchIndex: -1,
      sourceMode: "context"
    };
    const view = {
      scale: 1,
      offsetX: 0,
      offsetY: 0,
      isPanning: false,
      moved: false,
      startX: 0,
      startY: 0,
      startOffsetX: 0,
      startOffsetY: 0,
      startTarget: null
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

    function fetchJson(path) {
      return fetch(path).then((response) => {
        if (!response.ok) {
          throw new Error(`${path} returned ${response.status}`);
        }
        return response.json();
      });
    }

    function viewport() {
      return document.querySelector("#viewport");
    }

    function clamp(value, min, max) {
      return Math.min(max, Math.max(min, value));
    }

    function inspectorMaxWidth() {
      return Math.max(320, window.innerWidth - 460);
    }

    function setInspectorWidth(width, persist = true) {
      const nextWidth = clamp(Number(width) || 384, 280, inspectorMaxWidth());
      document.querySelector("main").style.setProperty("--inspector-width", `${nextWidth}px`);
      splitter.setAttribute("aria-valuenow", String(Math.round(nextWidth)));
      if (persist) {
        localStorage.setItem("coviz.quick.inspectorWidth", String(Math.round(nextWidth)));
      }
    }

    function loadInspectorWidth() {
      setInspectorWidth(localStorage.getItem("coviz.quick.inspectorWidth") || 384, false);
    }

    function edgeKey(call) {
      return `${call.caller}->${call.callee}`;
    }

    function splitEdgeKey(value) {
      const [caller, callee] = String(value || "").split("->");
      return { caller, callee };
    }

    function functionLabel(id) {
      const item = state.functions.get(id);
      return item ? `${item.name} ${item.file}:${item.line}` : id;
    }

    function applyViewTransform() {
      const target = viewport();
      if (!target) {
        return;
      }

      target.style.transform = `translate(${view.offsetX}px, ${view.offsetY}px) scale(${view.scale})`;
      updateMinimap();
    }

    function resetView() {
      view.scale = 1;
      view.offsetX = 0;
      view.offsetY = 0;
      applyViewTransform();
    }

    function setGraphContent(html) {
      canvas.innerHTML = `
        <div id="viewport" class="graph-viewport">${html}</div>
        <div id="minimap" class="minimap" aria-label="Graph minimap" title="Click or drag to move the viewport">
          <div id="minimap-graph" class="minimap-graph"></div>
          <div id="minimap-window" class="minimap-window"></div>
        </div>
      `;
      resetView();
    }

    function initData(graph, source) {
      state.graph = graph;
      state.functions = new Map(graph.functions.map((item) => [item.id, item]));
      state.sources = new Map((source.functions || []).map((item) => [item.id, item]));
      state.files = new Map((source.files || []).map((item) => [item.file, item]));
      state.allFiles = [...new Set(graph.functions.map((item) => item.file))].sort();
      state.activeFiles = new Set(state.allFiles);
      state.incoming = new Map(graph.functions.map((item) => [item.id, []]));
      state.outgoing = new Map(graph.functions.map((item) => [item.id, []]));
      state.callsByEdge = new Map();

      graph.calls.forEach((call) => {
        state.outgoing.get(call.caller)?.push(call);
        state.incoming.get(call.callee)?.push(call);
        if (!state.callsByEdge.has(edgeKey(call))) {
          state.callsByEdge.set(edgeKey(call), []);
        }
        state.callsByEdge.get(edgeKey(call)).push(call);
      });

      renderFileFilters();
    }

    function fileLabel(file) {
      const parts = String(file).split("/");
      if (parts.length <= 2) {
        return file;
      }
      return `${parts.at(-2)}/${parts.at(-1)}`;
    }

    function renderFileFilters() {
      if (!state.allFiles.length) {
        fileFilters.innerHTML = "";
        return;
      }

      const allActive = state.activeFiles.size === state.allFiles.length;
      fileFilters.innerHTML = `
        <button class="chip ${allActive ? "active" : ""}" type="button" data-file-action="all">All files</button>
        <button class="chip" type="button" data-file-action="none">None</button>
        ${state.allFiles.map((file) => `
          <button class="chip ${state.activeFiles.has(file) ? "active" : ""}" type="button" data-file="${escapeHtml(file)}" title="${escapeHtml(file)}">
            ${escapeHtml(fileLabel(file))}
          </button>
        `).join("")}
      `;
    }

    function callKind(call) {
      return call?.kind || "unknown";
    }

    function callKindLabel(call) {
      return callKind(call).replace(/^\w/, (value) => value.toUpperCase());
    }

    function edgeCalls(key) {
      return state.callsByEdge.get(key) || [];
    }

    function firstEdgeCall(key) {
      return edgeCalls(key)[0] || null;
    }

    function isIsolated(id) {
      return !(state.incoming.get(id) || []).length && !(state.outgoing.get(id) || []).length;
    }

    function hasCycleEdge(call) {
      if (!call) {
        return false;
      }
      if (call.caller === call.callee) {
        return true;
      }
      return state.graph.calls.some((candidate) => candidate.caller === call.callee && candidate.callee === call.caller);
    }

    function passesPreset(id) {
      if (state.preset === "fan-in") {
        return (state.incoming.get(id) || []).length > 0;
      }
      if (state.preset === "fan-out") {
        return (state.outgoing.get(id) || []).length > 0;
      }
      if (state.preset === "cycles") {
        return (state.incoming.get(id) || []).some(hasCycleEdge) || (state.outgoing.get(id) || []).some(hasCycleEdge);
      }
      return true;
    }

    function edgePassesPreset(call, caller, callee) {
      if (state.preset === "fan-in") {
        return (state.incoming.get(callee) || []).length > 0;
      }
      if (state.preset === "fan-out") {
        return (state.outgoing.get(caller) || []).length > 0;
      }
      if (state.preset === "cycles") {
        return hasCycleEdge(call);
      }
      return true;
    }

    function nodeBaseVisible(id) {
      const item = state.functions.get(id);
      if (!item) {
        return true;
      }
      if (!state.activeFiles.has(item.file)) {
        return false;
      }
      return !(state.hideIsolated && isIsolated(id));
    }

    function edgeBaseVisible(call, caller, callee) {
      return nodeBaseVisible(caller) && nodeBaseVisible(callee);
    }

    function graphElementBounds() {
      const target = viewport();
      if (!target) {
        return { width: 1, height: 1 };
      }

      const svg = target.querySelector("svg");
      if (svg?.viewBox?.baseVal?.width && svg?.viewBox?.baseVal?.height) {
        return {
          width: svg.viewBox.baseVal.width,
          height: svg.viewBox.baseVal.height
        };
      }

      const fallback = target.querySelector(".fallback");
      if (fallback) {
        return {
          width: fallback.offsetWidth || fallback.scrollWidth || 1,
          height: fallback.offsetHeight || fallback.scrollHeight || 1
        };
      }

      return {
        width: target.scrollWidth || target.offsetWidth || 1,
        height: target.scrollHeight || target.offsetHeight || 1
      };
    }

    function renderMinimap() {
      const minimapGraph = document.querySelector("#minimap-graph");
      const target = viewport();
      if (!minimapGraph || !target) {
        return;
      }

      const svg = target.querySelector("svg");
      if (svg) {
        const clone = svg.cloneNode(true);
        clone.removeAttribute("width");
        clone.removeAttribute("height");
        minimapGraph.replaceChildren(clone);
      } else {
        minimapGraph.innerHTML = "";
      }
      updateMinimap();
    }

    function updateMinimap() {
      const minimap = document.querySelector("#minimap");
      const minimapGraph = document.querySelector("#minimap-graph");
      const windowBox = document.querySelector("#minimap-window");
      if (!minimap || !minimapGraph || !windowBox || !viewport()) {
        return;
      }

      const graph = graphElementBounds();
      const scale = Math.min(minimap.clientWidth / graph.width, minimap.clientHeight / graph.height);
      const graphWidth = graph.width * scale;
      const graphHeight = graph.height * scale;
      const graphLeft = (minimap.clientWidth - graphWidth) / 2;
      const graphTop = (minimap.clientHeight - graphHeight) / 2;
      minimapGraph.style.transform = `translate(${graphLeft}px, ${graphTop}px) scale(${scale})`;

      const left = graphLeft + (-view.offsetX / view.scale) * scale;
      const top = graphTop + (-view.offsetY / view.scale) * scale;
      const width = (canvas.clientWidth / view.scale) * scale;
      const height = (canvas.clientHeight / view.scale) * scale;
      windowBox.style.left = `${clamp(left, 0, minimap.clientWidth)}px`;
      windowBox.style.top = `${clamp(top, 0, minimap.clientHeight)}px`;
      windowBox.style.width = `${clamp(width, 8, minimap.clientWidth)}px`;
      windowBox.style.height = `${clamp(height, 8, minimap.clientHeight)}px`;
    }

    function panFromMinimap(event) {
      const minimap = document.querySelector("#minimap");
      if (!minimap || !viewport()) {
        return;
      }

      const rect = minimap.getBoundingClientRect();
      const graph = graphElementBounds();
      const scale = Math.min(minimap.clientWidth / graph.width, minimap.clientHeight / graph.height);
      const graphLeft = (minimap.clientWidth - graph.width * scale) / 2;
      const graphTop = (minimap.clientHeight - graph.height * scale) / 2;
      const graphX = clamp((event.clientX - rect.left - graphLeft) / scale, 0, graph.width);
      const graphY = clamp((event.clientY - rect.top - graphTop) / scale, 0, graph.height);
      view.offsetX = canvas.clientWidth / 2 - graphX * view.scale;
      view.offsetY = canvas.clientHeight / 2 - graphY * view.scale;
      applyViewTransform();
    }

    function renderHome() {
      const topFanout = [...state.functions.values()]
        .map((item) => ({ item, count: (state.outgoing.get(item.id) || []).length }))
        .sort((left, right) => right.count - left.count || left.item.name.localeCompare(right.item.name))
        .slice(0, 6);

      inspector.innerHTML = `
        <h2>Inspector</h2>
        <p class="muted">Click a function or call edge to inspect local context.</p>
        <div class="stat-grid">
          <div class="stat"><strong>${state.graph.functions.length}</strong><span>functions</span></div>
          <div class="stat"><strong>${state.graph.calls.length}</strong><span>calls</span></div>
        </div>
        <h3>Highest fan-out</h3>
        <ul class="call-list">
          ${topFanout.map(({ item, count }) => `<li data-node-id="${item.id}">${escapeHtml(item.name)} <span class="muted">${count} calls</span><br><span class="muted">${escapeHtml(item.file)}:${item.line}</span></li>`).join("")}
        </ul>
      `;
    }

    function renderCallList(calls, direction) {
      if (!calls.length) {
        return '<p class="muted">None.</p>';
      }

      return `<ul class="call-list">${calls.map((call) => {
        const target = direction === "out" ? call.callee : call.caller;
        const item = state.functions.get(target);
        return `<li data-node-id="${target}" data-edge-key="${edgeKey(call)}">${escapeHtml(item?.name || target)}<br><span class="muted">${escapeHtml(call.file)}:${call.line} · ${escapeHtml(callKindLabel(call))}</span></li>`;
      }).join("")}</ul>`;
    }

    function highlightCode(text) {
      const html = escapeHtml(text);
      const commentIndex = html.indexOf("//");
      const code = commentIndex >= 0 ? html.slice(0, commentIndex) : html;
      const comment = commentIndex >= 0 ? html.slice(commentIndex) : "";
      const highlighted = code
        .replace(/(&quot;.*?&quot;)/g, '<span class="str">$1</span>')
        .replace(/\b(async|await|break|const|continue|crate|else|enum|fn|for|if|impl|let|loop|match|mod|mut|pub|return|self|Self|struct|trait|type|use|where|while)\b/g, '<span class="kw">$1</span>');
      return comment ? `${highlighted}<span class="com">${comment}</span>` : highlighted;
    }

    function sourceLinesFor(id) {
      const source = state.sources.get(id);
      if (!source || !source.lines.length) {
        return null;
      }

      const file = state.files.get(source.file);
      const allLines = file?.lines?.length ? file.lines : source.lines;
      const radius = state.sourceMode === "wide" ? 20 : 5;
      const lines = state.sourceMode === "full"
        ? allLines
        : allLines.filter((line) => Math.abs(line.number - source.line) <= radius);

      return { source, file, lines };
    }

    function renderSource(id) {
      const context = sourceLinesFor(id);
      if (!context) {
        return '<p class="muted">Source snippet unavailable.</p>';
      }

      const { source, file, lines } = context;
      const absolutePath = file?.absolute_path || source.file;
      const openUrl = `vscode://file/${encodeURI(absolutePath)}:${source.line}`;
      return `
        <div class="source-actions">
          <button type="button" data-copy-text="${escapeHtml(`${absolutePath}:${source.line}`)}">Copy path</button>
          <a href="${escapeHtml(openUrl)}">Open in editor</a>
        </div>
        <div class="source-modes" aria-label="Source context size">
          <button type="button" class="${state.sourceMode === "context" ? "active" : ""}" data-source-mode="context">±5 lines</button>
          <button type="button" class="${state.sourceMode === "wide" ? "active" : ""}" data-source-mode="wide">±20 lines</button>
          <button type="button" class="${state.sourceMode === "full" ? "active" : ""}" data-source-mode="full">Full file</button>
        </div>
        <pre class="source">${lines.map((line) => {
        const number = String(line.number).padStart(4, " ");
        const focus = line.number === source.line ? " focus" : "";
        return `<span class="source-line${focus}"><span class="line-number">${number}</span>  ${highlightCode(line.text)}</span>`;
      }).join("")}</pre>
      `;
    }

    function renderBreadcrumb(id) {
      const current = state.functions.get(id);
      if (!current) {
        return "";
      }

      const incoming = state.incoming.get(id) || [];
      const outgoing = state.outgoing.get(id) || [];
      const caller = incoming[0] ? state.functions.get(incoming[0].caller) : null;
      const callee = outgoing[0] ? state.functions.get(outgoing[0].callee) : null;
      return `
        <div class="breadcrumb" aria-label="Selection path">
          ${caller ? `<button type="button" data-node-id="${caller.id}" title="${escapeHtml(caller.file)}:${caller.line}">${escapeHtml(caller.name)}</button><span>-></span>` : ""}
          <strong title="${escapeHtml(current.file)}:${current.line}">${escapeHtml(current.name)}</strong>
          ${callee ? `<span>-></span><button type="button" data-node-id="${callee.id}" title="${escapeHtml(callee.file)}:${callee.line}">${escapeHtml(callee.name)}</button>` : ""}
        </div>
      `;
    }

    function renderNodeInspector(id) {
      const item = state.functions.get(id);
      if (!item) {
        renderHome();
        return;
      }

      const incoming = state.incoming.get(id) || [];
      const outgoing = state.outgoing.get(id) || [];
      inspector.innerHTML = `
        <h2>${escapeHtml(item.name)}</h2>
        <p class="muted">${escapeHtml(item.file)}:${item.line}</p>
        ${renderBreadcrumb(id)}
        <div class="stat-grid">
          <div class="stat"><strong>${incoming.length}</strong><span>incoming</span></div>
          <div class="stat"><strong>${outgoing.length}</strong><span>outgoing</span></div>
        </div>
        <h3>Outgoing calls</h3>
        ${renderCallList(outgoing, "out")}
        <h3>Incoming calls</h3>
        ${renderCallList(incoming, "in")}
        <h3>Source context</h3>
        ${renderSource(id)}
      `;
    }

    function renderEdgeInspector(key) {
      const { caller, callee } = splitEdgeKey(key);
      const callerItem = state.functions.get(caller);
      const calleeItem = state.functions.get(callee);
      const calls = state.graph.calls.filter((call) => edgeKey(call) === key);
      inspector.innerHTML = `
        <h2>Call edge</h2>
        <p><strong>${escapeHtml(callerItem?.name || caller)}</strong> -> <strong>${escapeHtml(calleeItem?.name || callee)}</strong></p>
        <h3>Call sites</h3>
        <ul class="call-list">
          ${calls.map((call) => `<li data-node-id="${caller}" data-edge-key="${key}">${escapeHtml(call.file)}:${call.line}<br><span class="muted">${escapeHtml(callKindLabel(call))} call</span></li>`).join("")}
        </ul>
        <h3>Caller source</h3>
        ${renderSource(caller)}
      `;
    }

    function selectNode(id) {
      state.selectedNode = id;
      state.selectedEdge = null;
      renderNodeInspector(id);
      applyGraphState();
    }

    function selectEdge(key) {
      state.selectedEdge = key;
      state.selectedNode = null;
      renderEdgeInspector(key);
      applyGraphState();
    }

    function selectFromElement(target) {
      const edge = target?.closest?.(".edge");
      const key = edge?.dataset?.edge || edge?.querySelector?.("title")?.textContent;
      if (key && key.includes("->")) {
        selectEdge(key);
        return;
      }

      const node = target?.closest?.(".node");
      const id = node?.dataset?.id || node?.querySelector?.("title")?.textContent;
      if (id && state.functions.has(id)) {
        selectNode(id);
      }
    }

    function neighborhood() {
      if (!state.isolate || !state.selectedNode) {
        return null;
      }

      const ids = new Set([state.selectedNode]);
      (state.incoming.get(state.selectedNode) || []).forEach((call) => ids.add(call.caller));
      (state.outgoing.get(state.selectedNode) || []).forEach((call) => ids.add(call.callee));
      return ids;
    }

    function nodeMatchesQuery(id, query) {
      return !query || functionLabel(id).toLowerCase().includes(query);
    }

    function edgeMatchesQuery(call, caller, callee, query) {
      if (!query) {
        return true;
      }
      const label = `${functionLabel(caller)} ${functionLabel(callee)} ${call?.file || ""}`.toLowerCase();
      return label.includes(query);
    }

    function updateSearchMatches(query) {
      state.searchMatches = [...state.functions.keys()].filter((id) => nodeBaseVisible(id) && passesPreset(id) && nodeMatchesQuery(id, query));
      if (!query || !state.searchMatches.length) {
        state.searchIndex = -1;
      } else if (state.selectedNode && state.searchMatches.includes(state.selectedNode)) {
        state.searchIndex = state.searchMatches.indexOf(state.selectedNode);
      } else if (state.searchIndex < 0 || state.searchIndex >= state.searchMatches.length) {
        state.searchIndex = 0;
      }

      searchCount.textContent = query ? `${Math.max(state.searchIndex + 1, 0)} / ${state.searchMatches.length}` : `${state.searchMatches.length} nodes`;
    }

    function centerNode(id) {
      const node = [...document.querySelectorAll("#canvas .node")].find((element) => {
        const nodeId = element.dataset.id || element.querySelector?.("title")?.textContent;
        return nodeId === id;
      });
      if (!node) {
        return;
      }

      const canvasRect = canvas.getBoundingClientRect();
      const nodeRect = node.getBoundingClientRect();
      const nodeX = nodeRect.left - canvasRect.left + nodeRect.width / 2;
      const nodeY = nodeRect.top - canvasRect.top + nodeRect.height / 2;
      view.offsetX += canvas.clientWidth / 2 - nodeX;
      view.offsetY += canvas.clientHeight / 2 - nodeY;
      applyViewTransform();
    }

    function focusSearchMatch(delta) {
      const query = filter.value.trim().toLowerCase();
      updateSearchMatches(query);
      if (!state.searchMatches.length) {
        return;
      }

      state.searchIndex = (state.searchIndex + delta + state.searchMatches.length) % state.searchMatches.length;
      const id = state.searchMatches[state.searchIndex];
      selectNode(id);
      centerNode(id);
    }

    function applyGraphState() {
      const query = filter.value.trim().toLowerCase();
      updateSearchMatches(query);
      const isolated = neighborhood();

      document.querySelectorAll("#canvas .node").forEach((node) => {
        const id = node.dataset.id || node.querySelector?.("title")?.textContent;
        const visible = nodeBaseVisible(id);
        const dim = visible && (
          !passesPreset(id)
          || !nodeMatchesQuery(id, query)
          || (isolated && !isolated.has(id))
        );
        node.classList.toggle("filtered-out", !visible);
        node.classList.toggle("dimmed", Boolean(dim));
        node.classList.toggle("selected", id === state.selectedNode);
      });

      document.querySelectorAll("#canvas .edge").forEach((edge) => {
        const key = edge.dataset.edge || edge.querySelector?.("title")?.textContent;
        const { caller, callee } = splitEdgeKey(key);
        const call = firstEdgeCall(key);
        const visible = edgeBaseVisible(call, caller, callee);
        const dim = visible && (
          !edgePassesPreset(call, caller, callee)
          || !edgeMatchesQuery(call, caller, callee, query)
          || (isolated && !(isolated.has(caller) && isolated.has(callee)))
        );
        edge.classList.toggle("filtered-out", !visible);
        edge.classList.toggle("dimmed", Boolean(dim));
        edge.classList.toggle("selected", key === state.selectedEdge);
      });

      isolateButton.classList.toggle("active", state.isolate);
      hideIsolatedButton.classList.toggle("active", state.hideIsolated);
      layoutPreset.value = state.preset;
      renderFileFilters();
      updateMinimap();
    }

    function wireGraphElements() {
      document.querySelectorAll("#canvas svg .node").forEach((node) => {
        node.dataset.id = node.querySelector("title")?.textContent || "";
      });
      document.querySelectorAll("#canvas svg .edge").forEach((edge) => {
        edge.dataset.edge = edge.querySelector("title")?.textContent || "";
        const call = firstEdgeCall(edge.dataset.edge);
        edge.classList.add(`kind-${callKind(call)}`);
      });
      applyGraphState();
      renderMinimap();
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
      if (event.button !== 0 || !viewport() || event.target.closest?.("#minimap")) {
        return;
      }

      event.preventDefault();
      view.isPanning = true;
      view.moved = false;
      view.startX = event.clientX;
      view.startY = event.clientY;
      view.startOffsetX = view.offsetX;
      view.startOffsetY = view.offsetY;
      view.startTarget = event.target;
      canvas.classList.add("is-panning");
      canvas.setPointerCapture(event.pointerId);
    });

    canvas.addEventListener("pointermove", (event) => {
      if (!view.isPanning) {
        return;
      }

      const deltaX = event.clientX - view.startX;
      const deltaY = event.clientY - view.startY;
      view.moved = view.moved || Math.hypot(deltaX, deltaY) > 4;
      view.offsetX = view.startOffsetX + deltaX;
      view.offsetY = view.startOffsetY + deltaY;
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
      if (!view.moved) {
        selectFromElement(view.startTarget);
      }
    }

    canvas.addEventListener("pointerup", stopPanning);
    canvas.addEventListener("pointercancel", stopPanning);
    splitter.addEventListener("pointerdown", (event) => {
      if (event.button !== 0) {
        return;
      }

      event.preventDefault();
      view.resizeStartX = event.clientX;
      view.resizeStartWidth = inspector.getBoundingClientRect().width;
      document.body.classList.add("resizing");
      splitter.setPointerCapture(event.pointerId);
    });
    splitter.addEventListener("pointermove", (event) => {
      if (view.resizeStartX === undefined) {
        return;
      }

      const deltaX = event.clientX - view.resizeStartX;
      setInspectorWidth(view.resizeStartWidth - deltaX);
    });

    function stopResizing(event) {
      if (view.resizeStartX === undefined) {
        return;
      }

      delete view.resizeStartX;
      delete view.resizeStartWidth;
      document.body.classList.remove("resizing");
      if (splitter.hasPointerCapture(event.pointerId)) {
        splitter.releasePointerCapture(event.pointerId);
      }
    }

    splitter.addEventListener("pointerup", stopResizing);
    splitter.addEventListener("pointercancel", stopResizing);
    splitter.addEventListener("dblclick", () => setInspectorWidth(384));
    window.addEventListener("resize", () => {
      setInspectorWidth(inspector.getBoundingClientRect().width, false);
      updateMinimap();
    });
    filter.addEventListener("input", () => {
      state.searchIndex = 0;
      applyGraphState();
    });
    searchPrevButton.addEventListener("click", () => focusSearchMatch(-1));
    searchNextButton.addEventListener("click", () => focusSearchMatch(1));
    resetButton.addEventListener("click", () => {
      state.isolate = false;
      resetView();
      applyGraphState();
    });
    isolateButton.addEventListener("click", () => {
      state.isolate = !state.isolate;
      applyGraphState();
    });
    hideIsolatedButton.addEventListener("click", () => {
      state.hideIsolated = !state.hideIsolated;
      applyGraphState();
    });
    layoutPreset.addEventListener("change", () => {
      state.preset = layoutPreset.value;
      applyGraphState();
    });
    fileFilters.addEventListener("click", (event) => {
      const item = event.target.closest("button");
      if (!item) {
        return;
      }

      if (item.dataset.fileAction === "all") {
        state.activeFiles = new Set(state.allFiles);
      } else if (item.dataset.fileAction === "none") {
        state.activeFiles = new Set();
      } else if (item.dataset.file) {
        if (state.activeFiles.has(item.dataset.file)) {
          state.activeFiles.delete(item.dataset.file);
        } else {
          state.activeFiles.add(item.dataset.file);
        }
      }
      applyGraphState();
    });
    inspector.addEventListener("click", (event) => {
      const sourceMode = event.target.closest("[data-source-mode]");
      if (sourceMode) {
        state.sourceMode = sourceMode.dataset.sourceMode;
        if (state.selectedNode) {
          renderNodeInspector(state.selectedNode);
        } else if (state.selectedEdge) {
          renderEdgeInspector(state.selectedEdge);
        }
        return;
      }

      const copy = event.target.closest("[data-copy-text]");
      if (copy) {
        navigator.clipboard?.writeText(copy.dataset.copyText);
        copy.textContent = "Copied";
        return;
      }

      const item = event.target.closest("[data-node-id]");
      if (!item) {
        return;
      }
      if (item.dataset.edgeKey) {
        state.selectedEdge = item.dataset.edgeKey;
      }
      selectNode(item.dataset.nodeId);
    });

    canvas.addEventListener("pointerdown", (event) => {
      if (event.button !== 0 || !event.target.closest?.("#minimap")) {
        return;
      }

      event.preventDefault();
      view.isMiniPanning = true;
      panFromMinimap(event);
      canvas.setPointerCapture(event.pointerId);
    });
    canvas.addEventListener("pointermove", (event) => {
      if (view.isMiniPanning) {
        panFromMinimap(event);
      }
    });
    canvas.addEventListener("pointerup", (event) => {
      if (!view.isMiniPanning) {
        return;
      }
      view.isMiniPanning = false;
      if (canvas.hasPointerCapture(event.pointerId)) {
        canvas.releasePointerCapture(event.pointerId);
      }
    });

    document.addEventListener("keydown", (event) => {
      if (event.target.matches?.("input, textarea, select")) {
        if (event.key === "Escape") {
          event.target.blur();
        }
        return;
      }

      if (event.key === "/") {
        event.preventDefault();
        filter.focus();
      } else if (event.key === "Escape") {
        filter.value = "";
        state.isolate = false;
        applyGraphState();
      } else if (event.key === "r") {
        resetView();
      } else if (event.key === "i") {
        state.isolate = !state.isolate;
        applyGraphState();
      } else if (event.key === "h") {
        state.hideIsolated = !state.hideIsolated;
        applyGraphState();
      } else if (event.key === "n" || event.key === "]") {
        focusSearchMatch(1);
      } else if (event.key === "p" || event.key === "[") {
        focusSearchMatch(-1);
      } else if (event.key === "?") {
        inspector.innerHTML = `
          <h2>Shortcuts</h2>
          <ul class="call-list">
            <li><strong>/</strong><br><span class="muted">Focus search</span></li>
            <li><strong>n / ]</strong><br><span class="muted">Next search match</span></li>
            <li><strong>p / [</strong><br><span class="muted">Previous search match</span></li>
            <li><strong>i</strong><br><span class="muted">Toggle isolate selected</span></li>
            <li><strong>h</strong><br><span class="muted">Toggle hide isolated</span></li>
            <li><strong>r</strong><br><span class="muted">Reset view</span></li>
            <li><strong>Esc</strong><br><span class="muted">Clear filter and isolate</span></li>
          </ul>
        `;
      }
    });

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
          wireGraphElements();
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

      const depth = layoutDepths(nodes, calls, incoming, outgoing);

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
          element.dataset.id = node.id;
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
        line.classList.add("edge", `kind-${callKind(call)}`);
        line.dataset.edge = edgeKey(call);
        line.setAttribute("x1", caller.x);
        line.setAttribute("y1", caller.y);
        line.setAttribute("x2", callee.x);
        line.setAttribute("y2", callee.y);
        line.setAttribute("stroke", "#934f12");
        line.setAttribute("stroke-width", "1.4");
        line.setAttribute("marker-end", "url(#arrow)");
        edgeLayer.appendChild(line);
      });
      applyGraphState();
      renderMinimap();
    }

    function layoutDepths(nodes, calls, incoming, outgoing) {
      const largeGraph = nodes.length > 1500 || calls.length > 3000;
      if (largeGraph) {
        const files = [...new Set(nodes.map((node) => node.file))].sort();
        const fileIndex = new Map(files.map((file, index) => [file, index]));
        return new Map(nodes.map((node) => [node.id, fileIndex.get(node.file) || 0]));
      }

      const depth = new Map(nodes.map((node) => [node.id, 0]));
      const roots = nodes.filter((node) => (incoming.get(node.id) || 0) === 0).map((node) => node.id);
      const queue = roots.length ? roots : nodes.slice(0, 64).map((node) => node.id);
      const seen = new Set(queue);

      for (let cursor = 0; cursor < queue.length; cursor += 1) {
        const id = queue[cursor];
        const nextDepth = (depth.get(id) || 0) + 1;
        (outgoing.get(id) || []).forEach((callee) => {
          if (!seen.has(callee)) {
            seen.add(callee);
            depth.set(callee, nextDepth);
            queue.push(callee);
          }
        });
      }

      nodes.forEach((node, index) => {
        if (!seen.has(node.id)) {
          depth.set(node.id, Math.min(index, 12));
        }
      });

      return depth;
    }

    Promise.all([
      fetchJson("/graph.json"),
      fetchJson("/source.json").catch(() => ({ functions: [] }))
    ])
      .then(([graph, source]) => {
        loadInspectorWidth();
        initData(graph, source);
        renderHome();
        return renderSvg().catch(() => fallbackDraw(graph));
      })
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
        assert!(html.contains("source.json"));
        assert!(html.contains("Inspector"));
        assert!(html.contains("Resize inspector"));
        assert!(html.contains("coviz.quick.inspectorWidth"));
        assert!(html.contains("Graph minimap"));
        assert!(html.contains("Hide isolated"));
        assert!(html.contains("layout-preset"));
        assert!(html.contains("search-next"));
        assert!(html.contains("Open in editor"));
        assert!(html.contains("data-source-mode=\"wide\""));
    }
}
