use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line as RLine, Span as RSpan, Text};
use ratatui::widgets::{
    Block, BorderType, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Sparkline, Table,
};
use ratatui::Frame;

use super::util;
use super::{
    App, EditTarget, PickerKind, COLOR_NAMES, DECK_ROWS, GLYPH_FIXED_ROWS, PAGE_ARTWORK, PAGE_DECK,
    PAGE_GLYPHS, PAGE_KEYS, PAGE_MODULES, PAGE_PALETTE, PAGE_PREVIEW, PAGE_THRESHOLDS, PAGE_TITLES,
    PALETTE_ROWS,
};

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    app.artwork_pane = None;
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
        PAGE_MODULES => modules(frame, chunks[1], app),
        PAGE_DECK => deck(frame, chunks[1], app),
        PAGE_GLYPHS => glyphs(frame, chunks[1], app),
        PAGE_PALETTE => palette(frame, chunks[1], app),
        PAGE_THRESHOLDS => thresholds(frame, chunks[1], app),
        PAGE_ARTWORK => artwork(frame, chunks[1], app),
        PAGE_PREVIEW => preview(frame, chunks[1], app),
        PAGE_KEYS => keys(frame, chunks[1], app),
        _ => {}
    }

    let dirty = if app.dirty { "*" } else { "" };
    let hints = match app.page {
        PAGE_MODULES => "space toggle · shift+enter / m move · J/K reorder · x restore",
        PAGE_DECK => "-/+ adjust · enter act · x reset",
        PAGE_GLYPHS => "enter glyph library · e value · space pin/random · G pool from font",
        PAGE_PALETTE => "enter pick · space marks the rotation set · p copy · x clear",
        PAGE_ARTWORK => "enter browse · a add photo · -/+ rotate · x clear",
        PAGE_PREVIEW => "j/k scroll · g top · G bottom",
        _ => "h/l page · R reset",
    };
    let status = RLine::from(vec![
        RSpan::styled(" s ", util::fg(pal.accent).add_modifier(Modifier::BOLD)),
        RSpan::styled("save  ", util::fg(pal.dim)),
        RSpan::styled(" r ", util::fg(pal.accent2).add_modifier(Modifier::BOLD)),
        RSpan::styled("refresh  ", util::fg(pal.dim)),
        RSpan::styled(" h/l ", util::fg(pal.accent2).add_modifier(Modifier::BOLD)),
        RSpan::styled("page  ", util::fg(pal.dim)),
        RSpan::styled(" R ", util::fg(pal.warn).add_modifier(Modifier::BOLD)),
        RSpan::styled("reset  ", util::fg(pal.dim)),
        RSpan::styled(" q ", util::fg(pal.err).add_modifier(Modifier::BOLD)),
        RSpan::styled("quit   ", util::fg(pal.dim)),
        RSpan::styled(format!("{} {}", hints, dirty), util::fg(pal.dim)),
        RSpan::styled(format!("  {}", app.status), util::fg(pal.accent2)),
    ]);
    frame.render_widget(
        Paragraph::new(status).block(Block::default().borders(ratatui::widgets::Borders::TOP)),
        chunks[2],
    );

    if let Some(picker) = &app.picker {
        picker_popup(frame, area, app, picker);
    }
    if app.edit.is_some() {
        edit_popup(frame, area, app);
    }
    if app.color_edit.is_some() {
        color_popup(frame, area, app);
    }
    if app.reset.is_some() {
        reset_popup(frame, area, app);
    }
}

fn panel(title: &str, app: &App) -> Block<'static> {
    let pal = app.theme.pal;
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(util::fg(pal.dim))
        .title(RSpan::styled(
            format!(" {} ", title),
            util::fg(pal.accent2).add_modifier(Modifier::BOLD),
        ))
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

fn row(name: &str, value: &str, app: &App) -> Row<'static> {
    let pal = app.theme.pal;
    Row::new(vec![
        Cell::from(RSpan::styled(name.to_string(), util::fg(pal.dim))),
        Cell::from(RSpan::styled(
            value.to_string(),
            util::fg(pal.fg).add_modifier(Modifier::BOLD),
        )),
    ])
}

fn modules(frame: &mut Frame, area: Rect, app: &App) {
    let pal = app.theme.pal;
    let items: Vec<ListItem> = app
        .module_order()
        .into_iter()
        .map(|(id, label)| {
            let enabled = app.module_enabled(id);
            let mark = if enabled {
                RSpan::styled("[x]", util::fg(pal.accent).add_modifier(Modifier::BOLD))
            } else {
                RSpan::styled("[ ]", util::fg(pal.dim))
            };
            let glyph = crate::config::module_glyph(&app.cfg, id, app.glyph_seed)
                .map(|glyph| glyph.to_string())
                .unwrap_or_else(|| " ".to_string());
            let picked = app.picked.as_ref().is_some_and(|(picked, _)| picked == id);
            let cursor = if picked {
                RSpan::styled("↕ ", util::fg(pal.warn).add_modifier(Modifier::BOLD))
            } else {
                RSpan::raw("  ")
            };
            ListItem::new(RLine::from(vec![
                cursor,
                mark,
                RSpan::raw(" "),
                RSpan::styled(format!("{} ", glyph), util::fg(pal.accent2)),
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
    let block = panel(
        &format!("modules {}/{}", enabled, crate::MODULES.len()),
        app,
    );
    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, area, &mut list_state(app.cursors[PAGE_MODULES]));
}

fn deck(frame: &mut Frame, area: Rect, app: &App) {
    let pal = app.theme.pal;
    let layout = app
        .cfg
        .layout
        .clone()
        .unwrap_or_else(|| app.theme.layout.clone());
    let motion = app
        .cfg
        .motion
        .clone()
        .unwrap_or_else(|| app.theme.motion.clone());
    let logo_mode = app.cfg.logo_mode();
    let logo_text = app
        .cfg
        .logo_text
        .clone()
        .unwrap_or_else(|| "(hostname)".to_string());
    let font = app
        .cfg
        .logo_font
        .clone()
        .unwrap_or_else(|| "monospace".to_string());
    let rows = vec![
        row("layout", &layout, app),
        row("motion", &motion, app),
        row("rule", &app.cfg.rule_char().to_string(), app),
        row(
            "units",
            app.cfg.units.as_deref().unwrap_or("binary (GiB)"),
            app,
        ),
        row(
            "temperature",
            if app.cfg.fahrenheit() {
                "fahrenheit"
            } else {
                "celsius"
            },
            app,
        ),
        row("logo mode", logo_mode, app),
        row("logo text", &logo_text, app),
        row("font", &font, app),
        row("font size", &app.cfg.logo_size_px().to_string(), app),
        row("logo tint", app.cfg.logo_tint(), app),
        row(
            "width %",
            &app.cfg.logo_width.unwrap_or(34).to_string(),
            app,
        ),
        row("min cols", &app.cfg.logo_min.unwrap_or(26).to_string(), app),
        row("max cols", &app.cfg.logo_max.unwrap_or(46).to_string(), app),
        row("gap", &app.cfg.logo_gap.unwrap_or(3).to_string(), app),
        row("quote", &quote_value(app), app),
    ];
    let table = Table::new(rows, [Constraint::Length(16), Constraint::Min(20)])
        .block(panel(&format!("deck · {} · {}", logo_mode, font), app))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(table, area, &mut table_state(app.cursors[PAGE_DECK]));
    let _ = (DECK_ROWS, pal);
}

fn quote_value(app: &App) -> String {
    match app.cfg.quote.as_deref() {
        None => "random".to_string(),
        Some("off") | Some("none") => "off".to_string(),
        Some(text) => {
            let mut short: String = text.chars().take(64).collect();
            if text.chars().count() > 64 {
                short.push('…');
            }
            short
        }
    }
}

fn glyph_mode(app: &App, row: usize) -> String {
    let spec = app.glyph_spec(row);
    match row {
        0..=3 => {
            if matches!(spec.as_str(), "random" | "mixed") {
                "random".to_string()
            } else if spec.is_empty() {
                "header".to_string()
            } else {
                "pinned".to_string()
            }
        }
        4 => "global".to_string(),
        5 => format!("{} chars", spec.chars().count()),
        _ => {
            if spec == "auto" {
                "auto".to_string()
            } else if matches!(spec.as_str(), "random" | "mixed") {
                "random".to_string()
            } else {
                "pinned".to_string()
            }
        }
    }
}

fn glyphs(frame: &mut Frame, area: Rect, app: &App) {
    let pal = app.theme.pal;
    let rows: Vec<Row> = (0..GLYPH_FIXED_ROWS + crate::MODULES.len())
        .map(|index| {
            let label = app.glyph_label(index);
            let display = app.glyph_display(index);
            let mode = glyph_mode(app, index);
            Row::new(vec![
                Cell::from(RSpan::styled(label.to_string(), util::fg(pal.dim))),
                Cell::from(RSpan::styled(
                    format!(" {} ", display),
                    util::fg(pal.accent).add_modifier(Modifier::BOLD),
                )),
                Cell::from(RSpan::styled(mode, util::fg(pal.fg))),
            ])
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(18),
            Constraint::Length(16),
            Constraint::Min(12),
        ],
    )
    .column_spacing(1)
    .block(panel("glyphs · every fetched object", app))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(table, area, &mut table_state(app.cursors[PAGE_GLYPHS]));
}

fn palette(frame: &mut Frame, area: Rect, app: &App) {
    let pal = app.theme.pal;
    let theme_name = app
        .cfg
        .theme
        .clone()
        .unwrap_or_else(|| app.theme.name.clone());
    let variant = app
        .cfg
        .variant
        .clone()
        .unwrap_or_else(|| app.theme.mode.clone());
    let set = app
        .cfg
        .rotate_themes
        .as_ref()
        .map(|list| list.len())
        .unwrap_or(0);
    let mut rows = vec![
        row("theme", &theme_name, app),
        row("variant", &variant, app),
        row("rotate", app.cfg.rotate_palette_mode(), app),
        row("rotation set", &format!("{} themes · enter", set), app),
    ];
    for (index, name) in COLOR_NAMES.iter().enumerate() {
        let color = app.effective_color(index);
        let overridden = app.color(index).is_some();
        let swatch = RSpan::styled("    ", util::bg(color));
        let label = if overridden {
            format!("{} *", name)
        } else {
            name.to_string()
        };
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
        .block(panel(
            &format!("palette · {} themes", app.palette_names().len()),
            app,
        ))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(table, area, &mut table_state(app.cursors[PAGE_PALETTE]));
    let _ = PALETTE_ROWS;
}

fn thresholds(frame: &mut Frame, area: Rect, app: &App) {
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
            let bar = util::bar(
                current[index],
                20,
                if current[index] >= *value {
                    pal.warn
                } else {
                    pal.ok
                },
            );
            Row::new(vec![
                Cell::from(RSpan::styled(*name, util::fg(pal.dim))),
                Cell::from(RSpan::styled(
                    format!("{}{}", value, unit),
                    util::fg(pal.fg),
                )),
                Cell::from(RLine::from(vec![
                    bar,
                    RSpan::raw(" "),
                    RSpan::styled(format!("now {}", current[index]), util::fg(pal.dim)),
                ])),
            ])
        })
        .collect();
    let table = Table::new(
        rows,
        [
            Constraint::Length(14),
            Constraint::Length(8),
            Constraint::Min(24),
        ],
    )
    .block(panel("thresholds", app))
    .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(table, area, &mut table_state(app.cursors[PAGE_THRESHOLDS]));
}

fn artwork(frame: &mut Frame, area: Rect, app: &App) {
    let pal = app.theme.pal;
    let chunks = Layout::vertical([Constraint::Min(6), Constraint::Length(4)]).split(area);
    let image = app
        .cfg
        .image
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| {
            app.artwork
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "(none)".to_string())
        });
    let album = app
        .cfg
        .album
        .clone()
        .unwrap_or_else(|| "(none)".to_string());
    let rows = vec![
        row("album", &album, app),
        row("image", &image, app),
        row("rotate art", app.cfg.rotate_art_mode(), app),
        row("cadence", app.cfg.rotate_every_mode(), app),
    ];
    let table = Table::new(rows, [Constraint::Length(14), Constraint::Min(24)])
        .block(panel("artwork · album first", app))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(
        table,
        chunks[0],
        &mut table_state(app.cursors[PAGE_ARTWORK]),
    );

    let members = app
        .cfg
        .album
        .as_ref()
        .map(|album| crate::album_images(album).len())
        .unwrap_or(0);
    let note = Text::from(vec![
        RLine::from(RSpan::styled(
            format!(
                "  {} images in this album · palette rotation {} · every {}",
                members,
                app.cfg.rotate_palette_mode(),
                app.cfg.rotate_every_mode()
            ),
            util::fg(pal.dim),
        )),
        RLine::from(RSpan::styled(
            "  enter album → pick a frame or rotate the whole album; a adds a photo",
            util::fg(pal.dim),
        )),
    ]);
    frame.render_widget(Paragraph::new(note).block(panel("album", app)), chunks[1]);
}

fn preview(frame: &mut Frame, area: Rect, app: &mut App) {
    let pal = app.theme.pal;
    let chunks = Layout::vertical([Constraint::Min(6), Constraint::Length(3)]).split(area);

    if app.cfg.logo_mode() == "text" {
        let cap = app.logo_cap(area.width as usize).min(40);
        if let Some(logo) = crate::text_logo(&app.cfg, &app.data.hostname, cap, &pal) {
            let gap = app.cfg.logo_gap.unwrap_or(3) as usize;
            if area.width as usize >= logo.width + gap + 44 {
                let width = area.width as usize - logo.width - gap - 2;
                let lines = crate::render(
                    &app.theme,
                    &app.data,
                    &app.quote,
                    width.max(30),
                    app.glyph_seed,
                    true,
                    app.demo,
                    &app.cfg,
                );
                let lines = crate::compose_logo(&logo, lines, gap);
                draw_readout(frame, chunks[0], app, lines);
            } else {
                let text: Vec<RLine> = logo
                    .lines
                    .iter()
                    .map(|line| RLine::from(RSpan::styled(line.text.clone(), util::fg(pal.accent))))
                    .collect();
                frame.render_widget(
                    Paragraph::new(Text::from(text)).block(panel("text logo", app)),
                    chunks[0],
                );
            }
        } else {
            frame.render_widget(
                Paragraph::new(Text::from(vec![RLine::from(RSpan::styled(
                    "  text logo: font not resolvable — pick a font on Deck",
                    util::fg(pal.warn),
                ))]))
                .block(panel("live readout", app)),
                chunks[0],
            );
        }
    } else {
        let body = if chunks[0].width >= 92 {
            let split =
                Layout::horizontal([Constraint::Min(50), Constraint::Length(36)]).split(chunks[0]);
            (split[0], Some(split[1]))
        } else {
            (chunks[0], None)
        };
        app.artwork_pane = body.1.map(|pane| {
            pane.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 1,
            })
        });
        if let Some(art) = body.1 {
            let block = panel("artwork", app);
            if app.artwork.is_some() && crate::in_kitty() {
                frame.render_widget(block, art);
            } else {
                let note = app
                    .artwork
                    .as_ref()
                    .and_then(|path| {
                        path.file_name()
                            .map(|name| name.to_string_lossy().to_string())
                    })
                    .unwrap_or_else(|| "no artwork selected".to_string());
                let hint = if crate::in_kitty() {
                    "kitty image"
                } else {
                    "image renders in kitty"
                };
                let text = Text::from(vec![
                    RLine::from(""),
                    RLine::from(RSpan::styled(format!("  {}", note), util::fg(pal.fg))),
                    RLine::from(RSpan::styled(format!("  {}", hint), util::fg(pal.dim))),
                ]);
                frame.render_widget(Paragraph::new(text).block(block), art);
            }
        }
        let width = body.0.width.saturating_sub(2) as usize;
        let lines = crate::render(
            &app.theme,
            &app.data,
            &app.quote,
            width.max(20),
            app.glyph_seed,
            true,
            app.demo,
            &app.cfg,
        );
        draw_readout(frame, body.0, app, lines);
    }

    let graphs = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);
    let memory = Sparkline::default()
        .data(&app.history_mem)
        .max(100)
        .style(util::fg(pal.accent));
    frame.render_widget(
        memory.block(Block::default().title(RSpan::styled(" memory %", util::fg(pal.dim)))),
        graphs[0],
    );
    let swap = Sparkline::default()
        .data(&app.history_swap)
        .max(100)
        .style(util::fg(pal.accent2));
    frame.render_widget(
        swap.block(Block::default().title(RSpan::styled(" swap %", util::fg(pal.dim)))),
        graphs[1],
    );
}

fn draw_readout(frame: &mut Frame, area: Rect, app: &mut App, lines: Vec<crate::Line>) {
    let view_height = area.height.saturating_sub(2) as usize;
    let max_scroll = lines.len().saturating_sub(view_height);
    if app.scroll > max_scroll {
        app.scroll = max_scroll;
    }
    let paragraph = Paragraph::new(Text::from(util::lines(&lines)))
        .block(panel("live readout", app))
        .scroll((app.scroll as u16, 0));
    frame.render_widget(paragraph, area);
}

fn keys(frame: &mut Frame, area: Rect, app: &App) {
    let pal = app.theme.pal;
    let rows = [
        ("h l / ← →", "previous · next page"),
        ("1-8 / tab", "jump · cycle pages"),
        ("j k / ↑ ↓", "move · preview scroll"),
        ("+ -", "adjust value or cycle glyph"),
        ("space", "toggle · pin/random · mark a set"),
        ("enter", "glyph library · open picker · apply"),
        ("e / g / G", "type a glyph value · library · pool from font"),
        ("shift+enter / m", "pick up · move with j/k · drop"),
        ("p", "pin the current glyph · copy palette"),
        ("r", "randomize all glyphs · refresh data"),
        ("a", "add a photo to the album"),
        ("x", "clear the current override"),
        ("R", "reset: setting · page · config"),
        ("s", "save to fetch.conf"),
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
        "  every key in fetch.conf is reachable here; comments and unknown lines survive save",
        util::fg(pal.dim),
    )));
    frame.render_widget(
        Paragraph::new(Text::from(text)).block(panel("keys", app)),
        area,
    );
}

fn picker_popup(frame: &mut Frame, area: Rect, app: &App, picker: &super::Picker) {
    let pal = app.theme.pal;
    let width = 78.min(area.width.saturating_sub(4)).max(30);
    let height = 24.min(area.height.saturating_sub(4)).max(8);
    let popup = util::centered(area, width, height);
    frame.render_widget(Clear, popup);
    let block = panel(&picker.title, app);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(inner);

    let filter = RLine::from(vec![
        RSpan::styled(" / ", util::fg(pal.accent2)),
        RSpan::styled(picker.filter.clone(), util::fg(pal.fg)),
        RSpan::styled("▏", util::fg(pal.accent)),
    ]);
    frame.render_widget(Paragraph::new(filter), rows[0]);

    let visible = picker.visible();
    let items: Vec<ListItem> = visible
        .iter()
        .map(|index| {
            let item = &picker.items[*index];
            let label = if matches!(picker.kind, PickerKind::ThemeSet | PickerKind::GlyphPool) {
                format!("[{}] {}", if item.checked { "x" } else { " " }, item.label)
            } else {
                item.label.clone()
            };
            ListItem::new(RLine::from(RSpan::styled(label, util::fg(pal.fg))))
        })
        .collect();
    if items.is_empty() {
        frame.render_widget(
            Paragraph::new(RSpan::styled("  no matches", util::fg(pal.dim))),
            rows[1],
        );
    } else {
        let list = List::new(items).highlight_style(
            util::bg(pal.accent)
                .fg(ratatui::style::Color::Rgb(pal.bg.0, pal.bg.1, pal.bg.2))
                .add_modifier(Modifier::BOLD),
        );
        let mut state = ListState::default();
        state.select(Some(picker.cursor.min(visible.len().saturating_sub(1))));
        frame.render_stateful_widget(list, rows[1], &mut state);
    }

    let hint = match picker.kind {
        PickerKind::ThemeSet => " space mark · ↑↓ move · enter close · esc cancel",
        PickerKind::GlyphPool => " space mark · ↑↓ move · enter add · ctrl-f font · esc cancel",
        PickerKind::Glyph(_) => " ↑↓ move · type filter · enter pin · ctrl-f font · esc cancel",
        _ => " ↑↓ move · type to filter · enter pick · esc cancel",
    };
    frame.render_widget(
        Paragraph::new(RSpan::styled(hint, util::fg(pal.dim))),
        rows[2],
    );
}

fn edit_popup(frame: &mut Frame, area: Rect, app: &App) {
    let pal = app.theme.pal;
    let Some(edit) = &app.edit else {
        return;
    };
    let popup = util::centered(area, 60.min(area.width.saturating_sub(4)), 3);
    frame.render_widget(Clear, popup);
    let title = match edit.target {
        EditTarget::Image => "image path".to_string(),
        EditTarget::Glyph(row) => format!("glyph · {}", app.glyph_label(row)),
        EditTarget::Rule => "rule character".to_string(),
        EditTarget::Photo => "photo path".to_string(),
        EditTarget::LogoText => "logo text".to_string(),
        EditTarget::Pool => "glyph pool".to_string(),
        EditTarget::Quote => "quote text".to_string(),
    };
    let line = RLine::from(vec![
        RSpan::styled(" ", util::fg(pal.fg)),
        RSpan::styled(
            edit.text.clone(),
            util::fg(pal.fg).add_modifier(Modifier::BOLD),
        ),
        RSpan::styled("▏", util::fg(pal.accent)),
    ]);
    frame.render_widget(Paragraph::new(line).block(panel(&title, app)), popup);
}

fn color_popup(frame: &mut Frame, area: Rect, app: &App) {
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
        Paragraph::new(Text::from(lines))
            .block(panel(&format!("color · {}", COLOR_NAMES[edit.row]), app)),
        popup,
    );
}

fn reset_popup(frame: &mut Frame, area: Rect, app: &App) {
    let pal = app.theme.pal;
    let Some(prompt) = &app.reset else {
        return;
    };
    let popup = util::centered(area, 58.min(area.width.saturating_sub(4)), 7);
    frame.render_widget(Clear, popup);
    let options = [
        "reset the current setting",
        "reset this whole page",
        "reset the entire config",
    ];
    let mut lines = Vec::new();
    for (index, text) in options.iter().enumerate() {
        let selected = index == prompt.scope;
        let style = if selected {
            util::fg(pal.accent).add_modifier(Modifier::BOLD)
        } else {
            util::fg(pal.dim)
        };
        let marker = if selected { "▶" } else { " " };
        lines.push(RLine::from(RSpan::styled(
            format!(" {} {} ", marker, text),
            style,
        )));
    }
    lines.push(RLine::from(RSpan::styled(
        "  j/k choose · 1-3 · enter apply · esc cancel",
        util::fg(pal.dim),
    )));
    frame.render_widget(
        Paragraph::new(Text::from(lines)).block(panel("reset", app)),
        popup,
    );
}
