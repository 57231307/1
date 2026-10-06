//! 委外加工订单 Service impl 子模块（outsourcing_ops/order）
//!
//! 批次 489 D10-2b 拆分：从原 `outsourcing_service.rs` L291-963 迁移。
//! 包含 OutsourcingOrderService 的 12 个方法：
//! - create / update / delete（CRUD）
//! - issue_order / record_processing / settle / close_order / cancel（状态机）
//! - get_by_id / get_by_no / list（查询）
//! - validate_receipt_eligibility / compute_receipt_calculation（共享 helper）
//!
//! 业务规则：
//! - 状态机：draft → issued → processing → received → settled → closed；任意非 closed/cancelled → cancelled
//! - 收回时计算损耗分类与单位成本（§5.4 三步分录）
//! - 凭证号统一由 DocumentNumberGenerator 生成：OV{类型段}{YYYYMMDD}{3位流水}

use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    Set, SqlErr, TransactionTrait,
};

use crate::models::outsourcing_order::{
    self, ActiveModel as OrderActiveModel, Entity as OrderEntity, Model as OrderModel,
};
use crate::models::outsourcing_order_item::{self, Entity as ItemEntity, Model as ItemModel};
use crate::models::outsourcing_voucher::{
    ActiveModel as VoucherActiveModel, Column as VoucherColumn, Entity as VoucherEntity,
};
use crate::models::status::outsourcing_loss_type;
use crate::models::status::outsourcing_order_status;
use crate::models::status::outsourcing_voucher_type;
use crate::utils::error::AppError;
use crate::utils::number_generator::DocumentNumberGenerator;

use crate::services::outsourcing_ops::receipt::ReceiptCalculation;
use crate::services::outsourcing_ops::types::{
    CreateOutsourcingOrderRequest, OutsourcingOrderQuery, UpdateOutsourcingOrderRequest,
};
use crate::services::outsourcing_service::{
    OutsourcingOrderService, classify_loss, compute_abnormal_loss_amount, compute_loss_rate,
    compute_standard_loss_rate, compute_total_cost, compute_unit_cost, validate_order_type,
};

/// 校验收回前置条件：订单状态与收回数量
pub(crate) fn validate_receipt_eligibility(
    model: &OrderModel,
    return_quantity: Decimal,
) -> Result<(), AppError> {
    if model.status != outsourcing_order_status::PROCESSING
        && model.status != outsourcing_order_status::ISSUED
    {
        return Err(AppError::business(format!(
            "仅已发料(issued)或加工中(processing)状态可收回，当前状态: {}",
            model.status
        )));
    }
    let loss_quantity = model.issue_quantity - return_quantity;
    if loss_quantity < Decimal::ZERO {
        return Err(AppError::business(format!(
            "收回数量 {} 不能大于发出数量 {}",
            return_quantity, model.issue_quantity
        )));
    }
    Ok(())
}

/// 计算收回损耗与成本指标
pub(crate) fn compute_receipt_calculation(
    model: &OrderModel,
    return_quantity: Decimal,
) -> ReceiptCalculation {
    let loss_quantity = model.issue_quantity - return_quantity;
    let actual_loss_rate = compute_loss_rate(loss_quantity, model.issue_quantity);
    let standard_loss_rate = model.standard_loss_rate.unwrap_or(Decimal::ZERO);
    let loss_type_str = classify_loss(actual_loss_rate, standard_loss_rate);
    let is_loss_normal = loss_type_str == outsourcing_loss_type::NORMAL;
    let unit_material_cost = if model.issue_quantity > Decimal::ZERO {
        model.material_cost / model.issue_quantity
    } else {
        Decimal::ZERO
    };
    let abnormal_loss_amount = compute_abnormal_loss_amount(
        model.issue_quantity,
        return_quantity,
        unit_material_cost,
        standard_loss_rate,
    );
    let total_cost = compute_total_cost(
        model.material_cost,
        model.processing_fee,
        model.freight_fee,
        abnormal_loss_amount,
    );
    let unit_cost = compute_unit_cost(total_cost, return_quantity);
    ReceiptCalculation {
        loss_quantity,
        actual_loss_rate,
        loss_type_str,
        is_loss_normal,
        abnormal_loss_amount,
        total_cost,
        unit_cost,
    }
}

impl OutsourcingOrderService {
    /// 校验委外订单创建请求（类型/数量/加工厂/生产订单/缸号/订单号唯一性）
    async fn validate_create_request(
        &self,
        req: &CreateOutsourcingOrderRequest,
    ) -> Result<(), AppError> {
        validate_order_type(&req.order_type)?;
        if req.issue_quantity < Decimal::ZERO {
            return Err(AppError::business("发出数量不能为负"));
        }
        if req.material_cost < Decimal::ZERO {
            return Err(AppError::business("发出材料成本不能为负"));
        }
        if req.processing_fee < Decimal::ZERO {
            return Err(AppError::business("加工费不能为负"));
        }
        if req.freight_fee < Decimal::ZERO {
            return Err(AppError::business("运费不能为负"));
        }
        if req.tax_amount < Decimal::ZERO {
            return Err(AppError::business("税额不能为负"));
        }
        self.validate_create_references(req).await?;
        self.validate_order_no_unique(&req.order_no).await
    }

    /// 校验关联引用存在性（加工厂/生产订单/缸号）——"缺前置"拒绝按可外显业务族
    /// 出参（回显的都是用户自己提交的 ID，先例：chemical_ops 前置校验族），
    /// 禁止被脱敏常量「业务处理失败」吞掉真因；真因同步落 WARN，不静默
    async fn validate_create_references(
        &self,
        req: &CreateOutsourcingOrderRequest,
    ) -> Result<(), AppError> {
        if crate::models::supplier::Entity::find_by_id(req.supplier_id)
            .one(&*self.db)
            .await?
            .is_none()
        {
            tracing::warn!(
                "创建委外订单被拒：加工厂 supplier_id={} 不存在",
                req.supplier_id
            );
            return Err(AppError::business_displayable(format!(
                "委外加工厂 {} 不存在，请重新选择加工厂",
                req.supplier_id
            )));
        }
        if let Some(order_id) = req.production_order_id {
            if crate::models::production_order::Entity::find_by_id(order_id)
                .one(&*self.db)
                .await?
                .is_none()
            {
                tracing::warn!("创建委外订单被拒：生产订单 production_order_id={order_id} 不存在");
                return Err(AppError::business_displayable(format!(
                    "生产订单 {} 不存在，请重新选择生产订单",
                    order_id
                )));
            }
        }
        if let Some(dye_batch_id) = req.dye_batch_id {
            if crate::models::dye_batch::Entity::find_by_id(dye_batch_id)
                .one(&*self.db)
                .await?
                .is_none()
            {
                tracing::warn!("创建委外订单被拒：缸号 dye_batch_id={dye_batch_id} 不存在");
                return Err(AppError::business_displayable(format!(
                    "缸号 {} 不存在，请重新选择缸号",
                    dye_batch_id
                )));
            }
        }
        Ok(())
    }

    /// 校验委外订单号唯一性 —— "你填的单号已存在，换一个再提交"属可执行公开规则，
    /// 归 business_displayable 族（先例：#165 流程编码、chemical_ops 编码族）；
    /// 文案只回显用户自己提交的单号，不含表名/约束名/其它单据；真因落 WARN
    async fn validate_order_no_unique(&self, order_no: &str) -> Result<(), AppError> {
        if OrderEntity::find()
            .filter(outsourcing_order::Column::OrderNo.eq(order_no))
            .filter(outsourcing_order::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .is_some()
        {
            tracing::warn!("创建委外订单被拒：订单号 {order_no} 在未删除行中已存在");
            return Err(AppError::business_displayable(format!(
                "委外订单号 {} 已存在，请更换单号后重试",
                order_no
            )));
        }
        Ok(())
    }

    /// 构建委外订单 ActiveModel（含标准损耗率与单位默认值计算）；建单人取服务端会话身份
    fn build_order_active_model(
        req: CreateOutsourcingOrderRequest,
        user_id: i32,
        now: chrono::DateTime<chrono::FixedOffset>,
    ) -> OrderActiveModel {
        let standard_loss_rate = req
            .standard_loss_rate
            .unwrap_or_else(|| compute_standard_loss_rate(&req.order_type));
        let issue_unit = req.issue_unit.unwrap_or_else(|| "kg".to_string());
        OrderActiveModel {
            id: Default::default(),
            order_no: Set(req.order_no),
            order_type: Set(req.order_type),
            supplier_id: Set(req.supplier_id),
            production_order_id: Set(req.production_order_id),
            dye_batch_id: Set(req.dye_batch_id),
            color_no: Set(req.color_no),
            dye_lot_no: Set(req.dye_lot_no),
            issue_date: Set(req.issue_date),
            expected_return_date: Set(req.expected_return_date),
            actual_return_date: Set(None),
            issue_quantity: Set(req.issue_quantity),
            issue_unit: Set(issue_unit),
            return_quantity: Set(Decimal::ZERO),
            loss_quantity: Set(Decimal::ZERO),
            loss_type: Set(None),
            loss_rate: Set(None),
            standard_loss_rate: Set(Some(standard_loss_rate)),
            material_cost: Set(req.material_cost),
            // 三费真实入参（CreateOutsourcingOrderRequest，NOT NULL 列
            // v15/mod.rs:3247-3249）：建单缺省键经 serde(default) 为 0，与原
            // Set(ZERO) 初始化同值；draft 期亦可经 PUT 补录（update 三态）
            processing_fee: Set(req.processing_fee),
            freight_fee: Set(req.freight_fee),
            tax_amount: Set(req.tax_amount),
            abnormal_loss_amount: Set(Decimal::ZERO),
            // 总成本与收回/结算同一权威公式（compute_total_cost：材料+加工费+运费-非正常损耗，
            // 建单阶段非正常损耗为 0）
            total_cost: Set(compute_total_cost(
                req.material_cost,
                req.processing_fee,
                req.freight_fee,
                Decimal::ZERO,
            )),
            unit_cost: Set(Decimal::ZERO),
            status: Set(outsourcing_order_status::DRAFT.to_string()),
            voucher_no_issue: Set(None),
            voucher_no_fee: Set(None),
            voucher_no_receipt: Set(None),
            remarks: Set(req.remarks),
            is_deleted: Set(false),
            created_by: Set(Some(user_id)),
            created_at: Set(now),
            updated_at: Set(now),
        }
    }

    /// 创建委外订单（draft 状态）；建单人取服务端会话身份，请求体不承载身份
    pub async fn create(
        &self,
        req: CreateOutsourcingOrderRequest,
        user_id: i32,
    ) -> Result<OrderModel, AppError> {
        self.validate_create_request(&req).await?;
        let now = crate::utils::date_utils::utc_now_fixed();
        // 业务上下文（订单号）留日志侧供按单排查；active 构造会移动 req，先取值
        let order_no_for_log = req.order_no.clone();
        let active = Self::build_order_active_model(req, user_id, now);
        // DbErr 非唯一类一律经 `?`/AppError::from（From<DbErr>）统一分类归
        // DATABASE_ERROR、真实原因只进 tracing::error，出参脱敏「数据库错误」；
        // 唯一类(23505)按竞态兜底降级为与预校验同口径的业务拒绝（见下）。
        let result = active.insert(&*self.db).await.map_err(|e| {
            // 竞态兜底（照 role_permission/chemical_ops「预校验 + 23505 归类」范式）：
            // 上方 order_no 查重通过后、INSERT 落库前，并发请求可能已插入同码未删行。
            // 本表当前 order_no 仅 NOT NULL 无 DB UNIQUE（v15/mod.rs:3229，待数据库
            // 专家补部分唯一索引）；索引就位后此处将以 23505 显式拒绝——单语句
            // INSERT 原子失败、不留半行，归类必须与预检同口径，禁止拍平 500 吞真因。
            if matches!(e.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) {
                tracing::error!(
                    "委外订单创建撞单号唯一约束（并发同码，order_no={}）：行未写入，整语句回滚，底层错误={}",
                    order_no_for_log,
                    e
                );
                AppError::business_displayable(format!(
                    "委外订单号 {} 已存在，请更换单号后重试",
                    order_no_for_log
                ))
            } else {
                tracing::error!(order_no = %order_no_for_log, "委外订单创建落库失败");
                AppError::from(e)
            }
        })?;
        Ok(result)
    }

    /// 更新委外订单（仅 draft 状态可更新）
    ///
    /// 三态写入（RFC 7386，对齐 department_service::update）：
    /// None=不 Set、Some(None)=Set(None) 置 NULL（仅 DB 可空列）、Some(Some(v))=Set(v) 覆盖；
    /// NOT NULL 列（order_type/supplier_id/issue_date/issue_quantity/issue_unit/material_cost/
    /// processing_fee/freight_fee/tax_amount，v15 outsourcing_order DDL v15/mod.rs:3227-3262）
    /// 的显式 null 在任何 DB 访问前拒绝（外显不脱敏）。
    pub async fn update(
        &self,
        id: i32,
        req: UpdateOutsourcingOrderRequest,
    ) -> Result<OrderModel, AppError> {
        if matches!(req.order_type, Some(None)) {
            return Err(AppError::business_displayable(
                "委外类型不能清空：该字段为必填项",
            ));
        }
        if matches!(req.supplier_id, Some(None)) {
            return Err(AppError::business_displayable(
                "委外加工厂不能清空：该字段为必填项",
            ));
        }
        if matches!(req.issue_date, Some(None)) {
            return Err(AppError::business_displayable(
                "发料日期不能清空：该字段为必填项",
            ));
        }
        if matches!(req.issue_quantity, Some(None)) {
            return Err(AppError::business_displayable(
                "发出数量不能清空：该字段为必填项",
            ));
        }
        if matches!(req.issue_unit, Some(None)) {
            return Err(AppError::business_displayable(
                "发出单位不能清空：该字段为必填项",
            ));
        }
        if matches!(req.material_cost, Some(None)) {
            return Err(AppError::business_displayable(
                "发出材料成本不能清空：该字段为必填项",
            ));
        }
        if matches!(req.processing_fee, Some(None)) {
            return Err(AppError::business_displayable(
                "加工费不能清空：该字段为必填项",
            ));
        }
        if matches!(req.freight_fee, Some(None)) {
            return Err(AppError::business_displayable(
                "运费不能清空：该字段为必填项",
            ));
        }
        if matches!(req.tax_amount, Some(None)) {
            return Err(AppError::business_displayable(
                "税额不能清空：该字段为必填项",
            ));
        }

        let model = self.get_by_id(id).await?;
        // 更新权限分态：draft 全量可改；received 仅允许补录/更正成本四项
        // （material_cost / processing_fee / freight_fee / tax_amount）。
        // 依据：本批把「零费用不得结算」落成硬拒并外显「请先补录委外加工成本」，而委外加工费
        // 在行业惯例里通常于收回/对账时才最终确定（先加工后计价）；若 received 态完全锁死，
        // 0 费用订单既补不了成本也结不了算＝无出口死单。其余业务字段（供应商/数量/缸号/日期等）
        // 收回后再改会让已生成的发料与成本凭证同事实脱节，故仍锁 draft。
        let has_cost_fields = req.material_cost.is_some()
            || req.processing_fee.is_some()
            || req.freight_fee.is_some()
            || req.tax_amount.is_some();
        let has_non_cost_fields = req.order_type.is_some()
            || req.supplier_id.is_some()
            || req.production_order_id.is_some()
            || req.dye_batch_id.is_some()
            || req.color_no.is_some()
            || req.dye_lot_no.is_some()
            || req.issue_date.is_some()
            || req.expected_return_date.is_some()
            || req.issue_quantity.is_some()
            || req.issue_unit.is_some()
            || req.standard_loss_rate.is_some()
            || req.remarks.is_some();
        if model.status != outsourcing_order_status::DRAFT {
            let cost_correction_allowed = model.status == outsourcing_order_status::RECEIVED
                && has_cost_fields
                && !has_non_cost_fields;
            if !cost_correction_allowed {
                return Err(AppError::business(format!(
                    "仅草稿(draft)状态可更新，当前状态: {}",
                    model.status
                )));
            }
        }

        // 成本链生效值快照（model 随后被 move 进 active）：材料成本/加工费/运费任一
        // 被覆盖即联动重算 total_cost/unit_cost——与收回（compute_receipt_calculation）、
        // 结算（settle）同一权威公式，禁止各写一套
        let (mut material_cost, mut processing_fee, mut freight_fee) =
            (model.material_cost, model.processing_fee, model.freight_fee);
        let (abnormal_loss_amount, return_quantity) =
            (model.abnormal_loss_amount, model.return_quantity);
        let mut cost_inputs_changed = false;

        let mut active: OrderActiveModel = model.into();

        // NOT NULL 列（Some(None) 已在入口拒绝）：仅覆盖/保持
        if let Some(v) = req.order_type.flatten() {
            validate_order_type(&v)?;
            active.order_type = Set(v);
        }
        if let Some(v) = req.supplier_id.flatten() {
            // 校验委外加工厂存在
            if crate::models::supplier::Entity::find_by_id(v)
                .one(&*self.db)
                .await?
                .is_none()
            {
                return Err(AppError::business(format!("委外加工厂 {} 不存在", v)));
            }
            active.supplier_id = Set(v);
        }
        if let Some(v) = req.issue_date.flatten() {
            active.issue_date = Set(v);
        }
        if let Some(v) = req.issue_quantity.flatten() {
            if v < Decimal::ZERO {
                return Err(AppError::business("发出数量不能为负"));
            }
            active.issue_quantity = Set(v);
        }
        if let Some(v) = req.issue_unit.flatten() {
            active.issue_unit = Set(v);
        }
        // DB 可空列：Some(None)=Set(None) 清空、Some(Some(v))=Set(Some(v)) 覆盖
        if let Some(v) = req.production_order_id {
            active.production_order_id = Set(v);
        }
        if let Some(v) = req.dye_batch_id {
            active.dye_batch_id = Set(v);
        }
        if let Some(v) = req.color_no {
            active.color_no = Set(v);
        }
        if let Some(v) = req.dye_lot_no {
            active.dye_lot_no = Set(v);
        }
        if let Some(v) = req.expected_return_date {
            active.expected_return_date = Set(v);
        }
        if let Some(v) = req.standard_loss_rate {
            active.standard_loss_rate = Set(v);
        }
        if let Some(v) = req.remarks {
            active.remarks = Set(v);
        }
        // material_cost：NOT NULL 列——覆盖时进入成本链重算（与三费同一落点）
        if let Some(v) = req.material_cost.flatten() {
            if v < Decimal::ZERO {
                return Err(AppError::business("发出材料成本不能为负"));
            }
            active.material_cost = Set(v);
            material_cost = v;
            cost_inputs_changed = true;
        }
        // 三费：NOT NULL 列（v15 outsourcing_order DDL :3247-3249，Some(None) 已在入口拒绝）
        // ——有值覆盖、缺席保持；加工费/运费参与 total_cost 成本链，税额单独记入
        // 结算 FEE 凭证的 tax_amount（settle 语义 order.rs:484 注释，本请求补齐后成立）
        if let Some(v) = req.processing_fee.flatten() {
            if v < Decimal::ZERO {
                return Err(AppError::business("加工费不能为负"));
            }
            active.processing_fee = Set(v);
            processing_fee = v;
            cost_inputs_changed = true;
        }
        if let Some(v) = req.freight_fee.flatten() {
            if v < Decimal::ZERO {
                return Err(AppError::business("运费不能为负"));
            }
            active.freight_fee = Set(v);
            freight_fee = v;
            cost_inputs_changed = true;
        }
        if let Some(v) = req.tax_amount.flatten() {
            if v < Decimal::ZERO {
                return Err(AppError::business("税额不能为负"));
            }
            active.tax_amount = Set(v);
        }
        // 成本链联动：total_cost=材料+加工费+运费-非正常损耗；unit_cost=总成本/收回量
        // （draft 期收回量为 0 时 compute_unit_cost 归零，与建单口径一致）
        if cost_inputs_changed {
            let total_cost = compute_total_cost(
                material_cost,
                processing_fee,
                freight_fee,
                abnormal_loss_amount,
            );
            active.total_cost = Set(total_cost);
            active.unit_cost = Set(compute_unit_cost(total_cost, return_quantity));
        }

        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 软删除委外订单（仅 draft 状态可删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_order_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可删除，当前状态: {}",
                model.status
            )));
        }
        let mut active: OrderActiveModel = model.into();
        active.is_deleted = Set(true);
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        active.update(&*self.db).await?;
        Ok(())
    }

    /// 发料：draft → issued，创建发料凭证（借：委托加工物资 / 贷：自制半成品-胚布）
    ///
    /// V15 主线审计 P0 修复：原实现顺序执行 3 步（凭证创建 / 主单更新 / 事件发布），
    /// 任一步失败都会留下半成品数据。把凭证创建和主单更新放进同一数据库事务，
    /// 事件发布在事务 commit 后执行（事件发布失败不影响业务数据一致性）。
    ///
    /// 匹状态占用闭环：明细读取与 CAS 占用（AVAILABLE→RESERVED）都在本事务内、
    /// 凭证取号之前执行——占用与凭证/状态推进原子提交，任一行 CAS 不命中整单
    /// 回滚（不允许部分匹被占用），并消除原「事务外只读校验→事务内提交」的
    /// TOCTOU 重复发料窗口。
    pub async fn issue_order(
        &self,
        id: i32,
        operator_id: Option<i32>,
    ) -> Result<OrderModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_order_status::DRAFT {
            return Err(AppError::business(format!(
                "仅草稿(draft)状态可发料，当前状态: {}",
                model.status
            )));
        }

        let now = crate::utils::date_utils::utc_now_fixed();

        let txn = (*self.db).begin().await?;

        // 匹号领域二期：明细读取与占用全部在事务内——染色/印花外发必须精确到生产匹，
        // 且匹存在、状态可用；占用以 CAS 条件更新落库（归因文案由域服务统一）。
        // 读取顺序与 OutsourcingOrderItemService::list_by_order 同源（按 id 倒序）。
        let items: Vec<ItemModel> = ItemEntity::find()
            .filter(outsourcing_order_item::Column::OutsourcingOrderId.eq(id))
            .order_by_desc(outsourcing_order_item::Column::Id)
            .all(&txn)
            .await?;
        crate::services::piece_domain_service::reserve_pieces_for_issue(&txn, &items, operator_id)
            .await?;

        // 生成发料凭证号（统一生成器，事务内取号：`OVIS{YYYYMMDD}{3位流水}`）
        let voucher_no = DocumentNumberGenerator::generate_no_with_txn(
            &txn,
            "OVIS",
            VoucherEntity,
            VoucherColumn::VoucherNo,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "委外发料凭证号生成失败");
            AppError::business_displayable("委外发料凭证号生成失败，请稍后重试")
        })?;

        // 阶段 1：创建发料凭证
        let voucher_active = VoucherActiveModel {
            id: Default::default(),
            voucher_no: Set(voucher_no.clone()),
            outsourcing_order_id: Set(id),
            voucher_type: Set(outsourcing_voucher_type::ISSUE.to_string()),
            debit_account: Set("委托加工物资".to_string()),
            credit_account: Set("自制半成品-胚布".to_string()),
            amount: Set(model.material_cost),
            tax_amount: Set(Decimal::ZERO),
            tax_transfer_amount: Set(Decimal::ZERO),
            voucher_date: Set(model.issue_date),
            is_posted: Set(false),
            posted_at: Set(None),
            remarks: Set(Some(format!("委外订单 {} 发料", model.order_no))),
            created_by: Set(model.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };
        voucher_active.insert(&txn).await.map_err(|e| {
            // 真实原因只进 From<DbErr> 的 tracing::error（DATABASE_ERROR/500、出参脱敏）；
            // 订单号+凭证号作为业务上下文留在本条 ERROR 日志，不进错误体。
            tracing::error!(
                order_id = id,
                voucher_no = %voucher_no,
                "委外发料凭证落库失败"
            );
            AppError::from(e)
        })?;

        // 阶段 2：更新订单主单
        let mut active: OrderActiveModel = model.into();
        active.status = Set(outsourcing_order_status::ISSUED.to_string());
        active.voucher_no_issue = Set(Some(voucher_no.clone()));
        active.updated_at = Set(now);
        let updated = active.update(&txn).await.map_err(|e| {
            tracing::error!(order_id = id, voucher_no = %voucher_no, "委外发料后订单状态落库失败");
            AppError::from(e)
        })?;

        txn.commit().await?;

        // 阶段 3：事件发布（事务外，业务数据已落库；事件失败由 EVENT_BUS 自行重试/降级）
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::OutsourcingMaterialIssued {
                order_id: updated.id,
                order_no: updated.order_no.clone(),
                order_type: updated.order_type.clone(),
                supplier_id: updated.supplier_id,
                issue_quantity: updated.issue_quantity,
                voucher_no_issue: updated.voucher_no_issue.clone(),
            },
        );
        tracing::info!(order_id = updated.id, "委外发料事件已发布");

        Ok(updated)
    }

    /// 标记加工中：issued → processing
    pub async fn record_processing(&self, id: i32) -> Result<OrderModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_order_status::ISSUED {
            return Err(AppError::business(format!(
                "仅已发料(issued)状态可标记加工中，当前状态: {}",
                model.status
            )));
        }
        let mut active: OrderActiveModel = model.into();
        active.status = Set(outsourcing_order_status::PROCESSING.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;

        // V15 Batch04-P1-5：发布委外加工中事件，供生产看板/进度追踪订阅
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::OutsourcingProcessingRecorded {
                order_id: updated.id,
                order_no: updated.order_no.clone(),
                order_type: updated.order_type.clone(),
                supplier_id: updated.supplier_id,
            },
        );
        tracing::info!(order_id = updated.id, "委外加工中事件已发布");

        Ok(updated)
    }

    /// 结算：received → settled，创建加工费凭证（借：委托加工物资+应交税费 / 贷：银行存款）
    /// 业务规则：加工费/运费/税额需在订单更新时填入（processing_fee / freight_fee / tax_amount 字段）；加工费凭证金额 = processing_fee + freight_fee；税额单独记录在 tax_amount 字段
    ///
    /// V15 主线审计 P0 修复：原实现顺序执行 2 步（凭证创建 / 主单更新），
    /// 任一步失败都会留下半成品数据。把凭证创建和主单更新放进同一数据库事务，
    /// 事件发布在事务 commit 后执行。
    pub async fn settle(&self, id: i32) -> Result<OrderModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_order_status::RECEIVED {
            return Err(AppError::business(format!(
                "仅已收回(received)状态可结算，当前状态: {}",
                model.status
            )));
        }

        // 零费用不得结算：`settled` 的词表定义即「加工费已结算，已生成加工费凭证」，
        // 而加工费凭证金额恒 = processing_fee + freight_fee，两者皆 0 时会落一张金额为 0 的
        // 空壳 OVFE 凭证（并把它回写 voucher_no_fee、随事件外发），与状态语义矛盾，
        // 还会污染以金额为导向的审计抽样——零金额凭证是 SAP/金蝶等同类系统的典型审计问题。
        // 口径取"拒绝结算"而非"允许结算不出凭证"：后者会让 settled 名不副实。
        // 放在取号与 begin() 之前，拒绝时零副作用（也使其可在 sqlite 真实跑全链）。
        // 守卫取 `<= 0` 而非 `== 0`：负值虽已被 create/update 的「不能为负」挡在写入侧
        // （本文件 :415-444），但结算读的是库中既有行——legacy/手工改库可能带负，
        // 负金额凭证比空壳凭证更坏，故此处 fail-closed。文案因此按「合计需大于 0」陈述，
        // 与守卫逐字符对应，不写成只描述 0 的「均为 0」（复审 G4）。
        let fee_amount = model.processing_fee + model.freight_fee;
        if fee_amount <= Decimal::ZERO {
            return Err(AppError::business_displayable(
                "加工费与运费合计需大于 0 才能结算，请先补录委外加工成本",
            ));
        }

        let now = crate::utils::date_utils::utc_now_fixed();

        let txn = (*self.db).begin().await?;

        // 生成加工费凭证号（统一生成器，事务内取号：`OVFE{YYYYMMDD}{3位流水}`）
        let voucher_no = DocumentNumberGenerator::generate_no_with_txn(
            &txn,
            "OVFE",
            VoucherEntity,
            VoucherColumn::VoucherNo,
        )
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "委外加工费凭证号生成失败");
            AppError::business_displayable("委外加工费凭证号生成失败，请稍后重试")
        })?;

        // 创建加工费凭证（§5.4 第二步分录）
        let voucher_active = VoucherActiveModel {
            id: Default::default(),
            voucher_no: Set(voucher_no.clone()),
            outsourcing_order_id: Set(id),
            voucher_type: Set(outsourcing_voucher_type::FEE.to_string()),
            debit_account: Set("委托加工物资".to_string()),
            credit_account: Set("银行存款".to_string()),
            amount: Set(fee_amount),
            tax_amount: Set(model.tax_amount),
            tax_transfer_amount: Set(Decimal::ZERO),
            voucher_date: Set(now.date_naive()),
            is_posted: Set(false),
            posted_at: Set(None),
            remarks: Set(Some(format!("委外订单 {} 加工费结算", model.order_no))),
            created_by: Set(model.created_by),
            created_at: Set(now),
            updated_at: Set(now),
        };
        voucher_active.insert(&txn).await.map_err(|e| {
            // 同发料路径：DbErr 走 From<DbErr> 统一分类（DATABASE_ERROR/500、出参脱敏），
            // 订单号/凭证号业务上下文只进 ERROR 日志，不拼错误原文。
            tracing::error!(
                order_id = id,
                voucher_no = %voucher_no,
                "委外加工费凭证落库失败"
            );
            AppError::from(e)
        })?;

        // 更新订单总成本与状态
        let total_cost = compute_total_cost(
            model.material_cost,
            model.processing_fee,
            model.freight_fee,
            model.abnormal_loss_amount,
        );
        let unit_cost = compute_unit_cost(total_cost, model.return_quantity);

        let mut active: OrderActiveModel = model.into();
        active.total_cost = Set(total_cost);
        active.unit_cost = Set(unit_cost);
        active.voucher_no_fee = Set(Some(voucher_no.clone()));
        active.status = Set(outsourcing_order_status::SETTLED.to_string());
        active.updated_at = Set(now);
        let updated = active.update(&txn).await?;

        txn.commit().await?;

        // V15 Batch04-P1-5：发布委外结算事件，供成本归集/应付账款订阅
        let normal_loss = (updated.loss_quantity - updated.abnormal_loss_amount).max(Decimal::ZERO);
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::OutsourcingOrderSettled {
                order_id: updated.id,
                order_no: updated.order_no.clone(),
                order_type: updated.order_type.clone(),
                supplier_id: updated.supplier_id,
                processing_fee: updated.processing_fee,
                freight_fee: updated.freight_fee,
                normal_loss,
                abnormal_loss: updated.abnormal_loss_amount,
                total_cost: updated.total_cost,
                unit_cost: updated.unit_cost,
                voucher_no_fee: updated.voucher_no_fee.clone(),
            },
        );
        tracing::info!(order_id = updated.id, "委外结算事件已发布");

        Ok(updated)
    }

    /// 关闭：settled → closed
    pub async fn close_order(&self, id: i32) -> Result<OrderModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status != outsourcing_order_status::SETTLED {
            return Err(AppError::business(format!(
                "仅已结算(settled)状态可关闭，当前状态: {}",
                model.status
            )));
        }
        let mut active: OrderActiveModel = model.into();
        active.status = Set(outsourcing_order_status::CLOSED.to_string());
        active.updated_at = Set(crate::utils::date_utils::utc_now_fixed());
        let updated = active.update(&*self.db).await?;

        // V15 Batch04-P1-5：发布委外完成事件，供库存入库/成本结转订阅
        crate::services::event_bus::EVENT_BUS.publish(
            crate::services::event_bus::BusinessEvent::OutsourcingOrderCompleted {
                order_id: updated.id,
                order_no: updated.order_no.clone(),
                order_type: updated.order_type.clone(),
                supplier_id: updated.supplier_id,
                return_quantity: updated.return_quantity,
                voucher_no_receipt: updated.voucher_no_receipt.clone(),
            },
        );
        tracing::info!(order_id = updated.id, "委外完成事件已发布");

        Ok(updated)
    }

    /// 取消：任意非 closed 状态 → cancelled
    ///
    /// 占用闭环释放：issued/processing 单的取消必须把发料时 CAS 占用（RESERVED）的
    /// 生产匹释放回 AVAILABLE，且与主单状态推进同事务原子提交（明细读取也在事务内）。
    /// draft 单从未占用匹；received/settled 单的匹已在收回确认时转 SHIPPED，
    /// 取消不得把它们回退成 AVAILABLE——这两类路径维持原有单行更新，不触碰库存。
    pub async fn cancel(&self, id: i32, operator_id: Option<i32>) -> Result<OrderModel, AppError> {
        let model = self.get_by_id(id).await?;
        if model.status == outsourcing_order_status::CLOSED {
            return Err(AppError::business("已关闭状态不可取消"));
        }
        if model.status == outsourcing_order_status::CANCELLED {
            return Err(AppError::business("已取消状态不可重复取消"));
        }
        let holds_reserved_pieces = model.status == outsourcing_order_status::ISSUED
            || model.status == outsourcing_order_status::PROCESSING;
        let now = crate::utils::date_utils::utc_now_fixed();

        if holds_reserved_pieces {
            let txn = (*self.db).begin().await?;
            let items: Vec<ItemModel> = ItemEntity::find()
                .filter(outsourcing_order_item::Column::OutsourcingOrderId.eq(id))
                .order_by_desc(outsourcing_order_item::Column::Id)
                .all(&txn)
                .await?;
            crate::services::piece_domain_service::release_reserved_pieces_on_cancel(
                &txn,
                &items,
                operator_id,
            )
            .await?;
            let mut active: OrderActiveModel = model.into();
            active.status = Set(outsourcing_order_status::CANCELLED.to_string());
            active.updated_at = Set(now);
            let updated = active.update(&txn).await?;
            txn.commit().await?;
            tracing::info!(order_id = updated.id, "委外订单取消，匹占用释放事务已提交");
            return Ok(updated);
        }

        let mut active: OrderActiveModel = model.into();
        active.status = Set(outsourcing_order_status::CANCELLED.to_string());
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// 按 ID 查询
    pub async fn get_by_id(&self, id: i32) -> Result<OrderModel, AppError> {
        OrderEntity::find_by_id(id)
            .filter(outsourcing_order::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("委外订单 {} 不存在", id)))
    }

    /// 按订单号查询
    pub async fn get_by_no(&self, order_no: &str) -> Result<OrderModel, AppError> {
        OrderEntity::find()
            .filter(outsourcing_order::Column::OrderNo.eq(order_no))
            .filter(outsourcing_order::Column::IsDeleted.eq(false))
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found(format!("委外订单号 {} 不存在", order_no)))
    }

    /// 分页查询
    pub async fn list(
        &self,
        query: OutsourcingOrderQuery,
    ) -> Result<(Vec<OrderModel>, u64), AppError> {
        let mut q = OrderEntity::find().filter(outsourcing_order::Column::IsDeleted.eq(false));
        if let Some(v) = query.order_type {
            q = q.filter(outsourcing_order::Column::OrderType.eq(v));
        }
        if let Some(v) = query.supplier_id {
            q = q.filter(outsourcing_order::Column::SupplierId.eq(v));
        }
        if let Some(v) = query.production_order_id {
            q = q.filter(outsourcing_order::Column::ProductionOrderId.eq(v));
        }
        if let Some(v) = query.dye_batch_id {
            q = q.filter(outsourcing_order::Column::DyeBatchId.eq(v));
        }
        if let Some(v) = query.dye_lot_no {
            q = q.filter(outsourcing_order::Column::DyeLotNo.eq(v));
        }
        if let Some(v) = query.status {
            q = q.filter(outsourcing_order::Column::Status.eq(v));
        }
        if let Some(v) = query.issue_date_from {
            q = q.filter(outsourcing_order::Column::IssueDate.gte(v));
        }
        if let Some(v) = query.issue_date_to {
            q = q.filter(outsourcing_order::Column::IssueDate.lte(v));
        }
        if let Some(kw) = query.keyword {
            q = q.filter(
                Condition::any()
                    .add(outsourcing_order::Column::OrderNo.contains(&kw))
                    .add(outsourcing_order::Column::ColorNo.contains(&kw))
                    .add(outsourcing_order::Column::DyeLotNo.contains(&kw)),
            );
        }

        let page = query.page.unwrap_or(1).max(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 200);

        let total = q.clone().count(&*self.db).await?;
        let items = q
            .order_by_desc(outsourcing_order::Column::Id)
            .paginate(&*self.db, page_size)
            .fetch_page(page - 1)
            .await?;
        Ok((items, total))
    }
}
