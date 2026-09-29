//! Web (wasm32-unknown-unknown) 后端。
//!
//! 浏览器是回调驱动的，与 poll 模型相反：DOM 事件监听器把事件
//! **缓冲进队列**，`poll_event` 排空；主循环节奏由调用者驱动
//! （解释器 tick / `setTimeout`）。`requestAnimationFrame` 循环只置
//! 一个重绘标志（合并，不堆积），使 wgpu 用户每帧轮询到一次
//! `RedrawRequested`。
//!
//! 窗口实体是 `<canvas>`：按 id `starfish-window` 查找，其次取页面
//! 第一个 canvas，都没有则自建并铺满页面。
//! 坐标按 `devicePixelRatio` 换算为物理像素（与桌面约定一致）。
//! 键盘走 W3C `KeyboardEvent.code`，与 [`KeyCode`] 命名同源，直接映射。
//! 页面关闭即结束，Web 没有 `CloseRequested` 语义。

use std::cell::RefCell;
use std::rc::Rc;

#[cfg(feature = "event")]
use std::collections::VecDeque;

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Document, HtmlCanvasElement, KeyboardEvent, MouseEvent, WheelEvent};

use crate::error::WindowError;
#[cfg(feature = "event")]
use crate::event::{Event, KeyCode, Modifiers, MouseButton, TouchPhase};
use crate::handle::RawWindowHandle;
use crate::window::WindowOptions;

thread_local! {
    /// 单窗口（wasm 单线程）：存活窗口的状态。
    static STATE: RefCell<Option<Rc<WebState>>> = const { RefCell::new(None) };
    /// rAF 重绘循环的自引用句柄；drop 即停止调度。
    static FRAME_LOOP: RefCell<Option<Closure<dyn FnMut()>>> = const { RefCell::new(None) };
}

/// DOM 闭包共享的上下文（画布 + 事件通道）。
/// 闭包只捕获它，不捕获 `WebState`，避免 Rc 循环。
#[cfg(feature = "event")]
#[derive(Clone)]
struct Ctx {
    canvas: HtmlCanvasElement,
    shared: Rc<RefCell<Shared>>,
}

struct WebState {
    canvas: HtmlCanvasElement,
    /// DOM 回调与 `poll_event` 之间的事件通道。
    #[cfg(feature = "event")]
    shared: Rc<RefCell<Shared>>,
    /// 存活期内持有监听器闭包；drop 即解除监听。
    #[cfg(feature = "event")]
    _listeners: Vec<Closure<dyn FnMut(JsValue)>>,
}

#[cfg(feature = "event")]
struct Shared {
    /// DOM 回调翻译出的待发事件。
    pending: VecDeque<Event>,
    /// rAF 置位、poll 消费的重绘请求（合并去重）。
    redraw: bool,
}

pub(crate) struct WindowInner {
    state: Rc<WebState>,
}

impl WindowInner {
    pub(crate) fn new(options: &WindowOptions) -> Result<Self, WindowError> {
        let document = Self::document()?;
        let canvas = Self::acquire_canvas(&document)?;
        canvas.set_tab_index(0);

        Self::sync_canvas_size(&canvas); // 返回值无关紧要：首同步必装配尺寸

        #[cfg_attr(not(feature = "event"), allow(unused_mut))]
        let mut state = Rc::new(WebState {
            canvas: canvas.clone(),
            #[cfg(feature = "event")]
            shared: Rc::new(RefCell::new(Shared {
                pending: VecDeque::new(),
                redraw: false,
            })),
            #[cfg(feature = "event")]
            _listeners: Vec::new(),
        });

        // 闭包只捕获 Ctx（canvas + 通道），因此可以先挂监听再回填 state。
        #[cfg(feature = "event")]
        let listeners = Self::attach_listeners(&Ctx {
            canvas: canvas.clone(),
            shared: state.shared.clone(),
        })?;
        #[cfg(feature = "event")]
        if let Some(inner) = Rc::get_mut(&mut state) {
            inner._listeners = listeners;
        }

        if STATE.with_borrow_mut(|slot| slot.replace(state).is_some()) {
            // Window::build 的全局守卫已挡住第二窗口；此处仅兜底。
            return Err(WindowError::AlreadyExists);
        }
        #[cfg(feature = "event")]
        Self::start_frame_loop(&document);
        let _ = options; // 标题走 document.title 语义，尺寸以画布为准
        Ok(Self {
            // 上面 slot.replace 消耗了 Rc，这里重新取回。
            state: STATE.with_borrow(|slot| slot.clone().expect("STATE 刚刚写入")),
        })
    }

    #[cfg(feature = "event")]
    pub(crate) fn poll_event(&mut self) -> Option<Event> {
        let mut shared = self.state.shared.borrow_mut();
        if let Some(event) = shared.pending.pop_front() {
            return Some(event);
        }
        if shared.redraw {
            shared.redraw = false;
            return Some(Event::RedrawRequested);
        }
        None
    }

    pub(crate) fn size(&self) -> (u32, u32) {
        Self::measure(&self.state.canvas)
    }

    pub(crate) fn set_title(&mut self, title: &str) {
        if let Ok(document) = Self::document() {
            document.set_title(title);
        }
    }

    pub(crate) fn raw_handle(&self) -> RawWindowHandle {
        // canvas 元素的 JsValue 对象指针（wgpu Web 后端 WebCanvas 路径同源）
        let canvas: &HtmlCanvasElement = &self.state.canvas;
        let js: &JsValue = canvas;
        RawWindowHandle::Web {
            canvas: js as *const JsValue as *mut std::ffi::c_void,
        }
    }

    // ── 运行期窗口能力（平台矩阵：Web 列）────────────────────────
    // 浏览器做不到的 = warn-once no-op；能做的直接映射 DOM/canvas。
    // warn-once 用 thread_local 集合（每 key 一次，不随帧刷屏）。

    fn warn_once(key: &'static str, msg: &str) {
        thread_local! {
            static WARNED: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
        }
        WARNED.with(|set| {
            let mut set = set.borrow_mut();
            if !set.contains(&key) {
                set.push(key);
                web_sys::console::warn_1(&wasm_bindgen::JsValue::from_str(msg));
            }
        });
    }

    /// 样式写入辅助（canvas.style）
    fn set_canvas_style(&self, prop: &str, value: &str) {
        self.state.canvas.style().set_property(prop, value).ok();
    }

    pub(crate) fn set_resizable(&mut self, _resizable: bool) {
        // 浏览器窗口尺寸由用户/页面管理：canvas 不提供 resize 把手
        Self::warn_once(
            "resizable",
            "[starfish-window] web: RESIZABLE 由页面布局决定,忽略",
        );
    }

    pub(crate) fn is_resizable(&self) -> bool {
        false
    }

    /// 设置客户区尺寸（物理像素语义，与桌面约定一致）。
    ///
    /// 直接写两份：属性 = 物理像素，内联样式 = 物理/dpr 的 CSS 像素。
    /// 不走 [`Self::sync_canvas_size`]（那会读布局再乘 dpr，物理参数
    /// 被二次缩放）。
    pub(crate) fn set_size(&mut self, width: u32, height: u32) {
        let dpr = self.dpi_scale() as f64;
        let style = self.state.canvas.style();
        style
            .set_property("width", &format!("{}px", (width as f64 / dpr).round().max(1.0)))
            .ok();
        style
            .set_property("height", &format!("{}px", (height as f64 / dpr).round().max(1.0)))
            .ok();
        self.state.canvas.set_width(width.max(1));
        self.state.canvas.set_height(height.max(1));
    }

    /// 全屏 v0 no-op（Element::requestFullscreen 接入为后续项）
    pub(crate) fn set_fullscreen(&mut self, _fullscreen: bool) {
        Self::warn_once(
            "fullscreen",
            "[starfish-window] web: FULLSCREEN v0 暂不支持(后续接 requestFullscreen)",
        );
    }

    pub(crate) fn is_fullscreen(&self) -> bool {
        false
    }

    pub(crate) fn set_borderless(&mut self, _borderless: bool) {
        // 浏览器窗口无装饰概念：no-op
    }

    pub(crate) fn is_borderless(&self) -> bool {
        false
    }

    /// 鼠标抓取 v0 no-op（Pointer Lock API 接入为后续项）
    pub(crate) fn set_mouse_grabbed(&mut self, grabbed: bool) {
        Self::warn_once(
            "mouse_grabbed",
            "[starfish-window] web: MOUSE_GRABBED v0 暂不支持(后续接 Pointer Lock)",
        );
        let _ = grabbed;
    }

    pub(crate) fn is_mouse_grabbed(&self) -> bool {
        false
    }

    pub(crate) fn set_mouse_relative(&mut self, _enable: bool) {
        Self::warn_once(
            "mouse_relative",
            "[starfish-window] web: 相对鼠标 v0 暂不支持",
        );
    }

    pub(crate) fn set_cursor_visible(&mut self, visible: bool) {
        self.set_canvas_style("cursor", if visible { "default" } else { "none" });
    }

    pub(crate) fn is_cursor_visible(&self) -> bool {
        self.state
            .canvas
            .style()
            .get_property_value("cursor")
            .map(|v| v != "none")
            .unwrap_or(true)
    }

    pub(crate) fn set_visible(&self, visible: bool) {
        self.set_canvas_style("visibility", if visible { "visible" } else { "hidden" });
    }

    pub(crate) fn is_visible(&self) -> bool {
        self.state
            .canvas
            .style()
            .get_property_value("visibility")
            .map(|v| v != "hidden")
            .unwrap_or(true)
    }

    pub(crate) fn set_always_on_top(&self, _top: bool) {
        Self::warn_once(
            "always_on_top",
            "[starfish-window] web: ALWAYS_ON_TOP 不适用,忽略",
        );
    }

    pub(crate) fn is_always_on_top(&self) -> bool {
        false
    }

    pub(crate) fn minimize(&mut self) {
        Self::warn_once("minimize", "[starfish-window] web: minimize 不适用,忽略");
    }

    pub(crate) fn maximize(&mut self) {
        Self::warn_once("maximize", "[starfish-window] web: maximize 不适用,忽略");
    }

    pub(crate) fn restore(&mut self) {}

    pub(crate) fn raise(&mut self) {
        Self::warn_once("raise", "[starfish-window] web: raise 不适用,忽略");
    }

    pub(crate) fn is_minimized(&self) -> bool {
        // 页面隐藏 ≈ 最小化（visibilitychange 语义）
        web_sys::window()
            .and_then(|w| w.document())
            .map(|d| d.visibility_state() == web_sys::VisibilityState::Hidden)
            .unwrap_or(false)
    }

    pub(crate) fn is_maximized(&self) -> bool {
        false
    }

    pub(crate) fn has_input_focus(&self) -> bool {
        web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.has_focus().ok())
            .unwrap_or(false)
    }

    pub(crate) fn center_on_screen(&mut self) {
        // 浏览器管理窗口位置：no-op
    }

    pub(crate) fn is_occluded(&self) -> bool {
        false
    }

    pub(crate) fn set_keyboard_grabbed(&mut self, _grab: bool) {}

    pub(crate) fn set_position(&mut self, _x: i32, _y: i32) {
        Self::warn_once("position", "[starfish-window] web: 窗口定位不适用,忽略");
    }

    pub(crate) fn position(&self) -> (i32, i32) {
        (0, 0)
    }

    pub(crate) fn title(&self) -> String {
        Self::document().map(|d| d.title()).unwrap_or_default()
    }

    pub(crate) fn set_opacity(&mut self, opacity: f32) {
        let v = opacity.clamp(0.0, 1.0);
        self.set_canvas_style("opacity", &format!("{v}"));
    }

    pub(crate) fn opacity(&self) -> f32 {
        self.state
            .canvas
            .style()
            .get_property_value("opacity")
            .ok()
            .and_then(|v| v.trim().parse::<f32>().ok())
            .unwrap_or(1.0)
    }

    /// DPI 缩放比（devicePixelRatio；物理像素约定的基准）
    pub(crate) fn dpi_scale(&self) -> f32 {
        web_sys::window()
            .map(|w| w.device_pixel_ratio() as f32)
            .unwrap_or(1.0)
    }

    /// 高像素密度：物理像素坐标以 dpr 为基准，恒 true
    pub(crate) fn high_pixel_density(&self) -> bool {
        true
    }

    /// 底层 `<canvas>`（仅 wasm 构建，供 Web 渲染路径使用）。
    pub(crate) fn canvas(&self) -> HtmlCanvasElement {
        self.state.canvas.clone()
    }
}

impl Drop for WindowInner {
    fn drop(&mut self) {
        STATE.with_borrow_mut(|slot| *slot = None);
        FRAME_LOOP.with_borrow_mut(|cb| *cb = None);
    }
}

impl WindowInner {
    fn document() -> Result<Document, WindowError> {
        web_sys::window()
            .and_then(|w| w.document())
            .ok_or_else(|| WindowError::OsError("浏览器环境不可用（无 document）".into()))
    }

    fn acquire_canvas(document: &Document) -> Result<HtmlCanvasElement, WindowError> {
        let found = document
            .get_element_by_id("starfish-window")
            .or_else(|| document.query_selector("canvas").ok().flatten());
        let canvas: HtmlCanvasElement = match found {
            Some(el) => el.dyn_into().map_err(|_| {
                WindowError::OsError("id=starfish-window 的元素不是 <canvas>".into())
            })?,
            None => {
                let canvas: HtmlCanvasElement = document
                    .create_element("canvas")
                    .map_err(|_| WindowError::OsError("创建 canvas 失败".into()))?
                    .dyn_into()
                    .map_err(|_| WindowError::OsError("元素不是 canvas".into()))?;
                canvas
                    .style()
                    .set_css_text("position:fixed;inset:0;width:100vw;height:100vh;display:block;");
                let body = document
                    .body()
                    .ok_or_else(|| WindowError::OsError("无 body 元素".into()))?;
                body.append_child(&canvas)
                    .map_err(|_| WindowError::OsError("挂载 canvas 失败".into()))?;
                canvas
            }
        };
        Ok(canvas)
    }

    /// CSS 像素 × devicePixelRatio → 物理像素（至少 1，避免 0 尺寸 surface）。
    fn measure(canvas: &HtmlCanvasElement) -> (u32, u32) {
        let dpr = web_sys::window()
            .map(|w| w.device_pixel_ratio())
            .unwrap_or(1.0);
        let scale = |css: i32| ((css as f64 * dpr).round() as u32).max(1);
        (scale(canvas.client_width()), scale(canvas.client_height()))
    }

    /// 把画布分辨率（width/height 属性）同步为 CSS 尺寸 × dpr。
    ///
    /// 返回属性是否真的变化（供 resize 监听去重）。
    ///
    /// **必须同时钉住内联 CSS 尺寸**（仅当内联样式未定宽高时）：canvas
    /// 无 CSS 尺寸时布局尺寸由 width/height 属性决定，只写属性不钉样式
    /// 会形成正反馈——下一轮 measure() 读到的 client 尺寸已是放大后的
    /// 属性值，再乘 dpr 写回属性，每次 resize 事件放大 dpr 倍。实测
    /// DevTools 停靠 / 窗口缩放连发 resize 时 14 轮即 22763×17073，
    /// 爆掉 max texture limit（8192）后 depth/swapchain 全链失效、渲染
    /// 永久黑屏。钉住样式后属性只管渲染密度（物理像素），样式只管布局
    /// 尺寸（CSS 像素），二者解耦、循环消除。
    fn sync_canvas_size(canvas: &HtmlCanvasElement) -> bool {
        let dpr = web_sys::window()
            .map(|w| w.device_pixel_ratio())
            .unwrap_or(1.0);
        let style = canvas.style();
        // 内联样式已定宽高（如自建画布的 100vw/100vh）= 布局与属性已解耦，
        // 不覆盖（保留页面的响应式布局语义）。
        let inline = |prop: &str| {
            style
                .get_property_value(prop)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
        };
        let rect = canvas.get_bounding_client_rect();
        let css_w = rect.width().round().max(1.0);
        let css_h = rect.height().round().max(1.0);
        if !inline("width") {
            style
                .set_property("width", &format!("{css_w}px"))
                .ok();
        }
        if !inline("height") {
            style
                .set_property("height", &format!("{css_h}px"))
                .ok();
        }
        let width = ((css_w * dpr).round() as u32).max(1);
        let height = ((css_h * dpr).round() as u32).max(1);
        let changed = canvas.width() != width || canvas.height() != height;
        canvas.set_width(width);
        canvas.set_height(height);
        changed
    }

    /// rAF 循环：每帧置一次重绘标志。自引用句柄放在 [`FRAME_LOOP`]，
    /// 窗口 drop 时清空即停摆（闭包不持有自身，无泄漏）。
    #[cfg(feature = "event")]
    fn start_frame_loop(document: &Document) -> () {
        let _ = document;
        let Some(js_window) = web_sys::window() else {
            return;
        };
        let shared = STATE.with_borrow(|slot| slot.as_ref().expect("STATE 已写入").shared.clone());
        let closure = Closure::wrap(Box::new(move || {
            shared.borrow_mut().redraw = true;
            if let Some(w) = web_sys::window() {
                FRAME_LOOP.with_borrow(|cb| {
                    if let Some(cb) = cb.as_ref() {
                        let _ = w.request_animation_frame(cb.as_ref().unchecked_ref());
                    }
                });
            }
        }) as Box<dyn FnMut()>);
        let _ = js_window.request_animation_frame(closure.as_ref().unchecked_ref());
        FRAME_LOOP.with_borrow_mut(|slot| *slot = Some(closure));
    }

    // ---------------- 事件监听与翻译（仅 event feature） ----------------

    #[cfg(feature = "event")]
    fn attach_listeners(ctx: &Ctx) -> Result<Vec<Closure<dyn FnMut(JsValue)>>, WindowError> {
        let js_window =
            web_sys::window().ok_or_else(|| WindowError::OsError("浏览器环境不可用".into()))?;
        let document = Self::document()?;
        let canvas: &web_sys::EventTarget = &ctx.canvas;
        let mut listeners = Vec::new();

        // 键盘（window 级）。
        for (ty, down) in [("keydown", true), ("keyup", false)] {
            let ctx = ctx.clone();
            listeners.push(add_listener(js_window.as_ref(), ty, move |v| {
                on_key(&ctx, v, down)
            })?);
        }
        // 鼠标（canvas 级）。
        for ty in ["mousedown", "mousemove", "mouseup"] {
            let ctx = ctx.clone();
            listeners.push(add_listener(canvas, ty, move |v| on_mouse(&ctx, v))?);
        }
        // 滚轮。
        {
            let ctx = ctx.clone();
            listeners.push(add_listener(canvas, "wheel", move |v| on_wheel(&ctx, v))?);
        }
        // 焦点。
        for (ty, gained) in [("focus", true), ("blur", false)] {
            let ctx = ctx.clone();
            listeners.push(add_listener(canvas, ty, move |_| {
                ctx.shared.borrow_mut().pending.push_back(if gained {
                    Event::FocusGained
                } else {
                    Event::FocusLost
                });
            })?);
        }
        // 触摸（含主手指合成鼠标）。
        for ty in ["touchstart", "touchmove", "touchend", "touchcancel"] {
            let ctx = ctx.clone();
            listeners.push(add_listener(canvas, ty, move |v| on_touch(&ctx, v, ty))?);
        }
        // 窗口尺寸变化：先同步画布分辨率，物理尺寸真的变了才上报
        // Resized（DevTools 停靠/浏览器缩放常触发同尺寸 resize，不去重
        // 会让应用侧做无意义的 surface 重建）。
        {
            let ctx = ctx.clone();
            listeners.push(add_listener(js_window.as_ref(), "resize", move |_| {
                if Self::sync_canvas_size(&ctx.canvas) {
                    let (width, height) = Self::measure(&ctx.canvas);
                    ctx.shared
                        .borrow_mut()
                        .pending
                        .push_back(Event::Resized { width, height });
                }
            })?);
        }
        // 页面可见性 → 生命周期。
        {
            let ctx = ctx.clone();
            listeners.push(add_listener(
                document.as_ref(),
                "visibilitychange",
                move |_| {
                    let hidden = Self::document()
                        .is_ok_and(|d| d.visibility_state() == web_sys::VisibilityState::Hidden);
                    ctx.shared.borrow_mut().pending.push_back(if hidden {
                        Event::Suspended
                    } else {
                        Event::Resumed
                    });
                },
            )?);
        }

        Ok(listeners)
    }
}

#[cfg(feature = "event")]
fn add_listener<F>(
    target: &web_sys::EventTarget,
    ty: &str,
    handler: F,
) -> Result<Closure<dyn FnMut(JsValue)>, WindowError>
where
    F: FnMut(JsValue) + 'static,
{
    let mut handler = handler;
    let ty_owned = ty.to_string();
    let closure = Closure::wrap(Box::new(move |v| {
        web_sys::console::log_1(&format!("[listener] {ty_owned}").into());
        handler(v)
    }) as Box<dyn FnMut(JsValue)>);
    target
        .add_event_listener_with_callback(ty, closure.as_ref().unchecked_ref())
        .map_err(|_| WindowError::OsError(format!("addEventListener({ty}) 失败")))?;
    Ok(closure)
}

/// 事件坐标：CSS 客户坐标 − canvas 原点，再乘 dpr → 物理像素。
#[cfg(feature = "event")]
fn canvas_coords(ctx: &Ctx, client_x: f64, client_y: f64) -> (f64, f64) {
    let rect = ctx.canvas.get_bounding_client_rect();
    let dpr = web_sys::window()
        .map(|w| w.device_pixel_ratio())
        .unwrap_or(1.0);
    (
        (client_x - rect.left()) * dpr,
        (client_y - rect.top()) * dpr,
    )
}

#[cfg(feature = "event")]
fn on_key(ctx: &Ctx, value: JsValue, down: bool) {
    let Some(ev) = value.dyn_ref::<KeyboardEvent>() else {
        return;
    };
    if ev.is_composing() {
        return; // IME 组合过程首版不接（约定），只收最终文本。
    }
    let modifiers = Modifiers {
        shift: ev.shift_key(),
        ctrl: ev.ctrl_key(),
        alt: ev.alt_key(),
        meta: ev.meta_key(),
    };
    // 未识别的 code 不携带更多信息（Other 只收原生数字键码）。
    let key = KeyCode::from_w3c_code(&ev.code()).unwrap_or(KeyCode::Other(0));
    let mut shared = ctx.shared.borrow_mut();
    if down {
        // 可打印单字符 → TextInput（W3C `key`；无 IME 组合）。
        let text = ev.key();
        let mut chars = text.chars();
        if let (Some(c), None) = (chars.next(), chars.next()) {
            if !c.is_control() {
                shared.pending.push_back(Event::TextInput {
                    text: c.to_string(),
                });
            }
        }
        shared.pending.push_back(Event::KeyDown {
            key,
            modifiers,
            repeat: ev.repeat(),
        });
    } else {
        shared.pending.push_back(Event::KeyUp { key, modifiers });
    }
}

#[cfg(feature = "event")]
fn on_mouse(ctx: &Ctx, value: JsValue) {
    let Some(ev) = value.dyn_ref::<MouseEvent>() else {
        return;
    };
    let (x, y) = canvas_coords(ctx, ev.client_x() as f64, ev.client_y() as f64);
    let mut shared = ctx.shared.borrow_mut();
    match ev.type_().as_str() {
        "mousemove" => shared.pending.push_back(Event::MouseMoved { x, y }),
        "mousedown" | "mouseup" => {
            // W3C button：0 左 / 1 中 / 2 右 / 3 后退 / 4 前进。
            let button = match ev.button() {
                0 => MouseButton::Left,
                1 => MouseButton::Middle,
                2 => MouseButton::Right,
                3 => MouseButton::Back,
                4 => MouseButton::Forward,
                n => MouseButton::Other(n as u8),
            };
            let event = if ev.type_() == "mousedown" {
                Event::MouseDown { button }
            } else {
                Event::MouseUp { button }
            };
            shared.pending.push_back(event);
        }
        _ => {}
    }
}

#[cfg(feature = "event")]
fn on_wheel(ctx: &Ctx, value: JsValue) {
    let Some(ev) = value.dyn_ref::<WheelEvent>() else {
        return;
    };
    // deltaMode：0 像素 / 1 行 / 2 页，统一折算成像素量纲。
    let unit = match ev.delta_mode() {
        1 => 16.0,
        2 => web_sys::window()
            .and_then(|w| w.inner_height().ok())
            .and_then(|h| h.as_f64())
            .unwrap_or(800.0),
        _ => 1.0,
    };
    // 浏览器滚动向下为正；本 crate 约定正值向上，取反。
    ctx.shared
        .borrow_mut()
        .pending
        .push_back(Event::MouseWheel {
            delta_x: -ev.delta_x() * unit,
            delta_y: -ev.delta_y() * unit,
        });
}

#[cfg(feature = "event")]
fn on_touch(ctx: &Ctx, value: JsValue, ty: &str) {
    let Some(ev) = value.dyn_ref::<web_sys::TouchEvent>() else {
        return;
    };
    let phase = match ty {
        "touchstart" => TouchPhase::Started,
        "touchmove" => TouchPhase::Moved,
        "touchend" => TouchPhase::Ended,
        _ => TouchPhase::Cancelled,
    };
    let touches = ev.changed_touches();
    let mut shared = ctx.shared.borrow_mut();
    for i in 0..touches.length() {
        let Some(t) = touches.get(i) else {
            continue;
        };
        let (x, y) = canvas_coords(ctx, t.client_x() as f64, t.client_y() as f64);
        let finger = t.identifier() as u64;
        shared.pending.push_back(Event::Touch {
            phase,
            finger,
            x,
            y,
        });
        // 主手指（identifier 0）合成鼠标事件（SDL 惯例）。
        if t.identifier() == 0 {
            let mouse = match phase {
                TouchPhase::Started => Event::MouseDown {
                    button: MouseButton::Left,
                },
                TouchPhase::Moved => Event::MouseMoved { x, y },
                TouchPhase::Ended | TouchPhase::Cancelled => Event::MouseUp {
                    button: MouseButton::Left,
                },
            };
            shared.pending.push_back(mouse);
        }
    }
}
