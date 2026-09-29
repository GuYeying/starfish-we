//! wgpu 对接便捷性验证：完整走通
//! 建窗 → `create_surface(&window)` → configure → 取第一帧。
//!
//! 只在 `--features wgpu` + Windows 下编译运行（其余平台后端尚未实现，
//! 且 wgpu dev 依赖仅登记在 Windows target，不进入库的依赖树）。
//!
//! 便捷性结论看代码即可：从 [`Window`] 到可用 surface，
//! 本测试**没有一行 unsafe、没有裸指针转换**——这正是 `wgpu` feature
//! 相对 `raw_handle()` + 手动桥接的差异。

#![cfg(all(windows, feature = "wgpu"))]

use starfish_window::Window;
use starfish_window::flags::WindowFlags;

#[test]
fn wgpu_surface_from_window_zero_unsafe() {
    // 1. 建窗（真实窗口，测试时会闪一下）。
    let window =
        Window::new("starfish wgpu smoke", (256, 256), WindowFlags::default()).expect("建窗失败");
    let (width, height) = window.size();

    // 2. 核心：直接把 &window 交给 wgpu。
    //    可行性来自 `wgpu` feature 下 Window 实现了
    //    HasWindowHandle + HasDisplayHandle（raw-window-handle 0.6）。
    let instance = wgpu::Instance::default();
    let surface = instance
        .create_surface(&window)
        .expect("create_surface(&window) 失败：wgpu feature 未生效？");

    // 3. adapter / device（异步 API 用 pollster 阻塞）。
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: Some(&surface),
        ..Default::default()
    }))
    .expect("未找到可用 GPU adapter");

    let (device, _queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device 失败");

    // 4. 用窗口当前尺寸 configure surface。
    let capabilities = surface.get_capabilities(&adapter);
    let format = capabilities.formats[0];
    let make_config = || wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width,
        height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: capabilities.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
        color_space: wgpu::SurfaceColorSpace::Auto,
    };
    surface.configure(&device, &make_config());

    // 5. 取到第一帧 = surface 与我们的 Win32 窗口真实对接成功。
    let acquire = || match surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(frame)
        | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => Some(frame),
        other => {
            eprintln!("取帧未成功（忽略具体帧）: {other:?}");
            None
        }
    };
    let frame = acquire().expect("get_current_texture 失败：surface 未正确挂接窗口");
    assert_eq!(frame.texture.width(), width);
    assert_eq!(frame.texture.height(), height);
    drop(frame);

    // 6. 重新 configure（对应收到 Event::Resized 后的正常操作）后再次取帧。
    surface.configure(&device, &make_config());
    assert!(acquire().is_some());

    // drop 顺序：先 surface 后 window（surface 借用窗口句柄）。
    drop(surface);
    drop(window);
}
