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

    #canvas svg .dimmed,
    #canvas svg .edge.dimmed {
      opacity: 0.12;
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
        <input id="filter" type="search" placeholder="Filter function or file" autocomplete="off">
        <button id="reset-view" type="button">Reset view</button>
        <button id="isolate" type="button">Isolate selected</button>
        <a href="/graph.svg">graph.svg</a>
        <a href="/graph.dot">graph.dot</a>
        <a href="/graph.json">graph.json</a>
        <a href="/source.json">source.json</a>
        <span class="hint">Wheel zoom / left-drag pan / click inspect</span>
      </div>
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
    const inspector = document.querySelector("#inspector");
    const splitter = document.querySelector("#splitter");
    const resetButton = document.querySelector("#reset-view");
    const isolateButton = document.querySelector("#isolate");
    const state = {
      graph: { functions: [], calls: [] },
      functions: new Map(),
      sources: new Map(),
      incoming: new Map(),
      outgoing: new Map(),
      selectedNode: null,
      selectedEdge: null,
      isolate: false
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

    function initData(graph, source) {
      state.graph = graph;
      state.functions = new Map(graph.functions.map((item) => [item.id, item]));
      state.sources = new Map((source.functions || []).map((item) => [item.id, item]));
      state.incoming = new Map(graph.functions.map((item) => [item.id, []]));
      state.outgoing = new Map(graph.functions.map((item) => [item.id, []]));

      graph.calls.forEach((call) => {
        state.outgoing.get(call.caller)?.push(call);
        state.incoming.get(call.callee)?.push(call);
      });
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
        return `<li data-node-id="${target}" data-edge-key="${edgeKey(call)}">${escapeHtml(item?.name || target)}<br><span class="muted">${escapeHtml(call.file)}:${call.line}</span></li>`;
      }).join("")}</ul>`;
    }

    function renderSource(id) {
      const source = state.sources.get(id);
      if (!source || !source.lines.length) {
        return '<p class="muted">Source snippet unavailable.</p>';
      }

      return `<pre class="source">${source.lines.map((line) => {
        const number = String(line.number).padStart(4, " ");
        const focus = line.number === source.line ? " focus" : "";
        return `<span class="source-line${focus}">${number}  ${escapeHtml(line.text)}</span>`;
      }).join("")}</pre>`;
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
          ${calls.map((call) => `<li data-node-id="${caller}" data-edge-key="${key}">${escapeHtml(call.file)}:${call.line}</li>`).join("")}
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

    function applyGraphState() {
      const query = filter.value.trim().toLowerCase();
      const isolated = neighborhood();

      document.querySelectorAll("#canvas .node").forEach((node) => {
        const id = node.dataset.id || node.querySelector?.("title")?.textContent;
        const label = functionLabel(id).toLowerCase();
        const dim = (query && !label.includes(query)) || (isolated && !isolated.has(id));
        node.classList.toggle("dimmed", Boolean(dim));
        node.classList.toggle("selected", id === state.selectedNode);
      });

      document.querySelectorAll("#canvas .edge").forEach((edge) => {
        const key = edge.dataset.edge || edge.querySelector?.("title")?.textContent;
        const { caller, callee } = splitEdgeKey(key);
        const label = `${functionLabel(caller)} ${functionLabel(callee)}`.toLowerCase();
        const dim = (query && !label.includes(query))
          || (isolated && !(isolated.has(caller) && isolated.has(callee)));
        edge.classList.toggle("dimmed", Boolean(dim));
        edge.classList.toggle("selected", key === state.selectedEdge);
      });

      isolateButton.classList.toggle("active", state.isolate);
    }

    function wireGraphElements() {
      document.querySelectorAll("#canvas svg .node").forEach((node) => {
        node.dataset.id = node.querySelector("title")?.textContent || "";
      });
      document.querySelectorAll("#canvas svg .edge").forEach((edge) => {
        edge.dataset.edge = edge.querySelector("title")?.textContent || "";
      });
      applyGraphState();
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
    window.addEventListener("resize", () => setInspectorWidth(inspector.getBoundingClientRect().width, false));
    filter.addEventListener("input", applyGraphState);
    resetButton.addEventListener("click", resetView);
    isolateButton.addEventListener("click", () => {
      state.isolate = !state.isolate;
      applyGraphState();
    });
    inspector.addEventListener("click", (event) => {
      const item = event.target.closest("[data-node-id]");
      if (!item) {
        return;
      }
      if (item.dataset.edgeKey) {
        state.selectedEdge = item.dataset.edgeKey;
      }
      selectNode(item.dataset.nodeId);
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
        line.classList.add("edge");
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
    }
}
