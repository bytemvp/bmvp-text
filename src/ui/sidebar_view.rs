use crate::core::sidebar::Sidebar;
use crate::platform::window_controls;
use crate::ui::theme::Theme;
use crate::ui::{UiAction, UiAction1, UiAction2, UiMouseAction};
use gpui::*;
use std::path::PathBuf;
use std::rc::Rc;

pub struct SidebarCallbacks {
    pub on_header_mouse_down: UiMouseAction,
    pub on_new_tab: UiAction,
    pub on_refresh: UiAction,
    pub on_open_folder: UiAction,
    pub on_toggle_dir: UiAction1<usize>,
    pub on_open_file: UiAction1<PathBuf>,
    pub on_scroll: UiAction2<i32, usize>,
    pub on_hide_sidebar: UiAction,
    pub on_open_settings: UiAction,
}

pub struct SidebarView;

impl SidebarView {
    pub fn render(
        sidebar: &Sidebar,
        project_name: &str,
        sidebar_width: f32,
        font_family: &str,
        viewport_height: Pixels,
        theme: &Theme,
        callbacks: SidebarCallbacks,
    ) -> AnyElement {
        let ui_font = font_family.to_string();
        let avail_h = (f32::from(viewport_height) - 64.0).max(24.0);
        let max_visible = (avail_h / 24.0).floor().max(1.0) as usize;
        let max_scroll = sidebar.items.len().saturating_sub(max_visible);
        let scroll_start = sidebar.scroll_offset.min(max_scroll);
        let scroll_end = (scroll_start + max_visible).min(sidebar.items.len());

        let mut file_items = Vec::new();
        for (idx, item) in sidebar.items[scroll_start..scroll_end]
            .iter()
            .enumerate()
            .map(|(offset, item)| (scroll_start + offset, item))
        {
            let path = item.path.clone();
            let is_dir = item.is_dir;
            let is_expanded = item.is_expanded;
            let depth = item.depth;
            let icon = Sidebar::file_icon(is_dir, is_expanded, &item.name);

            let indent_px = 10.0 + (depth as f32) * 12.0;
            let arrow = if is_dir {
                if is_expanded { "⌄ " } else { "› " }
            } else {
                "  "
            };

            let on_toggle_dir = Rc::clone(&callbacks.on_toggle_dir);
            let on_open_file = Rc::clone(&callbacks.on_open_file);

            let item_div = div()
                .h_6()
                .pl(px(indent_px))
                .pr_2()
                .mx_1p5()
                .rounded_xs()
                .flex()
                .items_center()
                .gap_1p5()
                .cursor_pointer()
                .hover(|s| s.bg(theme.bg_hover))
                .text_color(if is_dir {
                    theme.text_primary
                } else {
                    theme.text_secondary
                })
                .text_xs()
                .child(
                    div()
                        .text_color(theme.text_muted)
                        .font_family(ui_font.clone())
                        .child(arrow),
                )
                .child(
                    div()
                        .w(px(20.0))
                        .text_xs()
                        .text_color(if is_dir {
                            theme.accent
                        } else {
                            theme.text_muted
                        })
                        .font_family(ui_font.clone())
                        .child(icon),
                )
                .child(div().child(item.name.clone()))
                .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                    cx.stop_propagation();
                    if is_dir {
                        on_toggle_dir(idx, win, cx);
                    } else {
                        on_open_file(path.clone(), win, cx);
                    }
                })
                .into_any_element();

            file_items.push(item_div);
        }

        let on_open_folder_empty = Rc::clone(&callbacks.on_open_folder);
        let on_new_tab_empty = Rc::clone(&callbacks.on_new_tab);
        let on_scroll = callbacks.on_scroll;

        let sidebar_body = if file_items.is_empty() {
            div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .px_4()
                .gap_2p5()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.text_secondary)
                        .text_center()
                        .child("Choose an option below to open files"),
                )
                .child(
                    div()
                        .px_3()
                        .py_1()
                        .rounded_xs()
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.bg_hover))
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_xs()
                        .child(div().text_color(theme.text_primary).child("Open Project"))
                        .child(
                            div()
                                .text_color(theme.text_muted)
                                .child(format!("{}+O", window_controls::PRIMARY_MODIFIER_LABEL)),
                        )
                        .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                            cx.stop_propagation();
                            on_open_folder_empty(win, cx);
                        }),
                )
                .child(
                    div()
                        .w(px(140.0))
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().flex_1().h(px(1.0)).bg(theme.border_subtle))
                        .child(div().text_xs().text_color(theme.text_muted).child("or"))
                        .child(div().flex_1().h(px(1.0)).bg(theme.border_subtle)),
                )
                .child(
                    div()
                        .px_3()
                        .py_1()
                        .rounded_xs()
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.bg_hover))
                        .text_xs()
                        .text_color(theme.text_primary)
                        .child("New Untitled File")
                        .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                            cx.stop_propagation();
                            on_new_tab_empty(win, cx);
                        }),
                )
                .into_any_element()
        } else {
            div()
                .flex_1()
                .overflow_hidden()
                .py_1()
                .on_scroll_wheel(move |event: &ScrollWheelEvent, win, cx| {
                    let delta_y = match event.delta {
                        ScrollDelta::Pixels(p) => (p.y / px(20.0)) as i32,
                        ScrollDelta::Lines(l) => l.y as i32,
                    };
                    if delta_y != 0 {
                        on_scroll(delta_y, max_visible, win, cx);
                    }
                    cx.stop_propagation();
                })
                .children(file_items)
                .into_any_element()
        };

        let on_header_drag_1 = Rc::clone(&callbacks.on_header_mouse_down);
        let on_header_drag_2 = callbacks.on_header_mouse_down;
        let on_new_tab = callbacks.on_new_tab;
        let on_refresh = callbacks.on_refresh;
        let on_open_folder = callbacks.on_open_folder;
        let on_hide = callbacks.on_hide_sidebar;
        let on_settings = callbacks.on_open_settings;

        let mut header = div().h(px(32.0)).pr_3();
        if window_controls::USES_MACOS_TRAFFIC_LIGHTS {
            header = header.pl(px(window_controls::MACOS_TRAFFIC_LIGHT_PADDING_PX));
        } else {
            header = header.pl_3();
        }
        let header = header
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .h_full()
                    .cursor_default()
                    .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
                        cx.stop_propagation();
                        on_header_drag_1(ev, win, cx);
                    })
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text_primary)
                            .child(project_name.to_string()),
                    ),
            )
            .child(div().flex_1().h_full().cursor_default().on_mouse_down(
                MouseButton::Left,
                move |ev, win, cx| {
                    cx.stop_propagation();
                    on_header_drag_2(ev, win, cx);
                },
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .w_5()
                            .h_5()
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .child("+")
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_new_tab(win, cx);
                            }),
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
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .child("⟳")
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_refresh(win, cx);
                            }),
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
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .child("📂")
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_open_folder(win, cx);
                            }),
                    ),
            )
            .into_any_element();

        let footer = div()
            .h(px(26.0))
            .px_2p5()
            .bg(theme.bg_surface)
            .border_t_1()
            .border_color(theme.border_subtle)
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .w_5()
                    .h_5()
                    .rounded_xs()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                    .text_color(theme.text_secondary)
                    .text_xs()
                    .child("◧")
                    .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                        cx.stop_propagation();
                        on_hide(win, cx);
                    }),
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
                    .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                    .text_color(theme.text_secondary)
                    .text_xs()
                    .child("⚙")
                    .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                        cx.stop_propagation();
                        on_settings(win, cx);
                    }),
            )
            .into_any_element();

        div()
            .w(px(sidebar_width))
            .h_full()
            .bg(theme.bg_surface)
            .flex()
            .flex_col()
            .child(header)
            .child(sidebar_body)
            .child(footer)
            .into_any_element()
    }
}
