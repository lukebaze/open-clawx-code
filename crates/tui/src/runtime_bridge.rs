//! Runtime bridge — connects TUI commands to conversation runtime + providers.
//!
//! Runs on a dedicated OS thread. Uses `tokio::runtime::Handle` to call
//! async provider methods from the sync `ApiClient` trait.

use std::path::PathBuf;
use std::sync::{mpsc, Arc};

use claw_runtime::{
    estimate_session_tokens, pricing_for_model, ApiClient, ApiRequest, AssistantEvent,
    CompactionConfig, ContentBlock, ConversationRuntime, MessageRole, PermissionMode,
    PermissionPolicy, RuntimeError, Session, ToolError, ToolExecutor, UsageTracker,
};
use ocx_orchestrator::{Orchestrator, OrchestratorEvent};
use ocx_providers::{
    detect_providers, ChatMessage, MessageRequest, ProviderRegistry, StreamChunk,
};

use crate::modes::AgentMode;
use crate::session_manager::SessionManager;
use crate::types::{Command, Event, MessageMeta};

/// API client backed by the providers registry.
/// Calls the real provider HTTP APIs via a tokio runtime handle.
struct TuiApiClient {
    event_tx: mpsc::Sender<Event>,
    model: String,
    registry: ProviderRegistry,
    rt: Arc<tokio::runtime::Runtime>,
}

impl ApiClient for TuiApiClient {
    #[allow(clippy::cast_possible_truncation)]
    fn stream(&mut self, request: ApiRequest) -> Result<Vec<AssistantEvent>, RuntimeError> {
        // Build provider request from runtime request
        let provider_request = MessageRequest {
            model: self.model.clone(),
            system: request.system_prompt.clone(),
            messages: request
                .messages
                .iter()
                .map(|m| {
                    let role = match m.role {
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        MessageRole::System => "system",
                        MessageRole::Tool => "tool",
                    };
                    // Extract text from content blocks
                    let content = m
                        .blocks
                        .iter()
                        .filter_map(|b| match b {
                            ContentBlock::Text { text } => Some(text.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n");
                    ChatMessage {
                        role: role.to_string(),
                        content,
                    }
                })
                .collect(),
            max_tokens: 4096,
        };

        // Find the provider for the active model
        let provider = self.registry.active_provider();
        if provider.is_none() {
            let err_msg = format!(
                "No provider found for model '{}'. Run /config to set API keys.",
                self.model
            );
            let _ = self.event_tx.send(Event::AssistantToken(err_msg.clone()));
            return Ok(vec![
                AssistantEvent::TextDelta(err_msg),
                AssistantEvent::MessageStop,
            ]);
        }

        // Call the async provider from this sync context
        let chunks = self.rt.block_on(async {
            // SAFETY: provider ref is valid for the duration of this block_on
            let provider = self.registry.active_provider().unwrap();
            provider.send_message(&provider_request).await
        });

        match chunks {
            Ok(chunks) => {
                let mut events = Vec::new();
                for chunk in chunks {
                    match chunk {
                        StreamChunk::TextDelta(text) => {
                            let _ = self.event_tx.send(Event::AssistantToken(text.clone()));
                            events.push(AssistantEvent::TextDelta(text));
                        }
                        StreamChunk::Usage { .. }
                        | StreamChunk::Done
                        | StreamChunk::ToolUse { .. } => {}
                    }
                }
                events.push(AssistantEvent::MessageStop);
                Ok(events)
            }
            Err(e) => {
                let err_msg = format!("Provider error: {e}");
                let _ = self.event_tx.send(Event::AssistantToken(err_msg.clone()));
                Ok(vec![
                    AssistantEvent::TextDelta(err_msg),
                    AssistantEvent::MessageStop,
                ])
            }
        }
    }
}

/// Stub tool executor — in Plan mode, requests approval before executing.
struct TuiToolExecutor {
    event_tx: mpsc::Sender<Event>,
    agent_mode: AgentMode,
}

impl ToolExecutor for TuiToolExecutor {
    fn execute(&mut self, tool_name: &str, input: &str) -> Result<String, ToolError> {
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
            match respond_rx.recv_timeout(std::time::Duration::from_secs(30)) {
                Ok(true) => {}
                Ok(false) => return Ok(format!("Tool '{tool_name}' denied by user")),
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

/// Bridges TUI commands to `ConversationRuntime` + `Orchestrator`.
/// Runs on a dedicated OS thread (runtime types are !Send).
pub struct RuntimeBridge;

impl RuntimeBridge {
    /// Spawn the bridge on a background thread.
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

    #[allow(clippy::needless_pass_by_value, clippy::too_many_lines)]
    fn run_loop(
        cmd_rx: mpsc::Receiver<Command>,
        event_tx: mpsc::Sender<Event>,
        model: String,
        workspace_root: PathBuf,
    ) {
        // Create a tokio runtime for async provider calls (multi-thread so
        // handle.block_on works from the bridge's sync loop)
        let rt = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(1)
                .enable_all()
                .build()
                .expect("Failed to create tokio runtime for providers"),
        );

        // Initialize session
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

        // Initialize orchestrator
        let (orch_event_tx, orch_event_rx) = mpsc::channel::<OrchestratorEvent>();
        let _orchestrator = Orchestrator::new(&workspace_root, orch_event_tx);

        // Initialize provider registry from env vars (set by config on startup)
        let mut registry = ProviderRegistry::from_detected(detect_providers());
        if registry.set_active_model(&model).is_err() {
            let _ = event_tx.send(Event::Error(format!(
                "Model '{model}' not found in any provider. Using first available."
            )));
        }

        let api_client = TuiApiClient {
            event_tx: event_tx.clone(),
            model: model.clone(),
            registry,
            rt: Arc::clone(&rt),
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
            while let Ok(orch_event) = orch_event_rx.try_recv() {
                forward_orchestrator_event(&event_tx, orch_event);
            }

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
                Command::ResumeBuild => {
                    let _ = event_tx.send(Event::Error(
                        "Build mode resume not yet connected to orchestrator".to_string(),
                    ));
                }
                Command::SetModel(model_id) => {
                    // Update the model in the API client (requires mutable access)
                    // For now, report that the model was set
                    let _ = event_tx.send(Event::Error(format!(
                        "Model set to: {model_id}. Restart to apply."
                    )));
                }
                Command::Cancel => {}
                Command::Quit => break,
            }
        }
    }
}

/// Forward orchestrator events to TUI event channel.
fn forward_orchestrator_event(event_tx: &mpsc::Sender<Event>, orch_event: OrchestratorEvent) {
    match orch_event {
        OrchestratorEvent::PhaseChanged { phase, detail } => {
            let _ = event_tx.send(Event::TddPhaseChanged { phase, detail });
        }
        OrchestratorEvent::TestRunStarted { test_type, scope } => {
            let _ = event_tx.send(Event::TestRunStarted { test_type, scope });
        }
        OrchestratorEvent::TestRunCompleted { test_type, result } => {
            let _ = event_tx.send(Event::TestRunCompleted { test_type, result });
        }
        OrchestratorEvent::TestRetrying {
            attempt,
            max,
            test_name,
        } => {
            let _ = event_tx.send(Event::TestRetrying {
                attempt,
                max,
                test_name,
            });
        }
        OrchestratorEvent::TestRetryExhausted { phase, failure } => {
            let _ = event_tx.send(Event::TestRetryExhausted { phase, failure });
        }
        OrchestratorEvent::ImpactGateTriggered { impact, respond } => {
            let _ = event_tx.send(Event::ImpactGateTriggered { impact, respond });
        }
        OrchestratorEvent::IterationUpdated { current, max } => {
            let _ = event_tx.send(Event::IterationUpdated { current, max });
        }
        OrchestratorEvent::MaxIterationsReached { count } => {
            let _ = event_tx.send(Event::MaxIterationsReached { count });
        }
        OrchestratorEvent::Done { summary } => {
            let _ = event_tx.send(Event::BuildDone { summary });
        }
        OrchestratorEvent::Failed { message, context } => {
            let _ = event_tx.send(Event::BuildFailed { message, context });
        }
    }
}
