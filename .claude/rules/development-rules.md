# Development Rules

## Principles
**YAGNI - KISS - DRY** — always.

## General
- **File Naming:** snake_case for Rust files. Descriptive names.
- **File Size:** Keep code files under 200 lines. Split into modules when exceeded.
- **Unsafe Code:** Forbidden workspace-wide (`#![forbid(unsafe_code)]`).

## Code Quality
- `cargo fmt` before commit
- `cargo clippy --workspace --all-targets -- -D warnings` must pass
- `cargo test --workspace` must pass (pre-existing upstream failures excluded)
- Prioritize readability over cleverness

## Verification Commands
```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Commit Rules
- Conventional commits: `feat:`, `fix:`, `refactor:`, `test:`, `docs:`
- No AI references in commit messages
- No secrets (API keys, tokens, .env files) in commits
