# 任务一句话总结

> 每个任务一行摘要，是 doto-su.md 中详细任务内容的一句话总结。禁止写入详细内容。
> 详细任务内容见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，未完成任务见 [doto.md](file:///workspace/.monkeycode/doto.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。

---

## v14 深度调研报告修复阶段（批次 237+）

| 批次 | PR | 一句话总结 |
|------|-----|-----------|
| 322 | #TBD | 修复 v9 低危代码质量问题 3 项（1. 抽取 backup.rs+upgrade.rs 重复路径校验到 utils/path_validator 共享模块+4 个单元测试 2. 抽取 system_update_service.rs compare_versions+compare_versions_for_sort 重复 parse_version 为共享函数+3 个单元测试 3. WebhookDeliveryResult 保持 pub 并补充可见性说明） |
| 321 | #493 | 修复 v9-M5 中危问题 1 项（elastic.rs ElasticClient::real + ensure_indices 添加 ssrf_guard::validate_url_and_resolve 校验 + resolve_to_addrs 固定 IP 防 DNS Rebinding TOCTOU，新增 try_real 返回 Result 便于测试，13 个单元测试覆盖 SSRF 拦截逻辑） |
| 320 | #492 | 修复 v9-M3+M4 中危问题 2 项（M-3 retry_webhook 新增 WEBHOOK_RETRY_LIMITER 限流防 SSRF 放大 + M-4 迁移 m0048 新增 user_id 列+verify_ownership 所有权校验防 IDOR，webhook_handler+webhook_integration_handler 全部端点传递 auth.user_id，新增 5 个单元测试） |
| 319 | #491 | 修复 v9-M1+M2 中危问题 2 项（M-1 fetch_latest_release 添加 resolve_to_addrs 防 DNS Rebinding + M-2 新增 validate_asset_name 校验 asset.name 防路径穿越，新增 3 个单元测试） |
| 318 | #490 | 修复 v9-H1+H2 高危问题 2 项（H-1 upgrade Tar Slip 改 UUID 随机目录+先 tar -tf 校验再解压+二次校验 + H-2 admin 密码移除 --password 改 --password-stdin+BINGXI_ADMIN_PASSWORD 环境变量，新增 read_password + 4 个单元测试） |
| 317 | #489 | 修复 v9-P0+P1 严重问题 3 项（P0-1 backup pg_dump 失败未 return false + P0-2 system_update 目录权限掩码未应用 is_dir 永假 + P1 backup psql 失败未 return false，新增 set_safe_permissions 辅助函数 + 2 个权限掩码单元测试） |
| 308-316 | #488 | 修复 v8-L1~L9 低风险全部 9 项（L1 重定向限制 + L2 SQL 参数化 + L3 解压路径校验 + L4 函数返回 bool + L5 币种码白名单 + L6 SQL 参数索引统一 + L7 文件权限 0o600 + L8 WebhookPayload 降 pub(crate) + L9 rollback 降私有） |
| 307 | #487 | 修复 v8-M8 补充 5 个修改文件单元测试（currency_service/tracking_service/backup/webhook_service/system_update_service 共 23 个单元测试，覆盖安全校验和核心工具函数） |
| 306 | #486 | 修复 v8-M6 webhook 测试端点限流器改分布式（rate_limit.rs check_rate_limit 改 pub(crate)，webhook_handler.rs test_webhook 改用 check_rate_limit Redis 优先 + 内存回退，多实例共享计数） |
| 305 | #485 | 修复 v8-M5+M7 硬编码系统路径和 API URL（backup.rs /etc/bingxi/.env 和 /etc/systemd/system 改 BINGXI_ENV_FILE/BINGXI_SYSTEMD_DIR 环境变量，currency_service.rs API URL 改 EXCHANGE_RATE_API_URL 环境变量，.env.example 声明 3 个新变量） |
| 304 | #484 | 修复 v8-M4 后置校验 TOCTOU 风险（先 tar -tf 列出内容逐文件校验路径再解压，防止恶意文件在校验前写入磁盘，解压后保留 canonicalize 二次校验双重防护） |
| 303 | #483 | 修复 v8-M3 Python 密码拼接注入风险（admin.rs 密码从字符串拼接改为 stdin pipe 传递，避免 ps 泄露和注入风险，移除 run_cmd 依赖） |
| 302 | #482 | 修复 v8-M2 ES 客户端缺少 SSRF 重定向限制（elastic.rs 两处添加 redirect(Policy::none())，real() 的 unwrap_or_else 改为 eprintln+exit 合规处理） |
| 301 | #481 | 修复 v8-M1 download_update 缺少 resolve_to_addrs（复用 ssrf_guard::validate_url_and_resolve + resolve_to_addrs 固定 IP，消除 DNS Rebinding TOCTOU） |
| 300 | #480 | 修复 v8-H4 日志泄露完整 URL 凭据（app_state.rs ELASTICSEARCH_URL + rate_limit.rs RATE_LIMIT_REDIS_URL 改为只记录"已配置"，防止 user:password@host 凭据泄露） |
| 299 | #479 | 修复 v8-H3 临时目录硬编码且可预测（/tmp/bingxi_restore 固定路径改 uuid::Uuid::new_v4() 随机生成，消除符号链接竞争 TOCTOU 攻击） |
| 298 | #478 | 修复 v8-H2 validate_dir_recursive 缺少递归深度限制（添加 MAX_RECURSION_DEPTH=100 常量和 depth 参数，防止恶意 tar 千层嵌套导致栈溢出 DoS） |
| 297 | #477 | 修复 v8-H1 SSRF 防护被 unwrap_or_default 静默绕过（webhook_service.rs:217 build().unwrap_or_default() 改为 map_err 错误传播，build 失败直接返回错误不创建客户端） |
| 296 | #476 | 修复备份文件权限安全漏洞（压缩成功后设置 0o600 仅所有者可读，防止 .env 敏感信息泄露，bug.md 全部清零） |
| 295 | #475 | 修复 system_update_service 文件权限安全漏洞（unix_mode 改为 mode & 0o755 重置权限掩码，移除 SUID/SGID/粘性位） |
| 294 | #474 | 修复 webhook 测试端点缺少速率限制漏洞（test_webhook 添加 WEBHOOK_TEST_LIMITER 10次/分钟/用户，LazyLock<MemoryRateLimiter> + TooManyRequests 429） |
| 293 | #473 | 修复 webhook_service 日志信息泄露漏洞（webhook_url 完整 URL 改为 webhook_host 只记录主机名，url::Url::parse 提取 host_str） |
| 292 | #472 | 修复 currency_service SSRF 防护不完整漏洞（复用 ssrf_guard::validate_url_and_resolve + resolve_to_addrs 固定 IP，消除 DNS Rebinding TOCTOU） |
| 291 | #471 | 修复 backup cmd_restore 命令注入/Tar Slip 漏洞（新增 validate_extracted_paths 递归校验 + canonicalize 解析符号链接，规则 12 合规） |
| 290 | #470 | 修复 tracking_service get_popular_pages LIMIT SQL 注入漏洞（字符串拼接改参数化绑定 `LIMIT $N`，规则 12 合规） |
| 289 | #469 | finance/voucher + data-import composable 接入 useTableApi（9 文件，useVchr reactive 包装 + handleSearch/handleReset + VchrFilter localQuery + VchrTbl page/pageSize props + useDi 双表 useTableApi 实例 + DiTplTbl/DiTaskTbl localQuery + useDiProc 简化 DiCallbacks + voucherFormRef getter/setter 代理避免 vue-tsc 自动解包） |
| 288 | #468 | scheduling + material-shortage + capacity composable 接入 useTableApi（9 文件，filterStatus 独立 ref + syncFilterToQuery + watch 自动同步 stats + useMsProc 适配 syncFilterToQuery + capacity initOnMount 仅加载辅助数据） |
| 287 | #467 | logistics + voucher composable 接入 useTableApi（8 文件，useLgs dateRange 独立 ref + syncDateRangeToQuery + watch 自动同步 stats + VoucherListTab toRef 保持 proc 响应性 + 移除 useLgs 未使用 logisticsApi import） |
| 286 | #466 | purchase-return + purchase-inspection composable 接入 useTableApi（9 文件，dateRange 独立 ref + syncDateRangeToQuery + watch 自动同步 stats + usePiProc queryParams 放宽为 Record） |
| 285 | #465 | purchaseReceipt + purchase-price composable 接入 useTableApi（9 文件，usePrcProc 适配 queryParams 放宽 + page 独立字段 + 移除 handlePageChange/handlePageSizeChange） |
| 284 | #464 | sales-contract + sales-price + purchase-contract composable 接入 useTableApi（12 文件，localQuery + handleSearch 模式，date_range 特殊处理，更新 clippy baseline 加入 33 个预存 dead_code 警告） |
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
