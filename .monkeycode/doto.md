# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单）。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近整理：2026-07-28（**P1 PR #758 CI 真实状态核查：7 项 FAIL 未全绿，PR OPEN/BLOCKED 未合并，分支 fix/p0-d08-d09-d10-batch-resume 未删除，修改不正确**；**Model 字段缺失编译错误已修复：18 处初始化位置补 ..Default::default()/None，涉及 7 文件**）
>
> **CI 失败清单（PR #758，运行 #30320573270，2026-07-28 01:33 UTC）**：
> - 🔧 Rust 格式检查 FAIL：`backend/tests/sales_delivery_workflow_test.rs`、`backend/tests/test_csrf_middleware.rs`、`backend/tests/websocket_test.rs` 等测试文件格式差异
> - 🔍 Rust Clippy FAIL：编译错误 + 多个 unused import warning
> - 🏗️ Rust 后端构建 FAIL：30+ 类编译错误（详见下方 0.0 节）
> - 🔍 前端 ESLint FAIL：大量 prettier/prettier 格式错误 + 1 处 Parsing error（Unterminated string literal）+ vue/no-mutating-props 错误
> - 🔬 前端类型检查 FAIL：`src/views/businessTrace/index.vue(224,18): error TS2322`
> - 🧪 前端测试 FAIL：76/76 测试通过，但 CI step FAIL（疑为覆盖率或 Vue warn 阻塞）
> - 🔧 前端格式检查 FAIL
>
> **PR 状态**：#758 OPEN/BLOCKED/MERGEABLE，base=main，head=fix/p0-d08-d09-d10-batch-resume，未合并
> **分支状态**：fix/p0-d08-d09-d10-batch-resume 本地+远程均存在，未删除
> **工作区状态**：dashboard_handler.rs + inventory_stock_handler.rs 2 文件本地修改未提交（修改方向正确但未 push）
>
> P0 全部完成已归档到 doto-su.md；P1 修复进行中（详见 doto-su.md 与 CHANGELOG.md）；原整理内容：P0 全部完成已归档到 doto-su.md；P1 修复进行中：P1-A + P1-B1 + P1-B2 + P1-C + **P1 面料行业深化 2 批次（batch-04 + batch-05）22 项 P1 已完成** + **P1-D 法律合规 batch-08 P1-08-22 加班工时 + batch-20 前端架构 10 项 P1（含 P1-20-2 移动端侧边栏抽屉化）已完成** + **P1-batch13/14 类十五业务主体 1 项 P1（已并入 P1-C）+ 类十六 AI 模块 24 项 P1 已完成** + **P1-Batch16 隐私合规 5 项 P1（缺陷 7.2/7.3/7.4/8.3/8.4）已完成** + **P1-batch11/12 类十三打印导出 14 项 P1 + 类十四权限维度 14 项 P1 已完成** + **P1-08 法律合规 batch-08 第二批 11 项 P1（缺陷 7/8/9/10/13/14/15/18/19/21/23/24）已完成** 待 CI 验证）

---

## 〇、P1 级任务进度总览（2026-07-27 启动）

### 0.1 按批次状态归类

| 状态 | 数量 | 批次 |
|------|------|------|
| 🔵 代码完成待 CI | 11 批 | P1-A、P1-B1、P1-B2、P1-C、**P1-面料行业深化（batch-04 + batch-05）**、**P1-D（batch-08 P1-08-22 加班工时 + batch-20 前端架构 10 项 P1）**、**P1-batch13/14（类十五业务主体 1 项 P1 + 类十六 AI 模块 24 项 P1）**、**P1-Batch16 隐私合规 5 项 P1（缺陷 7.2/7.3/7.4/8.3/8.4）**、**P1-batch11/12（类十三打印导出 14 项 P1 + 类十四权限维度 14 项 P1）**、**P1-batch19（类二十三组织定制物流 10 项 P1）**、**P1-08 法律合规 batch-08 第二批 11 项 P1（环保/劳动/财税法律合规）**（详见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md) 与 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)） |
| ⏳ 进行中 | 0 批 | — |
| ❌ 未开始 | 剩余约 14 批 | P1-B3 起（脱敏扩展等） |

### 0.2 P1-08 法律合规 batch-08 第二批 11 项 P1 完成清单（环保/劳动/财税法律合规）

**依据**：[batch-08/audit-report.md](file:///workspace/.monkeycode/docs/audits/v15/batch-08/audit-report.md)（缺陷 7/8/9/10/13/14/15/18/19/21/23/24）

1. ✅ 缺陷 7：染整报表/色卡/工资/能耗导出补齐 .xlsx（export_service.rs export_xlsx 系列方法）
2. ✅ 缺陷 8：合同/发票/报表新增 .docx 格式（docx-rs 依赖 + utils/docx_export.rs + print_service.rs generate_docx + export_service.rs export_docx/generate_reconciliation_docx）
3. ✅ 缺陷 9：面料执行标准登记（GB/T 系列，产品模型扩展）
4. ✅ 缺陷 10：销售合同电子签章（sales_contracts 表 5 字段 + sales_contract.rs 模型 + contract_signature_service.rs sign_contract/verify_signature/revoke_signature + compute_contract_hash SHA-256 防篡改）
5. ✅ 缺陷 13：委外凭证进项税转出（outsourcing_voucher 表 tax_transfer_amount + outsourcing_voucher.rs 模型 + outsourcing_ops/order.rs 非正常损耗加工费进项税转出计算）
6. ✅ 缺陷 14：出口退税免抵退核算（3 表 + 3 model + export_refund_service.rs calculate_exempt_credit_refund 纯函数）
7. ✅ 缺陷 15：环保税核算（pollutant_discharge_records 表 + model + environmental_tax_service.rs calculate_tax 纯函数 + generate_tax_declaration 按期间汇总）
8. ✅ 缺陷 18：排污许可证管理（pollution_permits 表 + model + pollution_permit_service.rs scan_expiry_warnings 90/60/30 天三级预警）
9. ✅ 缺陷 19：污染物监测+固废处置（2 表 + 2 model + pollution_monitoring_service.rs 自动超标判定 + PollutionLimitReference GB 4287-2012/GB 16297/GB 12348 国标限值）
10. ✅ 缺陷 21：劳动合同电子化（labor_contracts 表 + model + labor_contract_service.rs validate_probation《劳动合同法》第19/20条合规校验 + scan_expiry_warnings）
11. ✅ 缺陷 23：社保公积金扣缴（social_insurance_records 表 + model + social_insurance_service.rs calculate_insurance 五险一金费率 + validate_base_amount 缴费基数合规 + pending→paid/cancelled 状态机）
12. ✅ 缺陷 24：职业健康合规（3 表 + 3 model + occupational_health_service.rs 危害因素检测自动超标预警 + 体检档案 90/60/30 天到期预警 + PPE 发放回收/过期扫描 + OccupationalHazardLimitReference GBZ 2.1/2.2 国标限值）

**辅助工程**：
- migration m0079_batch08_compliance_legal_env_tax_labor 统一迁移（所有新表新字段 NULLABLE 或 DEFAULT，蓝绿部署兼容）
- 8 新服务已在 services/mod.rs 注册（contract_signature_service / export_refund_service / environmental_tax_service / pollution_permit_service / pollution_monitoring_service / labor_contract_service / social_insurance_service / occupational_health_service）
- 12 新 model 已在 models/mod.rs 注册（export_customs_declaration / foreign_exchange_verification / export_refund_declaration / pollutant_discharge_record / pollution_permit / pollutant_monitoring_record / solid_waste_disposal_record / labor_contract / social_insurance_record / occupational_hazard_monitoring / occupational_health_exam / ppe_distribution_record）
- migration 已在 migration/src/lib.rs 注册（m0079_batch08_compliance_legal_env_tax_labor）

### 0.3 P1-D（batch-08 P1-08-22 + batch-20 前端架构 10 项 P1）完成清单（11 项 P1）

**batch-08 类八法律合规 P1**：
1. ✅ P1-08-22：wage_record_detail 新增加班工时字段（weekday/weekend/holiday_overtime_minutes + overtime_pay）+ calculate_overtime_pay 函数（《劳动法》第 44 条 1.5x/2x/3x）+ migration 055 + m0074 整合迁移

**batch-20 类二十四前端架构 P1**：
1. ✅ P1-20-1：PWA 支持（manifest.json + Service Worker + index.html 注册 + theme-color）
2. ✅ P1-20-2：移动端侧边栏抽屉化（useBreakpoint composable + MainLayout 动态组件 ElDrawer/ElAside + 汉堡按钮 ≥44px + 路由切换自动关闭抽屉 + i18n 双语）
3. ✅ P1-20-3：vite manualChunks（vue/element-plus/echarts/utils 4 chunk）+ optimizeDeps include
4. ✅ P1-20-4：echarts 按需引入（utils/echarts.ts + BaseChart.vue 改用 echarts/core）
5. ✅ P1-20-6：vitest 覆盖率门槛 60%→70%
6. ✅ P1-20-7：nginx 安全头（CSP/X-Frame-Options DENY/X-Content-Type-Options/Referrer-Policy/Permissions-Policy）+ SW/manifest 缓存规则
7. ✅ P1-20-9：Vue 3 ErrorBoundary 组件（onErrorCaptured + 重试/回首页/错误详情 + i18n + logger 上报）
8. ✅ P1-20-14：keep-alive 状态保留（MainLayout router-view 包裹 keep-alive + cachedViewNames 高频页面）
9. ✅ P1-20-15：CSS 变量替代硬编码（styles/theme.css 全局变量 + MainLayout 局部变量）
10. ✅ P1-20-16：暗黑模式切换（useTheme composable + html.dark 选择器 + localStorage 持久化 + 系统偏好跟随）

### 0.4 P1 面料行业深化（batch-04 + batch-05）完成清单（22 项 P1）

**batch-04 类四 面料行业深化（11 项 P1，全部完成）**：
1. ✅ batch_trace_log 字段扩展（dye_lot_no/color_no/product_id/from_status/to_status + operation_type 注释扩展，migration 051）
2. ✅ 面料检验物理指标建模（fabric_physical_test_record 新模型 + migration 052 + 10 项指标）
3. ✅ grade_inspection 增强 A 级判定（含物理指标检查）
4. ✅ QualityInspectionCompleted 事件发布（fabric_inspection_service.rs）
5. ✅ 工资凭证生成（create_wage_confirm_voucher/create_wage_pay_voucher）
6. ✅ WageConfirmed/WagePaid 事件发布（wage_ops/record.rs）
7. ✅ 能耗分摊 dye_lot_no 修正（group_step_duration_by_key 查询 production_flow_card）
8. ✅ 委外 4 事件发布（OutsourcingOrderCreated/StatusChanged/ReceiptConfirmed/CostSettled）
9. ✅ 业务模式 2 事件发布（BusinessModeChanged/OrderBusinessModeLinked）
10. ✅ 事件总线扩展 + Kafka payload 序列化
11. ✅ 事件监听器实现（幂等处理）

**batch-05 类五 运行逻辑闭环（11 项 P1，全部完成）**：
1. ✅ 缸号状态机 OnHold+Failed 2 新状态（quality_dyeing.rs + state_machine_service + migration 053，HOLD/RESUME/FAIL 流转码）
2. ✅ 面料行业 6 配置项（FabricIndustryConfig：DYEHOUSE_VAT_COUNT/PROCESS_UNIT_PRICE_BASE/ENERGY_ALLOCATION_RULE/QUALITY_GRADE_THRESHOLD_A/B/C/DYEBATCH_STATUS_TIMEOUT，.env.example + config.yaml.example）
3. ✅ 染整工序扫码上报事件（ProcessStepReported）
4. ✅ 缸号状态变更事件（DyeBatchStatusChanged）
5. ✅ 验布分级事件（FabricInspectionGraded）
6. ✅ 产量上报事件（ProductionQuantityReported）
7. ✅ 能耗采集事件（EnergyConsumptionRecorded）
8. ✅ 色卡发放事件（ColorCardIssued）
9. ✅ 生产订单成本归集按缸号（dye_lot_no 从 production_order 读取传入 cost_collection）
10. ✅ 染色成本归集 dye_lot_no 从 dye_batch 表查询（dye_batch_cost_bridge_service.rs）
11. ✅ **销售成本移动加权平均法**（fetch_purchase_unit_price + update_moving_average_cost + create_purchase_receipt_voucher 使用实际采购价 + create_sales_delivery_voucher 使用移动加权平均成本）

### 0.5 P1-batch13/14（类十五业务主体 1 项 + 类十六 AI 模块 24 项 P1）完成清单（25 项 P1）

**batch-13 类十五 业务主体（1 项 P1，已并入 P1-C）**：
1. ✅ supplier_evaluation_records 表无 migration → m0069_create_supplier_evaluation_records 迁移补齐（FK/CHECK/4 索引 + 注册到 Migrator）

**batch-14 类十六 AI 模块（24 项 P1，全部完成）**：
1. ✅ 缺陷 4.1：AI 端点权限码注册到 init_admin_permissions.sql（非 admin 角色可访问）
2. ✅ 缺陷 4.2：advanced 域 AI 端点路径解析修复（path_utils.rs + permission.rs extract_resource_info 处理嵌套模块前缀）
3. ✅ 缺陷 4.3：AI 推理数据范围按用户过滤（DataScopeContext 透传）
4. ✅ 缺陷 5.1：AI 推理超时控制（tokio::time::timeout 2s 包装 recipe_opt/quality_pred 算法）
5. ✅ 缺陷 5.2：AI 并发控制（Semaphore permits=10，AI_CONCURRENCY_LIMIT 常量）
6. ✅ 缺陷 5.3：AI 缓存策略（moka Cache TTL 5min + capacity 1000，recipe_cache/quality_cache）
7. ✅ 缺陷 9.1：模型不可用降级（build_degraded_response 返回典型参数表/保守默认值 + degraded=true）
8. ✅ 缺陷 9.5：AI 推理超时降级（超时返回降级结果而非 500）
9. ✅ 缺陷 6.1：AI 数据脱敏（field_mask.rs 新增 mask_text_pii 捕获手机/邮箱/身份证 PII + 单元测试）
10. ✅ 缺陷 6.2：推理数据最小化（查询结果 LIMIT 限制）
11. ✅ 缺陷 1.1：染料-布类配伍性校验（is_dye_fabric_compatible + validate_dye_fabric_compatibility 不配伍返回 422）
12. ✅ 缺陷 1.3+8.1：工艺优化→化验室打样集成（push_to_lab_dip 推送推荐参数到 lab_dip）
13. ✅ 缺陷 2.1+8.3：质量预测实际结果回填（record_actual_quality_result 更新 actual_risk_level/actual_avg_qualification_rate）
14. ✅ 缺陷 2.2：质量预测特征完整化（dye_type/auxiliary_type/temperature_range/batch_no/fabric_source 5 面料行业特征）
15. ✅ 缺陷 2.4+8.3：质量预测准确率对账（AiQualityReconciliationService.reconcile_monthly 按月对账 + ai_quality_accuracy_reports 表）
16. ✅ 缺陷 3.1+10.2：模型版本管理（create_model_version/approve_model_version/change_model_status draft→active→retired→archived 状态机 + ai_model_versions 表）
17. ✅ 缺陷 3.4：模型评估指标（create_model_evaluation + ai_model_evaluations 表）
18. ✅ 缺陷 3.5：模型漂移检测（detect_model_drift 对比准确率/置信度阈值）
19. ✅ 缺陷 8.2：工艺优化→生产执行集成（link_to_production_recipe 关联 production_recipe_id）
20. ✅ 缺陷 8.4：补货推荐与 MRP 引擎对账（reconcile_suggestion_with_mrp 差异>20% 标注人工复核）
21. ✅ 缺陷 10.1：AI 决策审计日志（log_decision 异步记录 + ai_decision_logs 表）
22. ✅ 缺陷 10.2：模型变更审计日志（approve_model_version 审批流 + status 变更记录）

### 0.6 P1-batch11/12（类十三打印导出 14 项 P1 + 类十四权限维度 14 项 P1）完成清单（28 项 P1）

**batch-11 类十三 打印导出（14 项 P1，全部完成）**：
1. ✅ 缺陷 1-4：色卡导出补审计（OperationType::Export + resource_type=color_card）
2. ✅ 缺陷 1-5：5 个 print_html handler 补审计接入（AuthContext + AuditLogService::record_async）
3. ✅ 缺陷 1-6：MRP/AR 对账单导出补审计落库（替换 info! 为 record_async）
4. ✅ 缺陷 1-7：销售/采购订单导出补审计（export_count + query_filter 写入 after_snapshot）
5. ✅ 缺陷 1-8：CRM 线索/商机导出补审计
6. ✅ 缺陷 2-3：前端导出按钮补 v-permission 权限指令（25+ 按钮）
7. ✅ 缺陷 2-4：禁止打印/导出角色清单实现（PRINT_DENIED_ROLES/EXPORT_DENIED_ROLES 常量）
8. ✅ 缺陷 3-3：audit_logs 表补导出专属字段（export_record_count/export_query_filter/export_file_format/export_approval_token/export_watermark_user）
9. ✅ 缺陷 4-3：永久禁止导出规则实现（lab_dip/production_recipe/flow_card 资源黑名单）
10. ✅ 缺陷 5-3：printData 补后端审计埋点（api.post('/audit/record')）
11. ✅ 缺陷 7-1：omni_audit 分类增强（classify_operation 识别 PRINT/EXPORT/DOWNLOAD）
12. ✅ 缺陷 9-1：导出全局并发控制（ExportConcurrencyGuard + AtomicUsize + MAX_CONCURRENT_EXPORTS=10）
13. ✅ 缺陷 9-3：sales_order/purchase_order 导出条数上限（.limit(10000)）
14. ✅ 缺陷 10-1/10-2：每日合规审查定时任务 + 异常导出行为识别（6 类规则：高频/大批量/非工作时间/离职/跨权限/敏感无审批）

**batch-12 类十四 权限维度（14 项 P1，全部完成）**：
1. ✅ 缺陷 14.2-C：admin 持有 audit:read 违反职责分离（移除 admin audit:read，审计职责独立到 auditor 角色）
2. ✅ 缺陷 14.3-D：采购/销售审批与创建未分离（拆分 sales/purchase create 与 approve 权限，SoD 校验 validate_sod_create_approve）
3. ✅ 缺陷 14.4-C：权限码与路由资源类型不匹配（resolve_module_prefixed_resource 消歧映射 purchase/orders→purchase-orders）
4. ✅ 缺陷 14.4-D：模块前缀不在白名单时资源类型提取错误（补齐 path_utils.rs 模块前缀白名单 + is_business_module_prefix）
5. ✅ 缺陷 14.7-B：业务角色无 dashboard:read 权限（14 类业务角色补 dashboard:read 种子）
6. ✅ 缺陷 14.8-B：字段级权限种子数据为空（migration 20260730000001_init_field_permission_seed 敏感字段权限）
7. ✅ 缺陷 14.9-C：权限缓存无 Redis pub/sub 热更新（start_permission_cache_pubsub_subscriber + 频道 permission_cache_invalidation）
8. ✅ 缺陷 14.10-B：异常权限分配识别规则（permission_compliance_service 6 类检测规则）
9. ✅ 缺陷 14.10-C：定期合规审查机制（compliance_review 3 项系统级检查）
10. ✅ 缺陷 14.11-A：非 admin 角色权限拒绝集成测试（test_permission_rbac.rs 10 个测试场景）
11. ✅ 缺陷 14.11-B：is_system=true 注入 *:* 测试（admin_checker.rs 8 个单元测试验证角色 code 判定）
12. ✅ 缺陷 14.11-C：权限缓存失效生命周期测试（permission.rs 5 个场景：insert→invalidate→reload→expiry 完整链路 + 多角色隔离）
13. ✅ 缺陷 14.12-B：模块前缀不在白名单时资源类型提取错误（同 14.4-D 修复 + fail-closed）
14. ✅ 缺陷 14.12-E：role.code 可被修改导致权限提升（update_role 移除 code 字段更新 + 唯一约束）

### 0.7 待启动批次（优先级从高到低）

- **P1-B3**：脱敏扩展到 customer/supplier 模块 + 规则 4 注释精简（剩余部分）
- **P1-E ~ P1-?**：剩余批次逐步推进

### 0.8 P1-batch19 类二十三组织定制物流完成清单（10 项 P1）

1. ✅ 23.1.2 一人多部门（user_departments 关联表 + is_primary/start_date/end_date + model 注册 + migration m0079）
2. ✅ 23.2.2 定制订单客户签字确认（custom_order 加 customer_approved_at/customer_approval_comment/quality_standard_id + QualityStandard 关联）
3. ✅ 23.2.3 定制订单变更二级审批（custom_order 加 approval_instance_id/approved_by/approved_at/rejection_reason）
4. ✅ 23.3.2 售后流程闭环（after_sales 加 accepted_at/evaluation_score/evaluation_comment/evaluated_at，6 步流程 opened→accepted→processing→resolved→evaluated→closed）
5. ✅ 23.3.3 售后原因分析（after_sales 加 reason_category/reason_detail，quality/logistics/customer_preference/other 分类）
6. ✅ 23.4.1 运单关联采购订单（logistics_waybill 加 order_type：sales_order/purchase_order/transfer_order）
7. ✅ 23.4.2 物流跟踪历史（logistics_tracking_events 新模型 + has_many 关联 + migration m0079）
8. ✅ 23.4.3 运费核算（logistics_waybill 加 total_weight/total_volume/distance_km/freight_rate/freight_bearer）
9. ✅ 23.5.2 术语与价格构成集成（sales_quotation 加 freight_cost/insurance_cost/duty_cost）
10. ✅ 23.5.4 术语使用月报（finance_report_service.rs 新增 get_incoterm_monthly_report + IncotermMonthlyReport/IncotermStatItem 结构体，SQL 参数化绑定合规）

---

## 一、P1/P2/P3 任务规划（按类别汇总）

> P0 完成后按优先级顺序推进。详细内容见 V15 审计报告 [docs/audits/v15/](file:///workspace/.monkeycode/docs/audits/v15/)。

### 1.1 P1 高优先级（257 项，预估 45-55 批次，按每批 9-12 文件计算）

| 模块 | P1 数 | 主要内容 | 关键批次预估 |
|------|-------|----------|--------------|
| 类二 通用代码质量 | 3 | api 命名/缩写命名/DbErr 包装 | 2 批 |
| 类三 安全性 | 6 | refresh_token/PUBLIC_PATHS/validator/Webhook/magic bytes/zip bomb | 3 批 |
| 类四 面料行业深化 | 11 | batch_trace/检验指标/工资凭证/能耗/委外/事件发布/工时 | 4 批 |
| 类五 运行逻辑闭环 | 11 | 状态机/配置/业务事件/成本归集/加权平均 | 4 批 |
| 类六 测试体系 | 11 | 覆盖率/mock/fixtures/文档 | 4 批 |
| 类七 可维护性 | 11 | i18n/aria/缓存/文档 | 4 批 |
| 类八 法律合规 | 16 | 用户协议/HTTPS/脱敏/导出/docx/标准/签章/税/环保/排污/劳动/工时/社保/职业健康 | 6 批 |
| 类九 色卡发放 | 9 | 清单/通知/报表 | 3 批 |
| 类十 大货批色 | 7 | 提醒/报表/统计 | 3 批 |
| 类十三 打印导出 | 14 | 审计字段/水印/性能 | 5 批 |
| 类十四 权限维度 | 14 | 权限测试/审计/缓存 | 5 批 |
| 类十五 业务主体 | 1 | supplier_evaluation migration | 1 批 |
| 类十六 AI 模块 | 24 | 配伍性/化验室/准确率/版本/权限/超时/并发/缓存/脱敏/MLOps | 8 批 |
| 类十七 财务深化 | 35 | 期间/反结账/年结/回转/账龄/杜邦/预测/差异/折旧 | 12 批 |
| 类十八 CRM | 12 | 线索评分/去重/转移审批 | 4 批 |
| 类十九 报表 BI | 5 | 版本管理/缓存 | 2 批 |
| 类二十 可观测性 | 9 | trace/metrics/WebSocket | 3 批 |
| 类二十一 胚布拆匹 | 10 | 库存/委外/继承 | 4 批 |
| 类二十二 库存排程 | 9 | 调拨/安全/排程 | 3 批 |
| 类二十三 组织物流 | 11 | 组织树/售后/运费 | 4 批 |
| 类二十四 前端架构 | 16 | PWA/移动端/chunks/ErrorBoundary/CSP/keep-alive/CSS/暗黑 | 6 批 |
| 类二十五 部署升级 | 11 | set -euo/SHA256/schema/蓝绿/健康/优雅/回滚 | 4 批 |
| **合计** | **257** | | **约 45 批**（每批 9-12 文件） |

### 1.2 P2 中优先级（248 项，预估 35-45 批次）

| 类别 | P2 数 | 主要内容 |
|------|-------|----------|
| 类一~类四 | 19 | 代码质量 / 安全防护 / 面料行业字段补齐 |
| 类五~类八 | 47 | 运行逻辑 / 测试补充 / 可维护性 / 法律合规细节 |
| 类九~类十二 | 33 | 色卡发放细节 / 大货批色细节 / 打印导出 / 权限细节 |
| 类十三~类十四 | 25 | 打印导出 P2 / 权限 P2 |
| 类十五~类十六 | 53 | 业务主体 P2 / AI 模块 P2 |
| 类十七~类十九 | 39 | 财务 P2 / CRM P2 / 报表 BI P2 |
| 类二十~类二十二 | 25 | 可观测性 / 胚布 / 库存 P2 |
| 类二十三~类二十五 | 83 | 组织物流 / 前端架构 / 部署升级 P2 |
| **合计** | **248** | |

### 1.3 P3 低优先级（123 项，按需修复）

| 类别 | P3 数 | 主要内容 |
|------|-------|----------|
| 类一~类四 | 11 | 文档 / 注释 / 命名优化 |
| 类五~类八 | 17 | 测试增强 / 可维护性增强 / 法律合规增强 |
| 类九~类十二 | 9 | 色卡 / 批色 / 打印 / 权限增强 |
| 类十三~类十四 | 5 | 打印导出 / 权限增强 |
| 类十五~类十六 | 25 | 业务主体增强 / AI 增强 |
| 类十七~类十九 | 11 | 财务 / CRM / 报表增强 |
| 类二十~类二十二 | 12 | 可观测性 / 胚布 / 库存增强 |
| 类二十三~类二十五 | 41 | 组织物流 / 前端架构 / 部署升级增强 |
| **合计** | **123** | |

---

## 二、规则节点提醒

| 规则 | 优先级 | 内容 |
|------|--------|------|
| 规则 0/1/2/8 | 🔴 | 真实实现强制：所有 P0/P1 修复必须真实实现，禁止占位符 |
| 规则 3 | 🔴 | 成品文档格式：导出必须 .xlsx / 报表必须 .docx |
| 规则 4 | 🔴 | `///` 注释精简为 1 行（首选），最多 2 行，禁止 3 行+注释块 |
| 规则 5 | 🟡 | E2E 独立工作流：每 30 批次触发（批次 30/60/90...） |
| 规则 6 | 🔴 | 测试 mock 数据禁止硬编码：所有测试 mock 数据抽取到 fixtures |
| 规则 10 | 🟡 | 每 15 批次记忆整理 + 实时归档：每批完成后立即归档到 doto-su.md |
| 规则 11/12 | 🔴 | 法律合规与安全标准：所有修复必须符合中国法律法规 + 安全标准 |
| 规则 13 | 🔴 | 修复流程自动化：CI 全绿后自动开始下一批；步骤 0 确定审计结果内容是否存在 + 步骤 4 修复后推送前自审 |
| 规则 14 | 🔴 | 移除所有警告抑制：所有警告视为错误需修复（baseline 213/213 ✅ 全部清零） |
| 规则 15 | 🟢 | V15 全项目综合审计：25 大类 195 维度审计 ✅ 已完成 |
| 规则 19 | 🟡 | 工具连接异常分级响应：L1 60s / L2 60-180s / L3 30min 周期 |
| 规则 20 | 🔴 | 注释与功能一致性：代码注释必须与功能实现一致，禁止随意编写；CI 强制检查 |

---

## 三、历史归档索引

> 详细历史任务归档见 [archives/2026-07-22/doto-historical-tasks.md](file:///workspace/.monkeycode/docs/archives/2026-07-22/doto-historical-tasks.md)，包含：
> - P0 批次规划表（39 项 → 22 批次）
> - 已完成模块 A-F 清单（39 项 P0 任务全部完成）
> - 历史阶段任务（v13/v14 复审修复 + V15 审计 + V15 修复阶段一/续/复审归档/复审报告）

> P0 模块 G（D01-D17）已完成归档见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md) §📋 P0 模块 G 任务归档。

---

## 四、P1-batch-07 ElMessage i18n 硬编码修复完成（2026-07-28）

### 4.1 修复摘要

将前端 47 个文件中 159 处 `ElMessage.success/error/warning/info('中文硬编码')` 调用替换为 i18n 国际化调用，使用 `msg` 对象（`@/utils/message`）封装的 `msg.success('i18nKey')` / `msg.error('i18nKey')` / `msg.translate('i18nKey')` 模式。

### 4.2 修复模式

1. **直接调用替换**：`ElMessage.success('操作成功')` → `msg.success('operationSuccess')`
2. **回退模式替换**：`ElMessage.error(err.message || '操作失败')` → `ElMessage.error(err.message || msg.translate('operationFailed'))`
3. **动态参数替换**：`` ElMessage.success(`发现新版本: ${res.data.version}`) `` → `msg.success('newVersionFound', { version: res.data.version })`
4. **模板字符串替换**：`` ElMessage.success(`排程完成: ${result.scheduled_count} 个任务, ${result.conflict_count} 个冲突`) `` → `msg.success('scheduleComplete', { scheduledCount: result.scheduled_count, conflictCount: result.conflict_count })`

### 4.3 修改文件清单（20 个文件，本轮修复）

| 文件 | 修复数 | 说明 |
|------|--------|------|
| locales/zh-CN.ts | +3 键 | 新增 checkFailed/sendNotificationFailed/markFailed |
| locales/en-US.ts | +3 键 | 对应英文翻译 |
| useMsProc.ts | 4 处 | 重命名局部 msg 变量为 errMsg 避免冲突 |
| useApiKey.ts | 1 处 | 直接调用替换 |
| usePrdProc.ts | 2 处 | 回退模式替换 |
| usePp.ts | 2 处 | 回退模式替换 |
| useSp.ts | 2 处 | 回退模式替换 |
| useSpProc.ts | 3 处 | 回退模式替换 |
| useSc.ts | 2 处 | 回退模式替换 |
| useScProc.ts | 4 处 | 回退模式替换 |
| useCp.ts | 3 处 | 回退模式替换 |
| useSchM.ts | 2 处 | 回退模式替换 |
| useSchG.ts | 2 处 | 回退模式替换 |
| useSchMProc.ts | 2 处 | 新增 msg 导入 + 模板字符串替换 |
| useSchGProc.ts | 2 处 | 新增 msg 导入 + 模板字符串替换 |
| usePurchList.ts | 1 处 | 新增 msg 导入 + 回退模式替换 |
| useApiLog.ts | 1 处 | 新增 msg 导入 + 回退模式替换 |
| usePrc.ts | 1 处 | 新增 msg 导入 + 回退模式替换 |
| useOlvProc.ts | 1 处 | 回退模式替换 |
| useSysUpdProc.ts | 7 处 | 回退模式替换 |

### 4.4 验证结果

- Grep 搜索 `ElMessage.(success|error|warning|info)(['"][^'"]*[\u4e00-\u9fa5]` → 0 匹配
- Grep 搜索 `ElMessage.(success|error|warning|info)(\`[^\`]*[\u4e00-\u9fa5]` → 0 匹配
- Grep 搜索 `ElMessage.[a-z]+\([^)]*\|\| ['"][^'"]*[\u4e00-\u9fa5]` → 0 匹配

---

## 五、P1-batch-07 CI 失败修复（2026-07-28）

### 5.1 背景

P1-batch-07 ElMessage i18n 硬编码修复（§四）引入的前端 CI 失败（ESLint + 类型检查 + 测试）。本轮修复针对 i18n 修改（ElMessage → msg）引入的 4 类已知问题模式进行排查与修复。

### 5.2 排查结果（全部通过）

| 检查项 | 结果 | 说明 |
|--------|------|------|
| 未使用的 ElMessage 导入 | ✅ 0 处 | 23 个修改文件中，导入 ElMessage 的文件均仍在 catch 块使用 `ElMessage.error()`（回退模式），无多余导入；9 个仅导入 ElMessageBox 的文件均使用 ElMessageBox |
| 缺少 msg 导入 | ✅ 0 处 | 所有使用 `msg.X()` 的文件均已包含 `import { msg } from '@/utils/message'` |
| 变量名冲突（局部 msg） | ✅ 0 处 | useMs.ts / useDiProc.ts 的 `const msg =` 已重命名为 `errMsg`；代码库其余 `const msg =` 所在文件均未导入 msg 对象 |
| 翻译键缺失 | ✅ 0 处 | 修改文件中 83 个唯一翻译键在 zh-CN.ts / en-US.ts 均已存在；`exportBlockedResource` 键已补齐 |
| msg.warning/info() 参数 | ✅ 0 处 | 所有 `msg.warning()` / `msg.info()` 调用均传入必填 key 参数 |
| 行长超 100 字符 | ✅ 0 处 | 仅 useVchrProc.ts:118 一行超 100 字符，位于模板字面量（打印 HTML），Prettier 不换行模板字面量，非违规 |
| 测试影响 | ✅ 0 处 | 无测试文件直接引用修改的 composables；login.test.ts / audit-log.test.ts 的 ElMessage mock 对 msg 包装器仍生效 |

### 5.3 本轮修改文件清单（23 个文件，+162 -64 行）

locales/zh-CN.ts（+exportBlockedResource 键）、locales/en-US.ts（+exportBlockedResource 键）、useAi.ts、useRcp.ts、useApiEp.ts、useApiKey.ts、useApiLog.ts、useBpmDfProc.ts、useCp.ts、useDb.ts、useDiProc.ts（局部 msg→errMsg）、useVchrProc.ts、useMs.ts（局部 msg→errMsg）、useMsProc.ts、useCreate.ts、usePurchList.ts、usePurchRcv.ts、useSr.ts、useSchGProc.ts（msg.success 换行）、useSchM.ts（msg.warning 换行）、useSchMProc.ts（msg.success 换行）、useSysUpd.ts、useSysUpdProc.ts（7 处 ElMessage.error 换行）。

### 5.4 修复模式

1. **变量名冲突修复**：catch 块 `const msg = error instanceof Error ? ...` → `const errMsg = ...`（useMs.ts / useDiProc.ts）
2. **Prettier 行长修复**：超过 100 字符的 `ElMessage.error(...)` / `msg.success(...)` / `msg.warning(...)` 调用换行格式化
3. **翻译键补齐**：zh-CN.ts / en-US.ts 新增 `exportBlockedResource` 键（utils/export.ts 已使用）
