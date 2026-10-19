use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::util::adjust;
use super::{App, ColorEdit, Edit, EditTarget, PAGE_TITLES, RULES};

pub fn handle(app: &mut App, key: KeyEvent) -> bool {
    if app.color_edit.is_some() {
        color_key(app, key);
        return false;
    }
    if app.edit.is_some() {
        edit_key(app, key);
        return false;
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
        return true;
    }

    match key.code {
        KeyCode::Char('q') => return true,
        KeyCode::Tab => app.page = (app.page + 1) % PAGE_TITLES.len(),
        KeyCode::BackTab => app.page = (app.page + PAGE_TITLES.len() - 1) % PAGE_TITLES.len(),
        KeyCode::Char(value @ '1'..='7') => {
            app.page = value as usize - '1' as usize;
        }
        KeyCode::Char('s') => app.save(),
        KeyCode::Char('r') => app.refresh(),
        KeyCode::Char('j') | KeyCode::Down => {
            if app.page == 5 {
                app.scroll = app.scroll.saturating_add(1);
            } else {
                move_cursor(app, 1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.page == 5 {
                app.scroll = app.scroll.saturating_sub(1);
            } else {
                move_cursor(app, -1);
            }
        }
        KeyCode::PageDown => app.scroll = app.scroll.saturating_add(10),
        KeyCode::PageUp => app.scroll = app.scroll.saturating_sub(10),
        KeyCode::Char('g') if app.page == 5 => app.scroll = 0,
        KeyCode::Char('G') if app.page == 5 => app.scroll = 999,
        KeyCode::Right | KeyCode::Char('l') => adjust_value(app, 1),
        KeyCode::Left | KeyCode::Char('h') => adjust_value(app, -1),
        KeyCode::Enter => activate(app),
        KeyCode::Char(' ') if app.page == 0 => toggle_module(app),
        KeyCode::Char('x') => clear_value(app),
        KeyCode::Char('a') if app.page == 4 => {
            app.edit = Some(Edit { target: EditTarget::Photo, text: String::new() });
        }
        KeyCode::Char('p') if app.page == 2 => copy_palette(app),
        _ => {}
    }
    false
}

fn page_len(app: &App) -> usize {
    match app.page {
        0 => crate::MODULES.len(),
        1 => 6,
        2 => 11,
        3 => 7,
        4 => 6,
        _ => 0,
    }
}

fn move_cursor(app: &mut App, delta: i64) {
    let len = page_len(app) as i64;
    if len == 0 {
        return;
    }
    let current = app.cursors[app.page] as i64;
    app.cursors[app.page] = (current + delta).clamp(0, len - 1) as usize;
}

fn toggle_module(app: &mut App) {
    let (id, _) = crate::MODULES[app.cursors[0]];
    let list = app.enabled_list();
    if let Some(position) = list.iter().position(|item| item == id) {
        list.remove(position);
    } else {
        list.push(id.to_string());
        let order: Vec<String> = crate::MODULES.iter().map(|(id, _)| id.to_string()).collect();
        list.sort_by_key(|item| order.iter().position(|known| known == item).unwrap_or(usize::MAX));
    }
    let len = list.len();
    app.dirty = true;
    app.status = format!("modules {}/{}", len, crate::MODULES.len());
}

fn glyph_index(app: &App) -> usize {
    let current = app.cfg.glyph.clone().unwrap_or_else(|| "random".to_string());
    crate::GLYPHS.iter().position(|choice| *choice == current).unwrap_or(0)
}

fn adjust_value(app: &mut App, delta: i64) {
    let cursor = app.cursors[app.page];
    match app.page {
        1 => {
            app.dirty = true;
            match cursor {
                0 => {
                    let current = app.cfg.layout.clone().unwrap_or_else(|| app.theme.layout.clone());
                    app.cfg.layout = Some(if current == "hud" { "stack" } else { "hud" }.to_string());
                }
                1 => {
                    let index = super::util::wrap_index(glyph_index(app), crate::GLYPHS.len(), delta) as usize;
                    app.cfg.glyph = Some(crate::GLYPHS[index].to_string());
                }
                2 => {
                    let current = app.cfg.motion.clone().unwrap_or_else(|| app.theme.motion.clone());
                    app.cfg.motion = Some(if current == "reveal" { "off" } else { "reveal" }.to_string());
                }
                3 => {
                    let current = app.cfg.rule_char();
                    let index = RULES.iter().position(|item| *item == current).unwrap_or(0);
                    let index = super::util::wrap_index(index, RULES.len(), delta) as usize;
                    app.cfg.rule = Some(RULES[index].to_string());
                }
                4 => {
                    let current = app.cfg.units.clone().unwrap_or_else(|| "binary".to_string());
                    app.cfg.units = Some(if current == "binary" { "si" } else { "binary" }.to_string());
                    app.refresh();
                }
                _ => {
                    let current = app.cfg.temp_unit.clone().unwrap_or_else(|| "c".to_string());
                    app.cfg.temp_unit = Some(if current == "c" { "f" } else { "c" }.to_string());
                    app.refresh();
                }
            }
        }
        2 => {
            if cursor == 0 {
                cycle_theme(app, delta);
            } else if cursor == 1 {
                let current = app.cfg.variant.clone().unwrap_or_else(|| app.theme.mode.clone());
                app.cfg.variant = Some(if current == "dark" { "light" } else { "dark" }.to_string());
                app.dirty = true;
                app.rebuild_theme();
            }
        }
        3 => {
            let slot: &mut Option<u8> = match cursor {
                0 => &mut app.cfg.memory_warn,
                1 => &mut app.cfg.swap_warn,
                2 => &mut app.cfg.disk_warn,
                3 => &mut app.cfg.battery_low,
                4 => &mut app.cfg.temp_warn,
                5 => &mut app.cfg.temp_crit,
                _ => &mut app.cfg.gpu_warn,
            };
            let default = [85u8, 50, 90, 15, 85, 95, 95][cursor.min(6)];
            let current = slot.unwrap_or(default) as i64;
            *slot = Some(adjust(current, delta, 0, 100));
            app.dirty = true;
        }
        4 => {
            match cursor {
                2 => {
                    app.cfg.logo_width = Some(adjust(app.cfg.logo_width.unwrap_or(34) as i64, delta, 10, 90));
                }
                3 => {
                    app.cfg.logo_min = Some(adjust(app.cfg.logo_min.unwrap_or(26) as i64, delta, 8, 80));
                }
                4 => {
                    app.cfg.logo_max = Some(adjust(app.cfg.logo_max.unwrap_or(46) as i64, delta, 8, 90));
                }
                5 => {
                    app.cfg.logo_gap = Some(adjust(app.cfg.logo_gap.unwrap_or(3) as i64, delta, 0, 20));
                }
                _ => {}
            }
            if cursor >= 2 {
                app.dirty = true;
            }
        }
        _ => {}
    }
}

fn activate(app: &mut App) {
    let cursor = app.cursors[app.page];
    match app.page {
        0 => toggle_module(app),
        1 => match cursor {
            1 => app.edit = Some(Edit { target: EditTarget::Glyph, text: app.cfg.glyph.clone().unwrap_or_default() }),
            3 => app.edit = Some(Edit { target: EditTarget::Rule, text: app.cfg.rule_char().to_string() }),
            _ => adjust_value(app, 1),
        },
        2 => {
            match cursor {
                0 => cycle_theme(app, 1),
                1 => adjust_value(app, 1),
                2..=9 => {
                    app.color_edit = Some(ColorEdit { row: cursor - 2, channel: 0 });
                }
                _ => {
                    for index in 0..8 {
                        app.set_color(index, None);
                    }
                    app.dirty = true;
                    app.rebuild_theme();
                    app.status = "color overrides cleared".to_string();
                }
            }
        }
        3 => adjust_value(app, 1),
        4 => {
            if cursor == 0 {
                let text = app.cfg.image.as_ref().map(|path| path.display().to_string()).unwrap_or_default();
                app.edit = Some(Edit { target: EditTarget::Image, text });
            } else if cursor == 1 {
                cycle_album(app, 1);
            } else {
                adjust_value(app, 1);
            }
        }
        _ => {}
    }
}

fn clear_value(app: &mut App) {
    match app.page {
        2 => {
            let cursor = app.cursors[2];
            if (2..=9).contains(&cursor) {
                let index = cursor - 2;
                app.set_color(index, None);
                app.dirty = true;
                app.rebuild_theme();
            }
        }
        4 => {
            if app.cursors[4] == 0 {
                app.cfg.image = None;
                app.dirty = true;
                app.reload_artwork();
            }
        }
        _ => {}
    }
}

fn copy_palette(app: &mut App) {
    let pal = app.theme.pal;
    for (index, color) in [
        pal.bg, pal.fg, pal.dim, pal.accent, pal.accent2, pal.ok, pal.warn, pal.err,
    ]
    .iter()
    .enumerate()
    {
        app.set_color(index, Some(*color));
    }
    app.dirty = true;
    app.rebuild_theme();
    app.status = "palette copied to overrides".to_string();
}

fn cycle_theme(app: &mut App, delta: i64) {
    let names = app.palette_names();
    if names.is_empty() {
        return;
    }
    let current = app
        .cfg
        .theme
        .clone()
        .unwrap_or_else(|| app.theme.name.clone());
    let index = names.iter().position(|name| *name == current).unwrap_or(0);
    let index = super::util::wrap_index(index, names.len(), delta) as usize;
    app.cfg.theme = Some(names[index].clone());
    app.dirty = true;
    app.rebuild_theme();
    app.status = format!("theme {}", names[index]);
}

fn cycle_album(app: &mut App, delta: i64) {
    let mut choices = vec![String::new()];
    choices.extend(app.albums());
    let current = app.cfg.album.clone().unwrap_or_default();
    let index = choices.iter().position(|item| *item == current).unwrap_or(0);
    let index = super::util::wrap_index(index, choices.len(), delta) as usize;
    app.cfg.album = if choices[index].is_empty() { None } else { Some(choices[index].clone()) };
    app.dirty = true;
    app.reload_artwork();
}

fn color_key(app: &mut App, key: KeyEvent) {
    let Some(mut edit) = app.color_edit.take() else {
        return;
    };
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::Char('q') => return,
        KeyCode::Char('x') => {
            app.set_color(edit.row, None);
            app.dirty = true;
            app.rebuild_theme();
            return;
        }
        KeyCode::Down | KeyCode::Char('j') => edit.channel = (edit.channel + 1) % 3,
        KeyCode::Up | KeyCode::Char('k') => edit.channel = (edit.channel + 2) % 3,
        KeyCode::Right | KeyCode::Char('l') | KeyCode::Left | KeyCode::Char('h') => {
            let step = 1;
            let delta = if matches!(key.code, KeyCode::Left | KeyCode::Char('h')) { -step } else { step };
            let mut color = app.effective_color(edit.row);
            match edit.channel {
                0 => color.0 = adjust(color.0 as i64, delta, 0, 255),
                1 => color.1 = adjust(color.1 as i64, delta, 0, 255),
                _ => color.2 = adjust(color.2 as i64, delta, 0, 255),
            }
            app.set_color(edit.row, Some(color));
            app.dirty = true;
            app.rebuild_theme();
        }
        _ => {}
    }
    app.color_edit = Some(edit);
}

fn edit_key(app: &mut App, key: KeyEvent) {
    let Some(mut edit) = app.edit.take() else {
        return;
    };
    match key.code {
        KeyCode::Esc => return,
        KeyCode::Enter => {
            let text = edit.text.trim().to_string();
            match edit.target {
                EditTarget::Image => {
                    app.cfg.image = if text.is_empty() { None } else { Some(std::path::PathBuf::from(text)) };
                    app.reload_artwork();
                }
                EditTarget::Glyph => {
                    app.cfg.glyph = text.chars().next().map(|value| value.to_string());
                }
                EditTarget::Rule => {
                    app.cfg.rule = text.chars().next().map(|value| value.to_string());
                }
                EditTarget::Photo => {
                    let album = app.cfg.album.clone().unwrap_or_else(|| "default".to_string());
                    match crate::add_photo(&album, std::path::Path::new(&text)) {
                        Ok(path) => app.status = format!("added {}", path.display()),
                        Err(error) => app.status = error,
                    }
                }
            }
            app.dirty = true;
            app.status = "value updated".to_string();
            return;
        }
        KeyCode::Backspace => {
            edit.text.pop();
        }
        KeyCode::Char(value) => edit.text.push(value),
        _ => {}
    }
    app.edit = Some(edit);
}
