/// Agent operating mode — determines tool execution behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMode {
    /// Suggest-only: tool calls require user approval (Y/n prompt).
    Plan,
    /// Autonomous execution (stub in this phase — behaves like Plan).
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

/// State tracking for mode and pending tool approval.
pub struct ModeState {
    pub current: AgentMode,
    pub pending_approval: Option<PendingToolCall>,
}

impl Default for ModeState {
    fn default() -> Self {
        Self {
            current: AgentMode::Plan,
            pending_approval: None,
        }
    }
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
