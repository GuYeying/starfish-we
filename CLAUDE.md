# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **对接/使用本库**：先读《reference/wgpu接入指南.md》（API 全表、事件语义矩阵、
> 各平台驱动原理、wgpu 对接契约、红线清单、诊断速查；英文版
> reference/wgpu-integration-guide.md）。代码注释为英/中双语（英前中后）。
> 本文聚焦仓库工程约定。

## 项目定位

starfish-window 是一个跨平台的**单窗口 + 事件** crate，用于替换 winit。起因：winit 的事件循环是控制反转式的（调用者把闭包 / `ApplicationHandler` 交给 EventLoop，控制流由 winit 驱动，且 macOS 上必须独占主线程），这与宿主项目（脚本语言解释器需要自己持有并驱动主循环）冲突。

本 crate 采用 **SDL 风格运行模式：调用者拥有主循环，主动轮询（poll）事件**，窗口操作都是普通方法调用。

## 设计原则

- API 风格遵循 Rust 惯例（所有权、`Result`、命名约定）；运行模式遵循 SDL 风格（用户主循环 + poll 事件，非回调式）。
- 可读性、可维护性优先，不追求极致性能。
- 跨平台是硬约束：任何公共功能必须能在全部目标平台上实现，平台差异不允许泄漏进公共 API。
- 横向 cfg 架构：各平台是平行的 `#[cfg]` 模块，实现同一套公共抽象；不做运行时 trait 对象式的后端分发。
- 单窗口假设：进程内同一时刻只有一个窗口，事件无需按窗口 id 路由。

## wgpu 集成策略（主要服务对象是 wgpu）

分层解耦，底层零依赖：

- **基础层（默认）**：`Window` 暴露裸平台句柄（自有 `RawWindowHandle` 枚举：HWND / NSWindow / UIView / X11 Display+Window / ANativeWindow / canvas），纯指针、零依赖。用户可用 unsafe 自行桥接任何渲染后端。
- **`wgpu` feature（可选）**：依赖 `raw-window-handle`（极小、无传递依赖），为 `Window` 实现 `HasWindowHandle` + `HasDisplayHandle`，使 `instance.create_surface(&window)` 零 unsafe 直通。**不直接依赖 wgpu 本体**（避免版本钉死与编译膨胀）。
- 渲染循环分工：本 crate 只管窗口+事件（`Resized` / `RedrawRequested`），用户拿到 surface 自行 `configure`。

## feature 规划

```toml
[features]
wgpu          = ["dep:raw-window-handle"]  # 安全对接 wgpu
wgpu-auto     = ["wgpu", "dep:wgpu", "dep:pollster"]  # AutoSurface 自动 resize→configure
android-demo  = ["android_logger", "log", "libc"]  # 内置 Android 演示宿主
```

**AutoSurface（wgpu-auto）**：`AutoSurface::new(&window, &instance)` 一次
构建（内部 adapter/device/configure），`acquire()` 每帧对账窗口尺寸并
透明重配置（消化 Suboptimal/Outdated；Timeout/Occluded 返回 None 跳帧，
Lost 需重建）。真机测试：set_size 后零处理取到新尺寸帧。

事件是核心能力，**没有** event/window feature（2026-09-29 移除：
"仅建窗无事件"是无人能用的配置，且 cfg 噪音遍布全库）。

**自定义事件**：`Event::User(UserEvent)` + `Window::push_event`（同线程）/
`Window::event_proxy() -> EventProxy`（Clone+Send+Sync，跨线程注入，
`send` 失败返回 `EventProxyClosed`）。通道在公共层（mpsc + Mutex<Receiver>，
Mutex 仅为保住 `Window: Sync`），**平台模块零感知**；载荷 `Any+Send+Sync`
按 Arc 共享（`Event: Clone` 共享载荷）；因动态载荷 `Event` 不实现
`PartialEq`。时序：send 后下一次 poll 可见，User 先于本轮平台事件。

## 目标平台

windows / linux(ubuntu) / macos / ios / android / web(wasm)。

当前状态：**Windows 后端已实现并通过冒烟测试**（真实建窗/事件/单窗口守卫），其余五平台仍为 stub（返回 `UnsupportedPlatform`）。公共 API：`Window` / `Event` / `KeyCode` / `Modifiers` / `MouseButton` / `WindowError` / `RawWindowHandle`。各平台在 `src/platform/<os>/` 平行实现同一套 `WindowInner` 接口，经 `platform::imp` 做 cfg 分发；单窗口由 `Window::new` 内的 `AtomicBool` 占位保证。

Windows 后端分层（值得沿用到其他后端）：`platform/windows/mod.rs` 只管建窗、`PeekMessageW` 泵、状态挂载（`GWLP_USERDATA`）与系统默认兜底；消息→`Event` 的翻译全部在 `platform/windows/translate.rs`（仅 `event` feature 编译，返回 `Option<LRESULT>`，`None` 回落系统默认）。因此 `--no-default-features --features window`（仅建窗）同样成立。

已约定的实现取舍：坐标/尺寸一律**物理像素**（Per-Monitor V2 DPI 感知）；`TextInput` 走 `WM_CHAR` 直通、无 IME 组合（首版约定）；`WM_CLOSE` 只上报 `CloseRequested` 不销毁，销毁由 drop 决定；Alt+F4 等交系统默认。**Event 枚举是全平台统一词汇表，不按平台 cfg**：桌面端永不产生 `Touch` / `Suspended` / `Resumed`；移动端后端落地时把主手指合成为鼠标事件（SDL 惯例）。raw-window-handle 为 0.6（`#[non_exhaustive]`，须用 `new()` 构造器 + 字段赋值；`WindowsDisplayHandle` 是空结构）。Windows 的 `WindowInner` 有 `unsafe impl Sync`（仅为满足 wgpu `create_surface` 的 bound，方法仍限创建线程）。六平台 × {default, wgpu} 的 `cargo check` 全部通过。

**wgpu 对接已实测**：`tests/wgpu_integration.rs`（`cargo test --features wgpu --test wgpu_integration`）真实走通 create_surface → configure → 取帧，零 unsafe。wgpu 30 注意事项：`create_surface` 要求 `Sync`；`get_current_texture` 返回枚举 `CurrentSurfaceTexture`（非 Result）；`SurfaceConfiguration` 多了必填 `color_space: SurfaceColorSpace`。wgpu/pollster 仅在 `[target.'cfg(windows)'.dev-dependencies]`，不进库依赖树。

**窗口选项用 builder 而非 SDL 式位掩码 flag**：`Window::builder().title(..).size(..).resizable(..).borderless(..).min_size(..).max_size(..).always_on_top(..).build()`；`Window::new(title, size)` 是 builder 快捷方式。新增选项 = builder 加方法 + `WindowOptions`（pub(crate)）加字段，非破坏性。平台接口为 `WindowInner::new(&WindowOptions)`。Win32 映射：resizable=THICKFRAME+MAXIMIZEBOX、borderless=WS_POPUP（可拖边保留 THICKFRAME）、min/max=WM_GETMINMAXINFO、置顶=SetWindowPos(HWND_TOPMOST)。延后项：`set_fullscreen`（需显示器枚举）、初始位置、图标、运行时 set_resizable/set_borderless。注意：tests/windows_smoke.rs 的断言必须在单个测试函数内顺序执行（单窗口占位是进程级全局状态，cargo 并行测试会互抢）。

## Android / Web 后端（已实现，打包测试流程）

两平台同用"回调写入缓冲队列、poll_event 排空"的反转装置。

- **Web**（wasm-bindgen + web-sys）：窗口 = `<canvas>`（按 id `starfish-window` 找，缺省自建铺满）；键盘走 W3C `code`（`KeyCode::from_w3c_code`，仅 wasm 编译）；坐标 × devicePixelRatio = 物理像素；rAF 循环只置重绘标志（合并）；visibilitychange → Suspended/Resumed；Web 无 CloseRequested。打包：`scripts/build_web.sh` → `testweb/`，`python -m http.server 8000 -d testweb` 浏览器测试。**wasm-bindgen crate 版本必须与 wasm-bindgen-cli 精确一致（当前 =0.2.127，被 wgpu dev-dep→web-sys 0.3.104 钉死）**。
- **Android**（android-activity 0.6 + NativeActivity）：宿主入口用 `starfish_window::android_main!(their_main)` 宏一行接入（展开为 `android_main`：先 `android_init` 再交控制权；手写 `android_main` + `android_init(app)` 亦可）；`Window::new` 阻塞等 surface（InitWindow，上限 10s）；生命周期 Pause/Stop/TerminateWindow→Suspended、Resume/InitWindow→Resumed、Destroy/返回键→CloseRequested；触摸→Touch+主手指(id 0)合成鼠标。内置演示宿主 = `android-demo` feature（`src/platform/android/entry.rs`），**apps/ 目录已并入 lib、workspace 已移除**。
- **Android 进程复用（踩过的坑）**：用户"返回"只销毁 Activity，进程被系统缓存；再次启动时 `android_main` 在**同一进程、新线程**上二次执行。三个连环问题及修复：① `android_init` 曾用 `OnceLock` → 二次装配被静默忽略 → 闪退（已改 `Mutex<Option<AndroidApp>>` 覆盖式装配，并重置 `PENDING`/`WINDOW_READY`/单窗口占位 `reset_single_window_slot`）；② 旧实例的 `Destroy` 事件串扰进新实例事件流 → 二次启动秒退（已加 `WINDOW_READY` 门：未见 `InitWindow` 前丢弃 `Destroy`）；③ android-activity 0.6.1 胶水 static 不支持同进程"销毁中途重建"——新旧交叠时其 UI 线程回调 abort / 线程断言 panic（上游无修复版）。**宿主退出契约：收到 `CloseRequested` 或 `Suspended` 后立即终结进程**（demo 用 `libc::raise(SIGKILL)`，`exit(0)` 会留 RenderThread/JIT 的 SIGABRT/SIGSEGV 退出噪音）。另注意：返回键的 KeyDown 可能被系统拦截不派发（只见 Suspended），不能只依赖它。诊断手法：`adb logcat -s starfish RustStdoutStderr` 收 panic；模拟器可完整复现时序。关联参考：《android绑定运行问题与解决方案.md》（TLS 键耗尽 / surface 就绪时序，同类"平台运行时差异"问题）。
- **Android 打包**：`export ANDROID_NDK_ROOT=D:/SDK/Android/ndk/30.0.16248370 && cargo apk build --features android-demo`（根包，无需 -p）→ `target/debug/apk/starfish-demo.apk`，`adb install` 安装，`adb logcat -s starfish` 看事件。APK 元数据在根 Cargo.toml `[package.metadata.android]`。本机 SDK 平台目录叫 `android-37.0`，已建 junction `platforms/android-37` 供 cargo-apk 识别（Google 源被网络拦截，sdkmanager 装不了别的平台）。注意 `[lib] crate-type = ["lib","cdylib"]`（APK 需要），桌面构建会多产出一个动态库文件属已知成本；cargo-apk 会尝试把 examples 也打包成 cdylib（对 `examples/basic` 报 `libbasic.so` 不存在——**非致命**，退出码仍 0，APK 正常产出）。
- **rwh/wgpu（Android）**：`RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(NonNull))`（注意 rwh 0.6 变体名叫 AndroidNdk 不是 Android）；display 侧 `AndroidDisplayHandle::new()` 空结构。
- **示例注意**：examples/web_demo.rs 的 wasm 代码逐项 `#[cfg]`，`fn main` 无条件保留（cargo test/clippy 在宿主构建示例）。

## 常用命令

- `cargo check` / `cargo build`（纯 lib；示例：`cargo run --example basic`）
- `cargo test`；运行单个测试：`cargo test <test_name>`（tests/windows_smoke.rs 会真实建窗）
- feature 组合检查：`cargo check --no-default-features`、`--no-default-features --features window`、`--features wgpu`
- `cargo clippy` / `cargo fmt`
- 远端平台的语法/类型检查（Windows 本机即可，无需对应系统 SDK）：
  `rustup target add <target>` 后 `cargo check --target <target>`
  常用 target：`x86_64-pc-windows-msvc`、`x86_64-unknown-linux-gnu`、`aarch64-apple-darwin`、`aarch64-apple-ios`、`aarch64-linux-android`、`wasm32-unknown-unknown`
