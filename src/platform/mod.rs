//! 横向 cfg 平台层。
//!
//! 每个目标平台一个平行模块，各自实现同一套 `WindowInner` 接口
//! （`new` / `poll_event` / `size` / `set_title` / `Drop`）。
//! 平台模块只对 crate 内部可见，绝不进入公共 API；
//! `window.rs` 只面向 [`imp`] 编程。

#[cfg(target_os = "android")]
#[allow(dead_code)]
pub(crate) mod android;
#[cfg(target_os = "ios")]
#[allow(dead_code)]
pub(crate) mod ios;
#[cfg(target_os = "linux")]
#[allow(dead_code)]
pub(crate) mod linux;
#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub(crate) mod macos;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[allow(dead_code)]
pub(crate) mod web;
#[cfg(target_os = "windows")]
#[allow(dead_code)] // 骨架阶段：接口先于实现，实现补齐后移除
pub(crate) mod windows;

#[cfg(target_os = "android")]
pub(crate) use android as imp;
#[cfg(target_os = "ios")]
pub(crate) use ios as imp;
#[cfg(target_os = "linux")]
pub(crate) use linux as imp;
#[cfg(target_os = "macos")]
pub(crate) use macos as imp;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub(crate) use web as imp;
#[cfg(target_os = "windows")]
pub(crate) use windows as imp;

#[cfg(not(any(
    target_os = "windows",
    target_os = "macos",
    target_os = "ios",
    target_os = "linux",
    target_os = "android",
    all(target_arch = "wasm32", target_os = "unknown"),
)))]
compile_error!("starfish-window 尚不支持该目标平台");
