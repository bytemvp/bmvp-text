use crate::core::buffer::SearchOptions;
use crate::ui::theme::Theme;
use crate::ui::{UiAction, UiAction1};
use gpui::*;
use std::ops::Range;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindField {
    Query,
    Replace,
}

pub struct FindReplaceCallbacks {
    pub on_toggle_replace: UiAction,
    pub on_focus_field: UiAction1<FindField>,
    pub on_toggle_case_sensitive: UiAction,
    pub on_toggle_whole_word: UiAction,
    pub on_toggle_regex: UiAction,
    pub on_prev_match: UiAction,
    pub on_next_match: UiAction,
    pub on_close: UiAction,
    pub on_replace_current: UiAction,
    pub on_replace_all: UiAction,
}

pub struct FindState {
    pub is_open: bool,
    pub show_replace: bool,
    pub query: String,
    pub replace_query: String,
    pub active_field: FindField,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
    pub regex_error: Option<String>,
    pub matches: Vec<Range<usize>>,
    pub current_index: usize,
}

impl Default for FindState {
    fn default() -> Self {
        Self::new()
    }
}

impl FindState {
    pub fn new() -> Self {
        Self {
            is_open: false,
            show_replace: false,
            query: String::new(),
            replace_query: String::new(),
            active_field: FindField::Query,
            case_sensitive: false,
            whole_word: false,
            use_regex: false,
            regex_error: None,
            matches: Vec::new(),
            current_index: 0,
        }
    }

    pub fn open(&mut self) {
        self.is_open = true;
        self.active_field = FindField::Query;
    }

    pub fn open_replace(&mut self) {
        self.is_open = true;
        self.show_replace = true;
        self.active_field = FindField::Query;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.show_replace = false;
        self.query.clear();
        self.replace_query.clear();
        self.active_field = FindField::Query;
        self.regex_error = None;
        self.matches.clear();
        self.current_index = 0;
    }

    pub fn search_options(&self) -> SearchOptions {
        SearchOptions {
            case_sensitive: self.case_sensitive,
            whole_word: self.whole_word,
            use_regex: self.use_regex,
        }
    }

    pub fn active_text_mut(&mut self) -> &mut String {
        match self.active_field {
            FindField::Query => &mut self.query,
            FindField::Replace => &mut self.replace_query,
        }
    }

    pub fn toggle_case_sensitive(&mut self) {
        self.case_sensitive = !self.case_sensitive;
    }

    pub fn toggle_whole_word(&mut self) {
        self.whole_word = !self.whole_word;
    }

    pub fn toggle_regex(&mut self) {
        self.use_regex = !self.use_regex;
        if !self.use_regex {
            self.regex_error = None;
        }
    }

    pub fn next_match(&mut self) -> Option<Range<usize>> {
        if self.matches.is_empty() {
            return None;
        }
        self.current_index = (self.current_index + 1) % self.matches.len();
        self.matches.get(self.current_index).cloned()
    }

    pub fn prev_match(&mut self) -> Option<Range<usize>> {
        if self.matches.is_empty() {
            return None;
        }
        if self.current_index == 0 {
            self.current_index = self.matches.len() - 1;
        } else {
            self.current_index -= 1;
        }
        self.matches.get(self.current_index).cloned()
    }

    fn option_pill(
        label: &'static str,
        active: bool,
        theme: &Theme,
        on_click: UiAction,
    ) -> AnyElement {
        div()
            .h_5()
            .px_1p5()
            .rounded_xs()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .bg(if active { theme.accent } else { theme.bg_hover })
            .text_color(if active {
                theme.text_on_accent
            } else {
                theme.text_secondary
            })
            .hover(|s| s.opacity(0.88))
            .text_xs()
            .child(label)
            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                cx.stop_propagation();
                on_click(win, cx);
            })
            .into_any_element()
    }

    pub fn render(
        &self,
        theme: &Theme,
        find_focused: bool,
        callbacks: FindReplaceCallbacks,
    ) -> impl IntoElement {
        let match_info = if self.regex_error.is_some() {
            "invalid regex".to_string()
        } else if self.query.is_empty() {
            String::new()
        } else if self.matches.is_empty() {
            "0 matches".to_string()
        } else {
            format!("{}/{}", self.current_index + 1, self.matches.len())
        };

        let query_focused = find_focused && self.active_field == FindField::Query;
        let replace_focused = find_focused && self.active_field == FindField::Replace;

        let on_toggle_replace = callbacks.on_toggle_replace;
        let on_focus_query = Rc::clone(&callbacks.on_focus_field);
        let on_focus_replace = callbacks.on_focus_field;
        let on_prev = callbacks.on_prev_match;
        let on_next = callbacks.on_next_match;
        let on_close = callbacks.on_close;
        let on_replace_cur = callbacks.on_replace_current;
        let on_replace_all = callbacks.on_replace_all;

        let find_row = div()
            .h(px(28.0))
            .flex()
            .items_center()
            .gap_1p5()
            .child(
                div()
                    .w_4()
                    .h_5()
                    .rounded_xs()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                    .text_color(theme.text_secondary)
                    .child(if self.show_replace { "▾" } else { "▸" })
                    .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                        cx.stop_propagation();
                        on_toggle_replace(win, cx);
                    }),
            )
            .child(div().text_color(theme.accent).child("⌕"))
            .child(
                div()
                    .w(px(176.0))
                    .h_6()
                    .bg(theme.bg_editor)
                    .border_1()
                    .border_color(if query_focused {
                        theme.accent
                    } else {
                        theme.border_subtle
                    })
                    .rounded_xs()
                    .px_2()
                    .flex()
                    .items_center()
                    .overflow_hidden()
                    .cursor_text()
                    .text_color(if self.query.is_empty() {
                        theme.text_muted
                    } else {
                        theme.text_primary
                    })
                    .child(if self.query.is_empty() {
                        "Find in file...".to_string()
                    } else {
                        self.query.clone()
                    })
                    .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                        cx.stop_propagation();
                        on_focus_query(FindField::Query, win, cx);
                    }),
            )
            .child(Self::option_pill(
                "Aa",
                self.case_sensitive,
                theme,
                callbacks.on_toggle_case_sensitive,
            ))
            .child(Self::option_pill(
                "W",
                self.whole_word,
                theme,
                callbacks.on_toggle_whole_word,
            ))
            .child(Self::option_pill(
                ".*",
                self.use_regex,
                theme,
                callbacks.on_toggle_regex,
            ))
            .child(
                div()
                    .min_w(px(46.0))
                    .text_xs()
                    .text_color(
                        if self.regex_error.is_some()
                            || (self.matches.is_empty() && !self.query.is_empty())
                        {
                            theme.error
                        } else {
                            theme.text_secondary
                        },
                    )
                    .child(match_info),
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
                    .text_color(theme.text_muted)
                    .child("↑")
                    .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                        cx.stop_propagation();
                        on_prev(win, cx);
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
                    .text_color(theme.text_muted)
                    .child("↓")
                    .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                        cx.stop_propagation();
                        on_next(win, cx);
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
                    .hover(|s| s.bg(theme.bg_hover).text_color(theme.error))
                    .text_color(theme.text_muted)
                    .child("✕")
                    .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                        cx.stop_propagation();
                        on_close(win, cx);
                    }),
            );

        let replace_row = if self.show_replace {
            Some(
                div()
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .pl(px(34.0))
                    .child(
                        div()
                            .w(px(176.0))
                            .h_6()
                            .bg(theme.bg_editor)
                            .border_1()
                            .border_color(if replace_focused {
                                theme.accent
                            } else {
                                theme.border_subtle
                            })
                            .rounded_xs()
                            .px_2()
                            .flex()
                            .items_center()
                            .overflow_hidden()
                            .cursor_text()
                            .text_color(if self.replace_query.is_empty() {
                                theme.text_muted
                            } else {
                                theme.text_primary
                            })
                            .child(if self.replace_query.is_empty() {
                                "Replace with...".to_string()
                            } else {
                                self.replace_query.clone()
                            })
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_focus_replace(FindField::Replace, win, cx);
                            }),
                    )
                    .child(
                        div()
                            .h_5()
                            .px_2()
                            .rounded_xs()
                            .cursor_pointer()
                            .bg(theme.bg_hover)
                            .text_color(theme.text_primary)
                            .hover(|s| s.opacity(0.85))
                            .flex()
                            .items_center()
                            .text_xs()
                            .child("Replace")
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_replace_cur(win, cx);
                            }),
                    )
                    .child(
                        div()
                            .h_5()
                            .px_2()
                            .rounded_xs()
                            .cursor_pointer()
                            .bg(theme.bg_hover)
                            .text_color(theme.text_primary)
                            .hover(|s| s.opacity(0.85))
                            .flex()
                            .items_center()
                            .text_xs()
                            .child("All")
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_replace_all(win, cx);
                            }),
                    ),
            )
        } else {
            None
        };

        div()
            .absolute()
            .top(px(8.0))
            .right(px(16.0))
            .bg(theme.bg_surface)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .shadow_md()
            .flex()
            .flex_col()
            .gap_1()
            .py_1()
            .px_2()
            .text_xs()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .child(find_row)
            .children(replace_row)
    }
}

#[cfg(test)]
mod tests {
    use super::{FindState, SearchOptions};

    #[test]
    fn test_find_state_options_and_navigation() {
        let mut st = FindState::new();
        st.open_replace();
        assert!(st.is_open);
        assert!(st.show_replace);

        st.toggle_case_sensitive();
        st.toggle_whole_word();
        st.toggle_regex();
        assert_eq!(
            st.search_options(),
            SearchOptions {
                case_sensitive: true,
                whole_word: true,
                use_regex: true,
            }
        );

        st.matches = vec![0..2, 5..7];
        assert_eq!(st.next_match(), Some(5..7));
        assert_eq!(st.next_match(), Some(0..2));
        assert_eq!(st.prev_match(), Some(5..7));

        st.close();
        assert!(!st.is_open);
        assert!(!st.show_replace);
        assert!(st.query.is_empty());
        assert!(st.matches.is_empty());
    }
}
