//! Linux/Wayland backend smoke test: real window creation via the
//! Wayland backend (map → configure → shm buffer commit), event drain,
//! title round-trip and the single-window guard.
//! Linux/Wayland 后端冒烟测试：经 Wayland 后端真实建窗（映射 →
//! configure → shm 缓冲提交）、事件排空、标题往返与单窗口守卫。
//!
//! Skip-gate: without a Wayland compositor (`WAYLAND_DISPLAY` unset and
//! no default socket) the test prints "skipped" and passes — plain
//! `cargo test` stays green on compositor-less machines/CI. CI runs it
//! under headless Weston (see `.github/workflows/linux.yml`).
//! skip 门：没有 Wayland 合成器时（`WAYLAND_DISPLAY` 未设且无默认
//! socket）打印 "skipped" 并通过——无合成器机器/CI 上裸 `cargo test`
//! 依旧全绿。CI 在 headless Weston 下运行本测试（见
//! `.github/workflows/linux.yml`）。
//!
//! The single-window slot is process-global state — every assertion must
//! stay inside **one test function**, executed sequentially (cargo runs
//! tests in parallel threads).
//! 单窗口占位是进程级全局状态——所有断言必须放在**单个测试函数**内
//! 顺序执行（cargo 会并行跑测试）。
//!
//! Interactive input is NOT asserted here: headless compositors have no
//! seat devices, and scripted input injection is compositor-specific.
//! Verify typing/mouse/close manually via `cargo run --example basic`
//! (see the guide's Linux acceptance checklist).
//! 此处不断言交互输入：headless 合成器没有 seat 设备，脚本化注入又
//! 依赖具体合成器。打字/鼠标/关闭请在 `cargo run --example basic` 里
//! 手动验收（见指南的 Linux 验收清单）。

#![cfg(target_os = "linux")]

use starfish_we::{Event, Window, WindowError};

fn compositor_available() -> bool {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return true;
    }
    // Default socket: $XDG_RUNTIME_DIR/wayland-0.
    // 默认 socket：$XDG_RUNTIME_DIR/wayland-0。
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(|dir| dir.join("wayland-0"))
        .is_some_and(|path| path.exists())
}

#[test]
fn wayland_window_roundtrip() {
    if !compositor_available() {
        println!("skipped: no Wayland compositor / 无 Wayland 合成器，跳过");
        return;
    }

    // 1. Creation + physical size (headless Weston honors the request).
    // 1. 建窗 + 物理尺寸（headless Weston 尊重请求尺寸）。
    let window = Window::new("wayland smoke", (320, 240), Default::default())
        .expect("window creation on a live compositor / 有合成器时建窗必须成功");
    assert_ne!(
        window.size(),
        (0, 0),
        "size must be known after creation / 建窗后尺寸必须已知"
    );

    // 2. The configure→commit loop must have queued Resized (the init
    //    roundtrip makes this deterministic — no waiting needed).
    // 2. configure→commit 回路必须已入队 Resized（init 的 roundtrip 使其
    //    确定——无需等待）。
    let mut saw_resized = false;
    for _ in 0..64 {
        match window.poll_event() {
            Some(Event::Resized { width, height }) => {
                saw_resized = true;
                assert_eq!(
                    (width, height),
                    window.size(),
                    "Resized must carry the window's physical size / Resized 必须携带窗口物理尺寸"
                );
            }
            Some(_) => {}
            None => break,
        }
    }
    assert!(
        saw_resized,
        "expected Resized from the initial configure / 初始 configure 应产生 Resized"
    );

    // 3. Title round-trip.
    // 3. 标题往返。
    window.set_title("smoke 2");
    assert_eq!(window.title(), "smoke 2");

    // 4. Single-window guard.
    // 4. 单窗口守卫。
    let second = Window::new("second", (100, 100), Default::default());
    assert!(
        matches!(second, Err(WindowError::AlreadyExists)),
        "second window must hit the single-window guard / 第二个窗口必须命中单窗口守卫"
    );

    // 5. Drop → builder recreate (min/max options) → OK.
    // 5. drop → builder 重建（min/max 选项）→ 成功。
    drop(window);
    let rebuilt = Window::builder()
        .title("wayland smoke 2")
        .size((200, 150))
        .min_size((100, 75))
        .max_size((800, 600))
        .build()
        .expect("recreation after drop / drop 后重建必须成功");
    assert_ne!(rebuilt.size(), (0, 0));
    drop(rebuilt);
}
