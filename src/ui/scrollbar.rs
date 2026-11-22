use crate::ui::UiMouseAction;
use crate::ui::theme::Theme;
use gpui::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollbarMetrics {
    pub thumb_top_ratio: f32,
    pub thumb_height_ratio: f32,
}

pub fn compute_metrics(
    total_items: usize,
    visible_items: usize,
    scroll_offset: usize,
) -> Option<ScrollbarMetrics> {
    if visible_items == 0 || total_items <= visible_items {
        return None;
    }
    let thumb_height_ratio = (visible_items as f32 / total_items as f32).clamp(0.08, 1.0);
    let max_scroll = total_items - visible_items;
    let progress = (scroll_offset.min(max_scroll) as f32) / (max_scroll.max(1) as f32);
    let thumb_top_ratio = progress * (1.0 - thumb_height_ratio);
    Some(ScrollbarMetrics {
        thumb_top_ratio,
        thumb_height_ratio,
    })
}

pub fn render_vertical_scrollbar(
    total_items: usize,
    visible_items: usize,
    scroll_offset: usize,
    track_height: f32,
    theme: &Theme,
    on_track_mouse_down: Option<UiMouseAction>,
) -> Option<AnyElement> {
    let metrics = compute_metrics(total_items, visible_items, scroll_offset)?;
    let usable_track = (track_height - 4.0).max(20.0);
    let thumb_h = (usable_track * metrics.thumb_height_ratio)
        .max(20.0)
        .min(usable_track);
    let max_top = (usable_track - thumb_h).max(0.0);
    let avail_ratio = 1.0 - metrics.thumb_height_ratio;
    let progress = if avail_ratio > f32::EPSILON {
        (metrics.thumb_top_ratio / avail_ratio).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let thumb_top = (max_top * progress).max(0.0);

    let thumb = div()
        .w(px(4.0))
        .mt(px(thumb_top))
        .h(px(thumb_h))
        .rounded_full()
        .bg(theme.scrollbar_thumb)
        .hover(|s| s.bg(theme.scrollbar_thumb_hover));

    let mut track = div()
        .w(px(10.0))
        .h_full()
        .py_0p5()
        .flex()
        .flex_col()
        .items_center()
        .child(thumb);

    if let Some(on_down) = on_track_mouse_down {
        track = track
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, move |ev, win, cx| {
                cx.stop_propagation();
                on_down(ev, win, cx);
            });
    }

    Some(track.into_any_element())
}

#[cfg(test)]
mod tests {
    use super::{ScrollbarMetrics, compute_metrics};

    #[test]
    fn test_scrollbar_compute_metrics_edge_cases() {
        assert_eq!(compute_metrics(10, 0, 0), None);
        assert_eq!(compute_metrics(10, 10, 0), None);
        assert_eq!(compute_metrics(5, 10, 0), None);

        let top = compute_metrics(100, 25, 0).expect("scrollable");
        assert_eq!(
            top,
            ScrollbarMetrics {
                thumb_top_ratio: 0.0,
                thumb_height_ratio: 0.25,
            }
        );

        let bottom = compute_metrics(100, 25, 75).expect("at bottom");
        assert!((bottom.thumb_top_ratio - 0.75).abs() < 1e-5);
        assert!((bottom.thumb_height_ratio - 0.25).abs() < 1e-5);

        let clamped = compute_metrics(100, 25, 500).expect("clamped overflow");
        assert_eq!(clamped, bottom);

        let huge = compute_metrics(10_000, 10, 0).expect("huge file");
        assert!((huge.thumb_height_ratio - 0.08).abs() < 1e-6);
    }
}
