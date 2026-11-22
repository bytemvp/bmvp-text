# bmvp-text 优化审查与演进路线图

> 本文档整理自对当前全量源码、CI/CD 配置与打包脚本的系统性审查，按风险程度与用户价值划分为 **P0 / P1 / P2 / P3** 四个优先级，作为后续迭代与重构的执行依据。

---

## P0 — 严重缺陷与崩溃/死循环风险（必须立即修复）

> **状态更新（2026-11-22）**：以下 6 项 P0 严重缺陷均已**修复并补充回归单元测试**，验证通过 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings` 与 `cargo test --all-targets`。

### 1. ✅ 已修复：语法高亮遇到 `#` 字符时死循环冻结编辑器
- **定位**：[src/syntax.rs](file:///d:/projects/bytemvp/bmvp-text/src/syntax.rs)
- **问题现象与根因**：
  - 原 `highlight_line` 仅在 `is_hash_comment`（`Python`、`TOML`、`Shell`）时将 `#` 识别为注释。
  - 在 Rust（如 `#[derive(Debug)]`、`#![allow(dead_code)]`）、C/C++（`#include`）、Markdown（`# 标题`）等文件中，操作符集合未包含 `#`，兜底循环遇到 `#` 立即 `break`，导致 `idx` 永不推进、界面卡死。
- **已实施修复**：
  - 将 `#` 加入标点/操作符集合，统一兜底循环停顿条件与第 3/4 步保持一致（`is_alphabetic() || is_ascii_digit()`），并在兜底分支末尾增加单字符强制推进保护，确保任意输入都不会死循环。
  - 在 `src/syntax.rs` 新增 4 个单元测试（覆盖 Rust `#`、C/Markdown `#`、`Python/TOML/Shell` 注释及非 ASCII 数字字符）。

### 2. ✅ 已修复：文档搜索存在 UTF-8 切片 Panic 风险与 $O(M \cdot N)$ 性能退化
- **定位**：[src/buffer.rs](file:///d:/projects/bytemvp/bmvp-text/src/buffer.rs)
- **问题现象与根因**：
  - 原 `find_matches` 用 `to_lowercase()` 的字节偏移直接切片原字符串 `full_text[..match_start_byte]`，遇到大小写改变字节长度的 Unicode 字符（如 `İ`）会触发 UTF-8 边界 Panic，且每次命中重复 `.chars().count()` 退化为平方级开销。
- **已实施修复**：
  - 单遍遍历 `self.rope.chars()` 构建 `lower_text` 与 `byte_to_char` 字节-字符索引映射表，实现 $O(1)$ 字符区间换算，彻底消除切片 Panic 与重复计数。
  - 在 `src/buffer.rs` 新增 `test_find_matches_unicode_and_turkish_i_no_panic` 单元测试。

### 3. ✅ 已修复：行注释切换混用字节/字符偏移且硬编码 `// `
- **定位**：[src/buffer.rs](file:///d:/projects/bytemvp/bmvp-text/src/buffer.rs)
- **问题现象与根因**：
  - 原 `toggle_comment` 将字节差 `line_str.len() - trimmed.len()` 直接加到字符偏移 `line_start` 上，在全角空格等缩进下发生错位或 Panic；且硬编码 `"// "`，空行也会吞掉换行符错位注释到下一行。
- **已实施修复**：
  - 改为剥离行尾换行符后以字符数计算缩进，新增 `line_comment_prefix()` 按语言选择 `#` / `//`，支持多行选区逆序批量注释/反注释并准确更新光标。
  - 新增 `test_toggle_comment_unicode_indent_empty_line_and_languages` 单元测试。

### 4. ✅ 已修复：CRLF (`\r\n`) 行尾截断错误破坏换行符
- **定位**：[src/buffer.rs](file:///d:/projects/bytemvp/bmvp-text/src/buffer.rs)
- **问题现象与根因**：
  - `position_to_char` 与 `move_end` 仅扣除 1 字符，导致 CRLF 文件行尾光标落入 `\r` 与 `\n` 之间，进而破坏换行符。
- **已实施修复**：
  - 新增 `line_content_len()` 统一准确扣除 `\r\n` / `\n` / `\r`；同步使 `move_left`、`move_right`、`backspace`、`delete` 原子跨越/删除 `\r\n`，`duplicate_line` 与 `Enter` 智能缩进尊重文件行尾风格。
  - 新增 `test_crlf_cursor_movement_and_editing` 单元测试。

### 5. ✅ 已修复：鼠标事件未阻止冒泡导致交互错乱
- **定位**：[src/editor.rs](file:///d:/projects/bytemvp/bmvp-text/src/editor.rs)、[src/file_finder.rs](file:///d:/projects/bytemvp/bmvp-text/src/file_finder.rs)、[src/find_replace.rs](file:///d:/projects/bytemvp/bmvp-text/src/find_replace.rs)
- **问题现象与根因**：
  - Tab/终端 Tab 关闭按钮 `×`、终端 Dock 关闭按钮 `✕`、`FileFinder` 内容卡片及 `FindState` 浮层缺少 `cx.stop_propagation()` 与 `.occlude()`，导致误切换 Tab、误关闭弹窗或焦点错乱。
- **已实施修复**：
  - 补全相关嵌套按钮与浮层卡片的 `cx.stop_propagation()` 与 `.occlude()`，并在点击查找条 `✕` 时同步重置 `find_focused = false`。

### 6. ✅ 已修复：未保存修改（Dirty Buffer）缺乏关闭与退出保护
- **定位**：[src/confirm_modal.rs](file:///d:/projects/bytemvp/bmvp-text/src/confirm_modal.rs)、[src/editor.rs](file:///d:/projects/bytemvp/bmvp-text/src/editor.rs)、[src/main.rs](file:///d:/projects/bytemvp/bmvp-text/src/main.rs)
- **问题现象与根因**：
  - 关闭标签页、`Ctrl+Q`、标题栏 `✕` 及系统窗口关闭按钮均未检查 `is_dirty()`，直接丢弃未保存内容；异步另存为回调捕获瞬时数组下标存在竞态。
- **已实施修复**：
  - 为 `TextBuffer` 增加全局唯一 `id()`，使异步保存与确认关闭均基于稳定 `tab_id` 定位。
  - 新增完全解耦的 `ConfirmModal`（通过纯回调通信，不耦合 `EditorView`），覆盖 Tab 关闭（`×`、`Ctrl+W`）、应用退出（`Ctrl+Q`、标题栏 `✕`）及窗口系统关闭钩子（`window.on_window_should_close`），支持 `Save` / `Don't Save` / `Cancel`（以及 `Enter` / `Esc` 快捷键）。

---

## P1 — 核心编辑器功能与交互体验短板

> **状态更新（2026-11-23）**：以下 6 项 P1 核心优化已全部**实现并补充离线单元测试**，验证通过 `cargo fmt --all -- --check`、`cargo clippy --all-targets -- -D warnings` 与 `cargo test --all-targets`。

### 1. ✅ 已优化：视口计算硬编码、滚动夹紧与列表不可滚动
- **定位**：[src/buffer.rs](file:///d:/projects/bytemvp/bmvp-text/src/buffer.rs)、[src/code_canvas.rs](file:///d:/projects/bytemvp/bmvp-text/src/code_canvas.rs)、[src/sidebar.rs](file:///d:/projects/bytemvp/bmvp-text/src/sidebar.rs)、[src/file_finder.rs](file:///d:/projects/bytemvp/bmvp-text/src/file_finder.rs)、[src/editor.rs](file:///d:/projects/bytemvp/bmvp-text/src/editor.rs)
- **已实施优化**：
  - 将 `scroll_top` 与 `scroll_left` 迁入每个 `TextBuffer`，切换标签页独立保持各自滚动位置。
  - `CodeCanvas` 与 `editor_viewport_lines` 根据实际容器高度动态计算可见行数，不再硬编码 `38/30/28`；编辑器滚轮向下严格夹紧到 `total_lines.saturating_sub(1)`，并支持 `Shift + 滚轮` / 横向滚动。
  - 侧边栏与 `FileFinder` 均增加 `scroll_offset`、滚轮滚动及高亮选中项自动视口跟随，同时消除了 `FileFinder` 与侧边栏硬编码 `"Consolas"` 字体。

### 2. ✅ 已优化：输入法（IME）支持与精确鼠标文本定位
- **定位**：[src/code_canvas.rs](file:///d:/projects/bytemvp/bmvp-text/src/code_canvas.rs)、[src/editor.rs](file:///d:/projects/bytemvp/bmvp-text/src/editor.rs)、[src/buffer.rs](file:///d:/projects/bytemvp/bmvp-text/src/buffer.rs)
- **已实施优化**：
  - 在 `EditorView` 实现完整的 GPUI `EntityInputHandler`（支持 UTF-16 转换、IME 预编辑下划线 `ime_marked_range`、不污染撤销栈的组合编辑及候选框精准光标坐标），并配合 `handle_key_down` 放行普通文本输入至输入法管道，同步支持编辑区、查找替换框、文件搜索、字体过滤与终端的中文/IME 输入。
  - 新建 `CodeCanvas` 基于 GPUI `TextSystem::shape_line` 与 `closest_index_for_x` 实现像素到字符索引的精确命中测试（修复中文/宽字符/比例字体定位偏移），并补全鼠标按住拖拽划词选区、双击选词（`select_word_at`）与三击选行（`select_line_at`）。

### 3. ✅ 已优化：光标行与选区渲染缺陷
- **定位**：[src/code_canvas.rs](file:///d:/projects/bytemvp/bmvp-text/src/code_canvas.rs)
- **已实施优化**：
  - `CodeCanvas` 每行仅调用一次 `highlight_line` 生成完整 `TextRun`（不再在光标处拆段），随后按行背景 -> 全量查找命中 -> 文本选区 -> 语法着色文本 -> 2px 竖线光标的层级叠加绘制，保证光标行同时保留完整语法着色与选区高亮。

### 4. ✅ 已优化：查找与替换功能闭环
- **定位**：[src/find_replace.rs](file:///d:/projects/bytemvp/bmvp-text/src/find_replace.rs)、[src/buffer.rs](file:///d:/projects/bytemvp/bmvp-text/src/buffer.rs)、[src/editor.rs](file:///d:/projects/bytemvp/bmvp-text/src/editor.rs)
- **已实施优化**：
  - 接入 `regex` 依赖，在 `TextBuffer::find_matches_with_options` 与 `FindState` 补齐 `Aa`（区分大小写）、`W`（全词匹配）、`.*`（正则表达式与语法错误提示）、替换（Replace / All）以及 `Ctrl+H` 快捷键、`Tab` 切换查找/替换框。
  - `CodeCanvas` 在编辑区绘制所有命中项半透明高亮并高亮区分当前命中项；点击 `✕` 关闭时同步清除 `find_focused`。

### 5. ✅ 已优化：编辑手感、历史栈与原子保存安全
- **定位**：[src/buffer.rs](file:///d:/projects/bytemvp/bmvp-text/src/buffer.rs)、[src/editor.rs](file:///d:/projects/bytemvp/bmvp-text/src/editor.rs)、[src/status_bar.rs](file:///d:/projects/bytemvp/bmvp-text/src/status_bar.rs)
- **已实施优化**：
  - `undo_stack`/`redo_stack` 改为 `VecDeque`，引入 `revision` / `saved_revision` 对比（撤销回保存点自动清除 `Modified` 脏标志）、500ms 同类单字符输入/退格合并，并跳过边界空操作压栈。
  - 补齐 `preferred_col` 粘性列、词级导航/删除（`Ctrl+Left/Right/Backspace/Delete`）、`PageUp/PageDown` 与多行 `Tab`/`Shift+Tab` 缩进反缩进。
  - `open_file` 显式校验 UTF-8 编码，`save_file` 采用同目录临时文件 + `sync_all` + 原子重命名替换；读写错误不再静默吞掉，统一在状态栏展示可点击关闭的错误反馈徽章。

### 6. ✅ 已优化：工作区真实状态与命令行启动参数
- **定位**：[src/git.rs](file:///d:/projects/bytemvp/bmvp-text/src/git.rs)、[src/main.rs](file:///d:/projects/bytemvp/bmvp-text/src/main.rs)、[src/sidebar.rs](file:///d:/projects/bytemvp/bmvp-text/src/sidebar.rs)、[src/file_finder.rs](file:///d:/projects/bytemvp/bmvp-text/src/file_finder.rs)、[src/status_bar.rs](file:///d:/projects/bytemvp/bmvp-text/src/status_bar.rs)
- **已实施优化**：
  - 新增 `src/git.rs`，从工作区向上解析 `.git/HEAD`（支持分支、worktree gitfile 与 detached HEAD），在状态栏展示真实 Git 分支。
  - 放开 `.gitignore`、`.github`、`.env` 等常规点文件显示（仅屏蔽 `.git`、`target`、`node_modules` 及根目录 `.gitignore` 简单条目）。
  - `main.rs` 新增 `parse_startup_args` 支持通过 CLI 参数打开指定目录或文件（如 `bmvp-text <file|dir>`）。

---

## P2 — 架构重构与性能优化

### 1. 拆解 `src/editor.rs` 上帝类并解除组件循环耦合（✅ 已优化）
- **定位**：[src/lib.rs](file:///d:/projects/bytemvp/bmvp-text/src/lib.rs)、[src/ui/editor_view.rs](file:///d:/projects/bytemvp/bmvp-text/src/ui/editor_view.rs)、[src/ui/editor_commands.rs](file:///d:/projects/bytemvp/bmvp-text/src/ui/editor_commands.rs)、[src/ui/editor_input.rs](file:///d:/projects/bytemvp/bmvp-text/src/ui/editor_input.rs)、[src/ui/mod.rs](file:///d:/projects/bytemvp/bmvp-text/src/ui/mod.rs)、[tests/core_integration.rs](file:///d:/projects/bytemvp/bmvp-text/tests/core_integration.rs)
- **已落地修复**：
  - 将原扁平 `src/*.rs` 重构为 `core/`（`buffer.rs`、`syntax.rs`、`sidebar.rs`、`git.rs`）、`config/`（`settings.rs`）、`platform/`（`window_controls.rs`）、`terminal/`（`ansi.rs`、`session.rs`）与 `ui/`（`title_bar.rs`、`tab_bar.rs`、`breadcrumb.rs`、`sidebar_view.rs`、`terminal_dock.rs`、`status_bar.rs`、`find_replace.rs`、`file_finder.rs`、`settings_modal.rs`、`confirm_modal.rs`、`code_canvas.rs`、`editor_view.rs`、`editor_commands.rs`、`editor_input.rs`、`theme.rs`）分层结构，彻底删除旧扁平文件。
  - 全部 UI 子组件移除 `&Context<EditorView>` 反向依赖，统一通过 `src/ui/mod.rs` 中的 `UiAction` / `UiAction1` / `UiAction2` / `UiMouseAction` 回调结构体与宿主通信；新增 `src/lib.rs` 并落地 `tests/core_integration.rs` 集成测试。

### 2. 消除 UI 线程阻塞 I/O 与全量重建开销（✅ 已优化）
- **定位**：[src/ui/file_finder.rs](file:///d:/projects/bytemvp/bmvp-text/src/ui/file_finder.rs)、[src/core/sidebar.rs](file:///d:/projects/bytemvp/bmvp-text/src/core/sidebar.rs)、[src/terminal/session.rs](file:///d:/projects/bytemvp/bmvp-text/src/terminal/session.rs)
- **已落地修复**：
  - `FileFinder` 新增 `begin_open`（生成代数防乱序）、`scan_workspace_files`（后台线程 `WalkDir` 异步扫描）与 `finish_scan` + `fuzzy_score` 文件名/路径模糊匹配打分排序。
  - `Sidebar::toggle_expand` 改为原地增量展开与折叠（仅读取当前目录一级子项并原地 `splice`/`drain`），不再清空整棵目录树重建。
  - `SystemTerminal` 启动检测改用纯内存 `PATH` 扫描 `find_executable_in_path`（避免同步阻塞拉起 `where`/`which`），`kill_session` 将进程树清理与 `wait()` 下沉到后台线程，不阻塞 UI 退出或销毁。

### 3. 高频渲染缓存与事件驱动通知（✅ 已优化）
- **定位**：[src/core/syntax.rs](file:///d:/projects/bytemvp/bmvp-text/src/core/syntax.rs)、[src/ui/theme.rs](file:///d:/projects/bytemvp/bmvp-text/src/ui/theme.rs)、[src/terminal/session.rs](file:///d:/projects/bytemvp/bmvp-text/src/terminal/session.rs)、[src/ui/editor_commands.rs](file:///d:/projects/bytemvp/bmvp-text/src/ui/editor_commands.rs)
- **已落地修复**：
  - `syntax.rs` 新增行级 `HighlightCache`，按行内容哈希与语言缓存词法分析结果；`Theme` 新增 `SyntaxColors` (`theme.syntax.*`)，使 `ZedLight`、`ZedDark`、`OneDark`、`Nord` 拥有各自独立的语法高亮配色。
  - `SystemTerminal` 引入 `AnsiLineCache`（`spans_for_line`）与零依赖跨线程唤醒器 `TerminalNotifier`；`stdout`/`stderr` Reader 线程收到数据后按需唤醒 UI 异步消费任务，彻底移除 `render()` 中的 `poll_output()` 副作用与 50ms 空转轮询。

### 4. 内嵌终端能力升级（✅ 短期已优化 · PTY 保留为中长期演进）
- **定位**：[src/terminal/ansi.rs](file:///d:/projects/bytemvp/bmvp-text/src/terminal/ansi.rs)、[src/terminal/session.rs](file:///d:/projects/bytemvp/bmvp-text/src/terminal/session.rs)、[src/ui/terminal_dock.rs](file:///d:/projects/bytemvp/bmvp-text/src/ui/terminal_dock.rs)
- **已落地短期优化**：
  - 新增 `quote_path_for_shell` 适配 `PowerShell`/`Cmd`/`Bash`/`Zsh`/`Fish` 对含 `$`/`"`/`'` 路径的安全转义。
  - `input_buffer` 的插入/退格/前删改用字节偏移原地 `String::insert`/`remove` 替代整行 `chars().collect()` 重建。
  - `parse_ansi_spans` 新增 SGR `1/22`（粗体）、`3/23`（斜体）、`4/24`（下划线）、`7/27`（反显）并改用单遍扫描 `strip_ansi`；`ShellKind` 与 `AVAILABLE_SHELLS` 新增 `zsh` / `fish`；终端面板新增 `Ctrl+Shift+C` 复制最近输出行与 `Shift + 滚轮` 水平滚动。
- **中长期保留演进**：当前基于匿名管道的方式在按下 `Ctrl+C` 时仍会重启 Shell 会话，且不支持 `vim`/`less`/`\r` 行内刷新等全屏交互程序；后续可按需评估以 `portable-pty` + `vte` 替换管道层。

---

## P3 — 跨平台、测试覆盖与工程化

### 1. 跨平台完整性补全（✅ 已优化）
- **定位**：[src/config/settings.rs](file:///d:/projects/bytemvp/bmvp-text/src/config/settings.rs)、[src/terminal/session.rs](file:///d:/projects/bytemvp/bmvp-text/src/terminal/session.rs)、[src/platform/window_controls.rs](file:///d:/projects/bytemvp/bmvp-text/src/platform/window_controls.rs)、[src/main.rs](file:///d:/projects/bytemvp/bmvp-text/src/main.rs)、[src/ui/title_bar.rs](file:///d:/projects/bytemvp/bmvp-text/src/ui/title_bar.rs)、[src/ui/sidebar_view.rs](file:///d:/projects/bytemvp/bmvp-text/src/ui/sidebar_view.rs)
- **已落地修复**：
  - **跨平台配置目录与完整向后兼容**：`AppConfig::resolve_config_path_for` 与 `config_path()` 支持 Windows（`%APPDATA%` / `%USERPROFILE%`）、macOS（`~/Library/Application Support/bmvp-text/settings.json`）与 Linux（`$XDG_CONFIG_HOME` / `~/.config/bmvp-text/settings.json`）；为 `AppConfig` 与 `EditorSettings` 开启结构体级 `#[serde(default)]`，缺省字段及空 JSON 对象 `{}` 均可无缝回退默认值，并通过 `save_to_path` 临时文件替换写入与 `load_from_path` 明确错误返回。
  - **平台默认值与独立终端**：按 `#[cfg(target_os = ...)]` 为 macOS（默认 `Menlo` / `zsh`）与 Linux（默认 `monospace` / `bash`）及 Windows（默认 `Consolas` / `pwsh`）提供独立默认字体、Shell 与选项表；`SystemTerminal::open_external()` 在 macOS 使用 `open -a Terminal`，Linux 按序探测主流终端模拟器并返回明确 `Result<(), String>`。
  - **macOS 交通灯与快捷键文案适配**：`main.rs` 在 macOS 配置 `traffic_light_position`；`window_controls.rs` 提供 `PRIMARY_MODIFIER_LABEL`（`Cmd`/`Ctrl`）、`USES_MACOS_TRAFFIC_LIGHTS` 与 `MACOS_TRAFFIC_LIGHT_PADDING_PX`；`TitleBar` 与 `SidebarView` 在 macOS 自动保留左侧交通灯安全区并隐藏右侧 Win32 按钮，面包屑已在 P1/P3 测试验证对 Windows `\` 与 POSIX `/` 的完整支持。

### 2. 测试覆盖体系建设（✅ 已优化）
- **定位**：[src/core/buffer.rs](file:///d:/projects/bytemvp/bmvp-text/src/core/buffer.rs)、[src/core/syntax.rs](file:///d:/projects/bytemvp/bmvp-text/src/core/syntax.rs)、[src/core/logging.rs](file:///d:/projects/bytemvp/bmvp-text/src/core/logging.rs)、[src/config/settings.rs](file:///d:/projects/bytemvp/bmvp-text/src/config/settings.rs)、[src/ui/breadcrumb.rs](file:///d:/projects/bytemvp/bmvp-text/src/ui/breadcrumb.rs)、[tests/core_integration.rs](file:///d:/projects/bytemvp/bmvp-text/tests/core_integration.rs)
- **已落地修复**：
  - 全量单测离线化/纯内存化（终端单测统一使用 `SystemTerminal::new_offline`）；补充 `settings.rs` 三端路径与部分 JSON 迁移测试、`syntax.rs` 转义字符串/类型/函数调用/多语言测试、`buffer.rs` 多步撤销重做与 CRLF 复制行/注释测试、`breadcrumb.rs` 双端路径格式化测试及 `logging.rs` 测试。
  - 扩充 [tests/core_integration.rs](file:///d:/projects/bytemvp/bmvp-text/tests/core_integration.rs) 至 5 个端到端跨模块集成用例，`cargo test --all-targets` 共 **43 个用例**（38 单测 + 5 集成测试）全部离线稳定通过。

### 3. CI/CD、打包脚本、结构化日志与文档一致性修复（✅ 已优化）
- **定位**：[src/core/logging.rs](file:///d:/projects/bytemvp/bmvp-text/src/core/logging.rs)、[package/windows/installer.iss](file:///d:/projects/bytemvp/bmvp-text/package/windows/installer.iss)、[scripts/](file:///d:/projects/bytemvp/bmvp-text/scripts/package-windows.ps1)、[.github/workflows/release.yml](file:///d:/projects/bytemvp/bmvp-text/.github/workflows/release.yml)、[README.md](file:///d:/projects/bytemvp/bmvp-text/README.md)
- **已落地修复**：
  - 引入零依赖 [core::logging](file:///d:/projects/bytemvp/bmvp-text/src/core/logging.rs) 统一结构化日志格式，清理 `buffer.rs`、`settings.rs`、`terminal/session.rs`、`terminal_dock.rs`（新增 `on_open_external` 回调与状态栏错误回显）、`editor_commands.rs` 中的静默 `let _ = ...` 错误吞没。
  - 修复 `installer.iss` 默认产物路径（`target\x86_64-pc-windows-msvc\release\...` 与 `/DSourceExe` 参数传递）并补充 `addtopath` 所需的 `[Registry]` 与 `NeedsAddPath` 检查逻辑；统一 `release.yml` 直接调用 `scripts/package-windows.ps1`、`scripts/package-macos.sh`、`scripts/package-linux.sh`（支持 `-SkipBuild`/`SKIP_BUILD=1`，确保 `.deb` 包含 `.desktop`、`icon.svg` 与 `Depends`）。
  - 同步更新 `README.md`（Rust `1.88+` 工具链要求、终端/快捷键/构建说明）、`docs/architecture.md` 与 `AGENTS.md`。
