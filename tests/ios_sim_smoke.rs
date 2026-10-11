//! iOS backend smoke test (simulator): window creation via the UIKit
//! backend → raw-handle assertion → deterministic injected-event test
//! (`ios_inject_test_event` → poll → `Event::User` payload verification).
//! iOS 后端冒烟测试（模拟器）：经 UIKit 后端建窗 → 裸句柄断言 →
//! 确定性注入事件测试（`ios_inject_test_event` → poll → `Event::User`
//! 载荷校验）。
//!
//! Entry: `ios_main!(tick)` — UIApplicationMain owns the main thread and
//! CADisplayLink drives the tick each frame; the tick returns
//! `Some(code)` to exit the app with that status.
//! 入口：`ios_main!(tick)`——UIApplicationMain 掌管主线程，
//! CADisplayLink 每帧驱动一次 tick；tick 返回 `Some(code)` 即以该状态码
//! 退出应用。
//!
//! Result protocol (staged, in `tmp/result.txt` inside the app's data
//! container; CI polls it via `simctl get_app_container … data`):
//! `STARTED` (process up) → `FRAME0` (tick reached) → terminal `OK` /
//! `FAIL: …` / `PANIC: …` (a panic hook installed in `main` captures
//! crashes from the whole bootstrap, including `UIApplicationMain` and
//! `didFinishLaunching`).
//! 结果协议（分阶段，写入应用数据容器内的 `tmp/result.txt`，CI 经
//! `simctl get_app_container … data` 轮询）：`STARTED`（进程已起）→
//! `FRAME0`（已到达 tick）→ 终态 `OK` / `FAIL: …` / `PANIC: …`
//! （`main` 里安装的 panic 钩子捕获整个引导期的崩溃，含
//! `UIApplicationMain` 与 `didFinishLaunching`）。

#[cfg(target_os = "ios")]
mod ios {
    use std::cell::Cell;

    use starfish_we::{Event, RawWindowHandle, UserEvent, Window};

    thread_local! {
        static FRAME: Cell<u32> = const { Cell::new(0) };
        static INJECTED: Cell<bool> = const { Cell::new(false) };
    }

    /// The result file path inside the app container's tmp directory.
    /// 应用容器 tmp 目录下的结果文件路径。
    pub fn result_path() -> String {
        format!(
            "{}result.txt",
            unsafe { objc2_foundation::NSTemporaryDirectory() }.to_string()
        )
    }

    /// Writes the smoke result into the app container's tmp directory.
    /// 将冒烟结果写入应用容器 tmp 目录下的结果文件。
    pub fn write_result(ok: bool, detail: &str) {
        let content = if ok {
            format!("OK / 通过: {detail}")
        } else {
            format!("FAIL / 失败: {detail}")
        };
        let _ = std::fs::write(result_path(), content);
    }

    /// The per-frame host tick (called on the main thread via
    /// CADisplayLink). Frame 0 writes the FRAME0 marker (distinguishing
    /// "bootstrap finished" from "process up") and injects the
    /// deterministic test event; the test passes once it comes back
    /// through the poll.
    /// 每帧宿主 tick（经 CADisplayLink 在主线程调用）。第 0 帧写入
    /// FRAME0 标记（区分“引导完成”与“进程已起”）并注入确定性测试
    /// 事件；经 poll 收回即通过。
    pub fn tick(window: &Window) -> Option<i32> {
        let frame = FRAME.get();
        FRAME.set(frame + 1);
        eprintln!("[smoke] tick frame {frame} / tick 第 {frame} 帧");

        if frame == 0 {
            // Marker: the app reached the tick (the earlier STARTED only
            // proves the process launched).
            // 标记：应用已到达 tick（更早的 STARTED 只证明进程已启动）。
            let _ = std::fs::write(result_path(), "FRAME0");

            // Window sanity: nonzero size, UiKit handle.
            // 窗口健全性：非零尺寸 + UiKit 句柄。
            let (w, h) = window.size();
            eprintln!("[smoke] frame 0 size {w}x{h} / 窗口尺寸 {w}x{h}");
            if w == 0 || h == 0 {
                write_result(false, "window size is zero / 窗口尺寸为零");
                std::process::exit(1);
            }
            if !matches!(window.raw_handle(), RawWindowHandle::UiKit { .. }) {
                write_result(false, "raw_handle is not UiKit / raw_handle 不是 UiKit");
                std::process::exit(1);
            }
            // Inject the deterministic test event.
            // 注入确定性测试事件。
            starfish_we::ios_inject_test_event(Event::User(UserEvent::new(
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
                    write_result(true, "injected event roundtrip / 注入事件往返通过");
                    std::process::exit(0);
                }
            }
        }

        // Timeout: 600 frames without the injected event coming back.
        // 超时：600 帧内注入事件未回流。
        if frame > 600 {
            eprintln!("injected event not received within 600 frames / 600 帧内未收到注入事件");
            write_result(
                false,
                "injected event not received within 600 frames / 600 帧内未收到注入事件",
            );
            std::process::exit(1);
        }
        None
    }
}

fn main() {
    // Earliest marker + panic hook: the hook must be installed before
    // `UIApplicationMain` so crashes anywhere in the UIKit bootstrap land
    // in the result file (CI stderr is invisible).
    // 最早标记 + panic 钩子：钩子必须装在 `UIApplicationMain` 之前，
    // 这样 UIKit 引导期任何位置的崩溃都会落进结果文件（CI 上 stderr
    // 不可见）。
    #[cfg(target_os = "ios")]
    {
        let tmp = unsafe { objc2_foundation::NSTemporaryDirectory() }.to_string();
        let _ = std::fs::write(format!("{tmp}result.txt"), "STARTED");

        let hook_path = format!("{tmp}result.txt");
        std::panic::set_hook(Box::new(move |info| {
            let _ = std::fs::write(&hook_path, format!("PANIC: {info}"));
        }));
    }

    #[cfg(target_os = "ios")]
    starfish_we::ios_main!(ios::tick);
    #[cfg(not(target_os = "ios"))]
    {
        println!("skipped: iOS-only smoke / 非 iOS 平台跳过");
    }
}
