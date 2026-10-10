//! Shared bounds for owned-transcript action previews; expansion uses retained source content.

use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::terminal_hyperlinks::HyperlinkLine;
use ratatui::style::Stylize;
use ratatui::text::Line;
use ratatui::text::Span;

pub(crate) const DETAIL_PREVIEW_LINES: usize = 3;

/// Details an activity can reveal without materializing its expanded transcript.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ActivityDisclosure {
    Generic,
    /// Retained output lines not fully visible in the preview, excluding storage omissions.
    OutputLines(usize),
}

/// Clip a preview row without teaching selection to copy text that is currently hidden.
pub(crate) fn clipped_line(line: Line<'static>, width: u16) -> HyperlinkLine {
    truncate_line_with_ellipsis_if_overflow(line, usize::from(width)).into()
}

pub(crate) fn short_path(path: &str) -> String {
    let parts = path
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    parts[parts.len().saturating_sub(2)..].join("/")
}

pub(crate) fn activity_header(
    marker: Span<'static>,
    title: &str,
    detail: &str,
    suffix: Vec<Span<'static>>,
    width: u16,
) -> HyperlinkLine {
    let mut header = Line::from(vec![
        marker,
        " ".into(),
        title.to_owned().bold(),
        " ".into(),
    ]);
    let suffix = Line::from(suffix);
    let remaining = usize::from(width).saturating_sub(header.width() + suffix.width());
    let detail = detail.replace(['\n', '\r', '\t'], " ");
    header.extend(truncate_line_with_ellipsis_if_overflow(Line::from(detail), remaining).spans);
    header.extend(suffix.spans);
    clipped_line(header, width)
}
