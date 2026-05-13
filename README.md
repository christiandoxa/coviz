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

When Graphviz `dot` is installed, quick mode renders `graph.svg` with Graphviz
for a cleaner clustered layout. Without Graphviz, it falls back to the built-in
browser renderer.

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
