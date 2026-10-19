use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use fontdue::{Font, FontSettings};

#[derive(Clone, Debug, PartialEq)]
pub struct TextArt {
    pub rows: Vec<String>,
    pub width: usize,
}

pub fn list_system_fonts() -> Vec<String> {
    let Ok(output) = Command::new("fc-list")
        .args(["--format", "%{family}\t%{file}\n"])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    parse_fc_list(&String::from_utf8_lossy(&output.stdout))
}

fn looks_like_font(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".ttf") || lower.ends_with(".otf")
}

pub fn parse_fc_list(text: &str) -> Vec<String> {
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    for line in text.lines() {
        let Some((families, file)) = line.split_once('\t') else {
            continue;
        };
        if !looks_like_font(file.trim()) {
            continue;
        }
        for family in families.split(',') {
            let family = family.trim();
            if family.is_empty() {
                continue;
            }
            seen.entry(family.to_lowercase())
                .or_insert_with(|| family.to_string());
        }
    }
    let mut names: Vec<String> = seen.into_values().collect();
    names.sort_by_key(|name| name.to_lowercase());
    names
}

pub fn resolve_font(family: &str) -> Option<PathBuf> {
    let family = family.trim();
    if family.is_empty() {
        return None;
    }
    if family.starts_with('/') {
        let path = PathBuf::from(family);
        return path.exists().then_some(path);
    }
    let output = Command::new("fc-match")
        .args(["-f", "%{file}", family])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return None;
    }
    let path = PathBuf::from(path);
    path.exists().then_some(path)
}

pub fn render_text(text: &str, path: &Path, px: f32, max_cols: usize) -> Option<TextArt> {
    let text = text.trim();
    if text.is_empty() || max_cols < 4 {
        return None;
    }
    let data = fs::read(path).ok()?;
    let font = Font::from_bytes(data, FontSettings::default()).ok()?;
    let px = px.clamp(6.0, 220.0);
    let line = font.horizontal_line_metrics(px)?;
    let ascent = line.ascent.max(1.0);
    let descent = line.descent.min(-1.0);
    let height = (ascent - descent).ceil() as usize;
    if height == 0 {
        return None;
    }
    let mut pen = Vec::new();
    let mut cursor = 0.0f32;
    let mut previous = None;
    for character in text.chars() {
        if let Some(previous) = previous {
            cursor += font.horizontal_kern(previous, character, px).unwrap_or(0.0);
        }
        let metrics = font.metrics(character, px);
        pen.push((character, cursor));
        cursor += metrics.advance_width;
        previous = Some(character);
    }
    let width = cursor.ceil().max(1.0) as usize;
    let mut coverage = vec![0u8; width * height];
    for (character, offset) in pen {
        let (metrics, bitmap) = font.rasterize(character, px);
        if metrics.width == 0 || metrics.height == 0 {
            continue;
        }
        let x0 = (offset + metrics.xmin as f32).round() as i32;
        let y0 = (ascent - metrics.ymin as f32 - metrics.height as f32).round() as i32;
        for row in 0..metrics.height {
            let y = y0 + row as i32;
            if y < 0 || y as usize >= height {
                continue;
            }
            for col in 0..metrics.width {
                let x = x0 + col as i32;
                if x < 0 || x as usize >= width {
                    continue;
                }
                let value = bitmap[row * metrics.width + col];
                let slot = &mut coverage[y as usize * width + x as usize];
                *slot = (*slot).max(value);
            }
        }
    }
    Some(coverage_to_braille(&coverage, width, height, max_cols))
}

fn dot_bit(dx: usize, dy: usize) -> u32 {
    match (dx, dy) {
        (0, 0) => 0x01,
        (0, 1) => 0x02,
        (0, 2) => 0x04,
        (0, 3) => 0x40,
        (1, 0) => 0x08,
        (1, 1) => 0x10,
        (1, 2) => 0x20,
        _ => 0x80,
    }
}

pub fn coverage_to_braille(
    coverage: &[u8],
    width: usize,
    height: usize,
    max_cols: usize,
) -> TextArt {
    if width == 0 || height == 0 || coverage.len() < width * height {
        return TextArt {
            rows: Vec::new(),
            width: 0,
        };
    }
    let cells = width.div_ceil(2);
    let scale = if cells > max_cols.max(1) {
        cells as f32 / max_cols.max(1) as f32
    } else {
        1.0
    };
    let out_cols = ((cells as f32) / scale).ceil().max(1.0) as usize;
    let out_rows = ((height as f32) / (4.0 * scale)).ceil().max(1.0) as usize;
    let mut rows = Vec::with_capacity(out_rows);
    let mut art_width = 0usize;
    for row in 0..out_rows {
        let mut line = String::with_capacity(out_cols);
        for col in 0..out_cols {
            let mut bits = 0u32;
            for dy in 0..4 {
                for dx in 0..2 {
                    let sx = ((col * 2 + dx) as f32 * scale) as usize;
                    let sy = ((row * 4 + dy) as f32 * scale) as usize;
                    if sx >= width || sy >= height {
                        continue;
                    }
                    let x0 = sx;
                    let y0 = sy;
                    let x1 = ((sx as f32 + scale).ceil() as usize).min(width).max(x0 + 1);
                    let y1 = ((sy as f32 + scale).ceil() as usize)
                        .min(height)
                        .max(y0 + 1);
                    let mut sum = 0u32;
                    let mut count = 0u32;
                    for y in y0..y1 {
                        for x in x0..x1 {
                            sum += coverage[y * width + x] as u32;
                            count += 1;
                        }
                    }
                    let value = sum as f32 / (count.max(1) as f32 * 255.0);
                    if value > 0.30 {
                        bits |= dot_bit(dx, dy);
                    }
                }
            }
            let character = if bits == 0 {
                ' '
            } else {
                char::from_u32(0x2800 + bits).unwrap_or(' ')
            };
            line.push(character);
        }
        let trimmed = line.trim_end();
        art_width = art_width.max(trimmed.chars().count());
        rows.push(trimmed.to_string());
    }
    TextArt {
        rows,
        width: art_width,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fc_list_dedupes_and_filters() {
        let text = "DejaVu Sans,DejaVu Sans Mono\t/usr/share/fonts/DejaVuSans.ttf\n\
                    JetBrainsMono Nerd Font\t/usr/share/fonts/JetBrainsMono.ttf\n\
                    DejaVu Sans\t/usr/share/fonts/DejaVuSans-Bold.ttf\n\
                    Weird Collection\t/usr/share/fonts/collection.ttc\n\
                    Broken\tno-font.txt\n";
        let fonts = parse_fc_list(text);
        assert!(fonts.contains(&"DejaVu Sans".to_string()));
        assert!(fonts.contains(&"DejaVu Sans Mono".to_string()));
        assert!(fonts.contains(&"JetBrainsMono Nerd Font".to_string()));
        assert!(!fonts.iter().any(|name| name == "Weird Collection"));
    }

    #[test]
    fn braille_dot_mapping_is_correct() {
        let mut coverage = vec![0u8; 2 * 4];
        coverage[0] = 255;
        let art = coverage_to_braille(&coverage, 2, 4, 10);
        assert_eq!(art.rows, vec!["⠁".to_string()]);
        assert_eq!(art.width, 1);
    }

    #[test]
    fn braille_full_cell_and_empty_cell() {
        let coverage = vec![255u8; 2 * 4];
        let art = coverage_to_braille(&coverage, 2, 4, 10);
        assert_eq!(art.rows, vec!["⣿".to_string()]);
        let blank = vec![0u8; 2 * 4];
        let art = coverage_to_braille(&blank, 2, 4, 10);
        assert_eq!(art.rows, vec![String::new()]);
        assert_eq!(art.width, 0);
    }

    #[test]
    fn braille_fits_the_requested_width() {
        let coverage = vec![255u8; 40 * 40];
        let art = coverage_to_braille(&coverage, 40, 40, 8);
        assert!(art.width <= 8, "{:?}", art);
        assert!(!art.rows.is_empty());
    }
}
