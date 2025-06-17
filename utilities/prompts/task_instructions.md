# Task Instructions

## Sanity Check
Before proceeding with any code modifications, perform the following checks:
- Verify that `tasks.md`, `modules.md`, and `architecture.md` exist and are accessible. If any are missing, inform the user and wait for a response.
- Analyze `tasks.md` to ensure each task is broken into small, testable steps. Confirm that each step is clear, valid, and aligns with the code in the referenced files.
- Cross-reference the steps with the code to ensure they achieve the task’s purpose and cover all necessary modifications.
- Check if any starting code files are excessively long (>500 lines) or likely to become too long after edits. Warn the user if this is the case.
- If any step is ambiguous, incomplete, or infeasible, or if there are issues with `tasks.md` or the code, notify the user with specific details and wait for clarification. Do not proceed until resolved.
- If an obviously better alternative to a task exists (e.g., simpler approach meeting all objectives), suggest it to the user and wait for approval before proceeding.

Do not begin coding until all checks are complete and any issues are resolved by the user.

## How to Complete Each Task
- Address only one task at a time from `tasks.md`.
- Do not proceed to the next task until the current task is approved by the user.
- Ensure every modification compiles without errors or warnings and is tested to confirm functionality.
- For files over 100 lines, do not write the entire file. Use the edit format below to show only the relevant changes.

## Code Modification Formatting Instructions
For files over 200 lines, do not include the entire file. Instead, use the format below to specify each change clearly and concisely. Repeat the format for each separate edit in the same file.

### Format for Each Edit

Line number: <Start line> - <End line>
Action: <Add | Remove | Replace>

<Current code being removed or replaced. For Add: show the line of code immediately preceding the insertion point.>
```
#Existing Code:
<If Action is Add or Replace>
```
```
#New Code:
<New code to be added or used as replacement>
```

### Comment Handling Instructions
- **Existing Comments**: Preserve existing comments unless they are no longer relevant or have become outdated due to code changes. If removing or modifying a comment, explain why in the edit description (e.g., "Comment removed as it referenced outdated logic").
- **New or Modified Code Comments**: Add comments only when necessary to clarify non-obvious logic or functionality. Comments must be brief, sparse, and concise. Do not add:
  - Comments on every line of new or modified code.
  - Obvious comments (e.g., "This increments the counter").
  - Placeholder comments for future changes (e.g., "TODO: Add more logic").
- **Comment Style**: Follow the existing comment style in the file (e.g., `//`, `#`, `/* */`) for consistency. Keep comments on separate lines unless the file uses inline comments consistently.


## Begin Coding