use std::env;
use std::ffi::CString;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

mod ascii;
mod config;
mod font;
mod rotate;
mod tui;

use config::{load_config, parse_color, write_config};
pub use config::{Config, Palette, Rgb};

const VERSION: &str = env!("CARGO_PKG_VERSION");

const FALLBACK_QUOTES: &[&str] = &[
    "“Nature, to be commanded, must be obeyed.” — Francis Bacon",
    "“Philosophy is written in this grand book—the universe—which stands continually open to our gaze. It is written in the language of mathematics, and its characters are triangles, circles, and other geometrical figures.” — Galileo Galilei",
    "“To explain all nature is too difficult a task for any one man or even for any one age.” — Isaac Newton",
    "“It is of great advantage to the student of any subject to read the original memoirs on that subject, for science is always most completely assimilated when it is in the nascent state…” — James Clerk Maxwell",
    "“There is grandeur in this view of life…from so simple a beginning endless forms most beautiful and most wonderful have been, and are being evolved.” — Charles Darwin",
    "“In the distant future I see open fields for far more important researches.” — Charles Darwin",
    "“The errors which arise from the absence of facts are far more numerous and durable than those which result from unsound reasoning respecting true data.” — Charles Babbage",
    "“We may say most aptly that the Analytical Engine weaves algebraical patterns just as the Jacquard-loom weaves flowers and leaves.” — Ada Lovelace",
    "“The Analytical Engine has no pretensions whatever to originate anything. It can do whatever we know how to order it to perform.” — Ada Lovelace",
    "“In almost every computation, a great variety of arrangements for the succession of the processes is possible.” — Ada Lovelace",
    "“The present is theirs; the future, for which I really worked, is mine.” — Nikola Tesla",
    "“A new idea must not be judged by its immediate results.” — Nikola Tesla",
    "“Earth is the cradle of humanity, but one cannot live in a cradle forever.” — Konstantin Tsiolkovsky",
    "“The scientist does not study nature because it is useful; he studies it because he delights in it, and he delights in it because it is beautiful.” — Henri Poincaré",
    "“To doubt everything or to believe everything are two equally convenient solutions; both dispense with the necessity of reflection.” — Henri Poincaré",
    "“Mathematics is the art of giving the same name to different things.” — Henri Poincaré",
    "“Music is the pleasure the human mind experiences from counting without being aware that it is counting.” — Gottfried Wilhelm Leibniz",
    "“The eternal silence of these infinite spaces frightens me.” — Blaise Pascal",
    "“We must know. We will know.” — David Hilbert",
    "“Mathematics, rightly viewed, possesses not only truth, but supreme beauty…” — Bertrand Russell",
    "“The good life is one inspired by love and guided by knowledge.” — Bertrand Russell",
    "“A mathematician, like a painter or poet, is a maker of patterns; the mathematician’s patterns, like the painter’s or poet’s, must be beautiful.” — G. H. Hardy",
    "“Either mathematics is too big for the human mind, or the human mind is more than a machine.” — Kurt Gödel",
    "“The beauty of mathematics only shows itself to more patient followers.” — Maryam Mirzakhani",
    "“There is always a vast field left to experimentation and I hope that we may have some beautiful progress in the following years.” — Marie Curie",
    "“The eternal mystery of the world is its comprehensibility.” — Albert Einstein",
    "“The most beautiful experience we can have is the mysterious.” — Albert Einstein",
    "“The entire range of living matter on Earth…could be regarded as constituting a single living entity.” — James Lovelock",
    "“What we observe is not nature itself, but nature exposed to our method of questioning.” — Werner Heisenberg",
    "“The opposite of a profound truth may well be another profound truth.” — Niels Bohr",
    "“It is more important to have beauty in one’s equations than to have them fit experiment.” — Paul Dirac",
    "“Mathematics is the tool specially suited for dealing with abstract concepts of any kind and there is no limit to its power in this field.” — Paul Dirac",
    "“My own suspicion is that the universe is not only queerer than we suppose, but queerer than we can suppose.” — J. B. S. Haldane",
    "“The first principle is that you must not fool yourself—and you are the easiest person to fool.” — Richard Feynman",
    "“Nature uses only the longest threads to weave her patterns, so each small piece of her fabric reveals the organization of the entire tapestry.” — Richard Feynman",
    "“Equipped with his five senses, man explores the universe around him and calls the adventure Science.” — Edwin Hubble",
    "“We are a way for the cosmos to know itself.” — Carl Sagan",
    "“Look again at that dot. That’s here. That’s home. That’s us.” — Carl Sagan",
    "“Everything that is not forbidden by laws of nature is achievable, given the right knowledge.” — David Deutsch",
    "“The first ultraintelligent machine is the last invention that man need ever make.” — Irving John Good",
    "“When we try to pick out anything by itself, we find it hitched to everything else in the universe.” — John Muir",
    "“Ascribing mental qualities like beliefs, intentions and wants to a machine is sometimes correct if done conservatively…” — John McCarthy",
    "“Will robots inherit the earth? Yes, but they will be our children.” — Marvin Minsky",
    "“A useful intellectual tool puts the human up where he can see what is going on and can point the direction to move next.” — Douglas Engelbart",
    "“You can’t think about thinking without thinking about thinking about something.” — Seymour Papert",
    "“Computer programming is an art…because it produces objects of beauty.” — Donald Knuth",
    "“We had to find a way, and we did.” — Margaret Hamilton",
    "“In nature nothing exists alone.” — Rachel Carson",
    "“Nature has introduced great variety into the landscape, but man has displayed a passion for simplifying it.” — Rachel Carson",
    "“Controlling complexity is the essence of computer programming.” — Brian Kernighan",
    "“The only way of discovering the limits of the possible is to venture a little way past them into the impossible.” — Arthur C. Clarke",
];

#[repr(C)]
struct NvmlUtilization {
    gpu: u32,
    memory: u32,
}

#[repr(C)]
struct NvmlMemory {
    total: u64,
    free: u64,
    used: u64,
}

struct Nvml {
    device: *mut libc::c_void,
    utilization: unsafe extern "C" fn(*mut libc::c_void, *mut NvmlUtilization) -> u32,
    memory: unsafe extern "C" fn(*mut libc::c_void, *mut NvmlMemory) -> u32,
    temperature: unsafe extern "C" fn(*mut libc::c_void, u32, *mut u32) -> u32,
}

unsafe fn nvml_symbol<T: Copy>(library: *mut libc::c_void, name: &[u8]) -> Option<T> {
    let pointer = libc::dlsym(library, name.as_ptr() as *const libc::c_char);
    if pointer.is_null() {
        return None;
    }
    Some(std::mem::transmute_copy::<*mut libc::c_void, T>(&pointer))
}

fn nvml_open() -> Option<Nvml> {
    unsafe {
        let library = libc::dlopen(
            c"libnvidia-ml.so.1".as_ptr(),
            libc::RTLD_LAZY | libc::RTLD_LOCAL,
        );
        if library.is_null() {
            return None;
        }
        let init: unsafe extern "C" fn() -> u32 = nvml_symbol(library, b"nvmlInit_v2\0")?;
        if init() != 0 {
            return None;
        }
        let get_handle: unsafe extern "C" fn(u32, *mut *mut libc::c_void) -> u32 =
            nvml_symbol(library, b"nvmlDeviceGetHandleByIndex_v2\0")?;
        let mut device: *mut libc::c_void = std::ptr::null_mut();
        if get_handle(0, &mut device) != 0 || device.is_null() {
            return None;
        }
        Some(Nvml {
            device,
            utilization: nvml_symbol(library, b"nvmlDeviceGetUtilizationRates\0")?,
            memory: nvml_symbol(library, b"nvmlDeviceGetMemoryInfo\0")?,
            temperature: nvml_symbol(library, b"nvmlDeviceGetTemperature\0")?,
        })
    }
}

fn nvml_stats() -> Option<(u8, String, u8)> {
    let nvml = nvml_open()?;
    unsafe {
        let mut utilization = NvmlUtilization { gpu: 0, memory: 0 };
        let has_utilization = (nvml.utilization)(nvml.device, &mut utilization) == 0;
        let mut memory = NvmlMemory {
            total: 0,
            free: 0,
            used: 0,
        };
        let has_memory = (nvml.memory)(nvml.device, &mut memory) == 0 && memory.total > 0;
        let mut temperature = 0u32;
        let has_temperature = (nvml.temperature)(nvml.device, 0, &mut temperature) == 0;
        if !has_utilization && !has_memory && !has_temperature {
            return None;
        }
        let utilization_value = if has_utilization {
            utilization.gpu.min(100) as u8
        } else {
            0
        };
        let memory_text = if has_memory {
            format!(
                "{} / {} ({:.0}%)",
                fmt_memory_bytes(memory.used),
                fmt_memory_bytes(memory.total),
                memory.used as f64 * 100.0 / memory.total as f64
            )
        } else {
            String::new()
        };
        Some((utilization_value, memory_text, temperature.min(150) as u8))
    }
}

fn fmt_memory_bytes(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    format!("{:.1} GiB", bytes as f64 / GIB)
}

const FALLBACK_PALETTE: Palette = Palette {
    bg: (16, 19, 21),
    fg: (202, 204, 204),
    dim: (75, 78, 85),
    accent: (121, 129, 134),
    accent2: (112, 112, 112),
    ok: (159, 165, 169),
    warn: (217, 219, 220),
    err: (222, 97, 69),
};

#[derive(Clone)]
struct Theme {
    name: String,
    mode: String,
    pal: Palette,
    layout: String,
    motion: String,
}

#[derive(Default)]
pub struct Opts {
    demo: bool,
    plain: bool,
    no_motion: bool,
    no_logo: bool,
    help: bool,
    version: bool,
    list_themes: bool,
    list_fonts: bool,
    list_logos: bool,
    tui: bool,
    rotated_art: Option<usize>,
    layout: Option<String>,
    glyph: Option<String>,
    album: Option<String>,
    theme: Option<String>,
    variant: Option<String>,
    seed: Option<u64>,
    logo: Option<PathBuf>,
    palettes: Option<PathBuf>,
    quotes: Option<PathBuf>,
    logos: Option<PathBuf>,
    config: Option<PathBuf>,
    state: Option<PathBuf>,
    convert_source: Option<PathBuf>,
    convert_destination: Option<PathBuf>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    System,
    Core,
    Power,
}

impl Section {
    fn title(self) -> &'static str {
        match self {
            Section::System => "SYSTEM",
            Section::Core => "CORE",
            Section::Power => "POWER",
        }
    }

    fn key(self) -> &'static str {
        match self {
            Section::System => "system",
            Section::Core => "core",
            Section::Power => "power",
        }
    }
}

struct Row {
    #[allow(dead_code)]
    id: &'static str,
    key: &'static str,
    value: String,
    tone: Option<Rgb>,
}

#[derive(Default)]
struct Sections {
    system: Vec<Row>,
    core: Vec<Row>,
    power: Vec<Row>,
}

struct FetchData {
    user: String,
    hostname: String,
    host: String,
    os: String,
    kernel: String,
    uptime: String,
    shell: String,
    terminal: String,
    compositor: String,
    session: String,
    cores: usize,
    load: Option<String>,
    cpu: String,
    gpu: Option<String>,
    gpu_util: Option<u8>,
    gpu_vram: Option<String>,
    memory: String,
    memory_pct: u8,
    swap: Option<String>,
    swap_pct: u8,
    disk: Option<String>,
    disk_pct: u8,
    media: Option<String>,
    media_pct: u8,
    code: Option<String>,
    code_pct: u8,
    ip: Option<String>,
    link: Option<String>,
    processes: Option<String>,
    battery: Option<String>,
    battery_pct: u8,
    charging: bool,
    battery_health: Option<u8>,
    cpu_temp: Option<u8>,
    gpu_temp: Option<u8>,
}

#[derive(Clone)]
pub struct Line {
    text: String,
    width: usize,
    spans: Vec<(String, Option<Rgb>, bool)>,
}

impl Line {
    fn new() -> Self {
        Self {
            text: String::new(),
            width: 0,
            spans: Vec::new(),
        }
    }

    fn push(&mut self, text: &str, color: Option<Rgb>, bold: bool) {
        self.push_segment(text, color, bold);
    }

    fn push_segment(&mut self, text: &str, color: Option<Rgb>, bold: bool) {
        self.width += width_of(text);
        self.spans.push((text.to_string(), color, bold));
        match color {
            Some((r, g, b)) => {
                if bold {
                    self.text
                        .push_str(&format!("\x1b[1;38;2;{};{};{}m", r, g, b));
                } else {
                    self.text.push_str(&format!("\x1b[38;2;{};{};{}m", r, g, b));
                }
                self.text.push_str(text);
                self.text.push_str("\x1b[0m");
            }
            None => {
                if bold {
                    self.text.push_str("\x1b[1m");
                    self.text.push_str(text);
                    self.text.push_str("\x1b[0m");
                } else {
                    self.text.push_str(text);
                }
            }
        }
    }

    fn pad(&mut self, n: usize) {
        if n > 0 {
            self.push_segment(&" ".repeat(n), None, false);
        }
    }
}

fn logo_side_right(config: &Config) -> bool {
    config.logo_side() == "right"
}

fn width_of(text: &str) -> usize {
    text.chars().count()
}

fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut escape = false;
    for c in text.chars() {
        if escape {
            if c == 'm' {
                escape = false;
            }
            continue;
        }
        if c == '\x1b' {
            escape = true;
        } else {
            out.push(c);
        }
    }
    out
}

fn truncate(text: &str, max: usize) -> String {
    if width_of(text) <= max {
        return text.to_string();
    }
    if max == 0 {
        return String::new();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

fn b64(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(b2 & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

fn home() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    env::var_os(var)
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(fallback))
}

fn palettes_path(opts: &Opts) -> PathBuf {
    if let Some(p) = &opts.palettes {
        return p.clone();
    }
    if let Some(p) = env::var_os("TACHYON_FETCH_PALETTES") {
        return PathBuf::from(p);
    }
    xdg("XDG_DATA_HOME", ".local/share").join("tachyon/palettes.tsv")
}

fn quotes_path(opts: &Opts) -> PathBuf {
    if let Some(p) = &opts.quotes {
        return p.clone();
    }
    if let Some(p) = env::var_os("TACHYON_FETCH_QUOTES") {
        return PathBuf::from(p);
    }
    xdg("XDG_CONFIG_HOME", ".config").join("tachyon/quotes.txt")
}

fn config_path(opts: &Opts) -> PathBuf {
    if let Some(path) = &opts.config {
        return path.clone();
    }
    if let Some(path) = env::var_os("TACHYON_FETCH_CONFIG") {
        return PathBuf::from(path);
    }
    xdg("XDG_CONFIG_HOME", ".config").join("tachyon/fetch.conf")
}

fn albums_root() -> PathBuf {
    env::var_os("TACHYON_FETCH_ALBUMS")
        .map(PathBuf::from)
        .unwrap_or_else(|| xdg("XDG_DATA_HOME", ".local/share").join("tachyon/albums"))
}

fn album_pick(album: &str, rng: &mut u64) -> Option<PathBuf> {
    let files = album_images(album);
    if files.is_empty() {
        None
    } else {
        Some(files[(next_rand(rng) % files.len() as u64) as usize].clone())
    }
}

fn state_path(opts: &Opts) -> PathBuf {
    if let Some(p) = &opts.state {
        return p.clone();
    }
    if let Some(p) = env::var_os("TACHYON_FETCH_STATE") {
        return PathBuf::from(p);
    }
    xdg("XDG_STATE_HOME", ".local/state").join("tachyon/theme")
}

fn rotation_path(opts: &Opts) -> PathBuf {
    if let Some(path) = env::var_os("TACHYON_FETCH_ROTATION") {
        return PathBuf::from(path);
    }
    state_path(opts).with_file_name("rotation")
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn seed_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(7)
        ^ process::id() as u64
}

fn read_state(path: &Path) -> (Option<String>, Option<String>) {
    match fs::read_to_string(path) {
        Ok(text) => {
            let mut lines = text.lines();
            let name = lines
                .next()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let mode = lines
                .next()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            (name, mode)
        }
        Err(_) => (None, None),
    }
}

fn parse_palette_row(line: &str) -> Option<(String, String, Palette, String, String)> {
    let fields: Vec<&str> = line.split('\t').collect();
    if fields.len() < 12 {
        return None;
    }
    let pal = Palette {
        bg: parse_color(fields[2])?,
        fg: parse_color(fields[3])?,
        dim: parse_color(fields[4])?,
        accent: parse_color(fields[5])?,
        accent2: parse_color(fields[6])?,
        ok: parse_color(fields[7])?,
        warn: parse_color(fields[8])?,
        err: parse_color(fields[9])?,
    };
    Some((
        fields[0].to_string(),
        fields[1].to_string(),
        pal,
        fields[10].to_string(),
        fields[11].to_string(),
    ))
}

fn default_mode(name: &str) -> &'static str {
    if name == "Flexoki Light" || name == "Bliss" {
        "light"
    } else {
        "dark"
    }
}

fn load_theme(opts: &Opts, config: &Config) -> Theme {
    let (state_name, state_mode) = read_state(&state_path(opts));
    let mut state_name = state_name;
    let mut state_mode = state_mode;
    if state_name.is_none() {
        let omarchy =
            xdg("XDG_STATE_HOME", ".local/state").join("omarchy/current/theme/theme.name");
        if let Ok(text) = fs::read_to_string(omarchy) {
            let name = text.trim();
            if !name.is_empty() {
                state_mode = Some(default_mode(name).to_string());
                state_name = Some(name.to_string());
            }
        }
    }
    let wanted = opts
        .theme
        .clone()
        .or_else(|| config.theme.clone())
        .or(state_name)
        .unwrap_or_else(|| "Solitude".to_string());
    let mode = opts
        .variant
        .clone()
        .or_else(|| config.variant.clone())
        .or(state_mode)
        .unwrap_or_else(|| "dark".to_string());

    let rows = palette_rows(opts);

    let pick = |variant: &str| {
        rows.iter()
            .find(|(name, v, _, _, _)| name == &wanted && v == variant)
            .cloned()
    };
    let chosen = pick(&mode)
        .or_else(|| pick("dark"))
        .or_else(|| pick("light"))
        .or_else(|| {
            rows.iter()
                .find(|(name, _, _, _, _)| name == &wanted)
                .cloned()
        });

    let mut theme = match chosen {
        Some((name, variant, pal, layout, motion)) => Theme {
            name,
            mode: variant,
            pal,
            layout,
            motion,
        },
        None => Theme {
            name: wanted,
            mode,
            pal: FALLBACK_PALETTE,
            layout: "hud".to_string(),
            motion: "reveal".to_string(),
        },
    };
    config.apply_colors(&mut theme.pal);
    theme
}

pub fn palette_rows(opts: &Opts) -> Vec<(String, String, Palette, String, String)> {
    let mut rows: Vec<(String, String, Palette, String, String)> = config::BUILTIN_THEMES
        .iter()
        .map(|theme| {
            (
                theme.name.to_string(),
                theme.variant.to_string(),
                Palette {
                    bg: theme.colors[0],
                    fg: theme.colors[1],
                    dim: theme.colors[2],
                    accent: theme.colors[3],
                    accent2: theme.colors[4],
                    ok: theme.colors[5],
                    warn: theme.colors[6],
                    err: theme.colors[7],
                },
                theme.layout.to_string(),
                theme.motion.to_string(),
            )
        })
        .collect();
    if let Ok(text) = fs::read_to_string(palettes_path(opts)) {
        for row in text.lines().filter_map(parse_palette_row) {
            if let Some(slot) = rows
                .iter_mut()
                .find(|(name, variant, _, _, _)| name == &row.0 && variant == &row.1)
            {
                *slot = row;
            } else {
                rows.push(row);
            }
        }
    }
    rows
}

pub fn theme_availability(
    rows: &[(String, String, Palette, String, String)],
) -> Vec<(String, bool, bool)> {
    let mut names: Vec<String> = Vec::new();
    for (name, _, _, _, _) in rows {
        if !names.contains(name) {
            names.push(name.clone());
        }
    }
    names
        .into_iter()
        .map(|name| {
            let dark = rows
                .iter()
                .find(|(row, variant, _, _, _)| row == &name && variant == "dark")
                .map(|(_, _, palette, _, _)| *palette);
            let light = rows
                .iter()
                .find(|(row, variant, _, _, _)| row == &name && variant == "light")
                .map(|(_, _, palette, _, _)| *palette);
            let has_dark = dark.is_some();
            let has_light = match (dark, light) {
                (Some(dark), Some(light)) => dark != light,
                (None, Some(_)) => true,
                _ => false,
            };
            (name, has_dark, has_light)
        })
        .collect()
}

pub fn valid_themes(
    rows: &[(String, String, Palette, String, String)],
    mode: &str,
    wanted: &[String],
) -> Vec<String> {
    let availability = theme_availability(rows);
    wanted
        .iter()
        .filter(|name| {
            availability.iter().any(|(known, dark, light)| {
                known == *name && if mode == "light" { *light } else { *dark }
            })
        })
        .cloned()
        .collect()
}

pub fn theme_names(opts: &Opts) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for (name, _, _, _, _) in palette_rows(opts) {
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

fn load_quotes(path: &Path) -> Vec<String> {
    let mut quotes: Vec<String> = fs::read_to_string(path)
        .ok()
        .map(|text| {
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    if quotes.is_empty() {
        quotes = FALLBACK_QUOTES.iter().map(|s| s.to_string()).collect();
    }
    quotes
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Note {
    pub text: String,
    pub author: Option<String>,
}

pub const EMOTICONS: [&str; 8] = [
    "(⌐■_■)",
    "( •_•)>⌐■-■",
    "ᕕ( ᐛ )ᕗ",
    "¯\\_(ツ)_/¯",
    "(￣▽￣)",
    "ヽ(•‿•)ノ",
    "ಠ_ಠ",
    "(￢_￢)",
];

fn split_note(line: &str) -> Note {
    if let Some((text, author)) = line.rsplit_once(" — ") {
        if !author.trim().is_empty() {
            return Note {
                text: text.to_string(),
                author: Some(author.trim().to_string()),
            };
        }
    }
    Note {
        text: line.to_string(),
        author: None,
    }
}

pub fn resolve_note(config: &Config, quotes: &[String], rng: &mut u64) -> Note {
    match config.note_mode() {
        "off" => Note::default(),
        "text" => Note {
            text: config
                .note_text
                .clone()
                .or_else(|| {
                    config
                        .quote
                        .clone()
                        .filter(|quote| quote != "off" && quote != "random")
                })
                .unwrap_or_default(),
            author: None,
        },
        "emoticon" => Note {
            text: config
                .emoticon
                .clone()
                .unwrap_or_else(|| EMOTICONS[0].to_string()),
            author: None,
        },
        _ => {
            let line = match config.quote.as_deref() {
                Some("off") | Some("none") => return Note::default(),
                Some("random") | None => {
                    if quotes.is_empty() {
                        return Note::default();
                    }
                    quotes[(next_rand(rng) % quotes.len() as u64) as usize].clone()
                }
                Some(value) => value.to_string(),
            };
            split_note(&line)
        }
    }
}

fn next_rand(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn parse_os_release(text: &str) -> String {
    let mut name = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
            let value = value.trim().trim_matches('"');
            if !value.is_empty() {
                name = Some(value.to_string());
            }
        } else if name.is_none() {
            if let Some(value) = line.strip_prefix("NAME=") {
                let value = value.trim().trim_matches('"');
                if !value.is_empty() {
                    name = Some(value.to_string());
                }
            }
        }
    }
    name.unwrap_or_else(|| "Linux".to_string())
}

fn collapse_spaces(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !space {
                out.push(' ');
                space = true;
            }
        } else {
            out.push(c);
            space = false;
        }
    }
    out.trim().to_string()
}

fn parse_cpuinfo(text: &str) -> (String, usize) {
    let mut model = String::new();
    let mut cores = 0usize;
    for line in text.lines() {
        if line.starts_with("processor") {
            cores += 1;
        }
        if model.is_empty() {
            if let Some((key, value)) = line.split_once(':') {
                if key.trim() == "model name" {
                    model = collapse_spaces(value)
                        .replace("(R)", "")
                        .replace("(TM)", "")
                        .replace("CPU ", "");
                    model = collapse_spaces(&model);
                }
            }
        }
    }
    (model, cores)
}

fn parse_meminfo(text: &str) -> Option<(u64, u64)> {
    let mut total = None;
    let mut available = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("MemTotal:") {
            total = value
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<u64>().ok());
        } else if let Some(value) = line.strip_prefix("MemAvailable:") {
            available = value
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<u64>().ok());
        }
    }
    match (total, available) {
        (Some(total), Some(available)) if total > 0 => Some((total, available)),
        _ => None,
    }
}

fn meminfo_value(text: &str, key: &str) -> Option<u64> {
    text.lines().find_map(|line| {
        line.strip_prefix(key)
            .and_then(|value| value.split_whitespace().next())
            .and_then(|value| value.parse::<u64>().ok())
    })
}

fn hwmon_temp(kinds: &[&str]) -> Option<u8> {
    for entry in fs::read_dir("/sys/class/hwmon").ok()?.flatten() {
        let name = match fs::read_to_string(entry.path().join("name")) {
            Ok(name) => name.trim().to_string(),
            Err(_) => continue,
        };
        if !kinds.iter().any(|kind| name == *kind) {
            continue;
        }
        if let Ok(raw) = fs::read_to_string(entry.path().join("temp1_input")) {
            if let Ok(value) = raw.trim().parse::<f64>() {
                return Some((value / 1000.0).round().clamp(0.0, 150.0) as u8);
            }
        }
    }
    None
}

fn link_info() -> Option<String> {
    for entry in fs::read_dir("/sys/class/net").ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "lo" {
            continue;
        }
        let state = read_trim(entry.path().join("operstate").to_str()?).unwrap_or_default();
        if state != "up" {
            continue;
        }
        let speed = read_trim(entry.path().join("speed").to_str()?)
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|value| *value > 0);
        return Some(match speed {
            Some(speed) => format!("{} · {} Mb/s", name, speed),
            None => format!("{} · up", name),
        });
    }
    None
}

fn process_count() -> Option<String> {
    let count = fs::read_dir("/proc")
        .ok()?
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .chars()
                .all(|c| c.is_ascii_digit())
        })
        .count();
    if count > 2 {
        Some(count.to_string())
    } else {
        None
    }
}

fn parse_fib_trie(text: &str) -> Option<String> {
    let mut candidate: Option<String> = None;
    for line in text.lines() {
        if let Some(pos) = line.find("|-- ") {
            let token = line[pos + 4..].trim();
            if is_ipv4(token) && !token.starts_with("127.") && !token.starts_with("0.") {
                candidate = Some(token.to_string());
            }
        } else if line.contains("/32 host LOCAL") {
            if let Some(found) = candidate.take() {
                return Some(found);
            }
        }
    }
    None
}

fn is_ipv4(text: &str) -> bool {
    let parts: Vec<&str> = text.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    parts.iter().all(|part| {
        !part.is_empty()
            && part.chars().all(|c| c.is_ascii_digit())
            && part.parse::<u16>().map(|v| v <= 255).unwrap_or(false)
    })
}

fn fmt_uptime(seconds: u64) -> String {
    let days = seconds / 86_400;
    let hours = (seconds % 86_400) / 3_600;
    let minutes = (seconds % 3_600) / 60;
    if days > 0 {
        format!("{}d {:02}h {:02}m", days, hours, minutes)
    } else if hours > 0 {
        format!("{}h {:02}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

fn fmt_size(bytes: u64, si: bool) -> String {
    let (base, small, large) = if si {
        (1000.0_f64, "GB", "TB")
    } else {
        (1024.0_f64, "GiB", "TiB")
    };
    let giga = base * base * base;
    let tera = giga * base;
    let value = bytes as f64;
    if value >= tera {
        format!("{:.1} {}", value / tera, large)
    } else {
        format!("{:.0} {}", value / giga, small)
    }
}

fn fmt_temp(celsius: u8, fahrenheit: bool) -> String {
    if fahrenheit {
        format!("{} °F", (celsius as f32 * 1.8 + 32.0).round() as u8)
    } else {
        format!("{} °C", celsius)
    }
}

fn read_text(path: &str) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn read_trim(path: &str) -> Option<String> {
    read_text(path)
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn term_size() -> (usize, usize, usize, usize) {
    let mut size: libc::winsize = unsafe { std::mem::zeroed() };
    let result = unsafe { libc::ioctl(libc::STDOUT_FILENO, libc::TIOCGWINSZ, &mut size) };
    if result == 0 && size.ws_col > 0 {
        (
            size.ws_col as usize,
            size.ws_row as usize,
            size.ws_xpixel as usize,
            size.ws_ypixel as usize,
        )
    } else {
        (100, 30, 0, 0)
    }
}

fn disk_usage(path: &str) -> Option<(u64, u64)> {
    let mut stats: libc::statvfs = unsafe { std::mem::zeroed() };
    let c_path = CString::new(path).ok()?;
    let result = unsafe { libc::statvfs(c_path.as_ptr(), &mut stats) };
    if result != 0 || stats.f_frsize == 0 || stats.f_blocks == 0 {
        return None;
    }
    let total = stats.f_blocks.saturating_mul(stats.f_frsize);
    let available = stats.f_bavail.saturating_mul(stats.f_frsize);
    let used = total.saturating_sub(available);
    Some((used, total))
}

fn gpu_name() -> Option<String> {
    let entries = fs::read_dir("/proc/driver/nvidia/gpus").ok()?;
    for entry in entries.flatten() {
        let info = entry.path().join("information");
        if let Ok(text) = fs::read_to_string(info) {
            for line in text.lines() {
                if let Some(value) = line.strip_prefix("Model:") {
                    let value = value.trim();
                    if !value.is_empty() {
                        return Some(value.to_string());
                    }
                }
            }
        }
    }
    None
}

fn battery_info() -> Option<(u8, String, bool, Option<u8>)> {
    let entries = fs::read_dir("/sys/class/power_supply").ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("BAT") {
            continue;
        }
        let capacity = fs::read_to_string(entry.path().join("capacity"))
            .ok()
            .and_then(|value| value.trim().parse::<u8>().ok())?;
        let status = fs::read_to_string(entry.path().join("status"))
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "Unknown".to_string());
        let charging = status == "Charging" || status == "Full";
        let health = ["energy_full", "charge_full"].iter().find_map(|full| {
            let design = if *full == "energy_full" {
                "energy_full_design"
            } else {
                "charge_full_design"
            };
            let full_value = fs::read_to_string(entry.path().join(full))
                .ok()
                .and_then(|value| value.trim().parse::<u64>().ok())?;
            let design_value = fs::read_to_string(entry.path().join(design))
                .ok()
                .and_then(|value| value.trim().parse::<u64>().ok())?;
            (full_value * 100)
                .checked_div(design_value)
                .map(|percent| percent as u8)
        });
        return Some((capacity, status, charging, health));
    }
    None
}

fn collect_data(config: &Config) -> FetchData {
    let si = config.units_si();
    let hostname =
        read_trim("/proc/sys/kernel/hostname").unwrap_or_else(|| "localhost".to_string());
    let user = env::var("USER")
        .or_else(|_| env::var("LOGNAME"))
        .unwrap_or_else(|_| "user".to_string());

    let os = read_text("/etc/os-release")
        .map(|text| parse_os_release(&text))
        .unwrap_or_else(|| "Linux".to_string());
    let kernel = read_trim("/proc/sys/kernel/osrelease").unwrap_or_else(|| "unknown".to_string());
    let uptime = read_text("/proc/uptime")
        .and_then(|text| {
            text.split_whitespace()
                .next()
                .and_then(|v| v.parse::<f64>().ok())
        })
        .map(|seconds| fmt_uptime(seconds as u64))
        .unwrap_or_else(|| "unknown".to_string());

    let host = read_trim("/sys/class/dmi/id/product_name").unwrap_or_else(|| hostname.clone());

    let shell = env::var("SHELL")
        .ok()
        .and_then(|value| {
            Path::new(&value)
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "sh".to_string());

    let terminal = if env::var_os("KITTY_WINDOW_ID").is_some() {
        "kitty".to_string()
    } else {
        env::var("TERM_PROGRAM")
            .ok()
            .filter(|value| !value.is_empty())
            .or_else(|| env::var("TERM").ok().filter(|value| !value.is_empty()))
            .unwrap_or_else(|| "tty".to_string())
    };

    let compositor = env::var("XDG_CURRENT_DESKTOP")
        .ok()
        .filter(|value| !value.is_empty())
        .or_else(|| {
            if env::var_os("WAYLAND_DISPLAY").is_some() {
                Some("Wayland".to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "tty".to_string());

    let session = env::var("XDG_SESSION_TYPE")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "tty".to_string());

    let load = read_text("/proc/loadavg").and_then(|text| {
        let values: Vec<&str> = text.split_whitespace().take(3).collect();
        if values.len() == 3 {
            Some(values.join(" "))
        } else {
            None
        }
    });

    let (cpu, cores) = read_text("/proc/cpuinfo")
        .map(|text| parse_cpuinfo(&text))
        .unwrap_or_else(|| ("unknown".to_string(), 0));
    let cpu = if cores > 0 {
        format!("{} ({})", cpu, cores)
    } else {
        cpu
    };

    let gpu = gpu_name();

    let meminfo = read_text("/proc/meminfo");
    let (memory, memory_pct) = match meminfo.as_deref().and_then(parse_meminfo) {
        Some((total, available)) => {
            let used = total.saturating_sub(available);
            let divisor = if si {
                1_000_000_000.0
            } else {
                1024.0 * 1024.0 * 1024.0
            };
            let unit = if si { "GB" } else { "GiB" };
            let total_value = total as f64 * 1024.0 / divisor;
            let used_value = used as f64 * 1024.0 / divisor;
            let percent = (used * 100).checked_div(total).unwrap_or(0) as u8;
            (
                format!(
                    "{:.1} / {:.1} {} ({}%)",
                    used_value, total_value, unit, percent
                ),
                percent,
            )
        }
        None => ("unknown".to_string(), 0),
    };

    let (swap, swap_pct) = match meminfo.as_deref() {
        Some(text) => match (
            meminfo_value(text, "SwapTotal:"),
            meminfo_value(text, "SwapFree:"),
        ) {
            (Some(total), Some(free)) if total > 0 => {
                let used = total.saturating_sub(free);
                let percent = (used * 100 / total) as u8;
                (
                    Some(format!(
                        "{:.1} / {:.1} {} ({}%)",
                        used as f64 * 1024.0
                            / if si {
                                1_000_000_000.0
                            } else {
                                1024.0 * 1024.0 * 1024.0
                            },
                        total as f64 * 1024.0
                            / if si {
                                1_000_000_000.0
                            } else {
                                1024.0 * 1024.0 * 1024.0
                            },
                        if si { "GB" } else { "GiB" },
                        percent
                    )),
                    percent,
                )
            }
            _ => (None, 0),
        },
        None => (None, 0),
    };

    let mount = |path: &str| -> Option<(String, u8)> {
        if !Path::new(path).is_dir() {
            return None;
        }
        let (used, total) = disk_usage(path)?;
        if total == 0 {
            return None;
        }
        let percent = (used * 100 / total) as u8;
        Some((
            format!(
                "{} / {} ({}%)",
                fmt_size(used, si),
                fmt_size(total, si),
                percent
            ),
            percent,
        ))
    };

    let (disk, disk_pct) = match mount("/") {
        Some((text, percent)) => (Some(text), percent),
        None => (None, 0),
    };
    let (media, media_pct) = match mount("/media") {
        Some((text, percent)) => (Some(text), percent),
        None => (None, 0),
    };
    let (code, code_pct) = match mount("/code") {
        Some((text, percent)) => (Some(text), percent),
        None => (None, 0),
    };

    let ip = read_text("/proc/net/fib_trie").and_then(|text| parse_fib_trie(&text));
    let link = link_info();
    let processes = process_count();

    let (battery, battery_pct, charging, battery_health) = match battery_info() {
        Some((capacity, status, charging, health)) => (
            Some(format!("{}% [{}]", capacity, status)),
            capacity,
            charging,
            health,
        ),
        None => (None, 0, false, None),
    };

    let cpu_temp = hwmon_temp(&["k10temp", "zenpower", "coretemp"]);
    let nvml = nvml_stats();
    let gpu_util = nvml.as_ref().map(|(utilization, _, _)| *utilization);
    let gpu_vram = nvml
        .as_ref()
        .map(|(_, memory, _)| memory.clone())
        .filter(|memory| !memory.is_empty());
    let gpu_temp = nvml
        .as_ref()
        .map(|(_, _, temperature)| *temperature)
        .filter(|temperature| *temperature > 0)
        .or_else(|| hwmon_temp(&["nvidia"]));

    FetchData {
        user,
        hostname,
        host,
        os,
        kernel,
        uptime,
        shell,
        terminal,
        compositor,
        session,
        cores,
        load,
        cpu,
        gpu,
        gpu_util,
        gpu_vram,
        memory,
        memory_pct,
        swap,
        swap_pct,
        disk,
        disk_pct,
        media,
        media_pct,
        code,
        code_pct,
        ip,
        link,
        processes,
        battery,
        battery_pct,
        charging,
        battery_health,
        cpu_temp,
        gpu_temp,
    }
}

fn demo_data() -> FetchData {
    FetchData {
        user: "pilot".to_string(),
        hostname: "tuf-a15".to_string(),
        host: "ASUS TUF Gaming A15".to_string(),
        os: "NixOS 26.11 (x86_64)".to_string(),
        kernel: "6.16.0".to_string(),
        uptime: "2h 41m".to_string(),
        shell: "fish 4.0.1".to_string(),
        terminal: "kitty".to_string(),
        compositor: "Umbriel (Wayland)".to_string(),
        session: "wayland".to_string(),
        cores: 16,
        load: Some("0.42 0.51 0.47".to_string()),
        cpu: "AMD Ryzen 7 (16)".to_string(),
        gpu: Some("NVIDIA GeForce RTX 4060 Laptop GPU".to_string()),
        gpu_util: Some(37),
        gpu_vram: Some("3.2 / 8.0 GiB (40%)".to_string()),
        memory: "4.6 / 15.0 GiB (31%)".to_string(),
        memory_pct: 31,
        swap: Some("0.0 / 8.0 GiB (0%)".to_string()),
        swap_pct: 0,
        disk: Some("214 GiB / 1.0 TiB (21%)".to_string()),
        disk_pct: 21,
        media: Some("612 GiB / 1.4 TiB (43%)".to_string()),
        media_pct: 43,
        code: Some("18 GiB / 256 GiB (7%)".to_string()),
        code_pct: 7,
        ip: Some("10.0.0.42".to_string()),
        link: Some("wlan0 · 866 Mb/s".to_string()),
        processes: Some("412".to_string()),
        battery: Some("87% [Charging]".to_string()),
        battery_pct: 87,
        charging: true,
        battery_health: Some(96),
        cpu_temp: Some(58),
        gpu_temp: Some(52),
    }
}

fn png_size(path: &Path) -> Option<(u32, u32)> {
    let mut file = fs::File::open(path).ok()?;
    let mut header = [0u8; 24];
    file.read_exact(&mut header).ok()?;
    if &header[0..8] != b"\x89PNG\r\n\x1a\n" {
        return None;
    }
    let width = u32::from_be_bytes([header[16], header[17], header[18], header[19]]);
    let height = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);
    if width == 0 || height == 0 {
        return None;
    }
    Some((width, height))
}

fn in_kitty() -> bool {
    if env::var_os("TMUX").is_some() {
        return false;
    }
    env::var_os("KITTY_WINDOW_ID").is_some()
        || env::var("TERM")
            .map(|value| value.contains("kitty"))
            .unwrap_or(false)
        || env::var("TERM_PROGRAM")
            .map(|value| value == "kitty")
            .unwrap_or(false)
}

fn select_logo(opts: &Opts, config: &Config, rng: &mut u64) -> Option<PathBuf> {
    let explicit = opts
        .logo
        .clone()
        .or_else(|| config.image.clone())
        .or_else(|| env::var_os("TACHYON_FETCH_LOGO").map(PathBuf::from));
    if let Some(path) = explicit {
        if png_size(&path).is_some() {
            return Some(path);
        }
    }
    let album = opts.album.clone().or_else(|| config.album.clone());
    if let Some(album) = album {
        if let Some(index) = opts.rotated_art {
            if let Some(path) = album_at(&album, index) {
                return Some(path);
            }
        }
        if let Some(path) = album_pick(&album, rng) {
            return Some(path);
        }
    }
    let path = xdg("XDG_DATA_HOME", ".local/share").join("tachyon/logo.png");
    png_size(&path).map(|_| path)
}

pub fn album_images(album: &str) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(albums_root().join(album))
        .ok()
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| {
                    path.extension()
                        .and_then(|value| value.to_str())
                        .map(|value| value.eq_ignore_ascii_case("png"))
                        .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    files
}

fn album_at(album: &str, index: usize) -> Option<PathBuf> {
    let files = album_images(album);
    files.get(index % files.len().max(1)).cloned()
}

#[allow(clippy::too_many_arguments)]
fn section(
    glyph: char,
    title: &str,
    rows: &[Row],
    width: usize,
    pal: &Palette,
    rule: char,
    config: &Config,
    seed: u64,
) -> Vec<Line> {
    if rows.is_empty() {
        return Vec::new();
    }
    let keyw = rows.iter().map(|row| width_of(row.key)).max().unwrap_or(4);
    let prefix = format!("{} {} ", glyph, title);
    let prefix_width = width_of(&prefix);
    let mut head = Line::new();
    head.push(&prefix, Some(pal.accent2), true);
    if width > prefix_width {
        head.push(
            &rule.to_string().repeat(width - prefix_width),
            Some(pal.dim),
            false,
        );
    }
    let mut lines = vec![head];
    for row in rows {
        let mut line = Line::new();
        if let Some(row_glyph) = config::module_glyph(config, row.id, seed) {
            line.push(&format!("{} ", row_glyph), Some(pal.accent), false);
        }
        let key_padded = format!("{:<keyw$}", row.key, keyw = keyw);
        line.push(&key_padded, Some(pal.dim), false);
        line.push("  ", None, false);
        let available = width.saturating_sub(keyw + 2);
        line.push(
            &truncate(&row.value, available),
            row.tone.or(Some(pal.fg)),
            false,
        );
        lines.push(line);
    }
    lines
}

pub struct TextLogo {
    pub lines: Vec<Line>,
    pub width: usize,
}

fn lerp(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    (
        (a.0 as f32 + (b.0 as f32 - a.0 as f32) * t).round() as u8,
        (a.1 as f32 + (b.1 as f32 - a.1 as f32) * t).round() as u8,
        (a.2 as f32 + (b.2 as f32 - a.2 as f32) * t).round() as u8,
    )
}

pub fn text_logo(
    config: &Config,
    hostname: &str,
    max_cols: usize,
    pal: &Palette,
) -> Option<TextLogo> {
    let text = config
        .logo_text
        .clone()
        .filter(|text| !text.trim().is_empty())
        .unwrap_or_else(|| hostname.to_uppercase());
    let family = config
        .logo_font
        .clone()
        .unwrap_or_else(|| "monospace".to_string());
    let path = font::resolve_font(&family)?;
    let art = font::render_text(&text, &path, config.logo_size_px() as f32, max_cols)?;
    if art.rows.is_empty() || art.width == 0 {
        return None;
    }
    let height = art.rows.len();
    let tint = config.logo_tint();
    let lines = art
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let mut line = Line::new();
            if !row.trim().is_empty() {
                let color = match tint {
                    "accent" => pal.accent,
                    "accent2" => pal.accent2,
                    "fg" => pal.fg,
                    _ => lerp(
                        pal.accent,
                        pal.accent2,
                        index as f32 / (height.max(2) - 1) as f32,
                    ),
                };
                line.push(row, Some(color), false);
            }
            line
        })
        .collect();
    Some(TextLogo {
        lines,
        width: art.width,
    })
}

pub fn compose_logo(logo: &TextLogo, info: Vec<Line>, gap: usize) -> Vec<Line> {
    hcat(&logo.lines, &info, logo.width, gap)
}

fn hcat(left: &[Line], right: &[Line], left_width: usize, gap: usize) -> Vec<Line> {
    let count = left.len().max(right.len());
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let mut line = left.get(i).cloned().unwrap_or_else(Line::new);
        if left_width > line.width {
            line.pad(left_width - line.width);
        }
        line.pad(gap);
        if let Some(right_line) = right.get(i) {
            for (text, color, bold) in &right_line.spans {
                line.push_segment(text, *color, *bold);
            }
        }
        out.push(line);
    }
    out
}

fn build_sections(data: &FetchData, pal: &Palette, config: &Config) -> Sections {
    let on = |id: &str| {
        config
            .enabled
            .as_deref()
            .is_none_or(|list| list.iter().any(|item| item == id))
    };
    let memory_warn = config.memory_warn.unwrap_or(85);
    let swap_warn = config.swap_warn.unwrap_or(50);
    let disk_warn = config.disk_warn.unwrap_or(90);
    let battery_low = config.battery_low.unwrap_or(15);
    let temp_warn = config.temp_warn.unwrap_or(85);
    let temp_crit = config.temp_crit.unwrap_or(95);
    let gpu_warn = config.gpu_warn.unwrap_or(95);
    let mut sections = Sections::default();

    if on("os") {
        sections.system.push(Row {
            id: "os",
            key: "OS",
            value: data.os.clone(),
            tone: None,
        });
    }
    if on("host") {
        sections.system.push(Row {
            id: "host",
            key: "Host",
            value: data.host.clone(),
            tone: None,
        });
    }
    if on("kernel") {
        sections.system.push(Row {
            id: "kernel",
            key: "Kernel",
            value: data.kernel.clone(),
            tone: None,
        });
    }
    if on("uptime") {
        sections.system.push(Row {
            id: "uptime",
            key: "Uptime",
            value: data.uptime.clone(),
            tone: None,
        });
    }
    if on("session") {
        sections.system.push(Row {
            id: "session",
            key: "Session",
            value: data.session.clone(),
            tone: None,
        });
    }
    if on("shell") {
        sections.system.push(Row {
            id: "shell",
            key: "Shell",
            value: data.shell.clone(),
            tone: None,
        });
    }
    if on("terminal") {
        sections.system.push(Row {
            id: "terminal",
            key: "Terminal",
            value: data.terminal.clone(),
            tone: None,
        });
    }
    if on("compositor") {
        sections.system.push(Row {
            id: "compositor",
            key: "Compositor",
            value: data.compositor.clone(),
            tone: None,
        });
    }
    if on("load") {
        if let Some(load) = &data.load {
            let first = load
                .split_whitespace()
                .next()
                .and_then(|v| v.parse::<f64>().ok());
            let tone = first
                .filter(|value| *value > data.cores as f64 * 0.9)
                .map(|_| pal.warn);
            sections.system.push(Row {
                id: "load",
                key: "Load",
                value: load.clone(),
                tone,
            });
        }
    }

    if on("cpu") && !data.cpu.is_empty() {
        sections.core.push(Row {
            id: "cpu",
            key: "CPU",
            value: data.cpu.clone(),
            tone: None,
        });
    }
    if on("gpu") {
        if let Some(gpu) = &data.gpu {
            sections.core.push(Row {
                id: "gpu",
                key: "GPU",
                value: gpu.clone(),
                tone: None,
            });
        }
    }
    if on("gpu_util") {
        if let Some(utilization) = data.gpu_util {
            sections.core.push(Row {
                id: "gpu_util",
                key: "GPU util",
                value: format!("{}%", utilization),
                tone: if utilization >= gpu_warn {
                    Some(pal.warn)
                } else {
                    None
                },
            });
        }
    }
    if on("gpu_vram") {
        if let Some(vram) = &data.gpu_vram {
            sections.core.push(Row {
                id: "gpu_vram",
                key: "VRAM",
                value: vram.clone(),
                tone: None,
            });
        }
    }
    if on("memory") {
        let tone = if data.memory_pct >= memory_warn {
            Some(pal.warn)
        } else {
            None
        };
        sections.core.push(Row {
            id: "memory",
            key: "Memory",
            value: data.memory.clone(),
            tone,
        });
    }
    if on("swap") {
        if let Some(swap) = &data.swap {
            let tone = if data.swap_pct >= swap_warn {
                Some(pal.warn)
            } else {
                None
            };
            sections.core.push(Row {
                id: "swap",
                key: "Swap",
                value: swap.clone(),
                tone,
            });
        }
    }
    let disk_tone = |pct: u8| {
        if pct >= disk_warn {
            Some(pal.warn)
        } else {
            None
        }
    };
    if on("disk_root") {
        if let Some(disk) = &data.disk {
            sections.core.push(Row {
                id: "disk_root",
                key: "Disk (/)",
                value: disk.clone(),
                tone: disk_tone(data.disk_pct),
            });
        }
    }
    if on("disk_media") {
        if let Some(media) = &data.media {
            sections.core.push(Row {
                id: "disk_media",
                key: "Media",
                value: media.clone(),
                tone: disk_tone(data.media_pct),
            });
        }
    }
    if on("disk_code") {
        if let Some(code) = &data.code {
            sections.core.push(Row {
                id: "disk_code",
                key: "Code",
                value: code.clone(),
                tone: disk_tone(data.code_pct),
            });
        }
    }
    if on("ip") {
        if let Some(ip) = &data.ip {
            sections.core.push(Row {
                id: "ip",
                key: "IP",
                value: ip.clone(),
                tone: None,
            });
        }
    }
    if on("link") {
        if let Some(link) = &data.link {
            sections.core.push(Row {
                id: "link",
                key: "Link",
                value: link.clone(),
                tone: None,
            });
        }
    }
    if on("processes") {
        if let Some(processes) = &data.processes {
            sections.core.push(Row {
                id: "processes",
                key: "Processes",
                value: processes.clone(),
                tone: None,
            });
        }
    }

    if on("battery") {
        if let Some(battery) = &data.battery {
            let tone = if data.charging {
                Some(pal.ok)
            } else if data.battery_pct <= battery_low {
                Some(pal.err)
            } else {
                None
            };
            sections.power.push(Row {
                id: "battery",
                key: "Battery",
                value: battery.clone(),
                tone,
            });
        }
    }
    if on("battery_health") {
        if let Some(health) = data.battery_health {
            let tone = if health <= 70 { Some(pal.warn) } else { None };
            sections.power.push(Row {
                id: "battery_health",
                key: "Battery health",
                value: format!("{}%", health),
                tone,
            });
        }
    }
    if on("cpu_temp") {
        if let Some(temp) = data.cpu_temp {
            let tone = if temp >= temp_crit {
                Some(pal.err)
            } else if temp >= temp_warn {
                Some(pal.warn)
            } else {
                None
            };
            sections.power.push(Row {
                id: "cpu_temp",
                key: "CPU temp",
                value: fmt_temp(temp, config.fahrenheit()),
                tone,
            });
        }
    }
    if on("gpu_temp") {
        if let Some(temp) = data.gpu_temp {
            let tone = if temp >= temp_crit {
                Some(pal.err)
            } else if temp >= temp_warn {
                Some(pal.warn)
            } else {
                None
            };
            sections.power.push(Row {
                id: "gpu_temp",
                key: "GPU temp",
                value: fmt_temp(temp, config.fahrenheit()),
                tone,
            });
        }
    }

    if let Some(list) = &config.enabled {
        let position = |id: &str| {
            list.iter()
                .position(|item| item == id)
                .unwrap_or(usize::MAX)
        };
        for rows in [
            &mut sections.system,
            &mut sections.core,
            &mut sections.power,
        ] {
            rows.sort_by_key(|row| position(row.id));
        }
    }
    sections
}

#[allow(clippy::too_many_arguments)]
fn render(
    theme: &Theme,
    data: &FetchData,
    note: &Note,
    width: usize,
    seed: u64,
    title_variant: bool,
    demo: bool,
    config: &Config,
) -> Vec<Line> {
    let pal = &theme.pal;
    let rule_char = config.rule_char();
    let mut lines = Vec::new();

    let mut header = Line::new();
    header.push("TACHYON", Some(pal.accent), true);
    header.push(" // ", Some(pal.dim), false);
    header.push(
        if title_variant {
            "SYSTEMS"
        } else {
            "FLIGHT DECK"
        },
        Some(pal.fg),
        true,
    );
    if demo {
        header.push(" · SIM", Some(pal.warn), false);
    }
    let right = format!(
        "{} · {}",
        theme.name.to_uppercase(),
        theme.mode.to_uppercase()
    );
    let right_width = width_of(&right);
    if header.width + right_width + 2 <= width {
        header.pad(width - header.width - right_width);
        header.push(&right, Some(pal.dim), false);
    }
    lines.push(header);

    let subtitle = format!("{}@{}", data.user, data.hostname);
    let mut sub = Line::new();
    sub.push(&subtitle, Some(pal.dim), false);
    lines.push(sub);

    let mut rule = Line::new();
    rule.push(&rule_char.to_string().repeat(width), Some(pal.dim), false);
    lines.push(rule);

    let sections = build_sections(data, pal, config);
    let two_columns = theme.layout != "stack"
        && width >= 120
        && !sections.core.is_empty()
        && !(sections.system.is_empty() && sections.power.is_empty());

    if two_columns {
        let left_width = (width - 2) / 2;
        let right_width = width - 2 - left_width;
        let mut left = section(
            config::section_glyph(config, "system", seed),
            Section::System.title(),
            &sections.system,
            left_width,
            pal,
            rule_char,
            config,
            seed,
        );
        if !sections.power.is_empty() {
            if !left.is_empty() {
                left.push(Line::new());
            }
            left.extend(section(
                config::section_glyph(config, "power", seed),
                Section::Power.title(),
                &sections.power,
                left_width,
                pal,
                rule_char,
                config,
                seed,
            ));
        }
        let right = section(
            config::section_glyph(config, "core", seed),
            Section::Core.title(),
            &sections.core,
            right_width,
            pal,
            rule_char,
            config,
            seed,
        );
        lines.extend(hcat(&left, &right, left_width, 2));
    } else {
        for (section_kind, rows) in [
            (Section::System, &sections.system),
            (Section::Core, &sections.core),
            (Section::Power, &sections.power),
        ] {
            if !rows.is_empty() {
                lines.extend(section(
                    config::section_glyph(config, section_kind.key(), seed),
                    section_kind.title(),
                    rows,
                    width,
                    pal,
                    rule_char,
                    config,
                    seed,
                ));
            }
        }
    }

    if !note.text.is_empty() {
        lines.push(Line::new());
        let mut quote = Line::new();
        quote.push(&truncate(&note.text, width), Some(pal.dim), false);
        lines.push(quote);
        if let Some(author) = &note.author {
            let mut line = Line::new();
            line.push("  -", Some(pal.accent2), false);
            line.push(
                &truncate(author, width.saturating_sub(3)),
                Some(pal.accent2),
                false,
            );
            lines.push(line);
        }
    }

    lines
}

fn argument(args: &mut impl Iterator<Item = String>, flag: &str) -> String {
    match args.next() {
        Some(value) => value,
        None => {
            eprintln!("tachyon: missing value for {}", flag);
            eprintln!("tachyon: run 'tachyon --help' for usage");
            process::exit(2);
        }
    }
}

fn parse_args() -> Opts {
    let mut opts = Opts::default();
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--demo" => opts.demo = true,
            "--plain" => opts.plain = true,
            "--static" => opts.no_motion = true,
            "--no-logo" => opts.no_logo = true,
            "-h" | "--help" => opts.help = true,
            "-V" | "--version" => opts.version = true,
            "--list-themes" => opts.list_themes = true,
            "--fonts" => opts.list_fonts = true,
            "--list-logos" => opts.list_logos = true,
            "--tui" => opts.tui = true,
            "--layout" => opts.layout = Some(argument(&mut args, "--layout")),
            "--glyph" => opts.glyph = Some(argument(&mut args, "--glyph")),
            "--album" => opts.album = Some(argument(&mut args, "--album")),
            "--theme" => opts.theme = Some(argument(&mut args, "--theme")),
            "--variant" => opts.variant = Some(argument(&mut args, "--variant")),
            "--seed" => {
                let raw = argument(&mut args, "--seed");
                match raw.parse() {
                    Ok(seed) => opts.seed = Some(seed),
                    Err(_) => {
                        eprintln!("tachyon: invalid seed: {}", raw);
                        process::exit(2);
                    }
                }
            }
            "--logo" => opts.logo = Some(PathBuf::from(argument(&mut args, "--logo"))),
            "--palettes" => opts.palettes = Some(PathBuf::from(argument(&mut args, "--palettes"))),
            "--quotes" => opts.quotes = Some(PathBuf::from(argument(&mut args, "--quotes"))),
            "--logos" => opts.logos = Some(PathBuf::from(argument(&mut args, "--logos"))),
            "--config" => opts.config = Some(PathBuf::from(argument(&mut args, "--config"))),
            "--state" => opts.state = Some(PathBuf::from(argument(&mut args, "--state"))),
            "--convert" => {
                let source = argument(&mut args, "--convert");
                let destination = argument(&mut args, "--convert");
                opts.convert_source = Some(PathBuf::from(source));
                opts.convert_destination = Some(PathBuf::from(destination));
            }
            other => {
                eprintln!("tachyon: unknown argument: {}", other);
                process::exit(2);
            }
        }
    }
    opts
}

fn help() {
    println!(
        "tachyon {}\n\
         \n\
         usage: tachyon [options]\n\
         \n\
         --demo            render synthetic values (no host reads)\n\
         --tui             open the module/layout/album configurator\n\
         --plain           no colours, no image\n\
         --static          disable the reveal animation\n\
         --no-logo         do not draw any logo\n\
         --layout <name>   hud or stack\n\
         --glyph <char>    header glyph (random or a character)\n\
         --album <name>    pick the artwork from an album\n\
         --theme <name>    override the active palette\n\
         --variant <mode>  dark or light\n\
         --seed <n>        deterministic layout/quote selection\n\
         --logo <path>     PNG for the kitty graphics protocol\n\
         --palettes <path> palette table (default ~/.local/share/tachyon/palettes.tsv)\n\
         --quotes <path>   quote pool (default ~/.config/tachyon/quotes.txt)\n\
         --config <path>   module/album config (default ~/.config/tachyon/fetch.conf)\n\
         --state <path>    theme state (default ~/.local/state/tachyon/theme)\n\
         --list-themes     print the built-in and installed palettes\n\
         --fonts           list system font families (fontconfig)\n\
         --convert <src> <dst>  convert an image to PNG (used for album ingest)\n\
         -h, --help        show this help\n\
         -V, --version     show the version",
        VERSION
    );
}

fn list_themes(opts: &Opts) {
    let mut names = theme_names(opts);
    names.sort();
    names.dedup();
    for name in names {
        println!("{}", name);
    }
}

fn list_fonts() {
    for family in font::list_system_fonts() {
        println!("{}", family);
    }
}

const MODULES: &[(&str, &str)] = &[
    ("os", "OS"),
    ("host", "Host"),
    ("kernel", "Kernel"),
    ("uptime", "Uptime"),
    ("session", "Session"),
    ("shell", "Shell"),
    ("terminal", "Terminal"),
    ("compositor", "Compositor"),
    ("load", "Load"),
    ("cpu", "CPU"),
    ("gpu", "GPU"),
    ("gpu_util", "GPU util"),
    ("gpu_vram", "VRAM"),
    ("memory", "Memory"),
    ("swap", "Swap"),
    ("disk_root", "Disk (/)"),
    ("disk_media", "Media"),
    ("disk_code", "Code"),
    ("ip", "IP"),
    ("link", "Link"),
    ("processes", "Processes"),
    ("battery", "Battery"),
    ("battery_health", "Battery health"),
    ("cpu_temp", "CPU temp"),
    ("gpu_temp", "GPU temp"),
];

fn list_albums() -> Vec<String> {
    let mut albums: Vec<String> = fs::read_dir(albums_root())
        .ok()
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().is_dir())
                .map(|entry| entry.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    albums.sort();
    albums
}

fn add_photo(album: &str, source: &Path) -> Result<PathBuf, String> {
    if !source.is_file() {
        return Err(format!("not a file: {}", source.display()));
    }
    let dir = albums_root().join(album);
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    let stem: String = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("image")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let stem = if stem.is_empty() {
        "image".to_string()
    } else {
        stem
    };
    let mut destination = dir.join(format!("{}.png", stem));
    let mut index = 1;
    while destination.exists() {
        destination = dir.join(format!("{}-{}.png", stem, index));
        index += 1;
    }
    let is_png = source
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.eq_ignore_ascii_case("png"))
        .unwrap_or(false);
    if is_png {
        fs::copy(source, &destination).map_err(|error| error.to_string())?;
    } else {
        convert_photo(source, &destination)?;
    }
    Ok(destination)
}

fn convert_photo(source: &Path, destination: &Path) -> Result<(), String> {
    let bytes = fs::read(source).map_err(|error| error.to_string())?;
    let format = image::guess_format(&bytes).map_err(|error| error.to_string())?;
    let picture =
        image::load_from_memory_with_format(&bytes, format).map_err(|error| error.to_string())?;
    let picture = if picture.width() > 1600 {
        picture.resize(1600, u32::MAX, image::imageops::FilterType::Lanczos3)
    } else {
        picture
    };
    picture
        .save_with_format(destination, image::ImageFormat::Png)
        .map_err(|error| error.to_string())
}

fn main() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    let mut opts = parse_args();
    if opts.help {
        help();
        return;
    }
    if opts.version {
        println!("tachyon {}", VERSION);
        return;
    }
    if opts.list_themes {
        list_themes(&opts);
        return;
    }
    if opts.list_fonts {
        list_fonts();
        return;
    }
    if opts.list_logos {
        let mut ids = ascii::ids();
        ids.extend(ascii::runtime_ids(&ascii::logos_dir(&opts)));
        ids.sort();
        ids.dedup();
        for id in ids {
            println!("{}", id);
        }
        return;
    }
    if let (Some(source), Some(destination)) = (&opts.convert_source, &opts.convert_destination) {
        match convert_photo(source, destination) {
            Ok(()) => return,
            Err(error) => {
                eprintln!("tachyon: convert failed: {}", error);
                process::exit(1);
            }
        }
    }
    let mut config = load_config(&config_path(&opts));
    if let Some(glyph) = opts.glyph.clone() {
        config.glyph = Some(glyph);
    }
    if opts.tui {
        if let Err(error) = tui::run(&opts, &config) {
            eprintln!("tachyon: tui failed: {}", error);
        }
        return;
    }

    let album = opts.album.clone().or_else(|| config.album.clone());
    let art_len = album
        .as_deref()
        .map(|album| album_images(album).len())
        .filter(|len| *len > 0);
    let rows = palette_rows(&opts);
    let mode = opts
        .variant
        .clone()
        .or_else(|| config.variant.clone())
        .or_else(|| read_state(&state_path(&opts)).1)
        .unwrap_or_else(|| "dark".to_string());
    let rotation_themes = valid_themes(
        &rows,
        &mode,
        &config.rotate_themes.clone().unwrap_or_default(),
    );
    let rotation = rotate::compute(
        &config,
        rotate::load_state(&rotation_path(&opts)),
        art_len,
        &rotation_themes,
        opts.seed.unwrap_or_else(seed_now),
        now_secs(),
    );
    if rotation.save {
        rotate::write_state(&rotation_path(&opts), rotation.state);
    }
    if let Some(theme) = rotation.theme {
        opts.theme = Some(theme);
    }
    opts.rotated_art = rotation.art;

    let theme = load_theme(&opts, &config);
    if let Some(layout) = &opts.layout {
        if layout != "hud" && layout != "stack" {
            eprintln!("tachyon: unknown layout: {}", layout);
            process::exit(2);
        }
    }
    let layout = opts
        .layout
        .clone()
        .or_else(|| config.layout.clone())
        .unwrap_or_else(|| theme.layout.clone());
    let motion = config
        .motion
        .clone()
        .unwrap_or_else(|| theme.motion.clone());
    let mut theme = Theme {
        layout,
        motion,
        ..theme
    };

    let mut rng = opts.seed.unwrap_or_else(|| {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        nanos ^ (process::id() as u64)
    });

    let quotes = load_quotes(&quotes_path(&opts));
    let note = resolve_note(&config, &quotes, &mut rng);
    let glyph_seed = next_rand(&mut rng);
    let title_variant = next_rand(&mut rng).is_multiple_of(2);

    let data = if opts.demo {
        demo_data()
    } else {
        collect_data(&config)
    };

    let (cols, _, xpixel, ypixel) = term_size();
    let tty = io::stdout().is_terminal();

    let logo_gap = config.logo_gap.unwrap_or(3) as usize;
    let logo_cap = |cols: usize| {
        let width_pct = config.logo_width.unwrap_or(34) as f64 / 100.0;
        let cap = ((cols as f64) * width_pct).round() as usize;
        cap.clamp(
            config.logo_min.unwrap_or(26) as usize,
            config.logo_max.unwrap_or(46) as usize,
        )
    };
    let logos_dir = ascii::logos_dir(&opts);
    let ascii_id = config
        .logo_art
        .clone()
        .unwrap_or_else(|| ascii::detect(opts.demo));
    let wants_ascii = config.logo_mode() == "ascii"
        && tty
        && !opts.plain
        && !opts.no_logo
        && theme.layout != "stack";
    let ascii_art = if wants_ascii {
        let fits = |art: &ascii::LoadedArt| {
            let width = ascii::width(&art.lines);
            width > 0 && width + logo_gap + 56 < cols
        };
        ascii::load(&ascii_id, &logos_dir, false)
            .filter(fits)
            .or_else(|| ascii::load(&ascii_id, &logos_dir, true).filter(fits))
    } else {
        None
    };
    if let Some(art) = &ascii_art {
        if config.logo_color() == "derive" {
            theme.pal = ascii::derive_palette(&art.colors, &theme.pal);
        }
    }
    let ascii_lines = ascii_art.as_ref().map(|art| {
        let colors = if config.logo_color() == "native" && !art.colors.is_empty() {
            art.colors.clone()
        } else {
            ascii::theme_colors(&theme.pal)
        };
        ascii::rendered(art, &colors)
            .into_iter()
            .map(|spans| {
                let mut line = Line::new();
                for (text, color) in spans {
                    line.push(&text, Some(color), false);
                }
                line
            })
            .collect::<Vec<Line>>()
    });
    let ascii_width = ascii_art
        .as_ref()
        .map(|art| ascii::width(&art.lines))
        .unwrap_or(0);
    let wants_text = config.logo_mode() == "text"
        && tty
        && !opts.plain
        && !opts.no_logo
        && theme.layout != "stack";
    let text_logo = if wants_text {
        text_logo(&config, &data.hostname, logo_cap(cols), &theme.pal)
            .filter(|logo| cols > logo.width + logo_gap + 56)
    } else {
        None
    };
    let logo = if config.logo_mode() == "image"
        && tty
        && !opts.plain
        && !opts.no_logo
        && theme.layout != "stack"
        && in_kitty()
    {
        select_logo(&opts, &config, &mut rng).and_then(|path| {
            let (img_w, img_h) = png_size(&path)?;
            let cols_for_logo = logo_cap(cols);
            let indent = cols_for_logo + logo_gap;
            if cols <= indent + 56 {
                return None;
            }
            let cell_ratio = if xpixel > 0 && ypixel > 0 {
                (xpixel as f64 / cols as f64) / (ypixel as f64 / term_size().1 as f64)
            } else {
                0.5
            };
            let rows = ((cols_for_logo as f64) * cell_ratio * (img_h as f64 / img_w as f64)).ceil()
                as usize;
            Some((path, cols_for_logo, rows.max(3)))
        })
    } else {
        None
    };

    let indent = if ascii_width > 0 {
        ascii_width + logo_gap
    } else if let Some(logo) = &text_logo {
        logo.width + logo_gap
    } else {
        logo.as_ref()
            .map(|(_, cols_for_logo, _)| cols_for_logo + logo_gap)
            .unwrap_or(0)
    };
    let width = cols.saturating_sub(indent + 1).max(30);
    let mut lines = render(
        &theme,
        &data,
        &note,
        width,
        glyph_seed,
        title_variant,
        opts.demo,
        &config,
    );
    if let (Some(art_lines), false) = (&ascii_lines, logo_side_right(&config)) {
        lines = hcat(art_lines, &lines, ascii_width, logo_gap);
    } else if let (Some(art_lines), true) = (&ascii_lines, logo_side_right(&config)) {
        lines = hcat(&lines, art_lines, width, logo_gap);
    } else if let Some(logo) = &text_logo {
        if logo_side_right(&config) {
            lines = hcat(&lines, &logo.lines, width, logo_gap);
        } else {
            lines = compose_logo(logo, lines, logo_gap);
        }
    }
    let print_indent = if ascii_lines.is_some() || text_logo.is_some() || logo_side_right(&config) {
        0
    } else {
        indent
    };

    let use_color = !opts.plain
        && env::var_os("NO_COLOR").is_none()
        && (tty || env::var_os("CLICOLOR_FORCE").is_some());
    let animate = !opts.no_motion && theme.motion != "off" && tty;

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if let Some((path, columns, _)) = &logo {
        let encoded = b64(path.to_string_lossy().as_bytes());
        let shift = if logo_side_right(&config) {
            width + logo_gap
        } else {
            0
        };
        let _ = write!(out, "\x1b[s");
        if shift > 0 {
            let _ = write!(out, "\x1b[{}C", shift);
        }
        let _ = write!(
            out,
            "\x1b_Ga=T,f=100,t=f,c={},C=1;{}\x1b\\\x1b[u",
            columns, encoded
        );
        if print_indent > 0 {
            let _ = write!(out, "\x1b[{}C", print_indent);
        }
    }

    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            let _ = write!(out, "\r\n");
            if print_indent > 0 {
                let _ = write!(out, "\x1b[{}C", print_indent);
            }
        }
        if use_color {
            let _ = write!(out, "{}", line.text);
        } else {
            let _ = write!(out, "{}", strip_ansi(&line.text));
        }
        if animate {
            let _ = out.flush();
            thread::sleep(Duration::from_millis(7));
        }
    }
    let _ = write!(out, "\r\n");

    if let Some((_, _, rows)) = &logo {
        if *rows > lines.len() {
            for _ in lines.len()..*rows {
                let _ = write!(out, "\r\n");
            }
        }
    }
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_release_pretty_name() {
        let text = "NAME=\"NixOS\"\nPRETTY_NAME=\"NixOS 26.11 (x86_64)\"\n";
        assert_eq!(parse_os_release(text), "NixOS 26.11 (x86_64)");
    }

    #[test]
    fn cpu_model_and_cores() {
        let text = "processor\t: 0\nmodel name\t: AMD Ryzen 7 7840HS w/ Radeon 780M Graphics\nprocessor\t: 1\nmodel name\t: AMD Ryzen 7 7840HS w/ Radeon 780M Graphics\n";
        let (model, cores) = parse_cpuinfo(text);
        assert_eq!(cores, 2);
        assert!(model.contains("Ryzen 7 7840HS"));
        assert!(!model.contains("(R)"));
    }

    #[test]
    fn meminfo_parses() {
        let text = "MemTotal:       16000000 kB\nMemFree:         1000000 kB\nMemAvailable:    6000000 kB\n";
        assert_eq!(parse_meminfo(text), Some((16_000_000, 6_000_000)));
    }

    #[test]
    fn fib_trie_skips_loopback() {
        let text = "     |-- 127.0.0.1\n        /32 host LOCAL\n     |-- 10.0.0.42\n        /32 host LOCAL\n";
        assert_eq!(parse_fib_trie(text).as_deref(), Some("10.0.0.42"));
    }

    #[test]
    fn uptime_formats() {
        assert_eq!(fmt_uptime(0), "0m");
        assert_eq!(fmt_uptime(3_600), "1h 00m");
        assert_eq!(fmt_uptime(90_061), "1d 01h 01m");
    }

    #[test]
    fn palette_row_parses() {
        let line = "Solitude\tdark\t#101315\t#cacccc\t#4b4e55\t#798186\t#707070\t#9fa5a9\t#de6145\t#de6145\thud\treveal";
        let parsed = parse_palette_row(line).expect("row");
        assert_eq!(parsed.0, "Solitude");
        assert_eq!(parsed.1, "dark");
        assert_eq!(parsed.2.accent, (0x79, 0x81, 0x86));
        assert_eq!(parsed.3, "hud");
        assert_eq!(parsed.4, "reveal");
    }

    #[test]
    fn base64_matches() {
        assert_eq!(b64(b"hi"), "aGk=");
        assert_eq!(b64(b"abc"), "YWJj");
    }

    #[test]
    fn truncate_keeps_width() {
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(truncate("abc", 4), "abc");
        assert_eq!(truncate("abc", 0), "");
    }

    #[test]
    fn base64_handles_padding() {
        assert_eq!(b64(b""), "");
        assert_eq!(b64(b"hi"), "aGk=");
        assert_eq!(b64(b"hai"), "aGFp");
    }

    #[test]
    fn parse_color_accepts_hex_and_rejects_garbage() {
        assert_eq!(parse_color("#0f10ff"), Some((0x0f, 0x10, 0xff)));
        assert_eq!(parse_color("#A1B2C3"), Some((0xa1, 0xb2, 0xc3)));
        assert_eq!(parse_color("a1b2c3"), None);
        assert_eq!(parse_color("#12345"), None);
        assert_eq!(parse_color("#1234567"), None);
        assert_eq!(parse_color("#gggggg"), None);
    }

    #[test]
    fn sizes_and_temperatures_format() {
        assert_eq!(fmt_size(1024 * 1024 * 1024, false), "1 GiB");
        assert_eq!(fmt_size(1000 * 1000 * 1000, true), "1 GB");
        assert_eq!(fmt_temp(100, false), "100 °C");
        assert_eq!(fmt_temp(100, true), "212 °F");
    }

    #[test]
    fn ipv4_validation() {
        assert!(is_ipv4("10.0.0.42"));
        assert!(is_ipv4("255.255.255.255"));
        assert!(!is_ipv4("10.0.0"));
        assert!(!is_ipv4("10.0.0.256"));
        assert!(!is_ipv4("10.0.0."));
        assert!(!is_ipv4("fd00::1"));
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = env::temp_dir().join(format!("tachyon-test-{}-{}", process::id(), name));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    #[test]
    fn config_roundtrip_keeps_colors_and_values() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("fetch.conf");
        let config = Config {
            enabled: Some(vec!["memory".to_string(), "swap".to_string()]),
            theme: Some("Rose Pine".to_string()),
            units: Some("si".to_string()),
            temp_unit: Some("f".to_string()),
            memory_warn: Some(77),
            logo_gap: Some(5),
            rule: Some("~".to_string()),
            color_accent: Some((0x6e, 0xe7, 0xff)),
            color_bg: Some((16, 19, 21)),
            ..Config::default()
        };
        write_config(&path, &config).expect("write");
        let loaded = load_config(&path);
        assert_eq!(
            loaded.enabled.as_deref(),
            Some(["memory".to_string(), "swap".to_string()].as_slice())
        );
        assert_eq!(loaded.theme.as_deref(), Some("Rose Pine"));
        assert_eq!(loaded.units.as_deref(), Some("si"));
        assert_eq!(loaded.temp_unit.as_deref(), Some("f"));
        assert_eq!(loaded.memory_warn, Some(77));
        assert_eq!(loaded.logo_gap, Some(5));
        assert_eq!(loaded.rule.as_deref(), Some("~"));
        assert_eq!(loaded.color_accent, Some((0x6e, 0xe7, 0xff)));
        assert_eq!(loaded.color_bg, Some((16, 19, 21)));
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn enabled_order_drives_the_sections() {
        let data = demo_data();
        let config = Config {
            enabled: Some(vec!["memory".to_string(), "swap".to_string()]),
            ..Config::default()
        };
        let sections = build_sections(&data, &FALLBACK_PALETTE, &config);
        assert!(sections.core.len() >= 2);
        assert_eq!(sections.core[0].id, "memory");
        assert_eq!(sections.core[1].id, "swap");
        assert!(sections.system.is_empty());
    }

    #[test]
    fn fallback_quotes_keep_the_full_attributed_pool() {
        assert_eq!(FALLBACK_QUOTES.len(), 51);
        for quote in FALLBACK_QUOTES {
            assert!(quote.starts_with('“'), "missing opening quote: {quote}");
            assert!(quote.contains("” — "), "missing attribution: {quote}");
        }
    }

    #[test]
    fn theme_variant_availability_is_honest() {
        let opts = Opts {
            demo: true,
            palettes: Some(PathBuf::from("/nonexistent-palettes.tsv")),
            ..Opts::default()
        };
        let rows = palette_rows(&opts);
        let availability = theme_availability(&rows);
        let find = |name: &str| {
            availability
                .iter()
                .find(|(theme, _, _)| theme == name)
                .map(|(_, dark, light)| (*dark, *light))
                .unwrap()
        };
        assert_eq!(find("Solitude"), (true, false));
        assert_eq!(find("Vantablack"), (true, false));
        assert_eq!(find("Hackerman"), (true, false));
        assert_eq!(find("Retro 82"), (true, false));
        assert_eq!(find("Rose Pine"), (true, true));
        assert_eq!(find("Flexoki Light"), (true, true));
        let wanted = vec!["Hackerman".to_string(), "Rose Pine".to_string()];
        assert_eq!(
            valid_themes(&rows, "light", &wanted),
            vec!["Rose Pine".to_string()]
        );
        assert_eq!(valid_themes(&rows, "dark", &wanted), wanted);
        let mut rng = 7u64;
        let quotes = vec!["one".to_string(), "two".to_string()];
        let mut config = Config::default();
        assert!(!resolve_note(&config, &quotes, &mut rng).text.is_empty());
        config.quote = Some("off".to_string());
        assert!(resolve_note(&config, &quotes, &mut rng).text.is_empty());
        config.quote = Some("“Pinned.” — Someone".to_string());
        let note = resolve_note(&config, &quotes, &mut rng);
        assert_eq!(note.text, "“Pinned.”");
        assert_eq!(note.author.as_deref(), Some("Someone"));
        config.note_mode = Some("text".to_string());
        config.note_text = Some("plain line".to_string());
        assert_eq!(resolve_note(&config, &quotes, &mut rng).text, "plain line");
        config.note_mode = Some("emoticon".to_string());
        config.emoticon = Some("(o_o)".to_string());
        let note = resolve_note(&config, &quotes, &mut rng);
        assert_eq!(note.text, "(o_o)");
        assert!(note.author.is_none());
        config.note_mode = Some("off".to_string());
        assert!(resolve_note(&config, &quotes, &mut rng).text.is_empty());
    }

    #[test]
    fn config_skips_comments_and_unknown_keys() {
        let dir = temp_dir("comments");
        let path = dir.join("fetch.conf");
        fs::write(
            &path,
            "# leading comment\n   \nenabled = memory\nnonsense = 5\ncolor.fg = #A1B2C3\nunits = si # inline annotations are not stripped\n",
        )
        .expect("write");
        let loaded = load_config(&path);
        assert_eq!(
            loaded.enabled.as_deref(),
            Some(["memory".to_string()].as_slice())
        );
        assert_eq!(loaded.color_fg, Some((0xa1, 0xb2, 0xc3)));
        assert_eq!(loaded.units, None);
        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn convert_photo_resizes_and_writes_png() {
        let dir = temp_dir("convert");
        let source = dir.join("in.png");
        let destination = dir.join("out.png");
        let wide = image::RgbImage::from_fn(2000, 10, |x, _| image::Rgb([x as u8, 40, 90]));
        wide.save_with_format(&source, image::ImageFormat::Png)
            .expect("save source");
        convert_photo(&source, &destination).expect("convert");
        assert_eq!(png_size(&destination), Some((1600, 8)));
        fs::remove_dir_all(&dir).ok();
    }
}
