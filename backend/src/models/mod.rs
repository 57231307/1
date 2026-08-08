#![allow(dead_code)]
pub mod customer;
pub mod customer_contact;
pub mod customer_followup;
// V15 P0-S08 修复：CRM 公海规则配置 + 客户转移审批
pub mod customer_pool_rule;
pub mod customer_transfer_approval;
// V15 P1 batch-15 18.4-D2/D3：CRM 团队协作 + 数据共享时效
// V15 P2 18.4-D5: 客户字段权限配置
pub mod customer_field_permission;
// V15 P2 18.4-D6: 客户操作日志
pub mod customer_audit_log;
// V15 P2 18.5-D5: 客户全生命周期价值（CLV）
pub mod customer_lifetime_value;
pub mod customer_team_member;
pub mod customer_share;
pub mod department;
pub mod dto;
// V15 P1 batch-19 缺陷 23.1.2：用户部门关联表（一人多部门，主部门+兼职）
pub mod user_department;
pub mod finance_invoice;
pub mod finance_payment;
pub mod inventory_adjustment;
pub mod inventory_adjustment_item;
pub mod inventory_count;
pub mod inventory_count_item;
pub mod inventory_reservation;
pub mod inventory_stock;
pub mod inventory_transaction;
pub mod inventory_transfer;
pub mod inventory_transfer_item;
pub mod operation_log;
// v11 批次 143 P1-2：用户行为追踪分析模块
pub mod page_view;
pub mod product;
pub mod product_category;
pub mod product_color;
pub mod role;
pub mod role_permission;
pub mod role_conflict;
pub mod role_change_approval;
pub mod role_relation;
// V15 P1 12.6：权限委托表
pub mod permission_delegation;
// V15 P0-S06：权限变更审计 model
pub mod permission_change_audit;
pub mod sales_order;
pub mod sales_order_change_history;
pub mod sales_order_item;
pub mod sales_quotation;
pub mod sales_quotation_item;
pub mod sales_quotation_term;
// 销售报价单 DTO（P12 批 1 P0 port PR-2：DTO + 基础 Service）
pub mod quotation_create_dto;
pub mod quotation_response_dto;
pub mod quotation_update_dto;
pub mod status;
pub mod user;
// v11 批次 143 P1-2：用户行为追踪分析模块
pub mod user_behavior;
// 批次 158 v11 真实接入：密码策略服务 - 密码历史持久化
pub mod password_history;
// MRP生产计划模块
pub mod bom;
pub mod bom_item;
pub mod mrp_result;
pub mod production_order;
pub mod scheduling_result;
pub mod work_center;
// P1 batch-18 缺陷 11.1：工作中心关联实体表（设备/人员/班次）
pub mod work_center_equipment;
pub mod work_center_shift;
pub mod work_center_worker;
// 应收对账与多币种模块
pub mod ar_reconciliation;
pub mod ar_reconciliation_item;
pub mod currency;
pub mod exchange_rate;
pub mod warehouse;
// 总账模块
pub mod account_balance;
pub mod account_subject;
pub mod accounting_period;
pub mod period_report_snapshot;
pub mod voucher;
pub mod voucher_item;
// 批次 365 v13 复审 B-P1-8：事件幂等去重表
pub mod processed_event;
// 批次 384 v13 复审 B-P1-7：事件死信队列
pub mod event_dead_letter;
// 辅助核算模块
pub mod assist_accounting_dimension;
pub mod assist_accounting_record;
pub mod assist_accounting_summary;
// 业务追溯模块
pub mod business_trace_chain;
pub mod business_trace_snapshot;
// 供应商管理模块
pub mod supplier;
pub mod supplier_category;
pub mod supplier_contact;
pub mod supplier_evaluation;
pub mod supplier_evaluation_record;
pub mod supplier_product;
pub mod supplier_qualification;
// 采购管理模块
pub mod purchase_order;
pub mod purchase_order_item;
pub mod purchase_receipt;
pub mod purchase_receipt_item;
// 应付管理模块
pub mod ap_invoice;
pub mod ap_payment;
pub mod ap_payment_request;
pub mod ap_payment_request_item;
pub mod ap_reconciliation;
pub mod ap_verification;
pub mod ap_verification_item;
// 应收账款模块
pub mod ar_collection;
pub mod ar_invoice;
pub mod aging_alert_rule;
// 成本管理模块
pub mod cost_analysis;
pub mod cost_collection;
// P1 模块
pub mod budget_management;
pub mod budget_plan;
pub mod budget_version;
pub mod customer_credit;
pub mod fixed_asset;
pub mod asset_category;
pub mod asset_impairment_test;
pub mod depreciation_policy_change;
pub mod fund_management;
pub mod purchase_contract;
pub mod quality_standard;
pub mod sales_contract;
pub mod sales_contract_item;
// P2 模块
pub mod financial_analysis;
pub mod financial_analysis_result;
pub mod purchase_price;
pub mod quality_inspection;
pub mod sales_analysis;
pub mod sales_price;
// P2 模块 - 质量检验记录
pub mod quality_inspection_record;
pub mod unqualified_product;
// 采购退货模块
pub mod purchase_return;
pub mod purchase_return_item;
// 销售报价单模块（Week 1）— 已在 L25-27 声明，此处仅声明新增的 DTO 模块
pub mod product_color_price;
pub mod quotation_convert_dto;
// 采购检验模块
pub mod purchase_inspection;
// 采购质检明细模块（批次 131 v9 复审 P0：替代 4 个明细 CRUD 端点占位）
pub mod purchase_inspection_item;
// 采购合同执行模块
pub mod purchase_contract_execution;
// 销售交货模块
pub mod sales_delivery;
pub mod sales_delivery_item;
// 固定资产处置模块
pub mod fixed_asset_disposal;
// 批次 88 PH-2：固定资产折旧期间记录模块
pub mod fixed_asset_depreciation_record;
// V15 P1 17.8-D4：固定资产盘点单/明细
pub mod fixed_asset_count;
pub mod fixed_asset_count_item;
// 资金转账记录模块
pub mod fund_transfer_record;
// 面料行业核心模块
pub mod dye_batch;
pub mod dye_lot_mapping;
pub mod dye_recipe;
pub mod greige_fabric;
// v14 批次 423B：化验室打样流程贯通（打样通知单 + ABCD 多版样 + 复样记录）
pub mod lab_dip_request;
pub mod lab_dip_resample;
pub mod lab_dip_sample;
// v14 批次 424：大货处方与加料处方流程（染色配料单 + 染色补料单）
pub mod production_recipe;
pub mod production_recipe_addition;
// v14 批次 425：流转卡条码与车间工序流转（流转卡 + 工序路线 + 工序记录 + 质量反馈单）
pub mod process_route;
pub mod process_step_record;
pub mod process_quality_feedback;
pub mod production_flow_card;
// v14 批次 426：验布打卷流程贯通（验布记录 + 疵点明细）
pub mod fabric_inspection_record;
pub mod fabric_defect_record;
// v14 批次 427：产量工资核算贯通（工序工价 + 工资记录 + 工资明细）
pub mod process_wage_rate;
pub mod wage_record;
pub mod wage_record_detail;
// v14 批次 428：能耗管理贯通（能源计量设备 + 能耗记录 + 能耗分摊规则 + 能耗分摊记录）
pub mod energy_meter;
pub mod energy_consumption_record;
pub mod energy_allocation_rule;
pub mod energy_allocation_record;
// v14 批次 429：染化料主数据完善（染化料主数据 + 分类 + 批次 + 领用单）
pub mod chemical_master;
pub mod chemical_category;
pub mod chemical_lot;
pub mod chemical_requisition;
// v14 批次 430：委托加工物资贯通（委外订单 + 发料明细 + 收回入库 + 会计凭证）
pub mod outsourcing_order;
pub mod outsourcing_order_item;
pub mod outsourcing_receipt;
pub mod outsourcing_voucher;
// 匹数管理模块
pub mod inventory_piece;
// 仓位/库位模块
pub mod location;
// BPM流程模块
pub mod bpm_process_definition;
pub mod bpm_process_instance;
pub mod bpm_task;
// 预算执行模块
pub mod budget_adjustment;
pub mod budget_execution;
// 业务追溯模块
pub mod business_trace;
// 审计模块
pub mod audit_log;
// V15 缺陷 10-4：审计日志导出二次审计表（防篡改，独立于 audit_logs）
pub mod audit_log_export_log;
// V15 P0-S14 敏感数据导出二级审批请求
pub mod export_approval_request;
// CRM 公海回收规则模块（批次 23 v5 P0-4：内存存储迁移至数据库）
pub mod crm_recycle_rule;
pub mod crm_lead;
pub mod crm_opportunity;
// V15 P2 18.2-D5: 商机阶段变更历史
pub mod opportunity_stage_history;
// V15 P2 18.2-D6: 竞争对手
pub mod competitor;
pub mod opportunity_competitor;
// V15 P2 18.2-D7: 商机跟进记录
pub mod opportunity_follow_up;
// 批次 122 v8 复审 P1 修复：CRM 标签字典表（替代 list_tags 硬编码 + create_tag/delete_tag 假实现）
pub mod crm_tag;
// V15 P2 18.1-D4: 线索来源 ROI 跟踪
pub mod lead_source_roi;
// V15 P2 18.1-D5: 线索分配规则
pub mod lead_allocation_rule;
// V15 P2 18.1-D6: 线索培育计划
pub mod lead_nurture_plan;
pub mod logistics_waybill;
// V15 P1 batch-19 缺陷 23.4.2：物流跟踪事件历史表
pub mod logistics_tracking_event;
pub mod omni_audit_log;
// V15 P2 B11-P2-9：安全告警日志模型（《网络安全法》《数据安全法》合规，保留 7 年）
pub mod security_alert_log;
pub mod sales_return;
pub mod sales_return_item;
pub mod slow_query;
// 扩展能力模块
pub mod api_endpoint;
pub mod api_key;
pub mod webhook;
// 消息通知模块
pub mod notification;
pub mod notification_setting;
pub mod notification_subscription;
pub mod notification_template;
pub mod user_notification_setting;
// 批次 127 v8 复审 P2 修复：导入任务记录表（替代 list_import_tasks 空列表占位）
pub mod import_task;
// 数据权限模块
pub mod data_permission;
// 字段权限模块
pub mod field_permission;
// 报表模板模块
pub mod report_subscription;
pub mod report_template;
// 缺陷 1.1 修复：报表模板历史版本（支持回滚）
pub mod report_template_version;
// 邮件模块
pub mod email_log;
pub mod email_template;
// CRM分配历史模块
pub mod assignment_history;
// 登录日志模块
pub mod log_login;
// 供应商产品颜色模块
pub mod supplier_product_color;
// 产品供应商映射模块
pub mod product_supplier_mapping;
// 产品编码映射模块
pub mod product_code_mapping;
// 资金账户模块
pub mod fund_account;
// API访问日志模块
pub mod log_api_access;
// 系统日志模块
pub mod log_system;
// 系统版本模块
pub mod system_version;
// 公告模块
pub mod oa_announcement;
// 缺陷 4.1 修复：仪表板布局配置（用户自定义卡片）
pub mod dashboard_layout;
// 颜色编码映射模块
pub mod color_code_mapping;
// 批次追溯日志模块
pub mod batch_trace_log;
// 批次染缸映射模块
pub mod batch_dye_lot;
// V15 P1-3: 面料物理指标检测记录（十项指标）
pub mod fabric_physical_test_record;
// 缺陷 3.3 修复：piece_mapping 模型已删除（改用 inventory_piece.parent_piece_id 作为唯一映射机制）
// 成本模块
// P1P2模块
// 应收账龄分析模块
pub mod ar_aging_analysis;
// 供应商基础模块
pub mod supplier_blacklist;
pub mod supplier_grade;
// 业务追溯辅助链接模块
pub mod business_trace_assist_link;
pub mod business_trace_view;
// 字段权限模块
pub mod report_definition;
// 增强审计日志已废弃（无对应 migration + 0 业务引用）→ 整个 _legacy/ 目录已清理
// P0-2 主备隔离模块
pub mod failover_config;
pub mod failover_event;
pub mod failover_status;
// V15 P0-B15（Batch 484）：缺料预警持久化（material_shortage_alerts + threshold_configs）
pub mod material_shortage;
// P0-3 定制订单全流程跟踪模块
pub mod custom_order;
pub mod process_node;
pub mod process_log;
pub mod quality_issue;
pub mod after_sales;
// P0-3 定制订单 DTO
pub mod custom_order_create_dto;
pub mod custom_order_update_dto;
pub mod custom_order_response_dto;
pub mod process_node_dto;
pub mod quality_issue_dto;
// V15 P0-F20 Batch 480：8D 质量管理流程 DTO + Model
pub mod quality_8d_dto;
pub mod quality_8d_report;
// P0-4 色卡仓储管理模型
pub mod color_card;
pub mod color_card_item;
// V15 P0-F04：新增 color_card_issue（替代 color_card_borrow_record）
pub mod color_card_issue;
pub mod color_card_create_dto;
pub mod color_card_item_dto;
// V15 P0-F03：删除 color_card_borrow_dto（borrow 模式已废弃）
pub mod color_card_response_dto;
// P0-5 面料多色号定价扩展模型
pub mod color_price_history;
pub mod color_price_tier;
pub mod customer_color_price;
pub mod seasonal_price_rule;
// P0-5 面料多色号定价扩展 DTO
pub mod color_price_dto;
pub mod color_price_history_dto;
pub mod color_price_tier_dto;
pub mod customer_color_price_dto;
pub mod seasonal_price_rule_dto;
// P2-4 AI 分析深化（工艺优化 + 质量预测）模型
pub mod ai_process_optimization;
pub mod ai_quality_prediction;
// V15 P1 批次14：AI 模型版本管理 + 评估 + 决策日志 + 准确率报告
pub mod ai_model_version;
pub mod ai_model_evaluation;
pub mod ai_decision_log;
pub mod ai_quality_accuracy_report;
// v14 批次 431：多业务模式支持（业务模式配置 + 流程步骤 + 规则 + 单据关联）
pub mod business_mode_config;
pub mod business_mode_flow_step;
pub mod business_mode_rule;
pub mod business_mode_order_link;
// v14 批次 432：缸号全生命周期状态机（生命周期日志 + 状态规则 + 回修记录 + 操作记录）
pub mod dye_batch_lifecycle_log;
pub mod dye_batch_state_rule;
pub mod dye_batch_rework;
pub mod dye_batch_operation;
// V15 P0-F15：大货批色审批表（8 状态机：pending→sampled→sent_to_customer→approved/rejected/rework→downgraded/scrapped）
pub mod bulk_color_approval;
// V15 P1-10：大货批色状态变更历史表（每次状态变更全量快照，支持客户投诉追溯/责任界定/合规审计）
pub mod bulk_color_approval_history;
// V15 P0-B01/B02 Batch 481：坏账准备 + 坏账核销 Model + DTO
pub mod bad_debt_provision;
pub mod bad_debt_writeoff;
pub mod bad_debt_dto;
// V15 P0-B03 Batch 481：催收任务 Model + DTO
pub mod collection_task;
pub mod collection_task_dto;
// V15 P1 17.3-D5：催收模板 Model（话术标准化）
pub mod collection_template;
// V15 P0-B04 Batch 481：财务预警 Model + DTO
pub mod finance_alert;
pub mod finance_alert_dto;
// V15 P1 batch-16 缺陷 7.3：用户隐私同意记录 Model（user_consents 表）
pub mod user_consent;
// V15 P1 batch-08 法律合规修复（环保/劳动/财税法律合规）：
// 缺陷 14：出口退税相关模型
pub mod export_customs_declaration;
pub mod foreign_exchange_verification;
pub mod export_refund_declaration;
// 缺陷 15：污染物排放记录（环保税核算基础）
pub mod pollutant_discharge_record;
// 缺陷 18：排污许可证
pub mod pollution_permit;
// 缺陷 19：污染物监测记录 + 固废处置联单
pub mod pollutant_monitoring_record;
pub mod solid_waste_disposal_record;
// 缺陷 21：劳动合同
pub mod labor_contract;
// 缺陷 23：社保公积金缴纳记录
pub mod social_insurance_record;
// 缺陷 24：职业健康（危害监测 + 体检档案 + PPE 发放）
pub mod occupational_hazard_monitoring;
pub mod occupational_health_exam;
pub mod ppe_distribution_record;
// V15 P2 B05-P2-6：染缸设备占用/释放记录 Model（dye_vat_occupation 表）
pub mod dye_vat_occupation;
// V15 P2 B05-P2-7：PDA/工控终端连接资源管理 Model（device_connection 表）
pub mod device_connection;
// V15 P2 B05-P2-7：设备连接管理 DTO
pub mod device_connection_dto;
// V15 P2 B05-P2-10：期末调整记录 Model（period_adjustment_record 表，暂估/摊销/预提）
pub mod period_adjustment_record;
// V15 P2 B08-P2-5：出口商检 + 产地证
pub mod certificate_of_origin;
pub mod export_inspection;
// V15 P2 B08-P2-6：存货跌价准备
pub mod inventory_write_down;
// V15 P2 B08-P2-8：环评合规存档
pub mod environmental_assessment;
// V15 P2 B08-P2-9：女职工与未成年工保护
pub mod female_worker_protection;
pub mod operation_certificate;
pub mod safety_accident_report;
