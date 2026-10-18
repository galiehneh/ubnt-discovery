//! Pure, GUI-free theme logic: the two colour palettes, persistence text and
//! the WCAG contrast ratio used to keep them readable.

pub type Rgb = (u8, u8, u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Theme {
    Light,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    pub background: Rgb,
    pub surface: Rgb,
    pub text: Rgb,
    pub secondary: Rgb,
    #[allow(dead_code)]
    pub accent: Rgb,
}

const LIGHT: Palette = Palette {
    background: (0xF6, 0xF8, 0xFC),
    surface: (0xFF, 0xFF, 0xFF),
    text: (0x0F, 0x17, 0x2A),
    secondary: (0x3C, 0x48, 0x58),
    accent: (0x0B, 0x57, 0xD0),
};

const DARK: Palette = Palette {
    background: (0x0F, 0x14, 0x19),
    surface: (0x1B, 0x22, 0x2A),
    text: (0xE3, 0xE8, 0xEF),
    secondary: (0xBF, 0xC8, 0xD4),
    accent: (0xA8, 0xC7, 0xFA),
};

impl Theme {
    pub fn palette(self) -> Palette {
        match self {
            Theme::Light => LIGHT,
            Theme::Dark => DARK,
        }
    }

    pub fn toggled(self) -> Theme {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }

    pub fn parse(text: &str) -> Option<Theme> {
        match text.trim().to_ascii_lowercase().as_str() {
            "light" => Some(Theme::Light),
            "dark" => Some(Theme::Dark),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    /// Caption of the toggle button: names the theme it switches to.
    pub fn toggle_label(self) -> &'static str {
        match self {
            Theme::Light => "Dark mode",
            Theme::Dark => "Light mode",
        }
    }
}

fn channel_luminance(value: u8) -> f64 {
    let c = f64::from(value) / 255.0;
    if c <= 0.03928 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn luminance((r, g, b): Rgb) -> f64 {
    0.2126 * channel_luminance(r) + 0.7152 * channel_luminance(g) + 0.0722 * channel_luminance(b)
}

/// WCAG 2.x contrast ratio, from 1.0 (identical) to 21.0 (black on white).
pub fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la >= lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    const THEMES: [Theme; 2] = [Theme::Light, Theme::Dark];

    #[test]
    fn contrast_ratio_matches_wcag_extremes() {
        let black = (0, 0, 0);
        let white = (255, 255, 255);
        assert!((contrast_ratio(black, white) - 21.0).abs() < 1e-9);
        assert!((contrast_ratio(white, white) - 1.0).abs() < 1e-9);
        assert_eq!(contrast_ratio(black, white), contrast_ratio(white, black));
    }

    #[test]
    fn primary_text_reaches_aaa_on_background_and_surface() {
        for theme in THEMES {
            let p = theme.palette();
            for ground in [p.background, p.surface] {
                let ratio = contrast_ratio(p.text, ground);
                assert!(ratio >= 7.0, "{theme:?} text {ratio:.2}");
            }
        }
    }

    #[test]
    fn secondary_text_and_accent_reach_aa_on_background_and_surface() {
        for theme in THEMES {
            let p = theme.palette();
            for ground in [p.background, p.surface] {
                for (name, fg) in [("secondary", p.secondary), ("accent", p.accent)] {
                    let ratio = contrast_ratio(fg, ground);
                    assert!(ratio >= 4.5, "{theme:?} {name} {ratio:.2}");
                }
            }
        }
    }

    #[test]
    fn parse_round_trips_and_tolerates_whitespace_and_case() {
        for theme in THEMES {
            assert_eq!(Theme::parse(theme.as_str()), Some(theme));
        }
        assert_eq!(Theme::parse(" Dark\r\n"), Some(Theme::Dark));
        assert_eq!(Theme::parse("LIGHT"), Some(Theme::Light));
    }

    #[test]
    fn parse_rejects_unknown_text() {
        assert_eq!(Theme::parse(""), None);
        assert_eq!(Theme::parse("blue"), None);
    }

    #[test]
    fn toggle_flips_theme_and_names_the_target() {
        assert_eq!(Theme::Light.toggled(), Theme::Dark);
        assert_eq!(Theme::Dark.toggled(), Theme::Light);
        assert_eq!(Theme::Light.toggle_label(), "Dark mode");
        assert_eq!(Theme::Dark.toggle_label(), "Light mode");
    }
}
