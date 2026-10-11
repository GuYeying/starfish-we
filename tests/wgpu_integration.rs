//! Verifies the wgpu integration end to end:
//! window → `create_surface(&window)` → configure → acquire the first
//! frame.
//! wgpu 对接便捷性验证：完整走通
//! 建窗 → `create_surface(&window)` → configure → 取第一帧。
//!
//! Only compiled/run with `--features wgpu` on Windows (other platform
//! backends are not implemented yet, and the wgpu dev-dependency is
//! registered on the Windows target only — it never enters the library's
//! dependency tree).
//! 只在 `--features wgpu` + Windows 下编译运行（其余平台后端尚未实现，
//! 且 wgpu dev 依赖仅登记在 Windows target，不进入库的依赖树）。
//!
//! The convenience verdict is visible in the code itself: from [`Window`]
//! to a usable surface, this test contains **not a single unsafe block or
//! raw-pointer cast** — exactly the difference the `wgpu` feature makes
//! over `raw_handle()` + manual bridging.
//! 便捷性结论看代码即可：从 [`Window`] 到可用 surface，
//! 本测试**没有一行 unsafe、没有裸指针转换**——这正是 `wgpu` feature
//! 相对 `raw_handle()` + 手动桥接的差异。

#![cfg(all(windows, feature = "wgpu"))]

use starfish_we::Window;
use starfish_we::flags::WindowFlags;

#[test]
fn wgpu_surface_from_window_zero_unsafe() {
    // 1. Create the window (a real window — it flashes during tests).
    // 1. 建窗（真实窗口，测试时会闪一下）。
    let window =
        Window::new("starfish wgpu smoke", (256, 256), WindowFlags::default()).expect("建窗失败");
    let (width, height) = window.size();

    // 2. The core: hand &window straight to wgpu.
    //    This works because under the `wgpu` feature, Window implements
    //    HasWindowHandle + HasDisplayHandle (raw-window-handle 0.6).
    // 2. 核心：直接把 &window 交给 wgpu。
    //    可行性来自 `wgpu` feature 下 Window 实现了
    //    HasWindowHandle + HasDisplayHandle（raw-window-handle 0.6）。
    let instance = wgpu::Instance::default();
    let surface = instance
        .create_surface(&window)
        .expect("create_surface(&window) failed: is the wgpu feature on? / create_surface(&window) 失败：wgpu feature 未生效？");

    // 3. Adapter / device (blocking the async APIs with pollster).
    // 3. adapter / device（异步 API 用 pollster 阻塞）。
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: Some(&surface),
        ..Default::default()
    }))
    .expect("no usable GPU adapter found / 未找到可用 GPU adapter");

    let (device, _queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("request_device failed / request_device 失败");

    // 4. Configure the surface with the window's current size.
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

    // 5. Acquiring the first frame = the surface is truly attached to our
    //    Win32 window.
    // 5. 取到第一帧 = surface 与我们的 Win32 窗口真实对接成功。
    let acquire = || match surface.get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(frame)
        | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => Some(frame),
        other => {
            eprintln!(
                "frame not acquired (ignoring this frame) / 取帧未成功（忽略具体帧）: {other:?}"
            );
            None
        }
    };
    let frame = acquire().expect("get_current_texture failed: surface not attached to the window / get_current_texture 失败：surface 未正确挂接窗口");
    assert_eq!(frame.texture.width(), width);
    assert_eq!(frame.texture.height(), height);
    drop(frame);

    // 6. Re-configure (the normal response to Event::Resized) and acquire
    //    again.
    // 6. 重新 configure（对应收到 Event::Resized 后的正常操作）后再次取帧。
    surface.configure(&device, &make_config());
    assert!(acquire().is_some());

    // Drop order: surface before window (the surface borrows the window's
    // handles).
    // drop 顺序：先 surface 后 window（surface 借用窗口句柄）。
    drop(surface);

    // 7. AutoSurface (wgpu-auto feature): the library owns the
    //    resize→configure cycle — resize via set_size and the next acquire
    //    returns the NEW size with zero consumer-side handling.
    // 7. AutoSurface（wgpu-auto feature）：库接管 resize→configure——
    //    set_size 之后下一次 acquire 直接返回新尺寸，消费侧零处理。
    #[cfg(feature = "wgpu-auto")]
    {
        let mut auto = starfish_we::AutoSurface::new(&window, &instance)
            .expect("AutoSurface::new failed / AutoSurface::new 失败");
        let frame = auto
            .acquire()
            .expect("AutoSurface first acquire failed / AutoSurface 首次取帧失败");
        assert_eq!((frame.texture.width(), frame.texture.height()), (256, 256));
        drop(frame);

        window.set_size((320, 200));
        // Pump WM_SIZE so the size cache updates.
        // 泵一轮 WM_SIZE 让尺寸缓存更新。
        for _ in 0..64 {
            if window.poll_event().is_none() {
                break;
            }
        }
        let frame = auto
            .acquire()
            .expect("AutoSurface post-resize acquire failed / AutoSurface 调整后取帧失败");
        assert_eq!((frame.texture.width(), frame.texture.height()), (320, 200));
        drop(frame);
    }

    drop(window);
}
