use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub type Rgb = (u8, u8, u8);

pub const MODULE_IDS: &[&str] = &[
    "os",
    "host",
    "kernel",
    "uptime",
    "session",
    "shell",
    "terminal",
    "compositor",
    "load",
    "cpu",
    "gpu",
    "gpu_util",
    "gpu_vram",
    "memory",
    "swap",
    "disk_root",
    "disk_media",
    "disk_code",
    "ip",
    "link",
    "processes",
    "battery",
    "battery_health",
    "cpu_temp",
    "gpu_temp",
];

pub const DEFAULT_POOL: &str = "▸▚◆▰⬢⬡■●";

pub const MODULE_GLYPHS: &[(&str, char)] = &[
    ("os", '◈'),
    ("host", '▲'),
    ("kernel", '◆'),
    ("uptime", '◷'),
    ("session", '◇'),
    ("shell", '❯'),
    ("terminal", '▤'),
    ("compositor", '▦'),
    ("load", '∿'),
    ("cpu", '▣'),
    ("gpu", '▧'),
    ("gpu_util", '◭'),
    ("gpu_vram", '▥'),
    ("memory", '●'),
    ("swap", '⇄'),
    ("disk_root", '▩'),
    ("disk_media", '◰'),
    ("disk_code", '◱'),
    ("ip", '⊙'),
    ("link", '⇉'),
    ("processes", '≡'),
    ("battery", '▮'),
    ("battery_health", '♡'),
    ("cpu_temp", '△'),
    ("gpu_temp", '▽'),
];

pub struct BuiltinTheme {
    pub name: &'static str,
    pub variant: &'static str,
    pub colors: [Rgb; 8],
    pub layout: &'static str,
    pub motion: &'static str,
}

pub const BUILTIN_THEMES: &[BuiltinTheme] = &[
    BuiltinTheme {
        name: "Solitude",
        variant: "dark",
        colors: [
            (0x10, 0x13, 0x15),
            (0xca, 0xcc, 0xcc),
            (0x4b, 0x4e, 0x55),
            (0x79, 0x81, 0x86),
            (0x70, 0x70, 0x70),
            (0x9f, 0xa5, 0xa9),
            (0xd9, 0xdb, 0xdc),
            (0x56, 0x5d, 0x60),
        ],
        layout: "hud",
        motion: "reveal",
    },
    BuiltinTheme {
        name: "Solitude",
        variant: "light",
        colors: [
            (0x10, 0x13, 0x15),
            (0xca, 0xcc, 0xcc),
            (0x4b, 0x4e, 0x55),
            (0x79, 0x81, 0x86),
            (0x70, 0x70, 0x70),
            (0x9f, 0xa5, 0xa9),
            (0xd9, 0xdb, 0xdc),
            (0x56, 0x5d, 0x60),
        ],
        layout: "hud",
        motion: "reveal",
    },
    BuiltinTheme {
        name: "Vantablack",
        variant: "dark",
        colors: [
            (0x00, 0x00, 0x00),
            (0xff, 0xff, 0xff),
            (0x7a, 0x7a, 0x7a),
            (0x8d, 0x8d, 0x8d),
            (0xb0, 0xb0, 0xb0),
            (0xb6, 0xb6, 0xb6),
            (0xce, 0xce, 0xce),
            (0xa4, 0xa4, 0xa4),
        ],
        layout: "hud",
        motion: "off",
    },
    BuiltinTheme {
        name: "Vantablack",
        variant: "light",
        colors: [
            (0x00, 0x00, 0x00),
            (0xff, 0xff, 0xff),
            (0x7a, 0x7a, 0x7a),
            (0x8d, 0x8d, 0x8d),
            (0xb0, 0xb0, 0xb0),
            (0xb6, 0xb6, 0xb6),
            (0xce, 0xce, 0xce),
            (0xa4, 0xa4, 0xa4),
        ],
        layout: "hud",
        motion: "off",
    },
    BuiltinTheme {
        name: "Hackerman",
        variant: "dark",
        colors: [
            (0x0b, 0x0c, 0x16),
            (0xdd, 0xf7, 0xff),
            (0x6a, 0x6e, 0x95),
            (0x82, 0xfb, 0x9c),
            (0x7c, 0xf8, 0xf7),
            (0x4f, 0xe8, 0x8f),
            (0x50, 0xf7, 0xd4),
            (0x50, 0xf8, 0x72),
        ],
        layout: "hud",
        motion: "reveal",
    },
    BuiltinTheme {
        name: "Hackerman",
        variant: "light",
        colors: [
            (0x0b, 0x0c, 0x16),
            (0xdd, 0xf7, 0xff),
            (0x6a, 0x6e, 0x95),
            (0x82, 0xfb, 0x9c),
            (0x7c, 0xf8, 0xf7),
            (0x4f, 0xe8, 0x8f),
            (0x50, 0xf7, 0xd4),
            (0x50, 0xf8, 0x72),
        ],
        layout: "hud",
        motion: "reveal",
    },
    BuiltinTheme {
        name: "Retro 82",
        variant: "dark",
        colors: [
            (0x05, 0x18, 0x2e),
            (0xf6, 0xdc, 0xac),
            (0x3f, 0x8f, 0x8a),
            (0xfa, 0xa9, 0x68),
            (0x8c, 0xbf, 0xb8),
            (0x02, 0x83, 0x91),
            (0xe9, 0x7b, 0x3c),
            (0xf8, 0x55, 0x25),
        ],
        layout: "hud",
        motion: "reveal",
    },
    BuiltinTheme {
        name: "Retro 82",
        variant: "light",
        colors: [
            (0x05, 0x18, 0x2e),
            (0xf6, 0xdc, 0xac),
            (0x3f, 0x8f, 0x8a),
            (0xfa, 0xa9, 0x68),
            (0x8c, 0xbf, 0xb8),
            (0x02, 0x83, 0x91),
            (0xe9, 0x7b, 0x3c),
            (0xf8, 0x55, 0x25),
        ],
        layout: "hud",
        motion: "reveal",
    },
    BuiltinTheme {
        name: "Rose Pine",
        variant: "dark",
        colors: [
            (0x19, 0x17, 0x24),
            (0xe0, 0xde, 0xf4),
            (0x90, 0x8c, 0xaa),
            (0xc4, 0xa7, 0xe7),
            (0xeb, 0xbc, 0xba),
            (0x31, 0x74, 0x8f),
            (0xf6, 0xc1, 0x77),
            (0xeb, 0x6f, 0x92),
        ],
        layout: "hud",
        motion: "reveal",
    },
    BuiltinTheme {
        name: "Rose Pine",
        variant: "light",
        colors: [
            (0xfa, 0xf4, 0xed),
            (0x57, 0x52, 0x79),
            (0x98, 0x93, 0xa5),
            (0x56, 0x94, 0x9f),
            (0xd7, 0x82, 0x7e),
            (0x28, 0x69, 0x83),
            (0xea, 0x9d, 0x34),
            (0xb4, 0x63, 0x7a),
        ],
        layout: "hud",
        motion: "reveal",
    },
    BuiltinTheme {
        name: "Flexoki Light",
        variant: "dark",
        colors: [
            (0x1c, 0x1b, 0x1a),
            (0xce, 0xcd, 0xc3),
            (0x87, 0x85, 0x80),
            (0x43, 0x85, 0xbe),
            (0x3a, 0xa9, 0x9f),
            (0x87, 0x9a, 0x39),
            (0xd0, 0xa2, 0x15),
            (0xd1, 0x4d, 0x41),
        ],
        layout: "stack",
        motion: "off",
    },
    BuiltinTheme {
        name: "Flexoki Light",
        variant: "light",
        colors: [
            (0xff, 0xfc, 0xf0),
            (0x10, 0x0f, 0x0f),
            (0x6f, 0x6e, 0x69),
            (0x20, 0x5e, 0xa6),
            (0x24, 0x83, 0x7b),
            (0x66, 0x80, 0x0b),
            (0xad, 0x83, 0x01),
            (0xaf, 0x30, 0x29),
        ],
        layout: "stack",
        motion: "off",
    },
];

#[derive(Default, Clone, PartialEq, Debug)]
pub struct Config {
    pub enabled: Option<Vec<String>>,
    pub quote: Option<String>,
    pub note_mode: Option<String>,
    pub note_text: Option<String>,
    pub emoticon: Option<String>,
    pub glyph: Option<String>,
    pub album: Option<String>,
    pub image: Option<PathBuf>,
    pub layout: Option<String>,
    pub motion: Option<String>,
    pub theme: Option<String>,
    pub variant: Option<String>,
    pub rule: Option<String>,
    pub units: Option<String>,
    pub temp_unit: Option<String>,
    pub memory_warn: Option<u8>,
    pub swap_warn: Option<u8>,
    pub disk_warn: Option<u8>,
    pub battery_low: Option<u8>,
    pub temp_warn: Option<u8>,
    pub temp_crit: Option<u8>,
    pub gpu_warn: Option<u8>,
    pub logo_mode: Option<String>,
    pub logo_text: Option<String>,
    pub logo_font: Option<String>,
    pub logo_size: Option<u8>,
    pub logo_tint: Option<String>,
    pub logo_width: Option<u8>,
    pub logo_min: Option<u8>,
    pub logo_max: Option<u8>,
    pub logo_gap: Option<u8>,
    pub glyph_system: Option<String>,
    pub glyph_core: Option<String>,
    pub glyph_power: Option<String>,
    pub glyph_row: Option<String>,
    pub glyph_pool: Option<String>,
    pub glyphs: BTreeMap<String, String>,
    pub rotate_art: Option<String>,
    pub rotate_palette: Option<String>,
    pub rotate_every: Option<String>,
    pub rotate_themes: Option<Vec<String>>,
    pub color_bg: Option<Rgb>,
    pub color_fg: Option<Rgb>,
    pub color_dim: Option<Rgb>,
    pub color_accent: Option<Rgb>,
    pub color_accent2: Option<Rgb>,
    pub color_ok: Option<Rgb>,
    pub color_warn: Option<Rgb>,
    pub color_err: Option<Rgb>,
}

impl Config {
    pub fn units_si(&self) -> bool {
        self.units.as_deref() == Some("si")
    }

    pub fn fahrenheit(&self) -> bool {
        self.temp_unit.as_deref() == Some("f")
    }

    pub fn rule_char(&self) -> char {
        self.rule
            .as_deref()
            .and_then(|value| value.chars().next())
            .unwrap_or('─')
    }

    pub fn logo_mode(&self) -> &str {
        self.logo_mode.as_deref().unwrap_or("image")
    }

    pub fn note_mode(&self) -> &str {
        if let Some(mode) = self.note_mode.as_deref() {
            return mode;
        }
        match self.quote.as_deref() {
            None | Some("random") => "quote",
            Some("off") | Some("none") => "off",
            Some(line) if line.contains(" — ") => "quote",
            Some(_) => "text",
        }
    }

    pub fn logo_size_px(&self) -> u8 {
        self.logo_size.unwrap_or(96)
    }

    pub fn logo_tint(&self) -> &str {
        self.logo_tint.as_deref().unwrap_or("duo")
    }

    pub fn glyph_row_mode(&self) -> &str {
        self.glyph_row.as_deref().unwrap_or("auto")
    }

    pub fn glyph_pool(&self) -> &str {
        self.glyph_pool.as_deref().unwrap_or(DEFAULT_POOL)
    }

    pub fn rotate_art_mode(&self) -> &str {
        self.rotate_art.as_deref().unwrap_or("shuffle")
    }

    pub fn rotate_palette_mode(&self) -> &str {
        self.rotate_palette.as_deref().unwrap_or("off")
    }

    pub fn rotate_every_mode(&self) -> &str {
        self.rotate_every.as_deref().unwrap_or("run")
    }

    pub fn apply_colors(&self, pal: &mut Palette) {
        let setters: [(&Option<Rgb>, &mut Rgb); 8] = [
            (&self.color_bg, &mut pal.bg),
            (&self.color_fg, &mut pal.fg),
            (&self.color_dim, &mut pal.dim),
            (&self.color_accent, &mut pal.accent),
            (&self.color_accent2, &mut pal.accent2),
            (&self.color_ok, &mut pal.ok),
            (&self.color_warn, &mut pal.warn),
            (&self.color_err, &mut pal.err),
        ];
        for (value, slot) in setters {
            if let Some(color) = value {
                *slot = *color;
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Palette {
    pub bg: Rgb,
    pub fg: Rgb,
    pub dim: Rgb,
    pub accent: Rgb,
    pub accent2: Rgb,
    pub ok: Rgb,
    pub warn: Rgb,
    pub err: Rgb,
}

pub fn parse_color(text: &str) -> Option<Rgb> {
    let hex = text.trim().strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

pub fn hex(color: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", color.0, color.1, color.2)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Place {
    Flat,
    Glyph,
    Rotate,
}

fn place_of(key: &str) -> Option<(Place, &str)> {
    const FLAT: &[&str] = &[
        "enabled",
        "quote",
        "note_mode",
        "note_text",
        "emoticon",
        "glyph",
        "album",
        "image",
        "layout",
        "motion",
        "theme",
        "variant",
        "rule",
        "units",
        "temp",
        "memory_warn",
        "swap_warn",
        "disk_warn",
        "battery_low",
        "temp_warn",
        "temp_crit",
        "gpu_warn",
        "logo_mode",
        "logo_text",
        "logo_font",
        "logo_size",
        "logo_tint",
        "logo_width",
        "logo_min",
        "logo_max",
        "logo_gap",
        "color.bg",
        "color.fg",
        "color.dim",
        "color.accent",
        "color.accent2",
        "color.ok",
        "color.warn",
        "color.err",
    ];
    if FLAT.contains(&key) {
        return Some((Place::Flat, key));
    }
    if let Some(rest) = key.strip_prefix("glyph.") {
        if rest == "header" {
            return Some((Place::Glyph, "header"));
        }
        if matches!(rest, "system" | "core" | "power" | "row" | "pool") {
            return Some((Place::Glyph, rest));
        }
        if MODULE_IDS.contains(&rest) {
            return Some((Place::Glyph, rest));
        }
    }
    if let Some(rest) = key.strip_prefix("rotate.") {
        if matches!(rest, "art" | "palette" | "every" | "themes") {
            return Some((Place::Rotate, rest));
        }
    }
    None
}

fn section_key(place: Place, key: &str) -> Option<&str> {
    match place {
        Place::Flat => place_of(key).map(|(_, k)| k),
        Place::Glyph => {
            if key == "header"
                || matches!(key, "system" | "core" | "power" | "row" | "pool")
                || MODULE_IDS.contains(&key)
            {
                Some(key)
            } else {
                None
            }
        }
        Place::Rotate => {
            if matches!(key, "art" | "palette" | "every" | "themes") {
                Some(key)
            } else {
                None
            }
        }
    }
}

pub fn load_config(path: &Path) -> Config {
    let mut config = Config::default();
    if let Ok(text) = fs::read_to_string(path) {
        let mut section: Option<Place> = Some(Place::Flat);
        for raw in text.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line
                .strip_prefix('[')
                .and_then(|rest| rest.strip_suffix(']'))
            {
                section = match name.trim() {
                    "glyphs" => Some(Place::Glyph),
                    "rotate" => Some(Place::Rotate),
                    _ => None,
                };
                continue;
            }
            let Some(section) = section else {
                continue;
            };
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            let (place, key) = match place_of(key) {
                Some(found) => found,
                None => {
                    if section == Place::Flat {
                        continue;
                    }
                    (section, key)
                }
            };
            if section_key(place, key).is_none() {
                continue;
            }
            match (place, key) {
                (Place::Flat, "enabled") => {
                    config.enabled = if value == "all" || value.is_empty() {
                        None
                    } else if value == "none" {
                        Some(Vec::new())
                    } else {
                        Some(
                            value
                                .split(',')
                                .map(str::trim)
                                .filter(|item| !item.is_empty())
                                .map(str::to_string)
                                .collect(),
                        )
                    };
                }
                (Place::Flat, "quote") => {
                    config.quote = if value.is_empty() || value == "random" {
                        None
                    } else {
                        Some(value.to_string())
                    }
                }
                (Place::Flat, "note_mode") => {
                    config.note_mode = choice(value, &["quote", "text", "emoticon", "off"])
                }
                (Place::Flat, "note_text") => {
                    config.note_text = if value.is_empty() {
                        None
                    } else {
                        Some(value.to_string())
                    }
                }
                (Place::Flat, "emoticon") => {
                    config.emoticon = if value.is_empty() {
                        None
                    } else {
                        Some(value.to_string())
                    }
                }
                (Place::Flat, "glyph") if !value.is_empty() => {
                    config.glyph = Some(value.to_string())
                }
                (Place::Flat, "album") if !value.is_empty() => {
                    config.album = Some(value.to_string())
                }
                (Place::Flat, "image") if !value.is_empty() => {
                    config.image = Some(PathBuf::from(value))
                }
                (Place::Flat, "layout") => config.layout = choice(value, &["hud", "stack"]),
                (Place::Flat, "motion") => config.motion = choice(value, &["reveal", "off"]),
                (Place::Flat, "theme") if !value.is_empty() => {
                    config.theme = Some(value.to_string())
                }
                (Place::Flat, "variant") => config.variant = choice(value, &["dark", "light"]),
                (Place::Flat, "rule") if !value.is_empty() => {
                    config.rule = Some(value.chars().next().unwrap().to_string())
                }
                (Place::Flat, "units") => config.units = choice(value, &["binary", "si"]),
                (Place::Flat, "temp") => config.temp_unit = choice(value, &["c", "f"]),
                (Place::Flat, "memory_warn") => config.memory_warn = value.parse().ok(),
                (Place::Flat, "swap_warn") => config.swap_warn = value.parse().ok(),
                (Place::Flat, "disk_warn") => config.disk_warn = value.parse().ok(),
                (Place::Flat, "battery_low") => config.battery_low = value.parse().ok(),
                (Place::Flat, "temp_warn") => config.temp_warn = value.parse().ok(),
                (Place::Flat, "temp_crit") => config.temp_crit = value.parse().ok(),
                (Place::Flat, "gpu_warn") => config.gpu_warn = value.parse().ok(),
                (Place::Flat, "logo_mode") => config.logo_mode = choice(value, &["image", "text"]),
                (Place::Flat, "logo_text") if !value.is_empty() => {
                    config.logo_text = Some(value.to_string())
                }
                (Place::Flat, "logo_font") if !value.is_empty() => {
                    config.logo_font = Some(value.to_string())
                }
                (Place::Flat, "logo_size") => config.logo_size = value.parse().ok(),
                (Place::Flat, "logo_tint") => {
                    config.logo_tint = choice(value, &["duo", "accent", "accent2", "fg"])
                }
                (Place::Flat, "logo_width") => config.logo_width = value.parse().ok(),
                (Place::Flat, "logo_min") => config.logo_min = value.parse().ok(),
                (Place::Flat, "logo_max") => config.logo_max = value.parse().ok(),
                (Place::Flat, "logo_gap") => config.logo_gap = value.parse().ok(),
                (Place::Flat, "color.bg") => config.color_bg = parse_color(value),
                (Place::Flat, "color.fg") => config.color_fg = parse_color(value),
                (Place::Flat, "color.dim") => config.color_dim = parse_color(value),
                (Place::Flat, "color.accent") => config.color_accent = parse_color(value),
                (Place::Flat, "color.accent2") => config.color_accent2 = parse_color(value),
                (Place::Flat, "color.ok") => config.color_ok = parse_color(value),
                (Place::Flat, "color.warn") => config.color_warn = parse_color(value),
                (Place::Flat, "color.err") => config.color_err = parse_color(value),
                (Place::Glyph, "header") if !value.is_empty() => {
                    config.glyph = Some(value.to_string())
                }
                (Place::Glyph, "system") if !value.is_empty() => {
                    config.glyph_system = Some(value.to_string())
                }
                (Place::Glyph, "core") if !value.is_empty() => {
                    config.glyph_core = Some(value.to_string())
                }
                (Place::Glyph, "power") if !value.is_empty() => {
                    config.glyph_power = Some(value.to_string())
                }
                (Place::Glyph, "row") if !value.is_empty() => {
                    config.glyph_row = Some(value.to_string())
                }
                (Place::Glyph, "pool") if !value.is_empty() => {
                    config.glyph_pool = Some(value.to_string())
                }
                (Place::Glyph, id) if MODULE_IDS.contains(&id) && !value.is_empty() => {
                    config.glyphs.insert(id.to_string(), value.to_string());
                }
                (Place::Rotate, "art") => {
                    config.rotate_art = choice(value, &["off", "next", "shuffle"])
                }
                (Place::Rotate, "palette") => {
                    config.rotate_palette = choice(value, &["off", "next", "shuffle"])
                }
                (Place::Rotate, "every") => {
                    config.rotate_every = choice(value, &["run", "hour", "day"])
                }
                (Place::Rotate, "themes") => {
                    config.rotate_themes = Some(
                        value
                            .split(',')
                            .map(str::trim)
                            .filter(|item| !item.is_empty())
                            .map(str::to_string)
                            .collect(),
                    );
                }
                _ => {}
            }
        }
    }
    config
}

fn choice(value: &str, allowed: &[&str]) -> Option<String> {
    if allowed.contains(&value) {
        Some(value.to_string())
    } else {
        None
    }
}

fn flat_lines(config: &Config) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let mut push = |key: &str, value: Option<String>| {
        if let Some(value) = value {
            out.push((key.to_string(), format!("{} = {}\n", key, value)));
        }
    };
    push(
        "enabled",
        config.enabled.as_ref().map(|list| {
            if list.is_empty() {
                "none".to_string()
            } else {
                list.join(",")
            }
        }),
    );
    push("quote", config.quote.clone());
    push("note_mode", config.note_mode.clone());
    push("note_text", config.note_text.clone());
    push("emoticon", config.emoticon.clone());
    push("glyph", config.glyph.clone());
    push("album", config.album.clone());
    push(
        "image",
        config.image.as_ref().map(|path| path.display().to_string()),
    );
    push("layout", config.layout.clone());
    push("motion", config.motion.clone());
    push("theme", config.theme.clone());
    push("variant", config.variant.clone());
    push("rule", config.rule.clone());
    push("units", config.units.clone());
    push("temp", config.temp_unit.clone());
    let numbers: [(&str, &Option<u8>); 12] = [
        ("memory_warn", &config.memory_warn),
        ("swap_warn", &config.swap_warn),
        ("disk_warn", &config.disk_warn),
        ("battery_low", &config.battery_low),
        ("temp_warn", &config.temp_warn),
        ("temp_crit", &config.temp_crit),
        ("gpu_warn", &config.gpu_warn),
        ("logo_size", &config.logo_size),
        ("logo_width", &config.logo_width),
        ("logo_min", &config.logo_min),
        ("logo_max", &config.logo_max),
        ("logo_gap", &config.logo_gap),
    ];
    for (key, value) in numbers {
        push(key, value.map(|number| number.to_string()));
    }
    push("logo_mode", config.logo_mode.clone());
    push("logo_text", config.logo_text.clone());
    push("logo_font", config.logo_font.clone());
    push("logo_tint", config.logo_tint.clone());
    let colors: [(&str, &Option<Rgb>); 8] = [
        ("color.bg", &config.color_bg),
        ("color.fg", &config.color_fg),
        ("color.dim", &config.color_dim),
        ("color.accent", &config.color_accent),
        ("color.accent2", &config.color_accent2),
        ("color.ok", &config.color_ok),
        ("color.warn", &config.color_warn),
        ("color.err", &config.color_err),
    ];
    for (key, value) in colors {
        push(key, value.map(hex));
    }
    out
}

fn glyph_lines(config: &Config) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut push = |key: &str, value: Option<&String>| {
        if let Some(value) = value {
            out.push((key.to_string(), format!("{} = {}\n", key, value)));
        }
    };
    push("system", config.glyph_system.as_ref());
    push("core", config.glyph_core.as_ref());
    push("power", config.glyph_power.as_ref());
    push("row", config.glyph_row.as_ref());
    push("pool", config.glyph_pool.as_ref());
    for id in MODULE_IDS {
        push(id, config.glyphs.get(*id));
    }
    out
}

fn rotate_lines(config: &Config) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut push = |key: &str, value: Option<String>| {
        if let Some(value) = value {
            out.push((key.to_string(), format!("{} = {}\n", key, value)));
        }
    };
    push("art", config.rotate_art.clone());
    push("palette", config.rotate_palette.clone());
    push("every", config.rotate_every.clone());
    push(
        "themes",
        config
            .rotate_themes
            .as_ref()
            .map(|themes| themes.join(","))
            .filter(|value| !value.is_empty()),
    );
    out
}

fn section_of_line(line: &str) -> Option<&str> {
    line.trim()
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .map(str::trim)
}

pub fn write_config(path: &Path, config: &Config) -> io::Result<()> {
    let existing = fs::read_to_string(path).unwrap_or_default();
    let flat = flat_lines(config);
    let glyphs = glyph_lines(config);
    let rotate = rotate_lines(config);

    let mut out: Vec<String> = Vec::new();
    let mut seen_flat: HashSet<String> = HashSet::new();
    let mut seen_glyph: HashSet<String> = HashSet::new();
    let mut seen_rotate: HashSet<String> = HashSet::new();
    let mut section = Place::Flat;
    let mut first_section: Option<usize> = None;
    let mut glyph_section: Option<usize> = None;
    let mut rotate_section: Option<usize> = None;

    for raw in existing.lines() {
        let line = raw.trim();
        if let Some(name) = section_of_line(line) {
            section = match name {
                "glyphs" => Place::Glyph,
                "rotate" => Place::Rotate,
                _ => Place::Flat,
            };
            if first_section.is_none() {
                first_section = Some(out.len());
            }
            match section {
                Place::Glyph => glyph_section = Some(out.len()),
                Place::Rotate => rotate_section = Some(out.len()),
                Place::Flat => {}
            }
            out.push(raw.to_string());
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            out.push(raw.to_string());
            continue;
        }
        let Some((key, _)) = line.split_once('=') else {
            out.push(raw.to_string());
            continue;
        };
        let key = key.trim();
        let Some((place, bare)) = (match section {
            Place::Flat => place_of(key),
            Place::Glyph => section_key(Place::Glyph, key).map(|bare| (Place::Glyph, bare)),
            Place::Rotate => section_key(Place::Rotate, key).map(|bare| (Place::Rotate, bare)),
        }) else {
            out.push(raw.to_string());
            continue;
        };
        if place != section {
            match place {
                Place::Flat => {
                    seen_flat.insert(bare.to_string());
                }
                Place::Glyph => {
                    seen_glyph.insert(bare.to_string());
                }
                Place::Rotate => {
                    seen_rotate.insert(bare.to_string());
                }
            }
            continue;
        }
        let (desired, seen) = match place {
            Place::Flat => (&flat, &mut seen_flat),
            Place::Glyph => (&glyphs, &mut seen_glyph),
            Place::Rotate => (&rotate, &mut seen_rotate),
        };
        let duplicate = !seen.insert(bare.to_string());
        if let Some((_, line)) = desired.iter().find(|(name, _)| name == bare) {
            if !duplicate {
                out.push(line.trim_end().to_string());
            }
        }
    }

    let missing_flat: Vec<&str> = flat
        .iter()
        .filter(|(name, _)| !seen_flat.contains(name))
        .map(|(_, line)| line.trim_end())
        .collect();
    if !missing_flat.is_empty() {
        let at = first_section.unwrap_or(out.len());
        for (offset, line) in missing_flat.iter().enumerate() {
            out.insert(at + offset, line.to_string());
        }
        if let Some(index) = first_section.as_mut() {
            *index += missing_flat.len();
        }
        if let Some(index) = glyph_section.as_mut() {
            *index += missing_flat.len();
        }
        if let Some(index) = rotate_section.as_mut() {
            *index += missing_flat.len();
        }
    }

    let missing_glyph: Vec<&str> = glyphs
        .iter()
        .filter(|(name, _)| !seen_glyph.contains(name))
        .map(|(_, line)| line.trim_end())
        .collect();
    if !missing_glyph.is_empty() {
        match glyph_section {
            Some(header) => {
                let mut end = header + 1;
                while end < out.len() && section_of_line(&out[end]).is_none() {
                    end += 1;
                }
                while end > header + 1 && out[end - 1].trim().is_empty() {
                    end -= 1;
                }
                for (offset, line) in missing_glyph.iter().enumerate() {
                    out.insert(end + offset, line.to_string());
                }
                if let Some(index) = rotate_section.as_mut() {
                    *index += missing_glyph.len();
                }
            }
            None => {
                while out.last().is_some_and(|line| line.trim().is_empty()) {
                    out.pop();
                }
                out.push(String::new());
                out.push("[glyphs]".to_string());
                for line in &missing_glyph {
                    out.push((*line).to_string());
                }
            }
        }
    }

    let missing_rotate: Vec<&str> = rotate
        .iter()
        .filter(|(name, _)| !seen_rotate.contains(name))
        .map(|(_, line)| line.trim_end())
        .collect();
    if !missing_rotate.is_empty() {
        match rotate_section {
            Some(header) => {
                let mut end = header + 1;
                while end < out.len() && section_of_line(&out[end]).is_none() {
                    end += 1;
                }
                while end > header + 1 && out[end - 1].trim().is_empty() {
                    end -= 1;
                }
                for (offset, line) in missing_rotate.iter().enumerate() {
                    out.insert(end + offset, line.to_string());
                }
            }
            None => {
                while out.last().is_some_and(|line| line.trim().is_empty()) {
                    out.pop();
                }
                out.push(String::new());
                out.push("[rotate]".to_string());
                for line in &missing_rotate {
                    out.push((*line).to_string());
                }
            }
        }
    }

    let mut cleaned: Vec<String> = Vec::new();
    let mut index = 0;
    while index < out.len() {
        let line = &out[index];
        let name = section_of_line(line);
        let mut next = index + 1;
        while next < out.len() && section_of_line(&out[next]).is_none() {
            next += 1;
        }
        let empty = out[index + 1..next]
            .iter()
            .all(|entry| entry.trim().is_empty());
        let removable = matches!(name, Some("glyphs") | Some("rotate")) && empty;
        if !removable {
            cleaned.extend(out[index..next].iter().cloned());
        }
        index = next;
    }
    while cleaned.last().is_some_and(|line| line.trim().is_empty()) {
        cleaned.pop();
    }
    let mut text = String::from("# tachyon configuration\n");
    for line in cleaned {
        if line == "# tachyon configuration" {
            continue;
        }
        text.push_str(&line);
        text.push('\n');
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

pub fn hash_index(seed: u64, key: &str, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let mut hash = seed ^ 0xcbf2_9ce4_8422_2325;
    for byte in key.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    (hash as usize) % len
}

pub fn random_glyph(pool: &str, seed: u64, key: &str) -> Option<char> {
    let chars: Vec<char> = pool.chars().collect();
    if chars.is_empty() {
        return None;
    }
    Some(chars[hash_index(seed, key, chars.len())])
}

pub fn module_glyph(config: &Config, id: &str, seed: u64) -> Option<char> {
    let spec = config.glyphs.get(id).map(String::as_str);
    if let Some(spec) = spec {
        return match spec {
            "random" | "mixed" => random_glyph(config.glyph_pool(), seed, id),
            "auto" | "off" | "none" | "" => module_glyph_auto(config, id, seed),
            value => value.chars().next(),
        };
    }
    module_glyph_auto(config, id, seed)
}

fn module_glyph_auto(config: &Config, id: &str, seed: u64) -> Option<char> {
    match config.glyph_row_mode() {
        "off" | "none" => None,
        "random" | "mixed" => random_glyph(config.glyph_pool(), seed, id),
        value if value.chars().count() == 1 => value.chars().next(),
        _ => default_module_glyph(id),
    }
}

pub fn default_module_glyph(id: &str) -> Option<char> {
    MODULE_GLYPHS
        .iter()
        .find(|(module, _)| *module == id)
        .map(|(_, glyph)| *glyph)
}

pub fn section_glyph(config: &Config, section: &str, seed: u64) -> char {
    let spec = match section {
        "system" => config.glyph_system.as_deref(),
        "core" => config.glyph_core.as_deref(),
        "power" => config.glyph_power.as_deref(),
        _ => None,
    }
    .or(config.glyph.as_deref());
    match spec {
        Some("random") | Some("mixed") | None => {
            random_glyph(config.glyph_pool(), seed, section).unwrap_or('▸')
        }
        Some(value) => value.chars().next().unwrap_or('▸'),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "tachyon-config-{}-{}-{}",
            name,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        path
    }

    #[test]
    fn parses_legacy_and_block_keys() {
        let path = temp("parse");
        fs::write(
            &path,
            "# deck\n\
             glyph = ◆\n\
             logo_mode = text\n\
             logo_text = SALAHY\n             quote = “We must know. We will know.” — David Hilbert\n\
             glyph.memory = ⬢\n\
             [glyphs]\n\
             row = random\n\
             swap = ✦\n\
             [rotate]\n\
             art = next\n\
             themes = Solitude, Hackerman\n\
             enabled = os,memory\n",
        )
        .unwrap();
        let config = load_config(&path);
        assert_eq!(config.glyph.as_deref(), Some("◆"));
        assert_eq!(config.logo_mode(), "text");
        assert_eq!(config.logo_text.as_deref(), Some("SALAHY"));
        assert_eq!(
            config.quote.as_deref(),
            Some("“We must know. We will know.” — David Hilbert")
        );
        assert_eq!(config.glyphs.get("memory").map(String::as_str), Some("⬢"));
        assert_eq!(config.glyphs.get("swap").map(String::as_str), Some("✦"));
        assert_eq!(config.glyph_row_mode(), "random");
        assert_eq!(config.rotate_art_mode(), "next");
        assert_eq!(
            config.rotate_themes,
            Some(vec!["Solitude".to_string(), "Hackerman".to_string()])
        );
        assert_eq!(
            config.enabled,
            Some(vec!["os".to_string(), "memory".to_string()])
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn write_preserves_unknown_lines_and_comments() {
        let path = temp("preserve");
        fs::write(
            &path,
            "# tachyon configuration\n\
             # my note\n\
             glyph = ◆\n\
             custom_thing = keep me\n\
             [glyphs]\n\
             # glyph note\n\
             memory = ⬢\n\
             [my-section]\n\
             mystery = 42\n",
        )
        .unwrap();
        let mut config = load_config(&path);
        config.glyph = Some("▸".to_string());
        config.theme = Some("Hackerman".to_string());
        config.glyphs.insert("memory".to_string(), "✦".to_string());
        config.glyphs.insert("swap".to_string(), "⇄".to_string());
        write_config(&path, &config).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("# my note"), "{}", text);
        assert!(text.contains("custom_thing = keep me"), "{}", text);
        assert!(text.contains("glyph = ▸"), "{}", text);
        assert!(text.contains("theme = Hackerman"), "{}", text);
        assert!(text.contains("# glyph note"), "{}", text);
        assert!(text.contains("memory = ✦"), "{}", text);
        assert!(text.contains("swap = ⇄"), "{}", text);
        assert!(text.contains("[my-section]"), "{}", text);
        assert!(text.contains("mystery = 42"), "{}", text);
        let again = load_config(&path);
        assert_eq!(again.glyph.as_deref(), Some("▸"));
        assert_eq!(again.theme.as_deref(), Some("Hackerman"));
        assert_eq!(again.glyphs.get("memory").map(String::as_str), Some("✦"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn removed_keys_disappear() {
        let path = temp("remove");
        fs::write(&path, "glyph = ▸\nimage = /tmp/a.png\nalbum = x\n").unwrap();
        let mut config = load_config(&path);
        config.glyph = None;
        config.image = None;
        config.album = None;
        write_config(&path, &config).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(!text.contains("glyph = ▸"), "{}", text);
        assert!(!text.contains("image ="), "{}", text);
        assert!(!text.contains("album ="), "{}", text);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn enabled_none_round_trips() {
        let path = temp("none");
        fs::write(&path, "enabled = none\n").unwrap();
        let config = load_config(&path);
        assert_eq!(config.enabled, Some(Vec::new()));
        write_config(&path, &config).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("enabled = none"), "{}", text);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn glyph_resolution_is_deterministic_and_scoped() {
        let mut config = Config::default();
        assert_eq!(module_glyph(&config, "memory", 7), Some('●'));
        config.glyph_row = Some("off".to_string());
        assert_eq!(module_glyph(&config, "memory", 7), None);
        config.glyph_row = Some("random".to_string());
        let first = module_glyph(&config, "memory", 7);
        let again = module_glyph(&config, "memory", 7);
        assert_eq!(first, again);
        assert!(first.is_some());
        config.glyphs.insert("memory".to_string(), "✦".to_string());
        assert_eq!(module_glyph(&config, "memory", 7), Some('✦'));
        config
            .glyphs
            .insert("memory".to_string(), "auto".to_string());
        assert_eq!(module_glyph(&config, "memory", 7), first);
    }

    #[test]
    fn section_glyphs_fall_back_to_header() {
        let mut config = Config {
            glyph: Some("◆".to_string()),
            ..Config::default()
        };
        assert_eq!(section_glyph(&config, "system", 1), '◆');
        config.glyph_core = Some("✦".to_string());
        assert_eq!(section_glyph(&config, "core", 1), '✦');
        config.glyph_system = Some("random".to_string());
        let first = section_glyph(&config, "system", 3);
        assert_eq!(first, section_glyph(&config, "system", 3));
    }

    #[test]
    fn unknown_sections_do_not_leak_into_the_config() {
        let path = temp("unknown-section");
        fs::write(
            &path,
            "theme = Solitude\n[future]\ntheme = Hackerman\nmemory = ⬢\n[rotate]\nart = next\n",
        )
        .unwrap();
        let config = load_config(&path);
        assert_eq!(config.theme.as_deref(), Some("Solitude"));
        assert!(config.glyphs.is_empty());
        assert_eq!(config.rotate_art.as_deref(), Some("next"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn every_module_has_a_default_glyph() {
        for (id, _) in MODULE_IDS.iter().map(|id| (*id, ())) {
            assert!(default_module_glyph(id).is_some(), "missing {}", id);
        }
    }

    #[test]
    fn builtin_themes_cover_the_six_names() {
        for name in [
            "Solitude",
            "Vantablack",
            "Hackerman",
            "Retro 82",
            "Rose Pine",
            "Flexoki Light",
        ] {
            assert!(
                BUILTIN_THEMES.iter().any(|theme| theme.name == name),
                "missing {}",
                name
            );
        }
    }
}
