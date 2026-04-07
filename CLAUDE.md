# CLAUDE.md

## Project
OCX (Open `ClawX` Code) — Pure Rust single-binary coding terminal. Combines claw-code runtime with ratatui TUI.

## Stack
- **Language:** Rust (2021 edition)
- **TUI:** ratatui 0.29 + crossterm 0.28
- **Async:** tokio (full features)
- **CLI:** clap 4 (derive)

## Workspace Layout
```
crates/
  app/          — binary crate (ocx CLI entry point)
  tui/          — TUI library (layout, widgets, event loop)
  claw-api/     — forked from claw-code api crate
  claw-runtime/ — forked from claw-code runtime crate
  claw-plugins/ — forked from claw-code plugins crate
  claw-telemetry/ — forked from claw-code telemetry crate
```

## Verification
```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Conventions
- `#![forbid(unsafe_code)]` workspace-wide
- Files under 200 lines; split into modules when exceeded
- snake_case for Rust files
- Forked crates (claw-*) have relaxed clippy lints since they are vendor code

## Rules
- `.claude/rules/development-rules.md` — code quality and commit standards
- `.claude/rules/orchestration-protocol.md` — subagent delegation
- `.claude/rules/primary-workflow.md` — plan → implement → test → review → docs
- `.claude/rules/team-coordination-rules.md` — multi-agent file ownership

## Agents (8)
planner, fullstack-developer, tester, code-reviewer, debugger, code-simplifier, researcher, docs-manager
