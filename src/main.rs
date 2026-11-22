use bmvp_text::parse_startup_args;
use bmvp_text::ui::editor_view::EditorView;
use gpui::*;

fn main() {
    let startup = parse_startup_args(std::env::args_os());
    let app = Application::new().with_assets(bmvp_text::ui::icons::EmbeddedAssets);

    app.run(move |cx| {
        let startup = startup.clone();
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: Point {
                    x: px(80.0),
                    y: px(80.0),
                },
                size: Size {
                    width: px(1100.0),
                    height: px(720.0),
                },
            })),
            titlebar: Some(TitlebarOptions {
                title: Some("bmvp-text".into()),
                appears_transparent: true,
                traffic_light_position: if cfg!(target_os = "macos") {
                    Some(point(px(10.0), px(10.0)))
                } else {
                    None
                },
            }),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            let view = cx.new(|cx| EditorView::new(window, startup, cx));
            view.read(cx).focus_handle.focus(window);
            let weak_view = view.downgrade();
            window.on_window_should_close(cx, move |_window, app| {
                if let Some(entity) = weak_view.upgrade() {
                    entity.update(app, |this, cx| this.handle_window_should_close(cx))
                } else {
                    true
                }
            });
            view
        })
        .expect("failed to open main window");
    });
}
