//! 供应商资质过期门控 + 到期预警扫描（采购侧）
//!
//! 决策分级（用户拍板，不做一刀切）：
//! - 法定许可类资质（排污许可、危化品经营/使用许可等）过期 → 阻断**新建采购订单**；
//! - 一般资质（营业执照、ISO 等）过期 → 不阻断存量供应商下单，但产生**到期预警**，
//!   且阻断「新供应商首单」（该供应商尚无脱离草稿的采购订单）；
//! - 收货侧不阻断（在途履约不应被卡死，对齐 SAP QM 把硬门槛放在使用决策/入库定性
//!   而非物理收货的分层）——本模块因此**不**挂接 `po/receipt.rs`；
//! - 允许具备 `supplier_qualification:waive` 权限键的角色做**显式例外放行**：
//!   必须携带放行原因，同一事务内落审计行（audit_logs），全程 tracing 不静默。
//!
//! 门控落点与事务性：由 `po/order_ops/crud.rs::validate_order_request` 在
//! **创建订单事务内**调用（与黑名单门控 `check_supplier_not_blacklisted` 同一落点、
//! 同一 `txn` 读取），消除 TOCTOU；资质行经泛型 `C: ConnectionTrait` 在调用方事务内读取。
//!
//! 「是否过期」判定**唯一派生源**：`supplier_service.rs::qualification_is_expired`
//! （valid_until 含当日之后视为过期）。本模块只调用、绝不重写第二套日期比较。
//!
//! 分类词表（法定许可类 vs 一般类）的真实取值来源见
//! [`statutory_qualification_keywords`]：仓库中 `qualification_type` 为自由文本
//! （前端 el-input 直录，DDL 无 CHECK，种子/迁移均无稳定取值集合），因此**不虚构枚举**，
//! 采用可回退形态——「法定类词表命中才阻断，其余过期一律只出预警」；分组关键词
//! 集中为本常量单点维护（禁止散落硬编码），可用环境变量覆盖。

use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde::Serialize;
use std::sync::{Arc, LazyLock};

use crate::models::audit_log::{OperationType, Severity};
use crate::models::status::purchase_order as po_status;
use crate::models::{audit_log, purchase_order, supplier, supplier_qualification, user};
use crate::services::role_permission_service::RolePermissionService;
use crate::services::supplier_service::SupplierService;
use crate::utils::error::AppError;

// ==================== 配置常量（单点维护，env 可覆盖） ====================

/// 法定许可类资质分类关键词的内置默认值。
///
/// 取值依据（行业法定许可，缺一即不得供货）：《排污许可管理条例》排污许可证、
/// 《危险化学品安全管理条例》危化品经营/使用/安全生产许可。
/// 文本按「qualification_type 子串命中」判定——仓库 `qualification_type` 为自由文本
/// （m0008 DDL VARCHAR(50) 无 CHECK、历史列注释仅举例 `营业执照/税务登记证/组织机构代码证/ISO9001`、
/// 前端为 el-input 直录、无种子枚举），不存在可依赖的稳定取值集合，故不虚构枚举、
/// 只维护这一处关键词分组表。
const DEFAULT_STATUTORY_QUALIFICATION_KEYWORDS: &str = "排污许可,危化品,危险化学品,安全生产许可";

/// 覆盖入口：`SUPPLIER_QUALIFICATION_STATUTORY_KEYWORDS`（逗号分隔，命中任一关键词即判法定许可类）。
pub static STATUTORY_QUALIFICATION_KEYWORDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    let raw = std::env::var("SUPPLIER_QUALIFICATION_STATUTORY_KEYWORDS")
        .unwrap_or_else(|_| DEFAULT_STATUTORY_QUALIFICATION_KEYWORDS.to_string());
    let keywords: Vec<String> = raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if keywords.is_empty() {
        tracing::error!(
            raw = %raw,
            "SUPPLIER_QUALIFICATION_STATUTORY_KEYWORDS 解析为空，回退内置默认法定类关键词（配置错误显式留痕，不静默）"
        );
        return DEFAULT_STATUTORY_QUALIFICATION_KEYWORDS
            .split(',')
            .map(|s| s.to_string())
            .collect();
    }
    tracing::info!(keywords = ?keywords, "供应商资质法定许可类关键词表已加载");
    keywords
});

/// 到期预警提前天数档位（从远到近），对齐劳动合同到期扫描的三级预警分层范式。
/// 覆盖入口：`SUPPLIER_QUALIFICATION_WARNING_DAYS`（逗号分隔，如 "90,60,30"）。
pub static EXPIRY_WARNING_DAYS_TIERS: LazyLock<Vec<i64>> = LazyLock::new(|| {
    let raw = std::env::var("SUPPLIER_QUALIFICATION_WARNING_DAYS")
        .unwrap_or_else(|_| "90,60,30".to_string());
    let tiers: Vec<i64> = raw
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.parse::<i64>())
        .filter_map(|r| match r {
            Ok(v) if v > 0 => Some(v),
            Ok(_) => {
                tracing::error!("SUPPLIER_QUALIFICATION_WARNING_DAYS 含非正数档位，已忽略（显式留痕）");
                None
            }
            Err(e) => {
                tracing::error!(error = %e, "SUPPLIER_QUALIFICATION_WARNING_DAYS 档位解析失败，已忽略（显式留痕）");
                None
            }
        })
        .collect();
    if tiers.is_empty() {
        tracing::error!(
            raw = %raw,
            "SUPPLIER_QUALIFICATION_WARNING_DAYS 无有效档位，回退默认 90/60/30（配置错误显式留痕，不静默）"
        );
        return vec![90, 60, 30];
    }
    tiers
});

/// 例外放行权限键（role_permissions 表 resource_type + action 的真实取值口径：
/// resource_type 用资质实体名 `supplier_qualification`，action 用放行语义 `waive`）。
/// 该键需在角色权限管理中授权给采购管理/质量授权等角色；未授权角色调用放行一律 403。
pub const WAIVER_PERMISSION_RESOURCE: &str = "supplier_qualification";
pub const WAIVER_PERMISSION_ACTION: &str = "waive";

/// 例外放行审计行的资源动作（audit_logs.action 自由文本列的稳定 token；
/// operation_type 枚举无 WAIVE 档，落 OTHER，由 action + resource_type 表达真实语义）。
const WAIVER_AUDIT_ACTION: &str = "WAIVE";

// ==================== 类型 ====================

/// 命中阻断的类别（决定拒绝文案与审计口径）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockingScope {
    /// 法定许可类资质过期（阻断一切新建采购订单）
    Statutory,
    /// 一般资质过期且该供应商尚无脱离草稿的历史订单（新供应商首单）
    GeneralFirstOrder,
}

impl BlockingScope {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Statutory => "法定许可类资质过期",
            Self::GeneralFirstOrder => "一般资质过期（新供应商首单）",
        }
    }
}

/// 单条已过期资质摘要（供拒绝文案、预警出参与审计快照复用；不含内部记录 ID）
#[derive(Debug, Clone, Serialize)]
pub struct ExpiredQualificationSummary {
    pub qualification_name: String,
    pub qualification_type: String,
    pub valid_until: chrono::NaiveDate,
}

/// 门控结果（Ok 分支携带预警信息，供调用方日志与测试断言；不影响响应结构）
#[derive(Debug, Clone)]
pub struct QualificationGateOutcome {
    /// 是否走了例外放行（命中阻断但携原因且有权限）
    pub waived: bool,
    /// 命中的阻断类别（waive 或未发生时为 None）
    pub blocked_scope: Option<BlockingScope>,
    /// 已过期法定许可类资质（放行/未命中时也可能非空）
    pub expired_statutory: Vec<ExpiredQualificationSummary>,
    /// 已过期一般资质（预警来源）
    pub expired_general: Vec<ExpiredQualificationSummary>,
}

/// 资质到期预警级别（输出侧 token，不落库；序列化口径与劳动合同预警一致）
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum QualificationExpiryLevel {
    Expired,
    Warning,
}

/// 到期预警项（扫描端点出参；供应商名称按既有 LeftJoin/批量富化口径取值）
#[derive(Debug, Clone, Serialize)]
pub struct QualificationExpiryWarning {
    pub supplier_id: i32,
    pub supplier_name: Option<String>,
    pub qualification_id: i32,
    pub qualification_name: String,
    pub qualification_type: String,
    /// 分类：statutory=法定许可类 / general=一般类（分组表见模块头）
    pub category: &'static str,
    pub valid_until: chrono::NaiveDate,
    pub days_until_expiry: i64,
    /// 是否已过期——唯一派生源 `SupplierService::qualification_is_expired` 的如实转写
    pub expired: bool,
    pub level: QualificationExpiryLevel,
}

// ==================== 服务 ====================

/// 供应商资质过期门控 + 到期预警扫描服务
pub struct SupplierQualificationGate {
    db: Arc<DatabaseConnection>,
}

impl SupplierQualificationGate {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// qualification_type 是否命中法定许可类词表（子串命中任一关键词）。
    /// 比较一律取 [`STATUTORY_QUALIFICATION_KEYWORDS`] 单点分组表，禁止在别处散写关键词字符串。
    pub fn is_statutory_qualification(qualification_type: &str) -> bool {
        STATUTORY_QUALIFICATION_KEYWORDS
            .iter()
            .any(|kw| !kw.is_empty() && qualification_type.contains(kw.as_str()))
    }

    /// 采购门控核心：校验供应商资质过期情况并决定是否阻断新建采购订单。
    ///
    /// 事务性：资质行与「是否首单」的探测全部经泛型 `conn` 在**调用方事务内**读取
    /// （落点与 `check_supplier_not_blacklisted` 一致，见 crud.rs::validate_order_request），
    /// 消除「先查后开事务」的 TOCTOU；取数失败经 `?` 显式上抛（禁止"查不到资质就放行"式 fail-open）。
    ///
    /// 命中阻断时的拒绝文案用 `AppError::business_displayable` 如实外显
    /// （只涉及该供应商资质类别/名称与公开业务规则，不含内部记录 ID/他人数据）。
    pub async fn check_purchase_order_gate<C: ConnectionTrait>(
        &self,
        conn: &C,
        supplier_id: i32,
        user_id: i32,
        waiver_reason: Option<&str>,
    ) -> Result<QualificationGateOutcome, AppError> {
        // 1. 事务内读取该供应商全部资质行（失败显式上抛，不吞、不兜底放行）
        let qualifications = supplier_qualification::Entity::find()
            .filter(supplier_qualification::Column::SupplierId.eq(supplier_id))
            .order_by(supplier_qualification::Column::ValidUntil, Order::Asc)
            .all(conn)
            .await?;

        // 2. 过期判定：唯一派生源 SupplierService::qualification_is_expired（禁止第二套日期比较）
        let mut expired_statutory: Vec<ExpiredQualificationSummary> = Vec::new();
        let mut expired_general: Vec<ExpiredQualificationSummary> = Vec::new();
        for q in &qualifications {
            if !SupplierService::qualification_is_expired(q.valid_until) {
                continue;
            }
            let summary = ExpiredQualificationSummary {
                qualification_name: q.qualification_name.clone(),
                qualification_type: q.qualification_type.clone(),
                valid_until: q.valid_until,
            };
            if Self::is_statutory_qualification(&q.qualification_type) {
                expired_statutory.push(summary);
            } else {
                expired_general.push(summary);
            }
        }

        // 3. 未命中任何过期资质：放行（正常路径也留 info 日志，不静默）
        if expired_statutory.is_empty() && expired_general.is_empty() {
            tracing::info!(
                supplier_id,
                qualification_rows = qualifications.len(),
                "供应商资质门控：无过期资质，放行新建采购订单"
            );
            return Ok(QualificationGateOutcome {
                waived: false,
                blocked_scope: None,
                expired_statutory,
                expired_general,
            });
        }

        // 4. 阻断分级：法定类过期 → 阻断一切新建单；一般类过期 → 仅阻断新供应商首单，
        //    存量供应商（已有脱离草稿的订单）放行但计入预警。
        let blocked_scope = if !expired_statutory.is_empty() {
            Some(BlockingScope::Statutory)
        } else if Self::is_first_order_supplier(conn, supplier_id).await? {
            Some(BlockingScope::GeneralFirstOrder)
        } else {
            None
        };

        match blocked_scope {
            Some(scope) => {
                let reason = waiver_reason.map(str::trim).filter(|s| !s.is_empty());
                match reason {
                    // 4a. 显式例外放行：权限校验 + 同事务落审计，二者皆过才放行
                    Some(reason) => {
                        self.enforce_waiver_permission(conn, user_id).await?;
                        self.record_waiver_audit(
                            conn,
                            user_id,
                            supplier_id,
                            scope,
                            reason,
                            &expired_statutory,
                            &expired_general,
                        )
                        .await?;
                        tracing::warn!(
                            supplier_id,
                            user_id,
                            scope = scope.as_str(),
                            reason,
                            "供应商资质门控：例外放行（已落审计行）"
                        );
                        Ok(QualificationGateOutcome {
                            waived: true,
                            blocked_scope: Some(scope),
                            expired_statutory,
                            expired_general,
                        })
                    }
                    // 4b. 未携带放行原因（或仅空白）：拒绝。文案只说明哪一类资质过期与续期/放行路径，
                    //     不含内部记录 ID；business 会被脱敏成「业务处理失败」，必须 displayable 外显。
                    None => {
                        let reason_provided = waiver_reason.is_some();
                        if reason_provided {
                            // 字段传了但全为空白：不能当「没带原因」静默处理，显式拒绝
                            return Err(AppError::business_displayable(
                                "资质例外放行必须填写放行原因，原因不能为空白",
                            ));
                        }
                        tracing::warn!(
                            supplier_id,
                            user_id,
                            scope = scope.as_str(),
                            "供应商资质门控：阻断新建采购订单（未申请例外放行）"
                        );
                        Err(AppError::business_displayable(Self::blocking_message(
                            scope,
                            &expired_statutory,
                            &expired_general,
                        )))
                    }
                }
            }
            // 4c. 一般资质过期、存量供应商（非首单）：不阻断，但预警必须显式留痕
            None => {
                tracing::warn!(
                    supplier_id,
                    expired_general = ?expired_general
                        .iter()
                        .map(|g| g.qualification_name.as_str())
                        .collect::<Vec<_>>(),
                    "供应商资质门控：存在已过期一般资质（存量供应商，不阻断），已计入到期预警"
                );
                Ok(QualificationGateOutcome {
                    waived: false,
                    blocked_scope: None,
                    expired_statutory,
                    expired_general,
                })
            }
        }
    }

    /// 「新供应商首单」探测：该供应商是否存在**已脱离草稿**（order_status != DRAFT）的采购订单。
    /// DRAFT 是下单动作的中间产物（首单门命中时草稿根本不会产生），不能作为成交历史证据。
    /// 词表取写入方常量 `models::status::purchase_order::DRAFT`，不手写第二套字符串。
    async fn is_first_order_supplier<C: ConnectionTrait>(
        conn: &C,
        supplier_id: i32,
    ) -> Result<bool, AppError> {
        let proven_history = purchase_order::Entity::find()
            .filter(purchase_order::Column::SupplierId.eq(supplier_id))
            .filter(purchase_order::Column::OrderStatus.ne(po_status::DRAFT))
            .count(conn)
            .await?;
        Ok(proven_history == 0)
    }

    /// 拒绝文案（用户可见；只含资质名称/类别与业务规则，不含记录 ID/他人数据）
    fn blocking_message(
        scope: BlockingScope,
        expired_statutory: &[ExpiredQualificationSummary],
        expired_general: &[ExpiredQualificationSummary],
    ) -> String {
        let names = |list: &[ExpiredQualificationSummary]| {
            list.iter()
                .map(|q| q.qualification_name.as_str())
                .collect::<Vec<_>>()
                .join("、")
        };
        match scope {
            BlockingScope::Statutory => format!(
                "该供应商的法定许可类资质已过期（{}），已阻断新建采购订单。请先完成资质续期；确有紧急业务时，需由具备资质例外放行权限的角色携带放行原因下单。",
                names(expired_statutory)
            ),
            BlockingScope::GeneralFirstOrder => format!(
                "该供应商的一般资质已过期（{}），且尚无已脱离草稿的采购订单，新供应商首单已阻断。请先完成资质续期；确有紧急业务时，需由具备资质例外放行权限的角色携带放行原因下单。",
                names(expired_general)
            ),
        }
    }

    /// 例外放行权限校验：操作人角色须持有 `supplier_qualification:waive` 权限键
    /// （admin 角色由 RolePermissionService 内部按角色编码绕过，此处不复制判定逻辑）。
    /// role_id 读取走调用方事务 `conn`（与资质读取同源）；权限表读取经连接池，
    /// 属操作人自身授权状态、无 TOCTOU 业务风险。
    async fn enforce_waiver_permission<C: ConnectionTrait>(
        &self,
        conn: &C,
        user_id: i32,
    ) -> Result<(), AppError> {
        let operator = user::Entity::find()
            .filter(user::Column::Id.eq(user_id))
            .select_only()
            .column(user::Column::RoleId)
            .into_tuple::<Option<i32>>()
            .one(conn)
            .await?
            .ok_or_else(|| AppError::internal("操作人账号不存在，无法校验资质例外放行权限"))?;

        let role_id = match operator {
            Some(role_id) => role_id,
            None => {
                tracing::warn!(user_id, "资质例外放行被拒：操作人未分配角色（fail-closed）");
                return Err(AppError::permission_denied(
                    "执行供应商资质例外放行需要放行权限，当前账号未获得该授权",
                ));
            }
        };

        let allowed = RolePermissionService::new(self.db.clone())
            .check_permission(
                role_id,
                WAIVER_PERMISSION_RESOURCE,
                WAIVER_PERMISSION_ACTION,
                None,
            )
            .await?;
        if !allowed {
            tracing::warn!(
                user_id,
                resource = WAIVER_PERMISSION_RESOURCE,
                action = WAIVER_PERMISSION_ACTION,
                "资质例外放行被拒：角色缺少放行权限键（fail-closed）"
            );
            return Err(AppError::permission_denied(
                "执行供应商资质例外放行需要放行权限，当前账号未获得该授权",
            ));
        }
        Ok(())
    }

    /// 例外放行审计行：与门控判定、订单创建在**同一事务**写入 audit_logs
    /// （下单被回滚则审计同滚，杜绝「有放行单据无审计」的错位；资源类型用真实实体名）。
    #[allow(
        clippy::too_many_arguments,
        reason = "审计快照需同时携带操作人/供应商/范围/原因/双侧过期清单"
    )]
    async fn record_waiver_audit<C: ConnectionTrait>(
        &self,
        conn: &C,
        user_id: i32,
        supplier_id: i32,
        scope: BlockingScope,
        reason: &str,
        expired_statutory: &[ExpiredQualificationSummary],
        expired_general: &[ExpiredQualificationSummary],
    ) -> Result<(), AppError> {
        let username: Option<String> = user::Entity::find()
            .filter(user::Column::Id.eq(user_id))
            .select_only()
            .column(user::Column::Username)
            .into_tuple()
            .one(conn)
            .await?
            .flatten();

        let snapshot = serde_json::json!({
            "supplier_id": supplier_id,
            "scope": scope.as_str(),
            "waiver_reason": reason,
            "expired_statutory": expired_statutory,
            "expired_general": expired_general,
        });

        let log = audit_log::ActiveModel {
            user_id: Set(Some(user_id)),
            username: Set(username),
            action: Set(WAIVER_AUDIT_ACTION.to_string()),
            resource_type: Set(Some(WAIVER_PERMISSION_RESOURCE.to_string())),
            resource_id: Set(Some(supplier_id.to_string())),
            description: Set(Some(format!(
                "供应商资质例外放行（{}）：放行原因：{}",
                scope.as_str(),
                reason
            ))),
            created_at: Set(Some(Utc::now())),
            operation_type: Set(Some(OperationType::Other.as_str().to_string())),
            severity: Set(Some(Severity::Warn.as_str().to_string())),
            after_snapshot: Set(Some(audit_log::AuditValue(snapshot))),
            ..Default::default()
        };
        log.insert(conn).await?;
        Ok(())
    }

    /// 到期预警扫描：全量资质按唯一派生源重算过期态，输出「已过期」与
    /// 「进入预警档位（默认 90/60/30 天，`SUPPLIER_QUALIFICATION_WARNING_DAYS` 可覆盖）」两级。
    ///
    /// 分层对齐劳动合同 `POST /labor-contracts/scan-expiry-warnings`：按需触发、不新建 job、
    /// 只读不写（is_expired 列的写入/读出重算归供应商域，本扫描不跨域改写他人表）。
    pub async fn scan_expiry_warnings(&self) -> Result<Vec<QualificationExpiryWarning>, AppError> {
        let today = Utc::now().date_naive();
        let rows = supplier_qualification::Entity::find()
            .order_by(supplier_qualification::Column::ValidUntil, Order::Asc)
            .all(&*self.db)
            .await?;

        let max_tier = *EXPIRY_WARNING_DAYS_TIERS.iter().max().unwrap_or(&90);
        let mut warnings: Vec<QualificationExpiryWarning> = Vec::new();
        for q in rows {
            // 过期判定同样走唯一派生源，禁止第二套日期比较
            let expired = SupplierService::qualification_is_expired(q.valid_until);
            let days_until_expiry = (q.valid_until - today).num_days();
            if !expired && days_until_expiry > max_tier {
                continue;
            }
            let category = if Self::is_statutory_qualification(&q.qualification_type) {
                "statutory"
            } else {
                "general"
            };
            warnings.push(QualificationExpiryWarning {
                supplier_id: q.supplier_id,
                supplier_name: None,
                qualification_id: q.id,
                qualification_name: q.qualification_name,
                qualification_type: q.qualification_type,
                category,
                valid_until: q.valid_until,
                days_until_expiry,
                expired,
                level: if expired {
                    QualificationExpiryLevel::Expired
                } else {
                    QualificationExpiryLevel::Warning
                },
            });
        }

        // 供应商名称富化（批量一次查询，避免 N+1；与黑名单列表既有口径一致）
        let supplier_ids: Vec<i32> = warnings.iter().map(|w| w.supplier_id).collect();
        if !supplier_ids.is_empty() {
            let names: Vec<(i32, String)> = supplier::Entity::find()
                .filter(supplier::Column::Id.is_in(supplier_ids))
                .select_only()
                .column(supplier::Column::Id)
                .column(supplier::Column::SupplierName)
                .into_tuple()
                .all(&*self.db)
                .await?;
            let name_map: std::collections::HashMap<i32, String> = names.into_iter().collect();
            for w in warnings.iter_mut() {
                w.supplier_name = name_map.get(&w.supplier_id).cloned();
            }
        }

        warnings.sort_by_key(|w| w.days_until_expiry);
        tracing::info!(
            warning_count = warnings.len(),
            expired_count = warnings.iter().filter(|w| w.expired).count(),
            statutory_count = warnings
                .iter()
                .filter(|w| w.category == "statutory")
                .count(),
            "供应商资质到期预警扫描完成"
        );
        Ok(warnings)
    }
}
