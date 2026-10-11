//! Stub-platform contract: on a platform without a backend, window
//! creation must fail explicitly with `UnsupportedPlatform` — not
//! panic, hang, or leak the single-window slot.
//! 无后端平台的契约：建窗必须显式返回 `UnsupportedPlatform`——
//! 不得 panic、不得挂起，也不得泄漏单窗口占位。
//!
//! Currently Linux is the only stub; when its X11 backend lands, flip
//! this file into a real smoke test (shape reference:
//! `tests/windows_smoke.rs`).
//! 当前仅 Linux 为 stub；X11 后端落地后，本文件应改写为真实冒烟
//! （形态参考 `tests/windows_smoke.rs`）。

#![cfg(target_os = "linux")]

use starfish_we::{Window, WindowError};

#[test]
fn new_reports_unsupported_platform() {
    let err = Window::new("stub", (100, 100), Default::default())
        .err()
        .expect("stub Window::new must fail / stub 建窗必须失败");
    assert_eq!(
        err,
        WindowError::UnsupportedPlatform,
        "expected UnsupportedPlatform / 预期 UnsupportedPlatform"
    );
}

#[test]
fn builder_reports_unsupported_platform() {
    let err = Window::builder()
        .title("stub")
        .size((100, 100))
        .build()
        .err()
        .expect("stub builder().build() must fail / stub builder 必须失败");
    assert_eq!(err, WindowError::UnsupportedPlatform);
}

#[test]
fn slot_is_released_after_failure() {
    // The single-window slot must not leak through the stub path: two
    // sequential attempts both hit UnsupportedPlatform — the second must
    // not be misreported as `AlreadyExists`.
    // 单窗口占位不得经 stub 路径泄漏：连续两次都应报
    // UnsupportedPlatform——第二次不得误报 `AlreadyExists`。
    for i in 0..2 {
        let err = Window::new("stub", (100, 100), Default::default())
            .err()
            .expect("stub attempt must fail / stub 尝试必须失败");
        assert_eq!(err, WindowError::UnsupportedPlatform, "attempt {i}");
    }
}
