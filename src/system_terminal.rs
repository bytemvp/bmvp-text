use std::io::{BufRead, BufReader, Write};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const DONE_PREFIX: &str = "__BMVP_DONE__";
const DONE_SUFFIX: &str = "__";
const PROMPT_PREFIX: &str = "__BMVP_PROMPT__";
const PROMPT_SUFFIX: &str = "__";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Pwsh,
    PowerShell,
    Cmd,
    Bash,
}

impl ShellKind {
    pub fn from_config(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "pwsh" => Self::Pwsh,
            "powershell" => Self::PowerShell,
            "cmd" => Self::Cmd,
            "bash" | "sh" => Self::Bash,
            _ => Self::Pwsh,
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            Self::Pwsh => "pwsh",
            Self::PowerShell => "powershell",
            Self::Cmd => "cmd",
            Self::Bash => "bash",
        }
    }

    pub fn executable(self) -> &'static str {
        #[cfg(target_os = "windows")]
        {
            match self {
                Self::Pwsh => "pwsh.exe",
                Self::PowerShell => "powershell.exe",
                Self::Cmd => "cmd.exe",
                Self::Bash => "bash.exe",
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            match self {
                Self::Pwsh => "pwsh",
                Self::PowerShell => "pwsh",
                Self::Cmd => "sh",
                Self::Bash => "bash",
            }
        }
    }
}

fn command_exists(bin: &str) -> bool {
    let mut cmd = Command::new(bin);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

pub fn resolve_available_shell(requested: &str) -> ShellKind {
    let preferred = ShellKind::from_config(requested);
    #[cfg(target_os = "windows")]
    {
        match preferred {
            ShellKind::Pwsh => {
                if command_exists("pwsh.exe") {
                    ShellKind::Pwsh
                } else if command_exists("powershell.exe") {
                    ShellKind::PowerShell
                } else {
                    ShellKind::Cmd
                }
            }
            ShellKind::PowerShell => {
                if command_exists("powershell.exe") {
                    ShellKind::PowerShell
                } else {
                    ShellKind::Cmd
                }
            }
            ShellKind::Bash => {
                if command_exists("bash.exe") {
                    ShellKind::Bash
                } else {
                    ShellKind::Cmd
                }
            }
            ShellKind::Cmd => ShellKind::Cmd,
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        match preferred {
            ShellKind::Pwsh if command_exists("pwsh") => ShellKind::Pwsh,
            ShellKind::Bash if command_exists("bash") => ShellKind::Bash,
            _ => ShellKind::Bash,
        }
    }
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

pub fn decode_base64_utf8(input: &str) -> Option<String> {
    fn val(b: u8) -> Option<u32> {
        match b {
            b'A'..=b'Z' => Some((b - b'A') as u32),
            b'a'..=b'z' => Some((b - b'a' + 26) as u32),
            b'0'..=b'9' => Some((b - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let bytes: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !bytes.len().is_multiple_of(4) {
        return None;
    }

    let mut out = Vec::with_capacity((bytes.len() / 4) * 3);
    for chunk in bytes.as_chunks::<4>().0 {
        let v0 = val(chunk[0])?;
        let v1 = val(chunk[1])?;
        let v2 = if chunk[2] == b'=' {
            None
        } else {
            Some(val(chunk[2])?)
        };
        let v3 = if chunk[3] == b'=' {
            None
        } else {
            Some(val(chunk[3])?)
        };

        let n = (v0 << 18) | (v1 << 12) | (v2.unwrap_or(0) << 6) | v3.unwrap_or(0);
        out.push(((n >> 16) & 0xff) as u8);
        if v2.is_some() {
            out.push(((n >> 8) & 0xff) as u8);
        }
        if v3.is_some() {
            out.push((n & 0xff) as u8);
        }
    }

    String::from_utf8(out).ok()
}

pub fn get_shell_banner(shell: ShellKind) -> Vec<String> {
    match shell {
        ShellKind::Pwsh | ShellKind::PowerShell => Vec::new(),
        ShellKind::Cmd => {
            #[cfg(target_os = "windows")]
            {
                vec![
                    "Microsoft Windows [Command Prompt]".to_string(),
                    String::new(),
                ]
            }
            #[cfg(not(target_os = "windows"))]
            {
                vec!["Terminal Session Initialized".to_string(), String::new()]
            }
        }
        ShellKind::Bash => vec![
            "Bash Terminal Session Initialized".to_string(),
            String::new(),
        ],
    }
}

#[derive(Debug, Clone)]
pub struct AnsiSpan {
    pub text: String,
    pub fg_rgb: Option<u32>,
    pub bg_rgb: Option<u32>,
}

fn ansi_16_fg(code: u32, is_light: bool) -> Option<u32> {
    if is_light {
        match code {
            30 => Some(0x24292e),
            31 => Some(0xcf222e),
            32 => Some(0x1a7f37),
            33 => Some(0x9a6700),
            34 => Some(0x0969da),
            35 => Some(0x8250df),
            36 => Some(0x0e7490),
            37 => Some(0x57606a),
            90 => Some(0x6e7781),
            91 => Some(0xa40e26),
            92 => Some(0x116329),
            93 => Some(0x7d4e00),
            94 => Some(0x0550ae),
            95 => Some(0x6639ba),
            96 => Some(0x155e75),
            97 => Some(0x24292e),
            _ => None,
        }
    } else {
        match code {
            30 => Some(0x4b5563),
            31 => Some(0xf87171),
            32 => Some(0x4ade80),
            33 => Some(0xfacc15),
            34 => Some(0x60a5fa),
            35 => Some(0xc084fc),
            36 => Some(0x22d3ee),
            37 => Some(0xe5e7eb),
            90 => Some(0x9ca3af),
            91 => Some(0xfca5a5),
            92 => Some(0x86efac),
            93 => Some(0xfde047),
            94 => Some(0x93c5fd),
            95 => Some(0xd8b4fe),
            96 => Some(0x67e8f9),
            97 => Some(0xffffff),
            _ => None,
        }
    }
}

fn ansi_16_bg(code: u32) -> Option<u32> {
    match code {
        40 => Some(0x1f2328),
        41 => Some(0xcf222e),
        42 => Some(0x1a7f37),
        43 => Some(0xbf8700),
        44 => Some(0x0969da),
        45 => Some(0x8250df),
        46 => Some(0x0e7490),
        47 => Some(0xd0d7de),
        100 => Some(0x57606a),
        101 => Some(0xfa4549),
        102 => Some(0x2da44e),
        103 => Some(0xd4a72c),
        104 => Some(0x218bff),
        105 => Some(0xa475f9),
        106 => Some(0x1b7c83),
        107 => Some(0xf6f8fa),
        _ => None,
    }
}

fn ansi_256_color(idx: u32) -> u32 {
    if idx < 16 {
        let base = [
            0x000000, 0xcd3131, 0x0dbc79, 0xe5e510, 0x2472c8, 0xbc3fbc, 0x11a8cd, 0xe5e5e5,
            0x666666, 0xf14c4c, 0x23d18b, 0xf5f543, 0x3b8eea, 0xd670d6, 0x29b8db, 0xffffff,
        ];
        return base[idx as usize];
    }
    if idx < 232 {
        let n = idx - 16;
        let r = n / 36;
        let g = (n % 36) / 6;
        let b = n % 6;
        let map = |v: u32| if v == 0 { 0 } else { 55 + v * 40 };
        return (map(r) << 16) | (map(g) << 8) | map(b);
    }
    let gray = 8 + (idx - 232) * 10;
    (gray << 16) | (gray << 8) | gray
}

pub fn strip_ansi(input: &str) -> String {
    let spans = parse_ansi_spans(input, false);
    let mut out = String::new();
    for s in spans {
        out.push_str(&s.text);
    }
    out
}

pub fn parse_ansi_spans(input: &str, is_light: bool) -> Vec<AnsiSpan> {
    let mut spans = Vec::new();
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;
    let mut current_fg: Option<u32> = None;
    let mut current_bg: Option<u32> = None;
    let mut buf = String::new();

    while i < len {
        if bytes[i] == 0x1b {
            if !buf.is_empty() {
                spans.push(AnsiSpan {
                    text: std::mem::take(&mut buf),
                    fg_rgb: current_fg,
                    bg_rgb: current_bg,
                });
            }
            if i + 1 < len && bytes[i + 1] == b'[' {
                // CSI sequence: ESC [ ... <final byte 0x40..=0x7E>
                let mut j = i + 2;
                while j < len && !(0x40..=0x7e).contains(&bytes[j]) {
                    j += 1;
                }
                if j < len {
                    if bytes[j] == b'm' {
                        let params_str = std::str::from_utf8(&bytes[i + 2..j]).unwrap_or("");
                        let nums: Vec<u32> = if params_str.is_empty() {
                            vec![0]
                        } else {
                            params_str
                                .split(';')
                                .filter_map(|p| p.parse::<u32>().ok())
                                .collect()
                        };
                        let mut k = 0;
                        while k < nums.len() {
                            match nums[k] {
                                0 => {
                                    current_fg = None;
                                    current_bg = None;
                                }
                                39 => current_fg = None,
                                49 => current_bg = None,
                                30..=37 | 90..=97 => {
                                    current_fg = ansi_16_fg(nums[k], is_light);
                                }
                                40..=47 | 100..=107 => {
                                    current_bg = ansi_16_bg(nums[k]);
                                }
                                38 if k + 2 < nums.len() && nums[k + 1] == 5 => {
                                    current_fg = Some(ansi_256_color(nums[k + 2]));
                                    k += 2;
                                }
                                48 if k + 2 < nums.len() && nums[k + 1] == 5 => {
                                    current_bg = Some(ansi_256_color(nums[k + 2]));
                                    k += 2;
                                }
                                38 if k + 4 < nums.len() && nums[k + 1] == 2 => {
                                    let r = nums[k + 2].min(255);
                                    let g = nums[k + 3].min(255);
                                    let b = nums[k + 4].min(255);
                                    current_fg = Some((r << 16) | (g << 8) | b);
                                    k += 4;
                                }
                                48 if k + 4 < nums.len() && nums[k + 1] == 2 => {
                                    let r = nums[k + 2].min(255);
                                    let g = nums[k + 3].min(255);
                                    let b = nums[k + 4].min(255);
                                    current_bg = Some((r << 16) | (g << 8) | b);
                                    k += 4;
                                }
                                _ => {}
                            }
                            k += 1;
                        }
                    }
                    i = j + 1;
                    continue;
                }
            } else if i + 1 < len && bytes[i + 1] == b']' {
                // OSC sequence: ESC ] ... BEL or ST
                let mut j = i + 2;
                while j < len {
                    if bytes[j] == 0x07 {
                        j += 1;
                        break;
                    }
                    if bytes[j] == 0x1b && j + 1 < len && bytes[j + 1] == b'\\' {
                        j += 2;
                        break;
                    }
                    j += 1;
                }
                i = j;
                continue;
            }
            i += 1;
            continue;
        }

        if let Some(ch) = input[i..].chars().next() {
            if ch != '\r' {
                buf.push(ch);
            }
            i += ch.len_utf8();
        } else {
            i += 1;
        }
    }

    if !buf.is_empty() {
        spans.push(AnsiSpan {
            text: buf,
            fg_rgb: current_fg,
            bg_rgb: current_bg,
        });
    }

    spans
}

enum TermEvent {
    Line(String),
    CommandDone(Option<PathBuf>),
    Prompt(String),
    Exited,
}

struct ShellSession {
    pid: u32,
    stdin: ChildStdin,
    rx: Receiver<TermEvent>,
    _child: Child,
}

pub struct SystemTerminal {
    #[allow(dead_code)]
    pub id: usize,
    pub title: String,
    pub shell: ShellKind,
    pub output_lines: Vec<String>,
    pub input_buffer: String,
    pub cursor_pos: usize,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub current_dir: PathBuf,
    pub scroll_offset: usize,
    pub is_running: bool,
    pub rendered_prompt: Option<String>,
    session: Option<ShellSession>,
}

impl SystemTerminal {
    pub fn new(id: usize, title: &str, current_dir: PathBuf, shell_name: &str) -> Self {
        let shell = resolve_available_shell(shell_name);
        let mut term = Self {
            id,
            title: title.to_string(),
            shell,
            output_lines: get_shell_banner(shell),
            input_buffer: String::new(),
            cursor_pos: 0,
            history: Vec::new(),
            history_idx: None,
            current_dir,
            scroll_offset: 0,
            is_running: false,
            rendered_prompt: None,
            session: None,
        };
        term.start_session();
        term
    }

    #[cfg(test)]
    pub fn new_offline(id: usize, title: &str, current_dir: PathBuf, shell: ShellKind) -> Self {
        Self {
            id,
            title: title.to_string(),
            shell,
            output_lines: vec!["PowerShell 7".to_string(), String::new()],
            input_buffer: String::new(),
            cursor_pos: 0,
            history: Vec::new(),
            history_idx: None,
            current_dir,
            scroll_offset: 0,
            is_running: false,
            rendered_prompt: None,
            session: None,
        }
    }

    fn spawn_reader(
        reader: impl std::io::Read + Send + 'static,
        tx: Sender<TermEvent>,
        send_exit: bool,
    ) {
        thread::spawn(move || {
            let buf_reader = BufReader::new(reader);
            for line_bytes in buf_reader.split(b'\n').map_while(Result::ok) {
                let raw = decode_oem(&line_bytes);
                let line = raw.trim_end_matches('\r').to_string();
                let clean = strip_ansi(&line);
                let trimmed_clean = clean.trim();

                if let Some(rest) = trimmed_clean.strip_prefix(DONE_PREFIX)
                    && let Some(dir_str) = rest.strip_suffix(DONE_SUFFIX)
                {
                    let dir = PathBuf::from(dir_str.trim());
                    let _ = tx.send(TermEvent::CommandDone(if dir.is_dir() {
                        Some(dir)
                    } else {
                        None
                    }));
                    continue;
                }

                if let Some(rest) = trimmed_clean.strip_prefix(PROMPT_PREFIX)
                    && let Some(b64) = rest.strip_suffix(PROMPT_SUFFIX)
                    && let Some(decoded) = decode_base64_utf8(b64.trim())
                {
                    let _ = tx.send(TermEvent::Prompt(decoded));
                    continue;
                }

                let _ = tx.send(TermEvent::Line(line));
            }
            if send_exit {
                let _ = tx.send(TermEvent::Exited);
            }
        });
    }

    fn ps_prompt_capture_snippet() -> &'static str {
        "$__bmvp_p = if (Test-Path Function:\\prompt) { (prompt) -join \"`n\" } else { \"PS $((Get-Location).Path)> \" }; Write-Output (\"__BMVP_PROMPT__\" + [System.Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($__bmvp_p)) + \"__\")"
    }

    fn start_session(&mut self) {
        self.kill_session();

        let dir = if self.current_dir.is_dir() {
            self.current_dir.clone()
        } else {
            PathBuf::from(".")
        };

        let mut cmd = match self.shell {
            ShellKind::Pwsh | ShellKind::PowerShell => {
                // 直接使用系统/用户自带的 pwsh 配置，不传入 -NoProfile 或新建 profile
                let mut c = Command::new(self.shell.executable());
                c.args(["-NoExit", "-Command", "-"]);
                c
            }
            ShellKind::Cmd => {
                let mut c = Command::new(self.shell.executable());
                #[cfg(target_os = "windows")]
                c.args(["/Q", "/K"]);
                c
            }
            ShellKind::Bash => {
                let mut c = Command::new(self.shell.executable());
                c.arg("-s");
                c
            }
        };

        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        cmd.current_dir(&dir);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Ok(mut child) = cmd.spawn() {
            let pid = child.id();
            if let (Some(mut stdin), Some(stdout), Some(stderr)) =
                (child.stdin.take(), child.stdout.take(), child.stderr.take())
            {
                match self.shell {
                    ShellKind::Pwsh | ShellKind::PowerShell => {
                        let init = format!(
                            "[Console]::InputEncoding = [System.Text.Encoding]::UTF8; [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $OutputEncoding = [System.Text.Encoding]::UTF8; Write-Output \"PowerShell $($PSVersionTable.PSVersion.ToString())\"; {}\n",
                            Self::ps_prompt_capture_snippet()
                        );
                        let _ = stdin.write_all(init.as_bytes());
                        let _ = stdin.flush();
                    }
                    ShellKind::Cmd => {
                        let _ = stdin.write_all(b"@echo off\r\nchcp 65001 > nul\r\n");
                        let _ = stdin.flush();
                    }
                    ShellKind::Bash => {}
                }

                let (tx, rx) = channel::<TermEvent>();
                Self::spawn_reader(stdout, tx.clone(), true);
                Self::spawn_reader(stderr, tx, false);

                self.session = Some(ShellSession {
                    pid,
                    stdin,
                    rx,
                    _child: child,
                });
            }
        }
    }

    fn kill_session(&mut self) {
        if let Some(mut sess) = self.session.take() {
            let pid = sess.pid;
            drop(sess.stdin);
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
            let _ = sess._child.wait();
        }
    }

    pub fn reload_session(&mut self, default_dir: Option<PathBuf>, shell_name: &str) {
        self.kill_session();
        self.shell = resolve_available_shell(shell_name);
        if let Some(dir) = default_dir
            && dir.is_dir()
        {
            self.current_dir = dir;
        }
        self.output_lines = get_shell_banner(self.shell);
        self.input_buffer.clear();
        self.cursor_pos = 0;
        self.history.clear();
        self.history_idx = None;
        self.scroll_offset = 0;
        self.is_running = false;
        self.rendered_prompt = None;
        self.start_session();
    }

    pub fn set_working_dir(&mut self, dir: PathBuf) {
        if dir.is_dir() {
            self.current_dir = dir.clone();
            if let Some(ref mut sess) = self.session {
                let cmd_line = match self.shell {
                    ShellKind::Pwsh | ShellKind::PowerShell => {
                        format!(
                            "Set-Location -LiteralPath \"{}\"\n{}\n",
                            dir.display(),
                            Self::ps_prompt_capture_snippet()
                        )
                    }
                    ShellKind::Cmd => format!("cd /d \"{}\"\r\n", dir.display()),
                    ShellKind::Bash => format!("cd \"{}\"\n", dir.display()),
                };
                let _ = sess.stdin.write_all(cmd_line.as_bytes());
                let _ = sess.stdin.flush();
            }
        }
    }

    pub fn lines(&self) -> &[String] {
        &self.output_lines
    }

    pub fn current_prompt(&self) -> String {
        match self.shell {
            ShellKind::Pwsh | ShellKind::PowerShell => {
                format!("PS {}>", self.current_dir.display())
            }
            ShellKind::Cmd => format!("{}>", self.current_dir.display()),
            ShellKind::Bash => format!("{} $", self.current_dir.display()),
        }
    }

    /// 返回当前生效的提示符多行文本（优先使用 pwsh 自身 prompt() 函数渲染的原生提示符）
    pub fn active_prompt_lines(&self) -> Vec<String> {
        if let Some(ref raw) = self.rendered_prompt {
            let mut lines: Vec<String> = raw
                .split('\n')
                .map(|l| l.trim_end_matches('\r').to_string())
                .collect();
            // 仅移除尾部的纯空行（如 Out-String 或末尾换行残留），保留如 "❯ " 等有效末行
            while lines.len() > 1 && strip_ansi(lines.last().unwrap_or(&String::new())).is_empty() {
                lines.pop();
            }
            if !lines.is_empty() && !(lines.len() == 1 && strip_ansi(&lines[0]).is_empty()) {
                return lines;
            }
        }
        vec![format!("{} ", self.current_prompt())]
    }

    /// 轮询后台交互式 Shell 输出队列，返回是否有新输出产生
    pub fn poll_output(&mut self) -> bool {
        let mut had_messages = false;
        let mut exited = false;
        if let Some(ref sess) = self.session {
            while let Ok(ev) = sess.rx.try_recv() {
                had_messages = true;
                match ev {
                    TermEvent::Line(msg) => {
                        self.output_lines.push(msg);
                    }
                    TermEvent::CommandDone(new_dir) => {
                        self.is_running = false;
                        if let Some(d) = new_dir {
                            self.current_dir = d;
                        }
                    }
                    TermEvent::Prompt(prompt_str) => {
                        self.rendered_prompt = Some(prompt_str);
                    }
                    TermEvent::Exited => {
                        self.is_running = false;
                        exited = true;
                    }
                }
            }
        }
        if exited {
            self.session = None;
        }
        if self.output_lines.len() > 5000 {
            let drain_count = self.output_lines.len() - 5000;
            self.output_lines.drain(0..drain_count);
        }
        had_messages
    }

    fn push_prompt_with_command(&mut self, cmd_text: &str) {
        let mut prompt_lines = self.active_prompt_lines();
        if let Some(last) = prompt_lines.pop() {
            for line in prompt_lines {
                self.output_lines.push(line);
            }
            let sep = if strip_ansi(&last).ends_with(' ') || cmd_text.is_empty() {
                ""
            } else {
                " "
            };
            self.output_lines
                .push(format!("{}{}{}", last, sep, cmd_text));
        }
    }

    pub fn send_input(&mut self, text: &str) {
        self.scroll_offset = 0;
        let trimmed = text.trim();

        if trimmed.is_empty() {
            self.push_prompt_with_command("");
            self.cursor_pos = 0;
            return;
        }

        self.history.push(trimmed.to_string());
        self.history_idx = None;
        self.push_prompt_with_command(trimmed);
        self.cursor_pos = 0;

        let lower = trimmed.to_lowercase();
        if lower == "cls" || lower == "clear" {
            self.output_lines.clear();
            return;
        }

        if self.session.is_none() {
            self.start_session();
        }

        if let Some(ref mut sess) = self.session {
            self.is_running = true;
            let payload = match self.shell {
                ShellKind::Pwsh | ShellKind::PowerShell => {
                    format!(
                        "{}\nWrite-Output \"{}$((Get-Location).Path){}\"\n{}\n",
                        trimmed,
                        DONE_PREFIX,
                        DONE_SUFFIX,
                        Self::ps_prompt_capture_snippet()
                    )
                }
                ShellKind::Cmd => {
                    format!("{}\r\necho {}%CD%{}\r\n", trimmed, DONE_PREFIX, DONE_SUFFIX)
                }
                ShellKind::Bash => {
                    format!(
                        "{}\nprintf \"{}\"\"%s{}\\n\" \"$PWD\"\n",
                        trimmed, DONE_PREFIX, DONE_SUFFIX
                    )
                }
            };
            if let Err(e) = sess
                .stdin
                .write_all(payload.as_bytes())
                .and_then(|_| sess.stdin.flush())
            {
                self.is_running = false;
                self.output_lines.push(format!("Shell write error: {}", e));
                self.session = None;
            }
        } else {
            self.output_lines.push(format!(
                "Failed to start shell: {}",
                self.shell.executable()
            ));
        }
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

        let path_obj = Path::new(last_word);
        let (search_dir, file_prefix) = if last_word.ends_with('/') || last_word.ends_with('\\') {
            let joined = self.current_dir.join(path_obj);
            (joined, String::new())
        } else if let Some(parent) = path_obj.parent()
            && !parent.as_os_str().is_empty()
        {
            let file_part = path_obj
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            (self.current_dir.join(parent), file_part)
        } else {
            (self.current_dir.clone(), last_word.to_string())
        };

        if let Ok(entries) = std::fs::read_dir(&search_dir) {
            let mut matches = Vec::new();
            let prefix_lower = file_prefix.to_lowercase();
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.to_lowercase().starts_with(&prefix_lower) {
                    let is_dir = entry.path().is_dir();
                    matches.push((name, is_dir));
                }
            }
            matches.sort_by_key(|a| a.0.to_lowercase());
            if let Some((first_match, is_dir)) = matches.first() {
                let prefix_chars = file_prefix.chars().count();
                let remaining: String = first_match.chars().skip(prefix_chars).collect();
                self.input_buffer.push_str(&remaining);
                if *is_dir {
                    #[cfg(target_os = "windows")]
                    self.input_buffer.push('\\');
                    #[cfg(not(target_os = "windows"))]
                    self.input_buffer.push('/');
                }
                self.cursor_pos = self.input_buffer.chars().count();
            }
        }
    }

    pub fn interrupt(&mut self) {
        let was_running = self.is_running;
        let input = self.input_buffer.clone();
        self.input_buffer.clear();
        self.cursor_pos = 0;
        self.push_prompt_with_command(&format!("{}^C", input));
        self.scroll_offset = 0;

        if was_running {
            self.is_running = false;
            self.start_session();
        }
    }

    pub fn clear(&mut self) {
        self.output_lines.clear();
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
            let wt = Command::new("wt.exe")
                .arg("-d")
                .arg(&dir)
                .arg(self.shell.executable())
                .spawn();

            if wt.is_err() {
                let _ = Command::new("cmd.exe")
                    .args(["/c", "start", self.shell.executable()])
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

impl Drop for SystemTerminal {
    fn drop(&mut self) {
        self.kill_session();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_basic_flow() {
        let mut term =
            SystemTerminal::new_offline(1, "pwsh 1", PathBuf::from("."), ShellKind::Pwsh);
        assert_eq!(term.id, 1);
        assert_eq!(term.title, "pwsh 1");
        assert!(!term.lines().is_empty());
        assert!(term.current_prompt().starts_with("PS "));

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
        let mut term =
            SystemTerminal::new_offline(1, "pwsh 1", PathBuf::from("."), ShellKind::Pwsh);
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

    #[test]
    fn test_ansi_parse_and_strip() {
        let raw = "\x1b[32mhello\x1b[0m \x1b[48;2;10;20;30m\x1b[38;2;255;0;0mworld\x1b[0m\r";
        let spans = parse_ansi_spans(raw, true);
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0].text, "hello");
        assert!(spans[0].fg_rgb.is_some());
        assert!(spans[0].bg_rgb.is_none());
        assert_eq!(spans[1].text, " ");
        assert!(spans[1].fg_rgb.is_none());
        assert_eq!(spans[2].text, "world");
        assert_eq!(spans[2].fg_rgb, Some(0xff0000));
        assert_eq!(spans[2].bg_rgb, Some(0x0a141e));
        assert_eq!(strip_ansi(raw), "hello world");
    }

    #[test]
    fn test_base64_decode() {
        assert_eq!(decode_base64_utf8("aGVsbG8=").as_deref(), Some("hello"));
        assert_eq!(decode_base64_utf8("5Lit5paH").as_deref(), Some("中文"));
    }
}
