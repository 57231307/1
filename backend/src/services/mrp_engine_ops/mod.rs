//! MRP 引擎操作子模块
//!
//! 批次 490 D10-3b 拆分：从 mrp_engine_service.rs 抽取的实现方法按职责分组。
//! - types：数据结构（请求/响应/参数对象）
//! - stock：库存查询与物料需求计算
//! - bom：BOM 递归展开
//! - calculation：MRP 计算执行（单次/批量）
//! - query：结果查询与导出
//! - order：订单转换与产品列表

pub mod types;
pub mod stock;
pub mod bom;
pub mod calculation;
pub mod query;
pub mod order;

pub use types::*;
