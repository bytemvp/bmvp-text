use crate::ui::icons::{self, IconName};
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

        let nav_arrows = if tab_count > 1 {
            Some(
                div()
                    .h_full()
                    .px_1p5()
                    .border_b_1()
                    .border_r_1()
                    .border_color(theme.border_subtle)
                    .flex()
                    .items_center()
                    .gap_0p5()
                    .child(
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover))
                            .child(icons::render_icon(
                                IconName::ChevronLeft,
                                px(12.0),
                                if can_prev {
                                    theme.text_secondary
                                } else {
                                    theme.text_muted
                                },
                            ))
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
                            .hover(|s| s.bg(theme.bg_hover))
                            .child(icons::render_icon(
                                IconName::ChevronRight,
                                px(12.0),
                                if can_next {
                                    theme.text_secondary
                                } else {
                                    theme.text_muted
                                },
                            ))
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_next(win, cx);
                            }),
                    )
                    .into_any_element(),
            )
        } else {
            None
        };

        let mut tab_items: Vec<AnyElement> = Vec::with_capacity(tab_count);
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

            let (badge_label, badge_color) = icons::file_badge_style(&tab.title, theme);
            let on_select = Rc::clone(&callbacks.on_select_tab);
            let on_close_left = Rc::clone(&callbacks.on_close_tab);
            let on_close_mid = Rc::clone(&callbacks.on_close_tab);

            let mut tab_btn = div()
                .relative()
                .h_full()
                .px_3()
                .flex()
                .items_center()
                .gap_1p5()
                .flex_shrink_0()
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
                    .text_color(theme.text_primary)
                })
                .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                    cx.stop_propagation();
                    on_select(idx, win, cx);
                })
                .on_mouse_down(MouseButton::Middle, move |_, win, cx| {
                    cx.stop_propagation();
                    on_close_mid(idx, win, cx);
                });

            if is_active {
                tab_btn = tab_btn.border_r_1().border_color(theme.border_subtle);
                if idx > 0 {
                    tab_btn = tab_btn.border_l_1();
                }
                tab_btn = tab_btn.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .h(px(2.0))
                        .bg(theme.accent),
                );
            } else {
                tab_btn = tab_btn
                    .border_b_1()
                    .border_r_1()
                    .border_color(theme.border_subtle);
            }

            tab_btn = tab_btn
                .child(
                    div()
                        .text_size(px(10.0))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(badge_color)
                        .child(badge_label),
                )
                .child(
                    div()
                        .max_w(px(160.0))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(tab.title.clone()),
                );

            if tab.is_dirty {
                tab_btn = tab_btn.child(
                    div()
                        .w(px(6.0))
                        .h(px(6.0))
                        .rounded_full()
                        .bg(theme.dirty_indicator),
                );
            }

            tab_btn = tab_btn.child(
                div()
                    .w(px(16.0))
                    .h(px(16.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_xs()
                    .hover(|s| s.bg(theme.bg_hover))
                    .child(
                        icons::render_icon(IconName::Close, px(11.0), theme.text_muted)
                            .hover(|s| s.text_color(theme.error)),
                    )
                    .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                        cx.stop_propagation();
                        on_close_left(idx, win, cx);
                    }),
            );

            tab_items.push(tab_btn.into_any_element());
        }

        div()
            .h(px(34.0))
            .w_full()
            .bg(theme.bg_tab_bar)
            .flex()
            .items_center()
            .overflow_hidden()
            .children(nav_arrows)
            .child(
                div()
                    .flex()
                    .items_center()
                    .h_full()
                    .overflow_hidden()
                    .children(tab_items),
            )
            .child(
                div()
                    .h_full()
                    .px_2p5()
                    .border_b_1()
                    .border_color(theme.border_subtle)
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.bg_hover))
                    .child(
                        icons::render_icon(IconName::Plus, px(13.0), theme.text_muted)
                            .hover(|s| s.text_color(theme.text_primary)),
                    )
                    .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                        cx.stop_propagation();
                        on_new(win, cx);
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .border_b_1()
                    .border_color(theme.border_subtle),
            )
            .into_any_element()
    }
}
