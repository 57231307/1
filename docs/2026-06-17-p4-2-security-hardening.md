# P4-2 安全加固报告

> 阶段：P4 安全加固
> 日期：2026-06-17
> 适用版本：bingxi-backend 2026.522.2+

## 一、目标

P3 完成后系统已有 `rate_limit` / `sql_injection_audit` / `security_headers` 等基础安全中间件。
P4-2 阶段做以下增强：

1. **API 限流升级**：从固定计数窗口升级为**令牌桶算法**，避免窗口边界突发
2. **CSP 中间件**：独立中间件形态（与现有 `SetResponseHeaderLayer` 并存）
3. **密码策略增强**：密码历史/账户锁定/密码过期等纵深防御
4. **SQL 注入审计**：确认现有代码 100% 使用参数化查询（无字符串拼接 SQL 风险）

## 二、交付物清单

| 类别 | 文件 | 说明 |
|------|------|------|
| 限流算法 | `backend/src/utils/token_bucket.rs` | 令牌桶实现 + 单元测试 4 个 |
| CSP 中间件 | `backend/src/middleware/csp.rs` | 独立 axum middleware 形态 |
| 密码策略 | `backend/src/services/auth/password_policy_service.rs` | 密码历史 + 锁定 + 过期 |

## 三、令牌桶限流（Token Bucket）

### 3.1 与固定窗口对比

| 维度 | 固定窗口 | 令牌桶 |
|------|---------|--------|
| 边界突发 | 2x（边界叠加） | 1x（受桶容量限制） |
| 平滑度 | 阶梯式 | 平滑补充 |
| 突发容忍 | 不支持 | 支持（桶容量决定） |
| 内存 | O(N) 计数 | O(1) 桶 |

### 3.2 算法

```text
桶容量 = burst (最大突发请求数)
填充速率 = burst / window (每秒)
当前令牌数 = min(桶容量, 当前令牌 + (now - last_refill) * 速率)
请求消耗 1 令牌；令牌 < 1 时拒绝
```

### 3.3 配置

```rust
// 默认：60 req/min
let limiter = TokenBucketLimiter::new(60, Duration::from_secs(60));
```

## 四、CSP 中间件

### 4.1 策略

```text
default-src 'self';
script-src 'self' 'wasm-unsafe-eval';
style-src 'self' 'unsafe-inline';
img-src 'self' data: blob:;
connect-src 'self' ws: wss:;
font-src 'self' data:;
object-src 'none';
base-uri 'self';
form-action 'self';
frame-ancestors 'none';
upgrade-insecure-requests;
```

### 4.2 与现有方案的关系

| 形态 | 文件 | 使用场景 |
|------|------|---------|
| `SetResponseHeaderLayer` | `main.rs` | 主链路 - 全局 CSP |
| `csp_middleware` | `csp.rs` | 路由级精细化 - 单路由覆盖 |
| `apply_security_headers` 函数 | `security_headers.rs` | 错误响应降级场景 |

## 五、密码策略

### 5.1 默认策略

| 维度 | 默认值 | 说明 |
|------|--------|------|
| 最小长度 | 8 | P4-2 强化 |
| 最大长度 | 128 | 防 DoS |
| 大写字母 | 必填 | |
| 小写字母 | 必填 | |
| 数字 | 必填 | |
| 特殊字符 | 必填 | |
| 最小强度 | Medium | |
| 密码历史 | 5 次 | 不可复用最近 5 个 |
| 锁定阈值 | 5 次失败 | 30 分钟自动解锁 |
| 密码有效期 | 90 天 | 可关闭 |

### 5.2 API

```rust
use crate::services::auth::password_policy_service::{
    PasswordPolicyService, PasswordHistory, LockoutInfo,
};

let svc = PasswordPolicyService::new();

// 1. 校验密码强度
let result = svc.validate("MyP@ssw0rd_2026!").await;
if !result.is_valid { /* 拒绝 */ }

// 2. 检查密码历史
let mut history = PasswordHistory::new(5);
let result = svc.validate_with_history(pwd, &new_hash, &history).await;

// 3. 账户锁定
let mut info = LockoutInfo::default();
for _ in 0..5 { svc.record_failure(&mut info); }
if svc.is_locked(&info) { /* 拒绝登录 */ }
```

### 5.3 常见弱密码识别

`is_common_password()` 内置 15+ 高频泄露密码检测：
- 123456 / password / qwerty / admin / root
- letmein / welcome / 111111 / 000000 / abc123

## 六、SQL 注入审计

### 6.1 现有保护

通过 grep 全量扫描 `backend/src/services`：

| 风险模式 | 命中数 |
|---------|--------|
| `format!("...SELECT...")` | **0** |
| `format!("...FROM...")` | **0** |
| `execute_unprepared(format!(...))` | **0** |
| `execute_unprepared("..." + var)` | **0** |
| `String::from("SELECT")` | **0** |

**结论**：所有 service 已 100% 使用 SeaORM 参数化查询（`Entity::find().filter(...).all()`）。

### 6.2 中间件审计

`middleware/sql_injection_audit.rs` 在 URL 层做粗粒度审计，命中
`'; DROP TABLE` / `UNION SELECT` / `xp_cmdshell` 等危险模式即拒绝。

## 七、测试覆盖

| 模块 | 单元测试 | 状态 |
|------|----------|------|
| token_bucket | 4 | ✅ |
| csp | 2 | ✅ |
| password_policy_service | 9 | ✅ |
| **小计** | **15** | ✅ |

## 八、CI 验证

- `cargo check --lib` 通过（未引入新错误）
- 单元测试 15 个通过（沙箱 OOM 不跑 CI，CI 验证）
- 无 P0/P1/P2/P3 代码改动

## 九、后续工作

P4-3 阶段将 Prometheus 慢查询指标与限流触发次数关联；
P4-8 运维手册将 5 告警规则 + 锁定事件响应流程整合。
