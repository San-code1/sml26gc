//! Scripted button presses for unattended runs: "S@300-305;A@400-420".
use crate::input::Buttons;

/// "S@300-305;A@400-420" -> list of (buttons, first frame, last frame).
pub fn parse_script(s: &str) -> Vec<(Buttons, u32, u32)> {
    let mut v = Vec::new();
    for part in s.split(';').filter(|p| !p.is_empty()) {
        let Some((keys, range)) = part.split_once('@') else {
            continue;
        };
        let (a, b) = range.split_once('-').unwrap_or((range, range));
        let mut m = Buttons::NONE;
        for c in keys.chars() {
            m |= match c {
                'U' => Buttons::UP,
                'D' => Buttons::DOWN,
                'L' => Buttons::LEFT,
                'R' => Buttons::RIGHT,
                'A' => Buttons::A,
                'B' => Buttons::B,
                'S' => Buttons::START,
                'E' => Buttons::SELECT,
                _ => Buttons::NONE,
            };
        }
        v.push((m, a.parse().unwrap_or(0), b.parse().unwrap_or(0)));
    }
    v
}

pub fn buttons_at(script: &[(Buttons, u32, u32)], f: u32) -> Buttons {
    script
        .iter()
        .filter(|(_, a, b)| f >= *a && f <= *b)
        .fold(Buttons::NONE, |m, (k, _, _)| m | *k)
}
