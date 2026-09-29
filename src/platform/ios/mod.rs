//! iOS (UIKit) 后端（待实现）。
//!
//! 计划：`objc2-ui-kit`。iOS 没有"main 之外建窗"的自由度，倾向不走
//! `UIApplicationMain`，而是手动实例化 `UIApplication` + `UIWindow`，
//! 把触摸 / 生命周期事件缓冲进队列供 `poll_event` 排空：
//! 触摸 → `Event::Touch`（主手指同时合成鼠标事件），后台 / 回前台 →
//! `Event::Suspended` / `Event::Resumed`。
//! 注意：view 的 layer 必须用 `CAMetalLayer`（wgpu 要求），建 view 时指定 layerClass。
//! 具体接线（scene delegate / 手动 runloop tick）实现时定。

use crate::error::WindowError;
use crate::event::Event;
use crate::handle::RawWindowHandle;
use crate::window::WindowOptions;

/// UIKit 平台窗口实现。
pub(crate) struct WindowInner;

impl WindowInner {
    pub(crate) fn new(_options: &WindowOptions) -> Result<Self, WindowError> {
        Err(WindowError::UnsupportedPlatform) // TODO: UIApplication + UIWindow
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
        None // TODO: 从事件缓冲队列排空
    }

    pub(crate) fn size(&self) -> (u32, u32) {
        (0, 0) // TODO（物理像素，bounds * scale）
    }

    pub(crate) fn set_title(&mut self, _title: &str) {
        // iOS 无标题栏概念；映射为顶层视图 accessible 标签或直接忽略
    }

    pub(crate) fn raw_handle(&self) -> RawWindowHandle {
        unreachable!("该平台后端尚未实现，Window 不可能被构造")
    }
}
