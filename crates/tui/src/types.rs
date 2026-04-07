/// Commands sent from TUI to the runtime bridge
pub enum Command {
    /// User submitted a message
    SendMessage(String),
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
    /// Error during runtime operation
    Error(String),
}

/// Metadata about a completed assistant message
#[derive(Debug, Clone, Default)]
pub struct MessageMeta {
    pub input_tokens: u32,
    pub output_tokens: u32,
}
