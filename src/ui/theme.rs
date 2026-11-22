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

#[derive(Clone, Debug)]
pub struct SyntaxColors {
    pub keyword: Hsla,
    pub type_name: Hsla,
    pub function: Hsla,
    pub string: Hsla,
    pub number: Hsla,
    pub comment: Hsla,
    pub operator: Hsla,
    pub default: Hsla,
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
    pub bg_selected: Hsla,     // 列表选中态背景
    pub bg_active_line: Hsla,  // 当前行微高亮
    pub bg_elevated: Hsla,     // 抬升卡片/胶囊背景
    pub bg_modal: Hsla,        // 弹窗背景
    pub backdrop: Hsla,        // 弹窗遮罩

    // 边框与分割线
    pub border: Hsla,         // 分割线
    pub border_subtle: Hsla,  // 弱分割线
    pub border_focused: Hsla, // 聚焦边框

    // 文字颜色
    pub text_primary: Hsla,   // 主文字
    pub text_secondary: Hsla, // 次级灰
    pub text_muted: Hsla,     // 静音灰
    pub text_on_accent: Hsla, // 强调色上的文字（如选中的设置按钮、块状光标字符）
    pub text_on_error: Hsla,  // 错误/关闭按钮高亮时的高对比文字

    // 状态与标识
    pub accent: Hsla,                // 品牌主色
    pub cursor_color: Hsla,          // 光标色
    pub selection: Hsla,             // 选区颜色
    pub dirty_indicator: Hsla,       // 修改黄点
    pub error: Hsla,                 // 错误红
    pub window_close_hover: Hsla,    // 窗口关闭按钮悬浮红
    pub scrollbar_thumb: Hsla,       // 极简滚动条滑块
    pub scrollbar_thumb_hover: Hsla, // 极简滚动条滑块悬浮态

    // 主题级语法高亮色板
    pub syntax: SyntaxColors,
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

    pub fn preview_swatches(mode: ThemeMode) -> (Hsla, Hsla, Hsla) {
        let theme = Self::from_mode(mode);
        (theme.bg_editor, theme.bg_surface, theme.accent)
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
        let text_primary: Hsla = rgb(0x24292e).into();
        let text_secondary: Hsla = rgb(0x57606a).into();
        let text_muted: Hsla = rgb(0x8c959f).into();
        let accent: Hsla = rgb(0x0969da).into();
        Self {
            mode: ThemeMode::ZedLight,
            bg_app: rgb(0xeaecef).into(),
            bg_activity_bar: rgb(0xeaecef).into(),
            bg_surface: rgb(0xf5f6f8).into(),
            bg_editor: rgb(0xffffff).into(),
            bg_terminal: rgb(0xfafafa).into(),
            bg_tab_bar: rgb(0xeaecef).into(),
            bg_tab_active: rgb(0xffffff).into(),
            bg_tab_inactive: rgb(0xeaecef).into(),
            bg_gutter: rgb(0xffffff).into(),
            bg_hover: rgb(0xe2e5ea).into(),
            bg_selected: rgba(0x0969da1c).into(),
            bg_active_line: rgb(0xf6f8fa).into(),
            bg_elevated: rgb(0xf3f4f6).into(),
            bg_modal: rgb(0xffffff).into(),
            backdrop: hsla(0.0, 0.0, 0.0, 0.24),

            border: rgb(0xd4d8df).into(),
            border_subtle: rgb(0xe5e7eb).into(),
            border_focused: accent,

            text_primary,
            text_secondary,
            text_muted,
            text_on_accent: rgb(0xffffff).into(),
            text_on_error: rgb(0xffffff).into(),

            accent,
            cursor_color: accent,
            selection: rgba(0x0969da28).into(),
            dirty_indicator: rgb(0xd97706).into(),
            error: rgb(0xcf222e).into(),
            window_close_hover: rgb(0xe81123).into(),
            scrollbar_thumb: rgba(0x24292e28).into(),
            scrollbar_thumb_hover: rgba(0x24292e4d).into(),

            syntax: SyntaxColors {
                keyword: rgb(0xa626a4).into(),
                type_name: rgb(0xc18401).into(),
                function: rgb(0x4078f2).into(),
                string: rgb(0x50a14f).into(),
                number: rgb(0x986801).into(),
                comment: text_muted,
                operator: text_secondary,
                default: text_primary,
            },
        }
    }

    pub fn zed_dark() -> Self {
        let text_primary: Hsla = rgb(0xf4f4f5).into();
        let text_secondary: Hsla = rgb(0x9ca3af).into();
        let text_muted: Hsla = rgb(0x63636e).into();
        let accent: Hsla = rgb(0x38bdf8).into();
        Self {
            mode: ThemeMode::ZedDark,
            bg_app: rgb(0x18181b).into(),
            bg_activity_bar: rgb(0x121215).into(),
            bg_surface: rgb(0x1b1b1f).into(),
            bg_editor: rgb(0x18181b).into(),
            bg_terminal: rgb(0x141417).into(),
            bg_tab_bar: rgb(0x121215).into(),
            bg_tab_active: rgb(0x18181b).into(),
            bg_tab_inactive: rgb(0x121215).into(),
            bg_gutter: rgb(0x18181b).into(),
            bg_hover: rgb(0x2a2a33).into(),
            bg_selected: rgba(0x38bdf824).into(),
            bg_active_line: rgb(0x212127).into(),
            bg_elevated: rgb(0x24242c).into(),
            bg_modal: rgb(0x1e1e24).into(),
            backdrop: hsla(0.0, 0.0, 0.0, 0.58),

            border: rgb(0x2e2e38).into(),
            border_subtle: rgb(0x23232b).into(),
            border_focused: accent,

            text_primary,
            text_secondary,
            text_muted,
            text_on_accent: rgb(0x09090b).into(),
            text_on_error: rgb(0xffffff).into(),

            accent,
            cursor_color: accent,
            selection: rgba(0x38bdf838).into(),
            dirty_indicator: rgb(0xf59e0b).into(),
            error: rgb(0xf43f5e).into(),
            window_close_hover: rgb(0xe81123).into(),
            scrollbar_thumb: rgba(0xf4f4f526).into(),
            scrollbar_thumb_hover: rgba(0xf4f4f54d).into(),

            syntax: SyntaxColors {
                keyword: rgb(0xec7279).into(),
                type_name: rgb(0xfbbf24).into(),
                function: rgb(0x38bdf8).into(),
                string: rgb(0x4ade80).into(),
                number: rgb(0xfb923c).into(),
                comment: rgb(0x71717a).into(),
                operator: text_secondary,
                default: text_primary,
            },
        }
    }

    pub fn one_dark() -> Self {
        let text_primary: Hsla = rgb(0xabb2bf).into();
        let text_secondary: Hsla = rgb(0x828997).into();
        let text_muted: Hsla = rgb(0x5c6370).into();
        let accent: Hsla = rgb(0x61afef).into();
        Self {
            mode: ThemeMode::OneDark,
            bg_app: rgb(0x1e2227).into(),
            bg_activity_bar: rgb(0x191c21).into(),
            bg_surface: rgb(0x21252b).into(),
            bg_editor: rgb(0x282c34).into(),
            bg_terminal: rgb(0x21252b).into(),
            bg_tab_bar: rgb(0x1e2227).into(),
            bg_tab_active: rgb(0x282c34).into(),
            bg_tab_inactive: rgb(0x1e2227).into(),
            bg_gutter: rgb(0x282c34).into(),
            bg_hover: rgb(0x2c313a).into(),
            bg_selected: rgba(0x61afef26).into(),
            bg_active_line: rgb(0x2c313c).into(),
            bg_elevated: rgb(0x262b33).into(),
            bg_modal: rgb(0x21252b).into(),
            backdrop: hsla(0.0, 0.0, 0.0, 0.58),

            border: rgb(0x383e4a).into(),
            border_subtle: rgb(0x2c313c).into(),
            border_focused: accent,

            text_primary,
            text_secondary,
            text_muted,
            text_on_accent: rgb(0x09090b).into(),
            text_on_error: rgb(0xffffff).into(),

            accent,
            cursor_color: rgb(0x528bff).into(),
            selection: rgba(0x61afef38).into(),
            dirty_indicator: rgb(0xe5c07b).into(),
            error: rgb(0xe06c75).into(),
            window_close_hover: rgb(0xe81123).into(),
            scrollbar_thumb: rgba(0xabb2bf28).into(),
            scrollbar_thumb_hover: rgba(0xabb2bf4d).into(),

            syntax: SyntaxColors {
                keyword: rgb(0xc678dd).into(),
                type_name: rgb(0xe5c07b).into(),
                function: rgb(0x61afef).into(),
                string: rgb(0x98c379).into(),
                number: rgb(0xd19a66).into(),
                comment: text_muted,
                operator: rgb(0x56b6c2).into(),
                default: text_primary,
            },
        }
    }

    pub fn nord() -> Self {
        let text_primary: Hsla = rgb(0xeceff4).into();
        let text_secondary: Hsla = rgb(0xd8dee9).into();
        let text_muted: Hsla = rgb(0x616e88).into();
        let accent: Hsla = rgb(0x88c0d0).into();
        Self {
            mode: ThemeMode::Nord,
            bg_app: rgb(0x242933).into(),
            bg_activity_bar: rgb(0x232832).into(),
            bg_surface: rgb(0x2b313d).into(),
            bg_editor: rgb(0x2e3440).into(),
            bg_terminal: rgb(0x272c36).into(),
            bg_tab_bar: rgb(0x232832).into(),
            bg_tab_active: rgb(0x2e3440).into(),
            bg_tab_inactive: rgb(0x232832).into(),
            bg_gutter: rgb(0x2e3440).into(),
            bg_hover: rgb(0x3b4252).into(),
            bg_selected: rgba(0x88c0d026).into(),
            bg_active_line: rgb(0x353c4a).into(),
            bg_elevated: rgb(0x363e4e).into(),
            bg_modal: rgb(0x2b313d).into(),
            backdrop: hsla(0.0, 0.0, 0.0, 0.58),

            border: rgb(0x434c5e).into(),
            border_subtle: rgb(0x373f4f).into(),
            border_focused: accent,

            text_primary,
            text_secondary,
            text_muted,
            text_on_accent: rgb(0x09090b).into(),
            text_on_error: rgb(0xffffff).into(),

            accent,
            cursor_color: accent,
            selection: rgba(0x88c0d038).into(),
            dirty_indicator: rgb(0xebcb8b).into(),
            error: rgb(0xbf616a).into(),
            window_close_hover: rgb(0xe81123).into(),
            scrollbar_thumb: rgba(0xeceff426).into(),
            scrollbar_thumb_hover: rgba(0xeceff44d).into(),

            syntax: SyntaxColors {
                keyword: rgb(0x81a1c1).into(),
                type_name: rgb(0x8fbcbb).into(),
                function: rgb(0x88c0d0).into(),
                string: rgb(0xa3be8c).into(),
                number: rgb(0xb48ead).into(),
                comment: rgb(0x616e88).into(),
                operator: rgb(0x81a1c1).into(),
                default: text_primary,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Theme, ThemeMode};
    use std::collections::HashSet;

    fn hsla_key(c: gpui::Hsla) -> (u32, u32, u32, u32) {
        (c.h.to_bits(), c.s.to_bits(), c.l.to_bits(), c.a.to_bits())
    }

    #[test]
    fn test_all_themes_have_distinct_surfaces_and_swatches() {
        let mut editors = HashSet::new();
        let mut accents = HashSet::new();

        for &mode in ThemeMode::all() {
            let theme = Theme::from_mode(mode);
            assert_eq!(theme.is_light(), mode == ThemeMode::ZedLight);
            editors.insert(hsla_key(theme.bg_editor));
            accents.insert(hsla_key(theme.accent));

            let (sw_editor, sw_surface, sw_accent) = Theme::preview_swatches(mode);
            assert_eq!(sw_editor, theme.bg_editor);
            assert_eq!(sw_surface, theme.bg_surface);
            assert_eq!(sw_accent, theme.accent);
        }

        assert_eq!(editors.len(), ThemeMode::all().len());
        assert_eq!(accents.len(), ThemeMode::all().len());
    }
}
