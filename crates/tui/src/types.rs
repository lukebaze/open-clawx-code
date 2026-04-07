use ocx_gitnexus::ImpactResult;
use ocx_orchestrator::{FailureContext, TddPhase, TestResult, TestType};

/// Commands sent from TUI to the runtime bridge
pub enum Command {
    /// User submitted a message
    SendMessage(String),
    /// Force-save the current session
    SaveSession,
    /// Trigger session compaction
    Compact,
    /// Cancel current operation
    Cancel,
    /// Resume Build mode after pause (wired in Phase 06+)
    #[allow(dead_code)]
    ResumeBuild,
    /// Quit the application
    Quit,
}

/// Events sent from the runtime bridge back to the TUI
pub enum Event {
    /// Incremental text token from assistant
    AssistantToken(String),
    /// Assistant turn completed
    AssistantDone(MessageMeta),
    /// Tool execution started
    ToolStart { name: String },
    /// Tool execution finished
    ToolEnd { result: String },
    /// Tool call needs user approval (Plan mode)
    ToolApprovalNeeded {
        tool_name: String,
        input_summary: String,
        /// Send `true` to approve, `false` to deny.
        respond: std::sync::mpsc::Sender<bool>,
    },
    /// Session was saved successfully
    SessionSaved,
    /// Compaction completed
    CompactDone {
        removed_messages: usize,
        summary: String,
    },

    // --- TDD Orchestrator events ---
    /// TDD phase transition
    TddPhaseChanged { phase: TddPhase, detail: String },
    /// Test suite started
    TestRunStarted { test_type: TestType, scope: String },
    /// Test suite completed
    TestRunCompleted {
        test_type: TestType,
        result: TestResult,
    },
    /// Retrying after test failure
    TestRetrying {
        attempt: u8,
        max: u8,
        test_name: String,
    },
    /// All retries exhausted
    TestRetryExhausted {
        phase: TddPhase,
        failure: FailureContext,
    },
    /// Orchestrator iteration count updated
    IterationUpdated { current: u32, max: u32 },
    /// Max iterations reached
    MaxIterationsReached { count: u32 },
    /// Build mode task completed
    BuildDone { summary: String },
    /// Build mode task failed
    BuildFailed {
        message: String,
        #[allow(dead_code)]
        context: Option<FailureContext>,
    },

    // --- GitNexus events ---
    /// Pre-edit impact gate triggered (HIGH/CRITICAL risk)
    ImpactGateTriggered {
        impact: ImpactResult,
        respond: std::sync::mpsc::Sender<bool>,
    },

    /// Error during runtime operation
    Error(String),
}

/// Metadata about a completed assistant message
#[derive(Debug, Clone, Default)]
pub struct MessageMeta {
    pub input_tokens: u32,
    pub output_tokens: u32,
    /// Cumulative tokens across the whole session
    pub total_tokens: u32,
    /// Estimated cumulative cost in USD
    pub total_cost_usd: f64,
}
