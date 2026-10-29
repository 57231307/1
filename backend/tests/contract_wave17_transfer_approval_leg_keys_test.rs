//! 客户转移审批腿权限键派生与补授静态锁（不连库、不起服务）
//!
//! 功能：钉死三件事——
//! 1. 键派生：POST /crm/transfer-approvals/{id}/manager-approve 与 director-approve 的
//!    运行时动作键各自独立（不再按方法塌缩成 create），建单腿仍派生 create、取消腿仍派生
//!    cancel；派生一律走生产谓词 `middleware/permission.rs::extract_action_from_path`，
//!    测试不复制第二套关键字表。
//! 2. 目录同源：两枚审批动作键可经 `assignable_actions()` 同源导出（目录勾得出、运行时
//!    才派生得出，杜绝目录/鉴权两套动作词表漂移）；资源名 transfer-approvals 已登记进
//!    `PERMISSION_RESOURCES`（未登记即授予也不命中的死键）。
//! 3. 补授与职责分离（SoD）：矩阵里建单键持岗（sales_rep）不含任何审批键；经理腿键只授
//!    sales_manager；总监腿键只授 sales_director；经理/总监两岗均不持有 create 键。
//!    矩阵定义表读取生产入口 `InitService::all_role_permission_definition_groups()`，
//!    与播种闸门①②共用同一份数据，不在 tests/ 侧另抄副本。
//! 调用方：CI 的 Rust 集成测试任务（静态锁）。
//! 入参：无外部输入；全部现读生产判据与生产定义表。
//! 传给谁：断言结果只进 CI 报告。
//! 存什么/存哪里：不落库、不写任何文件。

use bingxi_backend::middleware::permission::{assignable_actions, extract_action_from_path};
use bingxi_backend::services::init_service::{InitService, PERMISSION_RESOURCES};

/// 三枚腿路径的运行时动作键派生断言（含记录 ID 的形态与生产 URL 逐段同构）。
#[test]
fn approval_legs_derive_distinct_action_keys() {
    assert_eq!(
        extract_action_from_path("/api/v1/erp/crm/transfer-approvals/123/manager-approve")
            .as_deref(),
        Some("manager-approve"),
        "经理腿末段必须派生独立动作键 manager-approve（塌缩成 create 即职责分离失效）"
    );
    assert_eq!(
        extract_action_from_path("/api/v1/erp/crm/transfer-approvals/123/director-approve")
            .as_deref(),
        Some("director-approve"),
        "总监腿末段必须派生独立动作键 director-approve"
    );
    assert_eq!(
        extract_action_from_path("/api/v1/erp/crm/transfer-approvals/123/cancel").as_deref(),
        Some("cancel"),
        "取消腿派生既有 cancel 键，不受新关键字影响"
    );
    assert_eq!(
        extract_action_from_path("/api/v1/erp/crm/transfer-approvals"),
        None,
        "建单腿末段不是动作关键字，按方法派生 create（本锁断言 None 后由方法映射承接）"
    );
}

/// 目录同源：两枚审批动作键在 `assignable_actions()` 导出集内，资源名已登记。
#[test]
fn approval_action_keys_and_resource_share_single_source() {
    let actions = assignable_actions();
    for key in ["manager-approve", "director-approve"] {
        assert!(
            actions.iter().any(|a| *a == key),
            "动作键 {key} 必须经 PATH_ACTION_KEYWORDS 同源导出进权限目录，实得: {actions:?}"
        );
    }
    assert!(
        PERMISSION_RESOURCES.contains(&"transfer-approvals"),
        "资源名 transfer-approvals 必须登记进 PERMISSION_RESOURCES（未登记即死键）"
    );
}

/// 收集矩阵里某动作键的全部持键角色（现读生产定义表，不抄副本）。
fn roles_holding(action: &str) -> Vec<&'static str> {
    let mut holders = Vec::new();
    for definitions in InitService::all_role_permission_definition_groups() {
        for (role_code, resources) in definitions {
            if resources
                .iter()
                .any(|(resource, act)| *resource == "transfer-approvals" && *act == action)
            {
                holders.push(*role_code);
            }
        }
    }
    holders
}

/// SoD 锁：建单键、经理腿键、总监腿键三岗分离，任何持审批键的角色不得同时持建单键。
#[test]
fn create_and_approve_keys_are_role_separated() {
    let creators = roles_holding("create");
    let managers = roles_holding("manager-approve");
    let directors = roles_holding("director-approve");

    assert!(
        creators.contains(&"sales_rep"),
        "建单键应补授给发起转移申请的销售代表岗，实得: {creators:?}"
    );
    assert_eq!(
        managers,
        vec!["sales_manager"],
        "经理腿键必须只授销售经理一岗（同批补授收细口径），实得: {managers:?}"
    );
    assert_eq!(
        directors,
        vec!["sales_director"],
        "总监腿键必须只授销售总监一岗（同批补授收细口径），实得: {directors:?}"
    );
    for role in managers.iter().chain(directors.iter()) {
        assert!(
            !creators.contains(role),
            "角色 {role} 同时持有建单键与审批键，职责分离被破坏（能建单的人不能自批）"
        );
    }
    for role in creators.iter() {
        assert!(
            !managers.contains(role) && !directors.contains(role),
            "建单持岗 {role} 不得出现在任何审批腿持键集合中"
        );
    }
}
