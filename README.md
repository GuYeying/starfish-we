# starfish-window

跨平台**单窗口 + 事件**库（Rust），SDL 风格 poll 模型——调用者拥有主循环，
主动拉取事件，而不是把控制流交给事件循环的回调。主要为 [wgpu](https://wgpu.rs)
渲染层与脚本解释器宿主设计（winit 的控制反转与解释器自驱主循环冲突）。

```toml
[dependencies]
starfish-window = { path = ".", features = ["wgpu"] }
```

```rust
use starfish_window::{Event, Window};

let mut window = Window::new("demo", (800, 600), Default::default())?;
let surface = instance.create_surface(&window)?;   // wgpu feature：零 unsafe 直通

loop {
    while let Some(event) = window.poll_event() {  // 非阻塞拉取
        match event {
            Event::Resized { width, height } => { /* 重配 surface（物理像素） */ }
            Event::Suspended => { /* Android/Web：surface 失效，销毁待重建 */ }
            Event::Resumed => { /* 重建 surface */ }
            Event::CloseRequested => break,
            _ => {}
        }
    }
    // ← 你的 tick：执行字节码 / 渲染一帧
}
```

## 特性

- **SDL 风格运行模式**：调用者持有主循环，`poll_event()` 非阻塞逐条拉取
- **单窗口**：进程内最多一个 `Window`，无窗口 id 路由，API 极简
- **物理像素约定**：尺寸/坐标与 `wgpu::SurfaceConfiguration` 直接对齐
- **零依赖基础层**：默认 feature 不拖任何第三方库；`wgpu` feature 只依赖
  `raw-window-handle 0.6`（刻意不依赖 wgpu 本体）
- **自定义事件**：`EventProxy` 跨线程向主循环注入类型化载荷（SDL
  `RegisterEvents` 的类型化替代）
- **feature 可裁剪**：`window`（仅建窗）/ `event`（默认，窗口+事件）/ `wgpu`

## 平台状态

| 平台 | 窗口 | 事件 | 渲染对接 | 状态 |
|---|---|---|---|---|
| Windows | ✅ | ✅ 键鼠/滚轮/焦点 | ✅ wgpu 真机验证 | 可用 |
| Android | ✅ | ✅ 含触摸+生命周期 | ✅ 句柄就绪 | 可用（模拟器验证） |
| Web (wasm) | ✅ canvas | ✅ 含触摸+页面可见性 | `Window::canvas()` | 可用 |
| Linux / macOS / iOS | — | — | — | 计划中（stub） |

各平台驱动原理、事件语义矩阵、注意事项与诊断手段见
[`reference/wgpu接入指南.md`](reference/wgpu接入指南.md)。

## 开发

```bash
cargo check                                   # 六平台编译矩阵
cargo test                                    # Windows 冒烟 + wgpu 真机测试
cargo run --example basic                     # Windows 事件演示
cargo apk build --features android-demo       # Android 演示 APK（需 ANDROID_NDK_ROOT）
scripts/build_web.sh                          # Web 打包 → testweb/
```

## 开源协议

双协议，任选其一：

- [MIT](LICENSE-MIT)
- [Apache-2.0](LICENSE-APACHE)
