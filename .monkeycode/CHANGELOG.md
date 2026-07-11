# 任务一句话总结

> 每个任务一行摘要，是 doto-su.md 中详细任务内容的一句话总结。禁止写入详细内容。
> 详细任务内容见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，未完成任务见 [doto.md](file:///workspace/.monkeycode/doto.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。

---

## v14 深度调研报告修复阶段（批次 237+）

| 批次 | PR | 一句话总结 |
|------|-----|-----------|
| 283 | #463 | useSysUpd 3 表 + useBpmAp 2 表 composable 接入 useTableApi（reactive 包装返回 + watch 自动更新 stats + 子组件 page/pageSize/total props + v-model 绑定分页 + 移除 onMounted fetch） |
| 282 | #462 | security + bpm/definitions composable 接入 useTableApi（useSec loginLogs + useBpmDf definitions，子组件 page/pageSize props + handleSearch，proc queryParams 类型放宽为 Record<string, unknown>） |
| 281 | #461 | api-gateway 3 composable + AuditTab 接入 useTableApi（reactive 包装返回 + EpForm/KeyForm formRef 改为 v-model:formRef + 子组件 queryParams 类型放宽 + page/pageSize props + handleSearch 同步筛选条件） |
| 280 | #460 | 6 个 view 接入 useTableApi 第十一批（CountListTab + TransferTab + color-prices + process-optimization + quality-prediction + email 双表） |
| 279 | #459 | deploy.sh config.yaml auth 段注入 webhook_secret 字段（旧版部署脚本未同步批次 277 修复，导致后端 fail-fast 退出）+ 规则 00 关联影响评估强制写入 MEMORY.md |
| 278 | #458 | 4 个 view 接入 useTableApi 第十批（fund/Account + fixed-assets/AssetList + cost/CostCollection + budget/BudgetList） |
| 276 | #455 | 3 个 view 接入 useTableApi 第九批（customer + UserTab + BatchListTab） |
| 275 | #454 | 3 个 view 接入 useTableApi 第八批（notification + warehouse + bom）+ validate_secret 熵比阈值 0.3→0.15 修复（openssl rand -hex 32 生成的 hex 密钥 16/64=0.25 被误拒） |
| 274 | #452 | 3 个 view 接入 useTableApi 第七批（color-cards + custom-orders + mrp/history，移除 listColorCards/listCustomOrders/getMrpHistory + 手写分页，修复 mrp/history fetchHistory 未使用错误） |
| 273 | #451 | 2 个 view 接入 useTableApi 第六批（fiveDimension + omniAudit，修复 0-based 分页 bug + dashboard 误用 pagination + logs 缺失 pagination）+ .env.example 变量名统一（AUDIT__SECRET_KEY→AUDIT_SECRET_KEY）+ 规则 13 修复流程写入 MEMORY.md |
| 部署 | #450 | 修复部署配置路径与用户不一致导致后端无法启动（EnvironmentFile /etc/bingxi-erp/.env→/etc/bingxi/.env + 补建 bingxi 用户 + nginx 前端路径 /opt/bingxi-erp→/opt/bingxi，2 处） |
| 272 | #449 | 2 个 view 接入 useTableApi 第五批（customerCredit + arReconciliation，refresh 别名保留兼容事件绑定，修复 loading 未解构引用错误） |
| 271 | #448 | 2 个 view 接入 useTableApi 第四批（dye-batch + dye-recipe，移除 listDyeBatches/listDyeRecipes + 手写分页，refresh 替换 13 处 getList 调用） |
| 270 | - | 规则 5 E2E 触发（403 token 权限不足，需用户手动触发 e2e-batch.yml）+ 规则 10 记忆整理（doto.md 更新到准确状态：中风险 22/25、service 分页 35/35 清零、view 表格 7/56） |
| 269 | #447 | 3 个 CRM view 接入 useTableApi 第三批（leads + opportunities + pool，修复 pool 硬编码分页 bug + poolList 类型修复） |
| 268 | #446 | 2 个 view 接入 useTableApi 第二批（supplierEvaluation 配 pageSizeKey + quotations 移除兼容类型） |
| 267 | #445 | 2 个 view 接入 useTableApi 首批（audit-log + slow-query，测试 mock 适配 @/api/request） |
| 266 | #444 | 3 个 service 分页接入 paginate_with_total 第十批（inventory_stock_query 聚合查询 + fixed_asset + fund_management，service 分页重复实现全部清零） |
| 265 | #443 | quotation_service 分页接入 paginate_with_total 第九批（ServiceError 错误转换 + handler match 穷尽） |
| 264 | #442 | 4 个 service 分页接入 paginate_with_total 第八批（inventory_reservation 修复偏移 bug + color_price crud/history/seasonal 错误转换） |
| 263 | #440 | 5 个 service 分页接入 paginate_with_total 第七批（inventory_stock_query 2处 + inventory_stock_service + custom_order 3文件，修复 get_stock_by_product 偏移 bug） |
| 262 | #439 | Playwright E2E 增强：网络拦截/Mock/弱网/多浏览器/多上下文/多角色/RPA 工具集 + E2E 从 ci-cd.yml 独立到 e2e-batch.yml（每 30 批次运行 + 20/28/29 监控） |
| 261 | #438 | E2E 后端启动修复：AuthConfig serde(default) + initialize 系列加入 PUBLIC_PATHS + CSRF X-Requested-With 头（初始化步骤首次通过） |
| 260 | #437 | 4 个 service 分页接入 paginate_with_total 第六批（po/order/inventory_count/inventory_adjustment/finance_payment）+ 规则 5 E2E 检查（发现 auth 配置缺失问题） |
| 259 | #436 | 4 个 AP service 分页接入 paginate_with_total 第五批（ap_payment_request/ap_payment/ap_reconciliation/ap_verification） |
| 258 | #435 | 4 个 service 分页接入 paginate_with_total 第四批（purchase_receipt/purchase_inspection/purchase_return/supplier_evaluation） |
| 257 | #434 | 4 个 service 分页接入 paginate_with_total 第三批（currency/mrp_engine/production_order/scheduling_query） |
| 256 | #433 | 4 个 service 分页接入 paginate_with_total 第二批（report_subscription/report_template/email_template/email_log） |
| 255 | #432 | 4 个 service 分页接入 paginate_with_total 首批（sales_price/ap_invoice/role/supplier），修复 role_service 偏移 bug |
| 254 | #431 | 14 个 composable 文件 eslint-disable any 指令清理 |
| 253 | #430 | AdvancedFilter handleLogicChange 空函数改为真实实现，新增 logicChange emit 事件 |
| 252 | #429 | bi_analysis + dual_unit_converter unreachable!() 改为返回 AppError 错误，新增 6 个单元测试 |
| 251 | #428 | webhook retry 持久化 payload + retry_count 修复（新增迁移 m0047） |
| 250 | #427 | budget_management 审批流跳过改为完整审批闘环（DRAFT→PENDING→APPROVED/REJECTED） |
| 249 | #426 | capacity_service 硬编码置信度 0.8 改为动态计算（三维：历史订单+负荷+期限衰减） |
| 248 | #425 | AR/AP 报表 8 端点接入 CacheService 缓存（TTL 60s） |
| 247 | #424 | CLI 健康检查硬编码 URL 改为环境变量读取（SERVER__HOST/SERVER__PORT） |
| 246 | #423 | dye-recipe handleViewVersion 空实现改为复用主对话框只读模式 |
| 245 | #422 | ap_report_service 4 个报表方法 SQL 层聚合（O(N)→O(1) 内存） |
| 244 | #421 | ar_service 3 个报表方法 SQL 层聚合 + 删除 DailyAgg/MonthlyAgg 死代码 |
| 243 | #420 | report-templates XSS 防护（escapeHtml 双层）+ tracking_handler 输入验证（validator crate） |
| 242 | #419 | crm/cust get_rfm_distribution 从全 0 占位改为真实批量计算 RFM 评分聚合分布 |
| 241 | #418 | 恢复 docs.rs ApiDoc + 删除 openapi.rs 死文件 |
| 240 | #417 | permission.rs 权限校验新增 23 个单元测试（含垂直越权防护） |
| 239 | #416 | dye-batch/dye-recipe handleView 空实现改为只读模式查看详情 |
| 238 | #415 | ar_service get_aging_report 全表扫描改为 SQL CASE WHEN 分桶聚合 |
| 237 | #414 | auth_service/user_handler Argon2id 哈希计算 spawn_blocking 异步化 |

---

## 历史归档

> 批次 1-236 的详细记录已归档到 [docs/archives/](file:///workspace/.monkeycode/docs/archives/)。
