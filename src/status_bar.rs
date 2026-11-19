use crate::buffer::TextBuffer;
use crate::settings::EditorSettings;
use crate::theme::Theme;
use gpui::*;

pub struct StatusBar;

impl StatusBar {
    pub fn render(
        buffer: Option<&TextBuffer>,
        theme: &Theme,
        settings: &EditorSettings,
        show_sidebar: bool,
        show_terminal: bool,
        cx: &Context<crate::editor::EditorView>,
    ) -> impl IntoElement {
        let (pos_text, sel_text, lang_text, line_ending, is_dirty) = if let Some(buf) = buffer {
            let pos = buf.cursor_position();
            let pos_str = format!("Ln {}, Col {}", pos.row + 1, pos.col + 1);
            let sel_str = if let Some(sel) = buf.selection() {
                if !sel.is_empty() {
                    let range = sel.range();
                    format!("({} sel)", range.end - range.start)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };
            (
                pos_str,
                sel_str,
                buf.language(),
                buf.line_ending(),
                buf.is_dirty(),
            )
        } else {
            ("--".to_string(), String::new(), "Plain Text", "LF", false)
        };

        div()
            .h(px(24.0))
            .w_full()
            .bg(theme.bg_activity_bar)
            .border_t_1()
            .border_color(theme.border_subtle)
            .flex()
            .items_center()
            .justify_between()
            .px_2p5()
            .text_xs()
            .text_color(theme.text_secondary)
            // 左边：Explorer 开关 + Git 分支 + 保存状态 + Terminal 开关
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    // Explorer 侧边栏按钮
                    .child(
                        div()
                            .h(px(18.0))
                            .px_1p5()
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .gap_1()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .text_color(if show_sidebar {
                                theme.text_primary
                            } else {
                                theme.text_muted
                            })
                            .bg(if show_sidebar {
                                theme.bg_hover
                            } else {
                                theme.bg_activity_bar
                            })
                            .child("≡")
                            .child("Explorer")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.show_sidebar = !this.show_sidebar;
                                    cx.notify();
                                }),
                            ),
                    )
                    // Git 分支标识
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_color(theme.text_muted)
                            .child("⎇")
                            .child("main"),
                    )
                    // 保存状态指示器
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(div().w(px(6.0)).h(px(6.0)).rounded_full().bg(if is_dirty {
                                theme.dirty_indicator
                            } else {
                                theme.accent
                            }))
                            .child(
                                div()
                                    .text_color(if is_dirty {
                                        theme.dirty_indicator
                                    } else {
                                        theme.text_muted
                                    })
                                    .child(if is_dirty { "Modified" } else { "Saved" }),
                            ),
                    )
                    // 底部 Terminal 开关
                    .child(
                        div()
                            .h(px(18.0))
                            .px_1p5()
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .gap_1()
                            .cursor_pointer()
                            .bg(if show_terminal {
                                theme.bg_hover
                            } else {
                                theme.bg_activity_bar
                            })
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .text_color(if show_terminal {
                                theme.accent
                            } else {
                                theme.text_muted
                            })
                            .child(">_")
                            .child("Terminal")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.toggle_terminal(cx);
                                }),
                            ),
                    ),
            )
            // 右边：光标信息、选区、缩进空格数、编码、换行符、语言类型、设置入口
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2p5()
                    // 光标行号列号
                    .child(
                        div()
                            .h(px(18.0))
                            .px_1()
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_color(theme.text_primary)
                            .child(pos_text)
                            .children(if !sel_text.is_empty() {
                                Some(div().text_color(theme.accent).child(sel_text))
                            } else {
                                None
                            }),
                    )
                    // 缩进空格设置 (点击循环 2 -> 4 -> 8)
                    .child(
                        div()
                            .h(px(18.0))
                            .px_1p5()
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .text_color(theme.text_secondary)
                            .child(format!("Spaces: {}", settings.tab_size))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.settings.tab_size = match this.settings.tab_size {
                                        2 => 4,
                                        4 => 8,
                                        _ => 2,
                                    };
                                    this.save_config();
                                    cx.notify();
                                }),
                            ),
                    )
                    // 编码
                    .child(div().text_color(theme.text_muted).child("UTF-8"))
                    // 换行格式 LF/CRLF
                    .child(div().text_color(theme.text_muted).child(line_ending))
                    // 语言指示药丸 (Rust, TOML, Markdown, etc.)
                    .child(
                        div()
                            .h(px(18.0))
                            .px_2()
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .bg(theme.bg_surface)
                            .border_1()
                            .border_color(theme.border_subtle)
                            .text_color(theme.accent)
                            .child(lang_text),
                    )
                    // 设置图标
                    .child(
                        div()
                            .w(px(20.0))
                            .h(px(18.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .text_color(theme.text_muted)
                            .child("⚙")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.show_settings = !this.show_settings;
                                    cx.notify();
                                }),
                            ),
                    ),
            )
    }
}
