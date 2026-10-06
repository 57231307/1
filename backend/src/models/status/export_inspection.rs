//! 外贸/出口商检域状态词表
//!
//! 本文件收口出口商检记录结论列（`export_inspection.result`）的权威取值。词表唯一
//! 事实来源是写入方：本域写入点只有建单（落 pending）与「登记结果」动作（写
//! pass/fail），二者都以本 mod 常量为唯一 token 来源，比较点与门控值必须逐字符相同。

/// 出口商检结论（export_inspection.result，小写三值）
///
/// 词表与既有同族检验结论（purchase_inspection_result 小写英文口径）一致，区别于
/// quality_inspection_records 的中文四值结论（另一实体、另一词表，不可互抄）。
/// DB 侧 CHECK 约束与本 `ALL` 逐元素相等，由契约测试锁定；两侧任一侧单独增删值判红。
pub mod export_inspection_result {
    /// 待检：建单即此态，尚未由检验机构出具结论
    pub const PENDING: &str = "pending";

    /// 合格：商检通过，可放行出口
    pub const PASS: &str = "pass";

    /// 不合格：商检未通过，不得放行
    pub const FAIL: &str = "fail";

    /// 本列全部合法取值：入参校验与 DB CHECK 的唯一取值来源
    pub const ALL: &[&str] = &[PENDING, PASS, FAIL];

    /// 白名单判定：逐字符匹配，不做大小写/中英转换，词表外（含空串/变体）一律 false
    pub fn is_valid(result: &str) -> bool {
        ALL.contains(&result)
    }
}
