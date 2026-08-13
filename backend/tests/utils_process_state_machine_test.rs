use bingxi_backend::services::color_card_issue_service::*;
use bingxi_backend::utils::process_state_machine::*;

#[test]
fn test_next_status_normal_progression() {
    // P9-1: 用 match 处理 Result，失败时立即 panic 并说明 P9-1
    let unwrap_p9 =
        |res: Result<CustomOrderStatus, StateMachineError>, ctx: &str| -> CustomOrderStatus {
            match res {
                Ok(s) => s,
                Err(e) => panic!("P9-1: 测试夹具 {ctx} 状态机返回错误: {e}"),
            }
        };
    // V15 P0-B11：新增 lab_dip / quotation 状态，draft → lab_dip → quotation → yarn_purchasing
    assert_eq!(
        unwrap_p9(next_status("draft"), "draft"),
        CustomOrderStatus::LabDip
    );
    assert_eq!(
        unwrap_p9(next_status("lab_dip"), "lab_dip"),
        CustomOrderStatus::Quotation
    );
    assert_eq!(
        unwrap_p9(next_status("quotation"), "quotation"),
        CustomOrderStatus::YarnPurchasing
    );
    assert_eq!(
        unwrap_p9(next_status("yarn_purchasing"), "yarn_purchasing"),
        CustomOrderStatus::Dyeing
    );
    assert_eq!(
        unwrap_p9(next_status("dyeing"), "dyeing"),
        CustomOrderStatus::Finishing
    );
    assert_eq!(
        unwrap_p9(next_status("finishing"), "finishing"),
        CustomOrderStatus::Delivery
    );
    assert_eq!(
        unwrap_p9(next_status("delivery"), "delivery"),
        CustomOrderStatus::AfterSales
    );
    assert_eq!(
        unwrap_p9(next_status("after_sales"), "after_sales"),
        CustomOrderStatus::Completed
    );
}

#[test]
fn test_next_status_terminal_fails() {
    assert!(next_status("completed").is_err());
    assert!(next_status("cancelled").is_err());
}

#[test]
fn test_next_status_invalid_string() {
    assert!(next_status("invalid_state").is_err());
}

#[test]
fn test_can_transition_normal() {
    // V15 P0-B11：7 阶段工艺流程
    assert!(can_transition("draft", "lab_dip"));
    assert!(can_transition("lab_dip", "quotation"));
    assert!(can_transition("quotation", "yarn_purchasing"));
    assert!(can_transition("yarn_purchasing", "dyeing"));
    assert!(can_transition("dyeing", "finishing"));
    assert!(can_transition("finishing", "delivery"));
    assert!(can_transition("delivery", "after_sales"));
    assert!(can_transition("after_sales", "completed"));
}

#[test]
fn test_can_transition_to_cancelled() {
    assert!(can_transition("draft", "cancelled"));
    assert!(can_transition("lab_dip", "cancelled"));
    assert!(can_transition("quotation", "cancelled"));
    assert!(can_transition("yarn_purchasing", "cancelled"));
    assert!(can_transition("delivery", "cancelled"));
}

#[test]
fn test_cannot_transition_terminal() {
    assert!(!can_transition("completed", "draft"));
    assert!(!can_transition("cancelled", "yarn_purchasing"));
}

#[test]
fn test_cannot_skip_stages() {
    // V15 P0-B11：禁止跳过打样/报价阶段
    assert!(!can_transition("draft", "yarn_purchasing"));
    assert!(!can_transition("draft", "quotation"));
    assert!(!can_transition("draft", "dyeing"));
    assert!(!can_transition("draft", "delivery"));
    assert!(!can_transition("lab_dip", "yarn_purchasing"));
    assert!(!can_transition("yarn_purchasing", "finishing"));
}

#[test]
fn test_is_terminal() {
    assert!(CustomOrderStatus::Completed.is_terminal());
    assert!(CustomOrderStatus::Cancelled.is_terminal());
    assert!(!CustomOrderStatus::Draft.is_terminal());
    assert!(!CustomOrderStatus::Delivery.is_terminal());
}

#[test]
fn test_default_process_nodes() {
    let nodes = default_process_nodes();
    assert_eq!(nodes.len(), 5);
    assert_eq!(nodes[0].0, "yarn_purchasing");
    assert_eq!(nodes[4].0, "after_sales");
}
