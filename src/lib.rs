//! starfish-window：跨平台**单窗口 + 事件** crate（SDL 风格 poll 模型）。
//!
//! 用于替换 winit：调用者（脚本语言解释器）拥有主循环，主动轮询事件，
//! 而不是把控制流交给事件循环的回调。
//!
//! ```ignore
//! use starfish_window::{Event, Window};
//!
//! let mut window = Window::new("demo", (800, 600), WindowFlags::default())?;
//! loop {
//!     while let Some(event) = window.poll_event() {
//!         if let Event::CloseRequested = event {
//!             return Ok(());
//!         }
//!         // ... 处理其余事件
//!     }
//!     // 解释器在这里推进自己的 tick（执行字节码 / 调度）
//! }
//! ```
//!
//! # 分层与 feature
//!
//! - **基础层（默认）**：窗口 + 事件，零第三方依赖；[`Window::raw_handle`]
//!   暴露裸平台句柄，可用 unsafe 桥接任何渲染后端。
//! - **`wgpu` feature**：实现 raw-window-handle trait，
//!   `instance.create_surface(&window)` 零 unsafe 直通；不引入 wgpu 本体。
//!
//! | feature | 含义 |
//! |---|---|
//! | `window` | 仅建窗能力 |
//! | `event`（默认） | 窗口 + 事件（依赖 `window`） |
//! | `wgpu` | raw-window-handle 对接（依赖 `window`） |
//!
//! 架构：[`platform`] 下各目标平台一个平行模块（横向 cfg），
//! 各自实现同一套 `WindowInner` 接口，公共 API 平台无关。

#![deny(missing_docs)]

mod error;
#[cfg(feature = "event")]
mod event;
pub mod flags;
#[cfg(feature = "window")]
mod handle;
#[cfg(feature = "window")]
mod platform;
#[cfg(feature = "wgpu")]
mod rwh_impl;
#[cfg(feature = "window")]
mod window;

pub use error::WindowError;
#[cfg(feature = "event")]
pub use event::{Event, KeyCode, Modifiers, MouseButton, TouchPhase, UserEvent};
pub use flags::WindowFlags;
#[cfg(feature = "window")]
pub use handle::RawWindowHandle;
#[cfg(feature = "window")]
pub use window::Window;
#[cfg(all(feature = "window", feature = "event"))]
pub use window::{EventProxy, EventProxyClosed};

/// （仅 Android 构建）宿主在 `android_main` 中调用，装配系统回调。
///
/// 平台特有 API 的显式例外：只存在于 Android 构建中。
#[cfg(target_os = "android")]
pub use platform::android::android_init;

/// （仅 Android 构建）系统应用句柄——[`android_init`] 的参数类型，
/// 宿主 `android_main` 签名所需。
#[cfg(target_os = "android")]
pub use android_activity::AndroidApp;

// 内置 Android 演示宿主（android-demo feature）：一行挂上 APK 入口。
#[cfg(all(target_os = "android", feature = "android-demo"))]
crate::android_main!(platform::android::entry::demo::demo_main);
