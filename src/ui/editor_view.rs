use crate::StartupArgs;
use crate::config::settings::{AppConfig, EditorSettings};
use crate::core::buffer::{Position, TextBuffer};
use crate::core::git::detect_git_branch;
use crate::core::sidebar::Sidebar;
use crate::core::syntax::HighlightCache;
use crate::platform::window_controls;
use crate::terminal::session::{SystemTerminal, TerminalNotifier};
use crate::ui::breadcrumb::{BreadcrumbBar, BreadcrumbCallbacks};
use crate::ui::code_canvas::{CodeCanvas, CodeLayoutState};
use crate::ui::confirm_modal::{ConfirmModal, PendingClose};
use crate::ui::file_finder::{FileFinder, FileFinderCallbacks};
use crate::ui::find_replace::{FindReplaceCallbacks, FindState};
use crate::ui::settings_modal::{FontDropdownTarget, SettingsModal, SettingsModalCallbacks};
use crate::ui::sidebar_view::{SidebarCallbacks, SidebarView};
use crate::ui::status_bar::{StatusBar, StatusBarCallbacks};
use crate::ui::tab_bar::{TabBar, TabBarCallbacks, TabInfo};
use crate::ui::terminal_dock::{TerminalDock, TerminalDockCallbacks};
use crate::ui::theme::Theme;
use crate::ui::title_bar::{TitleBar, TitleBarCallbacks};
use gpui::*;
use std::cell::RefCell;
use std::ops::Range;
use std::path::PathBuf;
use std::rc::Rc;

pub struct EditorView {
    pub tabs: Vec<TextBuffer>,
    pub active_tab: usize,
    pub sidebar: Sidebar,
    pub show_sidebar: bool,
    pub show_settings: bool,
    pub show_terminal: bool,
    pub terminals: Vec<SystemTerminal>,
    pub active_terminal: usize,
    pub next_terminal_id: usize,
    pub terminal_focused: bool,
    pub terminal_scroll_left: f32,
    pub file_finder: FileFinder,
    pub find_state: FindState,
    pub find_focused: bool,
    pub settings: EditorSettings,
    pub theme: Theme,
    pub focus_handle: FocusHandle,
    pub code_layout: CodeLayoutState,
    pub highlight_cache: RefCell<HighlightCache>,
    pub ime_marked_range: Option<Range<usize>>,
    pub(super) is_selecting_text: bool,
    pub git_branch: Option<String>,
    pub status_message: Option<(String, bool)>,

    // 面板尺寸与拖拽状态
    pub sidebar_width: f32,
    pub terminal_height: f32,
    pub resizing_sidebar: bool,
    pub resizing_terminal: bool,
    pub(super) pending_window_drag: Option<Point<Pixels>>,
    pub(super) terminal_notifier: TerminalNotifier,
    pub(super) terminal_listener_active: bool,
    pub(super) window_handle: AnyWindowHandle,
    pub(super) pending_close: Option<PendingClose>,
    pub(super) allow_quit: bool,

    // 字体下拉列表状态
    pub system_fonts: Vec<String>,
    pub open_font_dropdown: Option<FontDropdownTarget>,
    pub font_dropdown_query: String,
    pub font_dropdown_scroll: usize,
}

impl EditorView {
    pub fn new(window: &mut Window, startup: StartupArgs, cx: &mut Context<Self>) -> Self {
        let mod_key = window_controls::PRIMARY_MODIFIER_LABEL;
        let initial_text = format!(
            r#"// Welcome to bmvp-text
// A lightweight, high-performance editor inspired by Zed
// Pure Rust + GPUI · Zero AI bloat · Instant response

fn main() {{
    let editor = "Zed-style Lightweight Editor";
    println!("Fast, clean, and distraction-free: {{}}", editor);
}}

// Shortcuts:
//   {m}+P       : File Finder / Quick Open
//   {m}+F / H   : Find / Replace in File
//   {m}+`       : Toggle Terminal Dock
//   {m}+,       : Preferences (Themes, Fonts, Shell)
//   {m}+B       : Toggle Project Explorer
//   {m}+O / S   : Open / Save File
//   {m}+N / W   : New / Close Tab
//   {m}+Z / Y   : Undo / Redo
//   {m}+C / X / V : Clipboard Copy, Cut, Paste
//   {m}+A       : Select All
//   {m}+D       : Duplicate Line
//   {m}+/       : Toggle Comment
//   {m}+Q       : Quit Application
"#,
            m = mod_key
        );
        let config = AppConfig::load();
        let mut sidebar = Sidebar::new();
        if let Some(ref root) = startup.root_dir {
            sidebar.set_root(root.clone());
        }
        let git_branch = sidebar.root_path.as_deref().and_then(detect_git_branch);

        let mut tabs = Vec::new();
        let mut status_message = None;
        for file in &startup.files {
            if file.exists() {
                match TextBuffer::open_file(file) {
                    Ok(buf) => tabs.push(buf),
                    Err(err) => {
                        status_message = Some((format!("Open failed: {}", err), true));
                    }
                }
            } else {
                tabs.push(TextBuffer::new_with_path(file));
            }
        }
        if tabs.is_empty() {
            tabs.push(TextBuffer::from_str(&initial_text));
        }

        Self {
            tabs,
            active_tab: 0,
            sidebar,
            show_sidebar: true,
            show_settings: false,
            show_terminal: false,
            terminals: Vec::new(),
            active_terminal: 0,
            next_terminal_id: 1,
            terminal_focused: false,
            terminal_scroll_left: 0.0,
            file_finder: FileFinder::new(),
            find_state: FindState::new(),
            find_focused: false,
            theme: Theme::from_mode(config.theme_mode),
            settings: config.editor.clone(),
            focus_handle: cx.focus_handle(),
            code_layout: CodeLayoutState::default(),
            highlight_cache: RefCell::new(HighlightCache::new()),
            ime_marked_range: None,
            is_selecting_text: false,
            git_branch,
            status_message,
            sidebar_width: config.editor.sidebar_width,
            terminal_height: config.editor.terminal_height,
            resizing_sidebar: false,
            resizing_terminal: false,
            pending_window_drag: None,
            terminal_notifier: TerminalNotifier::new(),
            terminal_listener_active: false,
            window_handle: window.window_handle(),
            pending_close: None,
            allow_quit: false,
            system_fonts: Vec::new(),
            open_font_dropdown: None,
            font_dropdown_query: String::new(),
            font_dropdown_scroll: 0,
        }
    }

    fn cb0(
        cx: &Context<Self>,
        f: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> crate::ui::UiAction {
        let weak = cx.weak_entity();
        Rc::new(move |win, cx| {
            let _ = weak.update(cx, |this, cx| f(this, win, cx));
        })
    }

    fn cb1<A: 'static>(
        cx: &Context<Self>,
        f: impl Fn(&mut Self, A, &mut Window, &mut Context<Self>) + 'static,
    ) -> crate::ui::UiAction1<A> {
        let weak = cx.weak_entity();
        Rc::new(move |arg, win, cx| {
            let _ = weak.update(cx, |this, cx| f(this, arg, win, cx));
        })
    }

    fn cb2<A: 'static, B: 'static>(
        cx: &Context<Self>,
        f: impl Fn(&mut Self, A, B, &mut Window, &mut Context<Self>) + 'static,
    ) -> crate::ui::UiAction2<A, B> {
        let weak = cx.weak_entity();
        Rc::new(move |a, b, win, cx| {
            let _ = weak.update(cx, |this, cx| f(this, a, b, win, cx));
        })
    }

    fn render_editor_pane(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = &self.theme;
        let line_height =
            px((self.settings.font_size * self.settings.line_height_factor).max(14.0));
        let font_family = self.settings.font_family.clone();
        let show_line_numbers = self.settings.show_line_numbers;

        let Some(buffer) = self.current_buffer() else {
            return div()
                .flex_1()
                .relative()
                .flex()
                .items_center()
                .justify_center()
                .bg(theme.bg_editor)
                .text_color(theme.text_muted)
                .child(div().absolute().inset_0().child(CodeCanvas {
                    editor: cx.entity(),
                }))
                .child(format!(
                    "No Open Files — Press {}+P to Open",
                    window_controls::PRIMARY_MODIFIER_LABEL
                ))
                .into_any_element();
        };

        let cursor_pos = buffer.cursor_position();
        let total_lines = buffer.len_lines();
        let visible_line_count = self.editor_viewport_lines(Some(window));
        let start_line = buffer.scroll_top().min(total_lines.saturating_sub(1));
        let end_line = (start_line + visible_line_count).min(total_lines);

        let mut gutter_elements = Vec::new();
        if show_line_numbers {
            for row_idx in start_line..end_line {
                let is_cursor_line = row_idx == cursor_pos.row;
                let line_num_str = format!("{}", row_idx + 1);
                let gutter_col = if is_cursor_line {
                    theme.text_primary
                } else {
                    theme.text_muted
                };
                let gutter_bg = if is_cursor_line {
                    theme.bg_active_line
                } else {
                    theme.bg_gutter
                };

                gutter_elements.push(
                    div()
                        .h(line_height)
                        .bg(gutter_bg)
                        .flex()
                        .items_center()
                        .justify_end()
                        .pr_3()
                        .text_color(gutter_col)
                        .cursor_pointer()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _, _win, cx| {
                                if let Some(buf) = this.current_buffer_mut() {
                                    let target = buf.position_to_char(Position {
                                        row: row_idx,
                                        col: 0,
                                    });
                                    buf.set_cursor(target, false);
                                    this.terminal_focused = false;
                                    this.find_focused = false;
                                    cx.notify();
                                }
                            }),
                        )
                        .child(line_num_str)
                        .into_any_element(),
                );
            }
        }

        let gutter_div = if show_line_numbers {
            Some(
                div()
                    .w(px(54.0))
                    .h_full()
                    .bg(theme.bg_gutter)
                    .py_1()
                    .font_family(font_family)
                    .children(gutter_elements)
                    .into_any_element(),
            )
        } else {
            None
        };

        let find_overlay = if self.find_state.is_open {
            Some(
                self.find_state
                    .render(
                        theme,
                        self.find_focused,
                        FindReplaceCallbacks {
                            on_toggle_replace: Self::cb0(cx, |this, _win, cx| {
                                this.find_state.show_replace = !this.find_state.show_replace;
                                this.find_focused = true;
                                cx.notify();
                            }),
                            on_focus_field: Self::cb1(cx, |this, field, _win, cx| {
                                this.find_focused = true;
                                this.find_state.active_field = field;
                                cx.notify();
                            }),
                            on_toggle_case_sensitive: Self::cb0(cx, |this, _win, cx| {
                                this.find_state.toggle_case_sensitive();
                                this.update_find_matches();
                                cx.notify();
                            }),
                            on_toggle_whole_word: Self::cb0(cx, |this, _win, cx| {
                                this.find_state.toggle_whole_word();
                                this.update_find_matches();
                                cx.notify();
                            }),
                            on_toggle_regex: Self::cb0(cx, |this, _win, cx| {
                                this.find_state.toggle_regex();
                                this.update_find_matches();
                                cx.notify();
                            }),
                            on_prev_match: Self::cb0(cx, |this, _win, cx| {
                                this.prev_find_match(cx);
                            }),
                            on_next_match: Self::cb0(cx, |this, _win, cx| {
                                this.next_find_match(cx);
                            }),
                            on_close: Self::cb0(cx, |this, _win, cx| {
                                this.find_state.close();
                                this.find_focused = false;
                                cx.notify();
                            }),
                            on_replace_current: Self::cb0(cx, |this, _win, cx| {
                                this.replace_current_match(cx);
                            }),
                            on_replace_all: Self::cb0(cx, |this, _win, cx| {
                                this.replace_all_matches(cx);
                            }),
                        },
                    )
                    .into_any_element(),
            )
        } else {
            None
        };

        div()
            .flex_1()
            .relative()
            .flex()
            .bg(theme.bg_editor)
            .overflow_hidden()
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _win, cx| {
                let (delta_x, delta_y) = match event.delta {
                    ScrollDelta::Pixels(p) => (f32::from(p.x), (p.y / px(20.0)) as i32),
                    ScrollDelta::Lines(l) => (l.x * 24.0, l.y as i32),
                };
                if let Some(buf) = this.current_buffer_mut() {
                    if event.modifiers.shift && delta_y != 0 {
                        let next_left = buf.scroll_left() - (delta_y as f32) * 24.0;
                        buf.set_scroll_left(next_left);
                    } else {
                        if delta_y < 0 {
                            let max_scroll = buf.len_lines().saturating_sub(1);
                            let next = buf
                                .scroll_top()
                                .saturating_add(delta_y.unsigned_abs() as usize)
                                .min(max_scroll);
                            buf.set_scroll_top(next);
                        } else if delta_y > 0 {
                            let next = buf.scroll_top().saturating_sub(delta_y as usize);
                            buf.set_scroll_top(next);
                        }
                        if delta_x.abs() > 0.1 {
                            buf.set_scroll_left(buf.scroll_left() - delta_x);
                        }
                    }
                }
                cx.notify();
            }))
            .children(gutter_div)
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .cursor_text()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, _win, cx| {
                            this.terminal_focused = false;
                            this.find_focused = false;
                            this.ime_marked_range = None;
                            let total_chars = this.current_buffer().map_or(0, |b| b.len_chars());
                            let hit = this.code_layout.hit_test(event.position, total_chars);
                            if let Some((row, char_idx)) = hit {
                                this.is_selecting_text = event.click_count == 1;
                                if let Some(buf) = this.current_buffer_mut() {
                                    if event.click_count >= 3 {
                                        buf.select_line_at(row);
                                    } else if event.click_count == 2 {
                                        buf.select_word_at(char_idx);
                                    } else {
                                        buf.set_cursor(char_idx, event.modifiers.shift);
                                    }
                                }
                            } else {
                                this.is_selecting_text = false;
                            }
                            cx.notify();
                        }),
                    )
                    .child(CodeCanvas {
                        editor: cx.entity(),
                    }),
            )
            .children(find_overlay)
            .into_any_element()
    }
}

impl Render for EditorView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_maximized = window_controls::is_window_maximized(window);
        let project_name = self
            .sidebar
            .root_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Open Recent Project".into());

        let theme = &self.theme;

        let sidebar_element = if self.show_sidebar {
            Some(SidebarView::render(
                &self.sidebar,
                &project_name,
                self.sidebar_width,
                &self.settings.font_family,
                window.viewport_size().height,
                theme,
                SidebarCallbacks {
                    on_header_mouse_down: Rc::new(cx.listener(
                        |this, event: &MouseDownEvent, win, cx| {
                            if event.click_count >= 2 {
                                this.pending_window_drag = None;
                                window_controls::toggle_maximize(win);
                                cx.notify();
                            } else {
                                this.pending_window_drag = Some(event.position);
                            }
                        },
                    )),
                    on_new_tab: Self::cb0(cx, |this, _win, cx| {
                        this.new_tab(cx);
                    }),
                    on_refresh: Self::cb0(cx, |this, _win, cx| {
                        this.sidebar.refresh();
                        this.refresh_git_branch();
                        cx.notify();
                    }),
                    on_open_folder: Self::cb0(cx, |this, _win, cx| {
                        this.open_folder_dialog(cx);
                    }),
                    on_toggle_dir: Self::cb1(cx, |this, idx, _win, cx| {
                        this.sidebar.toggle_expand(idx);
                        cx.notify();
                    }),
                    on_open_file: Self::cb1(cx, |this, path: PathBuf, _win, cx| {
                        this.open_file_path(&path, cx);
                    }),
                    on_scroll: Self::cb2(cx, |this, delta_y: i32, max_vis, _win, cx| {
                        if delta_y < 0 {
                            this.sidebar
                                .scroll_down(delta_y.unsigned_abs() as usize, max_vis);
                        } else if delta_y > 0 {
                            this.sidebar.scroll_up(delta_y as usize);
                        }
                        cx.notify();
                    }),
                    on_hide_sidebar: Self::cb0(cx, |this, _win, cx| {
                        this.show_sidebar = false;
                        cx.notify();
                    }),
                    on_open_settings: Self::cb0(cx, |this, _win, cx| {
                        this.toggle_settings_modal(cx);
                    }),
                },
            ))
        } else {
            None
        };

        let sidebar_resizer = if self.show_sidebar {
            let is_resizing = self.resizing_sidebar;
            Some(
                div()
                    .w(px(2.0))
                    .h_full()
                    .cursor_col_resize()
                    .bg(if is_resizing {
                        theme.accent
                    } else {
                        theme.border_subtle
                    })
                    .hover(|s| s.bg(theme.accent))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, _win, cx| {
                            if event.click_count == 2 {
                                this.sidebar_width = 230.0;
                                this.resizing_sidebar = false;
                                this.save_config();
                            } else {
                                this.resizing_sidebar = true;
                            }
                            cx.notify();
                        }),
                    )
                    .into_any_element(),
            )
        } else {
            None
        };

        let right_title_bar = TitleBar::render(
            &project_name,
            is_maximized,
            self.show_sidebar,
            self.show_terminal,
            self.show_settings,
            theme,
            TitleBarCallbacks {
                on_toggle_sidebar: Rc::new(cx.listener(|this, _, _win, cx| {
                    this.show_sidebar = !this.show_sidebar;
                    cx.notify();
                })),
                on_open_folder: Rc::new(cx.listener(|this, _, _win, cx| {
                    this.open_folder_dialog(cx);
                })),
                on_drag_area_mouse_down: Rc::new(cx.listener(
                    |this, event: &MouseDownEvent, win, cx| {
                        if event.click_count >= 2 {
                            this.pending_window_drag = None;
                            window_controls::toggle_maximize(win);
                            cx.notify();
                        } else {
                            this.pending_window_drag = Some(event.position);
                        }
                    },
                )),
                on_open_finder: Rc::new(cx.listener(|this, _, _win, cx| {
                    this.open_file_finder(cx);
                })),
                on_toggle_terminal: Rc::new(cx.listener(|this, _, _win, cx| {
                    this.toggle_terminal(cx);
                })),
                on_toggle_settings: Rc::new(cx.listener(|this, _, _win, cx| {
                    this.toggle_settings_modal(cx);
                })),
                on_quit: Rc::new(cx.listener(|this, _, _win, cx| {
                    this.request_quit(cx);
                })),
            },
        );

        let tab_infos: Vec<TabInfo> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(idx, tab)| TabInfo {
                title: tab.file_name(),
                is_dirty: tab.is_dirty(),
                is_active: idx == self.active_tab,
            })
            .collect();

        let tab_bar = TabBar::render(
            &tab_infos,
            self.active_tab,
            theme,
            TabBarCallbacks {
                on_prev_tab: Self::cb0(cx, |this, _win, cx| {
                    if this.active_tab > 0 {
                        this.active_tab -= 1;
                        this.terminal_focused = false;
                        cx.notify();
                    }
                }),
                on_next_tab: Self::cb0(cx, |this, _win, cx| {
                    if this.active_tab + 1 < this.tabs.len() {
                        this.active_tab += 1;
                        this.terminal_focused = false;
                        cx.notify();
                    }
                }),
                on_select_tab: Self::cb1(cx, |this, idx, _win, cx| {
                    this.active_tab = idx;
                    this.terminal_focused = false;
                    cx.notify();
                }),
                on_close_tab: Self::cb1(cx, |this, idx, _win, cx| {
                    this.request_close_tab(idx, cx);
                }),
                on_new_tab: Self::cb0(cx, |this, _win, cx| {
                    this.new_tab(cx);
                }),
            },
        );

        let path_display = BreadcrumbBar::format_path(
            self.current_buffer().and_then(|b| b.file_path()),
            self.sidebar.root_path.as_deref(),
        );
        let breadcrumb_bar = BreadcrumbBar::render(
            path_display,
            self.find_state.is_open,
            self.settings.show_line_numbers,
            self.show_terminal,
            theme,
            BreadcrumbCallbacks {
                on_open_find: Self::cb0(cx, |this, _win, cx| {
                    this.open_find_bar(cx);
                }),
                on_toggle_line_numbers: Self::cb0(cx, |this, _win, cx| {
                    this.settings.show_line_numbers = !this.settings.show_line_numbers;
                    this.save_config();
                    cx.notify();
                }),
                on_toggle_terminal: Self::cb0(cx, |this, _win, cx| {
                    this.toggle_terminal(cx);
                }),
                on_toggle_settings: Self::cb0(cx, |this, _win, cx| {
                    this.toggle_settings_modal(cx);
                }),
            },
        );

        let editor_pane = self.render_editor_pane(window, cx);

        let terminal_resizer = if self.show_terminal {
            let is_resizing = self.resizing_terminal;
            Some(
                div()
                    .h(px(2.0))
                    .w_full()
                    .cursor_row_resize()
                    .bg(if is_resizing {
                        theme.accent
                    } else {
                        theme.border_subtle
                    })
                    .hover(|s| s.bg(theme.accent))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, event: &MouseDownEvent, _win, cx| {
                            if event.click_count == 2 {
                                this.terminal_height = 240.0;
                                this.resizing_terminal = false;
                                this.save_config();
                            } else {
                                this.resizing_terminal = true;
                            }
                            cx.notify();
                        }),
                    )
                    .into_any_element(),
            )
        } else {
            None
        };

        let bottom_terminal = if self.show_terminal && !self.terminals.is_empty() {
            Some(TerminalDock::render(
                &self.terminals,
                self.active_terminal,
                self.terminal_height,
                self.terminal_scroll_left,
                self.terminal_focused,
                &self.settings,
                theme,
                TerminalDockCallbacks {
                    on_focus: Self::cb0(cx, |this, _win, cx| {
                        this.terminal_focused = true;
                        cx.notify();
                    }),
                    on_select_tab: Self::cb1(cx, |this, idx, _win, cx| {
                        this.active_terminal = idx;
                        this.terminal_focused = true;
                        this.terminal_scroll_left = 0.0;
                        cx.notify();
                    }),
                    on_close_tab: Self::cb1(cx, |this, idx, _win, cx| {
                        this.close_terminal_tab(idx, cx);
                    }),
                    on_new_tab: Self::cb0(cx, |this, _win, cx| {
                        this.new_terminal_tab(cx);
                    }),
                    on_interrupt: Self::cb0(cx, |this, _win, cx| {
                        if let Some(t) = this.current_terminal_mut() {
                            t.interrupt();
                        }
                        cx.notify();
                    }),
                    on_scroll_to_bottom: Self::cb0(cx, |this, _win, cx| {
                        if let Some(t) = this.current_terminal_mut() {
                            t.scroll_to_bottom();
                        }
                        cx.notify();
                    }),
                    on_restart: Self::cb0(cx, |this, _win, cx| {
                        this.restart_terminal();
                        cx.notify();
                    }),
                    on_open_external: Self::cb0(cx, |this, _win, cx| {
                        if let Some(term) = this.current_terminal()
                            && let Err(err) = term.open_external()
                        {
                            this.set_error_message(format!(
                                "Open external terminal failed: {}",
                                err
                            ));
                        }
                        cx.notify();
                    }),
                    on_clear: Self::cb0(cx, |this, _win, cx| {
                        if let Some(t) = this.current_terminal_mut() {
                            t.clear();
                        }
                        this.terminal_scroll_left = 0.0;
                        cx.notify();
                    }),
                    on_close_dock: Self::cb0(cx, |this, _win, cx| {
                        this.close_terminal_dock(cx);
                    }),
                    on_scroll_wheel: Self::cb2(cx, |this, delta_y, shift, _win, cx| {
                        if shift {
                            this.terminal_scroll_left =
                                (this.terminal_scroll_left - (delta_y as f32) * 24.0).max(0.0);
                        } else if let Some(t) = this.current_terminal_mut() {
                            if delta_y < 0 {
                                t.scroll_up(3);
                            } else if delta_y > 0 {
                                t.scroll_down(3);
                            }
                        }
                        cx.notify();
                    }),
                },
            ))
        } else {
            None
        };

        let status_bar = StatusBar::render(
            self.current_buffer(),
            &self.theme,
            &self.settings,
            self.show_sidebar,
            self.show_terminal,
            self.git_branch.as_deref(),
            self.status_message
                .as_ref()
                .map(|(msg, is_err)| (msg.as_str(), *is_err)),
            StatusBarCallbacks {
                on_toggle_terminal: Self::cb0(cx, |this, _win, cx| {
                    this.toggle_terminal(cx);
                }),
                on_open_find: Self::cb0(cx, |this, _win, cx| {
                    this.open_find_bar(cx);
                }),
                on_clear_status_message: Self::cb0(cx, |this, _win, cx| {
                    this.clear_status_message();
                    cx.notify();
                }),
                on_cycle_tab_size: Self::cb0(cx, |this, _win, cx| {
                    this.settings.tab_size = match this.settings.tab_size {
                        2 => 4,
                        4 => 8,
                        _ => 2,
                    };
                    this.save_config();
                    cx.notify();
                }),
                on_toggle_settings: Self::cb0(cx, |this, _win, cx| {
                    this.toggle_settings_modal(cx);
                }),
            },
        )
        .into_any_element();

        let modal_overlay = if self.file_finder.is_open {
            Some(self.file_finder.render(
                &self.theme,
                &self.settings.font_family,
                FileFinderCallbacks {
                    on_select_file: Self::cb1(cx, |this, path: PathBuf, _win, cx| {
                        this.open_file_path(&path, cx);
                        this.file_finder.close();
                        cx.notify();
                    }),
                    on_scroll: Self::cb1(cx, |this, delta_y: i32, _win, cx| {
                        if delta_y < 0 {
                            this.file_finder
                                .scroll_down(delta_y.unsigned_abs() as usize);
                        } else if delta_y > 0 {
                            this.file_finder.scroll_up(delta_y as usize);
                        }
                        cx.notify();
                    }),
                    on_close: Self::cb0(cx, |this, _win, cx| {
                        this.file_finder.close();
                        cx.notify();
                    }),
                },
            ))
        } else if self.show_settings {
            Some(SettingsModal::render(
                &self.settings,
                &self.theme,
                &self.system_fonts,
                self.open_font_dropdown,
                &self.font_dropdown_query,
                self.font_dropdown_scroll,
                SettingsModalCallbacks {
                    on_close: Self::cb0(cx, |this, _win, cx| {
                        this.show_settings = false;
                        this.open_font_dropdown = None;
                        this.font_dropdown_query.clear();
                        cx.notify();
                    }),
                    on_dismiss_dropdown: Self::cb0(cx, |this, _win, cx| {
                        if this.open_font_dropdown.is_some() {
                            this.open_font_dropdown = None;
                            this.font_dropdown_query.clear();
                            cx.notify();
                        }
                    }),
                    on_backdrop_click: Self::cb0(cx, |this, _win, cx| {
                        if this.open_font_dropdown.is_some() {
                            this.open_font_dropdown = None;
                            this.font_dropdown_query.clear();
                        } else {
                            this.show_settings = false;
                        }
                        cx.notify();
                    }),
                    on_select_theme: Self::cb1(cx, |this, mode, _win, cx| {
                        this.set_theme_mode(mode, cx);
                    }),
                    on_select_shell: Self::cb1(cx, |this, shell: String, _win, cx| {
                        this.settings.terminal_shell = shell;
                        this.restart_terminal();
                        this.save_config();
                        cx.notify();
                    }),
                    on_toggle_font_dropdown: Self::cb1(
                        cx,
                        |this, target: FontDropdownTarget, _win, cx| {
                            this.ensure_system_fonts_loaded(cx);
                            if this.open_font_dropdown == Some(target) {
                                this.open_font_dropdown = None;
                            } else {
                                this.open_font_dropdown = Some(target);
                            }
                            this.font_dropdown_query.clear();
                            this.font_dropdown_scroll = 0;
                            cx.notify();
                        },
                    ),
                    on_select_font: Self::cb2(
                        cx,
                        |this, target: FontDropdownTarget, chosen: String, _win, cx| {
                            match target {
                                FontDropdownTarget::Terminal => {
                                    this.settings.terminal_font_family = chosen;
                                }
                                FontDropdownTarget::Editor => {
                                    this.settings.font_family = chosen;
                                }
                            }
                            this.open_font_dropdown = None;
                            this.font_dropdown_query.clear();
                            this.font_dropdown_scroll = 0;
                            this.save_config();
                            cx.notify();
                        },
                    ),
                    on_scroll_font_dropdown: Self::cb2(
                        cx,
                        |this, delta_y: i32, max_scroll: usize, _win, cx| {
                            if delta_y < 0 {
                                this.font_dropdown_scroll =
                                    (this.font_dropdown_scroll + 2).min(max_scroll);
                            } else if delta_y > 0 {
                                this.font_dropdown_scroll =
                                    this.font_dropdown_scroll.saturating_sub(2);
                            }
                            cx.notify();
                        },
                    ),
                    on_select_terminal_font_size: Self::cb1(cx, |this, size, _win, cx| {
                        this.settings.terminal_font_size = size;
                        this.save_config();
                        cx.notify();
                    }),
                    on_select_editor_font_size: Self::cb1(cx, |this, size, _win, cx| {
                        this.settings.font_size = size;
                        this.save_config();
                        cx.notify();
                    }),
                    on_select_tab_size: Self::cb1(cx, |this, tab_size, _win, cx| {
                        this.settings.tab_size = tab_size;
                        this.save_config();
                        cx.notify();
                    }),
                    on_toggle_line_numbers: Self::cb0(cx, |this, _win, cx| {
                        this.settings.show_line_numbers = !this.settings.show_line_numbers;
                        this.save_config();
                        cx.notify();
                    }),
                },
            ))
        } else {
            None
        };

        let confirm_overlay = self.pending_close.as_ref().map(|pending| {
            ConfirmModal::render(
                pending,
                &self.theme,
                cx.listener(|this, _, _win, cx| {
                    this.confirm_save(cx);
                }),
                cx.listener(|this, _, _win, cx| {
                    this.confirm_discard(cx);
                }),
                cx.listener(|this, _, _win, cx| {
                    this.confirm_cancel(cx);
                }),
            )
        });

        let mut root = div()
            .key_context("Editor")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_mouse_move(cx.listener(Self::handle_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::handle_mouse_up))
            .size_full()
            .bg(self.theme.bg_app)
            .font_family(".SystemUIFont")
            .text_size(px(self.settings.font_size))
            .flex()
            .flex_row();

        if self.resizing_sidebar {
            root = root.cursor_col_resize();
        } else if self.resizing_terminal {
            root = root.cursor_row_resize();
        }

        root.children(sidebar_element)
            .children(sidebar_resizer)
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    .child(right_title_bar)
                    .child(tab_bar)
                    .child(breadcrumb_bar)
                    .child(editor_pane)
                    .children(terminal_resizer)
                    .children(bottom_terminal)
                    .child(status_bar),
            )
            .children(modal_overlay)
            .children(confirm_overlay)
    }
}
