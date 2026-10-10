//! iOS backend smoke test (simulator): window creation via the UIKit
//! backend → raw-handle assertion → deterministic injected-event test
//! (`ios_inject_test_event` → poll → `Event::User` payload verification).
//! iOS 后端冒烟测试（模拟器）：经 UIKit 后端建窗 → 裸句柄断言 →
//! 确定性注入事件测试（`ios_inject_test_event` → poll → `Event::User`
//! 载荷校验）。
//!
//! `harness = false` + dual main: the UIKit backend is main-thread-only,
//! and non-iOS hosts (Windows `cargo test`) build this file too — the
//! non-iOS main prints a skip line.
//! `harness = false` + 双 main：UIKit 后端仅限主线程，且非 iOS 宿主
//! （Windows 上的 `cargo test`）也会构建本文件——非 iOS 的 main 仅打印
//! 跳过行。
//!
//! CI runs it via `xcrun simctl spawn` inside a booted iPhone simulator.
//! CI 通过 `xcrun simctl spawn` 在已启动的 iPhone 模拟器内运行。

#[cfg(target_os = "ios")]
mod ios {
    use starfish_window::flags::WindowFlags;
    use starfish_window::{Event, RawWindowHandle, UserEvent, Window};

    pub fn run() -> i32 {
        std::panic::set_hook(Box::new(|info| eprintln!("panic: {info}")));

        // 1. Window creation with the real backend (bootstraps
        //    UIApplication + full-screen UIWindow).
        // 1. 经真实后端建窗（自动装配 UIApplication + 全屏 UIWindow）。
        let window = match Window::new("starfish ios smoke", (320, 240), WindowFlags::default()) {
            Ok(w) => w,
            Err(err) => {
                eprintln!("window creation failed / 建窗失败: {err}");
                return 1;
            }
        };

        // 2. Raw handle must resolve to the UiKit variant.
        // 2. 裸句柄必须解析为 UiKit 变体。
        if !matches!(window.raw_handle(), RawWindowHandle::UiKit { .. }) {
            eprintln!("raw_handle is not UiKit / raw_handle 不是 UiKit");
            return 1;
        }

        // 3. Deterministic injected-event test: inject → poll → verify
        //    payload (the exact "simulated event send" the library
        //    contract promises).
        // 3. 确定性注入事件测试：注入 → poll → 校验载荷（即本库承诺的
        //    "模拟发送事件"通道）。
        starfish_window::ios_inject_test_event(Event::User(UserEvent::new(
            7,
            "sim-inject".to_string(),
        )));

        let mut got_user = false;
        for _ in 0..128 {
            match window.poll_event() {
                Some(Event::User(user)) if user.code == 7 => {
                    got_user =
                        user.downcast_ref::<String>().map(String::as_str) == Some("sim-inject");
                    break;
                }
                Some(_) => {}
                None => std::thread::sleep(std::time::Duration::from_millis(5)),
            }
        }
        if !got_user {
            eprintln!("injected event not received / 未收到注入事件");
            return 1;
        }

        println!("iOS smoke OK / iOS 冒烟通过");
        0
    }
}

fn main() {
    #[cfg(target_os = "ios")]
    {
        std::process::exit(ios::run());
    }
    #[cfg(not(target_os = "ios"))]
    {
        println!("skipped: iOS-only smoke / 非 iOS 平台跳过");
    }
}
