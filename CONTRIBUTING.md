# Contributing to purrl

Thanks for your interest in contributing! 

## Getting Started

1. Fork the repo
2. Clone your fork
3. Create a new branch:
```bash
git checkout -b feat/my-feature
```

4. Make your changes
5. Build, test and lint -- the same four commands CI runs:
```bash
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all --check
```

`cargo test` does not need Ollama running. Please keep it that way: tests
that require a live server will not run in CI.

6. Commit and push
7. Open a Pull Request

---

## Guidelines

- Keep changes focused and small
- Prefer clear, readable Rust over clever code
- Avoid adding heavy dependencies without discussion
- Follow existing code style

---

## Good First Issues

Look for issues labeled:
- `good first issue`
- `help wanted`

---

## Code of Conduct

By contributing, you agree to follow our Code of Conduct.
