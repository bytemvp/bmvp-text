use crate::terminal::ansi::{
    AnsiSpan, decode_base64_utf8, decode_oem, parse_ansi_spans, strip_ansi,
};
use std::cell::RefCell;
use std::future::poll_fn;
use std::io::{BufRead, BufReader, Write};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::task::{Poll, Waker};
use std::thread;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

const DONE_PREFIX: &str = "__BMVP_DONE__";
const DONE_SUFFIX: &str = "__";
const PROMPT_PREFIX: &str = "__BMVP_PROMPT__";
const PROMPT_SUFFIX: &str = "__";

#[derive(Default)]
struct WakeState {
    pending: bool,
    waker: Option<Waker>,
}

/// 轻量级零依赖跨线程事件唤醒器，用于后台 Reader 线程按需唤醒 UI 异步消费任务
#[derive(Clone, Default)]
pub struct TerminalNotifier {
    state: Arc<Mutex<WakeState>>,
}

impl TerminalNotifier {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn notify(&self) {
        let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
        guard.pending = true;
        if let Some(waker) = guard.waker.take() {
            waker.wake();
        }
    }

    pub async fn wait(&self) {
        poll_fn(|cx| {
            let mut guard = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if guard.pending {
                guard.pending = false;
                Poll::Ready(())
            } else {
                guard.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        })
        .await;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellKind {
    Pwsh,
    PowerShell,
    Cmd,
    Bash,
    Zsh,
    Fish,
}

impl ShellKind {
    pub fn from_config(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "pwsh" => Self::Pwsh,
            "powershell" => Self::PowerShell,
            "cmd" => Self::Cmd,
            "bash" | "sh" => Self::Bash,
            "zsh" => Self::Zsh,
            "fish" => Self::Fish,
            _ => Self::Pwsh,
        }
    }

    pub fn short_name(self) -> &'static str {
        match self {
            Self::Pwsh => "pwsh",
            Self::PowerShell => "powershell",
            Self::Cmd => "cmd",
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
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
                Self::Zsh => "zsh.exe",
                Self::Fish => "fish.exe",
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            match self {
                Self::Pwsh => "pwsh",
                Self::PowerShell => "pwsh",
                Self::Cmd => "sh",
                Self::Bash => "bash",
                Self::Zsh => "zsh",
                Self::Fish => "fish",
            }
        }
    }
}

/// 纯文件系统 PATH 查找，避免在 UI 线程同步拉起子进程检测可执行文件
pub fn find_executable_in_path(bin: &str) -> bool {
    let bin_path = Path::new(bin);
    if bin_path.is_absolute() || bin_path.components().count() > 1 {
        return bin_path.is_file();
    }
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    #[cfg(target_os = "windows")]
    let exts: Vec<String> = if bin_path.extension().is_some() {
        vec![String::new()]
    } else {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".EXE;.CMD;.BAT;.COM".to_string())
            .split(';')
            .map(str::to_string)
            .collect()
    };

    for dir in std::env::split_paths(&path_var) {
        #[cfg(target_os = "windows")]
        {
            for ext in &exts {
                let candidate = if ext.is_empty() {
                    dir.join(bin)
                } else {
                    dir.join(format!("{}{}", bin, ext))
                };
                if candidate.is_file() {
                    return true;
                }
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            if dir.join(bin).is_file() {
                return true;
            }
        }
    }
    false
}

pub fn resolve_available_shell(requested: &str) -> ShellKind {
    let preferred = ShellKind::from_config(requested);
    #[cfg(target_os = "windows")]
    {
        match preferred {
            ShellKind::Pwsh => {
                if find_executable_in_path("pwsh.exe") {
                    ShellKind::Pwsh
                } else if find_executable_in_path("powershell.exe") {
                    ShellKind::PowerShell
                } else {
                    ShellKind::Cmd
                }
            }
            ShellKind::PowerShell => {
                if find_executable_in_path("powershell.exe") {
                    ShellKind::PowerShell
                } else {
                    ShellKind::Cmd
                }
            }
            ShellKind::Bash => {
                if find_executable_in_path("bash.exe") {
                    ShellKind::Bash
                } else {
                    ShellKind::Cmd
                }
            }
            ShellKind::Zsh => {
                if find_executable_in_path("zsh.exe") {
                    ShellKind::Zsh
                } else {
                    ShellKind::Cmd
                }
            }
            ShellKind::Fish => {
                if find_executable_in_path("fish.exe") {
                    ShellKind::Fish
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
            ShellKind::Pwsh if find_executable_in_path("pwsh") => ShellKind::Pwsh,
            ShellKind::Zsh if find_executable_in_path("zsh") => ShellKind::Zsh,
            ShellKind::Fish if find_executable_in_path("fish") => ShellKind::Fish,
            ShellKind::Bash if find_executable_in_path("bash") => ShellKind::Bash,
            _ if find_executable_in_path("zsh") => ShellKind::Zsh,
            _ => ShellKind::Bash,
        }
    }
}

pub fn quote_path_for_shell(shell: ShellKind, dir: &Path) -> String {
    let raw = dir.to_string_lossy();
    match shell {
        ShellKind::Pwsh | ShellKind::PowerShell => {
            format!("'{}'", raw.replace('\'', "''"))
        }
        ShellKind::Cmd => {
            format!("\"{}\"", raw.replace('"', ""))
        }
        ShellKind::Bash | ShellKind::Zsh | ShellKind::Fish => {
            format!("'{}'", raw.replace('\'', "'\\''"))
        }
    }
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
        ShellKind::Bash | ShellKind::Zsh | ShellKind::Fish => vec![
            format!("{} Terminal Session Initialized", shell.short_name()),
            String::new(),
        ],
    }
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

#[derive(Default)]
struct AnsiLineCache {
    is_light: bool,
    lines: Vec<Option<Vec<AnsiSpan>>>,
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
    notifier: Option<TerminalNotifier>,
    offline: bool,
    ansi_cache: RefCell<AnsiLineCache>,
}

impl SystemTerminal {
    pub fn new(
        id: usize,
        title: &str,
        current_dir: PathBuf,
        shell_name: &str,
        notifier: Option<TerminalNotifier>,
    ) -> Self {
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
            notifier,
            offline: false,
            ansi_cache: RefCell::new(AnsiLineCache::default()),
        };
        term.start_session();
        term
    }

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
            notifier: None,
            offline: true,
            ansi_cache: RefCell::new(AnsiLineCache::default()),
        }
    }

    fn spawn_reader(
        reader: impl std::io::Read + Send + 'static,
        tx: Sender<TermEvent>,
        notifier: Option<TerminalNotifier>,
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
                    if tx
                        .send(TermEvent::CommandDone(if dir.is_dir() {
                            Some(dir)
                        } else {
                            None
                        }))
                        .is_err()
                    {
                        break;
                    }
                    if let Some(ref n) = notifier {
                        n.notify();
                    }
                    continue;
                }

                if let Some(rest) = trimmed_clean.strip_prefix(PROMPT_PREFIX)
                    && let Some(b64) = rest.strip_suffix(PROMPT_SUFFIX)
                    && let Some(decoded) = decode_base64_utf8(b64.trim())
                {
                    if tx.send(TermEvent::Prompt(decoded)).is_err() {
                        break;
                    }
                    if let Some(ref n) = notifier {
                        n.notify();
                    }
                    continue;
                }

                if tx.send(TermEvent::Line(line)).is_err() {
                    break;
                }
                if let Some(ref n) = notifier {
                    n.notify();
                }
            }
            if send_exit
                && tx.send(TermEvent::Exited).is_ok()
                && let Some(ref n) = notifier
            {
                n.notify();
            }
        });
    }

    fn ps_prompt_capture_snippet() -> &'static str {
        "$__bmvp_p = if (Test-Path Function:\\prompt) { (prompt) -join \"`n\" } else { \"PS $((Get-Location).Path)> \" }; Write-Output (\"__BMVP_PROMPT__\" + [System.Convert]::ToBase64String([System.Text.Encoding]::UTF8.GetBytes($__bmvp_p)) + \"__\")"
    }

    fn start_session(&mut self) {
        self.kill_session();
        if self.offline {
            return;
        }

        let dir = if self.current_dir.is_dir() {
            self.current_dir.clone()
        } else {
            PathBuf::from(".")
        };

        let mut cmd = Command::new(self.shell.executable());
        match self.shell {
            ShellKind::Pwsh | ShellKind::PowerShell => {
                // 直接使用系统/用户自带的 pwsh 配置，不传入 -NoProfile 或新建 profile
                cmd.args(["-NoExit", "-Command", "-"]);
            }
            ShellKind::Cmd => {
                #[cfg(target_os = "windows")]
                cmd.args(["/Q", "/K"]);
            }
            ShellKind::Bash | ShellKind::Zsh => {
                cmd.arg("-s");
            }
            ShellKind::Fish => {}
        }

        #[cfg(target_os = "windows")]
        cmd.creation_flags(CREATE_NO_WINDOW);

        cmd.current_dir(&dir);
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        match cmd.spawn() {
            Ok(mut child) => {
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
                            if let Err(err) =
                                stdin.write_all(init.as_bytes()).and_then(|_| stdin.flush())
                            {
                                self.output_lines.push(format!("Shell init error: {}", err));
                            }
                        }
                        ShellKind::Cmd => {
                            if let Err(err) = stdin
                                .write_all(b"@echo off\r\nchcp 65001 > nul\r\n")
                                .and_then(|_| stdin.flush())
                            {
                                self.output_lines.push(format!("Shell init error: {}", err));
                            }
                        }
                        ShellKind::Bash | ShellKind::Zsh | ShellKind::Fish => {}
                    }

                    let (tx, rx) = channel::<TermEvent>();
                    Self::spawn_reader(stdout, tx.clone(), self.notifier.clone(), true);
                    Self::spawn_reader(stderr, tx, self.notifier.clone(), false);

                    self.session = Some(ShellSession {
                        pid,
                        stdin,
                        rx,
                        _child: child,
                    });
                }
            }
            Err(err) => {
                self.output_lines.push(format!(
                    "Failed to start shell {}: {}",
                    self.shell.executable(),
                    err
                ));
            }
        }
    }

    /// 非阻塞终止子进程：立即释放句柄并将进程树杀除与 wait() 下沉到后台线程
    fn kill_session(&mut self) {
        if let Some(mut sess) = self.session.take() {
            let pid = sess.pid;
            drop(sess.stdin);
            thread::spawn(move || {
                #[cfg(target_os = "windows")]
                {
                    if let Err(err) = Command::new("taskkill")
                        .args(["/F", "/T", "/PID", &pid.to_string()])
                        .creation_flags(CREATE_NO_WINDOW)
                        .output()
                        && err.kind() != std::io::ErrorKind::NotFound
                    {
                        crate::core::logging::warn(
                            "terminal",
                            &format!("taskkill PID {} failed: {}", pid, err),
                        );
                    }
                }
                #[cfg(not(target_os = "windows"))]
                {
                    if let Err(err) = Command::new("kill").args(["-9", &pid.to_string()]).output()
                        && err.kind() != std::io::ErrorKind::NotFound
                    {
                        crate::core::logging::warn(
                            "terminal",
                            &format!("kill -9 PID {} failed: {}", pid, err),
                        );
                    }
                }
                if let Err(err) = sess._child.wait() {
                    crate::core::logging::warn(
                        "terminal",
                        &format!("waiting child PID {} exit failed: {}", pid, err),
                    );
                }
            });
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
        self.ansi_cache.borrow_mut().lines.clear();
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
                let quoted = quote_path_for_shell(self.shell, &dir);
                let cmd_line = match self.shell {
                    ShellKind::Pwsh | ShellKind::PowerShell => {
                        format!(
                            "Set-Location -LiteralPath {}\n{}\n",
                            quoted,
                            Self::ps_prompt_capture_snippet()
                        )
                    }
                    ShellKind::Cmd => format!("cd /d {}\r\n", quoted),
                    ShellKind::Bash | ShellKind::Zsh | ShellKind::Fish => {
                        format!("cd {}\n", quoted)
                    }
                };
                if let Err(err) = sess
                    .stdin
                    .write_all(cmd_line.as_bytes())
                    .and_then(|_| sess.stdin.flush())
                {
                    self.output_lines.push(format!("cd failed: {}", err));
                }
            }
        }
    }

    pub fn lines(&self) -> &[String] {
        &self.output_lines
    }

    /// 读取并按需缓存指定行的 ANSI 解析结果
    pub fn spans_for_line(&self, line_idx: usize, is_light: bool) -> Vec<AnsiSpan> {
        let Some(raw) = self.output_lines.get(line_idx) else {
            return Vec::new();
        };
        let mut cache = self.ansi_cache.borrow_mut();
        if cache.is_light != is_light {
            cache.is_light = is_light;
            cache.lines.clear();
        }
        if cache.lines.len() < self.output_lines.len() {
            cache.lines.resize(self.output_lines.len(), None);
        }
        if let Some(ref cached) = cache.lines[line_idx] {
            return cached.clone();
        }
        let parsed = parse_ansi_spans(raw, is_light);
        cache.lines[line_idx] = Some(parsed.clone());
        parsed
    }

    pub fn current_prompt(&self) -> String {
        match self.shell {
            ShellKind::Pwsh | ShellKind::PowerShell => {
                format!("PS {}>", self.current_dir.display())
            }
            ShellKind::Cmd => format!("{}>", self.current_dir.display()),
            ShellKind::Bash | ShellKind::Zsh | ShellKind::Fish => {
                format!("{} $", self.current_dir.display())
            }
        }
    }

    /// 返回当前生效的提示符多行文本（优先使用 pwsh 自身 prompt() 函数渲染的原生提示符）
    pub fn active_prompt_lines(&self) -> Vec<String> {
        if let Some(ref raw) = self.rendered_prompt {
            let mut lines: Vec<String> = raw
                .split('\n')
                .map(|l| l.trim_end_matches('\r').to_string())
                .collect();
            while lines.len() > 1 && strip_ansi(lines.last().unwrap_or(&String::new())).is_empty() {
                lines.pop();
            }
            if !lines.is_empty() && !(lines.len() == 1 && strip_ansi(&lines[0]).is_empty()) {
                return lines;
            }
        }
        vec![format!("{} ", self.current_prompt())]
    }

    /// 消费后台交互式 Shell 输出队列，返回是否有新事件产生
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
            let mut cache = self.ansi_cache.borrow_mut();
            if cache.lines.len() >= drain_count {
                cache.lines.drain(0..drain_count);
            } else {
                cache.lines.clear();
            }
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
            self.ansi_cache.borrow_mut().lines.clear();
            return;
        }

        if self.offline {
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
                ShellKind::Bash | ShellKind::Zsh | ShellKind::Fish => {
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
        self.ansi_cache.borrow_mut().lines.clear();
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

    fn char_byte_index(&self, char_idx: usize) -> usize {
        self.input_buffer
            .char_indices()
            .nth(char_idx)
            .map(|(b, _)| b)
            .unwrap_or(self.input_buffer.len())
    }

    pub fn insert_char(&mut self, c: char) {
        let byte_idx = self.char_byte_index(self.cursor_pos);
        self.input_buffer.insert(byte_idx, c);
        self.cursor_pos += 1;
    }

    pub fn insert_str(&mut self, text: &str) {
        let filtered: String = text.chars().filter(|&c| c != '\r' && c != '\n').collect();
        if filtered.is_empty() {
            return;
        }
        let added_chars = filtered.chars().count();
        let byte_idx = self.char_byte_index(self.cursor_pos);
        self.input_buffer.insert_str(byte_idx, &filtered);
        self.cursor_pos += added_chars;
    }

    pub fn delete_backspace(&mut self) {
        if self.cursor_pos > 0 {
            let target_char = self.cursor_pos - 1;
            let byte_idx = self.char_byte_index(target_char);
            if byte_idx < self.input_buffer.len() {
                self.input_buffer.remove(byte_idx);
                self.cursor_pos = target_char;
            }
        }
    }

    pub fn delete_forward(&mut self) {
        let byte_idx = self.char_byte_index(self.cursor_pos);
        if byte_idx < self.input_buffer.len() {
            self.input_buffer.remove(byte_idx);
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

    pub fn open_external(&self) -> Result<(), String> {
        let dir = if self.current_dir.is_dir() {
            self.current_dir.clone()
        } else {
            PathBuf::from(".")
        };

        #[cfg(target_os = "windows")]
        {
            match Command::new("wt.exe")
                .arg("-d")
                .arg(&dir)
                .arg(self.shell.executable())
                .spawn()
            {
                Ok(_) => Ok(()),
                Err(wt_err) => Command::new("cmd.exe")
                    .args(["/c", "start", self.shell.executable()])
                    .current_dir(&dir)
                    .spawn()
                    .map(|_| ())
                    .map_err(|cmd_err| {
                        format!("wt.exe failed ({}); cmd.exe failed ({})", wt_err, cmd_err)
                    }),
            }
        }
        #[cfg(target_os = "macos")]
        {
            Command::new("open")
                .args(["-a", "Terminal"])
                .arg(&dir)
                .spawn()
                .map(|_| ())
                .map_err(|err| format!("failed to launch Terminal.app: {}", err))
        }
        #[cfg(all(unix, not(target_os = "macos")))]
        {
            const LINUX_TERMINALS: &[&str] = &[
                "x-terminal-emulator",
                "gnome-terminal",
                "konsole",
                "alacritty",
                "kitty",
                "xterm",
            ];
            for bin in LINUX_TERMINALS {
                if find_executable_in_path(bin) {
                    return Command::new(bin)
                        .current_dir(&dir)
                        .spawn()
                        .map(|_| ())
                        .map_err(|err| format!("failed to launch {}: {}", bin, err));
                }
            }
            Err("No supported external terminal emulator found in PATH".to_string())
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
    fn test_terminal_basic_and_unicode_inplace_editing() {
        let mut term =
            SystemTerminal::new_offline(1, "pwsh 1", PathBuf::from("."), ShellKind::Pwsh);
        assert_eq!(term.id, 1);
        assert_eq!(term.title, "pwsh 1");
        assert!(!term.lines().is_empty());
        assert!(term.current_prompt().starts_with("PS "));

        term.insert_str("echo 你好");
        assert_eq!(term.input_buffer, "echo 你好");
        assert_eq!(term.cursor_pos, 7);

        term.move_cursor_left();
        assert_eq!(term.cursor_pos, 6);
        term.insert_char('!');
        assert_eq!(term.input_buffer, "echo 你!好");

        term.delete_forward();
        assert_eq!(term.input_buffer, "echo 你!");

        term.move_cursor_home();
        assert_eq!(term.cursor_pos, 0);

        term.move_cursor_end();
        assert_eq!(term.cursor_pos, 7);

        term.delete_backspace();
        assert_eq!(term.input_buffer, "echo 你");

        term.clear_input_line();
        assert_eq!(term.input_buffer, "");
        assert_eq!(term.cursor_pos, 0);
    }

    #[test]
    fn test_terminal_history_and_clear_offline() {
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
    fn test_quote_path_for_shell_and_ansi_cache() {
        assert_eq!(
            quote_path_for_shell(ShellKind::Pwsh, Path::new("C:\\a'b\\$c")),
            "'C:\\a''b\\$c'"
        );
        assert_eq!(
            quote_path_for_shell(ShellKind::Bash, Path::new("/tmp/a'b")),
            "'/tmp/a'\\''b'"
        );
        assert_eq!(
            quote_path_for_shell(ShellKind::Cmd, Path::new("C:\\a\"b")),
            "\"C:\\ab\""
        );

        let mut term =
            SystemTerminal::new_offline(1, "pwsh 1", PathBuf::from("."), ShellKind::Pwsh);
        term.output_lines.push("\x1b[32mok\x1b[0m".to_string());
        let spans = term.spans_for_line(term.output_lines.len() - 1, true);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].text, "ok");
    }

    #[test]
    fn test_terminal_notifier_signal() {
        let notifier = TerminalNotifier::new();
        notifier.notify();
        let mut guard = notifier.state.lock().unwrap();
        assert!(guard.pending);
        guard.pending = false;
    }
}
