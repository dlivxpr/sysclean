# Header/Footer Progress Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move task progress into the header's top-right status area and replace the bottom status row with current-page shortcut hints.

**Architecture:** Keep behavior changes scoped to rendering. Add small UI helper functions for task-card content and footer shortcut text, then adjust layout composition in `src/ui.rs` so header and footer responsibilities are clearly separated.

**Tech Stack:** Rust, ratatui, cargo test

---

### Task 1: Add UI helper tests first

**Files:**
- Modify: `src/ui.rs`

- [ ] **Step 1: Write the failing tests**

Add unit tests in `src/ui.rs` for:

```rust
#[cfg(test)]
mod tests {
    use super::{footer_shortcuts, header_status_text, InputMode};
    use crate::app::{App, Page};
    use crate::models::BackgroundTaskStatus;

    #[test]
    fn footer_shortcuts_match_cache_cleanup_page() {
        let app = App::default();
        assert_eq!(
            footer_shortcuts(&app, InputMode::Normal),
            "↑↓ 选择   Space 勾选   a 全选   r 重扫   d 删除   ? 帮助   q 退出"
        );
    }

    #[test]
    fn footer_shortcuts_match_filtering_mode() {
        let mut app = App::default();
        app.next_page();
        assert_eq!(
            footer_shortcuts(&app, InputMode::Filtering),
            "输入关键字   Enter 应用   Esc 取消   Backspace 删除"
        );
    }

    #[test]
    fn header_status_text_prefers_task_summary_when_task_exists() {
        let mut app = App::default();
        app.status_message = "按 ? 查看帮助".into();
        app.task_status = Some(BackgroundTaskStatus::new("目录扫描", "正在计算大小", true));

        assert_eq!(header_status_text(&app), "目录扫描 · 正在计算大小");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test ui::tests --lib`
Expected: FAIL because `footer_shortcuts` and `header_status_text` do not exist yet.

- [ ] **Step 3: Write minimal implementation**

Add helper functions in `src/ui.rs`:

```rust
fn header_status_text(app: &App) -> String { ... }

fn footer_shortcuts(app: &App, input_mode: InputMode) -> &'static str { ... }
```

Keep the implementation simple and make it satisfy only the tested strings first.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test ui::tests --lib`
Expected: PASS

### Task 2: Move progress rendering into the header

**Files:**
- Modify: `src/ui.rs`

- [ ] **Step 1: Write the failing test**

Extend `src/ui.rs` tests with:

```rust
#[test]
fn task_card_supporting_text_prefers_progress_label() {
    let mut status = BackgroundTaskStatus::new("目录扫描", "正在计算大小", true);
    status.progress_label = Some("已完成 3/6".into());
    assert_eq!(task_card_supporting_text(&status), "已完成 3/6");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test ui::tests --lib`
Expected: FAIL because `task_card_supporting_text` does not exist yet.

- [ ] **Step 3: Write minimal implementation**

Add helper functions for the header task card:

```rust
fn task_card_supporting_text(task: &BackgroundTaskStatus) -> String { ... }
fn render_task_card(frame: &mut Frame, area: Rect, task: &BackgroundTaskStatus) { ... }
```

Then refactor `render_header` so it:

- keeps tabs on the left
- renders the status panel on the right
- shows a fixed-width task card when a task exists
- shows a progress bar in that card only when `progress_ratio()` returns `Some`

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test ui::tests --lib`
Expected: PASS

### Task 3: Simplify footer to shortcut-only layout

**Files:**
- Modify: `src/ui.rs`

- [ ] **Step 1: Write the failing test**

Add:

```rust
#[test]
fn footer_shortcuts_match_space_explorer_page() {
    let mut app = App::default();
    app.next_page();
    assert_eq!(
        footer_shortcuts(&app, InputMode::Normal),
        "↑↓ 选择   Enter 进入   Backspace 返回   / 过滤   o 打开目录   r 重扫   ? 帮助"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test ui::tests --lib`
Expected: FAIL until the space explorer shortcut set is implemented.

- [ ] **Step 3: Write minimal implementation**

Update `render` and `render_footer` in `src/ui.rs` so:

- the root layout uses a single footer row instead of a two-row footer block
- `render_footer` only renders the `常用快捷键` block
- the footer text comes entirely from `footer_shortcuts`

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test ui::tests --lib`
Expected: PASS

### Task 4: Verify full project

**Files:**
- Modify: `src/ui.rs`

- [ ] **Step 1: Format code**

Run: `cargo fmt`
Expected: no output

- [ ] **Step 2: Run test suite**

Run: `cargo test`
Expected: PASS

- [ ] **Step 3: Run lint checks**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: PASS
