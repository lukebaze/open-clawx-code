pub mod slash_commands;

use std::time::Instant;

/// Application input mode
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    /// Vim-style scroll mode: j/k, gg, G, /, ?
    Normal,
    /// Typing in input bar
    Insert,
    /// Slash command autocomplete is active
    SlashComplete,
    /// Help overlay is showing
    Help,
}

impl InputMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Insert | Self::SlashComplete => "INSERT",
            Self::Help => "HELP",
        }
    }
}

/// Tracks pending multi-key sequences (e.g., `gg`)
pub struct KeySequenceBuffer {
    pending: Option<(char, Instant)>,
}

impl KeySequenceBuffer {
    pub fn new() -> Self {
        Self { pending: None }
    }

    /// Feed a character. Returns the completed sequence if any.
    pub fn feed(&mut self, ch: char) -> KeyAction {
        if let Some((prev, ts)) = self.pending.take() {
            // Check if within timeout (300ms)
            if ts.elapsed().as_millis() < 300 && prev == 'g' && ch == 'g' {
                return KeyAction::ScrollToTop;
            }
        }

        match ch {
            'G' => KeyAction::ScrollToBottom,
            'g' => {
                self.pending = Some(('g', Instant::now()));
                KeyAction::Pending
            }
            _ => KeyAction::None,
        }
    }

    /// Check if pending key has timed out
    pub fn check_timeout(&mut self) {
        if let Some((_, ts)) = &self.pending {
            if ts.elapsed().as_millis() >= 300 {
                self.pending = None;
            }
        }
    }

    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }
}

/// Result of a multi-key sequence
pub enum KeyAction {
    ScrollToTop,
    ScrollToBottom,
    Pending,
    None,
}
