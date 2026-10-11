//! Linux backend — **Wayland-only** (X11 dropped by design decision).
//! Linux 后端——**仅 Wayland**（设计决策放弃 X11）。
//!
//! Built on `smithay-client-toolkit` 0.21 + `wayland-client` 0.31 (system
//! + dlopen backend: real `wl_display*`/`wl_surface*` pointers for wgpu,
//! no link-time libwayland). Same inversion as the other OS-owned-entry
//! platforms: Wayland callbacks translate into a buffer queue that
//! `poll_event` drains non-blockingly.
//! 基于 `smithay-client-toolkit` 0.21 + `wayland-client` 0.31（system +
//! dlopen 后端：为 wgpu 提供真实 `wl_display*`/`wl_surface*` 指针，免链接期
//! libwayland 依赖）。与"系统掌管入口"类平台相同的反转：Wayland 回调把
//! 事件翻译进缓冲队列，`poll_event` 非阻塞排空。
//!
//! sctk 0.21 wiring (the 0.21 breaking change: all per-module
//! `delegate_*!` macros were replaced by one `delegate_dispatch2!`):
//! sctk 0.21 装配（0.21 破坏性变更：各模块 `delegate_*!` 宏合并为单一的
//! `delegate_dispatch2!`）：
//! `delegate_registry!` + `impl ProvidesRegistryState` +
//! `delegate_dispatch2!` — see the bottom of this file.

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_registry,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers, RepeatInfo},
        pointer::{PointerEvent, PointerHandler},
        Capability, SeatHandler, SeatState,
    },
    shell::xdg::{
        window::{Window as SctkWindow, WindowConfigure, WindowHandler},
        XdgShell,
    },
    shm::{slot::Buffer, slot::SlotPool, Shm, ShmHandler},
};
use wayland_client::{
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_surface},
    Connection, QueueHandle,
};

// ── Wayland state ───────────────────────────────────────────────────

/// Everything the sctk handlers touch. Handlers run inside
/// `dispatch_pending(&mut WaylandState)` on the creating thread, so the
/// per-event buffer queue lives right here (no static needed — unlike
/// the Android backend, there are no cross-thread callbacks).
/// sctk handler 触及的全部状态。handler 都在创建线程的
/// `dispatch_pending(&mut WaylandState)` 内运行，所以逐事件缓冲队列直接
/// 放这里（无需 static——与 Android 后端不同，这里没有跨线程回调）。
#[allow(dead_code)] // spike: bodies are filled in with the backend proper
struct WaylandState {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    shm: Shm,
    xdg_shell: XdgShell,
    window: SctkWindow,
    pool: SlotPool,
    /// The buffer currently attached to the surface (double-buffered
    /// when the compositor hasn't released the previous one).
    /// 当前挂到 surface 的缓冲（合成器未释放上一帧时走双缓冲）。
    buffer: Option<Buffer>,
    /// Surface size in PHYSICAL pixels (configure's logical size × scale).
    /// surface 物理像素尺寸（configure 逻辑尺寸 × scale）。
    width: u32,
    height: u32,
    /// Integer output scale (`wl_output.scale` / preferred_buffer_scale).
    /// 整数输出缩放（`wl_output.scale` / preferred_buffer_scale）。
    scale: i32,
    /// True until the compositor's first configure arrives (drives the
    /// initial draw).
    /// 首个 configure 到达前为 true（驱动首次绘制）。
    first_configure: bool,
    fullscreen: bool,
    maximized: bool,
    /// Keyboard focus on our surface (wl_keyboard enter/leave).
    /// 键盘焦点在本 surface 上（wl_keyboard enter/leave）。
    keyboard_focus: bool,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    pointer: Option<wl_pointer::WlPointer>,
    /// Snapshot of the last `update_modifiers` (event-time semantics).
    /// 最近一次 `update_modifiers` 的快照（事件时语义）。
    modifiers: Modifiers,
    /// Compositor repeat config (rate/delay); drives client-side repeat
    /// synthesis — compositors don't repeat keys themselves.
    /// 合成器重复配置（rate/delay）；驱动客户端侧重复合成——合成器
    /// 自己不重复按键。
    repeat_info: RepeatInfo,
}

impl CompositorHandler for WaylandState {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _surface: &wl_surface::WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for WaylandState {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle, _output: wl_output::WlOutput) {}

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _output: wl_output::WlOutput,
    ) {
    }
}

impl WindowHandler for WaylandState {
    fn request_close(&mut self, _conn: &Connection, _qh: &QueueHandle, _window: &SctkWindow) {}

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _window: &SctkWindow,
        _configure: WindowConfigure,
        _serial: u32,
    ) {
    }
}

impl SeatHandler for WaylandState {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle, _seat: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _seat: wl_seat::WlSeat,
        _capability: Capability,
    ) {
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _seat: wl_seat::WlSeat,
        _capability: Capability,
    ) {
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle, _seat: wl_seat::WlSeat) {}
}

impl KeyboardHandler for WaylandState {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _modifiers: Modifiers,
        _raw_modifiers: RawModifiers,
        _layout: u32,
    ) {
    }
}

impl PointerHandler for WaylandState {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle,
        _pointer: &wl_pointer::WlPointer,
        _events: &[PointerEvent],
    ) {
    }
}

impl ShmHandler for WaylandState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

// ── sctk 0.21 delegate wiring ───────────────────────────────────────

delegate_registry!(WaylandState);

impl ProvidesRegistryState for WaylandState {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState,];
}

smithay_client_toolkit::delegate_dispatch2!(WaylandState);

// ── WindowInner (stub until the backend proper lands) ───────────────
// ── WindowInner（后端主体落地前保持 stub）────────────────────────────

use crate::error::WindowError;
use crate::event::Event;
use crate::handle::RawWindowHandle;
use crate::window::WindowOptions;

/// The Linux platform window implementation (Wayland).
/// Linux 平台窗口实现（Wayland）。
pub(crate) struct WindowInner;

impl WindowInner {
    pub(crate) fn new(_options: &WindowOptions) -> Result<Self, WindowError> {
        Err(WindowError::UnsupportedPlatform) // backend proper lands next / 后端主体随后落地
    }

    pub(crate) fn set_resizable(&self, _resizable: bool) {}
    pub(crate) fn is_resizable(&self) -> bool {
        false
    }
    pub(crate) fn set_size(&self, _width: u32, _height: u32) {}
    pub(crate) fn set_fullscreen(&self, _fullscreen: bool) {}
    pub(crate) fn is_fullscreen(&self) -> bool {
        false
    }
    pub(crate) fn set_borderless(&self, _borderless: bool) {}
    pub(crate) fn is_borderless(&self) -> bool {
        false
    }
    pub(crate) fn set_mouse_grabbed(&self, _grabbed: bool) {}
    pub(crate) fn is_mouse_grabbed(&self) -> bool {
        false
    }
    pub(crate) fn set_mouse_relative(&self, _enable: bool) {}
    pub(crate) fn set_cursor_visible(&self, _visible: bool) {}
    pub(crate) fn is_cursor_visible(&self) -> bool {
        true
    }
    pub(crate) fn set_visible(&self, _visible: bool) {}
    pub(crate) fn is_visible(&self) -> bool {
        true
    }
    pub(crate) fn set_always_on_top(&self, _top: bool) {}
    pub(crate) fn is_always_on_top(&self) -> bool {
        false
    }
    pub(crate) fn minimize(&self) {}
    pub(crate) fn maximize(&self) {}
    pub(crate) fn restore(&self) {}
    pub(crate) fn raise(&self) {}
    pub(crate) fn is_minimized(&self) -> bool {
        false
    }
    pub(crate) fn is_maximized(&self) -> bool {
        false
    }
    pub(crate) fn has_input_focus(&self) -> bool {
        false
    }
    pub(crate) fn is_occluded(&self) -> bool {
        false
    }
    pub(crate) fn set_keyboard_grabbed(&self, _grab: bool) {}
    pub(crate) fn set_position(&self, _x: i32, _y: i32) {}
    pub(crate) fn position(&self) -> (i32, i32) {
        (0, 0)
    }
    pub(crate) fn center_on_screen(&self) {}
    pub(crate) fn title(&self) -> String {
        String::new()
    }
    pub(crate) fn set_opacity(&self, _opacity: f32) {}
    pub(crate) fn opacity(&self) -> f32 {
        1.0
    }
    pub(crate) fn dpi_scale(&self) -> f32 {
        1.0
    }
    pub(crate) fn high_pixel_density(&self) -> bool {
        false
    }

    pub(crate) fn poll_event(&self) -> Option<Event> {
        None
    }

    pub(crate) fn size(&self) -> (u32, u32) {
        (0, 0)
    }

    pub(crate) fn set_title(&self, _title: &str) {}

    pub(crate) fn raw_handle(&self) -> RawWindowHandle {
        unreachable!(
            "backend not implemented for this platform; a Window cannot exist / 该平台后端尚未实现，Window 不可能被构造"
        )
    }
}
