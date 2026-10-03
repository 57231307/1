//! `customers.customer_type` 唯一词表模块（波0）
//!
//! 背景：`customer_type` 此前在多个写入口各写一套（标准入口内联白名单、增强 PUT 与
//! 线索转化无校验直落、导入硬编码大写 token、pool 规则另有四值作用域）。本模块是
//! **customers 表 customer_type 列**允许值集合在源码中的唯一出现点；六个入口一律改引
//! 此处（pool 规则列除外，见下）。
//!
//! ⚠️ 与 `crm_pool_handler` 公海规则的 `customer_type`（`all/wholesale/retail/vip`）
//! 是**两回事**：后者是 pool 规则表的"规则作用域"取值，不是 customers 行的类型值，
//! 不得并入本词表（该处已留注释点名）。
//!
//! ⚠️ 本波**不做值域收编/缩集**：渠道 vs 分层、`normal`/`retail`、`vip` 归属、
//! `POTENTIAL` 转化缺省、存量回填等业务口径待裁；`ALLOWED` 逐字符等于重构前
//! 标准入口内联校验器的五值白名单，不增不减。

use crate::utils::error::AppError;

/// 零售 token——`ALLOWED` 成员之一，同时是现行两个写入缺省点的真实值：
/// - 标准创建入口缺省（`handlers/customer_handler.rs` create_customer：
///   `payload.customer_type.unwrap_or_else(|| "retail")`）；
/// - 客户导入入口写值（`services/import_export_ops/import.rs`，波0 前误写大写
///   `"RETAIL"`，与全部读侧的小写精确匹配（`services/customer_ops/crud.rs:136`、
///   `services/customer_ops/query.rs:82`）永不相等 ⇒ 导入客户按类型筛选恒 0 行，
///   波0 归一为本常量）。
pub const RETAIL: &str = "retail";

/// customers.customer_type 当前允许值集合（唯一出现点；逐字符等于重构前内联白名单）。
pub const ALLOWED: &[&str] = &[RETAIL, "wholesale", "distributor", "manufacturer", "other"];

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
/// - `None` → `Ok(RETAIL)`：等于标准创建入口现行缺省写值；**更新类入口的"缺省=不改列"
///   语义由调用方以 `Option` 承载**（调用方仅在 `Some` 时校验并回传，None 原样透传，
///   不得把缺失静默写成 retail）；
/// - `Some(v)` 命中 `ALLOWED` → `Ok(v)`：**原文返回**，不 trim、不归一大小写；
/// - `Some(v)` 未命中 → `Err`：拒绝族别与文案与标准入口**同一构造函数**产生——
///   构造 `validator::ValidationErrors`（字段名 `customer_type`、code
///   `invalid_customer_type`）后走 `From<validator::ValidationErrors> for AppError`
///   （`utils/error.rs:641-645`）既有通道，出参信封 400 + `VALIDATION_ERROR`，
///   message `customer_type: invalid_customer_type`。非法值一律显式拒绝，不静默兜底。
pub fn validate(raw: Option<&str>) -> Result<String, AppError> {
    match raw {
        None => Ok(RETAIL.to_string()),
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
