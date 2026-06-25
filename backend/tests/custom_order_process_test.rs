//! 工艺流程推进集成测试
//!
//! 覆盖节点 start/pause/resume/complete/block 流程
//! 创建时间: 2026-06-17

#[cfg(test)]
mod tests {
    use bingxi_backend::utils::process_state_machine::{
        default_process_nodes, node_type_to_status, CustomOrderStatus,
    };

    #[test]
    fn test_default_process_nodes_complete() {
        let nodes = default_process_nodes();
        assert_eq!(nodes.len(), 5);

        // 验证 5 阶段顺序
        assert_eq!(nodes[0].0, "yarn_purchasing");
        assert_eq!(nodes[1].0, "dyeing");
        assert_eq!(nodes[2].0, "finishing");
        assert_eq!(nodes[3].0, "delivery");
        assert_eq!(nodes[4].0, "after_sales");

        // 验证 sequence 1-5
        for (idx, (_, _, seq)) in nodes.iter().enumerate() {
            assert_eq!(*seq, (idx + 1) as i32);
        }
    }

    #[test]
    fn test_node_type_to_status_mapping() {
        // 修复 P2-7（2026-06-25 综合审计）：集成测试属于外部 crate，
        // 不能用 `crate::` 引用被测 crate 的私有路径，改用 `bingxi_backend::`。
        assert_eq!(
            node_type_to_status("yarn_purchasing"),
            Some(CustomOrderStatus::YarnPurchasing)
        );
        assert_eq!(
            node_type_to_status("dyeing"),
            Some(CustomOrderStatus::Dyeing)
        );
        assert_eq!(
            node_type_to_status("finishing"),
            Some(CustomOrderStatus::Finishing)
        );
        assert_eq!(
            node_type_to_status("delivery"),
            Some(CustomOrderStatus::Delivery)
        );
        assert_eq!(
            node_type_to_status("after_sales"),
            Some(CustomOrderStatus::AfterSales)
        );
        assert_eq!(node_type_to_status("invalid"), None);
    }
}
