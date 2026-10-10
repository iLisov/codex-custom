//! Compact actions with the full command and output retained for local disclosure.

use super::model::ExecCall;
use super::model::ExecCell;
use super::summary::compact_action;
use super::summary::compact_path;
use crate::exec_command::strip_bash_lc_and_escape;
use crate::history_cell::ActivityDisclosure;
use crate::history_cell::HistoryRenderMode;
use crate::history_cell::activity_preview::activity_header;
use crate::motion::MotionMode;
use crate::motion::ReducedMotionIndicator;
use crate::motion::activity_indicator;
use crate::terminal_hyperlinks::HyperlinkLine;
use codex_protocol::parse_command::ParsedCommand;
use itertools::Itertools;
use ratatui::style::Stylize;

impl ExecCell {
    pub(super) fn command_disclosure(&self, width: u16) -> Option<ActivityDisclosure> {
        let hidden_output_lines = self
            .group
            .calls
            .iter()
            .filter(|call| !call.is_unified_exec_interaction())
            .filter_map(|call| call.output.as_ref())
            .map(|output| output.line_counts().1)
            .sum::<usize>();
        if hidden_output_lines > 0 {
            return Some(ActivityDisclosure::OutputLines(hidden_output_lines));
        }
        if !self
            .group
            .details
            .lines_after(1, width, HistoryRenderMode::Rich)
            .is_empty()
        {
            return Some(ActivityDisclosure::Generic);
        }
        let command_is_hidden = self.group.calls.iter().any(|call| {
            let script = strip_bash_lc_and_escape(&call.command);
            let (_, detail) = self.compact_summary(call);
            script != detail || script.lines().nth(1).is_some()
        });
        let compact_rows = if self.is_exploring_cell() {
            self.compact_exploration_lines(u16::MAX)
        } else {
            self.compact_command_lines(u16::MAX)
        };
        (command_is_hidden
            || compact_rows
                .iter()
                .any(|header| header.width() > usize::from(width)))
        .then_some(ActivityDisclosure::Generic)
    }

    pub(super) fn compact_exploration_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        let mut lines = Vec::new();
        let mut calls = self.group.calls.as_slice();
        while let Some((call, remaining)) = calls.split_first() {
            let goal = self.compact_group_goal(call);
            let count = if let Some(goal) = goal {
                1 + remaining
                    .iter()
                    .take_while(|next| {
                        self.compact_group_goal(next)
                            .is_some_and(|next_goal| next_goal == goal)
                    })
                    .count()
            } else {
                1
            };
            let (group, rest) = calls.split_at(count);
            calls = rest;
            let actions = group
                .iter()
                .flat_map(|call| self.compact_actions(call))
                .collect::<Vec<_>>();
            let title = goal.unwrap_or_else(|| self.compact_summary(call).0);
            let paths = actions
                .iter()
                .map(|(_, path)| path)
                .filter(|path| !path.is_empty())
                .unique()
                .collect::<Vec<_>>();
            let detail = if paths.is_empty() {
                self.compact_working_directory()
            } else {
                let shown = paths.iter().take(2).copied().join(", ");
                if paths.len() > 2 {
                    format!("{shown} +{}", paths.len() - 2)
                } else {
                    shown
                }
            };
            let active = group.iter().any(|call| call.duration.is_none());
            let mut suffix = Vec::new();
            if actions.len() > 1 {
                suffix.push(action_count_suffix(actions.len()).dim());
            }
            suffix.extend(self.call_status(call, active));
            lines.push(activity_header(
                self.call_marker(call, active),
                title,
                &detail,
                suffix,
                width,
            ));
        }
        lines
    }

    pub(super) fn compact_command_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        let [call] = self.group.calls.as_slice() else {
            return Vec::new();
        };
        if call.is_unified_exec_interaction()
            && !self.is_active()
            && call.output.as_ref().is_some_and(|output| {
                output.exit_code == 0 && output.lines().all(|line| line.trim().is_empty())
            })
        {
            return Vec::new();
        }
        let (title, detail) = self.compact_summary(call);
        let active = self.is_active();
        let lines = vec![activity_header(
            self.call_marker(call, active),
            title,
            &detail,
            self.call_status(call, active),
            width,
        )];
        lines
    }

    fn compact_summary(&self, call: &ExecCall) -> (&'static str, String) {
        if call.is_unified_exec_interaction() {
            return ("Терминал", self.compact_working_directory());
        }

        let actions = self.compact_actions(call);
        let has_unknown_action = call
            .parsed
            .iter()
            .any(|action| matches!(action, ParsedCommand::Unknown { .. }));
        if !actions.is_empty() && !has_unknown_action {
            let title = actions
                .iter()
                .all(|(other, _)| *other == actions[0].0)
                .then_some(actions[0].0)
                .unwrap_or("Обзор");
            let paths = actions
                .into_iter()
                .map(|(_, path)| path)
                .filter(|path| !path.is_empty())
                .unique()
                .collect::<Vec<_>>();
            let detail = if paths.is_empty() {
                self.compact_working_directory()
            } else {
                paths.join(" · ")
            };
            return (title, detail);
        }

        let title = compact_goal(&call.command);
        let detail = compact_command_path(&call.command, self.working_directory.as_deref())
            .or_else(|| {
                self.working_directory_hint()
                    .map(|path| compact_path(&path, None))
            })
            .unwrap_or_else(|| ".".to_owned());
        (title, detail)
    }

    fn compact_actions(&self, call: &ExecCall) -> Vec<(&'static str, String)> {
        call.parsed
            .iter()
            .filter_map(compact_action)
            .map(|(title, path)| {
                (
                    title,
                    if path.is_empty() {
                        self.compact_working_directory()
                    } else {
                        compact_path(&path, self.working_directory.as_deref())
                    },
                )
            })
            .collect()
    }

    fn compact_group_goal(&self, call: &ExecCall) -> Option<&'static str> {
        if !Self::is_exploring_call(call)
            || call
                .output
                .as_ref()
                .is_some_and(|output| output.exit_code != 0)
        {
            return None;
        }
        let actions = self.compact_actions(call);
        let first = actions.first()?.0;
        Some(if actions.iter().all(|(goal, _)| *goal == first) {
            first
        } else {
            "Обзор"
        })
    }

    fn working_directory_hint(&self) -> Option<String> {
        self.working_directory.clone()
    }

    fn compact_working_directory(&self) -> String {
        self.working_directory_hint()
            .map(|path| shorten_path(&path))
            .unwrap_or_else(|| ".".to_owned())
    }

    fn call_marker(&self, call: &ExecCall, active: bool) -> ratatui::text::Span<'static> {
        if active {
            activity_indicator(
                self.active_start_time(),
                MotionMode::from_animations_enabled(self.animations_enabled()),
                ReducedMotionIndicator::StaticBullet,
            )
            .unwrap_or_else(|| "●".dim())
        } else {
            match call.output.as_ref().map(|output| output.exit_code) {
                Some(1)
                    if call
                        .parsed
                        .iter()
                        .any(|item| matches!(item, ParsedCommand::Search { .. })) =>
                {
                    "●".dim()
                }
                Some(code) if code != 0 => "●".red(),
                Some(0) => "●".green(),
                _ => "●".dim(),
            }
        }
    }

    fn call_status(&self, call: &ExecCall, active: bool) -> Vec<ratatui::text::Span<'static>> {
        if active {
            return vec!["  · выполняется".dim()];
        }
        match call.output.as_ref().map(|output| output.exit_code) {
            Some(0) => vec!["  ✓".green()],
            Some(code)
                if code == 1
                    && call
                        .parsed
                        .iter()
                        .any(|item| matches!(item, ParsedCommand::Search { .. })) =>
            {
                vec!["  · совпадений нет".dim()]
            }
            Some(code) => vec![format!("  ✗ ошибка · код {code}").red()],
            None => Vec::new(),
        }
    }
}

fn compact_goal(command: &[String]) -> &'static str {
    let text = command.join(" ").to_ascii_lowercase();
    let words = text.split_whitespace().collect::<Vec<_>>();
    if [
        "remove-item",
        "set-content",
        "out-file",
        "move-item",
        "copy-item",
    ]
    .iter()
    .any(|verb| text.contains(verb))
        || text.contains("git reset")
    {
        "Изменение"
    } else if text.contains("list-tests") {
        "Список тестов"
    } else if text.contains("git grep")
        || text.contains("select-string")
        || words.contains(&"grep")
        || words.contains(&"rg")
    {
        "Поиск"
    } else if text.contains("get-content") || text.contains(" cat ") {
        "Чтение"
    } else if text.contains("get-childitem") && text.contains("-filter") {
        "Найти"
    } else if text.contains("get-childitem") || text.contains("get-item") {
        "Список"
    } else if text.contains("cargo test")
        || text.contains("npm test")
        || text.contains("pnpm test")
        || words.contains(&"test")
    {
        "Тесты"
    } else if text.contains("cargo check")
        || text.contains("cargo clippy")
        || text.contains("git diff --check")
        || words.contains(&"check")
        || words.contains(&"clippy")
    {
        "Проверка"
    } else if text.contains("cargo build") || words.contains(&"build") {
        "Сборка"
    } else if text.contains("cargo fmt") || words.contains(&"fmt") {
        "Форматирование"
    } else {
        "Команда"
    }
}

fn compact_command_path(command: &[String], working_directory: Option<&str>) -> Option<String> {
    let start = command
        .first()
        .map(|shell| {
            shell
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(shell)
                .to_ascii_lowercase()
        })
        .filter(|shell| {
            shell.contains("powershell") || shell == "pwsh.exe" || shell == "bash" || shell == "sh"
        })
        .and_then(|_| {
            command
                .iter()
                .position(|part| part.eq_ignore_ascii_case("-command") || part == "-c")
                .map(|index| index + 1)
        })
        .unwrap_or(0);
    let mut paths = command
        .iter()
        .skip(start)
        .flat_map(|part| part.split_whitespace())
        .filter_map(|token| {
            let token = token
                .split_once('=')
                .map_or(token, |(_, value)| value)
                .trim_matches(|ch: char| matches!(ch, '\'' | '"' | ';' | ',' | '(' | ')'));
            (token.contains(['/', '\\'])
                && !token.starts_with('$')
                && !token.contains("://")
                && !token.to_ascii_lowercase().ends_with(".exe"))
            .then(|| {
                token
                    .strip_prefix("./")
                    .or_else(|| token.strip_prefix(".\\"))
                    .unwrap_or(token)
                    .to_owned()
            })
        })
        .map(|path| compact_path(&path, working_directory))
        .unique()
        .take(3)
        .collect::<Vec<_>>();
    if paths.len() > 1 {
        paths.retain(|path| path != ".");
    }
    (!paths.is_empty()).then(|| paths.join(" · "))
}

fn shorten_path(path: &str) -> String {
    if path.len() <= 72 && !path.contains(':') {
        path.to_owned()
    } else {
        crate::history_cell::activity_preview::short_path(path)
    }
}

fn action_count_suffix(count: usize) -> String {
    let last_two = count % 100;
    let noun = if (11..=14).contains(&last_two) {
        "действий"
    } else {
        match count % 10 {
            2..=4 => "действия",
            _ => "действий",
        }
    };
    format!(" · {count} {noun}")
}

#[cfg(test)]
#[path = "compact_tests.rs"]
mod tests;
