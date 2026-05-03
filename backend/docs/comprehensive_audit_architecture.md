# 秉羲 ERP 全维度综合审计体系架构与数据模型设计文档

## 1. 架构概览

为了满足全维度（代码层、UI层、用户层）的审计需求，且保证高性能（异步）、防篡改（数字签名）与分级存储，我们设计了一套名为 **OmniAudit** 的综合审计子系统。

### 1.1 核心组件
1. **采集层 (Collectors)**:
   - **后端拦截器 (Backend Middleware)**: 利用 Axum 中间件和 SeaORM 钩子拦截 API 请求、SQL 变更。
   - **前端探针 (Frontend Tracker)**: Yew 应用内挂载的全局事件监听器（点击、路由跳转、错误捕捉）。
2. **传输层 (Transport)**:
   - 后端使用 `tokio::sync::mpsc` (多生产者单消费者) 通道，实现无阻塞异步日志投递。
3. **处理与存储层 (Processor & Storage)**:
   - **签名引擎**: 对关键数据计算 HMAC-SHA256，防止数据库被直接篡改。
   - **告警引擎**: 正则与阈值匹配，触发控制台或外部通知。
   - **分级存储引擎**: 
     - **热数据 (0-7天)**: 存储在 Redis 或高频查询的主数据库表中。
     - **温数据 (8-30天)**: 自动归档至 PostgreSQL 分区表。
     - **冷数据 (>30天)**: 导出为离线文件 (CSV/Parquet) 或转存入 OSS 归档。

## 2. 审计数据模型设计 (Data Models)

### 2.1 综合审计日志表 (`omni_audit_logs`)
这是记录所有维度行为的核心事实表。

| 字段名 | 类型 | 说明 | 维度关联 |
|--------|------|------|----------|
| `id` | BIGINT | 主键自增 | - |
| `trace_id` | VARCHAR(64) | 全链路追踪ID，关联前后端请求 | 统一性审计 |
| `user_id` | INT | 操作人ID，未登录为0 | 用户层审计 |
| `event_type` | VARCHAR(32) | 事件分类: UI_CLICK, PAGE_VIEW, API_CALL, SQL_MUTATION, SECURITY_ALERT | 所有维度 |
| `event_name` | VARCHAR(128)| 事件名称，如: `login_attempt`, `approve_order` | 功能审计 |
| `resource` | VARCHAR(128)| 涉及资源，如表名、URL、按钮ID | 按钮/业务审计 |
| `action` | VARCHAR(32) | 动作类型: CREATE, UPDATE, DELETE, VIEW, CLICK | - |
| `payload` | JSONB | 事件上下文，含请求参数、旧数据、新数据、DOM元素属性 | 财务/业务审计 |
| `ip_address` | VARCHAR(64) | 客户端 IP | 安全审计 |
| `user_agent` | VARCHAR(256)| 浏览器及设备信息 | 响应式/用户层 |
| `duration_ms` | INT | 操作耗时，用于性能瓶颈排查 | 代码整洁度审计 |
| `status` | VARCHAR(16) | SUCCESS, FAILED, DENIED | 异常/安全审计 |
| `error_msg` | TEXT | 失败原因或异常堆栈 | 异常审计 |
| `signature` | VARCHAR(128)| SHA256 签名，用于防篡改校验 | 安全要求 |
| `created_at` | TIMESTAMP | 发生时间 | - |

### 2.2 审计告警规则表 (`audit_alert_rules`)

| 字段名 | 类型 | 说明 |
|--------|------|------|
| `id` | INT | 主键 |
| `rule_name` | VARCHAR(64) | 规则名称，如: "异地登录告警", "越权访问" |
| `event_type` | VARCHAR(32) | 匹配的事件类型 |
| `condition_expr`| JSONB | 匹配条件表达式 (例如: `{"duration_ms": {">": 5000}}`) |
| `alert_level` | VARCHAR(16) | INFO, WARNING, CRITICAL |
| `is_active` | BOOLEAN | 是否启用 |

## 3. 关键技术实现路径

### 3.1 异步日志收集与签名 (Rust 后端)
- 使用 `tokio::spawn` 启动一个后台守护任务 (Daemon Task)。
- 业务代码通过 `OmniAudit::log(event)` 瞬间将日志压入 `mpsc::channel`，不会阻塞业务主流程（耗时 < 1ms）。
- Daemon Task 从 channel 取出数据，序列化 `payload`，拼接 `id` + `payload` + `SECRET_KEY` 计算 SHA256 签名，然后批量批量写入数据库。

### 3.2 告警引擎 (Alerting)
- 在 Daemon Task 写入数据库前，遍历 `audit_alert_rules` 在内存中的缓存。
- 若事件匹配规则（如 `status == "DENIED"` 且 `event_type == "SECURITY_ALERT"`），则推送到告警通道（WebSocket / Email）。

### 3.3 前端埋点机制 (WASM / Yew)
- **全局点击拦截**: 在 `index.html` 注入 JS 捕获 `click` 事件，过滤出带有特定 `data-audit` 属性的按钮，通过 WASM 接口传给 Rust 发送异步审计请求。
- **页面追踪**: 在 Yew 的 Router 切换处记录 `PAGE_VIEW`，统计 `duration`。
- **错误边界 (Error Boundary)**: 捕获前端未处理的 Panic，记录浏览器环境和堆栈，发送至后端。

## 4. 分级存储策略 (Data Tiering)
1. **热数据**: 最近 7 天的日志保留在 `omni_audit_logs` 表，提供大屏实时可视化。
2. **温/冷数据处理**: 编写一个 Cron Job (每日凌晨执行)，将 `created_at` < 30 天的数据归档到历史分区或 `history_logs` 表；> 30天的导出为 CSV 文件并从数据库物理删除。
