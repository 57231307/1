//! 定制订单售后服务
//!
//! 5 种售后类型：客诉 / 维修 / 换货 / 退货 / 退款（权威白名单见 `create` 校验）
//! 状态机（权威词表 `models/status/sales.rs::custom_order_ext::AFTERSALES_ALL`）：
//! opened → accepted → processing → resolved → evaluated → closed；rejected/closed 为终态
//! 创建时间: 2026-06-17

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, JoinType, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait, Set, TransactionTrait,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;

use crate::container::AppState;
use crate::models::after_sales::{self, ActiveModel, Entity};
use crate::models::custom_order_response_dto::AfterSalesInfo;
use crate::models::customer;
use crate::models::quality_issue;
use crate::models::status::custom_order_ext as ext;
use crate::utils::error::AppError;
use crate::utils::pagination::paginate_with_total;

/// 售后工单状态权威词表 = `models/status/sales.rs::custom_order_ext::AFTERSALES_ALL`
/// （本服务是唯一写入方，状态机节点集与词表逐 token 相等，见 `AFTERSALES_TRANSITIONS`）。
///
/// DB 侧 CHECK `chk_aftersales_status`（`migration/src/domain/production/
/// m0044_integrate_unreferenced_migrations.rs:251`）目前缺 `accepted`/`evaluated`
/// 两态 ⇒ 写这两态撞 CHECK 被裸映射成 500（CI #4669 用例 65-01 的
/// `PUT /custom-orders/after-sales/{id}`）。补齐 CHECK 属迁移改动，
/// 已随本轮报告列出取值集合与 up/down 写法交数据库专家，此处不自写迁移。
const AFTERSALES_TRANSITIONS: &[(&str, &[&str])] = &[
    (
        ext::AFTERSALES_OPENED,
        &[
            ext::AFTERSALES_ACCEPTED,
            ext::AFTERSALES_REJECTED,
            ext::AFTERSALES_CLOSED,
        ],
    ),
    (
        ext::AFTERSALES_ACCEPTED,
        &[
            ext::AFTERSALES_PROCESSING,
            ext::AFTERSALES_REJECTED,
            ext::AFTERSALES_CLOSED,
        ],
    ),
    (
        ext::AFTERSALES_PROCESSING,
        &[
            ext::AFTERSALES_RESOLVED,
            ext::AFTERSALES_CLOSED,
            ext::AFTERSALES_REJECTED,
        ],
    ),
    (
        ext::AFTERSALES_RESOLVED,
        &[ext::AFTERSALES_EVALUATED, ext::AFTERSALES_CLOSED],
    ),
    (ext::AFTERSALES_EVALUATED, &[ext::AFTERSALES_CLOSED]),
    (ext::AFTERSALES_CLOSED, &[]),
    (ext::AFTERSALES_REJECTED, &[]),
];

/// 创建售后工单 DTO
///
/// 任务 #148 契约修复：`custom_order_id`（工单归属）由路由
/// `POST /custom-orders/{orderId}/after-sales` 的 path 参数权威提供，不再属于
/// 请求体字段。此前该字段为非 Option 必填且无 serde default，前端 payload 从不
/// 携带它，导致反序列化层 "missing field custom_order_id" —— 创建必失败；而
/// handler 又在反序列化成功后用 path 值覆盖 body 值，body 携带本无任何语义。
/// 若客户端仍在 body 发送 `custom_order_id`（含伪造他人订单 ID），serde 默认忽略
/// 未知字段，归属一律以 path 为准（越权防护不变，对齐 color_card items 先例：
/// `handlers/color_card/items.rs::create_color_item` 的 `service.create(id, dto)`）。
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CreateAfterSalesDto {
    pub customer_id: i32,
    /// 售后类型：complaint / repair / exchange / return_goods / refund
    pub issue_type: String,
    pub description: String,
    pub refund_amount: Option<Decimal>,
    /// V15 P0-B12：可选关联已有质量异常 ID
    pub quality_issue_id: Option<i64>,
    /// V15 P1 batch-19 缺陷 23.3.3：原因分类（quality/logistics/customer_preference/other）
    pub reason_category: Option<String>,
    /// V15 P1 batch-19 缺陷 23.3.3：原因明细
    pub reason_detail: Option<String>,
}

/// 更新售后工单 DTO
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct UpdateAfterSalesDto {
    pub status: Option<String>,
    pub resolution: Option<String>,
    pub refund_amount: Option<Decimal>,
}

/// 业务错误
#[derive(Debug, Error)]
pub enum AfterSalesError {
    #[error("售后工单不存在")]
    NotFound,
    #[error("非法状态: {0}")]
    InvalidState(String),
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
    /// 批次 263：接入 paginate_with_total（返回 AppError）所需的错误转换
    #[error("应用错误: {0}")]
    App(#[from] AppError),
    /// V15 P0-B12：售后工单已关联质量异常，禁止重复触发
    #[error("售后工单 {0} 已关联质量异常 {1}，禁止重复触发质量调查")]
    AlreadyLinked(i64, i64),
}

/// 售后服务
pub struct CustomOrderAfterSalesService {
    db: Arc<DatabaseConnection>,
}

impl CustomOrderAfterSalesService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self {
            db: state.db.clone(),
        }
    }

    /// 创建售后工单
    ///
    /// 任务 #148：`custom_order_id` 由调用方（handler）从路由 path 参数权威传入，
    /// 不从请求体 DTO 取值，客户端 body 伪造归属被结构性排除。
    pub async fn create(
        &self,
        custom_order_id: i64,
        dto: CreateAfterSalesDto,
    ) -> Result<after_sales::Model, AfterSalesError> {
        // 校验售后类型
        // V15 P2 23.3 缺陷1 修复：增加 return_goods（退货）类型。
        // 原因：审计计划 23.3 要求支持退货/换货/维修/投诉 4 类，原实现仅有
        // complaint/repair/exchange/refund，缺失"退货"独立类型；退货涉及物流收货、
        // 库存回库，与退款（财务出账）是不同业务。此处保留 refund 以兼容既有场景。
        if !["complaint", "repair", "exchange", "return_goods", "refund"]
            .contains(&dto.issue_type.as_str())
        {
            return Err(AfterSalesError::Validation(format!(
                "非法售后类型: {}",
                dto.issue_type
            )));
        }

        // 退款类型必须有金额
        if dto.issue_type == "refund" && dto.refund_amount.is_none() {
            return Err(AfterSalesError::Validation(
                "退款类型工单必须填写退款金额".to_string(),
            ));
        }

        let now = Utc::now();
        let active = ActiveModel {
            id: Default::default(),
            custom_order_id: Set(custom_order_id),
            issue_type: Set(dto.issue_type),
            customer_id: Set(dto.customer_id),
            description: Set(dto.description),
            status: Set(ext::AFTERSALES_OPENED.to_string()),
            opened_at: Set(now),
            closed_at: Set(None),
            resolution: Set(None),
            refund_amount: Set(dto.refund_amount),
            quality_issue_id: Set(dto.quality_issue_id),
            reason_category: Set(dto.reason_category),
            reason_detail: Set(dto.reason_detail),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        let result = active.insert(&*self.db).await?;
        Ok(result)
    }

    /// 更新售后工单
    pub async fn update(
        &self,
        id: i64,
        dto: UpdateAfterSalesDto,
    ) -> Result<after_sales::Model, AfterSalesError> {
        let existing = Entity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or(AfterSalesError::NotFound)?;

        // 校验状态转换
        if let Some(new_status) = &dto.status {
            // 取值域先行：非词表 token（大小写漂移/中文词/自造值）属用户输入越界，
            // 必须给出「合法取值有哪些」的可执行拒绝；否则它会一路走到状态机被判成
            // 「非法转换」，或（词表内但 DB CHECK 未覆盖时）被 CHECK 打成裸 500。
            if !ext::AFTERSALES_ALL.contains(&new_status.as_str()) {
                tracing::warn!(
                    "售后工单 {} 更新被拒：状态取值不在词表内（提交值 = {}）",
                    id,
                    new_status
                );
                return Err(AfterSalesError::Validation(format!(
                    "非法售后状态 '{}'，合法取值为：{}",
                    new_status,
                    ext::AFTERSALES_ALL.join("/")
                )));
            }
            if !is_valid_transition(&existing.status, new_status) {
                // 状态门回显的是本工单自身状态与用户提交值（公开业务规则）→ InvalidState
                tracing::warn!(
                    "售后工单 {} 状态转换被拒：{} → {}",
                    id,
                    existing.status,
                    new_status
                );
                return Err(AfterSalesError::InvalidState(format!(
                    "售后工单当前状态 {} 不能变更为 {}，请按状态机顺序流转",
                    existing.status, new_status
                )));
            }
        }

        let now = Utc::now();
        let mut active: ActiveModel = existing.into();
        if let Some(v) = &dto.status {
            active.status = Set(v.clone());
            // 终态/结论态落关闭时间：取值一律引用词表常量，禁止字面量
            if v == ext::AFTERSALES_CLOSED
                || v == ext::AFTERSALES_RESOLVED
                || v == ext::AFTERSALES_REJECTED
            {
                active.closed_at = Set(Some(now));
            }
        }
        if let Some(v) = dto.resolution {
            active.resolution = Set(Some(v));
        }
        if let Some(v) = dto.refund_amount {
            active.refund_amount = Set(Some(v));
        }
        active.updated_at = Set(now);
        let updated = active.update(&*self.db).await?;
        Ok(updated)
    }

    /// V15 P0-B12：触发质量调查
    /// 根据售后工单信息自动创建一条 quality_issue 记录，并回填 quality_issue_id 到售后工单。；用于售后→质量改进闭环：客诉/维修/换货类售后工单可触发质量调查，避免同类问题重复发生。；业务规则：1. 售后工单必须存在且未关闭（status != closed/rejected）；2. 售后工单不能已关联 quality_issue_id（禁止重复触发，避免产生冗余质量异常）；3. 自动创建的 quality_issue 字段映射：custom_order_id：从售后工单继承；issue_type："after_sales_reported"（售后上报）；severity：根据售后类型推断（complaint=high / repair=medium / exchange=low / refund=high）；description：售后工单描述；discovered_at：当前时间；status："open"；4. 注：8D 流程（quality_8d_service）当前不存在，本方法仅创建 quality_issue 记录，；8D 触发部分待后续批次补齐；参数说明：`after_sales_id`：售后工单 ID；`severity_override`：可选严重程度覆盖（high/medium/low），None 时按售后类型自动推断；返回：(更新后的售后工单, 新创建的质量异常)
    pub async fn trigger_quality_investigation(
        &self,
        after_sales_id: i64,
        severity_override: Option<String>,
    ) -> Result<(after_sales::Model, quality_issue::Model), AfterSalesError> {
        let existing = Entity::find_by_id(after_sales_id)
            .one(&*self.db)
            .await?
            .ok_or(AfterSalesError::NotFound)?;

        // 校验：已关闭/已拒绝的售后工单不允许触发质量调查
        if existing.status == ext::AFTERSALES_CLOSED || existing.status == ext::AFTERSALES_REJECTED
        {
            // 状态门：工单已处于关闭/拒绝终态，前置状态未满足，归业务族（InvalidState）；
            // 原走 Validation 通道族与本域其余状态门不一致
            return Err(AfterSalesError::InvalidState(format!(
                "售后工单状态为 {}，已关闭/拒绝的工单不允许触发质量调查",
                existing.status
            )));
        }

        // 校验：禁止重复触发（已关联 quality_issue_id 的工单不允许再次触发）
        if let Some(existing_qi_id) = existing.quality_issue_id {
            return Err(AfterSalesError::AlreadyLinked(
                after_sales_id,
                existing_qi_id,
            ));
        }

        // 严重程度推断：优先使用 severity_override，否则按售后类型自动推断
        let severity = severity_override.unwrap_or_else(|| match existing.issue_type.as_str() {
            "complaint" | "refund" => "high".to_string(),
            "repair" => "medium".to_string(),
            "exchange" => "low".to_string(),
            _ => "medium".to_string(),
        });

        let now = Utc::now();

        // 创建 quality_issue 记录
        let new_issue = quality_issue::ActiveModel {
            id: Default::default(),
            custom_order_id: Set(existing.custom_order_id),
            process_node_id: Set(None),
            issue_type: Set("after_sales_reported".to_string()),
            severity: Set(severity),
            description: Set(format!(
                "[售后工单 #{}] {}",
                after_sales_id, existing.description
            )),
            discovered_at: Set(now),
            resolved_at: Set(None),
            resolution: Set(None),
            status: Set("open".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        let inserted_issue = new_issue.insert(&*self.db).await?;

        // 回填 quality_issue_id 到售后工单
        let mut active: ActiveModel = existing.into();
        active.quality_issue_id = Set(Some(inserted_issue.id));
        active.updated_at = Set(now);
        let updated_after_sales = active.update(&*self.db).await?;

        Ok((updated_after_sales, inserted_issue))
    }

    /// 列出订单的售后工单（分页）
    ///
    /// 读侧为单次查询的关联名富化：`LEFT JOIN customers` +
    /// `column_as(customer::Column::CustomerName, "customer_name")` +
    /// `into_model::<AfterSalesInfo>()`（本仓唯一正解范式，对照
    /// `services/po/order_ops/crud.rs::list_orders`），客户名由 JOIN 结果忠实回显，
    /// 客户行缺失时 `customer_name` 为 NULL，禁止逐项再查或拼装假名。
    /// 批次 263：paginate_with_total 内部已做 page.saturating_sub(1) 偏移，调用方不可再减 1；
    /// clamp(1, 1000) 防 DoS（恶意请求 page=999999 不会导致超大偏移查询）。
    pub async fn list_by_order(
        &self,
        order_id: i64,
        page: u64,
        page_size: u64,
    ) -> Result<(Vec<AfterSalesInfo>, u64), AfterSalesError> {
        let query = Entity::find()
            .column_as(customer::Column::CustomerName, "customer_name")
            .join(JoinType::LeftJoin, after_sales::Relation::Customer.def())
            .filter(after_sales::Column::CustomOrderId.eq(order_id));

        let paginator = query
            .order_by_desc(after_sales::Column::OpenedAt)
            .into_model::<AfterSalesInfo>()
            .paginate(&*self.db, page_size);

        let (items, total) = paginate_with_total(paginator, page.clamp(1, 1000)).await?;
        Ok((items, total))
    }

    /// 与 `list_by_order` 同一条 LEFT JOIN 富化链路的单条回读（按 id）。
    ///
    /// 创建 / 更新端点写库后必须经本方法回读一次，出参的 `customer_name` 与
    /// 列表 / 详情同源（真实客户名或 NULL），禁止在构造点填 None 或拼装名蒙混。
    pub async fn find_dto_by_id(&self, id: i64) -> Result<Option<AfterSalesInfo>, AfterSalesError> {
        let dto = Entity::find()
            .column_as(customer::Column::CustomerName, "customer_name")
            .join(JoinType::LeftJoin, after_sales::Relation::Customer.def())
            .filter(after_sales::Column::Id.eq(id))
            .into_model::<AfterSalesInfo>()
            .one(&*self.db)
            .await?;
        Ok(dto)
    }

    /// V15 P1 batch-19 缺陷 23.3.2：受理售后工单（opened → accepted）
    pub async fn accept_after_sales(&self, id: i64) -> Result<after_sales::Model, AfterSalesError> {
        let txn = self.db.begin().await?;
        let existing = Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AfterSalesError::NotFound)?;

        if existing.status != ext::AFTERSALES_OPENED {
            return Err(AfterSalesError::InvalidState(format!(
                "售后工单当前状态 {} 尚未受理，只有已开启（{}）的工单可以受理",
                existing.status,
                ext::AFTERSALES_OPENED
            )));
        }

        let mut active: ActiveModel = existing.into();
        active.status = Set(ext::AFTERSALES_ACCEPTED.to_string());
        active.accepted_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// V15 P1 batch-19 缺陷 23.3.2：客户评价售后处理结果（resolved → evaluated）
    pub async fn evaluate_after_sales(
        &self,
        id: i64,
        score: i32,
        comment: Option<String>,
    ) -> Result<after_sales::Model, AfterSalesError> {
        if !(1..=5).contains(&score) {
            return Err(AfterSalesError::Validation(
                "评价分数必须在 1-5 之间".to_string(),
            ));
        }

        let txn = self.db.begin().await?;
        let existing = Entity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or(AfterSalesError::NotFound)?;

        if existing.status != ext::AFTERSALES_RESOLVED {
            return Err(AfterSalesError::InvalidState(format!(
                "售后工单当前状态 {} 尚未解决，只有已解决（{}）的工单可以评价",
                existing.status,
                ext::AFTERSALES_RESOLVED
            )));
        }

        let mut active: ActiveModel = existing.into();
        active.status = Set(ext::AFTERSALES_EVALUATED.to_string());
        active.evaluation_score = Set(Some(score));
        active.evaluation_comment = Set(comment);
        active.evaluated_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());
        let updated = active.update(&txn).await?;
        txn.commit().await?;
        Ok(updated)
    }

    /// V15 P1 batch-19 缺陷 23.3.3：生成售后原因 TOP5 月报
    pub async fn monthly_top5_report(
        &self,
        year: i32,
        month: u32,
    ) -> Result<MonthlyTop5Report, AfterSalesError> {
        use sea_orm::{FromQueryResult, Statement};

        #[derive(Debug, FromQueryResult)]
        struct Row {
            reason_category: String,
            reason_detail: Option<String>,
            count: i64,
        }

        let start_date = chrono::NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or(AfterSalesError::Validation("无效的年月".to_string()))?;
        let next_month = if month == 12 {
            chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .ok_or(AfterSalesError::Validation("无效的年月".to_string()))?;

        let sql = r#"
            SELECT reason_category, reason_detail, COUNT(*) as count
            FROM after_sales
            WHERE created_at >= $1 AND created_at < $2
            AND reason_category IS NOT NULL
            GROUP BY reason_category, reason_detail
            ORDER BY count DESC
            LIMIT 5
        "#;

        let rows = Row::find_by_statement(Statement::from_sql_and_values(
            sea_orm::DbBackend::Postgres,
            sql,
            vec![
                start_date.and_hms_opt(0, 0, 0).unwrap().into(),
                next_month.and_hms_opt(0, 0, 0).unwrap().into(),
            ],
        ))
        .all(&*self.db)
        .await?;

        let items: Vec<Top5ReasonItem> = rows
            .into_iter()
            .map(|r| Top5ReasonItem {
                reason_category: r.reason_category,
                reason_detail: r.reason_detail,
                count: r.count,
            })
            .collect();

        Ok(MonthlyTop5Report { year, month, items })
    }
}

/// V15 P1 batch-19 缺陷 23.3.3：售后 TOP5 原因月报 DTO
#[derive(Debug, Serialize)]
pub struct MonthlyTop5Report {
    pub year: i32,
    pub month: u32,
    pub items: Vec<Top5ReasonItem>,
}

/// V15 P1 batch-19 缺陷 23.3.3：TOP5 原因项
#[derive(Debug, Serialize)]
pub struct Top5ReasonItem {
    pub reason_category: String,
    pub reason_detail: Option<String>,
    pub count: i64,
}

/// 状态转换校验：直接由 `AFTERSALES_TRANSITIONS` 表驱动（与词表同源，不再另建 HashMap 字面量）。
/// 未知来源态（含 DB 里遗留的历史值）一律判非法，交由调用方给出可执行拒绝。
fn is_valid_transition(from: &str, to: &str) -> bool {
    AFTERSALES_TRANSITIONS
        .iter()
        .find(|(src, _)| *src == from)
        .map(|(_, targets)| targets.contains(&to))
        .unwrap_or(false)
}
