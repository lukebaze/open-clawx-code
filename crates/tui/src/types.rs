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
