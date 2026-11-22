use crate::config::settings::EditorSettings;
use crate::ui::theme::{Theme, ThemeMode};
use crate::ui::{UiAction, UiAction1, UiAction2};
use gpui::*;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontDropdownTarget {
    Editor,
    Terminal,
}

pub struct SettingsModalCallbacks {
    pub on_close: UiAction,
    pub on_dismiss_dropdown: UiAction,
    pub on_backdrop_click: UiAction,
    pub on_select_theme: UiAction1<ThemeMode>,
    pub on_select_shell: UiAction1<String>,
    pub on_toggle_font_dropdown: UiAction1<FontDropdownTarget>,
    pub on_select_font: UiAction2<FontDropdownTarget, String>,
    pub on_scroll_font_dropdown: UiAction2<i32, usize>,
    pub on_select_terminal_font_size: UiAction1<f32>,
    pub on_select_editor_font_size: UiAction1<f32>,
    pub on_select_tab_size: UiAction1<usize>,
    pub on_toggle_line_numbers: UiAction,
}

pub struct SettingsModal;

impl SettingsModal {
    fn render_option_pill(
        label: impl Into<SharedString>,
        font_family: Option<&'static str>,
        is_selected: bool,
        bold_when_selected: bool,
        compact: bool,
        theme: &Theme,
        listener: impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static,
    ) -> AnyElement {
        let bg = if is_selected {
            theme.accent
        } else {
            theme.bg_hover
        };
        let fg = if is_selected {
            theme.text_on_accent
        } else {
            theme.text_secondary
        };

        let mut btn = div()
            .py_1()
            .rounded_sm()
            .cursor_pointer()
            .bg(bg)
            .text_color(fg)
            .hover(|s| s.opacity(0.85))
            .text_xs()
            .child(label.into())
            .on_mouse_down(MouseButton::Left, listener);

        btn = if compact { btn.px_2p5() } else { btn.px_3() };

        if let Some(font) = font_family {
            btn = btn.font_family(font);
        }
        if bold_when_selected && is_selected {
            btn = btn.font_weight(FontWeight::BOLD);
        }

        btn.into_any_element()
    }

    fn render_field(label: &'static str, items: Vec<AnyElement>, theme: &Theme) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.text_secondary)
                    .child(label),
            )
            .child(div().flex().flex_wrap().gap_1p5().children(items))
            .into_any_element()
    }

    fn render_font_dropdown_trigger(
        label: &'static str,
        current_font: &str,
        target: FontDropdownTarget,
        is_open: bool,
        total_fonts: usize,
        theme: &Theme,
        on_toggle: UiAction1<FontDropdownTarget>,
    ) -> AnyElement {
        let font_preview = current_font.to_string();
        let font_label = current_font.to_string();

        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text_secondary)
                            .child(label),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(format!("{} system fonts", total_fonts)),
                    ),
            )
            .child(
                div()
                    .h(px(30.0))
                    .w_full()
                    .px_3()
                    .rounded_sm()
                    .bg(theme.bg_editor)
                    .border_1()
                    .border_color(if is_open { theme.accent } else { theme.border })
                    .cursor_pointer()
                    .hover(|s| s.border_color(theme.accent))
                    .flex()
                    .items_center()
                    .justify_between()
                    .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                        cx.stop_propagation();
                        on_toggle(target, win, cx);
                    })
                    .child(
                        div()
                            .text_xs()
                            .font_family(font_preview)
                            .text_color(theme.text_primary)
                            .child(font_label),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(if is_open {
                                theme.accent
                            } else {
                                theme.text_muted
                            })
                            .child(if is_open { "▴" } else { "▾" }),
                    ),
            )
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn render_font_dropdown_menu(
        target: FontDropdownTarget,
        current_font: &str,
        system_fonts: &[String],
        query: &str,
        scroll_offset: usize,
        theme: &Theme,
        on_select: UiAction2<FontDropdownTarget, String>,
        on_scroll: UiAction2<i32, usize>,
    ) -> AnyElement {
        let q = query.trim().to_lowercase();
        let filtered: Vec<&String> = if q.is_empty() {
            system_fonts.iter().collect()
        } else {
            system_fonts
                .iter()
                .filter(|name| name.to_lowercase().contains(&q))
                .collect()
        };

        let visible_rows = 8usize;
        let max_scroll = filtered.len().saturating_sub(visible_rows);
        let scroll = scroll_offset.min(max_scroll);
        let end = (scroll + visible_rows).min(filtered.len());

        let mut rows: Vec<AnyElement> = Vec::new();
        if filtered.is_empty() {
            rows.push(
                div()
                    .h(px(36.0))
                    .px_3()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_xs()
                    .text_color(theme.text_muted)
                    .child("No matching fonts found")
                    .into_any_element(),
            );
        } else {
            for &font_name in &filtered[scroll..end] {
                let is_selected = font_name == current_font;
                let chosen = font_name.clone();
                let preview_font = font_name.clone();
                let label = font_name.clone();
                let on_select = Rc::clone(&on_select);

                rows.push(
                    div()
                        .h(px(26.0))
                        .px_3()
                        .flex()
                        .items_center()
                        .justify_between()
                        .cursor_pointer()
                        .bg(if is_selected {
                            theme.selection
                        } else {
                            theme.bg_modal
                        })
                        .hover(|s| s.bg(theme.bg_hover))
                        .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                            cx.stop_propagation();
                            on_select(target, chosen.clone(), win, cx);
                        })
                        .child(
                            div()
                                .text_xs()
                                .font_family(preview_font)
                                .text_color(if is_selected {
                                    theme.accent
                                } else {
                                    theme.text_primary
                                })
                                .font_weight(if is_selected {
                                    FontWeight::MEDIUM
                                } else {
                                    FontWeight::NORMAL
                                })
                                .child(label),
                        )
                        .children(if is_selected {
                            Some(div().text_xs().text_color(theme.accent).child("✓"))
                        } else {
                            None
                        })
                        .into_any_element(),
                );
            }
        }

        let search_bar = div()
            .h(px(30.0))
            .px_3()
            .bg(theme.bg_surface)
            .border_b_1()
            .border_color(theme.border_subtle)
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_xs().text_color(theme.accent).child("⌕"))
                    .child(
                        div()
                            .text_xs()
                            .text_color(if query.is_empty() {
                                theme.text_muted
                            } else {
                                theme.text_primary
                            })
                            .child(if query.is_empty() {
                                "Type to filter system fonts...".to_string()
                            } else {
                                query.to_string()
                            }),
                    ),
            )
            .child(div().text_xs().text_color(theme.text_muted).child(format!(
                "{}/{}",
                filtered.len(),
                system_fonts.len()
            )))
            .into_any_element();

        let mut panel = div()
            .absolute()
            .left(px(16.0))
            .right(px(16.0))
            .bg(theme.bg_modal)
            .border_1()
            .border_color(theme.accent)
            .rounded_md()
            .shadow_lg()
            .overflow_hidden()
            .occlude()
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_scroll_wheel(move |event: &ScrollWheelEvent, win, cx| {
                let delta_y = match event.delta {
                    ScrollDelta::Pixels(p) => (p.y / px(20.0)) as i32,
                    ScrollDelta::Lines(l) => l.y as i32,
                };
                if delta_y != 0 {
                    on_scroll(delta_y, max_scroll, win, cx);
                }
                cx.stop_propagation();
            })
            .child(search_bar)
            .child(div().flex().flex_col().py_1().children(rows));

        panel = match target {
            FontDropdownTarget::Terminal => panel.top(px(252.0)),
            FontDropdownTarget::Editor => panel.bottom(px(124.0)),
        };

        panel.into_any_element()
    }

    fn render_header(theme: &Theme, on_close: UiAction) -> AnyElement {
        div()
            .h(px(40.0))
            .px_4()
            .bg(theme.bg_surface)
            .rounded_t_md()
            .border_b_1()
            .border_color(theme.border_subtle)
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme.text_primary)
                            .child("Preferences"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child("— Settings & Terminal Configuration"),
                    ),
            )
            .child(
                div()
                    .w_5()
                    .h_5()
                    .rounded_xs()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(|s| s.bg(theme.bg_hover).text_color(theme.error))
                    .text_color(theme.text_muted)
                    .text_xs()
                    .child("✕")
                    .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                        cx.stop_propagation();
                        on_close(win, cx);
                    }),
            )
            .into_any_element()
    }

    fn render_theme_section(theme: &Theme, on_select_theme: UiAction1<ThemeMode>) -> AnyElement {
        let current_theme = theme.mode;
        let theme_buttons: Vec<AnyElement> = ThemeMode::all()
            .iter()
            .map(|&mode| {
                let on_select = Rc::clone(&on_select_theme);
                Self::render_option_pill(
                    mode.name(),
                    None,
                    mode == current_theme,
                    true,
                    false,
                    theme,
                    move |_, win, cx| {
                        cx.stop_propagation();
                        on_select(mode, win, cx);
                    },
                )
            })
            .collect();

        Self::render_field("Color Theme", theme_buttons, theme)
    }

    fn render_terminal_section(
        settings: &EditorSettings,
        theme: &Theme,
        total_fonts: usize,
        is_font_dropdown_open: bool,
        callbacks: &SettingsModalCallbacks,
    ) -> AnyElement {
        let current_shell = &settings.terminal_shell;
        let current_term_size = settings.terminal_font_size;

        let shell_buttons: Vec<AnyElement> = EditorSettings::AVAILABLE_SHELLS
            .iter()
            .map(|&(shell_title, shell_bin)| {
                let bin_name = shell_bin.to_string();
                let on_shell = Rc::clone(&callbacks.on_select_shell);
                Self::render_option_pill(
                    shell_title,
                    None,
                    shell_bin == current_shell,
                    true,
                    false,
                    theme,
                    move |_, win, cx| {
                        cx.stop_propagation();
                        on_shell(bin_name.clone(), win, cx);
                    },
                )
            })
            .collect();

        let term_size_buttons: Vec<AnyElement> = EditorSettings::AVAILABLE_FONT_SIZES
            .iter()
            .map(|&size| {
                let on_size = Rc::clone(&callbacks.on_select_terminal_font_size);
                Self::render_option_pill(
                    format!("{:.0}", size),
                    None,
                    (size - current_term_size).abs() < f32::EPSILON,
                    false,
                    true,
                    theme,
                    move |_, win, cx| {
                        cx.stop_propagation();
                        on_size(size, win, cx);
                    },
                )
            })
            .collect();

        div()
            .pt_2()
            .border_t_1()
            .border_color(theme.border_subtle)
            .flex()
            .flex_col()
            .gap_2p5()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.accent)
                    .child("TERMINAL SETTINGS"),
            )
            .child(Self::render_field(
                "Terminal Shell Program",
                shell_buttons,
                theme,
            ))
            .child(Self::render_font_dropdown_trigger(
                "Terminal Font Family",
                &settings.terminal_font_family,
                FontDropdownTarget::Terminal,
                is_font_dropdown_open,
                total_fonts,
                theme,
                Rc::clone(&callbacks.on_toggle_font_dropdown),
            ))
            .child(Self::render_field(
                "Terminal Font Size",
                term_size_buttons,
                theme,
            ))
            .into_any_element()
    }

    fn render_editor_section(
        settings: &EditorSettings,
        theme: &Theme,
        total_fonts: usize,
        is_font_dropdown_open: bool,
        callbacks: &SettingsModalCallbacks,
    ) -> AnyElement {
        let current_size = settings.font_size;

        let size_buttons: Vec<AnyElement> = EditorSettings::AVAILABLE_FONT_SIZES
            .iter()
            .map(|&size| {
                let on_size = Rc::clone(&callbacks.on_select_editor_font_size);
                Self::render_option_pill(
                    format!("{:.0}", size),
                    None,
                    (size - current_size).abs() < f32::EPSILON,
                    false,
                    true,
                    theme,
                    move |_, win, cx| {
                        cx.stop_propagation();
                        on_size(size, win, cx);
                    },
                )
            })
            .collect();

        div()
            .pt_2()
            .border_t_1()
            .border_color(theme.border_subtle)
            .flex()
            .flex_col()
            .gap_2p5()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::BOLD)
                    .text_color(theme.text_secondary)
                    .child("EDITOR SETTINGS"),
            )
            .child(Self::render_font_dropdown_trigger(
                "Editor Font Family",
                &settings.font_family,
                FontDropdownTarget::Editor,
                is_font_dropdown_open,
                total_fonts,
                theme,
                Rc::clone(&callbacks.on_toggle_font_dropdown),
            ))
            .child(Self::render_field("Editor Font Size", size_buttons, theme))
            .into_any_element()
    }

    fn render_misc_section(
        settings: &EditorSettings,
        theme: &Theme,
        callbacks: &SettingsModalCallbacks,
    ) -> AnyElement {
        let current_tab = settings.tab_size;
        let show_line_num = settings.show_line_numbers;

        let tab_buttons: Vec<AnyElement> = EditorSettings::AVAILABLE_TAB_SIZES
            .iter()
            .map(|&tab_size| {
                let on_tab = Rc::clone(&callbacks.on_select_tab_size);
                Self::render_option_pill(
                    format!("{} spaces", tab_size),
                    None,
                    tab_size == current_tab,
                    false,
                    false,
                    theme,
                    move |_, win, cx| {
                        cx.stop_propagation();
                        on_tab(tab_size, win, cx);
                    },
                )
            })
            .collect();

        let on_line_num = Rc::clone(&callbacks.on_toggle_line_numbers);
        let line_num_toggle = Self::render_option_pill(
            if show_line_num { "Enabled" } else { "Disabled" },
            None,
            show_line_num,
            false,
            false,
            theme,
            move |_, win, cx| {
                cx.stop_propagation();
                on_line_num(win, cx);
            },
        );

        div()
            .pt_2()
            .border_t_1()
            .border_color(theme.border_subtle)
            .flex()
            .items_center()
            .justify_between()
            .child(Self::render_field("Tab Indent Size", tab_buttons, theme))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .items_end()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text_secondary)
                            .child("Line Numbers"),
                    )
                    .child(line_num_toggle),
            )
            .into_any_element()
    }

    pub fn render(
        settings: &EditorSettings,
        theme: &Theme,
        system_fonts: &[String],
        open_dropdown: Option<FontDropdownTarget>,
        dropdown_query: &str,
        dropdown_scroll: usize,
        callbacks: SettingsModalCallbacks,
    ) -> AnyElement {
        let total_fonts = system_fonts.len();

        let dropdown_overlay = open_dropdown.map(|target| {
            let cur_font = match target {
                FontDropdownTarget::Terminal => settings.terminal_font_family.as_str(),
                FontDropdownTarget::Editor => settings.font_family.as_str(),
            };
            Self::render_font_dropdown_menu(
                target,
                cur_font,
                system_fonts,
                dropdown_query,
                dropdown_scroll,
                theme,
                Rc::clone(&callbacks.on_select_font),
                Rc::clone(&callbacks.on_scroll_font_dropdown),
            )
        });

        let on_dismiss = Rc::clone(&callbacks.on_dismiss_dropdown);
        let on_backdrop = Rc::clone(&callbacks.on_backdrop_click);
        let on_close = Rc::clone(&callbacks.on_close);
        let on_theme = Rc::clone(&callbacks.on_select_theme);

        let modal_card = div()
            .relative()
            .w(px(620.0))
            .rounded_md()
            .bg(theme.bg_modal)
            .border_1()
            .border_color(theme.border)
            .shadow_lg()
            .occlude()
            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                cx.stop_propagation();
                on_dismiss(win, cx);
            })
            .child(Self::render_header(theme, on_close))
            .child(
                div()
                    .p_4()
                    .rounded_b_md()
                    .flex()
                    .flex_col()
                    .gap_3p5()
                    .child(Self::render_theme_section(theme, on_theme))
                    .child(Self::render_terminal_section(
                        settings,
                        theme,
                        total_fonts,
                        open_dropdown == Some(FontDropdownTarget::Terminal),
                        &callbacks,
                    ))
                    .child(Self::render_editor_section(
                        settings,
                        theme,
                        total_fonts,
                        open_dropdown == Some(FontDropdownTarget::Editor),
                        &callbacks,
                    ))
                    .child(Self::render_misc_section(settings, theme, &callbacks)),
            )
            .children(dropdown_overlay)
            .into_any_element();

        div()
            .absolute()
            .inset_0()
            .flex()
            .flex_col()
            .items_center()
            .pt(px(40.0))
            .bg(if theme.is_light() {
                hsla(0.0, 0.0, 0.0, 0.30)
            } else {
                hsla(0.0, 0.0, 0.0, 0.65)
            })
            .occlude()
            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                cx.stop_propagation();
                on_backdrop(win, cx);
            })
            .child(modal_card)
            .into_any_element()
    }
}
