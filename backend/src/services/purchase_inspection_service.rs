//! 采购质检 Service
//!
//! 采购质检服务层，负责采购质检的核心业务逻辑

use crate::models::purchase_inspection;
use crate::models::purchase_inspection_item;
use crate::models::status::purchase_inspection as pis_status;
use crate::models::status::purchase_inventory::purchase_inspection_result;
use crate::models::{purchase_receipt, supplier, user};
use crate::utils::error::AppError;
// 批次 258 修复：接入 paginate_with_total 统一分页逻辑
use crate::utils::pagination::paginate_with_total;
use crate::utils::sql_escape::safe_like_pattern;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, ExprTrait, FromQueryResult,
    JoinType, Order, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, RelationTrait, Select,
    Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

/// 采购质检单读模型：实体列 + LEFT JOIN 关联出的入库单号、供应商名、质检员名。
///
/// 三个 JOIN 名列均 `Option<String>`（关联可空 / 缺失时为 NULL）；实体自身列按
/// `purchase_inspection` 的约束保持原类型（NOT NULL 列非可选）。
#[derive(Debug, Clone, Serialize, FromQueryResult)]
pub struct PurchaseInspectionView {
    pub id: i32,
    pub inspection_no: String,
    pub receipt_id: Option<i32>,
    pub order_id: Option<i32>,
    pub supplier_id: i32,
    pub inspection_date: chrono::NaiveDate,
    pub inspector_id: Option<i32>,
    pub inspection_type: Option<String>,
    pub sample_size: Option<Decimal>,
    pub defect_count: Option<i32>,
    pub pass_quantity: Option<Decimal>,
    pub reject_quantity: Option<Decimal>,
    pub inspection_status: Option<String>,
    pub inspection_result: Option<String>,
    pub quality_score: Option<Decimal>,
    pub defect_description: Option<String>,
    pub attachment_urls: Option<String>,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_by: Option<i32>,
    pub receipt_no: Option<String>,
    pub supplier_name: Option<String>,
    pub inspector_name: Option<String>,
}

/// 解析日期筛选边界：前端下发 ISO/`YYYY-MM-DD` 字符串，取日期部分转 `NaiveDate`；解析失败视为未提供。
fn parse_date_bound(raw: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(&raw[..raw.len().min(10)], "%Y-%m-%d").ok()
}

/// 采购质检服务
pub struct PurchaseInspectionService {
    db: Arc<DatabaseConnection>,
}

impl PurchaseInspectionService {
    /// 创建服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // 生成质检单号
    // 格式：IQ + 年月日 + 三位序号（IQ20260315001）
    crate::impl_generate_no!(
        generate_inspection_no,
        "PI",
        purchase_inspection::Entity,
        purchase_inspection::Column::InspectionNo
    );

    /// 创建采购质检单
    pub async fn create_inspection(
        &self,
        req: CreatePurchaseInspectionRequest,
        _user_id: i32,
    ) -> Result<purchase_inspection::Model, AppError> {
        // 引用存在性校验先于任何写库动作（含取号）：receipt_id 指向的入库单不存在时
        // 显式 404（含 ID 的真实原因走脱敏 not_found），不把坏引用交给 DB 外键行为
        // 兜底——外键违约会是裸 500 而非业务 4xx。
        if let Some(receipt_id) = req.receipt_id {
            purchase_receipt::Entity::find_by_id(receipt_id)
                .one(&*self.db)
                .await?
                .ok_or_else(|| AppError::not_found(format!("采购入库单 {}", receipt_id)))?;
        }

        let inspection_no = self.generate_inspection_no().await?;

        let inspection = purchase_inspection::ActiveModel {
            id: Default::default(),
            inspection_no: Set(inspection_no),
            receipt_id: Set(req.receipt_id),
            order_id: Set(req.order_id),
            // 供应商 ID 缺失时拒绝创建，避免脏 supplier_id=0 记录
            supplier_id: Set(req
                .supplier_id
                .ok_or_else(|| AppError::validation_displayable("采购验收单缺少供应商ID"))?),
            inspection_date: Set(req
                .inspection_date
                .unwrap_or_else(|| Utc::now().date_naive())),
            inspector_id: Set(req.inspector_id),
            inspection_type: Set(req.inspection_type),
            sample_size: Set(req.sample_size),
            defect_count: Set(Some(0)),
            pass_quantity: Set(None),
            reject_quantity: Set(None),
            inspection_status: Set(Some(pis_status::PENDING.to_string())),
            inspection_result: Set(None),
            quality_score: Set(None),
            defect_description: Set(None),
            attachment_urls: Set(None),
            notes: Set(req.notes),
            completed_at: Set(None),
            completed_by: Set(None),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };

        let inspection = inspection.insert(&*self.db).await?;

        Ok(inspection)
    }

    /// 更新采购质检单
    pub async fn update_inspection(
        &self,
        inspection_id: i32,
        req: UpdatePurchaseInspectionRequest,
        user_id: i32,
    ) -> Result<purchase_inspection::Model, AppError> {
        let inspection = purchase_inspection::Entity::find_by_id(inspection_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购质检单 {}", inspection_id)))?;

        if inspection.inspection_status.as_deref() != Some(pis_status::PENDING) {
            return Err(AppError::business(format!(
                "质检单状态不允许修改，当前状态：{:?}",
                inspection.inspection_status
            )));
        }

        let mut inspection_active: purchase_inspection::ActiveModel = inspection.into();

        if let Some(sample_size) = req.sample_size {
            inspection_active.sample_size = Set(Some(sample_size));
        }
        if let Some(defect_description) = req.defect_description {
            inspection_active.defect_description = Set(Some(defect_description));
        }
        if let Some(notes) = req.notes {
            inspection_active.notes = Set(Some(notes));
        }
        inspection_active.updated_at = Set(Utc::now());

        let inspection = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &*self.db,
            "auto_audit",
            inspection_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        Ok(inspection)
    }

    /// 完成采购质检单
    pub async fn complete_inspection(
        &self,
        inspection_id: i32,
        req: CompleteInspectionRequest,
        user_id: i32,
    ) -> Result<purchase_inspection::Model, AppError> {
        // 质检结论白名单强校验先于任何写库：取值域唯一来源是本域权威表
        // purchase_inspection_result（pass/fail/partial，英文小写码，逐字符匹配，
        // 不做大小写/中英转换）。其写入方是前端「完成」三连 prompt 的结论录入
        // pattern（usePiProc.ts，由 utils/purchase-inspection-result.ts 常量构造，
        // 三端同源）；与通用质检记录域中文词表 quality_inspection_result（待检/合格/
        // 不合格）分属两张表两套词表，禁止跨域借用——拿中文表校验本列会把合法生产
        // 数据判成非法。校验与"结论→入库单检验状态"映射共用同一函数
        // to_receipt_inspection_status（Some ⟺ 词表内，同一取值域单点把关），
        // 词表外 → 400 可外显文案（取值清单由权威表 ALL join 生成，不手写第二套），
        // 杜绝"校验一套、映射另一套"的取值域漂移。
        let receipt_inspection_status =
            purchase_inspection_result::to_receipt_inspection_status(&req.inspection_result)
                .ok_or_else(|| {
                    AppError::validation_displayable(format!(
                        "质检结果只能是{}，提交值「{}」不在取值域内",
                        purchase_inspection_result::ALL.join("/"),
                        req.inspection_result
                    ))
                })?;

        let txn = (*self.db).begin().await?;

        // 批次 26 v6 P1 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        let inspection = purchase_inspection::Entity::find_by_id(inspection_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购质检单 {}", inspection_id)))?;

        if inspection.inspection_status.as_deref() != Some(pis_status::PENDING) {
            return Err(AppError::business(format!(
                "质检单状态不允许完成，当前状态：{:?}",
                inspection.inspection_status
            )));
        }

        // 回写目标入库单（receipt_id 可空：为空则本次完成没有入库单检验状态可回写）
        let receipt_id = inspection.receipt_id;

        // 计算质量得分
        let quality_score = self
            .calculate_quality_score(req.pass_quantity, req.reject_quantity)
            .await?;

        let mut inspection_active: purchase_inspection::ActiveModel = inspection.into();
        inspection_active.pass_quantity = Set(Some(req.pass_quantity));
        inspection_active.reject_quantity = Set(Some(req.reject_quantity));
        inspection_active.inspection_result = Set(Some(req.inspection_result));
        inspection_active.inspection_status = Set(Some(pis_status::COMPLETED.to_string()));
        inspection_active.quality_score = Set(Some(quality_score));
        inspection_active.updated_at = Set(Utc::now());

        let inspection = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            inspection_active,
            // P1 1-1 修复（批次 59b）：原 Some(0) 占位符改为真实操作人 user_id
            Some(user_id),
        )
        .await?;

        // 同一事务内把质检结论回写入库单检验状态（purchase_receipt.inspection_status，
        // 大写词表 PENDING/PASSED/REJECTED），映射经权威表同源函数
        // purchase_inspection_result::to_receipt_inspection_status（pass→PASSED，
        // fail/partial→REJECTED，裁定依据见该函数文档注释）。回写失败一律 `?` 上抛、
        // 事务不提交整体回滚——禁止"质检显示已完成但入库单状态未回写"的静默半成功；
        // 关联入库单缺失同样显式报错（含 ID 的真实原因走脱敏 not_found，见 utils/error.rs 口径）。
        match receipt_id {
            Some(id) => {
                let receipt = purchase_receipt::Entity::find_by_id(id)
                    .one(&txn)
                    .await?
                    .ok_or_else(|| {
                        AppError::not_found(format!(
                            "采购质检单 {} 关联的入库单 {} 不存在，检验状态无法回写",
                            inspection_id, id
                        ))
                    })?;
                let mut receipt_active: purchase_receipt::ActiveModel = receipt.into();
                receipt_active.inspection_status = Set(receipt_inspection_status.to_string());
                receipt_active.updated_at = Set(Utc::now());
                crate::services::audit_log_service::AuditLogService::update_with_audit(
                    &txn,
                    "auto_audit",
                    receipt_active,
                    Some(user_id),
                )
                .await?;
            }
            None => {
                tracing::info!(
                    "采购质检单 {} 未关联入库单（receipt_id 为空），本次完成无检验状态可回写",
                    inspection_id
                );
            }
        }

        txn.commit().await?;

        Ok(inspection)
    }

    /// 计算质量得分
    pub async fn calculate_quality_score(
        &self,
        pass_quantity: Decimal,
        reject_quantity: Decimal,
    ) -> Result<Decimal, AppError> {
        let total = pass_quantity + reject_quantity;

        if total == Decimal::new(0, 0) {
            return Ok(Decimal::new(0, 2));
        }

        let score = (pass_quantity / total) * Decimal::new(100, 0);

        Ok(score)
    }

    /// 列表与统计共用的基础查询构造：同一套 LEFT JOIN + 同一套筛选条件。
    ///
    /// 这是"统计分母与列表同源"的**唯一条件构造点**——列表分页 total 与统计卡
    /// 四值都由本函数产出的查询派生，任何筛选口径的修改只需改这一处，杜绝
    /// "列表一套条件、统计另一套条件"的口径分叉（页内计数漂移缺陷的根因即
    /// 分子/分母不同源）。行级权限口径与列表现状一致：本域列表路径不经过
    /// data_scope/归属过滤（handler→service 仅按查询参数过滤），统计沿用同一
    /// 口径，不另行发明行级过滤。
    ///
    /// JOIN 均为主表到 to-one 关联的 LEFT JOIN，不复制行，COUNT 与分页 total
    /// 语义等价；keyword 需要 `purchase_receipt.receipt_no` 参与匹配，故 JOIN
    /// 保留在共用构造内。
    fn base_filtered_query(
        &self,
        status: Option<&str>,
        supplier_id: Option<i32>,
        keyword: Option<&str>,
        result: Option<&str>,
        date_from: Option<&str>,
        date_to: Option<&str>,
    ) -> Select<purchase_inspection::Entity> {
        let mut query = purchase_inspection::Entity::find()
            .join(
                JoinType::LeftJoin,
                purchase_inspection::Relation::Receipt.def(),
            )
            .join(
                JoinType::LeftJoin,
                purchase_inspection::Relation::Supplier.def(),
            )
            .join(
                JoinType::LeftJoin,
                purchase_inspection::Relation::Inspector.def(),
            );

        if let Some(status) = status {
            query = query.filter(purchase_inspection::Column::InspectionStatus.eq(status));
        }
        if let Some(supplier_id) = supplier_id {
            query = query.filter(purchase_inspection::Column::SupplierId.eq(supplier_id));
        }
        // 结果按写入方 token 等值筛选（不校验词表、不做英中转换）
        if let Some(result) = result {
            query = query.filter(purchase_inspection::Column::InspectionResult.eq(result));
        }
        // 关键字：匹配质检单号或入库单号（两列均真正参与过滤）
        if let Some(kw) = keyword.filter(|s| !s.is_empty()) {
            let pattern = safe_like_pattern(kw);
            query = query.filter(
                purchase_inspection::Column::InspectionNo
                    .like(&pattern)
                    .or(purchase_receipt::Column::ReceiptNo.like(&pattern)),
            );
        }
        // 质检日期范围
        if let Some(d) = date_from.and_then(parse_date_bound) {
            query = query.filter(purchase_inspection::Column::InspectionDate.gte(d));
        }
        if let Some(d) = date_to.and_then(parse_date_bound) {
            query = query.filter(purchase_inspection::Column::InspectionDate.lte(d));
        }

        query
    }

    /// 获取质检单列表
    pub async fn list_inspections(
        &self,
        page: u64,
        page_size: u64,
        status: Option<String>,
        supplier_id: Option<i32>,
        keyword: Option<String>,
        result: Option<String>,
        date_from: Option<String>,
        date_to: Option<String>,
    ) -> Result<(Vec<PurchaseInspectionView>, u64), AppError> {
        // 单次查询：实体 + 入库单号 / 供应商名 / 质检员名（均为 LEFT JOIN 名列，无 N+1）。
        // 筛选条件构造与统计端点共用 base_filtered_query（分母同源的唯一条件点）。
        let query = self
            .base_filtered_query(
                status.as_deref(),
                supplier_id,
                keyword.as_deref(),
                result.as_deref(),
                date_from.as_deref(),
                date_to.as_deref(),
            )
            .column_as(purchase_receipt::Column::ReceiptNo, "receipt_no")
            .column_as(supplier::Column::SupplierName, "supplier_name")
            .column_as(user::Column::RealName, "inspector_name");

        // 批次 258 修复：接入 paginate_with_total 统一分页逻辑（内部已处理 saturating_sub(1) 偏移）
        let paginator = query
            .order_by(purchase_inspection::Column::CreatedAt, Order::Desc)
            .into_model::<PurchaseInspectionView>()
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;

        Ok((items, total))
    }

    /// 统计卡聚合（GET /purchase/inspections/stats）：与列表同筛选条件、同分母来源。
    ///
    /// 分桶口径（全部取权威词表常量，与写入侧逐字符同源，禁止内联字符串）：
    /// - `total`：同筛选条件下的全量行数；
    /// - `pending`：inspection_status = purchase_inspection::PENDING；
    /// - `passed`：inspection_result = purchase_inspection_result::PASS；
    /// - `failed`：inspection_result IN (FAIL, PARTIAL)——partial 归不合格侧的
    ///   依据是既有裁定 `purchase_inspection_result::to_receipt_inspection_status`
    ///   （fail/partial 一并回写入库单 REJECTED），统计与本列写入/校验共用同一常量。
    ///
    /// 恒等式 `pending + passed + failed == total` 在**当前代码写入规则**下成立：
    /// 建单（create_inspection）恒写 status=PENDING、result=NULL；完成（
    /// complete_inspection）状态机守卫要求源状态为 PENDING，且结论经 ALL 白名单
    /// 强校验（Some ⟺ is_valid）后恒写 status=COMPLETED、result∈{pass,fail,partial}；
    /// 本表无其他写方（全仓 grep `purchase_inspection::ActiveModel` 仅命中本服务）。
    /// 但该恒等式**没有数据库层约束**兜底：若历史上存在 result 为 NULL 的
    /// completed 行、或词表外 status 行（异常形态），它们只会出现在 `total`，
    /// **不**被塞进任何分桶（不用兜底掩盖），前端四卡之和可能小于总数，
    /// 差值即异常行数——这是如实呈现，不是缺陷。
    pub async fn inspection_stats(
        &self,
        status: Option<String>,
        supplier_id: Option<i32>,
        keyword: Option<String>,
        result: Option<String>,
        date_from: Option<String>,
        date_to: Option<String>,
    ) -> Result<purchase_inspection::PurchaseInspectionStats, AppError> {
        let args = (status, supplier_id, keyword, result, date_from, date_to);

        let total = self
            .base_filtered_query(
                args.0.as_deref(),
                args.1,
                args.2.as_deref(),
                args.3.as_deref(),
                args.4.as_deref(),
                args.5.as_deref(),
            )
            .count(&*self.db)
            .await?;

        let pending = self
            .base_filtered_query(
                args.0.as_deref(),
                args.1,
                args.2.as_deref(),
                args.3.as_deref(),
                args.4.as_deref(),
                args.5.as_deref(),
            )
            .filter(purchase_inspection::Column::InspectionStatus.eq(pis_status::PENDING))
            .count(&*self.db)
            .await?;

        let passed = self
            .base_filtered_query(
                args.0.as_deref(),
                args.1,
                args.2.as_deref(),
                args.3.as_deref(),
                args.4.as_deref(),
                args.5.as_deref(),
            )
            .filter(
                purchase_inspection::Column::InspectionResult.eq(purchase_inspection_result::PASS),
            )
            .count(&*self.db)
            .await?;

        // partial 归不合格侧（fail/partial 同属 REJECTED 回写映射），NULL result
        // 行在 SQL 三值逻辑下不命中 IN 列表，天然不入本桶。
        let failed = self
            .base_filtered_query(
                args.0.as_deref(),
                args.1,
                args.2.as_deref(),
                args.3.as_deref(),
                args.4.as_deref(),
                args.5.as_deref(),
            )
            .filter(purchase_inspection::Column::InspectionResult.is_in(vec![
                purchase_inspection_result::FAIL,
                purchase_inspection_result::PARTIAL,
            ]))
            .count(&*self.db)
            .await?;

        Ok(purchase_inspection::PurchaseInspectionStats {
            total,
            pending,
            passed,
            failed,
        })
    }

    /// 获取质检单详情
    pub async fn get_inspection(
        &self,
        inspection_id: i32,
    ) -> Result<purchase_inspection::Model, AppError> {
        let inspection = purchase_inspection::Entity::find_by_id(inspection_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("采购质检单 {}", inspection_id)))?;

        Ok(inspection)
    }

    // =====================================================
    // 质检明细 CRUD（批次 131 v9 复审 P0：替代 4 个占位端点）
    // =====================================================

    /// 获取质检明细列表（返回指定质检单下所有明细，按 id 升序排列。）
    pub async fn list_inspection_items(
        &self,
        inspection_id: i32,
    ) -> Result<Vec<purchase_inspection_item::Model>, AppError> {
        // 校验质检单存在
        self.get_inspection(inspection_id).await?;

        let items = purchase_inspection_item::Entity::find()
            .filter(purchase_inspection_item::Column::InspectionId.eq(inspection_id))
            .order_by(purchase_inspection_item::Column::Id, Order::Asc)
            .all(&*self.db)
            .await?;
        Ok(items)
    }

    /// 创建质检明细（在指定质检单下新增一条明细记录（产品 + 检验项目 + 合格/不合格数量）。）
    pub async fn create_inspection_item(
        &self,
        inspection_id: i32,
        req: CreateInspectionItemRequest,
    ) -> Result<purchase_inspection_item::Model, AppError> {
        // 校验质检单存在
        self.get_inspection(inspection_id).await?;

        let item = purchase_inspection_item::ActiveModel {
            id: Default::default(),
            inspection_id: Set(inspection_id),
            product_id: Set(req.product_id),
            item_name: Set(req.item_name),
            qualified_quantity: Set(req.qualified_quantity),
            unqualified_quantity: Set(req.unqualified_quantity),
            remark: Set(req.remark),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
        };
        let item = item.insert(&*self.db).await?;
        Ok(item)
    }

    /// 更新质检明细（更新指定明细的合格数量 / 不合格数量 / 备注（部分字段可选）。）
    pub async fn update_inspection_item(
        &self,
        inspection_id: i32,
        item_id: i32,
        req: UpdateInspectionItemRequest,
    ) -> Result<purchase_inspection_item::Model, AppError> {
        // 校验质检单存在
        self.get_inspection(inspection_id).await?;

        let item = purchase_inspection_item::Entity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("质检明细 {}", item_id)))?;

        // 校验明细归属指定质检单
        if item.inspection_id != inspection_id {
            return Err(AppError::validation(format!(
                "明细 {} 不属于质检单 {}",
                item_id, inspection_id
            )));
        }

        let mut item_active: purchase_inspection_item::ActiveModel = item.into();
        if let Some(q) = req.qualified_quantity {
            item_active.qualified_quantity = Set(q);
        }
        if let Some(q) = req.unqualified_quantity {
            item_active.unqualified_quantity = Set(q);
        }
        if let Some(remark) = req.remark {
            item_active.remark = Set(Some(remark));
        }
        item_active.updated_at = Set(Utc::now());

        let item = item_active.update(&*self.db).await?;
        Ok(item)
    }

    /// 删除质检明细（删除指定明细，返回是否删除成功。）
    pub async fn delete_inspection_item(
        &self,
        inspection_id: i32,
        item_id: i32,
    ) -> Result<bool, AppError> {
        // 校验质检单存在
        self.get_inspection(inspection_id).await?;

        let item = purchase_inspection_item::Entity::find_by_id(item_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("质检明细 {}", item_id)))?;

        // 校验明细归属指定质检单
        if item.inspection_id != inspection_id {
            return Err(AppError::validation(format!(
                "明细 {} 不属于质检单 {}",
                item_id, inspection_id
            )));
        }

        let res = purchase_inspection_item::Entity::delete_by_id(item_id)
            .exec(&*self.db)
            .await?;
        Ok(res.rows_affected > 0)
    }
}

// =====================================================
// 请求/响应 DTO
// =====================================================

/// 创建采购质检单请求
#[derive(Debug, Validate, Deserialize)]
pub struct CreatePurchaseInspectionRequest {
    /// 入库单 ID
    pub receipt_id: Option<i32>,

    /// 采购订单 ID
    pub order_id: Option<i32>,

    /// 供应商 ID
    pub supplier_id: Option<i32>,

    /// 质检日期
    pub inspection_date: Option<chrono::NaiveDate>,

    /// 质检员 ID
    pub inspector_id: Option<i32>,

    /// 质检类型
    pub inspection_type: Option<String>,

    /// 样品大小
    pub sample_size: Option<Decimal>,

    /// 备注
    pub notes: Option<String>,
}

/// 更新采购质检单请求
#[derive(Debug, Default, Deserialize)]
pub struct UpdatePurchaseInspectionRequest {
    pub sample_size: Option<Decimal>,
    pub defect_description: Option<String>,
    pub notes: Option<String>,
}

/// 完成质检单请求
#[derive(Debug, Validate, Deserialize)]
pub struct CompleteInspectionRequest {
    /// 合格数量
    pub pass_quantity: Decimal,

    /// 不合格数量
    pub reject_quantity: Decimal,

    /// 质检结果
    pub inspection_result: String,
}

// =====================================================
// 质检明细 DTO（批次 131 v9 复审 P0）
// =====================================================

/// 创建质检明细请求（service 层 DTO）
#[derive(Debug, Deserialize)]
pub struct CreateInspectionItemRequest {
    /// 产品 ID
    pub product_id: i32,
    /// 检验项目名称
    pub item_name: String,
    /// 合格数量
    pub qualified_quantity: Decimal,
    /// 不合格数量
    pub unqualified_quantity: Decimal,
    /// 备注
    pub remark: Option<String>,
}

/// 更新质检明细请求（service 层 DTO）
#[derive(Debug, Default, Deserialize)]
pub struct UpdateInspectionItemRequest {
    /// 合格数量
    pub qualified_quantity: Option<Decimal>,
    /// 不合格数量
    pub unqualified_quantity: Option<Decimal>,
    /// 备注
    pub remark: Option<String>,
}
