//! `customers.customer_type` 唯一词表模块
//!
//! 本模块是 **customers 表 customer_type 列**允许值集合在源码中的唯一出现点；
//! 各写入口一律引此处，不得再抄内联白名单。
//!
//! ## 本列承载的语义：渠道（channel）
//!
//! `ALLOWED` 五值一律是**渠道** token，写入方与读取方都按"这批货从哪条销路走"理解。
//!
//! ⚠️ `vip` / `normal` 是**分层**（tier）词，不属本列值域，一律被 `check`/`validate`
//! 拒绝。本仓**没有**客户分层列可作载体：`chk_tier_customer_level` 挂在
//! `color_price_tiers`（定价阶梯表）上，约束的是价格阶梯的 `customer_level`，
//! 不是 `customers` 行的分层，不能拿来承接 `vip`/`normal`。要落分层需另立列 +
//! 另立词表模块（属维度重设计，另立功能波）。
//!
//! ⚠️ DB 侧对本列的 CHECK 约束名为 `chk_customers_customer_type`（与本 `ALLOWED`
//! 逐值同源，由同批迁移引入并 `SET NOT NULL DEFAULT 'other'`）。迁移侧与本模块必须
//! 同一集合，任何一侧单独增删值都算契约漂移。
//!
//! ## 缺省值：一律 `other`
//!
//! 未知渠道不猜零售：调用方没给类型时写 `OTHER`（语义=渠道未知），而不是
//! `RETAIL`（那是替业务方做了一个可能为假的断言）。线索转化入口的"来源=线索"
//! 这一事实由 `customers.source` 列承载（`services/crm/lead.rs` 写 `source='lead'`），
//! 不靠 `customer_type` 里的自建 token 表达。
//!
//! 区分"缺省补值"与"显式业务值"：只有 **请求未提供该字段时代码自行补出的值**才算缺省，
//! 受本条约束（现仅 `validate(None)` 一条路径 + 线索转化 service 层缺省分支）；
//! 业务语境本身确定的值属于显式写值，不受缺省口径影响——
//! 例：客户导入入口写 `RETAIL`（导入模板承载的就是零售销路客户，业务口径明确）。
//!
//! ## 与其它同名取值集合的关系
//!
//! ⚠️ 与 `crm_pool_handler` 公海规则的 `customer_type`（`all/wholesale/retail/vip`）
//! 是**两回事**：后者是 pool 规则表的"规则作用域"取值，不是 customers 行的类型值，
//! 不得并入本词表（该处已留注释点名；该作用域运行时只有 `all` 被消费，
//! per-type 为死分支，维度重设计另立功能波）。
//!
//! ⚠️ 与 CLV 分层的 segment 词表（`services/crm/cust.rs` 的
//! champion/loyal/potential/at_risk/lost，`models/customer_lifetime_value.rs`）
//! 也是**两回事**：`potential` 属分层侧 token，禁止出现在本列（含其大写形式
//! `"POTENTIAL"`）——把分层词写进渠道列就是混维撞名，读取侧按渠道精确匹配必然永不命中。

use crate::utils::error::AppError;

/// 零售 token——`ALLOWED` 成员之一，当前唯一的显式业务写值：
/// 客户导入入口（`services/import_export_ops/import.rs`）按导入模板语境写本常量，
/// 大小写必须与读侧的小写精确匹配一致（`services/customer_ops/crud.rs:136`、
/// `services/customer_ops/query.rs:82`）——写大写 `"RETAIL"` 会让导入客户按类型筛选
/// 恒 0 行。**不是缺省值**：缺省见 `OTHER`。
pub const RETAIL: &str = "retail";

/// "其它/未知渠道"token——`ALLOWED` 成员之一，同时是**全仓唯一的缺省写值**：
/// - `validate(None)`（标准创建入口未提供 `customer_type` 时的补值路径）；
/// - 线索转化 service 层缺省分支（`services/crm/lead.rs` `build_customer_active`）；
/// - DB 列的 `NOT NULL DEFAULT`（迁移侧 `SET NOT NULL DEFAULT 'other'`，与本常量同源）。
///
/// 选型依据：缺省要表达"渠道未知"，不能猜成零售——把未知当已知写死会污染
/// 渠道维度的全部下游筛选与统计。
pub const OTHER: &str = "other";

/// customers.customer_type 当前允许值集合（渠道维度唯一出现点，与 DB 侧
/// `chk_customers_customer_type` 逐值同源；不增不减，分层词 `vip`/`normal` 永不并入）。
pub const ALLOWED: &[&str] = &[RETAIL, "wholesale", "distributor", "manufacturer", OTHER];

/// 核心成员判定：行为 = 重构前 `customer_handler.rs:76-89` 内联校验器的逐字符等价搬运——
/// **不 trim、不做大小写归一**，精确匹配 `ALLOWED`；拒绝时规则 code 仍为
/// `invalid_customer_type`（该 code 经 `readable_validation_errors` 无 message 分支
/// 出参形如 `customer_type: invalid_customer_type`，族别 VALIDATION_ERROR / 400）。
///
/// 供标准入口 `#[validate(custom(function = validate_customer_type))]` 通道复用。
pub fn check(raw: &str) -> Result<(), validator::ValidationError> {
    if ALLOWED.contains(&raw) {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid_customer_type"))
    }
}

/// 非 validator 通道写入口的统一校验函数（增强 PUT、线索转化、标准创建入口的
/// 缺省归一均经此），行为与标准入口逐值等价：
/// - `None` → `Ok(OTHER)`：**缺省补值唯一路径**，语义=渠道未知（不猜零售）；
///   **更新类入口的"缺省=不改列"语义由调用方以 `Option` 承载**（调用方仅在 `Some`
///   时校验并回传，None 原样透传，不得把缺失静默写成 any 具体渠道）；
/// - `Some(v)` 命中 `ALLOWED` → `Ok(v)`：**原文返回**，不 trim、不归一大小写；
/// - `Some(v)` 未命中 → `Err`：拒绝族别与文案与标准入口**同一构造函数**产生——
///   构造 `validator::ValidationErrors`（字段名 `customer_type`、code
///   `invalid_customer_type`）后走 `From<validator::ValidationErrors> for AppError`
///   （`utils/error.rs:641-645`）既有通道，出参信封 400 + `VALIDATION_ERROR`，
///   message `customer_type: invalid_customer_type`。非法值一律显式拒绝，不静默兜底。
pub fn validate(raw: Option<&str>) -> Result<String, AppError> {
    match raw {
        None => Ok(OTHER.to_string()),
        Some(v) => match check(v) {
            Ok(()) => Ok(v.to_string()),
            Err(e) => {
                let mut errors = validator::ValidationErrors::new();
                errors.add("customer_type", e);
                Err(AppError::from(errors))
            }
        },
    }
}
