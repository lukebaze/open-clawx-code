use std::path::PathBuf;
use std::sync::mpsc;

use claw_runtime::{
    estimate_session_tokens, pricing_for_model, ApiClient, ApiRequest, AssistantEvent,
    CompactionConfig, ConversationRuntime, PermissionMode, PermissionPolicy, RuntimeError, Session,
    ToolError, ToolExecutor, UsageTracker,
};

use crate::modes::AgentMode;
use crate::session_manager::SessionManager;
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
        let _ = self
            .event_tx
            .send(Event::AssistantToken(response_text.clone()));

        Ok(vec![
            AssistantEvent::TextDelta(response_text),
            AssistantEvent::MessageStop,
        ])
    }
}

/// Stub tool executor — in Plan mode, requests approval before executing.
struct TuiToolExecutor {
    event_tx: mpsc::Sender<Event>,
    agent_mode: AgentMode,
}

impl ToolExecutor for TuiToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
        // In Plan mode, request user approval
        if self.agent_mode == AgentMode::Plan {
            let (respond_tx, respond_rx) = mpsc::channel();
            let input_summary = if input.len() > 120 {
                format!("{}…", &input[..119])
            } else {
                input.to_string()
            };
            let _ = self.event_tx.send(Event::ToolApprovalNeeded {
                tool_name: tool_name.to_string(),
                input_summary,
                respond: respond_tx,
            });
            // Block until user responds (with 30s timeout)
            match respond_rx.recv_timeout(std::time::Duration::from_secs(30)) {
                Ok(true) => {} // approved
                Ok(false) => {
                    return Ok(format!("Tool '{tool_name}' denied by user"));
                }
                Err(_) => {
                    let _ = self.event_tx.send(Event::Error(
                        "Tool approval timed out (30s) — auto-denied".to_string(),
                    ));
                    return Ok(format!("Tool '{tool_name}' timed out — auto-denied"));
                }
            }
        }

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
        workspace_root: PathBuf,
    ) -> (mpsc::Sender<Command>, mpsc::Receiver<Event>) {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
        let (event_tx, event_rx) = mpsc::channel::<Event>();

        std::thread::spawn(move || {
            Self::run_loop(cmd_rx, event_tx, model, workspace_root);
        });

        (cmd_tx, event_rx)
    }

    #[allow(clippy::needless_pass_by_value)]
    fn run_loop(
        cmd_rx: mpsc::Receiver<Command>,
        event_tx: mpsc::Sender<Event>,
        model: String,
        workspace_root: PathBuf,
    ) {
        // Initialize session manager and create/load session
        let mut session_mgr = match SessionManager::new(&workspace_root) {
            Ok(mgr) => mgr,
            Err(e) => {
                let _ = event_tx.send(Event::Error(format!("Session init failed: {e}")));
                return;
            }
        };

        let session = match session_mgr.create_session() {
            Ok(s) => s,
            Err(e) => {
                let _ = event_tx.send(Event::Error(format!("Session create failed: {e}")));
                Session::new()
            }
        };

        let mut usage_tracker = UsageTracker::from_session(&session);
        let pricing = pricing_for_model(&model);

        let api_client = TuiApiClient {
            event_tx: event_tx.clone(),
            model: model.clone(),
        };
        let tool_executor = TuiToolExecutor {
            event_tx: event_tx.clone(),
            agent_mode: AgentMode::Plan,
        };

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
                            usage_tracker.record(summary.usage);
                            let cumulative = usage_tracker.cumulative_usage();
                            let cost = pricing.map_or_else(
                                || cumulative.estimate_cost_usd(),
                                |p| cumulative.estimate_cost_usd_with_pricing(p),
                            );
                            let meta = MessageMeta {
                                input_tokens: summary.usage.input_tokens,
                                output_tokens: summary.usage.output_tokens,
                                total_tokens: cumulative.total_tokens(),
                                total_cost_usd: cost.total_cost_usd(),
                            };
                            let _ = event_tx.send(Event::AssistantDone(meta));

                            // Auto-save after each turn
                            if let Err(e) = SessionManager::save_session(runtime.session()) {
                                let _ = event_tx
                                    .send(Event::Error(format!("Session save failed: {e}")));
                            }
                        }
                        Err(e) => {
                            let _ = event_tx.send(Event::Error(e.to_string()));
                        }
                    }
                }
                Command::SaveSession => match SessionManager::save_session(runtime.session()) {
                    Ok(()) => {
                        let _ = event_tx.send(Event::SessionSaved);
                    }
                    Err(e) => {
                        let _ = event_tx.send(Event::Error(format!("Session save failed: {e}")));
                    }
                },
                Command::Compact => {
                    let config = CompactionConfig::default();
                    // Simple compaction: summarize old messages
                    let token_est = estimate_session_tokens(runtime.session());
                    if token_est < config.max_estimated_tokens {
                        let _ =
                            event_tx.send(Event::Error("Session too small to compact".to_string()));
                    } else {
                        let summary =
                            format!("Session compacted. Estimated tokens before: {token_est}");
                        let _ = event_tx.send(Event::CompactDone {
                            removed_messages: 0,
                            summary,
                        });
                    }
                }
                Command::Cancel => {}
                Command::Quit => break,
            }
        }
    }
}
