/// A slash command definition
pub struct SlashCommand {
    pub name: &'static str,
    pub description: &'static str,
}

/// All available slash commands
pub fn all_commands() -> Vec<SlashCommand> {
    vec![
        SlashCommand { name: "/model", description: "Switch AI model" },
        SlashCommand { name: "/clear", description: "Clear conversation" },
        SlashCommand { name: "/compact", description: "Compact conversation context" },
        SlashCommand { name: "/session list", description: "List saved sessions" },
        SlashCommand { name: "/session load", description: "Load a saved session" },
        SlashCommand { name: "/session save", description: "Save current session" },
        SlashCommand { name: "/config", description: "Show configuration" },
        SlashCommand { name: "/help", description: "Show keybinding help" },
        SlashCommand { name: "/quit", description: "Quit OCX" },
    ]
}

/// Autocomplete state for slash commands
pub struct SlashCompleter {
    commands: Vec<SlashCommand>,
    filtered_indices: Vec<usize>,
    pub selected: usize,
    pub visible: bool,
}

impl SlashCompleter {
    pub fn new() -> Self {
        Self {
            commands: all_commands(),
            filtered_indices: Vec::new(),
            selected: 0,
            visible: false,
        }
    }

    /// Update filter based on current input (text after `/`)
    pub fn filter(&mut self, query: &str) {
        let query_lower = query.to_lowercase();
        self.filtered_indices = self
            .commands
            .iter()
            .enumerate()
            .filter(|(_, cmd)| {
                cmd.name.to_lowercase().contains(&query_lower)
                    || cmd.description.to_lowercase().contains(&query_lower)
            })
            .map(|(i, _)| i)
            .collect();
        self.selected = 0;
        self.visible = !self.filtered_indices.is_empty();
    }

    pub fn move_up(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    pub fn move_down(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected = (self.selected + 1).min(self.filtered_indices.len() - 1);
        }
    }

    /// Get the selected command name (for insertion into input)
    pub fn selected_name(&self) -> Option<&str> {
        self.filtered_indices
            .get(self.selected)
            .map(|&i| self.commands[i].name)
    }

    /// Get filtered commands for display
    pub fn filtered_commands(&self) -> Vec<(&str, &str, bool)> {
        self.filtered_indices
            .iter()
            .enumerate()
            .map(|(display_idx, &cmd_idx)| {
                let cmd = &self.commands[cmd_idx];
                (cmd.name, cmd.description, display_idx == self.selected)
            })
            .collect()
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.filtered_indices.clear();
        self.selected = 0;
    }
}
