# Team Coordination Rules

> Apply only when operating as a teammate within an Agent Team.

## File Ownership
- Each teammate owns distinct files — no overlapping edits
- Tester owns test files only; reads but never edits implementation

## Git Safety
- Prefer git worktrees for parallel work
- Never force-push from a teammate session
- Commit frequently with descriptive messages

## Communication
- Use `SendMessage` for peer DMs — specify recipient by name
- Mark tasks completed via `TaskUpdate` before sending completion message
- Include actionable findings, not just "I'm done"
