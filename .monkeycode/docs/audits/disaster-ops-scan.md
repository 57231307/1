# 灾备与运维扫描审计报告

- **审计代理**：MonkeyCode Audit Agent
- **审计日期**：2026-08-22
- **审计范围**：灾备与业务连续性（29.x）、SLO/SLI 与告警（21.9-21.11）
- **扫描项数**：9 项
- **扫描方式**：静态 grep + 目录列举，只读审计，未修改任何业务代码

---

## 一、扫描结果总览

| 编号 | 扫描项 | 命中 | 结论等级 |
|------|--------|------|----------|
| 29.1 | RTO/RPO 指标 | 仅文档计划，无代码落地 | 高风险 |
| 29.2 | 主备切换实现 | failover_service.rs 有切换计数器 | 中风险 |
| 29.3 | 异地灾备 | 无任何命中 | 高风险 |
| 29.4 | 业务降级 | 部分模块有降级，未系统化 | 中风险 |
| 29.5 | 灾难恢复剧本 | 无专门文档 | 高风险 |
| 29.6 | 灾备演练记录 | 全部未完成 | 高风险 |
| 21.9 | SLO/SLI | 无 SLO/SLI 定义 | 高风险 |
| 21.10 | 告警降噪 | 有分组/抑制，缺 rules.yml | 中风险 |
| 21.11 | 压测 | 仅模板建议，无落地脚本 | 中风险 |

---

## 二、逐项评估

### 29.1 RTO/RPO 指标定义与验证 — 高风险

**扫描命令**：`grep -rn "RTO\|RPO\|recovery.*time" .monkeycode/docs/`

**命中**：
- `.monkeycode/docs/audits/v15/batch-21/audit-report.md:685` 记录了缺陷 25.4-M："无回滚时间目标（RTO/RPO）"，审计计划要求"回滚 RTO ≤ 5 分钟，RPO ≤ 0"。
- `.monkeycode/docs/tech-debt-repayment-plan.md:113` 列为 P3 技术债："RTO/RPO 指标定义与验证，预计 2-3d，无负责人"。
- `.monkeycode/docs/task-breakdown-min.md:177` 仅定义"检查是否有 RTO/RPO 文档定义"这一步任务。

**评估**：
- RTO/RPO 仅停留在审计报告与技术债清单中，**运维文档未落地实际指标定义**。
- batch-17 报告指出数据库/缓存故障时需运维手动调用 `POST /api/v1/erp/admin/failover/test/switch`，故障恢复时间可能超过 SLA。
- 无任何代码或配置中定义 RTO/RPO 阈值或自动验证机制。

**结论**：RTO/RPO 目标未落地，灾难发生时无量化恢复承诺可依。需在运维文档中正式定义并接入自动化验证。

---

### 29.2 主备切换实现 — 中风险

**扫描命令**：`grep -rn "switch.*backup\|primary.*backup\|failover" backend/src/services/failover_service.rs`

**命中**：
- `failover_service.rs:21-22` 引用 `failover_event` 和 `failover_status` 数据模型。
- `:49` `failover_primary_total`（主调用总次数计数器）。
- `:53` `failover_primary_failed_total`（主调用失败总次数）。
- `:57` `failover_backup_total`（备用调用总次数）。
- `:61` `failover_switch_total`（主备切换总次数）。
- `:66` `failover_circuit_state`（熔断状态）。
- `:143-165` 指标注册到 `failover_metrics` target。

**评估**：
- failover_service 已实现主备切换计数、熔断状态追踪，具备切换观测能力。
- 但 batch-17 报告指出**切换需手动触发**，缺少连续 N 次健康检查失败自动转移的逻辑（计划要求 5s 间隔、3 次失败触发）。
- 切换后备用数据同步落后风险（RPO=0 要求未满足）未在代码层体现补偿。

**结论**：主备切换基础设施存在，但自动化触发与数据一致性保障缺失，属半成品。

---

### 29.3 异地灾备 — 高风险

**扫描命令**：`grep -rn "dr_site\|异地\|standby\|disaster" backend/`

**命中**：无任何输出。

**评估**：
- backend/ 目录下完全无异地灾备、DR 站点、standby 集群相关代码或配置。
- 结合 29.1 RPO=0 要求未落地，说明**当前架构为单机房部署**，无跨可用区/跨地域容灾能力。

**结论**：异地灾备能力为零，单机房故障即等同于全局宕机。属架构级高风险缺口。

---

### 29.4 业务降级机制 — 中风险

**扫描命令**：`grep -rn "degrade\|降级\|read_only\|fallback.*mode" backend/src/`

**命中**：
- `cli/util/upgrade.rs:215,240,243,480` 版本降级检查（禁止降级，除非 `--force-downgrade`）。
- `cli/util/mod.rs:367` 系统时间异常时 `.unwrap_or_default()` 安全降级。
- `bootstrap/routes_bootstrap.rs:52,54,57,116` 锁中毒时优雅降级（`e.into_inner()`），返回上次成功写入值。
- `routes/bulk_color_approval.rs:12` 降级处理路由注释。

**评估**：
- 降级逻辑**零散分布在各模块**，多为局部容错（锁中毒、版本降级），未形成系统化的业务降级框架。
- 缺少 read_only 模式、限流降级、依赖熔断后 fallback mode 等标准降级策略。
- 无统一的降级开关中心管理（如 feature flag 或 config 驱动）。

**结论**：降级能力碎片化，缺乏全局降级编排。灾难场景下难以快速将系统切至保命模式。

---

### 29.5 灾难恢复剧本 — 高风险

**扫描命令**：`ls .monkeycode/docs/ | grep -i "disaster\|recovery\|runbook\|dr"`

**命中**：无任何输出。

**评估**：
- `.monkeycode/docs/` 下无 disaster/recovery/runbook/dr 任何相关文档。
- 灾难发生时运维无标准操作剧本可循，依赖个人经验处置。

**结论**：无灾难恢复剧本，RTO 无法保证。必须建立分级故障处置 runbook（数据库主库宕机、缓存集群失效、第三方 API 不可用等场景）。

---

### 29.6 灾备演练记录 — 高风险

**扫描命令**：`grep -rn "演练\|drill\|exercise" .monkeycode/`

**命中**：
- `.monkeycode/doto.md:131` "28.7 第三方依赖故障演练 — ❌ 未完成"。
- `.monkeycode/doto.md:138` "29.2 数据库主备切换演练 — ❌ 未完成"。
- `.monkeycode/doto.md:142` "29.6 定期灾备演练记录 — ❌ 未完成"。
- `.monkeycode/docs/audits/v15/batch-21/audit-report.md:725` "缺陷 25.4-R：无升级演练要求，无 staging 环境演练文档"。

**评估**：
- 灾备相关演练（第三方依赖故障、数据库主备切换、定期灾备）**三项全部未完成**。
- 无任何历史演练记录或演练报告。

**结论**：灾备演练完全空白。未演练即未验证，RTO/RPO 承诺无证据支撑。

---

### 21.9 SLO/SLI 指标 — 高风险

**扫描命令**：`grep -rn "slo\|sli\|latency.*target\|availability" backend/src/`

**命中**：
- `routes/analytics.rs:499,554,555` BI 钻取/slice 业务路由（与可用性指标无关）。
- `routes/system.rs:287-297` 慢查询审计路由（`/slow-queries` 列表/统计/手动采集）。
- 无 SLO/SLI 定义、无 latency target、无 availability 目标。

**评估**：
- 慢查询审计路由存在，说明有性能观测的萌芽，但**未形成 SLO/SLI 体系**。
- 无错误预算（error budget）、无可用性目标（如 99.9%）、无延迟分位目标（p99 < 200ms）。
- 告警无 SLO 驱动，属于阈值告警，无法量化服务质量。

**结论**：SLO/SLI 缺失，服务质量无量化承诺，告警与业务可用性脱钩。

---

### 21.10 告警降噪 — 中风险

**扫描命令**：`ls monitoring/alertmanager/` + `cat monitoring/alertmanager/rules.yml`

**命中**：
- `alertmanager.yml` 存在，`rules.yml` **不存在**（告警规则文件缺失）。
- alertmanager.yml 配置：
  - `group_by: ['alertname', 'severity', 'service']`（分组合理）
  - `group_wait: 30s` / `group_interval: 5m` / `repeat_interval: 4h`（默认路由）
  - 分级路由：critical（10s/1h）、warning（30s/4h）、info（5m/12h）
  - `inhibit_rules:` 存在（抑制规则已配置）
  - `send_resolved: true` 全局开启

**评估**：
- alertmanager 侧降噪配置**基本完善**：分组、抑制、重复间隔、分级路由齐全。
- 但 **rules.yml 缺失**，意味着 Prometheus 告警规则未定义，alertmanager 无告警可路由——降噪配置形同虚设。
- 无法评估告警规则是否做了阈值收敛、是否避免告警风暴。

**结论**：降噪管道已铺，但告警规则源头缺失。需补齐 rules.yml 并验证端到端告警链路。

---

### 21.11 压测 — 中风险

**扫描命令**：`grep -rn "wrk\|k6\|locust\|stress\|benchmark" backend/`

**命中**：
- `backend/scripts/perf-report-template.md:47,165,166` 性能报告模板，建议使用 `wrk` / `hey` / `k6`，含 `wrk -t4 -c50 -d30s --latency` 示例命令。
- `routes/finance.rs:1212,1247` `industry_benchmarks` 业务路由（与压测无关）。

**评估**：
- 仅有**性能报告模板**，建议工具为 wrk/hey/k6，但**无实际压测脚本、无压测结果、无压测 CI 集成**。
- 无 k6/locust 脚本目录，无定期压测流水线。
- 压测能力停留在文档建议阶段。

**结论**：压测未落地，容量规划无数据支撑。建议建立 k6 脚本库并接入发布前压测门禁。

---

## 三、综合结论与优先级建议

### 高风险项（6 项，需优先治理）

| 编号 | 缺口 | 建议 |
|------|------|------|
| 29.1 | RTO/RPO 未落地 | 在运维文档中定义分级 RTO/RPO，并接入自动化验证 |
| 29.3 | 异地灾备为零 | 评估跨可用区部署，至少实现数据库跨 AZ 同步复制 |
| 29.5 | 无灾难恢复剧本 | 编写分级 runbook（DB/缓存/第三方故障场景） |
| 29.6 | 灾备演练全空白 | 启动季度演练计划，覆盖主备切换、依赖故障、全链路恢复 |
| 21.9 | 无 SLO/SLI | 定义核心接口 SLO（可用性 p99、延迟 p99），建立 error budget |
| 21.10 | rules.yml 缺失 | 补齐 Prometheus 告警规则，打通告警端到端链路 |

### 中风险项（3 项，需补强）

| 编号 | 缺口 | 建议 |
|------|------|------|
| 29.2 | 主备切换半自动 | 实现健康检查驱动的自动 failover，补齐数据同步校验 |
| 29.4 | 降级碎片化 | 建立统一降级中心，支持 config/flag 驱动的 read_only/限流降级 |
| 21.11 | 压测仅模板 | 建立 k6 脚本库，接入发布前压测门禁 |

### 核心风险摘要

当前系统在**灾备与业务连续性**维度存在系统性缺口：
1. **单机房部署**（29.3），无异地容灾；
2. **RTO/RPO 无量化承诺**（29.1），灾难时无恢复目标；
3. **无恢复剧本、无演练记录**（29.5、29.6），处置全靠人工经验；
4. **SLO/SLI 缺失**（21.9），服务质量无法度量；
5. **告警规则源头缺失**（21.10），降噪配置空转。

这五项叠加意味着：一旦发生机房级或数据库主库级故障，系统将面临**恢复时间不可控、数据丢失量不可控、运维处置无章法**的三重风险。建议将灾备治理列为 P1 级专项，按"定义指标 → 编写剧本 → 落地自动切换 → 定期演练"的路径推进。

---

*本报告由审计代理生成，仅记录扫描结论与建议，未修改任何业务代码，未创建 PR，未执行推送。*
