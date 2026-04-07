# Codebase Summary

**Last Updated:** 2026-04-07  
**Total Crates:** 10  
**Phase Status:** Phase 08 Complete (Multi-Agent & LSP)

## Crate Inventory

### Production Crates (7)

#### `crates/app` — CLI Binary
**Purpose:** Entry point for OCX terminal application  
**Key Files:**
- `src/main.rs` — Tokio runtime setup, TUI initialization, event loop
- `src/cli.rs` — Command-line argument parsing (clap derive)
- `src/error.rs` — CLI error types and formatting

**Dependencies:** tui, orchestrator, claw-runtime, providers  
**Exports:** None (binary crate)  
**Features:** CLI arg parsing, help text, version display

---

#### `crates/tui` — TUI Library
**Purpose:** Terminal UI using ratatui and crossterm  
**Key Modules:**
- `src/widgets/` (20+ widgets)
  - `approval_dialog.rs` — Impact risk approval gate
  - `agent_team_panel.rs` — (NEW Phase 08) Multi-agent status display
  - `diagnostics_tab.rs` — (NEW Phase 08) LSP diagnostics renderer
  - `conversation_panel.rs` — Chat history + pending input
  - `context_panel.rs` — GitNexus results + insights
  - `status_bar.rs` — Mode, phase, iteration indicators
  - `tool_block.rs` — Tool output formatting
  - `files_tab.rs`, `git_tab.rs`, `gitnexus_tab.rs` — Content tabs
  - `input_bar.rs`, `autocomplete_dropdown.rs` — Input handling
  - `help_overlay.rs`, `impact_dialog.rs` — Dialogs
  - `session_picker.rs` — Session selection UI
  - `placeholder.rs` — Empty state display
- `src/layout.rs` — Frame composition, panel positioning
- `src/theme.rs` — Colors (foreground, error, warning, success)
- `src/event.rs` — Event types and routing
- `src/lib.rs` — Module declarations

**Dependencies:** ratatui, crossterm, orchestrator, gitnexus, lsp  
**Exports:** Widget types, Frame, EventRouter, Theme  
**Key Types:**
- `struct Frame` — Composable UI layout
- `enum Event` — User input, timer, backend signals
- `struct Theme` — Color palette

**Phase 08 Updates:**
- Added `AgentTeamPanel` for team status
- Added `DiagnosticsTab` for LSP errors/warnings
- Extended `Theme` with error/warning/success colors

---

#### `crates/orchestrator` — TDD State Machine & Agent Team
**Purpose:** Orchestration engine for TDD cycles and multi-agent coordination  
**Key Modules:**
- `src/agent_team.rs` — (NEW Phase 08)
  - `struct AgentContext` — Individual agent with orchestrator context
  - `enum AgentStatus` — Idle, Working, WaitingForPeer, Done, Failed
  - `struct AgentTeam` — Container for up to 5 agents
  - `struct AgentMessage` — Inter-agent message type
  - `enum MessageContent` — TaskAssignment, TaskStatus, ResultReady, etc.
  - `fn spawn_team()` — Initialize team with agents
  - `fn route_message()` — Dispatch to agent by ID or broadcast
- `src/state_machine.rs` (implied)
  - `enum TddPhase` — 6-phase cycle (Analyzing, TestWriting, Implementing, UnitVerify, E2EVerify, Refactoring)
  - `struct Orchestrator` — State blob + event queue
  - State transitions with guards (3-retry limit, 25-iteration guard)
- `src/test_runner.rs` (implied)
  - `fn detect_framework()` — Auto-detect test runner (Cargo, pytest, jest, go test, etc.)
  - `fn run_tests()` — Execute test suite, parse output
  - `enum TestFramework` — Cargo, Pytest, Jest, GoTest, etc.
  - `struct TestResult` — Passed, FailedCount, Error
- `src/events.rs` (implied)
  - `enum OrchestratorEvent` — PhaseChanged, TestRunStarted, TestRunCompleted, TestRetrying, TestRetryExhausted, ImpactGateTriggered, IterationUpdated
  - `enum TddPhase` — Short labels for status bar

**Dependencies:** tokio, anyhow, serde, gitnexus  
**Exports:** AgentTeam, AgentContext, AgentStatus, OrchestratorEvent, TddPhase, TestRunner, TestResult

**Phase 08 Updates:**
- New `agent_team.rs` module with full multi-agent coordination
- Message bus for inter-agent communication
- Agent lifecycle management

---

#### `crates/gitnexus` — Code Intelligence Client
**Purpose:** GitNexus integration for impact analysis and code queries  
**Key Modules:**
- `src/reader_trait.rs` — (NEW Phase 08) Abstract backend interface
  - `trait GitNexusReader` — Unified API for different backends
  - Methods: `impact()`, `context()`, `query()`, `is_available()`
  - Implemented by: GitNexusCliRunner, NativeGitNexusReader, AutoGitNexusReader
- `src/cli_runner.rs`
  - `struct GitNexusClient` — Wraps `gitnexus` CLI subprocess
  - Shell-out to `gitnexus impact`, `gitnexus context`, `gitnexus query`
  - 100ms timeout guard
  - Graceful degradation if CLI unavailable
- `src/native_reader.rs` — (NEW Phase 08)
  - `struct NativeGitNexusReader` — Reads `.gitnexus/graph.json` directly
  - ~1ms latency (file-based)
  - JSON parsing with serde_json
- `src/auto_reader.rs` — (NEW Phase 08)
  - `struct AutoGitNexusReader` — Smart backend selector
  - Tries native first (fast), falls back to CLI
  - Graceful degradation chain
- `src/types.rs` (implied)
  - `struct ImpactResult` — Callers, risk level, action items
  - `enum RiskLevel` — Low, Medium, High, Critical
  - `struct CallerInfo` — Symbol, file, line, depth
  - `struct ContextResult` — Callers, callees, processes
  - `struct QueryResult` — Matching symbols, process grouping

**Dependencies:** tokio, serde_json, anyhow  
**Exports:** GitNexusReader, ImpactResult, RiskLevel, ContextResult, QueryResult

**Phase 08 Updates:**
- New `reader_trait.rs` trait abstraction
- New `native_reader.rs` for fast direct file access
- New `auto_reader.rs` for smart fallback selection

---

#### `crates/lsp` — Language Server Protocol Client
**Purpose:** LSP integration for language-aware diagnostics and code intelligence  
**Key Modules:**
- `src/client.rs` (implied)
  - `struct LspClient` — Manages LSP server lifecycle
  - Methods: `new()`, `open_file()`, `did_change()`, `shutdown()`
  - Handles initialization handshake
- `src/transport.rs` (implied)
  - `struct StdioTransport` — Stdin/stdout JSON-RPC codec
  - Encoding: JSON-RPC 2.0 messages with Content-Length header
  - `fn encode_message()` — Serialize to wire format
  - `fn decode_message()` — Parse from wire format
- `src/diagnostics.rs` (implied)
  - Aggregates diagnostics from language server
  - Batch updates, severity filtering
  - Integration with TUI DiagnosticsTab
- `src/auto_detect.rs` (implied)
  - `fn detect_language_server()` — Inspect file extension → language server
  - Supported: TypeScript (ts-language-server), Go (gopls), Python (pylsp), Rust (rust-analyzer)
  - `fn spawn_server()` — Fork language server process
  - Graceful fallback if server unavailable
- `src/lib.rs` — Module exports

**Dependencies:** tokio, serde_json, anyhow  
**Exports:** LspClient, Diagnostic, DiagnosticSeverity

**Phase 08 Launch:**
- Full crate created with complete LSP transport layer
- Auto-detection for TypeScript, Go, Python, Rust
- Stdio JSON-RPC implementation

---

#### `crates/server` — HTTP/WebSocket Server
**Purpose:** Optional remote access layer (cloud deployment)  
**Key Files:**
- `src/http.rs` — HTTP endpoint handlers
- `src/ws.rs` — WebSocket message forwarding
- `src/lib.rs` — Server lifecycle

**Dependencies:** tokio, axum, serde  
**Exports:** Server, HttpHandler, WsHandler  
**Status:** Stub; ready for future cloud features

---

#### `crates/providers` — LLM Provider Clients
**Purpose:** Abstraction over OpenAI, Claude, and other LLM providers  
**Key Modules:**
- `src/openai.rs` — OpenAI API client
- `src/anthropic.rs` — Claude/Anthropic API client
- `src/traits.rs` — Common provider interface
- `src/lib.rs` — Module exports

**Dependencies:** tokio, reqwest, serde  
**Exports:** ProviderClient, Message, Response  
**Status:** Partially integrated; used by claw-runtime

---

### Vendored Crates (3)

#### `crates/claw-runtime`
**Purpose:** Forked from claw-code runtime; tool execution engine  
**Key Exports:** Tool execution context, session state  
**Status:** Relaxed linting; minimal changes from upstream

#### `crates/claw-api`
**Purpose:** Forked from claw-code API; request/response types  
**Status:** Relaxed linting; vendored for consistency

#### `crates/claw-plugins` & `crates/claw-telemetry`
**Purpose:** Plugin system and telemetry (vendored)  
**Status:** Relaxed linting; passive integration

---

## Key Data Types

### Phase Information

| Type | Location | Purpose |
|------|----------|---------|
| `TddPhase` | orchestrator | Current iteration phase |
| `OrchestratorEvent` | orchestrator | Phase transitions, test results |
| `TestResult` | orchestrator | Passed/failed/error outcome |
| `TestFramework` | orchestrator | Auto-detected test runner |

### Code Intelligence

| Type | Location | Purpose |
|------|----------|---------|
| `ImpactResult` | gitnexus | Callers, risk assessment |
| `RiskLevel` | gitnexus | Low/Medium/High/Critical |
| `ContextResult` | gitnexus | 360-degree symbol context |
| `QueryResult` | gitnexus | Semantic search results |
| `GitNexusReader` | gitnexus | Abstract backend interface |

### Agent Team

| Type | Location | Purpose |
|------|----------|---------|
| `AgentContext` | orchestrator | Individual agent state |
| `AgentStatus` | orchestrator | Idle/Working/Done/Failed |
| `AgentTeam` | orchestrator | Team container (max 5) |
| `AgentMessage` | orchestrator | Inter-agent message |
| `MessageContent` | orchestrator | TaskAssignment, TaskStatus, etc. |

### LSP Integration

| Type | Location | Purpose |
|------|----------|---------|
| `LspClient` | lsp | Manages LSP server lifecycle |
| `Diagnostic` | lsp | File error/warning/info |
| `DiagnosticSeverity` | lsp | Error/Warning/Information/Hint |

### UI & Themes

| Type | Location | Purpose |
|------|----------|---------|
| `Theme` | tui | Color palette (6 fields) |
| `Frame` | tui | Layout composition |
| `Event` | tui | Input/timer/backend events |
| `Widget` | tui | Trait for ratatui components |

---

## Module Dependency Graph

```
app
├── tui
│   ├── ratatui
│   ├── crossterm
│   ├── orchestrator
│   ├── gitnexus
│   └── lsp
├── orchestrator
│   ├── tokio
│   ├── anyhow
│   └── gitnexus
├── claw-runtime
│   ├── providers
│   ├── serde
│   └── tokio
└── providers
    ├── tokio
    ├── reqwest
    └── serde

tui
├── ratatui
├── crossterm
├── orchestrator
├── gitnexus
└── lsp

orchestrator
├── tokio
├── anyhow
├── serde
└── gitnexus

gitnexus
├── tokio
├── serde_json
└── anyhow

lsp
├── tokio
├── serde_json
└── anyhow

server
├── tokio
├── axum
└── serde

providers
├── tokio
├── reqwest
└── serde
```

---

## File Count by Crate

| Crate | Source Files | Tests | Total |
|-------|--------------|-------|-------|
| app | 3 | 0 | 3 |
| tui | 22 | 5 | 27 |
| orchestrator | 5 | 3 | 8 |
| gitnexus | 4 | 2 | 6 |
| lsp | 4 | 2 | 6 |
| server | 3 | 0 | 3 |
| providers | 5 | 1 | 6 |
| claw-runtime | 8 | 1 | 9 |
| claw-api | 4 | 0 | 4 |
| claw-plugins | 6 | 0 | 6 |
| **Total** | **64** | **14** | **78** |

---

## Critical Paths (High-Frequency Code Flows)

### Path 1: User Input → Orchestrator State Change
```
app/main.rs:event_loop()
  → tui/event.rs:dispatch()
  → orchestrator/state_machine.rs:on_event()
  → orchestrator/test_runner.rs:run_tests()
  → orchestrator/events.rs:emit()
  → app/main.rs:render_frame()
```
**Latency:** ~5-10ms per cycle  
**Criticality:** CRITICAL — blocks UI responsiveness

### Path 2: Impact Analysis Gate
```
tui/approval_dialog.rs:render()
  → gitnexus/auto_reader.rs:impact()
  → (tries native, fallback CLI)
  → return ImpactResult {callers, risk}
  → approval_dialog.rs:show_dialog()
```
**Latency:** 1-150ms (native vs CLI)  
**Criticality:** HIGH — user-facing delay

### Path 3: LSP Diagnostics Update
```
lsp/client.rs:did_open()
  → lsp/transport.rs:send_message()
  → language_server stdout
  → lsp/diagnostics.rs:aggregate()
  → tui/widgets/diagnostics_tab.rs:render()
```
**Latency:** 50-200ms (depends on language server)  
**Criticality:** MEDIUM — non-blocking

### Path 4: Multi-Agent Task Assignment
```
tui/agent_team_panel.rs:on_task_create()
  → orchestrator/agent_team.rs:assign_task()
  → message_bus.send(TaskAssignment)
  → agent.on_message()
  → claw-runtime.execute_tool()
  → orchestrator/agent_team.rs:update_status()
```
**Latency:** 0.1-5s (depends on task)  
**Criticality:** HIGH — core agent coordination

---

## External Dependencies

### Direct Dependencies (Top-Level)

| Crate | Version | Purpose | Used By |
|-------|---------|---------|---------|
| tokio | 1.40+ | Async runtime | app, orchestrator, gitnexus, lsp, server, providers |
| ratatui | 0.29 | TUI framework | tui |
| crossterm | 0.28 | Terminal I/O | tui |
| serde | 1.0 | Serialization | orchestrator, gitnexus, lsp, providers |
| serde_json | 1.0 | JSON | gitnexus, lsp, providers |
| anyhow | 1.0 | Error handling | orchestrator, gitnexus, lsp |
| clap | 4.0 | CLI parsing | app |
| reqwest | 0.11 | HTTP client | providers |
| axum | 0.7 | Web framework | server |

### Workspace-Managed
All dependencies pinned in root `Cargo.toml` for consistency.

---

## Build Configuration

### Workspace Settings
```toml
[workspace.lints.rust]
unsafe_code = "forbid"
unused_imports = "warn"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
```

### Testing Commands
```bash
cargo test --workspace              # All tests
cargo test --lib                    # Unit tests only
cargo test --test '*'               # Integration tests only
cargo clippy --all-targets -- -D warnings  # Linting
cargo fmt --check                   # Format check
```

---

## Documentation Structure

| File | Purpose | Size |
|------|---------|------|
| `docs/project-overview-pdr.md` | Executive summary + PDR | ~400 LOC |
| `docs/code-standards.md` | Code conventions, file organization | ~500 LOC |
| `docs/system-architecture.md` | Data flows, crate interactions | ~700 LOC |
| `docs/codebase-summary.md` | This file | ~600 LOC |
| `CLAUDE.md` | Project-specific AI guidelines | ~100 LOC |

---

## Lifecycle & Maturity

| Component | Phase | Status | Risks |
|-----------|-------|--------|-------|
| TUI Core | 3 | Production | Rendering latency at high message volume |
| Orchestrator (TDD) | 6 | Production | 25-iteration guard may be too restrictive |
| Orchestrator (Agent Team) | 8 | NEW | Message bus capacity limits untested |
| GitNexus Integration | 6 | Production | CLI timeout (100ms) may be too aggressive |
| GitNexus Reader Trait | 8 | NEW | Native reader JSON parsing needs stress test |
| LSP Client | 8 | NEW | Language server auto-detection needs hardening |
| Runtime Bridge | 6 | Production | Tool execution context isolation unclear |

---

## Known Limitations

1. **LSP Server Auto-Discovery** — Currently hard-coded for 4 languages; custom servers require manual config
2. **Agent Team Scaling** — Limited to 5 concurrent agents per instance; clustering not supported
3. **GitNexus Index Freshness** — Native reader reads stale graph if index not updated
4. **Diagnostics Pagination** — Large diagnostic batches not paginated; UI may lag
5. **Message Bus Capacity** — Unbounded queue; potential memory leak under sustained high-message load

---

## Phase 08 Summary

**Implemented:**
- Multi-agent team framework with message bus
- GitNexus reader trait abstraction + native implementation
- LSP client with stdio JSON-RPC transport
- Diagnostics tab widget
- Agent team status panel widget
- Theme color extensions

**Testing Status:** All unit tests pass; E2E agent team tests in progress

**Acceptance Criteria:** 8/8 met (see project-overview-pdr.md)

**Next Phase:** Fine-grained UI responsiveness (streaming outputs, real-time progress bars)
