pub mod assist_accounting_service;
pub mod auth_service;
// 批次 D10 拆分：auth_service.rs facade 的业务实现子模块（auth/jti）
pub mod auth_service_ops;
// P4-2 安全加固 - 密码策略服务（密码历史/锁定/过期）
pub mod auth;
pub mod batch_service;
// P4-1 性能优化 - 进程内 LRU 缓存
pub mod cache_service;
// 批次 106 修复：performance_optimizer 模块已删除（死代码样板，能力已被 utils/n_plus_one + cache_service + slow_query 中间件覆盖）
// P3-4 数据仓库/BI 关键路径 demo
pub mod bi_analysis_service;
// 批次 490 D10-3a 拆分：bi_analysis_service.rs facade 的业务实现子模块（sales/profit/drilldown/olap/types）
pub mod bi_analysis_ops;
// P4-3 监控告警 - 业务指标扩展（20+ 指标）
pub mod business_metrics;
// P4-5 单元测试覆盖 - 5 个 service 单元测试模块
#[cfg(test)]
pub mod sales_unit_tests;
#[cfg(test)]
pub mod purchase_unit_tests;
#[cfg(test)]
pub mod inventory_unit_tests;
#[cfg(test)]
pub mod ar_unit_tests;
#[cfg(test)]
pub mod bi_unit_tests;
// P0-D11：测试公共夹具模块（抽取自 18 处 src/services/ 重复定义的 setup_test_db）
#[cfg(test)]
pub mod test_common;
pub mod business_trace_service;
pub mod crm;
pub mod customer_service;
pub mod dashboard_service;
pub mod department_service;
pub mod finance_invoice_service;
pub mod finance_payment_service;
pub mod finance_report_service;
pub mod five_dimension_service;
pub mod init_service;
// 批次 491 D10-5 拆分：init_service.rs facade 的业务实现子模块（setup/role/permission/dept_user）
pub mod init_service_ops;
pub mod inv;
pub mod inventory_adjustment_service;
// v11 批次 143 P1-1：inventory_count_service 真实实现（盘点单 CRUD + 差异计算 + 审批流）
pub mod inventory_count_service;
pub mod inventory_finance_bridge_service;
// 拆分：inventory_finance_bridge_service.rs facade 的业务实现子模块（listener/voucher）
pub mod inventory_finance_bridge_ops;
// v14 批次 422 T-P1-7：染色完成→成本归集桥接监听器
pub mod dye_batch_cost_bridge_service;
// v14 批次 423A：染色配方 Service 抽象层（CRUD + 状态流转 + 版本管理）
pub mod dye_recipe_service;
// v14 批次 423B：化验室打样流程贯通（打样通知单 + ABCD 多版样 + OK 样确认 + 复样记录）
pub mod lab_dip_service;
// v14 批次 424：大货处方与加料处方流程（染色配料单 + 染色补料单）
pub mod production_recipe_service;
// v14 批次 425：流转卡条码与车间工序流转
pub mod flow_card_service;
// 批次 491 D10-5 拆分：flow_card_service.rs facade 的业务实现子模块（route/card_crud/card_state/step/feedback）
pub mod flow_card_ops;
// v14 批次 426：验布打卷流程贯通
pub mod fabric_inspection_service;
// v14 批次 427：产量工资核算贯通
pub mod wage_service;
// 批次 490 D10-4a 拆分：wage_service.rs facade 的业务实现子模块（rate/record/calculation）
pub mod wage_ops;
// v14 批次 428：能耗管理贯通
pub mod energy_service;
// 批次 488 D10-2a 拆分：energy_service.rs facade 的业务实现子模块（meter/consumption/allocation_rule/allocation_record）
pub mod energy_ops;
// v14 批次 429：染化料主数据完善（染化料主数据 + 分类 + 批次 + 领用单）
pub mod chemical_service;
// 批次 490 D10-3a 拆分：chemical_service.rs facade 的业务实现子模块（master/category/lot/requisition/types）
pub mod chemical_ops;
// v14 批次 430：委托加工物资贯通（委外订单 + 发料明细 + 收回入库 + 会计凭证）
pub mod outsourcing_service;
// 批次 489 D10-2b 拆分：委外加工 ops 子模块（order/order_item/receipt/voucher/types）
pub mod outsourcing_ops;
pub mod inventory_reservation_service;
pub mod inventory_stock_query;
pub mod inventory_stock_service;
pub mod inventory_stock_txn;
// P9-2 拆分：库存子模块
pub mod stock_alert;
pub mod product_category_service;
pub mod product_service;
pub mod role_permission_service;
pub mod so;
pub mod user_service;
pub mod warehouse_service;
// 供应商管理模块
pub mod supplier_evaluation_service;
pub mod supplier_service;
// 采购管理模块
pub mod po;
pub mod purchase_inspection_service;
pub mod purchase_receipt_dto;
pub mod purchase_receipt_private;
pub mod purchase_receipt_service;
pub mod purchase_return_service;
// 应付管理模块
pub mod ap_invoice_service;
// 批次 490 D10-4b 拆分：ap_invoice_service.rs facade 的业务实现子模块（receipt/crud/report/types）
pub mod ap_invoice_ops;
pub mod ap_payment_request_service;
pub mod ap_payment_service;
pub mod ap_reconciliation_service;
// D10-5 拆分：ap_reconciliation_service.rs facade 的业务实现子模块（crud/confirm/report/auto/types）
pub mod ap_reconciliation_ops;
pub mod ap_report_service;
pub mod ap_verification_service;
// 应收管理模块
// 批次 348 v12 复审 P2-1：ar_collection_service 模块已删除（死代码，功能被 ar_service 完全覆盖）
pub mod ar_invoice_service;
pub mod ar_service;
// 批次 488 D10-1 拆分：ar_service.rs facade 的业务实现子模块（collection/verification/report/types/json_helpers）
pub mod ar_ops;
// 总账管理模块
pub mod account_subject_service;
pub mod accounting_period_service;
pub mod voucher_service;
// 批次 488 D10-2a 拆分：voucher_service.rs facade 的业务实现子模块（crud/workflow/balance/assist）
pub mod voucher_ops;
// 成本管理模块
pub mod audit_log_service;
pub mod bpm_service;
pub mod bpm_service_dto;
// 批次 95 P3-15：bpm_service_stub.rs 重命名为 bpm_process_definition_service.rs（消除 stub 误导）
pub mod bpm_process_definition_service;
pub mod budget_management_service;
pub mod cost_collection_service;
pub mod customer_credit_evaluate;
pub mod customer_credit_limit;
pub mod customer_credit_service;
pub mod event_bus;
// 批次 491 D10-6 拆分：event_bus.rs facade 的业务实现子模块（kafka/listener）
pub mod event_bus_ops;
pub mod event_kafka;
pub mod event_kafka_payload;
// 批次 365 v13 复审 B-P1-8：事件幂等服务
pub mod event_idempotency_service;
// 批次 384 v13 复审 B-P1-7：事件重试与死信队列服务
pub mod event_retry_service;
pub mod financial_analysis_service;
pub mod fixed_asset_service;
pub mod fund_management_service;
pub mod metrics_service;
pub mod omni_audit_query_service;
pub mod omni_audit_service;
// 批次 106 修复：operation_log_service 模块已删除（零业务引用，已被 omni_audit_service 完全替代）
pub mod order_change_history_service;
pub mod purchase_contract_service;
pub mod purchase_delivery_calculator;
pub mod purchase_price_service;
pub mod quality_inspection_service;
pub mod quality_standard_service;
pub mod sales_analysis_service;
pub mod sales_contract_service;
pub mod sales_price_service;
pub mod sales_return_service;
// 销售报价单 Service（P12 批 1 P0 port PR-2：DTO + 基础 Service）
pub mod quotation_pricing_service;
pub mod quotation_service;
// 销售报价单转销售订单 Service（P12 批 1 P0 port PR-4：审批流 + 报价转订单 + 集成测试）
pub mod quotation_convert_service;
pub mod system_update_service;
pub mod totp_service;
// v11 批次 143 P1-2：用户行为追踪分析服务
pub mod tracking_service;
// MRP生产计划模块
pub mod bom_service;
pub mod mrp_engine_service;
// 批次 490 D10-3b 拆分：mrp_engine_service 业务实现子模块
pub mod mrp_engine_ops;
pub mod production_order_service;
// 批次 488 D10-2 拆分：production_order_service 业务实现子模块
pub mod production_order_ops;
// 应收对账与多币种模块
pub mod ar;
pub mod currency_service;
// AI智能分析与报表模块
pub mod ai;
pub mod report;
// P2-4 AI 分析深化（工艺优化 + 质量预测）持久化
pub mod ai_extend_service;
// V15 P0-S14 敏感数据导出二级审批
pub mod export_approval_service;
// 扩展能力模块
pub mod api_key_service;
pub mod webhook_service;
// 消息通知模块
pub mod data_permission_service;
pub mod email_service;
pub mod event_notification_service;
pub mod notification_service;
pub mod user_notification_setting_service;
// 产能分析模块
pub mod capacity_service;
// 缺料预警模块
pub mod material_shortage_service;
// 生产排程模块
pub mod scheduling_service;
// P9-2 拆分：排程子模块
pub mod scheduling_auto;
pub mod scheduling_manual;
pub mod scheduling_query;
// 字段权限模块
pub mod field_permission_service;
// 导入导出模块
pub mod import_export_service;
// 报表模板模块
pub mod report_template_service;
// 邮件模板模块
pub mod email_template_service;
// 邮件发送记录模块
pub mod email_log_service;
// 分配历史模块
pub mod assignment_history_service;
// 报表订阅模块
pub mod report_subscription_service;
// P0-D16（Batch 488）：报表订阅调度任务（后台扫描 next_run_at 到期的订阅）
pub mod report_subscription_scheduler;
// 导出服务模块
pub mod export_service;
// 通用打印服务
pub mod print_service;
// 审计日志清理服务
pub mod audit_cleanup_service;
// 敏感操作告警服务
pub mod sensitive_action_alert;
// 增强日志服务
pub mod enhanced_logger;
pub mod slow_query_collector;
// 销售报价单服务（Week 1）— 已在 L113-114 声明，此处仅声明新增的
// 销售报价单审批服务（Week 2 Task 7）
pub mod quotation_approval_service;
// P0-2 主备隔离服务
pub mod failover_service;
// P0-3 定制订单全流程跟踪服务
pub mod custom_order_crud_service;
pub mod custom_order_state_service;
pub mod custom_order_process_service;
pub mod custom_order_quality_service;
pub mod custom_order_aftersales_service;
// P0-4 色卡仓储管理服务
pub mod color_card_crud_service;
pub mod color_card_item_service;
// V15 P0-F04：新增 color_card_issue_service（替代 color_card_borrow_service）
pub mod color_card_issue_service;
pub mod color_card_scan_service;
// P0-5 面料多色号定价扩展服务
pub mod color_price_crud_service;
pub mod color_price_batch_service;
pub mod color_price_history_service;
pub mod color_price_seasonal_service;
pub mod color_price_tier_service;
// v14 批次 431：多业务模式支持服务（业务模式配置 + 流程步骤 + 规则 + 单据关联）
pub mod business_mode_service;
// 批次 489 D10-2b 拆分：多业务模式 ops 子模块（config/flow_step/rule/order_link/types）
pub mod business_mode_ops;
// v14 批次 432：缸号全生命周期状态机服务（生命周期日志 + 状态规则 + 回修记录 + 操作记录 + 纯函数校验）
pub mod dye_batch_state_machine_service;
// 批次 490 D10-4a 拆分：dye_batch_state_machine_service.rs facade 的业务实现子模块（lifecycle_log/state_rule/rework/operation）
pub mod dye_batch_state_machine_ops;
// V15 P0-F15：大货批色审批服务（8 状态机：pending→sampled→sent_to_customer→approved/rejected/rework→downgraded/scrapped）
pub mod bulk_color_approval_service;
// V15 P0-F20 Batch 480：8D 质量管理流程服务（11 状态机：not_started→d0_plan→d1_team→...→d8_recognize→closed）
pub mod quality_8d_service;
// V15 P0-B01/B02/B03/B04 Batch 481：坏账管理 + 催收任务 + 财务预警服务
pub mod bad_debt_service;
pub mod collection_task_service;
pub mod finance_alert_service;
// P0-D17（Batch 488）：OA 公告 service（CRUD + 发布/归档状态转换）
pub mod oa_announcement_service;
