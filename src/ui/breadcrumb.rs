use crate::ui::UiAction;
use crate::ui::icons::{self, IconName};
use crate::ui::theme::Theme;
use gpui::*;
use std::path::Path;

pub struct BreadcrumbCallbacks {
    pub on_open_find: UiAction,
    pub on_toggle_line_numbers: UiAction,
}

pub struct BreadcrumbBar;

impl BreadcrumbBar {
    pub fn format_path(file_path: Option<&str>, root_path: Option<&Path>) -> String {
        if let Some(p) = file_path {
            if let Some(root) = root_path {
                if let Ok(rel) = Path::new(p).strip_prefix(root) {
                    return rel.to_string_lossy().replace(['\\', '/'], " › ");
                }
                let root_str = root.to_string_lossy();
                if p.contains('\\') || root_str.contains('\\') {
                    let norm_p = p.replace('\\', "/");
                    let norm_root = root_str.replace('\\', "/");
                    if let Ok(rel) = Path::new(&norm_p).strip_prefix(Path::new(&norm_root)) {
                        return rel.to_string_lossy().replace(['\\', '/'], " › ");
                    }
                }
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
        theme: &Theme,
        callbacks: BreadcrumbCallbacks,
    ) -> AnyElement {
        let on_find = callbacks.on_open_find;
        let on_line_nums = callbacks.on_toggle_line_numbers;

        let segments: Vec<&str> = path_display
            .split(" › ")
            .filter(|s| !s.is_empty())
            .collect();
        let seg_count = segments.len();
        let mut crumb_items: Vec<AnyElement> = Vec::new();
        for (idx, seg) in segments.into_iter().enumerate() {
            if idx > 0 {
                crumb_items.push(
                    icons::render_icon(IconName::ChevronRight, px(10.0), theme.text_muted)
                        .into_any_element(),
                );
            }
            let is_last = idx + 1 == seg_count;
            crumb_items.push(
                div()
                    .text_size(px(11.5))
                    .text_color(if is_last {
                        theme.text_primary
                    } else {
                        theme.text_muted
                    })
                    .font_weight(if is_last {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .child(seg.to_string())
                    .into_any_element(),
            );
        }

        div()
            .h(px(26.0))
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
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .gap_1()
                    .children(crumb_items),
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
                            .bg(if find_is_open {
                                theme.bg_hover
                            } else {
                                theme.bg_editor
                            })
                            .hover(|s| s.bg(theme.bg_hover))
                            .child(icons::render_icon(
                                IconName::Search,
                                px(12.0),
                                if find_is_open {
                                    theme.accent
                                } else {
                                    theme.text_secondary
                                },
                            ))
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
                            .bg(if show_line_numbers {
                                theme.bg_hover
                            } else {
                                theme.bg_editor
                            })
                            .hover(|s| s.bg(theme.bg_hover))
                            .child(icons::render_icon(
                                IconName::Hash,
                                px(12.0),
                                if show_line_numbers {
                                    theme.text_primary
                                } else {
                                    theme.text_muted
                                },
                            ))
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_line_nums(win, cx);
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
        assert_eq!(
            BreadcrumbBar::format_path(Some(r"C:\other\file.rs"), Some(root_win)),
            "C: › other › file.rs"
        );
        assert_eq!(
            BreadcrumbBar::format_path(
                Some(r"C:\work\bmvp-text/src/main.rs"),
                Some(Path::new(r"C:\work\bmvp-text\"))
            ),
            "src › main.rs"
        );
    }
}
