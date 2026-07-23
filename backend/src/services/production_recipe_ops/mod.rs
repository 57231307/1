//! 大货处方与加料处方业务实现子模块（production_recipe_ops）
//!
//! D10-6a 拆分：从原 production_recipe_service.rs 迁移 2 个 Service 的 impl 块。
//! struct 定义 + new 构造函数 + 7 个 DTOs + 纯函数（单号生成/浴比解析/用量计算/
//! 状态校验）+ 单元测试保留在 facade production_recipe_service.rs。
//!
//! 模块层级关系：
//! - production_recipe_ops 与 production_recipe_service 同为 crate::services 下的兄弟模块
//! - production_recipe_service.rs 作为 facade，保留 2 个 Service struct + new 方法 +
//!  DTOs + 纯函数 + 测试
//! - 子模块 impl facade 定义的 struct（依赖 db 字段为 pub(crate)）
//! - 子模块通过 use crate::services::production_recipe_service::{...} 复用 facade 的
//!  DTOs 和纯函数
//!
//! 子模块划分：
//! - `recipe_crud`：大货处方 CRUD + 查询（create / update / delete / get_by_id /
//!  list / get_by_work_order / list_additions_by_recipe 及 update 辅助方法）
//! - `recipe_state`：大货处方状态流转（approve / close / cancel）
//! - `addition`：加料处方全部 impl 方法（create / get_by_id / list_by_recipe /
//!  list / approve / close）

pub mod addition;
pub mod recipe_crud;
pub mod recipe_state;
