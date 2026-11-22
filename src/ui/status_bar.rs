use crate::config::settings::EditorSettings;
use crate::core::buffer::TextBuffer;
use crate::ui::UiAction;
use crate::ui::icons::{self, IconName};
use crate::ui::theme::Theme;
use gpui::*;
use std::rc::Rc;

pub struct StatusBarCallbacks {
    pub on_toggle_sidebar: UiAction,
    pub on_toggle_terminal: UiAction,
    pub on_clear_status_message: UiAction,
    pub on_cycle_tab_size: UiAction,
}

pub struct StatusBar;

impl StatusBar {
    pub fn render(
        buffer: Option<&TextBuffer>,
        theme: &Theme,
        settings: &EditorSettings,
        show_sidebar: bool,
        show_terminal: bool,
        status_message: Option<(&str, bool)>,
        callbacks: StatusBarCallbacks,
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

        let on_sidebar = callbacks.on_toggle_sidebar;
        let on_terminal = callbacks.on_toggle_terminal;
        let on_clear_msg = callbacks.on_clear_status_message;
        let on_cycle_tab = callbacks.on_cycle_tab_size;

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
            .text_size(px(11.5))
            .text_color(theme.text_secondary)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .h(px(20.0))
                            .w(px(22.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .bg(if show_sidebar {
                                theme.bg_hover
                            } else {
                                theme.bg_activity_bar
                            })
                            .hover(|s| s.bg(theme.bg_hover))
                            .child(icons::render_icon(
                                IconName::SidebarLeft,
                                px(13.0),
                                if show_sidebar {
                                    theme.accent
                                } else {
                                    theme.text_secondary
                                },
                            ))
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_sidebar(win, cx);
                            }),
                    )
                    .child(
                        div()
                            .h(px(20.0))
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
                            .text_color(if show_terminal {
                                theme.text_primary
                            } else {
                                theme.text_secondary
                            })
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .child(icons::render_icon(
                                IconName::Terminal,
                                px(13.0),
                                if show_terminal {
                                    theme.text_primary
                                } else {
                                    theme.text_secondary
                                },
                            ))
                            .child("Terminal")
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_terminal(win, cx);
                            }),
                    )
                    .child(div().w(px(1.0)).h(px(12.0)).bg(theme.border_subtle))
                    .children(if is_dirty {
                        Some(
                            div()
                                .flex()
                                .items_center()
                                .gap_1()
                                .child(
                                    div()
                                        .w(px(6.0))
                                        .h(px(6.0))
                                        .rounded_full()
                                        .bg(theme.dirty_indicator),
                                )
                                .child(div().text_color(theme.dirty_indicator).child("Modified")),
                        )
                    } else {
                        None
                    })
                    .children(status_message.map(|(msg, is_error)| {
                        let on_clear_msg = Rc::clone(&on_clear_msg);
                        div()
                            .h(px(18.0))
                            .px_2()
                            .rounded_xs()
                            .cursor_pointer()
                            .bg(theme.bg_elevated)
                            .border_1()
                            .border_color(theme.border_subtle)
                            .flex()
                            .items_center()
                            .text_color(if is_error { theme.error } else { theme.accent })
                            .child(msg.to_string())
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_clear_msg(win, cx);
                            })
                    })),
            )
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
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_cycle_tab(win, cx);
                            }),
                    )
                    .child(div().text_color(theme.text_muted).child("UTF-8"))
                    .child(div().text_color(theme.text_muted).child(line_ending)),
            )
    }
}
