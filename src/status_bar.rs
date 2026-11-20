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
        _show_sidebar: bool,
        show_terminal: bool,
        cx: &Context<crate::editor::EditorView>,
    ) -> impl IntoElement {
        let (pos_text, sel_text, lang_text, line_ending, is_dirty) = if let Some(buf) = buffer {
            let pos = buf.cursor_position();
            let pos_str = format!("{}:{}", pos.row + 1, pos.col + 1);
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
            .h(px(26.0))
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
            // 左侧：终端开关 + 搜索开关 + Git 分支 + 保存状态
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    // Terminal 开关图标
                    .child(
                        div()
                            .h(px(20.0))
                            .px_1p5()
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .bg(if show_terminal {
                                theme.bg_hover
                            } else {
                                theme.bg_activity_bar
                            })
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .text_color(if show_terminal {
                                theme.text_primary
                            } else {
                                theme.text_secondary
                            })
                            .child(">_")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.toggle_terminal(cx);
                                }),
                            ),
                    )
                    // 搜索开关图标
                    .child(
                        div()
                            .h(px(20.0))
                            .w(px(22.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .text_color(theme.text_secondary)
                            .child("⌕")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.open_find_bar(cx);
                                }),
                            ),
                    )
                    // 分隔竖线
                    .child(div().w(px(1.0)).h(px(12.0)).bg(theme.border_subtle))
                    // Git 分支
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_color(theme.text_secondary)
                            .child("⎇")
                            .child("main"),
                    )
                    // 保存状态指示点
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .child(div().w(px(6.0)).h(px(6.0)).rounded_full().bg(if is_dirty {
                                theme.dirty_indicator
                            } else {
                                theme.text_muted
                            }))
                            .children(if is_dirty {
                                Some(div().text_color(theme.dirty_indicator).child("Modified"))
                            } else {
                                None
                            }),
                    ),
            )
            // 右侧：行列 (1:1)、语言名 (Plain Text / Rust)、缩进、编码、换行、设置
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2p5()
                    .child(
                        div()
                            .h(px(20.0))
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
                    .child(
                        div()
                            .h(px(20.0))
                            .px_1p5()
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .text_color(theme.text_primary)
                            .child(lang_text),
                    )
                    .child(
                        div()
                            .h(px(20.0))
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
                    .child(div().text_color(theme.text_muted).child("UTF-8"))
                    .child(div().text_color(theme.text_muted).child(line_ending))
                    .child(
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .text_color(theme.text_secondary)
                            .child("⚙")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.toggle_settings_modal(cx);
                                }),
                            ),
                    ),
            )
    }
}
