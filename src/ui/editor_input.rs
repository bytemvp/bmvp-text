use crate::platform::window_controls;
use crate::terminal::ansi::strip_ansi;
use crate::ui::editor_view::EditorView;
use crate::ui::find_replace::FindField;
use gpui::*;
use std::ops::Range;

impl EditorView {
    pub(super) fn handle_mouse_move(
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
                window_controls::start_window_drag(window);
            }
            return;
        }
        if self.is_selecting_text {
            let total_chars = self.current_buffer().map_or(0, |b| b.len_chars());
            if let Some((_row, char_idx)) = self.code_layout.hit_test(event.position, total_chars)
                && let Some(buf) = self.current_buffer_mut()
            {
                buf.set_cursor(char_idx, true);
                cx.notify();
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
            let max_h = (win_h - 120.0).max(100.0);
            let new_h = (win_h - 26.0 - y).clamp(80.0, max_h);
            if (self.terminal_height - new_h).abs() > 0.5 {
                self.terminal_height = new_h;
                cx.notify();
            }
        }
    }

    pub(super) fn handle_mouse_up(
        &mut self,
        event: &MouseUpEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.button == MouseButton::Left {
            self.pending_window_drag = None;
            self.is_selecting_text = false;
            if self.resizing_sidebar || self.resizing_terminal {
                self.resizing_sidebar = false;
                self.resizing_terminal = false;
                self.save_config();
                cx.notify();
            }
        }
    }

    pub(super) fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let key = event.keystroke.key.as_str();
        let is_shift = event.keystroke.modifiers.shift;
        let is_ctrl = event.keystroke.modifiers.control || event.keystroke.modifiers.platform;
        let is_alt = event.keystroke.modifiers.alt;

        // 0. 未保存关闭/退出确认弹窗截获
        if self.pending_close.is_some() {
            match key {
                "escape" => self.confirm_cancel(cx),
                "enter" => self.confirm_save(cx),
                _ => {}
            }
            cx.stop_propagation();
            return;
        }

        // 1. File Finder 弹窗状态下的键盘截获
        if self.file_finder.is_open {
            match key {
                "escape" => {
                    self.file_finder.close();
                    cx.stop_propagation();
                    cx.notify();
                }
                "up" => {
                    self.file_finder.select_prev();
                    cx.stop_propagation();
                    cx.notify();
                }
                "down" => {
                    self.file_finder.select_next();
                    cx.stop_propagation();
                    cx.notify();
                }
                "enter" => {
                    if let Some(path) = self.file_finder.selected_path() {
                        self.open_file_path(&path, cx);
                    }
                    self.file_finder.close();
                    cx.stop_propagation();
                    cx.notify();
                }
                "backspace" => {
                    self.file_finder.query.pop();
                    self.file_finder.refilter();
                    cx.stop_propagation();
                    cx.notify();
                }
                _ if is_ctrl || is_alt => {
                    cx.stop_propagation();
                }
                _ => {}
            }
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
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "backspace" => {
                        self.font_dropdown_query.pop();
                        self.font_dropdown_scroll = 0;
                        cx.stop_propagation();
                        cx.notify();
                    }
                    "up" => {
                        self.font_dropdown_scroll = self.font_dropdown_scroll.saturating_sub(1);
                        cx.stop_propagation();
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
                        cx.stop_propagation();
                        cx.notify();
                    }
                    _ if is_ctrl || is_alt => {
                        cx.stop_propagation();
                    }
                    _ => {}
                }
                return;
            }
            if key == "escape" {
                self.show_settings = false;
                cx.notify();
            }
            cx.stop_propagation();
            return;
        }

        // 3. Find/Replace 搜索栏状态下的交互
        if self.find_state.is_open && self.find_focused && !self.terminal_focused {
            match key {
                "escape" => {
                    self.find_state.close();
                    self.find_focused = false;
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "f" | "F" if is_ctrl => {
                    self.find_state.close();
                    self.find_focused = false;
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "h" | "H" if is_ctrl => {
                    self.find_state.show_replace = !self.find_state.show_replace;
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "v" | "V" if is_ctrl => {
                    if let Some(item) = cx.read_from_clipboard()
                        && let Some(text) = item.text()
                    {
                        let clean = text.replace("\r\n", " ").replace('\n', " ");
                        self.find_state.active_text_mut().push_str(&clean);
                        if self.find_state.active_field == FindField::Query {
                            self.update_find_matches();
                        }
                        cx.notify();
                    }
                    cx.stop_propagation();
                    return;
                }
                "tab" if self.find_state.show_replace => {
                    self.find_state.active_field = match self.find_state.active_field {
                        FindField::Query => FindField::Replace,
                        FindField::Replace => FindField::Query,
                    };
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "enter" => {
                    if self.find_state.active_field == FindField::Replace {
                        self.replace_current_match(cx);
                    } else if is_shift {
                        self.prev_find_match(cx);
                    } else {
                        self.next_find_match(cx);
                    }
                    cx.stop_propagation();
                    return;
                }
                "backspace" => {
                    self.find_state.active_text_mut().pop();
                    if self.find_state.active_field == FindField::Query {
                        self.update_find_matches();
                    }
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                _ if !is_ctrl && !is_alt => {
                    return;
                }
                _ => {}
            }
        }

        // 4. 终端获得焦点时的输入与交互
        if self.show_terminal && self.terminal_focused {
            if is_ctrl {
                match key {
                    "`" | "~" => {
                        self.toggle_terminal(cx);
                        cx.stop_propagation();
                        return;
                    }
                    "t" | "T" => {
                        self.new_terminal_tab(cx);
                        cx.stop_propagation();
                        return;
                    }
                    "w" | "W" => {
                        let cur = self.active_terminal;
                        self.close_terminal_tab(cur, cx);
                        cx.stop_propagation();
                        return;
                    }
                    "p" | "P" => {
                        self.open_file_finder(cx);
                        cx.stop_propagation();
                        return;
                    }
                    "," => {
                        self.toggle_settings_modal(cx);
                        cx.stop_propagation();
                        return;
                    }
                    "b" | "B" => {
                        self.show_sidebar = !self.show_sidebar;
                        cx.stop_propagation();
                        cx.notify();
                        return;
                    }
                    "c" | "C" if is_shift => {
                        if let Some(term) = self.current_terminal() {
                            let content = if !term.input_buffer.is_empty() {
                                term.input_buffer.clone()
                            } else {
                                term.lines()
                                    .iter()
                                    .rev()
                                    .take(20)
                                    .collect::<Vec<_>>()
                                    .into_iter()
                                    .rev()
                                    .map(|l| strip_ansi(l))
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            };
                            cx.write_to_clipboard(ClipboardItem::new_string(content));
                            self.status_message = Some(("Copied terminal text".to_string(), false));
                            cx.notify();
                        }
                        cx.stop_propagation();
                        return;
                    }
                    "c" | "C" => {
                        if let Some(term) = self.current_terminal_mut() {
                            term.interrupt();
                            cx.notify();
                        }
                        cx.stop_propagation();
                        return;
                    }
                    "l" | "L" => {
                        if let Some(term) = self.current_terminal_mut() {
                            term.clear();
                            cx.notify();
                        }
                        cx.stop_propagation();
                        return;
                    }
                    "u" | "U" => {
                        if let Some(term) = self.current_terminal_mut() {
                            term.clear_input_line();
                            cx.notify();
                        }
                        cx.stop_propagation();
                        return;
                    }
                    "a" | "A" => {
                        if let Some(term) = self.current_terminal_mut() {
                            term.move_cursor_home();
                            cx.notify();
                        }
                        cx.stop_propagation();
                        return;
                    }
                    "e" | "E" => {
                        if let Some(term) = self.current_terminal_mut() {
                            term.move_cursor_end();
                            cx.notify();
                        }
                        cx.stop_propagation();
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
                        }
                        cx.stop_propagation();
                        return;
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
                            cx.stop_propagation();
                            return;
                        }
                        term.send_input(&cmd);
                        self.ensure_terminal_listener(cx);
                        cx.stop_propagation();
                        cx.notify();
                        return;
                    }
                    "backspace" => {
                        term.delete_backspace();
                        cx.stop_propagation();
                        cx.notify();
                        return;
                    }
                    "delete" => {
                        term.delete_forward();
                        cx.stop_propagation();
                        cx.notify();
                        return;
                    }
                    "left" => {
                        term.move_cursor_left();
                        cx.stop_propagation();
                        cx.notify();
                        return;
                    }
                    "right" => {
                        term.move_cursor_right();
                        cx.stop_propagation();
                        cx.notify();
                        return;
                    }
                    "home" => {
                        term.move_cursor_home();
                        cx.stop_propagation();
                        cx.notify();
                        return;
                    }
                    "end" => {
                        term.move_cursor_end();
                        cx.stop_propagation();
                        cx.notify();
                        return;
                    }
                    "tab" => {
                        term.auto_complete();
                        cx.stop_propagation();
                        cx.notify();
                        return;
                    }
                    "up" => {
                        term.history_prev();
                        cx.stop_propagation();
                        cx.notify();
                        return;
                    }
                    "down" => {
                        term.history_next();
                        cx.stop_propagation();
                        cx.notify();
                        return;
                    }
                    "pageup" => {
                        term.scroll_up(15);
                        cx.stop_propagation();
                        cx.notify();
                        return;
                    }
                    "pagedown" => {
                        term.scroll_down(15);
                        cx.stop_propagation();
                        cx.notify();
                        return;
                    }
                    "escape" => {
                        self.terminal_focused = false;
                        cx.stop_propagation();
                        cx.notify();
                        return;
                    }
                    _ if !is_alt => {
                        return;
                    }
                    _ => {}
                }
            }
        }

        // 5. 全局核心快捷键处理 (Ctrl + ...)
        if is_ctrl {
            match key {
                "p" | "P" => {
                    self.open_file_finder(cx);
                    cx.stop_propagation();
                    return;
                }
                "f" | "F" => {
                    self.open_find_bar(cx);
                    cx.stop_propagation();
                    return;
                }
                "h" | "H" => {
                    self.open_replace_bar(cx);
                    cx.stop_propagation();
                    return;
                }
                "`" | "~" => {
                    self.toggle_terminal(cx);
                    cx.stop_propagation();
                    return;
                }
                "," => {
                    self.toggle_settings_modal(cx);
                    cx.stop_propagation();
                    return;
                }
                "b" | "B" => {
                    self.show_sidebar = !self.show_sidebar;
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "o" | "O" => {
                    self.open_file_dialog(cx);
                    cx.stop_propagation();
                    return;
                }
                "s" | "S" => {
                    self.save_current_file(cx);
                    cx.stop_propagation();
                    return;
                }
                "n" | "N" => {
                    self.new_tab(cx);
                    cx.stop_propagation();
                    return;
                }
                "w" | "W" => {
                    self.request_close_tab(self.active_tab, cx);
                    cx.stop_propagation();
                    return;
                }
                "q" | "Q" => {
                    self.request_quit(cx);
                    cx.stop_propagation();
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
                    self.ime_marked_range = None;
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "y" | "Y" => {
                    if let Some(buf) = self.current_buffer_mut() {
                        buf.redo();
                    }
                    self.ime_marked_range = None;
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "c" | "C" => {
                    self.copy_selection(cx);
                    cx.stop_propagation();
                    return;
                }
                "x" | "X" => {
                    self.cut_selection(cx);
                    cx.stop_propagation();
                    return;
                }
                "v" | "V" => {
                    self.paste_clipboard(cx);
                    cx.stop_propagation();
                    return;
                }
                "a" | "A" => {
                    if let Some(buf) = self.current_buffer_mut() {
                        buf.select_all();
                    }
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "d" | "D" => {
                    if let Some(buf) = self.current_buffer_mut() {
                        buf.duplicate_line();
                    }
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "/" => {
                    if let Some(buf) = self.current_buffer_mut() {
                        buf.toggle_comment();
                    }
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "left" => {
                    if let Some(buf) = self.current_buffer_mut() {
                        buf.move_word_left(is_shift);
                    }
                    let vp = self.editor_viewport_lines(Some(window));
                    self.ensure_cursor_visible(vp);
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "right" => {
                    if let Some(buf) = self.current_buffer_mut() {
                        buf.move_word_right(is_shift);
                    }
                    let vp = self.editor_viewport_lines(Some(window));
                    self.ensure_cursor_visible(vp);
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "backspace" => {
                    if let Some(buf) = self.current_buffer_mut() {
                        buf.delete_word_backward();
                    }
                    let vp = self.editor_viewport_lines(Some(window));
                    self.ensure_cursor_visible(vp);
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "delete" => {
                    if let Some(buf) = self.current_buffer_mut() {
                        buf.delete_word_forward();
                    }
                    let vp = self.editor_viewport_lines(Some(window));
                    self.ensure_cursor_visible(vp);
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                _ => {
                    cx.stop_propagation();
                    return;
                }
            }
        }

        if is_alt {
            cx.stop_propagation();
            return;
        }

        // 6. 编辑器正文命令/导航按键（可打印字符放行至 EntityInputHandler 以支持 IME）
        let tab_size = self.settings.tab_size;
        let vp = self.editor_viewport_lines(Some(window));
        let mut handled_command = true;
        if let Some(buf) = self.current_buffer_mut() {
            match key {
                "backspace" => buf.backspace(),
                "delete" => buf.delete(),
                "enter" => {
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
                    let newline = if buf.line_ending() == "CRLF" {
                        "\r\n"
                    } else {
                        "\n"
                    };
                    buf.insert(&format!("{}{}{}", newline, indent, extra_indent));
                }
                "tab" => {
                    if is_shift {
                        buf.outdent_lines(tab_size);
                    } else {
                        buf.indent_lines(tab_size);
                    }
                }
                "left" => buf.move_left(is_shift),
                "right" => buf.move_right(is_shift),
                "up" => buf.move_up(is_shift),
                "down" => buf.move_down(is_shift),
                "home" => buf.move_home(is_shift),
                "end" => buf.move_end(is_shift),
                "pageup" => buf.move_page_up(vp, is_shift),
                "pagedown" => buf.move_page_down(vp, is_shift),
                "escape" => {
                    self.find_focused = false;
                }
                _ => {
                    handled_command = false;
                }
            }
        } else {
            handled_command = false;
        }

        if handled_command {
            self.status_message = None;
            self.ensure_cursor_visible(vp);
            cx.stop_propagation();
            cx.notify();
        }
    }
}

impl EntityInputHandler for EditorView {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        adjusted_range: &mut Option<Range<usize>>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<String> {
        let buf = self.current_buffer()?;
        let start_char = buf.utf16_to_char(range_utf16.start);
        let end_char = buf.utf16_to_char(range_utf16.end).max(start_char);
        adjusted_range.replace(buf.char_to_utf16(start_char)..buf.char_to_utf16(end_char));
        Some(buf.slice_chars(start_char..end_char))
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<UTF16Selection> {
        if self.pending_close.is_some()
            || self.file_finder.is_open
            || self.show_settings
            || (self.find_state.is_open && self.find_focused && !self.terminal_focused)
            || (self.show_terminal && self.terminal_focused)
        {
            return Some(UTF16Selection {
                range: 0..0,
                reversed: false,
            });
        }
        let buf = self.current_buffer()?;
        if let Some(sel) = buf.selection() {
            let r = sel.range();
            Some(UTF16Selection {
                range: buf.char_to_utf16(r.start)..buf.char_to_utf16(r.end),
                reversed: sel.head < sel.anchor,
            })
        } else {
            let u16 = buf.char_to_utf16(buf.cursor());
            Some(UTF16Selection {
                range: u16..u16,
                reversed: false,
            })
        }
    }

    fn marked_text_range(
        &self,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Range<usize>> {
        let buf = self.current_buffer()?;
        let mr = self.ime_marked_range.as_ref()?;
        Some(buf.char_to_utf16(mr.start)..buf.char_to_utf16(mr.end))
    }

    fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.ime_marked_range.take().is_some() {
            cx.notify();
        }
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_close.is_some() {
            return;
        }
        if self.file_finder.is_open {
            self.file_finder.query.push_str(text);
            self.file_finder.refilter();
            cx.notify();
            return;
        }
        if self.show_settings {
            if self.open_font_dropdown.is_some() {
                self.font_dropdown_query.push_str(text);
                self.font_dropdown_scroll = 0;
                cx.notify();
            }
            return;
        }
        if self.find_state.is_open && self.find_focused && !self.terminal_focused {
            self.find_state.active_text_mut().push_str(text);
            if self.find_state.active_field == FindField::Query {
                self.update_find_matches();
            }
            cx.notify();
            return;
        }
        if self.show_terminal && self.terminal_focused {
            if let Some(term) = self.current_terminal_mut() {
                term.insert_str(text);
                cx.notify();
            }
            return;
        }

        let marked = self.ime_marked_range.take();
        let vp = self.editor_viewport_lines(Some(window));
        if let Some(buf) = self.current_buffer_mut() {
            if let Some(ref r16) = range_utf16 {
                let start = buf.utf16_to_char(r16.start);
                let end = buf.utf16_to_char(r16.end).max(start);
                buf.replace_char_range(start..end, text);
            } else if let Some(mr) = marked {
                buf.replace_char_range(mr, text);
            } else {
                buf.insert(text);
            }
        }
        self.status_message = None;
        self.ensure_cursor_visible(vp);
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_close.is_some()
            || self.file_finder.is_open
            || self.show_settings
            || (self.find_state.is_open && self.find_focused && !self.terminal_focused)
            || (self.show_terminal && self.terminal_focused)
        {
            if !new_text.is_empty() {
                self.replace_text_in_range(range_utf16, new_text, window, cx);
            }
            return;
        }

        let marked = self.ime_marked_range.take();
        let vp = self.editor_viewport_lines(Some(window));
        let mut next_marked = None;
        if let Some(buf) = self.current_buffer_mut() {
            let target_range = if let Some(ref r16) = range_utf16 {
                let s = buf.utf16_to_char(r16.start);
                let e = buf.utf16_to_char(r16.end).max(s);
                s..e
            } else if let Some(mr) = marked {
                mr
            } else if let Some(sel) = buf.selection() {
                sel.range()
            } else {
                let c = buf.cursor();
                c..c
            };

            let new_range = buf.replace_char_range_without_undo(target_range, new_text);
            if !new_text.is_empty() {
                next_marked = Some(new_range.clone());
                if let Some(sel16) = new_selected_range_utf16 {
                    let base_u16 = buf.char_to_utf16(new_range.start);
                    let sel_start = buf.utf16_to_char(base_u16 + sel16.start);
                    let sel_end = buf.utf16_to_char(base_u16 + sel16.end);
                    if sel_start == sel_end {
                        buf.set_cursor(sel_end, false);
                    } else {
                        buf.select_range(sel_start..sel_end);
                    }
                }
            }
        }
        self.ime_marked_range = next_marked;
        self.ensure_cursor_visible(vp);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        _element_bounds: Bounds<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let buf = self.current_buffer()?;
        let char_idx = buf.utf16_to_char(range_utf16.start);
        let pos = buf.char_to_position(char_idx);
        self.code_layout.caret_bounds_for_pos(pos.row, pos.col)
    }

    fn character_index_for_point(
        &mut self,
        point: Point<Pixels>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Option<usize> {
        let buf = self.current_buffer()?;
        let (_, char_idx) = self.code_layout.hit_test(point, buf.len_chars())?;
        Some(buf.char_to_utf16(char_idx))
    }
}
