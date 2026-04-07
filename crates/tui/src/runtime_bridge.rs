use std::sync::mpsc;

use claw_runtime::{
    ApiClient, ApiRequest, AssistantEvent, ConversationRuntime, PermissionMode, PermissionPolicy,
    RuntimeError, Session, ToolError, ToolExecutor,
};

use crate::types::{Command, Event, MessageMeta};

/// Stub API client that echoes back a placeholder response.
/// Real provider integration happens in Phase 07.
struct TuiApiClient {
    event_tx: mpsc::Sender<Event>,
    model: String,
}

impl ApiClient for TuiApiClient {
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        let _ = &request;
        let response_text = format!(
            "I received your message. (model: {}, provider not yet connected)",
            self.model
        );
        let _ = self.event_tx.send(Event::AssistantToken(response_text.clone()));

        Ok(vec![
            AssistantEvent::TextDelta(response_text),
            AssistantEvent::MessageStop,
        ])
    }
}

/// Stub tool executor — returns "not implemented" for all tools
struct TuiToolExecutor {
    event_tx: mpsc::Sender<Event>,
}

impl ToolExecutor for TuiToolExecutor {
    fn execute(&mut self, tool_name: &str, _input: &str) -> Result<String, ToolError> {
        let _ = self.event_tx.send(Event::ToolStart {
            name: tool_name.to_string(),
        });
        let result = format!("Tool '{tool_name}' execution not yet implemented");
        let _ = self.event_tx.send(Event::ToolEnd {
            result: result.clone(),
        });
        Ok(result)
    }
}

/// Bridges TUI commands to `ConversationRuntime`.
/// Runs on a dedicated OS thread (runtime types are !Send).
pub struct RuntimeBridge;

impl RuntimeBridge {
    /// Spawn the bridge on a background thread.
    /// Returns the command sender (TUI → bridge) and event receiver (bridge → TUI).
    pub fn spawn(
        model: String,
    ) -> (mpsc::Sender<Command>, mpsc::Receiver<Event>) {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
        let (event_tx, event_rx) = mpsc::channel::<Event>();

        std::thread::spawn(move || {
            Self::run_loop(cmd_rx, event_tx, model);
        });

        (cmd_tx, event_rx)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn run_loop(
        cmd_rx: mpsc::Receiver<Command>,
        event_tx: mpsc::Sender<Event>,
        model: String,
    ) {
        let api_client = TuiApiClient {
            event_tx: event_tx.clone(),
            model,
        };
        let tool_executor = TuiToolExecutor {
            event_tx: event_tx.clone(),
        };

        let session = Session::new();
        let mut runtime = ConversationRuntime::new(
            session,
            api_client,
            tool_executor,
            PermissionPolicy::new(PermissionMode::Allow),
            vec!["You are a helpful coding assistant.".to_string()],
        )
        .with_max_iterations(10);

        while let Ok(cmd) = cmd_rx.recv() {
            match cmd {
                Command::SendMessage(text) => {
                    let result = runtime.run_turn(&text, None);
                    match result {
                        Ok(summary) => {
                            let meta = MessageMeta {
                                input_tokens: summary.usage.input_tokens,
                                output_tokens: summary.usage.output_tokens,
                            };
                            let _ = event_tx.send(Event::AssistantDone(meta));
                        }
                        Err(e) => {
                            let _ = event_tx.send(Event::Error(e.to_string()));
                        }
                    }
                }
                Command::Cancel => {}
                Command::Quit => break,
            }
        }
    }
}
