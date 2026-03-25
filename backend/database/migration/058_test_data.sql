-- ========================================
-- 秉羲 ERP 系统 - 功能模块整合测试数据
-- 版本：2026-03-16
-- 模块：全模块测试数据
-- 说明：为所有新增模块提供基础测试数据
-- ========================================

-- ========================================
-- 1. 四级批次管理测试数据
-- ========================================

-- 供应商成品编码映射
INSERT INTO product_code_mapping (internal_product_code, supplier_product_code, supplier_id, mapping_date, validation_status) VALUES
('PROD001', 'SPROD001', 1, CURRENT_DATE, 'validated'),
('PROD002', 'SPROD002', 1, CURRENT_DATE, 'validated'),
('PROD001', 'SPROD001-A', 2, CURRENT_DATE, 'validated');

-- 供应商色号编码映射
INSERT INTO color_code_mapping (internal_color_no, supplier_color_code, supplier_id, mapping_date, validation_status) VALUES
('COLOR001', 'SCOLOR001', 1, CURRENT_DATE, 'validated'),
('COLOR002', 'SCOLOR002', 1, CURRENT_DATE, 'validated'),
('COLOR001', 'SCOLOR-A01', 2, CURRENT_DATE, 'validated');

-- 缸号管理
INSERT INTO batch_dye_lot (dye_lot_no, product_id, color_id, supplier_dye_lot_no, supplier_id, production_date, quality_grade, quality_status) VALUES
('DL20260316001', 1, 1, 'SDL001', 1, CURRENT_DATE, 'A', 'passed'),
('DL20260316002', 1, 1, 'SDL002', 1, CURRENT_DATE, 'A', 'passed'),
('DL20260316003', 1, 2, 'SDL003', 1, CURRENT_DATE, 'B', 'passed'),
('DL20260316004', 2, 1, 'SDL004', 2, CURRENT_DATE, 'A', 'passed');

-- 匹号管理
INSERT INTO inventory_piece (piece_no, dye_lot_id, supplier_piece_no, length, weight, quality_status, inventory_status) VALUES
('P20260316000001', 1, 'SP001', 100.00, 20.50, 'passed', 'available'),
('P20260316000002', 1, 'SP002', 105.00, 21.00, 'passed', 'available'),
('P20260316000003', 1, 'SP003', 98.00, 19.80, 'passed', 'available'),
('P20260316000004', 2, 'SP004', 102.00, 20.80, 'passed', 'available'),
('P20260316000005', 2, 'SP005', 103.00, 21.20, 'passed', 'available');

-- ========================================
-- 2. BPM 流程引擎测试数据
-- ========================================

-- 流程定义 - 采购审批流程
INSERT INTO bpm_process_definition (process_key, process_name, process_version, process_category, description, flow_definition, status, is_published, published_at) VALUES
('procurement_approval', '采购审批流程', 'v1.0.0', 'procurement', 
 '用于采购订单的审批流程',
 '{
   "nodes": [
     {"id": "start", "type": "start", "name": "开始"},
     {"id": "dept_manager", "type": "user_task", "name": "部门经理审批", "assignee_type": "role", "assignee_value": "dept_manager"},
     {"id": "finance_manager", "type": "user_task", "name": "财务经理审批", "assignee_type": "role", "assignee_value": "finance_manager"},
     {"id": "general_manager", "type": "user_task", "name": "总经理审批", "assignee_type": "role", "assignee_value": "general_manager", "condition": "amount > 100000"},
     {"id": "end", "type": "end", "name": "结束"}
   ],
   "transitions": [
     {"from": "start", "to": "dept_manager"},
     {"from": "dept_manager", "to": "finance_manager", "condition": "approved"},
     {"from": "finance_manager", "to": "end", "condition": "approved AND amount <= 100000"},
     {"from": "finance_manager", "to": "general_manager", "condition": "approved AND amount > 100000"},
     {"from": "general_manager", "to": "end", "condition": "approved"}
   ]
 }'::JSONB,
 'active', TRUE, CURRENT_TIMESTAMP);

-- 流程定义 - 销售审批流程
INSERT INTO bpm_process_definition (process_key, process_name, process_version, process_category, description, flow_definition, status, is_published, published_at) VALUES
('sales_approval', '销售审批流程', 'v1.0.0', 'sales',
 '用于销售订单的审批流程',
 '{
   "nodes": [
     {"id": "start", "type": "start", "name": "开始"},
     {"id": "sales_manager", "type": "user_task", "name": "销售经理审批", "assignee_type": "role", "assignee_value": "sales_manager"},
     {"id": "end", "type": "end", "name": "结束"}
   ],
   "transitions": [
     {"from": "start", "to": "sales_manager"},
     {"from": "sales_manager", "to": "end", "condition": "approved"}
   ]
 }'::JSONB,
 'active', TRUE, CURRENT_TIMESTAMP);

-- ========================================
-- 3. CRM 扩展测试数据
-- ========================================

-- 销售线索
INSERT INTO crm_lead (lead_no, lead_source, lead_status, company_name, contact_name, mobile_phone, estimated_amount, owner_id, owner_name, priority) VALUES
('LEAD20260316001', 'exhibition', 'qualified', '江苏纺织有限公司', '张经理', '13800138001', 500000.00, 1, '张三', 'high'),
('LEAD20260316002', 'referral', 'new', '浙江印染厂', '李总', '13800138002', 300000.00, 1, '张三', 'medium'),
('LEAD20260316003', 'website', 'contacted', '广东服装厂', '王经理', '13800138003', 200000.00, 2, '李四', 'low');

-- 商机
INSERT INTO crm_opportunity (opportunity_no, opportunity_name, customer_id, opportunity_stage, win_probability, estimated_amount, owner_id, owner_name, opportunity_status) VALUES
('OPP20260316001', '江苏纺织年度采购', 1, 'negotiation', 70.00, 500000.00, 1, '张三', 'open'),
('OPP20260316002', '浙江印染厂批量订单', 2, 'proposal', 50.00, 300000.00, 1, '张三', 'open'),
('OPP20260316003', '广东服装厂试单', 3, 'closing', 90.00, 200000.00, 2, '李四', 'open');

-- 客户跟进记录
INSERT INTO crm_follow_up (follow_up_no, lead_id, opportunity_id, follow_up_type, follow_up_date, subject, content, owner_id, owner_name) VALUES
('FU20260316001', 1, NULL, 'phone_call', CURRENT_DATE, '初次联系', '与客户沟通了产品需求，客户对我们的产品质量很感兴趣', 1, '张三'),
('FU20260316002', NULL, 1, 'meeting', CURRENT_DATE, '商务谈判', '与客户进行了面对面的商务谈判，讨论了价格和交货期', 1, '张三');

-- ========================================
-- 4. OA 协同办公测试数据
-- ========================================

-- 通知公告
INSERT INTO oa_announcement (announcement_no, title, content, announcement_type, priority, publisher_id, publisher_name, publish_date, status, is_top) VALUES
('ANN20260316001', '关于 2026 年春节放假的通知', '根据公司安排，2026 年春节放假时间为 2 月 15 日至 2 月 22 日，共 8 天。请各部门做好工作安排。', 'company_notice', 'high', 1, '行政部', CURRENT_DATE, 'published', TRUE),
('ANN20260316002', '系统升级维护通知', '公司 ERP 系统将于本周末进行升级维护，届时系统将暂停使用。请提前做好工作安排。', 'system_notice', 'normal', 1, 'IT 部', CURRENT_DATE, 'published', FALSE);

-- 站内消息
INSERT INTO oa_message (message_no, message_type, title, content, receiver_type, receiver_ids, business_type, business_id) VALUES
('MSG20260316001', 'approval', '您有新的采购订单待审批', '您有一笔采购订单（PO20260316001）需要审批，请及时处理。', 'user', ARRAY[1], 'purchase_order', 1),
('MSG20260316002', 'task', '新的销售线索分配', '您有新的销售线索（LEAD20260316001）已分配给您，请及时跟进。', 'user', ARRAY[1], 'crm_lead', 1);

-- ========================================
-- 5. 报表测试数据
-- ========================================

-- 报表组件
INSERT INTO report_widget (widget_name, widget_type, chart_type, is_system, is_active) VALUES
('销售趋势图', 'chart', 'line', TRUE, TRUE),
('销售占比饼图', 'chart', 'pie', TRUE, TRUE),
('产品销售排行', 'table', NULL, TRUE, TRUE),
('关键指标卡片', 'card', NULL, TRUE, TRUE),
('销售地图', 'map', NULL, TRUE, TRUE);

-- ========================================
-- 6. 日志测试数据（可选）
-- ========================================

-- 操作日志示例
INSERT INTO log_operation (module, operation_type, business_type, business_id, user_id, username, operation_desc, ip_address) VALUES
('procurement', 'create', 'purchase_order', 1, 1, 'admin', '创建采购订单 PO20260316001', '192.168.1.100'),
('sales', 'update', 'sales_order', 1, 2, 'user1', '更新销售订单 SO20260316001', '192.168.1.101'),
('inventory', 'approve', 'inventory_transfer', 1, 3, 'user2', '审批库存调拨单', '192.168.1.102');

-- 登录日志示例
INSERT INTO log_login (username, login_status, login_type, ip_address, browser, os) VALUES
('admin', 'success', 'password', '192.168.1.100', 'Chrome 120', 'Windows 10'),
('user1', 'success', 'password', '192.168.1.101', 'Firefox 121', 'Windows 11'),
('user2', 'failed', 'password', '192.168.1.102', 'Chrome 120', 'macOS', '密码错误');

-- ========================================
-- 7. 批次追溯日志测试
-- ========================================

-- 批次转换日志
INSERT INTO batch_trace_log (trace_no, business_type, business_id, trace_direction, internal_product_code, internal_color_no, supplier_product_code, supplier_color_code, validation_result, operator_id, operation_type) VALUES
('TR20260316001', 'purchase_receipt', 1, 'supplier_to_internal', 'PROD001', 'COLOR001', 'SPROD001', 'SCOLOR001', 'success', 1, 'convert'),
('TR20260316002', 'sales_delivery', 1, 'internal_to_supplier', 'PROD001', 'COLOR001', 'SPROD001', 'SCOLOR001', 'success', 1, 'convert');

-- ========================================
-- 8. 更新相关统计表
-- ========================================

-- BPM 统计
INSERT INTO bpm_statistics_daily (statistics_date, process_definition_id, initiated_count, completed_count, pending_tasks, completed_tasks) VALUES
(CURRENT_DATE, 1, 5, 3, 10, 25),
(CURRENT_DATE, 2, 8, 6, 15, 40);

-- ========================================
-- 测试数据创建完成
-- ========================================
