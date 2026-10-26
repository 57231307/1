#![allow(dead_code)]
//! 纺织品安全送检单服务（建单校验、状态机、项目集推导、童装核查登记、整单判定汇总）
//!
//! 送检流程真实口径：建单 → 按人群与接触方式与市场推项目清单 → 取样（同品种同色号同工艺
//! 一批）→ 送有资质机构 → 收报告 → 核对要素与有效期 → 判定归档 → 关闭；撤回为旁支终态。
//! 状态迁移白名单单源在本文件 `ALLOWED_TRANSITIONS`，与词表 `submission_status` 同步维护。
use crate::models::status::{
    export_market, sampling_color_role, submission_status, textile_basis_standard,
    textile_check_kind, textile_contact_type, textile_end_user_group, textile_exemption_tag,
    textile_flag_value, textile_item_kind, textile_item_source, textile_product_form,
    textile_safety_category, textile_verdict,
};
use crate::models::textile_child_safety_check::{
    ActiveModel as CheckActiveModel, Column as CheckColumn, Entity as ChildCheck,
    Model as ChildCheckModel,
};
use crate::models::textile_report_item_result::{
    Column as ResultColumn, Entity as ReportResult,
};
use crate::models::textile_safety_submission::{
    ActiveModel, Column, Entity as Submission, Model,
};
use crate::models::textile_safety_submission_item::{
    ActiveModel as ItemActiveModel, Column as ItemColumn, Entity as SubmissionItem, Model as ItemModel,
};
use crate::models::textile_test_item::{
    Column as ItemMasterColumn, Entity as TestItem, Model as TestItemModel,
};
use crate::models::inventory_stock::{
    ActiveModel as StockActiveModel, Column as StockColumn, Entity as InventoryStock,
};
use crate::models::product::Entity as ProductEntity;
use crate::models::status::purchase_inventory::{
    inventory_stock_quality_status as stock_quality, inventory_stock_status as stock_status,
};
use crate::models::textile_third_party_report::{
    Column as ReportColumn, Entity as ThirdPartyReport,
};
use crate::services::audit_log_service::AuditLogService;
use crate::services::textile_safety_limit_judge::{
    fetch_rules_for_item, resolve_verdict, CandidateRule, JudgeDims, MeasuredInput,
};
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Condition, DatabaseConnection, DeleteMany,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};
use std::sync::Arc;

/// 状态迁移白名单（from, to），判据与词表 `submission_status` 的状态机注释一致：
/// 撤回只能从非终态进入；judged 只能由整单判定动作产出，不能手工 PUT 直达。
const ALLOWED_TRANSITIONS: &[(&str, &str)] = &[
    (submission_status::DRAFT, submission_status::SAMPLING),
    (submission_status::SAMPLING, submission_status::SENT),
    (submission_status::SENT, submission_status::REPORTED),
    (submission_status::JUDGED, submission_status::CLOSED),
    (submission_status::DRAFT, submission_status::WITHDRAWN),
    (submission_status::SAMPLING, submission_status::WITHDRAWN),
    (submission_status::SENT, submission_status::WITHDRAWN),
    (submission_status::REPORTED, submission_status::WITHDRAWN),
];

/// 建单入参（handler 从请求体拆出，审计字段由 handler 注入会话用户，不接受请求体造人）。
pub struct CreateSubmissionReq {
    pub submission_no: String,
    pub product_id: i32,
    pub batch_no: Option<String>,
    pub color_no: String,
    pub process_no: String,
    pub color_role: String,
    pub end_user_group: String,
    pub contact_type: String,
    pub safety_category: String,
    pub product_form: String,
    pub target_market: String,
    /// 请求侧豁免标签 token 列表（service 负责白名单校验并拼逗号串入库）
    pub exemption_tags: Vec<String>,
    pub remarks: Option<String>,
    pub created_by: i32,
}

/// 列表过滤入参。
pub struct SubmissionListParams {
    pub product_id: Option<i32>,
    pub status: Option<String>,
    pub target_market: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 童装核查登记入参（一行的现场数据；measurable 与 manual 的必填差异由 service 校验）。
pub struct ChildCheckUpsertReq {
    pub item_code: String,
    pub measured_value: Option<Decimal>,
    pub measured_unit: Option<String>,
    pub measured_flag: Option<String>,
    pub check_desc: Option<String>,
    pub manual_conclusion: Option<String>,
}

/// 库存联动的动作语义（判定与状态机对 inventory_stocks.quality_status 的四种合法改写）。
///
/// 依据既有裁定"待检与不合格不出库"：可出库量筛选只认 quality_status 为合格，因此本域
/// 把送检流程的三个阶段映射到同一质量状态列（裁定一"质量状态列保持关闭不新建重复列"），
/// 不新建列、不改库存域文件，只经统一审计通道写本列。
#[derive(Clone, Copy, Debug)]
pub enum StockLinkAction {
    /// 送检出样（sampling→sent）：范围内合格行降为待检，取样批次送检期间不得出库
    HoldOnSent,
    /// 撤回已送检/已收报告的单据：待检行回合格（撤回不代表结论，解除的是送检预防性挂起）
    ReleaseOnWithdraw,
    /// 整单判定 fail：范围内合格与待检行一律降为不合格（批次冻结，出库门自然拦下）
    FreezeOnFail,
    /// 整单判定 pass：范围内待检与不合格行回到合格（本域最新权威检验结论放行）
    ReleaseOnPass,
}

impl StockLinkAction {
    /// 回显与日志用的动作文案（单源，避免三处手写漂移）。
    fn label(self) -> &'static str {
        match self {
            StockLinkAction::HoldOnSent => "送检挂起",
            StockLinkAction::ReleaseOnWithdraw => "撤回解除挂起",
            StockLinkAction::FreezeOnFail => "不合格冻结",
            StockLinkAction::ReleaseOnPass => "合格放行",
        }
    }
}

/// 库存联动回显（写了吗、写了谁、为什么没写，全部点名回给调用方，不静默）。
#[derive(Clone, Debug, serde::Serialize)]
pub struct StockLinkResult {
    /// 品种乘批次命中且在库（正常台账状态）的库存行数
    pub matched: u64,
    /// 本次实际改写质量状态的库存行 id
    pub updated_stock_ids: Vec<i32>,
    /// 未联动或联动口径的人类可读说明（无批次号、零命中、na 结论不解除挂起等都要在这里点名）
    pub scope_note: String,
}

/// 整单判定产出（单据本体加联动与告警回显；判定成功但联动细节不可见就是假绿）。
#[derive(serde::Serialize)]
pub struct JudgementOutcome {
    pub submission: Model,
    pub stock_link: StockLinkResult,
    /// 已受理但有效期已过或被复检替代而排除出本次汇总口径的报告编号（点名告警面）
    pub expired_reports_named: Vec<String>,
}

pub struct TextileSafetySubmissionService {
    db: Arc<DatabaseConnection>,
}

impl TextileSafetySubmissionService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 建单：新建一张送检单，落草稿态。
    ///
    /// 判据：全部维度字段逐字符命中各自词表（人群、接触方式、类别、形态、市场、色号角色），
    /// 词表外 400 fail-visible；申报安全类别不得松于人群最低要求类别（婴幼儿须 A 类、
    /// 儿童须 B 类及以上，GB 31701 第 4 章）；豁免标签逐 token 白名单后拼逗号串；
    /// product_id 必须命中 products（只读关联，FK 之外的显式友好报错）。
    /// 传给谁：handler create_submission 端点（POST /textile-submissions）。
    pub async fn create(&self, req: CreateSubmissionReq) -> Result<Model, AppError> {
        self.validate_dims(
            &req.end_user_group,
            &req.contact_type,
            &req.safety_category,
            &req.product_form,
            &req.target_market,
            &req.color_role,
            &req.exemption_tags,
        )?;
        if req.submission_no.trim().is_empty() {
            return Err(AppError::validation_displayable("送检单号不能为空"));
        }
        if req.color_no.trim().is_empty() || req.process_no.trim().is_empty() {
            return Err(AppError::validation_displayable(
                "取样批次口径为同品种同色号同工艺，色号与工艺缺一不可",
            ));
        }
        let product_exists = ProductEntity::find_by_id(req.product_id)
            .one(self.db.as_ref())
            .await?
            .is_some();
        if !product_exists {
            return Err(AppError::not_found(format!(
                "产品 {} 不存在，送检单必须挂真实品种",
                req.product_id
            )));
        }
        let now = chrono::Utc::now();
        let tags = Self::join_exemption_tags(&req.exemption_tags)?;
        let active = ActiveModel {
            submission_no: ActiveValue::Set(req.submission_no),
            product_id: ActiveValue::Set(req.product_id),
            batch_no: ActiveValue::Set(req.batch_no),
            color_no: ActiveValue::Set(req.color_no),
            process_no: ActiveValue::Set(req.process_no),
            color_role: ActiveValue::Set(req.color_role),
            end_user_group: ActiveValue::Set(req.end_user_group),
            contact_type: ActiveValue::Set(req.contact_type),
            safety_category: ActiveValue::Set(req.safety_category),
            product_form: ActiveValue::Set(req.product_form),
            target_market: ActiveValue::Set(req.target_market),
            exemption_tags: ActiveValue::Set(tags),
            status: ActiveValue::Set(submission_status::DRAFT.to_string()),
            overall_verdict: ActiveValue::Set(None),
            remarks: ActiveValue::Set(req.remarks),
            created_by: ActiveValue::Set(req.created_by),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            ..Default::default()
        };
        Ok(active.insert(self.db.as_ref()).await?)
    }

    /// 分页列表（品种、状态、市场过滤）。分页语义与全仓一致：page 1-based，越界 400 不夹紧。
    pub async fn list(
        &self,
        params: SubmissionListParams,
    ) -> Result<(Vec<Model>, u64), AppError> {
        let page = params.page.unwrap_or(1);
        let page_size = params.page_size.unwrap_or(20);
        if page == 0 {
            return Err(AppError::validation_displayable(
                "page 必须是从 1 开始的页码，第一页请传 page=1",
            ));
        }
        if page_size == 0 || page_size > 100 {
            return Err(AppError::validation_displayable(
                "page_size 必须在 1-100 之间（每页返回的行数）",
            ));
        }
        let mut query = Submission::find();
        if let Some(product_id) = params.product_id {
            query = query.filter(Column::ProductId.eq(product_id));
        }
        if let Some(status) = &params.status {
            if !submission_status::is_valid(status) {
                return Err(AppError::validation_displayable(format!(
                    "状态 {} 不在送检单状态词表内（合法值 {}）",
                    status,
                    submission_status::ALL.join(",")
                )));
            }
            query = query.filter(Column::Status.eq(status));
        }
        if let Some(market) = &params.target_market {
            if !export_market::is_valid(market) {
                return Err(AppError::validation_displayable(format!(
                    "目标市场 {} 不在词表内（合法值 {}）",
                    market,
                    export_market::ALL.join(",")
                )));
            }
            query = query.filter(Column::TargetMarket.eq(market));
        }
        let paginator = query.order_by_desc(Column::CreatedAt).paginate(self.db.as_ref(), page_size);
        let (items, total) = paginate_with_total(paginator, page).await?;
        Ok((items, total))
    }

    pub async fn get(&self, id: i32) -> Result<Model, AppError> {
        Submission::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found(format!("送检单 {} 不存在", id)))
    }

    /// 修改送检要素：仅可编辑态（draft、sampling、sent）允许，reported 之后封账。
    ///
    /// 判据：可编辑集合取词表 `submission_status::EDITABLE` 单源；维度字段全量重校验，
    /// 与建单同一判据函数，杜绝"建单严、改单松"的漂移。
    pub async fn update(&self, id: i32, req: CreateSubmissionReq) -> Result<Model, AppError> {
        let model = self.get(id).await?;
        self.ensure_editable(&model)?;
        self.validate_dims(
            &req.end_user_group,
            &req.contact_type,
            &req.safety_category,
            &req.product_form,
            &req.target_market,
            &req.color_role,
            &req.exemption_tags,
        )?;
        let tags = Self::join_exemption_tags(&req.exemption_tags)?;
        let mut active: ActiveModel = model.into();
        active.batch_no = ActiveValue::Set(req.batch_no);
        active.color_no = ActiveValue::Set(req.color_no);
        active.process_no = ActiveValue::Set(req.process_no);
        active.color_role = ActiveValue::Set(req.color_role);
        active.end_user_group = ActiveValue::Set(req.end_user_group);
        active.contact_type = ActiveValue::Set(req.contact_type);
        active.safety_category = ActiveValue::Set(req.safety_category);
        active.product_form = ActiveValue::Set(req.product_form);
        active.target_market = ActiveValue::Set(req.target_market);
        active.exemption_tags = ActiveValue::Set(tags);
        active.remarks = ActiveValue::Set(req.remarks);
        active.updated_at = ActiveValue::Set(chrono::Utc::now());
        Ok(active.update(self.db.as_ref()).await?)
    }

    /// 草稿删除：仅 draft 态且尚无第三方报告挂单时允许物理删行。
    ///
    /// 判据：删除是草稿误建的唯一清理通道（状态机没有 draft 的回退态）；一旦有报告引用
    /// （报告外键 RESTRICT 的源头），必须走撤回留痕，不允许抹掉已送检事实。
    pub async fn delete_draft(&self, id: i32, operator: i32) -> Result<(), AppError> {
        let model = self.get(id).await?;
        if model.status != submission_status::DRAFT {
            return Err(AppError::business(format!(
                "送检单 {} 处于 {} 态，仅草稿可删除；已推进的单据请走撤回留痕",
                id, model.status
            )));
        }
        let report_count = ThirdPartyReport::find()
            .filter(ReportColumn::SubmissionId.eq(id))
            .count(self.db.as_ref())
            .await?;
        if report_count > 0 {
            return Err(AppError::business(format!(
                "送检单 {} 已挂 {} 份第三方报告，禁止删除草稿通道绕过报告留痕",
                id, report_count
            )));
        }
        SubmissionItem::delete_many()
            .filter(ItemColumn::SubmissionId.eq(id))
            .exec(self.db.as_ref())
            .await?;
        ChildCheck::delete_many()
            .filter(CheckColumn::SubmissionId.eq(id))
            .exec(self.db.as_ref())
            .await?;
        Submission::delete_by_id(id).exec(self.db.as_ref()).await?;
        tracing::info!(
            submission_id = id,
            operator,
            "纺织品安全送检单草稿已删除"
        );
        Ok(())
    }

    /// 状态迁移：白名单推进（draft→sampling→sent→reported 与 judged→closed、非终态→withdrawn）。
    ///
    /// 判据：reported 由报告登记动作自动产出、judged 由整单判定动作自动产出，本入口只承接
    /// 人工可推进的相邻档（draft→sampling 要求清单非空、sampling→sent）与撤回；
    /// judged 不允许手工直达（否则绕过判定汇总产出无依据结论）。
    /// 库存联动：推进到 sent 即取样批次进入送检期间，按"待检不出库"把范围内合格行降为待检；
    /// 从 sent 或 reported 撤回则解除该预防性挂起。返回（单据, 联动回显）。
    pub async fn set_status(
        &self,
        id: i32,
        to: &str,
        operator: i32,
    ) -> Result<(Model, StockLinkResult), AppError> {
        if !submission_status::is_valid(to) {
            return Err(AppError::validation_displayable(format!(
                "目标状态 {} 不在送检单状态词表内（合法值 {}）",
                to,
                submission_status::ALL.join(",")
            )));
        }
        let model = self.get(id).await?;
        let from = model.status.clone();
        if !ALLOWED_TRANSITIONS.contains(&(from.as_str(), to)) {
            return Err(AppError::business(format!(
                "送检单 {} 不允许从 {} 迁移到 {}；reported 由报告登记自动产出，judged 由整单判定自动产出，其余非法迁移拒绝",
                id, from, to
            )));
        }
        if to == submission_status::SAMPLING {
            let item_count = SubmissionItem::find()
                .filter(ItemColumn::SubmissionId.eq(id))
                .count(self.db.as_ref())
                .await?;
            if item_count == 0 {
                return Err(AppError::business(format!(
                    "送检单 {} 尚未生成检测项目清单，不能进入待取样",
                    id
                )));
            }
        }
        let mut active: ActiveModel = model.into();
        active.status = ActiveValue::Set(to.to_string());
        active.updated_at = ActiveValue::Set(chrono::Utc::now());
        let saved = active.update(self.db.as_ref()).await?;
        let link = match (from.as_str(), to) {
            (_, submission_status::SENT) => {
                self.apply_stock_link(&saved, StockLinkAction::HoldOnSent, operator)
                    .await?
            }
            (submission_status::SENT, submission_status::WITHDRAWN)
            | (submission_status::REPORTED, submission_status::WITHDRAWN) => {
                self.apply_stock_link(&saved, StockLinkAction::ReleaseOnWithdraw, operator)
                    .await?
            }
            _ => StockLinkResult {
                matched: 0,
                updated_stock_ids: Vec::new(),
                scope_note: "本次状态迁移不触及库存联动档位（仅 sent 推进与 sent 或 reported 的撤回改写质量状态）".to_string(),
            },
        };
        tracing::info!(
            submission_id = id,
            from,
            to,
            operator,
            link = link.scope_note.as_str(),
            "纺织品安全送检单状态已迁移"
        );
        Ok((saved, link))
    }

    /// 库存质量状态联动原语：按品种乘批次命中在库（正常）行，依动作语义改写 quality_status，
    /// 全部经 `AuditLogService::update_with_audit` 留痕。
    ///
    /// 判据：无批次号的送检单无法点名库存行，返回 matched 0 并在 scope_note 显式说明，
    /// 由调用方回显给操作者（不静默）；词表外的 quality_status 残值不改写（那是别的域的
    /// 脏数据，交各自域定性，本域不越权清洗）。
    async fn apply_stock_link(
        &self,
        model: &Model,
        action: StockLinkAction,
        operator: i32,
    ) -> Result<StockLinkResult, AppError> {
        let batch = model
            .batch_no
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .to_string();
        if batch.is_empty() {
            return Ok(StockLinkResult {
                matched: 0,
                updated_stock_ids: Vec::new(),
                scope_note: "送检单未登记批次号，库存行无法按品种乘批次点名联动；本次只记录单据级结论，如需冻结库存请按库存端点人工处置并在补录批次后重送判定".to_string(),
            });
        }
        let rows = InventoryStock::find()
            .filter(StockColumn::ProductId.eq(model.product_id))
            .filter(StockColumn::BatchNo.eq(batch.as_str()))
            .filter(StockColumn::StockStatus.eq(stock_status::NORMAL))
            .all(self.db.as_ref())
            .await?;
        let matched = rows.len() as u64;
        let mut updated: Vec<i32> = Vec::new();
        for row in rows {
            let target: Option<&'static str> = match (action, row.quality_status.as_str()) {
                (StockLinkAction::HoldOnSent, stock_quality::PASS) => Some(stock_quality::PENDING),
                (StockLinkAction::ReleaseOnWithdraw, stock_quality::PENDING) => {
                    Some(stock_quality::PASS)
                }
                (StockLinkAction::FreezeOnFail, stock_quality::PASS)
                | (StockLinkAction::FreezeOnFail, stock_quality::PENDING) => {
                    Some(stock_quality::FAIL)
                }
                (StockLinkAction::ReleaseOnPass, stock_quality::PENDING)
                | (StockLinkAction::ReleaseOnPass, stock_quality::FAIL) => {
                    Some(stock_quality::PASS)
                }
                    _ => None,
            };
            if let Some(target_value) = target {
                let mut active: StockActiveModel = row.into();
                active.quality_status = ActiveValue::Set(target_value.to_string());
                active.updated_at = ActiveValue::Set(chrono::Utc::now());
                let saved = AuditLogService::update_with_audit::<InventoryStock, _, _>(
                    &*self.db,
                    "inventory_stock",
                    active,
                    Some(operator),
                )
                .await?;
                updated.push(saved.id);
            }
        }
        tracing::info!(
            submission_id = model.id,
            action = action.label(),
            matched,
            updated = updated.len(),
            "纺织品安全合规联动库存质量状态完成"
        );
        Ok(StockLinkResult {
            matched,
            updated_stock_ids: updated,
            scope_note: format!(
                "动作 {}：品种 {} 批次 {} 命中在库行 {} 行，实际改写质量状态 {} 行",
                action.label(),
                model.product_id,
                batch,
                matched,
                updated.len()
            ),
        })
    }

    /// 生成（重生成）送检项目清单：按人群与接触方式与市场推项目集。
    ///
    /// 推导规则（全部来自项目主数据的 basis 分组，不硬编码项目编码）：
    /// - basis GB18401 的项目恒入列（法定基本安全项，逐项目是否考核由判定器按矩阵出 na）；
    /// - 人群为婴幼儿或儿童时加挂 basis GB31701 的加考项目；
    /// - 市场非内销时加挂 basis EXPORT 的项目（无种子行时为空集，不报错）。
    /// - extra_item_codes 为客户点名项，逐码存在性校验后以 CUSTOMER 归因入列。
    /// 重生成冲掉自动归因行、保留 CUSTOMER 行（人工点名不被机器重算抹掉）。
    /// 仅可编辑态允许。传给谁：handler generate_items 端点（POST /textile-submissions/{id}/items）。
    pub async fn generate_items(
        &self,
        id: i32,
        extra_item_codes: Vec<String>,
        operator: i32,
    ) -> Result<Vec<(ItemModel, TestItemModel)>, AppError> {
        let model = self.get(id).await?;
        self.ensure_editable(&model)?;

        // 推导规则逐条（全部来自项目主数据 basis 分组，不硬编码项目编码清单）
        let mut bases: Vec<(String, String)> = vec![(
            textile_basis_standard::GB18401.to_string(),
            textile_item_source::GB18401.to_string(),
        )];
        if model.end_user_group == textile_end_user_group::INFANT
            || model.end_user_group == textile_end_user_group::CHILD
        {
            bases.push((
                textile_basis_standard::GB31701.to_string(),
                textile_item_source::GB31701.to_string(),
            ));
        }
        if model.target_market != export_market::DOM {
            bases.push((
                textile_basis_standard::EXPORT.to_string(),
                textile_item_source::EXPORT.to_string(),
            ));
        }
        // (item_id, item_model, source) 三元组，先入归因优先（国标自动项排前）
        let mut picked: Vec<(i32, TestItemModel, String)> = Vec::new();
        for (basis, source) in &bases {
            let items = TestItem::find()
                .filter(ItemMasterColumn::BasisStandard.eq(basis.as_str()))
                .order_by(ItemMasterColumn::ItemCode, sea_orm::Order::Asc)
                .all(self.db.as_ref())
                .await?;
            for item in items {
                if !picked.iter().any(|(pid, _, _)| *pid == item.id) {
                    picked.push((item.id, item, source.clone()));
                }
            }
        }
        for code in &extra_item_codes {
            let item = TestItem::find()
                .filter(ItemMasterColumn::ItemCode.eq(code.as_str()))
                .one(self.db.as_ref())
                .await?
                .ok_or_else(|| {
                    AppError::validation_displayable(format!(
                        "点名加验项目编码 {} 不在检测项目主数据内，请先维护项目主数据",
                        code
                    ))
                })?;
            // 点名项覆盖既有自动归因：客户点名的行按 CUSTOMER 归因存活于下次重生成
            if let Some(slot) = picked.iter_mut().find(|(pid, _, _)| *pid == item.id) {
                slot.2 = textile_item_source::CUSTOMER.to_string();
            } else {
                picked.push((item.id, item, textile_item_source::CUSTOMER.to_string()));
            }
        }
        if picked.is_empty() {
            return Err(AppError::business(
                "推导结果为空项目集：主数据未播种或点名项缺失，拒绝生成空清单",
            ));
        }

        // 冲掉自动归因行、保留既有 CUSTOMER 行；随后全量重插（已存在且保留的点名行不重复插）
        self.auto_rows_delete(id).exec(self.db.as_ref()).await?;
        let existing_kept = SubmissionItem::find()
            .filter(ItemColumn::SubmissionId.eq(id))
            .all(self.db.as_ref())
            .await?;
        let now = chrono::Utc::now();
        let mut out: Vec<(ItemModel, TestItemModel)> = Vec::new();
        for (item_id, item, source) in picked {
            if existing_kept
                .iter()
                .any(|row| row.item_id == item_id && row.source == source)
            {
                let row = existing_kept
                    .iter()
                    .find(|row| row.item_id == item_id && row.source == source)
                    .cloned()
                    .expect("上行 find 命中即存在，clone 后必可再定位");
                out.push((row, item));
                continue;
            }
            if existing_kept.iter().any(|row| row.item_id == item_id) {
                // 同项目已存在归因行（必为 CUSTOMER 保留行）：不再插第二条，同项唯一键约束
                let row = existing_kept
                    .iter()
                    .find(|row| row.item_id == item_id)
                    .cloned()
                    .expect("存在性判定与取值同源");
                out.push((row, item));
                continue;
            }
            let active = ItemActiveModel {
                submission_id: ActiveValue::Set(id),
                item_id: ActiveValue::Set(item_id),
                source: ActiveValue::Set(source),
                created_at: ActiveValue::Set(now),
                ..Default::default()
            };
            let saved = active.insert(self.db.as_ref()).await?;
            out.push((saved, item));
        }
        tracing::info!(
            submission_id = id,
            items = out.len(),
            operator,
            "送检项目清单已按人群接触方式与市场推导生成"
        );
        Ok(out)
    }

    fn auto_rows_delete(&self, submission_id: i32) -> DeleteMany<SubmissionItem> {
        SubmissionItem::delete_many().filter(
            Condition::all()
                .add(ItemColumn::SubmissionId.eq(submission_id))
                .add(ItemColumn::Source.ne(textile_item_source::CUSTOMER)),
        )
    }

    /// 童装加考核查登记（upsert 语义，按 item_code 定位行）。
    ///
    /// 判据：项目必须存在且 basis 为 GB31701（加考条款挂加考表，基本安全项走报告结果表）；
    /// check_kind 快照取项目主数据，manual 项目必须给 manual_conclusion 或留空待补
    /// （整单判定时缺结论即拒绝完成）；measurable 项目的数值与 flag 按项目测量类型校验
    /// （numeric 要值、grade 要半级编码域内值、boolean 与 sensory 要词表内 flag）。
    /// reported 之前任意状态可登记；judged 与 closed 与 withdrawn 封账。
    pub async fn upsert_child_checks(
        &self,
        id: i32,
        rows: Vec<ChildCheckUpsertReq>,
        operator: i32,
    ) -> Result<Vec<ChildCheckModel>, AppError> {
        let model = self.get(id).await?;
        if ![
            submission_status::DRAFT,
            submission_status::SAMPLING,
            submission_status::SENT,
            submission_status::REPORTED,
        ]
        .contains(&model.status.as_str())
        {
            return Err(AppError::business(format!(
                "送检单 {} 处于 {} 态，童装加考核查登记已封账",
                id, model.status
            )));
        }
        let mut out = Vec::new();
        for req in rows {
            let item = TestItem::find()
                .filter(ItemMasterColumn::ItemCode.eq(req.item_code.as_str()))
                .one(self.db.as_ref())
                .await?
                .ok_or_else(|| {
                    AppError::validation_displayable(format!(
                        "加考项目编码 {} 不在检测项目主数据内",
                        req.item_code
                    ))
                })?;
            if item.basis_standard != textile_basis_standard::GB31701 {
                return Err(AppError::business(format!(
                    "项目 {}（{}）不是 GB31701 加考项目；基本安全项目走第三方报告逐项目结果通道",
                    item.item_code, item.item_name
                )));
            }
            Self::validate_check_input(&item, &req)?;
            let existing = ChildCheck::find()
                .filter(CheckColumn::SubmissionId.eq(id))
                .filter(CheckColumn::ItemId.eq(item.id))
                .one(self.db.as_ref())
                .await?;
            let now = chrono::Utc::now();
            let saved = match existing {
                Some(row) => {
                    let mut active: CheckActiveModel = row.into();
                    active.measured_value = ActiveValue::Set(req.measured_value);
                    active.measured_unit = ActiveValue::Set(req.measured_unit);
                    active.measured_flag = ActiveValue::Set(req.measured_flag);
                    active.check_desc = ActiveValue::Set(req.check_desc);
                    active.manual_conclusion = ActiveValue::Set(req.manual_conclusion);
                    // 重录实测即复位待判定：旧结论不得跨输入版本存活
                    active.verdict = ActiveValue::Set(textile_verdict::PENDING.to_string());
                    active.judged_at = ActiveValue::Set(None);
                    active.updated_at = ActiveValue::Set(now);
                    active.update(self.db.as_ref()).await?
                }
                None => {
                    let active = CheckActiveModel {
                        submission_id: ActiveValue::Set(id),
                        item_id: ActiveValue::Set(item.id),
                        check_kind: ActiveValue::Set(item.check_kind.clone()),
                        measured_value: ActiveValue::Set(req.measured_value),
                        measured_unit: ActiveValue::Set(req.measured_unit),
                        measured_flag: ActiveValue::Set(req.measured_flag),
                        check_desc: ActiveValue::Set(req.check_desc),
                        manual_conclusion: ActiveValue::Set(req.manual_conclusion),
                        verdict: ActiveValue::Set(textile_verdict::PENDING.to_string()),
                        judged_at: ActiveValue::Set(None),
                        created_at: ActiveValue::Set(now),
                        updated_at: ActiveValue::Set(now),
                        ..Default::default()
                    };
                    active.insert(self.db.as_ref()).await?
                }
            };
            out.push(saved);
        }
        tracing::info!(
            submission_id = id,
            rows = out.len(),
            operator,
            "GB 31701 童装加考核查已登记"
        );
        Ok(out)
    }

    /// 整单判定汇总：判童装 measurable 行、搬运 manual 结论、按已受理报告的逐项目结果汇总整单。
    ///
    /// 前置（任一不满足即拒绝，不落半态）：状态为 reported；至少一份已受理且在有效期内的
    /// 报告；全部在有效期内已受理报告的逐项目结果均已判定；全部童装核查行均已判定（manual
    /// 缺结论即拒绝完成，不许假称自动判定）。
    /// 过期点名：已受理但有效期已过（或缺有效期这类无法核验的行）的报告排除出汇总口径并
    /// 逐枚点名回给调用方，绝不静默消费过期结论。
    /// 汇总三态：任一行 fail 则整单 fail；无 fail 且存在 pass 则整单 pass；
    /// 全部 na 则整单 na（无一项适用的极端形态如实呈现，不折算成合格）。
    /// 库存联动：fail 冻结批次库存（不合格不出库），pass 放行，na 维持送检挂起并点名。
    /// 产出：童装 measurable 行与 manual 行的 verdict、送检单 overall_verdict 与 judged 态、
    /// 联动回显与过期告警清单。
    pub async fn judge_submission(
        &self,
        id: i32,
        operator: i32,
    ) -> Result<JudgementOutcome, AppError> {
        let model = self.get(id).await?;
        if model.status != submission_status::REPORTED {
            return Err(AppError::business(format!(
                "送检单 {} 处于 {} 态，整单判定只从已收报告态推进",
                id, model.status
            )));
        }
        let reports = ThirdPartyReport::find()
            .filter(ReportColumn::SubmissionId.eq(id))
            .all(self.db.as_ref())
            .await?;
        let accepted_all: Vec<_> = reports
            .iter()
            .filter(|r| {
                r.acceptance_status == crate::models::status::report_acceptance_status::ACCEPTED
            })
            .collect();
        if accepted_all.is_empty() {
            return Err(AppError::business(format!(
                "送检单 {} 没有已受理的第三方报告，完整性门未放行前不得出整单结论",
                id
            )));
        }
        let today = chrono::Utc::now().date_naive();
        let mut expired_reports_named: Vec<String> = Vec::new();
        let accepted: Vec<_> = accepted_all
            .into_iter()
            .filter(|r| match r.valid_until {
                Some(vu) if today <= vu => true,
                _ => {
                    expired_reports_named.push(r.report_no.clone());
                    false
                }
            })
            .collect();
        if accepted.is_empty() {
            return Err(AppError::business(format!(
                "送检单 {} 的已受理报告全部超过有效期或缺有效期登记（点名 {}），过期结论不得作为归档依据；请取得有效报告或走复检更新报告后再判定",
                id,
                expired_reports_named.join("、")
            )));
        }
        if !expired_reports_named.is_empty() {
            tracing::warn!(
                submission_id = id,
                expired = expired_reports_named.join("|"),
                "整单判定已受理但过期的报告被排除出汇总口径（点名告警，不静默）"
            );
        }
        let mut verdicts: Vec<String> = Vec::new();
        for report in &accepted {
            let rows = ReportResult::find()
                .filter(ResultColumn::ReportId.eq(report.id))
                .all(self.db.as_ref())
                .await?;
            for row in &rows {
                if !textile_verdict::JUDGED.contains(&row.verdict.as_str()) {
                    return Err(AppError::business(format!(
                        "报告 {} 仍有待判定结果行（result id {}），请先跑报告逐项目判定",
                        report.report_no, row.id
                    )));
                }
                verdicts.push(row.verdict.clone());
            }
        }
        let tags_snapshot = Self::split_exemption_tags(model.exemption_tags.as_deref())?;
        let checks = ChildCheck::find()
            .filter(CheckColumn::SubmissionId.eq(id))
            .all(self.db.as_ref())
            .await?;
        for row in checks {
            let item = TestItem::find_by_id(row.item_id)
                .one(self.db.as_ref())
                .await?
                .ok_or_else(|| AppError::business(format!("童装核查行 {} 的项目主数据缺失", row.id)))?;
            let verdict = if row.check_kind == textile_check_kind::MANUAL {
                match row.manual_conclusion.as_deref() {
                    Some(c) if textile_verdict::JUDGED.contains(&c) => c.to_string(),
                    _ => {
                        return Err(AppError::business(format!(
                            "项目 {}（{}）是人工核查项且结论位为空或非法，整单判定拒绝完成；条款项必须有结论位，绝不代判",
                            item.item_code, item.item_name
                        )));
                    }
                }
            } else {
                let measured = MeasuredInput {
                    value: row.measured_value,
                    text: row.measured_flag.clone(),
                };
                let rules = fetch_rules_for_item(&self.db, item.id).await?;
                let snapshot: Vec<CandidateRule> = rules.iter().map(CandidateRule::from).collect();
                let selected = crate::services::textile_safety_limit_judge::select_candidates(
                    &snapshot,
                    &JudgeDims {
                        safety_category: &model.safety_category,
                        product_form: &model.product_form,
                        condition_tags: &tags_snapshot,
                    },
                );
                if selected.is_empty() {
                    return Err(AppError::business(format!(
                        "可测加考项目 {}（{}）在矩阵中无限值行，请先维护判定表",
                        item.item_code, item.item_name
                    )));
                }
                crate::services::textile_safety_limit_judge::judge_conjunction(&selected, &measured)?
                    .into_verdict_token()
                    .to_string()
            };
            let now = chrono::Utc::now();
            let mut active: CheckActiveModel = row.into();
            active.verdict = ActiveValue::Set(verdict.clone());
            active.judged_at = ActiveValue::Set(Some(now));
            active.updated_at = ActiveValue::Set(now);
            active.update(self.db.as_ref()).await?;
            verdicts.push(verdict);
        }
        if verdicts.is_empty() {
            return Err(AppError::business(format!(
                "送检单 {} 没有任何已判定的逐项目结果，整单结论必须建立在真实判定上",
                id
            )));
        }
        let overall = if verdicts.iter().any(|v| v == textile_verdict::FAIL) {
            textile_verdict::FAIL
        } else if verdicts.iter().any(|v| v == textile_verdict::PASS) {
            textile_verdict::PASS
        } else {
            textile_verdict::NOT_APPLICABLE
        };
        let now = chrono::Utc::now();
        let mut active: ActiveModel = model.into();
        active.overall_verdict = ActiveValue::Set(Some(overall.to_string()));
        active.status = ActiveValue::Set(submission_status::JUDGED.to_string());
        active.updated_at = ActiveValue::Set(now);
        let saved = active.update(self.db.as_ref()).await?;
        let stock_link = match overall {
            textile_verdict::FAIL => {
                self.apply_stock_link(&saved, StockLinkAction::FreezeOnFail, operator)
                    .await?
            }
            textile_verdict::PASS => {
                self.apply_stock_link(&saved, StockLinkAction::ReleaseOnPass, operator)
                    .await?
            }
            _ => StockLinkResult {
                matched: 0,
                updated_stock_ids: Vec::new(),
                scope_note: "整单结论为全部不适用（na），送检期间的待检挂起不解除；请质量岗复核豁免口径后经库存端点人工改判".to_string(),
            },
        };
        tracing::info!(
            submission_id = id,
            overall,
            operator,
            link = stock_link.scope_note.as_str(),
            expired = expired_reports_named.join("|"),
            "纺织品安全送检单整单判定完成"
        );
        Ok(JudgementOutcome {
            submission: saved,
            stock_link,
            expired_reports_named,
        })
    }

    /// 查询送检项目清单（带项目主数据快照供展示，join 由两趟查询完成，SeaORM 无定义关系）。
    pub async fn list_items(&self, id: i32) -> Result<Vec<(ItemModel, TestItemModel)>, AppError> {
        let rows = SubmissionItem::find()
            .filter(ItemColumn::SubmissionId.eq(id))
            .all(self.db.as_ref())
            .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let item = TestItem::find_by_id(row.item_id)
                .one(self.db.as_ref())
                .await?
                .ok_or_else(|| AppError::business(format!("清单行 {} 的项目主数据缺失", row.id)))?;
            out.push((row, item));
        }
        Ok(out)
    }

    pub async fn list_child_checks(&self, id: i32) -> Result<Vec<ChildCheckModel>, AppError> {
        Ok(ChildCheck::find()
            .filter(CheckColumn::SubmissionId.eq(id))
            .order_by(CheckColumn::ItemId, sea_orm::Order::Asc)
            .all(self.db.as_ref())
            .await?)
    }

    /// 供 resolve 前复用的公开判定入口（报告服务按单行判定同一套特异度与合取语义）。
    pub async fn resolve_item_verdict(
        &self,
        item: &TestItemModel,
        dims: &JudgeDims<'_>,
        measured: &MeasuredInput,
    ) -> Result<String, AppError> {
        resolve_verdict(&self.db, item, dims, measured).await
    }

    fn validate_dims(
        &self,
        end_user_group: &str,
        contact_type: &str,
        safety_category: &str,
        product_form: &str,
        target_market: &str,
        color_role: &str,
        exemption_tags: &[String],
    ) -> Result<(), AppError> {
        let checks: [(&str, bool, &str); 6] = [
            (end_user_group, textile_end_user_group::is_valid(end_user_group), "终端人群"),
            (contact_type, textile_contact_type::is_valid(contact_type), "接触方式"),
            (safety_category, textile_safety_category::is_valid(safety_category), "安全类别"),
            (product_form, textile_product_form::is_valid(product_form), "产品形态"),
            (target_market, export_market::is_valid(target_market), "目标市场"),
            (color_role, sampling_color_role::is_valid(color_role), "色号角色"),
        ];
        for (value, ok, label) in checks {
            if !ok {
                return Err(AppError::validation_displayable(format!(
                    "{} 取值 {} 不在本域词表内（合法值见词表模块，存储用带前缀稳定码）",
                    label, value
                )));
            }
        }
        for tag in exemption_tags {
            if !textile_exemption_tag::is_valid(tag) {
                return Err(AppError::validation_displayable(format!(
                    "豁免标签 {} 不在词表内（合法值 {}）",
                    tag,
                    textile_exemption_tag::ALL.join(",")
                )));
            }
        }
        let required = textile_end_user_group::required_safety_category(end_user_group)
            .ok_or_else(|| AppError::validation_displayable("终端人群词表外"))?;
        let declared_rank = textile_safety_category::strictness_rank(safety_category)
            .ok_or_else(|| AppError::validation_displayable("安全类别词表外"))?;
        let required_rank = textile_safety_category::strictness_rank(required)
            .ok_or_else(|| AppError::internal("人群到类别映射漂移，请核查词表"))?;
        if declared_rank > required_rank {
            return Err(AppError::validation_displayable(format!(
                "{}人群的法定最低安全类别是 {}，申报 {} 松于要求（GB 31701 第 4 章），请更正申报类别或人群口径",
                end_user_group, required, safety_category
            )));
        }
        if contact_type == textile_contact_type::INFANT_USE
            && safety_category != textile_safety_category::CLASS_A
        {
            return Err(AppError::validation_displayable(
                "接触方式为婴幼儿专用档时安全类别必须是 A 类（GB 18401 第 5 章），两者口径冲突",
            ));
        }
        Ok(())
    }

    fn join_exemption_tags(tags: &[String]) -> Result<Option<String>, AppError> {
        if tags.is_empty() {
            return Ok(None);
        }
        let mut seen: Vec<String> = Vec::new();
        for t in tags {
            if seen.contains(t) {
                continue;
            }
            seen.push(t.clone());
        }
        Ok(Some(seen.join(",")))
    }

    pub fn split_exemption_tags(raw: Option<&str>) -> Result<Vec<String>, AppError> {
        let Some(raw) = raw else { return Ok(Vec::new()) };
        if raw.trim().is_empty() {
            return Ok(Vec::new());
        }
        let mut out = Vec::new();
        for token in raw.split(',') {
            if token.is_empty() {
                return Err(AppError::internal(format!(
                    "送检单豁免标签列形态漂移（出现空 token）：{:?}，逗号串必须无空段",
                    raw
                )));
            }
            if !textile_exemption_tag::is_valid(token) {
                return Err(AppError::internal(format!(
                    "送检单豁免标签列含词表外 token {}：{:?}，判定链路拒绝消费脏词表列",
                    token, raw
                )));
            }
            out.push(token.to_string());
        }
        Ok(out)
    }

    fn ensure_editable(&self, model: &Model) -> Result<(), AppError> {
        if !submission_status::EDITABLE.contains(&model.status.as_str()) {
            return Err(AppError::business(format!(
                "送检单 {} 处于 {} 态，已收报告后送检要素封账（可编辑态 {}）",
                model.id,
                model.status,
                submission_status::EDITABLE.join(",")
            )));
        }
        Ok(())
    }

    /// measurable 与 manual 行的录入形态校验（按项目测量类型分派，输入不足 fail-visible）。
    fn validate_check_input(
        item: &TestItemModel,
        req: &ChildCheckUpsertReq,
    ) -> Result<(), AppError> {
        if item.check_kind == textile_check_kind::MANUAL {
            if req.measured_value.is_some() || req.measured_flag.is_some() {
                return Err(AppError::validation_displayable(format!(
                    "项目 {}（{}）是人工核查项，只接受条目描述与结论位，不接受实测值（判定器绝不代判）",
                    item.item_code, item.item_name
                )));
            }
            if let Some(c) = req.manual_conclusion.as_deref() {
                if !textile_verdict::JUDGED.contains(&c) {
                    return Err(AppError::validation_displayable(format!(
                        "人工结论位取值 {} 非法（合法值 {}）",
                        c,
                        textile_verdict::JUDGED.join(",")
                    )));
                }
            }
            return Ok(());
        }
        if req.manual_conclusion.is_some() {
            return Err(AppError::validation_displayable(format!(
                "项目 {}（{}）是可测值项目，结论由判定器按矩阵产出，不接受人工结论位覆盖",
                item.item_code, item.item_name
            )));
        }
        match item.item_kind.as_str() {
            textile_item_kind::NUMERIC | textile_item_kind::GRADE => {
                if req.measured_value.is_none() {
                    return Err(AppError::validation_displayable(format!(
                        "项目 {}（{}）需要实测数值（等级型按半级编码录入），当前为空",
                        item.item_code, item.item_name
                    )));
                }
            }
            textile_item_kind::SENSORY | textile_item_kind::BOOLEAN => {
                let flag = req.measured_flag.as_deref().ok_or_else(|| {
                    AppError::validation_displayable(format!(
                        "项目 {}（{}）需要实测 flag（{} 或 {}），当前为空",
                        item.item_code,
                        item.item_name,
                        textile_flag_value::NONE,
                        textile_flag_value::DETECTED
                    ))
                })?;
                if flag != textile_flag_value::NONE && flag != textile_flag_value::DETECTED {
                    return Err(AppError::validation_displayable(format!(
                        "项目 {}（{}）的实测 flag {} 非法（合法值 {}）",
                        item.item_code,
                        item.item_name,
                        flag,
                        textile_flag_value::ALL.join(",")
                    )));
                }
            }
            other => {
                return Err(AppError::internal(format!(
                    "项目 {} 的测量类型 {} 不在词表内，主数据被旁路写坏",
                    item.item_code, other
                )));
            }
        }
        Ok(())
    }
}
