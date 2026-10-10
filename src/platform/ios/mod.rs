//! iOS (UIKit) backend — not implemented yet.
//! iOS (UIKit) 后端（待实现）。
//!
//! Plan: `objc2-ui-kit`. iOS has no freedom to create a window outside
//! "main"; the lean is to skip `UIApplicationMain` and instantiate
//! `UIApplication` + `UIWindow` manually, buffering touch / lifecycle
//! events into a queue drained by `poll_event`:
//! touch → `Event::Touch` (the primary finger also synthesizes mouse
//! events), background / foreground → `Event::Suspended` /
//! `Event::Resumed`.
//! 计划：`objc2-ui-kit`。iOS 没有"main 之外建窗"的自由度，倾向不走
//! `UIApplicationMain`，而是手动实例化 `UIApplication` + `UIWindow`，
//! 把触摸 / 生命周期事件缓冲进队列供 `poll_event` 排空：
//! 触摸 → `Event::Touch`（主手指同时合成鼠标事件），后台 / 回前台 →
//! `Event::Suspended` / `Event::Resumed`。
//! Note: the view's layer must be a `CAMetalLayer` (a wgpu requirement);
//! specify layerClass when creating the view. The exact wiring (scene
//! delegate / manual runloop tick) is decided at implementation time.
//! 注意：view 的 layer 必须用 `CAMetalLayer`（wgpu 要求），建 view 时指定 layerClass。
//! 具体接线（scene delegate / 手动 runloop tick）实现时定。

use crate::error::WindowError;
use crate::event::Event;
use crate::handle::RawWindowHandle;
use crate::window::WindowOptions;

/// The UIKit platform window implementation.
/// UIKit 平台窗口实现。
pub(crate) struct WindowInner;

impl WindowInner {
    pub(crate) fn new(_options: &WindowOptions) -> Result<Self, WindowError> {
        Err(WindowError::UnsupportedPlatform) // TODO: UIApplication + UIWindow
    }

    pub(crate) fn set_resizable(&self, _resizable: bool) {}
    pub(crate) fn is_resizable(&self) -> bool {
        false
    }
    pub(crate) fn set_size(&self, _width: u32, _height: u32) {}
    pub(crate) fn set_fullscreen(&self, _fullscreen: bool) {}
    pub(crate) fn is_fullscreen(&self) -> bool {
        false
    }
    pub(crate) fn set_borderless(&self, _borderless: bool) {}
    pub(crate) fn is_borderless(&self) -> bool {
        false
    }
    pub(crate) fn set_mouse_grabbed(&self, _grabbed: bool) {}
    pub(crate) fn is_mouse_grabbed(&self) -> bool {
        false
    }
    pub(crate) fn set_mouse_relative(&self, _enable: bool) {}
    pub(crate) fn set_cursor_visible(&self, _visible: bool) {}
    pub(crate) fn is_cursor_visible(&self) -> bool {
        true
    }
    pub(crate) fn set_visible(&self, _visible: bool) {}
    pub(crate) fn is_visible(&self) -> bool {
        true
    }
    pub(crate) fn set_always_on_top(&self, _top: bool) {}
    pub(crate) fn is_always_on_top(&self) -> bool {
        false
    }
    pub(crate) fn minimize(&self) {}
    pub(crate) fn maximize(&self) {}
    pub(crate) fn restore(&self) {}
    pub(crate) fn raise(&self) {}
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
    pub(crate) fn set_keyboard_grabbed(&self, _grab: bool) {}
    pub(crate) fn set_position(&self, _x: i32, _y: i32) {}
    pub(crate) fn position(&self) -> (i32, i32) {
        (0, 0)
    }
    pub(crate) fn center_on_screen(&self) {}
    pub(crate) fn title(&self) -> String {
        String::new()
    }
    pub(crate) fn set_opacity(&self, _opacity: f32) {}
    pub(crate) fn opacity(&self) -> f32 {
        1.0
    }
    pub(crate) fn dpi_scale(&self) -> f32 {
        1.0
    }
    pub(crate) fn high_pixel_density(&self) -> bool {
        false
    }

    pub(crate) fn poll_event(&self) -> Option<Event> {
        None // TODO: drain from the buffered event queue / 从事件缓冲队列排空
    }

    pub(crate) fn size(&self) -> (u32, u32) {
        // TODO (physical pixels, bounds * scale / 物理像素，bounds * scale)
        (0, 0)
    }

    pub(crate) fn set_title(&self, _title: &str) {
        // iOS has no title-bar concept; map to the root view's accessible
        // label or ignore entirely.
        // iOS 无标题栏概念；映射为顶层视图 accessible 标签或直接忽略
    }

    pub(crate) fn raw_handle(&self) -> RawWindowHandle {
        unreachable!(
            "backend not implemented for this platform; a Window cannot exist / 该平台后端尚未实现，Window 不可能被构造"
        )
    }
}
