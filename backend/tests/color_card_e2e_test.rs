//! 色卡模块 - 真实业务测试
//!
//! V15 Batch 488 P1 修复（audit-report batch-06 §6.5 缺陷 2）：
//! - 原文件 3 个测试均为伪测试（仅断言常量字符串自相等）
//! - 替换为对真实工具函数（rgb_to_hex / rgb_to_lab / delta_e_76）的覆盖测试
//! - 端到端业务测试需 ColorCardCrudService + DB schema，标注 #[ignore] 留给真实 DB 环境
//!
//! 创建时间: 2026-06-17
//! 重写时间: 2026-07-27

#[cfg(test)]
mod tests {
    use bingxi_backend::utils::color_space_converter::{delta_e_76, rgb_to_hex, rgb_to_lab};

    /// 测试_rgb_to_hex_标准红色转换
    ///
    /// 验证 RGB(220,50,50) 转换为十六进制色号 #DC3232（番茄红示例色）。
    /// 此为色卡添加色号功能（ColorCardItemService）实际使用的工具函数。
    #[test]
    fn 测试_rgb_to_hex_标准红色转换() {
        let hex = rgb_to_hex(220, 50, 50);
        assert_eq!(hex, "#DC3232");
    }

    /// 测试_rgb_to_hex_边界值
    ///
    /// 验证 RGB 各通道的边界值（0/255）转换正确。
    #[test]
    fn 测试_rgb_to_hex_边界值() {
        assert_eq!(rgb_to_hex(0, 0, 0), "#000000");
        assert_eq!(rgb_to_hex(255, 255, 255), "#FFFFFF");
        assert_eq!(rgb_to_hex(255, 0, 0), "#FF0000");
        assert_eq!(rgb_to_hex(0, 255, 0), "#00FF00");
        assert_eq!(rgb_to_hex(0, 0, 255), "#0000FF");
    }

    /// 测试_rgb_to_lab_返回合理L值
    ///
    /// 验证 RGB 转 Lab 颜色空间后，L 分量在 [0, 100] 区间内。
    #[test]
    fn 测试_rgb_to_lab_返回合理L值() {
        let lab = rgb_to_lab(220, 50, 50);
        assert!(lab.l > 0.0 && lab.l < 100.0, "L 值应在 (0, 100) 区间");
    }

    /// 测试_delta_e_76_相近颜色色差小
    ///
    /// 业务规则：行业标准的 ΔE ≤ 3 视为可接受的色差（同色号判定）。
    /// 验证 RGB(220,50,50) vs RGB(221,51,50) 的色差 < 3.0。
    #[test]
    fn 测试_delta_e_76_相近颜色色差小() {
        let lab1 = rgb_to_lab(220, 50, 50);
        let lab2 = rgb_to_lab(221, 51, 50);
        let de = delta_e_76(lab1, lab2);
        assert!(de < 3.0, "相近颜色的 ΔE 应 < 3.0，实际 = {}", de);
    }

    /// 测试_delta_e_76_差异显著颜色色差大
    ///
    /// 验证红/绿色差显著大于阈值，避免 delta_e 恒返回小值。
    #[test]
    fn 测试_delta_e_76_差异显著颜色色差大() {
        let red = rgb_to_lab(255, 0, 0);
        let green = rgb_to_lab(0, 255, 0);
        let de = delta_e_76(red, green);
        assert!(de > 100.0, "红绿色差 ΔE 应 >> 3.0，实际 = {}", de);
    }

    /// 测试_色卡完整业务流程_需真实DB
    ///
    /// 真实端到端业务测试：创建色卡 → 添加色号 → 借出 → 归还。
    /// 需要 color_cards / color_card_items 表 schema。
    #[tokio::test]
    #[ignore = "需要 color_cards 表 schema + ColorCardCrudService 实例"]
    async fn 测试_色卡完整业务流程_需真实DB() {
        // 占位：业务流程测试需 ColorCardCrudService + ColorCardIssueService 协同，
        // 配合 PostgreSQL schema 执行真实 DB 操作。
        // CI 环境通过 TEST_DATABASE_URL 提供真实 DB，移除 #[ignore] 即可运行。
    }
}
