use crate::platform::window_controls;
use crate::ui::icons::{self, IconName};
use crate::ui::theme::Theme;
use gpui::*;
use std::rc::Rc;

pub type MouseClickHandler = Rc<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static>;

pub struct TitleBarCallbacks {
    pub on_toggle_sidebar: MouseClickHandler,
    pub on_open_folder: MouseClickHandler,
    pub on_drag_area_mouse_down: MouseClickHandler,
    pub on_open_finder: MouseClickHandler,
    pub on_toggle_settings: MouseClickHandler,
    pub on_quit: MouseClickHandler,
}

pub struct TitleBar;

impl TitleBar {
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        project_name: &str,
        git_branch: Option<&str>,
        is_maximized: bool,
        show_sidebar: bool,
        show_settings: bool,
        theme: &Theme,
        callbacks: TitleBarCallbacks,
    ) -> AnyElement {
        let max_icon = if is_maximized {
            IconName::WindowRestore
        } else {
            IconName::WindowMaximize
        };
        let on_quit = callbacks.on_quit;

        let window_controls_row = if window_controls::USES_MACOS_TRAFFIC_LIGHTS {
            None
        } else {
            Some(
                div()
                    .flex()
                    .h_full()
                    .items_center()
                    .child(
                        div()
                            .w(px(40.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover))
                            .on_mouse_down(MouseButton::Left, |_, win, cx| {
                                window_controls::minimize_window(win);
                                cx.stop_propagation();
                            })
                            .child(icons::render_icon(
                                IconName::WindowMinimize,
                                px(13.0),
                                theme.text_secondary,
                            )),
                    )
                    .child(
                        div()
                            .w(px(40.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover))
                            .on_mouse_down(MouseButton::Left, |_, win, cx| {
                                window_controls::toggle_maximize(win);
                                cx.stop_propagation();
                            })
                            .child(icons::render_icon(max_icon, px(12.0), theme.text_secondary)),
                    )
                    .child(
                        div()
                            .group("win_close")
                            .w(px(44.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.window_close_hover))
                            .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
                                cx.stop_propagation();
                                on_quit(ev, win, cx);
                            })
                            .child(
                                icons::render_icon(
                                    IconName::WindowClose,
                                    px(13.0),
                                    theme.text_secondary,
                                )
                                .group_hover("win_close", |s| s.text_color(theme.text_on_error)),
                            ),
                    ),
            )
        };

        let on_toggle_sidebar = callbacks.on_toggle_sidebar;
        let on_open_folder = callbacks.on_open_folder;
        let on_drag_area = callbacks.on_drag_area_mouse_down;
        let on_open_finder = callbacks.on_open_finder;
        let on_toggle_settings = callbacks.on_toggle_settings;

        let mut bar = div()
            .h(px(34.0))
            .w_full()
            .bg(theme.bg_activity_bar)
            .border_b_1()
            .border_color(theme.border_subtle);
        if window_controls::USES_MACOS_TRAFFIC_LIGHTS {
            bar = bar
                .pl(px(window_controls::MACOS_TRAFFIC_LIGHT_PADDING_PX))
                .pr_2();
        } else {
            bar = bar.pl_2p5().pr_0();
        }

        let kbd_shortcut = format!("{}+P", window_controls::PRIMARY_MODIFIER_LABEL);

        bar.flex()
            .items_center()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .child(
                        div()
                            .w(px(24.0))
                            .h(px(24.0))
                            .rounded_sm()
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
                                px(14.0),
                                if show_sidebar {
                                    theme.accent
                                } else {
                                    theme.text_secondary
                                },
                            ))
                            .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
                                cx.stop_propagation();
                                on_toggle_sidebar(ev, win, cx);
                            }),
                    )
                    .child(
                        div()
                            .h(px(24.0))
                            .px_2()
                            .rounded_sm()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover))
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text_primary)
                            .child(icons::render_icon(
                                IconName::Folder,
                                px(13.0),
                                theme.text_secondary,
                            ))
                            .child(project_name.to_string())
                            .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
                                cx.stop_propagation();
                                on_open_folder(ev, win, cx);
                            }),
                    )
                    .children(git_branch.map(|branch| {
                        div()
                            .h(px(20.0))
                            .px_2()
                            .rounded_sm()
                            .bg(theme.bg_elevated)
                            .border_1()
                            .border_color(theme.border_subtle)
                            .flex()
                            .items_center()
                            .gap_1()
                            .text_size(px(11.0))
                            .text_color(theme.text_secondary)
                            .child(icons::render_icon(
                                IconName::GitBranch,
                                px(12.0),
                                theme.accent,
                            ))
                            .child(branch.to_string())
                    })),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .min_w(px(24.0))
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_default()
                    .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
                        cx.stop_propagation();
                        on_drag_area(ev, win, cx);
                    })
                    .child(
                        div()
                            .w(px(280.0))
                            .max_w_full()
                            .h(px(22.0))
                            .px_2p5()
                            .rounded_md()
                            .bg(theme.bg_elevated)
                            .border_1()
                            .border_color(theme.border_subtle)
                            .hover(|s| s.border_color(theme.border).bg(theme.bg_hover))
                            .cursor_pointer()
                            .flex()
                            .items_center()
                            .justify_between()
                            .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
                                cx.stop_propagation();
                                on_open_finder(ev, win, cx);
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1p5()
                                    .child(icons::render_icon(
                                        IconName::Search,
                                        px(12.0),
                                        theme.text_muted,
                                    ))
                                    .child(
                                        div()
                                            .text_size(px(11.5))
                                            .text_color(theme.text_muted)
                                            .child("Search files..."),
                                    ),
                            )
                            .child(
                                div()
                                    .px_1p5()
                                    .py(px(1.0))
                                    .rounded_xs()
                                    .bg(theme.bg_surface)
                                    .text_size(px(10.5))
                                    .text_color(theme.text_muted)
                                    .child(kbd_shortcut),
                            ),
                    ),
            )
            .child(
                div()
                    .flex()
                    .h_full()
                    .items_center()
                    .child(
                        div().flex().items_center().gap_1().pr_1p5().child(
                            div()
                                .w(px(24.0))
                                .h(px(24.0))
                                .rounded_sm()
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .bg(if show_settings {
                                    theme.bg_hover
                                } else {
                                    theme.bg_activity_bar
                                })
                                .hover(|s| s.bg(theme.bg_hover))
                                .child(icons::render_icon(
                                    IconName::Settings,
                                    px(14.0),
                                    if show_settings {
                                        theme.accent
                                    } else {
                                        theme.text_secondary
                                    },
                                ))
                                .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
                                    cx.stop_propagation();
                                    on_toggle_settings(ev, win, cx);
                                }),
                        ),
                    )
                    .children(window_controls_row),
            )
            .into_any_element()
    }
}
