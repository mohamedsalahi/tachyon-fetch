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

pub const COLOR_NAMES: [&str; 8] = [
    "bg", "fg", "dim", "accent", "accent2", "ok", "warn", "err",
];

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
    placed: Option<(PathBuf, u16, u16)>,
    rng: u64,
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
        let data = if demo { crate::demo_data() } else { crate::collect_data(&cfg) };
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
            placed: None,
            rng: seed,
        };
        app.refresh();
        app
    }

    pub fn refresh(&mut self) {
        self.data = if self.demo { crate::demo_data() } else { crate::collect_data(&self.cfg) };
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
            self.cfg.enabled = Some(crate::MODULES.iter().map(|(id, _)| id.to_string()).collect());
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
        self.color(index).unwrap_or_else(|| match index {
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
        if let Err(error) = place_artwork(&mut app) {
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

fn place_artwork(app: &mut App) -> io::Result<()> {
    let mut out = io::stdout();
    match app.artwork.clone().zip(app.artwork_pane) {
        Some((path, area)) if crate::in_kitty() && area.width >= 8 && area.height >= 3 => {
            let key = (path.clone(), area.width, area.height);
            if app.placed.as_ref() != Some(&key) {
                let _ = write!(out, "\x1b_Ga=d,i=4242,q=2;\x1b\\");
                let encoded = crate::b64(path.to_string_lossy().as_bytes());
                write!(out, "\x1b_Ga=T,f=100,t=f,i=4242,q=2;{}\x1b\\", encoded)?;
                app.placed = Some(key);
                out.flush()?;
            }
            write!(out, "\x1b[{};{}H", area.y + 1, area.x + 1)?;
            write!(
                out,
                "\x1b_Ga=p,i=4242,c={},r={},C=1,q=2;\x1b\\",
                area.width, area.height
            )?;
            out.flush()?;
        }
        _ => {
            if app.placed.is_some() {
                let _ = write!(out, "\x1b_Ga=d,i=4242,q=2;\x1b\\");
                out.flush()?;
                app.placed = None;
            }
        }
    }
    Ok(())
}
