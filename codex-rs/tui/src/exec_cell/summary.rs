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

pub(super) fn action(item: &ParsedCommand) -> (&'static str, String) {
    match item {
        ParsedCommand::Read { path, .. } => ("Read", short_path(&path.to_string_lossy())),
        ParsedCommand::ListFiles { path, .. } => {
            ("List", path.clone().unwrap_or_else(|| ".".to_owned()))
        }
        ParsedCommand::Search { query, path, cmd } => {
            let detail = match (query, path) {
                (Some(query), Some(path)) => format!("{query} in {path}"),
                (Some(query), None) => query.clone(),
                _ => cmd.clone(),
            };
            ("Search", detail)
        }
        ParsedCommand::Unknown { cmd } => {
            let fallback = || ("Run", cmd.lines().next().unwrap_or(cmd).to_owned());
            let Some(mut tokens) = shlex::split(cmd) else {
                return fallback();
            };
            if tokens.first().is_some_and(|token| token == "&") {
                tokens.remove(0);
            }
            if cmd.lines().nth(1).is_some()
                || tokens.iter().any(|token| token.contains([';', '|', '&']))
            {
                return fallback();
            }
            let Some(executable) = tokens.first() else {
                return fallback();
            };
            let name = executable
                .rsplit(['/', '\\'])
                .next()
                .unwrap_or(executable)
                .to_ascii_lowercase();
            let name = name.trim_end_matches(".exe").to_owned();
            let title = match (name.as_str(), tokens.get(1).map(String::as_str)) {
                ("git", Some("diff")) if tokens.iter().any(|token| token == "--check") => "Check",
                ("cargo", Some("fmt")) => "Format",
                ("cargo", Some("test")) | ("npm" | "pnpm", Some("test")) => "Test",
                ("cargo", Some("check" | "clippy")) => "Check",
                ("cargo", Some("build")) => "Build",
                _ => return fallback(),
            };
            tokens[0] = name;
            (title, shlex_join(&tokens))
        }
    }
}
