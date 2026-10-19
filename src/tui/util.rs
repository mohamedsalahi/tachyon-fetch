use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line as RLine, Span as RSpan};

use crate::{Line, Rgb};

pub fn line(line: &Line) -> RLine<'static> {
    let mut spans = Vec::with_capacity(line.spans.len());
    for (text, color, bold) in &line.spans {
        let mut style = Style::default();
        if let Some((r, g, b)) = color {
            style = style.fg(Color::Rgb(*r, *g, *b));
        }
        if *bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        spans.push(RSpan::styled(text.clone(), style));
    }
    RLine::from(spans)
}

pub fn lines(lines: &[Line]) -> Vec<RLine<'static>> {
    lines.iter().map(line).collect()
}

pub fn fg(rgb: Rgb) -> Style {
    Style::default().fg(Color::Rgb(rgb.0, rgb.1, rgb.2))
}

pub fn bg(rgb: Rgb) -> Style {
    Style::default().bg(Color::Rgb(rgb.0, rgb.1, rgb.2))
}

pub fn hex(rgb: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2)
}

pub fn bar(pct: u8, width: usize, color: Rgb) -> RSpan<'static> {
    let filled = ((pct as usize).min(100) * width) / 100;
    let mut text = String::with_capacity(width);
    for i in 0..width {
        text.push(if i < filled { '█' } else { '░' });
    }
    RSpan::styled(text, fg(color))
}

pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(area.height.saturating_sub(height) / 2),
            Constraint::Length(height.min(area.height)),
            Constraint::Min(0),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(area.width.saturating_sub(width) / 2),
            Constraint::Length(width.min(area.width)),
            Constraint::Min(0),
        ])
        .split(vertical[1])[1]
}

pub fn adjust(value: i64, delta: i64, min: i64, max: i64) -> u8 {
    (value + delta).clamp(min, max) as u8
}

pub fn wrap_index(index: usize, len: usize, delta: i64) -> usize {
    if len == 0 {
        return 0;
    }
    let value = index as i64 + delta;
    value.rem_euclid(len as i64) as usize
}
