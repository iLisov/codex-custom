//! Pointer targets are measured in terminal cells and checked against the last painted footer.
//! Clipped labels and transient hints never retain invisible clickable regions.

use super::*;
use crate::bottom_pane::StatusLineMouseTarget;
use crate::slash_command::SlashCommand;
use crossterm::event::MouseButton;
use crossterm::event::MouseEvent;
use crossterm::event::MouseEventKind;
use ratatui::layout::Position;
use std::cell::Cell;
use std::cell::RefCell;
use unicode_width::UnicodeWidthStr;

#[derive(Default)]
pub(super) struct StatusLineMouse {
    targets: Vec<StatusLineMouseTarget>,
    regions: RefCell<Vec<(Rect, SlashCommand)>>,
    pointer: Cell<Option<Position>>,
}

impl StatusLineMouse {
    pub(super) fn clear_regions(&self) {
        self.regions.borrow_mut().clear();
    }

    fn set_targets(&mut self, targets: Vec<StatusLineMouseTarget>) -> bool {
        if self.targets == targets {
            return false;
        }
        self.targets = targets;
        self.clear_regions();
        true
    }

    pub(super) fn render(&self, area: Option<Rect>, buf: &mut Buffer) {
        self.clear_regions();
        let Some(area) = area.filter(|area| !area.is_empty()) else {
            return;
        };
        if area.intersection(buf.area) != area {
            return;
        }
        const SETTINGS_LABEL: &str = "⚙ Settings";
        let width = SETTINGS_LABEL.width() as u16;
        if area.width >= width + 2 {
            let x = area.right() - width - 1;
            // Keep existing status text intact, including clipped labels in narrow terminals.
            let previous_button: String = (x..x + width)
                .map(|column| buf[(column, area.y)].symbol())
                .collect();
            let free = (x - 1..area.right())
                .all(|column| buf[(column, area.y)].symbol().trim().is_empty())
                || (previous_button == SETTINGS_LABEL
                    && buf[(x - 1, area.y)].symbol().trim().is_empty()
                    && buf[(area.right() - 1, area.y)].symbol().trim().is_empty());
            if free {
                buf.set_string(x, area.y, SETTINGS_LABEL, Style::default().cyan());
                let region = Rect::new(x, area.y, width, 1);
                self.regions
                    .borrow_mut()
                    .push((region, SlashCommand::Settings));
                if self
                    .pointer
                    .get()
                    .is_some_and(|pointer| region.contains(pointer))
                {
                    buf.set_style(
                        region,
                        Style::default().add_modifier(Modifier::REVERSED | Modifier::UNDERLINED),
                    );
                }
            }
        }
        for target in &self.targets {
            let start = usize::from(area.x) + FOOTER_INDENT_COLS + target.columns.start;
            let end = usize::from(area.x) + FOOTER_INDENT_COLS + target.columns.end;
            if start >= end || end > usize::from(area.right()) {
                continue;
            }
            let region = Rect::new(start as u16, area.y, (end - start) as u16, 1);
            if region.intersection(buf.area) != region {
                continue;
            }
            let mut painted = String::new();
            let mut column = region.x;
            while column < region.right() {
                let symbol = buf[(column, region.y)].symbol();
                painted.push_str(symbol);
                column += symbol.width().max(1) as u16;
            }
            if painted != target.label {
                continue;
            }
            self.regions.borrow_mut().push((region, target.command));
            if self
                .pointer
                .get()
                .is_some_and(|pointer| region.contains(pointer))
            {
                buf.set_style(
                    region,
                    Style::default().add_modifier(Modifier::REVERSED | Modifier::UNDERLINED),
                );
            }
        }
    }

    fn handle(&self, event: MouseEvent) -> (bool, Option<SlashCommand>) {
        let position = Position::new(event.column, event.row);
        let was_hovered = self.pointer.get().is_some_and(|pointer| {
            self.regions
                .borrow()
                .iter()
                .any(|(region, _)| region.contains(pointer))
        });
        self.pointer.set(Some(position));
        let hovered = self
            .regions
            .borrow()
            .iter()
            .find(|(region, _)| region.contains(position))
            .map(|(_, command)| *command);
        let clicked = hovered.filter(|_| {
            event.modifiers.is_empty() && event.kind == MouseEventKind::Down(MouseButton::Left)
        });
        let handled = clicked.is_some()
            || (event.kind == MouseEventKind::Moved && (hovered.is_some() || was_hovered))
            || (hovered.is_some() && event.kind == MouseEventKind::Up(MouseButton::Left));
        (handled, clicked)
    }
}

impl ChatComposer {
    pub(crate) fn set_status_line_mouse_targets(
        &mut self,
        targets: Vec<StatusLineMouseTarget>,
    ) -> bool {
        self.status_line_mouse.set_targets(targets)
    }

    pub(crate) fn clear_status_line_mouse_regions(&self) {
        self.status_line_mouse.clear_regions();
    }

    pub(crate) fn handle_status_line_mouse(
        &self,
        event: MouseEvent,
    ) -> (bool, Option<SlashCommand>) {
        self.status_line_mouse.handle(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bottom_pane::StatusLineItem;
    use crate::bottom_pane::footer::render_footer_line;
    use crate::bottom_pane::status_line_mouse_targets;

    fn mouse(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
        MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        }
    }

    #[test]
    fn settings_mouse_button_is_right_aligned_and_clickable() {
        let target = StatusLineMouse::default();
        let area = Rect::new(5, 4, 60, 1);
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
        render_footer_line(area, &mut buf, "Model · Context 94% left".into());
        target.render(Some(area), &mut buf);
        let x = area.right() - "⚙ Settings".width() as u16 - 1;
        assert_eq!(
            target.handle(mouse(MouseEventKind::Moved, x, 4)),
            (true, None)
        );
        target.render(Some(area), &mut buf);
        assert!(buf[(x, 4)].modifier.contains(Modifier::REVERSED));
        assert_eq!(
            target.handle(mouse(MouseEventKind::Down(MouseButton::Left), x, 4)),
            (true, Some(SlashCommand::Settings))
        );
        target.render(None, &mut buf);
        assert_eq!(
            target
                .handle(mouse(MouseEventKind::Down(MouseButton::Left), x, 4))
                .1,
            None
        );
    }

    #[test]
    fn settings_mouse_button_does_not_overwrite_status_text() {
        let target = StatusLineMouse::default();
        let area = Rect::new(0, 0, 25, 1);
        let mut buf = Buffer::empty(area);
        render_footer_line(area, &mut buf, "1234567890123456789012345".into());
        let before = buf.clone();
        target.render(Some(area), &mut buf);
        assert_eq!(buf, before);
        assert!(target.regions.borrow().is_empty());
    }

    fn target() -> StatusLineMouse {
        let mut target = StatusLineMouse::default();
        target.set_targets(status_line_mouse_targets(&[
            (StatusLineItem::ModelName, "模型".to_string()),
            (StatusLineItem::Permissions, "Ask for approval".to_string()),
        ]));
        target
    }

    #[test]
    fn hover_and_click_use_painted_cells_after_a_wide_prefix() {
        let target = target();
        let area = Rect::new(5, 4, 50, 1);
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
        render_footer_line(area, &mut buf, "模型 · Ask for approval".into());
        target.render(Some(area), &mut buf);
        assert_eq!(
            target.handle(mouse(MouseEventKind::Moved, 14, 4)),
            (true, None)
        );
        target.render(Some(area), &mut buf);
        assert!(buf[(14, 4)].modifier.contains(Modifier::REVERSED));
        assert_eq!(
            target.handle(mouse(MouseEventKind::Down(MouseButton::Left), 14, 4)),
            (true, Some(SlashCommand::Permissions))
        );
        assert_eq!(
            target.handle(mouse(MouseEventKind::Moved, 5, 2)),
            (true, None)
        );
    }

    #[test]
    fn model_and_permissions_mouse_clicks_dispatch_their_own_commands() {
        use crate::bottom_pane::status_line_from_segments;

        for item in [
            StatusLineItem::ModelName,
            StatusLineItem::ModelWithReasoning,
        ] {
            let segments = vec![
                (item, "gpt-6.1-sol high".to_string()),
                (StatusLineItem::Permissions, "Ask for approval".to_string()),
            ];
            let mut target = StatusLineMouse::default();
            target.set_targets(status_line_mouse_targets(&segments));
            let area = Rect::new(3, 6, 70, 1);
            let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
            render_footer_line(
                area,
                &mut buf,
                status_line_from_segments(segments, false, None).unwrap(),
            );
            target.render(Some(area), &mut buf);
            for (index, command) in [SlashCommand::Model, SlashCommand::Permissions]
                .into_iter()
                .enumerate()
            {
                let column =
                    area.x + FOOTER_INDENT_COLS as u16 + target.targets[index].columns.start as u16;
                assert_eq!(
                    target.handle(mouse(MouseEventKind::Moved, column, area.y)),
                    (true, None)
                );
                target.render(Some(area), &mut buf);
                assert!(buf[(column, area.y)].modifier.contains(Modifier::REVERSED));
                assert_eq!(
                    target.handle(mouse(
                        MouseEventKind::Down(MouseButton::Left),
                        column,
                        area.y
                    )),
                    (true, Some(command))
                );
            }
        }
    }

    #[test]
    fn clipped_and_replaced_status_lines_are_not_clickable() {
        let target = target();
        let area = Rect::new(0, 4, 15, 1);
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
        render_footer_line(area, &mut buf, "模型 · Ask for approval".into());
        target.render(Some(area), &mut buf);
        assert_eq!(
            target.handle(mouse(MouseEventKind::Down(MouseButton::Left), 9, 4)),
            (false, None)
        );
        let area = Rect::new(0, 4, 60, 1);
        render_footer_line(area, &mut buf, "Press Esc to cancel".into());
        target.render(Some(area), &mut buf);
        assert_eq!(
            target.handle(mouse(MouseEventKind::Down(MouseButton::Left), 9, 4)),
            (false, None)
        );
    }

    #[test]
    fn modifier_clicks_and_other_rows_do_not_open_permissions() {
        let target = target();
        let area = Rect::new(0, 4, 60, 1);
        let mut buf = Buffer::empty(Rect::new(0, 0, 80, 10));
        render_footer_line(area, &mut buf, "模型 · Ask for approval".into());
        target.render(Some(area), &mut buf);
        let mut event = mouse(MouseEventKind::Down(MouseButton::Left), 9, 4);
        event.modifiers = KeyModifiers::CONTROL;
        assert_eq!(target.handle(event), (false, None));
        assert_eq!(
            target.handle(mouse(MouseEventKind::Down(MouseButton::Left), 9, 5)),
            (false, None)
        );
        target.render(None, &mut buf);
        assert_eq!(
            target.handle(mouse(MouseEventKind::Down(MouseButton::Left), 9, 4)),
            (false, None)
        );
    }
}
