//! Web demo: window on a canvas, events printed onto an in-page overlay.
//! Web 演示：canvas 建窗 + 事件打印到页面覆盖层。
//!
//! Packaging: `scripts/build_web.sh` (build → wasm-bindgen → testweb/);
//! then serve testweb/ with any static server and test in a browser.
//! 打包：`scripts/build_web.sh`（构建 → wasm-bindgen → testweb/），
//! 之后用任意静态服务器伺服 testweb/ 即可在浏览器测试。
//!
//! The `setTimeout` recursion here plays the role of the "interpreter
//! tick": the caller drives the main loop and drains events each tick.
//! 这里的 `setTimeout` 递归就是"解释器 tick"的角色：
//! 调用者驱动主循环，每 tick 排空事件。
//!
//! Note: wasm-only code is `#[cfg]`-gated item by item, while `fn main`
//! must exist unconditionally (`cargo test` / clippy build this example
//! on the host).
//! 注意：wasm 专属代码逐项 `#[cfg]`，`fn main` 必须无条件存在
//! （`cargo test` / clippy 会在宿主上构建本示例）。

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use std::cell::RefCell;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use std::rc::Rc;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use wasm_bindgen::JsCast;
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use wasm_bindgen::prelude::*;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
use starfish_window::{Event, Window};

/// The real Web entry point; wasm-bindgen calls it via the `start` hook.
/// 真正的 Web 入口；wasm-bindgen 以 `start` 钩子调用它。
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
#[wasm_bindgen(start)]
fn start_demo() -> Result<(), JsValue> {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&format!("starfish panic: {info}").into());
    }));

    let js_window = web_sys::window().expect("无 window");
    let document = js_window.document().expect("无 document");

    // 事件日志覆盖层（pointer-events:none 不挡交互）。
    let log: web_sys::HtmlElement = document
        .create_element("pre")
        .expect("创建日志元素失败")
        .dyn_into()
        .expect("元素不是 HtmlElement");
    log.style().set_css_text(
        "position:fixed;top:8px;left:8px;margin:0;padding:8px;\
         background:rgba(0,0,0,.55);color:#7f7;font:12px/1.5 monospace;\
         pointer-events:none;max-width:90vw;max-height:50vh;overflow:hidden;\
         white-space:pre-wrap;z-index:10;",
    );
    document
        .body()
        .expect("无 body")
        .append_child(&log)
        .expect("挂载日志失败");

    let window = Window::builder()
        .title("starfish web demo")
        .size((800, 600))
        .build()
        .expect("建窗失败");
    log_line(
        &log,
        &format!(
            "窗口就绪 size={:?}；动一动键盘/鼠标/滚轮/窗口大小（RedrawRequested 已折叠不打印）",
            window.size()
        ),
    );

    // setTimeout(16ms) 递归 = 解释器 tick 的替身。
    let window = Rc::new(RefCell::new(window));
    thread_local! {
        static TICK: RefCell<Option<Closure<dyn FnMut()>>> = const { RefCell::new(None) };
    }

    let tick: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let tick_self = tick.clone();
    let log2 = log;
    let w2 = window;
    *tick.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let mut w = w2.borrow_mut();
        while let Some(event) = w.poll_event() {
            if matches!(event, Event::RedrawRequested) {
                continue; // rAF 每帧一条，刷屏故不打日志
            }
            let line = format!("{event:?}");
            web_sys::console::log_1(&format!("[starfish] {line}").into());
            log_line(&log2, &line);
        }
        drop(w);
        if let (Some(js), Some(cb)) = (web_sys::window(), tick_self.borrow().as_ref()) {
            let _ = js.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                16,
            );
        }
    }) as Box<dyn FnMut()>));

    if let (Some(js), Some(cb)) = (web_sys::window(), tick.borrow().as_ref()) {
        let _ = js
            .set_timeout_with_callback_and_timeout_and_arguments_0(cb.as_ref().unchecked_ref(), 16);
    }

    Ok(())
}

/// 追加一行日志，保留最近 30 行。
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
fn log_line(log: &web_sys::HtmlElement, line: &str) {
    let text = log.text_content().unwrap_or_default();
    let mut lines: Vec<&str> = text.lines().collect();
    lines.push(line);
    if lines.len() > 30 {
        lines.drain(..lines.len() - 30);
    }
    log.set_text_content(Some(&lines.join("\n")));
}

/// wasm32 上真正的入口是 `#[wasm_bindgen(start)] start_demo`；
/// 宿主目标（cargo test / clippy 会构建示例）只需要这个空 main。
fn main() {}
