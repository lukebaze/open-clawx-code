# Primary Workflow

## 1. Plan
Delegate to `planner` agent. Create plan in `./plans/` with phases.

## 2. Implement
Delegate to `fullstack-developer`. Follow phase specs. Run `cargo build` + `cargo clippy` after changes.

## 3. Test
Delegate to `tester`. Run `cargo test --workspace`. Fix failures before proceeding.

## 4. Review
Delegate to `code-reviewer`. Address critical findings.

## 5. Document
Delegate to `docs-manager`. Update `./docs/` if changes warrant.

## 6. Debug
When bugs reported: delegate to `debugger` → fix → `tester` → repeat until green.
