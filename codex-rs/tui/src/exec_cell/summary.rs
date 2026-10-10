//! Presentation metadata only; the original command remains available in the transcript.

use codex_protocol::parse_command::ParsedCommand;
use codex_shell_command::parse_command::parse_command;
use codex_shell_command::parse_command::shlex_join;
use codex_shell_command::powershell::parse_powershell_command_into_plain_commands;

use crate::history_cell::activity_preview::short_path;

pub(super) fn display_commands(
    command: &[String],
    parsed: Vec<ParsedCommand>,
) -> Vec<ParsedCommand> {
    if parsed.is_empty()
        || !parsed
            .iter()
            .any(|item| matches!(item, ParsedCommand::Unknown { .. }))
    {
        return parsed;
    }
    let Some(commands) = parse_powershell_command_into_plain_commands(command) else {
        return parsed;
    };
    let mut summaries = Vec::new();
    for tokens in commands {
        let Some(executable) = tokens.first() else {
            return parsed;
        };
        if executable.eq_ignore_ascii_case("Select-Object") {
            if !matches!(summaries.last(), Some(ParsedCommand::Read { .. }))
                || !tokens[1..].iter().all(|token| {
                    ["-Skip", "-First", "-Last"]
                        .iter()
                        .any(|flag| token.eq_ignore_ascii_case(flag))
                        || token.parse::<usize>().is_ok()
                })
            {
                return parsed;
            }
            continue;
        }
        let normalized = if executable.eq_ignore_ascii_case("Get-Content") {
            let mut normalized = vec!["Get-Content".to_owned()];
            let mut args = tokens[1..].iter();
            while let Some(argument) = args.next() {
                if ["-Encoding", "-TotalCount", "-Tail", "-ReadCount"]
                    .iter()
                    .any(|flag| argument.eq_ignore_ascii_case(flag))
                {
                    if args.next().is_none() {
                        return parsed;
                    }
                } else {
                    normalized.push(argument.clone());
                }
            }
            normalized
        } else {
            tokens.clone()
        };
        let mut summary = parse_command(&normalized);
        for item in &mut summary {
            if let ParsedCommand::Read { cmd, .. } = item {
                *cmd = shlex_join(&tokens);
            }
        }
        summaries.extend(summary);
    }
    if summaries.is_empty() {
        parsed
    } else {
        summaries
    }
}

pub(super) fn compact_action(item: &ParsedCommand) -> Option<(&'static str, String)> {
    match item {
        ParsedCommand::Read { path, .. } => Some(("Чтение", path.to_string_lossy().into_owned())),
        ParsedCommand::ListFiles { path, .. } => Some(("Список", path.clone().unwrap_or_default())),
        ParsedCommand::Search { path, .. } => Some(("Поиск", path.clone().unwrap_or_default())),
        ParsedCommand::Unknown { .. } => None,
    }
}

pub(super) fn compact_path(path: &str, working_directory: Option<&str>) -> String {
    let path = path.replace('\\', "/");
    let path = path.trim_start_matches("./");
    if let Some(working_directory) = working_directory {
        let working_directory = working_directory.replace('\\', "/");
        let working_directory = working_directory.trim_end_matches('/');
        let normalized = path.replace('\\', "/");
        if let Some(prefix) = normalized.get(..working_directory.len())
            && prefix.eq_ignore_ascii_case(working_directory)
        {
            let rest = normalized
                .get(working_directory.len()..)
                .unwrap_or_default();
            if rest.is_empty() {
                return ".".to_owned();
            }
            if let Some(relative) = rest.strip_prefix('/') {
                return if relative.len() > 72 {
                    short_path(relative)
                } else {
                    relative.to_owned()
                };
            }
        }
    }
    if path.contains(':') || path.starts_with('/') || path.len() > 72 {
        short_path(path)
    } else {
        path.to_owned()
    }
}
