use crate::core::sidebar::Sidebar;
use crate::ui::theme::Theme;
use crate::ui::{UiAction, UiAction1};
use gpui::*;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use walkdir::WalkDir;

pub struct FileFinderCallbacks {
    pub on_select_file: UiAction1<PathBuf>,
    pub on_scroll: UiAction1<i32>,
    pub on_close: UiAction,
}

pub struct FileFinder {
    pub is_open: bool,
    pub is_loading: bool,
    pub scan_generation: u64,
    pub query: String,
    pub all_files: Vec<(PathBuf, String)>, // (full path, relative display path)
    pub filtered: Vec<usize>,
    pub selected_index: usize,
    pub scroll_offset: usize,
}

const VISIBLE_ROWS: usize = 12;

impl Default for FileFinder {
    fn default() -> Self {
        Self::new()
    }
}

impl FileFinder {
    pub fn new() -> Self {
        Self {
            is_open: false,
            is_loading: false,
            scan_generation: 0,
            query: String::new(),
            all_files: Vec::new(),
            filtered: Vec::new(),
            selected_index: 0,
            scroll_offset: 0,
        }
    }

    pub fn begin_open(&mut self) -> u64 {
        self.is_open = true;
        self.is_loading = true;
        self.scan_generation = self.scan_generation.wrapping_add(1);
        self.query.clear();
        self.selected_index = 0;
        self.scroll_offset = 0;
        self.all_files.clear();
        self.filtered.clear();
        self.scan_generation
    }

    pub fn finish_scan(&mut self, generation: u64, files: Vec<(PathBuf, String)>) {
        if generation != self.scan_generation || !self.is_open {
            return;
        }
        self.is_loading = false;
        self.all_files = files;
        self.refilter();
    }

    pub fn scan_workspace_files(root: &Path, max_files: usize) -> Vec<(PathBuf, String)> {
        let mut files = Vec::new();
        let gitignore_names = Sidebar::load_gitignore_names(root);
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                if e.depth() == 0 {
                    return true;
                }
                let name = e.file_name().to_string_lossy();
                !Sidebar::is_ignored_entry(&name, &gitignore_names)
            })
            .flatten()
        {
            if entry.file_type().is_file() {
                let path = entry.path().to_path_buf();
                let rel = path
                    .strip_prefix(root)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| entry.file_name().to_string_lossy().to_string());
                files.push((path, rel));
                if files.len() >= max_files {
                    break;
                }
            }
        }
        files
    }

    pub fn close(&mut self) {
        self.is_open = false;
        self.is_loading = false;
        self.scan_generation = self.scan_generation.wrapping_add(1);
        self.query.clear();
        self.scroll_offset = 0;
    }

    pub fn refilter(&mut self) {
        let q = self.query.trim().to_lowercase();
        self.filtered.clear();

        if q.is_empty() {
            for i in 0..self.all_files.len() {
                self.filtered.push(i);
            }
        } else {
            let mut scored: Vec<(i32, usize)> = self
                .all_files
                .iter()
                .enumerate()
                .filter_map(|(i, (path, rel))| {
                    let name = path
                        .file_name()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| rel.clone());
                    Self::fuzzy_score(&q, rel, &name).map(|score| (score, i))
                })
                .collect();
            scored.sort_by(|a, b| {
                b.0.cmp(&a.0)
                    .then_with(|| {
                        self.all_files[a.1]
                            .1
                            .len()
                            .cmp(&self.all_files[b.1].1.len())
                    })
                    .then_with(|| a.1.cmp(&b.1))
            });
            self.filtered = scored.into_iter().map(|(_, i)| i).collect();
        }

        if self.selected_index >= self.filtered.len() {
            self.selected_index = 0;
        }
        self.scroll_offset = 0;
        self.ensure_selected_visible();
    }

    pub fn fuzzy_score(query_lower: &str, rel_path: &str, file_name: &str) -> Option<i32> {
        if query_lower.is_empty() {
            return Some(0);
        }
        let name_lower = file_name.to_lowercase();
        let rel_lower = rel_path.to_lowercase();

        if name_lower == query_lower {
            return Some(10_000 - rel_lower.len() as i32);
        }
        if name_lower.starts_with(query_lower) {
            return Some(8_000 - rel_lower.len() as i32);
        }
        if let Some(pos) = name_lower.find(query_lower) {
            return Some(6_000 - (pos as i32) * 10 - rel_lower.len() as i32);
        }
        if let Some(pos) = rel_lower.find(query_lower) {
            return Some(4_000 - (pos as i32) * 5 - rel_lower.len() as i32);
        }

        Self::subsequence_score(query_lower, &rel_lower).map(|s| 1_000 + s - rel_lower.len() as i32)
    }

    fn subsequence_score(pattern: &str, text: &str) -> Option<i32> {
        let mut score = 0i32;
        let mut p_chars = pattern.chars();
        let mut target = p_chars.next()?;
        let mut prev_matched_idx: Option<usize> = None;
        let mut prev_char: Option<char> = None;

        for (idx, c) in text.chars().enumerate() {
            if c == target {
                score += 10;
                if prev_matched_idx.is_some_and(|p| p + 1 == idx) {
                    score += 15;
                }
                if idx == 0 || matches!(prev_char, Some('/' | '\\' | '_' | '-' | '.')) {
                    score += 12;
                }
                prev_matched_idx = Some(idx);
                if let Some(next) = p_chars.next() {
                    target = next;
                } else {
                    return Some(score);
                }
            }
            prev_char = Some(c);
        }
        None
    }

    fn ensure_selected_visible(&mut self) {
        if self.selected_index < self.scroll_offset {
            self.scroll_offset = self.selected_index;
        } else if self.selected_index >= self.scroll_offset + VISIBLE_ROWS {
            self.scroll_offset = self.selected_index + 1 - VISIBLE_ROWS;
        }
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub fn scroll_down(&mut self, lines: usize) {
        let max_scroll = self.filtered.len().saturating_sub(VISIBLE_ROWS);
        self.scroll_offset = (self.scroll_offset + lines).min(max_scroll);
    }

    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        } else if !self.filtered.is_empty() {
            self.selected_index = self.filtered.len() - 1;
        }
        self.ensure_selected_visible();
    }

    pub fn select_next(&mut self) {
        if !self.filtered.is_empty() {
            if self.selected_index + 1 < self.filtered.len() {
                self.selected_index += 1;
            } else {
                self.selected_index = 0;
            }
        }
        self.ensure_selected_visible();
    }

    pub fn selected_path(&self) -> Option<PathBuf> {
        self.filtered
            .get(self.selected_index)
            .and_then(|&idx| self.all_files.get(idx).map(|(p, _)| p.clone()))
    }

    pub fn render(
        &self,
        theme: &Theme,
        font_family: &str,
        callbacks: FileFinderCallbacks,
    ) -> AnyElement {
        let visible_items = self
            .filtered
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(VISIBLE_ROWS);
        let mut list_elements = Vec::new();
        let icon_font = font_family.to_string();

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
            let on_select = Rc::clone(&callbacks.on_select_file);

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
                    .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                        cx.stop_propagation();
                        on_select(path_clone.clone(), win, cx);
                    })
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
                                    .font_family(icon_font.clone())
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
            let empty_msg = if self.is_loading {
                "Scanning workspace files..."
            } else {
                "No files matching query"
            };
            list_elements.push(
                div()
                    .py_4()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_xs()
                    .text_color(theme.text_muted)
                    .child(empty_msg),
            );
        }

        let on_close = callbacks.on_close;
        let on_scroll = callbacks.on_scroll;
        let result_badge = if self.is_loading {
            "scanning...".to_string()
        } else {
            format!("{} results", self.filtered.len())
        };

        div()
            .absolute()
            .inset_0()
            .bg(rgba(0x000000aa))
            .flex()
            .items_start()
            .justify_center()
            .pt(px(60.0))
            .occlude()
            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                cx.stop_propagation();
                on_close(win, cx);
            })
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
                    .occlude()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
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
                                    .child(result_badge),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .py_1p5()
                            .flex()
                            .flex_col()
                            .gap_0p5()
                            .on_scroll_wheel(move |event: &ScrollWheelEvent, win, cx| {
                                let delta_y = match event.delta {
                                    ScrollDelta::Pixels(p) => (p.y / px(20.0)) as i32,
                                    ScrollDelta::Lines(l) => l.y as i32,
                                };
                                if delta_y != 0 {
                                    on_scroll(delta_y, win, cx);
                                }
                                cx.stop_propagation();
                            })
                            .children(list_elements),
                    )
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
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::FileFinder;
    use std::path::PathBuf;

    #[test]
    fn test_file_finder_scroll_follows_selection() {
        let mut finder = FileFinder::new();
        let scan_gen = finder.begin_open();
        let files = (0..25)
            .map(|i| (PathBuf::from(format!("f{}.rs", i)), format!("f{}.rs", i)))
            .collect();
        finder.finish_scan(scan_gen, files);
        assert_eq!(finder.filtered.len(), 25);
        assert_eq!(finder.scroll_offset, 0);

        for _ in 0..14 {
            finder.select_next();
        }
        assert_eq!(finder.selected_index, 14);
        assert_eq!(finder.scroll_offset, 3);

        finder.select_prev();
        assert_eq!(finder.selected_index, 13);
        finder.select_prev();
        assert_eq!(finder.selected_index, 12);

        finder.selected_index = 0;
        finder.scroll_offset = 0;
        finder.select_prev();
        assert_eq!(finder.selected_index, 24);
        assert_eq!(finder.scroll_offset, 13);
    }

    #[test]
    fn test_file_finder_fuzzy_ranking_and_stale_generation_ignored() {
        let mut finder = FileFinder::new();
        let gen1 = finder.begin_open();
        let gen2 = finder.begin_open();

        // 过期代际扫描结果应被忽略
        finder.finish_scan(
            gen1,
            vec![(PathBuf::from("stale.rs"), "stale.rs".to_string())],
        );
        assert!(finder.is_loading);
        assert!(finder.all_files.is_empty());

        finder.finish_scan(
            gen2,
            vec![
                (
                    PathBuf::from("src/other/my_editor_helper.rs"),
                    "src/other/my_editor_helper.rs".to_string(),
                ),
                (
                    PathBuf::from("src/ui/editor.rs"),
                    "src/ui/editor.rs".to_string(),
                ),
                (
                    PathBuf::from("src/editor_view.rs"),
                    "src/editor_view.rs".to_string(),
                ),
            ],
        );
        assert!(!finder.is_loading);
        finder.query = "editor.rs".to_string();
        finder.refilter();
        assert_eq!(
            finder.selected_path(),
            Some(PathBuf::from("src/ui/editor.rs"))
        );
    }
}
