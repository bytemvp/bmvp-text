#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::thread;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn MultiByteToWideChar(
        CodePage: u32,
        dwFlags: u32,
        lpMultiByteStr: *const u8,
        cbMultiByte: i32,
        lpWideCharStr: *mut u16,
        cchWideChar: i32,
    ) -> i32;
}

pub fn decode_oem(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    #[cfg(target_os = "windows")]
    {
        let len = bytes.len() as i32;
        let wide_len =
            unsafe { MultiByteToWideChar(1, 0, bytes.as_ptr(), len, std::ptr::null_mut(), 0) };
        if wide_len > 0 {
            let mut wide_buf = vec![0u16; wide_len as usize];
            unsafe {
                MultiByteToWideChar(1, 0, bytes.as_ptr(), len, wide_buf.as_mut_ptr(), wide_len);
            }
            return String::from_utf16_lossy(&wide_buf);
        }
    }
    String::from_utf8_lossy(bytes).into_owned()
}

pub fn get_system_cmd_banner() -> Vec<String> {
    #[cfg(target_os = "windows")]
    {
        let ver_res = Command::new("cmd.exe")
            .args(["/c", "ver"])
            .creation_flags(CREATE_NO_WINDOW)
            .output();

        let ver_str = if let Ok(out) = ver_res {
            let s = decode_oem(&out.stdout).trim().to_string();
            if !s.is_empty() {
                s
            } else {
                "Microsoft Windows [Version 10.0]".to_string()
            }
        } else {
            "Microsoft Windows [Version 10.0]".to_string()
        };

        vec![
            ver_str,
            "(c) Microsoft Corporation. 保留所有权利。".to_string(),
            String::new(),
        ]
    }
    #[cfg(not(target_os = "windows"))]
    {
        vec!["Terminal Session Initialized".to_string(), String::new()]
    }
}

pub struct SystemTerminal {
    #[allow(dead_code)]
    pub id: usize,
    pub title: String,
    pub output_lines: Vec<String>,
    pub input_buffer: String,
    pub cursor_pos: usize, // character offset in input_buffer
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub current_dir: PathBuf,
    pub scroll_offset: usize,
    pub is_running: bool,
    output_rx: Option<Receiver<String>>,
    running_child_pid: Arc<Mutex<Option<u32>>>,
}

impl SystemTerminal {
    pub fn new(id: usize, title: &str, current_dir: PathBuf) -> Self {
        Self {
            id,
            title: title.to_string(),
            output_lines: get_system_cmd_banner(),
            input_buffer: String::new(),
            cursor_pos: 0,
            history: Vec::new(),
            history_idx: None,
            current_dir,
            scroll_offset: 0,
            is_running: false,
            output_rx: None,
            running_child_pid: Arc::new(Mutex::new(None)),
        }
    }

    pub fn reload_profile(&mut self, default_dir: Option<PathBuf>) {
        self.interrupt();
        if let Some(dir) = default_dir
            && dir.is_dir()
        {
            self.current_dir = dir;
        }
        self.output_lines = get_system_cmd_banner();
        self.input_buffer.clear();
        self.cursor_pos = 0;
        self.history.clear();
        self.history_idx = None;
        self.scroll_offset = 0;
        self.is_running = false;
        self.output_rx = None;
    }

    pub fn set_working_dir(&mut self, dir: PathBuf) {
        if dir.is_dir() {
            self.current_dir = dir;
        }
    }

    pub fn lines(&self) -> &[String] {
        &self.output_lines
    }

    pub fn current_prompt(&self) -> String {
        format!("{}>", self.current_dir.display())
    }

    /// 轮询后台异步命令输出队列，返回是否有新行产生
    pub fn poll_output(&mut self) -> bool {
        let mut had_messages = false;
        if let Some(ref rx) = self.output_rx {
            while let Ok(msg) = rx.try_recv() {
                had_messages = true;
                if msg == "__BMVP_CMD_FINISHED__" {
                    self.is_running = false;
                    self.output_lines.push(String::new());
                } else {
                    self.output_lines.push(msg);
                }
            }
        }
        if !self.is_running {
            self.output_rx = None;
        }
        if self.output_lines.len() > 5000 {
            let drain_count = self.output_lines.len() - 5000;
            self.output_lines.drain(0..drain_count);
        }
        had_messages
    }

    pub fn send_input(&mut self, text: &str) {
        self.scroll_offset = 0;
        let trimmed = text.trim();
        let prompt = self.current_prompt();

        if trimmed.is_empty() {
            self.output_lines.push(prompt);
            self.cursor_pos = 0;
            return;
        }

        self.history.push(trimmed.to_string());
        self.history_idx = None;
        self.output_lines.push(format!("{} {}", prompt, trimmed));
        self.cursor_pos = 0;

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let first = parts[0].to_lowercase();

        // 1. 内置 cls / clear 清屏命令
        if first == "cls" || first == "clear" {
            self.output_lines = get_system_cmd_banner();
            return;
        }

        // 2. 内置 cd 路径跳转命令 (瞬时执行，同步更新当前路径)
        if first == "cd" || first == "chdir" {
            if parts.len() == 1 {
                self.output_lines
                    .push(self.current_dir.display().to_string());
                self.output_lines.push(String::new());
                return;
            }
            let target = &trimmed[parts[0].len()..].trim();
            #[cfg(target_os = "windows")]
            {
                let res = Command::new("cmd.exe")
                    .args(["/c", &format!("cd /d \"{}\" && cd", target)])
                    .current_dir(&self.current_dir)
                    .creation_flags(CREATE_NO_WINDOW)
                    .output();

                if let Ok(out) = res {
                    if out.status.success() {
                        let new_dir = decode_oem(&out.stdout).trim().to_string();
                        if !new_dir.is_empty() {
                            self.current_dir = PathBuf::from(new_dir);
                        }
                    } else {
                        let err = decode_oem(&out.stderr);
                        if !err.is_empty() {
                            for line in err.lines() {
                                self.output_lines.push(line.to_string());
                            }
                        } else {
                            self.output_lines.push("系统找不到指定的路径。".to_string());
                        }
                    }
                }
            }
            #[cfg(not(target_os = "windows"))]
            {
                let target_path = PathBuf::from(target);
                let resolved = if target_path.is_absolute() {
                    target_path
                } else {
                    self.current_dir.join(target_path)
                };
                if resolved.is_dir() {
                    self.current_dir = resolved;
                } else {
                    self.output_lines
                        .push("No such file or directory".to_string());
                }
            }
            self.output_lines.push(String::new());
            return;
        }

        // 3. 盘符切换（如 d: 或 c:）
        if trimmed.len() == 2 && trimmed.ends_with(':') {
            let drive_path = format!("{}\\", trimmed);
            let target = PathBuf::from(&drive_path);
            if target.is_dir() {
                self.current_dir = target;
            }
            self.output_lines.push(String::new());
            return;
        }

        // 4. 异步多线程执行原生系统命令：UI 绝不卡死，实时流式输出
        let (tx, rx) = channel::<String>();
        self.output_rx = Some(rx);
        self.is_running = true;

        let dir = self.current_dir.clone();
        let cmd_str = trimmed.to_string();
        let pid_holder = self.running_child_pid.clone();

        thread::spawn(move || {
            #[cfg(target_os = "windows")]
            let mut cmd = Command::new("cmd.exe");
            #[cfg(target_os = "windows")]
            cmd.args(["/c", &cmd_str]);
            #[cfg(target_os = "windows")]
            cmd.creation_flags(CREATE_NO_WINDOW);

            #[cfg(not(target_os = "windows"))]
            let mut cmd = Command::new("sh");
            #[cfg(not(target_os = "windows"))]
            cmd.args(["-c", &cmd_str]);

            cmd.current_dir(&dir);
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            match cmd.spawn() {
                Ok(mut child) => {
                    let pid = child.id();
                    if let Ok(mut lock) = pid_holder.lock() {
                        *lock = Some(pid);
                    }

                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();

                    let tx_out = tx.clone();
                    let t_out = thread::spawn(move || {
                        if let Some(stdout) = stdout {
                            use std::io::{BufRead, BufReader};
                            let reader = BufReader::new(stdout);
                            for line_bytes in reader.split(b'\n').map_while(Result::ok) {
                                let s = decode_oem(&line_bytes);
                                let trimmed_line = s.trim_end_matches('\r').to_string();
                                let _ = tx_out.send(trimmed_line);
                            }
                        }
                    });

                    let tx_err = tx.clone();
                    let t_err = thread::spawn(move || {
                        if let Some(stderr) = stderr {
                            use std::io::{BufRead, BufReader};
                            let reader = BufReader::new(stderr);
                            for line_bytes in reader.split(b'\n').map_while(Result::ok) {
                                let s = decode_oem(&line_bytes);
                                let trimmed_line = s.trim_end_matches('\r').to_string();
                                let _ = tx_err.send(trimmed_line);
                            }
                        }
                    });

                    let _ = t_out.join();
                    let _ = t_err.join();
                    let _ = child.wait();

                    if let Ok(mut lock) = pid_holder.lock() {
                        *lock = None;
                    }
                    let _ = tx.send("__BMVP_CMD_FINISHED__".to_string());
                }
                Err(e) => {
                    let _ = tx.send(format!("'{}' 执行失败: {}", cmd_str, e));
                    let _ = tx.send("__BMVP_CMD_FINISHED__".to_string());
                }
            }
        });
    }

    pub fn auto_complete(&mut self) {
        let trimmed = self.input_buffer.trim_end();
        if trimmed.is_empty() {
            return;
        }
        let last_word = trimmed.split_whitespace().last().unwrap_or("");
        if last_word.is_empty() {
            return;
        }

        if let Ok(entries) = std::fs::read_dir(&self.current_dir) {
            let mut matches = Vec::new();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.to_lowercase().starts_with(&last_word.to_lowercase()) {
                    matches.push(name);
                }
            }
            if let Some(first_match) = matches.first() {
                let prefix_len = last_word.len();
                let remaining = &first_match[prefix_len..];
                self.input_buffer.push_str(remaining);
                self.cursor_pos = self.input_buffer.chars().count();
            }
        }
    }

    pub fn interrupt(&mut self) {
        if let Ok(mut lock) = self.running_child_pid.lock()
            && let Some(pid) = lock.take()
        {
            #[cfg(target_os = "windows")]
            {
                let _ = Command::new("taskkill")
                    .args(["/F", "/T", "/PID", &pid.to_string()])
                    .creation_flags(CREATE_NO_WINDOW)
                    .output();
            }
            #[cfg(not(target_os = "windows"))]
            {
                let _ = Command::new("kill").args(["-9", &pid.to_string()]).output();
            }
        }
        self.is_running = false;
        self.output_rx = None;
        let prompt = self.current_prompt();
        let input = self.input_buffer.clone();
        self.input_buffer.clear();
        self.cursor_pos = 0;
        self.output_lines
            .push(format!("{} {}{}", prompt, input, "^C"));
        self.output_lines.push(String::new());
        self.scroll_offset = 0;
    }

    pub fn clear(&mut self) {
        self.output_lines = get_system_cmd_banner();
        self.scroll_offset = 0;
        self.input_buffer.clear();
        self.cursor_pos = 0;
    }

    pub fn scroll_up(&mut self, lines: usize) {
        let count = self.output_lines.len();
        self.scroll_offset = (self.scroll_offset + lines).min(count.saturating_sub(1));
    }

    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let next_idx = match self.history_idx {
            Some(idx) => idx.saturating_sub(1),
            None => self.history.len() - 1,
        };
        self.history_idx = Some(next_idx);
        if let Some(cmd) = self.history.get(next_idx) {
            self.input_buffer = cmd.clone();
            self.cursor_pos = self.input_buffer.chars().count();
        }
    }

    pub fn history_next(&mut self) {
        if let Some(idx) = self.history_idx {
            if idx + 1 < self.history.len() {
                let next_idx = idx + 1;
                self.history_idx = Some(next_idx);
                if let Some(cmd) = self.history.get(next_idx) {
                    self.input_buffer = cmd.clone();
                    self.cursor_pos = self.input_buffer.chars().count();
                }
            } else {
                self.history_idx = None;
                self.input_buffer.clear();
                self.cursor_pos = 0;
            }
        }
    }

    pub fn insert_char(&mut self, c: char) {
        let chars: Vec<char> = self.input_buffer.chars().collect();
        let idx = self.cursor_pos.min(chars.len());
        let mut new_chars = Vec::with_capacity(chars.len() + 1);
        new_chars.extend_from_slice(&chars[..idx]);
        new_chars.push(c);
        new_chars.extend_from_slice(&chars[idx..]);
        self.input_buffer = new_chars.into_iter().collect();
        self.cursor_pos += 1;
    }

    pub fn insert_str(&mut self, text: &str) {
        for c in text.chars() {
            if c != '\r' && c != '\n' {
                self.insert_char(c);
            }
        }
    }

    pub fn delete_backspace(&mut self) {
        if self.cursor_pos > 0 {
            let mut chars: Vec<char> = self.input_buffer.chars().collect();
            let idx = self.cursor_pos - 1;
            if idx < chars.len() {
                chars.remove(idx);
                self.input_buffer = chars.into_iter().collect();
                self.cursor_pos -= 1;
            }
        }
    }

    pub fn delete_forward(&mut self) {
        let mut chars: Vec<char> = self.input_buffer.chars().collect();
        if self.cursor_pos < chars.len() {
            chars.remove(self.cursor_pos);
            self.input_buffer = chars.into_iter().collect();
        }
    }

    pub fn move_cursor_left(&mut self) {
        self.cursor_pos = self.cursor_pos.saturating_sub(1);
    }

    pub fn move_cursor_right(&mut self) {
        let char_count = self.input_buffer.chars().count();
        self.cursor_pos = (self.cursor_pos + 1).min(char_count);
    }

    pub fn move_cursor_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_cursor_end(&mut self) {
        self.cursor_pos = self.input_buffer.chars().count();
    }

    pub fn clear_input_line(&mut self) {
        self.input_buffer.clear();
        self.cursor_pos = 0;
    }

    pub fn open_external(&self) {
        let dir = if self.current_dir.is_dir() {
            self.current_dir.clone()
        } else {
            PathBuf::from(".")
        };

        #[cfg(target_os = "windows")]
        {
            // 优先启动 Windows Terminal，若系统未安装则直接打开原生 cmd.exe
            let wt = Command::new("wt.exe").arg("-d").arg(&dir).spawn();

            if wt.is_err() {
                let _ = Command::new("cmd.exe")
                    .args(["/c", "start", "cmd.exe", "/k"])
                    .current_dir(&dir)
                    .spawn();
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = Command::new("x-terminal-emulator")
                .current_dir(&dir)
                .spawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_basic_flow() {
        let mut term = SystemTerminal::new(1, "cmd 1", PathBuf::from("."));
        assert_eq!(term.id, 1);
        assert_eq!(term.title, "cmd 1");
        assert!(!term.lines().is_empty());

        // Test editing
        term.insert_str("echo hello");
        assert_eq!(term.input_buffer, "echo hello");
        assert_eq!(term.cursor_pos, 10);

        term.move_cursor_left();
        assert_eq!(term.cursor_pos, 9);
        term.insert_char('!');
        assert_eq!(term.input_buffer, "echo hell!o");

        term.move_cursor_home();
        assert_eq!(term.cursor_pos, 0);

        term.move_cursor_end();
        assert_eq!(term.cursor_pos, 11);

        term.delete_backspace();
        assert_eq!(term.input_buffer, "echo hell!");

        term.clear_input_line();
        assert_eq!(term.input_buffer, "");
        assert_eq!(term.cursor_pos, 0);
    }

    #[test]
    fn test_terminal_history_and_clear() {
        let mut term = SystemTerminal::new(1, "cmd 1", PathBuf::from("."));
        term.send_input("dir");
        term.send_input("cargo --version");
        assert_eq!(term.history.len(), 2);

        term.history_prev();
        assert_eq!(term.input_buffer, "cargo --version");

        term.history_prev();
        assert_eq!(term.input_buffer, "dir");

        term.history_next();
        assert_eq!(term.input_buffer, "cargo --version");

        term.clear();
        assert_eq!(term.input_buffer, "");
        assert_eq!(term.scroll_offset, 0);
    }
}
