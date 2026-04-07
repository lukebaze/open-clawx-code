# Open-ClawX Code: Project Overview & PDR

## Executive Summary

**Open-ClawX Code (OCX)** is a pure Rust single-binary coding terminal that merges the claw-code runtime with a modern ratatui TUI. It provides agents with a sophisticated development environment featuring TDD orchestration, GitNexus integration, multi-agent team coordination, and LSP support for language-aware code analysis.

**Current Status:** Phase 08 Complete — Multi-Agent & LSP implementation finished
**Build:** cargo clippy -D warnings: PASS | All tests passing

## Product Vision

Enable AI agents to build, test, and refactor code collaboratively within a unified terminal interface, with real-time diagnostics, impact analysis gates, and seamless agent team coordination.

## Core Components

### 1. TUI Layer (`crates/tui`)
Modern terminal interface using ratatui with crossterm, supporting:
- Conversation panel with message history
- Tool output rendering (files, diffs, JSON)
- Context panels (GitNexus, diagnostics, agent status)
- Approval dialogs for high-risk operations
- Session management with persistence

### 2. Orchestrator (`crates/orchestrator`)
TDD-driven state machine managing:
- 6-phase iteration cycle (Analyze → Red → Green → Unit → E2E → Refactor)
- Test framework auto-detection (rust, go, python, javascript, etc.)
- 3-attempt retry logic with exponential backoff
- 25-iteration safety guard
- Message bus for multi-agent coordination

### 3. GitNexus Integration (`crates/gitnexus`)
Code intelligence layer supporting:
- Symbol impact analysis (upstream callers, blast radius assessment)
- 360-degree context retrieval (callers, callees, execution flows)
- Semantic code queries by concept
- CLI shell-out + native `.gitnexus/` file reader
- Auto-detection of available backends

### 4. LSP Client (`crates/lsp`)
Language Server Protocol support for:
- Auto-detection of language servers (typescript-language-server, gopls, pylsp, etc.)
- Stdio JSON-RPC transport
- Diagnostic aggregation
- Hover information and code completion
- Language-aware code actions

### 5. Multi-Agent Team (`crates/orchestrator::agent_team`)
Coordination framework for:
- Up to 5 concurrent agents per team
- Task assignment via message bus
- Status tracking (Idle, Working, WaitingForPeer, Done, Failed)
- Cross-agent message passing
- Context sharing

### 6. Runtime Integration (`crates/claw-runtime`)
Bridge to claw-code runtime:
- Tool execution context
- Session management
- Provider integration (OpenAI, Claude, etc.)

## Workspace Layout

```
crates/
├── app/                 — CLI binary (ocx entry point)
├── tui/                 — TUI library (layout, widgets, event loop)
├── orchestrator/        — TDD state machine + agent team
├── gitnexus/            — Code intelligence client
├── lsp/                 — LSP client + diagnostics
├── server/              — HTTP/WebSocket server support
├── providers/           — LLM provider clients
├── claw-runtime/        — Runtime fork (tool execution)
├── claw-api/            — API fork
├── claw-plugins/        — Plugins fork
└── claw-telemetry/      — Telemetry fork
```

## Phase 08: Multi-Agent & LSP (COMPLETE)

### Deliverables
- [x] `ocx-lsp` crate with auto-detection and JSON-RPC
- [x] `AgentTeam` framework with message bus
- [x] `GitNexusReader` trait abstraction (CLI vs native backend)
- [x] `NativeGitNexusReader` for direct `.gitnexus/graph.json` access
- [x] `AutoGitNexusReader` for smart backend selection
- [x] `diagnostics_tab` widget for LSP diagnostics display
- [x] `agent_team_panel` widget for team status visualization
- [x] Theme extensions: `error`, `warning`, `success` colors

### Architecture

```
[User Input] → [Event Dispatch]
                    ↓
        [Orchestrator State Machine]
                    ↓
    ┌───────┬───────┴──────┬────────┐
    ↓       ↓              ↓        ↓
 [LSP]  [GitNexus]  [Agent Team]  [Tools]
    ↓       ↓              ↓        ↓
[Widgets] [Panels]  [Message Bus] [Signals]
    ↓       ↓              ↓        ↓
[Render]──→[TUI Frame]←───┘        ↓
                                   ↓
                          [Runtime Execution]
```

## Key Features

### TDD Orchestration
6-phase iteration cycle optimized for agent-driven development:
1. **Analyze** — Understand requirements and test failures
2. **Red** — Write tests that fail
3. **Green** — Implement minimal solution
4. **Unit Verify** — Run unit test suite
5. **E2E Verify** — Run end-to-end tests
6. **Refactor** — Clean up code

### Impact-Gated Edits
Pre-edit impact analysis prevents breaking changes:
- Symbol blast radius calculation
- Upstream caller identification
- Risk level assessment (LOW/MEDIUM/HIGH/CRITICAL)
- User approval dialog before edits

### Multi-Agent Coordination
Team-based development with task assignment:
- Coordinator decomposes large tasks
- Workers execute assigned tasks
- Message passing for coordination
- Status visibility for team monitoring

### Language-Aware Development
LSP integration for real-time insights:
- Per-file diagnostics (errors, warnings)
- Hover information and type hints
- Code completion suggestions
- Language-aware code actions

## Development Standards

- **Language:** Rust 2021 edition
- **Unsafe Code:** Forbidden workspace-wide (`#![forbid(unsafe_code)]`)
- **File Size:** Max 200 lines per file; split when exceeded
- **Testing:** Comprehensive unit + E2E test suites
- **Code Quality:** Zero clippy warnings; 100% formatting compliance
- **Dependencies:** Workspace-managed for consistency

## Quality Metrics

| Metric | Target | Status |
|--------|--------|--------|
| Clippy warnings | 0 | PASS |
| Test coverage | >80% | PASS |
| Build time | <30s | PASS |
| Unsafe code | 0 blocks | PASS |
| Max file LOC | 200 | PASS |

## Integration Points

### Claw-Code Runtime Integration
- Tool execution context
- Session state management
- Provider lifecycle

### External Systems
- **Git** — Clone, checkout, diffs via libgit2
- **LSP Servers** — Language servers via stdio JSON-RPC
- **LLM Providers** — OpenAI, Claude, etc. via claw-runtime

## Next Phases (Planned)

### Phase 09: Fine-grained UI Responsiveness
- Streaming response rendering
- Incremental output panels
- Real-time test progress bars

### Phase 10: Deployment & Containerization
- Binary distribution (homebrew, cargo)
- Container images
- Release automation

## Acceptance Criteria (Phase 08)

- [x] `ocx-lsp` crate compiles and clippy clean
- [x] LSP transport handles stdio JSON-RPC correctly
- [x] Auto-detection selects language servers appropriately
- [x] GitNexusReader trait has 3+ implementations
- [x] Diagnostics tab renders errors/warnings with color
- [x] Agent team panel shows agent statuses in real-time
- [x] Theme colors (error, warning, success) render consistently
- [x] All tests pass; zero warnings

## Unresolved Questions

- LSP server auto-discovery strategy for custom/local servers
- Max LSP diagnostic batch size before pagination
- Agent team message bus capacity limits
