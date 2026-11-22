use crate::core::syntax::{HighlightCache, SyntaxToken, token_color};
use crate::ui::editor_view::EditorView;
use crate::ui::theme::Theme;
use gpui::*;
use std::ops::Range;

pub struct ShapedEditorLine {
    pub row: usize,
    pub line_start_char: usize,
    pub display_text: String,
    pub origin: Point<Pixels>,
    pub shaped: ShapedLine,
}

#[derive(Default)]
pub struct CodeLayoutState {
    pub text_bounds: Option<Bounds<Pixels>>,
    pub line_height: Pixels,
    pub viewport_lines: usize,
    pub lines: Vec<ShapedEditorLine>,
}

impl CodeLayoutState {
    /// 将窗口坐标映射为 `(row, char_idx)`
    pub fn hit_test(&self, pos: Point<Pixels>, total_chars: usize) -> Option<(usize, usize)> {
        let bounds = self.text_bounds?;
        if self.lines.is_empty() {
            return Some((0, 0));
        }
        let lh = f32::from(self.line_height).max(1.0);
        let rel_y = f32::from(pos.y) - f32::from(bounds.top());
        let line_idx = if rel_y <= 0.0 {
            0
        } else {
            ((rel_y / lh).floor() as usize).min(self.lines.len().saturating_sub(1))
        };
        let line = &self.lines[line_idx];
        let local_x = pos.x - line.origin.x;
        let byte_idx = if f32::from(local_x) <= 0.0 {
            0
        } else {
            line.shaped.closest_index_for_x(local_x)
        };
        let col_chars = byte_to_char_offset(&line.display_text, byte_idx);
        let char_idx = (line.line_start_char + col_chars).min(total_chars);
        Some((line.row, char_idx))
    }

    /// 返回指定行列（字符列）在窗口坐标系下的光标矩形，供 IME 候选窗定位
    pub fn caret_bounds_for_pos(&self, row: usize, col_chars: usize) -> Option<Bounds<Pixels>> {
        let line = self.lines.iter().find(|l| l.row == row)?;
        let byte_idx = char_to_byte_offset(&line.display_text, col_chars);
        let x = line.origin.x + line.shaped.x_for_index(byte_idx);
        let y = line.origin.y;
        Some(Bounds::new(point(x, y), size(px(2.0), self.line_height)))
    }
}

pub fn char_to_byte_offset(s: &str, char_idx: usize) -> usize {
    if char_idx == 0 {
        return 0;
    }
    s.char_indices()
        .nth(char_idx)
        .map(|(byte_idx, _)| byte_idx)
        .unwrap_or(s.len())
}

pub fn byte_to_char_offset(s: &str, byte_idx: usize) -> usize {
    let clamped = byte_idx.min(s.len());
    let mut safe = clamped;
    while safe > 0 && !s.is_char_boundary(safe) {
        safe -= 1;
    }
    s[..safe].chars().count()
}

fn build_text_runs(
    display_text: &str,
    tokens: &[SyntaxToken],
    font: &Font,
    theme: &Theme,
    marked_byte_range: Option<Range<usize>>,
) -> Vec<TextRun> {
    if display_text.is_empty() {
        return vec![TextRun {
            len: 0,
            font: font.clone(),
            color: theme.text_primary,
            background_color: None,
            underline: None,
            strikethrough: None,
        }];
    }

    let mut runs = Vec::with_capacity(tokens.len() + 2);
    let mut byte_cursor = 0usize;

    for tok in tokens {
        let tok_len = tok.text.len();
        if tok_len == 0 {
            continue;
        }
        let tok_start = byte_cursor;
        let tok_end = byte_cursor + tok_len;
        let color = token_color(tok.token_type, theme);

        if let Some(ref mark) = marked_byte_range {
            let ov_start = tok_start.max(mark.start);
            let ov_end = tok_end.min(mark.end);
            if ov_start < ov_end {
                if tok_start < ov_start {
                    runs.push(TextRun {
                        len: ov_start - tok_start,
                        font: font.clone(),
                        color,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    });
                }
                runs.push(TextRun {
                    len: ov_end - ov_start,
                    font: font.clone(),
                    color,
                    background_color: None,
                    underline: Some(UnderlineStyle {
                        color: Some(theme.accent),
                        thickness: px(1.5),
                        wavy: false,
                    }),
                    strikethrough: None,
                });
                if ov_end < tok_end {
                    runs.push(TextRun {
                        len: tok_end - ov_end,
                        font: font.clone(),
                        color,
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    });
                }
            } else {
                runs.push(TextRun {
                    len: tok_len,
                    font: font.clone(),
                    color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                });
            }
        } else {
            runs.push(TextRun {
                len: tok_len,
                font: font.clone(),
                color,
                background_color: None,
                underline: None,
                strikethrough: None,
            });
        }

        byte_cursor = tok_end;
    }

    if runs.is_empty() {
        runs.push(TextRun {
            len: display_text.len(),
            font: font.clone(),
            color: theme.text_primary,
            background_color: None,
            underline: None,
            strikethrough: None,
        });
    }

    runs
}

pub struct CodeCanvas {
    pub editor: Entity<EditorView>,
}

pub struct CodePrepaintState {
    layout: CodeLayoutState,
    quads: Vec<PaintQuad>,
    caret: Option<PaintQuad>,
    focus_handle: FocusHandle,
}

impl IntoElement for CodeCanvas {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for CodeCanvas {
    type RequestLayoutState = ();
    type PrepaintState = CodePrepaintState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.0).into();
        style.size.height = relative(1.0).into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let editor = self.editor.read(cx);
        let theme = &editor.theme;
        let settings = &editor.settings;
        let focus_handle = editor.focus_handle.clone();
        let line_height_f32 = (settings.font_size * settings.line_height_factor).max(14.0);
        let line_height = px(line_height_f32);
        let font_size = px(settings.font_size);
        let text_font = font(settings.font_family.clone());

        let inner_top = bounds.top() + px(4.0);
        let inner_left = bounds.left() + px(8.0);
        let usable_h = (f32::from(bounds.size.height) - 8.0).max(line_height_f32);
        let viewport_lines = (usable_h / line_height_f32).ceil().max(1.0) as usize;
        let text_bounds = Bounds::new(
            point(inner_left, inner_top),
            size((bounds.size.width - px(8.0)).max(px(1.0)), px(usable_h)),
        );

        let Some(buffer) = editor.current_buffer() else {
            return CodePrepaintState {
                layout: CodeLayoutState {
                    text_bounds: Some(text_bounds),
                    line_height,
                    viewport_lines,
                    lines: Vec::new(),
                },
                quads: Vec::new(),
                caret: None,
                focus_handle,
            };
        };

        let total_lines = buffer.len_lines();
        let scroll_top = buffer.scroll_top().min(total_lines.saturating_sub(1));
        let scroll_left = buffer.scroll_left();
        let end_line = (scroll_top + viewport_lines).min(total_lines);
        let lang = buffer.language();
        let cursor_pos = buffer.cursor_position();
        let selection_range = buffer
            .selection()
            .filter(|s| !s.is_empty())
            .map(|s| s.range());
        let marked_range = editor.ime_marked_range.clone();
        let find_matches = if editor.find_state.is_open {
            Some((&editor.find_state.matches, editor.find_state.current_index))
        } else {
            None
        };
        let editor_is_focused = focus_handle.is_focused(window)
            && !editor.terminal_focused
            && !editor.find_focused
            && !editor.file_finder.is_open
            && !editor.show_settings
            && !editor.has_pending_close();

        let mut shaped_lines = Vec::with_capacity(end_line.saturating_sub(scroll_top));
        let mut quads = Vec::new();
        let mut caret = None;
        let mut highlight_cache: std::cell::RefMut<'_, HighlightCache> =
            editor.highlight_cache.borrow_mut();

        let match_bg: Hsla = if theme.is_light() {
            rgba(0xf59e0b38).into()
        } else {
            rgba(0xf59e0b30).into()
        };
        let current_match_bg: Hsla = if theme.is_light() {
            rgba(0xf59e0b70).into()
        } else {
            rgba(0xf59e0b66).into()
        };

        for (offset, row_idx) in (scroll_top..end_line).enumerate() {
            let y = inner_top + line_height * (offset as f32);
            let origin = point(inner_left - px(scroll_left), y);
            let is_cursor_line = row_idx == cursor_pos.row;

            // 1. 当前行高亮背景
            if is_cursor_line {
                quads.push(fill(
                    Bounds::new(
                        point(bounds.left(), y),
                        size(bounds.size.width, line_height),
                    ),
                    theme.bg_active_line,
                ));
            }

            let raw_line = buffer
                .line(row_idx)
                .map(|s| s.to_string())
                .unwrap_or_default();
            let display_text = raw_line.trim_end_matches(&['\r', '\n'][..]).to_string();
            let line_start_char = buffer.line_to_char(row_idx);
            let line_char_len = display_text.chars().count();
            let line_end_char = line_start_char + line_char_len;
            let raw_line_end_char = line_start_char + raw_line.chars().count();

            let marked_byte_range = marked_range.as_ref().and_then(|mr| {
                let ov_start = mr.start.max(line_start_char);
                let ov_end = mr.end.min(line_end_char);
                if ov_start < ov_end {
                    let b_start = char_to_byte_offset(&display_text, ov_start - line_start_char);
                    let b_end = char_to_byte_offset(&display_text, ov_end - line_start_char);
                    Some(b_start..b_end)
                } else {
                    None
                }
            });

            let tokens = highlight_cache.get_or_highlight(&display_text, lang);
            let runs = build_text_runs(&display_text, tokens, &text_font, theme, marked_byte_range);
            let shaped = window.text_system().shape_line(
                display_text.clone().into(),
                font_size,
                &runs,
                None,
            );

            // 2. 查找匹配全量高亮（先于选区绘制）
            if let Some((matches, cur_idx)) = find_matches {
                for (m_idx, mr) in matches.iter().enumerate() {
                    if mr.end <= line_start_char || mr.start >= line_end_char {
                        continue;
                    }
                    let col_s = mr.start.saturating_sub(line_start_char).min(line_char_len);
                    let col_e = mr.end.saturating_sub(line_start_char).min(line_char_len);
                    if col_s < col_e {
                        let x1 = origin.x
                            + shaped.x_for_index(char_to_byte_offset(&display_text, col_s));
                        let x2 = origin.x
                            + shaped.x_for_index(char_to_byte_offset(&display_text, col_e));
                        let bg = if m_idx == cur_idx {
                            current_match_bg
                        } else {
                            match_bg
                        };
                        quads.push(fill(
                            Bounds::new(point(x1, y), size((x2 - x1).max(px(2.0)), line_height)),
                            bg,
                        ));
                    }
                }
            }

            // 3. 文本选区背景（在光标行同样生效）
            if let Some(ref sel) = selection_range
                && sel.start < raw_line_end_char
                && sel.end > line_start_char
            {
                let col_s = sel.start.saturating_sub(line_start_char).min(line_char_len);
                let col_e = sel.end.saturating_sub(line_start_char).min(line_char_len);
                let x1 = origin.x + shaped.x_for_index(char_to_byte_offset(&display_text, col_s));
                let mut x2 =
                    origin.x + shaped.x_for_index(char_to_byte_offset(&display_text, col_e));
                if sel.end > line_end_char {
                    x2 += px(6.0);
                }
                if x2 > x1 {
                    quads.push(fill(
                        Bounds::new(point(x1, y), size(x2 - x1, line_height)),
                        theme.selection,
                    ));
                }
            }

            // 4. 光标（2px 宽竖线覆盖层，不遮挡字符也不破坏语法高亮）
            if is_cursor_line {
                let col = cursor_pos.col.min(line_char_len);
                let cursor_byte = char_to_byte_offset(&display_text, col);
                let cx_pos = origin.x + shaped.x_for_index(cursor_byte);
                let caret_color = if editor_is_focused {
                    theme.cursor_color
                } else {
                    let mut muted = theme.cursor_color;
                    muted.a *= 0.4;
                    muted
                };
                caret = Some(fill(
                    Bounds::new(
                        point(cx_pos, y + px(1.0)),
                        size(px(2.0), (line_height - px(2.0)).max(px(10.0))),
                    ),
                    caret_color,
                ));
            }

            shaped_lines.push(ShapedEditorLine {
                row: row_idx,
                line_start_char,
                display_text,
                origin,
                shaped,
            });
        }

        CodePrepaintState {
            layout: CodeLayoutState {
                text_bounds: Some(text_bounds),
                line_height,
                viewport_lines,
                lines: shaped_lines,
            },
            quads,
            caret,
            focus_handle,
        }
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        window.handle_input(
            &prepaint.focus_handle,
            ElementInputHandler::new(bounds, self.editor.clone()),
            cx,
        );

        let line_height = prepaint.layout.line_height;
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            for quad in prepaint.quads.drain(..) {
                window.paint_quad(quad);
            }
            for line in &prepaint.layout.lines {
                if !line.display_text.is_empty() {
                    let _ = line.shaped.paint(line.origin, line_height, window, cx);
                }
            }
            if let Some(caret) = prepaint.caret.take() {
                window.paint_quad(caret);
            }
        });

        let layout = std::mem::take(&mut prepaint.layout);
        self.editor.update(cx, |editor, _cx| {
            editor.code_layout = layout;
        });
    }
}
