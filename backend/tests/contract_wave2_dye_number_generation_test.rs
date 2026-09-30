//! 批次 #137 第四子批（染整/生产域自动编码）契约测试：
//! 验证染整/生产域各单据前缀常量集中定义、且与原手写格式的业务前缀同源延续，
//! 防止后续重构把前缀改名导致业务识别键漂移（缸号/配方号/流转卡号/反馈单号/
//! 打样通知单号/复样单号/领用单号/计量设备号/能耗记录号/分摊规则号/分摊记录号）。
//!
//! 取号本身（事务内 advisory 锁 + 流水探测 + 23505 保存点重试）由
//! `utils/number_generator.rs` 及其既有测试（number_generator_*_test.rs）覆盖；
//! 本文件只锁"前缀语义不变 + 常量单一来源"这一层契约。

use bingxi_backend::handlers::dye_batch_handler::DYE_BATCH_NO_PREFIX;
use bingxi_backend::services::chemical_ops::requisition::CHEMICAL_REQUISITION_NO_PREFIX;
use bingxi_backend::services::dye_recipe_service::DYE_RECIPE_NO_PREFIX;
use bingxi_backend::services::energy_ops::allocation_record::ENERGY_ALLOCATION_NO_PREFIX;
use bingxi_backend::services::energy_ops::allocation_rule::ENERGY_ALLOCATION_RULE_NO_PREFIX;
use bingxi_backend::services::energy_ops::consumption::ENERGY_CONSUMPTION_NO_PREFIX;
use bingxi_backend::services::energy_ops::meter::ENERGY_METER_NO_PREFIX;
use bingxi_backend::services::flow_card_service::{
    FLOW_CARD_NO_PREFIX, FlowCardService, QUALITY_FEEDBACK_NO_PREFIX, QualityFeedbackService,
};
use bingxi_backend::services::lab_dip_service::{
    LAB_DIP_REQUEST_NO_PREFIX, LAB_DIP_RESAMPLE_NO_PREFIX, LabDipRequestService,
    LabDipResampleService,
};

/// 取字符串开头的连续大写字母段（"FC-20260920-001" → "FC"，"DB20260920001" → "DB"）
fn leading_alpha(s: &str) -> String {
    s.chars().take_while(|c| c.is_ascii_uppercase()).collect()
}

/// 各前缀常量必须为大写字母段（生成器格式 {前缀}{YYYYMMDD}{流水} 不允许含分隔符）
#[test]
fn test_prefix_constants_are_pure_uppercase_alpha() {
    for prefix in [
        DYE_BATCH_NO_PREFIX,
        DYE_RECIPE_NO_PREFIX,
        FLOW_CARD_NO_PREFIX,
        QUALITY_FEEDBACK_NO_PREFIX,
        LAB_DIP_REQUEST_NO_PREFIX,
        LAB_DIP_RESAMPLE_NO_PREFIX,
        CHEMICAL_REQUISITION_NO_PREFIX,
        ENERGY_METER_NO_PREFIX,
        ENERGY_CONSUMPTION_NO_PREFIX,
        ENERGY_ALLOCATION_RULE_NO_PREFIX,
        ENERGY_ALLOCATION_NO_PREFIX,
    ] {
        assert!(!prefix.is_empty(), "前缀常量不得为空");
        assert!(
            leading_alpha(prefix) == prefix,
            "前缀 {prefix} 必须是纯大写字母（不允许 '-' 等分隔符混入生成器前缀）"
        );
    }
}

/// 前缀语义延续原手写格式（业务识别键不许改名）：
/// 缸号 DB / 配方号 DR / 流转卡号 FC / 反馈单号 QF / 打样通知单号 LD /
/// 复样单号 RS / 领用单号 CR / 计量设备号 EM / 能耗记录号 EC / 分摊规则与记录 EAR
#[test]
fn test_prefix_constants_match_business_legacy_prefixes() {
    assert_eq!(DYE_BATCH_NO_PREFIX, "DB");
    assert_eq!(DYE_RECIPE_NO_PREFIX, "DR");
    assert_eq!(FLOW_CARD_NO_PREFIX, "FC");
    assert_eq!(QUALITY_FEEDBACK_NO_PREFIX, "QF");
    assert_eq!(LAB_DIP_REQUEST_NO_PREFIX, "LD");
    assert_eq!(LAB_DIP_RESAMPLE_NO_PREFIX, "RS");
    assert_eq!(CHEMICAL_REQUISITION_NO_PREFIX, "CR");
    assert_eq!(ENERGY_METER_NO_PREFIX, "EM");
    assert_eq!(ENERGY_CONSUMPTION_NO_PREFIX, "EC");
    // EAR 为分摊规则/分摊记录两表历史共用的业务前缀（既有语义，本批只改取号方式）
    assert_eq!(ENERGY_ALLOCATION_RULE_NO_PREFIX, "EAR");
    assert_eq!(ENERGY_ALLOCATION_NO_PREFIX, "EAR");
}

/// 新旧同源：保留给存量单测的遗留手写取号函数产出的前缀字母段，
/// 必须与新的事务内取号前缀常量完全一致（防止常量与原业务前缀漂移）。
#[test]
fn test_new_constants_same_origin_as_legacy_generators() {
    let legacy_card_no = FlowCardService::generate_card_no(); // FC-{ts}-{n}
    assert_eq!(leading_alpha(&legacy_card_no), FLOW_CARD_NO_PREFIX);

    let legacy_feedback_no = QualityFeedbackService::generate_feedback_no(); // QF-{ts}-{n}
    assert_eq!(
        leading_alpha(&legacy_feedback_no),
        QUALITY_FEEDBACK_NO_PREFIX
    );

    let legacy_request_no = LabDipRequestService::generate_request_no(); // LD-{ts}-{n}
    assert_eq!(leading_alpha(&legacy_request_no), LAB_DIP_REQUEST_NO_PREFIX);

    let legacy_resample_no = LabDipResampleService::generate_resample_no(); // RS-{ts}-{n}
    assert_eq!(
        leading_alpha(&legacy_resample_no),
        LAB_DIP_RESAMPLE_NO_PREFIX
    );

    let legacy_recipe_no =
        bingxi_backend::services::dye_recipe_service::DyeRecipeService::generate_recipe_no(None);
    assert_eq!(leading_alpha(&legacy_recipe_no), DYE_RECIPE_NO_PREFIX);
}

/// 除文档化的 EAR 共用以外的前缀两两互不相同，
/// 防止复制粘贴时把别的单据前缀带进新取号点。
#[test]
fn test_prefixes_distinct_except_documented_ear_sharing() {
    let all = [
        DYE_BATCH_NO_PREFIX,
        DYE_RECIPE_NO_PREFIX,
        FLOW_CARD_NO_PREFIX,
        QUALITY_FEEDBACK_NO_PREFIX,
        LAB_DIP_REQUEST_NO_PREFIX,
        LAB_DIP_RESAMPLE_NO_PREFIX,
        CHEMICAL_REQUISITION_NO_PREFIX,
        ENERGY_METER_NO_PREFIX,
        ENERGY_CONSUMPTION_NO_PREFIX,
        ENERGY_ALLOCATION_RULE_NO_PREFIX,
        ENERGY_ALLOCATION_NO_PREFIX,
    ];
    let unique: std::collections::HashSet<&str> = all.iter().copied().collect();
    // 11 个点位、10 个不同值：唯一的重复对是规则/记录共用的 EAR
    assert_eq!(unique.len(), all.len() - 1);
    assert_eq!(
        ENERGY_ALLOCATION_RULE_NO_PREFIX,
        ENERGY_ALLOCATION_NO_PREFIX
    );
}
