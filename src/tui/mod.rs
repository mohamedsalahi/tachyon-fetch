use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{
    self, Event, KeyEventKind, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::Terminal;

use crate::{Config, FetchData, Opts, Palette, Rgb, Theme};

pub mod input;
pub mod util;
pub mod view;

pub const PAGE_TITLES: [&str; 8] = [
    "Modules",
    "Deck",
    "Glyphs",
    "Palette",
    "Thresholds",
    "Artwork",
    "Preview",
    "Keys",
];

pub const PAGE_MODULES: usize = 0;
pub const PAGE_DECK: usize = 1;
pub const PAGE_GLYPHS: usize = 2;
pub const PAGE_PALETTE: usize = 3;
pub const PAGE_THRESHOLDS: usize = 4;
pub const PAGE_ARTWORK: usize = 5;
pub const PAGE_PREVIEW: usize = 6;
pub const PAGE_KEYS: usize = 7;

pub const DECK_ROWS: usize = 15;
pub const GLYPH_FIXED_ROWS: usize = 6;
pub const PALETTE_ROWS: usize = 13;
pub const ARTWORK_ROWS: usize = 4;

pub const COLOR_NAMES: [&str; 8] = ["bg", "fg", "dim", "accent", "accent2", "ok", "warn", "err"];

pub const RULES: [char; 8] = ['─', '═', '┄', '┈', '·', '.', '━', '~'];

pub struct ColorEdit {
    pub row: usize,
    pub channel: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EditTarget {
    Image,
    Glyph(usize),
    Rule,
    Photo,
    LogoText,
    Pool,
    Quote,
}

pub struct Edit {
    pub target: EditTarget,
    pub text: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PickerKind {
    Font,
    Theme,
    Album,
    Photo,
    ThemeSet,
    Quote,
    GlyphFont,
    Glyph(usize),
    GlyphPool,
}

#[derive(Clone)]
pub struct PickerItem {
    pub label: String,
    pub value: String,
    pub checked: bool,
}

pub struct Picker {
    pub kind: PickerKind,
    pub title: String,
    pub items: Vec<PickerItem>,
    pub filter: String,
    pub cursor: usize,
}

impl Picker {
    pub fn visible(&self) -> Vec<usize> {
        let needle = self.filter.to_lowercase();
        let alias = needle.strip_prefix("nf-").map(str::to_string);
        let glyphs = matches!(self.kind, PickerKind::Glyph(_) | PickerKind::GlyphPool);
        self.items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if needle.is_empty() {
                    return true;
                }
                let label = item.label.to_lowercase();
                label.contains(&needle)
                    || (glyphs && alias.as_ref().is_some_and(|alias| label.contains(alias)))
            })
            .map(|(index, _)| index)
            .collect()
    }

    pub fn selected(&self) -> Option<&PickerItem> {
        let visible = self.visible();
        visible
            .get(self.cursor.min(visible.len().saturating_sub(1)))
            .and_then(|index| self.items.get(*index))
    }
}

pub struct App<'a> {
    pub opts: &'a Opts,
    pub cfg: Config,
    pub theme: Theme,
    pub palettes: Vec<(String, String, Palette, String, String)>,
    pub page: usize,
    pub cursors: [usize; 8],
    pub data: FetchData,
    pub quote: String,
    pub quotes: Vec<String>,
    pub demo: bool,
    pub status: String,
    pub dirty: bool,
    pub history_mem: Vec<u64>,
    pub history_swap: Vec<u64>,
    pub last_refresh: Instant,
    pub edit: Option<Edit>,
    pub color_edit: Option<ColorEdit>,
    pub picker: Option<Picker>,
    pub scroll: usize,
    pub artwork: Option<PathBuf>,
    pub artwork_pane: Option<ratatui::layout::Rect>,
    pub reset: Option<ResetPrompt>,
    pub glyph_seed: u64,
    pub glyph_source: Option<Vec<crate::font::FontGlyph>>,
    pub glyph_font: Option<String>,
    pub glyph_target: Option<usize>,
    pub picked: Option<(String, usize)>,
    placed: Option<(PathBuf, u16, u16, u16, u16)>,
    rng: u64,
}

pub struct ResetPrompt {
    pub scope: usize,
}

impl<'a> App<'a> {
    pub fn new(opts: &'a Opts, base: &Config) -> Self {
        let cfg = base.clone();
        let theme = crate::load_theme(opts, &cfg);
        let palettes = crate::palette_rows(opts);
        let demo = opts.demo;
        let data = if demo {
            crate::demo_data()
        } else {
            crate::collect_data(&cfg)
        };
        let quotes = crate::load_quotes(&crate::quotes_path(opts));
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as u64)
            .unwrap_or(7)
            ^ std::process::id() as u64;
        let mut app = Self {
            opts,
            cfg,
            theme,
            palettes,
            page: 0,
            cursors: [0; 8],
            data,
            quote: String::new(),
            quotes,
            demo,
            status: format!("config {}", crate::config_path(opts).display()),
            dirty: false,
            history_mem: Vec::new(),
            history_swap: Vec::new(),
            last_refresh: Instant::now(),
            edit: None,
            color_edit: None,
            picker: None,
            scroll: 0,
            artwork: None,
            artwork_pane: None,
            reset: None,
            glyph_seed: seed,
            glyph_source: None,
            glyph_font: None,
            glyph_target: None,
            picked: None,
            placed: None,
            rng: seed,
        };
        app.refresh_quote();
        app.refresh();
        app.prune_rotation_set();
        app
    }

    pub fn refresh_quote(&mut self) {
        self.quote = crate::resolve_quote(&self.cfg, &self.quotes, &mut self.rng);
    }

    pub fn refresh(&mut self) {
        self.data = if self.demo {
            crate::demo_data()
        } else {
            crate::collect_data(&self.cfg)
        };
        self.history_mem.push(self.data.memory_pct as u64);
        self.history_swap.push(self.data.swap_pct as u64);
        if self.history_mem.len() > 120 {
            self.history_mem.remove(0);
            self.history_swap.remove(0);
        }
        self.last_refresh = Instant::now();
        self.reload_artwork();
    }

    pub fn reload_artwork(&mut self) {
        self.artwork = crate::select_logo(self.opts, &self.cfg, &mut self.rng);
    }

    pub fn save(&mut self) {
        match crate::write_config(&crate::config_path(self.opts), &self.cfg) {
            Ok(()) => {
                self.dirty = false;
                self.status = format!("saved {}", crate::config_path(self.opts).display());
            }
            Err(error) => self.status = format!("save failed: {}", error),
        }
    }

    pub fn rebuild_theme(&mut self) {
        self.theme = crate::load_theme(self.opts, &self.cfg);
    }

    pub fn effective_mode(&self) -> String {
        self.cfg
            .variant
            .clone()
            .unwrap_or_else(|| self.theme.mode.clone())
    }

    pub fn themes_for_mode(&self, mode: &str) -> Vec<String> {
        crate::theme_availability(&self.palettes)
            .into_iter()
            .filter(|(_, dark, light)| if mode == "light" { *light } else { *dark })
            .map(|(name, _, _)| name)
            .collect()
    }

    pub fn prune_rotation_set(&mut self) {
        let Some(set) = self.cfg.rotate_themes.clone() else {
            return;
        };
        let valid = crate::valid_themes(&self.palettes, &self.effective_mode(), &set);
        if valid.len() != set.len() {
            let removed = set.len() - valid.len();
            self.cfg.rotate_themes = if valid.is_empty() { None } else { Some(valid) };
            self.status = format!("{} theme(s) removed from the rotation set", removed);
        }
    }

    pub fn palette_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for (name, _, _, _, _) in &self.palettes {
            if !names.contains(name) {
                names.push(name.clone());
            }
        }
        names
    }

    pub fn enabled_list(&mut self) -> &mut Vec<String> {
        if self.cfg.enabled.is_none() {
            self.cfg.enabled = Some(
                crate::MODULES
                    .iter()
                    .map(|(id, _)| id.to_string())
                    .collect(),
            );
        }
        self.cfg.enabled.as_mut().unwrap()
    }

    pub fn module_order(&self) -> Vec<(&'static str, &'static str)> {
        match &self.cfg.enabled {
            None => crate::MODULES.to_vec(),
            Some(list) => {
                let mut ordered = Vec::new();
                for id in list {
                    if let Some(entry) = crate::MODULES.iter().find(|(known, _)| known == id) {
                        ordered.push(*entry);
                    }
                }
                for entry in crate::MODULES {
                    if !list.iter().any(|id| id == entry.0) {
                        ordered.push(*entry);
                    }
                }
                ordered
            }
        }
    }

    pub fn module_enabled(&self, id: &str) -> bool {
        self.cfg
            .enabled
            .as_ref()
            .is_none_or(|list| list.iter().any(|item| item == id))
    }

    pub fn color(&self, index: usize) -> Option<Rgb> {
        match index {
            0 => self.cfg.color_bg,
            1 => self.cfg.color_fg,
            2 => self.cfg.color_dim,
            3 => self.cfg.color_accent,
            4 => self.cfg.color_accent2,
            5 => self.cfg.color_ok,
            6 => self.cfg.color_warn,
            _ => self.cfg.color_err,
        }
    }

    pub fn set_color(&mut self, index: usize, value: Option<Rgb>) {
        match index {
            0 => self.cfg.color_bg = value,
            1 => self.cfg.color_fg = value,
            2 => self.cfg.color_dim = value,
            3 => self.cfg.color_accent = value,
            4 => self.cfg.color_accent2 = value,
            5 => self.cfg.color_ok = value,
            6 => self.cfg.color_warn = value,
            _ => self.cfg.color_err = value,
        }
    }

    pub fn effective_color(&self, index: usize) -> Rgb {
        self.color(index).unwrap_or(match index {
            0 => self.theme.pal.bg,
            1 => self.theme.pal.fg,
            2 => self.theme.pal.dim,
            3 => self.theme.pal.accent,
            4 => self.theme.pal.accent2,
            5 => self.theme.pal.ok,
            6 => self.theme.pal.warn,
            _ => self.theme.pal.err,
        })
    }

    pub fn albums(&self) -> Vec<String> {
        crate::list_albums()
    }

    pub fn glyph_row_id(&self, row: usize) -> Option<&'static str> {
        if row < GLYPH_FIXED_ROWS {
            None
        } else {
            crate::MODULES
                .get(row - GLYPH_FIXED_ROWS)
                .map(|(id, _)| *id)
        }
    }

    pub fn glyph_label(&self, row: usize) -> &'static str {
        match row {
            0 => "header glyph",
            1 => "system",
            2 => "core",
            3 => "power",
            4 => "row glyphs",
            5 => "pool",
            _ => self.glyph_row_id(row).unwrap_or("module"),
        }
    }

    pub fn glyph_spec(&self, row: usize) -> String {
        match row {
            0 => self
                .cfg
                .glyph
                .clone()
                .unwrap_or_else(|| "random".to_string()),
            1 => self.cfg.glyph_system.clone().unwrap_or_default(),
            2 => self.cfg.glyph_core.clone().unwrap_or_default(),
            3 => self.cfg.glyph_power.clone().unwrap_or_default(),
            4 => self.cfg.glyph_row_mode().to_string(),
            5 => self.cfg.glyph_pool().to_string(),
            _ => self
                .glyph_row_id(row)
                .and_then(|id| self.cfg.glyphs.get(id).cloned())
                .unwrap_or_else(|| "auto".to_string()),
        }
    }

    pub fn glyph_display(&self, row: usize) -> String {
        match row {
            0 => match self.cfg.glyph.as_deref() {
                Some("random") | Some("mixed") | None => {
                    crate::config::random_glyph(self.cfg.glyph_pool(), self.glyph_seed, "header")
                        .unwrap_or('▸')
                        .to_string()
                }
                Some(value) => value.chars().next().unwrap_or('▸').to_string(),
            },
            1 => crate::config::section_glyph(&self.cfg, "system", self.glyph_seed).to_string(),
            2 => crate::config::section_glyph(&self.cfg, "core", self.glyph_seed).to_string(),
            3 => crate::config::section_glyph(&self.cfg, "power", self.glyph_seed).to_string(),
            4 => match self.cfg.glyph_row_mode() {
                "auto" => "auto",
                other => other,
            }
            .to_string(),
            5 => self.cfg.glyph_pool().to_string(),
            _ => self
                .glyph_row_id(row)
                .and_then(|id| crate::config::module_glyph(&self.cfg, id, self.glyph_seed))
                .map(|glyph| glyph.to_string())
                .unwrap_or_else(|| "(off)".to_string()),
        }
    }

    pub fn set_glyph_spec(&mut self, row: usize, value: &str) -> Result<(), String> {
        let value = value.trim();
        if row == 5 {
            if value.is_empty() {
                return Err("pool cannot be empty".to_string());
            }
            self.cfg.glyph_pool = Some(value.chars().take(512).collect());
            self.dirty = true;
            return Ok(());
        }
        if row == 4 {
            let lower = value.to_lowercase();
            if lower == "auto" {
                self.cfg.glyph_row = None;
            } else if matches!(lower.as_str(), "off" | "none" | "random" | "mixed")
                || value.chars().count() == 1
            {
                self.cfg.glyph_row = Some(value.to_string());
            } else {
                return Err("auto, off, random or one character".to_string());
            }
            self.dirty = true;
            return Ok(());
        }
        let spec = if value.is_empty() {
            None
        } else if matches!(value.to_lowercase().as_str(), "random" | "mixed" | "auto") {
            Some(value.to_lowercase())
        } else if value.chars().count() == 1 {
            Some(value.to_string())
        } else {
            return Err("one glyph character, random or auto".to_string());
        };
        let spec = spec.filter(|value| value != "auto");
        match row {
            0 => self.cfg.glyph = spec,
            1 => self.cfg.glyph_system = spec,
            2 => self.cfg.glyph_core = spec,
            3 => self.cfg.glyph_power = spec,
            _ => {
                let Some(id) = self.glyph_row_id(row) else {
                    return Ok(());
                };
                match spec {
                    Some(value) => {
                        self.cfg.glyphs.insert(id.to_string(), value);
                    }
                    None => {
                        self.cfg.glyphs.remove(id);
                    }
                }
            }
        }
        self.dirty = true;
        Ok(())
    }

    pub fn cycle_glyph(&mut self, row: usize, delta: i64) {
        if row == 5 || row == 4 {
            return;
        }
        let pool: Vec<char> = self.cfg.glyph_pool().chars().collect();
        if pool.is_empty() {
            return;
        }
        let spec = self.glyph_spec(row);
        let current =
            if matches!(spec.as_str(), "random" | "auto" | "") || spec.chars().count() != 1 {
                self.glyph_display(row).chars().next().unwrap_or(pool[0])
            } else {
                spec.chars().next().unwrap_or(pool[0])
            };
        let index = pool.iter().position(|glyph| *glyph == current).unwrap_or(0);
        let index = util::wrap_index(index, pool.len(), delta);
        let _ = self.set_glyph_spec(row, &pool[index].to_string());
    }

    pub fn toggle_glyph_mode(&mut self, row: usize) {
        if row == 5 {
            return;
        }
        if row == 4 {
            let next = if self.cfg.glyph_row_mode() == "off" {
                "auto"
            } else {
                "off"
            };
            let _ = self.set_glyph_spec(row, next);
            return;
        }
        let current = self.glyph_spec(row);
        if matches!(current.as_str(), "random" | "mixed") {
            let pin = self.glyph_display(row);
            let _ = self.set_glyph_spec(row, &pin);
        } else {
            let _ = self.set_glyph_spec(row, "random");
        }
    }

    fn font_items(&self) -> (String, Vec<PickerItem>) {
        let fonts = crate::font::list_system_fonts();
        (
            format!("font · {} families", fonts.len()),
            fonts
                .into_iter()
                .map(|family| PickerItem {
                    label: family.clone(),
                    value: family,
                    checked: false,
                })
                .collect(),
        )
    }

    pub fn open_glyph_source(&mut self, target: Option<usize>) {
        self.glyph_target = target;
        self.glyph_font = None;
        let preferred = [
            "JetBrainsMono Nerd Font",
            "Symbols Nerd Font",
            "Symbols Nerd Font Mono",
        ];
        let mut source: Option<(String, std::path::PathBuf)> =
            preferred.iter().find_map(|family| {
                crate::font::resolve_font(family).map(|path| (family.to_string(), path))
            });
        if source.is_none() {
            source = crate::font::list_system_fonts()
                .into_iter()
                .find(|family| family.to_lowercase().contains("nerd font"))
                .and_then(|family| crate::font::resolve_font(&family).map(|path| (family, path)));
        }
        match source {
            Some((family, path)) => self.load_glyph_source(family, path),
            None => self.open_picker(PickerKind::GlyphFont),
        }
    }

    pub fn load_glyph_source(&mut self, family: String, path: std::path::PathBuf) {
        let glyphs = crate::font::font_glyphs(&path);
        if glyphs.is_empty() {
            self.status = format!("{} has no readable glyphs", family);
            self.open_picker(PickerKind::GlyphFont);
            return;
        }
        self.status = format!("{} · {} glyphs", family, glyphs.len());
        self.glyph_source = Some(glyphs);
        self.glyph_font = Some(family);
        match self.glyph_target {
            Some(row) => self.open_picker(PickerKind::Glyph(row)),
            None => self.open_picker(PickerKind::GlyphPool),
        }
    }

    pub fn open_picker(&mut self, kind: PickerKind) {
        let (title, items): (String, Vec<PickerItem>) = match kind {
            PickerKind::Font => self.font_items(),
            PickerKind::GlyphFont => {
                let (title, items) = self.font_items();
                (format!("glyph source · {}", title), items)
            }
            PickerKind::Glyph(row) => {
                let Some(glyphs) = &self.glyph_source else {
                    self.status = "pick a glyph source font first".to_string();
                    return;
                };
                (
                    format!(
                        "glyph · {} · {} · {}",
                        self.glyph_font.as_deref().unwrap_or("font"),
                        glyphs.len(),
                        self.glyph_label(row)
                    ),
                    glyphs
                        .iter()
                        .map(|glyph| PickerItem {
                            label: crate::font::glyph_label(glyph),
                            value: glyph.ch.to_string(),
                            checked: false,
                        })
                        .collect(),
                )
            }
            PickerKind::GlyphPool => {
                let Some(glyphs) = &self.glyph_source else {
                    self.status = "pick a glyph source font first".to_string();
                    return;
                };
                let pool = self.cfg.glyph_pool().to_string();
                (
                    format!(
                        "pool · {} · {} glyphs",
                        self.glyph_font.as_deref().unwrap_or("font"),
                        glyphs.len()
                    ),
                    glyphs
                        .iter()
                        .map(|glyph| PickerItem {
                            label: crate::font::glyph_label(glyph),
                            value: glyph.ch.to_string(),
                            checked: pool.contains(glyph.ch),
                        })
                        .collect(),
                )
            }
            PickerKind::Theme => (
                format!("theme · {} mode", self.effective_mode()),
                self.themes_for_mode(&self.effective_mode())
                    .into_iter()
                    .map(|name| PickerItem {
                        label: name.clone(),
                        value: name,
                        checked: false,
                    })
                    .collect(),
            ),
            PickerKind::Album => {
                let albums = self.albums();
                (
                    format!("album · {}", albums.len()),
                    albums
                        .into_iter()
                        .map(|name| PickerItem {
                            label: name.clone(),
                            value: name,
                            checked: false,
                        })
                        .collect(),
                )
            }
            PickerKind::Photo => {
                let Some(album) = self.cfg.album.clone() else {
                    self.status = "pick an album first".to_string();
                    return;
                };
                let mut items = vec![PickerItem {
                    label: format!("↻ rotate the whole album ({})", self.cfg.rotate_art_mode()),
                    value: "@rotate".to_string(),
                    checked: false,
                }];
                for path in crate::album_images(&album) {
                    let label = path
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string());
                    items.push(PickerItem {
                        label,
                        value: path.display().to_string(),
                        checked: false,
                    });
                }
                (
                    format!("album {} · {}", album, items.len().saturating_sub(1)),
                    items,
                )
            }
            PickerKind::Quote => {
                let mut items = vec![
                    PickerItem {
                        label: "↻ random".to_string(),
                        value: "@random".to_string(),
                        checked: false,
                    },
                    PickerItem {
                        label: "off".to_string(),
                        value: "@off".to_string(),
                        checked: false,
                    },
                    PickerItem {
                        label: "— custom —".to_string(),
                        value: "@custom".to_string(),
                        checked: false,
                    },
                ];
                items.extend(self.quotes.iter().map(|quote| PickerItem {
                    label: quote.clone(),
                    value: quote.clone(),
                    checked: false,
                }));
                ("quotes".to_string(), items)
            }
            PickerKind::ThemeSet => {
                let chosen = self.cfg.rotate_themes.clone().unwrap_or_default();
                (
                    "rotation set · space marks".to_string(),
                    self.themes_for_mode(&self.effective_mode())
                        .into_iter()
                        .map(|name| {
                            let checked = chosen.iter().any(|item| item == &name);
                            PickerItem {
                                label: name.clone(),
                                value: name,
                                checked,
                            }
                        })
                        .collect(),
                )
            }
        };
        if items.is_empty() {
            self.status = match kind {
                PickerKind::Font => "no fonts found (fontconfig missing?)".to_string(),
                PickerKind::Album => {
                    "no albums yet — press a on Artwork to add a photo".to_string()
                }
                PickerKind::Photo => "album has no images".to_string(),
                _ => "nothing to choose".to_string(),
            };
            return;
        }
        self.picker = Some(Picker {
            kind,
            title,
            items,
            filter: String::new(),
            cursor: 0,
        });
    }

    pub fn reset_current(&mut self) {
        let cursor = self.cursors[self.page];
        match self.page {
            PAGE_MODULES => {
                let id = self.module_order().get(cursor).map(|(id, _)| *id);
                if let (Some(id), Some(list)) = (id, self.cfg.enabled.as_mut()) {
                    list.retain(|item| item != id);
                    if list.is_empty() {
                        self.cfg.enabled = None;
                    }
                    self.dirty = true;
                    self.status = format!("module {} restored", id);
                } else {
                    self.status = "module already at default".to_string();
                }
            }
            PAGE_DECK => {
                match cursor {
                    0 => self.cfg.layout = None,
                    1 => self.cfg.motion = None,
                    2 => self.cfg.rule = None,
                    3 => self.cfg.units = None,
                    4 => self.cfg.temp_unit = None,
                    5 => self.cfg.logo_mode = None,
                    6 => self.cfg.logo_text = None,
                    7 => self.cfg.logo_font = None,
                    8 => self.cfg.logo_size = None,
                    9 => self.cfg.logo_tint = None,
                    10 => self.cfg.logo_width = None,
                    11 => self.cfg.logo_min = None,
                    12 => self.cfg.logo_max = None,
                    13 => self.cfg.logo_gap = None,
                    _ => self.cfg.quote = None,
                }
                self.refresh();
                self.dirty = true;
                self.status = "setting reset".to_string();
            }
            PAGE_GLYPHS => {
                let _ = if cursor < 4 {
                    self.set_glyph_spec(cursor, "")
                } else if cursor == 4 {
                    self.cfg.glyph_row = None;
                    self.dirty = true;
                    Ok(())
                } else if cursor == 5 {
                    self.cfg.glyph_pool = None;
                    self.dirty = true;
                    Ok(())
                } else {
                    let id = self.glyph_row_id(cursor).unwrap_or("");
                    self.cfg.glyphs.remove(id);
                    self.dirty = true;
                    Ok(())
                };
                self.status = "glyph reset".to_string();
            }
            PAGE_PALETTE => match cursor {
                0 => {
                    self.cfg.theme = None;
                    self.rebuild_theme();
                }
                1 => {
                    self.cfg.variant = None;
                    self.rebuild_theme();
                }
                2 => self.cfg.rotate_palette = None,
                3 => self.cfg.rotate_themes = None,
                4..=11 => self.set_color(cursor - 4, None),
                _ => {
                    for index in 0..8 {
                        self.set_color(index, None);
                    }
                }
            },
            PAGE_THRESHOLDS => match cursor {
                0 => self.cfg.memory_warn = None,
                1 => self.cfg.swap_warn = None,
                2 => self.cfg.disk_warn = None,
                3 => self.cfg.battery_low = None,
                4 => self.cfg.temp_warn = None,
                5 => self.cfg.temp_crit = None,
                _ => self.cfg.gpu_warn = None,
            },
            PAGE_ARTWORK => {
                match cursor {
                    0 => self.cfg.album = None,
                    1 => self.cfg.image = None,
                    2 => self.cfg.rotate_art = None,
                    _ => self.cfg.rotate_every = None,
                }
                self.reload_artwork();
            }
            _ => {
                self.status = "nothing to reset here".to_string();
                return;
            }
        }
        self.dirty = true;
        if self.page != PAGE_DECK && self.page != PAGE_GLYPHS {
            self.status = "setting reset".to_string();
        }
    }

    pub fn reset_page(&mut self) {
        match self.page {
            PAGE_MODULES => self.cfg.enabled = None,
            PAGE_DECK => {
                self.cfg.layout = None;
                self.cfg.motion = None;
                self.cfg.rule = None;
                self.cfg.units = None;
                self.cfg.temp_unit = None;
                self.cfg.logo_mode = None;
                self.cfg.logo_text = None;
                self.cfg.logo_font = None;
                self.cfg.logo_size = None;
                self.cfg.logo_tint = None;
                self.cfg.logo_width = None;
                self.cfg.logo_min = None;
                self.cfg.logo_max = None;
                self.cfg.logo_gap = None;
                self.cfg.quote = None;
                self.refresh_quote();
                self.refresh();
            }
            PAGE_GLYPHS => {
                self.cfg.glyph = None;
                self.cfg.glyph_system = None;
                self.cfg.glyph_core = None;
                self.cfg.glyph_power = None;
                self.cfg.glyph_row = None;
                self.cfg.glyph_pool = None;
                self.cfg.glyphs.clear();
            }
            PAGE_PALETTE => {
                self.cfg.theme = None;
                self.cfg.variant = None;
                self.cfg.rotate_palette = None;
                self.cfg.rotate_themes = None;
                for index in 0..8 {
                    self.set_color(index, None);
                }
                self.rebuild_theme();
            }
            PAGE_THRESHOLDS => {
                self.cfg.memory_warn = None;
                self.cfg.swap_warn = None;
                self.cfg.disk_warn = None;
                self.cfg.battery_low = None;
                self.cfg.temp_warn = None;
                self.cfg.temp_crit = None;
                self.cfg.gpu_warn = None;
            }
            PAGE_ARTWORK => {
                self.cfg.album = None;
                self.cfg.image = None;
                self.cfg.rotate_art = None;
                self.cfg.rotate_every = None;
                self.reload_artwork();
            }
            _ => {
                self.status = "nothing to reset here".to_string();
                return;
            }
        }
        self.dirty = true;
        self.status = "page reset to theme defaults".to_string();
    }

    pub fn reset_all(&mut self) {
        self.cfg = Config::default();
        self.rebuild_theme();
        self.refresh();
        self.dirty = true;
        self.status = "config reset to defaults".to_string();
    }

    pub fn logo_cap(&self, cols: usize) -> usize {
        let width_pct = self.cfg.logo_width.unwrap_or(34) as f64 / 100.0;
        let cap = ((cols as f64) * width_pct).round() as usize;
        cap.clamp(
            self.cfg.logo_min.unwrap_or(26) as usize,
            self.cfg.logo_max.unwrap_or(46) as usize,
        )
    }
}

pub fn run(opts: &Opts, base: &Config) -> io::Result<()> {
    let mut app = App::new(opts, base);
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        PushKeyboardEnhancementFlags(
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS
        )
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut outcome: io::Result<()> = Ok(());
    loop {
        if let Err(error) = terminal.draw(|frame| view::draw(frame, &mut app)) {
            outcome = Err(error);
            break;
        }
        if let Err(error) = place_artwork(&mut app, crate::in_kitty(), &mut io::stdout()) {
            outcome = Err(error);
            break;
        }
        if let Err(error) = terminal.set_cursor_position(ratatui::layout::Position::new(0, 0)) {
            outcome = Err(error);
            break;
        }
        match event::poll(Duration::from_millis(200)) {
            Ok(true) => match event::read() {
                Ok(Event::Key(key)) => {
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
                        && input::handle(&mut app, key)
                    {
                        break;
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    outcome = Err(error);
                    break;
                }
            },
            Ok(false) => {}
            Err(error) => {
                outcome = Err(error);
                break;
            }
        }
        if app.last_refresh.elapsed() >= Duration::from_secs(2) {
            app.refresh();
        }
    }

    if app.placed.is_some() {
        let mut out = io::stdout();
        let _ = write!(out, "\x1b_Ga=d,i=4242,q=2;\x1b\\");
        let _ = out.flush();
    }
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        PopKeyboardEnhancementFlags,
        LeaveAlternateScreen
    )?;
    terminal.show_cursor()?;
    outcome
}

fn place_artwork(app: &mut App, kitty: bool, out: &mut impl Write) -> io::Result<()> {
    let target = if kitty && app.cfg.logo_mode() != "text" {
        app.artwork.clone().zip(app.artwork_pane)
    } else {
        None
    };
    match target {
        Some((path, area)) if area.width >= 8 && area.height >= 3 => {
            let key = (path.clone(), area.x, area.y, area.width, area.height);
            if app.placed.as_ref() != Some(&key) {
                let _ = write!(out, "\x1b_Ga=d,i=4242,q=2;\x1b\\");
                let encoded = crate::b64(path.to_string_lossy().as_bytes());
                write!(out, "\x1b_Ga=t,f=100,t=f,i=4242,q=2;{}\x1b\\", encoded)?;
                let (cols, rows, x, y) = fit_artwork(&path, area);
                write!(out, "\x1b[{};{}H", y + 1, x + 1)?;
                write!(
                    out,
                    "\x1b_Ga=p,i=4242,p=1,c={},r={},C=1,q=2;\x1b\\",
                    cols, rows
                )?;
                app.placed = Some(key);
                out.flush()?;
            }
        }
        _ => {
            if app.placed.take().is_some() {
                let _ = write!(out, "\x1b_Ga=d,i=4242,q=2;\x1b\\");
                out.flush()?;
            }
        }
    }
    Ok(())
}

fn fit_artwork(path: &std::path::Path, area: ratatui::layout::Rect) -> (u16, u16, u16, u16) {
    let (img_w, img_h) = crate::png_size(path).unwrap_or((1, 1));
    let (cols, rows, xpixel, ypixel) = crate::term_size();
    let cell_ratio = if xpixel > 0 && ypixel > 0 && cols > 0 && rows > 0 {
        (xpixel as f64 / cols as f64) / (ypixel as f64 / rows as f64)
    } else {
        0.5
    };
    let rows_per_col = (img_h as f64 / img_w as f64) * cell_ratio;
    let mut width = area.width;
    let mut height = (width as f64 * rows_per_col).round() as u16;
    if height > area.height {
        height = area.height;
        width = ((height as f64 / rows_per_col).round() as u16).clamp(1, area.width);
    }
    let width = width.max(1);
    let height = height.max(1);
    let x = area.x + (area.width - width) / 2;
    let y = area.y + (area.height - height) / 2;
    (width, height, x, y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn demo_opts() -> Opts {
        Opts {
            demo: true,
            palettes: Some(PathBuf::from("/nonexistent-palettes.tsv")),
            quotes: Some(PathBuf::from("/nonexistent-quotes.txt")),
            config: Some(PathBuf::from("/nonexistent-fetch.conf")),
            state: Some(PathBuf::from("/nonexistent-state")),
            ..Opts::default()
        }
    }

    fn fake_png(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tachyon-tui-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("dir");
        let path = dir.join(name);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\x89PNG\r\n\x1a\n");
        bytes.extend_from_slice(&[0, 0, 0, 13]);
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&100u32.to_be_bytes());
        bytes.extend_from_slice(&100u32.to_be_bytes());
        fs::write(&path, bytes).expect("png");
        path
    }

    #[test]
    fn artwork_pane_is_scoped_to_the_preview_page() {
        let opts = demo_opts();
        let mut app = App::new(&opts, &Config::default());
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(150, 40)).expect("terminal");
        app.page = PAGE_PREVIEW;
        terminal
            .draw(|frame| view::draw(frame, &mut app))
            .expect("draw");
        let pane = app.artwork_pane.expect("pane on preview");
        assert!(pane.width >= 8 && pane.height >= 3);
        assert!(pane.right() <= 150 && pane.bottom() <= 40);
        app.page = PAGE_MODULES;
        terminal
            .draw(|frame| view::draw(frame, &mut app))
            .expect("draw");
        assert!(app.artwork_pane.is_none());
    }

    #[test]
    fn place_artwork_transmits_once_and_places_once() {
        let opts = demo_opts();
        let mut app = App::new(&opts, &Config::default());
        app.artwork = Some(fake_png("square.png"));
        app.artwork_pane = Some(ratatui::layout::Rect::new(115, 3, 34, 31));

        let mut first = Vec::new();
        place_artwork(&mut app, true, &mut first).expect("place");
        let text = String::from_utf8_lossy(&first);
        assert!(text.contains("\x1b_Ga=t,f=100,t=f,i=4242"));
        assert!(!text.contains("\x1b_Ga=T"));
        assert!(text.contains("\x1b[11;116H"));
        assert!(text.contains("p=1,c=34,r=17,C=1"));

        let mut second = Vec::new();
        place_artwork(&mut app, true, &mut second).expect("place");
        assert!(second.is_empty(), "unchanged pane must not re-place");

        let mut third = Vec::new();
        app.artwork_pane = None;
        place_artwork(&mut app, true, &mut third).expect("place");
        assert!(String::from_utf8_lossy(&third).contains("\x1b_Ga=d,i=4242"));

        let mut fourth = Vec::new();
        place_artwork(&mut app, false, &mut fourth).expect("place");
        assert!(fourth.is_empty());
    }

    #[test]
    fn text_mode_never_places_an_image() {
        let opts = demo_opts();
        let mut app = App::new(&opts, &Config::default());
        app.cfg.logo_mode = Some("text".to_string());
        app.artwork = Some(fake_png("text.png"));
        app.artwork_pane = Some(ratatui::layout::Rect::new(115, 3, 34, 31));
        let mut out = Vec::new();
        place_artwork(&mut app, true, &mut out).expect("place");
        assert!(out.is_empty());
    }

    #[test]
    fn reset_clears_the_current_setting() {
        let base = Config {
            memory_warn: Some(50),
            color_bg: Some((1, 2, 3)),
            theme: Some("Rose Pine".to_string()),
            ..Config::default()
        };
        let opts = demo_opts();
        let mut app = App::new(&opts, &base);
        app.page = PAGE_THRESHOLDS;
        app.reset_current();
        assert_eq!(app.cfg.memory_warn, None);
        assert_eq!(app.cfg.theme.as_deref(), Some("Rose Pine"));
        app.page = PAGE_PALETTE;
        app.cursors[PAGE_PALETTE] = 4;
        app.reset_current();
        assert_eq!(app.cfg.color_bg, None);
    }

    #[test]
    fn reset_page_and_all_restore_defaults() {
        let base = Config {
            memory_warn: Some(50),
            swap_warn: Some(10),
            image: Some(PathBuf::from("/tmp/picture.png")),
            ..Config::default()
        };
        let opts = demo_opts();
        let mut app = App::new(&opts, &base);
        app.page = PAGE_THRESHOLDS;
        app.reset_page();
        assert_eq!(app.cfg.memory_warn, None);
        assert_eq!(app.cfg.swap_warn, None);
        assert!(app.cfg.image.is_some());
        app.reset_all();
        assert_eq!(app.cfg, Config::default());
    }

    #[test]
    fn glyph_editing_pins_and_randomizes() {
        let opts = demo_opts();
        let mut app = App::new(&opts, &Config::default());
        let row = GLYPH_FIXED_ROWS
            + crate::MODULES
                .iter()
                .position(|(id, _)| *id == "memory")
                .unwrap();
        app.toggle_glyph_mode(row);
        assert_eq!(
            app.cfg.glyphs.get("memory").map(String::as_str),
            Some("random")
        );
        app.toggle_glyph_mode(row);
        let pinned = app.cfg.glyphs.get("memory").cloned().unwrap();
        assert_ne!(pinned, "random");
        assert_eq!(pinned.chars().count(), 1);
        app.set_glyph_spec(row, "◆").unwrap();
        assert_eq!(app.cfg.glyphs.get("memory").map(String::as_str), Some("◆"));
        app.set_glyph_spec(row, "two").unwrap_err();
        app.page = PAGE_GLYPHS;
        app.cursors[PAGE_GLYPHS] = row;
        app.reset_current();
        assert!(!app.cfg.glyphs.contains_key("memory"));
    }

    #[test]
    fn module_order_follows_the_enabled_list() {
        let opts = demo_opts();
        let mut app = App::new(&opts, &Config::default());
        app.cfg.enabled = Some(vec!["memory".to_string(), "os".to_string()]);
        let order = app.module_order();
        assert_eq!(order[0].0, "memory");
        assert_eq!(order[1].0, "os");
        assert_eq!(order.len(), crate::MODULES.len());
    }

    #[test]
    fn picker_scrolls_the_selected_item_into_view() {
        let opts = demo_opts();
        let mut app = App::new(&opts, &Config::default());
        let items = (0..120)
            .map(|index| PickerItem {
                label: format!("item {:03}", index),
                value: index.to_string(),
                checked: false,
            })
            .collect();
        app.picker = Some(Picker {
            kind: PickerKind::Font,
            title: "scroll".to_string(),
            items,
            filter: String::new(),
            cursor: 119,
        });
        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(150, 40)).expect("terminal");
        terminal
            .draw(|frame| view::draw(frame, &mut app))
            .expect("draw");
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer.cell((x, y)).map(|cell| cell.symbol()).unwrap_or(" "));
            }
            text.push('\n');
        }
        assert!(
            text.contains("item 119"),
            "selected item must be visible:\n{}",
            text
        );
    }

    #[test]
    fn shift_enter_picks_up_and_drops_modules() {
        use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
        let opts = demo_opts();
        let base = Config {
            enabled: Some(vec![
                "os".to_string(),
                "host".to_string(),
                "kernel".to_string(),
            ]),
            ..Config::default()
        };
        let mut app = App::new(&opts, &base);
        app.page = PAGE_MODULES;
        app.cursors[PAGE_MODULES] = 0;
        input::handle(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
        assert_eq!(app.picked.as_ref().map(|(id, _)| id.as_str()), Some("os"));
        input::handle(
            &mut app,
            KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE),
        );
        let order = app.cfg.enabled.clone().unwrap();
        assert_eq!(order[..2], ["host".to_string(), "os".to_string()]);
        input::handle(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
        assert!(app.picked.is_none());
        assert!(app.dirty);
    }

    #[test]
    fn picker_filters_and_selects() {
        let opts = demo_opts();
        let mut app = App::new(&opts, &Config::default());
        app.picker = Some(Picker {
            kind: PickerKind::Font,
            title: "test".to_string(),
            items: vec![
                PickerItem {
                    label: "DejaVu Sans".to_string(),
                    value: "DejaVu Sans".to_string(),
                    checked: false,
                },
                PickerItem {
                    label: "Hack".to_string(),
                    value: "Hack".to_string(),
                    checked: false,
                },
            ],
            filter: "deja".to_string(),
            cursor: 0,
        });
        assert_eq!(app.picker.as_ref().unwrap().visible(), vec![0]);
        app.picker.as_mut().unwrap().filter.clear();
        app.picker.as_mut().unwrap().cursor = 1;
        assert_eq!(
            app.picker
                .as_ref()
                .unwrap()
                .selected()
                .map(|item| item.value.clone()),
            Some("Hack".to_string())
        );
    }
}
