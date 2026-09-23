//! 高级 handler 子模块
//!
//! 将原 advanced_handler.rs 按业务领域拆分为以下子模块：
//! - forecast     销售预测 / 库存优化
//! - analytics    报表分析
//! - rec          智能推荐
//! - decide       异常检测
//! - recipe_opt   染色工艺参数智能推荐（A2-1）
//! - quality_pred 质量预测（A2-2）

pub mod analytics;
pub mod decide;
pub mod forecast;
pub mod quality_pred;
pub mod rec;
pub mod recipe_opt;

// 重新导出所有 pub 项，保持 `crate::handlers::advanced::xxx` 的访问路径可用
pub use analytics::*;
pub use decide::*;
pub use forecast::*;
pub use quality_pred::*;
pub use rec::*;
pub use recipe_opt::*;
