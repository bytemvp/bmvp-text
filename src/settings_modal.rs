use crate::settings::EditorSettings;
use crate::theme::{Theme, ThemeMode};
use gpui::*;

pub struct SettingsModal;

impl SettingsModal {
    pub fn render(
        settings: &EditorSettings,
        theme: &Theme,
        cx: &Context<crate::editor::EditorView>,
    ) -> impl IntoElement {
        let current_theme = theme.mode;
        let current_font = settings.font_family.clone();
        let current_size = settings.font_size;
        let current_tab = settings.tab_size;
        let show_line_num = settings.show_line_numbers;

        let current_shell = settings.terminal_shell.clone();
        let current_term_font = settings.terminal_font_family.clone();
        let current_term_size = settings.terminal_font_size;

        let text_on_accent: Hsla = rgb(0x09090b).into();

        // 1. 主题选择
        let mut theme_buttons = Vec::new();
        for &mode in ThemeMode::all() {
            let is_selected = mode == current_theme;
            let bg = if is_selected {
                theme.accent
            } else {
                theme.bg_hover
            };
            let fg = if is_selected {
                text_on_accent
            } else {
                theme.text_secondary
            };

            theme_buttons.push(
                div()
                    .px_3()
                    .py_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .bg(bg)
                    .text_color(fg)
                    .hover(|s| s.opacity(0.85))
                    .text_xs()
                    .font_weight(if is_selected {
                        FontWeight::BOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .child(mode.name())
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _win, cx| {
                            this.set_theme_mode(mode, cx);
                        }),
                    ),
            );
        }

        // 2. 编辑器字体
        let mut font_buttons = Vec::new();
        for &font in EditorSettings::AVAILABLE_FONTS {
            let is_selected = font == current_font;
            let bg = if is_selected {
                theme.accent
            } else {
                theme.bg_hover
            };
            let fg = if is_selected {
                text_on_accent
            } else {
                theme.text_secondary
            };
            let font_name = font.to_string();

            font_buttons.push(
                div()
                    .px_3()
                    .py_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .bg(bg)
                    .text_color(fg)
                    .hover(|s| s.opacity(0.85))
                    .text_xs()
                    .font_family(font)
                    .child(font)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _win, cx| {
                            this.settings.font_family = font_name.clone();
                            this.save_config();
                            cx.notify();
                        }),
                    ),
            );
        }

        // 3. 编辑器字号
        let mut size_buttons = Vec::new();
        for &size in EditorSettings::AVAILABLE_FONT_SIZES {
            let is_selected = (size - current_size).abs() < f32::EPSILON;
            let bg = if is_selected {
                theme.accent
            } else {
                theme.bg_hover
            };
            let fg = if is_selected {
                text_on_accent
            } else {
                theme.text_secondary
            };

            size_buttons.push(
                div()
                    .px_2p5()
                    .py_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .bg(bg)
                    .text_color(fg)
                    .hover(|s| s.opacity(0.85))
                    .text_xs()
                    .child(format!("{:.0}", size))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _win, cx| {
                            this.settings.font_size = size;
                            this.save_config();
                            cx.notify();
                        }),
                    ),
            );
        }

        // 4. 终端 Shell 程序选择
        let mut shell_buttons = Vec::new();
        for &(shell_title, shell_bin) in EditorSettings::AVAILABLE_SHELLS {
            let is_selected = shell_bin == current_shell;
            let bg = if is_selected {
                theme.accent
            } else {
                theme.bg_hover
            };
            let fg = if is_selected {
                text_on_accent
            } else {
                theme.text_secondary
            };
            let bin_name = shell_bin.to_string();

            shell_buttons.push(
                div()
                    .px_3()
                    .py_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .bg(bg)
                    .text_color(fg)
                    .hover(|s| s.opacity(0.85))
                    .text_xs()
                    .font_weight(if is_selected {
                        FontWeight::BOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .child(shell_title)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _win, cx| {
                            this.settings.terminal_shell = bin_name.clone();
                            this.restart_terminal();
                            this.save_config();
                            cx.notify();
                        }),
                    ),
            );
        }

        // 5. 终端字体
        let mut term_font_buttons = Vec::new();
        for &font in EditorSettings::AVAILABLE_FONTS {
            let is_selected = font == current_term_font;
            let bg = if is_selected {
                theme.accent
            } else {
                theme.bg_hover
            };
            let fg = if is_selected {
                text_on_accent
            } else {
                theme.text_secondary
            };
            let font_name = font.to_string();

            term_font_buttons.push(
                div()
                    .px_3()
                    .py_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .bg(bg)
                    .text_color(fg)
                    .hover(|s| s.opacity(0.85))
                    .text_xs()
                    .font_family(font)
                    .child(font)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _win, cx| {
                            this.settings.terminal_font_family = font_name.clone();
                            this.save_config();
                            cx.notify();
                        }),
                    ),
            );
        }

        // 6. 终端字号
        let mut term_size_buttons = Vec::new();
        for &size in EditorSettings::AVAILABLE_FONT_SIZES {
            let is_selected = (size - current_term_size).abs() < f32::EPSILON;
            let bg = if is_selected {
                theme.accent
            } else {
                theme.bg_hover
            };
            let fg = if is_selected {
                text_on_accent
            } else {
                theme.text_secondary
            };

            term_size_buttons.push(
                div()
                    .px_2p5()
                    .py_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .bg(bg)
                    .text_color(fg)
                    .hover(|s| s.opacity(0.85))
                    .text_xs()
                    .child(format!("{:.0}", size))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _win, cx| {
                            this.settings.terminal_font_size = size;
                            this.save_config();
                            cx.notify();
                        }),
                    ),
            );
        }

        // 7. Tab 大小
        let mut tab_buttons = Vec::new();
        for &tab_size in EditorSettings::AVAILABLE_TAB_SIZES {
            let is_selected = tab_size == current_tab;
            let bg = if is_selected {
                theme.accent
            } else {
                theme.bg_hover
            };
            let fg = if is_selected {
                text_on_accent
            } else {
                theme.text_secondary
            };

            tab_buttons.push(
                div()
                    .px_3()
                    .py_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .bg(bg)
                    .text_color(fg)
                    .hover(|s| s.opacity(0.85))
                    .text_xs()
                    .child(format!("{} spaces", tab_size))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _win, cx| {
                            this.settings.tab_size = tab_size;
                            this.save_config();
                            cx.notify();
                        }),
                    ),
            );
        }

        // 行号开关
        let line_num_toggle = div()
            .px_3()
            .py_1()
            .rounded_sm()
            .cursor_pointer()
            .bg(if show_line_num {
                theme.accent
            } else {
                theme.bg_hover
            })
            .text_color(if show_line_num {
                text_on_accent
            } else {
                theme.text_secondary
            })
            .text_xs()
            .font_weight(FontWeight::MEDIUM)
            .child(if show_line_num { "Enabled" } else { "Disabled" })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _win, cx| {
                    this.settings.show_line_numbers = !this.settings.show_line_numbers;
                    this.save_config();
                    cx.notify();
                }),
            );

        // Zed 风格弹窗
        div()
            .absolute()
            .inset_0()
            .flex()
            .flex_col()
            .items_center()
            .pt(px(40.0))
            .bg(hsla(0.0, 0.0, 0.0, 0.65))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _win, cx| {
                    this.show_settings = false;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .w(px(620.0))
                    .rounded_md()
                    .bg(theme.bg_modal)
                    .border_1()
                    .border_color(theme.border)
                    .shadow_lg()
                    .overflow_hidden()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {}) // 阻止冒泡
                    // 弹窗头部
                    .child(
                        div()
                            .h(px(40.0))
                            .px_4()
                            .bg(theme.bg_surface)
                            .border_b_1()
                            .border_color(theme.border_subtle)
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(theme.text_primary)
                                            .child("Preferences"),
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.text_muted)
                                            .child("— Settings & Terminal Configuration"),
                                    ),
                            )
                            .child(
                                div()
                                    .w_5()
                                    .h_5()
                                    .rounded_xs()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.bg_hover).text_color(theme.error))
                                    .text_color(theme.text_muted)
                                    .text_xs()
                                    .child("✕")
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _win, cx| {
                                            this.show_settings = false;
                                            cx.notify();
                                        }),
                                    ),
                            ),
                    )
                    // 设置主体内容区
                    .child(
                        div()
                            .p_4()
                            .flex()
                            .flex_col()
                            .gap_3p5()
                            // 1. 主题
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::MEDIUM)
                                            .text_color(theme.text_secondary)
                                            .child("Color Theme"),
                                    )
                                    .child(
                                        div().flex().flex_wrap().gap_1p5().children(theme_buttons),
                                    ),
                            )
                            // 2. 终端配置区块 (Terminal Shell & Font)
                            .child(
                                div()
                                    .pt_2()
                                    .border_t_1()
                                    .border_color(theme.border_subtle)
                                    .flex()
                                    .flex_col()
                                    .gap_2p5()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(theme.accent)
                                            .child("TERMINAL SETTINGS"),
                                    )
                                    // Shell
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.text_secondary)
                                                    .child("Terminal Shell Program"),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .gap_1p5()
                                                    .children(shell_buttons),
                                            ),
                                    )
                                    // 终端字体
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.text_secondary)
                                                    .child("Terminal Font Family"),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .gap_1p5()
                                                    .children(term_font_buttons),
                                            ),
                                    )
                                    // 终端字号
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.text_secondary)
                                                    .child("Terminal Font Size"),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .gap_1p5()
                                                    .children(term_size_buttons),
                                            ),
                                    ),
                            )
                            // 3. 编辑器字体与格式化区块
                            .child(
                                div()
                                    .pt_2()
                                    .border_t_1()
                                    .border_color(theme.border_subtle)
                                    .flex()
                                    .flex_col()
                                    .gap_2p5()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_weight(FontWeight::BOLD)
                                            .text_color(theme.text_secondary)
                                            .child("EDITOR SETTINGS"),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.text_secondary)
                                                    .child("Editor Font Family"),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .gap_1p5()
                                                    .children(font_buttons),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.text_secondary)
                                                    .child("Editor Font Size"),
                                            )
                                            .child(
                                                div()
                                                    .flex()
                                                    .flex_wrap()
                                                    .gap_1p5()
                                                    .children(size_buttons),
                                            ),
                                    ),
                            )
                            // 4. 底部杂项
                            .child(
                                div()
                                    .pt_2()
                                    .border_t_1()
                                    .border_color(theme.border_subtle)
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.text_secondary)
                                                    .child("Tab Indent Size"),
                                            )
                                            .child(div().flex().gap_1p5().children(tab_buttons)),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .items_end()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .text_color(theme.text_secondary)
                                                    .child("Line Numbers"),
                                            )
                                            .child(line_num_toggle),
                                    ),
                            ),
                    ),
            )
    }
}
