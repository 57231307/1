//! 库存盘点服务（inv/count）
//!
//! V15 主线审计 P2 修复：删除“占位模块”陈旧描述。
//! 盘点主流程已通过 `InventoryCountService` 落地，路由 /inventory/counts 暴露
//! list/get/create/update/delete + record/submit/approve/reject。
//! 本子模块保留命名空间与扩展点（后续可承接按仓库/批次的盘点对账辅助函数）。
