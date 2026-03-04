use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
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
    "current.tmTheme".to_string()
}
fn default_sidebar_bg() -> String {
    load_system_theme_colors()
        .sidebar_bg
        .unwrap_or_else(|| "#1e1e2e".to_string())
}
fn default_sidebar_fg() -> String {
    load_system_theme_colors()
        .sidebar_fg
        .unwrap_or_else(|| "#cdd6f4".to_string())
}
fn default_editor_bg() -> String {
    load_system_theme_colors()
        .editor_bg
        .unwrap_or_else(|| "#1e1e2e".to_string())
}
fn default_highlight() -> String {
    load_system_theme_colors()
        .highlight
        .unwrap_or_else(|| "#45475a".to_string())
}
fn default_sidebar_width() -> u8 {
    25
}

#[derive(Debug, Clone, Default)]
struct DerivedThemeColors {
    sidebar_bg: Option<String>,
    sidebar_fg: Option<String>,
    editor_bg: Option<String>,
    highlight: Option<String>,
}

fn load_system_theme_colors() -> DerivedThemeColors {
    load_kitty_theme_colors().or_else(load_gtk_theme_colors).unwrap_or_default()
}

fn load_kitty_theme_colors() -> Option<DerivedThemeColors> {
    let path = dirs::config_dir()?.join("kitty").join("current-theme.conf");
    let content = fs::read_to_string(path).ok()?;
    let map = parse_key_value_colors(&content);

    Some(DerivedThemeColors {
        sidebar_bg: map
            .get("inactive_tab_background")
            .or_else(|| map.get("tab_bar_background"))
            .or_else(|| map.get("color18"))
            .cloned(),
        sidebar_fg: map
            .get("foreground")
            .or_else(|| map.get("active_tab_foreground"))
            .cloned(),
        editor_bg: map.get("background").cloned(),
        highlight: map
            .get("selection_background")
            .or_else(|| map.get("active_border_color"))
            .cloned(),
    })
}

fn load_gtk_theme_colors() -> Option<DerivedThemeColors> {
    let path = dirs::config_dir()?.join("gtk-4.0").join("gtk.css");
    let content = fs::read_to_string(path).ok()?;
    let map = parse_define_color_map(&content);

    Some(DerivedThemeColors {
        sidebar_bg: map.get("sidebar_bg_color").cloned(),
        sidebar_fg: map
            .get("sidebar_fg_color")
            .or_else(|| map.get("window_fg_color"))
            .cloned(),
        editor_bg: map
            .get("view_bg_color")
            .or_else(|| map.get("window_bg_color"))
            .cloned(),
        highlight: map
            .get("accent_bg_color")
            .or_else(|| map.get("accent_color"))
            .cloned(),
    })
}

fn parse_key_value_colors(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut parts = line.split_whitespace();
        let Some(key) = parts.next() else {
            continue;
        };
        let Some(value) = parts.next() else {
            continue;
        };

        if value.starts_with('#') {
            map.insert(key.to_string(), value.to_string());
        }
    }

    map
}

fn parse_define_color_map(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if !line.starts_with("@define-color ") {
            continue;
        }

        let without_prefix = &line["@define-color ".len()..];
        let mut parts = without_prefix.split_whitespace();
        let Some(name) = parts.next() else {
            continue;
        };
        let Some(value) = parts.next() else {
            continue;
        };

        let value = value.trim_end_matches(';');
        if value.starts_with('#') {
            map.insert(name.to_string(), value.to_string());
        }
    }

    map
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
