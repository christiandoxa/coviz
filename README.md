# coviz

`coviz` is a Rust 2024 command-line tool for visualizing Go and Rust source code
as a call graph. It is inspired by `go-callvis`, but targets a smaller MVP:
parse local source files, build a simple function/call relationship graph, and
emit machine-readable output that other tools can render or inspect.

## MVP scope

- Analyze a file or directory of Go or Rust source code.
- Infer language from file extensions by default, or accept an explicit language.
- Emit Graphviz DOT for visualization.
- Emit JSON for downstream tooling.
- Keep output focused on source-level call relationships, not full compiler
  semantic resolution.

## Install

From this repository:

```bash
cargo install --path .
```

From crates.io, after the package is published:

```bash
cargo install coviz
```

## Usage

Analyze the current directory and print DOT to stdout:

```bash
coviz
```

Analyze a Go project and write DOT:

```bash
coviz ./examples/go --language go --format dot --output graph.dot
```

Render DOT with Graphviz:

```bash
dot -Tsvg graph.dot -o graph.svg
```

Analyze a Rust crate and write JSON:

```bash
coviz ./src --language rust --format json --output graph.json
```

Open a quick local browser viewer. The command writes temporary files under
`/tmp/coviz-*`, starts a localhost server, and opens the default browser:

```bash
coviz quick ./src --language rust
```

Quick mode analyzes files in parallel and avoids blocking startup on large
Graphviz layouts. Graphviz `dot` is still used for cleaner clustered SVG output
on small graphs, but `--graphviz auto` skips SVG rendering for large graphs
because `dot` is single-threaded. Use `--graphviz always` to force SVG rendering
in the background, or `--graphviz never` to disable Graphviz artifacts.

Quick mode excludes common test files, Rust `#[cfg(test)]` code, `tests/`, and
`target/` by default. Use `--include-tests` when you need those nodes.

Quick mode also includes a productivity UI for exploring larger graphs:

- Inspector panel with node details, callers, callees, and source context.
- Resizable side panel for balancing the graph view and inspection details.
- Minimap for keeping orientation while zooming or panning through dense graphs.
- File filters and a hide-isolated toggle to reduce visual noise.
- Search navigation for jumping between matching files, symbols, or calls.
- Keyboard shortcuts for common navigation and view actions.
- Layout presets for quickly switching graph organization.
- Open and copy file-link actions for moving from a graph node back to source.

Use automatic language detection and stdout:

```bash
coviz ./path/to/project --language auto --format json --output -
```

## Language support

- Go: `.go` files through `tree-sitter-go`.
- Rust: `.rs` files through `tree-sitter-rust`.
- Auto mode: infers supported languages from file extensions.

Mixed-language repositories are part of the MVP target, but language-specific
analysis is intentionally lightweight. Treat generated graphs as navigation and
inspection aids rather than compiler-accurate call graphs.

## Development

Use the stable Rust toolchain.

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

Run the CLI locally:

```bash
cargo run -- ./src --language rust --format dot
cargo run -- ./src --language rust --format json
```
