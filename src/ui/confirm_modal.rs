use crate::ui::theme::Theme;
use gpui::*;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PendingClose {
    Tab {
        tab_id: usize,
        file_name: String,
    },
    Quit {
        dirty_count: usize,
        first_file_name: String,
    },
}

pub struct ConfirmModal;

impl ConfirmModal {
    pub fn render(
        pending: &PendingClose,
        theme: &Theme,
        on_save: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
        on_discard: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
        on_cancel: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> AnyElement {
        let on_cancel = Rc::new(on_cancel);
        let on_cancel_backdrop = Rc::clone(&on_cancel);
        let on_cancel_btn = Rc::clone(&on_cancel);

        let (title, message, save_label) = match pending {
            PendingClose::Tab { file_name, .. } => (
                "Unsaved Changes",
                format!(
                    "Do you want to save the changes you made to \"{}\"?",
                    file_name
                ),
                "Save",
            ),
            PendingClose::Quit {
                dirty_count,
                first_file_name,
            } => {
                let msg = if *dirty_count <= 1 {
                    format!(
                        "Do you want to save the changes you made to \"{}\" before closing?",
                        first_file_name
                    )
                } else {
                    format!(
                        "You have {} unsaved files (including \"{}\"). Save changes before closing?",
                        dirty_count, first_file_name
                    )
                };
                ("Unsaved Changes", msg, "Save & Quit")
            }
        };

        let backdrop_bg = if theme.is_light() {
            hsla(0.0, 0.0, 0.0, 0.30)
        } else {
            hsla(0.0, 0.0, 0.0, 0.65)
        };

        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(backdrop_bg)
            .occlude()
            .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
                cx.stop_propagation();
                on_cancel_backdrop(ev, win, cx);
            })
            .child(
                div()
                    .w(px(440.0))
                    .rounded_md()
                    .bg(theme.bg_modal)
                    .border_1()
                    .border_color(theme.border)
                    .shadow_lg()
                    .occlude()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(
                        div()
                            .h(px(38.0))
                            .px_4()
                            .bg(theme.bg_surface)
                            .rounded_t_md()
                            .border_b_1()
                            .border_color(theme.border_subtle)
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(8.0))
                                    .h(px(8.0))
                                    .rounded_full()
                                    .bg(theme.dirty_indicator),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme.text_primary)
                                    .child(title),
                            ),
                    )
                    .child(
                        div()
                            .p_4()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.text_primary)
                                    .child(message),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.text_muted)
                                    .child("Your changes will be lost if you don't save them."),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .pt_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(theme.text_muted)
                                            .child("Enter to Save · Esc to Cancel"),
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .px_3()
                                                    .py_1()
                                                    .rounded_sm()
                                                    .cursor_pointer()
                                                    .bg(theme.bg_hover)
                                                    .text_color(theme.text_secondary)
                                                    .hover(|s| s.text_color(theme.text_primary))
                                                    .text_xs()
                                                    .child("Cancel")
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        move |ev, win, cx| {
                                                            cx.stop_propagation();
                                                            on_cancel_btn(ev, win, cx);
                                                        },
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .px_3()
                                                    .py_1()
                                                    .rounded_sm()
                                                    .cursor_pointer()
                                                    .bg(theme.bg_hover)
                                                    .text_color(theme.error)
                                                    .hover(|s| s.opacity(0.85))
                                                    .text_xs()
                                                    .child("Don't Save")
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        move |ev, win, cx| {
                                                            cx.stop_propagation();
                                                            on_discard(ev, win, cx);
                                                        },
                                                    ),
                                            )
                                            .child(
                                                div()
                                                    .px_3()
                                                    .py_1()
                                                    .rounded_sm()
                                                    .cursor_pointer()
                                                    .bg(theme.accent)
                                                    .text_color(theme.text_on_accent)
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .hover(|s| s.opacity(0.88))
                                                    .text_xs()
                                                    .child(save_label)
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        move |ev, win, cx| {
                                                            cx.stop_propagation();
                                                            on_save(ev, win, cx);
                                                        },
                                                    ),
                                            ),
                                    ),
                            ),
                    ),
            )
            .into_any_element()
    }
}
