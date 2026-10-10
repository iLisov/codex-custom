//! Compact actions with the full command and output retained for local disclosure.

use super::model::ExecCall;
use super::model::ExecCell;
use super::summary::action;
use crate::exec_command::strip_bash_lc_and_escape;
use crate::history_cell::ActivityDisclosure;
use crate::history_cell::HistoryRenderMode;
use crate::history_cell::activity_preview::DETAIL_PREVIEW_LINES;
use crate::history_cell::activity_preview::activity_header;
use crate::history_cell::activity_preview::clipped_line;
use crate::motion::MotionMode;
use crate::motion::ReducedMotionIndicator;
use crate::motion::activity_indicator;
use crate::terminal_hyperlinks::HyperlinkLine;
use codex_ansi_escape::ansi_escape_line;
use codex_protocol::parse_command::ParsedCommand;
use itertools::Itertools;
use ratatui::style::Stylize;

impl ExecCell {
    pub(super) fn command_disclosure(&self, width: u16) -> Option<ActivityDisclosure> {
        let [call] = self.group.calls.as_slice() else {
            return None;
        };
        if !call.is_unified_exec_interaction()
            && let Some(output) = &call.output
        {
            let retained = output.line_counts().1;
            let shown = if self.show_output_preview(call) {
                DETAIL_PREVIEW_LINES
            } else {
                0
            };
            let clipped = Self::preview_rows(output, call.duration.is_none())
                .iter()
                .take(shown)
                .filter(|raw| ansi_escape_line(raw.as_ref()).width() + 4 > usize::from(width))
                .count();
            let hidden = retained.saturating_sub(shown) + clipped;
            if hidden > 0 {
                return Some(ActivityDisclosure::OutputLines(hidden));
            }
        }
        if !self
            .group
            .details
            .lines_after(1, width, HistoryRenderMode::Rich)
            .is_empty()
        {
            return Some(ActivityDisclosure::Generic);
        }
        let script = strip_bash_lc_and_escape(&call.command);
        let (_, detail) = self.call_summary(call);
        (script != detail
            || script.lines().nth(1).is_some()
            || self
                .compact_command_lines(u16::MAX)
                .first()
                .is_some_and(|header| header.width() > usize::from(width)))
        .then_some(ActivityDisclosure::Generic)
    }

    pub(super) fn compact_exploration_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        let mut lines = Vec::new();
        let mut calls = self.group.calls.as_slice();
        while let Some((call, remaining)) = calls.split_first() {
            let is_read = |call: &ExecCall| {
                !call.parsed.is_empty()
                    && call
                        .parsed
                        .iter()
                        .all(|item| matches!(item, ParsedCommand::Read { .. }))
                    && call
                        .output
                        .as_ref()
                        .is_none_or(|output| output.exit_code == 0)
            };
            let count = if is_read(call) {
                1 + remaining.iter().take_while(|next| is_read(next)).count()
            } else {
                1
            };
            let (group, rest) = calls.split_at(count);
            calls = rest;
            let (title, detail, sections) = if is_read(call) {
                let actions = group
                    .iter()
                    .flat_map(|call| &call.parsed)
                    .collect::<Vec<_>>();
                let detail = actions
                    .iter()
                    .map(|item| action(item).1)
                    .unique()
                    .join(", ");
                ("Read", detail, actions.len())
            } else {
                let (title, detail) = self.call_summary(call);
                (title, detail, 1)
            };
            let active = group.iter().any(|call| call.duration.is_none());
            let mut suffix = Vec::new();
            if sections > 1 {
                suffix.push(format!(" · {sections} reads").dim());
            }
            suffix.extend(self.call_status(call, active));
            lines.push(activity_header(
                self.call_marker(call, active),
                title,
                &detail,
                suffix,
                width,
            ));
            if self.show_output_preview(call)
                && call
                    .output
                    .as_ref()
                    .is_some_and(|output| output.exit_code != 0)
            {
                lines.extend(self.output_preview(call, width));
            }
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
        let (title, detail) = self.call_summary(call);
        let active = self.is_active();
        let mut lines = vec![activity_header(
            self.call_marker(call, active),
            title,
            &detail,
            self.call_status(call, active),
            width,
        )];
        if self.show_output_preview(call) {
            lines.extend(self.output_preview(call, width));
        }
        lines
    }

    fn call_summary(&self, call: &ExecCall) -> (&'static str, String) {
        let script_preview = || {
            let script = strip_bash_lc_and_escape(&call.command);
            let first = script.lines().next().unwrap_or_default();
            if script.lines().nth(1).is_some() {
                format!("{first} …")
            } else {
                script
            }
        };
        if call.is_unified_exec_interaction() {
            return ("Output", script_preview());
        }
        let actions = call.parsed.iter().map(action).collect::<Vec<_>>();
        let title = actions.first().map_or("Run", |(title, _)| *title);
        if actions.is_empty() {
            return (title, script_preview());
        }
        let same_title = actions.iter().all(|(other, _)| *other == title);
        let detail = actions
            .iter()
            .map(|(title, detail)| {
                if same_title {
                    detail.clone()
                } else {
                    format!("{title} {detail}")
                }
            })
            .join(" · ");
        (if same_title { title } else { "Run" }, detail)
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
            return Vec::new();
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
                vec![format!("  exit {code}").dim()]
            }
            Some(code) => vec![format!("  ✗ exit {code}").red()],
            None => Vec::new(),
        }
    }

    fn show_output_preview(&self, call: &ExecCall) -> bool {
        !call.is_unified_exec_interaction()
            && (call.duration.is_none()
                || call
                    .output
                    .as_ref()
                    .is_some_and(|output| output.exit_code != 0))
    }

    fn output_preview(&self, call: &ExecCall, width: u16) -> Vec<HyperlinkLine> {
        call.output.as_ref().map_or_else(Vec::new, |output| {
            Self::preview_rows(output, call.duration.is_none())
                .into_iter()
                .enumerate()
                .map(|(index, raw)| {
                    let mut line = ansi_escape_line(raw.as_ref());
                    line.spans
                        .insert(0, if index == 0 { "  └ " } else { "    " }.dim());
                    clipped_line(line, width)
                })
                .collect()
        })
    }

    fn preview_rows(
        output: &super::model::CommandOutput,
        active: bool,
    ) -> Vec<std::borrow::Cow<'_, str>> {
        if active {
            let mut rows = output
                .lines()
                .rev()
                .take(DETAIL_PREVIEW_LINES)
                .collect::<Vec<_>>();
            rows.reverse();
            rows
        } else {
            output.lines().take(DETAIL_PREVIEW_LINES).collect()
        }
    }
}

#[cfg(test)]
#[path = "compact_tests.rs"]
mod tests;
