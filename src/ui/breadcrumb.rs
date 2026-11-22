use crate::ui::UiAction;
use crate::ui::theme::Theme;
use gpui::*;
use std::path::Path;

pub struct BreadcrumbCallbacks {
    pub on_open_find: UiAction,
    pub on_toggle_line_numbers: UiAction,
    pub on_toggle_terminal: UiAction,
    pub on_toggle_settings: UiAction,
}

pub struct BreadcrumbBar;

impl BreadcrumbBar {
    pub fn format_path(file_path: Option<&str>, root_path: Option<&Path>) -> String {
        if let Some(p) = file_path {
            if let Some(root) = root_path
                && let Ok(rel) = Path::new(p).strip_prefix(root)
            {
                return rel.to_string_lossy().replace(['\\', '/'], " › ");
            }
            p.replace(['\\', '/'], " › ")
        } else {
            "untitled".to_string()
        }
    }

    pub fn render(
        path_display: String,
        find_is_open: bool,
        show_line_numbers: bool,
        show_terminal: bool,
        theme: &Theme,
        callbacks: BreadcrumbCallbacks,
    ) -> AnyElement {
        let on_find = callbacks.on_open_find;
        let on_line_nums = callbacks.on_toggle_line_numbers;
        let on_terminal = callbacks.on_toggle_terminal;
        let on_settings = callbacks.on_toggle_settings;

        div()
            .h(px(28.0))
            .w_full()
            .bg(theme.bg_editor)
            .border_b_1()
            .border_color(theme.border_subtle)
            .flex()
            .items_center()
            .justify_between()
            .px_3()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .child(path_display),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .w(px(22.0))
                            .h(px(20.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .text_color(if find_is_open {
                                theme.accent
                            } else {
                                theme.text_secondary
                            })
                            .text_xs()
                            .child("⌕")
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_find(win, cx);
                            }),
                    )
                    .child(
                        div()
                            .w(px(22.0))
                            .h(px(20.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .text_color(if show_line_numbers {
                                theme.text_primary
                            } else {
                                theme.text_muted
                            })
                            .text_xs()
                            .child("#")
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_line_nums(win, cx);
                            }),
                    )
                    .child(
                        div()
                            .w(px(22.0))
                            .h(px(20.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .text_color(if show_terminal {
                                theme.accent
                            } else {
                                theme.text_secondary
                            })
                            .text_xs()
                            .child(">_")
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_terminal(win, cx);
                            }),
                    )
                    .child(
                        div()
                            .w(px(22.0))
                            .h(px(20.0))
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
                    ),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::BreadcrumbBar;
    use std::path::Path;

    #[test]
    fn test_breadcrumb_format_path_windows_and_posix() {
        assert_eq!(BreadcrumbBar::format_path(None, None), "untitled");

        let root_win = Path::new(r"C:\work\bmvp-text");
        let file_win = r"C:\work\bmvp-text\src\ui\editor_view.rs";
        assert_eq!(
            BreadcrumbBar::format_path(Some(file_win), Some(root_win)),
            "src › ui › editor_view.rs"
        );

        let root_posix = Path::new("/Users/alice/bmvp-text");
        let file_posix = "/Users/alice/bmvp-text/src/core/logging.rs";
        assert_eq!(
            BreadcrumbBar::format_path(Some(file_posix), Some(root_posix)),
            "src › core › logging.rs"
        );

        assert_eq!(
            BreadcrumbBar::format_path(Some("/etc/hosts"), Some(root_posix)),
            " › etc › hosts"
        );
    }
}
