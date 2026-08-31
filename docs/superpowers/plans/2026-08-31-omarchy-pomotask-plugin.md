# Implementation Plan: Omarchy Quattro Plugin & IPC Engine for PomoTask

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a native Omarchy Quattro bar widget, popout panel, Hyprland distraction monitor, and break locking integration backed by a robust CLI/IPC interface in the existing Rust `pomotask-cli` binary.

**Architecture:** A hybrid system where `pomotask-cli` exposes high-performance, non-blocking CLI/IPC subcommands (`ipc status`, `ipc timer`, `ipc tasks`, `ipc task complete/create/focus`, `ipc blocklist`) and synchronizes shared runtime state (`~/.config/pomotask/runtime_state.json`), while a native QML/Quickshell plugin (`io.github.pl402.pomotask`) renders the status bar, interactive task panel, distraction alerts, and break overlays.

**Tech Stack:** Rust (Tokio, Serde, Chrono, Clap/Lexopt or standard args), QML (QtQuick, Quickshell, Hyprland IPC, `qs.Commons`, `qs.Ui`).

**Spec:** `docs/superpowers/specs/2026-08-31-omarchy-pomotask-plugin-design.md`

## Global Constraints
* Plugin ID: `io.github.pl402.pomotask`
* Target platform: Linux (Hyprland / Omarchy Quattro / Quickshell)
* Config and cache paths: `~/.config/pomotask/`
* Language conventions: Comments and user messages in Spanish; code identifiers in idiomatic English/Rust/QML.
* Non-blocking UI: QML interacts with Rust engine via async `Process` and file state; UI never freezes.

---

### Task 1: Data Models & Runtime State Persistence in Rust

**Files:**
- Create: `src/ipc.rs`
- Modify: `src/app/mod.rs`
- Modify: `src/main.rs`
- Test: `src/ipc.rs` (unit tests)

**Interfaces:**
- Produces: `RuntimeState`, `BlocklistConfig`, `IpcCommand`, `handle_ipc_command()`

- [ ] **Step 1: Write unit tests for `RuntimeState` and `BlocklistConfig` serialization**

Add unit tests in `src/ipc.rs` verifying JSON serialization/deserialization, state updates, timer progression, and blocklist matching.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_state_serialization() {
        let state = RuntimeState {
            state: "running".to_string(),
            mode: "work".to_string(),
            remaining_seconds: 1500,
            total_seconds: 1500,
            session_pomodoros: 2,
            active_task_id: Some("task_123".to_string()),
            active_task_title: Some("Implement IPC".to_string()),
            strict_break: false,
            anti_distraction: true,
        };
        let json = serde_json::to_string(&state).expect("serialize");
        let decoded: RuntimeState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.state, "running");
        assert_eq!(decoded.remaining_seconds, 1500);
        assert_eq!(decoded.active_task_title.as_deref(), Some("Implement IPC"));
    }

    #[test]
    fn test_blocklist_matching() {
        let mut blocklist = BlocklistConfig::default();
        blocklist.title_keywords.push("facebook".to_string());
        blocklist.blocked_classes.push("steam".to_string());

        assert!(blocklist.is_distraction("Facebook - Log In", "firefox"));
        assert!(blocklist.is_distraction("Any Title", "steam"));
        assert!(!blocklist.is_distraction("GitHub - Pull Requests", "zen"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --lib ipc::tests`
Expected: FAIL with module `ipc` not found.

- [ ] **Step 3: Implement `src/ipc.rs`**

Implement structs `RuntimeState`, `BlocklistConfig`, `IpcCommand`, file loading/saving to `~/.config/pomotask/runtime_state.json` and `~/.config/pomotask/blocklist.json`.

```rust
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeState {
    pub state: String, // "running", "paused", "stopped"
    pub mode: String,  // "work", "short_break", "long_break"
    pub remaining_seconds: u32,
    pub total_seconds: u32,
    pub session_pomodoros: u32,
    pub active_task_id: Option<String>,
    pub active_task_title: Option<String>,
    pub strict_break: bool,
    pub anti_distraction: bool,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            state: "stopped".to_string(),
            mode: "work".to_string(),
            remaining_seconds: 25 * 60,
            total_seconds: 25 * 60,
            session_pomodoros: 0,
            active_task_id: None,
            active_task_title: None,
            strict_break: false,
            anti_distraction: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlocklistConfig {
    pub title_keywords: Vec<String>,
    pub blocked_classes: Vec<String>,
    pub action: String, // "warn", "warn_and_unfocus", "minimize"
}

impl Default for BlocklistConfig {
    fn default() -> Self {
        Self {
            title_keywords: vec![
                "facebook".to_string(),
                "twitter".to_string(),
                "x.com".to_string(),
                "instagram".to_string(),
                "reddit".to_string(),
                "youtube.com".to_string(),
                "tiktok".to_string(),
                "netflix".to_string(),
            ],
            blocked_classes: vec![
                "steam".to_string(),
                "discord".to_string(),
            ],
            action: "warn".to_string(),
        }
    }
}

impl BlocklistConfig {
    pub fn is_distraction(&self, title: &str, app_class: &str) -> bool {
        let title_lower = title.to_lowercase();
        let class_lower = app_class.to_lowercase();

        for kw in &self.title_keywords {
            if title_lower.contains(&kw.to_lowercase()) {
                return true;
            }
        }
        for cls in &self.blocked_classes {
            if class_lower == cls.to_lowercase() {
                return true;
            }
        }
        false
    }
}
```

- [ ] **Step 4: Run tests and verify they pass**

Run: `cargo test --lib ipc`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ipc.rs src/main.rs
git commit -m "feat(ipc): add runtime state and blocklist models"
```

---

### Task 2: CLI / IPC Subcommands Implementation

**Files:**
- Modify: `src/ipc.rs`
- Modify: `src/main.rs`
- Test: `tests/ipc_cli_test.rs`

**Interfaces:**
- Consumes: `App`, `ApiClient`, `RuntimeState`, `BlocklistConfig`
- Produces: CLI handler `execute_ipc_command(args: &[String]) -> Result<String, String>`

- [ ] **Step 1: Write integration tests for CLI IPC commands**

Create `tests/ipc_cli_test.rs` to test `status`, `timer`, `blocklist`, and `tasks` commands.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test ipc_cli_test`
Expected: FAIL

- [ ] **Step 3: Implement IPC command execution logic in `src/ipc.rs` and route in `src/main.rs`**

Add support for CLI arguments:
`pomotask-cli ipc status`
`pomotask-cli ipc timer <start|pause|toggle|skip|reset>`
`pomotask-cli ipc tasks list [--list-id <id>]`
`pomotask-cli ipc task complete <id>`
`pomotask-cli ipc task create --title <text> [--list-id <id>]`
`pomotask-cli ipc task focus <id|clear>`
`pomotask-cli ipc blocklist get`
`pomotask-cli ipc blocklist toggle-strict`
`pomotask-cli ipc blocklist toggle-anti-distraction`

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test`
Expected: All unit and integration tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/ipc.rs src/main.rs tests/ipc_cli_test.rs
git commit -m "feat(ipc): implement CLI subcommands for Omarchy IPC"
```

---

### Task 3: Omarchy Plugin Scaffold & `manifest.json`

**Files:**
- Create: `plugins/io.github.pl402.pomotask/manifest.json`
- Create: `plugins/io.github.pl402.pomotask/README.md`
- Create: `plugins/io.github.pl402.pomotask/PomotaskService.qml`

**Interfaces:**
- Produces: Omarchy manifest compliant with schemaVersion 1 and entry points.

- [ ] **Step 1: Write `manifest.json`**

```json
{
  "schemaVersion": 1,
  "id": "io.github.pl402.pomotask",
  "name": "PomoTask",
  "version": "1.0.0",
  "author": "Elias",
  "license": "MIT",
  "description": "Pomodoro timer, Google Tasks manager, and distraction blocker for Omarchy Quattro.",
  "kinds": ["bar-widget"],
  "entryPoints": {
    "barWidget": "BarWidget.qml"
  },
  "barWidget": {
    "displayName": "PomoTask",
    "category": "Productivity",
    "allowMultiple": false,
    "defaultSection": "center"
  }
}
```

- [ ] **Step 2: Validate manifest with Omarchy CLI**

Run: `omarchy plugin validate plugins/io.github.pl402.pomotask`
Expected: PASS (or note missing QML entry points to be created in Task 4).

- [ ] **Step 3: Create `PomotaskService.qml`**

Create singleton service that runs `pomotask-cli ipc status` and watches state file updates.

- [ ] **Step 4: Commit**

```bash
git add plugins/io.github.pl402.pomotask/
git commit -m "feat(omarchy): scaffold plugin manifest and service"
```

---

### Task 4: `BarWidget.qml` (Bar Component)

**Files:**
- Create: `plugins/io.github.pl402.pomotask/BarWidget.qml`

**Interfaces:**
- Consumes: `PomotaskService.qml`, `qs.Ui.BarWidget`, `qs.Ui.WidgetButton`
- Produces: Live status bar item with interactive clicks.

- [ ] **Step 1: Implement `BarWidget.qml`**

Write `BarWidget.qml` supporting:
- Displaying phase icon (`🍅` work, `☕` short break, `🌴` long break, `⏸` paused)
- Formatted countdown `MM:SS`
- Active task snippet if present
- Clic izquierdo toggles `Panel.qml`
- Clic derecho/scroll toggles timer play/pause

- [ ] **Step 2: Check QML syntax with `qmllint`**

Run: `qmllint -I /usr/share/omarchy/shell plugins/io.github.pl402.pomotask/BarWidget.qml`
Expected: Valid QML without structural errors.

- [ ] **Step 3: Commit**

```bash
git add plugins/io.github.pl402.pomotask/BarWidget.qml
git commit -m "feat(omarchy): implement BarWidget component"
```

---

### Task 5: `Panel.qml` (Rich Interactive Popup)

**Files:**
- Create: `plugins/io.github.pl402.pomotask/Panel.qml`

**Interfaces:**
- Consumes: `qs.Ui.KeyboardPanel`, `qs.Ui.PanelKeyCatcher`, `qs.Commons.Style`, `qs.Commons.Color`
- Produces: Interactive popup with Timer hero, Google Tasks list, task creation, and settings.

- [ ] **Step 1: Implement `Panel.qml` layout & components**

Implement:
1. Hero Timer: Big digital display, progress indicator, Play/Pause/Skip/Reset buttons.
2. Google Tasks Section:
   - Task list dropdown
   - Scrollable `ListView` of tasks with checkbox completion and 🎯 focus button.
   - Quick input `TextField` for new task creation.
3. Quick Toggles: Anti-distraction mode & Strict Break mode toggles.

- [ ] **Step 2: Check QML syntax with `qmllint`**

Run: `qmllint -I /usr/share/omarchy/shell plugins/io.github.pl402.pomotask/Panel.qml`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add plugins/io.github.pl402.pomotask/Panel.qml
git commit -m "feat(omarchy): implement Panel component with Google Tasks and controls"
```

---

### Task 6: Hyprland Distraction Monitor & Strict Break Overlay

**Files:**
- Create: `plugins/io.github.pl402.pomotask/DistractionMonitor.qml`
- Create: `plugins/io.github.pl402.pomotask/BreakOverlay.qml`
- Modify: `plugins/io.github.pl402.pomotask/BarWidget.qml`

**Interfaces:**
- Consumes: `Quickshell.Io.Process`, `hyprctl activewindow -j`, `omarchy system lock`
- Produces: Real-time window title inspection, OSD warnings, and break screen lock.

- [ ] **Step 1: Implement `DistractionMonitor.qml`**

Polls/listens to Hyprland active window (`hyprctl activewindow -j`). During `work` sessions:
- Checks if window `title` or `class` matches rules in `blocklist.json`.
- Triggers notification via desktop notification / OSD.

- [ ] **Step 2: Implement `BreakOverlay.qml` & Lock Trigger**

When mode changes to `short_break` or `long_break`:
- If `strict_break` is enabled, triggers `omarchy system lock`.
- Shows motivational break message with time remaining.

- [ ] **Step 3: Check QML syntax with `qmllint`**

Run: `qmllint -I /usr/share/omarchy/shell plugins/io.github.pl402.pomotask/DistractionMonitor.qml plugins/io.github.pl402.pomotask/BreakOverlay.qml`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add plugins/io.github.pl402.pomotask/
git commit -m "feat(omarchy): implement distraction monitor and break lock overlay"
```

---

### Task 7: Plugin Installation Script & End-to-End Verification

**Files:**
- Create: `install-plugin.sh`
- Modify: `README.md`

- [ ] **Step 1: Create `install-plugin.sh`**

Script that:
1. Builds `pomotask-cli` in release mode (`cargo build --release`).
2. Installs binary to `~/.local/bin/pomotask-cli` (or symlinks).
3. Installs plugin to `~/.config/omarchy/plugins/io.github.pl402.pomotask/`.
4. Runs `omarchy plugin validate` and `omarchy-shell shell rescanPlugins`.

- [ ] **Step 2: Run End-to-End Validation**

1. Run `cargo test` to verify all Rust tests pass.
2. Run `omarchy plugin validate ~/.config/omarchy/plugins/io.github.pl402.pomotask`.
3. Test CLI command `cargo run -- ipc status` and verify valid JSON output.

- [ ] **Step 3: Commit**

```bash
git add install-plugin.sh README.md
git commit -m "feat: add plugin installation script and documentation"
```
