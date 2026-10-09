//! Busy-row labels come from live tool operations and public commentary, never raw reasoning.
//! Parallel operations retain their own identity; finishing one restores the remaining action.

use super::*;
use codex_app_server_protocol::CommandAction;
use codex_protocol::models::MessagePhase;

#[derive(Debug, Default)]
pub(super) struct LiveActivity {
    progress: Option<String>,
    displayed_header: Option<String>,
    running: Vec<(String, String)>,
}

fn concise(text: &str) -> Option<String> {
    let text = text
        .lines()
        .find(|line| !line.trim().is_empty())?
        .trim()
        .trim_start_matches('#')
        .trim()
        .replace("**", "");
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut characters = text.chars().filter(|character| !character.is_control());
    let mut result = characters.by_ref().take(120).collect::<String>();
    if characters.next().is_some() {
        result.push('…');
    }
    (!result.is_empty()).then_some(result)
}

fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

fn command_label(command: &str, actions: &[CommandAction]) -> String {
    for action in actions {
        match action {
            CommandAction::Read { name, .. } => return format!("Читаю: {}", file_name(name)),
            CommandAction::Search { .. } => return "Ищу по файлам".to_string(),
            CommandAction::ListFiles { .. } => return "Смотрю список файлов".to_string(),
            CommandAction::Unknown { .. } => {}
        }
    }
    let lowercase = command.to_lowercase();
    if lowercase.contains("cargo test")
        || lowercase.contains("gradlew test")
        || lowercase.contains("npm test")
    {
        "Проверяю: тесты".to_string()
    } else if lowercase.contains("cargo build")
        || lowercase.contains("gradlew build")
        || lowercase.contains("npm run build")
    {
        "Собираю проект".to_string()
    } else {
        let executable = command
            .split_whitespace()
            .next()
            .unwrap_or("команда")
            .trim_matches(['\"', '\'']);
        format!("Запускаю: {}", file_name(executable))
    }
}

impl LiveActivity {
    pub(super) fn dismiss_header(&mut self) {
        self.displayed_header = None;
    }

    pub(super) fn clear(&mut self) {
        self.progress = None;
        self.displayed_header = None;
        self.running.clear();
    }

    fn start(&mut self, id: &str, label: String) {
        self.running.retain(|(active_id, _)| active_id != id);
        if let Some(label) = concise(&label) {
            self.running.push((id.to_string(), label));
        }
    }

    fn tool_label(&self, tool: &str, arguments: &serde_json::Value) -> String {
        arguments
            .get("title")
            .and_then(serde_json::Value::as_str)
            .and_then(concise)
            .map(|title| format!("Делаю: {title}"))
            .or_else(|| self.progress.clone())
            .unwrap_or_else(|| format!("Инструмент: {tool}"))
    }

    pub(super) fn started(&mut self, item: &ThreadItem) {
        let label = match item {
            ThreadItem::CommandExecution {
                command,
                command_actions,
                ..
            } => Some(command_label(command, command_actions)),
            ThreadItem::FileChange { changes, .. } => Some(match changes.as_slice() {
                [change] => format!("Правлю: {}", file_name(&change.path)),
                changes => format!("Правлю файлы: {}", changes.len()),
            }),
            ThreadItem::McpToolCall {
                tool, arguments, ..
            }
            | ThreadItem::DynamicToolCall {
                tool, arguments, ..
            } => Some(self.tool_label(tool, arguments)),
            ThreadItem::WebSearch(_) => Some("Ищу в интернете".to_string()),
            ThreadItem::ImageGeneration(_) => Some("Создаю изображение".to_string()),
            _ => None,
        };
        if let Some(label) = label {
            self.start(item.id(), label);
        }
    }

    pub(super) fn completed(&mut self, item: &ThreadItem) {
        self.running.retain(|(id, _)| id != item.id());
        if let ThreadItem::AgentMessage {
            text,
            phase: Some(MessagePhase::Commentary),
            ..
        } = item
        {
            self.progress = concise(text);
        }
    }

    pub(super) fn label(&self) -> Option<String> {
        self.running
            .last()
            .map(|(_, label)| label.clone())
            .or_else(|| self.progress.clone())
    }
}

impl ChatWidget {
    pub(super) fn refresh_live_activity_status(&mut self) {
        if !self.custom_live_activity_enabled()
            || !self.bottom_pane.is_task_running()
            || self.mcp_startup_status.is_some()
            || self.status_state.compaction.is_some()
            || self.status_state.retry_status_header.is_some()
            || !self.status_state.pending_guardian_review_status.is_empty()
            || self.safety_buffering_is_waiting()
            || self.unified_exec_wait_streak.is_some()
        {
            return;
        }
        if let Some(header) = self.status_state.live_activity.label() {
            self.status_state.live_activity.displayed_header = Some(header.clone());
            self.set_status_header(header);
        } else if self
            .status_state
            .live_activity
            .displayed_header
            .take()
            .as_deref()
            == Some(self.status_state.current_status.header.as_str())
        {
            self.restore_reasoning_status_header();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parallel_activity_completion_restores_the_remaining_operation() {
        let mut state = LiveActivity {
            progress: Some("Проверяю: сборку".into()),
            ..Default::default()
        };
        state.start("read", "Читаю: app.rs".into());
        state.start("search", "Ищу по файлам".into());
        assert_eq!(state.label().as_deref(), Some("Ищу по файлам"));
        state.running.retain(|(id, _)| id != "search");
        assert_eq!(state.label().as_deref(), Some("Читаю: app.rs"));
        state.running.clear();
        assert_eq!(state.label().as_deref(), Some("Проверяю: сборку"));
        state.clear();
        assert_eq!(state.label(), None);
    }

    #[test]
    fn command_activity_uses_safe_actions_without_printing_command_arguments() {
        assert_eq!(
            command_label("cargo test -p codex-tui", &[]),
            "Проверяю: тесты"
        );
        assert_eq!(
            command_label("cargo build --release", &[]),
            "Собираю проект"
        );
        assert_eq!(
            command_label("curl -H secret https://example.test", &[]),
            "Запускаю: curl"
        );
        assert_eq!(
            command_label(
                "ignored",
                &[CommandAction::Read {
                    command: "private arguments".into(),
                    name: "C:\\project\\app.rs".into(),
                    path: codex_utils_path_uri::LegacyAppPathString::from_string("app.rs"),
                }]
            ),
            "Читаю: app.rs"
        );
    }

    #[test]
    fn public_progress_and_tool_titles_remain_bounded_and_single_line() {
        assert_eq!(
            concise("**Правлю:** файл\nподробности").as_deref(),
            Some("Правлю: файл")
        );
        assert_eq!(concise(&"я".repeat(200)).unwrap().chars().count(), 121);
        let state = LiveActivity {
            progress: Some("Смотрю: API".into()),
            ..Default::default()
        };
        assert_eq!(
            state.tool_label("execute", &json!({"title":"Чтение файла"})),
            "Делаю: Чтение файла"
        );
        assert_eq!(
            state.tool_label("execute", &json!({"api_key":"secret"})),
            "Смотрю: API"
        );
    }

    #[test]
    fn final_answers_do_not_replace_the_public_progress_label() {
        let mut state = LiveActivity::default();
        for (phase, text) in [
            (MessagePhase::Commentary, "Проверяю: результат"),
            (MessagePhase::FinalAnswer, "Готово"),
        ] {
            state.completed(&ThreadItem::AgentMessage {
                id: "message".into(),
                text: text.into(),
                phase: Some(phase),
                memory_citation: None,
                delivery: None,
                questions: None,
            });
        }
        assert_eq!(state.label().as_deref(), Some("Проверяю: результат"));
    }

    #[test]
    fn tool_events_update_and_release_their_status() {
        use codex_app_server_protocol::DynamicToolCallStatus;
        let mut state = LiveActivity::default();
        let item = ThreadItem::DynamicToolCall {
            id: "call".into(),
            namespace: None,
            tool: "inspect".into(),
            arguments: json!({"title":"Свойства элемента"}),
            status: DynamicToolCallStatus::InProgress,
            content_items: None,
            success: None,
            duration_ms: None,
        };
        state.started(&item);
        assert_eq!(state.label().as_deref(), Some("Делаю: Свойства элемента"));
        state.completed(&item);
        assert_eq!(state.label(), None);
    }
}
