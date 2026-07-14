# 任务一句话总结

> 每个任务一行摘要，是 doto-su.md 中详细任务内容的一句话总结。禁止写入详细内容。
> 详细任务内容见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，未完成任务见 [doto.md](file:///workspace/.monkeycode/doto.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近整理：2026-07-13（精简所有条目为一句话，按阶段分段）。

---

## v13 复审 + 业务/财务/运行逻辑闭环修复阶段（批次 356+）

| 批次 | PR | 一句话总结 |
|------|-----|-----------|
| 405 | #580 | v14 低风险修复：CRUD 业务消息常量化第二批 - 5 handler 8 处硬编码替换（crm_handler 2 处 DELETE_OK + budget_management_handler 3 处 DELETE_OK/APPROVE_OK/EXECUTE_OK + webhook_handler 1 处 + bpm_definition_handler 1 处 + production_order_handler 1 处 APPROVE_OK），CI 全绿 |
| 404 | #579 | v14 低风险修复：supplier_service/bpm_service LazyLock<Regex> expect 改为 LazyLock<Option<Regex>> 优雅降级 + 新建 utils/messages.rs biz_msg 常量模块（CREATE_OK/UPDATE_OK/DELETE_OK/APPROVE_OK/EXECUTE_OK/OPERATE_OK）+ crud_macro.rs 6 处硬编码替换为常量引用 + fund_management_handler/missing_handlers 4 处硬编码替换，CI 全绿 |
| 403 | #578 | v14 低风险修复：omni_audit_handler 审计日志 DB 字段 unwrap_or_default 吞错改用 Option<T> 读取区分 NULL 与错误（nullable→默认值，错误→传播 500）+ import_export_service 产品导入价格转换失败静默写 0 改为返回验证错误 + audit_log_service/omni_audit_service shutdown 路径 Mutex::lock().unwrap() 改用 unwrap_or_else 安全访问 poisoned lock，CI 全绿 |
| 402 | #576 | clippy baseline 最后一条 `needless_reference` 警告清零：修复 [webhook_handler.rs#L308](file:///workspace/backend/src/handlers/webhook_handler.rs#L308) 测试代码中 `&*LazyLock` 模式（改为 `&LazyLock` + `addr_of!(**limiter)`），baseline 文件清空；11 个 `#[allow(clippy::too_many_arguments)]` 标注为批次 328（v10 复审 P3，PR #500）历史添加，本批次未新增，CI 全绿 |
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
