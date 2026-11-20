use crate::buffer::TextBuffer;
use crate::file_finder::FileFinder;
use crate::find_replace::FindState;
use crate::settings::EditorSettings;
use crate::settings_modal::{FontDropdownTarget, SettingsModal};
use crate::sidebar::Sidebar;
use crate::status_bar::StatusBar;
use crate::syntax::{highlight_line, token_color};
use crate::system_terminal::{SystemTerminal, parse_ansi_spans, resolve_available_shell};
use crate::theme::{Theme, ThemeMode};
use gpui::*;
use std::path::{Path, PathBuf};

pub struct EditorView {
    pub tabs: Vec<TextBuffer>,
    pub active_tab: usize,
    pub sidebar: Sidebar,
    pub show_sidebar: bool,
    pub show_settings: bool,
    pub show_terminal: bool,
    pub terminals: Vec<SystemTerminal>,
    pub active_terminal: usize,
    pub next_terminal_id: usize,
    pub terminal_focused: bool,
    pub file_finder: FileFinder,
    pub find_state: FindState,
    pub find_focused: bool,
    pub settings: EditorSettings,
    pub theme: Theme,
    pub focus_handle: FocusHandle,
    pub scroll_top: usize,

    // 面板尺寸与拖拽状态
    pub sidebar_width: f32,
    pub terminal_height: f32,
    pub resizing_sidebar: bool,
    pub resizing_terminal: bool,
    pending_window_drag: Option<Point<Pixels>>,
    terminal_polling: bool,
    window_handle: AnyWindowHandle,

    // 字体下拉列表状态
    pub system_fonts: Vec<String>,
    pub open_font_dropdown: Option<FontDropdownTarget>,
    pub font_dropdown_query: String,
    pub font_dropdown_scroll: usize,
}

fn extract_typed_text(keystroke: &Keystroke) -> Option<String> {
    if keystroke.modifiers.control || keystroke.modifiers.platform || keystroke.modifiers.alt {
        return None;
    }
    if let Some(ref kc) = keystroke.key_char
        && !kc.is_empty()
        && kc.chars().all(|c| !c.is_control())
    {
        return Some(kc.clone());
    }
    if keystroke.key == "space" {
        return Some(" ".to_string());
    }
    if keystroke.key.chars().count() == 1
        && let Some(ch) = keystroke.key.chars().next()
        && !ch.is_control()
    {
        return Some(keystroke.key.clone());
    }
    None
}

impl EditorView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let initial_text = r#"// Welcome to bmvp-text
// A lightweight, high-performance editor inspired by Zed
// Pure Rust + GPUI · Zero AI bloat · Instant response

fn main() {
    let editor = "Zed-style Lightweight Editor";
    println!("Fast, clean, and distraction-free: {}", editor);
}

// Shortcuts:
//   Ctrl+P       : File Finder / Quick Open
//   Ctrl+F       : Find in File
//   Ctrl+`       : Toggle Terminal Dock
//   Ctrl+,       : Preferences (Themes, Fonts, Shell)
//   Ctrl+B       : Toggle Project Explorer
//   Ctrl+O / S   : Open / Save File
//   Ctrl+N / W   : New / Close Tab
//   Ctrl+Z / Y   : Undo / Redo
//   Ctrl+C / X / V : Clipboard Copy, Cut, Paste
//   Ctrl+A       : Select All
//   Ctrl+D       : Duplicate Line
//   Ctrl+/       : Toggle Comment
//   Ctrl+Q       : Quit Application
"#;
        let config = crate::settings::AppConfig::load();
        let default_buffer = TextBuffer::from_str(initial_text);
        let sidebar = Sidebar::new();
        Self {
            tabs: vec![default_buffer],
            active_tab: 0,
            sidebar,
            show_sidebar: true,
            show_settings: false,
            show_terminal: false,
            terminals: Vec::new(),
            active_terminal: 0,
            next_terminal_id: 1,
            terminal_focused: false,
            file_finder: FileFinder::new(),
            find_state: FindState::new(),
            find_focused: false,
            theme: Theme::from_mode(config.theme_mode),
            settings: config.editor.clone(),
            focus_handle: cx.focus_handle(),
            scroll_top: 0,
            sidebar_width: config.editor.sidebar_width,
            terminal_height: config.editor.terminal_height,
            resizing_sidebar: false,
            resizing_terminal: false,
            pending_window_drag: None,
            terminal_polling: false,
            window_handle: window.window_handle(),
            system_fonts: Vec::new(),
            open_font_dropdown: None,
            font_dropdown_query: String::new(),
            font_dropdown_scroll: 0,
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
        let config = crate::settings::AppConfig {
            theme_mode: self.theme.mode,
            editor: self.settings.clone(),
        };
        config.save();
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
                ));
                self.active_terminal = 0;
            }
            self.terminal_focused = true;
            self.poll_active_terminals(cx);
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
        ));
        self.active_terminal = self.terminals.len() - 1;
        self.show_terminal = true;
        self.terminal_focused = true;
        self.poll_active_terminals(cx);
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
        } else if self.active_terminal >= self.terminals.len() {
            self.active_terminal = self.terminals.len() - 1;
        }
        cx.notify();
    }

    pub fn close_terminal_dock(&mut self, cx: &mut Context<Self>) {
        self.show_terminal = false;
        self.terminal_focused = false;
        // 关闭命令行后清空会话，下次重新打开时重新加载Profile
        self.terminals.clear();
        self.active_terminal = 0;
        self.next_terminal_id = 1;
        cx.notify();
    }

    pub fn restart_terminal(&mut self) {
        if let Some(term) = self.terminals.get_mut(self.active_terminal) {
            let dir = self.sidebar.root_path.clone();
            term.reload_session(dir, &self.settings.terminal_shell);
            term.title = format!("{} {}", term.shell.short_name(), term.id);
        }
    }

    pub fn poll_active_terminals(&mut self, cx: &mut Context<Self>) {
        if self.terminal_polling {
            return;
        }
        if (self.show_terminal && !self.terminals.is_empty())
            || self.terminals.iter().any(|t| t.is_running)
        {
            self.terminal_polling = true;
            let win_handle = self.window_handle;
            cx.spawn(move |view: WeakEntity<Self>, cx: &mut AsyncApp| {
                let background = cx.background_executor().clone();
                let mut cx = cx.clone();
                async move {
                    loop {
                        background.timer(std::time::Duration::from_millis(50)).await;
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
                                let keep = (this.show_terminal && !this.terminals.is_empty())
                                    || this.terminals.iter().any(|t| t.is_running);
                                if !keep {
                                    this.terminal_polling = false;
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
                cx.notify();
                return;
            }
        }

        if let Ok(buffer) = TextBuffer::open_file(path) {
            self.tabs.push(buffer);
            self.active_tab = self.tabs.len() - 1;
            self.terminal_focused = false;
            cx.notify();
        }
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
                if let Some(path) = picked {
                    let _ = cx.update_window(win_handle, |_, _window, app| {
                        if let Some(entity) = view.upgrade() {
                            entity.update(app, |this, cx| {
                                this.open_file_path(&path, cx);
                            });
                        }
                    });
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
                if let Some(folder) = picked {
                    let _ = cx.update_window(win_handle, |_, _window, app| {
                        if let Some(entity) = view.upgrade() {
                            entity.update(app, |this, cx| {
                                this.sidebar.set_root(folder.clone());
                                for term in &mut this.terminals {
                                    term.set_working_dir(folder.clone());
                                }
                                cx.notify();
                            });
                        }
                    });
                }
            }
        })
        .detach();
    }

    pub fn save_current_file(&mut self, cx: &mut Context<Self>) {
        let has_path = self
            .current_buffer()
            .is_some_and(|b| b.file_path().is_some());
        if has_path {
            if let Some(buf) = self.current_buffer_mut() {
                let _ = buf.save_file(None);
                cx.notify();
            }
            return;
        }
        if self.current_buffer().is_some() {
            let win_handle = self.window_handle;
            let active_idx = self.active_tab;
            cx.spawn(move |view: WeakEntity<Self>, cx: &mut AsyncApp| {
                let background = cx.background_executor().clone();
                let mut cx = cx.clone();
                async move {
                    let picked = background
                        .spawn(async { rfd::FileDialog::new().save_file() })
                        .await;
                    if let Some(path) = picked {
                        let _ = cx.update_window(win_handle, |_, _window, app| {
                            if let Some(entity) = view.upgrade() {
                                entity.update(app, |this, cx| {
                                    if let Some(buf) = this.tabs.get_mut(active_idx) {
                                        let _ = buf.save_file(Some(&path));
                                        cx.notify();
                                    }
                                });
                            }
                        });
                    }
                }
            })
            .detach();
        }
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

    pub fn new_tab(&mut self, cx: &mut Context<Self>) {
        self.tabs.push(TextBuffer::new());
        self.active_tab = self.tabs.len() - 1;
        self.terminal_focused = false;
        cx.notify();
    }

    pub fn open_file_finder(&mut self, cx: &mut Context<Self>) {
        let root = self.sidebar.root_path.clone();
        self.file_finder.open(root.as_deref());
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

    pub fn update_find_matches(&mut self) {
        if let Some(buf) = self.current_buffer() {
            let matches = buf.find_matches(&self.find_state.query);
            self.find_state.matches = matches;
            self.find_state.current_index = 0;
            if let Some(m) = self.find_state.matches.first().cloned() {
                self.jump_to_match(m);
            }
        }
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

    fn jump_to_match(&mut self, range: std::ops::Range<usize>) {
        if let Some(buf) = self.current_buffer_mut() {
            buf.select_range(range);
            let row = buf.cursor_position().row;
            if row < self.scroll_top {
                self.scroll_top = row;
            } else if row >= self.scroll_top + 28 {
                self.scroll_top = row.saturating_sub(27);
            }
        }
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

    fn handle_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(start_pos) = self.pending_window_drag {
            let dx = (f32::from(event.position.x) - f32::from(start_pos.x)).abs();
            let dy = (f32::from(event.position.y) - f32::from(start_pos.y)).abs();
            if dx > 3.0 || dy > 3.0 {
                self.pending_window_drag = None;
                crate::window_controls::start_window_drag(window);
            }
            return;
        }
        if self.resizing_sidebar {
            let x: f32 = event.position.x.into();
            let win_w: f32 = window.viewport_size().width.into();
            let max_w = (win_w - 200.0).max(180.0);
            let new_w = x.clamp(140.0, max_w);
            if (self.sidebar_width - new_w).abs() > 0.5 {
                self.sidebar_width = new_w;
                cx.notify();
            }
        } else if self.resizing_terminal {
            let y: f32 = event.position.y.into();
            let win_h: f32 = window.viewport_size().height.into();
            // 底部状态栏高度为 26px，终端位于状态栏正上方
            let max_h = (win_h - 120.0).max(100.0);
            let new_h = (win_h - 26.0 - y).clamp(80.0, max_h);
            if (self.terminal_height - new_h).abs() > 0.5 {
                self.terminal_height = new_h;
                cx.notify();
            }
        }
    }

    fn handle_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button == MouseButton::Left {
            self.pending_window_drag = None;
            if self.resizing_sidebar || self.resizing_terminal {
                self.resizing_sidebar = false;
                self.resizing_terminal = false;
                self.save_config();
                cx.notify();
            }
        }
    }

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        let is_shift = event.keystroke.modifiers.shift;
        let is_ctrl = event.keystroke.modifiers.control || event.keystroke.modifiers.platform;

        // 1. File Finder 弹窗状态下的键盘截获
        if self.file_finder.is_open {
            match key {
                "escape" => {
                    self.file_finder.close();
                }
                "up" => {
                    self.file_finder.select_prev();
                }
                "down" => {
                    self.file_finder.select_next();
                }
                "enter" => {
                    if let Some(path) = self.file_finder.selected_path() {
                        self.open_file_path(&path, cx);
                    }
                    self.file_finder.close();
                }
                "backspace" => {
                    self.file_finder.query.pop();
                    self.file_finder.refilter();
                }
                _ => {
                    if !is_ctrl && let Some(text) = extract_typed_text(&event.keystroke) {
                        self.file_finder.query.push_str(&text);
                        self.file_finder.refilter();
                    }
                }
            }
            cx.notify();
            return;
        }

        // 2. Settings Modal 弹窗快捷键
        if self.show_settings {
            if self.open_font_dropdown.is_some() {
                match key {
                    "escape" => {
                        self.open_font_dropdown = None;
                        self.font_dropdown_query.clear();
                        self.font_dropdown_scroll = 0;
                        cx.notify();
                    }
                    "backspace" => {
                        self.font_dropdown_query.pop();
                        self.font_dropdown_scroll = 0;
                        cx.notify();
                    }
                    "up" => {
                        self.font_dropdown_scroll = self.font_dropdown_scroll.saturating_sub(1);
                        cx.notify();
                    }
                    "down" => {
                        let q = self.font_dropdown_query.trim().to_lowercase();
                        let count = if q.is_empty() {
                            self.system_fonts.len()
                        } else {
                            self.system_fonts
                                .iter()
                                .filter(|f| f.to_lowercase().contains(&q))
                                .count()
                        };
                        let max_scroll = count.saturating_sub(8);
                        self.font_dropdown_scroll = (self.font_dropdown_scroll + 1).min(max_scroll);
                        cx.notify();
                    }
                    _ => {
                        if !is_ctrl && let Some(text) = extract_typed_text(&event.keystroke) {
                            self.font_dropdown_query.push_str(&text);
                            self.font_dropdown_scroll = 0;
                            cx.notify();
                        }
                    }
                }
                return;
            }
            if key == "escape" {
                self.show_settings = false;
                cx.notify();
            }
            return;
        }

        // 3. Find in File 搜索栏状态下的交互
        if self.find_state.is_open && self.find_focused && !self.terminal_focused {
            if key == "escape" {
                self.find_state.close();
                self.find_focused = false;
                cx.notify();
                return;
            }
            if is_ctrl && (key == "f" || key == "F") {
                self.find_state.close();
                self.find_focused = false;
                cx.notify();
                return;
            }
            if key == "enter" {
                if is_shift {
                    self.prev_find_match(cx);
                } else {
                    self.next_find_match(cx);
                }
                return;
            }
            if key == "backspace" {
                self.find_state.query.pop();
                self.update_find_matches();
                cx.notify();
                return;
            }
            if !is_ctrl && let Some(text) = extract_typed_text(&event.keystroke) {
                self.find_state.query.push_str(&text);
                self.update_find_matches();
                cx.notify();
                return;
            }
        }

        // 4. 终端获得焦点时的输入与交互
        if self.show_terminal && self.terminal_focused {
            if is_ctrl {
                match key {
                    "`" | "~" => {
                        self.toggle_terminal(cx);
                        return;
                    }
                    "t" | "T" => {
                        self.new_terminal_tab(cx);
                        return;
                    }
                    "w" | "W" => {
                        let cur = self.active_terminal;
                        self.close_terminal_tab(cur, cx);
                        return;
                    }
                    "p" | "P" => {
                        self.open_file_finder(cx);
                        return;
                    }
                    "," => {
                        self.toggle_settings_modal(cx);
                        return;
                    }
                    "b" | "B" => {
                        self.show_sidebar = !self.show_sidebar;
                        cx.notify();
                        return;
                    }
                    "c" | "C" => {
                        if let Some(term) = self.current_terminal_mut() {
                            term.interrupt();
                            cx.notify();
                        }
                        return;
                    }
                    "l" | "L" => {
                        if let Some(term) = self.current_terminal_mut() {
                            term.clear();
                            cx.notify();
                        }
                        return;
                    }
                    "u" | "U" => {
                        if let Some(term) = self.current_terminal_mut() {
                            term.clear_input_line();
                            cx.notify();
                        }
                        return;
                    }
                    "a" | "A" => {
                        if let Some(term) = self.current_terminal_mut() {
                            term.move_cursor_home();
                            cx.notify();
                        }
                        return;
                    }
                    "e" | "E" => {
                        if let Some(term) = self.current_terminal_mut() {
                            term.move_cursor_end();
                            cx.notify();
                        }
                        return;
                    }
                    "v" | "V" => {
                        if let Some(item) = cx.read_from_clipboard()
                            && let Some(text) = item.text()
                        {
                            let clean = text.replace("\r\n", " ").replace('\n', " ");
                            if let Some(term) = self.current_terminal_mut() {
                                term.insert_str(&clean);
                                cx.notify();
                            }
                            return;
                        }
                    }
                    _ => {}
                }
            } else if let Some(term) = self.current_terminal_mut() {
                match key {
                    "enter" => {
                        let cmd = term.input_buffer.clone();
                        term.input_buffer.clear();
                        term.cursor_pos = 0;
                        if cmd.trim().eq_ignore_ascii_case("exit") {
                            let cur = self.active_terminal;
                            self.close_terminal_tab(cur, cx);
                            return;
                        }
                        term.send_input(&cmd);
                        self.poll_active_terminals(cx);
                        cx.notify();
                        return;
                    }
                    "backspace" => {
                        term.delete_backspace();
                        cx.notify();
                        return;
                    }
                    "delete" => {
                        term.delete_forward();
                        cx.notify();
                        return;
                    }
                    "left" => {
                        term.move_cursor_left();
                        cx.notify();
                        return;
                    }
                    "right" => {
                        term.move_cursor_right();
                        cx.notify();
                        return;
                    }
                    "home" => {
                        term.move_cursor_home();
                        cx.notify();
                        return;
                    }
                    "end" => {
                        term.move_cursor_end();
                        cx.notify();
                        return;
                    }
                    "tab" => {
                        term.auto_complete();
                        cx.notify();
                        return;
                    }
                    "up" => {
                        term.history_prev();
                        cx.notify();
                        return;
                    }
                    "down" => {
                        term.history_next();
                        cx.notify();
                        return;
                    }
                    "pageup" => {
                        term.scroll_up(15);
                        cx.notify();
                        return;
                    }
                    "pagedown" => {
                        term.scroll_down(15);
                        cx.notify();
                        return;
                    }
                    "escape" => {
                        self.terminal_focused = false;
                        cx.notify();
                        return;
                    }
                    _ => {
                        if let Some(text) = extract_typed_text(&event.keystroke) {
                            term.insert_str(&text);
                            cx.notify();
                            return;
                        }
                    }
                }
            }
        }

        // 5. 全局核心快捷键处理 (Ctrl + ...)
        if is_ctrl {
            match key {
                "p" | "P" => {
                    self.open_file_finder(cx);
                    return;
                }
                "f" | "F" => {
                    self.open_find_bar(cx);
                    return;
                }
                "`" | "~" => {
                    self.toggle_terminal(cx);
                    return;
                }
                "," => {
                    self.toggle_settings_modal(cx);
                    return;
                }
                "b" | "B" => {
                    self.show_sidebar = !self.show_sidebar;
                    cx.notify();
                    return;
                }
                "o" | "O" => {
                    self.open_file_dialog(cx);
                    return;
                }
                "s" | "S" => {
                    self.save_current_file(cx);
                    return;
                }
                "n" | "N" => {
                    self.new_tab(cx);
                    return;
                }
                "w" | "W" => {
                    self.close_tab(self.active_tab, cx);
                    return;
                }
                "q" | "Q" => {
                    cx.quit();
                    return;
                }
                "z" | "Z" => {
                    if is_shift {
                        if let Some(buf) = self.current_buffer_mut() {
                            buf.redo();
                        }
                    } else if let Some(buf) = self.current_buffer_mut() {
                        buf.undo();
                    }
                    cx.notify();
                    return;
                }
                "y" | "Y" => {
                    if let Some(buf) = self.current_buffer_mut() {
                        buf.redo();
                    }
                    cx.notify();
                    return;
                }
                "c" | "C" => {
                    self.copy_selection(cx);
                    return;
                }
                "x" | "X" => {
                    self.cut_selection(cx);
                    return;
                }
                "v" | "V" => {
                    self.paste_clipboard(cx);
                    return;
                }
                "a" | "A" => {
                    if let Some(buf) = self.current_buffer_mut() {
                        buf.select_all();
                    }
                    cx.notify();
                    return;
                }
                "d" | "D" => {
                    if let Some(buf) = self.current_buffer_mut() {
                        buf.duplicate_line();
                    }
                    cx.notify();
                    return;
                }
                "/" => {
                    if let Some(buf) = self.current_buffer_mut() {
                        buf.toggle_comment();
                    }
                    cx.notify();
                    return;
                }
                _ => {}
            }
        }

        // 6. 编辑器正文按键输入处理
        let tab_size = self.settings.tab_size;
        if let Some(buf) = self.current_buffer_mut() {
            match key {
                "backspace" => buf.backspace(),
                "delete" => buf.delete(),
                "enter" => {
                    // 智能缩进处理
                    let pos = buf.cursor_position();
                    let current_line = buf.line(pos.row).map(|s| s.to_string()).unwrap_or_default();
                    let indent = current_line
                        .chars()
                        .take_while(|c| *c == ' ' || *c == '\t')
                        .collect::<String>();
                    let extra_indent = if current_line.trim_end().ends_with('{') {
                        " ".repeat(tab_size)
                    } else {
                        String::new()
                    };
                    buf.insert(&format!("\n{}{}", indent, extra_indent));
                }
                "tab" => {
                    let spaces = " ".repeat(tab_size);
                    buf.insert(&spaces);
                }
                "left" => buf.move_left(is_shift),
                "right" => buf.move_right(is_shift),
                "up" => buf.move_up(is_shift),
                "down" => buf.move_down(is_shift),
                "home" => buf.move_home(is_shift),
                "end" => buf.move_end(is_shift),
                _ => {
                    if !is_ctrl && let Some(text) = extract_typed_text(&event.keystroke) {
                        buf.insert(&text);
                    }
                }
            }

            // 光标视野跟踪 (滚动视口)
            let cursor_row = buf.cursor_position().row;
            if cursor_row < self.scroll_top {
                self.scroll_top = cursor_row;
            } else if cursor_row >= self.scroll_top + 30 {
                self.scroll_top = cursor_row.saturating_sub(29);
            }
        }

        cx.notify();
    }
}

impl EditorView {
    fn render_sidebar(&self, project_name: &str, cx: &mut Context<Self>) -> AnyElement {
        let theme = &self.theme;
        let mut file_items = Vec::new();
        for (idx, item) in self.sidebar.items.iter().enumerate() {
            let path = item.path.clone();
            let is_dir = item.is_dir;
            let is_expanded = item.is_expanded;
            let depth = item.depth;
            let icon = Sidebar::file_icon(is_dir, is_expanded, &item.name);

            let indent_px = 10.0 + (depth as f32) * 12.0;
            let arrow = if is_dir {
                if is_expanded { "⌄ " } else { "› " }
            } else {
                "  "
            };

            let item_div = div()
                .h_6()
                .pl(px(indent_px))
                .pr_2()
                .mx_1p5()
                .rounded_xs()
                .flex()
                .items_center()
                .gap_1p5()
                .cursor_pointer()
                .hover(|s| s.bg(theme.bg_hover))
                .text_color(if is_dir {
                    theme.text_primary
                } else {
                    theme.text_secondary
                })
                .text_xs()
                .child(
                    div()
                        .text_color(theme.text_muted)
                        .font_family("Consolas")
                        .child(arrow),
                )
                .child(
                    div()
                        .w(px(20.0))
                        .text_xs()
                        .text_color(if is_dir {
                            theme.accent
                        } else {
                            theme.text_muted
                        })
                        .font_family("Consolas")
                        .child(icon),
                )
                .child(div().child(item.name.clone()))
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _win, cx| {
                        if is_dir {
                            this.sidebar.toggle_expand(idx);
                        } else {
                            this.open_file_path(&path, cx);
                        }
                        cx.notify();
                    }),
                )
                .into_any_element();

            file_items.push(item_div);
        }

        let sidebar_body = if file_items.is_empty() {
            div()
                .flex_1()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .px_4()
                .gap_2p5()
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.text_secondary)
                        .text_center()
                        .child("Choose an option below to open files"),
                )
                .child(
                    div()
                        .px_3()
                        .py_1()
                        .rounded_xs()
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.bg_hover))
                        .flex()
                        .items_center()
                        .gap_2()
                        .text_xs()
                        .child(div().text_color(theme.text_primary).child("Open Project"))
                        .child(div().text_color(theme.text_muted).child("Ctrl+O"))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _win, cx| {
                                this.open_folder_dialog(cx);
                            }),
                        ),
                )
                .child(
                    div()
                        .w(px(140.0))
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(div().flex_1().h(px(1.0)).bg(theme.border_subtle))
                        .child(div().text_xs().text_color(theme.text_muted).child("or"))
                        .child(div().flex_1().h(px(1.0)).bg(theme.border_subtle)),
                )
                .child(
                    div()
                        .px_3()
                        .py_1()
                        .rounded_xs()
                        .cursor_pointer()
                        .hover(|s| s.bg(theme.bg_hover))
                        .text_xs()
                        .text_color(theme.text_primary)
                        .child("New Untitled File")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _win, cx| {
                                this.new_tab(cx);
                            }),
                        ),
                )
                .into_any_element()
        } else {
            div()
                .flex_1()
                .overflow_hidden()
                .py_1()
                .children(file_items)
                .into_any_element()
        };

        let header = div()
            .h(px(32.0))
            .px_3()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .h_full()
                    .cursor_default()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, win, cx| {
                            if event.click_count >= 2 {
                                this.pending_window_drag = None;
                                crate::window_controls::toggle_maximize(win);
                                cx.notify();
                            } else {
                                this.pending_window_drag = Some(event.position);
                            }
                            cx.stop_propagation();
                        }),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text_primary)
                            .child(project_name.to_string()),
                    ),
            )
            .child(div().flex_1().h_full().cursor_default().on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, win, cx| {
                    if event.click_count >= 2 {
                        this.pending_window_drag = None;
                        crate::window_controls::toggle_maximize(win);
                        cx.notify();
                    } else {
                        this.pending_window_drag = Some(event.position);
                    }
                    cx.stop_propagation();
                }),
            ))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_1()
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
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .child("+")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.new_tab(cx);
                                }),
                            ),
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
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .child("⟳")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.sidebar.refresh();
                                    cx.notify();
                                }),
                            ),
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
                            .text_color(theme.text_secondary)
                            .text_xs()
                            .child("📂")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.open_folder_dialog(cx);
                                }),
                            ),
                    ),
            )
            .into_any_element();

        let footer = div()
            .h(px(26.0))
            .px_2p5()
            .bg(theme.bg_surface)
            .border_t_1()
            .border_color(theme.border_subtle)
            .flex()
            .items_center()
            .justify_between()
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
                    .text_color(theme.text_secondary)
                    .text_xs()
                    .child("◧")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _win, cx| {
                            this.show_sidebar = false;
                            cx.notify();
                        }),
                    ),
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
                    .text_color(theme.text_secondary)
                    .text_xs()
                    .child("⚙")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _win, cx| {
                            this.toggle_settings_modal(cx);
                        }),
                    ),
            )
            .into_any_element();

        div()
            .w(px(self.sidebar_width))
            .h_full()
            .bg(theme.bg_surface)
            .flex()
            .flex_col()
            .child(header)
            .child(sidebar_body)
            .child(footer)
            .into_any_element()
    }

    fn render_title_bar(
        &self,
        project_name: &str,
        is_maximized: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = &self.theme;
        let max_icon = if is_maximized { "⧉" } else { "□" };

        let window_controls = div()
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
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_this, _, win, cx| {
                            crate::window_controls::minimize_window(win);
                            cx.stop_propagation();
                        }),
                    )
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
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_this, _, win, cx| {
                            crate::window_controls::toggle_maximize(win);
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    )
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
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_this, _, _win, cx| {
                            cx.stop_propagation();
                            cx.quit();
                        }),
                    )
                    .child("✕"),
            );

        div()
            .h(px(32.0))
            .w_full()
            .bg(theme.bg_activity_bar)
            .flex()
            .items_center()
            .pl_2p5()
            .pr_0()
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
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.show_sidebar = !this.show_sidebar;
                                    cx.notify();
                                }),
                            ),
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
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.open_folder_dialog(cx);
                                }),
                            ),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .min_w(px(20.0))
                    .cursor_default()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, win, cx| {
                            if event.click_count >= 2 {
                                this.pending_window_drag = None;
                                crate::window_controls::toggle_maximize(win);
                                cx.notify();
                            } else {
                                this.pending_window_drag = Some(event.position);
                            }
                            cx.stop_propagation();
                        }),
                    ),
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
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _win, cx| {
                                            this.open_file_finder(cx);
                                        }),
                                    ),
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
                                    .text_color(if self.show_terminal {
                                        theme.accent
                                    } else {
                                        theme.text_secondary
                                    })
                                    .child(">_")
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _win, cx| {
                                            this.toggle_terminal(cx);
                                        }),
                                    ),
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
                                    .text_color(if self.show_settings {
                                        theme.accent
                                    } else {
                                        theme.text_secondary
                                    })
                                    .child("⚙")
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _win, cx| {
                                            this.toggle_settings_modal(cx);
                                        }),
                                    ),
                            ),
                    )
                    .child(window_controls),
            )
            .into_any_element()
    }

    fn render_tab_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = &self.theme;
        let tab_count = self.tabs.len();
        let can_prev = self.active_tab > 0;
        let can_next = self.active_tab + 1 < tab_count;

        let mut tab_row_items: Vec<AnyElement> = Vec::new();

        tab_row_items.push(
            div()
                .h_full()
                .px_2()
                .border_b_1()
                .border_color(theme.border_subtle)
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .w(px(20.0))
                        .h(px(20.0))
                        .rounded_xs()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .text_xs()
                        .text_color(if can_prev {
                            theme.text_secondary
                        } else {
                            theme.text_muted
                        })
                        .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                        .child("←")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _win, cx| {
                                if this.active_tab > 0 {
                                    this.active_tab -= 1;
                                    this.terminal_focused = false;
                                    cx.notify();
                                }
                            }),
                        ),
                )
                .child(
                    div()
                        .w(px(20.0))
                        .h(px(20.0))
                        .rounded_xs()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .text_xs()
                        .text_color(if can_next {
                            theme.text_secondary
                        } else {
                            theme.text_muted
                        })
                        .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                        .child("→")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _, _win, cx| {
                                if this.active_tab + 1 < this.tabs.len() {
                                    this.active_tab += 1;
                                    this.terminal_focused = false;
                                    cx.notify();
                                }
                            }),
                        ),
                )
                .into_any_element(),
        );

        for (idx, tab) in self.tabs.iter().enumerate() {
            let is_active = idx == self.active_tab;
            let dirty = tab.is_dirty();
            let title = tab.file_name();

            let tab_bg = if is_active {
                theme.bg_tab_active
            } else {
                theme.bg_tab_inactive
            };
            let tab_fg = if is_active {
                theme.text_primary
            } else {
                theme.text_secondary
            };

            let mut tab_btn = div()
                .h_full()
                .px_3()
                .flex()
                .items_center()
                .gap_2()
                .bg(tab_bg)
                .text_color(tab_fg)
                .text_xs()
                .font_weight(if is_active {
                    FontWeight::MEDIUM
                } else {
                    FontWeight::NORMAL
                })
                .cursor_pointer()
                .hover(|s| {
                    s.bg(if is_active {
                        theme.bg_tab_active
                    } else {
                        theme.bg_hover
                    })
                })
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _win, cx| {
                        this.active_tab = idx;
                        this.terminal_focused = false;
                        cx.notify();
                    }),
                );

            if is_active {
                tab_btn = tab_btn
                    .border_t_1()
                    .border_l_1()
                    .border_r_1()
                    .border_color(theme.border_subtle);
            } else {
                tab_btn = tab_btn
                    .border_b_1()
                    .border_r_1()
                    .border_color(theme.border_subtle);
            }

            tab_btn = tab_btn.child(title);

            if dirty {
                tab_btn = tab_btn.child(
                    div()
                        .w_1p5()
                        .h_1p5()
                        .rounded_full()
                        .bg(theme.dirty_indicator),
                );
            }

            tab_btn = tab_btn.child(
                div()
                    .w_4()
                    .h_4()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_xs()
                    .text_color(theme.text_muted)
                    .hover(|s| s.bg(theme.bg_hover).text_color(theme.error))
                    .child("×")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, _win, cx| {
                            this.close_tab(idx, cx);
                        }),
                    ),
            );

            tab_row_items.push(tab_btn.into_any_element());
        }

        tab_row_items.push(
            div()
                .h_full()
                .px_2p5()
                .border_b_1()
                .border_color(theme.border_subtle)
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .text_xs()
                .text_color(theme.text_muted)
                .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                .child("+")
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, _win, cx| {
                        this.new_tab(cx);
                    }),
                )
                .into_any_element(),
        );

        tab_row_items.push(
            div()
                .flex_1()
                .h_full()
                .border_b_1()
                .border_color(theme.border_subtle)
                .into_any_element(),
        );

        div()
            .h(px(32.0))
            .w_full()
            .bg(theme.bg_tab_bar)
            .flex()
            .items_center()
            .children(tab_row_items)
            .into_any_element()
    }

    fn render_breadcrumb_bar(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = &self.theme;
        let current_path_str = self
            .current_buffer()
            .map(|b| {
                if let Some(p) = b.file_path() {
                    if let Some(ref root) = self.sidebar.root_path
                        && let Ok(rel) = Path::new(p).strip_prefix(root)
                    {
                        return rel.to_string_lossy().replace('\\', " › ");
                    }
                    p.replace('\\', " › ")
                } else {
                    "untitled".to_string()
                }
            })
            .unwrap_or_else(|| "untitled".to_string());

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
                    .child(current_path_str),
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
                            .text_color(if self.find_state.is_open {
                                theme.accent
                            } else {
                                theme.text_secondary
                            })
                            .text_xs()
                            .child("⌕")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.open_find_bar(cx);
                                }),
                            ),
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
                            .text_color(if self.settings.show_line_numbers {
                                theme.text_primary
                            } else {
                                theme.text_muted
                            })
                            .text_xs()
                            .child("#")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.settings.show_line_numbers =
                                        !this.settings.show_line_numbers;
                                    this.save_config();
                                    cx.notify();
                                }),
                            ),
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
                            .text_color(if self.show_terminal {
                                theme.accent
                            } else {
                                theme.text_secondary
                            })
                            .text_xs()
                            .child(">_")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.toggle_terminal(cx);
                                }),
                            ),
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
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.toggle_settings_modal(cx);
                                }),
                            ),
                    ),
            )
            .into_any_element()
    }

    fn render_editor_pane(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = &self.theme;
        let line_height = px(self.settings.font_size * self.settings.line_height_factor);
        let font_family = self.settings.font_family.clone();
        let show_line_numbers = self.settings.show_line_numbers;

        let Some(buffer) = self.current_buffer() else {
            return div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme.bg_editor)
                .text_color(theme.text_muted)
                .child("No Open Files — Press Ctrl+P to Open")
                .into_any_element();
        };

        let cursor_pos = buffer.cursor_position();
        let total_lines = buffer.len_lines();
        let visible_line_count = 38;
        let start_line = self.scroll_top;
        let end_line = (start_line + visible_line_count).min(total_lines);
        let lang = buffer.language();

        let selection_opt = buffer.selection();
        let char_w = self.settings.font_size * 0.6;
        let sidebar_w = if self.show_sidebar {
            self.sidebar_width + 2.0
        } else {
            0.0
        };
        let gutter_w = if show_line_numbers { 54.0 } else { 0.0 };
        let left_offset = sidebar_w + gutter_w + 8.0;

        let mut gutter_elements = Vec::new();
        let mut line_elements = Vec::new();

        for row_idx in start_line..end_line {
            let is_cursor_line = row_idx == cursor_pos.row;
            let line_num_str = format!("{}", row_idx + 1);
            let gutter_col = if is_cursor_line {
                theme.text_primary
            } else {
                theme.text_muted
            };
            let gutter_bg = if is_cursor_line {
                theme.bg_active_line
            } else {
                theme.bg_gutter
            };

            if show_line_numbers {
                gutter_elements.push(
                    div()
                        .h(line_height)
                        .bg(gutter_bg)
                        .flex()
                        .items_center()
                        .justify_end()
                        .pr_3()
                        .text_color(gutter_col)
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _win, cx| {
                                if let Some(buf) = this.current_buffer_mut() {
                                    let target = buf.position_to_char(crate::buffer::Position {
                                        row: row_idx,
                                        col: 0,
                                    });
                                    buf.set_cursor(target, false);
                                    cx.notify();
                                }
                            }),
                        )
                        .child(line_num_str)
                        .into_any_element(),
                );
            }

            let line_content = buffer
                .line(row_idx)
                .map(|s| s.to_string())
                .unwrap_or_default();
            let display_text = line_content.trim_end_matches(&['\r', '\n'][..]).to_string();

            let line_bg = if is_cursor_line {
                theme.bg_active_line
            } else {
                theme.bg_editor
            };

            let mut line_box = div()
                .h(line_height)
                .w_full()
                .bg(line_bg)
                .flex()
                .items_center()
                .px_2()
                .cursor_text()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, event: &MouseDownEvent, _win, cx| {
                        if let Some(buf) = this.current_buffer_mut() {
                            let click_x = f32::from(event.position.x);
                            let col = if click_x > left_offset {
                                ((click_x - left_offset) / char_w).round() as usize
                            } else {
                                0
                            };
                            let target =
                                buf.position_to_char(crate::buffer::Position { row: row_idx, col });
                            buf.set_cursor(target, event.modifiers.shift);
                            this.terminal_focused = false;
                            cx.notify();
                        }
                    }),
                );

            let line_start_char = buffer.line_to_char(row_idx);
            let text_chars: Vec<char> = display_text.chars().collect();
            let line_len = text_chars.len();
            let line_end_char = line_start_char + line_len;

            let mut sel_start_col = 0;
            let mut sel_end_col = 0;
            let has_sel = if let Some(sel) = selection_opt {
                if !sel.is_empty() {
                    let r = sel.range();
                    if r.start < line_end_char && r.end > line_start_char {
                        sel_start_col = r.start.saturating_sub(line_start_char).min(line_len);
                        sel_end_col = r.end.saturating_sub(line_start_char).min(line_len);
                        sel_start_col < sel_end_col
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            if is_cursor_line {
                let col = cursor_pos.col.min(line_len);
                let before: String = text_chars[..col].iter().collect();
                let cursor_char: String = if col < line_len {
                    text_chars[col].to_string()
                } else {
                    " ".to_string()
                };
                let after: String = if col < line_len {
                    text_chars[(col + 1)..].iter().collect()
                } else {
                    String::new()
                };

                let before_divs: Vec<AnyElement> = highlight_line(&before, lang)
                    .into_iter()
                    .map(|tok| {
                        div()
                            .text_color(token_color(tok.token_type, theme))
                            .child(tok.text)
                            .into_any_element()
                    })
                    .collect();

                let after_divs: Vec<AnyElement> = highlight_line(&after, lang)
                    .into_iter()
                    .map(|tok| {
                        div()
                            .text_color(token_color(tok.token_type, theme))
                            .child(tok.text)
                            .into_any_element()
                    })
                    .collect();

                line_box = line_box
                    .children(before_divs)
                    .child(
                        div()
                            .bg(theme.cursor_color)
                            .text_color(theme.text_on_accent)
                            .rounded_xs()
                            .child(cursor_char),
                    )
                    .children(after_divs);
            } else if display_text.is_empty() {
                line_box = line_box.child(" ");
            } else if has_sel {
                let before_sel: String = text_chars[..sel_start_col].iter().collect();
                let in_sel: String = text_chars[sel_start_col..sel_end_col].iter().collect();
                let after_sel: String = text_chars[sel_end_col..].iter().collect();

                line_box = line_box
                    .child(div().text_color(theme.text_primary).child(before_sel))
                    .child(
                        div()
                            .bg(theme.selection)
                            .rounded_xs()
                            .text_color(theme.text_primary)
                            .child(in_sel),
                    )
                    .child(div().text_color(theme.text_primary).child(after_sel));
            } else {
                let tok_divs: Vec<AnyElement> = highlight_line(&display_text, lang)
                    .into_iter()
                    .map(|tok| {
                        div()
                            .text_color(token_color(tok.token_type, theme))
                            .child(tok.text)
                            .into_any_element()
                    })
                    .collect();
                line_box = line_box.children(tok_divs);
            }

            line_elements.push(line_box.into_any_element());
        }

        let gutter_div = if show_line_numbers {
            Some(
                div()
                    .w(px(54.0))
                    .h_full()
                    .bg(theme.bg_gutter)
                    .py_1()
                    .font_family(font_family.clone())
                    .children(gutter_elements)
                    .into_any_element(),
            )
        } else {
            None
        };

        let find_overlay = if self.find_state.is_open {
            Some(self.find_state.render(theme, cx).into_any_element())
        } else {
            None
        };

        div()
            .flex_1()
            .relative()
            .flex()
            .bg(theme.bg_editor)
            .overflow_hidden()
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _win, cx| {
                let delta_y = match event.delta {
                    ScrollDelta::Pixels(p) => (p.y / px(20.0)) as i32,
                    ScrollDelta::Lines(l) => l.y as i32,
                };
                if delta_y < 0 {
                    this.scroll_top = this
                        .scroll_top
                        .saturating_add(delta_y.unsigned_abs() as usize);
                } else if delta_y > 0 {
                    this.scroll_top = this.scroll_top.saturating_sub(delta_y as usize);
                }
                cx.notify();
            }))
            .children(gutter_div)
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .py_1()
                    .font_family(font_family)
                    .children(line_elements),
            )
            .children(find_overlay)
            .into_any_element()
    }

    fn render_terminal_dock(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = &self.theme;
        let active_idx = self
            .active_terminal
            .min(self.terminals.len().saturating_sub(1));
        let term = &self.terminals[active_idx];
        let term_font = self.settings.terminal_font_family.clone();
        let term_size = px(self.settings.terminal_font_size);
        let line_h = (self.settings.terminal_font_size * 1.45).ceil().max(20.0);
        let is_focused = self.terminal_focused;
        let is_light = theme.is_light();

        let prompt_lines = term.active_prompt_lines();
        let input_rows = if term.scroll_offset == 0 {
            if term.is_running {
                1
            } else {
                prompt_lines.len().max(1)
            }
        } else {
            0
        };

        let avail_h = (self.terminal_height - 28.0 - 20.0).max(line_h);
        let max_rows = (avail_h / line_h).floor().max(1.0) as usize;
        let visible_count = max_rows.saturating_sub(input_rows);

        let lines = term.lines();
        let total = lines.len();
        let effective_end = total.saturating_sub(term.scroll_offset);
        let start = effective_end.saturating_sub(visible_count);
        let slice = if visible_count == 0 {
            &lines[0..0]
        } else if effective_end > 0 && start < total {
            &lines[start..effective_end.min(total)]
        } else if total > 0 && term.scroll_offset > 0 {
            &lines[0..visible_count.min(total)]
        } else {
            let start_idx = total.saturating_sub(visible_count);
            &lines[start_idx..]
        };

        let build_span_els = |raw: &str, default_fg: Hsla| -> Vec<AnyElement> {
            parse_ansi_spans(raw, is_light)
                .into_iter()
                .map(|sp| {
                    let fg = sp.fg_rgb.map(|c| rgb(c).into()).unwrap_or(default_fg);
                    let mut el = div().text_color(fg).child(sp.text);
                    if let Some(bg) = sp.bg_rgb {
                        el = el.bg(rgb(bg));
                    }
                    el.into_any_element()
                })
                .collect()
        };

        let mut output_divs: Vec<AnyElement> = slice
            .iter()
            .map(|line| {
                let span_els = build_span_els(line, theme.text_primary);
                if span_els.is_empty() {
                    div()
                        .w_full()
                        .h(px(line_h))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .flex()
                        .items_center()
                        .text_color(theme.text_primary)
                        .font_family(term_font.clone())
                        .text_size(term_size)
                        .child(" ")
                        .into_any_element()
                } else {
                    div()
                        .w_full()
                        .h(px(line_h))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .flex()
                        .items_center()
                        .font_family(term_font.clone())
                        .text_size(term_size)
                        .children(span_els)
                        .into_any_element()
                }
            })
            .collect();

        if term.scroll_offset == 0 {
            let chars: Vec<char> = term.input_buffer.chars().collect();
            let c_idx = term.cursor_pos.min(chars.len());
            let before_text: String = chars[..c_idx].iter().collect();
            let after_text: String = chars[c_idx..].iter().collect();
            let cursor_box = div()
                .w(px(7.0))
                .h(px((self.settings.terminal_font_size + 1.0).max(13.0)))
                .bg(if is_focused {
                    theme.cursor_color
                } else {
                    theme.text_muted
                });

            if term.is_running {
                output_divs.push(
                    div()
                        .w_full()
                        .h(px(line_h))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .flex()
                        .items_center()
                        .font_family(term_font.clone())
                        .text_size(term_size)
                        .child(div().text_color(theme.text_primary).child(before_text))
                        .child(cursor_box)
                        .child(div().text_color(theme.text_primary).child(after_text))
                        .into_any_element(),
                );
            } else {
                let last_idx = prompt_lines.len().saturating_sub(1);
                for p_line in &prompt_lines[..last_idx] {
                    let span_els = build_span_els(p_line, theme.text_secondary);
                    output_divs.push(
                        div()
                            .w_full()
                            .h(px(line_h))
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .flex()
                            .items_center()
                            .font_family(term_font.clone())
                            .text_size(term_size)
                            .children(span_els)
                            .into_any_element(),
                    );
                }

                let last_prompt = prompt_lines.last().map(String::as_str).unwrap_or("");
                let prompt_els = build_span_els(last_prompt, theme.text_secondary);
                output_divs.push(
                    div()
                        .w_full()
                        .h(px(line_h))
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .flex()
                        .items_center()
                        .font_family(term_font.clone())
                        .text_size(term_size)
                        .children(prompt_els)
                        .child(div().text_color(theme.text_primary).child(before_text))
                        .child(cursor_box)
                        .child(div().text_color(theme.text_primary).child(after_text))
                        .into_any_element(),
                );
            }
        }

        let mut tab_divs: Vec<AnyElement> = Vec::new();
        for (idx, t) in self.terminals.iter().enumerate() {
            let is_cur = idx == active_idx;
            let status_icon = if t.is_running { "● " } else { ">_ " };
            let status_color = if t.is_running {
                theme.dirty_indicator
            } else if is_cur {
                theme.text_primary
            } else {
                theme.text_muted
            };

            let mut term_tab = div()
                .h_full()
                .px_3()
                .flex()
                .items_center()
                .gap_1p5()
                .cursor_pointer()
                .bg(if is_cur {
                    theme.bg_terminal
                } else {
                    theme.bg_tab_bar
                })
                .hover(|s| {
                    s.bg(if is_cur {
                        theme.bg_terminal
                    } else {
                        theme.bg_hover
                    })
                })
                .text_xs()
                .text_color(if is_cur {
                    theme.text_primary
                } else {
                    theme.text_secondary
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .child(div().text_color(status_color).child(status_icon))
                        .child(t.title.clone()),
                )
                .child(
                    div()
                        .w(px(14.0))
                        .h(px(14.0))
                        .rounded_xs()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(theme.text_muted)
                        .hover(|s| s.bg(theme.bg_hover).text_color(theme.error))
                        .child("×")
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _win, cx| {
                                this.close_terminal_tab(idx, cx);
                            }),
                        ),
                )
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _, _win, cx| {
                        this.active_terminal = idx;
                        this.terminal_focused = true;
                        cx.notify();
                    }),
                );

            if is_cur {
                term_tab = term_tab
                    .border_t_1()
                    .border_l_1()
                    .border_r_1()
                    .border_color(theme.border_subtle);
            } else {
                term_tab = term_tab
                    .border_b_1()
                    .border_r_1()
                    .border_color(theme.border_subtle);
            }

            tab_divs.push(term_tab.into_any_element());
        }

        let dir_name = self.terminals[active_idx]
            .current_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| self.terminals[active_idx].current_dir.display().to_string());

        let header = div()
            .h(px(28.0))
            .w_full()
            .bg(theme.bg_tab_bar)
            .flex()
            .items_center()
            .child(div().h_full().flex().items_center().children(tab_divs))
            .child(
                div()
                    .h_full()
                    .px_2()
                    .border_b_1()
                    .border_color(theme.border_subtle)
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                    .child("+")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _, _win, cx| {
                            this.new_terminal_tab(cx);
                        }),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .border_b_1()
                    .border_color(theme.border_subtle),
            )
            .child(
                div()
                    .h_full()
                    .px_2()
                    .border_b_1()
                    .border_color(theme.border_subtle)
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .child(if self.terminals[active_idx].is_running {
                        div()
                            .px_1p5()
                            .py_0p5()
                            .rounded_xs()
                            .bg(theme.bg_hover)
                            .text_xs()
                            .text_color(theme.dirty_indicator)
                            .child("● running")
                    } else {
                        div()
                            .px_1p5()
                            .py_0p5()
                            .rounded_xs()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(format!("~/{}", dir_name))
                    })
                    .child(if self.terminals[active_idx].is_running {
                        div()
                            .h(px(20.0))
                            .px_2()
                            .rounded_xs()
                            .bg(theme.error)
                            .text_color(rgb(0xffffff))
                            .text_xs()
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .child("■ Stop")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    if let Some(t) = this.current_terminal_mut() {
                                        t.interrupt();
                                    }
                                    cx.notify();
                                }),
                            )
                    } else {
                        div()
                    })
                    .child(if self.terminals[active_idx].scroll_offset > 0 {
                        div()
                            .h(px(20.0))
                            .px_2()
                            .rounded_xs()
                            .bg(theme.accent)
                            .text_color(theme.text_on_accent)
                            .text_xs()
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .child("↓ Latest")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    if let Some(t) = this.current_terminal_mut() {
                                        t.scroll_to_bottom();
                                    }
                                    cx.notify();
                                }),
                            )
                    } else {
                        div()
                    })
                    .child(
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.text_secondary)
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .child("⟳")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.restart_terminal();
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.text_secondary)
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .child("↗")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, _cx| {
                                    if let Some(t) = this.current_terminal() {
                                        t.open_external();
                                    }
                                }),
                            ),
                    )
                    .child(
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.text_secondary)
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .child("⌫")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    if let Some(t) = this.current_terminal_mut() {
                                        t.clear();
                                    }
                                    cx.notify();
                                }),
                            ),
                    )
                    .child(
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.text_secondary)
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.error))
                            .child("✕")
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _, _win, cx| {
                                    this.close_terminal_dock(cx);
                                }),
                            ),
                    ),
            )
            .into_any_element();

        div()
            .h(px(self.terminal_height))
            .w_full()
            .bg(theme.bg_terminal)
            .flex()
            .flex_col()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _win, cx| {
                    this.terminal_focused = true;
                    cx.notify();
                }),
            )
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _win, cx| {
                let delta_y = match event.delta {
                    ScrollDelta::Pixels(p) => (p.y / px(20.0)) as i32,
                    ScrollDelta::Lines(l) => l.y as i32,
                };
                if let Some(t) = this.current_terminal_mut() {
                    if delta_y < 0 {
                        t.scroll_up(3);
                    } else if delta_y > 0 {
                        t.scroll_down(3);
                    }
                    cx.notify();
                }
            }))
            .child(header)
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .p_2p5()
                    .bg(theme.bg_terminal)
                    .children(output_divs),
            )
            .into_any_element()
    }
}

impl Render for EditorView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_maximized = crate::window_controls::is_window_maximized(window);

        for term in &mut self.terminals {
            term.poll_output();
        }

        let project_name = self
            .sidebar
            .root_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Open Recent Project".into());

        let theme = &self.theme;

        let sidebar_element = if self.show_sidebar {
            Some(self.render_sidebar(&project_name, cx))
        } else {
            None
        };

        let sidebar_resizer = if self.show_sidebar {
            let is_resizing = self.resizing_sidebar;
            Some(
                div()
                    .w(px(2.0))
                    .h_full()
                    .cursor_col_resize()
                    .bg(if is_resizing {
                        theme.accent
                    } else {
                        theme.border_subtle
                    })
                    .hover(|s| s.bg(theme.accent))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, _win, cx| {
                            if event.click_count == 2 {
                                this.sidebar_width = 230.0;
                                this.resizing_sidebar = false;
                                this.save_config();
                            } else {
                                this.resizing_sidebar = true;
                            }
                            cx.notify();
                        }),
                    )
                    .into_any_element(),
            )
        } else {
            None
        };

        let right_title_bar = self.render_title_bar(&project_name, is_maximized, cx);
        let tab_bar = self.render_tab_bar(cx);
        let breadcrumb_bar = self.render_breadcrumb_bar(cx);
        let editor_pane = self.render_editor_pane(cx);

        let terminal_resizer = if self.show_terminal {
            let is_resizing = self.resizing_terminal;
            Some(
                div()
                    .h(px(2.0))
                    .w_full()
                    .cursor_row_resize()
                    .bg(if is_resizing {
                        theme.accent
                    } else {
                        theme.border_subtle
                    })
                    .hover(|s| s.bg(theme.accent))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, _win, cx| {
                            if event.click_count == 2 {
                                this.terminal_height = 240.0;
                                this.resizing_terminal = false;
                                this.save_config();
                            } else {
                                this.resizing_terminal = true;
                            }
                            cx.notify();
                        }),
                    )
                    .into_any_element(),
            )
        } else {
            None
        };

        let bottom_terminal = if self.show_terminal && !self.terminals.is_empty() {
            Some(self.render_terminal_dock(cx))
        } else {
            None
        };

        let status_bar = StatusBar::render(
            self.current_buffer(),
            &self.theme,
            &self.settings,
            self.show_sidebar,
            self.show_terminal,
            cx,
        )
        .into_any_element();

        let modal_overlay = if self.file_finder.is_open {
            Some(self.file_finder.render(&self.theme, cx).into_any_element())
        } else if self.show_settings {
            Some(SettingsModal::render(
                &self.settings,
                &self.theme,
                &self.system_fonts,
                self.open_font_dropdown,
                &self.font_dropdown_query,
                self.font_dropdown_scroll,
                cx,
            ))
        } else {
            None
        };

        let mut root = div()
            .key_context("Editor")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            .size_full()
            .bg(self.theme.bg_app)
            .font_family(".SystemUIFont")
            .text_size(px(self.settings.font_size))
            .flex()
            .flex_row();

        if self.resizing_sidebar {
            root = root.cursor_col_resize();
        } else if self.resizing_terminal {
            root = root.cursor_row_resize();
        }

        root.children(sidebar_element)
            .children(sidebar_resizer)
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(right_title_bar)
                    .child(tab_bar)
                    .child(breadcrumb_bar)
                    .child(editor_pane)
                    .children(terminal_resizer)
                    .children(bottom_terminal)
                    .child(status_bar),
            )
            .children(modal_overlay)
    }
}
