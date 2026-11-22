# bmvp-text

> 一个完全参考 **Zed** 页面设计语言与交互体验的极致轻量级文本编辑器。  
> 基于 **Pure Rust + GPUI 0.2** 现代图形框架构建，追求秒开、低内存占用与纯粹打字体验。  
> **零 AI 膨胀 · 零重量级 LSP 进程消耗**，仅保留最核心、最顺手的轻量高响应代码编辑能力。

---

## 🎨 页面布局与视觉设计（完全参考 Zed 设计语言）

```text
┌────────────────────────────────────────────────────────────────────────────────────────┐
│ [B] bmvp-text · my-project      [ 🔍 Search or jump to file...  Ctrl+P ]   ⊞ ⌨ ⚙ │ — □ ✕│
├─────────────────┬──────────────────────────────────────────────────────────────────────┤
│ PROJECT   + ⟳ 📂│ [🦀 main.rs ×] [⚙ Cargo.toml ×] [+]                      🔍 ⌨ ⚙       │
├─────────────────┼──────────────────────────────────────────────────────────────────────┤
│ ⌄ src           │ 📁 my-project › src › main.rs                                        │
│   🦀 main.rs    ├──────┬───────────────────────────────────────────────────────────────┤
│   🦀 editor.rs  │    1 │ fn main() {                                                   │
│   🦀 buffer.rs  │    2 │     println!("Zed authentic styling!");                       │
│ ⚙ Cargo.toml    │    3 │ }                                                             │
│ 📝 README.md    │      │                                                               │
│                 ├──────┴───────────────────────────────────────────────────────────────┤
│                 │ [>_ cmd 1 ×] [>_ cmd 2 ×] [+]          📁 D:\code  ⟳ Profile  ↗ ⌫  ✕│
│                 │ D:\code> cargo check                                                 │
│                 │     Finished `dev` profile in 0.05s                                  │
├─────────────────┴──────────────────────────────────────────────────────────────────────┤
│ [⊞ Explorer]  ⎇ main  ● Saved  [⌨ Terminal]   Ln 2, Col 5  Spaces: 4  UTF-8  LF [Rust]⚙│
└────────────────────────────────────────────────────────────────────────────────────────┘
```

1. **整合式顶部栏（Integrated Title Bar & Command Palette）**：
   - 采用现代沉浸式无边框设计，全宽整合。
   - **左侧**：品牌 Logo 标志、应用名及当前工程目录名称，支持双击窗口最大化/还原与全栏拖拽。
   - **居中**：Zed 标志性搜索胶囊（`[ 🔍 Search or jump to file...  Ctrl+P ]`），带平滑悬浮微光与快捷键提示。
   - **右侧**：侧边栏 `⊞`、终端 Dock `⌨`、设置 `⚙` 快捷开关，搭配原生窗口控制按钮（最小化、最大化/恢复、关闭）。

2. **Zed 风格标签栏（Tab Bar）与无缝衔接**：
   - **激活标签（Active Tab）**：顶边缘带 2px 品牌主色线条（`theme.accent`），底色与下方编辑器（`bg_editor`）**物理融为一体、无横线分割**，打造 Zed 标志性的无缝视觉流。
   - **非激活标签**：底色与背景融合，右侧与下侧带有精致的弱分割线（`border_subtle`）。
   - **文件图标与状态**：按语言类型显示专属图标（Rust 🦀、TOML ⚙、Markdown 📝、JSON 📦、Lock 🔒、Python 🐍 等），内容修改时显示琥珀色脏数据圆点（`dirty_indicator`）。
   - **新建与关闭**：每个 Tab 均带平滑悬浮关闭按钮 `×`，右侧配备新建按钮 `+`。

3. **面包屑导航栏（Breadcrumb Bar）**：
   - 位于标签栏下方（高度 24px），底色与编辑器保持一致。
   - 动态解析展示文件层级路径（如 `📁 bmvp-text › src › editor.rs`），视觉轻盈优雅。

4. **鼠标拖拽自由调整面板大小（Drag Resizers）**：
   - **侧边栏 ↔ 编辑区**：纵向 3px 隐形把手，悬浮或拖拽时高亮主题主色（`cursor_col_resize`），双击自动复位默认宽度（230px）。
   - **编辑区 ↔ 命令行**：横向 3px 隐形把手，支持垂直拖拽调整终端高度（`cursor_row_resize`），双击自动复位默认高度（240px）。
   - 尺寸实时自动持久化至本地用户设置。

5. **内嵌系统终端 Dock（Embedded Terminal Dock）**：
   - **事件驱动异步交互**：支持 `pwsh`、`powershell`、`cmd`、`bash`、`zsh`、`fish`，通过后台 Reader 线程与事件唤醒及时刷新输出，告别 50ms 空转轮询。
   - **多命令行 Tab 与样式支持**：支持创建/关闭多个终端标签、ANSI 16/256/TrueColor 与粗体/斜体/下划线/反显（SGR 1/3/4/7）、`Shift + 滚轮` 横向滚动与 `Ctrl+Shift+C` 复制最近输出。
   - **工作目录同步与独立窗口**：支持执行后同步工作目录，点击 `↗` 一键呼出系统外部终端（Windows `wt.exe`/`cmd`、macOS `Terminal.app`、Linux 主流终端模拟器）。
   - **系统经典黑底银字**：遵循控制台纯正暗色（`#0c0c0e`），内联命令行输入提示符。

6. **Zed 风格底部状态栏（Status Bar）**：
   - 高度精简至 24px，左侧放置工程面板开关、Git 分支（`⎇ main`）、保存状态指示灯，右侧展示光标行号列号（`Ln X, Col Y`）、选区字数（`X sel`）、Tab 缩进设置（点击循环 2/4/8）、编码格式（`UTF-8`）、换行符（`LF`/`CRLF`）、语言药丸徽章（`[Rust]`）以及偏好设置图标（`⚙`）。

---

## ⌨️ 常用快捷键列表

| 快捷键（Windows/Linux · macOS 使用 `Cmd`） | 功能描述 |
| :--- | :--- |
| **Ctrl + P** | 打开全局快速文件查找弹窗（File Finder / Quick Open，支持模糊权重排序） |
| **Ctrl + F** / **Ctrl + H** | 打开/关闭文件内查找框或查找替换面板（支持大小写/全词/正则 `Aa`/`W`/`.*`） |
| **Ctrl + `** | 展开/收起底部交互式终端 Dock |
| **Ctrl + ,** | 打开偏好设置弹窗（切换主题、字体、终端 Shell、Tab 宽度等） |
| **Ctrl + B** | 展开/折叠左侧项目工程面板（Explorer） |
| **Ctrl + O** | 打开系统文件选择对话框 |
| **Ctrl + S** | 保存当前文件（原子写入；未命名文件弹出另存为对话框） |
| **Ctrl + N** / **Ctrl + W** | 新建 / 关闭标签页（在终端获得焦点时关闭当前终端 Tab；未保存将弹窗保护确认） |
| **Ctrl + T** | 在终端获得焦点时新建终端 Tab |
| **Ctrl + Shift + C** | 在终端获得焦点时复制最近终端输出行 |
| **Ctrl + Z** / **Ctrl + Y** | 撤销 / 重做（也支持 `Ctrl + Shift + Z` 重做） |
| **Ctrl + C** / **X** / **V** | 复制、剪切、粘贴（终端中 `Ctrl+C` 为中止当前命令，`Ctrl+V` 为粘贴） |
| **Ctrl + A** | 全选当前文档 |
| **Ctrl + D** | 快速复制当前行（Duplicate Line） |
| **Ctrl + /** | 快速切换当前行注释（Toggle Comment） |
| **Ctrl + Q** | 退出应用（存在未保存标签时触发确认保护） |
| **Shift + 方向键** | 扩展/缩减文本选区 |
| **Tab** | 插入设定宽度的空格缩进（终端中为文件名自动补全） |
| **Enter** | 智能保持缩进换行（遇到 `{` 自动递进缩进） |
| **Esc** | 关闭当前激活的浮动弹窗或退出焦点 |

---

## 🚀 CI / CD 与多平台构建流程

本项目已配置基于 **GitHub Actions** 的全自动 CI/CD 与跨平台安装包构建流程：

### 1. 持续集成（CI: `.github/workflows/ci.yml`）
- **触发条件**：对 `main`、`develop` 分支的代码 Push 或 PR。
- **矩阵检查**：涵盖 Windows (`windows-latest`)、macOS (`macos-latest`)、Linux (`ubuntu-22.04`)。
- **执行内容**：`cargo check`、`cargo test`、`cargo clippy`、`cargo fmt`。

### 2. 持续交付与版本发布（CD: `.github/workflows/release.yml`）
- **触发条件**：
  - **正式版**：创建并推送版本 Tag（如 `git tag v0.1.0 && git push origin v0.1.0`）。
  - **自动滚动测试版**：推送至 `main` 分支且 `CI` 通过后，自动构建并更新 `dev-latest` Pre-release（支持在 Actions 中手动触发）。
- **自动化产物构建**：
  - **Windows (x86_64)**：构建便携版 `bmvp-text-windows-x64.zip` 与独立安装包。
  - **macOS (Apple Silicon & Intel)**：生成标准的 `.app` 应用程序包并打包为 `.dmg` 与 `.tar.gz`。
  - **Linux (x86_64)**：自动打包便携版 `.tar.gz` 以及 Debian/Ubuntu 标准 `.deb` 安装包。
- **自动发布**：各平台产物自动上传至 GitHub Releases 页面，并自动生成 Release Notes。

### 3. 本地打包脚本

项目提供了针对各操作系统的快速打包构建脚本：

- **Windows 打包**：
  ```powershell
  .\scripts\package-windows.ps1
  ```
  产物位于 `dist/bmvp-text-windows-x64.zip`。若系统安装有 Inno Setup，将自动编译生成安装向导。

- **macOS 打包**：
  ```bash
  chmod +x ./scripts/package-macos.sh
  ./scripts/package-macos.sh aarch64-apple-darwin
  ```
  产物位于 `dist/bmvp-text-aarch64-apple-darwin.dmg`。

- **Linux 打包**：
  ```bash
  chmod +x ./scripts/package-linux.sh
  ./scripts/package-linux.sh
  ```
  产物位于 `dist/bmvp-text-linux-x64.tar.gz` 与 `dist/bmvp-text-linux-x64.deb`。

---

## 🛠️ 本地开发与构建

确保已安装 **Rust 1.88+** 工具链（项目使用 Edition `2024`、`let`-chains 与 `is_multiple_of` / `as_chunks`，已验证兼容最新稳定版及 `rustc 1.100.0`）：

```bash
# 检查格式化
cargo fmt --all -- --check

# 检查编译与 Clippy 零警告门禁
cargo clippy --all-targets -- -D warnings

# 运行全部单元测试与集成测试
cargo test --all-targets

# 启动编辑器（支持直接传入目录或文件路径）
cargo run -- . src/main.rs

# 构建高性能发布二进制
cargo build --release
```
