//! 应收对账单主流程服务门面（ar/recon）
//!
//! 批次 D10：原 `ar/recon.rs`（1070 行）按 facade 模式拆分，业务方法实现
//! 迁移至 `ar/recon_ops/` 子模块（crud / lifecycle）。本文件保留测试模块与
//! 对账流程说明；共享 DTO 与 `ArReconciliationService` 定义位于 `ar/mod.rs`。
//!
//! 对账单主流程方法（实现见 `recon_ops`）：
//! - `create`             创建对账单
//! - `get_by_id`          按ID查询对账单
//! - `list`               分页查询对账单列表
//! - `update`             更新对账单金额/备注
//! - `get_with_details`   获取对账单及其明细
//! - `delete`             删除对账单（仅 draft）
//! - `send`               发送对账单（draft → sent）
//! - `close`              关闭对账单（confirmed/disputed → closed，含凭证生成）
//! - `update_status`      通用状态更新（含白名单校验）
//!
//! 协作子模块：
//! - `vfy` 自动对账算法、自动生成、客户确认/争议
//! - `inv` PDF 导出
//!
//! 拆分自原 `ar_reconciliation_service.rs`。
//! 结构体定义与构造函数 `ArReconciliationService::new` 位于 `super`（`ar/mod.rs`）。
