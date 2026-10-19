use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line as RLine, Span as RSpan, Text};
use ratatui::widgets::{
    Block, BorderType, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Sparkline, Table,
};
use ratatui::Frame;

use super::util;
use super::{App, COLOR_NAMES, PAGE_TITLES};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let pal = app.theme.pal;
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(5),
        Constraint::Length(2),
    ])
    .split(area);

    let titles: Vec<RLine> = PAGE_TITLES
        .iter()
        .enumerate()
        .map(|(index, title)| {
            let style = if index == app.page {
                util::fg(pal.accent).add_modifier(Modifier::BOLD)
            } else {
                util::fg(pal.dim)
            };
            RLine::from(RSpan::styled(format!(" {} ", title), style))
        })
        .collect();
    let tabs = ratatui::widgets::Tabs::new(titles)
        .select(app.page)
        .divider(RSpan::styled("│", util::fg(pal.dim)));
    frame.render_widget(tabs, chunks[0]);

    match app.page {
        0 => modules(frame, chunks[1], app),
        1 => appearance(frame, chunks[1], app),
        2 => palette(frame, chunks[1], app),
        3 => thresholds(frame, chunks[1], app),
        4 => artwork(frame, chunks[1], app),
        5 => preview(frame, chunks[1], app),
        _ => keys(frame, chunks[1], app),
    }

    let dirty = if app.dirty { "*" } else { "" };
    let hints = match app.page {
        0 => "space toggle · j/k move",
        2 => "enter edit color · p copy palette · x clear",
        4 => "enter edit path · a add photo · x clear image",
        5 => "j/k scroll · g top · G bottom",
        _ => "←/→ adjust · enter act",
    };
    let status = RLine::from(vec![
        RSpan::styled(" s ", util::fg(pal.accent).add_modifier(Modifier::BOLD)),
        RSpan::styled("save  ", util::fg(pal.dim)),
        RSpan::styled(" r ", util::fg(pal.accent2).add_modifier(Modifier::BOLD)),
        RSpan::styled("refresh  ", util::fg(pal.dim)),
        RSpan::styled(" tab ", util::fg(pal.accent2).add_modifier(Modifier::BOLD)),
        RSpan::styled("page  ", util::fg(pal.dim)),
        RSpan::styled(" q ", util::fg(pal.err).add_modifier(Modifier::BOLD)),
        RSpan::styled("quit   ", util::fg(pal.dim)),
        RSpan::styled(format!("{} {}", hints, dirty), util::fg(pal.dim)),
    ]);
    frame.render_widget(
        Paragraph::new(status).block(Block::default().borders(ratatui::widgets::Borders::TOP)),
        chunks[2],
    );

    if app.edit.is_some() {
        edit_popup(frame, area, app);
    }
    if app.color_edit.is_some() {
        color_popup(frame, area, app);
    }
}

fn panel(title: &str, app: &App) -> Block<'static> {
    let pal = app.theme.pal;
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(util::fg(pal.dim))
        .title(RSpan::styled(format!(" {} ", title), util::fg(pal.accent2).add_modifier(Modifier::BOLD)))
}

fn list_state(index: usize) -> ListState {
    let mut state = ListState::default();
    state.select(Some(index));
    state
}

fn table_state(index: usize) -> ratatui::widgets::TableState {
    let mut state = ratatui::widgets::TableState::default();
    state.select(Some(index));
    state
}

fn modules(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let pal = app.theme.pal;
    let items: Vec<ListItem> = crate::MODULES
        .iter()
        .map(|(id, label)| {
            let enabled = app
                .cfg
                .enabled
                .as_ref()
                .map_or(true, |list| list.iter().any(|item| item == id));
            let mark = if enabled {
                RSpan::styled("[x]", util::fg(pal.accent).add_modifier(Modifier::BOLD))
            } else {
                RSpan::styled("[ ]", util::fg(pal.dim))
            };
            ListItem::new(RLine::from(vec![
                mark,
                RSpan::raw(" "),
                RSpan::styled(format!("{:<12}", id), util::fg(pal.fg)),
                RSpan::styled((*label).to_string(), util::fg(pal.dim)),
            ]))
        })
        .collect();
    let enabled = app
        .cfg
        .enabled
        .as_ref()
        .map(|list| list.len())
        .unwrap_or(crate::MODULES.len());
    let block = panel(&format!("modules {}/{}", enabled, crate::MODULES.len()), app);
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut list_state(app.cursors[0]));
}

fn appearance(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let pal = app.theme.pal;
    let layout = app.cfg.layout.clone().unwrap_or_else(|| app.theme.layout.clone());
    let motion = app.cfg.motion.clone().unwrap_or_else(|| app.theme.motion.clone());
    let rows = vec![
        row("layout", &layout, app),
        row("glyph", app.cfg.glyph.as_deref().unwrap_or("random (▸ ▚ ◆ ▰)"), app),
        row("motion", &motion, app),
        row("rule", &app.cfg.rule_char().to_string(), app),
        row("units", app.cfg.units.as_deref().unwrap_or("binary (GiB)"), app),
        row("temperature", if app.cfg.fahrenheit() { "fahrenheit" } else { "celsius" }, app),
    ];
    let table = Table::new(rows, [Constraint::Length(18), Constraint::Min(20)])
        .block(panel("appearance", app))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(table, area, &mut table_state(app.cursors[1]));
    let _ = pal;
}

fn row(name: &str, value: &str, app: &App) -> Row<'static> {
    let pal = app.theme.pal;
    Row::new(vec![
        Cell::from(RSpan::styled(name.to_string(), util::fg(pal.dim))),
        Cell::from(RSpan::styled(value.to_string(), util::fg(pal.fg).add_modifier(Modifier::BOLD))),
    ])
}

fn palette(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let pal = app.theme.pal;
    let theme_name = app.cfg.theme.clone().unwrap_or_else(|| app.theme.name.clone());
    let variant = app.cfg.variant.clone().unwrap_or_else(|| app.theme.mode.clone());
    let mut rows = vec![
        row("theme", &theme_name, app),
        row("variant", &variant, app),
    ];
    for (index, name) in COLOR_NAMES.iter().enumerate() {
        let color = app.effective_color(index);
        let overridden = app.color(index).is_some();
        let swatch = RSpan::styled("    ", util::bg(color));
        let label = if overridden { format!("{} *", name) } else { name.to_string() };
        rows.push(Row::new(vec![
            Cell::from(RSpan::styled(label, util::fg(pal.dim))),
            Cell::from(RLine::from(vec![
                swatch,
                RSpan::raw("  "),
                RSpan::styled(util::hex(color), util::fg(pal.fg)),
            ])),
        ]));
    }
    rows.push(Row::new(vec![
        Cell::from(RSpan::styled("reset", util::fg(pal.dim))),
        Cell::from(RSpan::styled("clear all overrides", util::fg(pal.warn))),
    ]));
    let table = Table::new(rows, [Constraint::Length(18), Constraint::Min(20)])
        .block(panel("palette", app))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(table, area, &mut table_state(app.cursors[2]));
}

fn thresholds(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let pal = app.theme.pal;
    let current = [
        app.data.memory_pct,
        app.data.swap_pct,
        app.data.disk_pct,
        app.data.battery_pct,
        app.data.cpu_temp.unwrap_or(0),
        app.data.cpu_temp.unwrap_or(0),
        app.data.gpu_util.unwrap_or(0),
    ];
    let values = [
        ("memory warn", app.cfg.memory_warn.unwrap_or(85), "%"),
        ("swap warn", app.cfg.swap_warn.unwrap_or(50), "%"),
        ("disk warn", app.cfg.disk_warn.unwrap_or(90), "%"),
        ("battery low", app.cfg.battery_low.unwrap_or(15), "%"),
        ("temp warn", app.cfg.temp_warn.unwrap_or(85), "°C"),
        ("temp crit", app.cfg.temp_crit.unwrap_or(95), "°C"),
        ("gpu warn", app.cfg.gpu_warn.unwrap_or(95), "%"),
    ];
    let rows: Vec<Row> = values
        .iter()
        .enumerate()
        .map(|(index, (name, value, unit))| {
            let bar = util::bar(current[index], 20, if current[index] >= *value { pal.warn } else { pal.ok });
            Row::new(vec![
                Cell::from(RSpan::styled(*name, util::fg(pal.dim))),
                Cell::from(RSpan::styled(format!("{}{}", value, unit), util::fg(pal.fg))),
                Cell::from(RLine::from(vec![bar, RSpan::raw(" "), RSpan::styled(format!("now {}", current[index]), util::fg(pal.dim))])),
            ])
        })
        .collect();
    let table = Table::new(rows, [Constraint::Length(14), Constraint::Length(8), Constraint::Min(24)])
        .block(panel("thresholds", app))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(table, area, &mut table_state(app.cursors[3]));
}

fn artwork(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let pal = app.theme.pal;
    let image = app
        .cfg
        .image
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(none)".to_string());
    let album = app.cfg.album.clone().unwrap_or_else(|| "(none)".to_string());
    let rows = vec![
        row("image", &image, app),
        row("album", &album, app),
        row("width %", &format!("{}", app.cfg.logo_width.unwrap_or(34)), app),
        row("min cols", &format!("{}", app.cfg.logo_min.unwrap_or(26)), app),
        row("max cols", &format!("{}", app.cfg.logo_max.unwrap_or(46)), app),
        row("gap", &format!("{}", app.cfg.logo_gap.unwrap_or(3)), app),
    ];
    let table = Table::new(rows, [Constraint::Length(14), Constraint::Min(24)])
        .block(panel("artwork", app))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(table, area, &mut table_state(app.cursors[4]));
    let _ = pal;
}

fn preview(frame: &mut Frame, area: ratatui::layout::Rect, app: &mut App) {
    let pal = app.theme.pal;
    let chunks = Layout::vertical([Constraint::Min(6), Constraint::Length(3)]).split(area);
    let width = chunks[0].width.saturating_sub(2) as usize;
    let glyph = app
        .cfg
        .glyph
        .as_deref()
        .filter(|value| *value != "random" && *value != "mixed")
        .and_then(|value| value.chars().next())
        .unwrap_or('▸');
    let lines = crate::render(
        &app.theme,
        &app.data,
        &app.quote,
        width.max(20),
        glyph,
        true,
        app.demo,
        &app.cfg,
    );
    let view_height = chunks[0].height.saturating_sub(2) as usize;
    let max_scroll = lines.len().saturating_sub(view_height);
    if app.scroll > max_scroll {
        app.scroll = max_scroll;
    }
    let paragraph = Paragraph::new(Text::from(util::lines(&lines)))
        .block(panel("live readout", app))
        .scroll((app.scroll as u16, 0));
    frame.render_widget(paragraph, chunks[0]);

    let graphs = Layout::horizontal([
        Constraint::Percentage(50),
        Constraint::Percentage(50),
    ])
    .split(chunks[1]);
    let memory = Sparkline::default()
        .data(&app.history_mem)
        .max(100)
        .style(util::fg(pal.accent));
    frame.render_widget(memory.block(Block::default().title(RSpan::styled(" memory %", util::fg(pal.dim)))), graphs[0]);
    let swap = Sparkline::default()
        .data(&app.history_swap)
        .max(100)
        .style(util::fg(pal.accent2));
    frame.render_widget(swap.block(Block::default().title(RSpan::styled(" swap %", util::fg(pal.dim)))), graphs[1]);
}

fn keys(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let pal = app.theme.pal;
    let rows = [
        ("tab / shift-tab / 1-7", "switch page"),
        ("j k / ↑ ↓", "move · preview scroll"),
        ("h l / ← →", "adjust value"),
        ("space", "toggle module"),
        ("enter", "edit · open · apply"),
        ("x", "clear override"),
        ("p (palette)", "copy palette into overrides"),
        ("s / r", "save · refresh data"),
        ("q / ctrl-c", "quit"),
    ];
    let lines: Vec<RLine> = rows
        .iter()
        .map(|(key, effect)| {
            RLine::from(vec![
                RSpan::styled(format!("  {:<22}", key), util::fg(pal.accent2)),
                RSpan::styled((*effect).to_string(), util::fg(pal.fg)),
            ])
        })
        .collect();
    let mut text = vec![RLine::from(RSpan::styled(
        format!("  tachyon-fetch {} · configurator", crate::VERSION),
        util::fg(pal.accent).add_modifier(Modifier::BOLD),
    ))];
    text.extend(lines);
    text.push(RLine::from(""));
    text.push(RLine::from(RSpan::styled(
        "  built on ratatui · every change lands in fetch.conf on save",
        util::fg(pal.dim),
    )));
    frame.render_widget(Paragraph::new(Text::from(text)).block(panel("keys", app)), area);
}

fn edit_popup(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let pal = app.theme.pal;
    let Some(edit) = &app.edit else {
        return;
    };
    let popup = util::centered(area, 60.min(area.width.saturating_sub(4)), 3);
    frame.render_widget(Clear, popup);
    let title = match edit.target {
        super::EditTarget::Image => "image path",
        super::EditTarget::Glyph => "glyph",
        super::EditTarget::Rule => "rule character",
        super::EditTarget::Photo => "photo path",
    };
    let line = RLine::from(vec![
        RSpan::styled(" ", util::fg(pal.fg)),
        RSpan::styled(edit.text.clone(), util::fg(pal.fg).add_modifier(Modifier::BOLD)),
        RSpan::styled("▏", util::fg(pal.accent)),
    ]);
    frame.render_widget(Paragraph::new(line).block(panel(title, app)), popup);
}

fn color_popup(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let pal = app.theme.pal;
    let Some(edit) = &app.color_edit else {
        return;
    };
    let popup = util::centered(area, 46.min(area.width.saturating_sub(4)), 7);
    frame.render_widget(Clear, popup);
    let color = app.effective_color(edit.row);
    let channels = [color.0, color.1, color.2];
    let names = ["R", "G", "B"];
    let mut lines = Vec::new();
    for index in 0..3 {
        let marker = if index == edit.channel { "▶" } else { " " };
        let style = if index == edit.channel {
            util::fg(pal.accent).add_modifier(Modifier::BOLD)
        } else {
            util::fg(pal.dim)
        };
        lines.push(RLine::from(vec![
            RSpan::styled(format!(" {} {} ", marker, names[index]), style),
            util::bar(channels[index], 32, pal.accent2),
            RSpan::styled(format!(" {}", channels[index]), util::fg(pal.fg)),
        ]));
    }
    lines.push(RLine::from(RSpan::styled(
        "  ←/→ adjust · ↑/↓ channel · x clear · enter close",
        util::fg(pal.dim),
    )));
    frame.render_widget(
        Paragraph::new(Text::from(lines)).block(panel(&format!("color · {}", COLOR_NAMES[edit.row]), app)),
        popup,
    );
}
