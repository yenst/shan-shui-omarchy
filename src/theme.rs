//! Colors from the active Omarchy theme.

use std::path::PathBuf;

#[derive(Clone, Copy, Debug)]
pub struct Palette {
    /// paper
    pub bg: [u8; 3],
    /// ink
    pub fg: [u8; 3],
}

/// The original's warm paper and black ink, used outside Omarchy.
pub const PAPER: Palette = Palette { bg: [242, 230, 206], fg: [0, 0, 0] };

pub fn parse_hex(s: &str) -> Option<[u8; 3]> {
    let h = s.trim().trim_matches('"').trim_start_matches('#');
    if h.len() < 6 || !h.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

fn theme_files() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let state = std::env::var_os("XDG_STATE_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/state"));
    vec![
        state.join("omarchy/current/theme/colors.toml"),
        home.join(".config/omarchy/current/theme/colors.toml"),
    ]
}

/// Read `background` and `foreground` from the current theme's colors.toml.
pub fn load() -> Option<Palette> {
    for path in theme_files() {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let (mut bg, mut fg) = (None, None);
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            match k.trim() {
                "background" => bg = parse_hex(v),
                "foreground" => fg = parse_hex(v),
                _ => {}
            }
        }
        if let (Some(bg), Some(fg)) = (bg, fg) {
            return Some(Palette { bg, fg });
        }
    }
    None
}
