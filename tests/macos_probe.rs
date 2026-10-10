//! WindowServer probe: verifies that a GitHub macOS runner can actually
//! create a real NSApplication + NSWindow — the assumption the whole macOS
//! backend (and its CI verification) rests on.
//! WindowServer 探针：验证 GitHub 的 macOS runner 能否真的创建
//! NSApplication + NSWindow——这是整个 macOS 后端（及其 CI 验证）的前提。
//!
//! On success it leaves `target/probe.png` (uploaded as a CI artifact):
//! visual confirmation that the window is really on screen.
//! 成功后留 `target/probe.png`（作为 CI artifact 上传）：
//! 窗口真实上屏的视觉证据。

#![cfg(target_os = "macos")]

use objc2_app_kit::{
    NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSWindow, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSRect, ns_string};

#[test]
fn macos_runner_can_create_a_real_window() {
    // Rust's test harness runs on the main thread — a MainThreadMarker is
    // available.
    // Rust 测试 harness 跑在主线程上——MainThreadMarker 可用。
    let mtm = MainThreadMarker::new().expect("not on the main thread / 不在主线程");

    // Bring up the app (regular, docked — like a real windowed app).
    // 拉起应用（Regular 激活策略，与真实窗口应用一致）。
    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Regular);
    app.finishLaunching();

    // Create a real 256×256 window.
    // 创建真实的 256×256 窗口。
    let rect = NSRect::new(
        objc2_foundation::NSPoint::new(100.0, 100.0),
        objc2_foundation::NSSize::new(256.0, 256.0),
    );
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            mtm.alloc::<NSWindow>(),
            rect,
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    window.setTitle(ns_string!("starfish probe"));
    window.makeKeyAndOrderFront(None);
    window.setIsVisible(true);

    assert!(
        window.isVisible(),
        "window did not become visible / 窗口未能上屏"
    );
    assert_eq!(window.title().to_string(), "starfish probe");

    // Screenshot: visual proof in the CI artifacts.
    // 截图：CI artifact 里的视觉证据。
    let out = std::path::Path::new("target/probe.png");
    let status = std::process::Command::new("screencapture")
        .args(["-x", "-t", "png"])
        .arg(out)
        .status()
        .expect("failed to spawn screencapture / 启动 screencapture 失败");
    assert!(status.success(), "screencapture failed / 截屏失败");
    assert!(out.exists(), "probe screenshot missing / 探针截图不存在");
}
