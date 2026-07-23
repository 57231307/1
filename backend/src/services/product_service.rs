//! 产品服务（面料行业版）— facade
//!
//! 本文件为 facade：保留 DTO struct 定义、`ProductService` struct 定义、
//! `new` 构造函数。impl 块已按职责拆分到 [`crate::services::product_ops`] 子模块：
//! - `sync`：ES 同步辅助（build_product_doc / sync_product_to_es）
//! - `crud`：产品 CRUD（list/get/create/update/delete + generate_product_code）
//! - `color`：色号管理（list/create/batch_create/update/delete product_color）
//! - `import_export`：CSV 导入导出（export/template/import + 字段校验/解析 helper）
//!
//! 批次 125 v8 复审 P1 修复：注入 search_syncer 实现 PG→ES 写入同步。
//! - create/update/delete 事务提交后调用 sync_product / delete_product 同步到 ES
//! - ES 同步失败仅记录 tracing::warn!（最终一致性），不回滚 PG 事务
//!
//! 批次 D10 拆分：`db` / `search_syncer` 字段改为 `pub(crate)` 供 ops 子模块访问；
//! 跨 ops 子模块调用的 `sync_product_to_es` 改为 `pub(crate)` 供 `crud` 子模块调用。
//! `ValidatedRowFields` struct 迁移到 `product_ops::import_export`（私有，仅 CSV 流程内部用）。

use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::search::{SearchClient, SearchSyncer};

/// 创建产品色号输入结构体
#[derive(Debug, Clone)]
pub struct CreateProductColorInput {
    pub color_no: String,
    pub color_name: String,
    pub pantone_code: Option<String>,
    pub color_type: String,
    pub dye_formula: Option<String>,
    pub extra_cost: f64,
}

/// 更新产品色号参数对象
///
/// 批次 330 v10 复审 P3 修复：引入参数对象消除 too_many_arguments 警告
#[derive(Debug)]
pub struct UpdateProductColorParams {
    /// 色号 ID
    pub id: i32,
    /// 色号名称
    pub color_name: Option<String>,
    /// 潘通色号
    pub pantone_code: Option<String>,
    /// 色号类型
    pub color_type: Option<String>,
    /// 染色配方
    pub dye_formula: Option<String>,
    /// 额外成本
    pub extra_cost: Option<f64>,
    /// 是否启用
    pub is_active: Option<bool>,
    /// 操作人 ID（审计日志）
    pub user_id: i32,
}

/// 产品服务（面料行业版）
///
/// 批次 125 v8 复审 P1 修复：注入 search_syncer 实现 PG→ES 写入同步。
/// - create/update/delete 事务提交后调用 sync_product / delete_product 同步到 ES
/// - ES 同步失败仅记录 tracing::warn!（最终一致性），不回滚 PG 事务
///
/// 批次 D10 拆分：impl 块已拆到 `product_ops` 子模块（sync/crud/color/import_export）。
/// `db` / `search_syncer` 字段为 `pub(crate)` 供 ops 子模块访问。
pub struct ProductService {
    pub(crate) db: Arc<DatabaseConnection>,
    /// ES 同步器（PG→ES 写入同步），批次 125 接入
    pub(crate) search_syncer: Arc<SearchSyncer>,
}

impl ProductService {
    pub fn new(db: Arc<DatabaseConnection>, search_client: Arc<dyn SearchClient>) -> Self {
        Self {
            db,
            search_syncer: Arc::new(SearchSyncer::new(search_client)),
        }
    }
}

/// 创建产品参数对象
///
/// 批次 339 v10 复审 P3 修复：引入参数对象消除 create_product 的 too_many_arguments 警告。
/// 聚合创建产品所需的全部字段（含面料行业字段），避免函数签名携带 19 个参数。
#[derive(Debug, Clone)]
pub struct CreateProductArgs {
    /// 产品名称
    pub name: String,
    /// 产品编码
    pub code: String,
    /// 分类 ID
    pub category_id: Option<i32>,
    /// 规格
    pub specification: Option<String>,
    /// 单位
    pub unit: String,
    /// 标准价
    pub standard_price: Option<f64>,
    /// 成本价
    pub cost_price: Option<f64>,
    /// 描述
    pub description: Option<String>,
    /// 状态
    pub status: String,
    /// 产品类型（面料行业字段）
    pub product_type: String,
    /// 面料成分
    pub fabric_composition: Option<String>,
    /// 纱支
    pub yarn_count: Option<String>,
    /// 密度
    pub density: Option<String>,
    /// 幅宽
    pub width: Option<f64>,
    /// 克重
    pub gram_weight: Option<f64>,
    /// 组织
    pub structure: Option<String>,
    /// 后整理
    pub finish: Option<String>,
    /// 最小起订量
    pub min_order_quantity: Option<f64>,
    /// 交期（天）
    pub lead_time: Option<i32>,
}

/// 更新产品参数对象
///
/// 批次 339 v10 复审 P3 修复：引入参数对象消除 update_product 的 too_many_arguments 警告。
/// 聚合更新产品所需的全部字段（含面料行业字段），避免函数签名携带 19 个参数。
#[derive(Debug, Clone)]
pub struct UpdateProductArgs {
    /// 产品 ID
    pub id: i32,
    /// 产品名称
    pub name: Option<String>,
    /// 规格
    pub specification: Option<String>,
    /// 单位
    pub unit: Option<String>,
    /// 标准价
    pub standard_price: Option<f64>,
    /// 成本价
    pub cost_price: Option<f64>,
    /// 描述
    pub description: Option<String>,
    /// 状态
    pub status: Option<String>,
    /// 产品类型
    pub product_type: Option<String>,
    /// 面料成分
    pub fabric_composition: Option<String>,
    /// 纱支
    pub yarn_count: Option<String>,
    /// 密度
    pub density: Option<String>,
    /// 幅宽
    pub width: Option<f64>,
    /// 克重
    pub gram_weight: Option<f64>,
    /// 组织
    pub structure: Option<String>,
    /// 后整理
    pub finish: Option<String>,
    /// 最小起订量
    pub min_order_quantity: Option<f64>,
    /// 交期（天）
    pub lead_time: Option<i32>,
    /// 操作人 ID
    pub user_id: i32,
}
