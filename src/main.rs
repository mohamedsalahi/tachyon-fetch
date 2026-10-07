use std::env;
use std::ffi::CString;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const VERSION: &str = "0.1.0";

const FALLBACK_QUOTES: &[&str] = &[
    "Precision is a form of mercy.",
    "Calm is a decision, not a state.",
    "The instrument does not care; the pilot does.",
    "Read twice, cut once, then verify.",
    "Slow is smooth. Smooth is fast.",
    "A cold system is a kind system.",
    "Own the tool or serve it.",
    "Simplify until it hurts, then stop.",
    "Signal over noise, always.",
    "The checklist is the shortcut.",
];

type Rgb = (u8, u8, u8);

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
            b"libnvidia-ml.so.1\0".as_ptr() as *const libc::c_char,
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
        let mut memory = NvmlMemory { total: 0, free: 0, used: 0 };
        let has_memory = (nvml.memory)(nvml.device, &mut memory) == 0 && memory.total > 0;
        let mut temperature = 0u32;
        let has_temperature = (nvml.temperature)(nvml.device, 0, &mut temperature) == 0;
        if !has_utilization && !has_memory && !has_temperature {
            return None;
        }
        let utilization_value = if has_utilization { utilization.gpu.min(100) as u8 } else { 0 };
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

#[derive(Clone, Copy)]
struct Palette {
    bg: Rgb,
    fg: Rgb,
    dim: Rgb,
    accent: Rgb,
    accent2: Rgb,
    ok: Rgb,
    warn: Rgb,
    err: Rgb,
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

struct Theme {
    name: String,
    mode: String,
    pal: Palette,
    layout: String,
    motion: String,
}

#[derive(Default)]
struct Opts {
    demo: bool,
    plain: bool,
    no_motion: bool,
    no_logo: bool,
    help: bool,
    version: bool,
    list_themes: bool,
    tui: bool,
    layout: Option<String>,
    glyph: Option<String>,
    album: Option<String>,
    theme: Option<String>,
    variant: Option<String>,
    seed: Option<u64>,
    logo: Option<PathBuf>,
    palettes: Option<PathBuf>,
    quotes: Option<PathBuf>,
    config: Option<PathBuf>,
    state: Option<PathBuf>,
    convert_source: Option<PathBuf>,
    convert_destination: Option<PathBuf>,
}

#[derive(Default, Clone)]
struct Config {
    enabled: Option<Vec<String>>,
    glyph: Option<String>,
    album: Option<String>,
    image: Option<PathBuf>,
    layout: Option<String>,
    motion: Option<String>,
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
}

struct Row {
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
struct Line {
    text: String,
    width: usize,
}

impl Line {
    fn new() -> Self {
        Self { text: String::new(), width: 0 }
    }

    fn push(&mut self, text: &str, color: Option<Rgb>, bold: bool) {
        self.width += width_of(text);
        match color {
            Some((r, g, b)) => {
                if bold {
                    self.text.push_str(&format!("\x1b[1;38;2;{};{};{}m", r, g, b));
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
            self.text.push_str(&" ".repeat(n));
            self.width += n;
        }
    }
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
    let mut out = String::with_capacity((input.len() + 2) / 3 * 4);
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
    env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    env::var_os(var).map(PathBuf::from).unwrap_or_else(|| home().join(fallback))
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

fn load_config(path: &Path) -> Config {
    let mut config = Config::default();
    if let Ok(text) = fs::read_to_string(path) {
        for line in text.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            match key {
                "enabled" => {
                    if value != "all" && !value.is_empty() {
                        config.enabled = Some(
                            value
                                .split(',')
                                .map(str::trim)
                                .filter(|item| !item.is_empty())
                                .map(str::to_string)
                                .collect(),
                        );
                    }
                }
                "glyph" => {},
                "album" => {},
                "image" => {},
                "layout" => {},
                "motion" => {},
                _ => {}
            }
        }
    }
    config
}

fn albums_root() -> PathBuf {
    env::var_os("TACHYON_FETCH_ALBUMS")
        .map(PathBuf::from)
        .unwrap_or_else(|| xdg("XDG_DATA_HOME", ".local/share").join("tachyon/albums"))
}

fn album_pick(album: &str, rng: &mut u64) -> Option<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(albums_root().join(album))
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .map(|value| value.eq_ignore_ascii_case("png"))
                .unwrap_or(false)
        })
        .collect();
    files.sort();
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

fn read_state(path: &Path) -> (Option<String>, Option<String>) {
    match fs::read_to_string(path) {
        Ok(text) => {
            let mut lines = text.lines();
            let name = lines.next().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
            let mode = lines.next().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
            (name, mode)
        }
        Err(_) => (None, None),
    }
}

fn parse_color(text: &str) -> Option<Rgb> {
    let hex = text.trim().strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
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

fn load_theme(opts: &Opts) -> Theme {
    let (state_name, state_mode) = read_state(&state_path(opts));
    let mut state_name = state_name;
    let mut state_mode = state_mode;
    if state_name.is_none() {
        let omarchy = xdg("XDG_STATE_HOME", ".local/state").join("omarchy/current/theme/theme.name");
        if let Ok(text) = fs::read_to_string(omarchy) {
            let name = text.trim();
            if !name.is_empty() {
                state_mode = Some(default_mode(name).to_string());
                state_name = Some(name.to_string());
            }
        }
    }
    let wanted = opts.theme.clone().or(state_name).unwrap_or_else(|| "Solitude".to_string());
    let mode = opts.variant.clone().or(state_mode).unwrap_or_else(|| "dark".to_string());

    let rows: Vec<_> = fs::read_to_string(palettes_path(opts))
        .ok()
        .map(|text| text.lines().filter_map(parse_palette_row).collect())
        .unwrap_or_default();

    let pick = |variant: &str| {
        rows.iter()
            .find(|(name, v, _, _, _)| name == &wanted && v == variant)
            .cloned()
    };
    let chosen = pick(&mode)
        .or_else(|| pick("dark"))
        .or_else(|| pick("light"))
        .or_else(|| rows.iter().find(|(name, _, _, _, _)| name == &wanted).cloned());

    match chosen {
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
    }
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

fn collapse_spaces(text: &str) -> String { unimplemented!("collapse_spaces") }

fn parse_cpuinfo(text: &str) -> (String, usize) { unimplemented!("parse_cpuinfo") }

fn parse_meminfo(text: &str) -> Option<(u64, u64)> { unimplemented!("parse_meminfo") }

fn meminfo_value(text: &str, key: &str) -> Option<u64> { unimplemented!("meminfo_value") }

fn hwmon_temp(kinds: &[&str]) -> Option<u8> { unimplemented!("hwmon_temp") }

fn link_info() -> Option<String> { unimplemented!("link_info") }

fn process_count() -> Option<String> { unimplemented!("process_count") }

fn parse_fib_trie(text: &str) -> Option<String> { unimplemented!("parse_fib_trie") }

fn is_ipv4(text: &str) -> bool { unimplemented!("is_ipv4") }

fn fmt_uptime(seconds: u64) -> String { unimplemented!("fmt_uptime") }

fn fmt_size(bytes: u64) -> String { unimplemented!("fmt_size") }

fn read_text(path: &str) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn read_trim(path: &str) -> Option<String> {
    read_text(path).map(|text| text.trim().to_string()).filter(|text| !text.is_empty())
}

fn term_size() -> (usize, usize, usize, usize) { unimplemented!("term_size") }

fn disk_usage(path: &str) -> Option<(u64, u64)> { unimplemented!("disk_usage") }

fn gpu_name() -> Option<String> { unimplemented!("gpu_name") }

fn battery_info() -> Option<(u8, String, bool, Option<u8>)> { unimplemented!("battery_info") }

fn collect_data() -> FetchData { unimplemented!("collect_data") }

fn demo_data() -> FetchData { unimplemented!("demo_data") }

fn png_size(path: &Path) -> Option<(u32, u32)> { unimplemented!("png_size") }

fn in_kitty() -> bool { unimplemented!("in_kitty") }

fn select_logo(opts: &Opts, config: &Config, rng: &mut u64) -> Option<PathBuf> { unimplemented!("select_logo") }

fn section(glyph: char, title: &str, rows: &[Row], width: usize, pal: &Palette) -> Vec<Line> { unimplemented!("section") }

fn hcat(left: &[Line], right: &[Line], left_width: usize, gap: usize) -> Vec<Line> { unimplemented!("hcat") }

fn build_sections(data: &FetchData, pal: &Palette, enabled: Option<&[String]>) -> Sections { unimplemented!("build_sections") }

fn render(
    theme: &Theme,
    data: &FetchData,
    quote: &str,
    width: usize,
    glyph: char,
    title_variant: bool,
    demo: bool,
    enabled: Option<&[String]>,
) -> Vec<Line> { unimplemented!("render") }

fn parse_args() -> Opts { unimplemented!("parse_args") }

fn help() { unimplemented!("help") }

fn list_themes(opts: &Opts) { unimplemented!("list_themes") }

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

const GLYPHS: &[&str] = &["random", "▸", "▚", "◆", "▰"];
const LAYOUTS: &[&str] = &["theme", "hud", "stack"];
const MOTIONS: &[&str] = &["theme", "reveal", "off"];

enum Key {
    Up,
    Down,
    Enter,
    Escape,
    Backspace,
    Char(char),
    Other,
}

struct RawTty {
    fd: i32,
    original: libc::termios,
}

impl RawTty {
    fn enable(file: &fs::File) -> Option<Self> { unimplemented!("enable") }
}

impl Drop for RawTty {
    fn drop(&mut self) {
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSANOW, &self.original);
        }
    }
}

struct ScreenGuard;

impl Drop for ScreenGuard {
    fn drop(&mut self) {
        let mut out = io::stdout();
        let _ = write!(out, "\x1b[?25h\x1b[?1049l");
        let _ = out.flush();
    }
}

fn read_key(tty: &mut fs::File) -> Option<Key> { unimplemented!("read_key") }

fn prompt_line(tty: &mut fs::File, label: &str) -> Option<String> { unimplemented!("prompt_line") }

fn list_albums() -> Vec<String> { unimplemented!("list_albums") }

fn add_photo(album: &str, source: &Path) -> Result<PathBuf, String> { unimplemented!("add_photo") }

fn convert_photo(source: &Path, destination: &Path) -> Result<(), String> { unimplemented!("convert_photo") }

struct TuiState {
    enabled: Vec<bool>,
    cursor: usize,
    glyph: usize,
    layout: usize,
    motion: usize,
    album: usize,
    albums: Vec<String>,
    status: String,
}

impl TuiState {
    fn new(config: &Config, _theme: &Theme) -> Self { unimplemented!("new") }
}

fn save_tui(path: &Path, state: &TuiState) -> io::Result<()> { unimplemented!("save_tui") }

fn draw_tui(state: &TuiState, theme: &Theme, path: &Path, width: usize, height: usize) { unimplemented!("draw_tui") }

fn run_tui(opts: &Opts) { unimplemented!("run_tui") }

fn main() {
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
    if let (Some(source), Some(destination)) = (&opts.convert_source, &opts.convert_destination) {
        match convert_photo(source, destination) {
            Ok(()) => return,
            Err(error) => {
                eprintln!("tachyon: convert failed: {}", error);
                process::exit(1);
            }
        }
    }
    if opts.tui {
        run_tui(&opts);
        return;
    }

    let config = load_config(&config_path(&opts));
    let theme = load_theme(&opts);
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
    let motion = config.motion.clone().unwrap_or_else(|| theme.motion.clone());
    let theme = Theme { layout, motion, ..theme };

    let mut rng = opts.seed.unwrap_or_else(|| {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
        nanos ^ (process::id() as u64)
    });

    let quotes = load_quotes(&quotes_path(&opts));
    let quote = if quotes.is_empty() {
        String::new()
    } else {
        quotes[(next_rand(&mut rng) % quotes.len() as u64) as usize].clone()
    };
    let glyphs = ['▸', '▚', '◆', '▰'];
    let glyph_choice = opts.glyph.clone().or_else(|| config.glyph.clone());
    let glyph = match glyph_choice.as_deref() {
        None | Some("random") | Some("mixed") => {
            glyphs[(next_rand(&mut rng) % glyphs.len() as u64) as usize]
        }
        Some(value) => value.chars().next().unwrap_or('▸'),
    };
    let title_variant = next_rand(&mut rng) % 2 == 0;

    let data = if opts.demo { demo_data() } else { collect_data() };

    let (cols, _, xpixel, ypixel) = term_size();
    let tty = io::stdout().is_terminal();

    let logo = if tty && !opts.plain && !opts.no_logo && theme.layout != "stack" && in_kitty() {
        select_logo(&opts, &config, &mut rng).and_then(|path| {
            let (img_w, img_h) = png_size(&path)?;
            let cols_for_logo = ((cols as f64) * 0.36).round() as usize;
            let cols_for_logo = cols_for_logo.clamp(24, 44);
            let indent = cols_for_logo + 3;
            if cols <= indent + 52 {
                return None;
            }
            let cell_ratio = if xpixel > 0 && ypixel > 0 {
                (xpixel as f64 / cols as f64) / (ypixel as f64 / term_size().1 as f64)
            } else {
                0.5
            };
            let rows = ((cols_for_logo as f64) * cell_ratio * (img_h as f64 / img_w as f64)).ceil() as usize;
            Some((path, cols_for_logo, rows.max(2)))
        })
    } else {
        None
    };

    let indent = logo.as_ref().map(|(_, cols_for_logo, _)| cols_for_logo + 3).unwrap_or(0);
    let width = cols.saturating_sub(indent + 1).max(30);
    let lines = render(
        &theme,
        &data,
        &quote,
        width,
        glyph,
        title_variant,
        opts.demo,
        config.enabled.as_deref(),
    );

    let use_color = !opts.plain
        && env::var_os("NO_COLOR").is_none()
        && (tty || env::var_os("CLICOLOR_FORCE").is_some());
    let animate = !opts.no_motion && theme.motion != "off" && tty;

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if let Some((path, columns, _)) = &logo {
        let encoded = b64(path.to_string_lossy().as_bytes());
        let _ = write!(
            out,
            "\x1b[s\x1b_Ga=T,f=100,t=f,c={},C=1;{}\x1b\\\x1b[u",
            columns, encoded
        );
        if indent > 0 {
            let _ = write!(out, "\x1b[{}C", indent);
        }
    }

    for (index, line) in lines.iter().enumerate() {
        if index > 0 {
            let _ = write!(out, "\r\n");
            if indent > 0 {
                let _ = write!(out, "\x1b[{}C", indent);
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
    }
}
