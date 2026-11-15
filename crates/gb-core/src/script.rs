//! Scripted button presses for unattended runs: "S@300-305;A@400-420".
use crate::bus::*;

/// "S@300-305;A@400-420" -> list of (buttons, first frame, last frame).
pub fn parse_script(s: &str) -> Vec<(u8, u32, u32)> {
    let mut v = Vec::new();
    for part in s.split(';').filter(|p| !p.is_empty()) {
        let Some((keys, range)) = part.split_once('@') else { continue };
        let (a, b) = range.split_once('-').unwrap_or((range, range));
        let mut m = 0u8;
        for c in keys.chars() {
            m |= match c {
                'U' => BTN_UP,
                'D' => BTN_DOWN,
                'L' => BTN_LEFT,
                'R' => BTN_RIGHT,
                'A' => BTN_A,
                'B' => BTN_B,
                'S' => BTN_START,
                'E' => BTN_SELECT,
                _ => 0,
            };
        }
        v.push((m, a.parse().unwrap_or(0), b.parse().unwrap_or(0)));
    }
    v
}

pub fn buttons_at(script: &[(u8, u32, u32)], f: u32) -> u8 {
    script.iter().filter(|(_, a, b)| f >= *a && f <= *b).fold(0, |m, (k, _, _)| m | k)
}
