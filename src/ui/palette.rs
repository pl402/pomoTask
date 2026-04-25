use ratatui::style::Color;
use serde::{Serialize, Deserialize};
use crate::app::App;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum Theme { 
    CatppuccinMocha, 
    Nord, 
    Gruvbox, 
    Dracula, 
    Monokai, 
    SolarizedDark, 
    Ocean, 
    Custom 
}

impl Theme {
    pub fn name(&self) -> &str {
        match self {
            Theme::CatppuccinMocha => "Catppuccin Mocha",
            Theme::Nord => "Nord",
            Theme::Gruvbox => "Gruvbox",
            Theme::Dracula => "Dracula",
            Theme::Monokai => "Monokai",
            Theme::SolarizedDark => "Solarized Dark",
            Theme::Ocean => "Ocean",
            Theme::Custom => "Custom",
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThemeColors {
    pub mauve: (u8, u8, u8),
    pub red: (u8, u8, u8),
    pub green: (u8, u8, u8),
    pub peach: (u8, u8, u8),
    pub yellow: (u8, u8, u8),
    pub blue: (u8, u8, u8),
    pub text: (u8, u8, u8),
    pub subtext0: (u8, u8, u8),
    pub overlay0: (u8, u8, u8),
    pub surface0: (u8, u8, u8),
    pub base: (u8, u8, u8),
}

impl Default for ThemeColors {
    fn default() -> Self {
        Self {
            mauve: (203, 166, 247),
            red: (243, 139, 168),
            green: (166, 227, 161),
            peach: (250, 179, 135),
            yellow: (249, 226, 175),
            blue: (137, 180, 250),
            text: (205, 214, 244),
            subtext0: (166, 173, 200),
            overlay0: (108, 112, 134),
            surface0: (49, 50, 68),
            base: (30, 30, 46),
        }
    }
}

pub struct Palette;
impl Palette {
    fn get_color(theme: Theme, custom: &Option<ThemeColors>, getter: impl Fn(&ThemeColors) -> (u8, u8, u8), preset: impl Fn(Theme) -> Color) -> Color {
        if theme == Theme::Custom {
            if let Some(c) = custom {
                let rgb = getter(c);
                return Color::Rgb(rgb.0, rgb.1, rgb.2);
            }
        }
        preset(theme)
    }

    pub fn mauve(app: &App) -> Color {
        Self::get_color(app.config.theme, &app.config.custom_theme, |c| c.mauve, |t| match t {
            Theme::CatppuccinMocha => Color::Rgb(203, 166, 247),
            Theme::Nord => Color::Rgb(180, 142, 173),
            Theme::Gruvbox => Color::Rgb(211, 134, 155),
            Theme::Dracula => Color::Rgb(189, 147, 249),
            Theme::Monokai => Color::Rgb(174, 129, 255),
            Theme::SolarizedDark => Color::Rgb(108, 113, 196),
            Theme::Ocean => Color::Rgb(192, 151, 187),
            _ => Color::Rgb(203, 166, 247),
        })
    }
    pub fn red(app: &App) -> Color {
        Self::get_color(app.config.theme, &app.config.custom_theme, |c| c.red, |t| match t {
            Theme::CatppuccinMocha => Color::Rgb(243, 139, 168),
            Theme::Nord => Color::Rgb(191, 97, 106),
            Theme::Gruvbox => Color::Rgb(251, 73, 52),
            Theme::Dracula => Color::Rgb(255, 85, 85),
            Theme::Monokai => Color::Rgb(249, 38, 114),
            Theme::SolarizedDark => Color::Rgb(220, 50, 47),
            Theme::Ocean => Color::Rgb(191, 97, 106),
            _ => Color::Rgb(243, 139, 168),
        })
    }
    pub fn green(app: &App) -> Color {
        Self::get_color(app.config.theme, &app.config.custom_theme, |c| c.green, |t| match t {
            Theme::CatppuccinMocha => Color::Rgb(166, 227, 161),
            Theme::Nord => Color::Rgb(163, 190, 140),
            Theme::Gruvbox => Color::Rgb(184, 187, 38),
            Theme::Dracula => Color::Rgb(80, 250, 123),
            Theme::Monokai => Color::Rgb(166, 226, 46),
            Theme::SolarizedDark => Color::Rgb(133, 153, 0),
            Theme::Ocean => Color::Rgb(163, 190, 140),
            _ => Color::Rgb(166, 227, 161),
        })
    }
    pub fn peach(app: &App) -> Color {
        Self::get_color(app.config.theme, &app.config.custom_theme, |c| c.peach, |t| match t {
            Theme::CatppuccinMocha => Color::Rgb(250, 179, 135),
            Theme::Nord => Color::Rgb(208, 135, 112),
            Theme::Gruvbox => Color::Rgb(254, 128, 25),
            Theme::Dracula => Color::Rgb(255, 184, 108),
            Theme::Monokai => Color::Rgb(253, 151, 31),
            Theme::SolarizedDark => Color::Rgb(203, 75, 22),
            Theme::Ocean => Color::Rgb(208, 135, 112),
            _ => Color::Rgb(250, 179, 135),
        })
    }
    pub fn yellow(app: &App) -> Color {
        Self::get_color(app.config.theme, &app.config.custom_theme, |c| c.yellow, |t| match t {
            Theme::CatppuccinMocha => Color::Rgb(249, 226, 175),
            Theme::Nord => Color::Rgb(235, 203, 139),
            Theme::Gruvbox => Color::Rgb(250, 189, 47),
            Theme::Dracula => Color::Rgb(241, 250, 140),
            Theme::Monokai => Color::Rgb(230, 219, 116),
            Theme::SolarizedDark => Color::Rgb(181, 137, 0),
            Theme::Ocean => Color::Rgb(235, 203, 139),
            _ => Color::Rgb(249, 226, 175),
        })
    }
    pub fn blue(app: &App) -> Color {
        Self::get_color(app.config.theme, &app.config.custom_theme, |c| c.blue, |t| match t {
            Theme::CatppuccinMocha => Color::Rgb(137, 180, 250),
            Theme::Nord => Color::Rgb(129, 161, 193),
            Theme::Gruvbox => Color::Rgb(131, 165, 152),
            Theme::Dracula => Color::Rgb(139, 233, 253),
            Theme::Monokai => Color::Rgb(102, 217, 239),
            Theme::SolarizedDark => Color::Rgb(38, 139, 210),
            Theme::Ocean => Color::Rgb(136, 192, 208),
            _ => Color::Rgb(137, 180, 250),
        })
    }
    pub fn text(app: &App) -> Color {
        Self::get_color(app.config.theme, &app.config.custom_theme, |c| c.text, |t| match t {
            Theme::CatppuccinMocha => Color::Rgb(205, 214, 244),
            Theme::Nord => Color::Rgb(236, 239, 244),
            Theme::Gruvbox => Color::Rgb(235, 219, 178),
            Theme::Dracula => Color::Rgb(248, 248, 242),
            Theme::Monokai => Color::Rgb(248, 248, 242),
            Theme::SolarizedDark => Color::Rgb(131, 148, 150),
            Theme::Ocean => Color::Rgb(236, 239, 244),
            _ => Color::Rgb(205, 214, 244),
        })
    }
    pub fn subtext0(app: &App) -> Color {
        Self::get_color(app.config.theme, &app.config.custom_theme, |c| c.subtext0, |t| match t {
            Theme::CatppuccinMocha => Color::Rgb(166, 173, 200),
            Theme::Nord => Color::Rgb(216, 222, 233),
            Theme::Gruvbox => Color::Rgb(168, 153, 132),
            Theme::Dracula => Color::Rgb(98, 114, 164),
            Theme::Monokai => Color::Rgb(117, 113, 94),
            Theme::SolarizedDark => Color::Rgb(101, 123, 131),
            Theme::Ocean => Color::Rgb(216, 222, 233),
            _ => Color::Rgb(166, 173, 200),
        })
    }
    pub fn overlay0(app: &App) -> Color {
        Self::get_color(app.config.theme, &app.config.custom_theme, |c| c.overlay0, |t| match t {
            Theme::CatppuccinMocha => Color::Rgb(108, 112, 134),
            Theme::Nord => Color::Rgb(76, 86, 106),
            Theme::Gruvbox => Color::Rgb(146, 131, 116),
            Theme::Dracula => Color::Rgb(68, 71, 90),
            Theme::Monokai => Color::Rgb(73, 72, 62),
            Theme::SolarizedDark => Color::Rgb(88, 110, 117),
            Theme::Ocean => Color::Rgb(76, 86, 106),
            _ => Color::Rgb(108, 112, 134),
        })
    }
    pub fn surface0(app: &App) -> Color {
        Self::get_color(app.config.theme, &app.config.custom_theme, |c| c.surface0, |t| match t {
            Theme::CatppuccinMocha => Color::Rgb(49, 50, 68),
            Theme::Nord => Color::Rgb(59, 66, 82),
            Theme::Gruvbox => Color::Rgb(60, 56, 54),
            Theme::Dracula => Color::Rgb(40, 42, 54),
            Theme::Monokai => Color::Rgb(39, 40, 34),
            Theme::SolarizedDark => Color::Rgb(7, 54, 66),
            Theme::Ocean => Color::Rgb(59, 66, 82),
            _ => Color::Rgb(49, 50, 68),
        })
    }
    pub fn base(app: &App) -> Color {
        Self::get_color(app.config.theme, &app.config.custom_theme, |c| c.base, |t| match t {
            Theme::CatppuccinMocha => Color::Rgb(30, 30, 46),
            Theme::Nord => Color::Rgb(46, 52, 64),
            Theme::Gruvbox => Color::Rgb(40, 40, 40),
            Theme::Dracula => Color::Rgb(40, 42, 54),
            Theme::Monokai => Color::Rgb(39, 40, 34),
            Theme::SolarizedDark => Color::Rgb(0, 43, 54),
            Theme::Ocean => Color::Rgb(46, 52, 64),
            _ => Color::Rgb(30, 30, 46),
        })
    }
}
