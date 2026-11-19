use gpui::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    ZedDark,
    OneDark,
    Nord,
    ZedLight,
}

impl ThemeMode {
    pub fn all() -> &'static [ThemeMode] {
        &[
            ThemeMode::ZedDark,
            ThemeMode::OneDark,
            ThemeMode::Nord,
            ThemeMode::ZedLight,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ThemeMode::ZedDark => "Zed Dark",
            ThemeMode::OneDark => "One Dark",
            ThemeMode::Nord => "Nord",
            ThemeMode::ZedLight => "Zed Light",
        }
    }
}

#[derive(Clone)]
pub struct Theme {
    pub mode: ThemeMode,
    // 基础背景层次 (符合 Zed 物理视觉层级)
    pub bg_app: Hsla,
    pub bg_activity_bar: Hsla, // 最底/最左侧面板 #131315
    pub bg_surface: Hsla,      // 侧边栏与标题栏 #18181b
    pub bg_editor: Hsla,       // 编辑器主背景 #18181b (与激活Tab浑然一体)
    pub bg_tab_bar: Hsla,      // 标签栏背景 #131315
    pub bg_tab_active: Hsla,   // 激活标签背景 #18181b (与编辑器无缝衔接)
    pub bg_tab_inactive: Hsla, // 未激活标签 #131315
    pub bg_gutter: Hsla,       // 行号栏 #18181b
    pub bg_hover: Hsla,        // 悬浮背景 #26262d
    pub bg_active_line: Hsla,  // 当前行微高亮 #212127
    pub bg_modal: Hsla,        // 弹窗背景 #1c1c22

    // 边框与分割线
    pub border: Hsla,        // 分割线 #282832
    pub border_subtle: Hsla, // 弱分割线 #1f1f26

    // 文字颜色
    pub text_primary: Hsla,   // 主文字白 #f4f4f5
    pub text_secondary: Hsla, // 次级灰 #9ca3af
    pub text_muted: Hsla,     // 静音灰 #52525b

    // 状态与标识
    pub accent: Hsla,          // 品牌主色 (Zed Sky Blue #38bdf8)
    pub cursor_color: Hsla,    // 光标色 #38bdf8
    pub selection: Hsla,       // 选区颜色
    pub dirty_indicator: Hsla, // 修改黄点 #f59e0b
    pub error: Hsla,           // 错误红 #f43f5e
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_mode(ThemeMode::ZedDark)
    }
}

impl Theme {
    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::ZedDark => Self::zed_dark(),
            ThemeMode::OneDark => Self::one_dark(),
            ThemeMode::Nord => Self::nord(),
            ThemeMode::ZedLight => Self::zed_light(),
        }
    }

    pub fn zed_dark() -> Self {
        Self {
            mode: ThemeMode::ZedDark,
            bg_app: rgb(0x18181b).into(),
            bg_activity_bar: rgb(0x131315).into(),
            bg_surface: rgb(0x1c1c1f).into(),
            bg_editor: rgb(0x18181b).into(),
            bg_tab_bar: rgb(0x131315).into(),
            bg_tab_active: rgb(0x18181b).into(),
            bg_tab_inactive: rgb(0x131315).into(),
            bg_gutter: rgb(0x18181b).into(),
            bg_hover: rgb(0x2a2a32).into(),
            bg_active_line: rgb(0x212127).into(),
            bg_modal: rgb(0x202026).into(),

            border: rgb(0x282832).into(),
            border_subtle: rgb(0x232329).into(),

            text_primary: rgb(0xf4f4f5).into(),
            text_secondary: rgb(0x9ca3af).into(),
            text_muted: rgb(0x52525b).into(),

            accent: rgb(0x38bdf8).into(),
            cursor_color: rgb(0x38bdf8).into(),
            selection: rgba(0x38bdf838).into(),
            dirty_indicator: rgb(0xf59e0b).into(),
            error: rgb(0xf43f5e).into(),
        }
    }

    pub fn one_dark() -> Self {
        Self {
            mode: ThemeMode::OneDark,
            bg_app: rgb(0x1e2227).into(),
            bg_activity_bar: rgb(0x1a1d22).into(),
            bg_surface: rgb(0x21252b).into(),
            bg_editor: rgb(0x282c34).into(),
            bg_tab_bar: rgb(0x21252b).into(),
            bg_tab_active: rgb(0x282c34).into(),
            bg_tab_inactive: rgb(0x21252b).into(),
            bg_gutter: rgb(0x282c34).into(),
            bg_hover: rgb(0x2c313a).into(),
            bg_active_line: rgb(0x2c313c).into(),
            bg_modal: rgb(0x21252b).into(),

            border: rgb(0x383e4a).into(),
            border_subtle: rgb(0x2b3038).into(),

            text_primary: rgb(0xabb2bf).into(),
            text_secondary: rgb(0x828997).into(),
            text_muted: rgb(0x5c6370).into(),

            accent: rgb(0x61afef).into(),
            cursor_color: rgb(0x528bff).into(),
            selection: rgba(0x61afef38).into(),
            dirty_indicator: rgb(0xe5c07b).into(),
            error: rgb(0xe06c75).into(),
        }
    }

    pub fn nord() -> Self {
        Self {
            mode: ThemeMode::Nord,
            bg_app: rgb(0x242933).into(),
            bg_activity_bar: rgb(0x20242d).into(),
            bg_surface: rgb(0x2e3440).into(),
            bg_editor: rgb(0x3b4252).into(),
            bg_tab_bar: rgb(0x2e3440).into(),
            bg_tab_active: rgb(0x3b4252).into(),
            bg_tab_inactive: rgb(0x2e3440).into(),
            bg_gutter: rgb(0x3b4252).into(),
            bg_hover: rgb(0x434c5e).into(),
            bg_active_line: rgb(0x434c5e).into(),
            bg_modal: rgb(0x2e3440).into(),

            border: rgb(0x4c566a).into(),
            border_subtle: rgb(0x3b4252).into(),

            text_primary: rgb(0xeceff4).into(),
            text_secondary: rgb(0xd8dee9).into(),
            text_muted: rgb(0x4c566a).into(),

            accent: rgb(0x88c0d0).into(),
            cursor_color: rgb(0x88c0d0).into(),
            selection: rgba(0x88c0d038).into(),
            dirty_indicator: rgb(0xebcb8b).into(),
            error: rgb(0xbf616a).into(),
        }
    }

    pub fn zed_light() -> Self {
        Self {
            mode: ThemeMode::ZedLight,
            bg_app: rgb(0xf4f4f5).into(),
            bg_activity_bar: rgb(0xebebee).into(),
            bg_surface: rgb(0xfafafa).into(),
            bg_editor: rgb(0xffffff).into(),
            bg_tab_bar: rgb(0xf4f4f6).into(),
            bg_tab_active: rgb(0xffffff).into(),
            bg_tab_inactive: rgb(0xf4f4f6).into(),
            bg_gutter: rgb(0xffffff).into(),
            bg_hover: rgb(0xe4e4e7).into(),
            bg_active_line: rgb(0xf4f4f6).into(),
            bg_modal: rgb(0xffffff).into(),

            border: rgb(0xe4e4e7).into(),
            border_subtle: rgb(0xf0f0f2).into(),

            text_primary: rgb(0x18181b).into(),
            text_secondary: rgb(0x52525b).into(),
            text_muted: rgb(0xa1a1aa).into(),

            accent: rgb(0x0284c7).into(),
            cursor_color: rgb(0x0284c7).into(),
            selection: rgba(0x0284c728).into(),
            dirty_indicator: rgb(0xd97706).into(),
            error: rgb(0xdc2626).into(),
        }
    }
}
