//! macOS backend smoke test (main thread, via our AppKit backend):
//! window creation → title/size → CGEventPost-injected events → poll
//! dispatch verification.
//! macOS 后端冒烟测试（主线程，经本库 AppKit 后端）：
//! 建窗 → 标题/尺寸 → CGEventPost 注入事件 → poll 派发验证。
//!
//! `harness = false`: the AppKit backend is main-thread-only (see
//! tests/macos_probe.rs for why libtest can't provide that).
//! `harness = false`：AppKit 后端仅限主线程（原因见
//! tests/macos_probe.rs——libtest 无法提供主线程）。
//!
//! Event injection uses CGEventPost against the session's event stream —
//! synthetic events are indistinguishable from real ones downstream, so
//! this exercises the full backend path (pump → translate → queue).
//! 事件注入用 CGEventPost 打进会话事件流——合成事件与真实事件在后端
//! 视角完全一致，因此验证覆盖完整路径（泵 → 翻译 → 队列）。

#![cfg(target_os = "macos")]

use starfish_window::flags::WindowFlags;
use starfish_window::{Event, Window};

// CGEvent injection (ApplicationServices; minimal FFI surface for tests).
// CGEvent 注入（ApplicationServices；测试所需的最小 FFI 面）。
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGEventCreateMouseEvent(
        source: *mut std::ffi::c_void,
        mouse_type: i32,
        point: crate_null_pointers::CGPoint,
        button: i32,
    ) -> *mut std::ffi::c_void;
    fn CGEventPost(tap: u32, event: *mut std::ffi::c_void);
}

mod crate_null_pointers {
    #[repr(C)]
    pub struct CGPoint {
        pub x: f64,
        pub y: f64,
    }
}

const MOUSE_MOVED: i32 = 5;
const TAP_SESSION: u32 = 0;

/// Posts a synthetic mouse-move into the session event stream.
/// 向会话事件流注入一条合成鼠标移动。
unsafe fn post_mouse_move(x: f64, y: f64) {
    unsafe {
        let event = CGEventCreateMouseEvent(
            std::ptr::null_mut(),
            MOUSE_MOVED,
            crate_null_pointers::CGPoint { x, y },
            0,
        );
        CGEventPost(TAP_SESSION, event);
    }
}

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let mut failures: Vec<String> = Vec::new();

    // 1. Window creation with the real backend.
    // 1. 经真实后端建窗。
    let window = match Window::new("starfish macos smoke", (320, 240), WindowFlags::default()) {
        Ok(w) => w,
        Err(err) => {
            failures.push(format!("window creation failed / 建窗失败: {err}"));
            report(&failures);
            return 1;
        }
    };

    // 2. Title / size round-trip.
    // 2. 标题 / 尺寸往返。
    window.set_title("starfish macos smoke 2");
    if window.title() != "starfish macos smoke 2" {
        failures.push("set_title did not round-trip / set_title 未生效".into());
    }
    let (w, h) = window.size();
    if w == 0 || h == 0 {
        failures.push(format!("window size is zero / 窗口尺寸为零: {w}x{h}"));
    }

    // 3. Inject synthetic mouse moves and expect them back through the
    //    poll (full backend path: pump → translate → queue).
    // 3. 注入合成鼠标移动，并期望经 poll 原样返回（完整后端路径：
    //    泵 → 翻译 → 队列）。
    unsafe {
        post_mouse_move(150.0, 150.0);
        post_mouse_move(160.0, 160.0);
    }

    let mut mouse_moves = 0;
    for _ in 0..200 {
        match window.poll_event() {
            Some(Event::MouseMoved { .. }) => mouse_moves += 1,
            Some(_) => {}
            None => std::thread::sleep(std::time::Duration::from_millis(5)),
        }
    }
    if mouse_moves == 0 {
        failures.push("no MouseMoved events received / 未收到任何 MouseMoved".into());
    }

    // 4. Raw handle must resolve to the AppKit variant.
    // 4. 裸句柄必须解析为 AppKit 变体。
    if !matches!(
        window.raw_handle(),
        starfish_window::RawWindowHandle::AppKit { .. }
    ) {
        failures.push("raw_handle is not AppKit / raw_handle 不是 AppKit".into());
    }

    report(&failures);
    if failures.is_empty() { 0 } else { 1 }
}

fn report(failures: &[String]) {
    if failures.is_empty() {
        println!("macOS smoke OK / macOS 冒烟通过");
    } else {
        for f in failures {
            eprintln!("smoke failure / 冒烟失败项: {f}");
        }
    }
}
