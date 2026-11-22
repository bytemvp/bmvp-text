use bmvp_text::config::settings::AppConfig;
use bmvp_text::core::buffer::{Position, SearchOptions, TextBuffer};
use bmvp_text::core::git::detect_git_branch;
use bmvp_text::core::logging::{self, LogLevel};
use bmvp_text::core::sidebar::Sidebar;
use bmvp_text::core::syntax::{HighlightCache, TokenType, token_color};
use bmvp_text::terminal::ansi::{parse_ansi_spans, strip_ansi};
use bmvp_text::terminal::session::{
    ShellKind, SystemTerminal, TerminalNotifier, quote_path_for_shell,
};
use bmvp_text::ui::breadcrumb::BreadcrumbBar;
use bmvp_text::ui::file_finder::FileFinder;
use bmvp_text::ui::theme::{Theme, ThemeMode};
use bmvp_text::{StartupArgs, parse_startup_args};
use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn new(prefix: &str) -> Self {
        let id = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "bmvp_text_it_{}_{}_{}",
            prefix,
            std::process::id(),
            id
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create integration test temp dir");
        Self { path: dir }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[test]
fn test_buffer_highlight_cache_and_theme_palettes() {
    let mut buf = TextBuffer::from_str("fn hello() {\r\n    let x = \"world\";\r\n}");
    assert_eq!(buf.len_lines(), 3);
    buf.move_end(false);
    assert_eq!(buf.cursor_position(), Position { row: 0, col: 12 });

    let found = buf
        .find_matches_with_options(
            "world",
            SearchOptions {
                case_sensitive: true,
                whole_word: true,
                use_regex: false,
            },
        )
        .expect("find matches");
    assert_eq!(found.len(), 1);
    let replaced = buf.replace_all_matches(&found, "rust");
    assert_eq!(replaced, 1);
    assert!(buf.is_dirty());
    buf.undo();
    assert!(!buf.is_dirty());

    let mut cache = HighlightCache::new();
    let line = "pub fn demo() -> i32 { 42 }";
    let first = cache.get_or_highlight(line, "rs").to_vec();
    assert_eq!(cache.len(), 1);
    let second = cache.get_or_highlight(line, "rs").to_vec();
    assert_eq!(first, second);
    cache.clear();
    assert!(cache.is_empty());

    // 四套主题拥有差异化语法高亮配色，且 token_color 直接读取 theme.syntax
    let keyword_colors: Vec<gpui::Hsla> = ThemeMode::all()
        .iter()
        .map(|mode| {
            let theme = Theme::from_mode(*mode);
            let col = token_color(TokenType::Keyword, &theme);
            assert_eq!(col, theme.syntax.keyword);
            col
        })
        .collect();
    let unique: HashSet<(u32, u32, u32, u32)> = keyword_colors
        .into_iter()
        .map(|c| (c.h.to_bits(), c.s.to_bits(), c.l.to_bits(), c.a.to_bits()))
        .collect();
    assert_eq!(unique.len(), 4);
}

#[test]
fn test_sidebar_incremental_expand_and_file_finder_scan() {
    let dir = TempDirGuard::new("workspace");
    let root = dir.path();

    fs::write(root.join(".gitignore"), "ignored_out/\n").unwrap();
    fs::create_dir_all(root.join("src/nested")).unwrap();
    fs::create_dir_all(root.join("target/debug")).unwrap();
    fs::create_dir_all(root.join("ignored_out")).unwrap();

    fs::write(root.join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
    fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();
    fs::write(root.join("src/nested/helper.rs"), "pub fn help() {}\n").unwrap();
    fs::write(root.join("target/debug/bin.exe"), "ignored").unwrap();
    fs::write(root.join("ignored_out/out.txt"), "ignored").unwrap();

    let mut sidebar = Sidebar::new();
    sidebar.rebuild_tree(root);
    let src_idx = sidebar
        .items
        .iter()
        .position(|n| n.is_dir && n.name == "src")
        .expect("src directory listed");
    assert!(!sidebar.items[src_idx].is_expanded);
    let initial_len = sidebar.items.len();

    // 增量展开 src 目录
    sidebar.toggle_expand(src_idx);
    assert!(sidebar.items[src_idx].is_expanded);
    assert!(sidebar.items.len() > initial_len);
    assert!(sidebar.items.iter().any(|n| n.name == "main.rs"));

    // 增量折叠还原节点数量
    sidebar.toggle_expand(src_idx);
    assert!(!sidebar.items[src_idx].is_expanded);
    assert_eq!(sidebar.items.len(), initial_len);

    // 异步 FileFinder 扫描器过滤掉 target 与 .gitignore 目录，并按文件名权重优先匹配
    let scanned = FileFinder::scan_workspace_files(root, 100);
    assert!(scanned.iter().any(|(_, rel)| rel.ends_with("helper.rs")));
    assert!(!scanned.iter().any(|(_, rel)| rel.contains("bin.exe")));
    assert!(!scanned.iter().any(|(_, rel)| rel.contains("out.txt")));

    let mut finder = FileFinder::new();
    let gen_id = finder.begin_open();
    finder.finish_scan(gen_id, scanned);
    finder.query = "helper.rs".to_string();
    finder.refilter();
    let selected = finder.selected_path().expect("matched helper.rs");
    assert_eq!(
        selected.file_name().and_then(|s| s.to_str()),
        Some("helper.rs")
    );
}

#[test]
fn test_terminal_ansi_styles_offline_session_quoting_and_notifier() {
    let styled = "\x1b[1;3;4;7;31mAlert\x1b[0m plain";
    let spans = parse_ansi_spans(styled, true);
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].text, "Alert");
    assert!(spans[0].bold);
    assert!(spans[0].italic);
    assert!(spans[0].underline);
    assert!(spans[0].inverse);
    assert!(spans[0].fg_rgb.is_some());
    assert_eq!(spans[1].text, " plain");
    assert!(!spans[1].bold);
    assert_eq!(strip_ansi(styled), "Alert plain");

    // 路径引用需针对不同 Shell 转义特殊字符
    let nasty_path = Path::new("C:\\work\\a'b\"c$d");
    assert!(quote_path_for_shell(ShellKind::Pwsh, nasty_path).contains("''"));
    assert!(quote_path_for_shell(ShellKind::Cmd, nasty_path).starts_with('"'));
    assert!(quote_path_for_shell(ShellKind::Bash, nasty_path).contains("'\\''"));
    assert!(quote_path_for_shell(ShellKind::Zsh, nasty_path).contains("'\\''"));
    assert!(quote_path_for_shell(ShellKind::Fish, nasty_path).contains("'\\''"));

    // 离线终端会话（不拉起任何真实子进程）+ 行级 ANSI 缓存 + 事件唤醒器
    let mut term = SystemTerminal::new_offline(1, "pwsh 1", PathBuf::from("."), ShellKind::Pwsh);
    term.insert_str("echo 世界");
    term.send_input("\x1b[32mhello 世界\x1b[0m");
    term.output_lines
        .push("\x1b[32mhello 世界\x1b[0m".to_string());
    let line_idx = term.output_lines.len() - 1;
    let cached_1 = term.spans_for_line(line_idx, false);
    let cached_2 = term.spans_for_line(line_idx, false);
    assert_eq!(cached_1, cached_2);
    assert_eq!(cached_1[0].text, "hello 世界");
    term.clear();
    assert!(term.output_lines.is_empty());

    let notifier = TerminalNotifier::new();
    notifier.notify();
    let waker = std::task::Waker::noop();
    let mut cx = std::task::Context::from_waker(waker);
    let mut fut = std::pin::pin!(notifier.wait());
    assert!(fut.as_mut().poll(&mut cx).is_ready());
}

#[test]
fn test_git_branch_startup_args_and_config_roundtrip() {
    let dir = TempDirGuard::new("git_and_args");
    let root = dir.path();
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(root.join(".git/HEAD"), "ref: refs/heads/feat/p2-modular\n").unwrap();
    fs::create_dir_all(root.join("src")).unwrap();
    assert_eq!(
        detect_git_branch(&root.join("src")),
        Some("feat/p2-modular".to_string())
    );

    let file_arg = root.join("Cargo.toml");
    fs::write(&file_arg, "[package]\n").unwrap();
    let parsed: StartupArgs = parse_startup_args([
        OsString::from("bmvp-text.exe"),
        root.as_os_str().to_os_string(),
        file_arg.as_os_str().to_os_string(),
    ]);
    assert_eq!(parsed.root_dir.as_deref(), Some(root));
    assert_eq!(parsed.files, vec![file_arg]);

    // 兼容旧版 settings.json（缺少 sidebar_width / terminal_height 等新字段）
    let legacy_json = r#"{
        "theme_mode": "ZedDark",
        "editor": {
            "font_family": "Consolas",
            "font_size": 14.0,
            "line_height_factor": 1.6,
            "tab_size": 4,
            "show_line_numbers": true,
            "terminal_shell": "pwsh",
            "terminal_font_family": "Consolas",
            "terminal_font_size": 13.0
        }
    }"#;
    let cfg: AppConfig = serde_json::from_str(legacy_json).expect("deserialize legacy config");
    assert_eq!(cfg.theme_mode, ThemeMode::ZedDark);
    assert_eq!(cfg.editor.sidebar_width, 230.0);
    assert_eq!(cfg.editor.terminal_height, 240.0);
}

#[test]
fn test_cross_platform_config_breadcrumb_and_logging_flow() {
    assert_eq!(
        AppConfig::resolve_config_path_for("macos", |k| {
            (k == "HOME").then(|| "/Users/dev".to_string())
        }),
        PathBuf::from("/Users/dev")
            .join("Library")
            .join("Application Support")
            .join("bmvp-text")
            .join("settings.json")
    );
    assert_eq!(
        AppConfig::resolve_config_path_for("linux", |k| {
            (k == "XDG_CONFIG_HOME").then(|| "/home/dev/.cfg".to_string())
        }),
        PathBuf::from("/home/dev/.cfg")
            .join("bmvp-text")
            .join("settings.json")
    );

    let dir = TempDirGuard::new("p3_flow");
    let cfg_path = dir.path().join("cfg").join("settings.json");
    let mut cfg: AppConfig =
        serde_json::from_str(r#"{"editor":{"tab_size":2}}"#).expect("partial json migration");
    assert_eq!(cfg.editor.tab_size, 2);
    cfg.theme_mode = ThemeMode::OneDark;
    cfg.save_to_path(&cfg_path).expect("save config");
    let reloaded = AppConfig::load_from_path(&cfg_path).expect("reload config");
    assert_eq!(reloaded.theme_mode, ThemeMode::OneDark);
    assert_eq!(reloaded.editor.tab_size, 2);

    assert_eq!(
        BreadcrumbBar::format_path(
            Some("/workspace/demo/src/lib.rs"),
            Some(Path::new("/workspace/demo")),
        ),
        "src › lib.rs"
    );

    let log_line = logging::format_log_line(LogLevel::Warn, "p3-test", "sample warning");
    assert!(log_line.contains("[WARN] [p3-test] sample warning"));

    let note_path = dir.path().join("note.rs");
    let mut buf = TextBuffer::from_str("fn a() {}");
    buf.move_end(false);
    buf.insert("\nfn b() {}");
    buf.save_file(Some(&note_path)).expect("save buffer");
    let reopened = TextBuffer::open_file(&note_path).expect("reopen buffer");
    assert_eq!(reopened.len_lines(), 2);
}
