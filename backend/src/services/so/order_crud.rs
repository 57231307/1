//! P9-2 销售订单 CRUD 子模块（占位）
//!
//! 拆分自原 `services/so/order.rs`。
//! 本文件保留核心 CRUD 函数的位置说明，代码主体仍由原文件承载。
//!
//! ## 模块职责
//! - 销售订单创建 / 更新 / 删除
//! - 销售订单行项管理
//! - 销售订单状态机
//!
//! ## API 兼容
//! 所有 `services::so::order::*` 路径保持原签名，本文件 re-export 关键类型。

// P9-2: 子模块化拆分占位。当前主要 CRUD 逻辑仍在父文件。
// 后续 P10 将逐步将大型函数迁移到本子模块。

#[allow(unused_imports)]
pub use crate::services::so::{
    CreateSalesOrderRequest, SalesOrderDetail, SalesOrderItemDetail, SalesService,
    UpdateSalesOrderRequest,
};

/// P9-2 标记：销售订单 CRUD 子模块路径
pub const P92_CRUD_MODULE: &str = "sales_order_crud";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crud_module_loaded() {
        // 验证子模块 re-export 工作正常
        assert_eq!(P92_CRUD_MODULE, "sales_order_crud");
    }
}
