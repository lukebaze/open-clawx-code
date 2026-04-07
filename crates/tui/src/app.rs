use std::io::{self, Stdout};
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use crossterm::{
    event::{self, Event as CrosstermEvent, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};

use crate::conversation::ConversationView;
use crate::input::slash_commands::SlashCompleter;
use crate::input::{InputMode, KeyAction, KeySequenceBuffer};
use crate::layout::AppLayout;
use crate::modes::{ModeState, PendingImpactApproval, PendingToolCall};
use crate::runtime_bridge::RuntimeBridge;
use crate::session_manager::SessionManager;
use crate::theme::Theme;
use crate::types::{Command, Event};
use crate::widgets::{
    agent_team_panel::AgentTeamPanel,
    approval_dialog::ApprovalDialog,
    autocomplete_dropdown::AutocompleteDropdown,
    context_panel::{ContextPanelWidget, ContextTab},
    conversation_panel::ConversationPanel,
    diagnostics_tab::DiagnosticsTab,
    files_tab::FilesTab,
    git_tab::GitTab,
    gitnexus_tab::GitNexusTab,
    help_overlay::HelpOverlay,
    impact_dialog::ImpactDialog,
    input_bar::{InputBar, InputBuffer},
    model_picker::{ModelEntry, ModelPicker, ModelPickerState},
    session_picker::{SessionPicker, SessionPickerState},
    status_bar::StatusBar,
};

/// Which panel has keyboard focus (within Normal mode)
#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Conversation,
    ContextPanel,
}

struct App {
    mode: InputMode,
    focus: Focus,
    should_quit: bool,
    theme: Theme,
    input: InputBuffer,
    conversation: ConversationView,
    cmd_tx: mpsc::Sender<Command>,
    event_rx: mpsc::Receiver<Event>,
    key_seq: KeySequenceBuffer,
    slash_completer: SlashCompleter,
    // Context panel
    active_tab: ContextTab,
    files_tab: FilesTab,
    git_tab: GitTab,
    right_panel_pct: u16,
    gitnexus_tab: GitNexusTab,
    diagnostics_tab: DiagnosticsTab,
    agent_team_panel: AgentTeamPanel,
    // Phase 05: session & mode state
    mode_state: ModeState,
    model_name: String,
    total_tokens: u32,
    total_cost_usd: f64,
    session_name: String,
    session_picker: Option<SessionPickerState>,
    model_picker: Option<ModelPickerState>,
}

impl App {
    fn new(
        cmd_tx: mpsc::Sender<Command>,
        event_rx: mpsc::Receiver<Event>,
        model_name: String,
        session_name: String,
    ) -> Self {
        let mut git_tab = GitTab::new();
        git_tab.refresh();

        let mut app = Self {
            mode: InputMode::Insert,
            focus: Focus::Conversation,
            should_quit: false,
            theme: Theme::default(),
            input: InputBuffer::new(),
            conversation: ConversationView::new(),
            cmd_tx,
            event_rx,
            key_seq: KeySequenceBuffer::new(),
            slash_completer: SlashCompleter::new(),
            active_tab: ContextTab::Files,
            files_tab: FilesTab::new(),
            git_tab,
            gitnexus_tab: GitNexusTab::new(false), // availability checked later
            diagnostics_tab: DiagnosticsTab::new(),
            agent_team_panel: AgentTeamPanel::new(),
            right_panel_pct: 30,
            mode_state: ModeState::new(),
            model_name,
            total_tokens: 0,
            total_cost_usd: 0.0,
            session_name,
            session_picker: None,
            model_picker: None,
        };

        // Show splash screen on startup
        app.conversation.push_token(crate::splash::SPLASH_ART);
        app.conversation.push_token(crate::splash::WELCOME_MSG);
        app.conversation
            .finish_assistant_message(crate::types::MessageMeta::default());

        app
    }

    #[allow(clippy::too_many_lines)]
    fn render(&self, frame: &mut Frame) {
        let layout = AppLayout::new(frame.area(), self.right_panel_pct);

        let tdd_phase_label = self
            .mode_state
            .tdd_phase
            .map(ocx_orchestrator::TddPhase::label);
        let iteration = if self.mode_state.current == crate::modes::AgentMode::Build {
            Some(self.mode_state.iteration)
        } else {
            None
        };
        frame.render_widget(
            StatusBar::new(
                &self.theme,
                self.mode,
                self.mode_state.current,
                self.key_seq.has_pending(),
                &self.model_name,
                self.total_tokens,
                self.total_cost_usd,
                &self.session_name,
                self.conversation.is_streaming,
                tdd_phase_label,
                iteration,
                self.mode_state.test_summary.as_deref(),
            ),
            layout.status_bar,
        );

        frame.render_widget(
            ConversationPanel::new(
                &self.conversation,
                &self.theme,
                self.mode == InputMode::Normal && self.focus == Focus::Conversation,
            ),
            layout.conversation,
        );

        frame.render_widget(
            ContextPanelWidget {
                active_tab: self.active_tab,
                files_tab: &self.files_tab,
                git_tab: &self.git_tab,
                diagnostics_tab: &self.diagnostics_tab,
                agent_team_panel: &self.agent_team_panel,
                theme: &self.theme,
                focused: self.mode == InputMode::Normal && self.focus == Focus::ContextPanel,
            },
            layout.context_panel,
        );

        frame.render_widget(
            InputBar::new(
                &self.theme,
                self.mode == InputMode::Insert || self.mode == InputMode::SlashComplete,
                self.input.text(),
                self.input.cursor(),
            ),
            layout.input_bar,
        );

        // Overlays (rendered last = on top)
        if self.slash_completer.visible {
            frame.render_widget(
                AutocompleteDropdown {
                    completer: &self.slash_completer,
                    theme: &self.theme,
                },
                layout.input_bar,
            );
        }

        if self.mode == InputMode::Help {
            frame.render_widget(HelpOverlay { theme: &self.theme }, frame.area());
        }

        // Session picker overlay
        if let Some(picker) = &self.session_picker {
            if picker.visible {
                frame.render_widget(
                    SessionPicker {
                        sessions: &picker.sessions,
                        selected: picker.selected,
                        theme: &self.theme,
                    },
                    frame.area(),
                );
            }
        }

        // Approval dialog overlay
        if let Some(pending) = &self.mode_state.pending_approval {
            frame.render_widget(
                ApprovalDialog {
                    tool_name: &pending.tool_name,
                    input_summary: &pending.input_summary,
                    theme: &self.theme,
                },
                frame.area(),
            );
        }

        // Model picker overlay
        if let Some(picker) = &self.model_picker {
            if picker.visible {
                frame.render_widget(
                    ModelPicker {
                        models: &picker.models,
                        selected: picker.selected,
                        theme: &self.theme,
                    },
                    frame.area(),
                );
            }
        }

        // Impact gate dialog overlay
        if let Some(pending) = &self.mode_state.pending_impact {
            frame.render_widget(
                ImpactDialog {
                    impact: &pending.impact,
                    theme: &self.theme,
                },
                frame.area(),
            );
        }
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // Model picker has priority when visible
        if let Some(picker) = &mut self.model_picker {
            if picker.visible {
                self.handle_model_picker_key(code);
                return;
            }
        }

        // Session picker has priority when visible
        if let Some(picker) = &mut self.session_picker {
            if picker.visible {
                self.handle_session_picker_key(code);
                return;
            }
        }

        // Approval dialog has priority when pending
        if self.mode_state.pending_approval.is_some() {
            self.handle_approval_key(code);
            return;
        }

        // Impact gate dialog has priority
        if self.mode_state.pending_impact.is_some() {
            self.handle_impact_key(code);
            return;
        }

        // Help mode: any key exits
        if self.mode == InputMode::Help {
            self.mode = InputMode::Normal;
            return;
        }

        match (code, modifiers) {
            // Global: Ctrl+C
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                if self.conversation.is_streaming {
                    let _ = self.cmd_tx.send(Command::Cancel);
                } else if self.mode == InputMode::Insert {
                    self.input.take();
                    self.slash_completer.close();
                    self.mode = InputMode::Insert;
                } else {
                    self.should_quit = true;
                    let _ = self.cmd_tx.send(Command::Quit);
                }
                return;
            }
            // Global: Ctrl+S — force save session
            (KeyCode::Char('s'), KeyModifiers::CONTROL) => {
                let _ = self.cmd_tx.send(Command::SaveSession);
                self.conversation.push_error("Session saving…".to_string());
                return;
            }
            // Global: Ctrl+M — toggle Plan/Build mode
            (KeyCode::Char('m'), KeyModifiers::CONTROL) => {
                self.mode_state.current = self.mode_state.current.toggle();
                return;
            }
            // Global: panel width
            (KeyCode::Char('['), KeyModifiers::CONTROL) => {
                self.right_panel_pct = self.right_panel_pct.saturating_sub(5).max(20);
                return;
            }
            (KeyCode::Char(']'), KeyModifiers::CONTROL) => {
                self.right_panel_pct = (self.right_panel_pct + 5).min(50);
                return;
            }
            // Global: focus switching
            (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                self.mode = InputMode::Normal;
                self.focus = Focus::Conversation;
                self.slash_completer.close();
                return;
            }
            (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                self.mode = InputMode::Normal;
                self.focus = Focus::ContextPanel;
                self.slash_completer.close();
                return;
            }
            _ => {}
        }

        match self.mode {
            InputMode::Normal => self.handle_normal_key(code, modifiers),
            InputMode::Insert => self.handle_insert_key(code, modifiers),
            InputMode::SlashComplete => self.handle_slash_complete_key(code, modifiers),
            InputMode::Help => {} // handled above
        }
    }

    fn handle_normal_key(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        match code {
            // Mode switching
            KeyCode::Char('i' | 'a') | KeyCode::Esc => {
                self.mode = InputMode::Insert;
            }
            // Quit
            KeyCode::Char('q') => {
                self.should_quit = true;
                let _ = self.cmd_tx.send(Command::Quit);
            }
            // Help
            KeyCode::Char('?') => {
                self.mode = InputMode::Help;
            }
            // Scroll
            KeyCode::Char('j') if self.focus == Focus::Conversation => {
                self.conversation.scroll_down(1);
            }
            KeyCode::Char('k') if self.focus == Focus::Conversation => {
                self.conversation.scroll_up(1);
            }
            // Multi-key sequences
            KeyCode::Char(ch @ ('G' | 'g')) => match self.key_seq.feed(ch) {
                KeyAction::ScrollToTop => self.conversation.scroll_to_top(),
                KeyAction::ScrollToBottom => self.conversation.scroll_to_bottom(),
                KeyAction::Pending | KeyAction::None => {}
            },
            // Tab: cycle focus
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Conversation => Focus::ContextPanel,
                    Focus::ContextPanel => Focus::Conversation,
                };
            }
            // Context tab switching
            KeyCode::Char('1') if self.focus == Focus::ContextPanel => {
                self.active_tab = ContextTab::Files;
            }
            KeyCode::Char('2') if self.focus == Focus::ContextPanel => {
                self.active_tab = ContextTab::Git;
            }
            KeyCode::Char('3') if self.focus == Focus::ContextPanel => {
                self.active_tab = ContextTab::GitNexus;
            }
            KeyCode::Char('4') if self.focus == Focus::ContextPanel => {
                self.active_tab = ContextTab::Diagnostics;
            }
            KeyCode::Char('5') if self.focus == Focus::ContextPanel => {
                self.active_tab = ContextTab::Agents;
            }
            // Any printable char → switch to insert mode
            KeyCode::Char(ch) if !ch.is_control() => {
                self.mode = InputMode::Insert;
                self.input.insert(ch);
            }
            _ => {}
        }
    }

    fn handle_insert_key(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        match code {
            KeyCode::Esc => {
                self.mode = InputMode::Normal;
                self.slash_completer.close();
            }
            KeyCode::Enter => {
                if !self.input.is_empty() && !self.conversation.is_streaming {
                    let text = self.input.take();
                    self.handle_submit(text);
                }
            }
            KeyCode::Char('/') if self.input.is_empty() => {
                self.input.insert('/');
                self.mode = InputMode::SlashComplete;
                self.slash_completer.filter("");
            }
            KeyCode::Char(ch) => self.input.insert(ch),
            KeyCode::Backspace => self.input.backspace(),
            KeyCode::Delete => self.input.delete(),
            KeyCode::Left => self.input.move_left(),
            KeyCode::Right => self.input.move_right(),
            KeyCode::Home => self.input.move_home(),
            KeyCode::End => self.input.move_end(),
            KeyCode::Tab => {
                self.mode = InputMode::Normal;
                self.focus = Focus::Conversation;
            }
            _ => {}
        }
    }

    fn handle_slash_complete_key(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        match code {
            KeyCode::Esc => {
                self.slash_completer.close();
                self.mode = InputMode::Insert;
            }
            KeyCode::Up => self.slash_completer.move_up(),
            KeyCode::Down => self.slash_completer.move_down(),
            KeyCode::Tab | KeyCode::Enter => {
                if let Some(name) = self.slash_completer.selected_name() {
                    let name = name.to_string();
                    self.input.take();
                    for ch in name.chars() {
                        self.input.insert(ch);
                    }
                }
                self.slash_completer.close();
                self.mode = InputMode::Insert;
            }
            KeyCode::Backspace => {
                self.input.backspace();
                let text = self.input.text().to_string();
                if let Some(query) = text.strip_prefix('/') {
                    self.slash_completer.filter(query);
                } else {
                    self.slash_completer.close();
                    self.mode = InputMode::Insert;
                }
            }
            KeyCode::Char(ch) => {
                self.input.insert(ch);
                let text = self.input.text().to_string();
                if let Some(query) = text.strip_prefix('/') {
                    self.slash_completer.filter(query);
                }
            }
            _ => {}
        }
    }

    fn handle_model_picker_key(&mut self, code: KeyCode) {
        let picker = self.model_picker.as_mut().unwrap();
        match code {
            KeyCode::Up | KeyCode::Char('k') => picker.move_up(),
            KeyCode::Down | KeyCode::Char('j') => picker.move_down(),
            KeyCode::Enter => {
                if let Some(id) = picker.selected_model_id() {
                    let id = id.to_string();
                    self.model_name.clone_from(&id);
                    self.conversation
                        .push_error(format!("Model switched to: {id}"));
                    let _ = self.cmd_tx.send(Command::SetModel(id));
                }
                picker.close();
                self.model_picker = None;
            }
            KeyCode::Esc => {
                picker.close();
                self.model_picker = None;
            }
            _ => {}
        }
    }

    fn handle_session_picker_key(&mut self, code: KeyCode) {
        let picker = self.session_picker.as_mut().unwrap();
        match code {
            KeyCode::Up | KeyCode::Char('k') => picker.move_up(),
            KeyCode::Down | KeyCode::Char('j') => picker.move_down(),
            KeyCode::Enter => {
                if let Some(id) = picker.selected_id() {
                    let id = id.to_string();
                    self.conversation
                        .push_error(format!("Loading session: {id}…"));
                    // TODO: actually load session into runtime (requires bridge command)
                }
                picker.close();
                self.session_picker = None;
            }
            KeyCode::Char('n') => {
                picker.close();
                self.session_picker = None;
                self.conversation
                    .push_error("New session created.".to_string());
            }
            KeyCode::Esc => {
                picker.close();
                self.session_picker = None;
            }
            _ => {}
        }
    }

    fn handle_approval_key(&mut self, code: KeyCode) {
        let approved = match code {
            KeyCode::Char('y' | 'Y') | KeyCode::Enter => true,
            KeyCode::Char('n' | 'N') | KeyCode::Esc => false,
            _ => return,
        };

        if let Some(pending) = self.mode_state.pending_approval.take() {
            let _ = pending.respond.send(approved);
            if !approved {
                self.conversation
                    .push_error(format!("Tool '{}' denied.", pending.tool_name));
            }
        }
    }

    fn handle_impact_key(&mut self, code: KeyCode) {
        let approved = match code {
            KeyCode::Char('y' | 'Y') | KeyCode::Enter => true,
            KeyCode::Char('n' | 'N') | KeyCode::Esc => false,
            _ => return,
        };

        if let Some(pending) = self.mode_state.pending_impact.take() {
            let _ = pending.respond.send(approved);
            if approved {
                self.conversation
                    .push_error(format!("Impact approved for '{}'.", pending.impact.symbol));
            } else {
                self.conversation.push_error(format!(
                    "Edit blocked: {} risk on '{}'.",
                    pending.impact.risk_level.label(),
                    pending.impact.symbol
                ));
            }
            // Store impact in the GitNexus tab
            self.gitnexus_tab.push_impact(pending.impact);
        }
    }

    fn handle_submit(&mut self, text: String) {
        // Handle slash commands
        if text.starts_with('/') {
            self.handle_slash_command(&text);
            return;
        }
        // Regular message
        self.conversation.push_user_message(text.clone());
        let _ = self.cmd_tx.send(Command::SendMessage(text));
    }

    fn handle_slash_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.splitn(3, ' ').collect();
        match parts[0] {
            "/clear" => {
                self.conversation = ConversationView::new();
            }
            "/help" => {
                self.mode = InputMode::Help;
            }
            "/quit" => {
                self.should_quit = true;
                let _ = self.cmd_tx.send(Command::Quit);
            }
            "/compact" => {
                let _ = self.cmd_tx.send(Command::Compact);
                self.conversation
                    .push_error("Compacting session…".to_string());
            }
            "/session" => {
                let sub = parts.get(1).copied().unwrap_or("list");
                match sub {
                    "list" => self.show_session_picker(),
                    "new" => {
                        self.conversation
                            .push_error("New session — restart to create.".to_string());
                    }
                    "load" => {
                        if let Some(id) = parts.get(2) {
                            self.conversation
                                .push_error(format!("Loading session: {id}…"));
                        } else {
                            self.show_session_picker();
                        }
                    }
                    _ => {
                        self.conversation
                            .push_error(format!("Unknown /session subcommand: {sub}"));
                    }
                }
            }
            "/model" => {
                if let Some(&model_id) = parts.get(1) {
                    // Direct switch: /model <id>
                    self.model_name = model_id.to_string();
                    self.conversation
                        .push_error(format!("Model switched to: {}", self.model_name));
                    let _ = self.cmd_tx.send(Command::SetModel(self.model_name.clone()));
                } else {
                    // Show model picker
                    self.show_model_picker();
                }
            }
            _ => {
                self.conversation
                    .push_error(format!("Unknown command: {cmd}"));
            }
        }
    }

    fn show_session_picker(&mut self) {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        match SessionManager::new(&cwd) {
            Ok(mgr) => match mgr.list_sessions() {
                Ok(sessions) => {
                    self.session_picker = Some(SessionPickerState::new(sessions));
                }
                Err(e) => {
                    self.conversation
                        .push_error(format!("Failed to list sessions: {e}"));
                }
            },
            Err(e) => {
                self.conversation
                    .push_error(format!("Session manager error: {e}"));
            }
        }
    }

    fn show_model_picker(&mut self) {
        // Build model list from known providers
        let models = vec![
            ModelEntry {
                id: "claude-sonnet-4-20250514".to_string(),
                name: "Claude Sonnet 4".to_string(),
                provider: "anthropic".to_string(),
                context_window: 200_000,
                is_active: self.model_name == "claude-sonnet-4-20250514",
            },
            ModelEntry {
                id: "claude-opus-4-20250514".to_string(),
                name: "Claude Opus 4".to_string(),
                provider: "anthropic".to_string(),
                context_window: 200_000,
                is_active: self.model_name == "claude-opus-4-20250514",
            },
            ModelEntry {
                id: "claude-haiku-4-20250514".to_string(),
                name: "Claude Haiku 4".to_string(),
                provider: "anthropic".to_string(),
                context_window: 200_000,
                is_active: self.model_name == "claude-haiku-4-20250514",
            },
            ModelEntry {
                id: "gpt-4o".to_string(),
                name: "GPT-4o".to_string(),
                provider: "openai".to_string(),
                context_window: 128_000,
                is_active: self.model_name == "gpt-4o",
            },
            ModelEntry {
                id: "gpt-4o-mini".to_string(),
                name: "GPT-4o Mini".to_string(),
                provider: "openai".to_string(),
                context_window: 128_000,
                is_active: self.model_name == "gpt-4o-mini",
            },
            ModelEntry {
                id: "gemini-2.5-pro".to_string(),
                name: "Gemini 2.5 Pro".to_string(),
                provider: "gemini".to_string(),
                context_window: 1_000_000,
                is_active: self.model_name == "gemini-2.5-pro",
            },
            ModelEntry {
                id: "gemini-2.5-flash".to_string(),
                name: "Gemini 2.5 Flash".to_string(),
                provider: "gemini".to_string(),
                context_window: 1_000_000,
                is_active: self.model_name == "gemini-2.5-flash",
            },
            ModelEntry {
                id: "llama-3.3-70b".to_string(),
                name: "Llama 3.3 70B".to_string(),
                provider: "groq".to_string(),
                context_window: 128_000,
                is_active: self.model_name == "llama-3.3-70b",
            },
        ];
        self.model_picker = Some(ModelPickerState::new(models));
    }

    #[allow(clippy::too_many_lines)]
    fn process_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                Event::AssistantToken(token) => {
                    self.conversation.push_token(&token);
                }
                Event::AssistantDone(meta) => {
                    self.total_tokens = meta.total_tokens;
                    self.total_cost_usd = meta.total_cost_usd;
                    self.conversation.finish_assistant_message(meta);
                }
                Event::ToolStart { name } => {
                    self.conversation
                        .push_token(&format!("\n[tool: {name}] running...\n"));
                }
                Event::ToolEnd { result } => {
                    self.conversation
                        .push_token(&format!("[result: {result}]\n"));
                    self.git_tab.mark_dirty();
                    self.git_tab.refresh();
                }
                Event::ToolApprovalNeeded {
                    tool_name,
                    input_summary,
                    respond,
                } => {
                    self.mode_state.pending_approval = Some(PendingToolCall {
                        tool_name,
                        input_summary,
                        respond,
                    });
                }
                Event::SessionSaved => {
                    self.conversation.push_error("Session saved.".to_string());
                }
                Event::CompactDone {
                    removed_messages,
                    summary,
                } => {
                    self.conversation.push_error(format!(
                        "Compact: removed {removed_messages} msgs. {summary}"
                    ));
                }
                // TDD Orchestrator events
                Event::TddPhaseChanged { phase, detail } => {
                    self.mode_state.tdd_phase = Some(phase);
                    let msg = if detail.is_empty() {
                        format!("TDD: {}", phase.label())
                    } else {
                        format!("TDD: {} ({})", phase.label(), detail)
                    };
                    self.conversation.push_error(msg);
                }
                Event::TestRunStarted { test_type, scope } => {
                    self.conversation
                        .push_error(format!("Running {} tests: {scope}…", test_type.label()));
                }
                Event::TestRunCompleted { test_type, result } => {
                    let status = if result.passed { "PASS" } else { "FAIL" };
                    let summary = format!("{}/{}", result.total - result.failed, result.total);
                    self.mode_state.test_summary = Some(summary.clone());
                    self.conversation.push_error(format!(
                        "{} tests: {status} ({summary}) in {}ms",
                        test_type.label(),
                        result.duration_ms
                    ));
                }
                Event::TestRetrying {
                    attempt,
                    max,
                    test_name,
                } => {
                    self.conversation
                        .push_error(format!("Retrying '{test_name}' ({attempt}/{max})…"));
                }
                Event::TestRetryExhausted { phase, failure } => {
                    self.conversation.push_error(format!(
                        "FAILED: {} after {} retries. Pausing Build mode.",
                        phase.label(),
                        failure.attempt_count
                    ));
                    self.mode_state.current = crate::modes::AgentMode::Plan;
                }
                Event::IterationUpdated { current, max } => {
                    self.mode_state.iteration = (current, max);
                }
                Event::MaxIterationsReached { count } => {
                    self.conversation.push_error(format!(
                        "Max iterations ({count}) reached — Build mode paused."
                    ));
                    self.mode_state.current = crate::modes::AgentMode::Plan;
                }
                Event::BuildDone { summary } => {
                    self.mode_state.tdd_phase = None;
                    self.conversation
                        .push_error(format!("Build complete: {summary}"));
                }
                Event::BuildFailed { message, .. } => {
                    self.mode_state.tdd_phase = None;
                    self.mode_state.current = crate::modes::AgentMode::Plan;
                    self.conversation
                        .push_error(format!("Build failed: {message}"));
                }
                // GitNexus events
                Event::ImpactGateTriggered { impact, respond } => {
                    self.gitnexus_tab.push_impact(impact.clone());
                    self.mode_state.pending_impact =
                        Some(PendingImpactApproval { impact, respond });
                }

                // Agent team events
                Event::AgentStatusChanged {
                    agent_id,
                    agent_name,
                    status,
                    current_task,
                    messages_sent,
                } => {
                    use crate::widgets::agent_team_panel::AgentView;
                    let view = AgentView {
                        id: agent_id.clone(),
                        name: agent_name,
                        status_label: status.label().to_string(),
                        current_task,
                        messages_sent,
                    };
                    // Update or add agent in panel
                    if let Some(existing) = self
                        .agent_team_panel
                        .agents
                        .iter_mut()
                        .find(|a| a.id == agent_id)
                    {
                        *existing = view;
                    } else {
                        self.agent_team_panel.agents.push(view);
                    }
                }
                Event::AgentTeamDone { summary } => {
                    self.conversation
                        .push_error(format!("Agent team done: {summary}"));
                }

                // LSP events
                Event::LspDiagnosticsUpdated { diagnostics, .. } => {
                    use crate::widgets::diagnostics_tab::DiagnosticEntry;
                    let entries: Vec<DiagnosticEntry> = diagnostics
                        .into_iter()
                        .map(|d| DiagnosticEntry {
                            file: d.file,
                            line: d.line,
                            severity: d.severity,
                            message: d.message,
                        })
                        .collect();
                    self.diagnostics_tab.set_diagnostics(entries);
                }

                Event::Error(msg) => {
                    self.conversation.push_error(msg);
                }
            }
        }
    }
}

fn setup_terminal() -> anyhow::Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Entry point — sets up terminal, spawns runtime bridge, runs event loop
pub async fn run() -> anyhow::Result<()> {
    let model =
        std::env::var("OCX_MODEL").unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());
    let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let (cmd_tx, event_rx) = RuntimeBridge::spawn(model.clone(), workspace_root);

    let mut terminal = setup_terminal()?;

    // Derive a short session name from the model
    let session_name = "new session".to_string();
    let mut app = App::new(cmd_tx, event_rx, model, session_name);

    loop {
        terminal.draw(|frame| app.render(frame))?;
        app.process_events();
        app.key_seq.check_timeout();

        if event::poll(Duration::from_millis(50))? {
            if let CrosstermEvent::Key(key) = event::read()? {
                app.handle_key(key.code, key.modifiers);
            }
        }

        if app.should_quit {
            break;
        }
    }

    restore_terminal(&mut terminal)?;
    Ok(())
}
