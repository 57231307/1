//! MRP物料需求计算引擎（facade，批次 490 D10-3b 拆分）
//!
//! 本文件为 facade 入口，仅保留 `MrpEngineService` struct + `new` 构造函数 + 单元测试。
//! 业务实现已按职责拆分到 `mrp_engine_ops/` 子模块（与 `mrp_engine_service` 同为 `crate::services` 下兄弟模块）：
//! - `mrp_engine_ops::types`：数据结构（请求/响应/参数对象，8 个 pub struct + 1 个 pub(crate) StockInfo）
//! - `mrp_engine_ops::stock`：库存查询与物料需求计算（5 方法）
//! - `mrp_engine_ops::bom`：BOM 递归展开（6 方法）
//! - `mrp_engine_ops::calculation`：MRP 计算执行（4 方法）
//! - `mrp_engine_ops::query`：结果查询与导出（4 方法）
//! - `mrp_engine_ops::order`：订单转换与产品列表（3 方法）
//!
//! 设计要点（与拆分前一致）：
//! - 基于 BOM 和库存数据计算物料需求，支持多层 BOM 展开和批量计算
//! - 库存查询支持单条/批量/缓存三种模式（v16 批次 43 修复 N+1）
//! - BOM 递归展开支持损耗率放大与提前期递减
//! - MRP 计算结果落库，支持查询/导出/转订单/取消
//! - 订单类型映射：PURCHASE→CONFIRMED、PRODUCTION→RELEASED
//! - 取消计算使用事务 + lock_exclusive 串行化并发状态变更
//!
//! 拆分兼容性：
//! - 外部 handler 通过 `crate::services::mrp_engine_service::MrpEngineService::new` 调用，路径不变
//! - 外部 handler 通过 `crate::services::mrp_engine_service::{MrpCalculationRequest, MrpCalculationItem, MaterialRequirement, MrpCalculationSummary, RequirementCalcParams, MrpExplodeQuery, MrpCalculationQuery}` 引用，路径不变（此处 re-export）
//! - `db` 字段使用 `pub(crate)` 可见性，mrp_engine_ops 子模块的 impl 块可直接访问
//! - impl 块分散在 mrp_engine_ops 子模块，Rust 允许同一 crate 多文件多 impl 块
//! - `StockInfo` 原 private struct 提升为 `pub(crate)`（在 ops::types 中），供 ops 子模块和测试模块共享；facade 不重导出，保持原 API 表面不变

use sea_orm::DatabaseConnection;
use std::sync::Arc;

// 批次 490 D10-3b 拆分：re-export 保持外部引用路径不变
// 注意：仅重导出原 pub struct，StockInfo 原 private 不重导出（保持 API 表面不变）
pub use crate::services::mrp_engine_ops::{
    MaterialRequirement, MrpCalculationItem, MrpCalculationQuery, MrpCalculationRequest,
    RequirementCalcParams,
};

/// MRP计算引擎（struct 定义保留在 facade，impl 块按职责分散到 `mrp_engine_ops/` 子模块。）
pub struct MrpEngineService {
    /// 数据库连接句柄（`pub(crate)` 可见性：mrp_engine_ops 兄弟模块的 impl 块需直接访问此字段。）
    pub db: Arc<DatabaseConnection>,
}

impl MrpEngineService {
    /// 创建 MRP 引擎服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}
