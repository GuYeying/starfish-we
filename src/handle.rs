//! 裸平台句柄：基础层的桥接出口（零依赖）。
//!
//! 这是"底层独立"的落点：任何渲染后端都能凭这些指针完成 unsafe 桥接。
//! 启用 `wgpu` feature 后可走零 unsafe 的 trait 路径（见 `rwh_impl`）。

use std::ffi::c_void;

/// 当前窗口的裸平台句柄。
///
/// # Safety（桥接方遵守）
///
/// - 指针仅在 [`Window`](crate::Window) 存活期间有效；
/// - 仅在窗口所属线程（通常为主线程）上使用；
/// - 交给图形 API 后，其正确用法（层配置、尺寸同步等）仍由该 API 约束。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RawWindowHandle {
    /// Windows：窗口句柄与进程实例句柄。
    Win32 {
        /// `HWND`。
        hwnd: *mut c_void,
        /// `HINSTANCE`（wgpu 的 display 侧需要）。
        hinstance: *mut c_void,
    },
    /// macOS：窗口与其内容视图。
    AppKit {
        /// `NSWindow *`。
        ns_window: *mut c_void,
        /// 内容 `NSView *`。
        ns_view: *mut c_void,
    },
    /// iOS：根视图与其 layer（wgpu 要求 layer 为 `CAMetalLayer`）。
    UiKit {
        /// `UIView *`。
        ui_view: *mut c_void,
        /// 视图的 `CALayer *`。
        layer: *mut c_void,
    },
    /// Linux (X11)：显示连接与窗口 id。
    Xlib {
        /// `Display *`。
        display: *mut c_void,
        /// `Window`（X 窗口 id）。
        window: u64,
    },
    /// Android：`ANativeWindow *`。
    Android {
        /// native window 指针。
        a_native_window: *mut c_void,
    },
    /// Web：宿主为 canvas 元素。指针 = `HtmlCanvasElement` 的 `JsValue`
    /// 对象指针（raw-window-handle 0.6 `WebCanvasWindowHandle` 同语义；
    /// wgpu 的 WebGPU/WebGL 后端均消费该指针）。
    Web {
        /// `HtmlCanvasElement` 的 `JsValue` 对象指针。
        canvas: *mut c_void,
    },
}
