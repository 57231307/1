//! 催收任务服务集成测试（纯逻辑层：状态/类型/优先级枚举与逾期分级）
//!
//! 覆盖缺口：collection_task_service.rs 在 CI 覆盖率报告中 0% 行覆盖，
//! 其逾期天数 → 催收方式/优先级的分级逻辑是催收业务的路由核心，公式回归
//! 直接影响催收任务派发正确性。
//!
//! 验证范围：
//! - TaskStatus/TaskType/TaskPriority 的 as_str 序列化契约（DB 存储值）
//! - TaskType::from_overdue_days 分级边界（<30 phone / 30-90 visit / >90 letter）
//! - TaskPriority::from_overdue_days 分级边界（<30 normal / 30-90 high / >90 urgent）

use bingxi_backend::services::collection_task_service::{TaskPriority, TaskStatus, TaskType};

#[test]
fn test_task_status_as_str_contract() {
    // DB 存储值契约：as_str 的输出直接写入 task_status 列，
    // 变更需同步迁移与查询过滤（回归防护）
    assert_eq!(TaskStatus::Pending.as_str(), "pending");
    assert_eq!(TaskStatus::InProgress.as_str(), "in_progress");
    assert_eq!(TaskStatus::Completed.as_str(), "completed");
    assert_eq!(TaskStatus::Cancelled.as_str(), "cancelled");
}

#[test]
fn test_task_type_as_str_contract() {
    assert_eq!(TaskType::Phone.as_str(), "phone");
    assert_eq!(TaskType::Visit.as_str(), "visit");
    assert_eq!(TaskType::Email.as_str(), "email");
    assert_eq!(TaskType::Letter.as_str(), "letter");
}

#[test]
fn test_task_priority_as_str_contract() {
    assert_eq!(TaskPriority::Low.as_str(), "low");
    assert_eq!(TaskPriority::Normal.as_str(), "normal");
    assert_eq!(TaskPriority::High.as_str(), "high");
    assert_eq!(TaskPriority::Urgent.as_str(), "urgent");
}

#[test]
fn test_task_type_from_overdue_days_boundaries() {
    // 边界精确断言（<30 / <=90 / >90 三段，覆盖每段端点）
    assert!(matches!(TaskType::from_overdue_days(0), TaskType::Phone));
    assert!(matches!(TaskType::from_overdue_days(29), TaskType::Phone));
    assert!(matches!(TaskType::from_overdue_days(30), TaskType::Visit));
    assert!(matches!(TaskType::from_overdue_days(90), TaskType::Visit));
    assert!(matches!(TaskType::from_overdue_days(91), TaskType::Letter));
    assert!(matches!(TaskType::from_overdue_days(365), TaskType::Letter));
}

#[test]
fn test_task_priority_from_overdue_days_boundaries() {
    assert!(matches!(
        TaskPriority::from_overdue_days(0),
        TaskPriority::Normal
    ));
    assert!(matches!(
        TaskPriority::from_overdue_days(29),
        TaskPriority::Normal
    ));
    assert!(matches!(
        TaskPriority::from_overdue_days(30),
        TaskPriority::High
    ));
    assert!(matches!(
        TaskPriority::from_overdue_days(90),
        TaskPriority::High
    ));
    assert!(matches!(
        TaskPriority::from_overdue_days(91),
        TaskPriority::Urgent
    ));
    assert!(matches!(
        TaskPriority::from_overdue_days(365),
        TaskPriority::Urgent
    ));
}

#[test]
fn test_overdue_type_priority_alignment() {
    // 同一分级带内类型与优先级的一致性：30-90 天必然 visit+high，
    // >90 天必然 letter+urgent（防止两公式漂移不同步）
    for days in [0i64, 15, 29, 30, 45, 90, 91, 120, 365] {
        let (ty, pr) = (
            TaskType::from_overdue_days(days),
            TaskPriority::from_overdue_days(days),
        );
        match days {
            0..=29 => {
                assert!(matches!(ty, TaskType::Phone) && matches!(pr, TaskPriority::Normal));
            }
            30..=90 => {
                assert!(matches!(ty, TaskType::Visit) && matches!(pr, TaskPriority::High));
            }
            _ => {
                assert!(matches!(ty, TaskType::Letter) && matches!(pr, TaskPriority::Urgent));
            }
        }
    }
}
