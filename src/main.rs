mod buffer;
mod editor;
mod file_finder;
mod find_replace;
mod system_terminal;
mod settings;
mod settings_modal;
mod sidebar;
mod status_bar;
mod syntax;
mod theme;
mod window_controls;

use gpui::*;
use editor::EditorView;

fn main() {
    let app = Application::new();

    app.run(|cx| {
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
                traffic_light_position: None,
            }),
            ..Default::default()
        };

        cx.open_window(options, |window, cx| {
            let view = cx.new(|cx| EditorView::new(window, cx));
            view.read(cx).focus_handle.focus(window);
            view
        })
        .expect("failed to open main window");
    });
}
