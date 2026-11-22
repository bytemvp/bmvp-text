use crate::core::sidebar::Sidebar;
use crate::platform::window_controls;
use crate::ui::icons::{self, IconName};
use crate::ui::scrollbar;
use crate::ui::theme::Theme;
use crate::ui::{UiAction, UiAction1, UiAction2};
use gpui::*;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub struct SidebarCallbacks {
    pub on_new_tab: UiAction,
    pub on_refresh: UiAction,
    pub on_open_folder: UiAction,
    pub on_toggle_dir: UiAction1<usize>,
    pub on_open_file: UiAction1<PathBuf>,
    pub on_scroll: UiAction2<i32, usize>,
}

pub struct SidebarView;

impl SidebarView {
    pub fn render(
        sidebar: &Sidebar,
        active_file_path: Option<&str>,
        sidebar_width: f32,
        font_family: &str,
        viewport_height: Pixels,
        theme: &Theme,
        callbacks: SidebarCallbacks,
    ) -> AnyElement {
        let ui_font = font_family.to_string();
        // TitleBar (34) + StatusBar (26) + Explorer Header (30) + list padding (8)
        let avail_h = (f32::from(viewport_height) - 34.0 - 26.0 - 30.0 - 8.0).max(24.0);
        let max_visible = (avail_h / 24.0).floor().max(1.0) as usize;
        let max_scroll = sidebar.items.len().saturating_sub(max_visible);
        let scroll_start = sidebar.scroll_offset.min(max_scroll);
        let scroll_end = (scroll_start + max_visible).min(sidebar.items.len());
        let active_path_obj = active_file_path.map(Path::new);

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
            let is_active_file = !is_dir
                && active_path_obj
                    .is_some_and(|ap| ap == path.as_path() || active_file_path == path.to_str());

            let indent_px = 10.0 + (depth as f32) * 14.0;
            let on_toggle_dir = Rc::clone(&callbacks.on_toggle_dir);
            let on_open_file = Rc::clone(&callbacks.on_open_file);

            let mut guides = Vec::with_capacity(depth);
            for d in 0..depth {
                let guide_x = 15.0 + (d as f32) * 14.0;
                guides.push(
                    div()
                        .absolute()
                        .left(px(guide_x))
                        .top_0()
                        .bottom_0()
                        .w(px(1.0))
                        .bg(theme.border_subtle)
                        .into_any_element(),
                );
            }

            let leading_chevron: AnyElement = if is_dir {
                icons::render_icon(
                    if is_expanded {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    },
                    px(11.0),
                    theme.text_muted,
                )
                .into_any_element()
            } else {
                div().w(px(11.0)).flex_shrink_0().into_any_element()
            };

            let type_badge: AnyElement = if is_dir {
                let folder_icon = if is_expanded {
                    IconName::FolderOpen
                } else {
                    IconName::Folder
                };
                let folder_color = if is_expanded {
                    theme.accent
                } else {
                    theme.text_secondary
                };
                div()
                    .w(px(22.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icons::render_icon(folder_icon, px(13.0), folder_color))
                    .into_any_element()
            } else {
                let (badge_label, badge_color) = icons::file_badge_style(&item.name, theme);
                div()
                    .w(px(22.0))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(10.0))
                    .font_family(ui_font.clone())
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(badge_color)
                    .child(badge_label)
                    .into_any_element()
            };

            let mut row = div()
                .relative()
                .h(px(24.0))
                .pl(px(indent_px))
                .pr_2()
                .mx_1()
                .rounded_xs()
                .flex()
                .items_center()
                .gap_1()
                .cursor_pointer()
                .bg(if is_active_file {
                    theme.bg_selected
                } else {
                    theme.bg_surface
                })
                .hover(|s| {
                    s.bg(if is_active_file {
                        theme.bg_selected
                    } else {
                        theme.bg_hover
                    })
                })
                .text_color(if is_active_file || is_dir {
                    theme.text_primary
                } else {
                    theme.text_secondary
                })
                .font_weight(if is_active_file {
                    FontWeight::MEDIUM
                } else {
                    FontWeight::NORMAL
                })
                .text_xs()
                .children(guides);

            if is_active_file {
                row = row.child(
                    div()
                        .absolute()
                        .left_0()
                        .top(px(4.0))
                        .bottom(px(4.0))
                        .w(px(2.0))
                        .rounded_r_xs()
                        .bg(theme.accent),
                );
            }

            let item_div = row
                .child(leading_chevron)
                .child(type_badge)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .child(item.name.clone()),
                )
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

        let sidebar_scrollbar = scrollbar::render_vertical_scrollbar(
            sidebar.items.len(),
            max_visible,
            scroll_start,
            avail_h,
            theme,
            None,
        );

        let sidebar_body = if file_items.is_empty() {
            div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .px_4()
                .gap_3()
                .child(icons::render_icon(
                    IconName::FolderOpen,
                    px(24.0),
                    theme.text_muted,
                ))
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.text_secondary)
                        .text_center()
                        .child("Open a project folder or start a new file"),
                )
                .child(
                    div()
                        .px_3()
                        .py_1p5()
                        .rounded_sm()
                        .cursor_pointer()
                        .bg(theme.accent)
                        .text_color(theme.text_on_accent)
                        .hover(|s| s.opacity(0.88))
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_xs()
                        .font_weight(FontWeight::MEDIUM)
                        .child("Open Project")
                        .child(
                            div()
                                .text_size(px(10.5))
                                .opacity(0.8)
                                .child(format!("{}+O", window_controls::PRIMARY_MODIFIER_LABEL)),
                        )
                        .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                            cx.stop_propagation();
                            on_open_folder_empty(win, cx);
                        }),
                )
                .child(
                    div()
                        .px_3()
                        .py_1()
                        .rounded_sm()
                        .cursor_pointer()
                        .bg(theme.bg_elevated)
                        .border_1()
                        .border_color(theme.border_subtle)
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
                .min_h_0()
                .flex()
                .flex_row()
                .overflow_hidden()
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
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .py_1()
                        .overflow_hidden()
                        .children(file_items),
                )
                .children(sidebar_scrollbar)
                .into_any_element()
        };

        let on_new_tab = callbacks.on_new_tab;
        let on_refresh = callbacks.on_refresh;
        let on_open_folder = callbacks.on_open_folder;

        let header = div()
            .h(px(30.0))
            .px_3()
            .border_b_1()
            .border_color(theme.border_subtle)
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .text_size(px(11.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme.text_muted)
                    .child("EXPLORER"),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .w(px(22.0))
                            .h(px(22.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover))
                            .child(icons::render_icon(
                                IconName::FilePlus,
                                px(13.0),
                                theme.text_secondary,
                            ))
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_new_tab(win, cx);
                            }),
                    )
                    .child(
                        div()
                            .w(px(22.0))
                            .h(px(22.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover))
                            .child(icons::render_icon(
                                IconName::Refresh,
                                px(13.0),
                                theme.text_secondary,
                            ))
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_refresh(win, cx);
                            }),
                    )
                    .child(
                        div()
                            .w(px(22.0))
                            .h(px(22.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover))
                            .child(icons::render_icon(
                                IconName::FolderOpen,
                                px(13.0),
                                theme.text_secondary,
                            ))
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_open_folder(win, cx);
                            }),
                    ),
            )
            .into_any_element();

        div()
            .w(px(sidebar_width))
            .h_full()
            .bg(theme.bg_surface)
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(header)
            .child(sidebar_body)
            .into_any_element()
    }
}
