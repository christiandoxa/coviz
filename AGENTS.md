# Agent Instructions

- Write all artifacts, comments, documentation, commit messages, and workflow
  names in English.
- This is a Rust 2024 project. Preserve the 2024 edition and use stable Rust
  unless a task explicitly requires otherwise.
- Use RTK for shell commands when it is available. Check with `which rtk`; if
  unavailable, run commands normally and note that RTK was unavailable.
- Protect user and other-agent changes. Inspect `git status --short` before
  editing, never revert unrelated changes, and only modify files required by the
  task.
- Prefer `rg` for searching files or text.
- Before handing off Rust changes, run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

- If a verification command cannot run or fails because of pre-existing work,
  report the exact command and failure summary.
