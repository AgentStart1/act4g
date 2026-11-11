//! Scheme A semantic colors. Pass a palette through rendering so cached content follows theme changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub bg: u32,
    pub surface: u32,
    pub border: u32,
    pub text: u32,
    pub subtext: u32,
    pub accent: u32,
    pub green: u32,
    pub purple: u32,
    pub yellow: u32,
    pub red: u32,
    pub code_bg: u32,
    pub on_accent: u32,
}
impl Palette {
    pub const fn for_dark(dark: bool) -> Self {
        if dark {
            Self {
                bg: 0x0D1117,
                surface: 0x161B22,
                border: 0x30363D,
                text: 0xE6EDF3,
                subtext: 0x9DA7B3,
                accent: 0x58A6FF,
                green: 0x3FB950,
                purple: 0xBC8CFF,
                yellow: 0xF2B36D,
                red: 0xFF938A,
                code_bg: 0x21262D,
                on_accent: 0x0D1117,
            }
        } else {
            Self {
                bg: 0xFFFFFF,
                surface: 0xF6F8FA,
                border: 0xD0D7DE,
                text: 0x1F2328,
                subtext: 0x59636E,
                accent: 0x0969DA,
                green: 0x1A7F37,
                purple: 0x8250DF,
                yellow: 0x9A5B00,
                red: 0xCF222E,
                code_bg: 0xEAEFF4,
                on_accent: 0xFFFFFF,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn luminance(color: u32) -> f64 {
        let linear = |shift: u32| {
            let v = ((color >> shift) & 255u32) as f64 / 255.;
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(16) + 0.7152 * linear(8) + 0.0722 * linear(0)
    }
    #[test]
    fn both_palettes_keep_normal_text_and_controls_readable() {
        for dark in [false, true] {
            let p = Palette::for_dark(dark);
            for (fg, bg) in [
                (p.text, p.bg),
                (p.subtext, p.surface),
                (p.accent, p.surface),
                (p.on_accent, p.accent),
                (p.red, p.surface),
                (p.green, p.surface),
                (p.purple, p.surface),
                (p.yellow, p.surface),
            ] {
                let (a, b) = (luminance(fg), luminance(bg));
                let ratio = (a.max(b) + 0.05) / (a.min(b) + 0.05);
                assert!(
                    ratio >= 4.5,
                    "dark={dark}, fg={fg:x}, bg={bg:x}, contrast={ratio}"
                );
            }
        }
    }
}
