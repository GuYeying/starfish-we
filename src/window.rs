//! 公共窗口类型：平台无关的壳，具体能力委托给当前平台的 `imp`。

use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::WindowError;
use crate::handle::RawWindowHandle;
use crate::platform::imp;

#[cfg(feature = "event")]
use std::any::Any;
#[cfg(feature = "event")]
use std::collections::VecDeque;
#[cfg(feature = "event")]
use std::sync::Mutex;
#[cfg(feature = "event")]
use std::sync::mpsc::{Receiver, Sender, channel};

#[cfg(feature = "event")]
use crate::event::{Event, UserEvent};

/// 进程内唯一的窗口。
///
/// - 创建：[`Window::new`]；drop 时销毁平台窗口。
/// - 事件：循环内调用 [`poll_event`](Window::poll_event) 逐个拉取（非阻塞）。
/// - 约束：同一时刻最多存在一个 `Window`，第二次 `new` 返回
///   [`WindowError::AlreadyExists`]。
/// - 线程亲和：窗口与其创建线程绑定（Win32 消息队列 / AppKit 主线程要求），
///   所有方法（含 `poll_event`）必须在创建线程上调用；
///   跨线程注入事件是唯一例外，走 [`EventProxy`]。
pub struct Window {
    inner: imp::WindowInner,
    /// 用户自定义事件通道的发送端（`push_event` / `EventProxy` 共享）。
    #[cfg(feature = "event")]
    user_tx: Sender<UserEvent>,
    /// 用户自定义事件通道的接收端（仅 `poll_event` 在创建线程消费）。
    /// 套 `Mutex` 只为保住 `Window: Sync`（wgpu `create_surface` 的 bound）；
    /// 锁实际无竞争——消费仅在创建线程。
    #[cfg(feature = "event")]
    user_rx: Mutex<Receiver<UserEvent>>,
    /// 已到达、待派发的用户事件（跨线程注入在 `poll_event` 中排空）。
    #[cfg(feature = "event")]
    user_pending: VecDeque<UserEvent>,
}

/// 单窗口不变量的全局占位；`Window` 构造成功期间为 `true`。
static WINDOW_ALIVE: AtomicBool = AtomicBool::new(false);

/// 重置单窗口占位（仅 Android 进程复用场景）。
///
/// Android 会缓存进程，新旧 Activity 实例可能在同一进程内短暂交叠；
/// 窗口生命周期按"运行"而非"进程"计算，新实例装配时由
/// `android_init` 调用此函数释放上一实例的占位。
#[cfg(target_os = "android")]
pub(crate) fn reset_single_window_slot() {
    WINDOW_ALIVE.store(false, Ordering::SeqCst);
}

impl Window {
    /// 创建一个标题为 `title`、客户区初始尺寸为 `size`（物理像素）、
    /// 带创建标志 `flags` 的窗口（SDL 风格快捷构造）。
    ///
    /// 等价于 [`WindowBuilder`] 的 `title` + `size` + `flags` + 其余默认值；
    /// 需要更多选项（无边框、尺寸范围、置顶等）用 [`Window::builder`]。
    pub fn new(
        title: &str,
        size: (u32, u32),
        flags: crate::flags::WindowFlags,
    ) -> Result<Self, WindowError> {
        Self::builder().title(title).size(size).flags(flags).build()
    }

    /// 返回窗口构造器（SDL flag 的 Rust 形态：类型化方法，见 [`WindowBuilder`]）。
    pub fn builder() -> WindowBuilder {
        WindowBuilder::new()
    }

    fn build(mut options: WindowOptions) -> Result<Self, WindowError> {
        if WINDOW_ALIVE.swap(true, Ordering::SeqCst) {
            return Err(WindowError::AlreadyExists);
        }
        // flags → options 翻译(flag 只增能力;HIDDEN/FULLSCREEN 等由后端消费)
        use crate::flags::WindowFlags as F;
        if options.flags.contains(F::RESIZABLE) {
            options.resizable = true;
        }
        if options.flags.contains(F::NOFRAME) {
            options.borderless = true;
        }
        if options.flags.contains(F::ALWAYS_ON_TOP) {
            options.always_on_top = true;
        }
        if options.flags.contains(F::HIDDEN) {
            options.hidden = true;
        }
        match imp::WindowInner::new(&options) {
            Ok(inner) => {
                #[cfg(feature = "event")]
                let (user_tx, user_rx) = channel();
                #[cfg(feature = "event")]
                return Ok(Self {
                    inner,
                    user_tx,
                    user_rx: Mutex::new(user_rx),
                    user_pending: VecDeque::new(),
                });
                #[cfg(not(feature = "event"))]
                Ok(Self { inner })
            }
            Err(err) => {
                WINDOW_ALIVE.store(false, Ordering::SeqCst);
                Err(err)
            }
        }
    }

    /// 拉取一个待处理事件；队列为空时返回 `None`（非阻塞，SDL 风格）。
    ///
    /// 内部会顺带把平台消息源泵干（Win32 `PeekMessageW`、macOS
    /// `nextEventMatchingMask` 等），调用者不需要、也不应自己碰平台循环。
    ///
    /// 已到达的用户自定义事件（[`push_event`](Window::push_event) /
    /// [`EventProxy`] 注入）先于本轮平台事件派发：`send` 之后下一次
    /// `poll_event` 必定可见；与平台事件的相对顺序不作全局 FIFO 承诺，
    /// 批量注入时按序逐条派发（期间平台事件短暂延迟、不丢失）。
    #[cfg(feature = "event")]
    pub fn poll_event(&mut self) -> Option<Event> {
        let rx = self.user_rx.lock().expect("用户事件锁中毒");
        while let Ok(user) = rx.try_recv() {
            self.user_pending.push_back(user);
        }
        drop(rx);
        if let Some(user) = self.user_pending.pop_front() {
            return Some(Event::User(user));
        }
        self.inner.poll_event()
    }

    /// 注入一条自定义事件（同线程便捷版；跨线程用
    /// [`event_proxy`](Window::event_proxy)）。经 `Event::User` 派发。
    #[cfg(feature = "event")]
    pub fn push_event(&self, code: u32, payload: impl Any + Send + Sync) {
        let _ = self.user_tx.send(UserEvent::new(code, payload));
    }

    /// 取一个跨线程事件代理：可 `Clone` 后交给解释器工作线程 / 任意任务，
    /// 在任意线程向主循环注入 [`Event::User`]。
    ///
    /// `Window` 销毁后 `send` 返回 `Err(EventProxyClosed)`。
    #[cfg(feature = "event")]
    pub fn event_proxy(&self) -> EventProxy {
        EventProxy {
            tx: self.user_tx.clone(),
        }
    }

    /// 当前客户区尺寸（物理像素）。
    pub fn size(&self) -> (u32, u32) {
        self.inner.size()
    }

    /// 设置窗口标题。
    pub fn set_title(&mut self, title: &str) {
        self.inner.set_title(title);
    }

    /// 取裸平台句柄，用于 unsafe 桥接渲染后端（wgpu / softbuffer / 原生 API）。
    ///
    /// 更进一步：启用 `wgpu` feature 后，`Window` 直接实现了
    /// raw-window-handle 的 trait，可零 unsafe 交给 wgpu。
    pub fn raw_handle(&self) -> RawWindowHandle {
        self.inner.raw_handle()
    }

    // ── 运行期窗口能力（SDL flag 语义；转发给平台后端）────────────

    /// 设置是否可由用户调整大小。
    pub fn set_resizable(&mut self, resizable: bool) {
        self.inner.set_resizable(resizable)
    }

    /// 是否可由用户调整大小。
    pub fn is_resizable(&self) -> bool {
        self.inner.is_resizable()
    }

    /// 设置客户区尺寸（物理像素）。
    pub fn set_size(&mut self, size: (u32, u32)) {
        self.inner.set_size(size.0, size.1)
    }

    /// 进入 / 退出无边框全屏（退出时还原进入前位置与尺寸）。
    pub fn set_fullscreen(&mut self, fullscreen: bool) {
        self.inner.set_fullscreen(fullscreen)
    }

    /// 是否处于全屏。
    pub fn is_fullscreen(&self) -> bool {
        self.inner.is_fullscreen()
    }

    /// 设置 / 取消无边框。
    pub fn set_borderless(&mut self, borderless: bool) {
        self.inner.set_borderless(borderless)
    }

    /// 是否无边框。
    pub fn is_borderless(&self) -> bool {
        self.inner.is_borderless()
    }

    /// 抓取 / 释放鼠标（限定在客户区内）。
    pub fn set_mouse_grabbed(&mut self, grabbed: bool) {
        self.inner.set_mouse_grabbed(grabbed)
    }

    /// 鼠标是否被抓取。
    pub fn is_mouse_grabbed(&self) -> bool {
        self.inner.is_mouse_grabbed()
    }

    /// 相对鼠标模式（近似实现：抓取 + 隐藏光标）。
    pub fn set_mouse_relative(&mut self, enable: bool) {
        self.inner.set_mouse_relative(enable)
    }

    /// 显示 / 隐藏光标。
    pub fn set_cursor_visible(&mut self, visible: bool) {
        self.inner.set_cursor_visible(visible)
    }

    /// 光标是否可见。
    pub fn is_cursor_visible(&self) -> bool {
        self.inner.is_cursor_visible()
    }

    /// 显示 / 隐藏窗口。
    pub fn set_visible(&mut self, visible: bool) {
        self.inner.set_visible(visible)
    }

    /// 窗口是否可见。
    pub fn is_visible(&self) -> bool {
        self.inner.is_visible()
    }

    /// 设置 / 取消窗口置顶。
    pub fn set_always_on_top(&mut self, top: bool) {
        self.inner.set_always_on_top(top)
    }

    /// 窗口是否置顶。
    pub fn is_always_on_top(&self) -> bool {
        self.inner.is_always_on_top()
    }

    /// 最小化窗口。
    pub fn minimize(&mut self) {
        self.inner.minimize()
    }

    /// 最大化窗口。
    pub fn maximize(&mut self) {
        self.inner.maximize()
    }

    /// 从最小化 / 最大化还原。
    pub fn restore(&mut self) {
        self.inner.restore()
    }

    /// 提升窗口到 Z 序顶部。
    pub fn raise(&mut self) {
        self.inner.raise()
    }

    /// 窗口是否最小化。
    pub fn is_minimized(&self) -> bool {
        self.inner.is_minimized()
    }

    /// 窗口是否最大化。
    pub fn is_maximized(&self) -> bool {
        self.inner.is_maximized()
    }

    /// 窗口是否持有输入焦点。
    pub fn has_input_focus(&self) -> bool {
        self.inner.has_input_focus()
    }

    /// 窗口是否被完全遮挡（Win32 无直接等价，恒 false）。
    pub fn is_occluded(&self) -> bool {
        self.inner.is_occluded()
    }

    /// 键盘抓取（Win32 无窗口级等价，恒 no-op）。
    pub fn set_keyboard_grabbed(&mut self, grab: bool) {
        self.inner.set_keyboard_grabbed(grab)
    }

    /// 设置窗口外框左上角位置（屏幕坐标，物理像素）。
    pub fn set_position(&mut self, position: (i32, i32)) {
        self.inner.set_position(position.0, position.1)
    }

    /// 窗口外框左上角位置（屏幕坐标，物理像素）。
    pub fn position(&self) -> (i32, i32) {
        self.inner.position()
    }

    /// 居中于所在显示器工作区。
    pub fn center_on_screen(&mut self) {
        self.inner.center_on_screen()
    }

    /// 当前标题。
    pub fn title(&self) -> String {
        self.inner.title()
    }

    /// 设置窗口不透明度（0.0~1.0；1.0 = 全不透明）。
    pub fn set_opacity(&mut self, opacity: f32) {
        self.inner.set_opacity(opacity)
    }

    /// 窗口不透明度。
    pub fn opacity(&self) -> f32 {
        self.inner.opacity()
    }

    /// DPI 缩放比（96 DPI = 1.0；Per-Monitor V2 感知）。
    pub fn dpi_scale(&self) -> f32 {
        self.inner.dpi_scale()
    }

    /// 高像素密度：进程声明 Per-Monitor V2，恒 true（SDL 同款 flag 语义）。
    pub fn high_pixel_density(&self) -> bool {
        self.inner.high_pixel_density()
    }

    /// （仅 wasm 构建）底层 `<canvas>` 元素。
    ///
    /// 平台特有 API 的显式例外：只存在于 wasm32 构建中，桌面 API 不受影响。
    /// Web 渲染（含 wgpu 的 Web 后端）以 canvas 为目标。
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    pub fn canvas(&self) -> web_sys::HtmlCanvasElement {
        self.inner.canvas()
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        // `inner` 自身负责销毁平台窗口，这里只释放全局占位。
        WINDOW_ALIVE.store(false, Ordering::SeqCst);
    }
}

/// 跨线程注入自定义事件的句柄（[`Window::event_proxy`] 的产物）。
///
/// `Clone` 后可交给解释器工作线程 / 任意任务；`Window` 销毁后
/// [`send`](EventProxy::send) 返回 `Err(EventProxyClosed)`。
/// 这是窗口 API 唯一的跨线程入口——`Window` 本身的方法仍限创建线程。
#[derive(Clone)]
pub struct EventProxy {
    #[cfg(feature = "event")]
    tx: Sender<UserEvent>,
}

impl EventProxy {
    /// 向主循环注入一条自定义事件，经 `Event::User` 派发。
    ///
    /// 任意线程可调用；事件在下一次 `poll_event` 可见。
    /// 载荷即发即弃（通道满 / 窗口已销毁时丢弃并返回 `Err`）。
    #[cfg(feature = "event")]
    pub fn send(&self, code: u32, payload: impl Any + Send + Sync) -> Result<(), EventProxyClosed> {
        self.tx
            .send(UserEvent::new(code, payload))
            .map_err(|_| EventProxyClosed)
    }
}

impl std::fmt::Debug for EventProxy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventProxy").finish_non_exhaustive()
    }
}

/// 事件代理已失效（窗口已销毁），事件未入队。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EventProxyClosed;

impl std::fmt::Display for EventProxyClosed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "事件代理已失效（窗口已销毁）")
    }
}

impl std::error::Error for EventProxyClosed {}

/// 平台后端拿到的解析后构造参数（由 [`WindowBuilder`] 归集）。
/// 新增选项 = 新增字段；对 builder 用户非破坏性。
#[derive(Debug, Clone, Default)]
pub(crate) struct WindowOptions {
    pub(crate) title: String,
    /// 客户区初始尺寸（物理像素）。
    pub(crate) size: (u32, u32),
    pub(crate) resizable: bool,
    pub(crate) borderless: bool,
    pub(crate) min_size: Option<(u32, u32)>,
    pub(crate) max_size: Option<(u32, u32)>,
    pub(crate) always_on_top: bool,
    /// 创建标志（SDL 语义；后端翻译为平台行为）。
    pub(crate) flags: crate::flags::WindowFlags,
    /// 创建即可见 = false（HIDDEN flag 或 builder 显式设置）。
    pub(crate) hidden: bool,
}

/// 窗口构造器。新增窗口选项 = 新增方法，对既有调用者非破坏性。
///
/// ```ignore
/// let window = Window::builder()
///     .title("demo")
///     .size((800, 600))
///     .resizable(false)
///     .borderless(true)
///     .min_size((400, 300))
///     .build()?;
/// ```
#[derive(Debug, Clone, Default)]
pub struct WindowBuilder {
    options: WindowOptions,
}

impl WindowBuilder {
    fn new() -> Self {
        Self::default()
    }

    /// 窗口标题。默认为空。
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.options.title = title.into();
        self
    }

    /// 客户区初始尺寸（物理像素）。默认 `(800, 600)`。
    pub fn size(mut self, size: (u32, u32)) -> Self {
        self.options.size = size;
        self
    }

    /// 是否可由用户调整大小（含最大化按钮）。默认 `true`。
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.options.resizable = resizable;
        self
    }

    /// 无边框（无系统标题栏与边框）。默认 `false`。
    pub fn borderless(mut self, borderless: bool) -> Self {
        self.options.borderless = borderless;
        self
    }

    /// 客户区最小尺寸（物理像素），约束用户拖拽调整。默认不限制。
    pub fn min_size(mut self, size: (u32, u32)) -> Self {
        self.options.min_size = Some(size);
        self
    }

    /// 客户区最大尺寸（物理像素），约束用户拖拽调整。默认不限制。
    pub fn max_size(mut self, size: (u32, u32)) -> Self {
        self.options.max_size = Some(size);
        self
    }

    /// 窗口置顶。默认 `false`。
    pub fn always_on_top(mut self, always_on_top: bool) -> Self {
        self.options.always_on_top = always_on_top;
        self
    }

    /// 创建标志（SDL 语义位组合；与各 setter 叠加时 flag 仅定初始状态）。
    pub fn flags(mut self, flags: crate::flags::WindowFlags) -> Self {
        self.options.flags = flags;
        self
    }

    /// 创建窗口。受单窗口约束（见 [`Window::new`]）。
    pub fn build(self) -> Result<Window, WindowError> {
        Window::build(self.options)
    }
}

#[cfg(all(test, feature = "window", not(windows)))]
mod tests {
    use super::*;

    // Windows 后端已实现，见 tests/windows_smoke.rs；
    // 其余平台仍是 stub，断言占位行为，落地后逐平台替换。
    #[test]
    fn stub_backends_report_unsupported() {
        assert!(matches!(
            Window::new("t", (1, 1), WindowFlags::default()),
            Err(WindowError::UnsupportedPlatform)
        ));
    }
}
