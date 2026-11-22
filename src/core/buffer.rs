use regex::RegexBuilder;
use ropey::Rope;
use std::collections::VecDeque;
use std::ops::Range;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

static NEXT_BUFFER_ID: AtomicUsize = AtomicUsize::new(1);

fn next_buffer_id() -> usize {
    NEXT_BUFFER_ID.fetch_add(1, Ordering::Relaxed)
}

pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditKind {
    InsertChar,
    Backspace,
    Other,
}

const MAX_UNDO_HISTORY: usize = 120;
const COALESCE_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: usize, // 字符索引 (char index)
    pub head: usize,   // 字符索引 (光标位置)
}

impl Selection {
    #[allow(dead_code)]
    pub fn point(pos: usize) -> Self {
        Self {
            anchor: pos,
            head: pos,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    pub fn range(&self) -> Range<usize> {
        let start = self.anchor.min(self.head);
        let end = self.anchor.max(self.head);
        start..end
    }
}

#[derive(Clone)]
struct HistoryEntry {
    rope: Rope,
    cursor: usize,
    selection: Option<Selection>,
    revision: u64,
}

pub struct TextBuffer {
    id: usize,
    rope: Rope,
    cursor: usize,
    selection: Option<Selection>,
    file_path: Option<String>,
    scroll_top: usize,
    scroll_left: f32,
    preferred_col: Option<usize>,
    revision: u64,
    saved_revision: u64,
    last_edit_kind: Option<EditKind>,
    last_edit_at: Option<Instant>,
    undo_stack: VecDeque<HistoryEntry>,
    redo_stack: VecDeque<HistoryEntry>,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            id: next_buffer_id(),
            rope: Rope::new(),
            cursor: 0,
            selection: None,
            file_path: None,
            scroll_top: 0,
            scroll_left: 0.0,
            preferred_col: None,
            revision: 0,
            saved_revision: 0,
            last_edit_kind: None,
            last_edit_at: None,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
        }
    }

    pub fn new_with_path(path: &std::path::Path) -> Self {
        let mut buf = Self::new();
        buf.file_path = Some(path.to_string_lossy().to_string());
        buf
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(text: &str) -> Self {
        Self {
            id: next_buffer_id(),
            rope: Rope::from_str(text),
            cursor: 0,
            selection: None,
            file_path: None,
            scroll_top: 0,
            scroll_left: 0.0,
            preferred_col: None,
            revision: 0,
            saved_revision: 0,
            last_edit_kind: None,
            last_edit_at: None,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
        }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn scroll_top(&self) -> usize {
        self.scroll_top
    }

    pub fn set_scroll_top(&mut self, scroll_top: usize) {
        self.scroll_top = scroll_top;
    }

    pub fn scroll_left(&self) -> f32 {
        self.scroll_left
    }

    pub fn set_scroll_left(&mut self, scroll_left: f32) {
        self.scroll_left = scroll_left.max(0.0);
    }

    fn record_undo(&mut self, kind: EditKind) {
        let now = Instant::now();
        let should_coalesce = matches!(kind, EditKind::InsertChar | EditKind::Backspace)
            && self.last_edit_kind == Some(kind)
            && self
                .last_edit_at
                .is_some_and(|t| now.duration_since(t) <= COALESCE_INTERVAL)
            && !self.undo_stack.is_empty();

        if !should_coalesce {
            if self.undo_stack.len() >= MAX_UNDO_HISTORY {
                self.undo_stack.pop_front();
            }
            self.undo_stack.push_back(HistoryEntry {
                rope: self.rope.clone(),
                cursor: self.cursor,
                selection: self.selection,
                revision: self.revision,
            });
        }
        self.redo_stack.clear();
        self.last_edit_kind = Some(kind);
        self.last_edit_at = Some(now);
        self.revision += 1;
        self.preferred_col = None;
    }

    fn push_undo(&mut self) {
        self.record_undo(EditKind::Other);
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop_back() {
            self.redo_stack.push_back(HistoryEntry {
                rope: self.rope.clone(),
                cursor: self.cursor,
                selection: self.selection,
                revision: self.revision,
            });
            self.rope = prev.rope;
            self.cursor = prev.cursor.min(self.rope.len_chars());
            self.selection = prev.selection;
            self.revision = prev.revision;
            self.last_edit_kind = None;
            self.last_edit_at = None;
            self.preferred_col = None;
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop_back() {
            self.undo_stack.push_back(HistoryEntry {
                rope: self.rope.clone(),
                cursor: self.cursor,
                selection: self.selection,
                revision: self.revision,
            });
            self.rope = next.rope;
            self.cursor = next.cursor.min(self.rope.len_chars());
            self.selection = next.selection;
            self.revision = next.revision;
            self.last_edit_kind = None;
            self.last_edit_at = None;
            self.preferred_col = None;
        }
    }

    #[allow(dead_code)]
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    #[allow(dead_code)]
    pub fn len_lines(&self) -> usize {
        self.rope.len_lines()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    pub fn line(&self, line_idx: usize) -> Option<ropey::RopeSlice<'_>> {
        if line_idx < self.rope.len_lines() {
            Some(self.rope.line(line_idx))
        } else {
            None
        }
    }

    pub fn line_to_char(&self, line_idx: usize) -> usize {
        let line_idx = line_idx.min(self.rope.len_lines());
        self.rope.line_to_char(line_idx)
    }

    #[allow(dead_code)]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn selection(&self) -> Option<Selection> {
        self.selection
    }

    pub fn cursor_position(&self) -> Position {
        self.char_to_position(self.cursor)
    }

    pub fn char_to_position(&self, char_idx: usize) -> Position {
        let char_idx = char_idx.min(self.rope.len_chars());
        let row = self.rope.char_to_line(char_idx);
        let line_start_char = self.rope.line_to_char(row);
        let col = char_idx.saturating_sub(line_start_char);
        Position { row, col }
    }

    pub fn line_content_len(&self, row: usize) -> usize {
        if row >= self.rope.len_lines() {
            return 0;
        }
        let line = self.rope.line(row);
        let mut len = line.len_chars();
        if len > 0 && line.char(len - 1) == '\n' {
            len -= 1;
            if len > 0 && line.char(len - 1) == '\r' {
                len -= 1;
            }
        } else if len > 0 && line.char(len - 1) == '\r' {
            len -= 1;
        }
        len
    }

    pub fn position_to_char(&self, pos: Position) -> usize {
        if pos.row >= self.rope.len_lines() {
            return self.rope.len_chars();
        }
        let line_start = self.rope.line_to_char(pos.row);
        let max_col = self.line_content_len(pos.row);
        let col = pos.col.min(max_col);
        (line_start + col).min(self.rope.len_chars())
    }

    #[allow(dead_code)]
    pub fn len_utf16_cu(&self) -> usize {
        self.rope.len_utf16_cu()
    }

    pub fn char_to_utf16(&self, char_idx: usize) -> usize {
        let clamped = char_idx.min(self.rope.len_chars());
        self.rope.char_to_utf16_cu(clamped)
    }

    pub fn utf16_to_char(&self, utf16_idx: usize) -> usize {
        let clamped = utf16_idx.min(self.rope.len_utf16_cu());
        self.rope.utf16_cu_to_char(clamped)
    }

    pub fn slice_chars(&self, range: Range<usize>) -> String {
        let start = range.start.min(self.rope.len_chars());
        let end = range.end.min(self.rope.len_chars()).max(start);
        self.rope.slice(start..end).to_string()
    }

    pub fn set_cursor(&mut self, char_idx: usize, extend_selection: bool) {
        let char_idx = char_idx.min(self.rope.len_chars());
        if extend_selection {
            let anchor = self.selection.map_or(self.cursor, |s| s.anchor);
            self.selection = Some(Selection {
                anchor,
                head: char_idx,
            });
        } else {
            self.selection = None;
        }
        self.cursor = char_idx;
        self.preferred_col = None;
    }

    pub fn select_all(&mut self) {
        let len = self.rope.len_chars();
        self.selection = Some(Selection {
            anchor: 0,
            head: len,
        });
        self.cursor = len;
        self.preferred_col = None;
    }

    pub fn select_range(&mut self, range: Range<usize>) {
        let start = range.start.min(self.rope.len_chars());
        let end = range.end.min(self.rope.len_chars());
        self.selection = Some(Selection {
            anchor: start,
            head: end,
        });
        self.cursor = end;
        self.preferred_col = None;
    }

    pub fn select_word_at(&mut self, char_idx: usize) {
        let total = self.rope.len_chars();
        if total == 0 {
            return;
        }
        let idx = char_idx.min(total.saturating_sub(1));
        let ch = self.rope.char(idx);
        let target_is_word = is_word_char(ch);
        let is_target = |c: char| -> bool {
            if target_is_word {
                is_word_char(c)
            } else if c.is_whitespace() {
                c.is_whitespace() && c != '\n' && c != '\r'
            } else {
                !is_word_char(c) && !c.is_whitespace()
            }
        };

        if ch == '\n' || ch == '\r' {
            self.set_cursor(char_idx.min(total), false);
            return;
        }

        let mut start = idx;
        while start > 0 && is_target(self.rope.char(start - 1)) {
            start -= 1;
        }
        let mut end = idx + 1;
        while end < total && is_target(self.rope.char(end)) {
            end += 1;
        }
        self.select_range(start..end);
    }

    pub fn select_line_at(&mut self, row: usize) {
        let total_lines = self.rope.len_lines();
        if total_lines == 0 || row >= total_lines {
            return;
        }
        let start = self.rope.line_to_char(row);
        let end = if row + 1 < total_lines {
            self.rope.line_to_char(row + 1)
        } else {
            self.rope.len_chars()
        };
        self.select_range(start..end);
    }

    pub fn selected_text(&self) -> Option<String> {
        self.selection.and_then(|sel| {
            if sel.is_empty() {
                None
            } else {
                let range = sel.range();
                let start = range.start.min(self.rope.len_chars());
                let end = range.end.min(self.rope.len_chars());
                Some(self.rope.slice(start..end).to_string())
            }
        })
    }

    pub fn open_file(path: &std::path::Path) -> std::io::Result<Self> {
        let bytes = std::fs::read(path)?;
        let content = String::from_utf8(bytes).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "File is not valid UTF-8 (binary or unsupported encoding)",
            )
        })?;
        let rope = Rope::from_str(&content);
        Ok(Self {
            id: next_buffer_id(),
            rope,
            cursor: 0,
            selection: None,
            file_path: Some(path.to_string_lossy().to_string()),
            scroll_top: 0,
            scroll_left: 0.0,
            preferred_col: None,
            revision: 0,
            saved_revision: 0,
            last_edit_kind: None,
            last_edit_at: None,
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
        })
    }

    pub fn save_file(&mut self, path: Option<&std::path::Path>) -> std::io::Result<()> {
        let target_path = path
            .map(|p| p.to_path_buf())
            .or_else(|| self.file_path.as_ref().map(std::path::PathBuf::from));

        if let Some(target) = target_path {
            let parent = target.parent().unwrap_or_else(|| std::path::Path::new("."));
            let tmp_name = format!(".bmvp-tmp-{}-{}.tmp", std::process::id(), self.id);
            let tmp_path = parent.join(tmp_name);

            let write_res = (|| -> std::io::Result<()> {
                let mut file = std::fs::File::create(&tmp_path)?;
                self.rope.write_to(&mut file)?;
                use std::io::Write;
                file.flush()?;
                file.sync_all()?;
                Ok(())
            })();

            if let Err(err) = write_res {
                if let Err(rm_err) = std::fs::remove_file(&tmp_path)
                    && rm_err.kind() != std::io::ErrorKind::NotFound
                {
                    crate::core::logging::warn(
                        "buffer",
                        &format!(
                            "failed to remove temp save file '{}': {}",
                            tmp_path.display(),
                            rm_err
                        ),
                    );
                }
                return Err(err);
            }

            if let Err(err) = std::fs::rename(&tmp_path, &target) {
                if let Err(rm_err) = std::fs::remove_file(&tmp_path)
                    && rm_err.kind() != std::io::ErrorKind::NotFound
                {
                    crate::core::logging::warn(
                        "buffer",
                        &format!(
                            "failed to remove temp save file '{}': {}",
                            tmp_path.display(),
                            rm_err
                        ),
                    );
                }
                return Err(err);
            }

            self.file_path = Some(target.to_string_lossy().to_string());
            self.saved_revision = self.revision;
            self.last_edit_kind = None;
            self.last_edit_at = None;
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No file path specified",
            ))
        }
    }

    pub fn file_name(&self) -> String {
        if let Some(ref path) = self.file_path {
            std::path::Path::new(path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "[Untitled]".into())
        } else {
            "[Untitled]".into()
        }
    }

    pub fn insert(&mut self, text: &str) {
        if text.is_empty() && self.selection.is_none() {
            return;
        }
        let is_single_non_ws = text.chars().count() == 1
            && text.chars().next().is_some_and(|c| !c.is_whitespace())
            && self.selection.is_none();
        self.record_undo(if is_single_non_ws {
            EditKind::InsertChar
        } else {
            EditKind::Other
        });
        if let Some(sel) = self.selection.take() {
            let range = sel.range();
            let start = range.start.min(self.rope.len_chars());
            let end = range.end.min(self.rope.len_chars());
            self.rope.remove(start..end);
            self.cursor = start;
        }
        self.rope.insert(self.cursor, text);
        self.cursor += text.chars().count();
    }

    pub fn replace_char_range(&mut self, range: Range<usize>, text: &str) {
        let total = self.rope.len_chars();
        let start = range.start.min(total);
        let end = range.end.min(total).max(start);
        if start == end && text.is_empty() && self.selection.is_none() {
            return;
        }
        let is_single_insert = start == end
            && text.chars().count() == 1
            && text.chars().next().is_some_and(|c| !c.is_whitespace())
            && self.selection.is_none();
        self.record_undo(if is_single_insert {
            EditKind::InsertChar
        } else {
            EditKind::Other
        });
        self.selection = None;
        if start < end {
            self.rope.remove(start..end);
        }
        if !text.is_empty() {
            self.rope.insert(start, text);
        }
        self.cursor = start + text.chars().count();
    }

    pub fn replace_char_range_without_undo(
        &mut self,
        range: Range<usize>,
        text: &str,
    ) -> Range<usize> {
        let total = self.rope.len_chars();
        let start = range.start.min(total);
        let end = range.end.min(total).max(start);
        self.selection = None;
        if start < end {
            self.rope.remove(start..end);
        }
        if !text.is_empty() {
            self.rope.insert(start, text);
        }
        let new_end = start + text.chars().count();
        self.cursor = new_end;
        self.preferred_col = None;
        start..new_end
    }

    #[allow(dead_code)]
    pub fn insert_char(&mut self, ch: char) {
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        self.insert(s);
    }

    pub fn backspace(&mut self) {
        if let Some(sel) = self.selection.take()
            && !sel.is_empty()
        {
            self.push_undo();
            let range = sel.range();
            let start = range.start.min(self.rope.len_chars());
            let end = range.end.min(self.rope.len_chars());
            self.rope.remove(start..end);
            self.cursor = start;
            return;
        }

        if self.cursor > 0 {
            let remove_count = if self.cursor >= 2
                && self.rope.char(self.cursor - 1) == '\n'
                && self.rope.char(self.cursor - 2) == '\r'
            {
                2
            } else {
                1
            };
            self.record_undo(if remove_count == 1 {
                EditKind::Backspace
            } else {
                EditKind::Other
            });
            self.rope.remove((self.cursor - remove_count)..self.cursor);
            self.cursor -= remove_count;
        }
    }

    pub fn delete(&mut self) {
        if let Some(sel) = self.selection.take()
            && !sel.is_empty()
        {
            self.push_undo();
            let range = sel.range();
            let start = range.start.min(self.rope.len_chars());
            let end = range.end.min(self.rope.len_chars());
            self.rope.remove(start..end);
            self.cursor = start;
            return;
        }

        if self.cursor < self.rope.len_chars() {
            self.push_undo();
            let remove_count = if self.cursor + 1 < self.rope.len_chars()
                && self.rope.char(self.cursor) == '\r'
                && self.rope.char(self.cursor + 1) == '\n'
            {
                2
            } else {
                1
            };
            self.rope.remove(self.cursor..(self.cursor + remove_count));
        }
    }

    pub fn delete_word_backward(&mut self) {
        if self.selection.is_some_and(|s| !s.is_empty()) {
            self.backspace();
            return;
        }
        if self.cursor == 0 {
            return;
        }
        let target = self.prev_word_boundary(self.cursor);
        if target < self.cursor {
            self.push_undo();
            self.rope.remove(target..self.cursor);
            self.cursor = target;
            self.selection = None;
        }
    }

    pub fn delete_word_forward(&mut self) {
        if self.selection.is_some_and(|s| !s.is_empty()) {
            self.delete();
            return;
        }
        let total = self.rope.len_chars();
        if self.cursor >= total {
            return;
        }
        let target = self.next_word_boundary(self.cursor);
        if target > self.cursor {
            self.push_undo();
            self.rope.remove(self.cursor..target);
            self.selection = None;
        }
    }

    pub fn indent_lines(&mut self, tab_size: usize) {
        let spaces = " ".repeat(tab_size);
        let Some(sel) = self.selection.filter(|s| !s.is_empty()) else {
            self.insert(&spaces);
            return;
        };
        let range = sel.range();
        let start_row = self
            .rope
            .char_to_line(range.start.min(self.rope.len_chars()));
        let mut end_row = self.rope.char_to_line(range.end.min(self.rope.len_chars()));
        if end_row > start_row && range.end == self.rope.line_to_char(end_row) {
            end_row -= 1;
        }
        if start_row == end_row {
            self.insert(&spaces);
            return;
        }

        self.push_undo();
        for row in (start_row..=end_row).rev() {
            let line_start = self.rope.line_to_char(row);
            self.rope.insert(line_start, &spaces);
        }
        let new_start = self.rope.line_to_char(start_row);
        let new_end = self.rope.line_to_char(end_row) + self.line_content_len(end_row);
        self.selection = Some(Selection {
            anchor: new_start,
            head: new_end,
        });
        self.cursor = new_end;
    }

    pub fn outdent_lines(&mut self, tab_size: usize) {
        let total_lines = self.rope.len_lines();
        if total_lines == 0 {
            return;
        }
        let had_selection = self.selection.is_some_and(|s| !s.is_empty());
        let (start_row, end_row) = if let Some(sel) = self.selection.filter(|s| !s.is_empty()) {
            let range = sel.range();
            let sr = self
                .rope
                .char_to_line(range.start.min(self.rope.len_chars()));
            let mut er = self.rope.char_to_line(range.end.min(self.rope.len_chars()));
            if er > sr && range.end == self.rope.line_to_char(er) {
                er -= 1;
            }
            (sr, er.min(total_lines - 1))
        } else {
            let row = self.cursor_position().row.min(total_lines - 1);
            (row, row)
        };

        let mut removals = Vec::new();
        for row in start_row..=end_row {
            let line_start = self.rope.line_to_char(row);
            let content_len = self.line_content_len(row);
            if content_len == 0 {
                continue;
            }
            let first = self.rope.char(line_start);
            let count = if first == '\t' {
                1
            } else {
                let mut n = 0;
                while n < tab_size && n < content_len && self.rope.char(line_start + n) == ' ' {
                    n += 1;
                }
                n
            };
            if count > 0 {
                removals.push((row, line_start, count));
            }
        }
        if removals.is_empty() {
            return;
        }

        self.push_undo();
        for &(row, line_start, count) in removals.iter().rev() {
            self.rope.remove(line_start..(line_start + count));
            if !had_selection && row == start_row {
                let line_col = self.cursor.saturating_sub(line_start);
                self.cursor = line_start + line_col.saturating_sub(count);
            }
        }
        if had_selection {
            let new_start = self.rope.line_to_char(start_row);
            let new_end = self.rope.line_to_char(end_row) + self.line_content_len(end_row);
            self.selection = Some(Selection {
                anchor: new_start,
                head: new_end,
            });
            self.cursor = new_end;
        }
    }

    pub fn duplicate_line(&mut self) {
        self.push_undo();
        let pos = self.cursor_position();
        if let Some(line) = self.line(pos.row) {
            let line_str = line.to_string();
            let has_newline = line_str.ends_with('\n') || line_str.ends_with('\r');
            let insert_str = if has_newline {
                line_str.clone()
            } else if self.line_ending() == "CRLF" {
                format!("\r\n{}", line_str)
            } else {
                format!("\n{}", line_str)
            };
            let line_end = if pos.row + 1 < self.rope.len_lines() {
                self.rope.line_to_char(pos.row + 1)
            } else {
                self.rope.len_chars()
            };
            self.rope.insert(line_end, &insert_str);
            self.cursor = line_end + insert_str.chars().count();
        }
    }

    pub fn line_comment_prefix(&self) -> &'static str {
        match self.language() {
            "Python" | "TOML" | "Shell" => "#",
            _ => "//",
        }
    }

    pub fn toggle_comment(&mut self) {
        let total_lines = self.rope.len_lines();
        if total_lines == 0 {
            return;
        }

        let had_selection = self.selection.is_some_and(|s| !s.is_empty());
        let (start_row, end_row) = if let Some(sel) = self.selection.filter(|s| !s.is_empty()) {
            let range = sel.range();
            let start = range.start.min(self.rope.len_chars());
            let end = range.end.min(self.rope.len_chars());
            let sr = self.rope.char_to_line(start);
            let mut er = self.rope.char_to_line(end);
            if er > sr && end == self.rope.line_to_char(er) {
                er -= 1;
            }
            (sr, er.min(total_lines - 1))
        } else {
            let row = self.cursor_position().row.min(total_lines - 1);
            (row, row)
        };

        let prefix = self.line_comment_prefix();
        let prefix_with_space = format!("{} ", prefix);
        let prefix_chars = prefix.chars().count();

        let mut row_infos: Vec<(usize, usize, bool, bool)> = Vec::new();
        let mut has_non_empty = false;
        let mut all_commented = true;

        for row in start_row..=end_row {
            let line_start = self.rope.line_to_char(row);
            let content_len = self.line_content_len(row);
            let content = self
                .rope
                .slice(line_start..(line_start + content_len))
                .to_string();
            let indent_chars = content.chars().take_while(|c| c.is_whitespace()).count();
            let after_indent: String = content.chars().skip(indent_chars).collect();
            let is_blank = after_indent.is_empty();
            let has_comment = after_indent.starts_with(prefix);
            let has_space_after = after_indent.starts_with(&prefix_with_space);

            if !is_blank {
                has_non_empty = true;
                if !has_comment {
                    all_commented = false;
                }
            }
            row_infos.push((row, indent_chars, has_comment, has_space_after));
        }

        let is_uncomment = if start_row == end_row {
            row_infos[0].2
        } else {
            has_non_empty && all_commented
        };

        self.push_undo();

        for &(row, indent_chars, has_comment, has_space_after) in row_infos.iter().rev() {
            let line_start = self.rope.line_to_char(row);
            let content_len = self.line_content_len(row);
            let is_blank = indent_chars == content_len;

            if is_uncomment {
                if has_comment {
                    let rem_len = if has_space_after {
                        prefix_chars + 1
                    } else {
                        prefix_chars
                    };
                    let rem_start = line_start + indent_chars;
                    let rem_end = rem_start + rem_len;
                    self.rope.remove(rem_start..rem_end);
                    if !had_selection && row == start_row {
                        if self.cursor >= rem_end {
                            self.cursor -= rem_len;
                        } else if self.cursor > rem_start {
                            self.cursor = rem_start;
                        }
                    }
                }
            } else if start_row == end_row || !is_blank {
                let ins_pos = line_start + indent_chars;
                let ins_len = prefix_with_space.chars().count();
                self.rope.insert(ins_pos, &prefix_with_space);
                if !had_selection && row == start_row && self.cursor >= ins_pos {
                    self.cursor += ins_len;
                }
            }
        }

        if had_selection {
            let new_start = self.rope.line_to_char(start_row);
            let new_end = self.rope.line_to_char(end_row) + self.line_content_len(end_row);
            self.selection = Some(Selection {
                anchor: new_start,
                head: new_end,
            });
            self.cursor = new_end;
        } else {
            self.cursor = self.cursor.min(self.rope.len_chars());
        }
    }

    fn prev_word_boundary(&self, from: usize) -> usize {
        let mut idx = from.min(self.rope.len_chars());
        while idx > 0 {
            let c = self.rope.char(idx - 1);
            if c.is_whitespace() {
                idx -= 1;
            } else {
                break;
            }
        }
        if idx == 0 {
            return 0;
        }
        let target_is_word = is_word_char(self.rope.char(idx - 1));
        while idx > 0 {
            let c = self.rope.char(idx - 1);
            if c.is_whitespace() || is_word_char(c) != target_is_word {
                break;
            }
            idx -= 1;
        }
        idx
    }

    fn next_word_boundary(&self, from: usize) -> usize {
        let total = self.rope.len_chars();
        let mut idx = from.min(total);
        if idx >= total {
            return total;
        }
        let first = self.rope.char(idx);
        if first.is_whitespace() {
            while idx < total && self.rope.char(idx).is_whitespace() {
                idx += 1;
            }
            return idx;
        }
        let target_is_word = is_word_char(first);
        while idx < total {
            let c = self.rope.char(idx);
            if c.is_whitespace() || is_word_char(c) != target_is_word {
                break;
            }
            idx += 1;
        }
        while idx < total && self.rope.char(idx).is_whitespace() {
            let c = self.rope.char(idx);
            if c == '\n' || c == '\r' {
                break;
            }
            idx += 1;
        }
        idx
    }

    pub fn move_word_left(&mut self, extend_selection: bool) {
        let target = self.prev_word_boundary(self.cursor);
        self.set_cursor(target, extend_selection);
    }

    pub fn move_word_right(&mut self, extend_selection: bool) {
        let target = self.next_word_boundary(self.cursor);
        self.set_cursor(target, extend_selection);
    }

    pub fn move_left(&mut self, extend_selection: bool) {
        if self.cursor > 0 {
            let step = if self.cursor >= 2
                && self.rope.char(self.cursor - 1) == '\n'
                && self.rope.char(self.cursor - 2) == '\r'
            {
                2
            } else {
                1
            };
            self.set_cursor(self.cursor - step, extend_selection);
        } else if !extend_selection {
            self.selection = None;
        }
    }

    pub fn move_right(&mut self, extend_selection: bool) {
        let total = self.rope.len_chars();
        if self.cursor < total {
            let step = if self.cursor + 1 < total
                && self.rope.char(self.cursor) == '\r'
                && self.rope.char(self.cursor + 1) == '\n'
            {
                2
            } else {
                1
            };
            self.set_cursor(self.cursor + step, extend_selection);
        } else if !extend_selection {
            self.selection = None;
        }
    }

    pub fn move_up(&mut self, extend_selection: bool) {
        let pos = self.cursor_position();
        let pref = self.preferred_col.unwrap_or(pos.col);
        if pos.row > 0 {
            let target = self.position_to_char(Position {
                row: pos.row - 1,
                col: pref,
            });
            self.set_cursor(target, extend_selection);
            self.preferred_col = Some(pref);
        }
    }

    pub fn move_down(&mut self, extend_selection: bool) {
        let pos = self.cursor_position();
        let pref = self.preferred_col.unwrap_or(pos.col);
        if pos.row + 1 < self.rope.len_lines() {
            let target = self.position_to_char(Position {
                row: pos.row + 1,
                col: pref,
            });
            self.set_cursor(target, extend_selection);
            self.preferred_col = Some(pref);
        }
    }

    pub fn move_page_up(&mut self, lines: usize, extend_selection: bool) {
        let pos = self.cursor_position();
        let pref = self.preferred_col.unwrap_or(pos.col);
        let target_row = pos.row.saturating_sub(lines.max(1));
        let target = self.position_to_char(Position {
            row: target_row,
            col: pref,
        });
        self.set_cursor(target, extend_selection);
        self.preferred_col = Some(pref);
    }

    pub fn move_page_down(&mut self, lines: usize, extend_selection: bool) {
        let total_lines = self.rope.len_lines();
        if total_lines == 0 {
            return;
        }
        let pos = self.cursor_position();
        let pref = self.preferred_col.unwrap_or(pos.col);
        let target_row = (pos.row + lines.max(1)).min(total_lines - 1);
        let target = self.position_to_char(Position {
            row: target_row,
            col: pref,
        });
        self.set_cursor(target, extend_selection);
        self.preferred_col = Some(pref);
    }

    pub fn move_home(&mut self, extend_selection: bool) {
        let pos = self.cursor_position();
        let target = self.rope.line_to_char(pos.row);
        self.set_cursor(target, extend_selection);
    }

    pub fn move_end(&mut self, extend_selection: bool) {
        let pos = self.cursor_position();
        let line_start = self.rope.line_to_char(pos.row);
        let len = self.line_content_len(pos.row);
        self.set_cursor(line_start + len, extend_selection);
    }

    pub fn file_path(&self) -> Option<&str> {
        self.file_path.as_deref()
    }

    pub fn is_dirty(&self) -> bool {
        self.revision != self.saved_revision
    }

    pub fn language(&self) -> &'static str {
        if let Some(ref path) = self.file_path {
            let lower = path.to_lowercase();
            if lower.ends_with(".rs") {
                "Rust"
            } else if lower.ends_with(".toml") {
                "TOML"
            } else if lower.ends_with(".md") {
                "Markdown"
            } else if lower.ends_with(".json") {
                "JSON"
            } else if lower.ends_with(".py") {
                "Python"
            } else if lower.ends_with(".js") || lower.ends_with(".jsx") {
                "JavaScript"
            } else if lower.ends_with(".ts") || lower.ends_with(".tsx") {
                "TypeScript"
            } else if lower.ends_with(".html") {
                "HTML"
            } else if lower.ends_with(".css") {
                "CSS"
            } else if lower.ends_with(".sh") || lower.ends_with(".bash") {
                "Shell"
            } else if lower.ends_with(".c") || lower.ends_with(".h") {
                "C"
            } else if lower.ends_with(".cpp") || lower.ends_with(".hpp") {
                "C++"
            } else {
                "Plain Text"
            }
        } else {
            "Rust"
        }
    }

    pub fn line_ending(&self) -> &'static str {
        if self.rope.len_lines() > 0
            && let Some(first_line) = self.line(0)
        {
            let s = first_line.to_string();
            if s.ends_with("\r\n") {
                return "CRLF";
            }
        }
        "LF"
    }

    fn is_whole_word_boundary(&self, range: &Range<usize>) -> bool {
        let before_ok = if range.start == 0 {
            true
        } else {
            !is_word_char(self.rope.char(range.start - 1))
        };
        let after_ok = if range.end >= self.rope.len_chars() {
            true
        } else {
            !is_word_char(self.rope.char(range.end))
        };
        before_ok && after_ok
    }

    pub fn find_matches_with_options(
        &self,
        query: &str,
        opts: SearchOptions,
    ) -> Result<Vec<Range<usize>>, String> {
        let mut matches = Vec::new();
        if query.is_empty() {
            return Ok(matches);
        }

        if opts.use_regex {
            let pattern = if opts.whole_word {
                format!(r"\b(?:{})\b", query)
            } else {
                query.to_string()
            };
            let re = RegexBuilder::new(&pattern)
                .case_insensitive(!opts.case_sensitive)
                .multi_line(true)
                .build()
                .map_err(|e| format!("Invalid regex: {}", e))?;

            let mut full_text = String::with_capacity(self.rope.len_bytes());
            let mut byte_to_char = Vec::with_capacity(self.rope.len_bytes());
            for (orig_char_idx, ch) in self.rope.chars().enumerate() {
                full_text.push(ch);
                byte_to_char.resize(full_text.len(), orig_char_idx);
            }

            for m in re.find_iter(&full_text) {
                if m.start() == m.end() {
                    continue;
                }
                let char_start = byte_to_char[m.start()];
                let char_end = byte_to_char[m.end() - 1] + 1;
                matches.push(char_start..char_end);
            }
            return Ok(matches);
        }

        if opts.case_sensitive {
            let mut full_text = String::with_capacity(self.rope.len_bytes());
            let mut byte_to_char = Vec::with_capacity(self.rope.len_bytes());
            for (orig_char_idx, ch) in self.rope.chars().enumerate() {
                full_text.push(ch);
                byte_to_char.resize(full_text.len(), orig_char_idx);
            }
            let mut start_byte = 0;
            while let Some(byte_idx) = full_text[start_byte..].find(query) {
                let match_start_byte = start_byte + byte_idx;
                let match_end_byte = match_start_byte + query.len();
                let range = byte_to_char[match_start_byte]..(byte_to_char[match_end_byte - 1] + 1);
                if !opts.whole_word || self.is_whole_word_boundary(&range) {
                    matches.push(range);
                }
                start_byte = match_end_byte;
                if start_byte >= full_text.len() {
                    break;
                }
            }
            return Ok(matches);
        }

        let query_lower = query.to_lowercase();
        if query_lower.is_empty() {
            return Ok(matches);
        }

        let mut lower_text = String::with_capacity(self.rope.len_bytes());
        let mut byte_to_char = Vec::with_capacity(self.rope.len_bytes());
        for (orig_char_idx, ch) in self.rope.chars().enumerate() {
            for lc in ch.to_lowercase() {
                lower_text.push(lc);
            }
            byte_to_char.resize(lower_text.len(), orig_char_idx);
        }

        let mut start_byte = 0;
        while let Some(byte_idx) = lower_text[start_byte..].find(&query_lower) {
            let match_start_byte = start_byte + byte_idx;
            let match_end_byte = match_start_byte + query_lower.len();

            let range = byte_to_char[match_start_byte]..(byte_to_char[match_end_byte - 1] + 1);
            if !opts.whole_word || self.is_whole_word_boundary(&range) {
                matches.push(range);
            }

            start_byte = match_end_byte;
            if start_byte >= lower_text.len() {
                break;
            }
        }
        Ok(matches)
    }

    #[allow(dead_code)]
    pub fn find_matches(&self, query: &str) -> Vec<Range<usize>> {
        self.find_matches_with_options(query, SearchOptions::default())
            .unwrap_or_default()
    }

    pub fn replace_range_match(&mut self, range: Range<usize>, replacement: &str) {
        self.replace_char_range(range, replacement);
    }

    pub fn replace_all_matches(&mut self, ranges: &[Range<usize>], replacement: &str) -> usize {
        if ranges.is_empty() {
            return 0;
        }
        self.push_undo();
        let total = self.rope.len_chars();
        let mut sorted: Vec<Range<usize>> = ranges
            .iter()
            .filter_map(|r| {
                let s = r.start.min(total);
                let e = r.end.min(total);
                (s <= e).then_some(s..e)
            })
            .collect();
        sorted.sort_by_key(|a| std::cmp::Reverse(a.start));
        let count = sorted.len();
        for r in sorted {
            if r.start < r.end {
                self.rope.remove(r.start..r.end);
            }
            if !replacement.is_empty() {
                self.rope.insert(r.start, replacement);
            }
        }
        self.selection = None;
        self.cursor = self.cursor.min(self.rope.len_chars());
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_unique_ids() {
        let a = TextBuffer::new();
        let b = TextBuffer::from_str("hello");
        assert_ne!(a.id(), b.id());
    }

    #[test]
    fn test_find_matches_unicode_and_turkish_i_no_panic() {
        let buf = TextBuffer::from_str("İstanbul hello 你好 İSTANBUL 世界 HELLO");
        let matches = buf.find_matches("hello");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0], 9..14);
        assert_eq!(matches[1], 30..35);

        let turkish = buf.find_matches("İSTANBUL");
        assert_eq!(turkish.len(), 2);
        assert_eq!(turkish[0], 0..8);
        assert_eq!(turkish[1], 18..26);
        assert!(buf.find_matches("istanbul").is_empty());

        let cn = buf.find_matches("你好");
        assert_eq!(cn, vec![15..17]);
    }

    #[test]
    fn test_toggle_comment_unicode_indent_empty_line_and_languages() {
        // 全角空格缩进不会因为字节/字符偏移混用发生 panic 或错位
        let mut buf = TextBuffer::from_str("\u{3000}\u{3000}let x = 1;\n\nlet y = 2;");
        buf.set_cursor(2, false);
        buf.toggle_comment();
        assert_eq!(
            buf.rope.to_string(),
            "\u{3000}\u{3000}// let x = 1;\n\nlet y = 2;"
        );
        assert_eq!(buf.cursor(), 5);

        // 取消注释
        buf.toggle_comment();
        assert_eq!(
            buf.rope.to_string(),
            "\u{3000}\u{3000}let x = 1;\n\nlet y = 2;"
        );
        assert_eq!(buf.cursor(), 2);

        // 空行本身加注释，不应跑到下一行
        let pos_line2 = buf.position_to_char(Position { row: 1, col: 0 });
        buf.set_cursor(pos_line2, false);
        buf.toggle_comment();
        assert_eq!(
            buf.rope.to_string(),
            "\u{3000}\u{3000}let x = 1;\n// \nlet y = 2;"
        );

        // Python 语言应使用 `# ` 注释且支持多行选区注释与反注释
        let mut py = TextBuffer::from_str("a = 1\n\n  b = 2\n");
        py.file_path = Some("main.py".to_string());
        py.select_range(0..py.len_chars());
        py.toggle_comment();
        assert_eq!(py.rope.to_string(), "# a = 1\n\n  # b = 2\n");
        py.toggle_comment();
        assert_eq!(py.rope.to_string(), "a = 1\n\n  b = 2\n");
    }

    #[test]
    fn test_crlf_cursor_movement_and_editing() {
        let mut buf = TextBuffer::from_str("abc\r\ndef\r\n");
        assert_eq!(buf.line_ending(), "CRLF");

        // 第一行 End 应停在 'c' 之后、'\r' 之前 (col = 3)
        buf.move_end(false);
        assert_eq!(buf.cursor_position(), Position { row: 0, col: 3 });

        // 在末尾插入字符，不应破坏 \r\n
        buf.insert("!");
        assert_eq!(buf.rope.to_string(), "abc!\r\ndef\r\n");

        // move_right 应直接跨越 \r\n 到下一行第 0 列
        buf.move_right(false);
        assert_eq!(buf.cursor_position(), Position { row: 1, col: 0 });

        // move_left 应直接跨回上一行末尾内容之后
        buf.move_left(false);
        assert_eq!(buf.cursor_position(), Position { row: 0, col: 4 });

        // 从第 1 行第 0 列 backspace 应一次删除整个 \r\n
        buf.move_right(false);
        buf.backspace();
        assert_eq!(buf.rope.to_string(), "abc!def\r\n");

        // position_to_char 夹紧列号不应落入 \r\n 之间
        let clamped = buf.position_to_char(Position { row: 0, col: 99 });
        assert_eq!(clamped, 7);

        // 在 \r\n 前按 delete 应一次删除整个 \r\n
        buf.set_cursor(clamped, false);
        buf.delete();
        assert_eq!(buf.rope.to_string(), "abc!def");
    }

    #[test]
    fn test_dirty_revision_and_noop_undo() {
        let mut buf = TextBuffer::from_str("hello");
        assert!(!buf.is_dirty());

        // 空操作不压栈、不标脏
        buf.backspace();
        buf.set_cursor(buf.len_chars(), false);
        buf.delete();
        buf.insert("");
        assert!(!buf.is_dirty());

        buf.insert(" world");
        assert!(buf.is_dirty());
        buf.undo();
        assert_eq!(buf.rope.to_string(), "hello");
        assert!(!buf.is_dirty());

        buf.redo();
        assert_eq!(buf.rope.to_string(), "hello world");
        assert!(buf.is_dirty());
    }

    #[test]
    fn test_preferred_col_word_nav_and_multiline_indent() {
        let mut buf = TextBuffer::from_str("long_first_line\nx\nanother_long_line");
        buf.set_cursor(10, false); // row 0, col 10
        buf.move_down(false);
        assert_eq!(buf.cursor_position(), Position { row: 1, col: 1 });
        buf.move_down(false);
        assert_eq!(buf.cursor_position(), Position { row: 2, col: 10 });

        // 双击选词与三击选行
        buf.select_word_at(20);
        assert_eq!(buf.selected_text().as_deref(), Some("another_long_line"));
        buf.select_line_at(0);
        assert_eq!(buf.selected_text().as_deref(), Some("long_first_line\n"));

        // 多行缩进与反缩进
        buf.select_range(0..buf.len_chars());
        buf.indent_lines(2);
        assert_eq!(
            buf.rope.to_string(),
            "  long_first_line\n  x\n  another_long_line"
        );
        buf.outdent_lines(2);
        assert_eq!(
            buf.rope.to_string(),
            "long_first_line\nx\nanother_long_line"
        );

        // 词级删除
        buf.set_cursor(4, false); // long|_first_line
        buf.move_word_right(false);
        assert_eq!(buf.cursor(), 15);
        buf.delete_word_backward();
        assert_eq!(buf.rope.to_string(), "\nx\nanother_long_line");
    }

    #[test]
    fn test_search_options_replace_all_and_utf16() {
        let mut buf = TextBuffer::from_str("Foo foo_bar foo 你好");
        let utf16_end = buf.char_to_utf16(buf.len_chars());
        assert_eq!(buf.utf16_to_char(utf16_end), buf.len_chars());

        let exact = buf
            .find_matches_with_options(
                "foo",
                SearchOptions {
                    case_sensitive: true,
                    whole_word: true,
                    use_regex: false,
                },
            )
            .unwrap();
        assert_eq!(exact, vec![12..15]);

        let regex_matches = buf
            .find_matches_with_options(
                r"foo(_bar)?",
                SearchOptions {
                    case_sensitive: false,
                    whole_word: false,
                    use_regex: true,
                },
            )
            .unwrap();
        assert_eq!(regex_matches.len(), 3);
        let replaced = buf.replace_all_matches(&regex_matches, "baz");
        assert_eq!(replaced, 3);
        assert_eq!(buf.rope.to_string(), "baz baz baz 你好");

        let invalid = buf.find_matches_with_options(
            "[unclosed",
            SearchOptions {
                use_regex: true,
                ..Default::default()
            },
        );
        assert!(invalid.is_err());
    }

    #[test]
    fn test_atomic_save_and_invalid_utf8_open() {
        let unique = format!("bmvp_buf_test_{}_{}", std::process::id(), next_buffer_id());
        let dir = std::env::temp_dir().join(unique);
        std::fs::create_dir_all(&dir).unwrap();

        let text_path = dir.join("sample.txt");
        let mut buf = TextBuffer::from_str("hello atomic");
        buf.move_end(false);
        buf.insert("!");
        assert!(buf.is_dirty());
        buf.save_file(Some(&text_path)).unwrap();
        assert!(!buf.is_dirty());
        assert_eq!(
            std::fs::read_to_string(&text_path).unwrap(),
            "hello atomic!"
        );

        let bin_path = dir.join("binary.bin");
        std::fs::write(&bin_path, [0xff, 0xfe, 0x00, 0x80]).unwrap();
        let err = TextBuffer::open_file(&bin_path).err().expect("should fail");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_undo_redo_duplicate_line_and_crlf_comment() {
        let mut buf = TextBuffer::from_str("let a = 1;\r\nlet b = 2;\r\n");
        buf.file_path = Some("main.rs".to_string());
        buf.move_end(false);
        buf.insert(" // x");
        assert_eq!(buf.rope.to_string(), "let a = 1; // x\r\nlet b = 2;\r\n");

        buf.duplicate_line();
        assert_eq!(
            buf.rope.to_string(),
            "let a = 1; // x\r\nlet a = 1; // x\r\nlet b = 2;\r\n"
        );

        buf.undo();
        assert_eq!(buf.rope.to_string(), "let a = 1; // x\r\nlet b = 2;\r\n");
        buf.redo();
        assert_eq!(
            buf.rope.to_string(),
            "let a = 1; // x\r\nlet a = 1; // x\r\nlet b = 2;\r\n"
        );

        buf.undo();
        buf.insert("!");
        // 新编辑清空 redo 栈
        buf.redo();
        assert_eq!(buf.rope.to_string(), "let a = 1; // x!\r\nlet b = 2;\r\n");

        buf.toggle_comment();
        assert_eq!(
            buf.rope.to_string(),
            "// let a = 1; // x!\r\nlet b = 2;\r\n"
        );
        assert_eq!(buf.line_ending(), "CRLF");
    }
}
