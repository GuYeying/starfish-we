//! Windows backend smoke test: real window creation (a window flashes
//! during the test run), draining initial events, the single-window
//! guard, full builder options and re-creation after drop.
//! Windows 后端冒烟测试：真实建窗（测试运行时会闪一闪窗口）、
//! 排空初始事件、验证单窗口守卫、builder 全选项与 drop 后重建。
//!
//! The single-window slot is process-global state — every assertion must
//! stay inside **one test function**, executed sequentially (cargo runs
//! multiple test functions in parallel by default, and they would race
//! for the slot).
//! 单窗口占位是进程级全局状态，所有断言必须保持在**同一个测试函数**内
//! 顺序执行（cargo 默认并行运行多个测试函数会互相争抢占位）。

#![cfg(windows)]

use starfish_window::flags::WindowFlags;
use starfish_window::{Event, EventProxyClosed, Window, WindowError};

#[test]
fn window_lifecycle_and_builder() {
    // ---- Basic path: Window::new / 基础路径：Window::new ----
    let window =
        Window::new("starfish smoke", (320, 240), WindowFlags::default()).expect("建窗失败");

    // The client area exactly equals the requested size (the frame was
    // converted via AdjustWindowRectEx).
    // 客户区精确等于请求尺寸（AdjustWindowRectEx 换算过外框）。
    assert_eq!(window.size(), (320, 240));

    window.set_title("starfish smoke 2");

    // Drain the initial creation messages (WM_SIZE / WM_PAINT / focus…).
    // 排空建窗初期消息（WM_SIZE / WM_PAINT / 焦点等）。
    for _ in 0..64 {
        if window.poll_event().is_none() {
            break;
        }
    }

    // Single-window guard: while one window lives, a second creation must
    // be rejected.
    // 单窗口守卫：存活期间第二次建窗必须被拒绝。
    assert!(matches!(
        Window::new("second", (10, 10), WindowFlags::default()),
        Err(WindowError::AlreadyExists)
    ));

    // ---- Builder, all options: borderless + fixed + size range +
    //      always-on-top / builder 全选项：无边框 + 不可调 + 尺寸范围 + 置顶
    drop(window);
    let window = Window::builder()
        .title("builder smoke")
        .size((400, 300))
        .resizable(false)
        .borderless(true)
        .min_size((200, 150))
        .max_size((800, 600))
        .always_on_top(true)
        .build()
        .expect("builder 建窗失败");

    // Borderless (WS_POPUP): the client area exactly equals the requested
    // size.
    // 无边框（WS_POPUP）下客户区精确等于请求尺寸。
    assert_eq!(window.size(), (400, 300));

    for _ in 0..64 {
        if window.poll_event().is_none() {
            break;
        }
    }

    // Builder-created windows are subject to the same single-window
    // guard.
    // builder 建的窗口同样受单窗口守卫约束。
    assert!(matches!(
        Window::builder().size((10, 10)).build(),
        Err(WindowError::AlreadyExists)
    ));

    // ---- After drop, the Window::new compatibility path still works /
    //      drop 后 Window::new 兼容路径仍可用 ----
    drop(window);
    let window = Window::new("compat", (100, 100), WindowFlags::default()).expect("重建失败");

    // ---- Custom events: cross-thread injection → poll dispatch →
    //      downcast → defunct after close /
    //      自定义事件：跨线程注入 → poll 派发 → downcast → 关闭后失效 ----
    let proxy = window.event_proxy();
    let proxy_for_thread = proxy.clone();
    std::thread::spawn(move || {
        proxy_for_thread
            .send(42, "from worker thread / 来自工作线程".to_string())
            .expect("send 失败");
    })
    .join()
    .expect("worker thread panicked / 工作线程 panic");

    let mut got_user = None;
    for _ in 0..1024 {
        match window.poll_event() {
            Some(Event::User(user)) => {
                got_user = Some(user);
                break;
            }
            Some(_) => {} // Platform events drain as usual / 平台事件照常排掉
            None => std::thread::sleep(std::time::Duration::from_millis(1)),
        }
    }
    let user = got_user.expect("custom event not received / 未收到自定义事件");
    assert_eq!(user.code, 42);
    assert_eq!(
        user.downcast_ref::<String>().map(String::as_str),
        Some("from worker thread / 来自工作线程")
    );

    // After the window is destroyed, the proxy fails explicitly.
    // 窗口销毁后，代理显式失效。
    drop(window);
    assert!(matches!(proxy.send(1, 2), Err(EventProxyClosed)));
}
