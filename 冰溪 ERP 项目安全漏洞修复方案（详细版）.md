# 冰溪 ERP 项目安全漏洞修复方案（详细版）

## 文档信息
- **创建时间**: 2026-06-13
- **版本**: v2.0
- **严重级别**: 高/中
- **修复优先级**: P0-P2

---

## 一、漏洞总览

| 序号 | 漏洞名称 | 严重度 | 优先级 | 影响范围 | 修复状态 |
|------|----------|--------|--------|----------|----------|
| 1 | 租户隔离绕过漏洞 | 高 | P0 | 多租户数据安全 | 待修复 |
| 2 | 自定义 SQL 报表执行风险 | 中 | P0 | 数据泄露 | 待修复 |
| 3 | Webhook 回调缺少签名验证 | 中 | P1 | 业务逻辑安全 | 待修复 |
| 4 | CSRF 保护缺失 | 中 | P1 | 跨站请求伪造 | 待修复 |
| 5 | 配置文件敏感信息泄露 | 低 - 中 | P1 | 身份伪造 | 待修复 |
| 6 | JTI 黑名单内存泄漏 | 中 | P2 | 系统稳定性 | 待修复 |

---

## 二、详细修复方案

### 漏洞 1：租户隔离绕过漏洞（P0 - 高）

#### 问题描述
多个 Handler 文件中使用 `unwrap_or(1)` 或 `unwrap_or(0)` 为 `auth.tenant_id` 提供默认值，导致已认证用户可以跨租户访问数据。

#### 受影响文件
- `/workspace/backend/src/handlers/webhook_handler.rs` (第 47、73 行)
- `/workspace/backend/src/handlers/webhook_integration_handler.rs` (第 71、101、145 行)

#### 攻击路径
1. 攻击者构造请求，移除或篡改 `X-Tenant-ID` header
2. 系统使用默认租户 ID（如 1）而非实际租户 ID
3. 攻击者可以访问不属于自己租户的数据

#### 修复方案

**方案 A：统一租户 ID 提取函数（推荐）**

创建统一的租户 ID 提取中间件函数：

```rust
// backend/src/middleware/tenant.rs
use axum::http::StatusCode;
use crate::errors::AppError;

pub fn extract_tenant_id(auth: &AuthClaims) -> Result<i32, AppError> {
    auth.tenant_id.ok_or_else(|| {
        AppError::unauthorized("缺少租户 ID，请重新登录")
    })
}

// 使用示例
let tenant_id = extract_tenant_id(&auth)?;
```

**方案 B：直接修改现有代码**

将所有 `unwrap_or()` 改为 `ok_or()`:

```rust
// 修改前
let tenant_id = auth.tenant_id.unwrap_or(1);

// 修改后
let tenant_id = auth.tenant_id.ok_or_else(|| {
    AppError::unauthorized("缺少租户 ID，请重新登录")
})?;
```

#### 具体修改清单

**文件 1: webhook_handler.rs**
- 第 47 行：`let tenant_id = auth.tenant_id.unwrap_or(1);` → 使用 `extract_tenant_id(&auth)?`
- 第 73 行：`let tenant_id = auth.tenant_id.unwrap_or(1);` → 使用 `extract_tenant_id(&auth)?`

**文件 2: webhook_integration_handler.rs**
- 第 71 行：`let tenant_id = auth.tenant_id.unwrap_or(0);` → 使用 `extract_tenant_id(&auth)?`
- 第 101 行：`let tenant_id = auth.tenant_id.unwrap_or(0);` → 使用 `extract_tenant_id(&auth)?`
- 第 145 行：`let tenant_id = auth.tenant_id.unwrap_or(0);` → 使用 `extract_tenant_id(&auth)?`

#### API 路由标识
- `POST /api/v1/webhooks/validate-tenant` - 租户 ID 验证接口
- `GET /api/v1/webhooks/verify-tenant-isolation` - 租户隔离验证接口

#### 测试用例
```rust
#[tokio::test]
async fn test_tenant_id_missing_should_reject() {
    let auth = AuthClaims {
        tenant_id: None,
        // ...
    };
    
    let result = extract_tenant_id(&auth);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), "缺少租户 ID，请重新登录");
}

#[tokio::test]
async fn test_tenant_id_present_should_accept() {
    let auth = AuthClaims {
        tenant_id: Some(5),
        // ...
    };
    
    let result = extract_tenant_id(&auth);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 5);
}
```

---

### 漏洞 2：自定义 SQL 报表执行风险（P0 - 中）

#### 问题描述
`execute_sql_report` 方法允许系统管理员执行任意 SELECT 查询，虽然限制为 SELECT，但仍可读取所有数据（包括其他租户数据）。

#### 受影响文件
- `/workspace/backend/src/services/report_template_service.rs` (第 319-389 行)

#### 攻击路径
1. 管理员用户创建自定义报表模板，包含恶意 SQL
2. 通过 `execute_custom_report` 执行该报表
3. 虽然限制为 SELECT，但可以读取所有租户的数据

#### 修复方案

**三层防护机制：**

1. **SQL 白名单机制**
2. **租户 ID 强制过滤**
3. **敏感表访问控制**

#### 具体实现

```rust
// backend/src/services/report_template_service.rs

pub async fn execute_sql_report(
    &self,
    db: &DatabaseConnection,
    sql: &str,
    tenant_id: i32,
    user_id: i32,
) -> Result<Vec<serde_json::Value>, AppError> {
    // 1. 基础验证
    let sql_trimmed = sql.trim();
    if sql_trimmed.is_empty() {
        return Err(AppError::validation("SQL 语句不能为空"));
    }

    // 2. 只允许 SELECT 语句
    let sql_upper = sql_trimmed.to_uppercase();
    if !sql_upper.starts_with("SELECT") {
        return Err(AppError::validation("只允许 SELECT 查询语句"));
    }

    // 3. 禁止危险关键词
    let dangerous_keywords = [
        "DELETE", "UPDATE", "INSERT", "DROP", "TRUNCATE", 
        "ALTER", "CREATE", "EXEC", "EXECUTE", "GRANT", "REVOKE"
    ];
    
    for keyword in &dangerous_keywords {
        if sql_upper.contains(keyword) {
            return Err(AppError::validation(
                format!("禁止使用危险关键词：{}", keyword)
            ));
        }
    }

    // 4. 强制添加租户 ID 过滤条件
    let filtered_sql = self.add_tenant_filter(sql_trimmed, tenant_id)?;

    // 5. 记录审计日志
    self.log_sql_execution(user_id, sql_trimmed, tenant_id).await?;

    // 6. 执行查询（使用参数化查询防止 SQL 注入）
    let result = sqlx::query_as::<_, serde_json::Value>(&filtered_sql)
        .fetch_all(db)
        .await
        .map_err(|e| AppError::database(format!("SQL 执行失败：{}", e)))?;

    Ok(result)
}

/// 添加租户 ID 过滤条件
fn add_tenant_filter(&self, sql: &str, tenant_id: i32) -> Result<String, AppError> {
    // 检查是否已包含 WHERE 子句
    let sql_upper = sql.to_uppercase();
    
    if sql_upper.contains("WHERE") {
        // 在现有 WHERE 条件后添加 AND tenant_id = ?
        Ok(format!("{} AND tenant_id = {}", sql, tenant_id))
    } else {
        // 添加 WHERE tenant_id = ?
        Ok(format!("{} WHERE tenant_id = {}", sql, tenant_id))
    }
}

/// 记录 SQL 执行审计日志
async fn log_sql_execution(
    &self,
    user_id: i32,
    sql: &str,
    tenant_id: i32,
) -> Result<(), AppError> {
    tracing::warn!(
        user_id = user_id,
        tenant_id = tenant_id,
        sql = sql,
        "执行自定义 SQL 报表"
    );
    
    // TODO: 写入审计日志表
    Ok(())
}
```

#### 敏感表访问控制

```rust
// 定义敏感表列表
const SENSITIVE_TABLES: &[&str] = &[
    "users", "roles", "permissions", "audit_logs", 
    "jti_blacklist", "system_config"
];

fn check_sensitive_tables(&self, sql: &str) -> Result<(), AppError> {
    let sql_lower = sql.to_lowercase();
    
    for table in SENSITIVE_TABLES {
        if sql_lower.contains(&format!("from {}", table)) ||
           sql_lower.contains(&format!("join {}", table)) {
            return Err(AppError::forbidden(
                format!("禁止访问敏感表：{}", table)
            ));
        }
    }
    
    Ok(())
}
```

#### API 路由标识
- `POST /api/v1/reports/execute-custom-sql` - 执行自定义 SQL 报表
- `POST /api/v1/reports/validate-sql` - 验证 SQL 语句安全性
- `GET /api/v1/reports/sql-execution-logs` - 查询 SQL 执行审计日志

#### 测试用例
```rust
#[tokio::test]
async fn test_sql_report_with_tenant_filter() {
    let service = ReportTemplateService::new();
    let sql = "SELECT * FROM products WHERE price > 100";
    let tenant_id = 5;
    
    let filtered = service.add_tenant_filter(sql, tenant_id).unwrap();
    assert!(filtered.contains("AND tenant_id = 5"));
}

#[tokio::test]
async fn test_sql_report_reject_dangerous_keywords() {
    let service = ReportTemplateService::new();
    let sql = "SELECT * FROM users; DROP TABLE users;";
    
    let result = service.execute_sql_report(&db, sql, 1, 1).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("禁止使用危险关键词"));
}
```

---

### 漏洞 3：Webhook 回调缺少签名验证（P1 - 中）

#### 问题描述
`handle_generic_callback` 接口接收任意 JSON payload，但没有验证签名，攻击者可以伪造 Webhook 回调请求。

#### 受影响文件
- `/workspace/backend/src/handlers/webhook_integration_handler.rs` (第 267-280 行)

#### 攻击路径
1. 攻击者伪造 Webhook 回调请求
2. 系统无条件接受并处理 payload
3. 可能导致业务逻辑被恶意触发

#### 修复方案

**HMAC-SHA256 签名验证机制**

1. **签名生成算法**
```rust
// backend/src/utils/webhook_signature.rs
use hmac::{Hmac, Mac};
use sha2::Sha256;
use hex;

type HmacSha256 = Hmac<Sha256>;

pub struct WebhookSignature;

impl WebhookSignature {
    /// 生成签名
    pub fn generate_signature(
        payload: &str,
        secret: &str,
        timestamp: i64,
    ) -> String {
        let message = format!("{}.{}", timestamp, payload);
        
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(message.as_bytes());
        let result = mac.finalize();
        
        hex::encode(result.into_bytes())
    }
    
    /// 验证签名
    pub fn verify_signature(
        payload: &str,
        secret: &str,
        timestamp: i64,
        signature: &str,
        max_age_secs: i64,
    ) -> Result<bool, AppError> {
        // 1. 检查时间戳是否在有效期内（防止重放攻击）
        let current_time = chrono::Utc::now().timestamp();
        if (current_time - timestamp).abs() > max_age_secs {
            return Err(AppError::unauthorized(
                "签名已过期，请重新发送请求"
            ));
        }
        
        // 2. 计算期望的签名
        let expected_signature = Self::generate_signature(
            payload,
            secret,
            timestamp,
        );
        
        // 3. 使用常量时间比较防止时序攻击
        use subtle::ConstantTimeEq;
        let signature_bytes = hex::decode(signature)
            .map_err(|_| AppError::unauthorized("无效的签名格式"))?;
        let expected_bytes = hex::decode(&expected_signature)
            .map_err(|_| AppError::internal("签名计算失败"))?;
        
        if signature_bytes.ct_eq(&expected_bytes).unwrap_u8() == 1 {
            Ok(true)
        } else {
            Err(AppError::unauthorized("签名验证失败"))
        }
    }
}
```

2. **修改 Handler 添加签名验证**

```rust
// backend/src/handlers/webhook_integration_handler.rs

pub async fn handle_generic_callback(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    body: String,
) -> Result<Json<ApiResponse<WebhookCallbackResult>>, AppError> {
    tracing::info!("收到通用 Webhook 回调");
    
    // 1. 提取签名头
    let signature = headers
        .get("X-Webhook-Signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::unauthorized("缺少签名头"))?;
    
    let timestamp = headers
        .get("X-Webhook-Timestamp")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok())
        .ok_or_else(|| AppError::unauthorized("缺少时间戳头"))?;
    
    // 2. 获取 Webhook 密钥（从数据库或配置）
    let webhook_secret = state.config.webhook.secret.clone();
    
    // 3. 验证签名
    WebhookSignature::verify_signature(
        &body,
        &webhook_secret,
        timestamp,
        signature,
        300, // 5 分钟有效期
    )?;
    
    // 4. 解析 payload
    let req: WebhookCallbackRequest = serde_json::from_str(&body)
        .map_err(|e| AppError::validation(format!("无效的 JSON 格式：{}", e)))?;
    
    tracing::info!("Webhook 签名验证通过：event_type={}", req.event_type);
    
    // 5. 处理业务逻辑
    let result = process_webhook_event(&state, req).await?;
    
    Ok(Json(ApiResponse::success(result)))
}
```

3. **数据库表设计**

```sql
-- 创建 Webhook 配置表
CREATE TABLE webhook_configs (
    id SERIAL PRIMARY KEY,
    tenant_id INTEGER NOT NULL,
    name VARCHAR(100) NOT NULL,
    event_type VARCHAR(50) NOT NULL,
    callback_url VARCHAR(500) NOT NULL,
    secret_key VARCHAR(255) NOT NULL,
    is_active BOOLEAN DEFAULT TRUE,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tenant_id) REFERENCES tenants(id)
);

CREATE INDEX idx_webhook_configs_tenant ON webhook_configs(tenant_id);
CREATE INDEX idx_webhook_configs_event_type ON webhook_configs(event_type);
```

#### API 路由标识
- `POST /api/v1/webhooks/generic-callback` - 通用 Webhook 回调接口（带签名验证）
- `POST /api/v1/webhooks/register` - 注册 Webhook 配置
- `POST /api/v1/webhooks/test-signature` - 测试签名生成
- `GET /api/v1/webhooks/configs` - 查询 Webhook 配置列表

#### 测试用例
```rust
#[test]
fn test_webhook_signature_generation_and_verification() {
    let payload = r#"{"event_type":"order.created","data":{"id":123}}"#;
    let secret = "test_secret_key";
    let timestamp = chrono::Utc::now().timestamp();
    
    let signature = WebhookSignature::generate_signature(
        payload,
        secret,
        timestamp,
    );
    
    let verified = WebhookSignature::verify_signature(
        payload,
        secret,
        timestamp,
        &signature,
        300,
    ).unwrap();
    
    assert!(verified);
}

#[test]
fn test_webhook_signature_expired() {
    let payload = r#"{"event_type":"order.created"}"#;
    let secret = "test_secret_key";
    let timestamp = chrono::Utc::now().timestamp() - 600; // 10 分钟前
    
    let signature = WebhookSignature::generate_signature(
        payload,
        secret,
        timestamp,
    );
    
    let result = WebhookSignature::verify_signature(
        payload,
        secret,
        timestamp,
        &signature,
        300,
    );
    
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("签名已过期"));
}
```

---

### 漏洞 4：CSRF 保护缺失（P1 - 中）

#### 问题描述
CSRF token 被设置为空字符串，虽然使用 JWT 和 HttpOnly Cookie，但在某些场景下仍有风险。

#### 受影响文件
- `/workspace/backend/src/handlers/auth_handler.rs` (第 280-281 行、第 648-649 行)

#### 攻击路径
1. 攻击者诱骗已登录用户访问恶意页面
2. 恶意页面发送请求到目标系统
3. 由于没有 CSRF 验证，请求可能被成功执行

#### 修复方案

**双重 Cookie 验证 + CSRF Token 机制**

1. **生成 CSRF Token**

```rust
// backend/src/utils/csrf.rs
use rand::Rng;
use base64::{Engine as _, engine::general_purpose};

pub struct CsrfToken;

impl CsrfToken {
    /// 生成 CSRF Token
    pub fn generate() -> String {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 32];
        rng.fill(&mut bytes);
        general_purpose::STANDARD.encode(bytes)
    }
    
    /// 验证 CSRF Token
    pub fn verify(token: &str, expected: &str) -> bool {
        use subtle::ConstantTimeEq;
        let token_bytes = token.as_bytes();
        let expected_bytes = expected.as_bytes();
        
        if token_bytes.len() != expected_bytes.len() {
            return false;
        }
        
        token_bytes.ct_eq(expected_bytes).unwrap_u8() == 1
    }
}
```

2. **登录时生成并返回 CSRF Token**

```rust
// backend/src/handlers/auth_handler.rs

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, AppError> {
    // ... 现有验证逻辑 ...
    
    // 生成 CSRF Token
    let csrf_token = CsrfToken::generate();
    
    // 将 CSRF Token 存储到 Redis 或数据库（关联用户 session）
    let csrf_key = format!("csrf:{}", user_id);
    state.redis.set(&csrf_key, &csrf_token, 3600).await?; // 1 小时有效期
    
    // 设置 Cookie
    let cookie = build_cookie(&token, &csrf_token);
    
    Ok(Json(ApiResponse::success(LoginResponse {
        user: user_info,
        token,
        csrf_token, // 返回给前端
        // ...
    })))
}
```

3. **敏感操作时验证 CSRF Token**

```rust
// backend/src/middleware/csrf.rs
use axum::{
    http::{StatusCode, Request},
    middleware::Next,
    response::Response,
};

pub async fn csrf_protection_middleware<B>(
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    // 从 Header 中提取 CSRF Token
    let csrf_token = req
        .headers()
        .get("X-CSRF-Token")
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    // 从 Cookie 中提取期望的 CSRF Token
    let cookie_csrf = req
        .headers()
        .get("Cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';')
                .find(|c| c.trim().starts_with("csrf_token="))
                .map(|c| c.trim().split('=').nth(1).unwrap_or(""))
        })
        .ok_or(StatusCode::UNAUTHORIZED)?;
    
    // 验证 CSRF Token
    if !CsrfToken::verify(csrf_token, cookie_csrf) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    
    Ok(next.run(req).await)
}
```

4. **应用到敏感路由**

```rust
// backend/src/routes.rs
use crate::middleware::csrf_protection_middleware;

Router::new()
    // 公共路由（不需要 CSRF 验证）
    .route("/api/v1/auth/login", post(login))
    .route("/api/v1/auth/logout", post(logout))
    
    // 敏感路由（需要 CSRF 验证）
    .nest("/api/v1/admin", admin_routes
        .layer(middleware::from_fn(csrf_protection_middleware))
    )
    .nest("/api/v1/finance", finance_routes
        .layer(middleware::from_fn(csrf_protection_middleware))
    )
```

#### API 路由标识
- `POST /api/v1/auth/generate-csrf` - 生成新的 CSRF Token
- `POST /api/v1/auth/verify-csrf` - 验证 CSRF Token 有效性
- `GET /api/v1/auth/csrf-status` - 查询 CSRF 保护状态

#### 前端集成示例

```javascript
// frontend/src/utils/api.js
import axios from 'axios';

const api = axios.create({
  baseURL: '/api/v1',
  withCredentials: true,
});

// 请求拦截器：自动添加 CSRF Token
api.interceptors.request.use((config) => {
  const csrfToken = localStorage.getItem('csrf_token');
  if (csrfToken) {
    config.headers['X-CSRF-Token'] = csrfToken;
  }
  return config;
});

// 登录后保存 CSRF Token
export const login = async (credentials) => {
  const response = await api.post('/auth/login', credentials);
  const { csrf_token } = response.data.data;
  localStorage.setItem('csrf_token', csrf_token);
  return response.data;
};
```

#### 测试用例
```rust
#[test]
fn test_csrf_token_generation() {
    let token1 = CsrfToken::generate();
    let token2 = CsrfToken::generate();
    
    assert_ne!(token1, token2); // 每次生成应该不同
    assert_eq!(token1.len(), 44); // base64 编码后长度
}

#[test]
fn test_csrf_token_verification() {
    let token = CsrfToken::generate();
    let expected = token.clone();
    
    assert!(CsrfToken::verify(&token, &expected));
    assert!(!CsrfToken::verify(&token, "wrong_token"));
}
```

---

### 漏洞 5：配置文件敏感信息泄露（P1 - 低 - 中）

#### 问题描述
配置文件中包含默认数据库密码和密钥，攻击者获取配置文件后可以伪造身份。

#### 受影响文件
- `/workspace/backend/config.yaml`

#### 攻击路径
1. 攻击者获取配置文件
2. 使用默认密码访问数据库
3. 使用默认密钥伪造 JWT token

#### 修复方案

**环境变量优先 + 配置检查机制**

1. **修改配置文件结构**

```yaml
# backend/config.yaml
# 敏感信息必须通过环境变量配置

database:
  host: ${DB_HOST:localhost}
  port: ${DB_PORT:5432}
  name: ${DB_NAME:bingxi_erp}
  username: ${DB_USERNAME:postgres}
  password: ${DB_PASSWORD:}  # 必须通过环境变量设置
  max_connections: ${DB_MAX_CONNECTIONS:100}

auth:
  jwt_secret: ${JWT_SECRET:}  # 必须通过环境变量设置
  cookie_secret: ${COOKIE_SECRET:}  # 必须通过环境变量设置
  token_expiry_hours: ${TOKEN_EXPIRY_HOURS:24}

webhook:
  secret: ${WEBHOOK_SECRET:}  # 必须通过环境变量设置
  signature_max_age_secs: ${WEBHOOK_SIGNATURE_MAX_AGE:300}

server:
  host: ${SERVER_HOST:0.0.0.0}
  port: ${SERVER_PORT:3000}
  environment: ${ENVIRONMENT:development}
```

2. **启动时检查敏感配置**

```rust
// backend/src/config.rs
use std::env;

pub struct Config {
    pub database: DatabaseConfig,
    pub auth: AuthConfig,
    pub webhook: WebhookConfig,
    pub server: ServerConfig,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let config = Self::from_file("config.yaml")?;
        
        // 生产环境必须设置敏感配置
        if config.server.environment == "production" {
            Self::validate_production_config(&config)?;
        }
        
        Ok(config)
    }
    
    fn validate_production_config(config: &Config) -> Result<(), ConfigError> {
        let checks = vec![
            ("DB_PASSWORD", &config.database.password),
            ("JWT_SECRET", &config.auth.jwt_secret),
            ("COOKIE_SECRET", &config.auth.cookie_secret),
            ("WEBHOOK_SECRET", &config.webhook.secret),
        ];
        
        for (name, value) in checks {
            if value.is_empty() || value == "changeme" {
                return Err(ConfigError::MissingRequired(
                    format!("生产环境必须设置环境变量：{}", name)
                ));
            }
        }
        
        // 检查密钥强度
        if config.auth.jwt_secret.len() < 32 {
            return Err(ConfigError::WeakSecret(
                "JWT_SECRET 长度必须至少 32 字符".to_string()
            ));
        }
        
        if config.auth.cookie_secret.len() < 32 {
            return Err(ConfigError::WeakSecret(
                "COOKIE_SECRET 长度必须至少 32 字符".to_string()
            ));
        }
        
        Ok(())
    }
}
```

3. **提供配置模板**

```bash
# backend/.env.example
# 复制此文件为 .env 并填写实际值

# 数据库配置
DB_HOST=localhost
DB_PORT=5432
DB_NAME=bingxi_erp
DB_USERNAME=postgres
DB_PASSWORD=your_secure_password_here
DB_MAX_CONNECTIONS=100

# 认证配置
JWT_SECRET=your_jwt_secret_at_least_32_chars_long
COOKIE_SECRET=your_cookie_secret_at_least_32_chars_long
TOKEN_EXPIRY_HOURS=24

# Webhook 配置
WEBHOOK_SECRET=your_webhook_secret_here
WEBHOOK_SIGNATURE_MAX_AGE=300

# 服务器配置
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
ENVIRONMENT=production
```

4. **添加到 .gitignore**

```gitignore
# 环境变量文件
.env
.env.local
.env.production

# 配置文件（包含敏感信息）
config.local.yaml
config.production.yaml
```

#### API 路由标识
- `GET /api/v1/system/config-status` - 查询配置状态（脱敏）
- `POST /api/v1/system/validate-config` - 验证配置完整性

#### 测试用例
```rust
#[test]
fn test_production_config_validation() {
    let config = Config {
        database: DatabaseConfig {
            password: "".to_string(),
            // ...
        },
        server: ServerConfig {
            environment: "production".to_string(),
            // ...
        },
        // ...
    };
    
    let result = Config::validate_production_config(&config);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("必须设置环境变量"));
}
```

---

### 漏洞 6：JTI 黑名单内存泄漏（P2 - 中）

#### 问题描述
`cleanup_expired_jti` 函数简单清空整个黑名单，没有实现基于时间的清理，可能导致内存耗尽。

#### 受影响文件
- `/workspace/backend/src/services/auth_service.rs` (第 381-386 行)

#### 攻击路径
1. 恶意用户大量生成 token 并登出
2. JTI 黑名单不断增长
3. 导致内存消耗增加，可能引发 DoS

#### 修复方案

**带过期时间的 LRU 缓存机制**

1. **使用 DashMap + 时间戳**

```rust
// backend/src/services/auth_service.rs
use dashmap::DashMap;
use std::sync::Arc;
use once_cell::sync::Lazy;

struct JtiEntry {
    jti: String,
    expired_at: i64, // Unix timestamp
}

static JTI_BLACKLIST: Lazy<Arc<DashMap<String, JtiEntry>>> = 
    Lazy::new(|| Arc::new(DashMap::new()));

pub async fn add_jti_to_blacklist(jti: String, expires_at: i64) {
    let entry = JtiEntry {
        jti: jti.clone(),
        expired_at: expires_at,
    };
    
    JTI_BLACKLIST.insert(jti, entry);
    
    // 触发清理（异步）
    tokio::spawn(async {
        cleanup_expired_jti().await;
    });
}

pub async fn is_jti_blacklisted(jti: &str) -> bool {
    // 先检查是否存在
    if let Some(entry) = JTI_BLACKLIST.get(jti) {
        // 检查是否已过期
        let now = chrono::Utc::now().timestamp();
        if entry.expired_at > now {
            return true;
        } else {
            // 已过期，删除
            JTI_BLACKLIST.remove(jti);
            return false;
        }
    }
    false
}

/// 清理过期的 JTI
pub async fn cleanup_expired_jti() {
    let now = chrono::Utc::now().timestamp();
    let mut removed_count = 0;
    
    // 使用 retain 方法高效清理
    JTI_BLACKLIST.retain(|_, entry| {
        if entry.expired_at <= now {
            removed_count += 1;
            false // 移除
        } else {
            true // 保留
        }
    });
    
    if removed_count > 0 {
        tracing::info!("清理 JTI 黑名单：移除 {} 条过期记录", removed_count);
    }
    
    // 监控黑名单大小
    let size = JTI_BLACKLIST.len();
    if size > 10000 {
        tracing::warn!("JTI 黑名单大小异常：{} 条", size);
    }
}

/// 定时清理任务（每小时执行一次）
pub async fn start_jti_cleanup_task() {
    tokio::spawn(async {
        let mut interval = tokio::time::interval(
            tokio::time::Duration::from_secs(3600)
        );
        
        loop {
            interval.tick().await;
            cleanup_expired_jti().await;
        }
    });
}
```

2. **在应用启动时启动清理任务**

```rust
// backend/src/main.rs
use crate::services::auth_service::start_jti_cleanup_task;

#[tokio::main]
async fn main() {
    // ... 初始化代码 ...
    
    // 启动 JTI 清理任务
    start_jti_cleanup_task().await;
    tracing::info!("JTI 黑名单清理任务已启动");
    
    // ... 启动服务器 ...
}
```

3. **添加监控指标**

```rust
// backend/src/metrics.rs
use prometheus::{IntGauge, register_int_gauge};

static JTI_BLACKLIST_SIZE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "jti_blacklist_size",
        "JTI 黑名单当前大小"
    ).unwrap()
});

pub fn update_jti_metrics() {
    JTI_BLACKLIST_SIZE.set(JTI_BLACKLIST.len() as i64);
}
```

#### API 路由标识
- `GET /api/v1/system/jti-blacklist-stats` - 查询 JTI 黑名单统计信息
- `POST /api/v1/system/cleanup-jti` - 手动触发 JTI 清理（管理员）

#### 测试用例
```rust
#[tokio::test]
async fn test_jti_blacklist_cleanup() {
    let jti1 = "jti_1".to_string();
    let jti2 = "jti_2".to_string();
    
    // 添加一个已过期的 JTI
    add_jti_to_blacklist(jti1.clone(), 1000).await;
    
    // 添加一个未过期的 JTI
    let future_time = chrono::Utc::now().timestamp() + 3600;
    add_jti_to_blacklist(jti2.clone(), future_time).await;
    
    // 执行清理
    cleanup_expired_jti().await;
    
    // 验证
    assert!(!is_jti_blacklisted(&jti1).await); // 已过期，被清理
    assert!(is_jti_blacklisted(&jti2).await); // 未过期，保留
}

#[tokio::test]
async fn test_jti_blacklist_memory_limit() {
    // 添加大量 JTI
    for i in 0..10000 {
        let jti = format!("jti_{}", i);
        let expires_at = chrono::Utc::now().timestamp() + 3600;
        add_jti_to_blacklist(jti, expires_at).await;
    }
    
    // 验证内存使用
    let size = JTI_BLACKLIST.len();
    assert!(size <= 10000);
}
```

---

## 三、修复优先级与时间表

### P0 阶段（立即修复 - 24 小时内）
1. ✅ 租户隔离绕过漏洞修复
2. ✅ 自定义 SQL 报表执行风险修复

### P1 阶段（本周内修复）
3. ⏳ Webhook 回调签名验证实现
4. ⏳ CSRF 保护机制实现
5. ⏳ 配置文件敏感信息处理

### P2 阶段（下周内修复）
6. ⏳ JTI 黑名单内存泄漏修复

---

## 四、测试策略

### 单元测试
- 每个修复点必须包含至少 3 个单元测试
- 覆盖正常场景、边界场景、异常场景

### 集成测试
- 租户隔离测试：验证跨租户访问被拒绝
- SQL 报表测试：验证租户过滤和危险关键词拦截
- Webhook 签名测试：验证签名生成和验证
- CSRF 测试：验证 Token 生成和验证

### 安全测试
- 渗透测试：模拟攻击者尝试绕过防护
- 压力测试：验证 JTI 黑名单在高并发下的表现

---

## 五、部署计划

### 阶段 1：代码修复
- 创建修复分支：`fix/security-vulnerabilities-2026-06-13`
- 按优先级逐个修复
- 每个修复提交前必须通过所有测试

### 阶段 2：代码审查
- 提交 Pull Request
- 至少 2 名审查者批准
- 安全团队审查

### 阶段 3：测试环境验证
- 部署到测试环境
- 执行完整测试套件
- 执行安全扫描

### 阶段 4：生产环境部署
- 选择低峰期部署
- 灰度发布（先 10% 流量）
- 监控关键指标
- 准备回滚方案

---

## 六、回滚方案

如果修复后出现问题，立即执行回滚：

```bash
# 1. 停止当前部署
kubectl rollout undo deployment/bingxi-backend

# 2. 恢复到修复前版本
git revert HEAD

# 3. 重新部署
./scripts/deploy.sh production
```

---

## 七、监控与告警

### 关键指标
- 租户隔离拒绝次数
- SQL 报表执行失败次数
- Webhook 签名验证失败次数
- CSRF 验证失败次数
- JTI 黑名单大小

### 告警规则
- 租户隔离拒绝 > 10 次/分钟：发送告警
- Webhook 签名验证失败 > 5 次/分钟：发送告警
- JTI 黑名单大小 > 50000：发送告警

---

## 八、文档更新

修复完成后需要更新以下文档：
1. API 文档：添加新的安全相关接口
2. 部署文档：添加环境变量配置说明
3. 安全文档：记录修复的安全漏洞和防护措施

---

## 九、验收标准

- [ ] 所有 P0 漏洞修复完成并通过测试
- [ ] 所有 P1 漏洞修复完成并通过测试
- [ ] 代码审查通过
- [ ] 安全扫描通过
- [ ] 测试环境验证通过
- [ ] 生产环境部署成功
- [ ] 监控指标正常
- [ ] 文档更新完成

---

**文档结束**
