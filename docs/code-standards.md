# Code Standards & Codebase Structure

## Codebase Organization

### Workspace Root
- `Cargo.toml` — Workspace manifest with shared dependencies, lints, and settings
- `.claude/` — Agent configuration and rules
- `.gitnexus/` — GitNexus index (generated; do not commit)
- `plans/` — Implementation plans and phase documentation
- `docs/` — Documentation (this directory)

### Crate Structure
Each crate follows a standard layout:

```
crates/{crate-name}/
├── Cargo.toml           # Package manifest
├── src/
│   ├── lib.rs           # Library root or main module tree
│   ├── main.rs          # Binary entry (if applicable)
│   ├── {module}/
│   │   ├── mod.rs       # Module declaration + exports
│   │   ├── {sub-module}.rs
│   │   └── ...
│   └── ...
├── tests/               # Integration tests
└── examples/            # Example code
```

### Crate Responsibilities

| Crate | Purpose | Max Files |
|-------|---------|-----------|
| `app` | CLI binary; main entry point | 3 |
| `tui` | TUI widgets, layout, event loop | 20+ |
| `orchestrator` | TDD state machine, agent team | 5 |
| `gitnexus` | Code intelligence client | 4 |
| `lsp` | LSP client, diagnostics | 4 |
| `server` | HTTP/WS server support | 3 |
| `providers` | LLM provider clients | 5 |
| `claw-*` | Vendored code (relaxed standards) | — |

## Code Quality Standards

### Unsafe Code
**ABSOLUTE:** No unsafe code anywhere in the workspace.

```rust
// FORBIDDEN workspace-wide
unsafe { /* ... */ }

// Option: Use safe wrappers instead
// - crossterm for terminal I/O
// - tokio for async runtime
// - standard library when possible
```

### File Size Limits
**Max 200 lines per file** (excluding tests and examples). When a file exceeds this:

1. Identify logical separation boundaries (modules, traits, implementations)
2. Extract related functions/types into separate files
3. Update `mod.rs` with module declarations
4. Add cross-module linking in docs

**Example Refactoring:**

```rust
// Before: src/orchestrator.rs (250+ lines)
// After:
// ├── src/orchestrator/mod.rs (exports)
// ├── src/orchestrator/state_machine.rs (state enum + transitions)
// ├── src/orchestrator/test_runner.rs (test execution)
// └── src/orchestrator/event.rs (event types)
```

### Naming Conventions

| Category | Convention | Example |
|----------|-----------|---------|
| **Rust files** | snake_case | `impact_dialog.rs`, `agent_team.rs` |
| **Rust modules** | snake_case | `pub mod orchestrator;` |
| **Rust types** | PascalCase | `struct AgentContext`, `enum TddPhase` |
| **Rust functions/vars** | snake_case | `fn run_tests()`, `let max_agents = 5;` |
| **Rust constants** | SCREAMING_SNAKE_CASE | `const MAX_AGENTS: usize = 5;` |
| **Rust traits** | PascalCase | `trait GitNexusReader` |
| **Widget files** | descriptive snake_case | `diagnostics_tab.rs`, `approval_dialog.rs` |
| **Shell scripts** | kebab-case | `build-and-test.sh`, `run-lsp-server.sh` |

### Module Organization

Use `mod.rs` as the single source of truth for module boundaries:

```rust
// src/orchestrator/mod.rs
pub mod agent_team;
pub mod test_runner;
pub mod state_machine;

pub use agent_team::{AgentContext, AgentMessage, AgentTeam};
pub use state_machine::TddPhase;
pub use test_runner::{TestRunner, TestResult};
```

**Rule:** If a module `foo` has submodules, create:
- `src/foo/mod.rs` — module root with re-exports
- `src/foo/bar.rs` — submodule implementation
- `src/foo/baz.rs` — another submodule

Never use inline module syntax (`mod foo { ... }`) for public modules.

### Documentation Comments

All public items **MUST** have doc comments:

```rust
/// Brief one-liner describing purpose.
///
/// Extended description if behavior is non-obvious.
///
/// # Examples
/// ```
/// let result = my_function(42);
/// assert_eq!(result, 43);
/// ```
///
/// # Panics
/// Panics if input is negative.
pub fn my_function(x: i32) -> i32 {
    x + 1
}
```

**Doc comment rules:**
- First line is a brief summary (under 80 chars)
- Blank line separates from extended description
- Include # Examples for complex functions
- Document panics, errors, and edge cases
- Use markdown formatting

### Error Handling

Prefer explicit error types over `.unwrap()`:

```rust
// BAD
let data = load_config().unwrap(); // panics if missing

// GOOD
let data = load_config()
    .context("Failed to load configuration")?;

// BETTER
let data = load_config()
    .map_err(|e| format!("Config error: {}", e))?;
```

Use `anyhow::Result` for application-level errors, custom `Result` types for library code.

### Testing

**Location:** `tests/` directory for integration tests; `#[cfg(test)] mod tests;` for unit tests in same file.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_context_initialization() {
        let ctx = AgentContext::new("agent-1".to_string());
        assert_eq!(ctx.status, AgentStatus::Idle);
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_operation().await;
        assert!(result.is_ok());
    }
}
```

**Test file naming:** `tests/{module_under_test}.rs` (e.g., `tests/orchestrator.rs`)

### Formatting & Linting

**Before committing, run:**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

**Auto-format on save:** Enable in IDE/editor
- VS Code: `rust-analyzer` with `editor.formatOnSave: true`
- Neovim: rust.vim or rustacean.nvim

### Dependency Management

**Workspace dependencies** (in root `Cargo.toml`):

```toml
[workspace]
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2021"
rust-version = "1.70"
license = "MIT"

[workspace.dependencies]
tokio = { version = "1.40", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
```

**Why:** Ensures all crates use consistent versions; reduces `Cargo.lock` churn.

**In crate Cargo.toml:**

```toml
[dependencies]
tokio.workspace = true
serde.workspace = true
my-local-crate = { path = "../my-local-crate", version = "0.1" }
```

### Visibility Rules

**Public API surface:** Minimal and intentional.

```rust
// lib.rs or mod.rs
pub use orchestrator::{Orchestrator, OrchestratorEvent};
pub use agent_team::AgentTeam;

// Keep internal types private
mod event_handler; // not exported

// Re-export important types
pub use event_handler::EventResult;
```

## Widget Standards (`crates/tui`)

### Widget Trait Implementation

```rust
use ratatui::widgets::StatefulWidget;
use ratatui::prelude::{Rect, Frame};

pub struct MyWidget {
    title: String,
    // ... state
}

pub struct MyWidgetState {
    selected: usize,
    // ... mutable state
}

impl StatefulWidget for MyWidget {
    type State = MyWidgetState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Render logic
    }
}
```

### Color Theme Integration

```rust
// All colors from theme module
use crate::theme::Theme;

fn render_with_theme(theme: &Theme, buf: &mut Buffer) {
    // Use theme.foreground, theme.error, theme.warning, theme.success
}
```

### Widget File Organization

```
crates/tui/src/widgets/
├── mod.rs                       # Widget declarations + re-exports
├── approval_dialog.rs           # One widget per file
├── agent_team_panel.rs
├── diagnostics_tab.rs
└── context_panel.rs
```

## Async/Await Patterns

**Always use `tokio` runtime:**

```rust
#[tokio::main]
async fn main() {
    // Async code here
}

// In library code
pub async fn process_async() -> Result<()> {
    Ok(())
}

// Spawning tasks
tokio::spawn(async { /* ... */ });

// In tests
#[tokio::test]
async fn test_something() { }
```

**Avoid blocking the event loop:**

```rust
// BAD: blocks UI
std::thread::sleep(Duration::from_secs(1));

// GOOD: non-blocking wait
tokio::time::sleep(Duration::from_secs(1)).await;
```

## Performance Considerations

### Clone vs Borrow

```rust
// BAD: unnecessary allocation
pub fn process(s: String) -> String { /* ... */ }

// GOOD: borrow when possible
pub fn process(s: &str) -> String { /* ... */ }

// When cloning is necessary, document why
pub fn queue_message(msg: AgentMessage) {
    // Clone needed because msg_tx takes ownership
    self.message_tx.send(msg).ok();
}
```

### Message Passing

For agent coordination, use message types optimized for passing:

```rust
#[derive(Clone, Debug)]
pub struct AgentMessage {
    from: String,
    to: String,
    content: MessageContent, // Enum, not String
}
```

## Git Commit Standards

**Use conventional commits:**

```
feat: add LSP diagnostics widget
fix: correct agent team message routing
refactor: split orchestrator into modules
test: add impact analysis tests
docs: update LSP integration guide
chore: update dependencies
```

**Guidelines:**
- Lowercase after `feat:`, `fix:`, etc.
- Max 50 chars for subject line
- Detailed body separated by blank line
- Reference issues/PRs: `Closes #123`

## Workspace Configuration

### `Cargo.toml` Lints

```toml
[lints.rust]
unsafe_code = "forbid"
unused_imports = "warn"

[lints.clippy]
pedantic = "warn"
nursery = "warn"
all = "warn"
```

### `.rustfmt.toml`

```toml
edition = "2021"
max_width = 100
hard_tabs = false
tab_spaces = 4
```

## Documentation Generation

Generate docs and serve locally:

```bash
cargo doc --open --no-deps
```

Check for missing docs:

```bash
RUSTDOCFLAGS="-D missing-docs" cargo doc
```

## Type System Standards

### Result Types

Define custom result types at crate level:

```rust
// In crate root
pub type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    GitNexusUnavailable,
    InvalidSymbol(String),
    NetworkError(String),
}
```

### Option vs Result

```rust
// Use Option for "might not exist"
pub fn find_config() -> Option<Config> { }

// Use Result for "operation might fail"
pub fn load_config() -> Result<Config> { }
```

## Module Stability

**Mark private implementation details:**

```rust
/// Public stable API
pub fn public_function() { }

/// Internal; subject to change
#[doc(hidden)]
pub fn _internal_helper() { }
```

## Version Control Practices

**Never commit:**
- `.env` files with secrets
- `.gitnexus/` (generated index)
- `target/` directory
- IDE-specific configs (`.vscode/`, `.idea/`)

**Always commit:**
- `Cargo.lock` (pinned dependencies for binaries)
- Documentation updates
- Test files and fixtures
