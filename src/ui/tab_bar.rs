use crate::ui::theme::Theme;
use crate::ui::{UiAction, UiAction1};
use gpui::*;
use std::rc::Rc;

pub struct TabInfo {
    pub title: String,
    pub is_dirty: bool,
    pub is_active: bool,
}

pub struct TabBarCallbacks {
    pub on_prev_tab: UiAction,
    pub on_next_tab: UiAction,
    pub on_select_tab: UiAction1<usize>,
    pub on_close_tab: UiAction1<usize>,
    pub on_new_tab: UiAction,
}

pub struct TabBar;

impl TabBar {
    pub fn render(
        tabs: &[TabInfo],
        active_tab: usize,
        theme: &Theme,
        callbacks: TabBarCallbacks,
    ) -> AnyElement {
        let tab_count = tabs.len();
        let can_prev = active_tab > 0;
        let can_next = active_tab + 1 < tab_count;

        let on_prev = callbacks.on_prev_tab;
        let on_next = callbacks.on_next_tab;
        let on_new = callbacks.on_new_tab;

        let mut tab_row_items: Vec<AnyElement> = Vec::new();

        tab_row_items.push(
            div()
                .h_full()
                .px_2()
                .border_b_1()
                .border_color(theme.border_subtle)
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .w(px(20.0))
                        .h(px(20.0))
                        .rounded_xs()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .text_xs()
                        .text_color(if can_prev {
                            theme.text_secondary
                        } else {
                            theme.text_muted
                        })
                        .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                        .child("←")
                        .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                            cx.stop_propagation();
                            on_prev(win, cx);
                        }),
                )
                .child(
                    div()
                        .w(px(20.0))
                        .h(px(20.0))
                        .rounded_xs()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .text_xs()
                        .text_color(if can_next {
                            theme.text_secondary
                        } else {
                            theme.text_muted
                        })
                        .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                        .child("→")
                        .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                            cx.stop_propagation();
                            on_next(win, cx);
                        }),
                )
                .into_any_element(),
        );

        for (idx, tab) in tabs.iter().enumerate() {
            let is_active = tab.is_active;
            let tab_bg = if is_active {
                theme.bg_tab_active
            } else {
                theme.bg_tab_inactive
            };
            let tab_fg = if is_active {
                theme.text_primary
            } else {
                theme.text_secondary
            };

            let on_select = Rc::clone(&callbacks.on_select_tab);
            let on_close = Rc::clone(&callbacks.on_close_tab);

            let mut tab_btn = div()
                .h_full()
                .px_3()
                .flex()
                .items_center()
                .gap_2()
                .bg(tab_bg)
                .text_color(tab_fg)
                .text_xs()
                .font_weight(if is_active {
                    FontWeight::MEDIUM
                } else {
                    FontWeight::NORMAL
                })
                .cursor_pointer()
                .hover(|s| {
                    s.bg(if is_active {
                        theme.bg_tab_active
                    } else {
                        theme.bg_hover
                    })
                })
                .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                    cx.stop_propagation();
                    on_select(idx, win, cx);
                });

            if is_active {
                tab_btn = tab_btn
                    .border_t_2()
                    .border_l_1()
                    .border_r_1()
                    .border_color(theme.accent);
            } else {
                tab_btn = tab_btn
                    .border_b_1()
                    .border_r_1()
                    .border_color(theme.border_subtle);
            }

            tab_btn = tab_btn.child(tab.title.clone());

            if tab.is_dirty {
                tab_btn = tab_btn.child(
                    div()
                        .w_1p5()
                        .h_1p5()
                        .rounded_full()
                        .bg(theme.dirty_indicator),
                );
            }

            tab_btn = tab_btn.child(
                div()
                    .w_4()
                    .h_4()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_xs()
                    .text_color(theme.text_muted)
                    .hover(|s| s.bg(theme.bg_hover).text_color(theme.error))
                    .child("×")
                    .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                        cx.stop_propagation();
                        on_close(idx, win, cx);
                    }),
            );

            tab_row_items.push(tab_btn.into_any_element());
        }

        tab_row_items.push(
            div()
                .h_full()
                .px_2p5()
                .border_b_1()
                .border_color(theme.border_subtle)
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .text_xs()
                .text_color(theme.text_muted)
                .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                .child("+")
                .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                    cx.stop_propagation();
                    on_new(win, cx);
                })
                .into_any_element(),
        );

        tab_row_items.push(
            div()
                .flex_1()
                .h_full()
                .border_b_1()
                .border_color(theme.border_subtle)
                .into_any_element(),
        );

        div()
            .h(px(32.0))
            .w_full()
            .bg(theme.bg_tab_bar)
            .flex()
            .items_center()
            .children(tab_row_items)
            .into_any_element()
    }
}
