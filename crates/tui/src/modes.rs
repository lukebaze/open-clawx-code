/// Agent operating mode — determines tool execution behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMode {
    /// Suggest-only: tool calls require user approval (Y/n prompt).
    Plan,
    /// Autonomous TDD execution: Analyze → Red → Green → E2E → Refactor.
    Build,
}

impl AgentMode {
    /// Toggle between Plan and Build.
    #[must_use]
    pub fn toggle(self) -> Self {
        match self {
            Self::Plan => Self::Build,
            Self::Build => Self::Plan,
        }
    }

    /// Short label for the status bar badge.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Plan => "Plan",
            Self::Build => "Build",
        }
    }
}

/// State tracking for mode, pending approval, and TDD progress.
pub struct ModeState {
    pub current: AgentMode,
    pub pending_approval: Option<PendingToolCall>,
    /// Pending impact gate approval (`GitNexus` pre-edit check).
    pub pending_impact: Option<PendingImpactApproval>,
    /// Current TDD phase (Build mode only).
    pub tdd_phase: Option<ocx_orchestrator::TddPhase>,
    /// Orchestrator iteration count / max.
    pub iteration: (u32, u32),
    /// Latest test result summary (e.g., "14/14 ✓" or "12/14 ✗").
    pub test_summary: Option<String>,
}

impl Default for ModeState {
    fn default() -> Self {
        Self {
            current: AgentMode::Plan,
            pending_approval: None,
            pending_impact: None,
            tdd_phase: None,
            iteration: (0, 25),
            test_summary: None,
        }
    }
}

/// A pre-edit impact gate awaiting user approval.
pub struct PendingImpactApproval {
    pub impact: ocx_gitnexus::ImpactResult,
    pub respond: std::sync::mpsc::Sender<bool>,
}

impl ModeState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// A tool call awaiting user approval in Plan mode.
pub struct PendingToolCall {
    pub tool_name: String,
    pub input_summary: String,
    /// Sends `true` to approve, `false` to deny.
    pub respond: std::sync::mpsc::Sender<bool>,
}
