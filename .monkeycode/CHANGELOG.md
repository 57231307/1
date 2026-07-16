# 任务一句话总结

> 每个任务一行摘要，是 doto-su.md 中详细任务内容的一句话总结。禁止写入详细内容。
> 详细任务内容见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，未完成任务见 [doto.md](file:///workspace/.monkeycode/doto.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近整理：2026-07-16（V15 修复阶段 Batch 433-452 完成，P0-S03/S04/S20/S21/S22/S01(基础设施+销售域+采购域+生产域+CRM域+财务域finance+AP+AR+库存调整+库存预留)/S18/S07/S05/S06/S10/S09(全部)/S11(全部) 修复，PR #611/#612/#613/#614/#616/#617/#618/#619/#620/#621/#622/#623/#624/#625/#626/#627/#628/#629/#630/#631/#632/#633/#634 已合并）。

---

## V15 修复阶段（2026-07-16 启动）

| 批次 | PR | 一句话总结 |
|------|-----|-----------|
| 433 | #611 | V15 P0-S03 修复超级权限注入漏洞：auth_handler.rs 将 is_system 判断改为 code==ADMIN_ROLE_CODE，仅 admin 注入超级通配权限；init_service.rs 新增 create_default_role_permissions 为 manager/operator 插入基本 role_permission 记录 |
| 434 | #612 | V15 P0-S04 补齐 31 类业务角色覆盖面料行业全业务场景（管理/销售/采购/库存/生产/质量/财务/CRM/物流/人力/安全/IT），为全部角色配置基本 role_permission 权限记录 |
| 435 | #613 | V15 P0-S20/S21/S22 权限资源缺口补齐：新增 60+ 类权限资源注册表 + 11 个操作权限码 + 33 个角色完整权限矩阵；path_utils.rs 清理 15+ 脏数据并新增 28 个模块前缀（production/auth/quotations 等）；permission.rs 新增白名单校验 + 路径动作提取（print/export/approve 等 11 个）；修复 2 个 clippy 警告（last→next_back→rfind） |
| 436 | #614 | V15 P0-S01 行级数据权限基础设施：新增 migration m0051（role 表 data_scope 字段 all/dept/self）+ data_scope.rs 工具模块（DataScope 枚举 + apply_data_scope + check_resource_owner + 15 单测）+ AuthContext 注入 data_scope/department_id + auth 中间件从 DB 加载 + 33 个角色配置 data_scope；修复编译错误（EntityTrait import）和 clippy 警告（from_str→parse_scope） |
| 437 | #616 | V15 P0-S18 新增 dye_recipe_master（染色配方主管）角色，含 dye-recipes 全部操作 + approve/audit 审批权限 + lab-dip/production-recipes/color-cards/color-prices 全部操作；与 lab_technician 区别为管理层 vs 执行层 |
| 438 | #617 | V15 P0-S07 权限缓存不失效修复：permission.rs 新增 invalidate_permission_cache/invalidate_all_permission_cache API + 3 单测；role_permission_service.rs assign_permission/remove_permission 接入缓存失效；user_service.rs update_user 角色变更失效旧+新角色缓存 + 禁用用户补 revoke_user_jtis JWT 吊销（原安全漏洞） |
| 439 | #618 | V15 P0-S05 SoD 职责分离互斥：新增 role_conflicts 表（migration m0052）+ 8 条预置互斥规则（财务三权分立/采购付款/销售收款/生产质量）+ role_conflict model + user_service check_role_conflict_for_user 校验方法 + update_user 角色变更时校验 |
| 440a | #619 | V15 P0-S06 权限变更审计基础设施：新增 migration m0053（permission_change_audits 表 13 字段 + 5 索引）+ permission_change_audit SeaORM model + 注册到 lib.rs/mod.rs；将原 Batch 440（5+ 文件）拆分为 3 个微批次避免卡死 |
| 440b | #620 | V15 P0-S06 role_permission_service 接入审计日志：assign_permission 保存旧 allowed 值写入审计（old/new value）+ remove_permission 删除前保存信息写入审计 + 新增 write_permission_audit 私有方法（best-effort）；修复 borrow after move + 删除错误 clippy baseline |
| 440c | #621 | V15 P0-S06 user_service 接入用户角色变更审计：update_user 新增 operator_id 参数 + 角色变更时写入 permission_change_audit（change_type=user_role_change，记录旧/新角色 ID）+ best-effort 策略；完成 P0-S06 全部修复 |
| 441 | #622 | V15 P0-S10 method_to_action 升级识别 print/export/download：新增 extract_action_from_query 函数（白名单 print/export/download，防止绕过）+ permission_middleware action 提取优先级升级（查询参数 > 路径关键字 > HTTP method）+ OperationType 新增 Print/Download 变体 + 8 个单元测试；修复 str_as_str 不稳定特性编译错误 |
| 442 | #623 | V15 P0-S09 染色域 export 端点补齐 AuthContext：dye_recipe_handler.rs export_dye_recipes + dye_batch_handler.rs export_dye_batches 新增 _auth: AuthContext 参数；权限校验由 permission_middleware 自动根据路径提取 action=export 校验 *:export 权限 |
| 443 | #624 | V15 P0-S09 print_handler AuthContext 补齐：print_handler.rs 7 个 print/export 函数（5 个 print_html：sales_order/sales_contract/purchase_order/purchase_receipt/inventory_transfer + list_print_templates + get_print_template）新增 _auth: AuthContext 参数；权限校验由 permission_middleware 自动根据路径提取 action=print 校验 *:print 权限；CI 12/12 全绿 |
| 444 | 无需 PR | V15 P0-S09 其他域 export AuthContext 核查：5 个目标文件（sales_order/purchase_order/product/report_engine/crm）export 函数均已含 AuthContext，无需修改；quotation/customer/supplier/inventory/finance/quality 无 export/print 端点。**P0-S09 全部完成** |
| 445 | #625 | V15 P0-S11 核心业务导出审计日志补齐（第 1 批）：5 文件 6 个 export 函数（sales_order/purchase_order/product/crm_leads/crm_opportunities/mrp_calculation）添加 AuditEvent + AuditLogService::record_async 审计写入（best-effort 异步）；修复 borrow of moved value（提前 clone String 查询条件）；CI 12/12 全绿 |
| 446 | #626 | V15 P0-S11 报表染色域导出审计日志补齐（第 2 批）：5 文件 5 个 export 函数（report_engine/ar_reconciliation_pdf/sales_analysis/dye_recipe/dye_batch）添加 AuditEvent + AuditLogService::record_async 审计写入（best-effort 异步）；修复 report_engine_handler state.db borrow of moved value（service 改用 state.db.clone()）；CI 15/15 全绿。**P0-S11 全部完成** |
| 447 | #627 | V15 P0-S01 行级数据权限注入-销售域：为销售域 service 查询入口注入 DataScopeContext（all/dept/self 三级）。so/order_query list_orders/get_order_detail + customer_service list_customers/list_customers_with_filter/get_customer/get_customer_with_filter + sales_return_service list_returns/get_return 增加 data_scope 参数；handler 层提取 auth.to_data_scope_context() 传入；内部调用点传 None；customer/sales_order/sales_return 表无 department_id，Dept 退化为 Self；CI 15/15 全绿 |
| 448 | #628 | V15 P0-S01 行级数据权限注入-采购域：为采购域 service 查询入口注入 DataScopeContext。po/order list_orders/get_order（PurchaseOrderDto 新增 created_by 字段）+ supplier_service list_suppliers/get_supplier（无 department_id，Dept 退化为 Self）+ purchase_return_service list_returns/get_return（完整 Dept）增加 data_scope 参数；3 个 handler 传 Some(&ctx)；3 处 service 内部调用传 None；CI 15/15 全绿（13 success + 2 skipped） |
| 449 | #629 | V15 P0-S01 行级数据权限注入-生产域：production_order_service get_by_id/list 增加 data_scope 参数 + check_resource_owner IDOR 校验（无 department_id，Dept 退化为 Self）；production_recipe_service 4+1 个查询方法（大货处方 get_by_id/list/get_by_work_order + 加料处方 list_additions_by_recipe/get_by_id）增加 data_scope 参数，7 处内部调用传 None；2 个 handler 传 Some(&ctx)；MRP 域无 created_by 跳过；CI 15/15 全绿（13 success + 2 skipped）；修复 clippy baseline 96fb6e9 误删历史警告（too many arguments 8/7） |
| 450 | #630 | V15 P0-S01 行级数据权限注入-CRM 域：lead.rs list_leads/get_lead + opp.rs list_opportunities/get_opportunity + cust.rs get_customer_360/list_follow_ups 增加 data_scope 参数 + IDOR 校验；6 处内部调用传 None；7 个 handler 传 Some(&ctx)（crm_handler 6 方法 + crm_customer_handler 2 方法）+ 4 处业务操作传 None（crm_pool_handler 3 处公海池共享数据 + crm_assignment_handler 1 处线索分配）；CRM 域使用 owner_id（业务负责人，i32 必填）作为 owner_column，比 created_by（Option<i32>，crm_lead 未显式设置）更可靠；CI 15/15 全绿（13 success + 2 skipped） |
| 451 | #631 | V15 P0-S01 行级数据权限注入-财务域（finance_payment+invoice）：finance_payment_service find_by_id/list_payments + finance_invoice_service list_invoices/get_invoice 增加 data_scope 参数 + IDOR 校验；2 个 handler（finance_payment_handler + finance_invoice_handler）的 get_payment/list_payments/list_finance_invoices/get_finance_invoice 新增 auth 参数，传 Some(&ctx)；finance_payment + finance_invoice 表均无 department_id，Dept 退化为 Self；finance_payment.created_by 已显式 Set，finance_invoice.created_by 恒为 None；AP/AR 域留待后续批次；CI 15/15 全绿（13 success + 2 skipped） |
| 451b | #632 | V15 P0-S01 行级数据权限注入-财务域 AP 域：ap_payment_service get_by_id/get_list + ap_payment_request_service get_by_id/get_list 增加 data_scope 参数 + IDOR 校验/过滤；2 个 handler（ap_payment_handler + ap_payment_request_handler）的 list_payments/get_payment/list_requests/get_request 提取 data_scope_ctx 传 Some(&ctx)；ap_payment + ap_payment_request 表 created_by 均为 i32 必填，无 department_id，Dept 退化为 Self 使用 created_by；CI 15/15 全绿（12 success + 2 skipped + 1 queued 通知）；P0-S01 进度 21→23/104 |
| 451c | #633 | V15 P0-S01 行级数据权限注入-财务域 AR 域：ar_service list_payments/get_payment/list_verifications/get_verification 增加 data_scope 参数 + IDOR 校验/过滤；2 个 handler（ar_payment_handler + ar_verification_handler）的 4 个查询函数 _auth → auth 传 Some(&ctx)；ar_collection.created_by 是 i32 必填，ar_reconciliation.created_by 是 Option<i32>，均无 department_id，Dept 退化为 Self；报表类方法（原生 SQL）留待后续批次；CI 15/15 全绿（13 success + 2 skipped）；P0-S01 进度 23→25/104 |
| 452 | #634 | V15 P0-S01 行级数据权限注入-库存域（调整+预留子域）：inventory_adjustment_service list_adjustments/get_adjustment + inventory_reservation_service list_reservations 增加 data_scope 参数 + IDOR 校验/过滤；inventory_adjustment/reservation.created_by 均为 Option<i32>，无 department_id，Dept 退化 Self；handler 补 auth 参数传 Some(&ctx)，3 处 handler 内部调用 + 3 处 service 内部调用 + 3 处单测传 None；库存域其他子域（查询/调拨/盘点）因 inventory_stock 无 created_by 或 handler 缺 AuthContext 待后续；CI 15/15 全绿（13 success + 2 skipped）；P0-S01 进度 25→27/104 |

---

## V15 审计执行阶段（2026-07-16）

| 批次 | 日期 | 一句话总结 |
|------|------|-----------|
| 01-21 | 2026-07-16 | V15 全项目综合审计 21 批 195 维度全部完成，发现 732 个问题（104 P0 + 257 P1 + 248 P2 + 123 P3），汇总报告 v15-summary-2026-07-16.md 已生成，等待用户通知进入 V15 修复阶段 |

---

## V15 审计计划三轮升级（2026-07-15）

| 日期 | 一句话总结 |
|------|-----------|
| 2026-07-15 | V15 审计计划第三轮升级完成：①符合项目规则和个人规则声明 ②类八法律合规从 4 维度扩展到 8 维度（新增纺织行业法律/财税/环保/劳动合规 4 维度）③新增类十一大货批色业务规则专项 6 维度（剪大货样/客户批色/交货门禁/返工降级报废），V15 升级为 11 大类 68 维度最全面审计体系 |
| 2026-07-15 | V15 审计计划第四轮深化：类十色卡发放专项从 5 维度深化到 7 维度（新增 10.6 前端重构规范 + 10.7 DB 数据迁移脚本），10.1-10.5 全面深化为代码级实现规范（SQL/Model/DTO/Service/Handler/路由代码骨架 + 校验矩阵 + 权限矩阵 + cron + 单元测试 23 项），V15 升级为 11 大类 70 维度 |
| 2026-07-15 | V15 审计计划第五轮升级：新增类十二 RBAC 权限控制机制专项 8 维度（12.1 数据模型 role_permission/user_role 关联表 DDL + 12 角色层级 + 权限码命名规范/12.2 权限矩阵与最小权限原则/12.3 Axum 权限校验中间件 require_permission 代码 + 数据权限过滤 + 字段级权限/12.4 前端 RBAC 集成 路由守卫 + v-permission 指令 + 菜单动态加载/12.5 permission_audit_log 审计日志表 DDL + 保留期限/12.6 动态授权与委托 Redis 缓存 + pub/sub 热更新/12.7 行级 RLS + 字段级 + apply_data_scope Rust 代码/12.8 RBAC 安全审计 权限提升/IDOR/绕过/会话固定/TOCTOU 防护）；V15 升级为 12 大类 78 维度最终版，audit_assignment.md §2.3 同步更新（维度全景表 + 8 批子代理执行流程 + 修复队列新增 RBAC 权限修正） |
| 2026-07-15 | V15 审计计划第六轮升级：基于完整代码扫描（13 个后端 print/export handler + 25+ 个前端本地导出按钮）新增类十三打印导出审计与权限控制专项 10 维度（13.1 端点合理性审计 14 现有端点矩阵 + 7 缺失端点 + 11 前端本地导出/13.2 角色权限矩阵 14 角色 × 13 操作 + 禁止角色清单 + 19 个 print/export 权限码 SQL + method_to_action 升级/13.3 业务级审计补齐 OperationType::Print/Download 新增 + 10 handler 补齐清单 + audit_logs 表扩展 5 字段/13.4 敏感数据二级审批 export_approval_request 表 DDL + 水印代码 + 7 资源禁止规则/13.5 前端本地导出强制走后端 export.ts/print.ts 重构 + 25+ 页面改造清单/13.6 审计完整性 15 端点 × 8 字段矩阵 + P0/P1/P2 修复优先级/13.7 omni_audit 中间件语义增强 classify_operation 代码 + 表扩展/13.8 文件水印 4 格式规范 + build_xlsx_with_watermark 代码/13.9 性能并发 9 资源上限 + AtomicUsize 并发控制 + StreamBody 流式导出/13.10 合规审计 6 异常模式检测 + 每日 cron 审查 + 4 类保留期限 + 审计日志导出二次审计表）；V15 升级为 13 大类 88 维度最终版，audit_assignment.md §2.3 同步更新（维度全景表 + 9 批子代理执行流程 + 修复队列新增打印导出审计补齐） |
| 2026-07-15 | V15 审计计划第七轮升级：基于完整 RBAC 代码扫描（schema 001/014/025 + init_service.rs + auth_handler.rs + permission.rs + path_utils.rs + 前端路由和指令）新增类十四权限维度审计与角色合理性专项 12 维度（14.1 角色清单合理性审计 现有角色合理性矩阵 + 14 个缺失业务角色补齐清单 + 角色命名规范/14.2 权限分配矩阵审计 8 项问题矩阵 + 14×11 目标权限矩阵 + 权限过大/过小识别规则代码/14.3 职责分离 SoD 审计 8 项职责冲突矩阵 + role_conflict 表 DDL + 互斥校验 Rust 代码/14.4 权限-路由匹配审计 60+ 类缺失权限资源 + 权限码不匹配 + 模块前缀白名单缺口/14.5 is_system 滥用治理 build_with_permissions 修正代码 + SQL 修复脚本 manager/operator 取消 is_system=true/14.6 前后端权限边界一致性审计 4 项不一致场景 + 修复方案 A/B/14.7 业务角色权限矩阵设计审计 销售/采购/库存/生产/财务/其他 6 域完整权限矩阵/14.8 权限粒度审计 行级 apply_data_scope 代码 + 字段级权限 SQL/14.9 权限缓存与性能审计 缓存问题矩阵 + invalidate_user_permission_cache 代码 + 5min TTL + Redis pub/sub 热更新/14.10 权限审计日志与合规审查 permission_change_audit 表 DDL + 6 项异常检测规则 + 每周合规审查 cron/14.11 权限测试覆盖率审计 10 类测试缺口 + 30+ 单元测试清单/14.12 权限安全审计 权限提升/绕过/注入漏洞矩阵）；V15 升级为 14 大类 100 维度最终版，audit_assignment.md §2.3 同步更新（维度全景表 + 10 批子代理执行流程 + 修复队列新增权限维度修正），doto.md §三同步更新为 14 大类 100 维度 + 类十四专项提醒 |
| 2026-07-15 | V15 审计计划第八轮升级：基于完整业务主体代码扫描（supplier_service.rs + supplier_evaluation_service.rs + purchase_order.rs + ap_reconciliation.rs + sales_order.rs + sales_order_item.rs + order_workflow.rs + delivery.rs + sales_return_service.rs + customer_service.rs + customer_credit_limit.rs + customer_credit_evaluate.rs + event_bus.rs + inventory_finance_bridge_service.rs + business_trace.rs）新增类十五业务主体维度审计与数据流转专项 15 维度（15.1 供货商主数据完整性审计 suppliers 主表 + 7 张关联表 + schema/model 命名不一致 + migration 缺失 + 分类未落地 + 资质管理不完整/15.2 供货商业务闭环审计 8/12 完整 评估自动触发/账户余额/供货历史/价格清单导入缺失/15.3 供货商面料行业特性审计 supplier_type 区分染料/助剂/坯布合理 色卡能力/染色能力/印花能力字段缺失不合理/15.4 加工商维度审计 完全未实现重大功能缺口 含完整 4 表 DDL 设计 outsourcing_orders + outsourcing_receipts + processor_payments + suppliers 扩展/15.5 加工商业务流程闭环审计 0/8 打通 0% 外发/核算/收回/损耗/付款/进度/缸号关联/报表全部缺失/15.6 销售订单数据模型与状态机审计 主表+明细+8态状态机+报价单完整 销售合同缺明细行表不合理/15.7 销售业务流程闭环审计 12/12 完整 100% 报价→订单→发货→收款→退货闭环 + TOCTOU 防护 + 双单位换算 + 对称恢复 + BPM 补偿/15.8 销售面料行业特性审计 5/6 完整 83% 缸号校验/双单位/等级价差/纸管重量完整 按匹号发货部分可改进/15.9 客户主数据完整性审计 customers+信用+联系人+色卡价格+行业/质量标准完整 多地址/多银行表缺失可选/15.10 客户信用与应收管理审计 11/12 完整 评级自动触发 cron 缺失不合理/15.11 客户面料行业特性审计 6/8 完整 75% 特殊工艺要求缺失可选/15.12 跨模块数据流转审计 销售/采购/生产三链路全通 + 事件总线 21 事件/双后端/幂等/死信/panic 隔离合理 染色→质检→入库监听器仅日志无回写不合理/15.13 数据流转业务回写审计 库存财务桥接 7 种类型幂等完整 business_traces 表模型存在无写入不合理 DyeBatchCompleted/QualityInspectionCompleted 仅日志无回写不合理/15.14 数据流转报表与追溯审计 销售分析/AP 账龄/业财一体化/PDF/CSV/财务指标完整 离线 ETL 未实现可选 business_traces 写入缺失不合理/15.15 数据流转审计与异常检测审计 操作日志/omni_audit/事务内审计/死信审计/幂等审计完整 business_traces 写入缺失/主动异常检测引擎/异常告警缺失不合理）；V15 升级为 15 大类 115 维度最终版，audit_assignment.md §2.3 同步更新（维度全景表 + 11 批子代理执行流程 + 修复队列新增业务主体维度修正含加工商功能补齐），doto.md §三同步更新为 15 大类 115 维度 + 类十五专项提醒 |
| 2026-07-15 | V15 审计计划第九轮升级：基于后端完整模块扫描（services/ 130+ 文件 + handlers/ 140+ 文件 + models/ 180+ 文件 + middleware/ 17 个 + utils/ 34 个 + routes/ 19 个 + observability/ + websocket/ + search/ + cli/ + bin/ + config/ + database/）识别出后端完全未覆盖 19 个模块（AI 模块群 14 个 ai_process_optimization/ai_quality_prediction/ai/{detect,pred,rec,recipe_opt}/ai_extend_service/advanced/{analytics,decide,forecast,quality_pred,rec,recipe_opt,reorder} + oa_announcement/user_behavior/page_view/five_dimension/incoterms 5 个零散模块）+ 部分覆盖 54 个模块（observability/websocket/middleware/csp/timeout/slow_query/metrics/accounting_period/assist_accounting/ar_collection/ar_aging_analysis/financial_analysis/fund_management/budget_management/fixed_asset/inventory_transfer/stock_alert/capacity/scheduling/material_shortage/greige_fabric/piece_split/quality_issue/unqualified_product/color_price/crm_*/report_*/dashboard/bi_analysis/notification/email/business_trace/custom_order/after_sales/logistics/department 等）+ 前端完整模块扫描（views/ 85+ 模块 + components/ 17 个 + composables/ 36+ 文件 + store/ 6 个 + router/ + api/ 90+ 文件 + utils/ 7 个 + directives/ + i18n/ + locales/ + types/）识别出前端覆盖率 < 5% + 75+ 完全未覆盖 views + 17 未覆盖 components + 36+ 未覆盖 composables + 5 未覆盖 stores + 85+ 未覆盖 api 文件 + 20 个前端独有维度全部缺失（响应式设计/路由懒加载/状态管理/组件设计/composables/ECharts/WebSocket 客户端/前端性能/Vite 构建/前端测试/XSS 防护/敏感数据/WCAG 可访问性/错误边界/表单验证/i18n 深化/权限粒度/路由元信息/API 拦截器/主题样式），新增 9 个新类别共 75 维度：类十六 AI 模块审计专项 10 维度（16.1 AI 模型可解释性与透明度 explanation/confidence_score/factors 字段 + 模型版本 + ai_decision_log 表 + 人工干预/16.2 AI 数据安全与隐私 训练数据脱敏 + 推理数据最小化 + 中间结果加密 + 接口认证/16.3 AI 模型训练与推理正确性 数据集合理性 + 一致性 + 评估指标 + 漂移检测/16.4 AI 权限控制与访问审计 14 端点权限矩阵 + 权限码注册 + 数据权限 + 调用审计/16.5 AI 配方优化业务正确性 输入校验 + 输出合理性 + 历史回溯 + 化验室打样集成/16.6 AI 质量预测准确性 准确率 ≥80% + 特征完整性 + 误判成本 + 对账/16.7 AI 推荐业务合理性 diversity_score ≥0.3 + 冷启动 + 反馈闭环 + 业务约束/16.8 AI 补货决策合理性 数量合理性 + 时机判断 + 供应商推荐 + MRP 集成/16.9 AI 接口性能与资源消耗 P95 ≤2s + 内存 ≤1GB + 并发控制 + 缓存 TTL 5min/16.10 AI 测试覆盖率与监控 单测 ≥70% + E2E + Grafana 看板 + 告警）+ 类十七财务深化审计专项 8 维度（17.1 会计期间结账与跨期处理 状态机 open→closing→closed→reopened + 月结/年结 + 结账锁定/17.2 多维度辅助核算完整性 5 维度 + 主辅账平衡 + 报表穿透 + 数据完整性/17.3 应收催收流程与坏账处理 催收闭环 + 自动任务分配 + 坏账准备计提 + 核销二级审批/17.4 应收账龄分析准确性 自定义分段 + 期末快照 + 与总账对账 + 趋势图/17.5 财务分析模型合理性 4 类比率 + 杜邦分析 3 层 + 趋势分析 + 预警机制/17.6 资金管理与调拨流程 账户区分 + 二级审批 + 资金预测 + 大额额外验证/17.7 预算编制执行调整闭环 3 种方法 + 执行控制 + 调整审批 + 差异分析/17.8 固定资产折旧处置盘点 4 种折旧方法 + 自动计提 + 处置流程 + 盘点闭环）+ 类十八 CRM 全链路审计专项 5 维度（18.1 线索管理与转化漏斗 评分模型 + 漏斗报表 + 来源追踪 + 去重/18.2 商机阶段与赢率预测 阶段状态机 + 赢率自动计算 + 预测准确率 + 输单分析/18.3 客户池公海私海回收策略 公海私海规则 + 自动回收 + 规则配置 + 领取限制/18.4 CRM 数据权限与团队协作 数据权限 + 团队协作 + 临时共享 + 客户转移/18.5 CRM 与销售模块数据流转 线索→客户 + 商机→报价 + 报价→订单 + 一致性校验）+ 类十九报表 BI 与通知协同审计专项 8 维度（19.1 报表定义与模板管理 元数据 + 版本管理 + 参数校验 + 权限控制/19.2 报表订阅与定时推送 订阅权限 + 定时推送 + 失败重试 + 退订/19.3 BI 分析与多维钻取 多维分析 + 数据缓存 + 大数据性能 + 数据权限/19.4 仪表板数据卡片实时刷新 自定义卡片 + 实时刷新 + 权限 + 性能/19.5 通知中心多渠道去重 4 渠道 + 去重 + 已读未读 + 模板管理/19.6 邮件服务 SMTP 队列重试 SMTP 加密 + 异步队列 + 失败重试 + 附件安全/19.7 OA 公告与用户行为分析 公告权限 + 可见性 + 行为采集合规 + 数据脱敏/19.8 五维度分析与页面浏览统计 业务对齐 + 数据归集 + SPA 统计 + 数据保留）+ 类二十可观测性与运维审计专项 8 维度（20.1 可观测性 trace 链路完整性 trace_id + span_context + 采样策略 + 数据保留/20.2 metrics 指标体系与告警 7 类指标 + 告警规则 + 分级 + Grafana 看板/20.3 WebSocket 实时推送可靠性 连接管理 + ACK 机制 + 多实例广播 + 鉴权/20.4 故障转移主备切换回切 故障检测 + 主备切换 + 数据同步 + 人工回切/20.5 慢查询阈值告警优化 阈值配置 + 自动告警 + 优化追踪 + 报表/20.6 API 网关路由转发限流熔断 路由配置 + 限流 + 熔断 + 鉴权/20.7 系统版本与升级管理 版本号 + 灰度升级 + migration 版本 + 向后兼容/20.8 日志增强与系统日志完整性 分级 + 脱敏 + 结构化 + 归档）+ 类二十一胚布拆匹与质量处理审计专项 5 维度（21.1 胚布库存与采购管理 独立库存模型 + 采购流程 + 安全库存 + 批次追溯/21.2 胚布委托加工流转 委外流程 + 损耗核算 + 质量追溯 + 加工费核算/21.3 拆匹后缸号匹号继承规则 缸号继承 + 匹号生成 + 数量校验 + 历史追溯/21.4 质量问题 8D 处理流程 8D 完整性 + 根因分析 + 纠正预防 + 8D 月报/21.5 不合格品降级返工报废流程 分类 + 降级 + 返工 + 报废）+ 类二十二库存排程物料审计专项 6 维度（22.1 库存调拨跨库位跨缸号 流程闭环 + 跨库位 + 跨缸号 + 分级审批/22.2 库存告警安全库存补货策略 安全库存 + 3 种补货策略 + 通知 + 去重/22.3 物料短缺预警闭环 识别 + 分级 + 处理闭环 + 月报/22.4 自动排程算法合理性 算法 + 冲突检测 + 可视化 + 生产集成/22.5 产能规划与瓶颈识别 产能模型 + 负荷告警 + 瓶颈识别 + 月报/22.6 工作中心调度与排程集成 工作中心模型 + 调度规则 + 排程下发 + 异常重排）+ 类二十三组织定制物流审计专项 5 维度（23.1 组织架构部门管理 树形结构 + 权限关联 + 用户关联 + 变更审计/23.2 定制订单流程与质量管控 流程 + 专属质量标准 + 客户确认 + 追溯/23.3 售后管理与工单流转 4 类工单 + 流程闭环 + 原因分析 + 质量集成/23.4 物流运单跟踪与运费核算 运单管理 + 物流跟踪 + 运费核算 + 电子签收/23.5 国际贸易术语 incoterms 完整性 11 种术语 + 价格集成 + 责任划分 + 报表）+ 类二十四前端架构与体验审计专项 20 维度（24.1 前端响应式设计与移动端适配/24.2 路由懒加载与代码分割/24.3 Pinia 状态管理与持久化/24.4 组件设计与 Props/Emits 类型安全/24.5 composables 响应式与内存泄漏/24.6 ECharts 图表性能与无障碍/24.7 WebSocket 客户端连接重连心跳/24.8 前端性能与 bundle 体积/24.9 Vite 构建与 Tree Shaking/24.10 前端测试覆盖率与 mock fixtures/24.11 前端 XSS 防护与 CSP 策略/24.12 敏感数据存储与 token 安全/24.13 前端可访问性 WCAG 2.1 AA/24.14 错误边界与全局错误处理/24.15 表单验证与异步校验/24.16 i18n 国际化深化与复数 RTL/24.17 前端权限粒度按钮字段行级/24.18 路由元信息与动态路由/24.19 API 请求拦截器与超时重试/24.20 主题样式与暗黑模式）；V15 升级为 24 大类 190 维度最终版，audit_assignment.md §2.3 同步更新（标题改为第九轮升级版 + 触发前置新增 9 项 + 核心目标改为 24 大类 190 维度 + 关键业务规则新增第 8 项全面维度补齐 + 维度全景表新增 9 行 + 合计改为 190 + 执行流程从 11 批改为 20 批 + 修复队列新增 9 类补齐），doto.md §三同步更新为 24 大类 190 维度 + 类十六~类二十四专项提醒 + 20 批子代理执行方式 + 触发前置新增 9 项 |

---

## v14 面料行业特性复审修复阶段（批次 416+）

| 批次 | PR | 一句话总结 |
|------|-----|-----------|
| 432 | #610 | v14 复审 P1 缸号全生命周期状态机完善——依据 §12.7 缸号状态机 + §3.2 缸号全生命周期追踪；实现 4 表（dye_batch_lifecycle_log 生命周期日志 + dye_batch_state_rule 状态流转规则 28 条预置 + dye_batch_rework 回修记录 + dye_batch_operation 操作记录，4 外键+多索引）+ 4 SeaORM 模型 + 5 组状态常量（dye_batch_lifecycle_status 14 种状态：pending_schedule/scheduled/preparing/dyeing/washing/fixing/dehydrating/drying/inspecting/stored/shipped/cancelled/terminated/rework + dye_batch_transition_code 13 种流转代码 + dye_batch_rework_type 4 种回修类型 + dye_batch_rework_status 5 种回修单状态 + dye_batch_operation_type 6 种操作类型 merge/split/priority_adjust/batch_change/schedule_change/terminate）+ 1 Service（dye_batch_state_machine_service.rs ~1525 行，4 Service：DyeBatchLifecycleLogService/DyeBatchStateRuleService/DyeBatchReworkService/DyeBatchOperationService + 11 个纯函数 + 25 单元测试）+ 26 Handler + 4 组路由（/dye-batch-lifecycle-logs、/dye-batch-state-rules、/dye-batch-reworks、/dye-batch-operations）；终态保护：shipped/cancelled/terminated 不可流转；回修 rework→dyeing；CI 修复 rustdoc `doc list item without indentation` 警告（4 个模型文件 + service + handler + routes 共 7 个文件的 `/// - ` 和 `/// + ` 列表标记改为 plain paragraph text），CI 全绿；已合并 PR #610 到 main (d4fdf5e6) |
| 431 | #609 | v14 复审 P2 多业务模式支持——依据 §6 业务模式（坯布销售/染色加工/印花加工/来料加工/贸易模式）；实现业务模式配置 + 单据流程适配 + 成本核算适配，已合并 PR #609 到 main |
| 430 | #608 | v14 复审 P2 委托加工物资贯通——依据 §5.4 委托加工物资核算三步分录 + §5.5 委外织布场景 + §5.7 损耗率标准 + §6.5 委托加工模式；实现 4 表（outsourcing_order 委外订单主表含三步分录凭证号+损耗分类+标准损耗率 + outsourcing_order_item 发料明细按面料四维标识追溯 + outsourcing_receipt 收回入库单含损耗分类与质量等级 + outsourcing_voucher 会计分录凭证 issue/fee/receipt/loss 四类，10 外键+25 索引+3 唯一约束）+ 4 SeaORM 模型 + 5 组状态常量（outsourcing_order_type/outsourcing_order_status/outsourcing_loss_type/outsourcing_receipt_status/outsourcing_voucher_type）+ 1 Service（outsourcing_service.rs ~1790 行，4 Service：OutsourcingOrderService/OutsourcingOrderItemService/OutsourcingReceiptService/OutsourcingVoucherService + 10 个纯函数：compute_loss_rate/compute_total_cost/compute_unit_cost/compute_standard_loss_rate/classify_loss/compute_abnormal_loss_amount/validate_order_type/validate_order_status/validate_loss_type/validate_voucher_type + 21 单元测试）+ 25 Handler + 26 路由（3 前缀组 /outsourcing-orders、/outsourcing-receipts、/outsourcing-vouchers）；三步分录：发料(借 委托加工物资/贷 自制半成品-胚布)→加工费(借 委托加工物资+应交税费-进项税额/贷 银行存款)→入库(借 库存商品-成品布/贷 委托加工物资)；状态机：draft→issued→processing→received→settled→closed→cancelled；损耗规则：正常损耗摊入成本，非正常损耗计入营业外支出；标准损耗率 dyeing=0.05/weaving=0.035/printing=0.05/finishing=0.03；已合并 PR #608 到 main |
| 429 | 待开 PR | v14 复审 P1 染化料主数据完善——依据 §4.3 染化料管理 + §11.4 染化料主数据管理 + §4.5 采购与坯布管理；实现 4 表（chemical_master 染化料主数据含 GHS 危险化学品/MSDS 安全数据表/染料色卡指数/助剂有效成分/安全库存/包装规格 + chemical_category 自引用分类树染料/助剂/化学品 + chemical_lot 批号管理含检验状态/存储区危险品仓/保质期 + chemical_requisition 领用单含生产/化验/研发类型 + 6 外键+多索引）+ 4 SeaORM 模型（chemical_master/chemical_category/chemical_lot/chemical_requisition 含 Relation 关联）+ 6 组状态常量（chemical_type/chemical_status/chemical_inspection_status/chemical_lot_status/chemical_requisition_type/chemical_requisition_status）+ 1 Service（chemical_service.rs ~1300 行，4 Service：ChemicalMasterService/ChemicalCategoryService/ChemicalLotService/ChemicalRequisitionService + 8 个纯函数：compute_remaining_shelf_life/compute_total_cost/validate_chemical_type/validate_inspection_status/validate_lot_status/validate_requisition_type/validate_requisition_status/check_low_stock + 17 单元测试）+ 25 Handler + 31 路由（4 前缀组 /chemicals、/chemical-categories、/chemical-lots、/chemical-requisitions）；状态机：批次 pending→passed/failed/quarantine + active→consumed/expired/scrapped；领用单 draft→approved→issued→partial_returned→closed/cancelled；遵循 energy_service.rs/energy_handler.rs/production.rs 模式；遵守 CI 规范（.one() 返回 Option 不是 Result、ActiveValue 不支持 unwrap_or_default、未使用变量加 _ 前缀、Condition::any() OR 关键字搜索、路由排序避免 axum 0.7 Overlapping method route panic），已推送 feat/batch429-chemical-master 分支等待 CI 验证 |
| 428 | #606 | v14 复审 P2 能耗管理贯通——基于真实业务调研（WebSearch 验证能源类型：水/电/蒸汽/天然气/压缩空气占总成本 35%+；采集方式：IoT 设备实时采集智能电表/蒸汽流量计/水质监测仪+手工录入；分摊方式：按工艺路线归集到缸号/工序/订单，按工时×功率系数分摊；基准管理：每道工序预设理论能耗基准超基准预警；月末分摊：自动核算每缸布水电汽实际消耗与标准成本生成 cost_collection 记录）；实现 4 表（energy_meter 能源计量设备 + energy_consumption_record 能耗记录 + energy_allocation_rule 分摊规则 + energy_allocation_record 分摊记录，4 外键+35 索引+4 唯一约束）+ 4 SeaORM 模型 + 5 组状态常量（能源类型/设备状态/采集方式/分摊基准/记录状态）+ 1 Service（energy_service.rs ~1300 行，4 Service + 8 个纯函数 + 18 单元测试 + 月末分摊 monthly_allocation_by_duration 按工时比例分摊）+ 27 Handler + 31 路由；CI 修复 5 类错误（①E0308 if let Ok→if let Some 因 .one() 返回 Option ②E0599 4 处 ActiveValue unwrap_or_default 改 model.into() 前记录 original_* 值 ③E0432 handler 导入 ConsumptionService/AllocationRecordService 改 Energy* 对应名 ④unused variable confirmed_by→_confirmed_by ⑤clippy::unnecessary_lazy_evaluations + type_complexity HashMap 元组 key → 结构体 DurationGroupKey + WorkshopEnergySummary），CI 全绿 |
| 427 | #605 | v14 复审 P1 产量工资核算贯通——基于真实业务调研（WebSearch 验证工序流转扫码→工价方案定义→工资计算→班组汇总→进入财务工资核算；三维度产量统计：工序产量+设备产量+工人产量工资；等级系数 A 级全额/B 级 8 折/C 级不计；按缸号计件工资核算）；实现 3 表（process_wage_rate 工序工价 + wage_record 工资记录 + wage_record_detail 工资明细）+ 3 SeaORM 模型 + 1 Service（wage_service.rs ~1500 行，3 Service：WageRateService/WageRecordService/WageCalculationService + 19 Handler + 21 路由），CI 修复 5 类错误，CI 全绿 |
| 426 | #604 | v14 复审 P1 验布打卷流程贯通——基于真实业务调研（WebSearch 验证验布机对接码表/电子称→疵点采集→生成验布报告→卷唛标签打印→PDA 扫码卷唛条码→自动入库；四分制 AATCC/ASTM D5430 ≤3寸=1分/3-6寸=2分/6-9寸=3分/>9寸=4分/破洞连续=4分/每百平方码≤40=首级；十分制梭织布经向 1寸下=1/1-5寸=3/5-10寸=5/10-36寸=10 纬向 半门幅以上=10/破洞=10/总扣分<总码数=首级；打卷入库生成匹号 {dye_lot_no}-{seq:03} 唯一校验）；实现 2 表（fabric_inspection_record 验布记录 + fabric_defect_record 疵点明细，2 外键+7 索引）+ 扩展 inventory_piece（inspection_id + piece_seq 字段）+ 2 SeaORM 模型 + 1 Service（fabric_inspection_service.rs ~650 行，FabricInspectionService/FabricDefectService + 评分计算纯函数 + 12 单元测试）+ 状态机 pending→inspecting→graded→rolled→closed + 13 Handler + 14 路由 + status.rs 3 组常量；CI 修复 DateTime<Utc> vs DateTime<FixedOffset> 类型不匹配（inventory_piece 用 chrono::Utc::now()），CI 全绿 |
| 425 | #603 | v14 复审 P1 流转卡条码与车间工序流转贯通——基于真实业务调研（WebSearch 验证流转卡=缸卡一缸一卡承载缸号/订单/染整要求/工序路线/条码；扫码场景：白坯出库/染色进度/称料/工序流转/成品入库/发货；工序路线后台自定义前处理→染色→印花→后整理→验布；缸号状态机 pending→scheduled→preparing→dyeing→dyed→inspecting→completed→shipped/terminated，验布中可回 DYEING 实现回修订单重新进缸）；实现 4 表（process_route/production_flow_card/process_step_record/process_quality_feedback，9 外键+19 索引+5 条默认工序路线）+ 4 SeaORM 模型 + 1 Service（flow_card_service.rs ~1270 行，4 Service：ProcessRouteService/FlowCardService/StepRecordService/QualityFeedbackService + 7 单元测试）+ 23 Handler + 28 路由；CI 修复 2 类错误（①production_recipe_handler import 遗漏 424 遗留 bug ②ActiveValue<Option<T>> 不支持 unwrap_or_default/unwrap_or 3 处改用 model/req 在 into() 前判断），CI 全绿 |
| 424 | #601 | v14 复审 P1 大货处方与加料处方流程贯通——基于真实业务调研（WebSearch 验证大货处方单=染色配料单/扫描流转卡条码/同一工单号只能开一张大货处方单/追加物料须开加料处方单/审核后自动建立生产领用单据；用量计算=浓度%×布重×浴比/100×加成系数；浴比解析支持 1:8/1：8（全角冒号）/1/8 三种格式）；实现 2 表（production_recipe + production_recipe_addition，7 外键+13 索引）+ 2 SeaORM 模型 + 1 Service（41KB，ProductionRecipeService 11 方法 + ProductionRecipeAdditionService 6 方法 + 12 单元测试）+ 15 Handler + 15 路由，CI 全绿 |
| 423B | #600 | v14 复审 P1 化验室打样流程贯通——基于真实业务调研（WebSearch 验证 ABCD=打样版数 4 版供客户选择、OK 样=客户从多版选 1 版、复样=车间半制品布+生产染化料模拟大生产色差 4-5 级方可投产、染色技术卡=复样通过后研发组长开卡含配方表+核可样+复色样）；实现 3 表（lab_dip_request/lab_dip_sample/lab_dip_resample）+ 3 模型 + 3 Service（900 行，状态机 pending→sampling→submitted→approved/rejected→completed + ABCD 多版样管理 + 复样判定 + 染色技术卡开具）+ 21 Handler + 21 路由；CI 修复 3 轮（①Text→String 8 处 ②FromJsonQueryResult 显式导入 ③serde::{Serialize,Deserialize} 显式导入 57 连锁错误根因 + manual_range_contains clippy 3 处），CI 全绿 |
| 423A | #599 | v14 复审 P1 第四批 A：染色配方 schema 修复 + Service 抽象层——迁移 036 补齐 dye_recipe 表 18 个缺失字段（recipe_no NOT NULL+回填 LEGACY-/color_no/formula/temperature/time_minutes/status/is_deleted/color_name/fabric_type/dye_type/chemical_formula/ph_value/liquor_ratio/auxiliaries/version/parent_recipe_id/approved_by/approved_at/remarks）+ 5 索引+2 外键约束；status.rs 新增 dye_recipe 状态常量（草稿/已审核/已停用）；新增 DyeRecipeService 抽象层（CRUD+状态流转校验 DRAFT→APPROVED/DISABLED+版本管理 create_new_version 版本树+色号/版本查询+generate_recipe_no）；handler 重构调用 service（11 个路由保持向后兼容，submit/export 保留原逻辑待 423B）；CI 修复 E0505 借用冲突（as_deref 改 clone）+ E0308 ActiveValue 类型不匹配 × 13（13 个字段补 Set 包装），CI 全绿 |
| 422 | #598 | v14 复审 P1 第三批（基于面料行业真实业务调研文档 §5.2 按缸号实际成本法 + §5.6 月末成本单价）：T-P1-6 按缸号核算成本打通（cost_collection_service CreateCostCollectionRequest/UpdateCostCollectionRequest 新增 dye_lot_no 字段 + create/update 写入 + handler DTO 同步）+ T-P1-7 染色完成成本结转事件监听器（新增 dye_batch_cost_bridge_service.rs 独立监听 DyeBatchCompleted 事件 + AssertUnwindSafe panic 隔离 + static Mutex 保存 JoinHandle 供 shutdown abort + 创建 cost_collection 草稿关联 batch_no/color_no）；CI 修复 production_order_service.rs 第 603 行构造 CreateCostCollectionRequest 缺失 dye_lot_no 字段（commit e2d04123）；同步深度补充面料行业真实业务调研文档第十一至十三章（化验室打样 5 步闭环/大货处方与加料处方/流转卡条码/车间工序流转/验布打卷/产量工资/能耗管理/缸号状态机 + 批次 423-432 共 10 批规划） |
| 421 | #597 | v14 复审 P1 面料行业特性首批修复（基于面料行业真实业务调研文档）：T-P1-4 质检 A/B/C 级分级判定（determine_quality_grade 合格率>=95% A 级/80-95% B 级让步接收降级销售/<80% C 级返工报废 + validate_handling_method_by_grade 等级与处理方式匹配校验 + CreateInspectionRecordRequest 新增 grade/color_no/dye_lot_no 字段 + process_unqualified 强制校验 + ProcessUnqualifiedRequest 新增 handling_result）+ T-P1-5 缸号同订单校验（validate_dye_lot_consistency 同一 product_id 必须使用相同 dye_lot_no + ShipOrderItemRequest 新增 color_no/dye_lot_no + ship_order 事务前校验）；迁移 035 为 quality_inspection_records/unqualified_products 添加 grade/color_no/dye_lot_no/handling_result 字段+索引；模型同步 2 文件；17 个单元测试（质检分级 9 个+缸号校验 8 个+build_ship_item 夹具）；CI 修复 Decimal::new 非 const fn 改为函数返回（grade_a_threshold/grade_b_threshold），CI 全绿 |
| 420 | #596 | v14 复审 P1 第一批事件贯通修复：T-P1-1 调拨 ship_transfer/receive_transfer 发布 InventoryTransactionCreated 事件（事务内收集+commit 后发布）+ T-P1-2 染色完成 complete_dye_batch 发布 DyeBatchCompleted 事件 + T-P1-3 新增 DyeBatchCompleted/QualityInspectionCompleted 事件变体（EventPayload 三段同步+event_type_name 映射）+ G-P1-3 主监听器 _ => {} 改为显式分支+warn 日志；CI 修复 pending_events 借用已移动值（先记录 events_count 再消费 Vec），CI 全绿；同步完成面料行业真实业务调研文档（.monkeycode/docs/research/fabric-industry-research.md，覆盖基础信息/染整工艺/ERP 模块/成本核算/业务模式/计量换算/项目映射/术语对照） |
| 419 | #595 | v14 复审 P0 第四批：生产订单+色卡借出补全缸号——迁移 034 为 3 个表（production_orders/inventory_piece/color_card_borrow_records）添加面料行业追溯字段 + 索引；7 个 Rust 文件修改修复 4 个 P0 问题（F-P0-1 production_order 添加 color_no/dye_lot_no/batch_no 字段 + F-P0-2 inventory_piece 添加 color_no/dye_lot_no + T-P0-3 color_card_borrow_record 添加 dye_lot_no + T-P0-5 销售退货 stock_map 改为四维索引按缸号退货入库），CI 修复 color_card_borrow_service ActiveModel 缺失 dye_lot_no 字段，CI 全绿 |
| 418 | #594 | v14 复审 P0 第三批：数据流转硬编码修复——5 个文件修复 5 个 P0 问题（D-P0-4 采购入库 DEFAULT 硬编码改为从采购订单明细获取真实值 + D-P0-5 销售发货 reduce_inventory 返回库存 color_no/dye_lot_no + D-P0-6 销售退货 dye_lot_no 误用 batch_no 修复 + G-P0-1 quantity_kg 调用 DualUnitConverter 双单位换算 + G-P0-2 凭证 unwrap_or_default 添加 warn 日志），CI 全绿 |
| 417 | #593 | v14 复审 P0 第二批：业务单据明细补全缸号字段——迁移 033 为 4 个表（sales_return_item/purchase_return_item/inventory_transfer_items/inventory_count_items）添加 color_no/dye_lot_no/batch_no 字段 + 索引；6 个 Rust 模型同步（sales_delivery_item 添加 dye_lot_id/dye_lot_no，purchase_order_item 添加 color_code/lot_no/batch_no 匹配 SQL 旧命名）；7 个 service 文件 ActiveModel 构造点同步更新（使用 NotSet 让 DB 默认值处理），CI 全绿 |
| 416 | #592 | v14 复审 P0 第一批：面料行业核心数据模型唯一约束补全——迁移 032 添加 product_colors UNIQUE(product_id, color_no) + inventory_stocks 四维联合唯一索引（warehouse+product+color_no+batch_no+COALESCE(dye_lot_no,'')）+ inventory_piece piece_no 改为 (dye_lot_id, piece_no) 联合唯一 + 补齐 DB 缺失字段；Rust 模型同步：inventory_piece.rs 添加 dye_lot_id（NOT NULL 关键修复）+ 12 个 SQL 表字段 + DyeLot 关联，dye_lot_mapping.rs 替换错误字段为 15 个正确字段 + Supplier/BatchDyeLot 关联，piece_split_handler.rs ActiveModel 构造同步更新，CI 全绿 |

---

## v13 复审 + 业务/财务/运行逻辑闭环修复阶段（批次 356+）

| 批次 | PR | 一句话总结 |
|------|-----|-----------|
| 415 | #591 | 遗留技术债务清理：修复 baseline 吞掉的 7 个编译错误（decs! 宏 FromStr 导入 4 处 + CustomOrderStatus FromStr→parse().ok() + event_kafka match 分支补全 + search_api SearchClient trait 导入 + customer_credit_limit Arc 导入）+ email_service needless_borrow 警告修复 + 删除格式不合规的 .clippy-baseline.txt（CI bootstrap 重建），CI 全绿 |
| 414 | #590 | CreditRatingRequest.credit_limit 语义模糊修复：Decimal → Option<Decimal>，service 层区分 None（保持原值）与 Some(v)（显式设置含 Some(0)），新增 validate_credit_limit_range（允许 0）+ 5 个单元测试，移除 TODO 注释，CI 全绿 |
| 413 | #589 | 事件+MRP+邮件 too_many_arguments 清理：5 个 service 方法引入 DTO 参数对象（NotificationPayload/MrpExplodeQuery/MrpCalculationQuery/TencentSignParams/UpdateApiKeyPayload），7参数→1参数，7 调用点同步更新，修复 clippy needless_borrow/needless_reference 警告，更新 baseline 纳入 119 条既有 dead_code 警告，CI 全绿 |
| 412 | #588 | 库存+产品 too_many_arguments 清理：inventory_stock_query::get_inventory_summary 新增 InventorySummaryQuery 参数对象（7参数→1参数）+ product_service::create_product_color 复用 CreateProductColorInput（7参数→2参数），4 文件调用点同步更新，纯重构 CI 全绿 |
| 411 | #587 | AP 模块 too_many_arguments 清理：4 个 service 方法引入 DTO 参数对象（ApInvoiceListQuery/ApPaymentListQuery/ApPaymentRequestListQuery/CreatePaymentInput），7参数→1参数，handler 调用点同步更新，纯重构 CI 全绿 |
| 410 | #586 | E2E SyntaxError 修复：3 个 e2e 文件 import('@playwright/test').Type → import { type Page } 标准写法 + color-card.spec.ts page.keyboard().press() → page.keyboard.press() API 误用修复 + playwright.config.ts /// → // 注释修复，根因为 Playwright 1.40.0 转译器不兼容 import type expression 语法，不升级版本最小化变更，CI 全绿 |
| 409 | #585 | P2-8 service 补测：6 个无测试核心 service 补充约 45 个单元测试（color_card_borrow BorrowStatus 状态机 + inventory_stock_query 7 级告警 + ar_invoice derive_paid_status 提取 + event_notification build_inventory_alert 提取 + customer_credit clamp_page 提取 + inventory_stock_txn 参数对象构造），CI 全绿 |
| 408 | #583 | FE-P2-6 大列表虚拟化：5 个 el-table 列表迁移到 V2Table（ApiLogTab/BpmApCompletedTbl/BpmApPendingTbl/LgsTbl/ScTbl）+ 规则 00 CI 类型错误修复（lgsFmts TagType '' → 'primary' 适配 Element Plus 新版 ElTag.type），ScTbl v-permission 改 can() 函数，CI 全绿 |
| 407 | #582 | v14 安全+数据完整性+业务正确性修复：9 handler 15 处（auth_handler 登录锁定 DB 错误传播+权限查询 fail-secure + api_gateway_handler 权限序列化错误传播 2 处 + dye_recipe_handler 配方辅料反序列化校验+创建回查错误传播+更新辅料校验 + dye_batch_handler 创建回查错误传播 + report_engine_handler filters_json 解析失败返回验证错误 2 处 + sales_order_handler warehouse_id 缺失校验 + barcode_scanner_handler order_id 缺失校验 + webhook_integration_handler 序列化错误传播 + customer_credit_handler credit_limit 技术债务标注）+ 4 处 redundant closure clippy 警告修复，CI 全绿 |
| 406 | #581 | v14 低风险修复：6 个 handler 文件 serde_json::to_value(x).unwrap_or_default() 序列化吞错改为 map_err(AppError::from).collect::<Result<Vec<_>, _>>()? 错误传播（ap_payment_request/inventory_stock/notification/purchase_order/purchase_receipt/quality_standard）+ 删除批次 402 错误创建的 1 行 clippy baseline 文件，CI bootstrap 模式自动重建 180 行完整基线，CI 全绿 |
| 405 | #580 | v14 低风险修复：CRUD 业务消息常量化第二批 - 5 handler 8 处硬编码替换（crm_handler 2 处 DELETE_OK + budget_management_handler 3 处 DELETE_OK/APPROVE_OK/EXECUTE_OK + webhook_handler 1 处 + bpm_definition_handler 1 处 + production_order_handler 1 处 APPROVE_OK），CI 全绿 |
| 404 | #579 | v14 低风险修复：supplier_service/bpm_service LazyLock<Regex> expect 改为 LazyLock<Option<Regex>> 优雅降级 + 新建 utils/messages.rs biz_msg 常量模块（CREATE_OK/UPDATE_OK/DELETE_OK/APPROVE_OK/EXECUTE_OK/OPERATE_OK）+ crud_macro.rs 6 处硬编码替换为常量引用 + fund_management_handler/missing_handlers 4 处硬编码替换，CI 全绿 |
| 403 | #578 | v14 低风险修复：omni_audit_handler 审计日志 DB 字段 unwrap_or_default 吞错改用 Option<T> 读取区分 NULL 与错误（nullable→默认值，错误→传播 500）+ import_export_service 产品导入价格转换失败静默写 0 改为返回验证错误 + audit_log_service/omni_audit_service shutdown 路径 Mutex::lock().unwrap() 改用 unwrap_or_else 安全访问 poisoned lock，CI 全绿 |
| 402 | #576 | clippy baseline 最后一条 `needless_reference` 警告清零：修复 [webhook_handler.rs#L308](file:///workspace/backend/src/handlers/webhook_handler.rs#L308) 测试代码中 `&*LazyLock` 模式（改为 `&LazyLock` + `addr_of!(**limiter)`）；11 个 `#[allow(clippy::too_many_arguments)]` 标注为批次 328（v10 复审 P3，PR #500）历史添加，本批次未新增；**技术债务**：本批次错误创建了仅 1 行的 baseline 文件（内容为 `warning: this expression creates a reference...`），导致后续 CI strict 模式误报 117 个新警告，该错误在批次 406 删除错误 baseline 文件后由 CI bootstrap 模式自动重建 180 行完整基线修复，CI 全绿 |
| 398 | #572 | 配置合规性修复：AppSettings::new() 启动时同步 config.yaml env 字段到 APP_ENV（消除 is_production() 部署陷阱）+ .env.example 移除中文占位符密码和 GRPC 残留变量 + deploy-latest.sh 移除 grpc 死配置段 + config.yaml.example 更新 env 字段注释 + clippy baseline 文件格式修复（118 条纯摘要行替换 274 行混合内容，修复 116 条误报新警告），CI 全绿 |
| 400 | #573 | v14 低风险修复：删除 InventoryStockService::record_transaction 非事务版本（已被 record_transaction_txn 取代）+ 接入 AccountSubjectService::refresh_balance handler + 路由 + 接入 ColorCardBorrowService::cancel_borrow handler + 路由 + DTO + batch_trace_log.rs 警告抑制收窄，CI 全绿 |
| 401 | #575 | v14 低风险修复：deploy-latest.sh 新增密钥自动生成逻辑（JWT/COOKIE/WEBHOOK/AUDIT 四个密钥）+ deploy.sh/deploy-latest.sh 密钥生成从 hex 改为 base64 提升熵比 + .env.example/backend/.env.example 更新生成建议 + backend/.clippy-baseline.txt 重建为 2 条纯摘要行（仅剩余 2 个 clippy 警告），CI 全绿 |
| 397 | #571 | v14 低风险修复首批：占位符/Mock 存根调研确认 21 项已清零（历史批次 290-308 修复）+ 4 处 unwrap_or_default 安全修复（omni_audit body 读取失败 warn 日志 + audit_enhanced_handler created_at 改 Option<String> + data_permission_handler 序列化失败 fail-fast），阶段 8 启动 |
| 396 | #570 | baseline 警告清零收官：移除 .clippy.toml disallowed-methods 错误配置（println/eprintln 是宏非方法）+ process_state_machine.rs inherent from_str 改为标准 FromStr trait + 删除 purchase_delivery_calculator.rs AvgLeadTimeResult 死代码 + unwrap_safe.rs 移除多余 use super::* + auth.rs/webhook_service.rs 修复 needless_borrow，baseline 213/213 ✅ 全部清零，阶段 7 完成 |
| 395 | #568+#569 | baseline 自动刷新机制：CI clippy job 添加 main 分支自动刷新步骤（FIXED_COUNT>0 且 NEW_COUNT=0 时用当前警告替换 baseline），修复 shallow clone 下 git log→git ls-files 追踪检查，baseline 从 1465 行缩减到 310 行（摘要 213→7 条，移除 206 条已修复警告），阶段 7 baseline 清零首批完成 |
| 394 | #567 | 测试覆盖补测：data_permission_handler（0→6 SQL 注入防御）+ print_handler（0→5 内置模板）+ system_update_handler（0→6 ZIP 头校验+DTO）+ color_card/error_map（0→6 错误映射 14 变体），共 23 个新测试，阶段 6 测试覆盖补测全部完成（批次 392-394 共 65 个新测试） |
| 393 | #566 | 测试覆盖补测：inventory_stock_service（0→6）+ voucher_service（29→33）+ ar_service（0→6）+ ap_invoice_service（2→10），共 24 个新测试，覆盖双计量换算/状态机门/账龄分桶/五维ID拼接/贪心匹配等纯函数复现场景 |
| 392 | #565 | 测试覆盖补测：user_service 新增 8 测试（原无测试）+ auth_service 补测 4 异步密码函数 + po/order 补测 6 状态校验门（update/delete/close），共 18 个新测试，遵循规则 6 mock 数据抽取到夹具函数 |
| 391 | #564 | useTableApi 接入：AdjustmentListTab + TransferListTab 从手写分页模板代码统一接入 useTableApi（1-based），stats 保留原语义改用 watch data 自动更新，defineExpose refresh 保持父组件接口（阶段 5 useTableApi 接入全部完成） |
| 390 | #563 | useTableApi 接入：assistAccounting + barcodeScanner 修复 0-based 分页 bug（原 page-1 与后端 1-based 约定不一致），统一接入 useTableApi 由 setup 自动加载+watch page/pageSize 触发，移除手动 pagination ref 与 loadXxx 函数 |
| 389 | #562 | FE-P2-3 前端 i18n 覆盖率提升（MainLayout/Login/Dashboard + zh-CN/en-US 新增 113 key）+ P2-2 后端日志规范（user_service 4 处审计日志 + ar_service 11 处状态门 warn! 日志）+ P2-3 配置项完善（config.yaml.example 移除 9 个无效字段 + .env.example 补充 4 个环境变量占位行） |
| 388 | #561 | FE-P2-1 前端 unknown 类型细化（bpm/api-response/trading）+ FE-P2-2 组件 props 泛型强化（BatchActions/ProcessFlow）+ P2-1 后端错误处理统一（customer/inventory_stock/voucher handler） |
| 387 | #560 | F-P2-2 报表穿透追溯（drill_down API）+ F-P2-4 AR/AP 对账单确认生成凭证（F-P2-1/F-P2-3 待后续批次） |
| 386 | #559 | B-P2-4 MrpEngineService 接入销售审批+生产创建联动 + B-P2-5 CapacityService 接入排产产能校验 + B-P2-6 已在批次 356 修复 |
| 385 | #558 | B-P2-1 移除 AR/AP 事件监听器冗余 mark_as_paid 调用 + B-P2-2/B-P2-3 调研确认无需修复 |
| 384 | #557 | B-P1-3 客户/供应商主数据变更事件 + B-P1-7 事件重试死信队列 + F-P1-1 期末结转逻辑 |
| 383 | #556 | 部署修复：docker-compose.yml + deploy-backend.sh 补全 WEBHOOK_SECRET 部署模板 |
| 382 | #555 | F-P0-6 销售→应收链路 + F-P0-7 采购→应付链路（财务场景 P0 8/8 完成） |
| 381 | #554 | F-P0-3 销售出库收入凭证 + F-P0-4 AR 收款凭证 + F-P0-5 AP 付款凭证 + F-P0-8 AR/AP 核销凭证 + 3 项 dead_code 抑制 |
| 380 | #553 | L-32 AuditLogService mpsc channel 重构 + config.yaml.example 补全 webhook_secret（运行逻辑环 P3 26/26 完成） |
| 379 | #552 | L-37+L-39+L-40+L-41+L-44 silent default 消除（main.rs/telemetry.rs/cli/.env.example） |
| 378 | #550 | L-16 CSRF 测试 expect 消除 + L-24 InitTaskStatus 终态文档 |
| 377 | #549 | L-17+L-18+L-19+L-20 测试 let _ = result 吞错修复（7 文件 12 处） |
| 376 | #548 | L-12+L-13+L-14+L-15 expect 消除（email/hash_password/date_utils/timeout） |
| 375 | #547 | L-5+L-7+L-8+L-9+L-10 吞错清理（5 文件 7 处，规则 10 记忆整理同步） |
| 374 | #546 | L-26 5 个后台定时任务缺 cancellation token（运行逻辑环 P1+P2 全部清零） |
| 373 | #545 | L-27+L-28+L-29 事件总线 spawn 句柄丢失（event_bus + inventory_finance_bridge） |
| 372 | #544 | L-30 OmniAudit spawn 句柄丢失（运行逻辑环 P2 14 项全部清零） |
| 371 | #543 | L-42+L-31 silent default + WebSocket 句柄泄漏 |
| 370 | #542 | L-36+L-38+L-43 配置项 silent default（auth/slow_query/.env.example） |
| 369 | #541 | L-2 升级脚本吞错 + L-3 备份脚本吞错 + L-23 DyeBatchStatus 缺异常态 |
| 368 | #540 | L-4 回滚吞错 + L-6 事件发送吞错 + L-22 BorrowStatus 缺取消态 |
| 367 | #539 | L-1 CLI 吞错 + L-21 MatchStatus 缺终态 |
| 366 | #538 | B-P1-8 剩余 5 个订阅者接入幂等（B-P1-8 完整闭环） |
| 365 | #537 | B-P1-8 事件幂等基础设施 + InventoryTransactionCreated 接入（新增 processed_events 表） |
| 364 | #536 | B-P1-6 删除 InventoryAdjusted 孤岛事件 |
| 363 | #535 | F-P1-2 剩余：资产负债表/现金流量表走凭证体系（F-P1-2 完整闭环） |
| 362 | #534 | F-P1-2 利润表走凭证体系（按科目编码前缀聚合替代硬编码比例） |
| 361 | #533 | B-P1-4 销售订单状态变更事件（5 个 BusinessEvent 变体） |
| 360 | #532 | B-P1-9 生产订单 BPM 回写 + F-P1-1 试算平衡校验 |
| 359 | #531 | B-P1-2 盘点完成事件 + F-P1-3 辅助核算记录写入 |
| 358 | #530 | B-P1-1 销售退货事务边界 + B-P1-5 采购订单审批事件 + F-P1-4 科目余额刷新方法 |
| 357 | #529 | baseline 清零 11 项 unused import warning（规则 14 合规首战） |
| 356 | #528 | v13 P0 业务/财务场景闭环修复（8 项 P0 完成：凭证回写+库存桥接+订单审批+成本核算） |

---

## v12 复审修复阶段（批次 347-355，15/15 完成）

| 批次 | PR | 一句话总结 |
|------|-----|-----------|
| 355 | #527 | P1-4 baseline 清理 + P3 upper_case_acronyms 修复收官（v12 15/15 完成） |
| 354 | #526 | P1-3 unused_imports 清理 5 项 |
| 353 | #525 | P1-3 unused_imports 清理 6 项 |
| 352 | #524 | P1-1 too_many_arguments 修复（mrp_engine + color_price_history 死代码删除） |
| 351 | #523 | P1-2 useless_asref + P1-3 unused_imports 首批 |
| 350 | #522 | P2-4 baseline 过时条目清理（P2 8/8 完成） |
| 349 | #521 | P2-3 cleanup_expired_jti 接入定时任务 |
| 348 | #520 | P2-1+P2-2 死代码删除（3 文件删除孤岛 service） |
| 347 | #519 | P2 死代码清理 4 项 |

---

## v11 复审修复阶段（批次 340-346，27/27 完成）

| 批次 | PR | 一句话总结 |
|------|-----|-----------|
| 346 | #518 | P1-6+P1-7 crud_macro 宏 metavariable 修复收官（v11 27/27 完成） |
| 345 | #517 | P2-8 app_state.rs Default 实现重构（P2 10/10 完成） |
| 344 | #516 | P1 FromStr trait 迁移 + 接入 lock/release 预留接口 |
| 343 | #515 | P3 测试模块 unused_imports 抑制移除 7 项 |
| 342 | #514 | P2+P3 警告抑制移除 5 项 |
| 341 | #513 | P2 过时警告抑制移除 3 项 |
| 340 | #512 | P0+P1 警告抑制移除 5 项 |

---

## v10 复审修复阶段（批次 325-339，P3 43/43 完成）

| 批次 | PR | 一句话总结 |
|------|-----|-----------|
| 339 | #511 | P3 too_many_arguments DTO 重构剩余 3 项收官（P3 43/43 完成） |
| 338 | #510 | P3 too_many_arguments DTO 重构 8 项（5 核心 service + 8 调用方） |
| 337 | #509 | P3 too_many_arguments DTO 重构 6 项（inventory_finance_bridge） |
| 336 | #508 | P3 too_many_arguments DTO 重构 1 项（mrp_engine calculate_requirement） |
| 335 | #507 | P3 too_many_arguments DTO 重构 1 项（inventory_stock_query list_transactions） |
| 334 | #506 | P3 too_many_arguments DTO 重构 1 项（make_voucher_item 12 调用点） |
| 333 | #505 | P3 too_many_arguments DTO 重构 1 项（create_purchase_suggestion_from_shortage） |
| 332 | #504 | P3 too_many_arguments DTO 重构 1 项（order_change_history_service） |
| 331 | #503 | P3 too_many_arguments DTO 重构 1 项（app_state with_secrets_and_cors） |
| 330 | #502 | P3 误报 too_many_arguments 删除 5 项 + DTO 重构 1 项（规则 10 记忆整理批次 290-329） |
| 329 | #501 | P3 too_many_arguments 参数对象重构 2 项（ar_service + budget_management） |
| 328 | #500 | P3 误报 too_many_arguments 抑制移除 9 项 |
| 327 | #499 | P3 too_many_arguments 抑制移除 3 项 |
| 326 | #498 | P2 clippy 警告抑制移除 2 项 |
| 325 | #497 | P0+P1 警告抑制移除 6 项（规则 14 合规首战） |

---

## v9 复审修复阶段（批次 317-323，16/16 完成）

| 批次 | PR | 一句话总结 |
|------|-----|-----------|
| 324 | #496 | sea-orm 版本调研 + 修正误导性注释 + 新增规则 14 |
| 323 | #495 | 低危代码味道 3 项（函数拆分：extract_update_package/cmd_backup/cmd_restore） |
| 322 | #494 | 低危代码质量 3 项（抽取 path_validator 共享模块 + parse_version 共享函数） |
| 321 | #493 | M5 中危 SSRF 防护（elastic.rs + 13 个单元测试） |
| 320 | #492 | M3+M4 中危（retry_webhook 限流 + m0048 user_id 列 IDOR 防护） |
| 319 | #491 | M1+M2 中危（fetch_latest_release + validate_asset_name 防 DNS Rebinding/路径穿越） |
| 318 | #490 | H1+H2 高危（Tar Slip 改 UUID 随机目录 + admin 密码改 --password-stdin） |
| 317 | #489 | P0+P1 严重 3 项（backup pg_dump 失败未 return + 目录权限掩码未应用） |

---

## v8 复审修复阶段（批次 290-316，21/21 完成）

| 批次 | PR | 一句话总结 |
|------|-----|-----------|
| 308-316 | #488 | L1~L9 低风险全部 9 项（重定向/SQL 参数化/解压路径/币种白名单/文件权限等） |
| 307 | #487 | M8 补充 5 个修改文件单元测试（23 个单元测试） |
| 306 | #486 | M6 webhook 测试端点限流器改分布式 |
| 305 | #485 | M5+M7 硬编码系统路径和 API URL（改环境变量） |
| 304 | #484 | M4 后置校验 TOCTOU 风险（先 tar -tf 校验再解压） |
| 303 | #483 | M3 Python 密码拼接注入（改 stdin pipe） |
| 302 | #482 | M2 ES 客户端缺少 SSRF 重定向限制 |
| 301 | #481 | M1 download_update 缺少 resolve_to_addrs |
| 300 | #480 | H4 日志泄露完整 URL 凭据 |
| 299 | #479 | H3 临时目录硬编码改 UUID 随机生成 |
| 298 | #478 | H2 validate_dir_recursive 缺递归深度限制 |
| 297 | #477 | H1 SSRF 防护被 unwrap_or_default 静默绕过 |
| 296 | #476 | 备份文件权限安全漏洞（0o600） |
| 295 | #475 | system_update_service 文件权限安全漏洞 |
| 294 | #474 | webhook 测试端点缺少速率限制漏洞 |
| 293 | #473 | webhook_service 日志信息泄露漏洞 |
| 292 | #472 | currency_service SSRF 防护不完整漏洞 |
| 291 | #471 | backup cmd_restore 命令注入/Tar Slip 漏洞 |
| 290 | #470 | tracking_service LIMIT SQL 注入漏洞 |

---

## v14 深度调研修复阶段（批次 237-289）

> 高风险 6/6 ✅ 已完成（v8 复审合并处理），中风险 3 项 + 低风险 74 项合并到 v13 修复队列。

| 批次 | PR | 一句话总结 |
|------|-----|-----------|
| 289 | #469 | finance/voucher + data-import composable 接入 useTableApi（9 文件） |
| 288 | #468 | scheduling + material-shortage + capacity composable 接入 useTableApi（9 文件） |
| 287 | #467 | logistics + voucher composable 接入 useTableApi（8 文件） |
| 286 | #466 | purchase-return + purchase-inspection composable 接入 useTableApi（9 文件） |
| 285 | #465 | purchaseReceipt + purchase-price composable 接入 useTableApi（9 文件） |
| 284 | #464 | sales-contract + sales-price + purchase-contract composable 接入 useTableApi（12 文件） |
| 283 | #463 | useSysUpd 3 表 + useBpmAp 2 表 composable 接入 useTableApi |
| 282 | #462 | security + bpm/definitions composable 接入 useTableApi |
| 281 | #461 | api-gateway 3 composable + AuditTab 接入 useTableApi |
| 280 | #460 | 6 个 view 接入 useTableApi 第十一批 |
| 279 | #459 | deploy.sh config.yaml auth 段注入 webhook_secret + 规则 00 写入 MEMORY.md |
| 278 | #458 | 4 个 view 接入 useTableApi 第十批 |
| 276 | #455 | 3 个 view 接入 useTableApi 第九批 + validate_secret 熵比阈值修复 |
| 275 | #454 | 3 个 view 接入 useTableApi 第八批 |
| 274 | #452 | 3 个 view 接入 useTableApi 第七批 |
| 273 | #451 | 2 个 view 接入 useTableApi 第六批 + .env.example 变量名统一 + 规则 13 写入 |
| 部署 | #450 | 修复部署配置路径不一致导致后端无法启动 |
| 272 | #449 | 2 个 view 接入 useTableApi 第五批 |
| 271 | #448 | 2 个 view 接入 useTableApi 第四批 |
| 270 | - | 规则 5 E2E 触发（token 权限不足）+ 规则 10 记忆整理 |
| 269 | #447 | 3 个 CRM view 接入 useTableApi 第三批 |
| 268 | #446 | 2 个 view 接入 useTableApi 第二批 |
| 267 | #445 | 2 个 view 接入 useTableApi 首批 |
| 266 | #444 | 3 个 service 分页接入 paginate_with_total 第十批（service 分页全部清零） |
| 265 | #443 | quotation_service 分页接入 paginate_with_total 第九批 |
| 264 | #442 | 4 个 service 分页接入 paginate_with_total 第八批 |
| 263 | #440 | 5 个 service 分页接入 paginate_with_total 第七批 |
| 262 | #439 | Playwright E2E 增强 + E2E 独立到 e2e-batch.yml |
| 261 | #438 | E2E 后端启动修复（AuthConfig serde + PUBLIC_PATHS + CSRF） |
| 260 | #437 | 4 个 service 分页接入 paginate_with_total 第六批 + 规则 5 E2E 检查 |
| 259 | #436 | 4 个 AP service 分页接入 paginate_with_total 第五批 |
| 258 | #435 | 4 个 service 分页接入 paginate_with_total 第四批 |
| 257 | #434 | 4 个 service 分页接入 paginate_with_total 第三批 |
| 256 | #433 | 4 个 service 分页接入 paginate_with_total 第二批 |
| 255 | #432 | 4 个 service 分页接入 paginate_with_total 首批 |
| 254 | #431 | 14 个 composable 文件 eslint-disable any 指令清理 |
| 253 | #430 | AdvancedFilter handleLogicChange 空函数改真实实现 |
| 252 | #429 | bi_analysis + dual_unit_converter unreachable!() 改返回 AppError |
| 251 | #428 | webhook retry 持久化 payload + retry_count（新增迁移 m0047） |
| 250 | #427 | budget_management 审批流跳过改完整审批闭环 |
| 249 | #426 | capacity_service 硬编码置信度 0.8 改动态计算 |
| 248 | #425 | AR/AP 报表 8 端点接入 CacheService 缓存 |
| 247 | #424 | CLI 健康检查硬编码 URL 改环境变量 |
| 246 | #423 | dye-recipe handleViewVersion 空实现改复用主对话框 |
| 245 | #422 | ap_report_service 4 个报表方法 SQL 层聚合 |
| 244 | #421 | ar_service 3 个报表方法 SQL 层聚合 + 删除死代码 |
| 243 | #420 | report-templates XSS 防护 + tracking_handler 输入验证 |
| 242 | #419 | crm/cust get_rfm_distribution 改真实批量计算 |
| 241 | #418 | 恢复 docs.rs ApiDoc + 删除 openapi.rs 死文件 |
| 240 | #417 | permission.rs 权限校验新增 23 个单元测试 |
| 239 | #416 | dye-batch/dye-recipe handleView 空实现改只读模式 |
| 238 | #415 | ar_service get_aging_report 改 SQL CASE WHEN 分桶聚合 |
| 237 | #414 | auth_service/user_handler Argon2id 哈希计算 spawn_blocking 异步化 |

---

## 历史归档

> 批次 1-236 的详细记录已归档到 [docs/archives/](file:///workspace/.monkeycode/docs/archives/)。
