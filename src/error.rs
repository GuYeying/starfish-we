//! 错误类型（平台无关）。

use std::fmt;

/// 窗口操作失败。
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WindowError {
    /// 平台 API 调用失败，附带底层错误描述。
    OsError(String),
    /// 进程内已存在一个存活的 [`Window`](crate::Window)（单窗口约束）。
    AlreadyExists,
    /// 该平台的后端尚未实现（骨架阶段占位）。
    UnsupportedPlatform,
}

impl fmt::Display for WindowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WindowError::OsError(msg) => write!(f, "平台窗口调用失败: {msg}"),
            WindowError::AlreadyExists => write!(f, "已存在一个窗口（单窗口约束）"),
            WindowError::UnsupportedPlatform => write!(f, "该平台后端尚未实现"),
        }
    }
}

impl std::error::Error for WindowError {}
