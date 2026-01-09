use ratatui::style::Color;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::Result;

/// Color theme configuration for the retro TUI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorTheme {
    /// Name of the theme
    pub name: String,
    
    /// Main terminal text color (for general output)
    pub terminal_green: ColorValue,
    
    /// Warning/system messages color
    pub terminal_amber: ColorValue,
    
    /// Border and dim text color
    pub terminal_dim_green: ColorValue,
    
    /// Background color
    pub terminal_bg: ColorValue,
    
    /// Highlight/emphasis color
    pub terminal_cyan: ColorValue,
    
    /// Error/negative diff color
    pub terminal_red: ColorValue,
    
    /// READY status color
    pub terminal_pale_blue: ColorValue,
    
    /// PROCESSING status color
    pub terminal_dark_amber: ColorValue,
    
    /// Bright/punchy text color
    pub terminal_white: ColorValue,
    
    /// Success status color (for tool completions)
    pub terminal_success: ColorValue,
}

/// Represents a color value that can be serialized/deserialized
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ColorValue {
    /// RGB color with r, g, b components
    Rgb { r: u8, g: u8, b: u8 },
    /// Named color
    Named(String),
}

impl ColorValue {
    /// Convert to ratatui Color
    pub fn to_color(&self) -> Color {
        match self {
            ColorValue::Rgb { r, g, b } => Color::Rgb(*r, *g, *b),
            ColorValue::Named(name) => match name.to_lowercase().as_str() {
                "black" => Color::Black,
                "red" => Color::Red,
                "green" => Color::Green,
                "yellow" => Color::Yellow,
                "blue" => Color::Blue,
                "magenta" => Color::Magenta,
                "cyan" => Color::Cyan,
                "gray" | "grey" => Color::Gray,
                "darkgray" | "darkgrey" => Color::DarkGray,
                "lightred" => Color::LightRed,
                "lightgreen" => Color::LightGreen,
                "lightyellow" => Color::LightYellow,
                "lightblue" => Color::LightBlue,
                "lightmagenta" => Color::LightMagenta,
                "lightcyan" => Color::LightCyan,
                "white" => Color::White,
                _ => Color::White, // Default fallback
            },
        }
    }
}

impl ColorTheme {
    /// Load a theme from a JSON file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let theme: ColorTheme = serde_json::from_str(&content)?;
        Ok(theme)
    }
    
    /// Get the default theme - Glitter is GB's signature aesthetic ✨💖👑
    pub fn default() -> Self {
        Self::glitter()
    }

    /// Get the retro sci-fi theme (G3's original - inspired by Alien terminals)
    pub fn retro_sci_fi() -> Self {
        ColorTheme {
            name: "Retro Sci-Fi".to_string(),
            terminal_green: ColorValue::Rgb { r: 136, g: 244, b: 152 },
            terminal_amber: ColorValue::Rgb { r: 242, g: 204, b: 148 },
            terminal_dim_green: ColorValue::Rgb { r: 154, g: 174, b: 135 },
            terminal_bg: ColorValue::Rgb { r: 0, g: 10, b: 0 },
            terminal_cyan: ColorValue::Rgb { r: 0, g: 255, b: 255 },
            terminal_red: ColorValue::Rgb { r: 239, g: 119, b: 109 },
            terminal_pale_blue: ColorValue::Rgb { r: 173, g: 234, b: 251 },
            terminal_dark_amber: ColorValue::Rgb { r: 204, g: 119, b: 34 },
            terminal_white: ColorValue::Rgb { r: 218, g: 218, b: 219 },
            terminal_success: ColorValue::Rgb { r: 136, g: 244, b: 152 }, // Same as terminal_green for retro theme
        }
    }
    
    /// Get the Dracula theme
    pub fn dracula() -> Self {
        ColorTheme {
            name: "Dracula".to_string(),
            terminal_green: ColorValue::Rgb { r: 248, g: 248, b: 242 }, // Use Dracula foreground (white) for main text
            terminal_amber: ColorValue::Rgb { r: 255, g: 184, b: 108 }, // Dracula orange
            terminal_dim_green: ColorValue::Rgb { r: 98, g: 114, b: 164 }, // Dracula comment
            terminal_bg: ColorValue::Rgb { r: 40, g: 42, b: 54 },      // Dracula background
            terminal_cyan: ColorValue::Rgb { r: 139, g: 233, b: 253 },  // Dracula cyan
            terminal_red: ColorValue::Rgb { r: 255, g: 85, b: 85 },     // Dracula red
            terminal_pale_blue: ColorValue::Rgb { r: 189, g: 147, b: 249 }, // Dracula purple
            terminal_dark_amber: ColorValue::Rgb { r: 255, g: 121, b: 198 }, // Dracula pink
            terminal_white: ColorValue::Rgb { r: 248, g: 248, b: 242 }, // Dracula foreground
            terminal_success: ColorValue::Rgb { r: 80, g: 250, b: 123 }, // Dracula green for success
        }
    }

    /// Get the Glitter theme - GB's signature pink aesthetic ✨💖👑
    pub fn glitter() -> Self {
        ColorTheme {
            name: "Glitter".to_string(),
            terminal_green: ColorValue::Rgb { r: 255, g: 20, b: 147 },  // Deep pink (#FF1493) - main text
            terminal_amber: ColorValue::Rgb { r: 255, g: 182, b: 193 }, // Light pink - warnings
            terminal_dim_green: ColorValue::Rgb { r: 219, g: 112, b: 147 }, // Pale violet red - dim text
            terminal_bg: ColorValue::Rgb { r: 25, g: 0, b: 15 },        // Very dark magenta bg
            terminal_cyan: ColorValue::Rgb { r: 255, g: 105, b: 180 },  // Hot pink - highlights
            terminal_red: ColorValue::Rgb { r: 255, g: 69, b: 0 },      // Red-orange for errors
            terminal_pale_blue: ColorValue::Rgb { r: 238, g: 130, b: 238 }, // Violet - READY status
            terminal_dark_amber: ColorValue::Rgb { r: 255, g: 0, b: 127 },  // Rose - PROCESSING
            terminal_white: ColorValue::Rgb { r: 255, g: 240, b: 245 }, // Lavender blush - bright text
            terminal_success: ColorValue::Rgb { r: 255, g: 20, b: 147 }, // Deep pink for success ✨
        }
    }
    
    /// Get a theme by name or from file
    pub fn load(theme_name: Option<&str>) -> Result<Self> {
        match theme_name {
            None => Ok(Self::default()),
            Some("default") | Some("glitter") | Some("pink") | Some("gb") => Ok(Self::default()),
            Some("retro") | Some("retro-sci-fi") => Ok(Self::retro_sci_fi()),
            Some("dracula") => Ok(Self::dracula()),
            Some(path) => {
                // Try to load from file
                if Path::new(path).exists() {
                    Self::from_file(path)
                } else {
                    // Try to find in standard locations
                    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
                    let theme_file = home.join(".config").join("g3").join("themes").join(format!("{}.json", path));
                    if theme_file.exists() {
                        Self::from_file(theme_file)
                    } else {
                        Err(anyhow::anyhow!("Theme '{}' not found", path))
                    }
                }
            }
        }
    }
}