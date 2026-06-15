//! 高级 handler 子模块
//!
//! 将原 advanced_handler.rs 按业务领域拆分为以下子模块：
//! - forecast  销售预测 / 库存优化
//! - analytics 报表分析
//! - rec       智能推荐
//! - reorder   采购合同 / 采购价格 / 销售退货
//! - decide    异常检测 / 销售合同 / 销售价格 / 租户管理

pub mod analytics;
pub mod decide;
pub mod forecast;
pub mod rec;
pub mod recipe_opt;
pub mod reorder;

// 重新导出所有 pub 项，保持 `crate::handlers::advanced::xxx` 的访问路径可用
pub use analytics::*;
pub use decide::*;
pub use forecast::*;
pub use rec::*;
pub use recipe_opt::*;
pub use reorder::*;
