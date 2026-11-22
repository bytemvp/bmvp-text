use crate::config::settings::EditorSettings;
use crate::terminal::ansi::parse_ansi_spans;
use crate::terminal::session::SystemTerminal;
use crate::ui::theme::Theme;
use crate::ui::{UiAction, UiAction1, UiAction2};
use gpui::*;
use std::rc::Rc;

pub struct TerminalDockCallbacks {
    pub on_focus: UiAction,
    pub on_select_tab: UiAction1<usize>,
    pub on_close_tab: UiAction1<usize>,
    pub on_new_tab: UiAction,
    pub on_interrupt: UiAction,
    pub on_scroll_to_bottom: UiAction,
    pub on_restart: UiAction,
    pub on_open_external: UiAction,
    pub on_clear: UiAction,
    pub on_close_dock: UiAction,
    pub on_scroll_wheel: UiAction2<i32, bool>,
}

pub struct TerminalDock;

impl TerminalDock {
    fn render_ansi_spans(
        spans: &[crate::terminal::ansi::AnsiSpan],
        default_fg: Hsla,
        default_bg: Hsla,
    ) -> Vec<AnyElement> {
        spans
            .iter()
            .map(|sp| {
                let mut fg: Hsla = sp.fg_rgb.map(|c| rgb(c).into()).unwrap_or(default_fg);
                let mut bg: Option<Hsla> = sp.bg_rgb.map(|c| rgb(c).into());
                if sp.inverse {
                    let prev_fg = fg;
                    fg = bg.unwrap_or(default_bg);
                    bg = Some(prev_fg);
                }
                let mut el = div().text_color(fg).child(sp.text.clone());
                if let Some(bg_col) = bg {
                    el = el.bg(bg_col);
                }
                if sp.bold {
                    el = el.font_weight(FontWeight::BOLD);
                }
                if sp.italic {
                    el = el.italic();
                }
                if sp.underline {
                    el = el.underline();
                }
                el.into_any_element()
            })
            .collect()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        terminals: &[SystemTerminal],
        active_terminal: usize,
        terminal_height: f32,
        terminal_scroll_left: f32,
        is_focused: bool,
        settings: &EditorSettings,
        theme: &Theme,
        callbacks: TerminalDockCallbacks,
    ) -> AnyElement {
        let active_idx = active_terminal.min(terminals.len().saturating_sub(1));
        let term = &terminals[active_idx];
        let term_font = settings.terminal_font_family.clone();
        let term_size = px(settings.terminal_font_size);
        let line_h = (settings.terminal_font_size * 1.45).ceil().max(20.0);
        let is_light = theme.is_light();

        let prompt_lines = term.active_prompt_lines();
        let input_rows = if term.scroll_offset == 0 {
            if term.is_running {
                1
            } else {
                prompt_lines.len().max(1)
            }
        } else {
            0
        };

        let avail_h = (terminal_height - 28.0 - 20.0).max(line_h);
        let max_rows = (avail_h / line_h).floor().max(1.0) as usize;
        let visible_count = max_rows.saturating_sub(input_rows);

        let lines = term.lines();
        let total = lines.len();
        let effective_end = total.saturating_sub(term.scroll_offset);
        let start = effective_end.saturating_sub(visible_count);
        let range = if visible_count == 0 {
            0..0
        } else if effective_end > 0 && start < total {
            start..effective_end.min(total)
        } else if total > 0 && term.scroll_offset > 0 {
            0..visible_count.min(total)
        } else {
            let start_idx = total.saturating_sub(visible_count);
            start_idx..total
        };

        let mut output_divs: Vec<AnyElement> = range
            .map(|line_idx| {
                let spans = term.spans_for_line(line_idx, is_light);
                let span_els =
                    Self::render_ansi_spans(&spans, theme.text_primary, theme.bg_terminal);
                let mut row = div()
                    .w_full()
                    .h(px(line_h))
                    .ml(px(-terminal_scroll_left))
                    .whitespace_nowrap()
                    .flex()
                    .items_center()
                    .font_family(term_font.clone())
                    .text_size(term_size);

                if span_els.is_empty() {
                    row = row.text_color(theme.text_primary).child(" ");
                } else {
                    row = row.children(span_els);
                }
                row.into_any_element()
            })
            .collect();

        if term.scroll_offset == 0 {
            let chars: Vec<char> = term.input_buffer.chars().collect();
            let c_idx = term.cursor_pos.min(chars.len());
            let before_text: String = chars[..c_idx].iter().collect();
            let after_text: String = chars[c_idx..].iter().collect();
            let cursor_box = div()
                .w(px(7.0))
                .h(px((settings.terminal_font_size + 1.0).max(13.0)))
                .bg(if is_focused {
                    theme.cursor_color
                } else {
                    theme.text_muted
                });

            if term.is_running {
                output_divs.push(
                    div()
                        .w_full()
                        .h(px(line_h))
                        .ml(px(-terminal_scroll_left))
                        .whitespace_nowrap()
                        .flex()
                        .items_center()
                        .font_family(term_font.clone())
                        .text_size(term_size)
                        .child(div().text_color(theme.text_primary).child(before_text))
                        .child(cursor_box)
                        .child(div().text_color(theme.text_primary).child(after_text))
                        .into_any_element(),
                );
            } else {
                let last_idx = prompt_lines.len().saturating_sub(1);
                for p_line in &prompt_lines[..last_idx] {
                    let parsed = parse_ansi_spans(p_line, is_light);
                    let span_els =
                        Self::render_ansi_spans(&parsed, theme.text_secondary, theme.bg_terminal);
                    output_divs.push(
                        div()
                            .w_full()
                            .h(px(line_h))
                            .ml(px(-terminal_scroll_left))
                            .whitespace_nowrap()
                            .flex()
                            .items_center()
                            .font_family(term_font.clone())
                            .text_size(term_size)
                            .children(span_els)
                            .into_any_element(),
                    );
                }

                let last_prompt = prompt_lines.last().map(String::as_str).unwrap_or("");
                let parsed_last = parse_ansi_spans(last_prompt, is_light);
                let prompt_els =
                    Self::render_ansi_spans(&parsed_last, theme.text_secondary, theme.bg_terminal);
                output_divs.push(
                    div()
                        .w_full()
                        .h(px(line_h))
                        .ml(px(-terminal_scroll_left))
                        .whitespace_nowrap()
                        .flex()
                        .items_center()
                        .font_family(term_font.clone())
                        .text_size(term_size)
                        .children(prompt_els)
                        .child(div().text_color(theme.text_primary).child(before_text))
                        .child(cursor_box)
                        .child(div().text_color(theme.text_primary).child(after_text))
                        .into_any_element(),
                );
            }
        }

        let mut tab_divs: Vec<AnyElement> = Vec::new();
        for (idx, t) in terminals.iter().enumerate() {
            let is_cur = idx == active_idx;
            let status_icon = if t.is_running { "● " } else { ">_ " };
            let status_color = if t.is_running {
                theme.dirty_indicator
            } else if is_cur {
                theme.text_primary
            } else {
                theme.text_muted
            };

            let on_select = Rc::clone(&callbacks.on_select_tab);
            let on_close = Rc::clone(&callbacks.on_close_tab);

            let mut term_tab = div()
                .h_full()
                .px_3()
                .flex()
                .items_center()
                .gap_1p5()
                .cursor_pointer()
                .bg(if is_cur {
                    theme.bg_terminal
                } else {
                    theme.bg_tab_bar
                })
                .hover(|s| {
                    s.bg(if is_cur {
                        theme.bg_terminal
                    } else {
                        theme.bg_hover
                    })
                })
                .text_xs()
                .text_color(if is_cur {
                    theme.text_primary
                } else {
                    theme.text_secondary
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .child(div().text_color(status_color).child(status_icon))
                        .child(t.title.clone()),
                )
                .child(
                    div()
                        .w(px(14.0))
                        .h(px(14.0))
                        .rounded_xs()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(theme.text_muted)
                        .hover(|s| s.bg(theme.bg_hover).text_color(theme.error))
                        .child("×")
                        .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                            cx.stop_propagation();
                            on_close(idx, win, cx);
                        }),
                )
                .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                    cx.stop_propagation();
                    on_select(idx, win, cx);
                });

            if is_cur {
                term_tab = term_tab
                    .border_t_1()
                    .border_l_1()
                    .border_r_1()
                    .border_color(theme.border_subtle);
            } else {
                term_tab = term_tab
                    .border_b_1()
                    .border_r_1()
                    .border_color(theme.border_subtle);
            }

            tab_divs.push(term_tab.into_any_element());
        }

        let dir_name = term
            .current_dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| term.current_dir.display().to_string());

        let on_new_tab = callbacks.on_new_tab;
        let on_interrupt = callbacks.on_interrupt;
        let on_latest = callbacks.on_scroll_to_bottom;
        let on_restart = callbacks.on_restart;
        let on_open_external = callbacks.on_open_external;
        let on_clear = callbacks.on_clear;
        let on_close_dock = callbacks.on_close_dock;
        let on_focus = callbacks.on_focus;
        let on_scroll_wheel = callbacks.on_scroll_wheel;

        let header = div()
            .h(px(28.0))
            .w_full()
            .bg(theme.bg_tab_bar)
            .flex()
            .items_center()
            .child(div().h_full().flex().items_center().children(tab_divs))
            .child(
                div()
                    .h_full()
                    .px_2()
                    .border_b_1()
                    .border_color(theme.border_subtle)
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .text_xs()
                    .text_color(theme.text_secondary)
                    .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                    .child("+")
                    .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                        cx.stop_propagation();
                        on_new_tab(win, cx);
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .border_b_1()
                    .border_color(theme.border_subtle),
            )
            .child(
                div()
                    .h_full()
                    .px_2()
                    .border_b_1()
                    .border_color(theme.border_subtle)
                    .flex()
                    .items_center()
                    .gap_1p5()
                    .child(if term.is_running {
                        div()
                            .px_1p5()
                            .py_0p5()
                            .rounded_xs()
                            .bg(theme.bg_hover)
                            .text_xs()
                            .text_color(theme.dirty_indicator)
                            .child("● running")
                    } else {
                        div()
                            .px_1p5()
                            .py_0p5()
                            .rounded_xs()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .child(format!("~/{}", dir_name))
                    })
                    .child(if term.is_running {
                        div()
                            .h(px(20.0))
                            .px_2()
                            .rounded_xs()
                            .bg(theme.error)
                            .text_color(rgb(0xffffff))
                            .text_xs()
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .child("■ Stop")
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_interrupt(win, cx);
                            })
                    } else {
                        div()
                    })
                    .child(if term.scroll_offset > 0 {
                        div()
                            .h(px(20.0))
                            .px_2()
                            .rounded_xs()
                            .bg(theme.accent)
                            .text_color(theme.text_on_accent)
                            .text_xs()
                            .flex()
                            .items_center()
                            .cursor_pointer()
                            .child("↓ Latest")
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_latest(win, cx);
                            })
                    } else {
                        div()
                    })
                    .child(
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.text_secondary)
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .child("⟳")
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_restart(win, cx);
                            }),
                    )
                    .child(
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.text_secondary)
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .child("↗")
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_open_external(win, cx);
                            }),
                    )
                    .child(
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.text_secondary)
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.text_primary))
                            .child("⌫")
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_clear(win, cx);
                            }),
                    )
                    .child(
                        div()
                            .w(px(20.0))
                            .h(px(20.0))
                            .rounded_xs()
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_xs()
                            .text_color(theme.text_secondary)
                            .hover(|s| s.bg(theme.bg_hover).text_color(theme.error))
                            .child("✕")
                            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                                cx.stop_propagation();
                                on_close_dock(win, cx);
                            }),
                    ),
            )
            .into_any_element();

        div()
            .h(px(terminal_height))
            .w_full()
            .bg(theme.bg_terminal)
            .flex()
            .flex_col()
            .on_mouse_down(MouseButton::Left, move |_, win, cx| {
                cx.stop_propagation();
                on_focus(win, cx);
            })
            .on_scroll_wheel(move |event: &ScrollWheelEvent, win, cx| {
                let delta_y = match event.delta {
                    ScrollDelta::Pixels(p) => (p.y / px(20.0)) as i32,
                    ScrollDelta::Lines(l) => l.y as i32,
                };
                if delta_y != 0 {
                    on_scroll_wheel(delta_y, event.modifiers.shift, win, cx);
                }
                cx.stop_propagation();
            })
            .child(header)
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .p_2p5()
                    .bg(theme.bg_terminal)
                    .children(output_divs),
            )
            .into_any_element()
    }
}
