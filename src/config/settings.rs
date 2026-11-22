use crate::core::logging;
use crate::ui::theme::ThemeMode;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
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
    pub sidebar_width: f32,
    pub terminal_height: f32,
}

impl Default for EditorSettings {
    fn default() -> Self {
        #[cfg(target_os = "macos")]
        let (default_font, default_shell) = ("Menlo", "zsh");
        #[cfg(all(unix, not(target_os = "macos")))]
        let (default_font, default_shell) = ("monospace", "bash");
        #[cfg(target_os = "windows")]
        let (default_font, default_shell) = ("Consolas", "pwsh");

        Self {
            font_family: default_font.to_string(),
            font_size: 14.0,
            line_height_factor: 1.6,
            tab_size: 4,
            show_line_numbers: true,

            terminal_shell: default_shell.to_string(),
            terminal_font_family: default_font.to_string(),
            terminal_font_size: 13.0,

            sidebar_width: 230.0,
            terminal_height: 240.0,
        }
    }
}

impl EditorSettings {
    #[cfg(target_os = "macos")]
    pub const AVAILABLE_FONTS: &'static [&'static str] =
        &["Menlo", "SF Mono", "JetBrains Mono", "Fira Code", "Monaco"];
    #[cfg(all(unix, not(target_os = "macos")))]
    pub const AVAILABLE_FONTS: &'static [&'static str] = &[
        "monospace",
        "JetBrains Mono",
        "Fira Code",
        "DejaVu Sans Mono",
        "Ubuntu Mono",
    ];
    #[cfg(target_os = "windows")]
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

    #[cfg(target_os = "macos")]
    pub const AVAILABLE_SHELLS: &'static [(&'static str, &'static str)] = &[
        ("Zsh", "zsh"),
        ("Bash", "bash"),
        ("Fish", "fish"),
        ("PowerShell 7 (pwsh)", "pwsh"),
    ];
    #[cfg(all(unix, not(target_os = "macos")))]
    pub const AVAILABLE_SHELLS: &'static [(&'static str, &'static str)] = &[
        ("Bash", "bash"),
        ("Zsh", "zsh"),
        ("Fish", "fish"),
        ("PowerShell 7 (pwsh)", "pwsh"),
    ];
    #[cfg(target_os = "windows")]
    pub const AVAILABLE_SHELLS: &'static [(&'static str, &'static str)] = &[
        ("PowerShell", "powershell"),
        ("PowerShell 7 (pwsh)", "pwsh"),
        ("Command Prompt (cmd)", "cmd"),
        ("Git Bash", "bash"),
        ("Zsh", "zsh"),
        ("Fish", "fish"),
    ];
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
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
    pub fn resolve_config_path_for(
        os: &str,
        env_lookup: impl Fn(&str) -> Option<String>,
    ) -> PathBuf {
        let non_empty = |key: &str| env_lookup(key).filter(|v| !v.trim().is_empty());

        match os {
            "windows" => {
                if let Some(app_data) = non_empty("APPDATA") {
                    PathBuf::from(app_data)
                        .join("bmvp-text")
                        .join("settings.json")
                } else if let Some(home) = non_empty("USERPROFILE") {
                    PathBuf::from(home).join(".bmvp-text").join("settings.json")
                } else {
                    PathBuf::from("bmvp_settings.json")
                }
            }
            "macos" => {
                if let Some(home) = non_empty("HOME") {
                    PathBuf::from(home)
                        .join("Library")
                        .join("Application Support")
                        .join("bmvp-text")
                        .join("settings.json")
                } else {
                    PathBuf::from("bmvp_settings.json")
                }
            }
            _ => {
                if let Some(xdg) = non_empty("XDG_CONFIG_HOME") {
                    PathBuf::from(xdg).join("bmvp-text").join("settings.json")
                } else if let Some(home) = non_empty("HOME") {
                    PathBuf::from(home)
                        .join(".config")
                        .join("bmvp-text")
                        .join("settings.json")
                } else {
                    PathBuf::from("bmvp_settings.json")
                }
            }
        }
    }

    pub fn config_path() -> PathBuf {
        Self::resolve_config_path_for(std::env::consts::OS, |key| std::env::var(key).ok())
    }

    pub fn load_from_path(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read '{}': {}", path.display(), e))?;
        serde_json::from_str::<Self>(&content)
            .map_err(|e| format!("failed to parse '{}': {}", path.display(), e))
    }

    pub fn save_to_path(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("failed to create '{}': {}", parent.display(), e))?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("failed to serialize settings: {}", e))?;
        let parent = path.parent().unwrap_or_else(|| Path::new("."));
        let tmp_path = parent.join(format!(".settings-{}.tmp", std::process::id()));
        if let Err(err) = std::fs::write(&tmp_path, &content) {
            return Err(format!("failed to write '{}': {}", tmp_path.display(), err));
        }
        if let Err(err) = std::fs::rename(&tmp_path, path) {
            if let Err(rm_err) = std::fs::remove_file(&tmp_path)
                && rm_err.kind() != std::io::ErrorKind::NotFound
            {
                logging::warn(
                    "config",
                    &format!(
                        "failed to remove temp config '{}': {}",
                        tmp_path.display(),
                        rm_err
                    ),
                );
            }
            return Err(format!("failed to save '{}': {}", path.display(), err));
        }
        Ok(())
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match Self::load_from_path(&path) {
                Ok(config) => return config,
                Err(err) => logging::warn("config", &err),
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        self.save_to_path(&Self::config_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

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

    #[test]
    fn test_config_path_resolution_all_platforms() {
        let env_win = HashMap::from([("APPDATA", "C:\\Users\\demo\\AppData\\Roaming")]);
        assert_eq!(
            AppConfig::resolve_config_path_for("windows", |k| env_win.get(k).map(|v| (*v).into())),
            PathBuf::from("C:\\Users\\demo\\AppData\\Roaming")
                .join("bmvp-text")
                .join("settings.json")
        );

        let env_win_profile = HashMap::from([("USERPROFILE", "C:\\Users\\demo")]);
        assert_eq!(
            AppConfig::resolve_config_path_for("windows", |k| {
                env_win_profile.get(k).map(|v| (*v).into())
            }),
            PathBuf::from("C:\\Users\\demo")
                .join(".bmvp-text")
                .join("settings.json")
        );

        let env_mac = HashMap::from([("HOME", "/Users/alice")]);
        assert_eq!(
            AppConfig::resolve_config_path_for("macos", |k| env_mac.get(k).map(|v| (*v).into())),
            PathBuf::from("/Users/alice")
                .join("Library")
                .join("Application Support")
                .join("bmvp-text")
                .join("settings.json")
        );

        let env_xdg = HashMap::from([("XDG_CONFIG_HOME", "/home/bob/.xdg"), ("HOME", "/home/bob")]);
        assert_eq!(
            AppConfig::resolve_config_path_for("linux", |k| env_xdg.get(k).map(|v| (*v).into())),
            PathBuf::from("/home/bob/.xdg")
                .join("bmvp-text")
                .join("settings.json")
        );

        let env_linux_home = HashMap::from([("HOME", "/home/bob"), ("XDG_CONFIG_HOME", "")]);
        assert_eq!(
            AppConfig::resolve_config_path_for("linux", |k| {
                env_linux_home.get(k).map(|v| (*v).into())
            }),
            PathBuf::from("/home/bob")
                .join(".config")
                .join("bmvp-text")
                .join("settings.json")
        );

        assert_eq!(
            AppConfig::resolve_config_path_for("linux", |_| None),
            PathBuf::from("bmvp_settings.json")
        );
    }

    #[test]
    fn test_settings_full_backward_compatibility() {
        let empty: AppConfig = serde_json::from_str("{}").expect("empty object fallback");
        assert_eq!(empty.theme_mode, ThemeMode::ZedLight);
        assert_eq!(empty.editor, EditorSettings::default());

        let theme_only: AppConfig =
            serde_json::from_str(r#"{"theme_mode":"OneDark"}"#).expect("theme-only fallback");
        assert_eq!(theme_only.theme_mode, ThemeMode::OneDark);
        assert_eq!(theme_only.editor, EditorSettings::default());

        let partial_editor: AppConfig =
            serde_json::from_str(r#"{"editor":{"tab_size":2,"show_line_numbers":false}}"#)
                .expect("partial editor fallback");
        assert_eq!(partial_editor.theme_mode, ThemeMode::ZedLight);
        assert_eq!(partial_editor.editor.tab_size, 2);
        assert!(!partial_editor.editor.show_line_numbers);
        assert_eq!(
            partial_editor.editor.font_family,
            EditorSettings::default().font_family
        );
        assert_eq!(partial_editor.editor.sidebar_width, 230.0);
        assert_eq!(partial_editor.editor.terminal_height, 240.0);
    }

    #[test]
    fn test_config_save_and_load_roundtrip_temp_file() {
        let dir = std::env::temp_dir().join(format!("bmvp_text_cfg_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let cfg_path = dir.join("nested").join("settings.json");

        let cfg = AppConfig {
            theme_mode: ThemeMode::ZedDark,
            editor: EditorSettings {
                tab_size: 8,
                ..EditorSettings::default()
            },
        };
        cfg.save_to_path(&cfg_path)
            .expect("save config to temp path");

        let loaded = AppConfig::load_from_path(&cfg_path).expect("load config from temp path");
        assert_eq!(loaded.theme_mode, ThemeMode::ZedDark);
        assert_eq!(loaded.editor.tab_size, 8);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
