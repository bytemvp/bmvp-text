use crate::config::settings::{AppConfig, EditorSettings};
use crate::core::buffer::TextBuffer;
use crate::core::git::detect_git_branch;
use crate::terminal::session::{SystemTerminal, resolve_available_shell};
use crate::ui::confirm_modal::PendingClose;
use crate::ui::editor_view::EditorView;
use crate::ui::file_finder::FileFinder;
use crate::ui::theme::{Theme, ThemeMode};
use gpui::*;
use std::ops::Range;
use std::path::{Path, PathBuf};

impl EditorView {
    pub fn has_pending_close(&self) -> bool {
        self.pending_close.is_some()
    }

    pub fn clear_status_message(&mut self) {
        self.status_message = None;
    }

    pub(super) fn set_error_message(&mut self, msg: String) {
        crate::core::logging::error("editor", &msg);
        self.status_message = Some((msg, true));
    }

    pub(super) fn refresh_git_branch(&mut self) {
        self.git_branch = self
            .sidebar
            .root_path
            .as_deref()
            .and_then(detect_git_branch);
    }

    pub fn editor_viewport_lines(&self, window: Option<&Window>) -> usize {
        if self.code_layout.viewport_lines > 0 {
            return self.code_layout.viewport_lines;
        }
        if let Some(win) = window {
            let win_h = f32::from(win.viewport_size().height);
            let term_h = if self.show_terminal && !self.terminals.is_empty() {
                self.terminal_height + 2.0
            } else {
                0.0
            };
            let avail_h = (win_h - 126.0 - term_h).max(20.0);
            let line_h = (self.settings.font_size * self.settings.line_height_factor).max(14.0);
            return ((avail_h / line_h).floor() as usize).max(1);
        }
        28
    }

    pub fn ensure_cursor_visible(&mut self, viewport_lines: usize) {
        let vp = viewport_lines.max(1);
        if let Some(buf) = self.current_buffer_mut() {
            let total_lines = buf.len_lines().max(1);
            let max_scroll = total_lines.saturating_sub(1);
            let mut scroll_top = buf.scroll_top().min(max_scroll);
            let row = buf.cursor_position().row;
            if row < scroll_top {
                scroll_top = row;
            } else if row >= scroll_top + vp {
                scroll_top = (row + 1).saturating_sub(vp);
            }
            buf.set_scroll_top(scroll_top.min(max_scroll));
        }
    }

    pub fn ensure_system_fonts_loaded(&mut self, cx: &Context<Self>) {
        if !self.system_fonts.is_empty() {
            return;
        }
        let mut fonts: Vec<String> = cx
            .text_system()
            .all_font_names()
            .into_iter()
            .filter(|name| !name.starts_with('.'))
            .collect();

        for &fallback in EditorSettings::AVAILABLE_FONTS {
            if !fonts.iter().any(|f| f.eq_ignore_ascii_case(fallback)) {
                fonts.push(fallback.to_string());
            }
        }
        for current in [
            &self.settings.font_family,
            &self.settings.terminal_font_family,
        ] {
            if !current.is_empty() && !fonts.iter().any(|f| f.eq_ignore_ascii_case(current)) {
                fonts.push(current.clone());
            }
        }

        fonts.sort_by_key(|a| a.to_lowercase());
        fonts.dedup();
        self.system_fonts = fonts;
    }

    pub fn toggle_settings_modal(&mut self, cx: &mut Context<Self>) {
        self.show_settings = !self.show_settings;
        if self.show_settings {
            self.ensure_system_fonts_loaded(cx);
        } else {
            self.open_font_dropdown = None;
            self.font_dropdown_query.clear();
            self.font_dropdown_scroll = 0;
        }
        cx.notify();
    }

    pub fn save_config(&mut self) {
        self.settings.sidebar_width = self.sidebar_width;
        self.settings.terminal_height = self.terminal_height;
        let config = AppConfig {
            theme_mode: self.theme.mode,
            editor: self.settings.clone(),
        };
        if let Err(err) = config.save() {
            self.set_error_message(format!("Save settings failed: {}", err));
        }
    }

    pub fn current_terminal(&self) -> Option<&SystemTerminal> {
        self.terminals.get(self.active_terminal)
    }

    pub fn current_terminal_mut(&mut self) -> Option<&mut SystemTerminal> {
        self.terminals.get_mut(self.active_terminal)
    }

    pub fn toggle_terminal(&mut self, cx: &mut Context<Self>) {
        self.show_terminal = !self.show_terminal;
        if self.show_terminal {
            if self.terminals.is_empty() {
                let dir = self.sidebar.root_path.clone().unwrap_or_else(|| {
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                });
                let id = self.next_terminal_id;
                self.next_terminal_id += 1;
                let shell_kind = resolve_available_shell(&self.settings.terminal_shell);
                let title = format!("{} {}", shell_kind.short_name(), id);
                self.terminals.push(SystemTerminal::new(
                    id,
                    &title,
                    dir,
                    &self.settings.terminal_shell,
                    Some(self.terminal_notifier.clone()),
                ));
                self.active_terminal = 0;
            }
            self.terminal_focused = true;
            self.ensure_terminal_listener(cx);
        } else {
            self.terminal_focused = false;
        }
        cx.notify();
    }

    pub fn new_terminal_tab(&mut self, cx: &mut Context<Self>) {
        let dir = self
            .sidebar
            .root_path
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        let id = self.next_terminal_id;
        self.next_terminal_id += 1;
        let shell_kind = resolve_available_shell(&self.settings.terminal_shell);
        let title = format!("{} {}", shell_kind.short_name(), id);
        self.terminals.push(SystemTerminal::new(
            id,
            &title,
            dir,
            &self.settings.terminal_shell,
            Some(self.terminal_notifier.clone()),
        ));
        self.active_terminal = self.terminals.len() - 1;
        self.show_terminal = true;
        self.terminal_focused = true;
        self.terminal_scroll_left = 0.0;
        self.ensure_terminal_listener(cx);
        cx.notify();
    }

    pub fn close_terminal_tab(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.terminals.len() {
            self.terminals.remove(idx);
        }
        if self.terminals.is_empty() {
            self.show_terminal = false;
            self.terminal_focused = false;
            self.active_terminal = 0;
            self.next_terminal_id = 1;
            self.terminal_scroll_left = 0.0;
        } else if self.active_terminal >= self.terminals.len() {
            self.active_terminal = self.terminals.len() - 1;
        }
        cx.notify();
    }

    pub fn close_terminal_dock(&mut self, cx: &mut Context<Self>) {
        self.show_terminal = false;
        self.terminal_focused = false;
        self.terminals.clear();
        self.active_terminal = 0;
        self.next_terminal_id = 1;
        self.terminal_scroll_left = 0.0;
        cx.notify();
    }

    pub fn restart_terminal(&mut self) {
        if let Some(term) = self.terminals.get_mut(self.active_terminal) {
            let dir = self.sidebar.root_path.clone();
            term.reload_session(dir, &self.settings.terminal_shell);
            term.title = format!("{} {}", term.shell.short_name(), term.id);
        }
        self.terminal_scroll_left = 0.0;
    }

    /// 事件驱动监听终端输出：后台 Reader 线程仅在有新事件时通过 TerminalNotifier 唤醒 UI，告别定时轮询
    pub fn ensure_terminal_listener(&mut self, cx: &mut Context<Self>) {
        if self.terminal_listener_active || self.terminals.is_empty() {
            return;
        }
        self.terminal_listener_active = true;
        let notifier = self.terminal_notifier.clone();
        let win_handle = self.window_handle;

        cx.spawn(move |view: WeakEntity<Self>, cx: &mut AsyncApp| {
            let mut cx = cx.clone();
            async move {
                loop {
                    notifier.wait().await;
                    let res = cx.update_window(win_handle, |_, _window, app| {
                        let Some(entity) = view.upgrade() else {
                            return false;
                        };
                        entity.update(app, |this, cx| {
                            let mut had_output = false;
                            for term in &mut this.terminals {
                                if term.poll_output() {
                                    had_output = true;
                                }
                            }
                            if had_output {
                                cx.notify();
                            }
                            let keep = !this.terminals.is_empty();
                            if !keep {
                                this.terminal_listener_active = false;
                            }
                            keep
                        })
                    });

                    match res {
                        Ok(true) => continue,
                        Ok(false) => break,
                        Err(e) => {
                            if view.upgrade().is_none() || !e.to_string().contains("borrow") {
                                break;
                            }
                        }
                    }
                }
            }
        })
        .detach();
    }

    pub fn set_theme_mode(&mut self, mode: ThemeMode, cx: &mut Context<Self>) {
        self.theme = Theme::from_mode(mode);
        self.save_config();
        cx.notify();
    }

    pub fn current_buffer(&self) -> Option<&TextBuffer> {
        self.tabs.get(self.active_tab)
    }

    pub fn current_buffer_mut(&mut self) -> Option<&mut TextBuffer> {
        self.tabs.get_mut(self.active_tab)
    }

    pub fn open_file_path(&mut self, path: &Path, cx: &mut Context<Self>) {
        for (idx, tab) in self.tabs.iter().enumerate() {
            if let Some(tab_path) = tab.file_path()
                && Path::new(tab_path) == path
            {
                self.active_tab = idx;
                self.terminal_focused = false;
                self.ime_marked_range = None;
                self.status_message = None;
                cx.notify();
                return;
            }
        }

        match TextBuffer::open_file(path) {
            Ok(buffer) => {
                self.tabs.push(buffer);
                self.active_tab = self.tabs.len() - 1;
                self.terminal_focused = false;
                self.ime_marked_range = None;
                self.status_message = None;
            }
            Err(err) => {
                self.set_error_message(format!(
                    "Cannot open {}: {}",
                    path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.display().to_string()),
                    err
                ));
            }
        }
        cx.notify();
    }

    pub fn open_file_dialog(&mut self, cx: &mut Context<Self>) {
        let win_handle = self.window_handle;
        cx.spawn(move |view: WeakEntity<Self>, cx: &mut AsyncApp| {
            let background = cx.background_executor().clone();
            let mut cx = cx.clone();
            async move {
                let picked = background
                    .spawn(async { rfd::FileDialog::new().pick_file() })
                    .await;
                if let Some(path) = picked
                    && let Err(err) = cx.update_window(win_handle, |_, _window, app| {
                        if let Some(entity) = view.upgrade() {
                            entity.update(app, |this, cx| {
                                this.open_file_path(&path, cx);
                            });
                        }
                    })
                {
                    crate::core::logging::warn(
                        "editor",
                        &format!("window update skipped after open file dialog: {}", err),
                    );
                }
            }
        })
        .detach();
    }

    pub fn open_folder_dialog(&mut self, cx: &mut Context<Self>) {
        let win_handle = self.window_handle;
        cx.spawn(move |view: WeakEntity<Self>, cx: &mut AsyncApp| {
            let background = cx.background_executor().clone();
            let mut cx = cx.clone();
            async move {
                let picked = background
                    .spawn(async { rfd::FileDialog::new().pick_folder() })
                    .await;
                if let Some(folder) = picked
                    && let Err(err) = cx.update_window(win_handle, |_, _window, app| {
                        if let Some(entity) = view.upgrade() {
                            entity.update(app, |this, cx| {
                                this.sidebar.set_root(folder.clone());
                                this.refresh_git_branch();
                                for term in &mut this.terminals {
                                    term.set_working_dir(folder.clone());
                                }
                                cx.notify();
                            });
                        }
                    })
                {
                    crate::core::logging::warn(
                        "editor",
                        &format!("window update skipped after open folder dialog: {}", err),
                    );
                }
            }
        })
        .detach();
    }

    pub fn save_current_file(&mut self, cx: &mut Context<Self>) {
        let Some(tab_id) = self.current_buffer().map(|b| b.id()) else {
            return;
        };
        let has_path = self
            .current_buffer()
            .is_some_and(|b| b.file_path().is_some());
        if has_path {
            if let Some(buf) = self.current_buffer_mut() {
                match buf.save_file(None) {
                    Ok(()) => self.status_message = None,
                    Err(err) => self.set_error_message(format!("Save failed: {}", err)),
                }
                cx.notify();
            }
            return;
        }
        let win_handle = self.window_handle;
        cx.spawn(move |view: WeakEntity<Self>, cx: &mut AsyncApp| {
            let background = cx.background_executor().clone();
            let mut cx = cx.clone();
            async move {
                let picked = background
                    .spawn(async { rfd::FileDialog::new().save_file() })
                    .await;
                if let Some(path) = picked
                    && let Err(err) = cx.update_window(win_handle, |_, _window, app| {
                        if let Some(entity) = view.upgrade() {
                            entity.update(app, |this, cx| {
                                if let Some(idx) = this.find_tab_by_id(tab_id)
                                    && let Some(buf) = this.tabs.get_mut(idx)
                                {
                                    match buf.save_file(Some(&path)) {
                                        Ok(()) => this.status_message = None,
                                        Err(err) => {
                                            this.set_error_message(format!("Save failed: {}", err))
                                        }
                                    }
                                    cx.notify();
                                }
                            });
                        }
                    })
                {
                    crate::core::logging::warn(
                        "editor",
                        &format!("window update skipped after save dialog: {}", err),
                    );
                }
            }
        })
        .detach();
    }

    fn find_tab_by_id(&self, tab_id: usize) -> Option<usize> {
        self.tabs.iter().position(|t| t.id() == tab_id)
    }

    pub fn has_dirty_tabs(&self) -> bool {
        self.tabs.iter().any(|t| t.is_dirty())
    }

    pub fn close_tab(&mut self, idx: usize, cx: &mut Context<Self>) {
        if idx < self.tabs.len() {
            self.tabs.remove(idx);
            if self.active_tab >= self.tabs.len() && !self.tabs.is_empty() {
                self.active_tab = self.tabs.len() - 1;
            }
            cx.notify();
        }
    }

    pub fn close_tab_by_id(&mut self, tab_id: usize, cx: &mut Context<Self>) {
        if let Some(idx) = self.find_tab_by_id(tab_id) {
            self.close_tab(idx, cx);
        }
    }

    pub fn request_close_tab(&mut self, idx: usize, cx: &mut Context<Self>) {
        let Some(tab) = self.tabs.get(idx) else {
            return;
        };
        if !tab.is_dirty() {
            self.close_tab(idx, cx);
            return;
        }
        let tab_id = tab.id();
        let file_name = tab.file_name();
        self.active_tab = idx;
        self.terminal_focused = false;
        self.pending_close = Some(PendingClose::Tab { tab_id, file_name });
        cx.notify();
    }

    pub fn request_quit(&mut self, cx: &mut Context<Self>) {
        let dirty_tabs: Vec<&TextBuffer> = self.tabs.iter().filter(|t| t.is_dirty()).collect();
        if dirty_tabs.is_empty() {
            self.allow_quit = true;
            cx.quit();
            return;
        }
        let dirty_count = dirty_tabs.len();
        let first_file_name = dirty_tabs[0].file_name();
        self.pending_close = Some(PendingClose::Quit {
            dirty_count,
            first_file_name,
        });
        cx.notify();
    }

    pub fn handle_window_should_close(&mut self, cx: &mut Context<Self>) -> bool {
        if self.allow_quit || !self.has_dirty_tabs() {
            return true;
        }
        self.request_quit(cx);
        false
    }

    pub fn confirm_cancel(&mut self, cx: &mut Context<Self>) {
        self.pending_close = None;
        cx.notify();
    }

    pub fn confirm_discard(&mut self, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_close.take() else {
            return;
        };
        match pending {
            PendingClose::Tab { tab_id, .. } => {
                self.close_tab_by_id(tab_id, cx);
            }
            PendingClose::Quit { .. } => {
                self.allow_quit = true;
                cx.quit();
            }
        }
    }

    pub fn confirm_save(&mut self, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_close.take() else {
            return;
        };
        match pending {
            PendingClose::Tab { tab_id, .. } => {
                let Some(idx) = self.find_tab_by_id(tab_id) else {
                    cx.notify();
                    return;
                };
                if self.tabs[idx].file_path().is_some() {
                    if let Err(err) = self.tabs[idx].save_file(None) {
                        self.set_error_message(format!("Save failed: {}", err));
                        cx.notify();
                        return;
                    }
                    self.status_message = None;
                    self.close_tab_by_id(tab_id, cx);
                } else {
                    self.save_tab_as_and_then(tab_id, false, cx);
                }
            }
            PendingClose::Quit { .. } => {
                for tab in &mut self.tabs {
                    if tab.is_dirty()
                        && tab.file_path().is_some()
                        && let Err(err) = tab.save_file(None)
                    {
                        self.set_error_message(format!("Save before quit failed: {}", err));
                        cx.notify();
                        return;
                    }
                }
                self.status_message = None;
                if let Some((idx, tab_id)) = self
                    .tabs
                    .iter()
                    .enumerate()
                    .find(|(_, t)| t.is_dirty())
                    .map(|(i, t)| (i, t.id()))
                {
                    self.active_tab = idx;
                    self.terminal_focused = false;
                    self.save_tab_as_and_then(tab_id, true, cx);
                    cx.notify();
                } else {
                    self.allow_quit = true;
                    cx.quit();
                }
            }
        }
    }

    fn save_tab_as_and_then(&mut self, tab_id: usize, continue_quit: bool, cx: &mut Context<Self>) {
        let win_handle = self.window_handle;
        cx.spawn(move |view: WeakEntity<Self>, cx: &mut AsyncApp| {
            let background = cx.background_executor().clone();
            let mut cx = cx.clone();
            async move {
                let picked = background
                    .spawn(async { rfd::FileDialog::new().save_file() })
                    .await;
                if let Some(path) = picked
                    && let Err(err) = cx.update_window(win_handle, |_, _window, app| {
                        if let Some(entity) = view.upgrade() {
                            entity.update(app, |this, cx| {
                                if let Some(idx) = this.find_tab_by_id(tab_id)
                                    && let Some(buf) = this.tabs.get_mut(idx)
                                {
                                    if let Err(err) = buf.save_file(Some(&path)) {
                                        this.set_error_message(format!("Save failed: {}", err));
                                        cx.notify();
                                        return;
                                    }
                                    this.status_message = None;
                                    if continue_quit {
                                        this.request_quit(cx);
                                    } else {
                                        this.close_tab_by_id(tab_id, cx);
                                    }
                                }
                            });
                        }
                    })
                {
                    crate::core::logging::warn(
                        "editor",
                        &format!("window update skipped after confirm-save dialog: {}", err),
                    );
                }
            }
        })
        .detach();
    }

    pub fn new_tab(&mut self, cx: &mut Context<Self>) {
        self.tabs.push(TextBuffer::new());
        self.active_tab = self.tabs.len() - 1;
        self.terminal_focused = false;
        self.ime_marked_range = None;
        cx.notify();
    }

    /// 异步打开 FileFinder：后台线程扫描目录，避免大工程卡顿 UI 线程
    pub fn open_file_finder(&mut self, cx: &mut Context<Self>) {
        let generation = self.file_finder.begin_open();
        let Some(root) = self.sidebar.root_path.clone() else {
            self.file_finder.finish_scan(generation, Vec::new());
            cx.notify();
            return;
        };
        let win_handle = self.window_handle;
        cx.spawn(move |view: WeakEntity<Self>, cx: &mut AsyncApp| {
            let background = cx.background_executor().clone();
            let mut cx = cx.clone();
            async move {
                let files = background
                    .spawn(async move { FileFinder::scan_workspace_files(&root, 20_000) })
                    .await;
                if let Err(err) = cx.update_window(win_handle, |_, _window, app| {
                    if let Some(entity) = view.upgrade() {
                        entity.update(app, |this, cx| {
                            this.file_finder.finish_scan(generation, files);
                            cx.notify();
                        });
                    }
                }) {
                    crate::core::logging::warn(
                        "editor",
                        &format!("window update skipped after file scan: {}", err),
                    );
                }
            }
        })
        .detach();
        cx.notify();
    }

    pub fn open_find_bar(&mut self, cx: &mut Context<Self>) {
        self.find_state.open();
        self.find_focused = true;
        if let Some(buf) = self.current_buffer()
            && let Some(sel_text) = buf.selected_text()
            && !sel_text.contains('\n')
            && !sel_text.trim().is_empty()
        {
            self.find_state.query = sel_text;
            self.update_find_matches();
        }
        cx.notify();
    }

    pub fn open_replace_bar(&mut self, cx: &mut Context<Self>) {
        self.find_state.open_replace();
        self.find_focused = true;
        if let Some(buf) = self.current_buffer()
            && let Some(sel_text) = buf.selected_text()
            && !sel_text.contains('\n')
            && !sel_text.trim().is_empty()
        {
            self.find_state.query = sel_text;
            self.update_find_matches();
        }
        cx.notify();
    }

    pub fn update_find_matches(&mut self) {
        let opts = self.find_state.search_options();
        let query = self.find_state.query.clone();
        if let Some(buf) = self.current_buffer() {
            match buf.find_matches_with_options(&query, opts) {
                Ok(matches) => {
                    self.find_state.regex_error = None;
                    self.find_state.matches = matches;
                    self.find_state.current_index = 0;
                    if let Some(m) = self.find_state.matches.first().cloned() {
                        self.jump_to_match(m);
                    }
                }
                Err(err) => {
                    self.find_state.regex_error = Some(err);
                    self.find_state.matches.clear();
                    self.find_state.current_index = 0;
                }
            }
        }
    }

    pub fn replace_current_match(&mut self, cx: &mut Context<Self>) {
        if self.find_state.matches.is_empty() {
            return;
        }
        let idx = self
            .find_state
            .current_index
            .min(self.find_state.matches.len() - 1);
        let range = self.find_state.matches[idx].clone();
        let replacement = self.find_state.replace_query.clone();
        if let Some(buf) = self.current_buffer_mut() {
            buf.replace_range_match(range, &replacement);
        }
        self.update_find_matches();
        if !self.find_state.matches.is_empty() {
            let next_idx = idx.min(self.find_state.matches.len() - 1);
            self.find_state.current_index = next_idx;
            let m = self.find_state.matches[next_idx].clone();
            self.jump_to_match(m);
        }
        cx.notify();
    }

    pub fn replace_all_matches(&mut self, cx: &mut Context<Self>) {
        if self.find_state.matches.is_empty() {
            return;
        }
        let matches = self.find_state.matches.clone();
        let replacement = self.find_state.replace_query.clone();
        let count = if let Some(buf) = self.current_buffer_mut() {
            buf.replace_all_matches(&matches, &replacement)
        } else {
            0
        };
        self.update_find_matches();
        if count > 0 {
            self.status_message = Some((format!("Replaced {} occurrences", count), false));
        }
        cx.notify();
    }

    pub fn next_find_match(&mut self, cx: &mut Context<Self>) {
        if let Some(m) = self.find_state.next_match() {
            self.jump_to_match(m);
            cx.notify();
        }
    }

    pub fn prev_find_match(&mut self, cx: &mut Context<Self>) {
        if let Some(m) = self.find_state.prev_match() {
            self.jump_to_match(m);
            cx.notify();
        }
    }

    fn jump_to_match(&mut self, range: Range<usize>) {
        let vp = self.editor_viewport_lines(None);
        if let Some(buf) = self.current_buffer_mut() {
            buf.select_range(range);
        }
        self.ensure_cursor_visible(vp);
    }

    pub fn copy_selection(&mut self, cx: &mut Context<Self>) {
        if let Some(buf) = self.current_buffer()
            && let Some(text) = buf.selected_text()
        {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    pub fn cut_selection(&mut self, cx: &mut Context<Self>) {
        self.copy_selection(cx);
        if let Some(buf) = self.current_buffer_mut() {
            buf.delete();
            cx.notify();
        }
    }

    pub fn paste_clipboard(&mut self, cx: &mut Context<Self>) {
        if let Some(item) = cx.read_from_clipboard()
            && let Some(text) = item.text()
            && let Some(buf) = self.current_buffer_mut()
        {
            buf.insert(&text);
            cx.notify();
        }
    }
}
