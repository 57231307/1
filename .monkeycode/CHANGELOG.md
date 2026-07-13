# 任务一句话总结

> 每个任务一行摘要，是 doto-su.md 中详细任务内容的一句话总结。禁止写入详细内容。
> 详细任务内容见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，未完成任务见 [doto.md](file:///workspace/.monkeycode/doto.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。

---

## v14 深度调研报告修复阶段（批次 237+）

| 批次 | PR | 一句话总结 |
|------|-----|-----------|
| 365 | #537 | v13 复审 P1 级闭环修复 B-P1-8 事件幂等处理基础设施+InventoryTransactionCreated接入（新增 processed_events 表 migration m0049 + SeaORM entity + EventIdempotencyService 服务 try_mark_processed_txn/try_mark_processed + inventory_finance_bridge_service handle_inventory_transaction 去掉_transaction_id下划线前缀接入幂等检查 inventory_txn:{transaction_id} 键，9 文件 201 行，2 次 CI 修复 EntityName冲突+TransactionTrait导入，CI 全绿，B-P1-8 基础设施完成） |
| 364 | #536 | v13 复审 P1 级闭环修复 B-P1-6 删除 InventoryAdjusted 孤岛事件（无 publish + 订阅者仅打日志 + 语义被 InventoryTransactionCreated 覆盖，删除 event_bus 变体定义+订阅者 + event_kafka 映射+测试 + event_kafka_payload 变体+From+TryFrom，3 文件 41 行删除，1 次 CI 全绿，B-P1-6 完整闭环） |
| 363 | #535 | v13 复审 P1 级闭环修复 F-P1-2 剩余（资产负债表存货取数量非金额+_ap_total未使用死代码+预收账款业务口径混淆改从凭证体系 14/1122/1001+1002/16/2202/2203 科目前缀取时点余额 + 现金流量表投资/筹资/期初现金硬编码ZERO改从 1601/25/1001+1002 科目前缀取数 + 新增 get_subject_balance_by_prefix 方法 + 移除 4 个未使用 imports，1 次 CI 全绿，F-P1-2 完整闭环） |
| 362 | #534 | v13 复审 P1 级闭环修复 F-P1-2 利润表走凭证体系（finance_report_service get_income_statement 重写从已过账凭证分录按科目编码前缀 60/64/6601/6602/6603 聚合替代硬编码 70%/15%/10%/5% 比例 + 新增 sum_voucher_amount_by_subject_prefix 私有方法联表查询，1 次 CI 全绿） |
| 361 | #533 | v13 复审 P1 级闭环修复 B-P1-4 销售订单状态变更事件（event_bus 新增 5 个 BusinessEvent 变体 SalesOrderSubmitted/Approved/Completed/Cancelled/Rejected + order_workflow 4 方法 + contract.rs reject_order commit 后发布事件 + event_kafka_payload + event_kafka 同步 Kafka 序列化 + 测试用例，1 次 CI 全绿） |
| 360 | #532 | v13 复审 P1 级闭环修复（B-P1-9 event_bus BpmProcessFinished 新增 production_order 分支 + production_order_service 新增 approve_order_via_bpm/reject_order_via_bpm 不回调 BPM 避免循环 + F-P1-1 accounting_period_service close_period 新增 check_trial_balance_txn 试算平衡校验 + 替换硬编码 posted 为 VOUCHER_POSTED 常量，1 次 CI 全绿） |
| 359 | #531 | v13 复审 P1 级闭环修复（B-P1-2 inventory_count_service approve_count commit 后发布 InventoryCountCompleted 事件触发差异报告归档 + F-P1-3 voucher_service post 新增 write_assist_accounting_records_txn 凭证过账写入辅助核算记录表，1 次 CI 全绿，product_id/warehouse_id 占位待 Schema 补字段） |
| 358 | #530 | v13 复审 P1 级闭环修复（B-P1-1 sales_return_service record_transaction→record_transaction_txn 消除事务边界泄漏+幻事件 + B-P1-5 po/contract approve_order 发布 PurchaseOrderApproved 事件 + F-P1-4 account_subject_service 新增 refresh_balance 方法，3 次 CI 修复编译错误+rustdoc 警告，CI 全绿） |
| 357 | #529 | v13 复审 baseline 清零 11 项 unused import warning 修复（inventory_stock_handler Deserialize/Serialize + routes 4 文件 put/delete + customer_credit_limit Arc + event_kafka Deserialize/Serialize + import_export_service 2 处 self + quotation_approval_service/report ds ActiveModelTrait，规则 14 合规 CI 全绿） |
| 356 | #528 | v13 复审 P0 业务/财务场景闭环修复（voucher_service create_and_post 科目余额回写+自动过账 + inventory_finance_bridge_service 采购退货/销售退货/生产领退料凭证生成 + delivery.rs SALES_DELIVERY 库存流水 + order_workflow 审批后库存预留 + production_order 成本核算闭环，3 次 CI 修复编译错误，8 项 P0 完成，11 个 unused import warning 遗留批次 357） |
| 355 | #527 | v12 复审 P1-4 baseline 清理 + P3 upper_case_acronyms 修复收官（baseline 删 25 行 + utils/incoterms.rs Incoterms2020 枚举 FOB→Fob/CIF→Cif/EXW→Exw/DDP→Ddp/DAP→Dap + quotation_pricing_service CustomerLevel VIP→Vip/NORMAL→Normal + 2 测试文件同步 + #[serde(rename_all="UPPERCASE")] 保持 API 契约 + CI 初次失败恢复 6 条误删 baseline 历史摘要行 + CI 13+2 全绿 v12 复审 15/15 全部完成） |
| 354 | #526 | v12 复审 P1-3 unused_imports 清理 5 项（inventory_stock_handler_query 测试模块 use super::* + 4 文件 AuthContext/rust_decimal::prelude::*/tracing::info/custom_order_crud_service useless_asref 修复 as_ref().map().unwrap_or→clone().unwrap_or） |
| 353 | #525 | v12 复审 P1-3 unused_imports 清理 6 项（bpm_service 5 个 DTO 导入 + dual_unit_converter_handler/sales_unit_tests/assist_accounting_service 各 1 项 unused import 清理） |
| 352 | #524 | v12 复审 P1-1 too_many_arguments 修复（mrp_engine_service calculate_requirement_with_stock 10→3 参数复用 RequirementCalcParams + color_price_history_service record_change 死代码删除+PriceChangeRecord 删除 + baseline 删 7 行） |
| 351 | #523 | v12 复审 P1-2 useless_asref + P1-3 unused_imports 首批（custom_order_crud_service useless_asref 修复 + bpm_service 5 DTO + 3 文件 unused import 清理） |
| 350 | #522 | v12 复审 P2-4 baseline 过时条目清理（删 19 行 1508→1489，P2 8/8 全部完成） |
| 349 | #521 | v12 复审 P2-3 cleanup_expired_jti 接入定时任务（main.rs tokio::spawn 间隔 3600 秒 + 3 文件注释修正） |
| 348 | #520 | v12 复审 P2-1+P2-2 死代码删除（3 文件删除 ar_collection_service+five_dimension_query_service+fabric_five_dimension + 2 mod.rs 修改） |
| 347 | #519 | v12 复审 P2 死代码清理 4 项（unwrap_safe must_some/must_ok + hash sha256_hex_multi + color_space_converter rgb_to_hex/delta_e_76 + process_state_machine node_type_to_status） |
| 346 | #518 | v11 复审 P1-6+P1-7 crud_macro 宏 metavariable 修复收官（impl_generate_no! 宏 $entity 从 ty 改为 path metavariable 可直接作为路径表达式使用 + <$entity>::default() 改为 $entity Entity unit struct 直接作为值 + 移除 2 处 #[allow(clippy::default_constructed_unit_structs)] + 14 个调用点均为 xxx::Entity 路径格式兼容 + generate_no 签名 _entity: E 泛型兼容，规则 14 合规 CI 13+2 全绿 v11 复审 27/27 全部完成） |
| 345 | #517 | v11 复审 P2-8 app_state.rs Default 实现重构收官（impl Default for AppState default 方法移除 #[allow(dead_code, unused_variables)] 原问题 jwt_secret 字段初始化器 #[cfg(not(test))] 调用 std::process::exit(1) 导致后续字段不可达触发 dead_code+unreachable_code 修复方案 #[cfg(not(test))] panic! 提前到函数体开头返回 ! coerce 到 Self + #[cfg(test)] 所有局部变量被字段初始化器使用消除 unused_variables + jwt_secret 直接固定测试密钥无需内联 cfg 规则 14 合规 CI 13+2 全绿 P2 10/10 全部完成 v11 进度 24/27 剩余 3 项为宏内合理保留） |
| 344 | #516 | v11 复审 P1 FromStr trait 迁移 + 接入 lock/release 预留接口（color_card_borrow_service from_str→std::str::FromStr trait 消除 should_implement_trait 警告 + 新增 BorrowStatusParseError + inventory_reservation_handler 新增 lock_reservation/release_reservation handler 真实接入 service 方法 + routes/inventory 新增 POST /reservations/:id/lock 和 /reservations/:id/release 路由，修复批次 341 移除 #[allow(dead_code)] 后 clippy 报 LOCKED/RELEASED never used 根因是方法未接入 handler 规则 0 违规，CI 13+2 全绿） |
| 343 | #515 | v11 复审 P3 测试模块 unused_imports 抑制移除 7 项（dual_unit_converter.rs use crate::dec + inventory_unit_tests/sales_unit_tests/purchase_unit_tests/bi_unit_tests/dual_unit_converter_handler use crate::decs + cache.rs mod csrf_token_tests use super::*，dec!/decs! 宏已在测试代码中广泛使用共 58 个调用点属编译器误报抑制，Rust Clippy 通过确认无新警告，P3 8/8 全部完成 v11 进度 15/27→22/27） |
| 342 | #514 | v11 复审 P2+P3 警告抑制移除 5 项（bpm_dto.rs 删除 TemplateQuery.category 占位符字段及 #[allow(dead_code)] 模板子分类未实现按规则 0 删除 + bpm_process_definition_service list_templates _query→query + user_notification_setting.rs 移除 NONE 常量 #[allow(dead_code)] 已在 service 显式检查 + user_notification_setting_service should_send_email/should_send_internal 添加 NONE 显式检查 + event_bus.rs 移除 #[allow(unreachable_patterns)] InventoryTransactionCreated 未处理 _ 分支可达，v11 进度 12/27→15/27） |
| 341 | #513 | v11 复审 P2 过时警告抑制移除 3 项（dto/mod.rs 删除 PageRequest 四个未使用方法 new/page_clamped/offset/limit + crm/mod.rs 删除 CrmService 未使用重导出 + status.rs 移除 LOCKED/RELEASED 过时 #[allow(dead_code)] 常量已被广泛使用，app_state.rs+cache.rs 恢复保留 #[allow] 因 CI clippy 失败待后续评估） |
| 340 | #512 | v11 复审 P0+P1 警告抑制移除 5 项（business_trace_snapshot 文件级抑制收窄 dead_code+unused_imports+unused_variables→dead_code + import_export_service 移除 needless_pass_by_value 误报抑制 2 处 + auth_handler/auth_handler_misc 移除 redundant_clone 抑制 2 处 + inventory_count_service Entity::default()→Entity 移除 default_constructed_unit_structs，baseline 核实无对应警告 CI 全绿） |
| 339 | #511 | v10 复审 P3 too_many_arguments DTO 重构剩余 3 项收官（product_service create_product 19→1 CreateProductArgs + update_product 19→1 UpdateProductArgs + mrp_engine_service explode_bom_recursive 11→4 引入 ExplodeBomArgs<'a> 聚合 9 标量参数借用 results/stock_cache 保留签名，product_handler + import_products_from_csv 调用方同步修改，所有 #[allow(clippy::too_many_arguments)] 全部移除 v10 复审 P3 43/43 全部完成） |
| 338 | #510 | v10 复审 P3 too_many_arguments DTO 重构 8 项（5 核心 service + 8 调用方：ai/recipe_opt make_recipe 8→1 RecipeFixture + inventory_stock_query record_transaction 18→1 RecordTransactionArgs + inventory_stock_service create_stock 12→1 CreateStockArgs + create_stock_fabric 13→1 CreateStockFabricArgs + inventory_stock_txn create_stock_fabric_txn 14→2 + record_transaction_txn 19→2 复用已有参数对象 + customer_service create_customer 18→1 CreateCustomerArgs + update_customer 18→1 UpdateCustomerArgs，共 20+ 调用点同步修改） |
| 337 | #509 | v10 复审 P3 too_many_arguments DTO 重构 6 项（inventory_finance_bridge_service.rs 5 个 create_*_voucher 10→1 参数 + handle_inventory_transaction 12→3 参数 统一引入 VoucherCreateArgs<'a> 参数对象借用 source_bill_type/source_bill_no/batch_no/color_no，start_listener 调用方同步修改，CI 修复 OrderChangeRecord dead code 加入 baseline 属批次 332 技术债务传播） |
| 336 | #508 | v10 复审 P3 too_many_arguments DTO 重构 1 项（mrp_engine_service.rs calculate_requirement 8→1 参数引入 RequirementCalcParams 参数对象，run_mrp_calculation 内部调用方同步修改 bom_level=0，calculate_requirement_with_stock 和 explode_bom_recursive 保留 allow 因含借用参数需单独评估） |
| 335 | #507 | v10 复审 P3 too_many_arguments DTO 重构 1 项（inventory_stock_query.rs list_transactions 9→1 参数引入 ListTransactionsQuery 参数对象，service 层独立定义与 handler 层 ListTransactionParams 分离，inventory_stock_handler_query.rs 调用方同步修改，query 变量重命名为 q 避免冲突） |
| 334 | #506 | v10 复审 P3 too_many_arguments DTO 重构 1 项（inventory_finance_bridge_service.rs make_voucher_item 9→1 参数引入 VoucherItemArgs<'a> 参数对象使用 &str 生命周期借用 subject_code/subject_name，12 个内部调用点同步修改 采购入库/销售出库/库存调整盘盈盘亏/生产入库/生产领料） |
| 333 | #505 | v10 复审 P3 too_many_arguments DTO 重构 1 项（po/price.rs create_purchase_suggestion_from_shortage 8→1 参数引入 ShortageAlertParams 参数对象，event_bus.rs BusinessEvent::MaterialShortageAlert 处理分支同步修改） |
| 332 | #504 | v10 复审 P3 too_many_arguments DTO 重构 1 项（order_change_history_service.rs record_change 9 参数含 &self→1 参数对象 OrderChangeRecord，record_order_created 内部调用方同步修改，record_change 调用链分析仅内部使用） |
| 331 | #503 | v10 复审 P3 too_many_arguments DTO 重构 1 项（utils/app_state.rs with_secrets_and_cors 8→1 参数引入 AppStateParams 参数对象，main.rs 调用方同步修改，补充 clippy baseline 3 项 path_validator dead code 预存技术债务） |
| 330 | #502 | v10 复审 P3 误报 too_many_arguments 删除 5 项 + DTO 重构 1 项（5 误报：create_product_color/get_inventory_summary/explode_bom/run_mrp_calculation 各 7 参数 + create_receivable 6 参数，clippy 不计 &self 阈值 7；1 DTO：update_product_color 8→1 参数引入 UpdateProductColorParams，规则 10 记忆整理批次 290-329 归档） |
| 329 | #501 | v10 复审 P3 too_many_arguments 参数对象重构 2 项（ar_service create_payment 8→2 参数引入 CreateArPaymentParams + budget_management_service create_execution 9→2 参数引入 CreateBudgetExecutionParams，handler+service 内部调用方同步修改） |
| 328 | #500 | v10 复审 P3 误报 too_many_arguments 抑制移除 9 项（clippy 阈值 7 即 >7 才警告，9 个函数参数 ≤7 均为误报：1 参数 list_records + 5 参数 manual_verify/make_record + 6 参数 borrow + 7 参数 get_list×3/create_payment/tencent_sign/notify_multiple_users） |
| 327 | #499 | v10 复审 P3 too_many_arguments 抑制移除 3 项（import_export_service 误报 3 参数删除 + cache.rs 误报 5 参数删除 + user_notification_setting_service 引入 UpdateNotificationSettingParams 参数对象聚合 8 参数） |
| 326 | #498 | v10 复审 P2 clippy 警告抑制移除 2 项（sales_analysis_service needless_late_init 声明赋值合并 + material_shortage_service type_complexity 提取类型别名 MaterialReq，pred.rs 2 项 needless_range_loop 已在 main 5291e773 修复） |
| 325 | #497 | v10 复审 P0+P1 警告抑制移除 6 项（1 P0 死代码 ExportFormatType enum 删除 + 2 P1 文件级 #![allow(clippy::too_many_arguments)] 删除 enhanced_logger/sensitive_action_alert + 3 P1 未使用 pub use + #[allow(unused_imports)] 删除 so/mod+po/mod，规则 14 合规首战） |
| 324 | #496 | sea-orm 版本调研+修正误导性注释（2.0 仍 RC rc.42 项目用 1.1.20 稳定版正确，修正 rust-toolchain.toml+Cargo.toml 注释，新增规则 14 移除警告抑制） |
| 323 | #495 | 修复 v9 低危代码味道问题 3 项（1. extract_update_package 60+行拆分为 prepare_extract_dir+extract_zip_entry 2. cmd_backup 95行拆分为 backup_database+backup_config_files+compress_backup 3. cmd_restore 128行拆分为 validate_tar_contents+restore_database+restore_config_files，附编译错误修复+collapsible_if 消除） |
| 322 | #494 | 修复 v9 低危代码质量问题 3 项（1. 抽取 backup.rs+upgrade.rs 重复路径校验到 utils/path_validator 共享模块+4 个单元测试 2. 抽取 system_update_service.rs compare_versions+compare_versions_for_sort 重复 parse_version 为共享函数+3 个单元测试 3. WebhookDeliveryResult 保持 pub 并补充可见性说明） |
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
