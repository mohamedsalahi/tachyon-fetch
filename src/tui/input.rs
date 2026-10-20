use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::util::adjust;
use super::{
    App, ColorEdit, Edit, EditTarget, PickerKind, ResetPrompt, ARTWORK_ROWS, DECK_ROWS,
    GLYPH_FIXED_ROWS, PAGE_ARTWORK, PAGE_DECK, PAGE_GLYPHS, PAGE_MODULES, PAGE_PALETTE,
    PAGE_PREVIEW, PAGE_THRESHOLDS, PAGE_TITLES, PALETTE_ROWS, RULES,
};

pub fn handle(app: &mut App, key: KeyEvent) -> bool {
    if app.picker.is_some() {
        picker_key(app, key);
        return false;
    }
    if app.reset.is_some() {
        reset_key(app, key);
        return false;
    }
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

    if app.picked.is_some() {
        match key.code {
            KeyCode::Esc => {
                cancel_pick(app);
                return false;
            }
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                drop_pick(app);
                return false;
            }
            KeyCode::Char('m') => {
                drop_pick(app);
                return false;
            }
            KeyCode::Char('j') | KeyCode::Down | KeyCode::Char('J') => {
                move_picked(app, 1);
                return false;
            }
            KeyCode::Char('k') | KeyCode::Up | KeyCode::Char('K') => {
                move_picked(app, -1);
                return false;
            }
            _ => {}
        }
    }

    match key.code {
        KeyCode::Char('q') if app.picked.is_none() => return true,
        KeyCode::Enter
            if app.page == PAGE_MODULES && key.modifiers.contains(KeyModifiers::SHIFT) =>
        {
            pick_module(app);
        }
        KeyCode::Char('m') if app.page == PAGE_MODULES => {
            if app.picked.is_some() {
                drop_pick(app);
            } else {
                pick_module(app);
            }
        }
        KeyCode::Tab | KeyCode::Char('l') | KeyCode::Right => {
            app.page = (app.page + 1) % PAGE_TITLES.len();
        }
        KeyCode::BackTab | KeyCode::Char('h') | KeyCode::Left => {
            app.page = (app.page + PAGE_TITLES.len() - 1) % PAGE_TITLES.len();
        }
        KeyCode::Char(value @ '1'..='8') => {
            app.page = value as usize - '1' as usize;
        }
        KeyCode::Char('s') => app.save(),
        KeyCode::Char('r') => {
            if app.page == PAGE_GLYPHS {
                randomize_all_glyphs(app);
            } else {
                app.refresh();
            }
        }
        KeyCode::Char('R') => app.reset = Some(ResetPrompt { scope: 0 }),
        KeyCode::Char('j') | KeyCode::Down => {
            if app.page == PAGE_PREVIEW {
                app.scroll = app.scroll.saturating_add(1);
            } else {
                move_cursor(app, 1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.page == PAGE_PREVIEW {
                app.scroll = app.scroll.saturating_sub(1);
            } else {
                move_cursor(app, -1);
            }
        }
        KeyCode::Char('J') if app.page == PAGE_MODULES => move_module(app, 1),
        KeyCode::Char('K') if app.page == PAGE_MODULES => move_module(app, -1),
        KeyCode::PageDown => app.scroll = app.scroll.saturating_add(10),
        KeyCode::PageUp => app.scroll = app.scroll.saturating_sub(10),
        KeyCode::Char('g') if app.page == PAGE_PREVIEW => app.scroll = 0,
        KeyCode::Char('G') if app.page == PAGE_PREVIEW => app.scroll = 999,
        KeyCode::Char('+') | KeyCode::Char('=') => adjust_value(app, 1),
        KeyCode::Char('-') => adjust_value(app, -1),
        KeyCode::Enter => activate(app),
        KeyCode::Char(' ') => space(app),
        KeyCode::Char('x') => app.reset_current(),
        KeyCode::Char('i') if app.page == PAGE_ARTWORK => {
            app.edit = Some(Edit {
                target: EditTarget::Image,
                text: app
                    .cfg
                    .image
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
            });
        }
        KeyCode::Char('e') if app.page == PAGE_GLYPHS && app.cursors[PAGE_GLYPHS] == 5 => {
            app.edit = Some(Edit {
                target: EditTarget::Pool,
                text: app.cfg.glyph_pool().to_string(),
            });
        }
        KeyCode::Char('e') if app.page == PAGE_GLYPHS => {
            let cursor = app.cursors[PAGE_GLYPHS];
            app.edit = Some(Edit {
                target: EditTarget::Glyph(cursor),
                text: app.glyph_spec(cursor),
            });
        }
        KeyCode::Char('g') if app.page == PAGE_GLYPHS => {
            let cursor = app.cursors[PAGE_GLYPHS];
            app.open_glyph_source(if cursor == 5 { None } else { Some(cursor) });
        }
        KeyCode::Char('G') if app.page == PAGE_GLYPHS => app.open_glyph_source(None),
        KeyCode::Char('a') if app.page == PAGE_ARTWORK => {
            app.edit = Some(Edit {
                target: EditTarget::Photo,
                text: String::new(),
            });
        }
        KeyCode::Char('p') => match app.page {
            PAGE_PALETTE => copy_palette(app),
            PAGE_GLYPHS => pin_glyph(app),
            _ => {}
        },
        _ => {}
    }
    false
}

fn page_len(app: &App) -> usize {
    match app.page {
        PAGE_MODULES => crate::MODULES.len(),
        PAGE_DECK => DECK_ROWS,
        PAGE_GLYPHS => GLYPH_FIXED_ROWS + crate::MODULES.len(),
        PAGE_PALETTE => PALETTE_ROWS,
        PAGE_THRESHOLDS => 7,
        PAGE_ARTWORK => ARTWORK_ROWS,
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
    let Some((id, _)) = app.module_order().get(app.cursors[PAGE_MODULES]).copied() else {
        return;
    };
    let list = app.enabled_list();
    if let Some(position) = list.iter().position(|item| item == id) {
        list.remove(position);
    } else {
        list.push(id.to_string());
    }
    let len = list.len();
    app.dirty = true;
    app.status = format!("modules {}/{}", len, crate::MODULES.len());
}

fn pick_module(app: &mut App) {
    let Some((id, _)) = app.module_order().get(app.cursors[PAGE_MODULES]).copied() else {
        return;
    };
    let list = app.enabled_list();
    let Some(position) = list.iter().position(|item| item == id) else {
        return;
    };
    app.picked = Some((id.to_string(), position));
    app.dirty = true;
    app.status = format!("moving {} — shift+enter drops, esc cancels", id);
}

fn drop_pick(app: &mut App) {
    let Some((id, position)) = app.picked.take() else {
        return;
    };
    app.status = format!("{} placed at {}", id, position + 1);
}

fn cancel_pick(app: &mut App) {
    let Some((id, position)) = app.picked.take() else {
        return;
    };
    let list = app.enabled_list();
    if let Some(current) = list.iter().position(|item| *item == id) {
        list.remove(current);
        let target = position.min(list.len());
        list.insert(target, id.clone());
        app.cursors[PAGE_MODULES] = target;
    }
    app.status = format!("{} returned to {}", id, position + 1);
}

fn move_picked(app: &mut App, delta: i64) {
    let Some((id, position)) = app.picked.clone() else {
        return;
    };
    let list = app.enabled_list();
    let target = position as i64 + delta;
    if target < 0 || target >= list.len() as i64 {
        return;
    }
    list.swap(position, target as usize);
    app.picked = Some((id, target as usize));
    app.cursors[PAGE_MODULES] = target as usize;
    app.dirty = true;
}

fn move_module(app: &mut App, delta: i64) {
    let Some(id) = app
        .module_order()
        .get(app.cursors[PAGE_MODULES])
        .map(|(id, _)| id.to_string())
    else {
        return;
    };
    let list = app.enabled_list();
    let Some(position) = list.iter().position(|item| *item == id) else {
        return;
    };
    let target = position as i64 + delta;
    if target < 0 || target >= list.len() as i64 {
        return;
    }
    list.swap(position, target as usize);
    app.cursors[PAGE_MODULES] = target as usize;
    app.dirty = true;
    app.status = format!("{} moved to {}", id, target + 1);
}

fn randomize_all_glyphs(app: &mut App) {
    let ids: Vec<String> = crate::MODULES
        .iter()
        .map(|(id, _)| id.to_string())
        .collect();
    for id in ids {
        app.cfg.glyphs.insert(id, "random".to_string());
    }
    app.dirty = true;
    app.status = "every module glyph is random".to_string();
}

fn pin_glyph(app: &mut App) {
    let row = app.cursors[PAGE_GLYPHS];
    if row == 4 || row == 5 {
        return;
    }
    let display = app.glyph_display(row);
    let Some(glyph) = display.chars().next() else {
        return;
    };
    match app.set_glyph_spec(row, &glyph.to_string()) {
        Ok(()) => app.status = format!("pinned {}", glyph),
        Err(error) => app.status = error,
    }
}

fn cycle_choice(field: &mut Option<String>, values: &[&str], current: &str, delta: i64) {
    let index = values
        .iter()
        .position(|value| *value == current)
        .unwrap_or(0);
    let index = super::util::wrap_index(index, values.len(), delta);
    *field = Some(values[index].to_string());
}

fn adjust_value(app: &mut App, delta: i64) {
    let cursor = app.cursors[app.page];
    match app.page {
        PAGE_DECK => {
            app.dirty = true;
            match cursor {
                0 => {
                    let current = app
                        .cfg
                        .layout
                        .clone()
                        .unwrap_or_else(|| app.theme.layout.clone());
                    cycle_choice(&mut app.cfg.layout, &["hud", "stack"], &current, delta);
                }
                1 => {
                    let current = app
                        .cfg
                        .motion
                        .clone()
                        .unwrap_or_else(|| app.theme.motion.clone());
                    cycle_choice(&mut app.cfg.motion, &["reveal", "off"], &current, delta);
                }
                2 => {
                    let current = app.cfg.rule_char();
                    let index = RULES.iter().position(|item| *item == current).unwrap_or(0);
                    let index = super::util::wrap_index(index, RULES.len(), delta);
                    app.cfg.rule = Some(RULES[index].to_string());
                }
                3 => {
                    let current = app
                        .cfg
                        .units
                        .clone()
                        .unwrap_or_else(|| "binary".to_string());
                    cycle_choice(&mut app.cfg.units, &["binary", "si"], &current, delta);
                    app.refresh();
                }
                4 => {
                    let current = app.cfg.temp_unit.clone().unwrap_or_else(|| "c".to_string());
                    cycle_choice(&mut app.cfg.temp_unit, &["c", "f"], &current, delta);
                    app.refresh();
                }
                5 => {
                    let current = app.cfg.logo_mode().to_string();
                    cycle_choice(&mut app.cfg.logo_mode, &["image", "text"], &current, delta);
                    app.reload_artwork();
                    app.status = format!("logo mode {}", app.cfg.logo_mode());
                }
                8 => {
                    app.cfg.logo_size = Some(adjust(app.cfg.logo_size_px() as i64, delta, 16, 200));
                }
                9 => {
                    let current = app.cfg.logo_tint().to_string();
                    cycle_choice(
                        &mut app.cfg.logo_tint,
                        &["duo", "accent", "accent2", "fg"],
                        &current,
                        delta,
                    );
                }
                10 => {
                    app.cfg.logo_width = Some(adjust(
                        app.cfg.logo_width.unwrap_or(34) as i64,
                        delta,
                        10,
                        90,
                    ));
                }
                11 => {
                    app.cfg.logo_min =
                        Some(adjust(app.cfg.logo_min.unwrap_or(26) as i64, delta, 8, 80));
                }
                12 => {
                    app.cfg.logo_max =
                        Some(adjust(app.cfg.logo_max.unwrap_or(46) as i64, delta, 8, 90));
                }
                13 => {
                    app.cfg.logo_gap =
                        Some(adjust(app.cfg.logo_gap.unwrap_or(3) as i64, delta, 0, 20));
                }
                _ => {}
            }
        }
        PAGE_GLYPHS => {
            if cursor != 5 {
                if cursor == 4 {
                    let current = app.cfg.glyph_row_mode().to_string();
                    cycle_choice(
                        &mut app.cfg.glyph_row,
                        &["auto", "off", "random"],
                        &current,
                        delta,
                    );
                } else {
                    app.cycle_glyph(cursor, delta);
                }
            }
        }
        PAGE_PALETTE => {
            if cursor == 0 {
                cycle_theme(app, delta);
            } else if cursor == 1 {
                let current = app
                    .cfg
                    .variant
                    .clone()
                    .unwrap_or_else(|| app.theme.mode.clone());
                cycle_choice(&mut app.cfg.variant, &["dark", "light"], &current, delta);
                app.dirty = true;
                app.rebuild_theme();
                app.prune_rotation_set();
            } else if cursor == 2 {
                let current = app.cfg.rotate_palette_mode().to_string();
                cycle_choice(
                    &mut app.cfg.rotate_palette,
                    &["off", "next", "shuffle"],
                    &current,
                    delta,
                );
                app.dirty = true;
            }
        }
        PAGE_THRESHOLDS => {
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
        PAGE_ARTWORK => {
            if cursor == 2 {
                let current = app.cfg.rotate_art_mode().to_string();
                cycle_choice(
                    &mut app.cfg.rotate_art,
                    &["off", "next", "shuffle"],
                    &current,
                    delta,
                );
                app.dirty = true;
                app.reload_artwork();
            } else if cursor == 3 {
                let current = app.cfg.rotate_every_mode().to_string();
                cycle_choice(
                    &mut app.cfg.rotate_every,
                    &["run", "hour", "day"],
                    &current,
                    delta,
                );
                app.dirty = true;
            }
        }
        _ => {}
    }
}

fn activate(app: &mut App) {
    let cursor = app.cursors[app.page];
    match app.page {
        PAGE_MODULES => toggle_module(app),
        PAGE_DECK => match cursor {
            2 => {
                app.edit = Some(Edit {
                    target: EditTarget::Rule,
                    text: app.cfg.rule_char().to_string(),
                })
            }
            6 => {
                app.edit = Some(Edit {
                    target: EditTarget::LogoText,
                    text: app.cfg.logo_text.clone().unwrap_or_default(),
                })
            }
            7 => app.open_picker(PickerKind::Font),
            14 => app.open_picker(PickerKind::Quote),
            _ => adjust_value(app, 1),
        },
        PAGE_GLYPHS => {
            if cursor == 5 {
                app.open_glyph_source(None);
            } else {
                app.open_glyph_source(Some(cursor));
            }
        }
        PAGE_PALETTE => match cursor {
            0 => app.open_picker(PickerKind::Theme),
            1 => adjust_value(app, 1),
            2 => adjust_value(app, 1),
            3 => app.open_picker(PickerKind::ThemeSet),
            4..=11 => {
                app.color_edit = Some(ColorEdit {
                    row: cursor - 4,
                    channel: 0,
                });
            }
            _ => {
                for index in 0..8 {
                    app.set_color(index, None);
                }
                app.dirty = true;
                app.rebuild_theme();
                app.status = "color overrides cleared".to_string();
            }
        },
        PAGE_THRESHOLDS => adjust_value(app, 1),
        PAGE_ARTWORK => match cursor {
            0 => app.open_picker(PickerKind::Album),
            1 => {
                if app.cfg.album.is_some() {
                    app.open_picker(PickerKind::Photo);
                } else {
                    app.edit = Some(Edit {
                        target: EditTarget::Image,
                        text: app
                            .cfg
                            .image
                            .as_ref()
                            .map(|path| path.display().to_string())
                            .unwrap_or_default(),
                    });
                }
            }
            _ => adjust_value(app, 1),
        },
        _ => {}
    }
}

fn space(app: &mut App) {
    match app.page {
        PAGE_MODULES => toggle_module(app),
        PAGE_GLYPHS => app.toggle_glyph_mode(app.cursors[PAGE_GLYPHS]),
        PAGE_PALETTE if app.cursors[PAGE_PALETTE] == 3 => app.open_picker(PickerKind::ThemeSet),
        _ => {}
    }
}

fn picker_key(app: &mut App, key: KeyEvent) {
    let Some(mut picker) = app.picker.take() else {
        return;
    };
    if matches!(picker.kind, PickerKind::Glyph(_) | PickerKind::GlyphPool)
        && key.code == KeyCode::Char('f')
        && key.modifiers.contains(KeyModifiers::CONTROL)
    {
        app.open_picker(PickerKind::GlyphFont);
        return;
    }
    if matches!(picker.kind, PickerKind::ThemeSet | PickerKind::GlyphPool) {
        if let KeyCode::Char(' ') = key.code {
            let visible = picker.visible();
            if let Some(index) = visible.get(picker.cursor.min(visible.len().saturating_sub(1))) {
                picker.items[*index].checked = !picker.items[*index].checked;
                let len = visible.len();
                if len > 0 {
                    picker.cursor = (picker.cursor + 1).min(len - 1);
                }
            }
            app.picker = Some(picker);
            return;
        }
    }
    match key.code {
        KeyCode::Esc => {
            if picker.filter.is_empty() {
                app.status = "picker closed".to_string();
                return;
            }
            picker.filter.clear();
            picker.cursor = 0;
        }
        KeyCode::Enter => {
            if !select_picker(app, &mut picker) {
                app.picker = Some(picker);
            }
            return;
        }
        KeyCode::Down | KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let len = picker.visible().len();
            if len > 0 {
                picker.cursor = (picker.cursor + 1).min(len - 1);
            }
        }
        KeyCode::Down => {
            let len = picker.visible().len();
            if len > 0 {
                picker.cursor = (picker.cursor + 1).min(len - 1);
            }
        }
        KeyCode::Up => {
            picker.cursor = picker.cursor.saturating_sub(1);
        }
        KeyCode::PageDown => {
            let len = picker.visible().len();
            if len > 0 {
                picker.cursor = (picker.cursor + 5).min(len - 1);
            }
        }
        KeyCode::PageUp => {
            picker.cursor = picker.cursor.saturating_sub(5);
        }
        KeyCode::Backspace => {
            picker.filter.pop();
            picker.cursor = 0;
        }
        KeyCode::Char(value) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            picker.filter.push(value);
            picker.cursor = 0;
        }
        _ => {}
    }
    app.picker = Some(picker);
}

fn select_picker(app: &mut App, picker: &mut super::Picker) -> bool {
    match picker.kind {
        PickerKind::GlyphPool => {
            let chosen: String = picker
                .items
                .iter()
                .filter(|item| item.checked)
                .filter_map(|item| item.value.chars().next())
                .collect();
            let mut pool = app.cfg.glyph_pool().to_string();
            let mut added = 0usize;
            for ch in chosen.chars() {
                if !pool.contains(ch) && pool.chars().count() < 512 {
                    pool.push(ch);
                    added += 1;
                }
            }
            app.cfg.glyph_pool = Some(pool);
            app.dirty = true;
            app.status = format!("{} glyph(s) added to the pool", added);
            true
        }
        PickerKind::ThemeSet => {
            let chosen: Vec<String> = picker
                .items
                .iter()
                .filter(|item| item.checked)
                .map(|item| item.value.clone())
                .collect();
            let count = chosen.len();
            app.cfg.rotate_themes = if chosen.is_empty() {
                None
            } else {
                Some(chosen)
            };
            if app.cfg.rotate_palette.is_none() && count > 0 {
                app.cfg.rotate_palette = Some("next".to_string());
            }
            app.dirty = true;
            app.status = format!("rotation set: {} themes", count);
            true
        }
        _ => {
            let Some(item) = picker.selected().cloned() else {
                app.status = "no match".to_string();
                return false;
            };
            match picker.kind {
                PickerKind::Font => {
                    app.cfg.logo_font = Some(item.value.clone());
                    app.cfg.logo_mode = Some("text".to_string());
                    app.dirty = true;
                    app.reload_artwork();
                    app.status = format!("font {} · logo text mode", item.label);
                }
                PickerKind::Theme => {
                    app.cfg.theme = Some(item.value.clone());
                    app.cfg.rotate_palette = Some("off".to_string());
                    app.dirty = true;
                    app.rebuild_theme();
                    app.status = format!("theme pinned {}", item.label);
                }
                PickerKind::Album => {
                    app.cfg.album = Some(item.value.clone());
                    app.cfg.image = None;
                    app.dirty = true;
                    app.reload_artwork();
                    app.status = format!("album {}", item.label);
                    app.open_picker(PickerKind::Photo);
                }
                PickerKind::Photo => {
                    if item.value == "@rotate" {
                        app.cfg.image = None;
                        if app.cfg.rotate_art_mode() == "off" {
                            app.cfg.rotate_art = Some("next".to_string());
                        }
                        app.status = format!(
                            "rotating {} · {}",
                            app.cfg.album.as_deref().unwrap_or("album"),
                            app.cfg.rotate_art_mode()
                        );
                    } else {
                        app.cfg.image = Some(std::path::PathBuf::from(&item.value));
                        app.status = format!("pinned {}", item.label);
                    }
                    app.dirty = true;
                    app.reload_artwork();
                }
                PickerKind::Quote => {
                    match item.value.as_str() {
                        "@random" => {
                            app.cfg.quote = None;
                            app.status = "quote random".to_string();
                        }
                        "@off" => {
                            app.cfg.quote = Some("off".to_string());
                            app.status = "quote off".to_string();
                        }
                        "@custom" => {
                            app.edit = Some(Edit {
                                target: EditTarget::Quote,
                                text: app
                                    .cfg
                                    .quote
                                    .clone()
                                    .filter(|q| q != "off")
                                    .unwrap_or_default(),
                            });
                            app.refresh_quote();
                            return true;
                        }
                        quote => {
                            app.cfg.quote = Some(quote.to_string());
                            app.status = "quote pinned".to_string();
                        }
                    }
                    app.refresh_quote();
                    app.dirty = true;
                }
                PickerKind::GlyphFont => {
                    let Some(path) = crate::font::resolve_font(&item.value) else {
                        app.status = "font not resolvable".to_string();
                        return true;
                    };
                    app.load_glyph_source(item.label.clone(), path);
                }
                PickerKind::Glyph(row) => match app.set_glyph_spec(row, &item.value) {
                    Ok(()) => {
                        app.status = format!("glyph {} pinned", item.value);
                    }
                    Err(error) => app.status = error,
                },
                PickerKind::GlyphPool => {}
                PickerKind::ThemeSet => {}
            }
            true
        }
    }
}

fn reset_key(app: &mut App, key: KeyEvent) {
    let Some(mut prompt) = app.reset.take() else {
        return;
    };
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            app.status = "reset cancelled".to_string();
            return;
        }
        KeyCode::Enter => {
            match prompt.scope {
                0 => app.reset_current(),
                1 => app.reset_page(),
                _ => app.reset_all(),
            }
            return;
        }
        KeyCode::Char('1') => prompt.scope = 0,
        KeyCode::Char('2') => prompt.scope = 1,
        KeyCode::Char('3') => prompt.scope = 2,
        KeyCode::Char('j') | KeyCode::Down | KeyCode::Char('l') | KeyCode::Right => {
            prompt.scope = (prompt.scope + 1).min(2);
        }
        KeyCode::Char('k') | KeyCode::Up | KeyCode::Char('h') | KeyCode::Left => {
            prompt.scope = prompt.scope.saturating_sub(1);
        }
        _ => {}
    }
    app.reset = Some(prompt);
}

fn copy_palette(app: &mut App) {
    let pal = app.theme.pal;
    for (index, color) in [
        pal.bg,
        pal.fg,
        pal.dim,
        pal.accent,
        pal.accent2,
        pal.ok,
        pal.warn,
        pal.err,
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
    let mode = app.effective_mode();
    let names = app.themes_for_mode(&mode);
    if names.is_empty() {
        return;
    }
    let current = app
        .cfg
        .theme
        .clone()
        .unwrap_or_else(|| app.theme.name.clone());
    let index = names.iter().position(|name| *name == current).unwrap_or(0);
    let index = super::util::wrap_index(index, names.len(), delta);
    app.cfg.theme = Some(names[index].clone());
    app.dirty = true;
    app.rebuild_theme();
    app.status = format!("theme {}", names[index]);
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
            let delta = if matches!(key.code, KeyCode::Left | KeyCode::Char('h')) {
                -step
            } else {
                step
            };
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
                EditTarget::Quote => {
                    app.cfg.quote = if text.is_empty() || text == "random" {
                        None
                    } else {
                        Some(text)
                    };
                    app.refresh_quote();
                    app.dirty = true;
                    app.status = "quote updated".to_string();
                }
                EditTarget::LogoText => {
                    app.cfg.logo_text = if text.is_empty() { None } else { Some(text) };
                    app.dirty = true;
                    app.status = "logo text updated".to_string();
                }
                EditTarget::Pool => match app.set_glyph_spec(5, &text) {
                    Ok(()) => app.status = "glyph pool updated".to_string(),
                    Err(error) => {
                        app.status = error;
                        app.edit = Some(edit);
                        return;
                    }
                },
                EditTarget::Glyph(row) => match app.set_glyph_spec(row, &text) {
                    Ok(()) => app.status = "glyph updated".to_string(),
                    Err(error) => {
                        app.status = error;
                        app.edit = Some(edit);
                        return;
                    }
                },
                EditTarget::Rule => {
                    app.cfg.rule = text.chars().next().map(|value| value.to_string());
                    app.dirty = true;
                    app.status = "value updated".to_string();
                }
                EditTarget::Image => {
                    app.cfg.image = if text.is_empty() {
                        None
                    } else {
                        Some(std::path::PathBuf::from(text))
                    };
                    app.dirty = true;
                    app.reload_artwork();
                    app.status = "value updated".to_string();
                }
                EditTarget::Photo => {
                    let album = app
                        .cfg
                        .album
                        .clone()
                        .unwrap_or_else(|| "default".to_string());
                    match crate::add_photo(&album, std::path::Path::new(&text)) {
                        Ok(path) => {
                            app.cfg.album = Some(album);
                            app.cfg.image = Some(path.clone());
                            app.dirty = true;
                            app.reload_artwork();
                            app.status = format!("added {}", path.display());
                        }
                        Err(error) => app.status = error,
                    }
                }
            }
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
