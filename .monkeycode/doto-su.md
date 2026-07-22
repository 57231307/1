# 已完成任务归档

> 本文件保存**已完成的任务**详细记录（修改内容、技术要点、CI 验证）。
> 未完成任务见 [doto.md](file:///workspace/.monkeycode/doto.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。

---

## 📦 V15 Batch 488 归档（部分完成：D01/D02/D03/D04/D06/D07/D11/D12/D15/D16/D17 + D08-1 第一二梯队）

### 任务概述

- **批次**：488（进行中，已合并 12/17 项；剩余 D05/D08 第三梯队/D09/D10/D13/D14 五项大型任务）
- **合并方式**：main 直接提交多个 commit（用户指令 D 系列 17 项打包为单批）
- **完成时间**：2026-07-19（D06 完成 22c842a）/ 2026-07-19（D12 完成 ae73f42）/ 2026-07-19（D08-1 完成 5c2f214 等）
- **审计项**：P0-D 系列 17 项打包（模块 G 部署与运维），已完成 10 项审计误判或重构 + D08-1 第一梯队 6 函数 + 第二梯队 22 函数
- **V15 P0 进度**：103/104（D08-1 已完成部分计入）
- **D08 后续梯队进度**（与 [doto.md §一当前状态](file:///workspace/.monkeycode/doto.md) 和 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md) 对齐）：第三梯队 53 函数全部完成（8 子批次）+ 第四梯队子批次 1-6 共 42 函数已完成（PR #672/#673 + main 8f8e81d0 CI 全绿 run 29920837489）；第四梯队 135 函数剩余 91 候选（约 13 子批次）。详细记录待各梯队完成后归档到本节。

### 已完成项详情

#### ✅ P0-D01 Docker 文件违规（审计误判）

- **审计来源**：batch-07 P0-07-1
- **结果**：审计误判 —— Batch 488 步骤 0 验证 5 个 Docker 文件均不存在（Dockerfile / backend/Dockerfile / frontend/Dockerfile / docker-compose.yml / .dockerignore），已在之前批次删除
- **commit**：无（无需修改代码）

#### ✅ P0-D02 install.sh 安装 PostgreSQL 客户端（审计误判）

- **审计来源**：batch-07 P0-07-2
- **结果**：审计误判 —— [install.sh](file:///workspace/快速部署/install.sh) L43 已有 `# P0-D02：移除 postgresql-client 安装` 注释，L44 `apt-get install -y curl jq unzip tar nginx`（已无 postgresql-client）
- **commit**：无（无需修改代码）

#### ✅ P0-D03 5 service 未接入缓存层（已完成 commit cead770）

- **审计来源**：batch-07 P0-07-3
- **结果**：新增 utils/redis_cache.rs L2 层双缓存工具，user/product/customer/supplier/role 5 service 读穿透+写失效，TTL 5 分钟，customer/supplier 缓存命中时 data_scope 权限校验仍执行防越权，REDIS_URL 未配置时优雅降级
- **关联文件**：[redis_cache.rs](file:///workspace/backend/src/utils/redis_cache.rs) + 5 service 文件（user_service / product_service / customer_service / supplier_service / role_service）
- **commit**：cead770

#### ✅ P0-D04 缓存是 moka 非 Redis（已完成 commit cead770）

- **审计来源**：batch-07 P0-07-4
- **结果**：与 D03 同批，moka + Redis 双缓存策略，moka 进程内 L1 + Redis 跨实例 L2
- **关联文件**：[redis_cache.rs](file:///workspace/backend/src/utils/redis_cache.rs) + [cache_service.rs](file:///workspace/backend/src/services/cache_service.rs)
- **commit**：cead770

#### ✅ P0-D06 aria-label 严重不足（已完成 55 子批次 ~225 文件 commit 22c842a）

- **审计来源**：batch-07 P0-07-6
- **证据**：仅 2 个文件 8 处 aria-label
- **结果**：所有交互元素补 aria-label（WCAG 2.1 AA），覆盖 views/ 所有子目录 + components/ 通用组件 + PascalCase 命名 Element Plus 组件
- **关键策略**：icon-only 按钮优先 + el-table/el-dialog/el-form/el-pagination 交互容器 + 动态 :title 用 :aria-label 同步绑定 + V2Table 迁移文件跳过 el-table + 已有 aria-label 文件跳过 + PascalCase 标签同样处理 + 续行 aria-label 检测避免误报
- **子批次列表**（55 个，commit 标注）：
  - D06-2 (aa103cb)：通用组件 8 文件
  - D06-3 (3d7635c)：views 高频页面 6 文件
  - D06-4 (4d14973)：views 高优先级 5 文件
  - D06-5 (5e09b20)：views priority 6-10 5 文件
  - D06-6 (f598caf, 另一 agent)：views priority 11-15 5 文件
  - D06-7 (cfb1fc6)：views priority 11-15 5 文件
  - D06-8 (4b4e690)：高缺失文件 5 个
  - D06-9 (957454a)：系统管理 + 工艺优化 5 文件
  - D06-10 (b93f12f)：system/tabs 剩余 5 个 Tab
  - D06-11 (8cc4506)：trading/tabs 5 个 Tab
  - D06-12 (c1f638a)：system-update + supplier + sales-price 5 文件
  - D06-13 (e77f276)：sales-price/components 5 个组件
  - D06-14 (b01b1c5)：sales-contract/components + sales-returns/components 5 文件
  - D06-15 (c41f443)：logistics/components + purchase-price/components 5 文件
  - D06-16 (a0e0986)：purchase-price/components 剩余 + purchase-contract/components 5 文件
  - D06-17 (9d1a109)：purchase-contract/components 剩余 + purchase-inspection/components 5 文件
  - D06-18 (ffc04cd)：purchase-inspection/components 剩余 + production/components 5 文件
  - D06-19 (a64dc0d)：material-shortage + purchaseReceipt + purchase components 5 文件
  - D06-20 (37685d4)：purchase + inventory components 5 文件
  - D06-21 (4701889)：sales-analysis + scheduling components 6 文件
  - D06-22 (ff269fa)：arReconciliation + purchase-return components 5 文件
  - D06-23 (76b7af5)：purchase-return components 剩余 4 文件
  - D06-24 (a94bb04)：dashboard + data-import components 5 文件
  - D06-25 (53c150a)：security/capacity/advanced components 5 文件
  - D06-26 (c892b8e)：advanced/api-gateway components 5 文件
  - D06-27 (0a1df5f)：api-gateway/system-update/admin components 4 文件
  - D06-28 (7325a7a)：api-gateway tabs 2 文件
  - D06-29 (d2584b5)：inventory tabs 3 文件
  - D06-30 (573e1a7)：finance/tabs/components 4 文件
  - D06-31 (e31ca81)：voucher/sales components 5 文件
  - D06-32 (b2f909a)：sales/finance/quotations/crm 5 文件
  - D06-33 (035bd83)：crm/tabs 批 1 5 文件
  - D06-34 (6ac2efb)：crm/tabs 批 2 + leads + opportunities 5 文件
  - D06-35 (3b0eca9)：bpm/definitions/components 5 文件
  - D06-36 (afc2448)：bpm/approval + system + security 5 文件
  - D06-37 (82444ed)：product/tabs + fabric/DyeTab 5 文件
  - D06-38 (1351018)：fabric/tabs 剩余 5 文件
  - D06-39 (3dbfdd1)：quality + inventoryAdjustment tabs 5 文件
  - D06-40 (2eb2ff5)：inventoryAdjustment + inventoryBatch + inventoryCount 5 文件
  - D06-41 (c9f16e1)：inventoryTransfer + ap/tabs 5 文件
  - D06-42 (8a54858)：ap/ar/fund/supplier/customerCredit 5 文件
  - D06-43 (85b0511)：customerCredit + accountSubject + accountingPeriod + financeReport 5 文件
  - D06-44 (d527e5e)：financial-analysis + bom + mrp 5 文件
  - D06-45 (4581376)：color-cards + color-prices 5 文件
  - D06-46 (f8211a0)：custom-orders + dataPermission + departments 4 文件
  - D06-47 (8818eda)：notification + quality-standards + user-profile + ai-extend + Setup 6 文件
  - D06-48 (d7cec20)：crm 多元素 + quotations/list 5 文件 16 处
  - D06-49 (d33deb6)：customer + customerCredit + scheduling 5 文件 10 处
  - D06-50 (30ae917)：bpm + bi + components-demo + quotations 5 文件 8 处
  - D06-51 (d91b036)：system/tabs + security el-form 5 文件 5 处
  - D06-52 (无 commit)：data-import/purchase-return/material-shortage/purchase-* el-pagination 已有 aria-label 跳过
  - D06-53 (无 commit)：sales-price/system-update tabs el-pagination 已有 aria-label 跳过
  - D06-54 (eaadd4d)：fiveDimension/barcodeScanner/businessTrace/arReconciliation/omniAudit/assistAccounting 6 个 PascalCase 文件 30 处
  - D06-55 (22c842a)：QualityCheck/color-cards/issues/product/tabs 最终收尾 3 文件 4 处
- **最终扫描确认全部补齐无遗漏**

#### ✅ P0-D07 图片 alt 属性完全缺失（审计误判）

- **审计来源**：batch-07 P0-07-7
- **结果**：审计误判 —— [user-profile/index.vue:30](file:///workspace/frontend/src/views/user-profile/index.vue#L30) 原生 `<img>` 已有 `:alt="profileForm.real_name ? '${profileForm.real_name}的头像' : '用户头像'"`；[TfaStep2.vue:14](file:///workspace/frontend/src/views/security/two-factor/components/TfaStep2.vue#L14) `<el-image>` 已有 `alt="二步验证二维码"`
- **commit**：无（无需修改代码）

#### ✅ P0-D11 setup_test_db 重复定义（审计误判）

- **审计来源**：batch-02 P0-02-03
- **结果**：审计误判 —— [test_common.rs](file:///workspace/backend/src/services/test_common.rs) 完整 setup_test_db 实现（18 行，模块头注释标注"抽取自 21 处重复定义"）+ [tests/common/mod.rs](file:///workspace/backend/tests/common/mod.rs) 完整 setup_test_db 实现（19 行，供 tests/ 下 3 个集成测试文件使用）
- **commit**：无（无需修改代码）

#### ✅ P0-D12 8 个函数圈复杂度 >15（已完成 commit 25efd76~ae73f42）

- **审计来源**：batch-02 P0-02-04
- **结果**：8 个目标函数全部处理：
  - 6 项实际重构：check_module_consistency CC 35→7 / auto_match CC 25→15 / update_account_balances CC 17→11 / auto_verify CC 20→15 / ship_order CC 17→13 / start_event_listener CC 33→10（提取 8 个 helper）
  - 2 项审计误判跳过：manual_verify CC=11 已低于阈值 15 / builtin_transition_rules CC=1 已远低于阈值
- **关联文件**：[business_mode_service.rs](file:///workspace/backend/src/services/business_mode_service.rs) / [ar/vfy.rs](file:///workspace/backend/src/services/ar/vfy.rs) / [voucher_service.rs](file:///workspace/backend/src/services/voucher_service.rs) / [ar_service.rs](file:///workspace/backend/src/services/ar_service.rs) / [so/delivery.rs](file:///workspace/backend/src/services/so/delivery.rs) / [event_bus.rs](file:///workspace/backend/src/services/event_bus.rs)
- **commit**：25efd76 + 319c471 + e32048b + 30a1352 + ae73f42（5 个本地 commit 待推送 CI 验证，因 git 认证丢失阻塞中）

#### ✅ P0-D15 升级流程非零停机（审计误判）

- **审计来源**：batch-21 P0-21-1
- **结果**：审计误判 —— [upgrade.rs](file:///workspace/backend/src/cli/util/upgrade.rs) 蓝绿部署已完整实现（14 个函数：is_blue_green_mode / get_active_instance / instance_service / instance_port / opposite_instance / health_check_instance / switch_nginx_upstream / cleanup_temp / cmd_rollback_blue_green / cmd_rollback_legacy / deploy_release / deploy_release_blue_green / deploy_release_legacy + 常量 BLUE_GREEN_TEMPLATE/BLUE_PORT/GREEN_PORT/NGINX_UPSTREAM_ACTIVE/HEALTH_PATH/HEALTH_CHECK_RETRIES）
- **commit**：无（无需修改代码）

#### ✅ P0-D16 报表订阅无后台调度任务（审计误判）

- **审计来源**：batch-16 P0-16-1
- **结果**：审计误判 —— [report_subscription_scheduler.rs](file:///workspace/backend/src/services/report_subscription_scheduler.rs) 完整实现 268 行（run_once / execute_subscription / extract_recipients / update_subscription_status / start_background_task）+ main.rs L696-L711 已接入启动 cron
- **commit**：无（无需修改代码）

#### ✅ P0-D17 OA 公告完全未实现（审计误判）

- **审计来源**：batch-16 P0-16-3
- **结果**：审计误判 —— [oa_announcement_service.rs](file:///workspace/backend/src/services/oa_announcement_service.rs) 完整 CRUD 实现（CreateOaAnnouncementRequest / UpdateOaAnnouncementRequest DTO + create/get_by_id/update/delete/publish/archive/list 7 方法 + validate_announcement_type/validate_status 校验）+ oa_announcement_handler + routes + model 4 件套均已存在
- **commit**：无（无需修改代码）

#### ✅ P0-D08-1 第一梯队 6 函数拆分（已完成 CI 全绿）

- **审计来源**：batch-07 P0-07-8
- **拆分函数列表**：
  1. `ship_order` (so/delivery.rs:110, 346 行 → 22+6helper+3struct)
  2. `create_order` (so/order_crud.rs:98, 344 行 → 36+9helper+1struct)
  3. `manual_verify` (ar_service.rs:993, 254 行 → 52+7helper+1struct)
  4. `approve_task` (bpm_service.rs:242, 211 行 → 29+7helper+1struct(ApproveContext))
  5. `calculate` (wage_service.rs:873, 211 行 → 44+7helper+2struct(WageTotals+StepWageComputed))
  6. `auto_verify` (ar_service.rs:706, 192 行 → 41+5helper+2struct(AutoVerifyData+VerifyTotals))

#### ✅ P0-D08-2 第二梯队 22 函数拆分（已完成 CI 全绿）

- **首批 5 函数**（commit）：
  - batch_update_products 197→59+5helper+1struct(BatchUpdateRollbackContext)
  - import_products_from_csv 197→18+12helper+1struct(ValidatedRowFields)
  - quotation update 189→38+6helper
  - detect_anomalies 187→41+12helper
  - auto_generate_from_receipt 184→27+7helper+1struct(ReceiptVoucherContext)
- **第 2 批 5 函数**：
  - ar create_payment 87→53+3helper
  - voucher update_account_balances 25保持+dispatch_balance_updates拆出
  - so update_order 37→32+1helper(finalize_order_update_after_commit)
  - purchase_return approve_return 前序已拆分本次仅清理 2 处违规注释块
  - ai predict_quality 65→25+1helper(build_history_response)
- **第 3 批 5 函数**：
  - omni_audit new 163→11+6helper(resolve_secret_key/spawn_audit_worker/process_single_message/compute_signature/log_alert_if_needed/build_audit_log_model)
  - ap_report get_statistics_report 161→33+3helper+1struct(ApStatisticsMainAggregate)
  - bi_analysis kpi_summary 159→15+3helper+1struct(KpiCurrentMetrics)
  - business_metrics new 157→40+6helper(register_business_core/session_cache/performance/security/business_feature/http_metrics)
  - outsourcing record_receipt 157→24+6helper+1struct(ReceiptCalculation)
- **第 4 批 7 函数**：
  - so list_orders 156→18+5helper
  - init_service create_default_roles 155→17+9helper
  - ap_report get_aging_report 153→20+3helper+2struct(AgingOverdueAggregate+AgingNotDueAggregate)
  - production_order increase_finished_goods_txn 152→42+3helper+1struct(ProductionOutputRecord)
  - chemical update 150→23+10helper(apply_basic_info/apply_chemical_properties/apply_pricing/apply_ghs_msds/apply_storage_params/apply_inventory_params/apply_packaging/apply_supplier_info/apply_dye_fastness/apply_status_and_remarks)
  - ar vfy get_aging_report 150→18+5helper
  - ap_verification auto_verify 171→33+7helper

### CI 验证

- D06 系列：55 子批次 CI 全绿
- D08-1 第一梯队：CI run 29718405482 全绿
- D08-2 第二梯队首批：CI 4 轮修复（BatchError 未实现 Clone + CI 自动刷新 baseline 误删预存警告 + apply_order_header_updates 借用引用后 String 字段 move E0507 + baseline 恢复 5 条预存警告）
- D08-2 第二梯队第 2 批：CI 1 轮通过
- D08-2 第二梯队第 3 批：CI 2 轮修复（clippy 退出码 101 时运行更远捕获 256 条结构化记录，157 条预存 dead_code 警告被报为新增，将摘要追加到 baseline warning 摘要 7→164 条 总行数 142→299 行）
- D08-2 第二梯队第 4 批：CI 2 轮修复（chemical_service.rs 8 个 apply_* helper 中 String 字段使用 `if let Some(v) = req.xxx` 尝试从 `&Option<String>` move 出 String 值触发 E0507 25 个错误，改为 `if let Some(v) = &req.xxx { ... Set(v.clone()) }`；ar/vfy.rs build_customer_aging_summaries 参数 `&mut Vec<AgingBucket>` 触发 clippy::ptr_arg 改为 `&mut [AgingBucket]`）
- CI 全绿：run 29718405482 + run 29720458274 + run 29725353598 + run 29729300636

### 关键技术教训

1. **CI 自动刷新 baseline 在编译错误时会误删预存警告**（第三次复发）：strict 模式下 CI 比较 clippy 输出与 baseline，编译错误导致 clippy 无法完整分析代码，预存警告暂时消失被误判为"已修复"并从 baseline 移除。修复：自动刷新条件增加 `CLIPPY_MAIN_EXIT = 0` 检查 + CLIPPY_MAIN_EXIT 写入文件供后续 step 读取
2. **subagent 拆分 helper 函数参数借用规则**：helper 函数参数 `&UpdateRequest` 中 String 字段必须用 `&req.xxx` 借用后 clone，不能用 `req.xxx` move（E0507 cannot move out of `Some` which is behind a shared reference）；Copy 类型字段（i32/Decimal）保持原样
3. **clippy::ptr_arg 警告**：`&mut Vec<T>` 参数建议用 `&mut [T]` slice 类型
4. **clippy 退出码 101 时输出不完整陷阱**：编译错误时 clippy 可能运行更远捕获更多预存警告，导致 baseline 误判，需将全部预存 dead_code 警告摘要追加到 baseline
5. **设计要点**：每个 helper ≤50 行 + 辅助 struct 传递上下文 + 事务边界保留 txn.commit() 仍在主函数 helper 通过 &txn 引用参与事务 + 公共 API 签名不变全部保留原始 pub async fn 签名
6. **D06 aria-label 策略**：icon-only 按钮优先 + el-table/el-dialog/el-form/el-pagination 交互容器 + 动态 :title 用 :aria-label 同步绑定 + V2Table 迁移文件跳过 el-table + 已有 aria-label 文件跳过 + PascalCase 标签同样处理 + 续行 aria-label 检测避免误报
7. **D12 圈复杂度优化**：纯数据表函数（如 builtin_transition_rules 27 条状态机三元组定义）可豁免拆分；CC=1 已远低于阈值 15 的函数可跳过

### 影响范围

- D06：~225 文件（views/ 所有子目录 + components/ 通用组件 + PascalCase 命名 Element Plus 组件）
- D08-1+D08-2：6+22=28 函数拆分，涉及 35+ 文件
- D12：6 文件重构
- D03+D04：utils/redis_cache.rs + 5 service 文件
- D01/D02/D07/D11/D15/D16/D17：7 项审计误判无需修改代码

### 自审门（规则 13 步骤 4）

- ✅ D06 每个子批次推送前 grep 验证 aria-label 覆盖
- ✅ D08 每个函数拆分后 grep 验证调用点未变化
- ✅ D12 每个函数重构后 grep 验证 match 表达式完整

---

## 📦 V15 Batch 485-487 摘要（详细已归档）

> 三个批次的完整详细记录（任务概述/修改文件清单/核心变更详解/CI 验证历程/关键决策与教训）已归档到 [doto-su-v15-batch-485-487.md](file:///workspace/.monkeycode/docs/archives/2026-07-22/doto-su-v15-batch-485-487.md)。
> 2026-07-22 按规则 10 深度整理：主文件仅保留摘要表格，控制文件大小。

| 批次 | 审计项 | 核心内容 | 修改文件 | CI 轮次 | 关键教训 |
|------|--------|----------|----------|---------|----------|
| **Batch 487** | P0-T02 + P0-T07 + P0-T05 | 7 项业务路径集成测试（73 测试）+ 4 service 性能基准（11 基准，criterion optional feature）+ E2E 配置修复（applyAuthMocks 移除 mockBusinessApi + webServer 数组化） | 28 文件 +1836 -29 | 3 轮（criterion 位置 + baseline 误删） | criterion 必须放 [dependencies] 非 [dev-dependencies]；#[ignore]+纯函数双模式；webServer 数组 + reuseExistingServer:true |
| **Batch 486** | P0-T01 | quotation_service + purchase_receipt_service 单测补全（各 19 测试，共 38 测试） | 2 文件 +730 行 | 1 轮全绿 | sea-orm 表不存在时返回 Err 而非 Ok(None)/Ok([])；DB 测试断言 is_err()；decs!/ymd! 宏 + setup_test_db 模式 |
| **Batch 485** | P0-T03 + P0-T08 | clippy baseline 机制恢复（仅新增警告阻塞，1781 预存警告渐进清理）+ cargo-tarpaulin 覆盖率 job（continue-on-error 不阻塞）+ rgb_to_hex 编译错误修复 + CI bash 算术 bug 修复（grep -c→awk） | 4 文件 +144 -40 | 7 轮 | baseline vs 零容忍策略（test 零容忍 + clippy baseline 渐进）；grep -c+\|\|echo 0 多行陷阱用 awk 替代；规则 20 注释与实现一致 |

---

## 📦 已归档批次索引

> 以下批次已迁移到归档目录以控制主文件大小。

| 批次范围 | 归档文件 | 归档日期 |
|----------|----------|----------|
| V15 Batch 485-487 | [doto-su-v15-batch-485-487.md](file:///workspace/.monkeycode/docs/archives/2026-07-22/doto-su-v15-batch-485-487.md) | 2026-07-22 |
| V15 Batch 477-484 | [doto-su-v15-batch-477-484.md](file:///workspace/.monkeycode/docs/archives/2026-07-22/doto-su-v15-batch-477-484.md) | 2026-07-22 |
