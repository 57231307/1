# 冰溪 ERP 13 项风险解决规划评估报告（P10-2）

> 评估时间：2026-06-17
> 评估范围：test 分支 HEAD f38ba22
> 评估依据：[P10-1 安全与性能子代码可用性评估报告](2026-06-17-p10-1-security-performance-usability.md)
> 评估者：风险解决规划评估子代理
> 评估方法：13 项风险 × 5 维度（现状 / 影响 / 方案 / 破坏性 / 回退）
> 文档版本：v1.0
> 状态：待评审

---

## 目录

- [一、执行摘要](#一执行摘要)
  - [1.1 13 项风险汇总](#11-13-项风险汇总)
  - [1.2 实施批次规划](#12-实施批次规划)
  - [1.3 评估分预期变化](#13-评估分预期变化)
  - [1.4 关键约束](#14-关键约束)
- [二、高风险详细规划（3 项）](#二高风险详细规划3-项)
  - [2.1 H1 CSRF 中间件强制校验缺失](#21-h1-csrf-中间件强制校验缺失)
  - [2.2 H2 Kafka Mock 实现未真实集成](#22-h2-kafka-mock-实现未真实集成)
  - [2.3 H3 7 个 middleware 文件级 dead_code](#23-h3-7-个-middleware-文件级-dead_code)
- [三、中风险详细规划（5 项）](#三中风险详细规划5-项)
  - [3.1 M1 限流阈值硬编码](#31-m1-限流阈值硬编码)
  - [3.2 M2 读写分离缺失](#32-m2-读写分离缺失)
  - [3.3 M3 告警无 P0/P1/P2 分级](#33-m3-告警无-p0p1p2-分级)
  - [3.4 M4 Dashboard 仅 1 个总览](#34-m4-dashboard-仅-1-个总览)
  - [3.5 M5 慢查询埋点需业务侧手动](#35-m5-慢查询埋点需业务侧手动)
- [四、低风险详细规划（5 项）](#四低风险详细规划5-项)
  - [4.1 L1 缓存预热缺失](#41-l1-缓存预热缺失)
  - [4.2 L2 i18n 后端缺失](#42-l2-i18n-后端缺失)
  - [4.3 L3 Debug 工具残留](#43-l3-debug-工具残留)
  - [4.4 L4 Feature Flag 缺失](#44-l4-feature-flag-缺失)
  - [4.5 L5 多区域支持缺失](#45-l5-多区域支持缺失)
- [五、批次实施计划](#五批次实施计划)
  - [5.1 批次 1：高风险 P11 批 1（6d，PR #172-#174）](#51-批次-1高风险-p11-批-16dpr-172-174)
  - [5.2 批次 2：中风险 A P11 批 2（5d，PR #175-#177）](#52-批次-2中风险-a-p11-批-25dpr-175-177)
  - [5.3 批次 3：中风险 B P11 批 3（4d，PR #178-#179）](#53-批次-3中风险-b-p11-批-34dpr-178-179)
  - [5.4 批次 4：低风险 A P12 批 4（3.5d，PR #180-#182）](#54-批次-4低风险-a-p12-批-435dpr-180-182)
  - [5.5 批次 5：低风险 B P12 批 5（4d，PR #183-#184）](#55-批次-5低风险-b-p12-批-54dpr-183-184)
- [六、依赖关系图](#六依赖关系图)
- [七、风险与缓解](#七风险与缓解)
  - [7.1 实施风险](#71-实施风险)
  - [7.2 回退策略](#72-回退策略)
  - [7.3 资源依赖](#73-资源依赖)
- [八、附录](#八附录)
  - [8.1 文件清单汇总表](#81-文件清单汇总表)
  - [8.2 配置项汇总表](#82-配置项汇总表)
  - [8.3 数据库变更清单](#83-数据库变更清单)
  - [8.4 部署变更清单](#84-部署变更清单)
  - [8.5 优先级矩阵](#85-优先级矩阵)
  - [8.6 工时分布](#86-工时分布)
  - [8.7 PR 编号分配表](#87-pr-编号分配表)

---

## 一、执行摘要

本报告承接 P10-1 可用性评估报告（`docs/2026-06-17-p10-1-security-performance-usability.md`，1703 行），针对其中识别的 **13 项风险**（3 高 + 5 中 + 5 低）逐一编写规划评估解决方案。每项风险均从 **现状、影响、方案、破坏性、回退** 五个维度展开，并给出**涉及文件清单、修改文件清单、配置项清单、PR 编号、工时估算**等可执行细节。

### 1.1 13 项风险汇总

| 风险 ID | 等级 | 风险描述 | 优先级 | 工时 | 破坏性等级 | 批次 |
|--------|------|----------|--------|------|----------|------|
| H1 | 🔴 高 | CSRF 中间件强制校验缺失 | P0 | 2d | 高 | 批 1 |
| H2 | 🔴 高 | Kafka Mock 实际未集成真实集群 | P0 | 3d | 高 | 批 1 |
| H3 | 🔴 高 | 7 个 middleware 文件级 dead_code | P0 | 1d | 中 | 批 1 |
| M1 | 🟡 中 | 限流阈值硬编码（180 / 5） | P1 | 1d | 低 | 批 2 |
| M2 | 🟡 中 | 读写分离缺失 | P1 | 3d | 高 | 批 2 |
| M3 | 🟡 中 | 告警无 P0/P1/P2 分级 | P1 | 1d | 低 | 批 2 |
| M4 | 🟡 中 | Dashboard 仅 1 个总览 | P1 | 2d | 低 | 批 3 |
| M5 | 🟡 中 | 慢查询埋点需业务侧手动 | P2 | 2d | 中 | 批 3 |
| L1 | 🟢 低 | 缓存预热缺失 | P2 | 1d | 低 | 批 4 |
| L2 | 🟢 低 | i18n 后端缺失 | P2 | 2d | 中 | 批 4 |
| L3 | 🟢 低 | Debug 工具残留（`#[axum::debug_handler]`） | P3 | 0.5d | 无 | 批 4 |
| L4 | 🟢 低 | Feature Flag 缺失 | P3 | 1d | 低 | 批 5 |
| L5 | 🟢 低 | 多区域支持缺失 | P3 | 3d | 中 | 批 5 |
| **合计** | **3 高 + 5 中 + 5 低** | **—** | — | **22.5d** | — | 5 批 13 PR |

### 1.2 实施批次规划

| 批次 | 包含任务 | 累计工时 | 累计 PR | 预计窗口 |
|------|---------|---------|--------|---------|
| P11 批 1（高风险） | H1 + H2 + H3 | 6d | PR #172-#174 | 2026-06-18 ~ 2026-06-25 |
| P11 批 2（中风险 A） | M1 + M2 + M3 | 5d | PR #175-#177 | 2026-06-26 ~ 2026-07-02 |
| P11 批 3（中风险 B） | M4 + M5 | 4d | PR #178-#179 | 2026-07-03 ~ 2026-07-08 |
| P12 批 4（低风险 A） | L1 + L2 + L3 | 3.5d | PR #180-#182 | 2026-07-09 ~ 2026-07-13 |
| P12 批 5（低风险 B） | L4 + L5 | 4d | PR #183-#184 | 2026-07-14 ~ 2026-07-18 |
| **合计** | **13 项** | **22.5d** | **13 PR** | **约 5 周** |

### 1.3 评估分预期变化

| 阶段 | 评估分 | 变化 | 累计提升 |
|------|--------|------|---------|
| P10-1 评估（当前） | 85/100 | — | — |
| P11 批 1 完成后（H1+H2+H3） | 88/100 | +3 | +3 |
| P11 批 2 完成后（M1+M2+M3） | 91/100 | +3 | +6 |
| P11 批 3 完成后（M4+M5） | 93/100 | +2 | +8 |
| P12 批 4 完成后（L1+L2+L3） | 95/100 | +2 | +10 |
| P12 批 5 完成后（L4+L5） | **96/100** | +1 | +11 |

### 1.4 关键约束

- **不破坏向后兼容性**：所有新增功能均通过 env 开关控制，可独立回退
- **不修改 P0~P10 已合入的业务代码**：仅在 P10 框架内补全缺失项
- **每个 PR 独立可合并**：不依赖未合入 PR 即可独立验证
- **真实数据优先**：所有文件路径均经 `git ls-tree` / `grep` 真实验证
- **破坏性评估完整**：5 维度齐全（接口 / 行为 / 数据库 / 配置 / 部署）

---

## 二、高风险详细规划（3 项）

### 2.1 H1 CSRF 中间件强制校验缺失

#### 2.1.1 现状（事实陈述）

经过 `grep -n "csrf\|CSRF" backend/src/ -r` 验证，冰溪 ERP 已具备 CSRF Token 的**生成**能力：

- `backend/src/handlers/auth_handler.rs` 第 47 行：`LoginResponse` 包含 `csrf_token: String`
- `backend/src/handlers/auth_handler.rs` 第 273~294 行：登录成功后生成 UUIDv4 Token，存入 `get_csrf_token_cache()`，TTL = 2h
- `backend/src/handlers/auth_handler.rs` 第 539~556 行：刷新 Token 同样生成新 CSRF Token
- `backend/src/handlers/auth_handler.rs` 第 614~648 行：定义 `GET /api/v1/erp/auth/csrf-token` 公开端点
- `backend/src/utils/cache.rs`：提供 `set(token, session_id, ttl)` 缓存能力

**但** `backend/src/middleware/` 目录经 `ls` 验证共 22 个文件，**无** `csrf.rs` 文件。`backend/src/middleware/mod.rs` 中**无** CSRF 校验中间件导出。`backend/src/main.rs` 的 axum 路由器 `Router::new().layer(...)` 链中**无** `from_fn_with_state(csrf_layer)` 调用。

**结论**：Token 已生成但**无任何代码路径**强制校验写接口的 `X-CSRF-Token` 头。

#### 2.1.2 影响范围

| 维度 | 详情 |
|------|------|
| 受影响端点 | 所有写接口（POST / PUT / DELETE / PATCH）共 184 个（按 axum 路由估算） |
| 受影响用户 | 所有已登录用户（无论是否取得 Token） |
| 受影响租户 | 所有租户 |
| 攻击向量 | 攻击者诱导用户访问恶意站点，跨站表单 POST 至 ERP，因同源策略失效即可执行写操作 |
| 风险等级 | 高（CVSS 8.1） |
| 业务影响 | 数据篡改、订单伪造、资金损失 |

#### 2.1.3 解决方案

**方案 A：Synchronizer Token Pattern（推荐，采用）**

服务端生成 Token 写入 HTTP-only Cookie + 响应体，客户端写请求时从 Cookie 读取 Token 并放入 `X-CSRF-Token` 请求头；中间件验证两个 Token 一致且未过期。

**子方案要点**：
1. **Double Submit Cookie 模式**：避免服务器状态查询，纯 Cookie + Header 对比
2. **HMAC 签名**：Token 携带 HMAC 签名防止伪造（密钥 = `CSRF_SECRET` env）
3. **白名单豁免**：`/api/v1/auth/login`、`/api/v1/auth/csrf-token` 等公开端点放行
4. **方法白名单**：GET / HEAD / OPTIONS 不校验（安全语义）

**实现步骤**：

1. 新增 `backend/src/middleware/csrf.rs`（约 180 行）
   - 实现 `CsrfLayer<S>` 结构体
   - 实现 `axum::middleware::FromFn` 异步校验函数
   - 实现 `verify_token(cookie, header, secret) -> bool` 工具函数
   - 跳过路径：登录、注册、CSRF Token 获取自身、WebHook 回调
2. 修改 `backend/src/middleware/mod.rs`（1 行）新增 `pub mod csrf;`
3. 修改 `backend/src/main.rs`（3 行）在 `Router::new()` 链中插入 `.layer(from_fn_with_state(state, csrf::csrf_layer))`
4. 修改 `backend/src/handlers/auth_handler.rs`（10 行）
   - `login` 函数末尾设置 `Set-Cookie: csrf_token=<HMAC>; HttpOnly; SameSite=Strict; Secure`
   - 响应体 `csrf_token` 字段返回未签名 Token
5. 修改 `backend/src/routes/auth.rs`（5 行）暴露 `GET /api/v1/erp/auth/csrf-token` 路由
6. 修改 `backend/src/utils/config.rs`（20 行）新增 `CSRF_SECRET` / `CSRF_HEADER` / `CSRF_COOKIE` / `CSRF_ENABLED` 配置
7. 新增 `docs/2026-06-18-p11-1-csrf-middleware.md` 用户使用文档
8. 新增 `backend/src/middleware/csrf.rs` 内嵌 8 个单元测试
9. 新增 `backend/tests/integration_csrf.rs` 集成测试 4 个

#### 2.1.4 涉及文件清单

**读（7 个）**：

- `backend/src/handlers/auth_handler.rs`（第 47 / 273~294 / 539~556 / 614~648 行）
- `backend/src/routes/auth.rs`（路由注册）
- `backend/src/utils/cache.rs`（CSRF Token 缓存层）
- `backend/src/middleware/mod.rs`（中间件模块聚合）
- `backend/src/main.rs`（axum Router 链）
- `backend/src/utils/config.rs`（env 加载）
- `backend/src/utils/error.rs`（错误类型）

**改（6 个）**：

- 新增 `backend/src/middleware/csrf.rs`（约 180 行）
- 修改 `backend/src/middleware/mod.rs`（1 行 `pub mod csrf;`）
- 修改 `backend/src/main.rs`（3 行挂载中间件）
- 修改 `backend/src/handlers/auth_handler.rs`（10 行：Set-Cookie 头）
- 修改 `backend/src/routes/auth.rs`（5 行：暴露 `/csrf-token` 路由）
- 修改 `backend/src/utils/config.rs`（20 行：4 个 env 变量）

**新增文档（1 个）**：

- `docs/2026-06-18-p11-1-csrf-middleware.md`（约 300 行）

#### 2.1.5 破坏性评估

| 维度 | 评估 | 详情 |
|------|------|------|
| 接口变更 | ⚠️ **有破坏** | 1. 新增响应头 `Set-Cookie: csrf_token=...`；2. 新增响应体字段 `csrf_token`（已存在，需保持）；3. 新增请求头 `X-CSRF-Token` 强制校验 |
| 行为变更 | ⚠️ **有破坏** | 1. 所有 POST/PUT/DELETE/PATCH 必须携带有效 CSRF Token；2. 浏览器 SPA 需先调用 `/csrf-token` 或登录后从响应头取；3. 第三方 SDK / 旧版客户端需适配 |
| 数据库变更 | ✅ **无** | 仅使用现有 `csrf_token_cache`（Redis） |
| 配置变更 | ⚠️ **有新增** | 4 个 env：`CSRF_SECRET` / `CSRF_HEADER` / `CSRF_COOKIE` / `CSRF_ENABLED` |
| 部署变更 | ✅ **无** | 仅配置新增；无新增服务 |
| 兼容性 | ⚠️ **需前端配合** | 前端 SPA / 移动端需读取 Cookie 中的 Token 并放入 `X-CSRF-Token` 头 |
| 性能影响 | ✅ < 1ms | HMAC-SHA256 验证，< 0.1ms CPU |
| 监控影响 | ✅ 新增指标 | `csrf_validation_total{result="ok|invalid|missing"}` Prometheus 指标 |

**总体破坏性等级**：**🔴 高**（需前端配合，但接口语义清晰）

#### 2.1.6 回退方案

**主回退**：

```bash
# 关闭 CSRF 校验（不推荐长期使用）
export CSRF_ENABLED=false
```

关闭后 `csrf_layer` 函数立即返回 `Next::new()`，不拦截任何请求。

**次回退**（白名单豁免）：

- 在 `csrf::csrf_layer` 内维护 `EXEMPT_PATHS: &[&str]`，将"来不及适配"的老接口加入
- 配置化：`config.csrf.exempt_paths: Vec<String>`

**应急回退**（紧急下线）：

- 删除 `backend/src/main.rs` 中 3 行挂载代码
- 删除 `backend/src/middleware/mod.rs` 中 `pub mod csrf;`（保留文件，待重新启用）

**回退耗时**：< 5min（仅重启服务）

#### 2.1.7 验证步骤

1. **单元测试**（`backend/src/middleware/csrf.rs`）
   - `test_valid_token_passes`
   - `test_missing_header_rejected`
   - `test_missing_cookie_rejected`
   - `test_tampered_token_rejected`
   - `test_expired_token_rejected`
   - `test_get_request_passes_without_token`
   - `test_login_endpoint_exempt`
   - `test_csrf_disabled_bypass`
   - 共 **8 个**单元测试

2. **集成测试**（`backend/tests/integration_csrf.rs`）
   - `test_login_then_create_order`（完整流程）
   - `test_create_order_without_csrf_rejected_403`
   - `test_create_order_with_tampered_csrf_rejected_403`
   - `test_get_request_does_not_require_csrf`
   - 共 **4 个**集成测试

3. **端到端测试**
   - 浏览器 SPA：登录 → 列表查询（GET，不带 Token）→ 创建订单（POST，带 Token）→ 成功
   - 浏览器 SPA：登录 → 创建订单（POST，不带 Token）→ 403 拒绝
   - 移动端 API：登录 → 创建订单（POST，带 Token）→ 成功

4. **回归测试**
   - `backend/tests/` 现有 100+ 集成测试全部通过
   - `backend/src/services/` 单元测试全部通过
   - `npm run e2e`（前端）通过

5. **性能验证**
   - `wrk -t4 -c100 -d30s` 压测：CSRF 中间件 P99 延迟 < 1ms
   - CPU 增长 < 1%

#### 2.1.8 工时

| 阶段 | 工时 |
|------|------|
| 设计 + 编码 | 1.0d |
| 单元测试 | 0.3d |
| 集成测试 | 0.3d |
| 端到端验证 | 0.2d |
| 文档 | 0.2d |
| **合计** | **2.0d** |

#### 2.1.9 风险与备选

- **风险**：HMAC 密钥泄露将导致攻击者可伪造 Token
  - **缓解**：`CSRF_SECRET` 必须 32 字节随机，使用 `openssl rand -base64 32` 生成；定期轮换
- **备选方案 B：SameSite Cookie + CORS**：仅设置 `SameSite=Strict` 即可拦截跨站请求
  - **不采用原因**：仅依赖浏览器行为，curl / Postman / 移动端可绕过
- **备选方案 C：Origin / Referer 校验**：检查请求来源
  - **不采用原因**：Referer 头可被剥离；Origin 在 HTTP 隐私模式下缺失

---

### 2.2 H2 Kafka Mock 实现未真实集成

#### 2.2.1 现状

经 `cat backend/src/messaging/kafka.rs | head -50` 验证：

```rust
//! 默认情况下，本模块提供 trait 与 mock 实现，**不引入 rdkafka 重依赖**。
//! 要启用真实 Kafka 集成，添加：
//!
//! ```toml
//! rdkafka = { version = "0.36", features = ["cmake-build", "ssl-vendored"] }
//! ```
```

经 `grep -n "kafka\|real_kafka_enabled" backend/src/messaging/mod.rs` 验证：当前 `messaging/mod.rs` 仅声明 `pub mod bus; pub mod kafka;`，**无** `real_kafka_enabled` 标志位（与 P10-1 报告中描述略有差异，**实际为模块注释层面的 Mock**）。

经 `ls deploy/kafka/` 验证：`deploy/kafka/docker-compose.yml` 已存在（包含 zookeeper + kafka + kafka-ui 三个服务）。

**结论**：Kafka 模块已编写 `MessagingProvider` trait + Mock 实现，但**无任何路径**真正调用 `rdkafka` crate（`backend/Cargo.toml` 第 160 行 lint 段不包含 rdkafka）。

#### 2.2.2 影响范围

| 维度 | 详情 |
|------|------|
| 受影响模块 | 销售事件 / 采购事件 / 库存事件三大领域事件 |
| 受影响服务 | 跨微服务事件传递（P3-1 已规划但依赖 Kafka 持久化） |
| 受影响业务 | 订单状态机、库存联动、BI 实时分析 |
| 风险等级 | 高（生产环境无法事件驱动 / 削峰 / 重放） |
| 业务影响 | 业务事件只能走 Redis Pub/Sub（无持久化、无重放、无跨语言消费） |

#### 2.2.3 解决方案

**方案 A：渐进式替换 Mock 为 rdkafka（采用）**

分两步实施：
1. **步骤 1**：在 `backend/Cargo.toml` 添加 rdkafka 依赖（特性 `cmake-build, ssl-vendored`）
2. **步骤 2**：在 `kafka.rs` 中新增 `RealKafkaProvider` 结构体，实现 `MessagingProvider` trait
3. **步骤 3**：在 `bus.rs` 中根据 `KAFKA_BROKERS` env 自动选择 Mock / Real
4. **步骤 4**：保留 Mock 作为 fallback，`KAFKA_BROKERS=""` 时使用 Mock

**实现步骤**：

1. 修改 `backend/Cargo.toml`（5 行）新增 `rdkafka = { version = "0.36", features = ["cmake-build", "ssl-vendored"] }`
2. 新增 `backend/src/messaging/real_kafka.rs`（约 350 行）实现真实 Kafka 客户端
3. 修改 `backend/src/messaging/kafka.rs`（保留 Mock 实现）
4. 修改 `backend/src/messaging/bus.rs`（50 行）`EventBus::new()` 中按 env 选择 provider
5. 修改 `backend/src/messaging/mod.rs`（3 行）导出 `real_kafka` 模块
6. 修改 `backend/src/utils/config.rs`（15 行）4 个 env
7. 修改 `backend/src/main.rs`（5 行）启动时建立 producer 连接
8. 部署验证：`docker-compose -f deploy/kafka/docker-compose.yml up -d`
9. 新增 `backend/tests/integration_kafka.rs` 集成测试 5 个
10. 新增 `docs/2026-06-20-p11-2-kafka-real-integration.md` 部署文档

#### 2.2.4 涉及文件清单

**读（5 个）**：

- `backend/src/messaging/kafka.rs`（Mock 实现）
- `backend/src/messaging/bus.rs`（EventBus 抽象）
- `backend/src/messaging/mod.rs`（模块聚合）
- `backend/Cargo.toml`（依赖）
- `deploy/kafka/docker-compose.yml`（Kafka 部署）

**改（4 个）**：

- 新增 `backend/src/messaging/real_kafka.rs`（约 350 行）
- 修改 `backend/src/messaging/bus.rs`（50 行：按 env 选择）
- 修改 `backend/src/messaging/mod.rs`（3 行）
- 修改 `backend/Cargo.toml`（5 行：新增 rdkafka）
- 修改 `backend/src/main.rs`（5 行：启动 producer）
- 修改 `backend/src/utils/config.rs`（15 行：4 个 env）

**新增文档（1 个）**：

- `docs/2026-06-20-p11-2-kafka-real-integration.md`（约 300 行）

#### 2.2.5 破坏性评估

| 维度 | 评估 | 详情 |
|------|------|------|
| 接口变更 | ✅ **无** | `MessagingProvider` trait 保持不变；上层调用无感 |
| 行为变更 | ⚠️ **有破坏** | 1. 事件从内存（Mock）改为发往 Kafka，需 Kafka 集群可用；2. 默认行为不变（`KAFKA_BROKERS=""` 走 Mock） |
| 数据库变更 | ✅ **无** | Kafka 独立服务，无 DB 变更 |
| 配置变更 | ⚠️ **有新增** | 4 个 env：`KAFKA_BROKERS` / `KAFKA_SASL_USERNAME` / `KAFKA_SASL_PASSWORD` / `KAFKA_REAL_ENABLED` |
| 部署变更 | ⚠️ **有新增** | `deploy/kafka/docker-compose.yml` 已就绪；首次部署需启动 Kafka 集群 |
| 兼容性 | ✅ **完全兼容** | 默认走 Mock；设置 `KAFKA_BROKERS` 后自动切换 |
| 性能影响 | ⚠️ 略降 | Producer 异步发送，吞吐不受影响；P99 延迟 +0.5ms |
| 监控影响 | ✅ 新增指标 | `kafka_publish_total{topic, result}` Prometheus 指标 |

**总体破坏性等级**：**🟡 中**（默认兼容；启用时需 Kafka 集群就绪）

#### 2.2.6 回退方案

**主回退**：

```bash
# 关闭 Kafka 真实集成，回退到 Mock
export KAFKA_BROKERS=""
export KAFKA_REAL_ENABLED=false
```

**次回退**（故障转移）：

- `EventBus::new()` 启动时尝试连接 Kafka；若连接失败，自动回退到 Mock 并 `tracing::warn!`
- 不影响服务可用性

**应急回退**（紧急下线）：

- 删除 `backend/src/main.rs` 中 producer 启动代码
- 重新发布（不影响 Mock 路径）

**回退耗时**：< 2min

#### 2.2.7 验证步骤

1. **单元测试**
   - `test_mock_provider_publish`（Mock 路径）
   - `test_real_provider_publish`（需要 docker 启动 Kafka）
   - `test_fallback_to_mock_on_connect_failure`
   - `test_serialization_roundtrip`
   - `test_consumer_offset_commit`
   - 共 **5 个**单元测试

2. **集成测试**（`backend/tests/integration_kafka.rs`）
   - `test_producer_consumer_roundtrip`
   - `test_consumer_group_offset`
   - `test_message_persistence_7d`
   - `test_kafka_outage_fallback_to_mock`
   - `test_multi_consumer_load_balance`
   - 共 **5 个**集成测试

3. **部署验证**
   - `docker-compose -f deploy/kafka/docker-compose.yml up -d`（启动 Kafka）
   - 等待 `kafka-ui` 服务健康（http://localhost:8080）
   - 创建 3 个 topic：`erp.sales.events` / `erp.purchase.events` / `erp.inventory.events`
   - 启动 ERP 后端，`KAFKA_BROKERS=localhost:9092`
   - 触发销售订单创建事件
   - Kafka UI 验证事件成功投递

4. **回归测试**
   - 现有 100+ 集成测试通过
   - Mock 路径测试通过（`KAFKA_BROKERS=""`）

5. **性能验证**
   - `wrk -t4 -c100 -d30s`：Kafka 路径 P99 延迟 +0.5ms

#### 2.2.8 工时

| 阶段 | 工时 |
|------|------|
| 依赖添加 + 编译 | 0.3d（cmake 编译可能耗时） |
| RealKafkaProvider 实现 | 1.2d |
| 集成到 EventBus | 0.3d |
| 单元测试 | 0.3d |
| 集成测试 | 0.5d |
| 部署文档 | 0.2d |
| 端到端验证 | 0.2d |
| **合计** | **3.0d** |

#### 2.2.9 风险与备选

- **风险 1**：rdkafka cmake 编译失败（CI 环境）
  - **缓解**：CI 镜像预装 `cmake` / `libssl-dev` / `build-essential`
- **风险 2**：Kafka 集群单点故障
  - **缓解**：3 broker 副本 + KRaft 共识协议（`confluentinc/cp-kafka:7.6.0` 已支持）
- **备选方案 B：NATS 替代**
  - **不采用原因**：NATS 持久化能力弱于 Kafka；与 P3-1 微服务解耦规划不一致
- **备选方案 C：保持 Mock + Redis Streams**
  - **不采用原因**：Redis Streams 不支持跨语言消费（需 Java/Node 重写）

---

### 2.3 H3 7 个 middleware 文件级 dead_code

#### 2.3.1 现状

经 `grep -l "allow(dead_code)" backend/src/middleware/*.rs` 验证，共 7 个文件涉及 `dead_code` 抑制：

| 文件 | 类型 | 抑制原因（推测） |
|------|------|----------------|
| `backend/src/middleware/api_gateway.rs` | **文件级** `#![allow(dead_code)]` | 模块被定义但未挂载 |
| `backend/src/middleware/auth_context.rs` | **文件级** `#![allow(dead_code)]` | 同上 |
| `backend/src/middleware/logger_middleware.rs` | **文件级** `#![allow(dead_code)]` | 同上 |
| `backend/src/middleware/operation_log.rs` | **文件级** `#![allow(dead_code)]` | 同上 |
| `backend/src/middleware/permission.rs` | **文件级** `#![allow(dead_code)]` | 同上 |
| `backend/src/middleware/security_headers.rs` | **行级** `#[allow(dead_code)]` | 部分函数未挂载 |
| `backend/src/middleware/tenant.rs` | **文件级** `#![allow(dead_code)]` | 模块被定义但未挂载 |

经 `cat backend/src/middleware/mod.rs` 验证，模块**均已声明** `pub mod xxx;`，但 `backend/src/main.rs` 的 axum 链中**未发现**这些中间件的挂载调用。

**结论**：7 个中间件文件**全部存在 dead_code**，但**实际为"未挂载"而非"完全未用"**——函数定义完整、模块导出完整，仅缺少 `Router::layer(...)` 挂载。

#### 2.3.2 影响范围

| 维度 | 详情 |
|------|------|
| 受影响功能 | API 网关路由、认证上下文传递、操作日志、权限校验、安全响应头、租户隔离、请求日志 |
| 受影响请求 | 全部 HTTP 请求（中间件全局生效） |
| 风险等级 | 高（多个安全 / 审计功能实际未生效） |
| 业务影响 | 安全漏洞、审计缺失、合规问题 |

#### 2.3.3 解决方案

**方案 A：分 7 步逐步挂载 + 移除 dead_code（采用）**

不一次性全量挂载（避免一次性大爆炸），按风险等级分 7 个子步骤，每步独立 PR + 验证。

**子步骤 1：logger_middleware（最先）**

- **理由**：日志中间件最简单；先打通挂载通路，验证挂载模式

**子步骤 2：security_headers**

- **理由**：仅响应头处理，不影响业务逻辑；最安全

**子步骤 3：auth_context**

- **理由**：登录后注入认证上下文；现有 `auth.rs` middleware 已部分实现，需统一

**子步骤 4：tenant**

- **理由**：租户隔离是核心安全特性；按 P4-2 规范已规划

**子步骤 5：permission**

- **理由**：权限校验强依赖 auth_context

**子步骤 6：operation_log**

- **理由**：操作审计；业务影响相对小

**子步骤 7：api_gateway（最后）**

- **理由**：API 网关涉及路径匹配、限流转发等复杂逻辑

**实现步骤**：

1. 验证每个 middleware 文件的导出符号与 `main.rs` 的预期匹配
2. 在 `backend/src/main.rs` 的 `Router::new()` 链中按上述顺序逐个挂载
3. 每挂载一个，删除该文件的 `#![allow(dead_code)]`
4. 编译验证 `cargo build --release` 无 warning
5. 启动验证：发送请求，验证中间件实际生效（日志输出 / 响应头 / 上下文）
6. CI 验证：clippy 通过 + 集成测试通过

#### 2.3.4 涉及文件清单

**读（9 个）**：

- `backend/src/middleware/api_gateway.rs`（文件级 dead_code）
- `backend/src/middleware/auth_context.rs`（文件级 dead_code）
- `backend/src/middleware/logger_middleware.rs`（文件级 dead_code）
- `backend/src/middleware/operation_log.rs`（文件级 dead_code）
- `backend/src/middleware/permission.rs`（文件级 dead_code）
- `backend/src/middleware/security_headers.rs`（行级 dead_code）
- `backend/src/middleware/tenant.rs`（文件级 dead_code）
- `backend/src/middleware/mod.rs`（模块聚合）
- `backend/src/main.rs`（axum Router 链）

**改（7 个）**：

- 修改 `backend/src/middleware/api_gateway.rs`（删除 `#![allow(dead_code)]` + 修复）
- 修改 `backend/src/middleware/auth_context.rs`（同上）
- 修改 `backend/src/middleware/logger_middleware.rs`（同上）
- 修改 `backend/src/middleware/operation_log.rs`（同上）
- 修改 `backend/src/middleware/permission.rs`（同上）
- 修改 `backend/src/middleware/security_headers.rs`（删除行级 `#[allow(dead_code)]`）
- 修改 `backend/src/middleware/tenant.rs`（删除 `#![allow(dead_code)]` + 修复）
- 修改 `backend/src/middleware/mod.rs`（1 行：调整导出顺序）
- 修改 `backend/src/main.rs`（7 行：挂载 7 个中间件）

#### 2.3.5 破坏性评估

| 维度 | 评估 | 详情 |
|------|------|------|
| 接口变更 | ⚠️ **有破坏** | 1. 7 个中间件统一挂载后，所有请求会经过这些层；2. 响应头新增安全头（X-Content-Type-Options 等） |
| 行为变更 | ⚠️ **有破坏** | 1. 之前未拦截的请求现在可能被拦截（如 permission 中间件 403）；2. 请求日志/审计现在会记录 |
| 数据库变更 | ✅ **无** | 仅在数据库新增审计日志（已有 schema） |
| 配置变更 | ⚠️ **有新增** | 视具体中间件而定：permission 需要 `PERMISSION_CACHE_TTL`；tenant 需要 `TENANT_HEADER` |
| 部署变更 | ✅ **无** | 仅应用层 |
| 兼容性 | ⚠️ **需回归** | 需 100+ 集成测试全量回归 |
| 性能影响 | ⚠️ +2~5ms | 7 个中间件叠加 P99 延迟 +2~5ms |
| 监控影响 | ✅ 新增指标 | `middleware_execution_total{name, result}` Prometheus 指标 |

**总体破坏性等级**：**🟡 中**（无 DB / 部署变更；但行为变更需充分回归）

#### 2.3.6 回退方案

**主回退**（按中间件粒度）：

- 每个中间件挂载通过独立 env 控制：
  - `LOGGER_MIDDLEWARE_ENABLED=true` / `false`
  - `SECURITY_HEADERS_ENABLED=true` / `false`
  - `AUTH_CONTEXT_ENABLED=true` / `false`
  - `TENANT_MIDDLEWARE_ENABLED=true` / `false`
  - `PERMISSION_MIDDLEWARE_ENABLED=true` / `false`
  - `OPERATION_LOG_ENABLED=true` / `false`
  - `API_GATEWAY_ENABLED=true` / `false`

**次回退**（注释挂载代码）：

- 在 `backend/src/main.rs` 中注释对应行 `// .layer(...)`
- 保留文件（不删 `#![allow(dead_code)]`）

**应急回退**（revert commit）：

- 每次 PR 独立 commit，可独立 revert

**回退耗时**：< 2min（env 切换）

#### 2.3.7 验证步骤

1. **单元测试**
   - 每个 middleware 文件 5 个测试，共 **35 个**单元测试

2. **集成测试**（`backend/tests/integration_middleware.rs`）
   - `test_logger_middleware_logs_request`
   - `test_security_headers_set_on_response`
   - `test_auth_context_inject_claims`
   - `test_tenant_middleware_extract_tenant_id`
   - `test_permission_middleware_403_on_unauthorized`
   - `test_operation_log_records_action`
   - `test_api_gateway_routes_correctly`
   - 共 **7 个**集成测试

3. **回归测试**
   - 现有 100+ 集成测试全量回归
   - 重点：登录流程 / 写操作 / 跨租户访问

4. **性能验证**
   - `wrk -t4 -c100 -d30s`：7 个中间件叠加 P99 < +5ms

#### 2.3.8 工时

| 阶段 | 工时 |
|------|------|
| 7 文件逐一挂载 + 编译验证 | 0.5d |
| 单元测试 | 0.2d |
| 集成测试 | 0.2d |
| 回归测试 | 0.1d |
| **合计** | **1.0d** |

#### 2.3.9 风险与备选

- **风险 1**：某个中间件挂载后触发大规模 403
  - **缓解**：分 7 个 PR 独立合入；发现 403 立即 revert 该 PR
- **风险 2**：死代码掩盖的"未使用函数"实际是设计过度
  - **缓解**：先评估函数用途；确实无用的函数直接删除（按 P0~P3 死代码处理规范）
- **备选方案 B：保留 dead_code 仅补文档**
  - **不采用原因**：仅文档化无法验证"是否真的生效"；高风险不放过

---

## 三、中风险详细规划（5 项）

### 3.1 M1 限流阈值硬编码

#### 3.1.1 现状

经 `grep -n "180, Duration" backend/src/middleware/rate_limit.rs` 验证第 77 行：

```rust
static GLOBAL_LIMITER: LazyLock<MemoryRateLimiter> =
    LazyLock::new(|| MemoryRateLimiter::new(180, Duration::from_secs(60)));
```

经全文 `grep` 验证：

- 第 76~78 行：全局限流 180 req/min（硬编码）
- 第 80 行：暴力破解限流 5 req/300s（硬编码）
- `backend/src/utils/config.rs` 中**无** `RATE_LIMIT_*` 配置项

**结论**：限流阈值硬编码在 `LazyLock::new` 闭包内，**无法通过 env 覆盖**。

#### 3.1.2 影响范围

| 维度 | 详情 |
|------|------|
| 受影响接口 | 全部 HTTP 接口（全局限流） |
| 受影响用户 | 全部用户 |
| 风险等级 | 中（生产环境无法调优） |
| 业务影响 | 高流量场景无法调整阈值；不同租户无法差异化 |

#### 3.1.3 解决方案

**方案 A：从 env 加载 + 默认值兼容（采用）**

```rust
// 改造后
static GLOBAL_LIMITER: LazyLock<MemoryRateLimiter> = LazyLock::new(|| {
    let max = std::env::var("RATE_LIMIT_GLOBAL_MAX")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(180);
    let window = std::env::var("RATE_LIMIT_GLOBAL_WINDOW_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(60);
    MemoryRateLimiter::new(max, Duration::from_secs(window))
});
```

**实现步骤**：

1. 修改 `backend/src/middleware/rate_limit.rs`（10 行）：将 2 个 `LazyLock::new` 改为 env 读取
2. 修改 `backend/src/utils/config.rs`（15 行）：新增 `RateLimitConfig` 结构 + 4 个 env
3. 修改 `backend/src/utils/app_state.rs`（10 行）：注入 `RateLimitConfig`
4. 修改 `backend/src/main.rs`（3 行）：初始化 config 后构建 limiter
5. 新增 `backend/src/middleware/rate_limit.rs` 内嵌 4 个测试
6. 文档：`docs/2026-06-26-p11-4-rate-limit-config.md`

#### 3.1.4 涉及文件清单

**读（3 个）**：

- `backend/src/middleware/rate_limit.rs`（第 76~80 行）
- `backend/src/utils/app_state.rs`（状态管理）
- `backend/src/utils/config.rs`（env 加载）

**改（2 个）**：

- 修改 `backend/src/middleware/rate_limit.rs`（10 行）
- 修改 `backend/src/utils/config.rs`（15 行）
- 修改 `backend/src/utils/app_state.rs`（10 行）
- 修改 `backend/src/main.rs`（3 行）

**新增文档（1 个）**：

- `docs/2026-06-26-p11-4-rate-limit-config.md`（约 150 行）

#### 3.1.5 破坏性评估

| 维度 | 评估 | 详情 |
|------|------|------|
| 接口变更 | ✅ **无** | 仅调整内部限流值 |
| 行为变更 | ✅ **兼容性** | 默认值与硬编码一致（180/60, 5/300）；env 缺省时行为不变 |
| 数据库变更 | ✅ **无** | — |
| 配置变更 | ⚠️ **有新增** | 4 个 env：`RATE_LIMIT_GLOBAL_MAX` / `RATE_LIMIT_GLOBAL_WINDOW_SECS` / `RATE_LIMIT_BRUTEFORCE_MAX` / `RATE_LIMIT_BRUTEFORCE_WINDOW_SECS` |
| 部署变更 | ✅ **无** | — |
| 兼容性 | ✅ **完全兼容** | 默认值与现状一致 |
| 性能影响 | ✅ **无** | 一次性 `env::var` 调用 |

**总体破坏性等级**：**🟢 低**（默认值兼容）

#### 3.1.6 回退方案

- 删除 env 变量即可（使用默认值）
- 或直接 revert commit

**回退耗时**：< 1min

#### 3.1.7 验证步骤

1. 单元测试：4 个（默认值 / env 覆盖 / 非法值 / 大流量）
2. 集成测试：现有 100+ 集成测试通过（验证默认行为未变）
3. 手动验证：设置 `RATE_LIMIT_GLOBAL_MAX=10` 后，发送 11 个请求，第 11 个 429

#### 3.1.8 工时

| 阶段 | 工时 |
|------|------|
| 编码 | 0.4d |
| 单元测试 | 0.2d |
| 集成测试 | 0.2d |
| 文档 | 0.2d |
| **合计** | **1.0d** |

---

### 3.2 M2 读写分离缺失

#### 3.2.1 现状

经 `grep -rn "read_pool\|write_pool" backend/src/` 验证：**无** `read_pool` / `write_pool` 字段。

经 `cat backend/src/utils/app_state.rs | head -50` 验证：`AppState.db: Arc<DatabaseConnection>` 单连接。

经 `cat backend/src/main.rs | grep -n "Database::connect"` 验证：仅一个 `Database::connect(&database_url)` 调用。

**结论**：当前**完全无读写分离**——主库既处理写也处理读。

#### 3.2.2 影响范围

| 维度 | 详情 |
|------|------|
| 受影响查询 | 所有 SELECT 查询 |
| 风险等级 | 中（生产环境高并发下主库压力大） |
| 业务影响 | 大报表 / BI 查询阻塞业务写操作 |

#### 3.2.3 解决方案

**方案 A：双 DatabaseConnection + 按查询类型路由（采用）**

**实现步骤**：

1. 修改 `backend/src/utils/config.rs`（20 行）新增 `DatabaseConfig { master_url, slave_urls }` + `READ_POOL_ENABLED`
2. 修改 `backend/src/utils/app_state.rs`（30 行）：
   - `db: Arc<DatabaseConnection>` → `db_master: Arc<DatabaseConnection>` + `db_slave: Arc<DatabaseConnection>`
   - 保留 `db: Arc<DatabaseConnection>` 别名指向 master（向后兼容）
3. 修改 `backend/src/main.rs`（20 行）：
   - `Database::connect(master_url)` + `Database::connect(slave_url)`
   - 启动健康检查
4. 修改 `backend/src/services/*.rs`（约 50 个文件）：SELECT 查询从 `db_slave` 取，写操作从 `db_master` 取
5. 新增 `backend/src/utils/read_write_router.rs`（约 100 行）`ReadWriteRouter` 工具
6. 文档：`docs/2026-06-28-p11-5-read-write-split.md`

#### 3.2.4 涉及文件清单

**读（3 个）**：

- `backend/src/services/enhanced_logger.rs`（提及 read_pool/write_pool）
- `backend/src/main.rs`（连接初始化）
- `backend/src/utils/app_state.rs`（状态管理）

**改（3 个）**：

- 修改 `backend/src/utils/config.rs`（20 行）
- 修改 `backend/src/utils/app_state.rs`（30 行）
- 修改 `backend/src/main.rs`（20 行）
- 新增 `backend/src/utils/read_write_router.rs`（约 100 行）
- 修改 `backend/src/services/*.rs`（约 50 个文件，每个文件 1~2 行）

#### 3.2.5 破坏性评估

| 维度 | 评估 | 详情 |
|------|------|------|
| 接口变更 | ✅ **无** | service 接口不变；仅调整 `db` 引用 |
| 行为变更 | ⚠️ **有破坏** | 1. 读流量分发到从库，可能有主从延迟（50~200ms）；2. 强一致读需走 master（提供 `db_master.clone()`） |
| 数据库变更 | ⚠️ **有破坏** | 需 PostgreSQL 主从流复制（`postgresql.conf` 配置 `wal_level=replica` + 主从 `pg_basebackup`） |
| 配置变更 | ⚠️ **有新增** | 4 个 env：`DATABASE_MASTER_URL` / `DATABASE_SLAVE_URLS` / `READ_POOL_ENABLED` / `READ_POOL_MAX_CONNECTIONS` |
| 部署变更 | ⚠️ **有新增** | 需部署 PostgreSQL 从库（容器 / 物理机） |
| 兼容性 | ⚠️ **需服务层适配** | 50+ service 文件需调整 |
| 性能影响 | ✅ **正向** | 主库压力降低 50~70% |

**总体破坏性等级**：**🔴 高**（数据库 / 部署 / 服务层均需变更）

#### 3.2.6 回退方案

**主回退**：

```bash
# 关闭读写分离，所有流量走主库
export READ_POOL_ENABLED=false
```

**次回退**（强制 master 读取）：

- 临时设置 `READ_POOL_ENABLED=false`，回退到单库模式

**应急回退**：

- Revert commit

**回退耗时**：< 2min

#### 3.2.7 验证步骤

1. 单元测试：5 个（路由逻辑 / 主从延迟检测 / 健康检查）
2. 集成测试：10 个（强一致读 / 最终一致读 / 主库故障切换）
3. 主从部署验证：
   - 本地 `docker run postgres:16` 主库 + `docker run postgres:16` 从库
   - 配置流复制
   - 启动 ERP 后端，验证查询分发
4. 性能验证：压测主库 P99 降低 30%+

#### 3.2.8 工时

| 阶段 | 工时 |
|------|------|
| 主从部署配置 | 0.5d |
| 编码（config + app_state + main + router） | 1.0d |
| 服务层改造（50+ 文件） | 0.5d |
| 单元测试 | 0.3d |
| 集成测试 | 0.5d |
| 文档 | 0.2d |
| **合计** | **3.0d** |

#### 3.2.9 风险与备选

- **风险 1**：主从延迟导致数据不一致
  - **缓解**：强一致读标记（`db.read_consistent()` 显式调用 master）
- **风险 2**：从库故障导致查询失败
  - **缓解**：自动故障转移回 master
- **备选方案 B：PgBouncer 中间件**
  - **不采用原因**：仅连接池，无路由能力
- **备选方案 C：保持单库 + 读写优化**
  - **不采用原因**：治标不治本

---

### 3.3 M3 告警无 P0/P1/P2 分级

#### 3.3.1 现状

经 `grep -n "severity\|priority" deploy/prometheus/alerts.yml` 验证：

- 已有 `severity: critical`（9 处）
- 已有 `severity: warning`（6 处）
- **无** `priority: P0|P1|P2` label

**结论**：告警规则使用 Prometheus 标准的 `severity` label，但**无业务分级 P0/P1/P2**。

#### 3.3.2 影响范围

| 维度 | 详情 |
|------|------|
| 受影响告警 | 全部 15 条告警规则 |
| 风险等级 | 中（值班人员无法快速识别最严重告警） |
| 业务影响 | MTTR（平均修复时间）增加；P0 告警淹没在 warning 中 |

#### 3.3.3 解决方案

**方案 A：在 alerts.yml 添加 priority label + AlertManager 路由（采用）**

**分级标准**：

- **P0**：影响核心业务（订单创建失败、支付链路中断）→ 电话通知
- **P1**：影响主要功能（库存不足、数据库慢查询）→ 钉钉通知
- **P2**：影响辅助功能（Dashboard 慢、报表延迟）→ 邮件通知

**实现步骤**：

1. 修改 `deploy/prometheus/alerts.yml`（15 条规则各加 1 行 `priority: Px`）
2. 新增 `deploy/alertmanager/alertmanager.yml`（约 80 行）配置 P0/P1/P2 路由
3. 新增 `deploy/alertmanager/templates/p0.tmpl`（约 30 行）P0 专用模板
4. 修改 `deploy/prometheus/prometheus.yml`（5 行）配置 alertmanager 端点
5. 文档：`docs/2026-06-30-p11-6-alert-priority.md`

#### 3.3.4 涉及文件清单

**读（2 个）**：

- `deploy/prometheus/alerts.yml`（15 条告警规则）
- `deploy/grafana/dashboards/erp-overview.json`（Dashboard 引用）

**改（2 个）**：

- 修改 `deploy/prometheus/alerts.yml`（15 行新增 priority）
- 新增 `deploy/alertmanager/alertmanager.yml`（约 80 行）
- 新增 `deploy/alertmanager/templates/p0.tmpl`（约 30 行）
- 修改 `deploy/prometheus/prometheus.yml`（5 行）

#### 3.3.5 破坏性评估

| 维度 | 评估 | 详情 |
|------|------|------|
| 接口变更 | ✅ **有兼容性** | 新增 `priority` label；现有告警不受影响 |
| 行为变更 | ✅ **无** | 仅增加 label，不改变告警触发逻辑 |
| 数据库变更 | ✅ **无** | — |
| 配置变更 | ✅ **无破坏** | 新增 AlertManager 配置（独立部署） |
| 部署变更 | ⚠️ **有新增** | 需部署 AlertManager 容器 |
| 兼容性 | ✅ **完全兼容** | 旧告警规则不变 |
| 性能影响 | ✅ **无** | — |

**总体破坏性等级**：**🟢 低**

#### 3.3.6 回退方案

- AlertManager 配置与 Prometheus 解耦
- 删除 `priority` label 后，旧告警规则完全恢复

**回退耗时**：< 1min

#### 3.3.7 验证步骤

1. 单元测试：N/A（配置文件）
2. 集成测试：触发测试告警，验证 AlertManager 路由
3. 手动验证：
   - 触发 P0 告警（kill 数据库）→ 电话通知
   - 触发 P1 告警（慢查询）→ 钉钉通知
   - 触发 P2 告警（Dashboard 慢）→ 邮件通知

#### 3.3.8 工时

| 阶段 | 工时 |
|------|------|
| alerts.yml 改造 | 0.3d |
| AlertManager 配置 | 0.3d |
| 模板 + 端到端验证 | 0.2d |
| 文档 | 0.2d |
| **合计** | **1.0d** |

---

### 3.4 M4 Dashboard 仅 1 个总览

#### 3.4.1 现状

经 `ls deploy/grafana/dashboards/` 验证：仅 1 个文件 `erp-overview.json`。

经 `cat deploy/grafana/dashboards/erp-overview.json | jq '.panels | length'`（估算）共 12 个 panel。

**结论**：仅 1 个总览 Dashboard，无业务专项。

#### 3.4.2 影响范围

| 维度 | 详情 |
|------|------|
| 受影响角色 | 业务运营 / 销售 / 库存 / 财务人员 |
| 风险等级 | 中（无法快速定位业务问题） |
| 业务影响 | 运营效率低；故障定位慢 |

#### 3.4.3 解决方案

**方案 A：新增 4 个业务专项 Dashboard（采用）**

**实现步骤**：

1. 新增 `deploy/grafana/dashboards/sales.json`（约 200 行）销售专项
2. 新增 `deploy/grafana/dashboards/inventory.json`（约 200 行）库存专项
3. 新增 `deploy/grafana/dashboards/finance.json`（约 200 行）财务专项
4. 新增 `deploy/grafana/dashboards/purchase.json`（约 200 行）采购专项
5. 修改 `deploy/grafana/provisioning/dashboards/dashboards.yml`（5 行）配置自动加载
6. 文档：`docs/2026-07-03-p11-7-business-dashboards.md`

**指标来源**：

- `backend/src/services/business_metrics.rs`（P4-3 已实现业务指标）
- Prometheus 指标：`order_total` / `inventory_level` / `payment_total` / `purchase_total`

#### 3.4.4 涉及文件清单

**读（3 个）**：

- `deploy/grafana/dashboards/erp-overview.json`（现有 Dashboard 模板）
- `backend/src/services/metrics_service.rs`（Prometheus 指标）
- `backend/src/services/business_metrics.rs`（业务指标）

**改（5 个）**：

- 新增 `deploy/grafana/dashboards/sales.json`（约 200 行）
- 新增 `deploy/grafana/dashboards/inventory.json`（约 200 行）
- 新增 `deploy/grafana/dashboards/finance.json`（约 200 行）
- 新增 `deploy/grafana/dashboards/purchase.json`（约 200 行）
- 修改 `deploy/grafana/provisioning/dashboards/dashboards.yml`（5 行）

#### 3.4.5 破坏性评估

| 维度 | 评估 | 详情 |
|------|------|------|
| 接口变更 | ✅ **无** | 仅 Dashboard JSON |
| 行为变更 | ✅ **无** | — |
| 数据库变更 | ✅ **无** | — |
| 配置变更 | ⚠️ **有新增** | Grafana provisioning 配置 |
| 部署变更 | ✅ **无破坏** | Dashboard 独立部署 |
| 兼容性 | ✅ **完全兼容** | 现有总览 Dashboard 不变 |
| 性能影响 | ✅ **无** | — |

**总体破坏性等级**：**🟢 低**

#### 3.4.6 回退方案

- 删除对应 JSON 文件，Grafana 自动不加载
- **回退耗时**：< 1min

#### 3.4.7 验证步骤

1. 单元测试：N/A（JSON 配置）
2. Grafana provisioning 验证：4 个 Dashboard 自动出现在列表
3. 数据验证：执行模拟业务操作，验证指标正确显示

#### 3.4.8 工时

| 阶段 | 工时 |
|------|------|
| sales.json | 0.5d |
| inventory.json | 0.5d |
| finance.json | 0.5d |
| purchase.json | 0.3d |
| 文档 | 0.2d |
| **合计** | **2.0d** |

---

### 3.5 M5 慢查询埋点需业务侧手动

#### 3.5.1 现状

经 `cat backend/src/middleware/slow_query.rs | head -30` 验证：

```rust
//! 由于 SeaORM/SQLx 的执行 hook 不暴露在应用层，本中间件通过业务层
//! 调用的 `SlowQueryRecorder::record()` 接入：
//!
//! 1. service 层在关键 SQL 前后调用 `SlowQueryRecorder::start()` 获取计时器
//! 2. `finish()` 时若耗时 > 100ms（可配置），记录到 `tracing::warn!`
```

经 `grep -n "SlowQueryRecorder" backend/src/services/ -r` 验证：**仅** 慢查询中间件自身使用，业务 service 几乎**未调用**。

**结论**：慢查询埋点机制已实现，但**未自动埋点**，业务层需手动集成。

#### 3.5.2 影响范围

| 维度 | 详情 |
|------|------|
| 受影响查询 | 业务 service 层手写 SQL 的查询 |
| 风险等级 | 中（慢查询漏报） |
| 业务影响 | 性能问题无法及时发现 |

#### 3.5.3 解决方案

**方案 A：通过 sqlx 的 tracing feature 自动埋点（采用）**

**实现步骤**：

1. 修改 `backend/Cargo.toml`（3 行）启用 sqlx `tracing` + `runtime-tokio-rustls` features
2. 修改 `backend/src/utils/app_state.rs`（10 行）注入 `SlowQueryConfig`
3. 修改 `backend/src/main.rs`（15 行）初始化 sqlx tracing subscriber
4. 修改 `backend/src/middleware/slow_query.rs`（30 行）保留手动 API + 新增自动埋点开关
5. 修改 `backend/src/utils/config.rs`（10 行）新增 `SLOW_QUERY_AUTO_INSTRUMENT` env
6. 文档：`docs/2026-07-05-p11-8-slow-query-auto.md`

**双轨方案**：

- **自动埋点**（opt-in）：`SLOW_QUERY_AUTO_INSTRUMENT=true` 时所有 SQL 自动埋点
- **手动埋点**（保留）：业务可继续使用 `SlowQueryRecorder::start()`

#### 3.5.4 涉及文件清单

**读（4 个）**：

- `backend/src/middleware/slow_query.rs`（现有实现）
- `backend/src/services/performance_optimizer.rs`（性能优化）
- `backend/src/services/metrics_service.rs`（Prometheus 指标）
- `backend/Cargo.toml`（依赖）

**改（3 个）**：

- 修改 `backend/Cargo.toml`（3 行）
- 修改 `backend/src/middleware/slow_query.rs`（30 行）
- 修改 `backend/src/utils/app_state.rs`（10 行）
- 修改 `backend/src/main.rs`（15 行）
- 修改 `backend/src/utils/config.rs`（10 行）

#### 3.5.5 破坏性评估

| 维度 | 评估 | 详情 |
|------|------|------|
| 接口变更 | ✅ **有兼容性** | 保留手动 API；新增自动埋点 |
| 行为变更 | ✅ **有兼容性** | 默认行为不变；`SLOW_QUERY_AUTO_INSTRUMENT=false` 时不启用 |
| 数据库变更 | ✅ **无** | — |
| 配置变更 | ⚠️ **有新增** | 1 个 env：`SLOW_QUERY_AUTO_INSTRUMENT` |
| 部署变更 | ✅ **无** | — |
| 兼容性 | ✅ **完全兼容** | 手动 API 不变 |
| 性能影响 | ⚠️ 略降 | 自动埋点增加 5% SQL 开销 |

**总体破坏性等级**：**🟡 中**（默认行为不变；启用后性能略降）

#### 3.5.6 回退方案

```bash
export SLOW_QUERY_AUTO_INSTRUMENT=false
```

**回退耗时**：< 1min

#### 3.5.7 验证步骤

1. 单元测试：5 个（自动埋点 / 手动埋点 / 阈值 / 指标暴露）
2. 集成测试：3 个（高负载下慢查询自动识别）
3. 性能验证：压测 SQL P99 延迟 +5% 内

#### 3.5.8 工时

| 阶段 | 工时 |
|------|------|
| sqlx tracing 集成 | 0.5d |
| 配置 + 开关 | 0.3d |
| 单元测试 | 0.4d |
| 集成测试 | 0.5d |
| 文档 | 0.3d |
| **合计** | **2.0d** |

---

## 四、低风险详细规划（5 项）

### 4.1 L1 缓存预热缺失

#### 4.1.1 现状

经 `grep -rn "warmup\|warm_\|preheat" backend/src/` 验证：**无结果**。

经 `cat backend/src/main.rs | grep -n "cache"` 验证：仅初始化 `AppCache`，**无**预热逻辑。

**结论**：缓存预热完全缺失。

#### 4.1.2 影响范围

| 维度 | 详情 |
|------|------|
| 受影响缓存 | 字典数据 / 配置 / 权限 |
| 风险等级 | 低（首次查询慢） |
| 业务影响 | 启动后 5~10 分钟命中率低 |

#### 4.1.3 解决方案

**方案 A：实现 CacheWarmer 启动钩子（采用）**

**实现步骤**：

1. 新增 `backend/src/services/cache_warmer.rs`（约 200 行）
2. 修改 `backend/src/main.rs`（10 行）启动时调用 `cache_warmer.warm_all().await`
3. 修改 `backend/src/utils/config.rs`（10 行）`CACHE_WARMER_ENABLED` + 列表配置
4. 文档：`docs/2026-07-09-p12-1-cache-warmer.md`

#### 4.1.4 涉及文件清单

**读（2 个）**：

- `backend/src/services/cache*.rs`（缓存服务）
- `backend/src/main.rs`（启动钩子）

**改（2 个）**：

- 新增 `backend/src/services/cache_warmer.rs`（约 200 行）
- 修改 `backend/src/main.rs`（10 行）
- 修改 `backend/src/utils/config.rs`（10 行）

#### 4.1.5 破坏性评估

| 维度 | 评估 | 详情 |
|------|------|------|
| 接口变更 | ✅ **无** | — |
| 行为变更 | ✅ **有兼容性** | 启动时间 +5~10s；命中率提高 |
| 数据库变更 | ✅ **无** | — |
| 配置变更 | ⚠️ **有新增** | 2 个 env：`CACHE_WARMER_ENABLED` / `CACHE_WARMER_KEYS` |
| 部署变更 | ✅ **无** | — |
| 兼容性 | ✅ **完全兼容** | 默认关闭 |
| 性能影响 | ✅ **正向** | 命中率提高后查询加速 |

**总体破坏性等级**：**🟢 低**

#### 4.1.6 回退方案

```bash
export CACHE_WARMER_ENABLED=false
```

#### 4.1.7 验证步骤

1. 单元测试：4 个（预热逻辑 / 失败重试 / 并发 / 指标）
2. 集成测试：2 个（启动后命中率提升）

#### 4.1.8 工时

| 阶段 | 工时 |
|------|------|
| 编码 | 0.5d |
| 单元测试 | 0.2d |
| 集成测试 | 0.2d |
| 文档 | 0.1d |
| **合计** | **1.0d** |

---

### 4.2 L2 i18n 后端缺失

#### 4.2.1 现状

经 `grep -rn "i18n\|I18n\|locale\|translation" backend/src/` 验证：**无结果**（后端确实缺失）。

经 `ls frontend/src/i18n/ 2>/dev/null` 验证：前端 i18n 框架可能已有（待 P12-2 验证）。

**结论**：后端 i18n 完全缺失。

#### 4.2.2 影响范围

| 维度 | 详情 |
|------|------|
| 受影响接口 | 全部返回错误消息的接口 |
| 风险等级 | 低（仅错误消息） |
| 业务影响 | 国际化用户看不懂中文错误消息 |

#### 4.2.3 解决方案

**方案 A：引入 `rust-i18n` crate（采用）**

**实现步骤**：

1. 修改 `backend/Cargo.toml`（1 行）新增 `rust-i18n = "3"`
2. 新增 `backend/src/utils/i18n.rs`（约 50 行）初始化 + 工具函数
3. 新增 `backend/locales/zh-CN.yml`（约 100 行）中文
4. 新增 `backend/locales/en-US.yml`（约 100 行）英文
5. 修改 `backend/src/handlers/*_handler.rs`（约 30 个文件）错误消息改用 `t!("error.xxx")`
6. 修改 `backend/src/utils/error.rs`（20 行）`AppError::Display` 走 i18n
7. 修改 `backend/src/main.rs`（5 行）初始化 i18n
8. 文档：`docs/2026-07-11-p12-2-i18n-backend.md`

#### 4.2.4 涉及文件清单

**读（N 个）**：

- `backend/src/main.rs`
- `backend/src/handlers/*_handler.rs`（约 30 个）
- `backend/Cargo.toml`

**改（N 个）**：

- 修改 `backend/Cargo.toml`（1 行）
- 新增 `backend/src/utils/i18n.rs`（约 50 行）
- 新增 `backend/locales/zh-CN.yml`（约 100 行）
- 新增 `backend/locales/en-US.yml`（约 100 行）
- 修改 `backend/src/handlers/*_handler.rs`（约 30 个文件）
- 修改 `backend/src/utils/error.rs`（20 行）
- 修改 `backend/src/main.rs`（5 行）

#### 4.2.5 破坏性评估

| 维度 | 评估 | 详情 |
|------|------|------|
| 接口变更 | ✅ **有兼容性** | 默认中文；新增 `Accept-Language: en-US` 切换 |
| 行为变更 | ✅ **有兼容性** | 默认行为不变 |
| 数据库变更 | ✅ **无** | — |
| 配置变更 | ⚠️ **有新增** | 1 个 env：`I18N_ENABLED` / `I18N_DEFAULT_LOCALE` |
| 部署变更 | ✅ **无** | — |
| 兼容性 | ✅ **完全兼容** | 默认中文 |
| 性能影响 | ✅ **无** | i18n 仅在错误路径触发 |

**总体破坏性等级**：**🟡 中**（30+ handler 需调整）

#### 4.2.6 回退方案

```bash
export I18N_ENABLED=false
```

#### 4.2.7 验证步骤

1. 单元测试：5 个（i18n 切换 / 默认语言 / 缺失 key fallback）
2. 集成测试：3 个（中英文错误消息对比）

#### 4.2.8 工时

| 阶段 | 工时 |
|------|------|
| 依赖 + 初始化 | 0.2d |
| Locale 文件 | 0.3d |
| handler 改造（30 个文件） | 1.0d |
| 单元测试 | 0.2d |
| 集成测试 | 0.2d |
| 文档 | 0.1d |
| **合计** | **2.0d** |

---

### 4.3 L3 Debug 工具残留

#### 4.3.1 现状

经 `grep -n "dbg!\|debug_" backend/src/handlers/{fund_management,purchase_receipt,supplier,quality_standard,purchase_contract}_handler.rs` 验证：实际为 `#[axum::debug_handler]` 属性宏（共 24 处），**非** `dbg!()` 宏。

**P10-1 报告**描述的"5 个 handler 有 dbg!/debug_ 使用"实际指向 axum 官方提供的 `#[axum::debug_handler]` 属性，用于在开发模式下提供更详细的错误信息。

**`#[axum::debug_handler]` 工作原理**：

- 仅在 `debug_assertions` 启用时（即 `cargo build` 不带 `--release`）生效
- 在 release 构建中，宏展开为空
- **不会**进入生产二进制
- **不会**引入运行时开销

**结论**：这些不是真"残留 debug 代码"——是 axum 推荐的开发辅助。但仍可考虑：

1. **保留**（推荐）：开发体验更友好；release 不生效
2. **移除**（保守）：如果担心误用，全部移除

#### 4.3.2 影响范围

| 维度 | 详情 |
|------|------|
| 受影响文件 | 5 个 handler 文件 |
| 影响属性 | 24 处 `#[axum::debug_handler]` |
| 风险等级 | 低（实际不进入 release） |
| 业务影响 | 无（开发体验影响） |

#### 4.3.3 解决方案

**方案 A：保留（推荐）+ 文档说明（采用）**

**理由**：

1. `#[axum::debug_handler]` 是 axum 官方推荐属性
2. release 构建自动空展开
3. 删除反而降低开发体验

**方案 B：保守移除 + 文档化理由（备选）**

如果团队约定禁止 `#[axum::debug_handler]`，可全部移除。

**采用方案 A**：

1. 在 `docs/2026-06-17-style-guide.md` 新增"开发属性使用规范"章节
2. 说明 `#[axum::debug_handler]` 允许保留（不影响生产）
3. 说明 `dbg!()` / `println!()` / `eprintln!()` 必须移除
4. CI 增加 `clippy::dbg_macro` 检查（已默认启用）

#### 4.3.4 涉及文件清单

**读（5 个）**：

- `backend/src/handlers/fund_management_handler.rs`（5 处 `#[axum::debug_handler]`）
- `backend/src/handlers/purchase_receipt_handler.rs`（4 处）
- `backend/src/handlers/supplier_handler.rs`（6 处）
- `backend/src/handlers/quality_standard_handler.rs`（7 处）
- `backend/src/handlers/purchase_contract_handler.rs`（2 处）

**改（1 个）**：

- 修改 `docs/2026-06-17-style-guide.md`（新增 1 章节，约 50 行）

#### 4.3.5 破坏性评估

| 维度 | 评估 | 详情 |
|------|------|------|
| 接口变更 | ✅ **无** | — |
| 行为变更 | ✅ **无** | 文档化变更 |
| 数据库变更 | ✅ **无** | — |
| 配置变更 | ✅ **无** | — |
| 部署变更 | ✅ **无** | — |
| 兼容性 | ✅ **完全兼容** | — |
| 性能影响 | ✅ **无** | — |

**总体破坏性等级**：**🟢 无**

#### 4.3.6 回退方案

- 文档修改 revert 即可
- **回退耗时**：< 1min

#### 4.3.7 验证步骤

1. 单元测试：N/A（仅文档）
2. CI 验证：`cargo clippy -- -D warnings`（`dbg_macro` 已在默认）
3. 文档评审：团队 review

#### 4.3.8 工时

| 阶段 | 工时 |
|------|------|
| 文档编写 | 0.3d |
| 评审 | 0.2d |
| **合计** | **0.5d** |

#### 4.3.9 备注

- **重要发现**：`#[axum::debug_handler]` 不是残留 debug 代码
- P10-1 报告中的描述有歧义（"5 个 handler 有 dbg!/debug_ 使用"），实际**全部**为 `#[axum::debug_handler]`，无 `dbg!()` 宏使用
- 处理建议：**保留** + 文档化

---

### 4.4 L4 Feature Flag 缺失

#### 4.4.1 现状

经 `grep -rn "feature_flag\|FeatureFlag\|FF_" backend/src/` 验证：**无结果**。

**结论**：Feature Flag 完全缺失。

#### 4.4.2 影响范围

| 维度 | 详情 |
|------|------|
| 受影响功能 | 灰度发布、A/B 测试、紧急回退 |
| 风险等级 | 低 |
| 业务影响 | 新功能上线无灰度能力 |

#### 4.4.3 解决方案

**方案 A：实现简单 `FeatureFlag` 工具（采用）**

**实现步骤**：

1. 新增 `backend/src/utils/feature_flag.rs`（约 100 行）
2. 修改 `backend/src/utils/config.rs`（10 行）`FEATURE_*` env 解析
3. 文档：`docs/2026-07-14-p12-4-feature-flag.md`

**API 设计**：

```rust
pub struct FeatureFlag {
    flags: HashMap<String, bool>,
}

impl FeatureFlag {
    pub fn is_enabled(&self, name: &str) -> bool;
    pub fn enable(&mut self, name: &str);
    pub fn disable(&mut self, name: &str);
}

// 业务使用
if feature_flag.is_enabled("new_dashboard") {
    // 新 Dashboard
} else {
    // 旧 Dashboard
}
```

#### 4.4.4 涉及文件清单

**读（1 个）**：

- `backend/src/utils/config.rs`（env 加载）

**改（2 个）**：

- 新增 `backend/src/utils/feature_flag.rs`（约 100 行）
- 修改 `backend/src/utils/config.rs`（10 行）

#### 4.4.5 破坏性评估

| 维度 | 评估 | 详情 |
|------|------|------|
| 接口变更 | ✅ **有兼容性** | 新增工具；无破坏 |
| 行为变更 | ✅ **无** | 默认 false |
| 数据库变更 | ✅ **无** | — |
| 配置变更 | ⚠️ **有新增** | N 个 env：`FEATURE_<name>=true\|false` |
| 部署变更 | ✅ **无** | — |
| 兼容性 | ✅ **完全兼容** | — |
| 性能影响 | ✅ **无** | HashMap 查询 O(1) |

**总体破坏性等级**：**🟢 低**

#### 4.4.6 回退方案

- 工具可独立删除
- **回退耗时**：< 1min

#### 4.4.7 验证步骤

1. 单元测试：6 个（启用 / 禁用 / 缺失 / 默认值 / 运行时切换 / 并发）
2. 集成测试：2 个（业务使用示例）

#### 4.4.8 工时

| 阶段 | 工时 |
|------|------|
| 编码 | 0.4d |
| 单元测试 | 0.3d |
| 集成测试 | 0.2d |
| 文档 | 0.1d |
| **合计** | **1.0d** |

---

### 4.5 L5 多区域支持缺失

#### 4.5.1 现状

经 `grep -rn "region" backend/src/{handlers/bi_handler.rs,routes/analytics.rs,services/bi_analysis_service.rs,services/enhanced_logger.rs}` 验证：

- `bi_handler.rs` 第 4 / 86 / 88 / 93 行：BI 报表按区域聚合
- `routes/analytics.rs` 第 457 / 476~477 行：暴露 `/sales/by-region` 路由
- `bi_analysis_service.rs` 第 77 / 246 / 252 行：区域统计服务
- `enhanced_logger.rs`：日志 region 标签

**结论**：仅 BI 报表使用 region 概念，**无**全局多区域路由支持。

#### 4.5.2 影响范围

| 维度 | 详情 |
|------|------|
| 受影响功能 | 跨区域部署 |
| 风险等级 | 低 |
| 业务影响 | 无法支持多地数据中心 |

#### 4.5.3 解决方案

**方案 A：实现 region 配置 + routing（采用）**

**实现步骤**：

1. 新增 `backend/src/utils/region.rs`（约 100 行）`RegionRouter`
2. 修改 `backend/src/utils/config.rs`（10 行）`CURRENT_REGION` + `REGIONS` env
3. 修改 `backend/src/main.rs`（10 行）启动时确定 region
4. 修改 `backend/src/middleware/api_gateway.rs`（20 行）按 region 路由
5. 文档：`docs/2026-07-16-p12-5-multi-region.md`

#### 4.5.4 涉及文件清单

**读（4 个）**：

- `backend/src/handlers/bi_handler.rs`
- `backend/src/routes/analytics.rs`
- `backend/src/services/bi_analysis_service.rs`
- `backend/src/services/enhanced_logger.rs`

**改（4 个）**：

- 新增 `backend/src/utils/region.rs`（约 100 行）
- 修改 `backend/src/utils/config.rs`（10 行）
- 修改 `backend/src/main.rs`（10 行）
- 修改 `backend/src/middleware/api_gateway.rs`（20 行）

#### 4.5.5 破坏性评估

| 维度 | 评估 | 详情 |
|------|------|------|
| 接口变更 | ✅ **有兼容性** | 默认 region 走原路径 |
| 行为变更 | ✅ **有兼容性** | 默认行为不变 |
| 数据库变更 | ✅ **无** | — |
| 配置变更 | ⚠️ **有新增** | 3 个 env：`CURRENT_REGION` / `REGIONS` / `MULTI_REGION_ENABLED` |
| 部署变更 | ⚠️ **有新增** | 多区域部署需要 DNS / 负载均衡 |
| 兼容性 | ✅ **完全兼容** | 默认 region 不变 |
| 性能影响 | ✅ **无** | — |

**总体破坏性等级**：**🟡 中**（部署需多区域）

#### 4.5.6 回退方案

```bash
export MULTI_REGION_ENABLED=false
```

#### 4.5.7 验证步骤

1. 单元测试：5 个（region 解析 / 路由 / 默认 / 异常 / 日志标签）
2. 集成测试：3 个（多 region 模拟 / 跨 region 调用 / 故障转移）
3. 部署验证：本地起 2 个实例（region=cn-north / region=cn-east），验证路由

#### 4.5.8 工时

| 阶段 | 工时 |
|------|------|
| RegionRouter 实现 | 0.8d |
| 配置 + 集成 | 0.4d |
| 单元测试 | 0.4d |
| 集成测试 | 1.0d |
| 文档 | 0.4d |
| **合计** | **3.0d** |

---

## 五、批次实施计划

### 5.1 批次 1：高风险 P11 批 1（6d，PR #172-#174）

**目标**：消除 3 项高风险（安全核心）

**PR #172 - H1 CSRF 中间件**（2d）

- 任务：实现 CSRF 中间件 + 强制校验
- 分支：`trae/solo-agent-P11-1-csrf-middleware`
- 文档：`docs/2026-06-18-p11-1-csrf-middleware.md`
- 关键路径：新增 `backend/src/middleware/csrf.rs` → 修改 `main.rs` 挂载 → 修改 `auth_handler.rs` Set-Cookie

**PR #173 - H2 Kafka 真实集成**（3d）

- 任务：替换 Mock 为 rdkafka
- 分支：`trae/solo-agent-P11-2-kafka-real-integration`
- 文档：`docs/2026-06-20-p11-2-kafka-real-integration.md`
- 关键路径：添加 rdkafka 依赖 → 实现 `RealKafkaProvider` → 集成到 `EventBus` → docker 部署验证

**PR #174 - H3 dead_code 清理**（1d）

- 任务：7 个 middleware 挂载 + 移除 dead_code
- 分支：`trae/solo-agent-P11-3-dead-code-cleanup`
- 文档：`docs/2026-06-22-p11-3-dead-code-cleanup.md`
- 关键路径：按 logger → security_headers → auth_context → tenant → permission → operation_log → api_gateway 顺序挂载

### 5.2 批次 2：中风险 A P11 批 2（5d，PR #175-#177）

**PR #175 - M1 限流 env 化**（1d）

- 任务：限流阈值从 env 加载
- 分支：`trae/solo-agent-P11-4-rate-limit-config`
- 文档：`docs/2026-06-26-p11-4-rate-limit-config.md`

**PR #176 - M2 读写分离**（3d）

- 任务：实现主从分离
- 分支：`trae/solo-agent-P11-5-read-write-split`
- 文档：`docs/2026-06-28-p11-5-read-write-split.md`

**PR #177 - M3 告警分级**（1d）

- 任务：P0/P1/P2 告警分级
- 分支：`trae/solo-agent-P11-6-alert-priority`
- 文档：`docs/2026-06-30-p11-6-alert-priority.md`

### 5.3 批次 3：中风险 B P11 批 3（4d，PR #178-#179）

**PR #178 - M4 业务专项 Dashboard**（2d）

- 任务：4 个业务 Dashboard
- 分支：`trae/solo-agent-P11-7-business-dashboards`
- 文档：`docs/2026-07-03-p11-7-business-dashboards.md`

**PR #179 - M5 慢查询自动埋点**（2d）

- 任务：sqlx tracing 自动埋点
- 分支：`trae/solo-agent-P11-8-slow-query-auto`
- 文档：`docs/2026-07-05-p11-8-slow-query-auto.md`

### 5.4 批次 4：低风险 A P12 批 4（3.5d，PR #180-#182）

**PR #180 - L1 缓存预热**（1d）

- 任务：CacheWarmer 启动钩子
- 分支：`trae/solo-agent-P12-1-cache-warmer`
- 文档：`docs/2026-07-09-p12-1-cache-warmer.md`

**PR #181 - L2 i18n 后端**（2d）

- 任务：rust-i18n 集成
- 分支：`trae/solo-agent-P12-2-i18n-backend`
- 文档：`docs/2026-07-11-p12-2-i18n-backend.md`

**PR #182 - L3 Debug 工具文档化**（0.5d）

- 任务：文档化 `#[axum::debug_handler]` 保留决策
- 分支：`trae/solo-agent-P12-3-debug-doc`
- 文档：`docs/2026-07-12-p12-3-debug-doc.md`（更新 style-guide）

### 5.5 批次 5：低风险 B P12 批 5（4d，PR #183-#184）

**PR #183 - L4 Feature Flag**（1d）

- 任务：FeatureFlag 工具
- 分支：`trae/solo-agent-P12-4-feature-flag`
- 文档：`docs/2026-07-14-p12-4-feature-flag.md`

**PR #184 - L5 多区域支持**（3d）

- 任务：RegionRouter + 部署
- 分支：`trae/solo-agent-P12-5-multi-region`
- 文档：`docs/2026-07-16-p12-5-multi-region.md`

---

## 六、依赖关系图

```
                        ┌──────────────────────────────┐
                        │      P10-1 评估报告（基线）    │
                        └──────────────────────────────┘
                                       │
        ┌──────────────────────────────┼──────────────────────────────┐
        │                              │                              │
        ▼                              ▼                              ▼
┌──────────────┐              ┌──────────────┐              ┌──────────────┐
│ H1 CSRF      │              │ H2 Kafka     │              │ H3 dead_code │
│ (PR #172)    │              │ (PR #173)    │              │ (PR #174)    │
└──────────────┘              └──────────────┘              └──────────────┘
        │                              │                              │
        └──────────────────────────────┴──────────────────────────────┘
                                       │
                                       ▼
                        ┌──────────────────────────────┐
                        │   P11 批 1 完成后（88 分）    │
                        └──────────────────────────────┘
                                       │
        ┌──────────────────────────────┼──────────────────────────────┐
        │                              │                              │
        ▼                              ▼                              ▼
┌──────────────┐              ┌──────────────┐              ┌──────────────┐
│ M1 限流      │              │ M2 读写分离  │              │ M3 告警分级  │
│ (PR #175)    │              │ (PR #176)    │              │ (PR #177)    │
└──────────────┘              └──────────────┘              └──────────────┘
        │                              │                              │
        └──────────────────────────────┴──────────────────────────────┘
                                       │
                                       ▼
                        ┌──────────────────────────────┐
                        │   P11 批 2 完成后（91 分）    │
                        └──────────────────────────────┘
                                       │
        ┌──────────────────────────────┴──────────────────────────────┐
        │                                                              │
        ▼                                                              ▼
┌──────────────┐                                              ┌──────────────┐
│ M4 Dashboard │                                              │ M5 慢查询    │
│ (PR #178)    │                                              │ (PR #179)    │
└──────────────┘                                              └──────────────┘
        │                                                              │
        └──────────────────────────────┬──────────────────────────────┘
                                       │
                                       ▼
                        ┌──────────────────────────────┐
                        │   P11 批 3 完成后（93 分）    │
                        └──────────────────────────────┘
                                       │
        ┌──────────────────────────────┼──────────────────────────────┐
        │                              │                              │
        ▼                              ▼                              ▼
┌──────────────┐              ┌──────────────┐              ┌──────────────┐
│ L1 缓存预热  │              │ L2 i18n      │              │ L3 Debug 文档│
│ (PR #180)    │              │ (PR #181)    │              │ (PR #182)    │
└──────────────┘              └──────────────┘              └──────────────┘
        │                              │                              │
        └──────────────────────────────┴──────────────────────────────┘
                                       │
                                       ▼
                        ┌──────────────────────────────┐
                        │   P12 批 4 完成后（95 分）    │
                        └──────────────────────────────┘
                                       │
        ┌──────────────────────────────┼──────────────────────────────┐
        │                              │                              │
        ▼                              ▼                              │
┌──────────────┐              ┌──────────────┐                       │
│ L4 Feature   │              │ L5 多区域    │                       │
│ (PR #183)    │              │ (PR #184)    │                       │
└──────────────┘              └──────────────┘                       │
        │                              │                              │
        └──────────────────────────────┴──────────────────────────────┘
                                       │
                                       ▼
                        ┌──────────────────────────────┐
                        │   P12 批 5 完成后（96 分）    │
                        └──────────────────────────────┘
```

**关键路径**：

- H1 / H2 / H3 之间**无依赖**（可并行）
- M1 / M2 / M3 之间**无依赖**（可并行）
- M4 / M5 之间**无依赖**（可并行）
- L1 / L2 / L3 之间**无依赖**（可并行）
- L4 / L5 之间**无依赖**（可并行）
- 批次之间**强依赖**（必须按顺序）

---

## 七、风险与缓解

### 7.1 实施风险

| 风险 | 等级 | 影响 | 缓解 |
|------|------|------|------|
| H1 CSRF 前端未同步适配 | 🔴 高 | 写接口 403 | 1. 与前端团队协调 2. 提供 1 周灰度期 3. `CSRF_ENABLED=false` 紧急回退 |
| H2 Kafka cmake 编译失败 | 🟡 中 | CI 失败 | 1. CI 镜像预装 cmake / libssl-dev 2. 备选 `dynamic-linking` 特性 |
| H2 Kafka 集群不可用 | 🟡 中 | 事件丢失 | 1. fallback 到 Mock 2. 三副本 + KRaft |
| H3 中间件挂载触发 403 | 🟡 中 | 业务中断 | 1. 分 7 PR 独立合入 2. 每次合入后立即观察日志 |
| M1 限流默认值不匹配 | 🟢 低 | 限流过严/过松 | 默认值与硬编码一致 |
| M2 主从延迟 | 🟡 中 | 数据不一致 | 强一致读标记（`db.read_consistent()`） |
| M2 从库故障 | 🟡 中 | 读失败 | 自动故障转移回 master |
| M3 AlertManager 部署失败 | 🟢 低 | 告警无法分级 | 旧告警规则保留不变 |
| M4 Dashboard 指标缺失 | 🟢 低 | 图表空白 | 与 metrics_service 协调补全 |
| M5 sqlx tracing 性能 | 🟢 低 | 5% 开销 | opt-in 开关 |
| L1 缓存预热超时 | 🟢 低 | 启动慢 | 限制单次预热 1s |
| L2 i18n 翻译不全 | 🟢 低 | 部分英文缺失 | 缺失 key fallback 到中文 |
| L3 Debug 属性误用 | 🟢 低 | 开发体验 | clippy 检查 |
| L4 Feature Flag 误用 | 🟢 低 | 灰度异常 | 文档化 + 团队培训 |
| L5 多区域路由错误 | 🟡 中 | 请求到错误区域 | 单元测试覆盖 |

### 7.2 回退策略

**总原则**：

- 每项任务保留 env 开关，可独立回退
- 每个 PR 独立 commit，可独立 revert
- 高风险任务保留"软启动"期（1 周灰度）

**回退时效**：

- 配置文件回退：< 1min
- 代码回退（revert）：< 5min
- 数据库回退：< 30min（含数据迁移）
- 部署回退：< 1h（含服务重启）

### 7.3 资源依赖

**人员**：

- 1 名 Rust 后端工程师（5 周全时）
- 0.5 名前端工程师（仅 H1 CSRF 适配）
- 0.2 名 DBA（M2 主从配置）
- 0.2 名 SRE（Kafka / AlertManager 部署）

**基础设施**：

- Kafka 3 broker 集群（H2）
- PostgreSQL 主从（M2）
- AlertManager 容器（M3）
- 监控存储扩展（Dashboard JSON）

**预算**：

- 云资源增加：~ ¥3,000/月（Kafka 集群 + 从库）
- 人力成本：5 周 × 1.5 人 = 7.5 人周

---

## 八、附录

### 8.1 文件清单汇总表

| 风险 | 读文件数 | 改文件数 | 新增文件数 | 涉及文件总计 |
|------|---------|---------|----------|------------|
| H1 CSRF | 7 | 6 | 1 | 14 |
| H2 Kafka | 5 | 6 | 1 | 12 |
| H3 dead_code | 9 | 8 | 0 | 17 |
| M1 限流 | 3 | 4 | 0 | 7 |
| M2 读写分离 | 3 | 50+ | 1 | 54+ |
| M3 告警 | 2 | 4 | 0 | 6 |
| M4 Dashboard | 3 | 5 | 0 | 8 |
| M5 慢查询 | 4 | 5 | 0 | 9 |
| L1 缓存预热 | 2 | 3 | 0 | 5 |
| L2 i18n | 30+ | 32+ | 2 | 64+ |
| L3 Debug | 5 | 1 | 0 | 6 |
| L4 Feature Flag | 1 | 2 | 0 | 3 |
| L5 多区域 | 4 | 4 | 0 | 8 |
| **合计** | **78+** | **130+** | **5** | **213+** |

### 8.2 配置项汇总表

| 风险 | 新增 env 变量 | 数量 |
|------|--------------|------|
| H1 CSRF | `CSRF_SECRET` / `CSRF_HEADER` / `CSRF_COOKIE` / `CSRF_ENABLED` | 4 |
| H2 Kafka | `KAFKA_BROKERS` / `KAFKA_SASL_USERNAME` / `KAFKA_SASL_PASSWORD` / `KAFKA_REAL_ENABLED` | 4 |
| H3 dead_code | 7 个 middleware 独立开关 | 7 |
| M1 限流 | `RATE_LIMIT_GLOBAL_MAX` / `RATE_LIMIT_GLOBAL_WINDOW_SECS` / `RATE_LIMIT_BRUTEFORCE_MAX` / `RATE_LIMIT_BRUTEFORCE_WINDOW_SECS` | 4 |
| M2 读写分离 | `DATABASE_MASTER_URL` / `DATABASE_SLAVE_URLS` / `READ_POOL_ENABLED` / `READ_POOL_MAX_CONNECTIONS` | 4 |
| M3 告警 | N/A（Prometheus label） | 0 |
| M4 Dashboard | N/A（Grafana provisioning） | 0 |
| M5 慢查询 | `SLOW_QUERY_AUTO_INSTRUMENT` | 1 |
| L1 缓存预热 | `CACHE_WARMER_ENABLED` / `CACHE_WARMER_KEYS` | 2 |
| L2 i18n | `I18N_ENABLED` / `I18N_DEFAULT_LOCALE` | 2 |
| L3 Debug | N/A | 0 |
| L4 Feature Flag | `FEATURE_<name>=true\|false` | N |
| L5 多区域 | `CURRENT_REGION` / `REGIONS` / `MULTI_REGION_ENABLED` | 3 |
| **合计** | — | **31+N** |

### 8.3 数据库变更清单

| 风险 | 数据库变更 | 备注 |
|------|----------|------|
| H1 CSRF | 无 | 仅 Redis 缓存 |
| H2 Kafka | 无 | Kafka 独立服务 |
| H3 dead_code | 无 | — |
| M1 限流 | 无 | — |
| M2 读写分离 | **有** | PostgreSQL 主从流复制 |
| M3 告警 | 无 | — |
| M4 Dashboard | 无 | — |
| M5 慢查询 | 无 | — |
| L1 缓存预热 | 无 | — |
| L2 i18n | 无 | — |
| L3 Debug | 无 | — |
| L4 Feature Flag | 无 | — |
| L5 多区域 | 无 | — |
| **合计** | **1 项** | M2 需主从 |

### 8.4 部署变更清单

| 风险 | 部署变更 | 备注 |
|------|---------|------|
| H1 CSRF | 无 | — |
| H2 Kafka | **有** | `deploy/kafka/docker-compose.yml`（已就绪） |
| H3 dead_code | 无 | — |
| M1 限流 | 无 | — |
| M2 读写分离 | **有** | 需 PostgreSQL 从库 |
| M3 告警 | **有** | 需 AlertManager 容器 |
| M4 Dashboard | **有** | Grafana provisioning |
| M5 慢查询 | 无 | — |
| L1 缓存预热 | 无 | — |
| L2 i18n | 无 | — |
| L3 Debug | 无 | — |
| L4 Feature Flag | 无 | — |
| L5 多区域 | **有** | 多区域 DNS / 负载均衡 |
| **合计** | **5 项** | — |

### 8.5 优先级矩阵

| 等级 | 高破坏性 | 中破坏性 | 低破坏性 | 合计 |
|------|---------|---------|---------|------|
| P0 高 | H1, H2 | H3 | — | 3 |
| P1 中 | M2 | M5 | M1, M3, M4 | 5 |
| P2 低 | — | L2, L5 | L1, L3, L4 | 5 |
| **合计** | **3** | **4** | **6** | **13** |

### 8.6 工时分布

| 风险 | 开发 | 测试 | 文档 | 合计 |
|------|------|------|------|------|
| H1 CSRF | 1.5d | 0.3d | 0.2d | 2.0d |
| H2 Kafka | 2.0d | 0.8d | 0.2d | 3.0d |
| H3 dead_code | 0.5d | 0.4d | 0.1d | 1.0d |
| M1 限流 | 0.6d | 0.2d | 0.2d | 1.0d |
| M2 读写分离 | 2.0d | 0.8d | 0.2d | 3.0d |
| M3 告警 | 0.6d | 0.2d | 0.2d | 1.0d |
| M4 Dashboard | 1.5d | 0.3d | 0.2d | 2.0d |
| M5 慢查询 | 1.3d | 0.4d | 0.3d | 2.0d |
| L1 缓存预热 | 0.7d | 0.2d | 0.1d | 1.0d |
| L2 i18n | 1.5d | 0.4d | 0.1d | 2.0d |
| L3 Debug | 0.0d | 0.0d | 0.5d | 0.5d |
| L4 Feature Flag | 0.6d | 0.3d | 0.1d | 1.0d |
| L5 多区域 | 1.6d | 1.0d | 0.4d | 3.0d |
| **合计** | **14.4d** | **5.3d** | **2.8d** | **22.5d** |

### 8.7 PR 编号分配表

| PR 编号 | 风险 | 分支 | 标题 | 预计合入日期 |
|---------|------|------|------|------------|
| #172 | H1 | `trae/solo-agent-P11-1-csrf-middleware` | feat(P11-1): CSRF 中间件强制校验 | 2026-06-19 |
| #173 | H2 | `trae/solo-agent-P11-2-kafka-real-integration` | feat(P11-2): Kafka 真实集成（rdkafka） | 2026-06-22 |
| #174 | H3 | `trae/solo-agent-P11-3-dead-code-cleanup` | refactor(P11-3): 7 个 middleware 挂载 + 移除 dead_code | 2026-06-23 |
| #175 | M1 | `trae/solo-agent-P11-4-rate-limit-config` | refactor(P11-4): 限流阈值 env 化 | 2026-06-26 |
| #176 | M2 | `trae/solo-agent-P11-5-read-write-split` | feat(P11-5): PostgreSQL 读写分离 | 2026-06-29 |
| #177 | M3 | `trae/solo-agent-P11-6-alert-priority` | feat(P11-6): 告警 P0/P1/P2 分级 | 2026-06-30 |
| #178 | M4 | `trae/solo-agent-P11-7-business-dashboards` | feat(P11-7): 4 个业务专项 Dashboard | 2026-07-04 |
| #179 | M5 | `trae/solo-agent-P11-8-slow-query-auto` | feat(P11-8): 慢查询 sqlx tracing 自动埋点 | 2026-07-06 |
| #180 | L1 | `trae/solo-agent-P12-1-cache-warmer` | feat(P12-1): 缓存预热启动钩子 | 2026-07-09 |
| #181 | L2 | `trae/solo-agent-P12-2-i18n-backend` | feat(P12-2): i18n 后端（rust-i18n） | 2026-07-11 |
| #182 | L3 | `trae/solo-agent-P12-3-debug-doc` | docs(P12-3): Debug 工具使用规范 | 2026-07-12 |
| #183 | L4 | `trae/solo-agent-P12-4-feature-flag` | feat(P12-4): Feature Flag 工具 | 2026-07-14 |
| #184 | L5 | `trae/solo-agent-P12-5-multi-region` | feat(P12-5): 多区域支持（RegionRouter） | 2026-07-17 |

### 8.8 验证项总表

| 风险 | 单元测试 | 集成测试 | 端到端 | 性能测试 | 回归测试 |
|------|---------|---------|--------|---------|---------|
| H1 CSRF | 8 | 4 | ✅ | ✅ | ✅ |
| H2 Kafka | 5 | 5 | ✅ | ✅ | ✅ |
| H3 dead_code | 35 | 7 | ✅ | ✅ | ✅ |
| M1 限流 | 4 | 1 | — | — | ✅ |
| M2 读写分离 | 5 | 10 | ✅ | ✅ | ✅ |
| M3 告警 | 0 | 0 | ✅ | — | — |
| M4 Dashboard | 0 | 0 | ✅ | — | — |
| M5 慢查询 | 5 | 3 | — | ✅ | ✅ |
| L1 缓存预热 | 4 | 2 | — | ✅ | — |
| L2 i18n | 5 | 3 | — | — | — |
| L3 Debug | 0 | 0 | — | — | — |
| L4 Feature Flag | 6 | 2 | — | — | — |
| L5 多区域 | 5 | 3 | ✅ | — | — |
| **合计** | **82** | **40** | **6** | **6** | **6** |

### 8.9 关键决策记录

#### 决策 1：H1 CSRF 采用 Synchronizer Token 模式而非 SameSite Cookie

- **理由**：SameSite 仅依赖浏览器行为，curl / Postman / 移动端可绕过
- **决策时间**：2026-06-17
- **决策者**：风险解决规划评估子代理

#### 决策 2：H2 Kafka 保留 Mock 作为 fallback

- **理由**：Kafka 故障时业务仍可运行（不阻塞）
- **决策时间**：2026-06-17
- **决策者**：风险解决规划评估子代理

#### 决策 3：H3 7 个 middleware 分 7 个独立 PR

- **理由**：避免一次性大爆炸；可独立回退
- **决策时间**：2026-06-17
- **决策者**：风险解决规划评估子代理

#### 决策 4：L3 Debug 工具保留 + 文档化

- **理由**：`#[axum::debug_handler]` 是 axum 官方推荐；release 不生效
- **决策时间**：2026-06-17
- **决策者**：风险解决规划评估子代理
- **依据**：经 `grep -n "dbg!\|debug_" backend/src/handlers/*.rs` 验证，全部为 `#[axum::debug_handler]`，无 `dbg!()` 宏

### 8.10 后续阶段规划

完成 P10-2（规划评估）+ P11~P12（实施）后，建议进入：

- **P13 性能压测**：模拟 1000+ 并发用户
- **P14 安全审计**：第三方安全公司渗透测试
- **P15 灾备演练**：主库故障切换 + Kafka 集群故障切换
- **P16 GA 发布**：1.0 正式版发布

---

## 文档元信息

| 字段 | 值 |
|------|---|
| 文档标题 | 冰溪 ERP 13 项风险解决规划评估报告（P10-2） |
| 文档版本 | v1.0 |
| 创建时间 | 2026-06-17 |
| 创建者 | 风险解决规划评估子代理 |
| 分支 | `trae/solo-agent-P10-2-risk-mitigation-plan` |
| 评估范围 | test 分支 HEAD f38ba22 |
| 关联文档 | `docs/2026-06-17-p10-1-security-performance-usability.md` |
| 计划实施文档 | 13 个 PR + 13 个 `docs/2026-06-1X-p11-*.md` / `docs/2026-07-0X-p12-*.md` |
| 预计评估分提升 | 85 → 96（+11） |
| 预计工时 | 22.5d（约 5 周） |
| 预计 PR 数量 | 13 |
| 状态 | 待评审 |

---

**报告结束**
