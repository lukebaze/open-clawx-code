//! TDD-driven orchestrator state machine for Build mode.
//!
//! Drives the cycle: Analyze → Red (write tests) → Green (implement) →
//! Unit verify → E2E verify → Refactor → Done.

pub mod agent_team;
pub mod test_runner;

use std::path::{Path, PathBuf};
use std::sync::mpsc;

pub use agent_team::{
    AgentContext, AgentMessage, AgentStatus, AgentTeam, AgentTeamEvent, MessageContent,
};
pub use test_runner::{detect_framework, FailedTest, TestFramework, TestResult, TestRunner};

use ocx_gitnexus::{GitNexusClient, ImpactResult};

/// TDD phase identifier for status display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TddPhase {
    Analyzing,
    TestWriting,
    Implementing,
    UnitVerify,
    E2EVerify,
    Refactoring,
}

impl TddPhase {
    /// Short label with emoji for status bar.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Analyzing => "ANALYZE",
            Self::TestWriting => "RED",
            Self::Implementing => "GREEN",
            Self::UnitVerify => "UNIT",
            Self::E2EVerify => "E2E",
            Self::Refactoring => "REFACTOR",
        }
    }
}

/// Context preserved when a test failure exhausts retries.
#[derive(Debug, Clone)]
pub struct FailureContext {
    pub phase: TddPhase,
    pub test_output: String,
    pub attempt_count: u8,
    pub last_fix_attempted: String,
}

/// Unit vs E2E test type for events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestType {
    Unit,
    E2E,
}

impl TestType {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Unit => "Unit",
            Self::E2E => "E2E",
        }
    }
}

/// Events emitted by the orchestrator to the TUI.
#[derive(Debug, Clone)]
pub enum OrchestratorEvent {
    /// TDD phase transition.
    PhaseChanged { phase: TddPhase, detail: String },
    /// Test suite started running.
    TestRunStarted { test_type: TestType, scope: String },
    /// Test suite completed.
    TestRunCompleted {
        test_type: TestType,
        result: TestResult,
    },
    /// Retrying after test failure.
    TestRetrying {
        attempt: u8,
        max: u8,
        test_name: String,
    },
    /// All retries exhausted — needs user intervention.
    TestRetryExhausted {
        phase: TddPhase,
        failure: FailureContext,
    },
    /// Pre-edit impact gate triggered (HIGH/CRITICAL risk).
    ImpactGateTriggered {
        impact: ImpactResult,
        /// Sender to approve (true) or deny (false) the edit.
        respond: mpsc::Sender<bool>,
    },
    /// Iteration count updated.
    IterationUpdated { current: u32, max: u32 },
    /// Max iterations reached — Build mode paused.
    MaxIterationsReached { count: u32 },
    /// Orchestrator completed the task.
    Done { summary: String },
    /// Unrecoverable failure.
    Failed {
        message: String,
        context: Option<FailureContext>,
    },
}

/// The TDD-driven orchestrator state machine.
///
/// In Build mode, drives: Idle → Analyzing → Test writing → Implementing →
/// Unit verify → E2E verify → (Refactoring) → Done.
///
/// In Plan mode, pauses at each phase transition for user approval.
pub struct Orchestrator {
    state: OrchestratorState,
    iteration_count: u32,
    max_iterations: u32,
    max_test_retries: u8,
    test_runner: TestRunner,
    gitnexus: GitNexusClient,
    event_tx: mpsc::Sender<OrchestratorEvent>,
    interrupted: bool,
}

/// Internal state of the orchestrator.
#[derive(Debug, Clone)]
pub enum OrchestratorState {
    Idle,
    Analyzing {
        task_description: String,
    },
    TestWriting {
        task_description: String,
        test_files: Vec<PathBuf>,
    },
    Implementing {
        task_description: String,
        retry_count: u8,
    },
    UnitVerifying {
        task_description: String,
        attempt: u8,
    },
    E2EVerifying {
        task_description: String,
        attempt: u8,
    },
    Refactoring {
        task_description: String,
    },
    WaitingApproval {
        reason: String,
    },
    Done {
        summary: String,
    },
    Failed {
        message: String,
        context: Option<FailureContext>,
    },
}

impl Orchestrator {
    /// Create a new orchestrator for a project.
    #[must_use]
    pub fn new(project_root: &Path, event_tx: mpsc::Sender<OrchestratorEvent>) -> Self {
        Self {
            state: OrchestratorState::Idle,
            iteration_count: 0,
            max_iterations: 25,
            max_test_retries: 3,
            test_runner: TestRunner::new(project_root),
            gitnexus: GitNexusClient::new(project_root),
            event_tx,
            interrupted: false,
        }
    }

    /// Current state reference.
    #[must_use]
    pub fn state(&self) -> &OrchestratorState {
        &self.state
    }

    /// Current TDD phase (if in a TDD state).
    #[must_use]
    pub fn current_phase(&self) -> Option<TddPhase> {
        match &self.state {
            OrchestratorState::Analyzing { .. } => Some(TddPhase::Analyzing),
            OrchestratorState::TestWriting { .. } => Some(TddPhase::TestWriting),
            OrchestratorState::Implementing { .. } => Some(TddPhase::Implementing),
            OrchestratorState::UnitVerifying { .. } => Some(TddPhase::UnitVerify),
            OrchestratorState::E2EVerifying { .. } => Some(TddPhase::E2EVerify),
            OrchestratorState::Refactoring { .. } => Some(TddPhase::Refactoring),
            _ => None,
        }
    }

    /// Current iteration count.
    #[must_use]
    pub fn iteration_count(&self) -> u32 {
        self.iteration_count
    }

    /// Max iterations limit.
    #[must_use]
    pub fn max_iterations(&self) -> u32 {
        self.max_iterations
    }

    /// Whether `GitNexus` is available.
    #[must_use]
    pub fn gitnexus_available(&self) -> bool {
        self.gitnexus.is_available()
    }

    /// Signal interrupt (Ctrl+C) — orchestrator pauses at next safe point.
    pub fn interrupt(&mut self) {
        self.interrupted = true;
    }

    /// Start a new TDD cycle for a task.
    pub fn start_task(&mut self, task_description: String) {
        self.iteration_count = 0;
        self.interrupted = false;
        self.transition_to(OrchestratorState::Analyzing { task_description });
    }

    /// Advance the state machine by one step. Called by the runtime bridge
    /// after the LLM completes an action for the current phase.
    ///
    /// Returns `true` if the orchestrator needs the LLM to take another action,
    /// `false` if it's waiting (approval, done, failed).
    #[allow(clippy::too_many_lines)]
    pub fn step(&mut self) -> bool {
        // Check interrupt
        if self.interrupted {
            self.interrupted = false;
            let reason = format!(
                "Interrupted at {:?} (iteration {}/{})",
                self.current_phase(),
                self.iteration_count,
                self.max_iterations,
            );
            self.transition_to(OrchestratorState::WaitingApproval { reason });
            return false;
        }

        // Check max iterations
        self.iteration_count += 1;
        let _ = self.event_tx.send(OrchestratorEvent::IterationUpdated {
            current: self.iteration_count,
            max: self.max_iterations,
        });
        if self.iteration_count >= self.max_iterations {
            let _ = self.event_tx.send(OrchestratorEvent::MaxIterationsReached {
                count: self.iteration_count,
            });
            self.transition_to(OrchestratorState::WaitingApproval {
                reason: format!(
                    "Max iterations ({}) reached — pausing Build mode",
                    self.max_iterations
                ),
            });
            return false;
        }

        // State transitions
        match self.state.clone() {
            OrchestratorState::Analyzing { task_description } => {
                // After analysis, move to test writing (Red phase)
                self.transition_to(OrchestratorState::TestWriting {
                    task_description,
                    test_files: vec![],
                });
                true
            }
            OrchestratorState::TestWriting {
                task_description, ..
            } => {
                // After test writing, move to implementing (Green phase)
                self.transition_to(OrchestratorState::Implementing {
                    task_description,
                    retry_count: 0,
                });
                true
            }
            OrchestratorState::Implementing {
                task_description, ..
            } => {
                // After implementation attempt, run unit tests
                self.transition_to(OrchestratorState::UnitVerifying {
                    task_description,
                    attempt: 1,
                });
                true
            }
            OrchestratorState::UnitVerifying {
                task_description,
                attempt,
            } => {
                // Run unit tests
                let _ = self.event_tx.send(OrchestratorEvent::TestRunStarted {
                    test_type: TestType::Unit,
                    scope: "changed modules".to_string(),
                });
                let result = self.test_runner.run_unit_tests(&[]);
                let _ = self.event_tx.send(OrchestratorEvent::TestRunCompleted {
                    test_type: TestType::Unit,
                    result: result.clone(),
                });

                if result.passed {
                    // Unit tests green → E2E verify
                    self.transition_to(OrchestratorState::E2EVerifying {
                        task_description,
                        attempt: 1,
                    });
                    true
                } else if attempt < self.max_test_retries {
                    // Retry
                    let _ = self.event_tx.send(OrchestratorEvent::TestRetrying {
                        attempt,
                        max: self.max_test_retries,
                        test_name: result
                            .failed_tests
                            .first()
                            .map_or("unknown".to_string(), |t| t.name.clone()),
                    });
                    self.transition_to(OrchestratorState::Implementing {
                        task_description,
                        retry_count: attempt,
                    });
                    true
                } else {
                    // Max retries exhausted
                    let failure = FailureContext {
                        phase: TddPhase::UnitVerify,
                        test_output: result.output,
                        attempt_count: attempt,
                        last_fix_attempted: String::new(),
                    };
                    let _ = self.event_tx.send(OrchestratorEvent::TestRetryExhausted {
                        phase: TddPhase::UnitVerify,
                        failure: failure.clone(),
                    });
                    self.transition_to(OrchestratorState::Failed {
                        message: "Unit tests failed after max retries".to_string(),
                        context: Some(failure),
                    });
                    false
                }
            }
            OrchestratorState::E2EVerifying {
                task_description,
                attempt,
            } => {
                // Run E2E tests
                let _ = self.event_tx.send(OrchestratorEvent::TestRunStarted {
                    test_type: TestType::E2E,
                    scope: "full suite".to_string(),
                });
                let result = self.test_runner.run_e2e_tests();
                let _ = self.event_tx.send(OrchestratorEvent::TestRunCompleted {
                    test_type: TestType::E2E,
                    result: result.clone(),
                });

                if result.passed {
                    // E2E green → Done
                    let summary = format!(
                        "TDD cycle complete: unit+E2E green in {} iterations",
                        self.iteration_count
                    );
                    let _ = self.event_tx.send(OrchestratorEvent::Done {
                        summary: summary.clone(),
                    });
                    self.transition_to(OrchestratorState::Done { summary });
                    false
                } else if attempt < self.max_test_retries {
                    let _ = self.event_tx.send(OrchestratorEvent::TestRetrying {
                        attempt,
                        max: self.max_test_retries,
                        test_name: result
                            .failed_tests
                            .first()
                            .map_or("unknown".to_string(), |t| t.name.clone()),
                    });
                    self.transition_to(OrchestratorState::Implementing {
                        task_description,
                        retry_count: attempt,
                    });
                    true
                } else {
                    let failure = FailureContext {
                        phase: TddPhase::E2EVerify,
                        test_output: result.output,
                        attempt_count: attempt,
                        last_fix_attempted: String::new(),
                    };
                    let _ = self.event_tx.send(OrchestratorEvent::TestRetryExhausted {
                        phase: TddPhase::E2EVerify,
                        failure: failure.clone(),
                    });
                    self.transition_to(OrchestratorState::Failed {
                        message: "E2E tests failed after max retries".to_string(),
                        context: Some(failure),
                    });
                    false
                }
            }
            OrchestratorState::Refactoring { task_description } => {
                // After refactoring, re-verify with unit tests
                self.transition_to(OrchestratorState::UnitVerifying {
                    task_description,
                    attempt: 1,
                });
                true
            }
            OrchestratorState::Idle
            | OrchestratorState::WaitingApproval { .. }
            | OrchestratorState::Done { .. }
            | OrchestratorState::Failed { .. } => false,
        }
    }

    /// Run pre-edit impact analysis. Returns the impact result.
    /// If risk is HIGH/CRITICAL, emits `ImpactGateTriggered` and blocks
    /// until the user responds.
    #[must_use]
    pub fn check_impact(&self, symbol: &str) -> Option<ImpactResult> {
        if !self.gitnexus.is_available() {
            return None;
        }

        match self.gitnexus.impact(symbol) {
            Ok(impact) => {
                if impact.risk_level.requires_approval() {
                    let (tx, rx) = mpsc::channel();
                    let _ = self.event_tx.send(OrchestratorEvent::ImpactGateTriggered {
                        impact: impact.clone(),
                        respond: tx,
                    });
                    // Block until user responds (30s timeout)
                    match rx.recv_timeout(std::time::Duration::from_secs(30)) {
                        Ok(true) => Some(impact),
                        Ok(false) | Err(_) => None, // denied or timeout
                    }
                } else {
                    Some(impact)
                }
            }
            Err(_) => None,
        }
    }

    /// Resume from `WaitingApproval` state, continuing the TDD cycle.
    pub fn resume(&mut self, task_description: String) {
        self.interrupted = false;
        self.transition_to(OrchestratorState::Analyzing { task_description });
    }

    fn transition_to(&mut self, new_state: OrchestratorState) {
        if let Some(phase) = phase_for_state(&new_state) {
            let detail = match &new_state {
                OrchestratorState::Implementing { retry_count, .. } => {
                    format!("retry {retry_count}")
                }
                OrchestratorState::UnitVerifying { attempt, .. }
                | OrchestratorState::E2EVerifying { attempt, .. } => {
                    format!("attempt {attempt}")
                }
                _ => String::new(),
            };
            let _ = self
                .event_tx
                .send(OrchestratorEvent::PhaseChanged { phase, detail });
        }
        self.state = new_state;
    }
}

fn phase_for_state(state: &OrchestratorState) -> Option<TddPhase> {
    match state {
        OrchestratorState::Analyzing { .. } => Some(TddPhase::Analyzing),
        OrchestratorState::TestWriting { .. } => Some(TddPhase::TestWriting),
        OrchestratorState::Implementing { .. } => Some(TddPhase::Implementing),
        OrchestratorState::UnitVerifying { .. } => Some(TddPhase::UnitVerify),
        OrchestratorState::E2EVerifying { .. } => Some(TddPhase::E2EVerify),
        OrchestratorState::Refactoring { .. } => Some(TddPhase::Refactoring),
        _ => None,
    }
}
