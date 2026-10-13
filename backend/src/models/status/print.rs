#![allow(dead_code)]
//! 打印子系统状态词表
//!
//! 本文件收口打印任务状态（`print_tasks.status`）与打印模板状态（`print_templates.status`）
//! 的权威取值。词表唯一事实来源是写入方：状态流转与建单/停用的写入都以本 mod 常量为唯一
//! token 来源，比较点与门控值必须逐字符相同。DB 侧 CHECK 约束与下方各 `ALL` 逐元素相等，
//! 由建表迁移与契约测试锁定；任一侧单独增删值判红。

/// 打印任务状态（print_tasks.status，小写英文闭合五值）
///
/// 生命周期：pending（已受理待入队）→ queued（已入队）→ dispatched（已派发至后端）→
/// succeeded（后端确认成功）/ failed（失败，error_message 必非空）。取值与建表迁移的
/// `chk_print_tasks_status` 逐项一致。
pub mod print_task_status {
    /// 待处理：任务已受理，尚未入队
    pub const PENDING: &str = "pending";

    /// 已入队：任务进入打印队列，等待派发
    pub const QUEUED: &str = "queued";

    /// 已派发：任务已提交至打印后端，等待回执
    pub const DISPATCHED: &str = "dispatched";

    /// 已成功：后端确认打印作业完成
    pub const SUCCEEDED: &str = "succeeded";

    /// 已失败：打印作业失败，失败原因落 error_message 列
    pub const FAILED: &str = "failed";

    /// 本列全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[PENDING, QUEUED, DISPATCHED, SUCCEEDED, FAILED];

    /// 白名单判定：逐字符匹配，不做大小写/中英转换，词表外（含空串/变体）一律 false
    pub fn is_valid(status: &str) -> bool {
        ALL.contains(&status)
    }
}

/// 打印模板状态（print_templates.status，小写英文闭合两值）
///
/// 取值与建表迁移的 `chk_print_templates_status` 逐项一致。
pub mod print_template_status {
    /// 启用：模板可被选用于打印
    pub const ACTIVE: &str = "active";

    /// 停用：模板不再供选用
    pub const INACTIVE: &str = "inactive";

    /// 本列全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[ACTIVE, INACTIVE];

    /// 白名单判定：逐字符匹配，词表外（含空串/变体）一律 false
    pub fn is_valid(status: &str) -> bool {
        ALL.contains(&status)
    }
}
