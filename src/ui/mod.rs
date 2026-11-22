pub mod breadcrumb;
pub mod code_canvas;
pub mod confirm_modal;
mod editor_commands;
mod editor_input;
pub mod editor_view;
pub mod file_finder;
pub mod find_replace;
pub mod icons;
pub mod scrollbar;
pub mod settings_modal;
pub mod sidebar_view;
pub mod status_bar;
pub mod tab_bar;
pub mod terminal_dock;
pub mod theme;
pub mod title_bar;

pub type UiAction = std::rc::Rc<dyn Fn(&mut gpui::Window, &mut gpui::App) + 'static>;
pub type UiAction1<A> = std::rc::Rc<dyn Fn(A, &mut gpui::Window, &mut gpui::App) + 'static>;
pub type UiAction2<A, B> = std::rc::Rc<dyn Fn(A, B, &mut gpui::Window, &mut gpui::App) + 'static>;
pub type UiMouseAction =
    std::rc::Rc<dyn Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + 'static>;
