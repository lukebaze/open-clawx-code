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
use crate::layout::AppLayout;
use crate::runtime_bridge::RuntimeBridge;
use crate::theme::Theme;
use crate::types::{Command, Event};
use crate::widgets::{
    context_panel::{ContextPanelWidget, ContextTab},
    conversation_panel::ConversationPanel,
    files_tab::FilesTab,
    git_tab::GitTab,
    input_bar::{InputBar, InputBuffer},
    status_bar::StatusBar,
};

/// Which panel has keyboard focus
#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Conversation,
    ContextPanel,
    InputBar,
}

struct App {
    focus: Focus,
    should_quit: bool,
    theme: Theme,
    input: InputBuffer,
    conversation: ConversationView,
    cmd_tx: mpsc::Sender<Command>,
    event_rx: mpsc::Receiver<Event>,
    // Context panel state
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
            focus: Focus::InputBar,
            should_quit: false,
            theme: Theme::default(),
            input: InputBuffer::new(),
            conversation: ConversationView::new(),
            cmd_tx,
            event_rx,
            active_tab: ContextTab::Files,
            files_tab: FilesTab::new(),
            git_tab,
            right_panel_pct: 30,
        }
    }

    fn render(&self, frame: &mut Frame) {
        let layout = AppLayout::new(frame.area(), self.right_panel_pct);

        frame.render_widget(StatusBar::new(&self.theme), layout.status_bar);

        frame.render_widget(
            ConversationPanel::new(
                &self.conversation,
                &self.theme,
                self.focus == Focus::Conversation,
            ),
            layout.conversation,
        );

        frame.render_widget(
            ContextPanelWidget {
                active_tab: self.active_tab,
                files_tab: &self.files_tab,
                git_tab: &self.git_tab,
                theme: &self.theme,
                focused: self.focus == Focus::ContextPanel,
            },
            layout.context_panel,
        );

        frame.render_widget(
            InputBar::new(
                &self.theme,
                self.focus == Focus::InputBar,
                self.input.text(),
                self.input.cursor(),
            ),
            layout.input_bar,
        );
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match (code, modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                if self.conversation.is_streaming {
                    let _ = self.cmd_tx.send(Command::Cancel);
                } else {
                    self.should_quit = true;
                    let _ = self.cmd_tx.send(Command::Quit);
                }
            }
            // Global quit (only when not typing)
            (KeyCode::Char('q'), KeyModifiers::NONE) if self.focus != Focus::InputBar => {
                self.should_quit = true;
                let _ = self.cmd_tx.send(Command::Quit);
            }
            // Escape → focus input bar
            (KeyCode::Esc, _) => {
                self.focus = Focus::InputBar;
            }
            // Focus switching
            (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                self.focus = Focus::Conversation;
            }
            (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                self.focus = Focus::ContextPanel;
            }
            (KeyCode::Tab, _) if self.focus != Focus::InputBar => {
                self.focus = match self.focus {
                    Focus::Conversation => Focus::ContextPanel,
                    Focus::ContextPanel => Focus::InputBar,
                    Focus::InputBar => Focus::Conversation,
                };
            }
            // Context panel tab cycling (when context focused)
            (KeyCode::Tab, _) if self.focus == Focus::ContextPanel => {
                self.active_tab = self.active_tab.next();
            }
            // Panel width adjustment
            (KeyCode::Char('['), KeyModifiers::CONTROL) => {
                self.right_panel_pct = self.right_panel_pct.saturating_sub(5).max(20);
            }
            (KeyCode::Char(']'), KeyModifiers::CONTROL) => {
                self.right_panel_pct = (self.right_panel_pct + 5).min(50);
            }
            // Conversation scroll (when conversation focused)
            (KeyCode::Char('j'), KeyModifiers::NONE) if self.focus == Focus::Conversation => {
                self.conversation.scroll_down(1);
            }
            (KeyCode::Char('k'), KeyModifiers::NONE) if self.focus == Focus::Conversation => {
                self.conversation.scroll_up(1);
            }
            // Context panel tab switch with number keys
            (KeyCode::Char('1'), KeyModifiers::NONE) if self.focus == Focus::ContextPanel => {
                self.active_tab = ContextTab::Files;
            }
            (KeyCode::Char('2'), KeyModifiers::NONE) if self.focus == Focus::ContextPanel => {
                self.active_tab = ContextTab::Git;
            }
            (KeyCode::Char('3'), KeyModifiers::NONE) if self.focus == Focus::ContextPanel => {
                self.active_tab = ContextTab::GitNexus;
            }
            // Input bar editing (when focused)
            _ if self.focus == Focus::InputBar => {
                self.handle_input_key(code, modifiers);
            }
            _ => {}
        }
    }

    fn handle_input_key(&mut self, code: KeyCode, _modifiers: KeyModifiers) {
        match code {
            KeyCode::Enter => {
                if !self.input.is_empty() && !self.conversation.is_streaming {
                    let text = self.input.take();
                    self.conversation.push_user_message(text.clone());
                    let _ = self.cmd_tx.send(Command::SendMessage(text));
                }
            }
            KeyCode::Tab => {
                self.focus = Focus::Conversation;
            }
            KeyCode::Char(ch) => self.input.insert(ch),
            KeyCode::Backspace => self.input.backspace(),
            KeyCode::Delete => self.input.delete(),
            KeyCode::Left => self.input.move_left(),
            KeyCode::Right => self.input.move_right(),
            KeyCode::Home => self.input.move_home(),
            KeyCode::End => self.input.move_end(),
            _ => {}
        }
    }

    /// Drain all pending runtime events
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
                    // Refresh git status after tool execution
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
