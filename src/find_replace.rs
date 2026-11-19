use crate::theme::Theme;
use gpui::*;
use std::ops::Range;

pub struct FindState {
    pub is_open: bool,
    pub query: String,
    pub matches: Vec<Range<usize>>,
    pub current_index: usize,
}

impl FindState {
    pub fn new() -> Self {
        Self {
            is_open: false,
            query: String::new(),
            matches: Vec::new(),
            current_index: 0,
        }
    }

    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.query.clear();
        self.matches.clear();
        self.current_index = 0;
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

    pub fn render(
        &self,
        theme: &Theme,
        cx: &Context<crate::editor::EditorView>,
    ) -> impl IntoElement {
        let match_info = if self.query.is_empty() {
            String::new()
        } else if self.matches.is_empty() {
            "0 matches".to_string()
        } else {
            format!("{}/{}", self.current_index + 1, self.matches.len())
        };

        div()
            .absolute()
            .top(px(8.0))
            .right(px(16.0))
            .h(px(32.0))
            .bg(theme.bg_surface)
            .border_1()
            .border_color(theme.border)
            .rounded_md()
            .shadow_md()
            .flex()
            .items_center()
            .gap_1p5()
            .px_2()
            .text_xs()
            .child(div().text_color(theme.accent).child("⌕"))
            .child(
                div()
                    .w(px(190.0))
                    .h_6()
                    .bg(theme.bg_editor)
                    .border_1()
                    .border_color(theme.border_subtle)
                    .rounded_xs()
                    .px_2()
                    .flex()
                    .items_center()
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
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _win, cx| {
                            this.find_focused = true;
                            cx.notify();
                        }),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(if self.matches.is_empty() && !self.query.is_empty() {
                        theme.error
                    } else {
                        theme.text_secondary
                    })
                    .child(match_info),
            )
            // 上一个
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
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _win, cx| {
                            this.prev_find_match(cx);
                        }),
                    ),
            )
            // 下一个
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
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _win, cx| {
                            this.next_find_match(cx);
                        }),
                    ),
            )
            // 关闭
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
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _win, cx| {
                            this.find_state.close();
                            cx.notify();
                        }),
                    ),
            )
    }
}
