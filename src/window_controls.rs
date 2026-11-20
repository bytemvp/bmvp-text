use gpui::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

#[cfg(target_os = "windows")]
#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[cfg(target_os = "windows")]
unsafe extern "system" {
    fn ReleaseCapture() -> i32;
    fn SendMessageW(hwnd: isize, msg: u32, wparam: usize, lparam: isize) -> isize;
    fn ShowWindowAsync(hwnd: isize, cmd: i32) -> i32;
    fn IsZoomed(hwnd: isize) -> i32;
    fn GetCursorPos(lp_point: *mut Point) -> i32;
}

#[cfg(target_os = "windows")]
const SW_RESTORE: i32 = 9;
#[cfg(target_os = "windows")]
const SW_MAXIMIZE: i32 = 3;
#[cfg(target_os = "windows")]
const SW_MINIMIZE: i32 = 6;

pub fn get_window_hwnd(window: &Window) -> Option<isize> {
    if let Ok(handle) = HasWindowHandle::window_handle(window)
        && let RawWindowHandle::Win32(win32) = handle.as_raw()
    {
        return Some(win32.hwnd.get());
    }
    None
}

pub fn is_window_maximized(window: &Window) -> bool {
    #[cfg(target_os = "windows")]
    if let Some(hwnd) = get_window_hwnd(window) {
        return unsafe { IsZoomed(hwnd) != 0 };
    }
    window.is_maximized()
}

pub fn toggle_maximize(window: &Window) {
    #[cfg(target_os = "windows")]
    if let Some(hwnd) = get_window_hwnd(window) {
        unsafe {
            let is_zoomed = IsZoomed(hwnd) != 0;
            if is_zoomed {
                ShowWindowAsync(hwnd, SW_RESTORE);
            } else {
                ShowWindowAsync(hwnd, SW_MAXIMIZE);
            }
        }
        return;
    }
    window.zoom_window();
}

pub fn minimize_window(window: &Window) {
    #[cfg(target_os = "windows")]
    if let Some(hwnd) = get_window_hwnd(window) {
        unsafe {
            ShowWindowAsync(hwnd, SW_MINIMIZE);
        }
        return;
    }
    window.minimize_window();
}

pub fn start_window_drag(window: &Window) {
    #[cfg(not(target_os = "windows"))]
    let _ = window;
    #[cfg(target_os = "windows")]
    if let Some(hwnd) = get_window_hwnd(window) {
        unsafe {
            ReleaseCapture();
            let mut pt = Point { x: 0, y: 0 };
            GetCursorPos(&mut pt);
            let lparam = ((pt.y as isize) << 16) | ((pt.x as isize) & 0xFFFF);
            // WM_NCLBUTTONDOWN = 0x00A1, HTCAPTION = 2
            SendMessageW(hwnd, 0x00A1, 2, lparam);
        }
    }
}
