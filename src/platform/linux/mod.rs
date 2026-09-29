//! Linux backend — not implemented yet (target distribution: Ubuntu).
//! Linux 后端（待实现，目标发行版 Ubuntu）。
//!
//! Plan: X11 first (`x11rb` or raw Xlib FFI), Wayland later — at that
//! point this directory splits horizontally into `x11` / `wayland`
//! submodules with runtime windowing-system detection.
//! `target_os = "linux"` maps to this module as a whole.
//! 计划：先做 X11（`x11rb` 或裸 Xlib FFI），Wayland 后续再加——
//! 届时在本目录下横向拆 `x11` / `wayland` 子模块，运行时探测窗口系统选择。
//! `target_os = "linux"` 整体对应本模块。

use crate::error::WindowError;
use crate::event::Event;
use crate::handle::RawWindowHandle;
use crate::window::WindowOptions;

/// The Linux platform window implementation (X11 first).
/// Linux 平台窗口实现（X11 先行）。
pub(crate) struct WindowInner;

impl WindowInner {
    pub(crate) fn new(_options: &WindowOptions) -> Result<Self, WindowError> {
        Err(WindowError::UnsupportedPlatform) // TODO: XOpenDisplay + XCreateSimpleWindow
    }

    pub(crate) fn set_resizable(&mut self, _resizable: bool) {}
    pub(crate) fn is_resizable(&self) -> bool {
        false
    }
    pub(crate) fn set_size(&mut self, _width: u32, _height: u32) {}
    pub(crate) fn set_fullscreen(&mut self, _fullscreen: bool) {}
    pub(crate) fn is_fullscreen(&self) -> bool {
        false
    }
    pub(crate) fn set_borderless(&mut self, _borderless: bool) {}
    pub(crate) fn is_borderless(&self) -> bool {
        false
    }
    pub(crate) fn set_mouse_grabbed(&mut self, _grabbed: bool) {}
    pub(crate) fn is_mouse_grabbed(&self) -> bool {
        false
    }
    pub(crate) fn set_mouse_relative(&mut self, _enable: bool) {}
    pub(crate) fn set_cursor_visible(&mut self, _visible: bool) {}
    pub(crate) fn is_cursor_visible(&self) -> bool {
        true
    }
    pub(crate) fn set_visible(&mut self, _visible: bool) {}
    pub(crate) fn is_visible(&self) -> bool {
        true
    }
    pub(crate) fn set_always_on_top(&mut self, _top: bool) {}
    pub(crate) fn is_always_on_top(&self) -> bool {
        false
    }
    pub(crate) fn minimize(&mut self) {}
    pub(crate) fn maximize(&mut self) {}
    pub(crate) fn restore(&mut self) {}
    pub(crate) fn raise(&mut self) {}
    pub(crate) fn is_minimized(&self) -> bool {
        false
    }
    pub(crate) fn is_maximized(&self) -> bool {
        false
    }
    pub(crate) fn has_input_focus(&self) -> bool {
        false
    }
    pub(crate) fn is_occluded(&self) -> bool {
        false
    }
    pub(crate) fn set_keyboard_grabbed(&mut self, _grab: bool) {}
    pub(crate) fn set_position(&mut self, _x: i32, _y: i32) {}
    pub(crate) fn position(&self) -> (i32, i32) {
        (0, 0)
    }
    pub(crate) fn center_on_screen(&mut self) {}
    pub(crate) fn title(&self) -> String {
        String::new()
    }
    pub(crate) fn set_opacity(&mut self, _opacity: f32) {}
    pub(crate) fn opacity(&self) -> f32 {
        1.0
    }
    pub(crate) fn dpi_scale(&self) -> f32 {
        1.0
    }
    pub(crate) fn high_pixel_density(&self) -> bool {
        false
    }

    pub(crate) fn poll_event(&mut self) -> Option<Event> {
        None // TODO: non-blocking XPending pump + KeySym translation / XPending 非阻塞泵 + KeySym 翻译
    }

    pub(crate) fn size(&self) -> (u32, u32) {
        (0, 0) // TODO
    }

    pub(crate) fn set_title(&mut self, _title: &str) {
        // TODO: XStoreName / _NET_WM_NAME (UTF-8)
    }

    pub(crate) fn raw_handle(&self) -> RawWindowHandle {
        unreachable!(
            "backend not implemented for this platform; a Window cannot exist / 该平台后端尚未实现，Window 不可能被构造"
        )
    }
}
