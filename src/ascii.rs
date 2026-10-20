use crate::config::{Palette, Rgb};
use std::fs;
use std::path::{Path, PathBuf};

pub struct Art {
    pub id: &'static str,
    pub wide: &'static str,
    pub compact: &'static str,
    pub colors: &'static [Rgb],
}

const NIXOS: &[Rgb] = &[
    (0x52, 0x77, 0xC3),
    (0x7E, 0xBA, 0xE4),
    (0xFF, 0xFF, 0xFF),
    (0xFF, 0xFF, 0xFF),
    (0x7E, 0xBA, 0xE4),
    (0x52, 0x77, 0xC3),
];
const ARCH: &[Rgb] = &[(0x17, 0x93, 0xD1), (0xFF, 0xFF, 0xFF)];
const FEDORA: &[Rgb] = &[(0x51, 0xA2, 0xDA), (0xFF, 0xFF, 0xFF)];
const UBUNTU: &[Rgb] = &[(0xE9, 0x54, 0x20), (0xFF, 0xFF, 0xFF)];
const MINT: &[Rgb] = &[(0x87, 0xCF, 0x3E), (0xFF, 0xFF, 0xFF)];
const GENTOO: &[Rgb] = &[(0x54, 0x48, 0x7A), (0xFF, 0xD3, 0xB5)];
const DEBIAN: &[Rgb] = &[(0xD7, 0x0A, 0x53), (0xFF, 0xFF, 0xFF)];
const OPENSUSE: &[Rgb] = &[(0x73, 0xBA, 0x25), (0xFF, 0xFF, 0xFF)];
const ALPINE: &[Rgb] = &[(0x0D, 0x59, 0x7F), (0xFF, 0xFF, 0xFF)];
const VOID: &[Rgb] = &[(0x47, 0x80, 0x61), (0xFF, 0xFF, 0xFF)];
const MANJARO: &[Rgb] = &[(0x35, 0xBF, 0xA4), (0xFF, 0xFF, 0xFF)];
const POP: &[Rgb] = &[(0x48, 0xB9, 0xC7), (0xF9, 0xC7, 0x4F)];
const KALI: &[Rgb] = &[(0x26, 0x8B, 0xEE), (0xFF, 0xFF, 0xFF)];
const LINUX: &[Rgb] = &[(0xFF, 0xFF, 0xFF), (0xF9, 0xC7, 0x4F), (0x00, 0x00, 0x00)];

pub const ARTS: &[Art] = &[
    Art {
        id: "nixos",
        wide: include_str!("../logos/nixos.txt"),
        compact: include_str!("../logos/nixos-small.txt"),
        colors: NIXOS,
    },
    Art {
        id: "arch",
        wide: include_str!("../logos/arch.txt"),
        compact: include_str!("../logos/arch-small.txt"),
        colors: ARCH,
    },
    Art {
        id: "fedora",
        wide: include_str!("../logos/fedora.txt"),
        compact: include_str!("../logos/fedora-small.txt"),
        colors: FEDORA,
    },
    Art {
        id: "ubuntu",
        wide: include_str!("../logos/ubuntu.txt"),
        compact: include_str!("../logos/ubuntu-small.txt"),
        colors: UBUNTU,
    },
    Art {
        id: "mint",
        wide: include_str!("../logos/mint.txt"),
        compact: include_str!("../logos/mint-small.txt"),
        colors: MINT,
    },
    Art {
        id: "gentoo",
        wide: include_str!("../logos/gentoo.txt"),
        compact: include_str!("../logos/gentoo-small.txt"),
        colors: GENTOO,
    },
    Art {
        id: "debian",
        wide: include_str!("../logos/debian.txt"),
        compact: include_str!("../logos/debian-small.txt"),
        colors: DEBIAN,
    },
    Art {
        id: "opensuse",
        wide: include_str!("../logos/opensuse.txt"),
        compact: include_str!("../logos/opensuse-small.txt"),
        colors: OPENSUSE,
    },
    Art {
        id: "alpine",
        wide: include_str!("../logos/alpine.txt"),
        compact: include_str!("../logos/alpine-small.txt"),
        colors: ALPINE,
    },
    Art {
        id: "void",
        wide: include_str!("../logos/void.txt"),
        compact: include_str!("../logos/void-small.txt"),
        colors: VOID,
    },
    Art {
        id: "manjaro",
        wide: include_str!("../logos/manjaro.txt"),
        compact: include_str!("../logos/manjaro-small.txt"),
        colors: MANJARO,
    },
    Art {
        id: "pop",
        wide: include_str!("../logos/pop.txt"),
        compact: include_str!("../logos/pop-small.txt"),
        colors: POP,
    },
    Art {
        id: "kali",
        wide: include_str!("../logos/kali.txt"),
        compact: include_str!("../logos/kali-small.txt"),
        colors: KALI,
    },
    Art {
        id: "linux",
        wide: include_str!("../logos/linux-small.txt"),
        compact: include_str!("../logos/linux-small.txt"),
        colors: LINUX,
    },
];

#[derive(Clone)]
pub struct LoadedArt {
    pub lines: Vec<String>,
    pub colors: Vec<Rgb>,
}

pub fn ids() -> Vec<String> {
    ARTS.iter().map(|art| art.id.to_string()).collect()
}

pub fn runtime_ids(dir: &Path) -> Vec<String> {
    let mut ids: Vec<String> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("txt") {
                return None;
            }
            path.file_stem()
                .map(|stem| stem.to_string_lossy().to_string())
        })
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

pub fn logos_dir(opts: &crate::Opts) -> PathBuf {
    opts.logos.clone().unwrap_or_else(|| {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
            .unwrap_or_else(|| PathBuf::from(".local/share"));
        base.join("tachyon/logos")
    })
}

pub fn load(id: &str, dir: &Path, compact: bool) -> Option<LoadedArt> {
    if !id.is_empty() && !id.contains(['/', '\\']) {
        let path = dir.join(format!("{id}.txt"));
        if let Ok(text) = fs::read_to_string(&path) {
            if let Some(art) = parse_loaded(&text, &[]) {
                if !art.lines.is_empty() {
                    return Some(art);
                }
            }
        }
    }
    let builtin = ARTS.iter().find(|art| art.id == id)?;
    let text = if compact {
        builtin.compact
    } else {
        builtin.wide
    };
    parse_loaded(text, builtin.colors)
}

fn parse_loaded(text: &str, colors: &[Rgb]) -> Option<LoadedArt> {
    let (lines, parsed_colors) = parse_text(text);
    Some(LoadedArt {
        lines,
        colors: parsed_colors.unwrap_or_else(|| colors.to_vec()),
    })
}

fn parse_text(text: &str) -> (Vec<String>, Option<Vec<Rgb>>) {
    let mut colors = None;
    let mut lines = Vec::new();
    for line in text.lines() {
        if let Some(rest) = line.trim().strip_prefix("# colors:") {
            let parsed: Vec<Rgb> = rest
                .split(',')
                .filter_map(|item| parse_hex(item.trim()))
                .collect();
            if !parsed.is_empty() {
                colors = Some(parsed);
            }
        } else {
            lines.push(line.to_string());
        }
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    (lines, colors)
}

pub fn parse_hex(text: &str) -> Option<Rgb> {
    let hex = text
        .trim()
        .trim_start_matches('#')
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    if hex.len() != 6 {
        return None;
    }
    let value = u32::from_str_radix(hex, 16).ok()?;
    Some((
        ((value >> 16) & 0xff) as u8,
        ((value >> 8) & 0xff) as u8,
        (value & 0xff) as u8,
    ))
}

pub fn spans(line: &str) -> Vec<(String, usize)> {
    let mut spans = Vec::new();
    let mut buffer = String::new();
    let mut slot = 1usize;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '$' {
            if let Some(digit) = chars.peek().and_then(|next| next.to_digit(10)) {
                if (1..=9).contains(&digit) {
                    chars.next();
                    if !buffer.is_empty() {
                        spans.push((std::mem::take(&mut buffer), slot));
                    }
                    slot = digit as usize;
                    continue;
                }
            }
        }
        buffer.push(ch);
    }
    if !buffer.is_empty() {
        spans.push((buffer, slot));
    }
    spans
}

pub fn width(lines: &[String]) -> usize {
    lines
        .iter()
        .map(|line| {
            spans(line)
                .iter()
                .map(|(text, _)| text.chars().count())
                .sum()
        })
        .max()
        .unwrap_or(0)
}

pub fn rendered(art: &LoadedArt, colors: &[Rgb]) -> Vec<Vec<(String, Rgb)>> {
    art.lines
        .iter()
        .map(|line| {
            spans(line)
                .into_iter()
                .map(|(text, slot)| {
                    let color = colors
                        .get(slot.saturating_sub(1))
                        .or_else(|| colors.first())
                        .copied()
                        .unwrap_or((0xca, 0xcc, 0xcc));
                    (text, color)
                })
                .collect()
        })
        .collect()
}

pub fn theme_colors(pal: &Palette) -> Vec<Rgb> {
    vec![
        pal.accent,
        pal.accent2,
        pal.fg,
        pal.dim,
        pal.ok,
        pal.warn,
        pal.err,
    ]
}

pub fn derive_palette(colors: &[Rgb], base: &Palette) -> Palette {
    let mut candidates: Vec<Rgb> = colors
        .iter()
        .copied()
        .filter(|color| saturation(*color) > 0.18 && luminance(*color) > 0.06)
        .collect();
    candidates.sort_by(|a, b| {
        saturation(*b)
            .partial_cmp(&saturation(*a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.dedup_by(|a, b| distance(*a, *b) < 24);
    let accent = candidates.first().copied().unwrap_or(base.accent);
    let accent2 = candidates
        .iter()
        .skip(1)
        .copied()
        .find(|color| distance(*color, accent) > 48)
        .unwrap_or(base.accent2);
    Palette {
        accent,
        accent2,
        ..*base
    }
}

fn saturation((r, g, b): Rgb) -> f64 {
    let max = r.max(g).max(b) as f64;
    let min = r.min(g).min(b) as f64;
    if max <= 0.0 {
        return 0.0;
    }
    (max - min) / max
}

fn luminance((r, g, b): Rgb) -> f64 {
    (0.2126 * r as f64 + 0.7152 * g as f64 + 0.0722 * b as f64) / 255.0
}

fn distance(a: Rgb, b: Rgb) -> u32 {
    let dr = a.0.abs_diff(b.0) as u32;
    let dg = a.1.abs_diff(b.1) as u32;
    let db = a.2.abs_diff(b.2) as u32;
    dr * dr + dg * dg + db * db
}

pub fn detect(demo: bool) -> String {
    if demo {
        return "nixos".to_string();
    }
    let text = fs::read_to_string("/etc/os-release").unwrap_or_default();
    detect_in(&text)
}

pub fn detect_in(text: &str) -> String {
    let mut id = None;
    let mut like = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("ID=") {
            id = Some(value.trim().trim_matches('"').to_lowercase());
        } else if let Some(value) = line.strip_prefix("ID_LIKE=") {
            like = Some(value.trim().trim_matches('"').to_lowercase());
        }
    }
    id.and_then(|value| known(&value))
        .or_else(|| like.and_then(|value| value.split_whitespace().find_map(known)))
        .unwrap_or_else(|| "linux".to_string())
}

fn known(id: &str) -> Option<String> {
    let mapped = match id {
        "linuxmint" | "mint" => "mint",
        "opensuse-leap" | "opensuse-tumbleweed" | "opensuse-slowroll" | "opensuse" => "opensuse",
        "pop" | "pop_os" => "pop",
        "archarm" => "arch",
        other => other,
    };
    ARTS.iter()
        .any(|art| art.id == mapped)
        .then(|| mapped.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_spans_keep_slots_and_text() {
        let parts = spans("$1ab$2cd$1ef");
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], ("ab".to_string(), 1));
        assert_eq!(parts[1], ("cd".to_string(), 2));
        assert_eq!(parts[2], ("ef".to_string(), 1));
        assert_eq!(spans("plain").len(), 1);
        assert_eq!(spans("plain")[0].1, 1);
    }

    #[test]
    fn os_release_detection_maps_common_ids() {
        assert_eq!(
            detect_in("ID=linuxmint\nID_LIKE=\"ubuntu debian\"\n"),
            "mint"
        );
        assert_eq!(detect_in("ID=omarchy\nID_LIKE=\"arch\"\n"), "arch");
        assert_eq!(detect_in("ID=unknown\nID_LIKE=\"debian\"\n"), "debian");
        assert_eq!(detect_in("ID=unknown\n"), "linux");
        assert_eq!(detect_in("ID=opensuse-tumbleweed\n"), "opensuse");
    }

    #[test]
    fn derived_palette_uses_the_most_saturated_logo_colors() {
        let base = crate::FALLBACK_PALETTE;
        let derived = derive_palette(&[(0x17, 0x93, 0xD1), (0xFF, 0xFF, 0xFF)], &base);
        assert_eq!(derived.accent, (0x17, 0x93, 0xD1));
        assert_eq!(derived.accent2, base.accent2);
        assert_eq!(derived.bg, base.bg);
    }

    #[test]
    fn every_embedded_art_has_content_and_colors() {
        for art in ARTS {
            assert!(!art.wide.trim().is_empty(), "{} wide", art.id);
            assert!(!art.compact.trim().is_empty(), "{} compact", art.id);
            assert!(!art.colors.is_empty(), "{} colors", art.id);
            assert!(width(&parse_text(art.wide).0) > 3, "{} wide width", art.id);
        }
    }
}
