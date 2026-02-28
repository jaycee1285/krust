use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

static CONFIG: OnceLock<Config> = OnceLock::new();

/// Load config once and cache it for the process lifetime.
pub fn init() -> &'static Config {
    CONFIG.get_or_init(Config::load)
}

/// Get the cached config. Panics if `init()` was not called first.
pub fn get() -> &'static Config {
    CONFIG.get().expect("config::init() must be called before config::get()")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub appearance: AppearanceConfig,
    #[serde(default)]
    pub behavior: BehaviorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppearanceConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    /// Syntax highlighting theme for the editor.
    /// Supported values: "r3bl", "ayu-light", "ayu-mirage".
    #[serde(default = "default_syntax_theme")]
    pub syntax_theme: String,
    #[serde(default)]
    pub colors: ColorConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorConfig {
    #[serde(default = "default_sidebar_bg")]
    pub sidebar_bg: String,
    #[serde(default = "default_sidebar_fg")]
    pub sidebar_fg: String,
    #[serde(default = "default_editor_bg")]
    pub editor_bg: String,
    #[serde(default = "default_highlight")]
    pub highlight: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorConfig {
    #[serde(default)]
    pub show_hidden: bool,
    #[serde(default)]
    pub follow_symlinks: bool,
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u8,
}

fn default_theme() -> String {
    "dark".to_string()
}
fn default_syntax_theme() -> String {
    "ayu-mirage".to_string()
}
fn default_sidebar_bg() -> String {
    "#1e1e2e".to_string()
}
fn default_sidebar_fg() -> String {
    "#cdd6f4".to_string()
}
fn default_editor_bg() -> String {
    "#1e1e2e".to_string()
}
fn default_highlight() -> String {
    "#45475a".to_string()
}
fn default_sidebar_width() -> u8 {
    25
}

impl Default for Config {
    fn default() -> Self {
        Self {
            appearance: AppearanceConfig::default(),
            behavior: BehaviorConfig::default(),
        }
    }
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            syntax_theme: default_syntax_theme(),
            colors: ColorConfig::default(),
        }
    }
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            sidebar_bg: default_sidebar_bg(),
            sidebar_fg: default_sidebar_fg(),
            editor_bg: default_editor_bg(),
            highlight: default_highlight(),
        }
    }
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            follow_symlinks: false,
            sidebar_width: default_sidebar_width(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::config_path();
        if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => match toml::from_str(&content) {
                    Ok(config) => return config,
                    Err(e) => {
                        tracing::warn!("Failed to parse config: {}", e);
                    }
                },
                Err(e) => {
                    tracing::warn!("Failed to read config: {}", e);
                }
            }
        }
        Self::default()
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("krust")
            .join("config.toml")
    }
}
