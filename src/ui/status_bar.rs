use crate::config::settings::EditorSettings;
use crate::core::buffer::TextBuffer;
use crate::ui::UiAction;
use crate::ui::theme::Theme;
use gpui::*;
use std::rc::Rc;

pub struct StatusBarCallbacks {
    pub on_toggle_terminal: UiAction,
    pub on_open_find: UiAction,
    pub on_clear_status_message: UiAction,
    pub on_cycle_tab_size: UiAction,
    pub on_toggle_settings: UiAction,
}

pub struct StatusBar;

impl StatusBar {
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        buffer: Option<&TextBuffer>,
        theme: &Theme,
        settings: &EditorSettings,
        _show_sidebar: bool,
        show_terminal: bool,
        git_branch: Option<&str>,
        status_message: Option<(&str, bool)>,
        callbacks: StatusBarCallbacks,
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

        let on_terminal = callbacks.on_toggle_terminal;
        let on_find = callbacks.on_open_find;
        let on_clear_msg = callbacks.on_clear_status_message;
        let on_cycle_tab = callbacks.on_cycle_tab_size;
        let on_settings = callbacks.on_toggle_settings;

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
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
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
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_terminal(win, cx);
                            }),
                    )
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
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_find(win, cx);
                            }),
                    )
                    .child(div().w(px(1.0)).h(px(12.0)).bg(theme.border_subtle))
                    .children(git_branch.map(|branch| {
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_color(theme.text_secondary)
                            .child("⎇")
                            .child(branch.to_string())
                    }))
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
                    )
                    .children(status_message.map(|(msg, is_error)| {
                        let on_clear_msg = Rc::clone(&on_clear_msg);
                        div()
                            .px_1p5()
                            .py_0p5()
                            .rounded_xs()
                            .cursor_pointer()
                            .bg(theme.bg_hover)
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
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_settings(win, cx);
                            }),
                    ),
            )
    }
}
