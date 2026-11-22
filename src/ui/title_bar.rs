use crate::platform::window_controls;
use crate::ui::theme::Theme;
use gpui::*;
use std::rc::Rc;

pub type MouseClickHandler = Rc<dyn Fn(&MouseDownEvent, &mut Window, &mut App) + 'static>;

pub struct TitleBarCallbacks {
    pub on_toggle_sidebar: MouseClickHandler,
    pub on_open_folder: MouseClickHandler,
    pub on_drag_area_mouse_down: MouseClickHandler,
    pub on_open_finder: MouseClickHandler,
    pub on_toggle_terminal: MouseClickHandler,
    pub on_toggle_settings: MouseClickHandler,
    pub on_quit: MouseClickHandler,
}

pub struct TitleBar;

impl TitleBar {
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        project_name: &str,
        is_maximized: bool,
        show_sidebar: bool,
        show_terminal: bool,
        show_settings: bool,
        theme: &Theme,
        callbacks: TitleBarCallbacks,
    ) -> AnyElement {
        let max_icon = if is_maximized { "⧉" } else { "□" };
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
                            .w(px(38.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.text_secondary)
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .on_mouse_down(MouseButton::Left, |_, win, cx| {
                                window_controls::minimize_window(win);
                                cx.stop_propagation();
                            })
                            .child("─"),
                    )
                    .child(
                        div()
                            .w(px(38.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.text_secondary)
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .on_mouse_down(MouseButton::Left, |_, win, cx| {
                                window_controls::toggle_maximize(win);
                                cx.stop_propagation();
                            })
                            .child(max_icon),
                    )
                    .child(
                        div()
                            .w(px(42.0))
                            .h_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.text_secondary)
                            .hover(|s| s.bg(rgb(0xe81123)).text_color(rgb(0xffffff)))
                            .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
                                cx.stop_propagation();
                                on_quit(ev, win, cx);
                            })
                            .child("✕"),
                    ),
            )
        };

        let on_toggle_sidebar = callbacks.on_toggle_sidebar;
        let on_open_folder = callbacks.on_open_folder;
        let on_drag_area = callbacks.on_drag_area_mouse_down;
        let on_open_finder = callbacks.on_open_finder;
        let on_toggle_terminal = callbacks.on_toggle_terminal;
        let on_toggle_settings = callbacks.on_toggle_settings;

        let mut bar = div().h(px(32.0)).w_full().bg(theme.bg_activity_bar);
        if window_controls::USES_MACOS_TRAFFIC_LIGHTS && !show_sidebar {
            bar = bar.pl(px(window_controls::MACOS_TRAFFIC_LIGHT_PADDING_PX));
        } else {
            bar = bar.pl_2p5();
        }
        if window_controls::USES_MACOS_TRAFFIC_LIGHTS {
            bar = bar.pr_2();
        } else {
            bar = bar.pr_0();
        }

        bar.flex()
            .items_center()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .child(
                        div()
                            .w(px(22.0))
                            .h(px(22.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .text_xs()
                            .text_color(theme.text_secondary)
                            .child("≡")
                            .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
                                cx.stop_propagation();
                                on_toggle_sidebar(ev, win, cx);
                            }),
                    )
                    .child(
                        div()
                            .px_1p5()
                            .py_0p5()
                            .rounded_xs()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover))
                            .text_xs()
                            .text_color(theme.text_primary)
                            .child(project_name.to_string())
                            .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
                                cx.stop_propagation();
                                on_open_folder(ev, win, cx);
                            }),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .min_w(px(20.0))
                    .cursor_default()
                    .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
                        cx.stop_propagation();
                        on_drag_area(ev, win, cx);
                    }),
            )
            .child(
                div()
                    .flex()
                    .h_full()
                    .items_center()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_1()
                            .pr_1()
                            .child(
                                div()
                                    .w(px(24.0))
                                    .h(px(22.0))
                                    .rounded_xs()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                                    .text_xs()
                                    .text_color(theme.text_secondary)
                                    .child("⌕")
                                    .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
                                        cx.stop_propagation();
                                        on_open_finder(ev, win, cx);
                                    }),
                            )
                            .child(
                                div()
                                    .w(px(24.0))
                                    .h(px(22.0))
                                    .rounded_xs()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                                    .text_xs()
                                    .text_color(if show_terminal {
                                        theme.accent
                                    } else {
                                        theme.text_secondary
                                    })
                                    .child(">_")
                                    .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
                                        cx.stop_propagation();
                                        on_toggle_terminal(ev, win, cx);
                                    }),
                            )
                            .child(
                                div()
                                    .w(px(24.0))
                                    .h(px(22.0))
                                    .rounded_xs()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .cursor_pointer()
                                    .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                                    .text_xs()
                                    .text_color(if show_settings {
                                        theme.accent
                                    } else {
                                        theme.text_secondary
                                    })
                                    .child("⚙")
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
