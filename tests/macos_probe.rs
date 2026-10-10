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
//!
//! `harness = false` (see Cargo.toml): AppKit is main-thread-only, and
//! libtest runs tests on spawned worker threads even with
//! `--test-threads=1` — so this probe manages its own `main` and thereby
//! runs on the process's real main thread.
//! `harness = false`（见 Cargo.toml）：AppKit 仅限主线程，而 libtest 即使
//! `--test-threads=1` 也在派生线程上执行测试——所以探针自管 `main`，
//! 从而跑在进程真正的主线程上。
//!
//! Non-macOS platforms compile a skipping stub (the objc2 dev-deps are
//! macOS-gated).
//! 非 macOS 平台编译为跳过桩（objc2 dev 依赖仅限 macOS）。

#[cfg(not(target_os = "macos"))]
fn main() {
    println!("skipped: macOS-only probe / 非 macOS 平台跳过");
}

#[cfg(target_os = "macos")]
fn main() {
    std::process::exit(run());
}

#[cfg(target_os = "macos")]
fn run() -> i32 {
    use objc2_app_kit::{
        NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSWindow,
        NSWindowStyleMask,
    };
    use objc2_foundation::{MainThreadMarker, NSRect, ns_string};

    // Rust's test harness is bypassed (harness = false): this main runs on
    // the process's real main thread — a MainThreadMarker is available.
    // 绕过了测试 harness（harness = false）：本 main 跑在进程真正的主线程
    // 上——MainThreadMarker 可用。
    let Some(mtm) = MainThreadMarker::new() else {
        eprintln!("not on the main thread / 不在主线程");
        return 1;
    };

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

    if !window.isVisible() {
        eprintln!("window did not become visible / 窗口未能上屏");
        return 1;
    }
    if window.title().to_string() != "starfish probe" {
        eprintln!("window title mismatch / 窗口标题不符");
        return 1;
    }

    // Screenshot: visual proof in the CI artifacts.
    // 截图：CI artifact 里的视觉证据。
    let out = std::path::Path::new("target/probe.png");
    let status = std::process::Command::new("screencapture")
        .args(["-x", "-t", "png"])
        .arg(out)
        .status()
        .expect("failed to spawn screencapture / 启动 screencapture 失败");
    if !status.success() {
        eprintln!("screencapture failed / 截屏失败");
        return 1;
    }
    if !out.exists() {
        eprintln!("probe screenshot missing / 探针截图不存在");
        return 1;
    }

    println!("probe OK: real NSWindow created and on screen / 探针通过：真实窗口已上屏");
    0
}
