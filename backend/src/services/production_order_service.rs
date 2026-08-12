//! 生产订单服务（facade，批次 488 D10-2 拆分）
//!
//! 本文件为 facade 入口，仅保留 `ProductionOrderService` struct + `new` 构造函数 + 单元测试。
//! 业务实现已按职责拆分到 `production_order_ops/` 子模块（与 `production_order_service` 同为 `crate::services` 下兄弟模块）：
//! - `production_order_ops::crud`：CRUD 与状态校验（14 方法，原 L92-624）
//! - `production_order_ops::completion`：完成生产订单与库存联动（20 方法，原 L626-1243）
//! - `production_order_ops::approval`：审批管理（7 方法，原 L1250-1501）
//! - `production_order_ops::types`：请求/查询 DTO + 内部辅助 struct
//!
//! 设计要点（与拆分前一致）：
//! - 创建订单后触发 MRP 物料需求计算（失败 warn 不阻塞）
//! - 返工订单使用 RW- 前缀，不触发 MRP
//! - 状态转换校验基于状态机白名单（validate_status_transition）
//! - COMPLETED 状态走 complete_production_order 专用路径（事务包裹状态变更 + 库存联动）
//! - 排产状态变更走 check_capacity_for_scheduling 产能校验
//! - delete 软删除（状态改为 CANCELLED），走 update_with_audit 保留审计
//! - 审批流程对接 BPM（启动/任务审批保留事务外，失败 warn 不阻断）
//! - BPM 回写方法不回调 BPM 避免循环
//!
//! 拆分兼容性：
//! - 外部 handler 通过 `crate::services::production_order_service::ProductionOrderService::new` 调用，路径不变
//! - 外部 handler 通过 `crate::services::production_order_service::{CreateProductionOrderRequest, UpdateProductionOrderRequest, ProductionOrderQuery}` 引用，路径不变（此处 re-export）
//! - `db` 字段使用 `pub(crate)` 可见性，production_order_ops 子模块的 impl 块可直接访问
//! - impl 块分散在 production_order_ops 子模块，Rust 允许同一 crate 多文件多 impl 块

use sea_orm::DatabaseConnection;
use std::sync::Arc;

// 批次 488 D10-2 拆分：re-export 保持外部引用路径不变
pub use crate::services::production_order_ops::{
    CreateProductionOrderRequest, ProductionOrderQuery, UpdateProductionOrderRequest,
};

/// 生产订单 Service（struct 定义保留在 facade，impl 块按职责分散到 `production_order_ops/` 子模块。）
pub struct ProductionOrderService {
    /// 数据库连接句柄（`pub(crate)` 可见性：production_order_ops 兄弟模块的 impl 块需直接访问此字段。）
    pub(crate) db: Arc<DatabaseConnection>,
}

impl ProductionOrderService {
    /// 创建生产订单服务实例
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }
}
