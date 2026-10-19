use std::env;
use std::ffi::CString;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::process;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const VERSION: &str = "0.2.0";

const FALLBACK_QUOTES: &[&str] = &[
    "Precision is a form of mercy.",
    "Calm is a decision, not a state.",
    "The instrument does not care; the pilot does.",
    "Read twice, cut once, then verify.",
    "Slow is smooth; smooth is fast.",
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
                "glyph" if !value.is_empty() => config.glyph = Some(value.to_string()),
                "album" if !value.is_empty() => config.album = Some(value.to_string()),
                "image" if !value.is_empty() => config.image = Some(PathBuf::from(value)),
                "layout" if value == "hud" || value == "stack" => {
                    config.layout = Some(value.to_string());
                }
                "motion" if value == "reveal" || value == "off" => {
                    config.motion = Some(value.to_string());
                }
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
            total = value.split_whitespace().next().and_then(|v| v.parse::<u64>().ok());
        } else if let Some(value) = line.strip_prefix("MemAvailable:") {
            available = value.split_whitespace().next().and_then(|v| v.parse::<u64>().ok());
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

fn fmt_size(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const TIB: f64 = GIB * 1024.0;
    let value = bytes as f64;
    if value >= TIB {
        format!("{:.1} TiB", value / TIB)
    } else {
        format!("{:.0} GiB", value / GIB)
    }
}

fn read_text(path: &str) -> Option<String> {
    fs::read_to_string(path).ok()
}

fn read_trim(path: &str) -> Option<String> {
    read_text(path).map(|text| text.trim().to_string()).filter(|text| !text.is_empty())
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
        let health = ["energy_full", "charge_full"]
            .iter()
            .find_map(|full| {
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
                if design_value > 0 {
                    Some(((full_value * 100) / design_value) as u8)
                } else {
                    None
                }
            });
        return Some((capacity, status, charging, health));
    }
    None
}

fn collect_data() -> FetchData {
    let hostname = read_trim("/proc/sys/kernel/hostname").unwrap_or_else(|| "localhost".to_string());
    let user = env::var("USER")
        .or_else(|_| env::var("LOGNAME"))
        .unwrap_or_else(|_| "user".to_string());

    let os = read_text("/etc/os-release").map(|text| parse_os_release(&text)).unwrap_or_else(|| "Linux".to_string());
    let kernel = read_trim("/proc/sys/kernel/osrelease").unwrap_or_else(|| "unknown".to_string());
    let uptime = read_text("/proc/uptime")
        .and_then(|text| text.split_whitespace().next().and_then(|v| v.parse::<f64>().ok()))
        .map(|seconds| fmt_uptime(seconds as u64))
        .unwrap_or_else(|| "unknown".to_string());

    let host = read_trim("/sys/class/dmi/id/product_name").unwrap_or_else(|| hostname.clone());

    let shell = env::var("SHELL")
        .ok()
        .and_then(|value| Path::new(&value).file_name().map(|name| name.to_string_lossy().to_string()))
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
            let total_gib = total as f64 / 1024.0 / 1024.0;
            let used_gib = used as f64 / 1024.0 / 1024.0;
            let percent = if total > 0 { (used * 100 / total) as u8 } else { 0 };
            (format!("{:.1} / {:.1} GiB ({}%)", used_gib, total_gib, percent), percent)
        }
        None => ("unknown".to_string(), 0),
    };

    let (swap, swap_pct) = match meminfo.as_deref() {
        Some(text) => match (meminfo_value(text, "SwapTotal:"), meminfo_value(text, "SwapFree:")) {
            (Some(total), Some(free)) if total > 0 => {
                let used = total.saturating_sub(free);
                let percent = (used * 100 / total) as u8;
                (
                    Some(format!(
                        "{:.1} / {:.1} GiB ({}%)",
                        used as f64 / 1024.0 / 1024.0,
                        total as f64 / 1024.0 / 1024.0,
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
        Some((format!("{} / {} ({}%)", fmt_size(used), fmt_size(total), percent), percent))
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
        Some((capacity, status, charging, health)) => {
            (Some(format!("{}% [{}]", capacity, status)), capacity, charging, health)
        }
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
        || env::var("TERM").map(|value| value.contains("kitty")).unwrap_or(false)
        || env::var("TERM_PROGRAM").map(|value| value == "kitty").unwrap_or(false)
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
        if let Some(path) = album_pick(&album, rng) {
            return Some(path);
        }
    }
    let path = xdg("XDG_DATA_HOME", ".local/share").join("tachyon/logo.png");
    png_size(&path).map(|_| path)
}

fn section(glyph: char, title: &str, rows: &[Row], width: usize, pal: &Palette) -> Vec<Line> {
    if rows.is_empty() {
        return Vec::new();
    }
    let keyw = rows.iter().map(|row| width_of(row.key)).max().unwrap_or(4);
    let prefix = format!("{} {} ", glyph, title);
    let prefix_width = width_of(&prefix);
    let mut head = Line::new();
    head.push(&prefix, Some(pal.accent2), true);
    if width > prefix_width {
        head.push(&"─".repeat(width - prefix_width), Some(pal.dim), false);
    }
    let mut lines = vec![head];
    for row in rows {
        let mut line = Line::new();
        let key_padded = format!("{:<keyw$}", row.key, keyw = keyw);
        line.push(&key_padded, Some(pal.dim), false);
        line.push("  ", None, false);
        let available = width.saturating_sub(keyw + 2);
        line.push(&truncate(&row.value, available), row.tone.or(Some(pal.fg)), false);
        lines.push(line);
    }
    lines
}

fn hcat(left: &[Line], right: &[Line], left_width: usize, gap: usize) -> Vec<Line> {
    let count = left.len().max(right.len());
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let mut text = String::new();
        let mut width = 0;
        if let Some(line) = left.get(i) {
            text.push_str(&line.text);
            width = line.width;
        }
        if left_width > width {
            text.push_str(&" ".repeat(left_width - width));
        }
        width = left_width + gap;
        text.push_str(&" ".repeat(gap));
        if let Some(line) = right.get(i) {
            text.push_str(&line.text);
            width += line.width;
        }
        out.push(Line { text, width });
    }
    out
}

fn build_sections(data: &FetchData, pal: &Palette, enabled: Option<&[String]>) -> Sections {
    let on = |id: &str| enabled.map_or(true, |list| list.iter().any(|item| item == id));
    let mut sections = Sections::default();

    if on("os") {
        sections.system.push(Row { id: "os", key: "OS", value: data.os.clone(), tone: None });
    }
    if on("host") {
        sections.system.push(Row { id: "host", key: "Host", value: data.host.clone(), tone: None });
    }
    if on("kernel") {
        sections.system.push(Row { id: "kernel", key: "Kernel", value: data.kernel.clone(), tone: None });
    }
    if on("uptime") {
        sections.system.push(Row { id: "uptime", key: "Uptime", value: data.uptime.clone(), tone: None });
    }
    if on("session") {
        sections.system.push(Row { id: "session", key: "Session", value: data.session.clone(), tone: None });
    }
    if on("shell") {
        sections.system.push(Row { id: "shell", key: "Shell", value: data.shell.clone(), tone: None });
    }
    if on("terminal") {
        sections.system.push(Row { id: "terminal", key: "Terminal", value: data.terminal.clone(), tone: None });
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
            let first = load.split_whitespace().next().and_then(|v| v.parse::<f64>().ok());
            let tone = first
                .filter(|value| *value > data.cores as f64 * 0.9)
                .map(|_| pal.warn);
            sections.system.push(Row { id: "load", key: "Load", value: load.clone(), tone });
        }
    }

    if on("cpu") && !data.cpu.is_empty() {
        sections.core.push(Row { id: "cpu", key: "CPU", value: data.cpu.clone(), tone: None });
    }
    if on("gpu") {
        if let Some(gpu) = &data.gpu {
            sections.core.push(Row { id: "gpu", key: "GPU", value: gpu.clone(), tone: None });
        }
    }
    if on("gpu_util") {
        if let Some(utilization) = data.gpu_util {
            sections.core.push(Row {
                id: "gpu_util",
                key: "GPU util",
                value: format!("{}%", utilization),
                tone: if utilization >= 95 { Some(pal.warn) } else { None },
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
        let tone = if data.memory_pct >= 85 { Some(pal.warn) } else { None };
        sections.core.push(Row { id: "memory", key: "Memory", value: data.memory.clone(), tone });
    }
    if on("swap") {
        if let Some(swap) = &data.swap {
            let tone = if data.swap_pct >= 50 { Some(pal.warn) } else { None };
            sections.core.push(Row { id: "swap", key: "Swap", value: swap.clone(), tone });
        }
    }
    let disk_tone = |pct: u8| if pct >= 90 { Some(pal.warn) } else { None };
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
            sections.core.push(Row { id: "ip", key: "IP", value: ip.clone(), tone: None });
        }
    }
    if on("link") {
        if let Some(link) = &data.link {
            sections.core.push(Row { id: "link", key: "Link", value: link.clone(), tone: None });
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
            } else if data.battery_pct <= 15 {
                Some(pal.err)
            } else {
                None
            };
            sections.power.push(Row { id: "battery", key: "Battery", value: battery.clone(), tone });
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
            let tone = if temp >= 95 {
                Some(pal.err)
            } else if temp >= 85 {
                Some(pal.warn)
            } else {
                None
            };
            sections.power.push(Row {
                id: "cpu_temp",
                key: "CPU temp",
                value: format!("{} °C", temp),
                tone,
            });
        }
    }
    if on("gpu_temp") {
        if let Some(temp) = data.gpu_temp {
            let tone = if temp >= 95 {
                Some(pal.err)
            } else if temp >= 85 {
                Some(pal.warn)
            } else {
                None
            };
            sections.power.push(Row {
                id: "gpu_temp",
                key: "GPU temp",
                value: format!("{} °C", temp),
                tone,
            });
        }
    }

    sections
}

fn render(
    theme: &Theme,
    data: &FetchData,
    quote: &str,
    width: usize,
    glyph: char,
    title_variant: bool,
    demo: bool,
    enabled: Option<&[String]>,
) -> Vec<Line> {
    let pal = &theme.pal;
    let mut lines = Vec::new();

    let mut header = Line::new();
    header.push("TACHYON", Some(pal.accent), true);
    header.push(" // ", Some(pal.dim), false);
    header.push(if title_variant { "SYSTEMS" } else { "FLIGHT DECK" }, Some(pal.fg), true);
    if demo {
        header.push(" · SIM", Some(pal.warn), false);
    }
    let right = format!("{} · {}", theme.name.to_uppercase(), theme.mode.to_uppercase());
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
    rule.push(&"─".repeat(width), Some(pal.dim), false);
    lines.push(rule);

    let sections = build_sections(data, pal, enabled);
    let two_columns = theme.layout != "stack"
        && width >= 120
        && !sections.core.is_empty()
        && !(sections.system.is_empty() && sections.power.is_empty());

    if two_columns {
        let left_width = (width - 2) / 2;
        let right_width = width - 2 - left_width;
        let mut left = section(glyph, Section::System.title(), &sections.system, left_width, pal);
        if !sections.power.is_empty() {
            if !left.is_empty() {
                left.push(Line::new());
            }
            left.extend(section(glyph, Section::Power.title(), &sections.power, left_width, pal));
        }
        let right = section(glyph, Section::Core.title(), &sections.core, right_width, pal);
        lines.extend(hcat(&left, &right, left_width, 2));
    } else {
        for (section_kind, rows) in [
            (Section::System, &sections.system),
            (Section::Core, &sections.core),
            (Section::Power, &sections.power),
        ] {
            if !rows.is_empty() {
                lines.extend(section(glyph, section_kind.title(), rows, width, pal));
            }
        }
    }

    if !quote.is_empty() {
        lines.push(Line::new());
        let mut note = Line::new();
        note.push("NOTE", Some(pal.accent2), true);
        note.push("  ", None, false);
        note.push(&truncate(quote, width.saturating_sub(6)), Some(pal.dim), false);
        lines.push(note);
    }

    lines
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
            "--tui" => opts.tui = true,
            "--layout" => opts.layout = args.next(),
            "--glyph" => opts.glyph = args.next(),
            "--album" => opts.album = args.next(),
            "--theme" => opts.theme = args.next(),
            "--variant" => opts.variant = args.next(),
            "--seed" => opts.seed = args.next().and_then(|value| value.parse().ok()),
            "--logo" => opts.logo = args.next().map(PathBuf::from),
            "--palettes" => opts.palettes = args.next().map(PathBuf::from),
            "--quotes" => opts.quotes = args.next().map(PathBuf::from),
            "--config" => opts.config = args.next().map(PathBuf::from),
            "--state" => opts.state = args.next().map(PathBuf::from),
            "--convert" => {
                opts.convert_source = args.next().map(PathBuf::from);
                opts.convert_destination = args.next().map(PathBuf::from);
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
         --no-logo         do not draw the kitty image logo\n\
         --layout <name>   hud or stack\n\
         --glyph <char>    section glyph (or random)\n\
         --album <name>    pick the artwork from an album\n\
         --theme <name>    override the active palette\n\
         --variant <mode>  dark or light\n\
         --seed <n>        deterministic layout/quote selection\n\
         --logo <path>     PNG for the kitty graphics protocol\n\
         --palettes <path> palette table (default ~/.local/share/tachyon/palettes.tsv)\n\
         --quotes <path>   quote pool (default ~/.config/tachyon/quotes.txt)\n\
         --config <path>   module/album config (default ~/.config/tachyon/fetch.conf)\n\
         --state <path>    theme state (default ~/.local/state/tachyon/theme)\n\
         --list-themes     print the palette table\n\
         --convert <src> <dst>  convert an image to PNG (used for album ingest)\n\
         -h, --help        show this help\n\
         -V, --version     show the version",
        VERSION
    );
}

fn list_themes(opts: &Opts) {
    if let Ok(text) = fs::read_to_string(palettes_path(opts)) {
        let mut seen: Vec<String> = Vec::new();
        for line in text.lines() {
            if let Some((name, _, _, _, _)) = parse_palette_row(line) {
                if !seen.contains(&name) {
                    seen.push(name);
                }
            }
        }
        seen.sort();
        for name in seen {
            println!("{}", name);
        }
    } else {
        println!("Solitude");
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
    fn enable(file: &fs::File) -> Option<Self> {
        let fd = file.as_raw_fd();
        unsafe {
            let mut current: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(fd, &mut current) != 0 {
                return None;
            }
            let original = current;
            let mut raw = current;
            libc::cfmakeraw(&mut raw);
            if libc::tcsetattr(fd, libc::TCSANOW, &raw) != 0 {
                return None;
            }
            Some(RawTty { fd, original })
        }
    }
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

fn read_key(tty: &mut fs::File) -> Option<Key> {
    let mut byte = [0u8; 1];
    tty.read_exact(&mut byte).ok()?;
    match byte[0] {
        0x1b => {
            let mut sequence = [0u8; 2];
            if tty.read_exact(&mut sequence).is_ok() {
                match sequence {
                    [0x5b, 0x41] => Some(Key::Up),
                    [0x5b, 0x42] => Some(Key::Down),
                    _ => Some(Key::Escape),
                }
            } else {
                Some(Key::Escape)
            }
        }
        0x0a | 0x0d => Some(Key::Enter),
        0x7f | 0x08 => Some(Key::Backspace),
        value if (32..127).contains(&value) => Some(Key::Char(value as char)),
        _ => Some(Key::Other),
    }
}

fn prompt_line(tty: &mut fs::File, label: &str) -> Option<String> {
    print!("\x1b[H\x1b[2J{}", label);
    let _ = io::stdout().flush();
    let mut value = String::new();
    loop {
        match read_key(tty)? {
            Key::Enter => return Some(value),
            Key::Escape => return None,
            Key::Backspace => {
                if value.pop().is_some() {
                    print!("\x08 \x08");
                    let _ = io::stdout().flush();
                }
            }
            Key::Char(c) => {
                value.push(c);
                print!("{}", c);
                let _ = io::stdout().flush();
            }
            _ => {}
        }
    }
}

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
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect();
    let stem = if stem.is_empty() { "image".to_string() } else { stem };
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
    let picture = image::load_from_memory_with_format(&bytes, format).map_err(|error| error.to_string())?;
    let picture = if picture.width() > 1600 {
        picture.resize(1600, u32::MAX, image::imageops::FilterType::Lanczos3)
    } else {
        picture
    };
    picture
        .save_with_format(destination, image::ImageFormat::Png)
        .map_err(|error| error.to_string())
}

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
    fn new(config: &Config, _theme: &Theme) -> Self {
        let enabled = MODULES
            .iter()
            .map(|(id, _)| {
                config
                    .enabled
                    .as_ref()
                    .map(|list| list.iter().any(|item| item == id))
                    .unwrap_or(true)
            })
            .collect();
        let glyph = config
            .glyph
            .as_deref()
            .and_then(|value| GLYPHS.iter().position(|item| *item == value))
            .unwrap_or(0);
        let layout = config
            .layout
            .as_deref()
            .and_then(|value| LAYOUTS.iter().position(|item| *item == value))
            .unwrap_or(0);
        let motion = config
            .motion
            .as_deref()
            .and_then(|value| MOTIONS.iter().position(|item| *item == value))
            .unwrap_or(0);
        let albums = list_albums();
        let album = config
            .album
            .as_ref()
            .and_then(|name| albums.iter().position(|item| item == name))
            .map(|index| index + 1)
            .unwrap_or(0);
        Self {
            enabled,
            cursor: 0,
            glyph,
            layout,
            motion,
            album,
            albums,
            status: "space toggle · s save · q quit".to_string(),
        }
    }
}

fn save_tui(path: &Path, state: &TuiState) -> io::Result<()> {
    let ids: Vec<&str> = MODULES
        .iter()
        .zip(&state.enabled)
        .filter(|(_, enabled)| **enabled)
        .map(|((id, _), _)| *id)
        .collect();
    let mut text = String::new();
    if ids.is_empty() {
        text.push_str("enabled=none\n");
    } else if ids.len() == MODULES.len() {
        text.push_str("enabled=all\n");
    } else {
        text.push_str(&format!("enabled={}\n", ids.join(",")));
    }
    text.push_str(&format!("glyph={}\n", GLYPHS[state.glyph]));
    if state.layout > 0 {
        text.push_str(&format!("layout={}\n", LAYOUTS[state.layout]));
    }
    if state.motion > 0 {
        text.push_str(&format!("motion={}\n", MOTIONS[state.motion]));
    }
    if state.album > 0 {
        text.push_str(&format!("album={}\n", state.albums[state.album - 1]));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

fn draw_tui(state: &TuiState, theme: &Theme, path: &Path, width: usize, height: usize) {
    let pal = &theme.pal;
    let left_width = 42.min(width.saturating_sub(30));
    let mut output = String::from("\x1b[H\x1b[2J");

    let mut header = Line::new();
    header.push("TACHYON // CONFIG", Some(pal.accent), true);
    let right = format!("{} · {}", theme.name.to_uppercase(), theme.mode.to_uppercase());
    if header.width + right.chars().count() + 2 < width {
        header.pad(width - header.width - right.chars().count());
        header.push(&right, Some(pal.dim), false);
    }
    output.push_str(&header.text);
    output.push_str("\r\n");

    let mut rule = Line::new();
    rule.push(&"─".repeat(width), Some(pal.dim), false);
    output.push_str(&rule.text);
    output.push_str("\r\n");

    let visible = height.saturating_sub(10).max(6);
    let start = state.cursor.saturating_sub(visible.saturating_sub(1));
    let end = (start + visible).min(MODULES.len());
    let enabled_count = state.enabled.iter().filter(|value| **value).count();

    let layout_label = if state.layout == 0 {
        format!("theme ({})", theme.layout)
    } else {
        LAYOUTS[state.layout].to_string()
    };
    let motion_label = if state.motion == 0 {
        format!("theme ({})", theme.motion)
    } else {
        MOTIONS[state.motion].to_string()
    };
    let album_label = if state.album == 0 {
        "(default)".to_string()
    } else {
        state.albums[state.album - 1].clone()
    };
    let settings = [
        ("GLYPH", GLYPHS[state.glyph].to_string(), "g"),
        ("LAYOUT", layout_label, "l"),
        ("MOTION", motion_label, "m"),
        ("ALBUM", album_label, "b"),
        ("ADD", "photo to album".to_string(), "a"),
        ("SAVE", path.display().to_string(), "s"),
        ("QUIT", "".to_string(), "q"),
    ];

    let mut list_header = Line::new();
    list_header.push(&format!("MODULES {}/{}", enabled_count, MODULES.len()), Some(pal.accent2), true);
    list_header.pad(left_width.saturating_sub(list_header.width));
    list_header.push("   SETTINGS", Some(pal.accent2), true);
    output.push_str(&list_header.text);
    output.push_str("\r\n");

    let rows = (end - start).max(settings.len());
    for index in 0..rows {
        let mut line = Line::new();
        if let Some(module_index) = (start + index < end).then_some(start + index) {
            let (_, label) = MODULES[module_index];
            let enabled = state.enabled[module_index];
            let selected = module_index == state.cursor;
            line.push(if selected { "▸ " } else { "  " }, Some(pal.accent), true);
            line.push(if enabled { "[x] " } else { "[ ] " }, Some(if enabled { pal.accent } else { pal.dim }), true);
            line.push(label, Some(if enabled { pal.fg } else { pal.dim }), false);
        }
        line.pad(left_width.saturating_sub(line.width));
        line.push("   ", None, false);
        if let Some((key, value, hint)) = settings.get(index) {
            line.push(key, Some(pal.dim), false);
            line.push("  ", None, false);
            line.push(&truncate(value, width.saturating_sub(left_width + 6)), Some(pal.fg), false);
            if !hint.is_empty() {
                line.push("  ", None, false);
                line.push(hint, Some(pal.accent2), false);
            }
        }
        output.push_str(&line.text);
        output.push_str("\x1b[K\r\n");
    }

    output.push_str("\r\n");
    let mut help = Line::new();
    help.push(
        "space toggle · j/k move · g glyph · l layout · m motion · b album · a add photo · s save · q quit",
        Some(pal.dim),
        false,
    );
    output.push_str(&truncate(&strip_ansi(&help.text), width));
    output.push_str("\x1b[K\r\n");

    let mut status = Line::new();
    if !state.status.is_empty() {
        status.push(&state.status, Some(pal.accent2), false);
    }
    output.push_str(&truncate(&strip_ansi(&status.text), width));
    output.push_str("\x1b[K\r\n");

    print!("{}", output);
    let _ = io::stdout().flush();
}

fn run_tui(opts: &Opts) {
    let mut tty = match fs::File::open("/dev/tty") {
        Ok(file) => file,
        Err(error) => {
            eprintln!("tachyon: cannot open /dev/tty: {}", error);
            return;
        }
    };
    let _raw = match RawTty::enable(&tty) {
        Some(raw) => raw,
        None => {
            eprintln!("tachyon: cannot enter raw mode");
            return;
        }
    };
    print!("\x1b[?1049h\x1b[?25l");
    let _ = io::stdout().flush();
    let _screen = ScreenGuard;

    let config_path = config_path(opts);
    let config = load_config(&config_path);
    let theme = load_theme(opts);
    let mut state = TuiState::new(&config, &theme);

    loop {
        let (width, height, _, _) = term_size();
        draw_tui(&state, &theme, &config_path, width, height);
        let key = match read_key(&mut tty) {
            Some(key) => key,
            None => break,
        };
        match key {
            Key::Char('q') => break,
            Key::Char('j') | Key::Down => {
                if !MODULES.is_empty() {
                    state.cursor = (state.cursor + 1) % MODULES.len();
                }
            }
            Key::Char('k') | Key::Up => {
                if !MODULES.is_empty() {
                    state.cursor = (state.cursor + MODULES.len() - 1) % MODULES.len();
                }
            }
            Key::Char(' ') => {
                if let Some(value) = state.enabled.get_mut(state.cursor) {
                    *value = !*value;
                }
            }
            Key::Char('g') => state.glyph = (state.glyph + 1) % GLYPHS.len(),
            Key::Char('l') => state.layout = (state.layout + 1) % LAYOUTS.len(),
            Key::Char('m') => state.motion = (state.motion + 1) % MOTIONS.len(),
            Key::Char('b') => state.album = (state.album + 1) % (state.albums.len() + 1),
            Key::Char('a') => {
                let album = if state.album == 0 {
                    "default".to_string()
                } else {
                    state.albums[state.album - 1].clone()
                };
                if let Some(source) = prompt_line(&mut tty, "photo path: ") {
                    match add_photo(&album, Path::new(&source)) {
                        Ok(destination) => {
                            state.status = format!("added {}", destination.display());
                            state.albums = list_albums();
                            state.album = state
                                .albums
                                .iter()
                                .position(|item| item == &album)
                                .map(|index| index + 1)
                                .unwrap_or(0);
                        }
                        Err(error) => state.status = error,
                    }
                } else {
                    state.status = "cancelled".to_string();
                }
            }
            Key::Char('s') => match save_tui(&config_path, &state) {
                Ok(()) => state.status = format!("saved {}", config_path.display()),
                Err(error) => state.status = format!("save failed: {}", error),
            },
            _ => {}
        }
    }
}

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
            let cols_for_logo = ((cols as f64) * 0.34).round() as usize;
            let cols_for_logo = cols_for_logo.clamp(26, 46);
            let indent = cols_for_logo + 3;
            if cols <= indent + 56 {
                return None;
            }
            let cell_ratio = if xpixel > 0 && ypixel > 0 {
                (xpixel as f64 / cols as f64) / (ypixel as f64 / term_size().1 as f64)
            } else {
                0.5
            };
            let rows = ((cols_for_logo as f64) * cell_ratio * (img_h as f64 / img_w as f64)).ceil() as usize;
            Some((path, cols_for_logo, rows.max(3)))
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
