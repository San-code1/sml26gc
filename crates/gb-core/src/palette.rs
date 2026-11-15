//! Turning the screen's four shades (0 = lightest .. 3 = darkest) into colours.

/// 0xRRGGBB for shades 0..3.
pub type Palette = [u32; 4];

pub const PALETTES: [Palette; 3] = [
    [0xC4CFA1, 0x8B956D, 0x4D533C, 0x1F1F1F], // original screen green-grey
    [0xE0F8D0, 0x88C070, 0x346856, 0x081820], // bright green
    [0xFFFFFF, 0xAAAAAA, 0x555555, 0x000000], // grey
];

pub fn fb_to_rgba(fb: &[u8], pal: &Palette, out: &mut [u8]) {
    for (i, &s) in fb.iter().enumerate() {
        let c = pal[s as usize & 3];
        out[i * 4] = (c >> 16) as u8;
        out[i * 4 + 1] = (c >> 8) as u8;
        out[i * 4 + 2] = c as u8;
        out[i * 4 + 3] = 255;
    }
}
