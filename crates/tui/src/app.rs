use std::io::{self, Stdout};
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Frame, Terminal};

use crate::layout::AppLayout;
use crate::theme::Theme;
use crate::widgets::{input_bar::InputBar, placeholder::PlaceholderPanel, status_bar::StatusBar};

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
}

impl App {
    fn new() -> Self {
        Self {
            focus: Focus::InputBar,
            should_quit: false,
            theme: Theme::default(),
        }
    }

    fn render(&self, frame: &mut Frame) {
        let layout = AppLayout::new(frame.area());

        frame.render_widget(StatusBar::new(&self.theme), layout.status_bar);

        frame.render_widget(
            PlaceholderPanel::new(
                "Conversation",
                &self.theme,
                self.focus == Focus::Conversation,
            ),
            layout.conversation,
        );

        frame.render_widget(
            PlaceholderPanel::new("Context", &self.theme, self.focus == Focus::ContextPanel),
            layout.context_panel,
        );

        frame.render_widget(
            InputBar::new(&self.theme, self.focus == Focus::InputBar),
            layout.input_bar,
        );
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        match (code, modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) | (KeyCode::Char('q'), _) => {
                self.should_quit = true;
            }
            // Ctrl+H → focus left (conversation)
            (KeyCode::Char('h'), KeyModifiers::CONTROL) => {
                self.focus = Focus::Conversation;
            }
            // Ctrl+L → focus right (context panel)
            (KeyCode::Char('l'), KeyModifiers::CONTROL) => {
                self.focus = Focus::ContextPanel;
            }
            // Tab → cycle focus
            (KeyCode::Tab, _) => {
                self.focus = match self.focus {
                    Focus::Conversation => Focus::ContextPanel,
                    Focus::ContextPanel => Focus::InputBar,
                    Focus::InputBar => Focus::Conversation,
                };
            }
            _ => {}
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

/// Entry point — sets up terminal, runs event loop, restores terminal on exit
pub async fn run() -> anyhow::Result<()> {
    let mut terminal = setup_terminal()?;
    let mut app = App::new();

    loop {
        terminal.draw(|frame| app.render(frame))?;

        // Poll for events with 250ms timeout (gives ~4 fps redraw)
        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
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
