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
      --controls-width: 19rem;
      --inspector-width: 24rem;
      display: grid;
      grid-template-columns: var(--controls-width) minmax(0, 1fr) var(--inspector-width);
      gap: 0.65rem;
      height: calc(100vh - 4.1rem);
      padding: 1rem;
    }

    main.hide-controls {
      --controls-width: 0;
    }

    main.hide-inspector {
      --inspector-width: 0;
    }

    .panel-toggle {
      display: inline-flex;
      gap: 0.4rem;
      align-items: center;
    }

    .control-panel,
    .inspector {
      height: calc(100vh - 6.1rem);
      overflow: auto;
      border: 1px solid #6f7888;
      background: #f6f1e2;
      box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.55);
      padding: 1rem;
    }

    main.hide-controls .control-panel,
    main.hide-inspector .inspector {
      border: 0;
      overflow: hidden;
      padding: 0;
      pointer-events: none;
      visibility: hidden;
    }

    .control-panel h2 {
      margin: 0 0 0.75rem;
      font-size: 1.1rem;
    }

    .toolbar {
      display: flex;
      flex-direction: column;
      flex-wrap: nowrap;
      gap: 0.75rem;
      align-items: stretch;
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

    .path-input {
      width: 11rem;
      padding: 0.52rem 0.65rem;
    }

    .search-tools {
      display: flex;
      min-width: 0;
      flex: 0 0 auto;
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
      align-items: center;
      flex: 0 0 auto;
      margin: 0 0 0.75rem;
      min-height: 2.65rem;
      max-width: 100%;
      overflow: visible;
      padding: 0.2rem 0 0.45rem;
      position: relative;
      z-index: 2;
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
      flex: 0 0 auto;
      padding: 0.35rem 0.58rem;
    }

    .chip.active {
      background: #fff8d4;
      color: var(--ink);
      border-color: var(--accent);
    }

    #canvas {
      position: relative;
      flex: 1 1 auto;
      min-height: 0;
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

    .graph-canvas {
      display: block;
      height: 100%;
      width: 100%;
    }

    .virtual-graph {
      display: block;
      height: 100%;
      min-height: 100%;
      position: relative;
      width: 100%;
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
    #canvas svg .edge.selected polygon,
    #canvas svg .edge.path path,
    #canvas svg .edge.path polygon {
      stroke: #d12f1f;
      fill: #d12f1f;
    }

    #canvas line.edge.selected,
    #canvas line.edge.path {
      stroke: #d12f1f;
    }

    #canvas svg .node.path ellipse,
    #canvas svg .node.path polygon,
    #canvas svg .node.path path,
    #canvas svg .node.hovered ellipse,
    #canvas svg .node.hovered polygon,
    #canvas svg .node.hovered path {
      stroke: #d12f1f;
      stroke-width: 3px;
    }

    #canvas svg .node.trace ellipse,
    #canvas svg .node.trace polygon,
    #canvas svg .node.trace path {
      fill: #b9e1ea;
    }

    .node.path,
    .node.hovered {
      outline: 3px solid #d12f1f;
      outline-offset: 3px;
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
      display: flex;
      flex-direction: column;
      height: 100%;
      min-width: 0;
      min-height: 0;
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
    <div class="panel-toggle">
      <button id="toggle-controls" type="button">Controls</button>
      <button id="toggle-inspector" type="button">Inspector</button>
      <div class="summary">__FUNCTION_COUNT__ functions / __CALL_COUNT__ calls</div>
    </div>
  </header>
  <main>
    <aside id="controls" class="control-panel" aria-label="Controls">
      <h2>Controls</h2>
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
            <option value="trace">Trace flow</option>
            <option value="by-folder">By folder flow</option>
            <option value="by-file">By file flow</option>
            <option value="fan-in">Fan-in only</option>
            <option value="fan-out">Fan-out only</option>
            <option value="cycles">Cycles</option>
          </select>
          <button id="reset-view" type="button">Reset view</button>
          <button id="isolate" type="button">Isolate selected</button>
          <button id="hide-isolated" type="button">Hide isolated</button>
        </div>
        <div class="toolbar-group">
          <button id="trace-entry" type="button" title="Trace from project entrypoint">Entry flow</button>
          <button id="trace-selected" type="button" title="Trace from selected function">Trace selected</button>
        </div>
        <div class="toolbar-group">
          <input id="path-from" class="path-input" type="search" placeholder="Path from" autocomplete="off">
          <input id="path-to" class="path-input" type="search" placeholder="Path to" autocomplete="off">
          <button id="find-path" type="button">Find path</button>
          <button id="clear-path" type="button">Clear path</button>
          <button id="collapse-clusters" type="button">Collapse</button>
          <button id="expand-clusters" type="button">Expand</button>
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
    </aside>
    <section class="workspace">
      <section id="canvas" aria-label="Call graph">
        <div class="empty">Loading graph...</div>
      </section>
    </section>
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
    const mainElement = document.querySelector("main");
    const toggleControlsButton = document.querySelector("#toggle-controls");
    const toggleInspectorButton = document.querySelector("#toggle-inspector");
    const resetButton = document.querySelector("#reset-view");
    const isolateButton = document.querySelector("#isolate");
    const hideIsolatedButton = document.querySelector("#hide-isolated");
    const layoutPreset = document.querySelector("#layout-preset");
    const traceEntryButton = document.querySelector("#trace-entry");
    const traceSelectedButton = document.querySelector("#trace-selected");
    const pathFromInput = document.querySelector("#path-from");
    const pathToInput = document.querySelector("#path-to");
    const findPathButton = document.querySelector("#find-path");
    const clearPathButton = document.querySelector("#clear-path");
    const collapseClustersButton = document.querySelector("#collapse-clusters");
    const expandClustersButton = document.querySelector("#expand-clusters");
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
      selectedGroup: null,
      hoverNode: null,
      hoverEdge: null,
      isolate: false,
      hideIsolated: false,
      preset: "all",
      entrypointNode: null,
      traceRoot: null,
      traceCache: null,
      pathFrom: null,
      pathTo: null,
      pathNodes: new Set(),
      pathEdges: new Set(),
      collapsedFiles: new Set(),
      searchMatches: [],
      searchIndex: -1,
      sourceMode: "context",
      canvasRenderer: null
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
      startTarget: null,
      redrawPending: false,
      minimapPending: false,
      lastWheelAt: 0,
      wheelSettleTimer: null
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

    function setPanelVisibility(panel, visible) {
      const key = panel === "controls" ? "hide-controls" : "hide-inspector";
      mainElement.classList.toggle(key, !visible);
      localStorage.setItem(`coviz.quick.${panel}Visible`, visible ? "1" : "0");
      toggleControlsButton.classList.toggle("active", !mainElement.classList.contains("hide-controls"));
      toggleInspectorButton.classList.toggle("active", !mainElement.classList.contains("hide-inspector"));
      requestAnimationFrame(() => {
        if (state.canvasRenderer) {
          drawCanvasGraph();
          renderMinimap();
        } else {
          updateMinimap();
        }
      });
    }

    function loadPanelVisibility() {
      setPanelVisibility("controls", localStorage.getItem("coviz.quick.controlsVisible") !== "0");
      setPanelVisibility("inspector", localStorage.getItem("coviz.quick.inspectorVisible") !== "0");
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

    function invalidateTrace() {
      state.traceCache = null;
    }

    function functionSearchText(item) {
      return `${item?.name || ""} ${item?.file || ""}:${item?.line || ""}`.toLowerCase();
    }

    function findFunctionByQuery(query) {
      const value = String(query || "").trim().toLowerCase();
      if (!value) {
        return null;
      }

      const items = [...state.functions.values()];
      return items.find((item) => item.id.toLowerCase() === value)
        || items.find((item) => item.name.toLowerCase() === value)
        || items.find((item) => `${item.file}:${item.line}`.toLowerCase() === value)
        || items.find((item) => functionSearchText(item).includes(value))
        || null;
    }

    function defaultEntrypointId() {
      const ranked = [...state.functions.values()]
        .sort((left, right) =>
          entrypointRankForFunction(left) - entrypointRankForFunction(right)
          || (state.outgoing.get(right.id) || []).length - (state.outgoing.get(left.id) || []).length
          || left.file.localeCompare(right.file)
          || left.line - right.line
        );
      return ranked[0]?.id || null;
    }

    function setTraceRoot(id) {
      state.traceRoot = id || state.entrypointNode || defaultEntrypointId();
      state.preset = "trace";
      state.selectedGroup = null;
      invalidateTrace();
      if (state.traceRoot) {
        state.selectedNode = state.traceRoot;
        state.selectedEdge = null;
        renderNodeInspector(state.traceRoot);
      }
      applyGraphState();
    }

    function traceGraph() {
      const root = state.traceRoot || state.entrypointNode || defaultEntrypointId();
      const cacheKey = `${root}`;
      if (state.traceCache?.key === cacheKey) {
        return state.traceCache;
      }

      const nodes = new Set();
      const edges = new Set();
      const limit = 1200;
      if (!root) {
        state.traceCache = { key: cacheKey, root, nodes, edges };
        return state.traceCache;
      }

      const queue = [root];
      nodes.add(root);
      for (let cursor = 0; cursor < queue.length; cursor += 1) {
        const id = queue[cursor];
        if (nodes.size >= limit) {
          continue;
        }
        for (const call of state.outgoing.get(id) || []) {
          if (nodes.has(call.callee)) {
            edges.add(edgeKey(call));
            continue;
          }
          if (nodes.size >= limit) {
            continue;
          }
          nodes.add(call.callee);
          edges.add(edgeKey(call));
          queue.push(call.callee);
        }
      }

      state.traceCache = { key: cacheKey, root, nodes, edges };
      return state.traceCache;
    }

    function findShortestPath(fromId, toId) {
      if (!fromId || !toId) {
        return null;
      }
      const queue = [fromId];
      const previous = new Map([[fromId, null]]);
      const previousEdge = new Map();

      for (let cursor = 0; cursor < queue.length; cursor += 1) {
        const id = queue[cursor];
        if (id === toId) {
          break;
        }
        for (const call of state.outgoing.get(id) || []) {
          if (previous.has(call.callee)) {
            continue;
          }
          previous.set(call.callee, id);
          previousEdge.set(call.callee, edgeKey(call));
          queue.push(call.callee);
        }
      }

      if (!previous.has(toId)) {
        return null;
      }

      const nodes = [];
      const edges = [];
      for (let id = toId; id; id = previous.get(id)) {
        nodes.push(id);
        const key = previousEdge.get(id);
        if (key) {
          edges.push(key);
        }
      }
      nodes.reverse();
      edges.reverse();
      return { nodes, edges };
    }

    function setPath(fromId, toId) {
      const result = findShortestPath(fromId, toId);
      state.pathFrom = fromId;
      state.pathTo = toId;
      state.pathNodes = new Set(result?.nodes || []);
      state.pathEdges = new Set(result?.edges || []);
      const fromItem = state.functions.get(fromId);
      const toItem = state.functions.get(toId);
      pathFromInput.value = fromItem?.name || fromId || "";
      pathToInput.value = toItem?.name || toId || "";
      if (result) {
        for (const id of result.nodes) {
          const item = state.functions.get(id);
          if (item) {
            state.collapsedFiles.delete(item.file);
          }
        }
        renderFileFilters();
      }
      if (fromId) {
        state.selectedNode = fromId;
        state.selectedEdge = null;
        renderPathInspector(result, fromId, toId);
        applyGraphState();
        centerNode(fromId);
      } else {
        applyGraphState();
      }
    }

    function clearPath() {
      state.pathFrom = null;
      state.pathTo = null;
      state.pathNodes = new Set();
      state.pathEdges = new Set();
      pathFromInput.value = "";
      pathToInput.value = "";
      applyGraphState();
    }

    function edgeInPath(callOrKey) {
      const key = typeof callOrKey === "string" ? callOrKey : edgeKey(callOrKey);
      return state.pathEdges.has(key);
    }

    function nodeInPath(id) {
      return state.pathNodes.has(id);
    }

    function activeTraceGraph() {
      return state.preset === "trace" ? traceGraph() : null;
    }

    function nodeInActiveTrace(id) {
      const trace = activeTraceGraph();
      return !trace || trace.nodes.has(id);
    }

    function edgeInActiveTrace(callOrKey) {
      const trace = activeTraceGraph();
      if (!trace) {
        return true;
      }
      const key = typeof callOrKey === "string" ? callOrKey : edgeKey(callOrKey);
      return trace.edges.has(key);
    }

    function hasFocusedCanvasCalls() {
      return Boolean(state.pathEdges.size || state.selectedNode || state.selectedEdge || state.hoverNode);
    }

    function canvasInteractionActive() {
      return Boolean(view.isPanning || view.isMiniPanning || performance.now() - (view.lastWheelAt || 0) < 180);
    }

    function fileCollapsed(file) {
      return state.collapsedFiles.has(file);
    }

    function scheduleCanvasDraw() {
      if (!state.canvasRenderer?.canvas) {
        return;
      }
      if (view.redrawPending) {
        return;
      }
      view.redrawPending = true;
      requestAnimationFrame(() => {
        view.redrawPending = false;
        drawCanvasGraph();
      });
    }

    function scheduleMinimapUpdate() {
      if (view.minimapPending) {
        return;
      }
      view.minimapPending = true;
      requestAnimationFrame(() => {
        view.minimapPending = false;
        updateMinimap();
      });
    }

    function applyViewTransform() {
      const target = viewport();
      if (!target) {
        return;
      }

      if (state.canvasRenderer) {
        target.style.transform = "";
        scheduleCanvasDraw();
      } else {
        target.style.transform = `translate(${view.offsetX}px, ${view.offsetY}px) scale(${view.scale})`;
      }
      scheduleMinimapUpdate();
    }

    function resetView() {
      view.scale = 1;
      view.offsetX = 0;
      view.offsetY = 0;
      applyViewTransform();
    }

    function setGraphContent(html) {
      state.canvasRenderer = null;
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

      state.entrypointNode = defaultEntrypointId();
      state.traceRoot = state.entrypointNode;
      invalidateTrace();
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
      const collapsedCount = state.collapsedFiles.size;
      fileFilters.innerHTML = `
        <button class="chip ${allActive ? "active" : ""}" type="button" data-file-action="all">All files</button>
        <button class="chip" type="button" data-file-action="none">None</button>
        <span class="count-pill">${collapsedCount} collapsed</span>
        ${state.allFiles.map((file) => `
          <button class="chip ${state.activeFiles.has(file) ? "active" : ""}" type="button" data-file="${escapeHtml(file)}" title="${escapeHtml(file)}">
            ${escapeHtml(fileLabel(file))}${state.collapsedFiles.has(file) ? " +" : ""}
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
      if (state.preset === "trace") {
        return traceGraph().nodes.has(id);
      }
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
      if (state.preset === "trace") {
        return traceGraph().edges.has(edgeKey(call));
      }
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

    function nodeRenderVisible(id) {
      const item = state.functions.get(id);
      return nodeBaseVisible(id) && !(item && fileCollapsed(item.file));
    }

    function nodeDisplayVisible(id) {
      return nodeRenderVisible(id) && nodeInActiveTrace(id);
    }

    function edgeTouchesCollapsedFile(call) {
      const caller = state.functions.get(call.caller);
      const callee = state.functions.get(call.callee);
      return Boolean((caller && fileCollapsed(caller.file)) || (callee && fileCollapsed(callee.file)));
    }

    function callDisplayVisible(call) {
      return Boolean(
        call
        && edgeBaseVisible(call, call.caller, call.callee)
        && !edgeTouchesCollapsedFile(call)
        && edgeInActiveTrace(call)
      );
    }

    function activeGroupMode() {
      if (state.preset === "by-folder" || state.preset === "by-file") {
        return state.preset;
      }
      return null;
    }

    function groupKeyForFile(file, mode = "by-folder") {
      const parts = String(file || "source").split("/").filter(Boolean);
      if (!parts.length) {
        return "source";
      }
      if (mode === "by-file") {
        return file;
      }
      if (parts[0] === "crates" && parts.length >= 2) {
        return `${parts[0]}/${parts[1]}`;
      }
      if (parts[0] === "src" && parts.length >= 2) {
        return `${parts[0]}/${parts[1]}`;
      }
      return parts.slice(0, Math.min(2, parts.length)).join("/");
    }

    function groupIdForFunction(id, mode = activeGroupMode() || "by-folder") {
      const item = state.functions.get(id);
      return item ? groupKeyForFile(item.file, mode) : null;
    }

    function graphElementBounds() {
      if (state.canvasRenderer) {
        const mode = activeGroupMode();
        if (mode) {
          return groupGraph(mode).bounds;
        }
        return state.canvasRenderer.bounds;
      }

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

      if (state.canvasRenderer) {
        minimapGraph.innerHTML = '<canvas class="graph-canvas" aria-hidden="true"></canvas>';
        drawCanvasMinimap();
        updateMinimap();
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
      minimapGraph.style.transform = state.canvasRenderer ? "" : `translate(${graphLeft}px, ${graphTop}px) scale(${scale})`;

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
        <div class="source-actions">
          <button type="button" data-trace-node="${id}">Trace from here</button>
          <button type="button" data-path-from="${id}">Path from</button>
          <button type="button" data-path-to="${id}">Path to</button>
          <button type="button" data-collapse-file="${escapeHtml(item.file)}">${state.collapsedFiles.has(item.file) ? "Expand file" : "Collapse file"}</button>
        </div>
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

    function renderPathInspector(result, fromId, toId) {
      const from = state.functions.get(fromId);
      const to = state.functions.get(toId);
      if (!result) {
        inspector.innerHTML = `
          <h2>Path finder</h2>
          <p>No path found from <strong>${escapeHtml(from?.name || fromId)}</strong> to <strong>${escapeHtml(to?.name || toId)}</strong>.</p>
          <p class="muted">Try a broader target name or use Trace selected from the source function.</p>
        `;
        return;
      }

      inspector.innerHTML = `
        <h2>Path finder</h2>
        <p class="muted">${result.nodes.length} functions / ${result.edges.length} calls</p>
        <div class="breadcrumb" aria-label="Found path">
          ${result.nodes.map((id, index) => {
            const item = state.functions.get(id);
            return `${index ? "<span>-></span>" : ""}<button type="button" data-node-id="${id}" title="${escapeHtml(item?.file || "")}:${item?.line || ""}">${escapeHtml(item?.name || id)}</button>`;
          }).join("")}
        </div>
        <h3>Path steps</h3>
        <ul class="call-list">
          ${result.edges.map((key) => {
            const { caller, callee } = splitEdgeKey(key);
            const callerItem = state.functions.get(caller);
            const calleeItem = state.functions.get(callee);
            return `<li data-node-id="${callee}" data-edge-key="${key}">${escapeHtml(callerItem?.name || caller)} -> ${escapeHtml(calleeItem?.name || callee)}<br><span class="muted">${escapeHtml(calleeItem?.file || "")}:${calleeItem?.line || ""}</span></li>`;
          }).join("")}
        </ul>
      `;
    }

    function renderGroupInspector(id) {
      const graph = groupGraph(activeGroupMode() || "by-folder");
      const group = graph.groups.get(id);
      if (!group) {
        renderHome();
        return;
      }

      const topFunctions = group.functionIds
        .map((functionId) => {
          const item = state.functions.get(functionId);
          return { item, count: (state.outgoing.get(functionId) || []).length };
        })
        .filter(({ item }) => Boolean(item))
        .sort((left, right) => right.count - left.count || left.item.name.localeCompare(right.item.name))
        .slice(0, 8);
      const outgoing = graph.edges
        .filter((edge) => edge.caller === id)
        .sort((left, right) => right.count - left.count)
        .slice(0, 8);
      const incoming = graph.edges
        .filter((edge) => edge.callee === id)
        .sort((left, right) => right.count - left.count)
        .slice(0, 8);

      inspector.innerHTML = `
        <h2>${escapeHtml(group.label)}</h2>
        <p class="muted">${group.files.size} files / ${group.functionIds.length} functions / ${group.internalCalls} internal calls</p>
        <div class="stat-grid">
          <div class="stat"><strong>${group.incomingCalls}</strong><span>incoming calls</span></div>
          <div class="stat"><strong>${group.outgoingCalls}</strong><span>outgoing calls</span></div>
        </div>
        <h3>Outgoing groups</h3>
        ${renderGroupEdgeList(outgoing, "callee", graph)}
        <h3>Incoming groups</h3>
        ${renderGroupEdgeList(incoming, "caller", graph)}
        <h3>Highest fan-out functions</h3>
        <ul class="call-list">
          ${topFunctions.map(({ item, count }) => `<li data-node-id="${item.id}">${escapeHtml(item.name)} <span class="muted">${count} calls</span><br><span class="muted">${escapeHtml(item.file)}:${item.line}</span></li>`).join("")}
        </ul>
      `;
    }

    function renderGroupEdgeList(edges, side, graph) {
      if (!edges.length) {
        return '<p class="muted">None.</p>';
      }

      return `<ul class="call-list">${edges.map((edge) => {
        const target = graph.groups.get(edge[side]);
        return `<li data-group-id="${escapeHtml(edge[side])}">${escapeHtml(target?.label || edge[side])}<br><span class="muted">${edge.count} calls</span></li>`;
      }).join("")}</ul>`;
    }

    function selectNode(id) {
      const item = state.functions.get(id);
      if (item && state.collapsedFiles.delete(item.file)) {
        renderFileFilters();
      }
      state.selectedNode = id;
      state.selectedEdge = null;
      state.selectedGroup = null;
      renderNodeInspector(id);
      applyGraphState();
    }

    function selectEdge(key) {
      state.selectedEdge = key;
      state.selectedNode = null;
      state.selectedGroup = null;
      renderEdgeInspector(key);
      applyGraphState();
    }

    function selectGroup(id) {
      state.selectedGroup = id;
      state.selectedNode = null;
      state.selectedEdge = null;
      renderGroupInspector(id);
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

    function selectFromPoint(clientX, clientY, target) {
      if (state.canvasRenderer && target?.closest?.("#graph-canvas")) {
        if (activeGroupMode()) {
          const group = hitCanvasGroup(clientX, clientY);
          if (group) {
            selectGroup(group);
          }
        } else {
          const hit = hitCanvasNode(clientX, clientY);
          if (hit) {
            selectNode(hit);
            return;
          }
          const file = hitCanvasFileCluster(clientX, clientY);
          if (file) {
            if (state.collapsedFiles.has(file)) {
              state.collapsedFiles.delete(file);
            } else {
              state.collapsedFiles.add(file);
            }
            renderFileFilters();
            applyGraphState();
          }
        }
        return;
      }

      selectFromElement(target);
    }

    function hitCanvasGroup(clientX, clientY) {
      const mode = activeGroupMode();
      if (!state.canvasRenderer || !mode) {
        return null;
      }

      const graph = groupGraph(mode);
      const rect = canvas.getBoundingClientRect();
      const graphX = (clientX - rect.left - view.offsetX) / view.scale;
      const graphY = (clientY - rect.top - view.offsetY) / view.scale;
      for (const [id, point] of graph.positions) {
        const group = graph.groups.get(id);
        if (!groupVisible(group)) {
          continue;
        }
        const dx = Math.abs(graphX - point.x);
        const dy = Math.abs(graphY - point.y);
        if (dx <= graph.nodeWidth / 2 && dy <= graph.nodeHeight / 2) {
          return id;
        }
      }
      return null;
    }

    function hitCanvasNode(clientX, clientY) {
      const renderer = state.canvasRenderer;
      if (!renderer) {
        return null;
      }

      const rect = canvas.getBoundingClientRect();
      const graphX = (clientX - rect.left - view.offsetX) / view.scale;
      const graphY = (clientY - rect.top - view.offsetY) / view.scale;
      const isolated = neighborhood();
      for (const [id, point] of renderer.positions) {
        const visible = isolated ? nodeBaseVisible(id) && isolated.has(id) : nodeDisplayVisible(id);
        if (!visible) {
          continue;
        }
        const dx = Math.abs(graphX - point.x);
        const dy = Math.abs(graphY - point.y);
        if (dx <= renderer.nodeWidth / 2 && dy <= renderer.nodeHeight / 2) {
          return id;
        }
      }
      return null;
    }

    function hitCanvasFileCluster(clientX, clientY) {
      const renderer = state.canvasRenderer;
      if (!renderer?.fileClusters?.length) {
        return null;
      }
      const rect = canvas.getBoundingClientRect();
      const graphX = (clientX - rect.left - view.offsetX) / view.scale;
      const graphY = (clientY - rect.top - view.offsetY) / view.scale;
      for (const cluster of renderer.fileClusters) {
        if (!state.activeFiles.has(cluster.file)) {
          continue;
        }
        if (
          graphX >= cluster.x
          && graphX <= cluster.x + cluster.width
          && graphY >= cluster.y
          && graphY <= cluster.y + cluster.height
        ) {
          return cluster.file;
        }
      }
      return null;
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

    function hoverNeighborhood() {
      const id = state.hoverNode;
      if (!id) {
        return null;
      }
      const ids = new Set([id]);
      (state.incoming.get(id) || []).forEach((call) => ids.add(call.caller));
      (state.outgoing.get(id) || []).forEach((call) => ids.add(call.callee));
      return ids;
    }

    function setHoverNode(id) {
      if (state.hoverNode === id) {
        return;
      }
      state.hoverNode = id;
      if (state.canvasRenderer) {
        scheduleCanvasDraw();
      } else {
        applyGraphState();
      }
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
      if (state.canvasRenderer) {
        const mode = activeGroupMode();
        const groupId = mode ? groupIdForFunction(id, mode) : null;
        const point = groupId
          ? groupGraph(mode).positions.get(groupId)
          : state.canvasRenderer.positions.get(id);
        if (!point) {
          return;
        }
        view.offsetX = canvas.clientWidth / 2 - point.x * view.scale;
        view.offsetY = canvas.clientHeight / 2 - point.y * view.scale;
        applyViewTransform();
        return;
      }

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

    function centerGroup(id) {
      const mode = activeGroupMode();
      if (!state.canvasRenderer || !mode) {
        return;
      }
      const point = groupGraph(mode).positions.get(id);
      if (!point) {
        return;
      }
      view.offsetX = canvas.clientWidth / 2 - point.x * view.scale;
      view.offsetY = canvas.clientHeight / 2 - point.y * view.scale;
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
      const isolated = neighborhood();
      const hovered = hoverNeighborhood();
      updateSearchMatches(query);
      traceEntryButton.classList.toggle("active", state.preset === "trace" && state.traceRoot === state.entrypointNode);
      traceSelectedButton.classList.toggle("active", state.preset === "trace" && state.selectedNode === state.traceRoot);
      if (state.canvasRenderer) {
        isolateButton.classList.toggle("active", state.isolate);
        hideIsolatedButton.classList.toggle("active", state.hideIsolated);
        layoutPreset.value = state.preset;
        renderFileFilters();
        scheduleCanvasDraw();
        drawCanvasMinimap();
        updateMinimap();
        return;
      }

      document.querySelectorAll("#canvas .node").forEach((node) => {
        const id = node.dataset.id || node.querySelector?.("title")?.textContent;
        const visible = isolated ? nodeBaseVisible(id) : nodeDisplayVisible(id);
        const trace = state.preset === "trace" && traceGraph().nodes.has(id);
        const path = nodeInPath(id);
        const dim = visible && (
          !nodeMatchesQuery(id, query)
          || (isolated && !isolated.has(id))
          || (hovered && !hovered.has(id))
          || (state.pathNodes.size && !path)
        );
        node.classList.toggle("filtered-out", !visible);
        node.classList.toggle("dimmed", Boolean(dim));
        node.classList.toggle("selected", id === state.selectedNode);
        node.classList.toggle("trace", trace);
        node.classList.toggle("path", path);
        node.classList.toggle("hovered", id === state.hoverNode);
      });

      document.querySelectorAll("#canvas .edge").forEach((edge) => {
        const key = edge.dataset.edge || edge.querySelector?.("title")?.textContent;
        const { caller, callee } = splitEdgeKey(key);
        const call = firstEdgeCall(key);
        const visible = Boolean(
          call
          && edgeBaseVisible(call, caller, callee)
          && (isolated || !edgeTouchesCollapsedFile(call))
          && (isolated || edgeInActiveTrace(call))
        );
        const trace = call && state.preset === "trace" && traceGraph().edges.has(key);
        const path = edgeInPath(key);
        const dim = visible && (
          !edgeMatchesQuery(call, caller, callee, query)
          || (isolated && !(isolated.has(caller) && isolated.has(callee)))
          || (hovered && !(hovered.has(caller) && hovered.has(callee)))
          || (state.pathEdges.size && !path)
        );
        edge.classList.toggle("filtered-out", !visible);
        edge.classList.toggle("dimmed", Boolean(dim));
        edge.classList.toggle("selected", key === state.selectedEdge);
        edge.classList.toggle("trace", Boolean(trace));
        edge.classList.toggle("path", path);
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
      view.lastWheelAt = performance.now();
      clearTimeout(view.wheelSettleTimer);
      view.wheelSettleTimer = setTimeout(scheduleCanvasDraw, 220);
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

    canvas.addEventListener("pointermove", (event) => {
      if (view.isPanning || view.isMiniPanning || !viewport()) {
        return;
      }
      if (state.canvasRenderer && event.target.closest?.("#graph-canvas")) {
        setHoverNode(hitCanvasNode(event.clientX, event.clientY));
        return;
      }
      const node = event.target.closest?.(".node");
      const id = node?.dataset?.id || node?.querySelector?.("title")?.textContent || null;
      setHoverNode(id && state.functions.has(id) ? id : null);
    });
    canvas.addEventListener("pointerleave", () => setHoverNode(null));

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
        selectFromPoint(event.clientX, event.clientY, view.startTarget);
      } else {
        scheduleCanvasDraw();
      }
    }

    canvas.addEventListener("pointerup", stopPanning);
    canvas.addEventListener("pointercancel", stopPanning);
    toggleControlsButton.addEventListener("click", () => {
      setPanelVisibility("controls", mainElement.classList.contains("hide-controls"));
    });
    toggleInspectorButton.addEventListener("click", () => {
      setPanelVisibility("inspector", mainElement.classList.contains("hide-inspector"));
    });
    window.addEventListener("resize", () => {
      scheduleCanvasDraw();
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
      clearPath();
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
    traceEntryButton.addEventListener("click", () => {
      state.isolate = false;
      setTraceRoot(state.entrypointNode || defaultEntrypointId());
      centerNode(state.traceRoot);
    });
    traceSelectedButton.addEventListener("click", () => {
      state.isolate = false;
      setTraceRoot(state.selectedNode || state.entrypointNode || defaultEntrypointId());
      centerNode(state.traceRoot);
    });
    findPathButton.addEventListener("click", () => {
      const from = findFunctionByQuery(pathFromInput.value) || (state.selectedNode ? state.functions.get(state.selectedNode) : null);
      const to = findFunctionByQuery(pathToInput.value);
      if (!from || !to) {
        inspector.innerHTML = '<h2>Path finder</h2><p class="muted">Set a valid source and target function.</p>';
        return;
      }
      state.isolate = false;
      state.preset = "all";
      setPath(from.id, to.id);
    });
    clearPathButton.addEventListener("click", () => clearPath());
    collapseClustersButton.addEventListener("click", () => {
      state.collapsedFiles = new Set([...state.activeFiles]);
      renderFileFilters();
      applyGraphState();
    });
    expandClustersButton.addEventListener("click", () => {
      state.collapsedFiles.clear();
      renderFileFilters();
      applyGraphState();
    });
    layoutPreset.addEventListener("change", () => {
      state.preset = layoutPreset.value;
      state.selectedGroup = null;
      invalidateTrace();
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
        if (event.altKey || event.shiftKey) {
          if (state.collapsedFiles.has(item.dataset.file)) {
            state.collapsedFiles.delete(item.dataset.file);
          } else {
            state.collapsedFiles.add(item.dataset.file);
          }
          renderFileFilters();
          applyGraphState();
          return;
        }
        if (state.activeFiles.has(item.dataset.file)) {
          state.activeFiles.delete(item.dataset.file);
        } else {
          state.activeFiles.add(item.dataset.file);
        }
      }
      invalidateTrace();
      applyGraphState();
    });
    inspector.addEventListener("click", (event) => {
      const traceNode = event.target.closest("[data-trace-node]");
      if (traceNode) {
        setTraceRoot(traceNode.dataset.traceNode);
        centerNode(traceNode.dataset.traceNode);
        return;
      }

      const pathFrom = event.target.closest("[data-path-from]");
      if (pathFrom) {
        const item = state.functions.get(pathFrom.dataset.pathFrom);
        pathFromInput.value = item?.name || pathFrom.dataset.pathFrom;
        state.pathFrom = pathFrom.dataset.pathFrom;
        return;
      }

      const pathTo = event.target.closest("[data-path-to]");
      if (pathTo) {
        const item = state.functions.get(pathTo.dataset.pathTo);
        pathToInput.value = item?.name || pathTo.dataset.pathTo;
        const from = state.pathFrom || findFunctionByQuery(pathFromInput.value)?.id || state.selectedNode;
        setPath(from, pathTo.dataset.pathTo);
        return;
      }

      const collapseFile = event.target.closest("[data-collapse-file]");
      if (collapseFile) {
        const file = collapseFile.dataset.collapseFile;
        if (state.collapsedFiles.has(file)) {
          state.collapsedFiles.delete(file);
        } else {
          state.collapsedFiles.add(file);
        }
        renderFileFilters();
        applyGraphState();
        return;
      }

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

      const group = event.target.closest("[data-group-id]");
      if (group) {
        selectGroup(group.dataset.groupId);
        centerGroup(group.dataset.groupId);
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

    function shouldUseCanvasRenderer(data) {
      return data.functions.length > 1200 || data.calls.length > 3000;
    }

    function fallbackDrawCanvas(data) {
      const layout = canvasLayout(data.functions, data.calls);
      state.canvasRenderer = {
        canvas: null,
        positions: layout.positions,
        bounds: layout.bounds,
        fileClusters: layout.fileClusters,
        fileEdges: layout.fileEdges,
        flowColumns: layout.columns,
        nodeWidth: layout.nodeWidth,
        nodeHeight: layout.nodeHeight,
        edgeBudget: data.calls.length > 8000 ? 3500 : data.calls.length,
        groupCache: new Map()
      };

      canvas.innerHTML = `
        <div id="viewport" class="graph-viewport virtual-graph">
          <canvas id="graph-canvas" class="graph-canvas" aria-label="Virtualized call graph canvas"></canvas>
        </div>
        <div id="minimap" class="minimap" aria-label="Graph minimap" title="Click or drag to move the viewport">
          <div id="minimap-graph" class="minimap-graph"></div>
          <div id="minimap-window" class="minimap-window"></div>
        </div>
      `;
      state.canvasRenderer.canvas = document.querySelector("#graph-canvas");
      view.scale = 1;
      view.offsetX = 24;
      view.offsetY = 24;
      drawCanvasGraph();
      renderMinimap();
      applyGraphState();
    }

    function canvasLayout(nodes, calls) {
      const sorted = [...nodes].sort(compareFunctions);
      const positions = new Map();
      const nodeWidth = 170;
      const nodeHeight = 48;
      if (!sorted.length) {
        return { positions, bounds: { width: 980, height: 680 }, columns: [], fileClusters: [], fileEdges: [], nodeWidth, nodeHeight };
      }

      const graphCalls = calls.filter((call) => state.functions.has(call.caller) && state.functions.has(call.callee));
      const fileLayouts = buildFileLayouts(sorted, graphCalls, nodeWidth, nodeHeight);
      const fileDepths = fileFlowDepths(fileLayouts, graphCalls);
      const filesByDepth = new Map();

      for (const file of fileLayouts) {
        file.depth = fileDepths.get(file.file) || 0;
        if (!filesByDepth.has(file.depth)) {
          filesByDepth.set(file.depth, []);
        }
        filesByDepth.get(file.depth).push(file);
      }

      const depthEntries = [...filesByDepth.entries()].sort((left, right) => left[0] - right[0]);
      const depthGap = 96;
      const laneGap = 36;
      const rowGap = 46;
      const leftPad = 48;
      const topPad = 48;
      const targetHeight = layoutTargetHeight(fileLayouts, topPad);
      let xCursor = leftPad;
      let boundsWidth = 980;
      let boundsHeight = 680;
      const columns = [];

      for (const [depth, files] of depthEntries) {
        files.sort(compareFileLayouts);
        const lanes = packFileLanes(files, targetHeight, rowGap);
        const depthWidth = lanes.reduce((total, lane) => total + lane.width, 0) + Math.max(0, lanes.length - 1) * laneGap;
        const functionCount = files.reduce((total, file) => total + file.nodes.length, 0);
        columns.push({ index: depth, x: xCursor + depthWidth / 2, width: depthWidth, count: functionCount });

        let laneX = xCursor;
        for (const lane of lanes) {
          let yCursor = topPad;
          for (const file of lane.files) {
            file.x = laneX;
            file.y = yCursor;
            placeFileNodes(file, positions, nodeWidth, nodeHeight);
            yCursor += file.height + rowGap;
            boundsWidth = Math.max(boundsWidth, file.x + file.width + leftPad);
            boundsHeight = Math.max(boundsHeight, file.y + file.height + topPad);
          }
          laneX += lane.width + laneGap;
        }

        xCursor += depthWidth + depthGap;
      }

      return {
        positions,
        columns,
        fileClusters: fileLayouts.map((file) => ({
          file: file.file,
          label: file.label,
          x: file.x,
          y: file.y,
          width: file.width,
          height: file.height,
          functionIds: file.nodes.map((node) => node.id),
          functionCount: file.nodes.length,
          internalCalls: file.internalCalls,
          incomingCalls: file.incomingCalls,
          outgoingCalls: file.outgoingCalls
        })),
        fileEdges: aggregateFileEdges(graphCalls),
        nodeWidth,
        nodeHeight,
        bounds: {
          width: boundsWidth,
          height: boundsHeight
        }
      };
    }

    function layoutTargetHeight(fileLayouts, topPad) {
      const area = fileLayouts.reduce((total, file) => total + file.width * file.height, 0);
      const viewportAspect = canvas.clientWidth && canvas.clientHeight
        ? canvas.clientWidth / canvas.clientHeight
        : 1.75;
      const targetAspect = clamp(viewportAspect, 1.45, 2.35);
      const tallestFile = Math.max(0, ...fileLayouts.map((file) => file.height));
      return Math.max(680, tallestFile + topPad * 2, Math.sqrt(area / targetAspect) * 1.15);
    }

    function packFileLanes(files, targetHeight, rowGap) {
      const lanes = [];
      for (const file of files) {
        let bestLane = null;
        for (const lane of lanes) {
          const nextHeight = lane.height + (lane.files.length ? rowGap : 0) + file.height;
          if (nextHeight <= targetHeight && (!bestLane || lane.height < bestLane.height)) {
            bestLane = lane;
          }
        }

        if (!bestLane) {
          bestLane = { files: [], width: 0, height: 0 };
          lanes.push(bestLane);
        }

        bestLane.height += (bestLane.files.length ? rowGap : 0) + file.height;
        bestLane.width = Math.max(bestLane.width, file.width);
        bestLane.files.push(file);
      }
      return lanes;
    }

    function placeFileNodes(file, positions, nodeWidth, nodeHeight) {
      file.layers.forEach(([_, layer], columnIndex) => {
        layer.forEach((node, row) => {
          positions.set(node.id, {
            x: file.x + file.paddingX + nodeWidth / 2 + columnIndex * file.cellWidth,
            y: file.y + file.headerHeight + nodeHeight / 2 + row * file.cellHeight
          });
        });
      });
    }

    function buildFileLayouts(nodes, calls, nodeWidth, nodeHeight) {
      const files = new Map();
      for (const node of nodes) {
        if (!files.has(node.file)) {
          files.set(node.file, {
            file: node.file,
            label: fileLabel(node.file),
            nodes: [],
            internalCalls: 0,
            incomingCalls: 0,
            outgoingCalls: 0
          });
        }
        files.get(node.file).nodes.push(node);
      }

      for (const call of calls) {
        const caller = state.functions.get(call.caller);
        const callee = state.functions.get(call.callee);
        if (!caller || !callee) {
          continue;
        }
        if (caller.file === callee.file) {
          files.get(caller.file).internalCalls += 1;
        } else {
          files.get(caller.file).outgoingCalls += 1;
          files.get(callee.file).incomingCalls += 1;
        }
      }

      const cellWidth = 218;
      const cellHeight = 72;
      const paddingX = 30;
      const headerHeight = 58;

      for (const file of files.values()) {
        file.entryRank = Math.min(...file.nodes.map(entrypointRankForFunction));
        file.nodes.sort(compareFunctions);
        const maxLocalColumns = Math.max(2, Math.min(9, Math.ceil(Math.sqrt(file.nodes.length * 0.85))));
        const depths = nodeDepthsForSubset(file.nodes, calls, maxLocalColumns);
        const layers = new Map();
        for (const node of file.nodes) {
          const layer = depths.get(node.id) || 0;
          if (!layers.has(layer)) {
            layers.set(layer, []);
          }
          layers.get(layer).push(node);
        }
        file.layers = [...layers.entries()].sort((left, right) => left[0] - right[0]);
        orderLayersByNeighborhood(file.layers, calls);
        const maxRows = Math.max(1, ...file.layers.map(([, layer]) => layer.length));
        file.cellWidth = cellWidth;
        file.cellHeight = cellHeight;
        file.paddingX = paddingX;
        file.headerHeight = headerHeight;
        file.width = Math.max(330, (file.layers.length - 1) * cellWidth + nodeWidth + paddingX * 2);
        file.height = Math.max(148, headerHeight + maxRows * cellHeight + 30);
        file.weight = file.nodes.length + file.internalCalls + file.incomingCalls + file.outgoingCalls;
      }

      return [...files.values()].sort(compareFileLayouts);
    }

    function aggregateFileEdges(calls) {
      const edges = new Map();
      for (const call of calls) {
        const callerFile = state.functions.get(call.caller)?.file;
        const calleeFile = state.functions.get(call.callee)?.file;
        if (!callerFile || !calleeFile || callerFile === calleeFile) {
          continue;
        }
        const key = `${callerFile}->${calleeFile}`;
        if (!edges.has(key)) {
          edges.set(key, {
            caller: callerFile,
            callee: calleeFile,
            count: 0,
            kind: callKind(call)
          });
        }
        edges.get(key).count += 1;
      }
      return [...edges.values()].sort((left, right) => right.count - left.count);
    }

    function nodeDepthsForSubset(nodes, calls, maxColumns) {
      const nodeIds = new Set(nodes.map((node) => node.id));
      const outgoing = new Map(nodes.map((node) => [node.id, []]));
      for (const call of calls) {
        if (nodeIds.has(call.caller) && nodeIds.has(call.callee)) {
          outgoing.get(call.caller).push(call.callee);
        }
      }
      const preferredIds = nodes
        .filter((node) => entrypointRankForFunction(node) < 1000000)
        .map((node) => node.id);
      return flowDepthsFromOutgoing([...nodeIds], outgoing, (id) => state.functions.get(id)?.file || id, maxColumns, preferredIds);
    }

    function fileFlowDepths(fileLayouts, calls) {
      const fileIds = fileLayouts.map((file) => file.file);
      const files = new Set(fileIds);
      const outgoing = new Map(fileIds.map((file) => [file, []]));
      for (const call of calls) {
        const callerFile = state.functions.get(call.caller)?.file;
        const calleeFile = state.functions.get(call.callee)?.file;
        if (!callerFile || !calleeFile || callerFile === calleeFile || !files.has(callerFile) || !files.has(calleeFile)) {
          continue;
        }
        outgoing.get(callerFile).push(calleeFile);
      }
      const maxColumns = Math.max(4, Math.min(20, Math.ceil(Math.sqrt(fileLayouts.length * 1.2))));
      const preferredFiles = fileLayouts
        .filter((file) => file.entryRank < 1000000 || isEntrypointFile(file.file))
        .map((file) => file.file);
      return flowDepthsFromOutgoing(fileIds, outgoing, (id) => id, maxColumns, preferredFiles);
    }

    function flowDepthsFromOutgoing(ids, outgoing, labelForId, maxColumns, preferredIds = []) {
      const idSet = new Set(ids);
      const incomingCount = new Map(ids.map((id) => [id, 0]));
      for (const id of ids) {
        outgoing.set(id, (outgoing.get(id) || []).filter((target) => idSet.has(target)));
        for (const target of outgoing.get(id)) {
          incomingCount.set(target, (incomingCount.get(target) || 0) + 1);
        }
      }

      const components = stronglyConnectedComponents(ids, outgoing);
      const componentById = new Map();
      components.forEach((component, index) => {
        component.forEach((id) => componentById.set(id, index));
      });

      const componentData = components.map((componentIds, index) => ({
        id: index,
        ids: componentIds,
        outgoing: new Set(),
        incoming: new Set(),
        weight: componentIds.reduce((total, id) => total + (outgoing.get(id)?.length || 0) + (incomingCount.get(id) || 0), 0),
        entryRank: Math.min(...componentIds.map(entrypointRankForId)),
        firstFile: componentIds.map(labelForId).sort()[0] || ""
      }));

      const dagEdges = new Set();
      for (const id of ids) {
        const caller = componentById.get(id);
        for (const target of outgoing.get(id) || []) {
          const callee = componentById.get(target);
          if (caller === undefined || callee === undefined || caller === callee) {
            continue;
          }
          const key = `${caller}->${callee}`;
          if (dagEdges.has(key)) {
            continue;
          }
          dagEdges.add(key);
          componentData[caller].outgoing.add(callee);
          componentData[callee].incoming.add(caller);
        }
      }

      const preferredComponents = new Set(
        preferredIds
          .map((id) => componentById.get(id))
          .filter((id) => id !== undefined)
      );
      const componentDepth = componentFlowDepths(componentData, preferredComponents);
      const rawMaxDepth = Math.max(0, ...componentDepth.values());
      const depthById = new Map();
      for (const id of ids) {
        const component = componentById.get(id);
        const depth = componentDepth.get(component) || 0;
        depthById.set(id, rawMaxDepth < maxColumns ? depth : Math.round((depth / rawMaxDepth) * (maxColumns - 1)));
      }

      return depthById;
    }

    function compareFileLayouts(left, right) {
      return left.entryRank - right.entryRank
        || Number(isEntrypointFile(right.file)) - Number(isEntrypointFile(left.file))
        || right.weight - left.weight
        || right.nodes.length - left.nodes.length
        || left.file.localeCompare(right.file);
    }

    function compareFunctions(left, right) {
      return entrypointRankForFunction(left) - entrypointRankForFunction(right)
        || left.file.localeCompare(right.file)
        || left.line - right.line
        || left.name.localeCompare(right.name)
        || left.id.localeCompare(right.id);
    }

    function entrypointRankForFunction(item) {
      const name = String(item?.name || "").toLowerCase();
      if (name === "main") {
        return 0;
      }
      if (isEntrypointFile(item?.file || "") && /^run(_|$)/.test(name)) {
        return 1;
      }
      return 1000000;
    }

    function entrypointRankForId(id) {
      const item = state.functions.get(id);
      if (item) {
        return entrypointRankForFunction(item);
      }
      return isEntrypointFile(id) ? 0 : 1000000;
    }

    function isEntrypointFile(file) {
      const value = String(file || "");
      return /(^|\/)main\.(rs|go)$/.test(value) || /(^|\/)(cmd|bin)\//.test(value);
    }

    function stronglyConnectedComponents(nodeIds, outgoing) {
      let nextIndex = 0;
      const stack = [];
      const onStack = new Set();
      const indexById = new Map();
      const lowLink = new Map();
      const components = [];

      function visit(id) {
        indexById.set(id, nextIndex);
        lowLink.set(id, nextIndex);
        nextIndex += 1;
        stack.push(id);
        onStack.add(id);

        for (const target of outgoing.get(id) || []) {
          if (!indexById.has(target)) {
            visit(target);
            lowLink.set(id, Math.min(lowLink.get(id), lowLink.get(target)));
          } else if (onStack.has(target)) {
            lowLink.set(id, Math.min(lowLink.get(id), indexById.get(target)));
          }
        }

        if (lowLink.get(id) !== indexById.get(id)) {
          return;
        }

        const component = [];
        while (stack.length) {
          const current = stack.pop();
          onStack.delete(current);
          component.push(current);
          if (current === id) {
            break;
          }
        }
        components.push(component);
      }

      for (const id of nodeIds) {
        if (!indexById.has(id)) {
          visit(id);
        }
      }

      return components;
    }

    function componentFlowDepths(components, preferredComponents = new Set()) {
      const depth = new Map(components.map((component) => [component.id, Number.POSITIVE_INFINITY]));

      function maxFiniteDepth() {
        return Math.max(0, ...[...depth.values()].filter(Number.isFinite));
      }

      function seedDepth(roots, baseDepth, updateReached) {
        const queue = [];
        for (const root of roots) {
          if (!root) {
            continue;
          }
          if (updateReached || !Number.isFinite(depth.get(root.id))) {
            depth.set(root.id, baseDepth);
            queue.push(root);
          }
        }

        for (let cursor = 0; cursor < queue.length; cursor += 1) {
          const component = queue[cursor];
          const nextDepth = (depth.get(component.id) || 0) + 1;
          const targets = [...component.outgoing].map((id) => components[id]).sort(compareComponents);
          for (const target of targets) {
            if (!target) {
              continue;
            }
            const current = depth.get(target.id);
            if (updateReached) {
              if (!Number.isFinite(current) || nextDepth > current) {
                depth.set(target.id, nextDepth);
                queue.push(target);
              }
            } else if (!Number.isFinite(current)) {
              depth.set(target.id, nextDepth);
              queue.push(target);
            }
          }
        }
      }

      const preferredRoots = components
        .filter((component) => preferredComponents.has(component.id))
        .sort(compareComponents);
      if (preferredRoots.length) {
        seedDepth(preferredRoots, 0, true);
      }

      const secondaryBase = preferredRoots.length ? maxFiniteDepth() + 1 : 0;
      const secondaryRoots = components
        .filter((component) => !Number.isFinite(depth.get(component.id)) && component.incoming.size === 0)
        .sort(compareComponents);
      seedDepth(secondaryRoots, secondaryBase, false);

      const tailBase = maxFiniteDepth() + 1;
      const remaining = components
        .filter((component) => !Number.isFinite(depth.get(component.id)))
        .sort(compareComponents);
      seedDepth(remaining, tailBase, false);

      for (const component of components) {
        if (!Number.isFinite(depth.get(component.id))) {
          depth.set(component.id, 0);
        }
      }

      return depth;
    }

    function compareComponents(left, right) {
      return left.entryRank - right.entryRank
        || right.weight - left.weight
        || left.firstFile.localeCompare(right.firstFile)
        || left.id - right.id;
    }

    function orderLayersByNeighborhood(layerEntries, calls) {
      const layerByNode = new Map();
      layerEntries.forEach(([layerIndex, layer]) => {
        layer.forEach((node) => layerByNode.set(node.id, layerIndex));
      });

      const previousNeighbors = new Map();
      const nextNeighbors = new Map();
      for (const call of calls) {
        const callerLayer = layerByNode.get(call.caller);
        const calleeLayer = layerByNode.get(call.callee);
        if (callerLayer === undefined || calleeLayer === undefined || callerLayer === calleeLayer) {
          continue;
        }
        if (callerLayer < calleeLayer) {
          appendNeighbor(nextNeighbors, call.caller, call.callee);
          appendNeighbor(previousNeighbors, call.callee, call.caller);
        } else {
          appendNeighbor(previousNeighbors, call.caller, call.callee);
          appendNeighbor(nextNeighbors, call.callee, call.caller);
        }
      }

      const order = new Map();
      layerEntries.forEach(([, layer]) => {
        layer.sort(compareFunctions);
        layer.forEach((node, index) => order.set(node.id, index));
      });

      for (let pass = 0; pass < 2; pass += 1) {
        for (const [, layer] of layerEntries) {
          layer.sort((left, right) =>
            neighborOrder(left.id, previousNeighbors, order)
            - neighborOrder(right.id, previousNeighbors, order)
            || compareFunctions(left, right)
          );
          layer.forEach((node, index) => order.set(node.id, index));
        }

        for (const [, layer] of [...layerEntries].reverse()) {
          layer.sort((left, right) =>
            neighborOrder(left.id, nextNeighbors, order)
            - neighborOrder(right.id, nextNeighbors, order)
            || compareFunctions(left, right)
          );
          layer.forEach((node, index) => order.set(node.id, index));
        }
      }
    }

    function appendNeighbor(map, id, neighbor) {
      if (!map.has(id)) {
        map.set(id, []);
      }
      map.get(id).push(neighbor);
    }

    function neighborOrder(id, neighborsById, order) {
      const neighbors = neighborsById.get(id) || [];
      if (!neighbors.length) {
        return Number.POSITIVE_INFINITY;
      }
      let total = 0;
      for (const neighbor of neighbors) {
        total += order.get(neighbor) || 0;
      }
      return total / neighbors.length;
    }

    function resizeCanvasForDisplay(canvasElement) {
      const width = Math.max(1, canvas.clientWidth);
      const height = Math.max(1, canvas.clientHeight);
      const ratio = window.devicePixelRatio || 1;
      const pixelWidth = Math.floor(width * ratio);
      const pixelHeight = Math.floor(height * ratio);
      if (canvasElement.width !== pixelWidth || canvasElement.height !== pixelHeight) {
        canvasElement.width = pixelWidth;
        canvasElement.height = pixelHeight;
      }
      canvasElement.style.width = `${width}px`;
      canvasElement.style.height = `${height}px`;
      const context = canvasElement.getContext("2d");
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      return { context, width, height };
    }

    function visibleCanvasPoint(point, width, height, padding = 180) {
      const x = point.x * view.scale + view.offsetX;
      const y = point.y * view.scale + view.offsetY;
      return x >= -padding && x <= width + padding && y >= -padding && y <= height + padding;
    }

    function visibleCanvasRect(rect, width, height, padding = 220) {
      const left = rect.x * view.scale + view.offsetX;
      const top = rect.y * view.scale + view.offsetY;
      const right = left + rect.width * view.scale;
      const bottom = top + rect.height * view.scale;
      return right >= -padding && left <= width + padding && bottom >= -padding && top <= height + padding;
    }

    function visibleScreenCurve(curve, width, height, padding = 260) {
      const minX = Math.min(curve.startX, curve.controlX1, curve.controlX2, curve.endX);
      const maxX = Math.max(curve.startX, curve.controlX1, curve.controlX2, curve.endX);
      const minY = Math.min(curve.startY, curve.controlY1, curve.controlY2, curve.endY);
      const maxY = Math.max(curve.startY, curve.controlY1, curve.controlY2, curve.endY);
      return maxX >= -padding && minX <= width + padding && maxY >= -padding && minY <= height + padding;
    }

    function canvasCallCurve(caller, callee) {
      const startX = caller.x * view.scale + view.offsetX;
      const startY = caller.y * view.scale + view.offsetY;
      const endX = callee.x * view.scale + view.offsetX;
      const endY = callee.y * view.scale + view.offsetY;
      const deltaX = endX - startX;
      const direction = deltaX >= 0 ? 1 : -1;
      if (Math.abs(deltaX) < 18 * view.scale) {
        const loop = 78 * view.scale;
        return {
          startX,
          startY,
          controlX1: startX + loop,
          controlY1: startY,
          controlX2: endX + loop,
          controlY2: endY,
          endX,
          endY
        };
      }

      const curve = Math.max(38 * view.scale, Math.abs(deltaX) * 0.42);
      return {
        startX,
        startY,
        controlX1: startX + direction * curve,
        controlY1: startY,
        controlX2: endX - direction * curve,
        controlY2: endY,
        endX,
        endY
      };
    }

    function curvePoint(curve, t) {
      const left = 1 - t;
      return {
        x: left * left * left * curve.startX
          + 3 * left * left * t * curve.controlX1
          + 3 * left * t * t * curve.controlX2
          + t * t * t * curve.endX,
        y: left * left * left * curve.startY
          + 3 * left * left * t * curve.controlY1
          + 3 * left * t * t * curve.controlY2
          + t * t * t * curve.endY
      };
    }

    function curveTangent(curve, t) {
      const left = 1 - t;
      return {
        x: 3 * left * left * (curve.controlX1 - curve.startX)
          + 6 * left * t * (curve.controlX2 - curve.controlX1)
          + 3 * t * t * (curve.endX - curve.controlX2),
        y: 3 * left * left * (curve.controlY1 - curve.startY)
          + 6 * left * t * (curve.controlY2 - curve.controlY1)
          + 3 * t * t * (curve.endY - curve.controlY2)
      };
    }

    function drawCanvasArrowhead(context, curve, color, size = 7) {
      const point = curvePoint(curve, 0.93);
      const tangent = curveTangent(curve, 0.93);
      const angle = Math.atan2(tangent.y, tangent.x);
      const renderScale = state.isolate ? 1 : view.scale;
      const length = Math.max(5, Math.min(13, size * Math.max(0.75, renderScale)));
      const spread = Math.PI / 7;

      context.save();
      context.fillStyle = color;
      context.beginPath();
      context.moveTo(point.x, point.y);
      context.lineTo(point.x - length * Math.cos(angle - spread), point.y - length * Math.sin(angle - spread));
      context.lineTo(point.x - length * Math.cos(angle + spread), point.y - length * Math.sin(angle + spread));
      context.closePath();
      context.fill();
      context.restore();
    }

    function drawCanvasEdgeLabel(context, curve, label, color, alpha = 0.72) {
      const renderScale = state.isolate ? 1 : view.scale;
      if (!label || renderScale < 0.74 || canvasInteractionActive()) {
        return;
      }

      const point = curvePoint(curve, 0.52);
      const tangent = curveTangent(curve, 0.52);
      let angle = Math.atan2(tangent.y, tangent.x);
      if (angle > Math.PI / 2 || angle < -Math.PI / 2) {
        angle += Math.PI;
      }

      context.save();
      context.globalAlpha = alpha;
      context.translate(point.x, point.y);
      context.rotate(angle);
      context.fillStyle = color;
      context.textAlign = "center";
      context.textBaseline = "bottom";
      context.font = `${Math.max(7, 9 * renderScale)}px Helvetica, Arial, sans-serif`;
      context.fillText(label, 0, -3 * renderScale);
      context.restore();
    }

    function canvasEdgeLabel(call) {
      return `${call.file}:${call.line}`;
    }

    function strokeScreenCurve(context, curve) {
      context.beginPath();
      context.moveTo(curve.startX, curve.startY);
      context.bezierCurveTo(curve.controlX1, curve.controlY1, curve.controlX2, curve.controlY2, curve.endX, curve.endY);
      context.stroke();
    }

    function drawCanvasDirectedCurve(context, curve, call, color, labelAlpha = 0.72) {
      strokeScreenCurve(context, curve);
      drawCanvasArrowhead(context, curve, color);
      drawCanvasEdgeLabel(context, curve, canvasEdgeLabel(call), color, labelAlpha);
    }

    function canvasCallColor(kind) {
      if (kind === "method") {
        return "#1d6f8f";
      }
      if (kind === "associated") {
        return "#6d4aa1";
      }
      if (kind === "unknown") {
        return "#6f7888";
      }
      return "#934f12";
    }

    function fileColor(file) {
      const colors = ["#ffffdd", "#d9f7d8", "#d8ecff", "#eadcff", "#ffe5c7"];
      let hash = 0;
      for (const char of String(file)) {
        hash = (hash + char.charCodeAt(0)) % colors.length;
      }
      return colors[hash];
    }

    function groupGraph(mode) {
      const renderer = state.canvasRenderer;
      const cached = renderer?.groupCache?.get(mode);
      if (cached) {
        return cached;
      }

      const groups = new Map();
      for (const item of state.functions.values()) {
        const id = groupKeyForFile(item.file, mode);
        if (!groups.has(id)) {
          groups.set(id, {
            id,
            label: id,
            top: id.split("/")[0] || id,
            files: new Set(),
            functionIds: [],
            incomingCalls: 0,
            outgoingCalls: 0,
            internalCalls: 0
          });
        }
        const group = groups.get(id);
        group.files.add(item.file);
        group.functionIds.push(item.id);
      }

      const edgeMap = new Map();
      for (const call of state.graph.calls) {
        const caller = groupIdForFunction(call.caller, mode);
        const callee = groupIdForFunction(call.callee, mode);
        if (!caller || !callee) {
          continue;
        }
        if (caller === callee) {
          groups.get(caller).internalCalls += 1;
          continue;
        }
        const key = `${caller}->${callee}`;
        if (!edgeMap.has(key)) {
          edgeMap.set(key, {
            caller,
            callee,
            count: 0,
            kind: callKind(call)
          });
        }
        const edge = edgeMap.get(key);
        edge.count += 1;
        groups.get(caller).outgoingCalls += 1;
        groups.get(callee).incomingCalls += 1;
      }

      const groupList = [...groups.values()];
      const positions = groupLayout(groupList);
      const graph = {
        groups,
        edges: [...edgeMap.values()],
        positions,
        nodeWidth: mode === "by-file" ? 250 : 285,
        nodeHeight: mode === "by-file" ? 78 : 88,
        bounds: groupBounds(positions)
      };
      renderer?.groupCache?.set(mode, graph);
      return graph;
    }

    function groupLayout(groups) {
      const topBuckets = new Map();
      for (const group of groups) {
        if (!topBuckets.has(group.top)) {
          topBuckets.set(group.top, []);
        }
        topBuckets.get(group.top).push(group);
      }

      const topEntries = [...topBuckets.entries()]
        .sort((left, right) => right[1].length - left[1].length || left[0].localeCompare(right[0]));
      const positions = new Map();
      const columnWidth = 340;
      const rowHeight = 120;
      topEntries.forEach(([_, bucket], column) => {
        bucket
          .sort((left, right) =>
            (right.outgoingCalls + right.incomingCalls + right.internalCalls)
            - (left.outgoingCalls + left.incomingCalls + left.internalCalls)
            || left.label.localeCompare(right.label)
          )
          .forEach((group, row) => {
            positions.set(group.id, {
              x: 170 + column * columnWidth,
              y: 95 + row * rowHeight
            });
          });
      });
      return positions;
    }

    function groupBounds(positions) {
      let width = 980;
      let height = 680;
      for (const point of positions.values()) {
        width = Math.max(width, point.x + 220);
        height = Math.max(height, point.y + 150);
      }
      return { width, height };
    }

    function groupVisible(group) {
      return group?.functionIds?.some((id) => nodeBaseVisible(id));
    }

    function groupMatchesQuery(group, query) {
      if (!query) {
        return true;
      }
      if (group.label.toLowerCase().includes(query)) {
        return true;
      }
      return group.functionIds.some((id) => nodeMatchesQuery(id, query));
    }

    function drawCanvasFlowGuides(context, width, height, renderer) {
      const columns = renderer.flowColumns || [];
      if (!columns.length) {
        return;
      }

      context.save();
      context.textAlign = "center";
      context.textBaseline = "middle";
      for (const column of columns) {
        const columnWidth = column.width || renderer.nodeWidth;
        const bandLeft = (column.x - columnWidth / 2 - 18) * view.scale + view.offsetX;
        const bandWidth = (columnWidth + 36) * view.scale;
        if (bandLeft > width || bandLeft + bandWidth < 0) {
          continue;
        }

        context.globalAlpha = 1;
        context.fillStyle = column.index % 2 === 0 ? "rgba(255, 255, 255, 0.18)" : "rgba(29, 111, 143, 0.06)";
        context.fillRect(bandLeft, 0, Math.max(1, bandWidth), height);

        if (view.scale >= 0.35) {
          const labelY = 34 * view.scale + view.offsetY;
          if (labelY >= -20 && labelY <= height + 20) {
            context.fillStyle = "rgba(83, 96, 112, 0.88)";
            context.font = `700 ${Math.max(9, 11 * view.scale)}px Helvetica, Arial, sans-serif`;
            context.fillText(`depth ${column.index} (${column.count})`, column.x * view.scale + view.offsetX, labelY);
          }
        }
      }
      context.restore();
    }

    function drawCanvasFileClusters(context, width, height, renderer) {
      const clusters = renderer.fileClusters || [];
      if (!clusters.length) {
        return;
      }

      const isolated = neighborhood();
      const hovered = hoverNeighborhood();
      const trace = activeTraceGraph();
      const hasPath = state.pathNodes.size > 0;
      context.save();
      for (const cluster of clusters) {
        if (isolated && !cluster.functionIds.some((id) => isolated.has(id))) {
          continue;
        }
        if (!cluster.functionIds.some((id) => nodeBaseVisible(id))) {
          continue;
        }
        if (!isolated && trace && !cluster.functionIds.some((id) => trace.nodes.has(id))) {
          continue;
        }
        if (!visibleCanvasRect(cluster, width, height)) {
          continue;
        }

        const collapsed = fileCollapsed(cluster.file);
        const selected = state.selectedNode && cluster.functionIds.includes(state.selectedNode);
        const highlighted = collapsed
          || selected
          || cluster.functionIds.some((id) => nodeInPath(id) || id === state.hoverNode);
        const dimmed = (hasPath && !cluster.functionIds.some((id) => nodeInPath(id)))
          || (hovered && !cluster.functionIds.some((id) => hovered.has(id)));
        const x = cluster.x * view.scale + view.offsetX;
        const y = cluster.y * view.scale + view.offsetY;
        const clusterWidth = cluster.width * view.scale;
        const clusterHeight = cluster.height * view.scale;
        const radius = 4 * view.scale;

        context.globalAlpha = highlighted ? 0.68 : dimmed ? 0.18 : 0.46;
        context.fillStyle = fileColor(cluster.file);
        roundedRect(context, x, y, clusterWidth, clusterHeight, radius);
        context.fill();

        context.globalAlpha = highlighted ? 0.96 : dimmed ? 0.28 : 0.82;
        context.strokeStyle = selected ? "#d12f1f" : collapsed ? "#934f12" : "#333333";
        context.lineWidth = highlighted ? Math.max(1.4, 2.2 * view.scale) : Math.max(0.8, 1.1 * view.scale);
        context.stroke();

        if (view.scale >= 0.26) {
          context.globalAlpha = dimmed ? 0.34 : 0.94;
          context.fillStyle = "#10131a";
          context.textAlign = "center";
          context.textBaseline = "middle";
          context.font = `700 ${Math.max(8, 12 * view.scale)}px Helvetica, Arial, sans-serif`;
          const suffix = collapsed ? " (collapsed)" : "";
          const labelText = `${cluster.label}${suffix}`;
          const label = labelText.length > 42 ? `${labelText.slice(0, 41)}...` : labelText;
          context.fillText(label, x + clusterWidth / 2, y + 16 * view.scale, clusterWidth - 24 * view.scale);
          if (view.scale >= 0.42) {
            context.fillStyle = "#536070";
            context.font = `${Math.max(7, 10 * view.scale)}px Helvetica, Arial, sans-serif`;
            context.fillText(
              `${cluster.functionCount} funcs / ${cluster.internalCalls} internal`,
              x + clusterWidth / 2,
              y + 34 * view.scale,
              clusterWidth - 24 * view.scale
            );
          }
        }
      }
      context.restore();
    }

    function drawCanvasFileEdges(context, width, height, renderer, collapsedOnly = false) {
      const clustersByFile = new Map((renderer.fileClusters || []).map((cluster) => [cluster.file, cluster]));
      const query = filter.value.trim().toLowerCase();
      const hovered = hoverNeighborhood();
      const hasPath = state.pathNodes.size > 0;

      context.save();
      context.lineCap = "round";
      for (const edge of renderer.fileEdges || []) {
        if (collapsedOnly && !fileCollapsed(edge.caller) && !fileCollapsed(edge.callee)) {
          continue;
        }
        const caller = clustersByFile.get(edge.caller);
        const callee = clustersByFile.get(edge.callee);
        if (!caller || !callee) {
          continue;
        }
        if (!caller.functionIds.some((id) => nodeBaseVisible(id)) || !callee.functionIds.some((id) => nodeBaseVisible(id))) {
          continue;
        }
        const dim = query && !caller.label.toLowerCase().includes(query) && !callee.label.toLowerCase().includes(query);
        const startX = (caller.x + caller.width) * view.scale + view.offsetX;
        const startY = (caller.y + caller.height / 2) * view.scale + view.offsetY;
        const endX = callee.x * view.scale + view.offsetX;
        const endY = (callee.y + callee.height / 2) * view.scale + view.offsetY;
        const deltaX = endX - startX;
        const direction = deltaX >= 0 ? 1 : -1;
        const bend = Math.max(44 * view.scale, Math.abs(deltaX) * 0.38);
        const curve = {
          startX,
          startY,
          controlX1: startX + direction * bend,
          controlY1: startY,
          controlX2: endX - direction * bend,
          controlY2: endY,
          endX,
          endY
        };
        if (!visibleScreenCurve(curve, width, height, 360)) {
          continue;
        }

        const highlighted = collapsedOnly || caller.functionIds.some((id) => nodeInPath(id)) || callee.functionIds.some((id) => nodeInPath(id));
        const hoverDim = hovered && !caller.functionIds.some((id) => hovered.has(id)) && !callee.functionIds.some((id) => hovered.has(id));
        context.globalAlpha = highlighted ? 0.58 : dim || hoverDim || (hasPath && !highlighted) ? 0.08 : 0.32;
        context.strokeStyle = canvasCallColor(edge.kind);
        context.lineWidth = Math.max(
          highlighted ? 1.4 : 0.8,
          Math.min(8, Math.log2(edge.count + 1)) * Math.max(highlighted ? 0.7 : 0.45, view.scale)
        );
        context.setLineDash(edge.kind === "method" ? [6, 4] : edge.kind === "unknown" ? [2, 4] : []);
        strokeScreenCurve(context, curve);
        drawCanvasArrowhead(context, curve, canvasCallColor(edge.kind), highlighted ? 8 : 6);
        if (view.scale >= 0.68 && !canvasInteractionActive()) {
          drawCanvasEdgeLabel(context, curve, `${edge.count} calls`, canvasCallColor(edge.kind), highlighted ? 0.82 : 0.52);
        }
      }
      context.restore();
    }

    function drawCanvasIsolatedCalls(context, width, height, renderer) {
      const isolated = neighborhood();
      if (!isolated) {
        return;
      }

      const query = filter.value.trim().toLowerCase();
      context.save();
      context.lineCap = "round";
      for (const call of selectedNeighborhoodCalls()) {
        if (!edgeBaseVisible(call, call.caller, call.callee) || !edgeMatchesQuery(call, call.caller, call.callee, query)) {
          continue;
        }
        const caller = renderer.positions.get(call.caller);
        const callee = renderer.positions.get(call.callee);
        if (!caller || !callee) {
          continue;
        }
        const curve = canvasCallCurve(caller, callee);

        const selected = edgeKey(call) === state.selectedEdge;
        context.globalAlpha = selected ? 1 : 0.82;
        const color = selected ? "#d12f1f" : canvasCallColor(callKind(call));
        context.strokeStyle = color;
        context.lineWidth = selected ? Math.max(1.8, view.scale * 2.8) : Math.max(1.2, view.scale * 1.8);
        context.setLineDash(callKind(call) === "method" ? [6, 4] : callKind(call) === "unknown" ? [2, 4] : []);
        drawCanvasDirectedCurve(context, curve, call, color, selected ? 1 : 0.82);
      }
      context.restore();
    }

    function drawCanvasHighlightedCalls(context, width, height, renderer, traceOnly = false) {
      const query = filter.value.trim().toLowerCase();
      const hovered = hoverNeighborhood();
      const trace = activeTraceGraph();
      let drawn = 0;

      context.save();
      context.lineCap = "round";
      for (const call of state.graph.calls) {
        const key = edgeKey(call);
        const traceEdge = Boolean(trace?.edges.has(key));
        const path = edgeInPath(key);
        const selected = key === state.selectedEdge;
        const endpointSelected = call.caller === state.selectedNode || call.callee === state.selectedNode;
        const hoverEdge = Boolean(hovered && hovered.has(call.caller) && hovered.has(call.callee));
        if (traceOnly && !traceEdge) {
          continue;
        }
        if (!traceOnly && !path && !selected && !endpointSelected && !hoverEdge) {
          continue;
        }
        if (!callDisplayVisible(call)) {
          continue;
        }

        const caller = renderer.positions.get(call.caller);
        const callee = renderer.positions.get(call.callee);
        if (!caller || !callee) {
          continue;
        }
        const curve = canvasCallCurve(caller, callee);
        if (!visibleScreenCurve(curve, width, height, 360)) {
          continue;
        }

        const dim = !edgeMatchesQuery(call, call.caller, call.callee, query);
        const strong = selected || path;
        context.globalAlpha = strong ? 0.94 : dim ? 0.18 : traceEdge ? 0.58 : 0.5;
        const color = strong ? "#d12f1f" : canvasCallColor(callKind(call));
        context.strokeStyle = color;
        context.lineWidth = strong ? Math.max(1.8, view.scale * 3) : Math.max(1.1, view.scale * 1.9);
        context.setLineDash(callKind(call) === "method" ? [6, 4] : callKind(call) === "unknown" ? [2, 4] : []);
        drawCanvasDirectedCurve(context, curve, call, color, strong ? 1 : 0.72);
        drawn += 1;
      }
      context.restore();
      return drawn;
    }

    function selectedNeighborhoodCalls() {
      if (!state.isolate || !state.selectedNode) {
        return [];
      }
      return [
        ...(state.incoming.get(state.selectedNode) || []),
        ...(state.outgoing.get(state.selectedNode) || [])
      ];
    }

    function drawCanvasNodeDots(context, width, height, renderer) {
      const query = filter.value.trim().toLowerCase();
      const isolated = neighborhood();
      const hovered = hoverNeighborhood();
      const dotSize = view.scale < 0.22 ? 2 : 3;

      context.save();
      for (const [id, point] of renderer.positions) {
        if (isolated && !isolated.has(id)) {
          continue;
        }
        const visible = isolated ? nodeBaseVisible(id) : nodeDisplayVisible(id);
        if (!visible || !visibleCanvasPoint(point, width, height, 80)) {
          continue;
        }
        const path = nodeInPath(id);
        const traceRoot = state.preset === "trace" && id === state.traceRoot;
        const hoveredNode = id === state.hoverNode;
        const dim = !nodeMatchesQuery(id, query)
          || (isolated && !isolated.has(id))
          || (hovered && !hovered.has(id))
          || (state.pathNodes.size && !path);
        const x = point.x * view.scale + view.offsetX;
        const y = point.y * view.scale + view.offsetY;
        const selected = id === state.selectedNode;
        const size = selected || path || hoveredNode || traceRoot ? dotSize + 2 : dotSize;

        context.globalAlpha = selected ? 1 : dim ? 0.12 : 0.72;
        context.fillStyle = selected || path ? "#d12f1f" : traceRoot ? "#934f12" : hoveredNode ? "#10131a" : "#1d6f8f";
        context.fillRect(x - size / 2, y - size / 2, size, size);
      }
      context.restore();
    }

    function drawCanvasCallEdge(context, caller, callee) {
      strokeScreenCurve(context, canvasCallCurve(caller, callee));
    }

    function drawCanvasFunctionNodes(context, width, height, renderer, isolated) {
      const query = filter.value.trim().toLowerCase();
      const hovered = hoverNeighborhood();
      for (const [id, point] of renderer.positions) {
        if (isolated && !isolated.has(id)) {
          continue;
        }
        if (!isolated && !visibleCanvasPoint(point, width, height)) {
          continue;
        }
        const visible = isolated ? nodeBaseVisible(id) : nodeDisplayVisible(id);
        if (!visible) {
          continue;
        }

        const item = state.functions.get(id);
        const path = nodeInPath(id);
        const traceRoot = state.preset === "trace" && id === state.traceRoot;
        const hoveredNode = id === state.hoverNode;
        const dim = !nodeMatchesQuery(id, query)
          || (isolated && !isolated.has(id))
          || (hovered && !hovered.has(id))
          || (state.pathNodes.size && !path);
        const x = point.x * view.scale + view.offsetX;
        const y = point.y * view.scale + view.offsetY;
        const nodeWidth = renderer.nodeWidth * view.scale;
        const nodeHeight = renderer.nodeHeight * view.scale;

        context.globalAlpha = dim ? 0.16 : 1;
        context.fillStyle = traceRoot ? "#fff8d4" : "#b9e1ea";
        context.strokeStyle = id === state.selectedNode || path || hoveredNode ? "#d12f1f" : "#111111";
        context.lineWidth = id === state.selectedNode || path || hoveredNode ? Math.max(2, view.scale * 3) : Math.max(1, view.scale * 1.4);
        context.beginPath();
        context.ellipse(x, y, nodeWidth / 2, nodeHeight / 2, 0, 0, Math.PI * 2);
        context.fill();
        context.stroke();

        if (view.scale >= 0.55 && item) {
          context.globalAlpha = dim ? 0.28 : 1;
          context.fillStyle = "#10131a";
          context.font = `${Math.max(9, 12 * view.scale)}px Helvetica, Arial, sans-serif`;
          context.textAlign = "center";
          context.textBaseline = "middle";
          const label = item.name.length > 24 ? `${item.name.slice(0, 23)}...` : item.name;
          context.fillText(label, x, y - (view.scale >= 0.8 ? 7 * view.scale : 0), nodeWidth - 12);
          if (view.scale >= 0.8) {
            context.fillStyle = "#536070";
            context.font = `${Math.max(8, 10 * view.scale)}px Helvetica, Arial, sans-serif`;
            context.fillText(fileLabel(item.file), x, y + 10 * view.scale, nodeWidth - 12);
          }
        }
      }
    }

    function drawCanvasNodesByLod(context, width, height, renderer, isolated = null) {
      if (isolated) {
        drawCanvasFunctionNodes(context, width, height, renderer, isolated);
        return;
      }
      if (canvasInteractionActive() || view.scale < 0.62) {
        drawCanvasNodeDots(context, width, height, renderer);
        return;
      }
      drawCanvasFunctionNodes(context, width, height, renderer, isolated);
    }

    function shouldDrawFullCanvasEdges(query) {
      if (canvasInteractionActive()) {
        return false;
      }
      if (state.preset !== "all" && state.preset !== "fan-in" && state.preset !== "fan-out" && state.preset !== "cycles") {
        return false;
      }
      if (!query && state.graph.calls.length > 6000) {
        return false;
      }
      if (state.graph.calls.length > 6000 && view.scale < 1.2) {
        return false;
      }
      return true;
    }

    function drawCanvasGraph() {
      const renderer = state.canvasRenderer;
      if (!renderer?.canvas) {
        return;
      }

      const { context, width, height } = resizeCanvasForDisplay(renderer.canvas);
      context.clearRect(0, 0, width, height);
      context.fillStyle = "#dde5f4";
      context.fillRect(0, 0, width, height);

      const groupMode = activeGroupMode();
      if (groupMode) {
        drawCanvasGroupGraph(context, width, height, groupGraph(groupMode));
        return;
      }

      const isolated = neighborhood();
      drawCanvasFlowGuides(context, width, height, renderer);
      drawCanvasFileClusters(context, width, height, renderer);

      if (view.scale < 0.38) {
        if (state.isolate && state.selectedNode) {
          drawCanvasIsolatedCalls(context, width, height, renderer);
          drawCanvasFunctionNodes(context, width, height, renderer, isolated);
        } else if (state.preset === "trace") {
          drawCanvasHighlightedCalls(context, width, height, renderer, true);
          drawCanvasNodeDots(context, width, height, renderer);
        } else {
          drawCanvasFileEdges(context, width, height, renderer);
          if (hasFocusedCanvasCalls()) {
            drawCanvasHighlightedCalls(context, width, height, renderer, false);
          }
          drawCanvasNodeDots(context, width, height, renderer);
        }
        context.globalAlpha = 1;
        return;
      }

      const query = filter.value.trim().toLowerCase();
      const hovered = hoverNeighborhood();
      if (isolated) {
        drawCanvasIsolatedCalls(context, width, height, renderer);
        drawCanvasNodesByLod(context, width, height, renderer, isolated);
        context.globalAlpha = 1;
        return;
      }

      if (state.preset === "trace") {
        drawCanvasHighlightedCalls(context, width, height, renderer, true);
        drawCanvasNodesByLod(context, width, height, renderer);
        context.globalAlpha = 1;
        return;
      }

      if (state.collapsedFiles.size) {
        drawCanvasFileEdges(context, width, height, renderer, true);
      }

      if (!shouldDrawFullCanvasEdges(query)) {
        if (!state.collapsedFiles.size) {
          drawCanvasFileEdges(context, width, height, renderer);
        }
        if (hasFocusedCanvasCalls()) {
          drawCanvasHighlightedCalls(context, width, height, renderer, false);
        }
        drawCanvasNodesByLod(context, width, height, renderer);
        context.globalAlpha = 1;
        return;
      }

      let drawnEdges = 0;
      context.lineCap = "round";

      for (const call of state.graph.calls) {
        const key = edgeKey(call);
        const selected = key === state.selectedEdge;
        const path = edgeInPath(key);
        const traceEdge = state.preset === "trace" && traceGraph().edges.has(key);
        const endpointSelected = call.caller === state.selectedNode || call.callee === state.selectedNode;
        const hoverEdge = Boolean(hovered && hovered.has(call.caller) && hovered.has(call.callee));
        const important = selected || path || traceEdge || endpointSelected || hoverEdge;
        if (state.pathEdges.size && !path && !selected) {
          continue;
        }
        if (drawnEdges >= renderer.edgeBudget && !important) {
          continue;
        }

        const caller = renderer.positions.get(call.caller);
        const callee = renderer.positions.get(call.callee);
        if (!caller || !callee) {
          continue;
        }
        const curve = canvasCallCurve(caller, callee);
        if (!visibleScreenCurve(curve, width, height, 320)) {
          continue;
        }
        if (!callDisplayVisible(call)) {
          continue;
        }

        const dim = !edgeMatchesQuery(call, call.caller, call.callee, query)
          || (hovered && !hoverEdge);
        if (dim && view.scale < 0.45 && !important) {
          continue;
        }

        context.globalAlpha = selected || path ? 0.92 : dim ? 0.08 : traceEdge ? 0.48 : important ? 0.44 : 0.28;
        const color = selected || path ? "#d12f1f" : canvasCallColor(callKind(call));
        context.strokeStyle = color;
        context.lineWidth = selected || path ? Math.max(1.8, view.scale * 3) : important ? Math.max(1, view.scale * 1.6) : Math.max(0.65, view.scale * 1.05);
        context.setLineDash(callKind(call) === "method" ? [6, 4] : callKind(call) === "unknown" ? [2, 4] : []);
        drawCanvasDirectedCurve(context, curve, call, color, selected || path || important ? 0.86 : 0.5);
        drawnEdges += 1;
      }
      context.setLineDash([]);
      drawCanvasNodesByLod(context, width, height, renderer);

      context.globalAlpha = 1;
    }

    function drawCanvasGroupGraph(context, width, height, graph) {
      const query = filter.value.trim().toLowerCase();
      const visibleGroups = new Set([...graph.groups.values()].filter(groupVisible).map((group) => group.id));

      context.lineCap = "round";
      for (const edge of graph.edges) {
        if (!visibleGroups.has(edge.caller) || !visibleGroups.has(edge.callee)) {
          continue;
        }
        const caller = graph.positions.get(edge.caller);
        const callee = graph.positions.get(edge.callee);
        if (!caller || !callee) {
          continue;
        }
        if (!visibleCanvasPoint(caller, width, height, 260) && !visibleCanvasPoint(callee, width, height, 260)) {
          continue;
        }
        const callerGroup = graph.groups.get(edge.caller);
        const calleeGroup = graph.groups.get(edge.callee);
        const dim = !groupMatchesQuery(callerGroup, query) && !groupMatchesQuery(calleeGroup, query);
        context.globalAlpha = dim ? 0.1 : 0.5;
        context.strokeStyle = canvasCallColor(edge.kind);
        context.lineWidth = Math.min(8, Math.max(1, Math.log2(edge.count + 1))) * Math.max(0.75, view.scale);
        context.beginPath();
        context.moveTo(caller.x * view.scale + view.offsetX, caller.y * view.scale + view.offsetY);
        const midX = (caller.x + callee.x) / 2 * view.scale + view.offsetX;
        context.bezierCurveTo(
          midX,
          caller.y * view.scale + view.offsetY,
          midX,
          callee.y * view.scale + view.offsetY,
          callee.x * view.scale + view.offsetX,
          callee.y * view.scale + view.offsetY
        );
        context.stroke();

        if (view.scale >= 0.5 && !dim) {
          context.globalAlpha = 0.86;
          context.fillStyle = "#10131a";
          context.font = `${Math.max(9, 11 * view.scale)}px Helvetica, Arial, sans-serif`;
          context.textAlign = "center";
          context.textBaseline = "middle";
          context.fillText(String(edge.count), midX, ((caller.y + callee.y) / 2) * view.scale + view.offsetY - 6);
        }
      }

      for (const [id, point] of graph.positions) {
        const group = graph.groups.get(id);
        if (!groupVisible(group) || !visibleCanvasPoint(point, width, height, 260)) {
          continue;
        }
        const dim = !groupMatchesQuery(group, query);
        const selected = state.selectedGroup === id || (state.selectedNode && group.functionIds.includes(state.selectedNode));
        const x = point.x * view.scale + view.offsetX;
        const y = point.y * view.scale + view.offsetY;
        const nodeWidth = graph.nodeWidth * view.scale;
        const nodeHeight = graph.nodeHeight * view.scale;
        const radius = 14 * view.scale;

        context.globalAlpha = dim ? 0.18 : 1;
        context.fillStyle = fileColor(id);
        context.strokeStyle = selected ? "#d12f1f" : "#111111";
        context.lineWidth = selected ? Math.max(2, 3 * view.scale) : Math.max(1, 1.5 * view.scale);
        roundedRect(context, x - nodeWidth / 2, y - nodeHeight / 2, nodeWidth, nodeHeight, radius);
        context.fill();
        context.stroke();

        if (view.scale >= 0.32) {
          context.globalAlpha = dim ? 0.32 : 1;
          context.fillStyle = "#10131a";
          context.font = `700 ${Math.max(10, 15 * view.scale)}px Helvetica, Arial, sans-serif`;
          context.textAlign = "center";
          context.textBaseline = "middle";
          const label = group.label.length > 34 ? `${group.label.slice(0, 33)}...` : group.label;
          context.fillText(label, x, y - 10 * view.scale, nodeWidth - 18);
          if (view.scale >= 0.48) {
            context.fillStyle = "#536070";
            context.font = `${Math.max(8, 11 * view.scale)}px Helvetica, Arial, sans-serif`;
            context.fillText(`${group.functionIds.length} funcs / ${group.files.size} files`, x, y + 10 * view.scale, nodeWidth - 18);
          }
        }
      }

      context.globalAlpha = 1;
    }

    function roundedRect(context, x, y, width, height, radius) {
      const nextRadius = Math.min(radius, width / 2, height / 2);
      context.beginPath();
      context.moveTo(x + nextRadius, y);
      context.lineTo(x + width - nextRadius, y);
      context.quadraticCurveTo(x + width, y, x + width, y + nextRadius);
      context.lineTo(x + width, y + height - nextRadius);
      context.quadraticCurveTo(x + width, y + height, x + width - nextRadius, y + height);
      context.lineTo(x + nextRadius, y + height);
      context.quadraticCurveTo(x, y + height, x, y + height - nextRadius);
      context.lineTo(x, y + nextRadius);
      context.quadraticCurveTo(x, y, x + nextRadius, y);
      context.closePath();
    }

    function drawCanvasMinimap() {
      const renderer = state.canvasRenderer;
      const minimap = document.querySelector("#minimap");
      const canvasElement = document.querySelector("#minimap-graph canvas");
      if (!renderer || !minimap || !canvasElement) {
        return;
      }

      const ratio = window.devicePixelRatio || 1;
      const width = Math.max(1, minimap.clientWidth);
      const height = Math.max(1, minimap.clientHeight);
      canvasElement.width = Math.floor(width * ratio);
      canvasElement.height = Math.floor(height * ratio);
      canvasElement.style.width = `${width}px`;
      canvasElement.style.height = `${height}px`;
      const context = canvasElement.getContext("2d");
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      context.clearRect(0, 0, width, height);

      const graph = activeGroupMode() ? groupGraph(activeGroupMode()) : renderer;
      const scale = Math.min(width / graph.bounds.width, height / graph.bounds.height);
      const left = (width - graph.bounds.width * scale) / 2;
      const top = (height - graph.bounds.height * scale) / 2;
      if (!activeGroupMode() && renderer.fileClusters?.length && (state.collapsedFiles.size || state.preset === "trace")) {
        const trace = activeTraceGraph();
        context.fillStyle = "rgba(147, 79, 18, 0.18)";
        for (const cluster of renderer.fileClusters) {
          if (!state.activeFiles.has(cluster.file)) {
            continue;
          }
          if (trace && !cluster.functionIds.some((id) => trace.nodes.has(id))) {
            continue;
          }
          context.fillRect(
            left + cluster.x * scale,
            top + cluster.y * scale,
            Math.max(1, cluster.width * scale),
            Math.max(1, cluster.height * scale)
          );
        }
      }
      context.fillStyle = "rgba(29, 111, 143, 0.32)";
      for (const [id, point] of graph.positions) {
        if (activeGroupMode()) {
          if (!groupVisible(graph.groups.get(id))) {
            continue;
          }
        } else if (!nodeDisplayVisible(id)) {
          continue;
        }
        context.fillRect(left + point.x * scale, top + point.y * scale, activeGroupMode() ? 3 : 1.5, activeGroupMode() ? 3 : 1.5);
      }
    }

    function fallbackDraw(data) {
      const nodes = data.functions;
      const calls = data.calls;

      if (!nodes.length) {
        canvas.innerHTML = '<div class="empty">No supported functions found.</div>';
        return;
      }

      if (shouldUseCanvasRenderer(data)) {
        fallbackDrawCanvas(data);
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
        loadPanelVisibility();
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
        assert!(html.contains("toggle-controls"));
        assert!(html.contains("control-panel"));
        assert!(html.contains("coviz.quick.controlsVisible"));
        assert!(html.contains("coviz.quick.inspectorVisible"));
        assert!(html.contains("Graph minimap"));
        assert!(html.contains("Hide isolated"));
        assert!(html.contains("layout-preset"));
        assert!(html.contains("search-next"));
        assert!(html.contains("Open in editor"));
        assert!(html.contains("data-source-mode=\"wide\""));
        assert!(html.contains("stronglyConnectedComponents"));
        assert!(html.contains("drawCanvasFlowGuides"));
        assert!(html.contains("drawCanvasFileClusters"));
        assert!(html.contains("drawCanvasFileEdges"));
        assert!(html.contains("drawCanvasNodeDots"));
        assert!(html.contains("packFileLanes"));
        assert!(html.contains("entrypointRankForFunction"));
        assert!(html.contains("visibleScreenCurve"));
        assert!(html.contains("drawCanvasIsolatedCalls"));
        assert!(html.contains("selectedNeighborhoodCalls"));
        assert!(html.contains("drawCanvasFunctionNodes"));
        assert!(html.contains("<option value=\"all\">All calls</option>"));
        assert!(html.contains("trace-entry"));
        assert!(html.contains("find-path"));
        assert!(html.contains("collapse-clusters"));
        assert!(html.contains("traceGraph"));
        assert!(html.contains("findShortestPath"));
        assert!(html.contains("drawCanvasHighlightedCalls"));
        assert!(html.contains("nodeDisplayVisible"));
        assert!(html.contains("scheduleCanvasDraw"));
    }
}
