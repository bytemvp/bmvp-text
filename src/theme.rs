use gpui::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeMode {
    ZedLight,
    ZedDark,
    OneDark,
    Nord,
}

impl ThemeMode {
    pub fn all() -> &'static [ThemeMode] {
        &[
            ThemeMode::ZedLight,
            ThemeMode::ZedDark,
            ThemeMode::OneDark,
            ThemeMode::Nord,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            ThemeMode::ZedLight => "Zed Light",
            ThemeMode::ZedDark => "Zed Dark",
            ThemeMode::OneDark => "One Dark",
            ThemeMode::Nord => "Nord",
        }
    }
}

#[derive(Clone)]
pub struct Theme {
    pub mode: ThemeMode,
    // 基础背景层次 (符合 Zed 物理视觉层级)
    pub bg_app: Hsla,
    pub bg_activity_bar: Hsla, // 顶栏与底栏背景
    pub bg_surface: Hsla,      // 侧边栏与面板表头背景
    pub bg_editor: Hsla,       // 编辑器主背景 (与激活Tab浑然一体)
    pub bg_terminal: Hsla,     // 终端内容区背景
    pub bg_tab_bar: Hsla,      // 标签栏背景
    pub bg_tab_active: Hsla,   // 激活标签背景 (与编辑器无缝衔接)
    pub bg_tab_inactive: Hsla, // 未激活标签
    pub bg_gutter: Hsla,       // 行号栏
    pub bg_hover: Hsla,        // 悬浮背景
    pub bg_active_line: Hsla,  // 当前行微高亮
    pub bg_modal: Hsla,        // 弹窗背景

    // 边框与分割线
    pub border: Hsla,        // 分割线
    pub border_subtle: Hsla, // 弱分割线

    // 文字颜色
    pub text_primary: Hsla,   // 主文字
    pub text_secondary: Hsla, // 次级灰
    pub text_muted: Hsla,     // 静音灰
    pub text_on_accent: Hsla, // 强调色上的文字（如选中的设置按钮、块状光标字符）

    // 状态与标识
    pub accent: Hsla,          // 品牌主色
    pub cursor_color: Hsla,    // 光标色
    pub selection: Hsla,       // 选区颜色
    pub dirty_indicator: Hsla, // 修改黄点
    pub error: Hsla,           // 错误红
}

impl Default for Theme {
    fn default() -> Self {
        Self::from_mode(ThemeMode::ZedLight)
    }
}

impl Theme {
    pub fn is_light(&self) -> bool {
        matches!(self.mode, ThemeMode::ZedLight)
    }

    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::ZedLight => Self::zed_light(),
            ThemeMode::ZedDark => Self::zed_dark(),
            ThemeMode::OneDark => Self::one_dark(),
            ThemeMode::Nord => Self::nord(),
        }
    }

    pub fn zed_light() -> Self {
        Self {
            mode: ThemeMode::ZedLight,
            bg_app: rgb(0xebebeb).into(),
            bg_activity_bar: rgb(0xebebeb).into(),
            bg_surface: rgb(0xebebeb).into(),
            bg_editor: rgb(0xffffff).into(),
            bg_terminal: rgb(0xffffff).into(),
            bg_tab_bar: rgb(0xebebeb).into(),
            bg_tab_active: rgb(0xffffff).into(),
            bg_tab_inactive: rgb(0xebebeb).into(),
            bg_gutter: rgb(0xffffff).into(),
            bg_hover: rgb(0xdfdfdf).into(),
            bg_active_line: rgb(0xf5f5f5).into(),
            bg_modal: rgb(0xffffff).into(),

            border: rgb(0xd0d0d0).into(),
            border_subtle: rgb(0xdcdcdc).into(),

            text_primary: rgb(0x24292e).into(),
            text_secondary: rgb(0x57606a).into(),
            text_muted: rgb(0x8c959f).into(),
            text_on_accent: rgb(0xffffff).into(),

            accent: rgb(0x0969da).into(),
            cursor_color: rgb(0x0969da).into(),
            selection: rgba(0x0969da28).into(),
            dirty_indicator: rgb(0xd97706).into(),
            error: rgb(0xcf222e).into(),
        }
    }

    pub fn zed_dark() -> Self {
        Self {
            mode: ThemeMode::ZedDark,
            bg_app: rgb(0x18181b).into(),
            bg_activity_bar: rgb(0x131315).into(),
            bg_surface: rgb(0x1c1c1f).into(),
            bg_editor: rgb(0x18181b).into(),
            bg_terminal: rgb(0x141416).into(),
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
            text_on_accent: rgb(0x09090b).into(),

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
            bg_terminal: rgb(0x21252b).into(),
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
            text_on_accent: rgb(0x09090b).into(),

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
            bg_terminal: rgb(0x2e3440).into(),
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
            text_on_accent: rgb(0x09090b).into(),

            accent: rgb(0x88c0d0).into(),
            cursor_color: rgb(0x88c0d0).into(),
            selection: rgba(0x88c0d038).into(),
            dirty_indicator: rgb(0xebcb8b).into(),
            error: rgb(0xbf616a).into(),
        }
    }
}
