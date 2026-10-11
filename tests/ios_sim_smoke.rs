//! iOS backend smoke test (simulator): window creation via the UIKit
//! backend → raw-handle assertion → deterministic injected-event test
//! (`ios_inject_test_event` → poll → `Event::User` payload verification).
//! iOS 后端冒烟测试（模拟器）：经 UIKit 后端建窗 → 裸句柄断言 →
//! 确定性注入事件测试（`ios_inject_test_event` → poll → `Event::User`
//! 载荷校验）。
//!
//! Entry: `ios_main!(tick)` — the library installs the tick, hands the
//! main thread to UIApplicationMain, and drives the tick each frame via
//! CADisplayLink. Returning `Some(code)` exits the app with that status.
//! 入口：`ios_main!(tick)`——库安装 tick 并把主线程交给
//! UIApplicationMain，经 CADisplayLink 每帧驱动一次 tick；
//! 返回 `Some(code)` 即以该状态码退出应用。
//!
//! CI runs it via `xcrun simctl spawn` inside a booted iPhone simulator
//! (packaged as a minimal .app bundle).
//! CI 通过 `xcrun simctl spawn` 在已启动的 iPhone 模拟器内运行
//! （打包为最小 .app bundle）。
//!
//! Note: no CGEvent usage here — event injection APIs are macOS-only and
//! would fail dyld at load. The deterministic injection goes through the
//! backend buffer (`ios_inject_test_event`).
//! 注意：此处不用 CGEvent——事件注入 API 为 macOS 专属，引用会导致
//! dyld 加载失败。确定性注入走后端缓冲（`ios_inject_test_event`）。

#[cfg(target_os = "ios")]
mod ios {
    use std::cell::Cell;

    use starfish_window::flags::WindowFlags;
    use starfish_window::{Event, RawWindowHandle, UserEvent, Window};

    thread_local! {
        static FRAME: Cell<u32> = const { Cell::new(0) };
        static INJECTED: Cell<bool> = const { Cell::new(false) };
    }

    /// The per-frame host tick (called on the main thread via
    /// CADisplayLink). Frame 0 injects the deterministic test event; the
    /// test passes once it comes back through the poll.
    /// 每帧宿主 tick（经 CADisplayLink 在主线程调用）。第 0 帧注入
    /// 确定性测试事件；经 poll 收回即通过。
    pub fn tick(window: &Window) -> Option<i32> {
        let frame = FRAME.get();
        FRAME.set(frame + 1);
        if frame % 100 == 0 {
            eprintln!("tick frame {frame} / tick 第 {frame} 帧");
        }

        if frame == 0 {
            // Window sanity: nonzero size, UiKit handle.
            // 窗口健全性：非零尺寸 + UiKit 句柄。
            let (w, h) = window.size();
            if w == 0 || h == 0 {
                eprintln!("window size is zero / 窗口尺寸为零: {w}x{h}");
                return Some(1);
            }
            if !matches!(window.raw_handle(), RawWindowHandle::UiKit { .. }) {
                eprintln!("raw_handle is not UiKit / raw_handle 不是 UiKit");
                return Some(1);
            }
            // Inject the deterministic test event.
            // 注入确定性测试事件。
            starfish_window::ios_inject_test_event(Event::User(UserEvent::new(
                7,
                "sim-inject".to_string(),
            )));
        }

        while let Some(event) = window.poll_event() {
            if let Event::User(user) = event {
                if user.code == 7
                    && user.downcast_ref::<String>().map(String::as_str) == Some("sim-inject")
                {
                    println!("iOS smoke OK / iOS 冒烟通过 (frame {frame})");
                    return Some(0);
                }
            }
        }

        // Timeout: 600 frames without the injected event coming back.
        // 超时：600 帧内注入事件未回流。
        if frame > 600 {
            eprintln!("injected event not received within 600 frames / 600 帧内未收到注入事件");
            return Some(1);
        }
        None
    }
}

fn main() {
    #[cfg(target_os = "ios")]
    starfish_window::ios_main!(ios::tick);
    #[cfg(not(target_os = "ios"))]
    {
        println!("skipped: iOS-only smoke / 非 iOS 平台跳过");
    }
}
