use crate::theme::ThemeMode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EditorSettings {
    pub font_family: String,
    pub font_size: f32,
    pub line_height_factor: f32,
    pub tab_size: usize,
    pub show_line_numbers: bool,

    // 终端设置
    pub terminal_shell: String,
    pub terminal_font_family: String,
    pub terminal_font_size: f32,

    // 面板尺寸设置 (支持鼠标拖拽改变并持久化)
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: f32,
    #[serde(default = "default_terminal_height")]
    pub terminal_height: f32,
}

fn default_sidebar_width() -> f32 {
    230.0
}

fn default_terminal_height() -> f32 {
    240.0
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_family: "Consolas".to_string(),
            font_size: 14.0,
            line_height_factor: 1.6,
            tab_size: 4,
            show_line_numbers: true,

            terminal_shell: "pwsh".to_string(),
            terminal_font_family: "Consolas".to_string(),
            terminal_font_size: 13.0,

            sidebar_width: 230.0,
            terminal_height: 240.0,
        }
    }
}

impl EditorSettings {
    pub const AVAILABLE_FONTS: &'static [&'static str] = &[
        "Consolas",
        "Cascadia Code",
        "Fira Code",
        "JetBrains Mono",
        "Courier New",
    ];

    pub const AVAILABLE_FONT_SIZES: &'static [f32] =
        &[11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 18.0, 20.0];

    pub const AVAILABLE_TAB_SIZES: &'static [usize] = &[2, 4, 8];

    pub const AVAILABLE_SHELLS: &'static [(&'static str, &'static str)] = &[
        ("PowerShell", "powershell"),
        ("PowerShell 7 (pwsh)", "pwsh"),
        ("Command Prompt (cmd)", "cmd"),
        ("Git Bash", "bash"),
    ];
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub theme_mode: ThemeMode,
    pub editor: EditorSettings,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme_mode: ThemeMode::ZedLight,
            editor: EditorSettings::default(),
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        if let Ok(app_data) = std::env::var("APPDATA") {
            let dir = PathBuf::from(app_data).join("bmvp-text");
            let _ = std::fs::create_dir_all(&dir);
            dir.join("settings.json")
        } else if let Ok(home) = std::env::var("USERPROFILE") {
            let dir = PathBuf::from(home).join(".bmvp-text");
            let _ = std::fs::create_dir_all(&dir);
            dir.join("settings.json")
        } else {
            PathBuf::from("bmvp_settings.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists()
            && let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(config) = serde_json::from_str::<Self>(&content)
        {
            return config;
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, content);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_config_serde() {
        let config = AppConfig {
            theme_mode: ThemeMode::Nord,
            editor: EditorSettings {
                font_family: "Consolas".to_string(),
                font_size: 16.0,
                line_height_factor: 1.6,
                tab_size: 4,
                show_line_numbers: false,
                terminal_shell: "pwsh".to_string(),
                terminal_font_family: "Cascadia Code".to_string(),
                terminal_font_size: 14.0,
                sidebar_width: 230.0,
                terminal_height: 240.0,
            },
        };

        let json = serde_json::to_string(&config).expect("serialize failed");
        let decoded: AppConfig = serde_json::from_str(&json).expect("deserialize failed");

        assert_eq!(decoded.theme_mode, ThemeMode::Nord);
        assert_eq!(decoded.editor.font_family, "Consolas");
        assert_eq!(decoded.editor.font_size, 16.0);
        assert_eq!(decoded.editor.tab_size, 4);
        assert!(!decoded.editor.show_line_numbers);
        assert_eq!(decoded.editor.terminal_shell, "pwsh");
    }
}
