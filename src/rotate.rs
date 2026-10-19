use std::fs;
use std::path::Path;

use crate::config::{hash_index, Config};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct State {
    pub art: usize,
    pub palette: usize,
    pub stamp: u64,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Rotation {
    pub art: Option<usize>,
    pub theme: Option<String>,
    pub state: State,
    pub save: bool,
}

pub fn load_state(path: &Path) -> State {
    let mut state = State::default();
    if let Ok(text) = fs::read_to_string(path) {
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            match key.trim() {
                "art" => state.art = value.parse().unwrap_or(0),
                "palette" => state.palette = value.parse().unwrap_or(0),
                "stamp" => state.stamp = value.parse().unwrap_or(0),
                _ => {}
            }
        }
    }
    state
}

pub fn write_state(path: &Path, state: State) {
    let Some(parent) = path.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let mut temporary = path.to_path_buf();
    temporary.set_extension("tmp");
    let text = format!(
        "# tachyon rotation state\nart = {}\npalette = {}\nstamp = {}\n",
        state.art, state.palette, state.stamp
    );
    if fs::write(&temporary, text).is_ok() {
        let _ = fs::rename(&temporary, path);
    }
}

fn step(mode: &str, current: usize, len: usize, seed: u64, key: &str, stamp: u64) -> usize {
    if len == 0 {
        return 0;
    }
    match mode {
        "next" => (current + 1) % len,
        "shuffle" => hash_index(seed ^ stamp, key, len),
        _ => current,
    }
}

fn threshold(every: &str) -> u64 {
    match every {
        "hour" => 3600,
        "day" => 86400,
        _ => 0,
    }
}

pub fn compute(
    config: &Config,
    mut state: State,
    art_len: Option<usize>,
    themes: &[String],
    seed: u64,
    now: u64,
) -> Rotation {
    let every = config.rotate_every_mode();
    let art_mode = config.rotate_art_mode();
    let palette_mode = config.rotate_palette_mode();
    let due = now.saturating_sub(state.stamp) >= threshold(every);
    let first = state.stamp == 0;

    let art = match art_len {
        Some(len) if len > 0 => {
            let index = if due && art_mode != "off" {
                let index = if first {
                    0
                } else {
                    step(art_mode, state.art, len, seed, "art", now)
                };
                state.art = index;
                state.stamp = now;
                index
            } else {
                state.art.min(len - 1)
            };
            Some(index)
        }
        _ => None,
    };

    let theme = if palette_mode != "off" && !themes.is_empty() {
        let len = themes.len();
        let index = if due {
            let index = if first {
                0
            } else {
                step(palette_mode, state.palette, len, seed, "palette", now)
            };
            state.palette = index;
            state.stamp = now;
            index
        } else {
            state.palette.min(len - 1)
        };
        Some(themes[index].clone())
    } else {
        None
    };

    Rotation {
        art,
        theme,
        state,
        save: (art.is_some() && art_mode != "off" && due)
            || (palette_mode != "off" && !themes.is_empty() && due),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(mode: &str, palette: &str, every: &str) -> Config {
        Config {
            rotate_art: Some(mode.to_string()),
            rotate_palette: Some(palette.to_string()),
            rotate_every: Some(every.to_string()),
            ..Config::default()
        }
    }

    #[test]
    fn next_wraps_forward() {
        assert_eq!(step("next", 0, 3, 1, "art", 1), 1);
        assert_eq!(step("next", 2, 3, 1, "art", 1), 0);
        assert_eq!(step("off", 2, 3, 1, "art", 1), 2);
    }

    #[test]
    fn off_keeps_the_stored_index() {
        let config = config("off", "off", "run");
        let state = State {
            art: 2,
            palette: 1,
            stamp: 10,
        };
        let rotation = compute(&config, state, Some(4), &[], 1, 20);
        assert_eq!(rotation.art, Some(2));
        assert_eq!(rotation.theme, None);
    }

    #[test]
    fn next_advances_when_due() {
        let config = config("next", "off", "run");
        let state = State {
            art: 1,
            palette: 0,
            stamp: 10,
        };
        let rotation = compute(&config, state, Some(3), &[], 1, 20);
        assert_eq!(rotation.art, Some(2));
    }

    #[test]
    fn hour_cadence_holds_until_elapsed() {
        let config = config("next", "off", "hour");
        let state = State {
            art: 1,
            palette: 0,
            stamp: 1000,
        };
        let rotation = compute(&config, state, Some(3), &[], 1, 1100);
        assert_eq!(rotation.art, Some(1));
        let rotation = compute(&config, state, Some(3), &[], 1, 5000);
        assert_eq!(rotation.art, Some(2));
    }

    #[test]
    fn palette_rotation_picks_from_the_set() {
        let config = config("off", "next", "run");
        let themes = vec!["Solitude".to_string(), "Hackerman".to_string()];
        let rotation = compute(&config, State::default(), None, &themes, 1, 20);
        assert_eq!(rotation.theme.as_deref(), Some("Solitude"));
        let rotation = compute(&config, rotation.state, None, &themes, 1, 21);
        assert_eq!(rotation.theme.as_deref(), Some("Hackerman"));
    }

    #[test]
    fn empty_set_disables_palette_rotation() {
        let config = config("off", "next", "run");
        let rotation = compute(&config, State::default(), None, &[], 1, 20);
        assert_eq!(rotation.theme, None);
    }
}
