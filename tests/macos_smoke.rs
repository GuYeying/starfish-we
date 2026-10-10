//! macOS backend smoke test (main thread, via our AppKit backend):
//! window creation → title/size → CGEventPost-injected events → poll
//! dispatch verification.
//! macOS 后端冒烟测试（主线程，经本库 AppKit 后端）：
//! 建窗 → 标题/尺寸 → CGEventPost 注入事件 → poll 派发验证。
//!
//! `harness = false` + dual main: the AppKit backend is main-thread-only
//! (libtest can't provide that), and the FFI/helpers must be cfg'd out on
//! non-macOS hosts (`cargo test` on Windows builds this file too).
//! `harness = false` + 双 main：AppKit 后端仅限主线程（libtest 无法提供，
//! 见 tests/macos_probe.rs），且 FFI/辅助代码在非 macOS 宿主上必须
//! cfg 掉（Windows 上的 `cargo test` 也会构建本文件）。
//!
//! Event injection uses CGEventPost against the session's event stream —
//! synthetic events are indistinguishable from real ones downstream, so
//! this exercises the full backend path (pump → translate → queue).
//! 事件注入用 CGEventPost 打进会话事件流——合成事件与真实事件在后端
//! 视角完全一致，因此验证覆盖完整路径（泵 → 翻译 → 队列）。

#[cfg(target_os = "macos")]
mod macos {
    use std::time::Duration;

    use starfish_window::flags::WindowFlags;
    use starfish_window::{Event, RawWindowHandle, Window};

    // CGEvent injection (ApplicationServices; minimal FFI surface for
    // tests).
    // CGEvent 注入（ApplicationServices；测试所需的最小 FFI 面）。
    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn CGEventCreateMouseEvent(
            source: *mut std::ffi::c_void,
            mouse_type: i32,
            point: CGPoint,
            button: i32,
        ) -> *mut std::ffi::c_void;
        fn CGEventPost(tap: u32, event: *mut std::ffi::c_void);
    }

    #[repr(C)]
    struct CGPoint {
        x: f64,
        y: f64,
    }

    const MOUSE_MOVED: i32 = 5;
    const TAP_SESSION: u32 = 0;

    /// Posts a synthetic mouse-move into the session event stream.
    /// 向会话事件流注入一条合成鼠标移动。
    unsafe fn post_mouse_move(x: f64, y: f64) {
        // SAFETY: null source = current process; the returned CF object is
        // posted and intentionally leaked (fire-and-forget injection).
        // SAFETY: null source = 当前进程；返回的 CF 对象上屏后有意泄漏
        // （即发即忘注入）。
        let event = unsafe {
            CGEventCreateMouseEvent(std::ptr::null_mut(), MOUSE_MOVED, CGPoint { x, y }, 0)
        };
        // SAFETY: posting into the session tap.
        // SAFETY: 投递到会话 tap。
        unsafe { CGEventPost(TAP_SESSION, event) };
    }

    pub fn run() -> i32 {
        std::panic::set_hook(Box::new(|info| eprintln!("panic: {info}")));

        // 1. Window creation with the real backend.
        // 1. 经真实后端建窗。
        let window = match Window::new("starfish macos smoke", (320, 240), WindowFlags::default()) {
            Ok(w) => w,
            Err(err) => {
                eprintln!("window creation failed / 建窗失败: {err}");
                return 1;
            }
        };

        // 2. Title / size round-trip.
        // 2. 标题 / 尺寸往返。
        window.set_title("starfish macos smoke 2");
        if window.title() != "starfish macos smoke 2" {
            eprintln!("set_title did not round-trip / set_title 未生效");
            return 1;
        }
        let (w, h) = window.size();
        if w == 0 || h == 0 {
            eprintln!("window size is zero / 窗口尺寸为零: {w}x{h}");
            return 1;
        }

        // 3. Inject synthetic mouse moves AT the window's own location —
        //    mouse-moved events deliver to the window under the cursor, so
        //    the injection point must land inside our frame. CG global
        //    coords are top-left points; window.position() is physical
        //    top-left, so pts = physical ÷ dpi_scale + small inset.
        // 3. 在窗口自身位置注入合成鼠标移动——mouse-moved 事件派发给
        //    光标下的窗口，注入点必须落在窗口框内。CG 全局坐标为左上
        //    原点；window.position() 是物理左上，点 = 物理 ÷ dpi_scale
        //    再加少量内缩。
        let (px, py) = window.position();
        let scale = window.dpi_scale() as f64;
        let cg_x = px as f64 / scale + 60.0;
        let cg_y = py as f64 / scale + 60.0;
        unsafe {
            post_mouse_move(cg_x, cg_y);
            post_mouse_move(cg_x + 10.0, cg_y + 10.0);
        }

        let mut mouse_moves = 0;
        for _ in 0..200 {
            match window.poll_event() {
                Some(Event::MouseMoved { .. }) => mouse_moves += 1,
                Some(_) => {} // Other events observed but not asserted / 其余事件仅观察
                None => std::thread::sleep(Duration::from_millis(5)),
            }
        }
        if mouse_moves == 0 {
            // Non-fatal on CI: CGEventPost delivery depends on TCC /
            // activation state of the runner session. The deterministic
            // event verification below (push_event roundtrip) always runs.
            // CI 上非致命：CGEventPost 的送达依赖 runner 会话的 TCC /
            // 激活状态。下面的确定性事件验证（push_event 往返）总会执行。
            eprintln!(
                "info: no OS-level MouseMoved (non-fatal on CI) / 未收到 OS 级 MouseMoved（CI 上非致命）"
            );
        }

        // 4. Deterministic event injection: push_event → poll → Event::User
        //    (exercises the queue + poll dispatch path).
        // 4. 确定性事件注入：push_event → poll → Event::User
        //    （验证队列与 poll 派发路径）。
        window.push_event(42, "injected".to_string());
        let mut got_user = false;
        for _ in 0..64 {
            match window.poll_event() {
                Some(Event::User(user)) if user.code == 42 => {
                    got_user =
                        user.downcast_ref::<String>().map(String::as_str) == Some("injected");
                    break;
                }
                Some(_) => {}
                None => std::thread::sleep(Duration::from_millis(5)),
            }
        }
        if !got_user {
            eprintln!("push_event roundtrip failed / push_event 往返失败");
            return 1;
        }

        // 5. Raw handle must resolve to the AppKit variant.
        // 5. 裸句柄必须解析为 AppKit 变体。
        if !matches!(window.raw_handle(), RawWindowHandle::AppKit { .. }) {
            eprintln!("raw_handle is not AppKit / raw_handle 不是 AppKit");
            return 1;
        }

        println!("macOS smoke OK / macOS 冒烟通过");
        0
    }
}

fn main() {
    #[cfg(target_os = "macos")]
    {
        std::process::exit(macos::run());
    }
    #[cfg(not(target_os = "macos"))]
    {
        println!("skipped: macOS-only smoke / 非 macOS 平台跳过");
    }
}
