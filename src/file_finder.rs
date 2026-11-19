use crate::sidebar::Sidebar;
use crate::theme::Theme;
use gpui::*;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub struct FileFinder {
    pub is_open: bool,
    pub query: String,
    pub all_files: Vec<(PathBuf, String)>, // (full path, relative display path)
    pub filtered: Vec<usize>,
    pub selected_index: usize,
}

impl FileFinder {
    pub fn new() -> Self {
        Self {
            is_open: false,
            query: String::new(),
            all_files: Vec::new(),
            filtered: Vec::new(),
            selected_index: 0,
        }
    }

    pub fn open(&mut self, root_dir: Option<&Path>) {
        self.is_open = true;
        self.query.clear();
        self.selected_index = 0;
        self.all_files.clear();

        if let Some(root) = root_dir {
            for entry in WalkDir::new(root)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| {
                    let name = e.file_name().to_string_lossy();
                    !name.starts_with('.') && name != "target" && name != "node_modules"
                })
                .flatten()
            {
                if entry.file_type().is_file() {
                    let path = entry.path().to_path_buf();
                    let rel = path
                        .strip_prefix(root)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| entry.file_name().to_string_lossy().to_string());
                    self.all_files.push((path, rel));
                }
            }
        }

        self.refilter();
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.query.clear();
    }

    pub fn refilter(&mut self) {
        let q = self.query.trim().to_lowercase();
        self.filtered.clear();

        if q.is_empty() {
            for i in 0..self.all_files.len() {
                self.filtered.push(i);
            }
        } else {
            for (i, (_, rel)) in self.all_files.iter().enumerate() {
                let rel_lower = rel.to_lowercase();
                if rel_lower.contains(&q) || Self::subsequence_match(&q, &rel_lower) {
                    self.filtered.push(i);
                }
            }
        }

        if self.selected_index >= self.filtered.len() {
            self.selected_index = 0;
        }
    }

    fn subsequence_match(pattern: &str, text: &str) -> bool {
        let mut p_chars = pattern.chars();
        if let Some(mut target) = p_chars.next() {
            for c in text.chars() {
                if c == target {
                    if let Some(next) = p_chars.next() {
                        target = next;
                    } else {
                        return true;
                    }
                }
            }
            false
        } else {
            true
        }
    }

    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else if !self.filtered.is_empty() {
            self.selected_index = self.filtered.len() - 1;
        }
    }

    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() {
            if self.selected_index + 1 < self.filtered.len() {
                self.selected_index += 1;
            } else {
                self.selected_index = 0;
            }
        }
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        self.filtered
            .get(self.selected_index)
            .and_then(|&idx| self.all_files.get(idx).map(|(p, _)| p.clone()))
    }

    pub fn render(
        &self,
        theme: &Theme,
        cx: &Context<crate::editor::EditorView>,
    ) -> impl IntoElement {
        let visible_items = self.filtered.iter().enumerate().take(12);
        let mut list_elements = Vec::new();

        for (display_idx, &file_idx) in visible_items {
            let (path, rel) = &self.all_files[file_idx];
            let is_selected = display_idx == self.selected_index;
            let file_name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| rel.clone());
            let icon = Sidebar::file_icon(false, false, &file_name);

            let bg = if is_selected {
                theme.bg_hover
            } else {
                theme.bg_modal
            };
            let path_clone = path.clone();

            list_elements.push(
                div()
                    .h_7()
                    .px_2p5()
                    .mx_1()
                    .rounded_sm()
                    .flex()
                    .items_center()
                    .justify_between()
                    .cursor_pointer()
                    .bg(bg)
                    .hover(|s| s.bg(theme.bg_hover))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _win, cx| {
                            this.open_file_path(&path_clone, cx);
                            this.file_finder.close();
                            cx.notify();
                        }),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(20.0))
                                    .text_xs()
                                    .text_color(theme.text_muted)
                                    .font_family("Consolas")
                                    .child(icon),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(if is_selected {
                                        FontWeight::BOLD
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .text_color(if is_selected {
                                        theme.accent
                                    } else {
                                        theme.text_primary
                                    })
                                    .child(file_name),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(rel.clone()),
                    ),
            );
        }

        if self.filtered.is_empty() {
            list_elements.push(
                div()
                    .py_4()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_xs()
                    .text_color(theme.text_muted)
                    .child("No files matching query"),
            );
        }

        // 居中半透明遮罩与 Zed 风格浮动弹窗
        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x000000aa))
            .flex()
            .items_start()
            .justify_center()
            .pt(px(60.0))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _win, cx| {
                    this.file_finder.close();
                    cx.notify();
                }),
            )
            .child(
                div()
                    .w(px(580.0))
                    .max_h(px(460.0))
                    .bg(theme.bg_modal)
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .shadow_lg()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .on_mouse_down(MouseButton::Left, |_, _, _| {}) // 阻止冒泡
                    // 头部搜索输入框
                    .child(
                        div()
                            .h(px(42.0))
                            .px_3()
                            .border_b_1()
                            .border_color(theme.border_subtle)
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().text_color(theme.accent).text_sm().child("⌕"))
                            .child(
                                div()
                                    .flex_1()
                                    .text_sm()
                                    .text_color(if self.query.is_empty() {
                                        theme.text_muted
                                    } else {
                                        theme.text_primary
                                    })
                                    .child(if self.query.is_empty() {
                                        "Search files by name (type to filter)...".to_string()
                                    } else {
                                        self.query.clone()
                                    }),
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_0p5()
                                    .rounded_xs()
                                    .bg(theme.bg_hover)
                                    .text_xs()
                                    .text_color(theme.text_muted)
                                    .child(format!("{} results", self.filtered.len())),
                            ),
                    )
                    // 结果文件列表
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .py_1p5()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .children(list_elements),
                    )
                    // 底部操作提示
                    .child(
                        div()
                            .h(px(26.0))
                            .px_3()
                            .bg(theme.bg_surface)
                            .border_t_1()
                            .border_color(theme.border_subtle)
                            .flex()
                            .items_center()
                            .justify_between()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child("↑↓ to navigate · Enter to open · Esc to close")
                            .child("Zed File Finder"),
                    ),
            )
    }
}
