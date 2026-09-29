//! Android 入口 glue 与内置演示宿主。
//!
//! 宿主用 [`android_main!`](crate::android_main) 宏一行接入（见宏文档）；
//! `android-demo` feature 提供内置演示宿主（logcat 事件循环），
//! 打 APK：`cargo apk build --features android-demo`。

/// Android 宿主入口（NativeActivity）。
///
/// 展开为导出的 `android_main`：先装配本库回调（[`android_init`]），
/// 再把控制权交给 `$main(app)`。宿主在 crate 根一行接入：
///
/// ```ignore
/// starfish_window::android_main!(my_app_main);
///
/// fn my_app_main(app: starfish_window::AndroidApp) {
///     // 你的主循环：建窗 + poll_event 驱动
/// }
/// ```
///
/// 注意：使用宏后不要再手写 `android_main`（符号冲突）；
/// 宿主 crate 需是 cdylib（APK 的加载单元）。
#[macro_export]
macro_rules! android_main {
    ($main:path) => {
        #[unsafe(no_mangle)]
        fn android_main(app: $crate::AndroidApp) {
            $crate::android_init(app.clone());
            $main(app);
        }
    };
}

/// 内置演示宿主（`android-demo` feature）：建窗 + logcat 打印全部事件，
/// 退出走契约（见下）。仅用于演示 / 冒烟，正式宿主用自己的 main + 宏。
#[cfg(feature = "android-demo")]
pub(crate) mod demo {
    use std::time::Duration;

    use android_activity::AndroidApp;
    use log::LevelFilter;

    use crate::{Event, Window};

    /// 演示主循环。
    pub(crate) fn demo_main(_app: AndroidApp) {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(LevelFilter::Info)
                .with_tag("starfish"),
        );
        log::info!("android_main: 演示宿主启动");

        // 阻塞等待 surface 就绪（上限在 Window::new 内部处理）。
        let mut window = match Window::builder().title("starfish android demo").build() {
            Ok(window) => window,
            Err(err) => {
                log::error!("建窗失败: {err}");
                return;
            }
        };
        log::info!("窗口就绪 size={:?}", window.size());

        // 退出约定（演示程序策略）：`CloseRequested`（销毁）或 `Suspended`
        // （返回键被系统拦截时不派发 KeyDown，只见暂停）任一出现即终结进程，
        // 规避 android-activity 0.6.1 胶水不支持同进程"销毁中途重建"的问题。
        // 库语义不变：Suspended ≠ 退出；这是演示宿主的退出策略，
        // 解释器宿主可按需改为 moveTaskToBack 等后台化方案。
        loop {
            while let Some(event) = window.poll_event() {
                log::info!("{event:?}");
                if matches!(event, Event::CloseRequested | Event::Suspended) {
                    log::info!("{event:?} → 干净退出进程");
                    clean_exit();
                }
            }
            std::thread::sleep(Duration::from_millis(16));
        }
    }

    /// 立即终结进程。
    ///
    /// 不用 `exit(0)`：它在仍有存活线程（RenderThread / JIT 池）时会走
    /// libc 清理路径，触发 SIGABRT / SIGSEGV 退出噪音并留 tombstone。
    /// SIGKILL 自杀即时、不可捕获、无任何清理噪音。
    fn clean_exit() -> ! {
        unsafe { libc::raise(libc::SIGKILL) };
        std::process::exit(0) // 兜底（SIGKILL 正常情况不可返回）
    }
}
