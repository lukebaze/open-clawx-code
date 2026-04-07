use crate::types::MessageMeta;

/// Role of a message in the conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
    System,
}

/// A single message in the conversation display
#[derive(Debug, Clone)]
pub struct DisplayMessage {
    pub role: Role,
    pub content: String,
    pub meta: Option<MessageMeta>,
    pub is_error: bool,
}

/// Manages the conversation history and scroll state
pub struct ConversationView {
    messages: Vec<DisplayMessage>,
    scroll_offset: usize,
    /// True while assistant is generating a response
    pub is_streaming: bool,
}

impl ConversationView {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
            is_streaming: false,
        }
    }

    pub fn push_user_message(&mut self, text: String) {
        self.messages.push(DisplayMessage {
            role: Role::User,
            content: text,
            meta: None,
            is_error: false,
        });
        self.scroll_to_bottom();
        self.is_streaming = true;
    }

    /// Append streaming token to the current assistant message
    pub fn push_token(&mut self, token: &str) {
        if let Some(last) = self.messages.last_mut() {
            if last.role == Role::Assistant && last.meta.is_none() {
                // Append to in-progress assistant message
                last.content.push_str(token);
                self.scroll_to_bottom();
                return;
            }
        }
        // Start new assistant message
        self.messages.push(DisplayMessage {
            role: Role::Assistant,
            content: token.to_string(),
            meta: None,
            is_error: false,
        });
        self.scroll_to_bottom();
    }

    /// Mark the current assistant message as complete
    pub fn finish_assistant_message(&mut self, meta: MessageMeta) {
        if let Some(last) = self.messages.last_mut() {
            if last.role == Role::Assistant {
                last.meta = Some(meta);
            }
        }
        self.is_streaming = false;
    }

    /// Push an error message into the conversation
    pub fn push_error(&mut self, text: String) {
        self.messages.push(DisplayMessage {
            role: Role::System,
            content: text,
            meta: None,
            is_error: true,
        });
        self.is_streaming = false;
        self.scroll_to_bottom();
    }

    pub fn messages(&self) -> &[DisplayMessage] {
        &self.messages
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(amount);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }
}
