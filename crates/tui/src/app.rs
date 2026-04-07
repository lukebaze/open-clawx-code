use std::io::{self, Stdout};
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
use crate::runtime_bridge::RuntimeBridge;
use crate::theme::Theme;
use crate::types::{Command, Event};
use crate::widgets::{
    autocomplete_dropdown::AutocompleteDropdown,
    context_panel::{ContextPanelWidget, ContextTab},
    conversation_panel::ConversationPanel,
    files_tab::FilesTab,
    git_tab::GitTab,
    help_overlay::HelpOverlay,
    input_bar::{InputBar, InputBuffer},
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
}

impl App {
    fn new(cmd_tx: mpsc::Sender<Command>, event_rx: mpsc::Receiver<Event>) -> Self {
        let mut git_tab = GitTab::new();
        git_tab.refresh();

        Self {
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
            right_panel_pct: 30,
        }
    }

    fn render(&self, frame: &mut Frame) {
        let layout = AppLayout::new(frame.area(), self.right_panel_pct);

        frame.render_widget(
            StatusBar::new(&self.theme, self.mode, self.key_seq.has_pending()),
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
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // Global keybindings (work in any mode except Help)
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
                    // Clear input in insert mode
                    self.input.take();
                    self.slash_completer.close();
                    self.mode = InputMode::Insert;
                } else {
                    self.should_quit = true;
                    let _ = self.cmd_tx.send(Command::Quit);
                }
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
            KeyCode::Char(ch @ ('G' | 'g')) => {
                match self.key_seq.feed(ch) {
                    KeyAction::ScrollToTop => self.conversation.scroll_to_top(),
                    KeyAction::ScrollToBottom => self.conversation.scroll_to_bottom(),
                    KeyAction::Pending | KeyAction::None => {}
                }
            }
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
                    // Insert the command name
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
        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
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
            _ => {
                self.conversation.push_error(format!("Unknown command: {cmd}"));
            }
        }
    }

    fn process_events(&mut self) {
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                Event::AssistantToken(token) => {
                    self.conversation.push_token(&token);
                }
                Event::AssistantDone(meta) => {
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

    let (cmd_tx, event_rx) = RuntimeBridge::spawn(model);

    let mut terminal = setup_terminal()?;
    let mut app = App::new(cmd_tx, event_rx);

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
