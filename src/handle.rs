//! Raw platform handles: the base layer's bridging exit (zero dependencies).
//! 裸平台句柄：基础层的桥接出口（零依赖）。
//!
//! This is where "the base layer stands alone" materializes: any rendering
//! backend can perform an unsafe bridge with these pointers. With the
//! `wgpu` feature enabled, the zero-unsafe trait path is available instead
//! (see `rwh_impl`).
//! 这是"底层独立"的落点：任何渲染后端都能凭这些指针完成 unsafe 桥接。
//! 启用 `wgpu` feature 后可走零 unsafe 的 trait 路径（见 `rwh_impl`）。

use std::ffi::c_void;

/// The raw platform handle of the current window.
/// 当前窗口的裸平台句柄。
///
/// # Safety（桥接方遵守 / Rules for the bridging side）
///
/// - Pointers are valid only while the [`Window`](crate::Window) is alive;
///   指针仅在 [`Window`](crate::Window) 存活期间有效；
/// - Use them only on the thread the window belongs to (usually the main
///   thread);
///   仅在窗口所属线程（通常为主线程）上使用；
/// - Once handed to a graphics API, correct usage (layer configuration,
///   size synchronization, …) is governed by that API.
///   交给图形 API 后，其正确用法（层配置、尺寸同步等）仍由该 API 约束。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RawWindowHandle {
    /// Windows: window handle and process instance handle.
    /// Windows：窗口句柄与进程实例句柄。
    Win32 {
        /// `HWND`.
        /// `HWND`。
        hwnd: *mut c_void,
        /// `HINSTANCE` (needed by wgpu's display side).
        /// `HINSTANCE`（wgpu 的 display 侧需要）。
        hinstance: *mut c_void,
    },
    /// macOS: the window and its content view.
    /// macOS：窗口与其内容视图。
    AppKit {
        /// `NSWindow *`.
        /// `NSWindow *`。
        ns_window: *mut c_void,
        /// The content `NSView *`.
        /// 内容 `NSView *`。
        ns_view: *mut c_void,
    },
    /// iOS: the root view and its layer (wgpu requires the layer to be a
    /// `CAMetalLayer`).
    /// iOS：根视图与其 layer（wgpu 要求 layer 为 `CAMetalLayer`）。
    UiKit {
        /// `UIView *`.
        /// `UIView *`。
        ui_view: *mut c_void,
        /// The view's `CALayer *`.
        /// 视图的 `CALayer *`。
        layer: *mut c_void,
    },
    /// Linux (X11): the display connection and the window id.
    /// Linux (X11)：显示连接与窗口 id。
    Xlib {
        /// `Display *`.
        /// `Display *`。
        display: *mut c_void,
        /// The X `Window` id.
        /// `Window`（X 窗口 id）。
        window: u64,
    },
    /// Android: `ANativeWindow *`.
    /// Android：`ANativeWindow *`。
    Android {
        /// The native window pointer.
        /// native window 指针。
        a_native_window: *mut c_void,
    },
    /// Web: the host is a canvas element. The pointer is the `JsValue`
    /// object pointer of the `HtmlCanvasElement` (same semantics as
    /// raw-window-handle 0.6 `WebCanvasWindowHandle`; consumed by both the
    /// WebGPU and WebGL backends of wgpu).
    /// Web：宿主为 canvas 元素。指针 = `HtmlCanvasElement` 的 `JsValue`
    /// 对象指针（raw-window-handle 0.6 `WebCanvasWindowHandle` 同语义；
    /// wgpu 的 WebGPU/WebGL 后端均消费该指针）。
    Web {
        /// The `JsValue` object pointer of the `HtmlCanvasElement`.
        /// `HtmlCanvasElement` 的 `JsValue` 对象指针。
        canvas: *mut c_void,
    },
}
