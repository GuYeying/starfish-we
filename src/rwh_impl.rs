//! `wgpu` feature：把 [`Window`] 接入 raw-window-handle 生态。
//!
//! 实现后，`instance.create_surface(&window)` 即可零 unsafe 拿到 surface。
//! 只做我们裸句柄 → rwh 类型的机械映射，不引入 wgpu 本体。

use std::num::NonZeroIsize;
use std::ptr::NonNull;

use raw_window_handle::{
    AndroidDisplayHandle, AndroidNdkWindowHandle, DisplayHandle, HandleError, HasDisplayHandle,
    HasWindowHandle, RawDisplayHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
    WindowsDisplayHandle,
};
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use raw_window_handle::{WebCanvasWindowHandle, WebDisplayHandle};

use crate::Window;
use crate::handle::RawWindowHandle as SfHandle;

impl HasWindowHandle for Window {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let raw = match self.raw_handle() {
            SfHandle::Win32 { hwnd, hinstance } => {
                // rwh 的句柄类型是 #[non_exhaustive]，用官方构造器创建、字段赋值补全。
                let hwnd = NonZeroIsize::new(hwnd as isize).ok_or(HandleError::Unavailable)?;
                let mut handle = Win32WindowHandle::new(hwnd);
                handle.hinstance = NonZeroIsize::new(hinstance as isize);
                RawWindowHandle::Win32(handle)
            }
            SfHandle::Android { a_native_window } => {
                let ptr = NonNull::new(a_native_window).ok_or(HandleError::Unavailable)?;
                RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(ptr))
            }
            // Web：canvas 元素的 JsValue 对象指针 → WebCanvasWindowHandle
            // （wgpu 的 WebGPU/WebGL 后端同源消费；仅 wasm 构建存在该变体）
            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
            SfHandle::Web { canvas } => {
                let ptr = NonNull::new(canvas).ok_or(HandleError::Unavailable)?;
                RawWindowHandle::WebCanvas(WebCanvasWindowHandle::new(ptr))
            }
            // 未落地平台（macOS/Linux/iOS 及非 wasm 的 Web 臂）
            _ => return Err(HandleError::Unavailable),
        };
        // SAFETY: 指针来自存活的窗口，'a 借用期与 &self 绑定。
        Ok(unsafe { WindowHandle::borrow_raw(raw) })
    }
}

impl HasDisplayHandle for Window {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        let raw = match self.raw_handle() {
            // rwh 0.6 的 Windows display 句柄不携带数据。
            SfHandle::Win32 { .. } => RawDisplayHandle::Windows(WindowsDisplayHandle::new()),
            SfHandle::Android { .. } => RawDisplayHandle::Android(AndroidDisplayHandle::new()),
            // Web：空 display 句柄（canvas 自带显示语义）
            #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
            SfHandle::Web { .. } => RawDisplayHandle::Web(WebDisplayHandle::new()),
            _ => RawDisplayHandle::Windows(WindowsDisplayHandle::new()),
        };
        // SAFETY: Windows display 句柄为空结构，无借用数据。
        Ok(unsafe { DisplayHandle::borrow_raw(raw) })
    }
}
