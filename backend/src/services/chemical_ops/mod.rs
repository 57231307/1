//! 染化料主数据 ops 子模块入口（chemical_ops）
//!
//! 批次 490 D10-3a 拆分：从原 `chemical_service.rs` 拆出 4 个 Service impl 块。
//! - `master`：ChemicalMasterService impl（主数据 CRUD + 状态流转 + 安全库存检查）
//! - `category`：ChemicalCategoryService impl（分类 CRUD + 树形结构）
//! - `lot`：ChemicalLotService impl（批次 CRUD + 效期管理 + 来料检验状态流转）
//! - `requisition`：ChemicalRequisitionService impl（领用单 CRUD + 状态机 + 取消）
//! - `types`：12 个 DTO struct
//!
//! 4 个 Service struct 定义与 `new` 构造函数保留在 facade `chemical_service` 中，
//! impl 块分散到本子模块，Rust 允许同一 crate 多文件多 impl 块。

pub mod category;
pub mod lot;
pub mod master;
pub mod requisition;
pub mod types;

// re-export DTOs，facade 通过 `pub use` 二次 re-export 保持外部引用路径不变
pub use types::{
    ChemicalCategoryQuery, ChemicalLotQuery, ChemicalMasterQuery, ChemicalRequisitionQuery,
    CreateChemicalCategoryRequest, CreateChemicalLotRequest, CreateChemicalMasterRequest,
    CreateChemicalRequisitionRequest, UpdateChemicalCategoryRequest, UpdateChemicalLotRequest,
    UpdateChemicalMasterRequest, UpdateChemicalRequisitionRequest,
};
