use bingxi_backend::models::status::dye_recipe as recipe_status;
use bingxi_backend::services::dye_recipe_service::DyeRecipeService;
use bingxi_backend::services::period_adjustment_service::*;

/// 测试配方编号自动生成格式
#[test]
fn test_generate_recipe_no_auto() {
    let no = DyeRecipeService::generate_recipe_no(None);
    assert!(no.starts_with("DR-"));
    // 格式：DR-{14位时间戳}-{4位随机}
    let parts: Vec<&str> = no.split('-').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[1].len(), 14); // 时间戳 YYYYMMDDHHMMSS
    assert_eq!(parts[2].len(), 4); // 4 位随机
}

/// 测试配方编号使用调用方提供的值
#[test]
fn test_generate_recipe_no_provided() {
    let no = DyeRecipeService::generate_recipe_no(Some("CUSTOM-001"));
    assert_eq!(no, "CUSTOM-001");
}

/// 测试配方编号空字符串时自动生成
#[test]
fn test_generate_recipe_no_empty() {
    let no = DyeRecipeService::generate_recipe_no(Some(""));
    assert!(no.starts_with("DR-"));
}

/// 测试状态流转：草稿 → 已审核（合法）
#[test]
fn test_status_transition_draft_to_approved() {
    assert!(
        DyeRecipeService::validate_status_transition(recipe_status::DRAFT, recipe_status::APPROVED)
            .is_ok()
    );
}

/// 测试状态流转：草稿 → 已停用（合法）
#[test]
fn test_status_transition_draft_to_disabled() {
    assert!(
        DyeRecipeService::validate_status_transition(recipe_status::DRAFT, recipe_status::DISABLED)
            .is_ok()
    );
}

/// 测试状态流转：已审核 → 已停用（合法）
#[test]
fn test_status_transition_approved_to_disabled() {
    assert!(
        DyeRecipeService::validate_status_transition(
            recipe_status::APPROVED,
            recipe_status::DISABLED
        )
        .is_ok()
    );
}

/// 测试状态流转：已停用 → 已审核（合法）
#[test]
fn test_status_transition_disabled_to_approved() {
    assert!(
        DyeRecipeService::validate_status_transition(
            recipe_status::DISABLED,
            recipe_status::APPROVED
        )
        .is_ok()
    );
}

/// 测试状态流转：草稿 → 草稿（非法，不能自转）
#[test]
fn test_status_transition_draft_to_draft_invalid() {
    assert!(
        DyeRecipeService::validate_status_transition(recipe_status::DRAFT, recipe_status::DRAFT)
            .is_err()
    );
}

/// 测试状态流转：已审核 → 草稿（非法，不能回退到草稿）
#[test]
fn test_status_transition_approved_to_draft_invalid() {
    assert!(
        DyeRecipeService::validate_status_transition(recipe_status::APPROVED, recipe_status::DRAFT)
            .is_err()
    );
}

/// 测试状态流转：已审核 → 已审核（非法，不能自转）
#[test]
fn test_status_transition_approved_to_approved_invalid() {
    assert!(
        DyeRecipeService::validate_status_transition(
            recipe_status::APPROVED,
            recipe_status::APPROVED
        )
        .is_err()
    );
}

/// 测试状态流转：未知状态（非法）
#[test]
fn test_status_transition_unknown_status() {
    assert!(DyeRecipeService::validate_status_transition("未知", recipe_status::APPROVED).is_err());
}

/// 测试删除校验：已审核配方不允许删除
#[test]
fn test_validate_can_delete_approved() {
    assert!(DyeRecipeService::validate_can_delete(Some(recipe_status::APPROVED)).is_err());
}

/// 测试删除校验：草稿配方允许删除
#[test]
fn test_validate_can_delete_draft() {
    assert!(DyeRecipeService::validate_can_delete(Some(recipe_status::DRAFT)).is_ok());
}

/// 测试删除校验：已停用配方允许删除
#[test]
fn test_validate_can_delete_disabled() {
    assert!(DyeRecipeService::validate_can_delete(Some(recipe_status::DISABLED)).is_ok());
}

/// 测试删除校验：None 状态允许删除
#[test]
fn test_validate_can_delete_none() {
    assert!(DyeRecipeService::validate_can_delete(None).is_ok());
}

/// 测试审核校验：草稿状态可审核
#[test]
fn test_validate_can_approve_draft() {
    assert!(DyeRecipeService::validate_can_approve(Some(recipe_status::DRAFT)).is_ok());
}

/// 测试审核校验：已审核状态不可审核
#[test]
fn test_validate_can_approve_approved() {
    assert!(DyeRecipeService::validate_can_approve(Some(recipe_status::APPROVED)).is_err());
}

/// 测试审核校验：已停用状态不可审核
#[test]
fn test_validate_can_approve_disabled() {
    assert!(DyeRecipeService::validate_can_approve(Some(recipe_status::DISABLED)).is_err());
}

/// 测试创建版本校验：已审核状态可创建新版本
#[test]
fn test_validate_can_create_version_approved() {
    assert!(DyeRecipeService::validate_can_create_version(Some(recipe_status::APPROVED)).is_ok());
}

/// 测试创建版本校验：草稿状态不可创建新版本
#[test]
fn test_validate_can_create_version_draft() {
    assert!(DyeRecipeService::validate_can_create_version(Some(recipe_status::DRAFT)).is_err());
}

/// 测试创建版本校验：已停用状态不可创建新版本
#[test]
fn test_validate_can_create_version_disabled() {
    assert!(DyeRecipeService::validate_can_create_version(Some(recipe_status::DISABLED)).is_err());
}

/// 测试状态常量值正确性
#[test]
fn test_status_constants() {
    assert_eq!(recipe_status::DRAFT, "草稿");
    assert_eq!(recipe_status::APPROVED, "已审核");
    assert_eq!(recipe_status::DISABLED, "已停用");
}
