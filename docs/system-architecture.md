# System Architecture

## High-Level Overview

Open-ClawX Code is a modular Rust system composed of 10 crates organized into 4 architectural layers:

```
┌─────────────────────────────────────────────────────────────┐
│                      CLI Layer (app)                        │
│                    ocx binary entry point                   │
└──────────────────────────┬──────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                      TUI Layer (tui)                        │
│    ratatui widgets, event routing, frame rendering         │
└──┬───────────────────────────┬─────────────────────────┬───┘
   ↓                           ↓                         ↓
┌──────────────┐    ┌──────────────────┐    ┌──────────────────┐
│  Widgets     │    │  Event System    │    │  Theme Manager   │
│  (20+ types) │    │  (dispatch,      │    │  (colors, style) │
│              │    │   routing)       │    │                  │
└──────────────┘    └──────────────────┘    └──────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                 Intelligence Layer                          │
│  ┌────────────────┐  ┌────────────┐  ┌─────────────────┐   │
│  │  Orchestrator  │  │ GitNexus   │  │     LSP         │   │
│  │  (TDD state    │  │ (code      │  │ (language       │   │
│  │   machine,     │  │  impact)   │  │  awareness)     │   │
│  │   agent team)  │  │            │  │                 │   │
│  └────────────────┘  └────────────┘  └─────────────────┘   │
└──┬──────────────────────────┬──────────────────────────┬───┘
   ↓                          ↓                          ↓
┌──────────────────┐  ┌──────────────┐  ┌──────────────────┐
│ State Machine    │  │ Code Index   │  │ Language Server  │
│ (TDD phases,     │  │ Reader Trait │  │ Client           │
│ retry logic,     │  │ (CLI/native) │  │ (stdio JSON-RPC) │
│ 25-iter guard)   │  │              │  │                  │
└──────────────────┘  └──────────────┘  └──────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────────┐
│                  Runtime Layer                              │
│  ┌──────────────┐  ┌──────────┐  ┌────────────────────┐    │
│  │   Claw       │  │Providers │  │   Tool Execution   │    │
│  │  Runtime     │  │(OpenAI,  │  │   (git, file I/O,  │    │
│  │              │  │Claude)   │  │    subprocesses)   │    │
│  └──────────────┘  └──────────┘  └────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## Crate Dependencies

### `crates/app` (CLI Binary)
- **Depends on:** `tui`, `orchestrator`, `claw-runtime`, `providers`
- **Responsibility:** Parse CLI args, initialize TUI, start event loop
- **File count:** 3 max (main.rs, cli.rs, error.rs)
- **Entry point:** `fn main()` sets up tokio runtime and TUI

### `crates/tui` (TUI Library)
- **Depends on:** `ratatui`, `crossterm`, `orchestrator`, `gitnexus`, `lsp`
- **Responsibility:** Widget definitions, event routing, frame rendering
- **Key exports:**
  - `Widget` trait implementations (approval_dialog, agent_team_panel, diagnostics_tab, etc.)
  - `Frame` composition and layout logic
  - Event handlers and signal broadcasting
  - Theme colors (foreground, error, warning, success)
- **File count:** 20+ (one widget per file, plus layout/theme modules)

### `crates/orchestrator` (TDD State Machine + Agent Team)
- **Depends on:** `tokio`, `anyhow`, `serde`, `gitnexus`
- **Responsibility:** TDD phase tracking, multi-agent coordination
- **Key modules:**
  - `state_machine.rs` — 6-phase TDD cycle with retry logic
  - `agent_team.rs` — AgentContext, AgentMessage, message bus
  - `test_runner.rs` — Framework detection, test execution
  - `events.rs` — OrchestratorEvent, TddPhase types
- **File count:** 5 (mod.rs + 4 submodules)
- **Critical:** Enforces 25-iteration guard to prevent infinite loops

### `crates/gitnexus` (Code Intelligence)
- **Depends on:** `tokio`, `serde_json`, `anyhow`
- **Responsibility:** GitNexus CLI client + reader trait abstraction
- **Key modules:**
  - `cli_runner.rs` — Shell-out to `gitnexus` CLI with timeout
  - `reader_trait.rs` — GitNexusReader trait (abstract interface)
  - `native_reader.rs` — Direct `.gitnexus/graph.json` reader
  - `auto_reader.rs` — Smart backend selection
- **File count:** 4
- **Types:** ImpactResult, ContextResult, QueryResult, RiskLevel
- **Graceful degradation:** Falls back to no-op if CLI unavailable

### `crates/lsp` (Language Server Protocol)
- **Depends on:** `tokio`, `serde_json`, `anyhow`
- **Responsibility:** LSP client with stdio JSON-RPC transport
- **Key modules:**
  - `client.rs` — LSP client lifecycle
  - `transport.rs` — stdio JSON-RPC encoding/decoding
  - `diagnostics.rs` — Diagnostic aggregation
  - `auto_detect.rs` — Language server auto-discovery
- **File count:** 4
- **Supported servers:** typescript-language-server, gopls, pylsp, rust-analyzer
- **Graceful degradation:** Continues without LSP if servers unavailable

### `crates/server` (HTTP/WebSocket)
- **Depends on:** `tokio`, `axum` or similar
- **Responsibility:** Optional HTTP/WebSocket server for remote access
- **File count:** 3
- **Use case:** Cloud-based OCX instances

### `crates/providers` (LLM Provider Clients)
- **Depends on:** `tokio`, `reqwest`, `serde`
- **Responsibility:** OpenAI, Claude, etc. client abstraction
- **File count:** 5 (one per provider + shared traits)

### `crates/claw-runtime`, `claw-api`, `claw-plugins`, `claw-telemetry`
- **Status:** Vendored from claw-code; relaxed linting
- **Responsibility:** Tool execution, API routing, plugins, telemetry
- **Integration point:** Runtime-bridge in `app` crate

## Event Flow Architecture

### Event Dispatch Cycle

```
┌─────────────────────────────────────────────────┐
│  TUI Event Loop (app/main.rs)                   │
│  - Crossterm input events                       │
│  - Internal timer events                        │
└────────────────┬────────────────────────────────┘
                 ↓
     ┌───────────────────────────┐
     │  Event Router (tui)       │
     │  Dispatch to handlers     │
     └────────┬──────────────────┘
              ↓
    ┌─────────────────────────┐
    │  Handler Match          │
    │  - Input mode?          │
    │  - Build mode?          │
    │  - Help overlay?        │
    └────────┬────────────────┘
             ↓
    ┌─────────────────────────────────────────┐
    │  Action Dispatchers                     │
    │  - Conversation: send user input        │
    │  - Approval: YES/NO for impact gate     │
    │  - Team: TaskCreate/TaskUpdate          │
    │  - Settings: theme, session, etc.       │
    └────────┬────────────────────────────────┘
             ↓
    ┌─────────────────────────────────────────┐
    │  Backend Updates                        │
    │  - Orchestrator state change            │
    │  - Agent team message dispatch          │
    │  - Tool execution kick-off              │
    │  - Runtime events enqueue               │
    └────────┬────────────────────────────────┘
             ↓
    ┌─────────────────────────────────────────┐
    │  Render Frame                           │
    │  - Redraw affected widgets              │
    │  - Update diagnostics from LSP          │
    │  - Refresh agent team panel             │
    │  - Render to terminal buffer            │
    └─────────────────────────────────────────┘
```

## TDD Orchestration Lifecycle

### Phase Transitions

```
Idle
  ↓ (user: "start build")
Analyzing
  ↓ (examine test failures)
TestWriting (RED)
  ↓ (write failing tests, user: "start green")
Implementing (GREEN)
  ↓ (implement minimal solution)
UnitVerify
  ↓ (run unit test suite)
  ├─→ FAIL: TestRetrying (retry 1-3x)
  │   └─→ FAIL after 3: TestRetryExhausted → user intervention
  └─→ PASS: E2EVerify
      ↓ (run end-to-end tests)
      ├─→ FAIL: TestRetrying
      │   └─→ FAIL after 3: TestRetryExhausted
      └─→ PASS: Refactoring
          ↓ (cleanup, optimization)
          ├─→ FAIL: Revert to GREEN
          └─→ PASS: Done (or loop again)
```

**Safety guards:**
- Max 3 retries per test phase
- Max 25 iterations total (prevents infinite loops)
- Timeout guards on external commands

## GitNexus Integration Architecture

### Reader Trait Abstraction

```
                ┌──────────────────┐
                │ GitNexusReader   │
                │ (trait)          │
                └────────┬─────────┘
                         │
        ┌────────────────┼────────────────┐
        ↓                ↓                ↓
   ┌────────────┐  ┌──────────────┐  ┌──────────────┐
   │CLI Runner  │  │Native Reader │  │Auto Selector │
   │(shell-out) │  │(graph.json)  │  │(smart pick)  │
   │            │  │              │  │              │
   │gitnexus    │  │reads         │  │tries native, │
   │CLI binary  │  │.gitnexus/    │  │falls back to │
   │            │  │graph.json    │  │CLI           │
   │$100ms      │  │$1ms          │  │$1ms init     │
   │timeout     │  │direct file   │  │              │
   └────────────┘  └──────────────┘  └──────────────┘
        │                │                  │
        └────────────────┼──────────────────┘
                         ↓
            ┌────────────────────────┐
            │ Cached ImpactResult    │
            │ ContextResult          │
            │ QueryResult            │
            └────────────────────────┘
```

**Implementations:**
1. **GitNexusCliRunner** — Spawns `gitnexus` CLI as subprocess
2. **NativeGitNexusReader** — Parses `.gitnexus/graph.json` directly (fast)
3. **AutoGitNexusReader** — Tries native first, falls back to CLI

### Impact Analysis Flow

```
[User edits symbol X] 
        ↓
[Impact analysis triggered]
        ↓
┌──────────────────────────────────────┐
│ GitNexusReader::impact(X)            │
├──────────────────────────────────────┤
│ 1. Find all upstream callers         │
│ 2. Calculate blast radius            │
│ 3. Assign risk level:                │
│    - LOW: 0-1 direct callers         │
│    - MEDIUM: 2-5 callers             │
│    - HIGH: 5-10 callers              │
│    - CRITICAL: 10+ callers           │
│ 4. Return ImpactResult               │
└──────────────────────────────────────┘
        ↓
[if risk >= HIGH: show approval dialog]
        ↓
[wait for user: YES/NO]
        ↓
[execute edit or abort]
```

## LSP Integration Architecture

### Lifecycle

```
┌──────────────────────────────┐
│ App startup                  │
└──────────────┬───────────────┘
               ↓
┌──────────────────────────────────────────┐
│ LSP crate: auto_detect()                 │
│ - Inspect file extensions                │
│ - Check PATH for language servers        │
│ - Match: TypeScript → ts-ls              │
│          Go → gopls                      │
│          Python → pylsp                  │
│          Rust → rust-analyzer            │
└──────────────┬───────────────────────────┘
               ↓
┌──────────────────────────────────────────┐
│ LSP crate: spawn_server()                │
│ - Fork language server process           │
│ - Open stdin/stdout pipes                │
│ - Initialize handshake                   │
│ - Send initialize request                │
└──────────────┬───────────────────────────┘
               ↓
┌──────────────────────────────────────────┐
│ TUI: subscription to diagnostics         │
│ - Render errors/warnings on file open    │
│ - Update diagnostics on edits            │
│ - Color by severity (red/yellow/blue)    │
└──────────────────────────────────────────┘
```

### Transport Layer

```
┌─────────────────────────────────────┐
│ Rust code → serde_json              │
├─────────────────────────────────────┤
│ Serialize to JSON-RPC message       │
│ {                                   │
│   "jsonrpc": "2.0",                 │
│   "method": "textDocument/didOpen", │
│   "params": {...}                   │
│ }                                   │
└──────────────┬──────────────────────┘
               ↓
┌──────────────────────────────────────┐
│ Write to language server stdin       │
│ Content-Length: NNN\r\n\r\n{...}    │
└──────────────┬──────────────────────┘
               ↓
┌──────────────────────────────────────┐
│ Language server processes            │
└──────────────┬──────────────────────┘
               ↓
┌──────────────────────────────────────┐
│ Read from language server stdout     │
│ {                                   │
│   "jsonrpc": "2.0",                 │
│   "id": 1,                          │
│   "result": {...diagnostics...}     │
│ }                                   │
└──────────────┬──────────────────────┘
               ↓
┌─────────────────────────────────────┐
│ serde_json → Rust diagnostic types  │
└─────────────────────────────────────┘
```

## Multi-Agent Team Architecture

### Message Bus

```
┌──────────────────────────────────────────────────────────────┐
│ Agent Team (up to 5 concurrent agents)                        │
│                                                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │ Agent 1      │  │ Agent 2      │  │ Agent N      │       │
│  │ ID: agent-1  │  │ ID: agent-2  │  │ ID: agent-n  │       │
│  │ Status: IDLE │  │ Status: WORK │  │ Status: DONE │       │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘       │
│         │                 │                 │                │
│         └─────────────────┼─────────────────┘                │
│                           ↓                                  │
│         ┌─────────────────────────────────┐                 │
│         │ Message Bus (mpsc::channel)     │                 │
│         │ Max capacity: bounded (default)  │                 │
│         ├─────────────────────────────────┤                 │
│         │ Queued messages:                │                 │
│         │ - TaskAssignment {task}         │                 │
│         │ - TaskStatus {progress}         │                 │
│         │ - ResultReady {summary}         │                 │
│         │ - BroadcastMessage {content}    │                 │
│         │ - Error {reason}                │                 │
│         └────────────┬────────────────────┘                 │
│                      ↓                                       │
│         ┌─────────────────────────────────┐                 │
│         │ Coordinator                     │                 │
│         │ - Decompose tasks               │                 │
│         │ - Route to workers              │                 │
│         │ - Aggregate results             │                 │
│         │ - Update TUI panel              │                 │
│         └─────────────────────────────────┘                 │
└──────────────────────────────────────────────────────────────┘
```

### Agent Lifecycle

```
NEW → IDLE
        ↓ (assigned task)
    WORKING {task}
        ↓ (waiting for peer)
    WAITINGFORPEER {peer_id}
        ↓ (peer responds / timeout)
    (WORKING) → DONE {summary}
        
      OR
    
    WORKING → FAILED {error}
```

## Data Flow: User Request to Output

```
1. User types: "implement X feature"
2. Submit via Enter
   ↓
3. [tui] ConversationPanel captures input
4. [app] Router dispatches UserMessage event
   ↓
5. [orchestrator] Phase: Analyzing
6. Run impact analysis via GitNexusReader
   ↓
7. If HIGH/CRITICAL risk:
   [tui] Show ApprovalDialog
   User: YES/NO
   ↓
8. [orchestrator] Phase: TestWriting (RED)
9. Call claw-runtime to write tests
10. Render tool output in ToolBlock widget
    ↓
11. [orchestrator] Phase: Implementing (GREEN)
12. Call claw-runtime to implement
13. Render changes in FilesDiffPanel
    ↓
14. [orchestrator] Phase: UnitVerify
15. Spawn test_runner, execute unit tests
16. Parse output; handle retries (3x max)
    ↓
17. If ALL pass:
    [orchestrator] Phase: E2EVerify → Refactoring → Done
18. If 1+ fail:
    [orchestrator] Show failure in TestRetryExhausted event
    User must investigate/fix
    ↓
19. [tui] Render final state:
    - AgentTeamPanel shows task completion
    - ConversationPanel has full history
    - DiagnosticsTab shows LSP errors/warnings
    - ContextPanel shows GitNexus insights
```

## State Management

### Orchestrator State Blob

```rust
pub struct Orchestrator {
    phase: TddPhase,                    // Current phase
    iteration_count: u8,                // 0-25 safety guard
    current_test_type: Option<TestType>, // Unit vs E2E
    retry_attempt: u8,                  // 0-3 per phase
    test_output: String,                // Raw test output
    failure_context: Option<FailureContext>, // For exhausted retries
    test_framework: TestFramework,      // Auto-detected (Cargo, pytest, etc.)
    events: Vec<OrchestratorEvent>,     // Emitted events
}
```

### Agent Team State Blob

```rust
pub struct AgentTeam {
    agents: Vec<AgentContext>,          // Up to 5 agents
    message_rx: mpsc::Receiver<AgentMessage>,
    coordinator: CoordinatorState,      // Task decomposition state
    completed_tasks: Vec<String>,
    failed_tasks: Vec<(String, String)>, // (task_id, error)
}

pub struct AgentContext {
    id: String,
    name: String,
    model: String,
    status: AgentStatus,
    current_task: Option<String>,
    messages_sent: usize,
}
```

## Performance Considerations

### Latency Targets

| Operation | Target | Actual |
|-----------|--------|--------|
| Impact analysis (CLI) | <100ms | ~150ms (depends on index size) |
| Impact analysis (native) | <1ms | ~1ms (direct file read) |
| LSP diagnostics (batch) | <50ms | ~50ms (type checking) |
| Frame render | <16.67ms | ~5ms (at 60 FPS) |
| Event dispatch | <1ms | <0.1ms |

### Memory Constraints

- **Agent team:** 5 agents × 10MB each = 50MB max
- **Diagnostics buffer:** 10k lines × 100 bytes = 1MB
- **Context panel:** 1MB (cached GitNexus results)
- **Total estimate:** <100MB at full capacity

## Testing Architecture

### Unit Test Structure

```
crates/tui/tests/
├── widget_tests.rs       # Widget rendering
├── event_routing_tests.rs # Event dispatch
└── theme_tests.rs        # Color rendering

crates/orchestrator/tests/
├── state_machine_tests.rs # TDD phase transitions
├── agent_team_tests.rs    # Message passing
└── retry_logic_tests.rs   # 3x retry + 25-iter guard

crates/gitnexus/tests/
├── cli_runner_tests.rs    # Shell-out + timeout
├── native_reader_tests.rs # JSON parsing
└── auto_reader_tests.rs   # Fallback logic

crates/lsp/tests/
├── transport_tests.rs     # JSON-RPC encoding
├── diagnostics_tests.rs   # Batch aggregation
└── auto_detect_tests.rs   # Server detection
```

### Integration Tests

- Full TUI → orchestrator → tool execution flow
- Error recovery (crash, timeout, resource exhaustion)
- Multi-agent coordination under load
- LSP server unavailability handling

## Future Extensibility

### Pluggable Language Servers
Currently hard-coded; future: load from config

### Remote Agent Execution
Orchestrator → HTTP service (via `server` crate)

### Custom Tool Integration
Runtime-bridge extensible for project-specific tools

### UI Themes
Current: 1 hardcoded theme; future: theme files + user config
