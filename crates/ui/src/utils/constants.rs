#[cfg(not(target_os = "macos"))]
use gpui::px;
use gpui::{Pixels, Window};

/// Returns the platform-appropriate title bar height.
///
/// On macOS, this queries the actual titlebar height from the platform,
/// which accounts for the native toolbar.
/// On Windows, this returns a fixed height of 32px.
/// On other platforms, it scales with the window's rem size (1.75x) with a minimum of 34px.
#[cfg(target_os = "macos")]
pub fn platform_title_bar_height(window: &Window) -> Pixels {
    window.titlebar_height()
}

/// Returns the leading title bar space reserved by platform-managed window
/// controls.
#[cfg(target_os = "macos")]
pub fn platform_window_controls_padding(window: &Window) -> Pixels {
    window.window_controls_padding()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn platform_title_bar_height(window: &Window) -> Pixels {
    (1.75 * window.rem_size()).max(px(34.))
}

#[cfg(target_os = "windows")]
pub fn platform_title_bar_height(_window: &Window) -> Pixels {
    // todo(windows) instead of hard coded size report the actual size to the Windows platform API
    px(32.)
}

#[cfg(not(target_os = "macos"))]
pub fn platform_window_controls_padding(_window: &Window) -> Pixels {
    px(0.)
}
