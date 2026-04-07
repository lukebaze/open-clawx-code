# Orchestration Protocol

## Delegation Context
When spawning subagents, always include:
1. Work context path
2. Reports path: `{work_context}/plans/reports/`
3. Plans path: `{work_context}/plans/`

## Subagent Status Protocol
Subagents report: **DONE**, **DONE_WITH_CONCERNS**, **BLOCKED**, or **NEEDS_CONTEXT**.

## Sequential Chaining
Planning → Implementation → Testing → Review → Documentation

## Parallel Execution
Independent tasks can run simultaneously. No file ownership conflicts.

## Context Isolation
Subagents get only what they need: task description, file paths, acceptance criteria.
