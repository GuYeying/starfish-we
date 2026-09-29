//! Basic demo: create a window and print events until close (the Windows
//! backend is currently usable).
//! 基础演示：建窗 + 打印事件直到关闭（Windows 后端当前可用）。
//!
//! Run: `cargo run --example basic`
//! 运行：`cargo run --example basic`

use std::time::Duration;

use starfish_window::flags::WindowFlags;
use starfish_window::{Event, Window};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new("starfish-window demo", (800, 600), WindowFlags::default())?;
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
