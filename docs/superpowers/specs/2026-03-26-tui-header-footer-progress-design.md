# TUI Header/Footer Progress Redesign

## Context

SysClean currently shows page tabs at the top, task status beside them, main content in the middle, and a two-row footer at the bottom.

The current bottom task progress area is not visually effective for users during scanning. The user requested:

- remove the current bottom progress presentation
- place a visible progress component in the top-right area
- change the bottom area to show the most useful current-page shortcuts so users can learn the app faster

The redesign must preserve the existing TUI visual language and avoid changing application behavior outside rendering.

## Goals

- Make background task progress easier to notice without stealing too much space from the main content
- Reduce footer noise by showing only the most useful shortcuts for the active page
- Preserve the current overall structure and color language of the application
- Keep the change scoped to UI rendering wherever possible

## Non-Goals

- No changes to task scheduling, scanning logic, cleanup logic, or app state transitions
- No new dialogs, new pages, or help-system redesign
- No attempt to add snapshot-based UI tests in this change

## Chosen Direction

The approved direction is the stronger visual variant from the explored mockups:

- keep the left side of the header as the existing page tabs
- keep the right side as a status panel
- inside that status panel, add a compact task card on the right when a task is active
- remove the bottom task progress/status row
- convert the footer into a single shortcut-focused panel that changes with the current page

This direction keeps the current layout recognizable while giving progress a clearer presence.

## Layout Design

### Header

The header remains a two-column layout:

- left: application tabs
- right: status block

The status block becomes an internal horizontal layout with two regions:

- status text region on the left
- task card region on the right

The task card is a fixed-width compact component styled as a distinct mini-panel. It should visually feel like a small status widget rather than plain inline text.

### Task Card Behavior

When `task_status` exists and exposes a progress ratio:

- show a small title such as `后台任务`
- show percentage text
- show a progress bar
- show one short supporting line, preferring progress detail text when available

When `task_status` exists but has no progress ratio:

- do not force a fake progress bar
- show a compact textual task summary in the task card area

When no task is active:

- do not show the task card
- let the status text region use the available width naturally

This avoids empty chrome when idle while keeping progress highly visible during real work.

### Footer

The footer becomes a single block titled `常用快捷键`.

It no longer shows:

- bottom task progress
- bottom task status summary

Instead, it shows only the most useful shortcuts for the active context.

## Shortcut Content

### Cache Cleanup Page

Show these seven shortcuts:

- `↑↓ 选择`
- `Space 勾选`
- `a 全选`
- `r 重扫`
- `d 删除`
- `? 帮助`
- `q 退出`

### Space Explorer Page

Show these seven shortcuts:

- `↑↓ 选择`
- `Enter 进入`
- `Backspace 返回`
- `/ 过滤`
- `o 打开目录`
- `r 重扫`
- `? 帮助`

`q 退出` is intentionally omitted here to keep the footer visually tight. Exit remains discoverable through help and prior terminal conventions.

### Filtering Mode

When filtering mode is active, the footer temporarily replaces normal page shortcuts with filtering-specific guidance:

- `输入关键字`
- `Enter 应用`
- `Esc 取消`
- `Backspace 删除`

This mode-specific swap is more helpful than mixing generic navigation hints with input-mode instructions.

## Visual Style

The redesign should stay aligned with the current TUI look:

- rounded borders remain unchanged
- current color palette remains unchanged or only minimally extended
- the task card should feel slightly stronger than plain status text, but not like a modal or warning box
- footer shortcuts should read like compact labeled chips or grouped hints, not as a long sentence

The key visual change is hierarchy, not a new theme.

## Implementation Plan Shape

The intended implementation scope is primarily `src/ui.rs`.

Expected rendering changes:

- update root vertical layout so the footer uses a single-row region
- refactor `render_header` to support a nested layout in the status panel
- move task-progress rendering logic from `render_footer` into new header-side helpers
- simplify `render_footer` to render shortcut content only
- extract small helper functions for shortcut text generation and task card rendering to keep functions readable

No application-state model changes are expected unless a tiny helper accessor is needed for cleaner rendering.

## Edge Cases

- If a task exists with a long message, the header should prioritize keeping the task card readable and wrap or trim status text as needed
- If the terminal is narrow, the layout should degrade gracefully without panicking or overflowing in an obviously broken way
- If filtering mode is active while a task is running, the footer should still prioritize filtering instructions while the header keeps showing task progress

## Testing Strategy

Verification for this change should focus on regression safety rather than deep UI automation:

- run `cargo fmt`
- run `cargo test`

If compilation or warnings suggest layout-related issues, also run:

- `cargo clippy --all-targets -- -D warnings`

The primary success criteria are:

- project builds and tests pass
- no existing interaction behavior changes outside presentation
- progress is now rendered in the header area
- footer now shows current-page shortcut hints only

## Risks

- Header width is more constrained than the previous footer progress area, so task text may need careful trimming or wrapping
- A footer made of very long shortcut text can still become noisy if rendered as one sentence, so helper formatting should keep it visually compact
- Narrow terminals may expose layout pressure earlier than before, especially when task details and long status messages appear together

## Accepted Scope Boundary

This spec covers only the approved UI restructure for header progress visibility and shortcut-focused footer guidance. Any future redesign of help content, full-page onboarding, richer task telemetry, or adaptive layouts for very narrow terminals belongs in a separate change.
