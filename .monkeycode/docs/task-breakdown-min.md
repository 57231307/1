# 大工程与基础设施任务最小拆分清单

> 将 60 项未完成任务拆分为可直接执行的子任务，每项可在 1 个子代理中完成。

---

## A 项（4 项父任务）

### A.9 AppState DI（4 步）
- A.9.2a 基础设施域 6 服务（db/audit/cache/metrics）注册到 DIContainer + 移除 AppState 字段 → `container/mod.rs`
- A.9.2b 报价定制域 7 服务注册到 DIContainer → `container/mod.rs`
- A.9.2c 通知搜索域 5 服务注册到 DIContainer → `container/mod.rs`
- A.9.2d 故障转移+计数器域 3 项注册到 DIContainer → `container/mod.rs`

### A.10 排程回溯（2 步）
- A.10.2 在 schedule_single_order 排不下时调用 backtrack_schedule 尝试回退 → `scheduling_auto.rs`
- A.10.3 加 max_depth=3 限制 + tracing 日志记录回溯次数 → `scheduling_auto.rs`

### A.18 状态大小写（2 步）
- A.18.4 general.rs + bpm_crm_contract.rs 加大小写统一规范注释 → `models/status/`
- A.18.5 purchase_inventory.rs 加大小写统一规范注释 → `models/status/`

### A.19 状态字面量替换（2 步）
- A.19.7 扫描 backend/src/services/ 剩余 ai_model/bad_debt 等域的状态字面量，输出清单 → 文档
- A.19.8 为 ai_model_management_service.rs 和 bad_debt_service.rs 缺失的状态值在 status 模块补常量 → `models/status/`

### A 项其他子任务（4 步）
- A.15.4 前端质检录入界面加 defect_type 下拉 → `frontend/views/quality/`
- A.20.3 已验证（标记完成）
- A.21.3 已完成（标记完成）

---

## 2.6 过时依赖升级（2 步）
- 2.6.1 运行 `cargo outdated` 或 grep Cargo.toml 版本 vs Cargo.lock 最新版本，输出清单 → 文档
- 2.6.2 升级非破坏性补丁版本（patch/minor），major 版本单独评估 → `Cargo.toml`

## 3.11 异步任务正确性（2 步）
- 3.11.1 grep -rn "tokio::spawn" backend/src/ 输出清单 + 检查是否有 CancellationToken → 文档
- 3.11.2 为无 CancellationToken 的 spawn 补上取消信号 → 对应文件

## 3.14 内存占用与泄漏（2 步）
- 3.14.1 grep -rn "Arc::new\|Box::leak\|static.*Mutex\|lazy_static\|OnceLock" backend/src/ 输出清单 → 文档
- 3.14.2 检查 email_send_counters 等无界集合是否已清理（A.17 已修）+ 其他无界集合扫描 → 文档

## 4.7 依赖漏洞扫描（1 步）
- 4.7.1 运行 `cargo audit` 扫描 CVE，输出报告 → `.monkeycode/docs/audits/`

## 4.8 供应链安全（2 步）
- 4.8.1 grep Cargo.toml 所有第三方 crate，输出清单 + 检查是否有已知供应链事件 → 文档
- 4.8.2 检查是否有不可信源（git+http 依赖）→ 文档

## 4.10 TLS 版本合规（2 步）
- 4.10.1 grep -rn "reqwest\|https\|tls\|rustls\|native-tls" backend/src/ 检查 HTTP 客户端 TLS 配置 → 文档
- 4.10.2 检查 Nginx 配置 TLS 版本（grep ssl_protocols deploy/nginx*.conf）→ 文档

## 5.4 面料规格参数（1 步）
- 5.4.1 grep -rn "weight\|width\|fabric_type\|yarn_spec\|composition" backend/src/models/product.rs 输出字段清单 + 检查完整性 → 文档

## 5.5 色牢度标准（1 步）
- 5.5.1 grep -rn "color_fastness\|色牢度\|light_fastness\|wash_fastness" backend/src/ 输出现有色牢度字段和标准 → 文档

## 5.6 工艺路线与 BOM 一致性（1 步）
- 5.6.1 grep -rn "process_route\|bom\|工艺路线" backend/src/models/ 检查工艺路线和 BOM 是否有交叉引用 → 文档

## 5.7 批次追溯全链路（1 步）
- 5.7.1 grep -rn "dye_lot_no\|batch_no\|匹号\|米数" backend/src/models/ 输出四维标识覆盖范围 → 文档

## 5.8 色差评级（1 步）
- 5.8.1 grep -rn "color_diff\|色差\|delta_e\|grade\|评级" backend/src/ 输出色差评级字段和判定逻辑 → 文档

## 5.9 面料缩水率/纬斜（1 步）
- 5.9.1 grep -rn "shrinkage\|缩水\|skew\|纬斜" backend/src/ 输出质量指标字段 → 文档

## 7.1 单测覆盖率（1 步）
- 7.1.1 检查 CI cobertura.xml 输出，统计行覆盖率，输出报告 → 文档

## 7.3 E2E 完整通过（1 步）
- 7.3.1 检查最近 E2E run 结果（main CI），输出通过/失败数 → 文档

## 7.6 性能基准（1 步）
- 7.6.1 检查 benches/ 目录 4 个 criterion 基准，确认 CI 是否跑基准 → 文档

## 7.7 覆盖率报告（1 步）
- 7.7.1 确认 CI 的 ci-coverage-rust job 是否输出 cobertura.xml artifact → 文档

## 7.8 测试代码有效性（1 步）
- 7.8.1 grep -rn "assert!\|assert_eq!\|assert_ne!" backend/tests/ 统计断言密度，检查空测试 → 文档

## 7.9 测试数据管理与隔离（1 步）
- 7.9.1 grep -rn "setup_test_db\|fixture\|test_data" backend/tests/ 检查测试隔离方式 → 文档

## 7.10 测试环境与生产一致性（1 步）
- 7.10.1 对比 CI PostgreSQL 16 vs 生产 PostgreSQL 15+ 版本差异 + sqlite 回退差距 → 文档

## 7.11 契约测试（1 步）
- 7.11.1 检查是否有 contract test 或 schema 验证（grep -rn "contract\|schema.*test\|snapshot" backend/tests/）→ 文档

## 7.12 故障注入/混沌测试（1 步）
- 7.12.1 检查是否有 chaos test（grep -rn "chaos\|fault\|inject" backend/tests/）→ 文档

## 7.13 测试 flaky 率监控（1 步）
- 7.13.1 检查 CI 是否有 retry/flaky 检测机制 → 文档

## 16.16 面料档案主数据（1 步）
- 16.16.1 grep -rn "product_no\|product_code\|SKU\|编码规则" backend/src/models/product.rs 输出 SKU 编码逻辑 → 文档

## 16.17 色卡与面料关联（1 步）
- 16.17.1 grep -rn "color_card.*product\|product.*color_card\|fabric_id\|color_card_id" backend/src/models/ 检查关联关系 → 文档

## 16.18 供应商面料认证（1 步）
- 16.18.1 grep -rn "qualification\|certification\|认证\|准入" backend/src/models/supplier*.rs 输出认证字段 → 文档

## 21.9 SLO/SLI（1 步）
- 21.9.1 grep -rn "slo\|sli\|latency.*target\|availability.*target" backend/src/ 检查是否有 SLO 定义 → 文档

## 21.10 告警降噪（1 步）
- 21.10.1 检查 monitoring/alertmanager 规则，输出告警规则清单 → 文档

## 21.11 全链路压测（1 步）
- 21.11.1 检查是否有压测脚本/工具（grep -rn "wrk\|k6\|locust\|benchmark\|stress" backend/）→ 文档

## 25.10 前端测试覆盖率（1 步）
- 25.10.1 检查 frontend/vitest.config.ts 是否配置 coverage，统计覆盖率 → 文档

## 26.4 蓝绿部署（1 步）
- 26.4.1 检查 deploy/nginx-upstream-blue.conf 和 green.conf，确认蓝绿配置完整性 → 文档

## 26.9 部署后自动回滚监控（1 步）
- 26.9.1 grep -rn "rollback\|回滚\|health.*check.*deploy" backend/src/services/system_update*.rs 检查回滚逻辑 → 文档

## 27.1 主数据完整性（1 步）
- 27.1.1 grep -rn "UNIQUE\|unique\|唯一" backend/migration/src/ 输出唯一约束清单 → 文档

## 27.2 数据标准与命名规范（1 步）
- 27.2.1 grep -rn "VARCHAR\|INTEGER\|TIMESTAMP\|DECIMAL" backend/migration/src/ 检查命名一致性 → 文档

## 27.3 数据血缘（1 步）
- 27.3.1 grep -rn "business_trace\|batch_trace_log\|trace_log" backend/src/models/ 输出血缘追踪表 → 文档

## 27.4 历史数据归档（1 步）
- 27.4.1 grep -rn "archive\|归档\|retention\|清理" backend/src/services/log_*service*.rs 检查归档逻辑 → 文档

## 27.5 数据脱敏与分级（1 步）
- 27.5.1 grep -rn "mask\|脱敏\|pii\|PII\|sensitive" backend/src/utils/ 检查脱敏覆盖范围 → 文档

## 27.6 数据变更审计追踪（1 步）
- 27.6.1 grep -rn "audit_log\|omni_audit\|operation_log" backend/src/models/ 输出审计表覆盖范围 → 文档

## 27.7 脏数据检测（1 步）
- 27.7.1 grep -rn "CHECK\|约束\|constraint\|validation.*data" backend/migration/src/ 检查脏数据防护 → 文档

## 27.8 数据迁移校验（1 步）
- 27.8.1 检查 init_service.rs 是否有迁移后数据校验逻辑 → 文档

## 28.1 第三方 API 鉴权（1 步）
- 28.1.1 grep -rn "api_key\|bearer\|oauth\|hmac\|signature" backend/src/services/email_service.rs backend/src/services/event_kafka.rs 检查外部调用鉴权 → 文档

## 28.2 接口幂等性（1 步）
- 28.2.1 grep -rn "idempotency\|幂等\|request_id\|dedup\|processed_events" backend/src/ 输出幂等机制覆盖 → 文档

## 28.3 数据同步一致性（1 步）
- 28.3.1 grep -rn "sync\|双向\|replicate" backend/src/ 检查是否有数据同步逻辑 → 文档

## 28.4 接口降级与熔断（1 步）
- 28.4.1 检查 circuit_breaker.rs（A.3 已修）+ grep -rn "fallback\|降级\|degrade" backend/src/ → 文档

## 28.5 回调/Webhook 可靠性（1 步）
- 28.5.1 grep -rn "webhook\|callback\|retry\|dead_letter" backend/src/services/ 输出 Webhook 可靠性机制 → 文档

## 28.6 对接文档与契约测试（1 步）
- 28.6.1 检查 docs.rs OpenAPI 覆盖率（A.25 已做）+ 是否有契约测试 → 文档

## 28.7 第三方依赖故障演练（1 步）
- 28.7.1 grep -rn "failover\|backup\|fallback\|circuit" backend/src/services/ 输出故障应对机制 → 文档

## 29.1 RTO/RPO 指标（1 步）
- 29.1.1 检查是否有 RTO/RPO 文档定义（grep -rn "RTO\|RPO\|recovery" .monkeycode/docs/）→ 文档

## 29.2 数据库主备切换（1 步）
- 29.2.1 grep -rn "failover\|switch.*backup\|primary.*backup" backend/src/services/failover_service.rs 输出切换逻辑 → 文档

## 29.3 异地灾备同步（1 步）
- 29.3.1 检查是否有异地灾备配置（grep -rn "dr_site\|异地\|standby" backend/）→ 文档

## 29.4 业务降级方案（1 步）
- 29.4.1 grep -rn "degrade\|降级\|fallback.*mode\|read_only" backend/src/ 输出降级方案 → 文档

## 29.5 灾难恢复剧本（1 步）
- 29.5.1 检查是否有 DR runbook（ls .monkeycode/docs/ | grep -i "disaster\|recovery\|runbook"）→ 文档

## 29.6 定期灾备演练记录（1 步）
- 29.6.1 检查是否有灾备演练记录（grep -rn "演练\|drill\|exercise" .monkeycode/）→ 文档
