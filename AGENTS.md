# agent.md — bmvp-text 项目开发与协作规范

> 本文件是 `bmvp-text` 仓库的权威开发规范，所有后续人机协同与 AI Agent 开发任务均须严格遵守本规范。

---

## 1. 项目定位与核心红线

- **项目定位**：基于 **Pure Rust (Edition 2024) + GPUI 0.2**、参考 Zed 视觉语言的极致轻量级代码编辑器。
- **核心红线**：
  1. **轻量与高响应优先**：严禁引入 AI 助手膨胀功能、重量级 LSP 引擎、内置 WebView/浏览器内核或重型异步运行时。
  2. **克制新增依赖**：新增任何外部 crate 前必须评估编译时长、Release 二进制体积及三端兼容性（Windows / macOS / Linux）。
  3. **三端 CI 零容忍**：代码必须在 `windows-latest`、`ubuntu-22.04`、`macos-latest` 三端保持可编译，且无警告通过格式化、Clippy 与全部测试。

---

## 2. 必读上下文与目录导航

开始任何功能开发、Bug 修复或重构前，先按需定位对应文档与模块：

| 文档 / 路径 | 用途 |
| :--- | :--- |
| [docs/architecture.md](docs/architecture.md) | 整体分层架构、模块职责矩阵、键盘事件路由、终端机制与目标演进蓝图 |
| [docs/optimization-roadmap.md](docs/optimization-roadmap.md) | 按 `P0 -> P1 -> P2 -> P3` 排序的问题清单与迭代路线图 |
| [Cargo.toml](Cargo.toml) | 依赖声明与 Release 构建优化参数 |
| `src/` | 编辑器源码（`main.rs`、`lib.rs` 及分层模块 `core/`、`config/`、`platform/`、`terminal/`、`ui/`） |
| `tests/` | 跨模块纯内存集成测试（`core_integration.rs`） |
| `scripts/` 与 `package/` | Windows / macOS / Linux 本地打包脚本与 Inno Setup 配置 |
| `.github/workflows/` | CI 检查（`ci.yml`）与跨平台发布流（`release.yml`） |

---

## 3. 架构与模块化开发规范

1. **禁止继续膨胀根视图与单文件模块**：
   - 根视图已从原 2888 行 `editor.rs` 拆分为 `src/ui/editor_view.rs`（视图状态与渲染）、`src/ui/editor_commands.rs`（命令与文件操作）与 `src/ui/editor_input.rs`（鼠标/键盘/IME 路由）。禁止再向 `editor_view.rs` 直接堆砌新组件或大段渲染代码。
   - 新增 UI 面板或核心功能必须独立建模块放入 `core/`、`ui/`、`terminal/`、`platform/`、`config/`。
2. **严禁反向强耦合**：
   - 新增或重构的 UI 子组件不得接收 `&Context<EditorView>` 并直接读写 `EditorView` 的 `pub` 字段。
   - 子组件与宿主之间必须通过 GPUI `Actions`、`EventEmitter` 或显式回调闭包解耦通信。
3. **UI 线程非阻塞原则**：
   - 严禁在 UI 渲染或按键/点击回调中执行同步递归扫描（如全量 `WalkDir`）、阻塞文件读写、同步进程探测或阻塞等待子进程退出（`wait()`/`taskkill`）。
   - 耗时 I/O 必须放到 `cx.background_executor().spawn(...)` 或专用后台线程，并通过异步通道或 `cx.update_window` 安全回写状态。
4. **`Render::render` 保持纯渲染**：
   - 禁止在 `render()` 内执行状态变更副作用（如轮询消费管道输出、触发持久化写盘）。
   - 文本语法高亮、ANSI 解析等高频计算应逐步建立按行缓存，避免每帧重复堆分配。

---

## 4. 代码安全与避坑铁律

以下规则均提炼自本项目已有真实隐患，编码与 Review 时逐条核对：

1. **UTF-8 字符索引与换行符安全（`TextBuffer`）**：
   - `TextBuffer` 所有位置与选区接口统一使用**字符索引（char index）**。
   - **禁止**把 `str::len()` 的字节差直接加减到字符索引或行起始字符偏移上。
   - **禁止**用 `text.to_lowercase()` 的字节偏移直接切片原字符串 `text[..byte_idx]`（Unicode 大小写转换会改变字节长度，触发 Panic）。
   - 所有行尾截取、行末移动（`End`）、列夹紧逻辑必须同时兼容 **LF (`\n`)** 与 **CRLF (`\r\n`)**，不得假定换行符恒为 1 字符。
2. **词法/文本扫描循环推进保证**：
   - 任何手工编写的字符/字节扫描循环（如 `syntax.rs`、`ansi` 解析），每一条分支与兜底分支都必须保证游标索引严格递增（如至少消费 1 字符），杜绝类似 `#` 字符引发死循环冻结界面的问题。
3. **GPUI 事件传播与异步索引安全**：
   - 嵌套可点击元素（如 Tab 上的 `×` 关闭按钮）及悬浮弹窗内容卡片，必须在 `on_mouse_down` 中显式调用 `cx.stop_propagation()`，防止事件冒泡误触切换或关闭背景遮罩。
   - 跨异步边界（如 `rfd` 文件/保存对话框回调）禁止捕获瞬时数组下标（如 `active_tab`、`active_terminal`）；必须使用稳定唯一 ID（如 `tab_id`）定位目标对象。
4. **视觉与主题一致性**：
   - 禁止在 UI 组件中硬编码具体字体名（如 `"Consolas"`）或脱离 `Theme` 的颜色常量；所有配色与字体应从 `Theme` 与 `EditorSettings` 读取，同时兼顾亮色/暗色主题对比度。
5. **配置结构向后兼容**：
   - `AppConfig` 与 `EditorSettings` 新增或调整字段时，必须提供 `#[serde(default)]`（或结构级默认回退），保证老版本 `settings.json` 升级后不丢失用户偏好；默认路径与默认值需适配 Windows / macOS / Linux 三端。
6. **错误处理与数据安全**：
   - 禁止使用 `let _ = ...` 静默吞掉文件读写、编码转换或子进程启动错误；操作失败应通过 [src/core/logging.rs](src/core/logging.rs) 记录结构化日志并向用户呈现明确反馈。
   - 涉及关闭标签、关闭窗口或退出程序的操作，必须检查未保存（`is_dirty`）状态并提供保护确认，避免数据静默丢失。
7. **平台特性严格隔离**：
   - Win32 / macOS / Linux 原生 API 与 FFI 调用必须隔离在 `window_controls.rs`（或后续 `platform/`）模块内，并使用 `#[cfg(target_os = "...")]` 提供非目标平台的完整 fallback 实现。

---

## 5. 测试、构建与提交验收标准

### 5.1 强制检查命令
每次代码修改完成、准备交付前，必须运行并确保以下命令全部以 **0 错误、0 警告**通过：

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```

### 5.2 测试编写规范
- **修 Bug 必补复现单测**：修复 P0/P1 类逻辑缺陷（如 `#` 高亮、CRLF 移动、Unicode 查找、注释切换）时，必须同步补充可在故障场景下失败、修复后通过的单元测试。
- **单测必须离线且纯内存化**：单元测试禁止拉起真实的 `pwsh`/`cmd`/`bash` 子进程，禁止读写用户真实的 `%APPDATA%` / `$HOME` 配置目录，禁止依赖图形窗口环境。
- **集成测试入口**：`src/lib.rs` 已对外暴露纯逻辑核心模块，跨模块行为回归测试统一放置在 `tests/` 目录下（如 `tests/core_integration.rs`）。

---

## 6. 当前迭代顺序与文档同步要求

### 6.1 迭代优先级与状态
后续开发与缺陷治理按 [docs/optimization-roadmap.md](docs/optimization-roadmap.md) 的优先级推进（`P0` 至 `P3` 核心条目均已落地，后续新增需求按同标准维护）：
1. **P0（已清零）**：修复 `syntax.rs` `#` 死循环、`buffer.rs` UTF-8/CRLF/注释缺陷、鼠标事件冒泡及未保存退出保护。
2. **P1（已闭环）**：补全 IME 输入法、精确字符定位与拖拽选区、光标行高亮/选区渲染修复、查找替换闭环、Tab 独立滚动与动态视口、真实 Git 分支与 CLI 参数打开。
3. **P2（已完成）**：拆分 `editor.rs`、解除子组件反向耦合、I/O 异步化、引入行级缓存与事件驱动终端通知。
4. **P3（已完成）**：补齐 macOS/Linux 配置与终端适配、结构化日志、完善测试覆盖率、统一打包脚本与 CI 发布流水线及文档同步。

### 6.2 文档同步规则
凡发生以下变更，必须在同一提交/任务中同步更新对应文档：
- 模块边界调整、新子系统接入 -> 更新 [docs/architecture.md](docs/architecture.md)
- 完成路线图中的条目或新发现重大隐患 -> 更新 [docs/optimization-roadmap.md](docs/optimization-roadmap.md)
- 新增/变更开发红线或验收流程 -> 更新 [agent.md](agent.md)
- 变更用户可见功能、快捷键、支持平台或最低 Rust 版本要求 -> 更新 [README.md](README.md)
