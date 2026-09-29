//! macOS (AppKit) backend — not implemented yet.
//! macOS (AppKit) 后端（待实现）。
//!
//! Plan: `objc2` + `objc2-app-kit`. The first `Window::new` initializes
//! `NSApplication` (`NSApplicationActivationPolicy::Regular`) and creates
//! the `NSWindow`; `poll_event` pumps events non-blockingly via
//! `nextEventMatchingMask:untilDate:inMode:dequeue:`.
//! 计划：`objc2` + `objc2-app-kit`。首次 `Window::new` 时初始化 `NSApplication`
//! （`NSApplicationActivationPolicy::Regular`），建 `NSWindow`；
//! `poll_event` 内用 `nextEventMatchingMask:untilDate:inMode:dequeue:` 非阻塞泵事件。
//! Hard constraint: all calls must happen on the main thread (an AppKit
//! requirement) — already documented in the public API.
//! 硬约束：所有调用必须在主线程（AppKit 要求），已写入公共 API 文档。

use crate::error::WindowError;
use crate::event::Event;
use crate::handle::RawWindowHandle;
use crate::window::WindowOptions;

/// The AppKit platform window implementation.
/// AppKit 平台窗口实现。
pub(crate) struct WindowInner;

impl WindowInner {
    pub(crate) fn new(_options: &WindowOptions) -> Result<Self, WindowError> {
        Err(WindowError::UnsupportedPlatform) // TODO: NSApplication + NSWindow
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
        None // TODO: NSEvent pump + translation / NSEvent 泵 + 翻译
    }

    pub(crate) fn size(&self) -> (u32, u32) {
        // TODO: mind backingScaleFactor vs the physical-pixel convention
        (0, 0) // TODO（注意 backingscaled 与物理像素约定）
    }

    pub(crate) fn set_title(&mut self, _title: &str) {
        // TODO: setTitle:
    }

    pub(crate) fn raw_handle(&self) -> RawWindowHandle {
        unreachable!(
            "backend not implemented for this platform; a Window cannot exist / 该平台后端尚未实现，Window 不可能被构造"
        )
    }
}
