use std::fs;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::Terminal;

use crate::{Config, FetchData, Opts, Palette, Rgb, Theme};

pub mod input;
pub mod util;
pub mod view;

pub const PAGE_TITLES: [&str; 7] = [
    "Modules",
    "Appearance",
    "Palette",
    "Thresholds",
    "Artwork",
    "Preview",
    "Keys",
];

pub const COLOR_NAMES: [&str; 8] = ["bg", "fg", "dim", "accent", "accent2", "ok", "warn", "err"];

pub const RULES: [char; 8] = ['─', '═', '┄', '┈', '·', '.', '━', '~'];

pub struct ColorEdit {
    pub row: usize,
    pub channel: usize,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EditTarget {
    Image,
    Glyph,
    Rule,
    Photo,
}

pub struct Edit {
    pub target: EditTarget,
    pub text: String,
}

pub struct App<'a> {
    pub opts: &'a Opts,
    pub cfg: Config,
    pub theme: Theme,
    pub palettes: Vec<(String, String, Palette, String, String)>,
    pub page: usize,
    pub cursors: [usize; 7],
    pub data: FetchData,
    pub quote: String,
    pub demo: bool,
    pub status: String,
    pub dirty: bool,
    pub history_mem: Vec<u64>,
    pub history_swap: Vec<u64>,
    pub last_refresh: Instant,
    pub edit: Option<Edit>,
    pub color_edit: Option<ColorEdit>,
    pub scroll: usize,
    pub artwork: Option<PathBuf>,
    pub artwork_pane: Option<ratatui::layout::Rect>,
    pub reset: Option<ResetPrompt>,
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
        let palettes = fs::read_to_string(crate::palettes_path(opts))
            .ok()
            .map(|text| text.lines().filter_map(crate::parse_palette_row).collect())
            .unwrap_or_default();
        let demo = opts.demo;
        let data = if demo {
            crate::demo_data()
        } else {
            crate::collect_data(&cfg)
        };
        let quotes = crate::load_quotes(&crate::quotes_path(opts));
        let quote = quotes.first().cloned().unwrap_or_default();
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
            cursors: [0; 7],
            data,
            quote,
            demo,
            status: format!("config {}", crate::config_path(opts).display()),
            dirty: false,
            history_mem: Vec::new(),
            history_swap: Vec::new(),
            last_refresh: Instant::now(),
            edit: None,
            color_edit: None,
            scroll: 0,
            artwork: None,
            artwork_pane: None,
            reset: None,
            placed: None,
            rng: seed,
        };
        app.refresh();
        app
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

    pub fn reset_current(&mut self) {
        let cursor = self.cursors[self.page];
        match self.page {
            0 => {
                if let Some(list) = self.cfg.enabled.as_mut() {
                    let id = crate::MODULES[cursor].0;
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
            1 => {
                match cursor {
                    0 => self.cfg.layout = None,
                    1 => self.cfg.glyph = None,
                    2 => self.cfg.motion = None,
                    3 => self.cfg.rule = None,
                    4 => self.cfg.units = None,
                    _ => self.cfg.temp_unit = None,
                }
                self.refresh();
                self.dirty = true;
                self.status = "setting reset".to_string();
            }
            2 => {
                match cursor {
                    0 => self.cfg.theme = None,
                    1 => self.cfg.variant = None,
                    2..=9 => self.set_color(cursor - 2, None),
                    _ => {
                        for index in 0..8 {
                            self.set_color(index, None);
                        }
                    }
                }
                self.rebuild_theme();
                self.dirty = true;
                self.status = "setting reset".to_string();
            }
            3 => {
                match cursor {
                    0 => self.cfg.memory_warn = None,
                    1 => self.cfg.swap_warn = None,
                    2 => self.cfg.disk_warn = None,
                    3 => self.cfg.battery_low = None,
                    4 => self.cfg.temp_warn = None,
                    5 => self.cfg.temp_crit = None,
                    _ => self.cfg.gpu_warn = None,
                }
                self.dirty = true;
                self.status = "setting reset".to_string();
            }
            4 => {
                match cursor {
                    0 => self.cfg.image = None,
                    1 => self.cfg.album = None,
                    2 => self.cfg.logo_width = None,
                    3 => self.cfg.logo_min = None,
                    4 => self.cfg.logo_max = None,
                    5 => self.cfg.logo_gap = None,
                    _ => {}
                }
                self.reload_artwork();
                self.dirty = true;
                self.status = "setting reset".to_string();
            }
            _ => self.status = "nothing to reset here".to_string(),
        }
    }

    pub fn reset_page(&mut self) {
        match self.page {
            0 => self.cfg.enabled = None,
            1 => {
                self.cfg.layout = None;
                self.cfg.glyph = None;
                self.cfg.motion = None;
                self.cfg.rule = None;
                self.cfg.units = None;
                self.cfg.temp_unit = None;
                self.refresh();
            }
            2 => {
                self.cfg.theme = None;
                self.cfg.variant = None;
                for index in 0..8 {
                    self.set_color(index, None);
                }
                self.rebuild_theme();
            }
            3 => {
                self.cfg.memory_warn = None;
                self.cfg.swap_warn = None;
                self.cfg.disk_warn = None;
                self.cfg.battery_low = None;
                self.cfg.temp_warn = None;
                self.cfg.temp_crit = None;
                self.cfg.gpu_warn = None;
            }
            4 => {
                self.cfg.image = None;
                self.cfg.album = None;
                self.cfg.logo_width = None;
                self.cfg.logo_min = None;
                self.cfg.logo_max = None;
                self.cfg.logo_gap = None;
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
}

pub fn run(opts: &Opts, base: &Config) -> io::Result<()> {
    let mut app = App::new(opts, base);
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
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
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    outcome
}

fn place_artwork(app: &mut App, kitty: bool, out: &mut impl Write) -> io::Result<()> {
    let target = if kitty {
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
        app.page = 5;
        terminal
            .draw(|frame| view::draw(frame, &mut app))
            .expect("draw");
        let pane = app.artwork_pane.expect("pane on preview");
        assert!(pane.width >= 8 && pane.height >= 3);
        assert!(pane.right() <= 150 && pane.bottom() <= 40);
        app.page = 0;
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
    fn reset_clears_the_current_setting() {
        let base = Config {
            memory_warn: Some(50),
            color_bg: Some((1, 2, 3)),
            theme: Some("Rose Pine".to_string()),
            ..Config::default()
        };
        let opts = demo_opts();
        let mut app = App::new(&opts, &base);
        app.page = 3;
        app.cursors[3] = 0;
        app.reset_current();
        assert_eq!(app.cfg.memory_warn, None);
        assert_eq!(app.cfg.theme.as_deref(), Some("Rose Pine"));
        app.page = 2;
        app.cursors[2] = 2;
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
        app.page = 3;
        app.reset_page();
        assert_eq!(app.cfg.memory_warn, None);
        assert_eq!(app.cfg.swap_warn, None);
        assert!(app.cfg.image.is_some());
        app.reset_all();
        assert_eq!(app.cfg, Config::default());
    }
}
