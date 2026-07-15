use crate::models::quality_inspection;
use crate::models::quality_inspection_record;
use crate::models::unqualified_product;
// 批次 212 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::utils::error::AppError;
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, Order, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

// =====================================================
// v14 批次 421 T-P1-4：面料行业质检 A/B/C 级分级判定
// 依据：fabric-industry-research.md §4.7 质量检验模块
// 业务规则：
//   A 级（合格）：qualification_rate >= 95%，正常入库销售
//   B 级（让步接收）：qualification_rate >= 80% 且 < 95%，降级销售（影响定价）
//   C 级（不合格）：qualification_rate < 80%，返工或报废
// =====================================================
pub const QUALITY_GRADE_A: &str = "A"; // 合格
pub const QUALITY_GRADE_B: &str = "B"; // 让步接收，降级销售
pub const QUALITY_GRADE_C: &str = "C"; // 不合格，返工或报废

// 合格率分级阈值（百分比，0-100）
// 注意：Decimal::new 非 const fn，用函数返回避免 const 上下文限制
pub fn grade_a_threshold() -> Decimal {
    Decimal::new(95, 0) // 95%
}
pub fn grade_b_threshold() -> Decimal {
    Decimal::new(80, 0) // 80%
}

// 不合格品处理方式常量（依据调研文档 §4.7 质检结果分级）
pub const HANDLING_DOWNGRADE_SALE: &str = "downgrade_sale"; // B 级降级销售
pub const HANDLING_REWORK: &str = "rework"; // C 级返工
pub const HANDLING_SCRAP: &str = "scrap"; // C 级报废

/// 根据合格率判定面料行业质检等级（A/B/C）
///
/// 业务规则（fabric-industry-research.md §4.7）：
/// - rate >= 95% → A 级（合格）
/// - 80% <= rate < 95% → B 级（让步接收，降级销售）
/// - rate < 80% → C 级（不合格，返工或报废）
///
/// 入参 qualification_rate 为百分比形式（0-100），None 视为 0% 处理为 C 级
pub fn determine_quality_grade(qualification_rate: Option<Decimal>) -> String {
    let rate = qualification_rate.unwrap_or(Decimal::ZERO);
    if rate >= grade_a_threshold() {
        QUALITY_GRADE_A.to_string()
    } else if rate >= grade_b_threshold() {
        QUALITY_GRADE_B.to_string()
    } else {
        QUALITY_GRADE_C.to_string()
    }
}

/// 根据质检等级校验处理方式是否符合面料行业业务规则
///
/// A 级品无需处理（合格）；B 级品必须降级销售；C 级品必须返工或报废
pub fn validate_handling_method_by_grade(
    grade: &str,
    handling_method: &str,
) -> Result<(), AppError> {
    match grade {
        QUALITY_GRADE_A => {
            // A 级品合格，无需不合格处理
            Err(AppError::business(
                "A 级（合格）品无需进行不合格处理，请检查等级判定",
            ))
        }
        QUALITY_GRADE_B => {
            // B 级品必须降级销售
            if handling_method == HANDLING_DOWNGRADE_SALE {
                Ok(())
            } else {
                Err(AppError::business(format!(
                    "B 级（让步接收）品处理方式必须为 {}（降级销售），当前：{}",
                    HANDLING_DOWNGRADE_SALE, handling_method
                )))
            }
        }
        QUALITY_GRADE_C => {
            // C 级品必须返工或报废
            if handling_method == HANDLING_REWORK || handling_method == HANDLING_SCRAP {
                Ok(())
            } else {
                Err(AppError::business(format!(
                    "C 级（不合格）品处理方式必须为 {}（返工）或 {}（报废），当前：{}",
                    HANDLING_REWORK, HANDLING_SCRAP, handling_method
                )))
            }
        }
        _ => Err(AppError::business(format!(
            "未知质检等级：{}，有效值为 A/B/C",
            grade
        ))),
    }
}

#[derive(Debug, Clone, Default)]
pub struct QualityInspectionQueryParams {
    pub inspection_type: Option<String>,
    pub status: Option<String>,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateQualityInspectionStandardRequest {
    pub standard_name: String,
    pub standard_code: String,
    pub inspection_type: String,
    pub product_id: Option<i32>,
    pub product_category_id: Option<i32>,
    pub inspection_items: Option<serde_json::Value>,
    pub sampling_method: Option<String>,
    pub sampling_rate: Option<Decimal>,
    pub acceptance_criteria: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInspectionRecordRequest {
    pub inspection_no: String,
    pub inspection_type: String,
    pub related_type: Option<String>,
    pub related_id: Option<i32>,
    pub product_id: i32,
    pub batch_no: Option<String>,
    pub supplier_id: Option<i32>,
    pub customer_id: Option<i32>,
    pub inspection_date: NaiveDate,
    pub inspector_id: Option<i32>,
    pub total_qty: Decimal,
    pub inspected_qty: Decimal,
    pub qualified_qty: Option<Decimal>,
    pub unqualified_qty: Option<Decimal>,
    pub qualification_rate: Option<Decimal>,
    pub inspection_result: String,
    pub remark: Option<String>,
    // v14 批次 421 T-P1-4：面料行业质检等级 A/B/C
    // 可选字段：None 时由 determine_quality_grade 根据 qualification_rate 自动判定
    pub grade: Option<String>,
    // v14 批次 421：按缸号追溯质检结果
    pub color_no: Option<String>,
    pub dye_lot_no: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessUnqualifiedRequest {
    pub unqualified_qty: Decimal,
    pub unqualified_reason: String,
    pub handling_method: String,
    pub remark: Option<String>,
    // v14 批次 421 T-P1-4：处理结果（降级销售单价/返工工时/报废损失金额）
    pub handling_result: Option<String>,
}

pub struct QualityInspectionService {
    db: Arc<DatabaseConnection>,
}

impl QualityInspectionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn get_standards_list(
        &self,
        params: QualityInspectionQueryParams,
    ) -> Result<(Vec<quality_inspection::Model>, u64), AppError> {
        let mut query = quality_inspection::Entity::find();

        if let Some(inspection_type) = &params.inspection_type {
            query = query.filter(quality_inspection::Column::InspectionType.eq(inspection_type));
        }

        if let Some(status) = &params.status {
            query = query.filter(quality_inspection::Column::Status.eq(status));
        }

        let total = query.clone().count(&*self.db).await?;

        let standards = query
            .order_by(quality_inspection::Column::Id, Order::Desc)
            .offset((params.page.clamp(1, 1000).saturating_sub(1) * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((standards, total))
    }

    pub async fn create_standard(
        &self,
        req: CreateQualityInspectionStandardRequest,
        user_id: i32,
    ) -> Result<quality_inspection::Model, AppError> {
        info!(
            "用户 {} 正在创建质量检验标准：{}",
            user_id, req.standard_code
        );

        let active_model = quality_inspection::ActiveModel {
            standard_name: Set(req.standard_name),
            standard_code: Set(req.standard_code),
            product_id: Set(req.product_id),
            product_category_id: Set(req.product_category_id),
            inspection_type: Set(req.inspection_type),
            inspection_items: Set(req.inspection_items),
            sampling_method: Set(req.sampling_method),
            sampling_rate: Set(req.sampling_rate),
            acceptance_criteria: Set(req.acceptance_criteria),
            status: Set(master_data::ACTIVE.to_string()),
            ..Default::default()
        };

        let result = active_model.insert(&*self.db).await?;
        info!("质量检验标准创建成功：{}", result.standard_code);
        Ok(result)
    }

    pub async fn get_standard_by_id(&self, id: i32) -> Result<quality_inspection::Model, AppError> {
        let standard = quality_inspection::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("质量检验标准不存在：{}", id)))?;
        Ok(standard)
    }

    pub async fn get_record_by_id(
        &self,
        id: i32,
    ) -> Result<quality_inspection_record::Model, AppError> {
        let record = quality_inspection_record::Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("质量检验记录不存在：{}", id)))?;
        Ok(record)
    }

    pub async fn get_records_list(
        &self,
        params: QualityInspectionQueryParams,
    ) -> Result<(Vec<quality_inspection_record::Model>, u64), AppError> {
        let mut query = quality_inspection_record::Entity::find();

        if let Some(inspection_result) = &params.inspection_type {
            query = query
                .filter(quality_inspection_record::Column::InspectionResult.eq(inspection_result));
        }

        let total = query.clone().count(&*self.db).await?;

        let records = query
            .order_by(quality_inspection_record::Column::Id, Order::Desc)
            .offset((params.page.clamp(1, 1000).saturating_sub(1) * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((records, total))
    }

    pub async fn create_record(
        &self,
        req: CreateInspectionRecordRequest,
        user_id: i32,
    ) -> Result<quality_inspection_record::Model, AppError> {
        info!(
            "用户 {} 正在创建质量检验记录：{}",
            user_id, req.inspection_no
        );

        // P1 5-3 修复（批次 63）：整体包裹 txn，质检记录与入库单状态更新原子化
        // 原实现质检记录用 &*self.db 插入，入库单状态更新也用 &*self.db，
        // 两者非事务包裹，并发或中途失败会导致质检记录已创建但入库单状态未更新（数据不一致）。
        use sea_orm::TransactionTrait;
        let txn = (*self.db).begin().await?;

        // v14 批次 421 T-P1-4：面料行业质检等级自动判定
        // grade 未显式提供时由 determine_quality_grade 根据 qualification_rate 自动判定
        // 依据：fabric-industry-research.md §4.7 - A 级 >= 95% / B 级 80-95% / C 级 < 80%
        let grade = req.grade.clone().unwrap_or_else(|| {
            let determined = determine_quality_grade(req.qualification_rate);
            info!(
                "质检记录 {} 未显式指定等级，根据合格率 {:?}% 自动判定为 {} 级",
                req.inspection_no, req.qualification_rate, determined
            );
            determined
        });

        let active_model = quality_inspection_record::ActiveModel {
            inspection_no: Set(req.inspection_no),
            inspection_type: Set(req.inspection_type),
            related_type: Set(req.related_type),
            related_id: Set(req.related_id),
            product_id: Set(req.product_id),
            batch_no: Set(req.batch_no),
            supplier_id: Set(req.supplier_id),
            customer_id: Set(req.customer_id),
            inspection_date: Set(req.inspection_date),
            inspector_id: Set(req.inspector_id),
            total_qty: Set(req.total_qty),
            inspected_qty: Set(req.inspected_qty),
            qualified_qty: Set(req.qualified_qty),
            unqualified_qty: Set(req.unqualified_qty),
            qualification_rate: Set(req.qualification_rate),
            inspection_result: Set(req.inspection_result),
            remark: Set(req.remark),
            grade: Set(Some(grade)),
            color_no: Set(req.color_no),
            dye_lot_no: Set(req.dye_lot_no),
            ..Default::default()
        };

        let result = active_model.insert(&txn).await?;
        info!("质量检验记录创建成功：{}", result.inspection_no);

        // 如果是采购入库的质检，同步更新入库单状态
        if result.related_type.as_deref() == Some("PURCHASE_RECEIPT") {
            if let Some(receipt_id) = result.related_id {
                let receipt = crate::models::purchase_receipt::Entity::find_by_id(receipt_id)
                    .one(&txn)
                    .await?;

                if let Some(r) = receipt {
                    let mut receipt_active: crate::models::purchase_receipt::ActiveModel = r.into();
                    receipt_active.inspection_status = Set(result.inspection_result.clone());
                    crate::services::audit_log_service::AuditLogService::update_with_audit(
                        &txn,
                        "auto_audit",
                        receipt_active,
                        // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
                        Some(user_id),
                    )
                    .await?;
                }
            }
        }

        txn.commit().await?;

        Ok(result)
    }

    pub async fn process_unqualified(
        &self,
        record_id: i32,
        req: ProcessUnqualifiedRequest,
        user_id: i32,
    ) -> Result<unqualified_product::Model, AppError> {
        info!("用户 {} 正在处理不合格品，记录ID：{}", user_id, record_id);

        let record = self.get_record_by_id(record_id).await?;

        // v14 批次 421 T-P1-4：根据质检等级校验处理方式符合面料行业业务规则
        // 依据：fabric-industry-research.md §4.7 - B 级降级销售，C 级返工或报废
        let grade = record.grade.clone().unwrap_or_else(|| {
            // 兼容历史无 grade 字段的质检记录：根据合格率自动判定
            determine_quality_grade(record.qualification_rate)
        });
        validate_handling_method_by_grade(&grade, &req.handling_method)?;

        let unqualified_no = format!("UQ{:08}", record_id);

        let active_model = unqualified_product::ActiveModel {
            unqualified_no: Set(unqualified_no),
            inspection_id: Set(Some(record_id)),
            product_id: Set(record.product_id),
            batch_no: Set(record.batch_no),
            unqualified_qty: Set(req.unqualified_qty),
            unqualified_reason: Set(req.unqualified_reason),
            handling_method: Set(req.handling_method),
            handling_status: Set("pending".to_string()),
            handling_by: Set(None),
            handling_at: Set(None),
            remark: Set(req.remark),
            grade: Set(Some(grade)),
            handling_result: Set(req.handling_result),
            ..Default::default()
        };

        let result = active_model.insert(&*self.db).await?;
        info!("不合格品处理记录创建成功：{}", result.unqualified_no);
        Ok(result)
    }

    pub async fn get_defects_list(
        &self,
        params: QualityInspectionQueryParams,
    ) -> Result<(Vec<unqualified_product::Model>, u64), AppError> {
        let mut query = unqualified_product::Entity::find();

        if let Some(status) = &params.status {
            query = query.filter(unqualified_product::Column::HandlingStatus.eq(status));
        }

        let total = query.clone().count(&*self.db).await?;

        let defects = query
            .order_by(unqualified_product::Column::Id, Order::Desc)
            .offset((params.page.clamp(1, 1000).saturating_sub(1) * params.page_size) as u64)
            .limit(params.page_size as u64)
            .all(&*self.db)
            .await?;

        Ok((defects, total))
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::decs;
    use std::str::FromStr;

    // ===== determine_quality_grade 合格率分级判定 =====

    /// 测试_质检分级_A级_合格率达标
    ///
    /// 验证 qualification_rate >= 95% 判定为 A 级（合格），覆盖边界值 95%。
    #[test]
    fn 测试_质检分级_A级_合格率达标() {
        // 边界：恰好 95% → A 级
        assert_eq!(
            determine_quality_grade(Some(decs!("95"))),
            QUALITY_GRADE_A
        );
        // 高于 95% → A 级
        assert_eq!(
            determine_quality_grade(Some(decs!("100"))),
            QUALITY_GRADE_A
        );
        assert_eq!(
            determine_quality_grade(Some(decs!("99.5"))),
            QUALITY_GRADE_A
        );
    }

    /// 测试_质检分级_B级_让步接收区间
    ///
    /// 验证 80% <= rate < 95% 判定为 B 级（让步接收，降级销售）。
    #[test]
    fn 测试_质检分级_B级_让步接收区间() {
        // 边界：恰好 80% → B 级
        assert_eq!(
            determine_quality_grade(Some(decs!("80"))),
            QUALITY_GRADE_B
        );
        // 区间内 → B 级
        assert_eq!(
            determine_quality_grade(Some(decs!("85"))),
            QUALITY_GRADE_B
        );
        assert_eq!(
            determine_quality_grade(Some(decs!("94.99"))),
            QUALITY_GRADE_B
        );
    }

    /// 测试_质检分级_C级_不合格区间
    ///
    /// 验证 rate < 80% 判定为 C 级（不合格，返工或报废）。
    #[test]
    fn 测试_质检分级_C级_不合格区间() {
        // 边界：恰好低于 80% → C 级
        assert_eq!(
            determine_quality_grade(Some(decs!("79.99"))),
            QUALITY_GRADE_C
        );
        assert_eq!(
            determine_quality_grade(Some(decs!("50"))),
            QUALITY_GRADE_C
        );
        assert_eq!(determine_quality_grade(Some(Decimal::ZERO)), QUALITY_GRADE_C);
    }

    /// 测试_质检分级_None视为零合格率
    ///
    /// 验证 qualification_rate 为 None 时按 0% 处理为 C 级。
    #[test]
    fn 测试_质检分级_None视为零合格率() {
        assert_eq!(determine_quality_grade(None), QUALITY_GRADE_C);
    }

    // ===== validate_handling_method_by_grade 等级与处理方式匹配校验 =====

    /// 测试_等级处理方式校验_A级品无需不合格处理
    ///
    /// A 级（合格）品调用任何处理方式都应返回错误。
    #[test]
    fn 测试_等级处理方式校验_A级品无需不合格处理() {
        // A 级 + 任意处理方式 → 拒绝
        assert!(validate_handling_method_by_grade(QUALITY_GRADE_A, HANDLING_DOWNGRADE_SALE).is_err());
        assert!(validate_handling_method_by_grade(QUALITY_GRADE_A, HANDLING_REWORK).is_err());
        assert!(validate_handling_method_by_grade(QUALITY_GRADE_A, HANDLING_SCRAP).is_err());

        let err = validate_handling_method_by_grade(QUALITY_GRADE_A, HANDLING_REWORK).unwrap_err();
        assert!(err.to_string().contains("A 级"));
    }

    /// 测试_等级处理方式校验_B级品必须降级销售
    ///
    /// B 级（让步接收）品处理方式必须为 downgrade_sale，其他拒绝。
    #[test]
    fn 测试_等级处理方式校验_B级品必须降级销售() {
        // B 级 + 降级销售 → 放行
        assert!(validate_handling_method_by_grade(QUALITY_GRADE_B, HANDLING_DOWNGRADE_SALE).is_ok());
        // B 级 + 返工/报废 → 拒绝
        assert!(validate_handling_method_by_grade(QUALITY_GRADE_B, HANDLING_REWORK).is_err());
        assert!(validate_handling_method_by_grade(QUALITY_GRADE_B, HANDLING_SCRAP).is_err());

        let err = validate_handling_method_by_grade(QUALITY_GRADE_B, HANDLING_REWORK).unwrap_err();
        assert!(err.to_string().contains("B 级"));
        assert!(err.to_string().contains("降级销售"));
    }

    /// 测试_等级处理方式校验_C级品返工或报废
    ///
    /// C 级（不合格）品处理方式必须为 rework 或 scrap，其他拒绝。
    #[test]
    fn 测试_等级处理方式校验_C级品返工或报废() {
        // C 级 + 返工/报废 → 放行
        assert!(validate_handling_method_by_grade(QUALITY_GRADE_C, HANDLING_REWORK).is_ok());
        assert!(validate_handling_method_by_grade(QUALITY_GRADE_C, HANDLING_SCRAP).is_ok());
        // C 级 + 降级销售 → 拒绝
        assert!(validate_handling_method_by_grade(QUALITY_GRADE_C, HANDLING_DOWNGRADE_SALE).is_err());

        let err = validate_handling_method_by_grade(QUALITY_GRADE_C, HANDLING_DOWNGRADE_SALE).unwrap_err();
        assert!(err.to_string().contains("C 级"));
        assert!(err.to_string().contains("返工"));
        assert!(err.to_string().contains("报废"));
    }

    /// 测试_等级处理方式校验_未知等级拒绝
    ///
    /// grade 为 D/X/空字符串等非 A/B/C 值时返回错误。
    #[test]
    fn 测试_等级处理方式校验_未知等级拒绝() {
        assert!(validate_handling_method_by_grade("D", HANDLING_REWORK).is_err());
        assert!(validate_handling_method_by_grade("X", HANDLING_SCRAP).is_err());
        assert!(validate_handling_method_by_grade("", HANDLING_DOWNGRADE_SALE).is_err());

        let err = validate_handling_method_by_grade("D", HANDLING_REWORK).unwrap_err();
        assert!(err.to_string().contains("未知质检等级"));
    }

    /// 测试_等级常量值正确性
    ///
    /// 校验 A/B/C 等级常量与处理方式常量值，避免硬编码字符串拼写错误。
    #[test]
    fn 测试_等级常量值正确性() {
        assert_eq!(QUALITY_GRADE_A, "A");
        assert_eq!(QUALITY_GRADE_B, "B");
        assert_eq!(QUALITY_GRADE_C, "C");
        assert_eq!(grade_a_threshold(), decs!("95"));
        assert_eq!(grade_b_threshold(), decs!("80"));
        assert_eq!(HANDLING_DOWNGRADE_SALE, "downgrade_sale");
        assert_eq!(HANDLING_REWORK, "rework");
        assert_eq!(HANDLING_SCRAP, "scrap");
    }

    /// 测试_decimal阈值解析正确性
    ///
    /// 校验 grade_a_threshold / grade_b_threshold 通过 Decimal::new 构造的值正确。
    #[test]
    fn 测试_decimal阈值解析正确性() {
        let d = Decimal::from_str("95").unwrap();
        assert_eq!(grade_a_threshold(), d);
        let d = Decimal::from_str("80").unwrap();
        assert_eq!(grade_b_threshold(), d);
    }
}
