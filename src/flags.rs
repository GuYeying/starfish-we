//! 窗口创建标志（SDL flag 语义；值/预留位为本 crate 自定）。
//!
//! - flag = **初始状态**；运行期可切项走 `Window` 的 `set_*` 方法族
//! - 高位预留：新 flag 加位不破坏旧调用者
//! - 后端不支持的位：忽略（个别 warn），跨平台脚本不受影响
//! - 每个位 × 后端的支持矩阵见 wgpu接入指南.md / 各后端模块文档

/// 窗口创建标志（位组合；`Default` = 仅 [`WindowFlags::RESIZABLE`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowFlags(u32);

impl WindowFlags {
    /// 可由用户调整大小（含最大化按钮）。默认已含。
    pub const RESIZABLE: Self = Self(1 << 0);
    /// 无边框全屏（进入时保存 placement，退出还原）。
    pub const FULLSCREEN: Self = Self(1 << 1);
    /// 无边框（无系统标题栏与边框）。
    pub const NOFRAME: Self = Self(1 << 2);
    /// 创建即可见 = false（隐藏；运行期 show/hide 可切）。
    pub const HIDDEN: Self = Self(1 << 3);
    /// 高像素密度（HiDPI 物理像素坐标）。
    pub const HIGH_PIXEL_DENSITY: Self = Self(1 << 4);
    /// 创建即抓取鼠标（限定客户区；运行期可切）。
    pub const MOUSE_GRABBED: Self = Self(1 << 5);
    /// 窗口置顶。
    pub const ALWAYS_ON_TOP: Self = Self(1 << 6);
    /// 窗口透明（合成器支持时；v0 部分 Windows 平台生效）。
    pub const TRANSPARENT: Self = Self(1 << 7);
    /// 创建即最小化。
    pub const MINIMIZED: Self = Self(1 << 8);
    /// 创建即最大化。
    pub const MAXIMIZED: Self = Self(1 << 9);
    // 1<<10 以上预留。

    /// 组合任意标志位。
    #[must_use]
    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }

    /// 原始位。
    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// 由原始位构造（未知的位原样保留，后端忽略）。
    #[must_use]
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// 是否含某位。
    #[must_use]
    pub const fn contains(self, flag: Self) -> bool {
        self.0 & flag.0 == flag.0
    }
}

impl std::ops::BitOr for WindowFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for WindowFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl Default for WindowFlags {
    /// 默认 = RESIZABLE（SDL 同款：默认可调大小）。
    fn default() -> Self {
        Self::RESIZABLE
    }
}
