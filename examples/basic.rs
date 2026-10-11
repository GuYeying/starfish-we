//! Basic demo: create a window and print events until close. Runs on
//! Windows and Linux/Wayland (macOS: `cargo run --example basic` too).
//! 基础演示：建窗 + 打印事件直到关闭。Windows 与 Linux/Wayland 可跑
//! （macOS 亦可：`cargo run --example basic`）。
//!
//! Run: `cargo run --example basic`
//! 运行：`cargo run --example basic`

use std::time::Duration;

use starfish_we::flags::WindowFlags;
use starfish_we::{Event, Window};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let window = Window::new("starfish-we demo", (800, 600), WindowFlags::default())?;
    println!(
        "window created, client area = {:?}; close the window to exit / 窗口已创建，客户区 = {:?}，关闭窗口即退出",
        window.size(),
        window.size()
    );

    'main: loop {
        // SDL style: drain this round's events, then advance the caller's
        // own tick.
        // SDL 风格：排空本轮事件，然后推进调用者自己的 tick。
        while let Some(event) = window.poll_event() {
            println!("{event:?}");
            if let Event::CloseRequested = event {
                break 'main;
            }
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    println!("window closed, exiting / 窗口已关闭，退出");
    Ok(())
}
