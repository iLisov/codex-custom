use super::*;
use crate::exec_cell::CommandOutput;
use crate::exec_cell::new_active_exec_command;
use crate::history_cell::HistoryCell;
use codex_app_server_protocol::CommandExecutionSource;
use codex_shell_command::parse_command::parse_command;
use pretty_assertions::assert_eq;
use std::time::Duration;

fn cell(shell: &str, script: &str, output: &str, code: i32) -> ExecCell {
    let command = vec![shell.to_owned(), "-Command".to_owned(), script.to_owned()];
    let parsed = parse_command(&command);
    let mut cell = new_active_exec_command(
        "test".to_owned(),
        command,
        parsed,
        CommandExecutionSource::Agent,
        None,
        false,
    );
    cell.complete_call(
        "test",
        CommandOutput::new(code, output.to_owned()),
        Duration::from_millis(5),
    );
    cell
}

fn compact(cell: &ExecCell, width: u16) -> String {
    cell.compact_hyperlink_lines(width)
        .iter()
        .map(|line| line.line.to_string())
        .join("\n")
}

#[test]
fn powershell_sections_collapse_without_losing_command_or_output() {
    let script = "Get-Content codex-rs/tui/src/bottom_pane/mod.rs -Encoding utf8 | Select-Object -Skip 311 -First 9; Get-Content codex-rs/tui/src/bottom_pane/mod.rs -Encoding utf8 | Select-Object -Skip 391 -First 9; Get-Content codex-rs/tui/src/bottom_pane/mod.rs -Encoding utf8 | Select-Object -Last 9";
    let cell = cell(
        "powershell.exe",
        script,
        "first section\nsecond section\nthird section",
        0,
    );
    assert!(cell.is_exploring_cell());
    assert_eq!(
        compact(&cell, 80),
        "● Чтение codex-rs/tui/src/bottom_pane/mod.rs · 3 действия  ✓"
    );
    let full = cell.raw_lines().iter().map(ToString::to_string).join("\n");
    assert!(full.contains(script));
    assert!(full.contains("first section\nsecond section\nthird section"));
}

#[test]
fn successful_commands_hide_output_and_count_every_hidden_line() {
    let cell = cell(
        "powershell.exe",
        "git diff --check",
        "one\ntwo\nthree\nfour",
        0,
    );
    assert_eq!(compact(&cell, 80), "● Проверка .  ✓");
    assert!(matches!(
        cell.activity_disclosure(80),
        Some(ActivityDisclosure::OutputLines(4))
    ));
    let empty = cell_fn("git diff --check", "", 0);
    assert_eq!(
        empty.activity_disclosure(80),
        Some(ActivityDisclosure::Generic)
    );
    assert_eq!(compact(&empty, 80), "● Проверка .  ✓");
}

fn cell_fn(script: &str, output: &str, code: i32) -> ExecCell {
    cell("powershell.exe", script, output, code)
}

#[test]
fn compact_failure_hides_diagnostics_but_keeps_them_available_to_expand() {
    let cell = cell_fn(
        "cargo check",
        "error: missing symbol\nwhere it happened\nhelp: fix this\nmore details",
        101,
    );
    assert_eq!(compact(&cell, 80), "● Проверка .  ✗ ошибка · код 101");
    assert!(matches!(
        cell.activity_disclosure(80),
        Some(ActivityDisclosure::OutputLines(4))
    ));
    assert!(
        cell.raw_lines()
            .iter()
            .any(|line| line.to_string() == "more details")
    );
}

#[test]
fn powershell_commands_are_summarized_in_compact_view() {
    let mixed = cell_fn(
        "Get-Content file.txt -Encoding utf8; Remove-Item file.txt",
        "",
        0,
    );
    assert!(!mixed.is_exploring_cell());
    assert!(!compact(&mixed, 100).contains("Remove-Item"));
    assert!(
        mixed
            .raw_lines()
            .iter()
            .any(|line| line.to_string().contains("Remove-Item"))
    );
    let dynamic = cell_fn("Get-Content $(Get-Item file.txt) -Encoding utf8", "", 0);
    assert!(!dynamic.is_exploring_cell());
    assert!(!compact(&dynamic, 100).contains("Get-Content"));
    let selector = cell_fn(
        "Get-Content file.txt | Select-Object -ExpandProperty Secret",
        "",
        0,
    );
    assert!(!selector.is_exploring_cell());
}

#[test]
fn read_errors_hide_output_in_compact_view() {
    let mut cell = cell_fn("Get-Content first.txt -Encoding utf8", "first output", 0);
    let command = vec![
        "powershell.exe".to_owned(),
        "-Command".to_owned(),
        "Get-Content missing.txt -Encoding utf8".to_owned(),
    ];
    assert!(cell.add_call(
        "missing".to_owned(),
        command.clone(),
        parse_command(&command),
        CommandExecutionSource::Agent,
        None
    ));
    cell.complete_call(
        "missing",
        CommandOutput::new(1, "file not found".to_owned()),
        Duration::from_millis(5),
    );
    assert_eq!(
        compact(&cell, 80),
        "● Чтение first.txt  ✓\n● Чтение missing.txt  ✗ ошибка · код 1"
    );
}

#[test]
fn compact_headers_preserve_status_on_narrow_terminals() {
    let cell = cell_fn(
        "& 'C:/Users/lisov/.cargo/bin/cargo.exe' fmt -p codex-tui",
        "",
        0,
    );
    assert_eq!(compact(&cell, 80), "● Форматирование .  ✓");
    for width in [1, 8, 24, 40, 80] {
        let lines = cell.compact_hyperlink_lines(width);
        assert!(lines.iter().all(|line| line.width() <= usize::from(width)));
        if width >= 24 {
            assert!(compact(&cell, width).ends_with("  ✓"));
        }
    }
}

#[test]
fn search_without_matches_keeps_neutral_exit_status() {
    let cell = cell_fn("rg absent .", "", 1);
    let lines = cell.compact_hyperlink_lines(80);
    assert!(compact(&cell, 80).ends_with("совпадений нет"));
    assert!(
        !lines[0]
            .line
            .spans
            .iter()
            .any(|span| span.style.fg == Some(ratatui::style::Color::Red))
    );
}

#[test]
fn active_command_hides_output_in_compact_view() {
    let command = vec![
        "powershell.exe".to_owned(),
        "-Command".to_owned(),
        "cargo check".to_owned(),
    ];
    let mut cell = new_active_exec_command(
        "live".to_owned(),
        command.clone(),
        parse_command(&command),
        CommandExecutionSource::Agent,
        None,
        false,
    );
    cell.append_output("live", "old progress\none\ntwo\nlatest progress\n");
    let preview = compact(&cell, 80);
    assert_eq!(preview, "● Проверка .  · выполняется");
    assert!(!preview.contains('✓'));
    assert!(matches!(
        cell.activity_disclosure(80),
        Some(ActivityDisclosure::OutputLines(4))
    ));
}
