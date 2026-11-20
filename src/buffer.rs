use ropey::Rope;
use std::ops::Range;

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
}

pub struct TextBuffer {
    rope: Rope,
    cursor: usize,
    selection: Option<Selection>,
    file_path: Option<String>,
    is_dirty: bool,
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBuffer {
    pub fn new() -> Self {
        Self {
            rope: Rope::new(),
            cursor: 0,
            selection: None,
            file_path: None,
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn from_str(text: &str) -> Self {
        Self {
            rope: Rope::from_str(text),
            cursor: 0,
            selection: None,
            file_path: None,
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    fn push_undo(&mut self) {
        if self.undo_stack.len() >= 120 {
            self.undo_stack.remove(0);
        }
        self.undo_stack.push(HistoryEntry {
            rope: self.rope.clone(),
            cursor: self.cursor,
            selection: self.selection,
        });
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(HistoryEntry {
                rope: self.rope.clone(),
                cursor: self.cursor,
                selection: self.selection,
            });
            self.rope = prev.rope;
            self.cursor = prev.cursor.min(self.rope.len_chars());
            self.selection = prev.selection;
            self.is_dirty = true;
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(HistoryEntry {
                rope: self.rope.clone(),
                cursor: self.cursor,
                selection: self.selection,
            });
            self.rope = next.rope;
            self.cursor = next.cursor.min(self.rope.len_chars());
            self.selection = next.selection;
            self.is_dirty = true;
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

    pub fn position_to_char(&self, pos: Position) -> usize {
        if pos.row >= self.rope.len_lines() {
            return self.rope.len_chars();
        }
        let line_start = self.rope.line_to_char(pos.row);
        let line = self.rope.line(pos.row);
        let max_col = if pos.row + 1 < self.rope.len_lines() {
            line.len_chars().saturating_sub(1)
        } else {
            line.len_chars()
        };
        let col = pos.col.min(max_col);
        (line_start + col).min(self.rope.len_chars())
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
    }

    pub fn select_all(&mut self) {
        let len = self.rope.len_chars();
        self.selection = Some(Selection {
            anchor: 0,
            head: len,
        });
        self.cursor = len;
    }

    pub fn select_range(&mut self, range: Range<usize>) {
        let start = range.start.min(self.rope.len_chars());
        let end = range.end.min(self.rope.len_chars());
        self.selection = Some(Selection {
            anchor: start,
            head: end,
        });
        self.cursor = end;
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
        let content = std::fs::read_to_string(path)?;
        let rope = Rope::from_str(&content);
        Ok(Self {
            rope,
            cursor: 0,
            selection: None,
            file_path: Some(path.to_string_lossy().to_string()),
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        })
    }

    pub fn save_file(&mut self, path: Option<&std::path::Path>) -> std::io::Result<()> {
        let target_path = path
            .map(|p| p.to_path_buf())
            .or_else(|| self.file_path.as_ref().map(std::path::PathBuf::from));

        if let Some(target) = target_path {
            let mut file = std::fs::File::create(&target)?;
            self.rope.write_to(&mut file)?;
            self.file_path = Some(target.to_string_lossy().to_string());
            self.is_dirty = false;
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
        self.push_undo();
        if let Some(sel) = self.selection.take() {
            let range = sel.range();
            let start = range.start.min(self.rope.len_chars());
            let end = range.end.min(self.rope.len_chars());
            self.rope.remove(start..end);
            self.cursor = start;
        }
        self.rope.insert(self.cursor, text);
        self.cursor += text.chars().count();
        self.is_dirty = true;
    }

    #[allow(dead_code)]
    pub fn insert_char(&mut self, ch: char) {
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        self.insert(s);
    }

    pub fn backspace(&mut self) {
        self.push_undo();
        if let Some(sel) = self.selection.take() {
            let range = sel.range();
            let start = range.start.min(self.rope.len_chars());
            let end = range.end.min(self.rope.len_chars());
            self.rope.remove(start..end);
            self.cursor = start;
            self.is_dirty = true;
            return;
        }

        if self.cursor > 0 {
            self.rope.remove((self.cursor - 1)..self.cursor);
            self.cursor -= 1;
            self.is_dirty = true;
        }
    }

    pub fn delete(&mut self) {
        self.push_undo();
        if let Some(sel) = self.selection.take() {
            let range = sel.range();
            let start = range.start.min(self.rope.len_chars());
            let end = range.end.min(self.rope.len_chars());
            self.rope.remove(start..end);
            self.cursor = start;
            self.is_dirty = true;
            return;
        }

        if self.cursor < self.rope.len_chars() {
            self.rope.remove(self.cursor..(self.cursor + 1));
            self.is_dirty = true;
        }
    }

    pub fn duplicate_line(&mut self) {
        self.push_undo();
        let pos = self.cursor_position();
        if let Some(line) = self.line(pos.row) {
            let line_str = line.to_string();
            let has_newline = line_str.ends_with('\n');
            let insert_str = if has_newline {
                line_str.clone()
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
            self.is_dirty = true;
        }
    }

    pub fn toggle_comment(&mut self) {
        self.push_undo();
        let pos = self.cursor_position();
        let line_start = self.rope.line_to_char(pos.row);
        if let Some(line) = self.line(pos.row) {
            let line_str = line.to_string();
            let trimmed = line_str.trim_start();
            let indent_len = line_str.len() - trimmed.len();
            if trimmed.starts_with("//") {
                let comment_len = if trimmed.starts_with("// ") { 3 } else { 2 };
                let rem_start = line_start + indent_len;
                let rem_end = rem_start + comment_len;
                self.rope.remove(rem_start..rem_end);
                self.cursor = self.cursor.saturating_sub(comment_len);
            } else {
                let ins_pos = line_start + indent_len;
                self.rope.insert(ins_pos, "// ");
                self.cursor += 3;
            }
            self.is_dirty = true;
        }
    }

    pub fn move_left(&mut self, extend_selection: bool) {
        if self.cursor > 0 {
            self.set_cursor(self.cursor - 1, extend_selection);
        }
    }

    pub fn move_right(&mut self, extend_selection: bool) {
        if self.cursor < self.rope.len_chars() {
            self.set_cursor(self.cursor + 1, extend_selection);
        }
    }

    pub fn move_up(&mut self, extend_selection: bool) {
        let pos = self.cursor_position();
        if pos.row > 0 {
            let target = self.position_to_char(Position {
                row: pos.row - 1,
                col: pos.col,
            });
            self.set_cursor(target, extend_selection);
        }
    }

    pub fn move_down(&mut self, extend_selection: bool) {
        let pos = self.cursor_position();
        if pos.row + 1 < self.rope.len_lines() {
            let target = self.position_to_char(Position {
                row: pos.row + 1,
                col: pos.col,
            });
            self.set_cursor(target, extend_selection);
        }
    }

    pub fn move_home(&mut self, extend_selection: bool) {
        let pos = self.cursor_position();
        let target = self.rope.line_to_char(pos.row);
        self.set_cursor(target, extend_selection);
    }

    pub fn move_end(&mut self, extend_selection: bool) {
        let pos = self.cursor_position();
        let line = self.rope.line(pos.row);
        let line_start = self.rope.line_to_char(pos.row);
        let len = if pos.row + 1 < self.rope.len_lines() {
            line.len_chars().saturating_sub(1)
        } else {
            line.len_chars()
        };
        self.set_cursor(line_start + len, extend_selection);
    }

    pub fn file_path(&self) -> Option<&str> {
        self.file_path.as_deref()
    }

    pub fn is_dirty(&self) -> bool {
        self.is_dirty
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

    pub fn find_matches(&self, query: &str) -> Vec<Range<usize>> {
        let mut matches = Vec::new();
        if query.is_empty() {
            return matches;
        }
        let full_text = self.rope.to_string();
        let query_lower = query.to_lowercase();
        let full_lower = full_text.to_lowercase();

        let mut start_byte = 0;
        while let Some(byte_idx) = full_lower[start_byte..].find(&query_lower) {
            let match_start_byte = start_byte + byte_idx;
            let match_end_byte = match_start_byte + query_lower.len();

            let char_start = full_text[..match_start_byte].chars().count();
            let char_end = full_text[..match_end_byte].chars().count();
            matches.push(char_start..char_end);

            start_byte = match_end_byte;
            if start_byte >= full_lower.len() {
                break;
            }
        }
        matches
    }
}
