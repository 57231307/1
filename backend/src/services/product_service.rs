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

use rust_decimal::Decimal;
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

/// 更新产品色号参数对象（批次 330 v10 复审 P3 修复：引入参数对象消除 too_many_arguments 警告）
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
/// 批次 125 v8 复审 P1 修复：注入 search_syncer 实现 PG→ES 写入同步。；create/update/delete 事务提交后调用 sync_product / delete_product 同步到 ES；ES 同步失败仅记录 tracing::warn!（最终一致性），不回滚 PG 事务；批次 D10 拆分：impl 块已拆到 `product_ops` 子模块（sync/crud/color/import_export）。；`db` / `search_syncer` 字段为 `pub(crate)` 供 ops 子模块访问。
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
/// 批次 339 v10 复审 P3 修复：引入参数对象消除 create_product 的 too_many_arguments 警告。；聚合创建产品所需的全部字段（含面料行业字段），避免函数签名携带 19 个参数。
#[derive(Debug, Clone)]
pub struct CreateProductArgs {
    /// 产品名称
    pub name: String,
    /// 产品编码
    pub code: String,
    /// 产品条码（EAN/UPC 等，未编码为空）
    pub barcode: Option<String>,
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
    /// V15 P1 合规字段：面料执行标准号（GB/T 406-2018 等）
    pub execution_standard: Option<String>,
    /// V15 P1 合规字段：生产厂名（《产品质量法》第 27 条）
    pub factory_name: Option<String>,
    /// V15 P1 合规字段：生产厂址
    pub factory_address: Option<String>,
    /// V15 P1 合规字段：产品等级（优等品/一等品/合格品）
    pub product_grade: Option<String>,
    /// 每匹米数（匹↔米换算元数据，可选数值，None 表示未录入）
    pub meters_per_piece: Option<Decimal>,
    /// 每卷米数（卷↔米换算元数据，可选数值，None 表示未录入）
    pub meters_per_roll: Option<Decimal>,
}

/// 更新产品参数对象
/// 批次 339 v10 复审 P3 修复：引入参数对象消除 update_product 的 too_many_arguments 警告。；聚合更新产品所需的全部字段（含面料行业字段），避免函数签名携带 19 个参数。
#[derive(Debug, Clone)]
pub struct UpdateProductArgs {
    /// 产品 ID
    pub id: i32,
    /// 产品名称
    pub name: Option<String>,
    /// 产品条码（未编码为空）
    pub barcode: Option<String>,
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
    /// V15 P1 合规字段：面料执行标准号（GB/T 406-2018 等）
    pub execution_standard: Option<String>,
    /// V15 P1 合规字段：生产厂名
    pub factory_name: Option<String>,
    /// V15 P1 合规字段：生产厂址
    pub factory_address: Option<String>,
    /// V15 P1 合规字段：产品等级（优等品/一等品/合格品）
    pub product_grade: Option<String>,
    /// 每匹米数（匹↔米换算元数据，三态部分更新）。外层 None 即键缺席、本列保持原值，
    /// Some(None) 即显式 null、清空为 NULL，Some(Some(v)) 即覆盖为该值。
    pub meters_per_piece: Option<Option<Decimal>>,
    /// 每卷米数（卷↔米换算元数据，三态部分更新），形态与语义同上。
    pub meters_per_roll: Option<Option<Decimal>>,
    /// 操作人 ID
    pub user_id: i32,
}

/// 匹与卷的每单位米数同源补齐。
///
/// 读的是请求携带的每匹米数与每卷米数两值的存在性。只带其中一个时把另一列补成同值，
/// 两个都带时各自原样返回（含不等也不取其一、不互相覆写），两个都不带时不补齐、
/// 不引入默认值掩盖缺键。
/// 写给产品单条创建与单条更新两条链路（handler 层），以及批量创建与批量更新
/// （batch_service 层），结果交给 CreateProductArgs 与 UpdateProductArgs 的同名两字段，
/// 再由 product_ops 的 crud 与 batch_service 的 ActiveModel 落库。
/// 存的是 products 表的每匹米数与每卷米数两列，DECIMAL 可空，NULL 表示未录入。
/// 为什么泛型。创建侧 T 为 Decimal、单层 Option 即两态（带值或缺值）。更新侧 T 为
/// Option<Decimal>、双层 Option 即三态（覆盖、清空、不动）。补齐只判断外层存在性，
/// 单层 Some(x) 与双层 Some(显式声明值) 都被视为本次带了这个键，复制到另一键。
/// 为什么放服务层而不是只留在 handler。批量链路在 batch_service 也需要同一份补齐语义，
/// 服务与处理器共用一个真源，避免两处各写一套导致语义漂移。
/// 边界。更新侧只声明清空每匹米数时，每卷米数按同源补成同样的清空声明。
pub fn pair_piece_roll_meters<T: Clone>(
    meters_per_piece: Option<T>,
    meters_per_roll: Option<T>,
) -> (Option<T>, Option<T>) {
    match (meters_per_piece, meters_per_roll) {
        (Some(piece), None) => (Some(piece.clone()), Some(piece)),
        (None, Some(roll)) => (Some(roll.clone()), Some(roll)),
        (piece, roll) => (piece, roll),
    }
}
