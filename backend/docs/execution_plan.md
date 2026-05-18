#  ERP 系统 P0-P3 级问题修复执行计划

> **面向 AI 代理的工作者：** 本计划采用子代理驱动开发方式逐任务执行。
> 必需子技能：使用 subagent-driven-development 逐任务实现此计划。

**项目名称**: 面料管理 ERP 系统  
**技术栈**: Rust (Axum 0.7 + SeaORM 0.12 + PostgreSQL 18) + Yew  
**计划制定日期**: 2026-05-09  
**计划版本**: v1.0

---

## 一、执行概览

### 1.1 问题统计

| 阶段 | 问题数 | 预估工时 | 关键里程碑 |
|------|--------|---------|-----------|
| **P0** | 5项 | 4.5h | 系统可上线 |
| **P1** | 5项 | 37h | 核心功能完善 |
| **P2** | 7项 | 89h | 系统稳定 |
| **P3** | 5项 | 88h | 系统可扩展 |
| **总计** | **22项** | **218.5h** | |

### 1.2 P0-P3 问题清单

#### P0级（立即执行）- 安全漏洞修复

| 序号 | 问题编号 | 问题名称 | 文件位置 | 预估工时 | 验收标准 |
|------|---------|---------|---------|---------|---------|
| 1 | SEC-001 | 认证中间件绕过 | auth.rs:21-30 | 0.5h | 所有请求必须携带有效JWT |
| 2 | SEC-002 | 防暴力攻击禁用 | rate_limit.rs:96-98 | 0.5h | 登录失败5次后锁定15分钟 |
| 3 | SEC-003 | CSRF保护禁用 | request_validator.rs:19-21 | 0.5h | 非同源POST请求被拒绝 |
| 4 | SEC-004 | 明文密码配置 | config.yaml:8-13 | 2h | 密码从环境变量读取 |
| 5 | SEC-005 | 硬编码审计密钥 | omni_audit_service.rs:12 | 1h | 密钥从环境变量读取 |

#### P1级（1-2周内）- 功能与安全完善

| 序号 | 问题编号 | 问题名称 | 预估工时 | 验收标准 |
|------|---------|---------|---------|---------|
| 6 | SEC-006 | 用户身份硬编码 | 8h | 所有操作记录正确用户 |
| 7 | SEC-008 | 输入长度限制 | 4h | 所有文本输入有长度限制 |
| 8 | SEC-010 | JWT过期时间过长 | 1h | Token 2小时过期 |
| 9 | FUNC-001 | 销售订单变更历史 | 8h | 支持订单变更追踪 |
| 10 | FUNC-002 | 采购交期自动计算 | 16h | 自动计算交货日期 |

#### P2级（3-4周内）- 功能增强与性能优化

| 序号 | 问题编号 | 问题名称 | 预估工时 | 验收标准 |
|------|---------|---------|---------|---------|
| 11 | FUNC-003 | 成本计算 | 24h | 支持加权平均/先进先出 |
| 12 | FUNC-004 | 发票关联 | 16h | 发票必须关联有效订单 |
| 13 | FUNC-005 | 对账自动生成 | 12h | 支持对账单自动生成 |
| 14 | SEC-007 | 密码哈希强度 | 4h | 使用Argon2id |
| 15 | PERF-001 | 连接池优化 | 1h | 优化数据库连接配置 |
| 16 | PERF-007 | 缓存策略 | 8h | 热点数据缓存 |
| 17 | ISLAND-001 | 成本数据关联 | 24h | 成本追溯完整 |

#### P3级（后续迭代）- 架构优化

| 序号 | 问题编号 | 问题名称 | 预估工时 | 验收标准 |
|------|---------|---------|---------|---------|
| 18 | FUNC-016 | BPM可视化设计 | 32h | 支持流程设计器 |
| 19 | ISLAND-002 | CRM线索关联 | 8h | 线索转化追踪 |
| 20 | ISLAND-003 | BPM业务关联 | 16h | 流程与业务强关联 |
| 21 | ARCH-001 | DI容器 | 8h | 依赖注入架构 |
| 22 | ARCH-002-006 | 事件驱动等 | 24h | 架构改进 |

---

## 二、详细任务分解

### ============================================================================
### 阶段一：P0级安全漏洞修复（4.5小时）
### ============================================================================

### 任务 1：修复 SEC-001 认证中间件绕过漏洞

**目标**: 删除调试代码，恢复正常的JWT认证逻辑

**修改文件**:
- `backend/src/middleware/auth.rs` (第21-30行)

**详细步骤**:

- [ ] **步骤1：读取当前auth.rs文件**
  ```
  读取: backend/src/middleware/auth.rs
  确认问题代码位置
  ```

- [ ] **步骤2：删除调试绕过代码**
  ```
  删除第21-30行的硬编码AuthContext代码
  恢复正常的JWT验证逻辑
  ```

- [ ] **步骤3：实现正确的JWT验证**
  ```rust
  pub async fn auth_middleware(
      State(state): State<AppState>,
      mut request: Request,
  ) -> Result<Response, StatusCode> {
      // 1. 从请求头提取 JWT Token
      let token = request
          .headers()
          .get("Authorization")
          .and_then(|v| v.to_str().ok())
          .and_then(|v| v.strip_prefix("Bearer "))
          .ok_or(StatusCode::UNAUTHORIZED)?;

      // 2. 验证 JWT Token
      let claims = state
          .auth_service
          .verify_token(token)
          .map_err(|_| StatusCode::UNAUTHORIZED)?;

      // 3. 构建 AuthContext
      let auth_context = AuthContext {
          user_id: claims.sub,
          username: claims.username,
          role_id: claims.role_id,
      };

      // 4. 插入到请求上下文
      request.extensions_mut().insert(auth_context);

      // 5. 继续处理
      Ok(next.run(request).await)
  }
  ```

- [ ] **步骤4：验证修改**
  ```
  编译: cargo build
  确保无编译错误
  ```

**验收标准**:
- [ ] 调试代码已删除
- [ ] JWT验证逻辑正常
- [ ] 无有效Token的请求返回401

---

### 任务 2：修复 SEC-002 防暴力攻击禁用

**目标**: 删除调试代码，恢复速率限制逻辑

**修改文件**:
- `backend/src/middleware/rate_limit.rs` (第96-98行)

**详细步骤**:

- [ ] **步骤1：读取rate_limit.rs文件**
  ```
  读取: backend/src/middleware/rate_limit.rs
  确认anti_brute_force函数
  ```

- [ ] **步骤2：删除调试绕过代码**
  ```
  删除第96-98行的直接返回代码
  恢复速率限制逻辑
  ```

- [ ] **步骤3：实现防暴力攻击逻辑**
  ```rust
  pub async fn anti_brute_force(
      State(state): State<AppState>,
      req: Request,
  ) -> Result<Response, AppError> {
      // 1. 获取客户端标识
      let client_ip = req
          .headers()
          .get("x-forwarded-for")
          .and_then(|v| v.to_str().ok())
          .unwrap_or("unknown");
      
      let client_id = format!("{}", client_ip);
      
      // 2. 查询登录尝试次数
      let attempts_key = format!("login_attempts:{}", client_id);
      let attempts: i32 = state.redis.get(&attempts_key).await.unwrap_or(0);
      
      // 3. 检查是否超过阈值
      if attempts >= 5 {
          return Err(AppError::TooManyRequests {
              retry_after: 300,
              message: "登录尝试次数过多，请5分钟后再试".to_string(),
          });
      }
      
      // 4. 继续处理请求
      let response = next.run(req).await?;
      
      Ok(response)
  }
  ```

- [ ] **步骤4：验证修改**
  ```
  编译: cargo build
  确保无编译错误
  ```

**验收标准**:
- [ ] 调试代码已删除
- [ ] 防暴力攻击逻辑正常
- [ ] 连续5次失败后返回429

---

### 任务 3：修复 SEC-003 CSRF保护禁用

**目标**: 删除调试代码，恢复CSRF检查逻辑

**修改文件**:
- `backend/src/middleware/request_validator.rs` (第19-21行)

**详细步骤**:

- [ ] **步骤1：读取request_validator.rs文件**
  ```
  读取: backend/src/middleware/request_validator.rs
  确认request_validator_middleware函数
  ```

- [ ] **步骤2：删除调试绕过代码**
  ```
  删除第19-21行的直接返回代码
  恢复CSRF检查逻辑
  ```

- [ ] **步骤3：实现CSRF检查逻辑**
  ```rust
  pub async fn request_validator_middleware(
      mut request: Request,
      next: Next,
  ) -> Result<Response, StatusCode> {
      // 1. 仅对状态变更请求进行验证
      let method = request.method().clone();
      if !is_state_changing_method(&method) {
          return Ok(next.run(request).await);
      }
      
      // 2. 验证 Origin 头
      let origin = request
          .headers()
          .get("origin")
          .and_then(|v| v.to_str().ok());
      
      let allowed_origins = state.config.allowed_origins.clone();
      
      if let Some(origin) = origin {
          if !allowed_origins.contains(&origin.to_string()) {
              return Err(StatusCode::FORBIDDEN);
          }
      } else {
          // 无Origin头的请求也检查Referer
          let referer = request
              .headers()
              .get("referer")
              .and_then(|v| v.to_str().ok());
          
          if referer.is_none() {
              return Err(StatusCode::FORBIDDEN);
          }
      }
      
      Ok(next.run(request).await)
  }
  ```

- [ ] **步骤4：验证修改**
  ```
  编译: cargo build
  确保无编译错误
  ```

**验收标准**:
- [ ] 调试代码已删除
- [ ] CSRF检查逻辑正常
- [ ] 非同源POST请求被拒绝

---

### 任务 4：修复 SEC-004 明文密码配置

**目标**: 将敏感配置迁移到环境变量

**修改文件**:
- `backend/config.yaml` (第8-13行)
- `backend/src/config/mod.rs` (新增配置加载)
- 创建 `.env.example` 文件

**详细步骤**:

- [ ] **步骤1：读取当前配置文件**
  ```
  读取: backend/config.yaml
  读取: backend/src/config/mod.rs
  ```

- [ ] **步骤2：创建.env.example文件**
  ```bash
  # .env.example - 环境变量示例
  # 数据库配置
  DATABASE_HOST=39.99.34.194
  DATABASE_PORT=5432
  DATABASE_NAME=bingxi
  DATABASE_USER=bingxi
  DATABASE_PASSWORD=your_secure_password_here
  
  # 认证配置
  JWT_SECRET=your_jwt_secret_at_least_32_bytes
  JWT_SECRET_MIN_BYTES=32
  COOKIE_SECRET=your_cookie_secret_at_least_32_bytes
  
  # 审计配置
  AUDIT_SECRET_KEY=your_audit_signing_key_at_least_32_bytes
  ```

- [ ] **步骤3：创建配置加载模块**
  ```rust
  // backend/src/config/mod.rs
  use std::env;
  
  #[derive(Debug, Clone)]
  pub struct DatabaseConfig {
      pub host: String,
      pub port: String,
      pub name: String,
      pub username: String,
      pub password: String,  // 从环境变量读取
  }
  
  impl DatabaseConfig {
      pub fn from_env() -> Result<Self, ConfigError> {
          Ok(Self {
              host: env::var("DATABASE_HOST")
                  .unwrap_or_else(|_| "localhost".to_string()),
              port: env::var("DATABASE_PORT")
                  .unwrap_or_else(|_| "5432".to_string()),
              name: env::var("DATABASE_NAME")
                  .unwrap_or_else(|_| "bingxi".to_string()),
              username: env::var("DATABASE_USER")
                  .unwrap_or_else(|_| "postgres".to_string()),
              password: env::var("DATABASE_PASSWORD")
                  .map_err(|_| ConfigError::MissingEnv("DATABASE_PASSWORD"))?,
          })
      }
  }
  
  #[derive(Debug, Clone)]
  pub struct AuthConfig {
      pub jwt_secret: String,  // 从环境变量读取
      pub cookie_secret: String,
  }
  
  impl AuthConfig {
      pub fn from_env() -> Result<Self, ConfigError> {
          let jwt_secret = env::var("JWT_SECRET")
              .map_err(|_| ConfigError::MissingEnv("JWT_SECRET"))?;
          
          if jwt_secret.contains("test") || jwt_secret.contains("local-dev") {
              return Err(ConfigError::InsecureConfig(
                  "JWT Secret 不能使用测试/开发默认值".to_string()
              ));
          }
          
          if jwt_secret.len() < 32 {
              return Err(ConfigError::InsecureConfig(
                  "JWT Secret 长度必须至少 32 字节".to_string()
              ));
          }
          
          Ok(Self {
              jwt_secret,
              cookie_secret: env::var("COOKIE_SECRET")
                  .map_err(|_| ConfigError::MissingEnv("COOKIE_SECRET"))?,
          })
      }
  }
  
  #[derive(Debug, Clone)]
  pub struct AuditConfig {
      pub secret_key: String,  // 从环境变量读取
  }
  
  impl AuditConfig {
      pub fn from_env() -> Result<Self, ConfigError> {
          let secret_key = env::var("AUDIT_SECRET_KEY")
              .map_err(|_| ConfigError::MissingEnv("AUDIT_SECRET_KEY"))?;
          
          Ok(Self { secret_key })
      }
  }
  ```

- [ ] **步骤4：更新config.yaml**
  ```yaml
  # config.yaml - 删除敏感信息
  server:
    host: "0.0.0.0"
    port: 8080
  
  database:
    host: "39.99.34.194"  # 保留非敏感配置
    port: "5432"
    name: "bingxi"
    username: "bingxi"
    # password 从环境变量 DATABASE_PASSWORD 读取
  
  auth:
    allowed_origins:
      - "http://localhost:3000"
      - "https://erp.example.com"
    # jwt_secret 从环境变量 JWT_SECRET 读取
    # cookie_secret 从环境变量 COOKIE_SECRET 读取
  
  # audit 从环境变量 AUDIT_SECRET_KEY 读取
  ```

- [ ] **步骤5：验证修改**
  ```
  编译: cargo build
  确保无编译错误
  ```

**验收标准**:
- [ ] 敏感信息从环境变量读取
- [ ] 配置文件不包含明文密码
- [ ] .env.example作为配置指南

---

### 任务 5：修复 SEC-005 硬编码审计密钥

**目标**: 将审计密钥从环境变量读取

**修改文件**:
- `backend/src/services/omni_audit_service.rs` (第12行)

**详细步骤**:

- [ ] **步骤1：读取omni_audit_service.rs文件**
  ```
  读取: backend/src/services/omni_audit_service.rs
  确认硬编码位置
  ```

- [ ] **步骤2：修改审计服务**
  ```rust
  // 修改前
  const AUDIT_SECRET_KEY: &[u8] = b"bingxi_erp_audit_super_secret_key_2026";
  
  // 修改后
  pub struct OmniAuditService {
      db: Arc<DatabaseConnection>,
      secret_key: Vec<u8>,  // 从环境变量加载
  }
  
  impl OmniAuditService {
      pub fn new(db: Arc<DatabaseConnection>) -> Result<Self, AppError> {
          let secret_key = std::env::var("AUDIT_SECRET_KEY")
              .map_err(|_| AppError::ConfigError("AUDIT_SECRET_KEY 未设置".to_string()))?;
          
          if secret_key.len() < 32 {
              return Err(AppError::ConfigError(
                  "AUDIT_SECRET_KEY 长度必须至少 32 字节".to_string()
              ));
          }
          
          Ok(Self {
              db,
              secret_key: secret_key.into_bytes(),
          })
      }
  }
  ```

- [ ] **步骤3：更新sign_audit_log方法**
  ```rust
  pub fn sign_audit_log(&self, log: &AuditLog) -> String {
      let payload = serde_json::to_string(log).unwrap();
      let signature = hmac_sha256::HMAC_SHA256::mac(
          payload.as_bytes(), 
          &self.secret_key  // 使用实例字段
      );
      base64::encode(signature)
  }
  ```

- [ ] **步骤4：验证修改**
  ```
  编译: cargo build
  确保无编译错误
  ```

**验收标准**:
- [ ] 审计密钥从环境变量读取
- [ ] 启动时验证密钥长度
- [ ] 代码中无硬编码密钥

---

### ============================================================================
### 阶段二：P1级问题修复（37小时）
### ============================================================================

### 任务 6：修复 SEC-006 用户身份硬编码

**目标**: 所有Handler从AuthContext获取用户ID

**修改文件**: 所有67个Handler文件

**详细步骤**:

- [ ] **步骤1：扫描所有Handler文件**
  ```
  grep -r "let user_id = 1" backend/src/handlers/
  列出所有硬编码位置
  ```

- [ ] **步骤2：修改supplier_handler.rs**
  ```rust
  // 修改前
  pub async fn create_supplier(
      State(state): State<AppState>,
      Json(req): Json<CreateSupplierRequest>,
  ) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
      let user_id = 1;  // 硬编码
      let supplier = service.create_supplier(req, user_id).await?;
      // ...
  }
  
  // 修改后
  pub async fn create_supplier(
      State(state): State<AppState>,
      auth: AuthContext,  // 从中间件获取
      Json(req): Json<CreateSupplierRequest>,
  ) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
      let user_id = auth.user_id;  // 使用实际用户ID
      let supplier = service.create_supplier(req, user_id).await?;
      // ...
  }
  ```

- [ ] **步骤3：逐个修改其他Handler**
  ```
  customer_handler.rs
  product_handler.rs
  sales_order_handler.rs
  purchase_order_handler.rs
  ... (所有涉及创建的Handler)
  ```

- [ ] **步骤4：验证修改**
  ```
  编译: cargo build
  测试: cargo test
  ```

**验收标准**:
- [ ] 所有Handler使用AuthContext获取user_id
- [ ] 编译无错误
- [ ] 测试通过

---

### 任务 7：修复 SEC-008 输入长度限制

**目标**: 为所有文本字段添加长度验证

**修改文件**: 所有DTO定义

**详细步骤**:

- [ ] **步骤1：安装validator crate (已安装)**
  ```
  确认Cargo.toml包含validator依赖
  ```

- [ ] **步骤2：添加验证到DTO**
  ```rust
  #[derive(Debug, Deserialize, Validate)]
  pub struct CreateCustomerRequest {
      #[validate(length(min = 1, max = 100, message = "客户名称长度必须在1-100之间"))]
      pub customer_name: String,
      
      #[validate(length(max = 200, message = "联系人名称不能超过200字符"))]
      pub contact_person: Option<String>,
      
      #[validate(length(max = 20, message = "电话不能超过20位"))]
      pub contact_phone: Option<String>,
      
      #[validate(email, message = "邮箱格式不正确"))]
      pub contact_email: Option<String>,
      
      #[validate(length(max = 1000, message = "备注不能超过1000字符"))]
      pub notes: Option<String>,
  }
  ```

- [ ] **步骤3：在Handler中调用验证**
  ```rust
  pub async fn create_customer(
      State(state): State<AppState>,
      Json(payload): Json<CreateCustomerRequest>,
  ) -> Result<Json<ApiResponse<JsonValue>>, AppError> {
      // 调用验证
      payload.validate()?;
      // ...
  }
  ```

- [ ] **步骤4：验证修改**
  ```
  编译: cargo build
  测试: 发送超长输入验证被拒绝
  ```

**验收标准**:
- [ ] 所有文本字段有长度限制
- [ ] 超长输入返回400错误
- [ ] 编译无错误

---

### 任务 8：修复 SEC-010 JWT过期时间过长

**目标**: 缩短Token过期时间，增加刷新机制

**修改文件**:
- `backend/src/services/auth_service.rs`

**详细步骤**:

- [ ] **步骤1：修改JWT Claims结构**
  ```rust
  #[derive(Debug, Serialize, Deserialize)]
  pub struct Claims {
      pub sub: i32,                    // 用户ID
      pub username: String,
      pub role_id: Option<i32>,
      pub exp: DateTime<Utc>,         // Token过期时间（2小时）
      pub iat: DateTime<Utc>,
      pub session_id: String,          // 会话ID
      pub refresh_exp: DateTime<Utc>, // 刷新过期时间（7天）
  }
  ```

- [ ] **步骤2：修改Token生成**
  ```rust
  pub fn generate_token(&self, user_id: i32, username: String, role_id: Option<i32>) 
      -> Result<String, AppError> {
      let now = Utc::now();
      
      let claims = Claims {
          sub: user_id,
          username: username.clone(),
          role_id,
          exp: now + Duration::hours(2),           // 2小时过期
          iat: now,
          session_id: Uuid::new_v4().to_string(),
          refresh_exp: now + Duration::days(7),     // 7天刷新期
      };
      
      // ...
  }
  ```

- [ ] **步骤3：添加刷新Token接口**
  ```rust
  pub async fn refresh_token(
      State(state): State<AppState>,
      Json(req): Json<RefreshTokenRequest>,
  ) -> Result<Json<TokenResponse>, AppError> {
      // 验证刷新Token
      let claims = state.auth_service.verify_token(&req.refresh_token)?;
      
      // 检查是否在刷新期内
      if Utc::now() > claims.refresh_exp {
          return Err(AppError::Unauthorized("刷新令牌已过期".to_string()));
      }
      
      // 生成新Token
      let new_token = state.auth_service.generate_token(
          claims.sub,
          claims.username,
          claims.role_id,
      )?;
      
      Ok(Json(TokenResponse {
          access_token: new_token,
          refresh_token: req.refresh_token,
          expires_in: 7200,  // 2小时
          token_type: "Bearer".to_string(),
      }))
  }
  ```

- [ ] **步骤4：验证修改**
  ```
  编译: cargo build
  测试: 获取Token并验证过期时间
  ```

**验收标准**:
- [ ] Token 2小时过期
- [ ] 支持刷新Token机制
- [ ] 编译无错误

---

### 任务 9：实现 FUNC-001 销售订单变更历史

**目标**: 支持订单变更追踪

**新增文件**:
- `backend/database/migration/002_order_change_history.sql`
- `backend/src/models/order_change_history.rs`
- `backend/src/services/order_change_history_service.rs`

**详细步骤**:

- [ ] **步骤1：创建数据库迁移**
  ```sql
  CREATE TABLE sales_order_change_history (
      id SERIAL PRIMARY KEY,
      order_id INTEGER NOT NULL REFERENCES sales_orders(id),
      change_type VARCHAR(20) NOT NULL,
      field_name VARCHAR(100),
      old_value TEXT,
      new_value TEXT,
      changed_by INTEGER NOT NULL REFERENCES users(id),
      changed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      change_reason TEXT,
      ip_address VARCHAR(45)
  );
  
  CREATE INDEX idx_order_history_order_id ON sales_order_change_history(order_id);
  CREATE INDEX idx_order_history_changed_at ON sales_order_change_history(changed_at);
  ```

- [ ] **步骤2：创建Model**
  ```rust
  use sea_orm::entity::prelude::*;
  
  #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
  #[sea_orm(table_name = "sales_order_change_history")]
  pub struct Model {
      #[sea_orm(primary_key)]
      pub id: i32,
      pub order_id: i32,
      pub change_type: String,
      pub field_name: Option<String>,
      pub old_value: Option<String>,
      pub new_value: Option<String>,
      pub changed_by: i32,
      pub changed_at: DateTime<Utc>,
      pub change_reason: Option<String>,
      pub ip_address: Option<String>,
  }
  ```

- [ ] **步骤3：创建服务**
  ```rust
  pub struct OrderChangeHistoryService {
      db: Arc<DatabaseConnection>,
  }
  
  impl OrderChangeHistoryService {
      pub async fn record_change(
          &self,
          order_id: i32,
          change_type: &str,
          field_name: &str,
          old_value: &str,
          new_value: &str,
          user_id: i32,
          reason: Option<String>,
      ) -> Result<(), AppError> {
          let history = order_change_history::ActiveModel {
              order_id: Set(order_id),
              change_type: Set(change_type.to_string()),
              field_name: Set(Some(field_name.to_string())),
              old_value: Set(Some(old_value.to_string())),
              new_value: Set(Some(new_value.to_string())),
              changed_by: Set(user_id),
              changed_at: Set(Utc::now()),
              change_reason: Set(reason),
              ip_address: Set(None),
              ..Default::default()
          };
          
          history.insert(&*self.db).await?;
          Ok(())
      }
  }
  ```

- [ ] **步骤4：在OrderService中集成**
  ```rust
  // 在update_order方法中添加
  pub async fn update_order(
      &self,
      order_id: i32,
      req: UpdateOrderRequest,
      user_id: i32,
  ) -> Result<Order, AppError> {
      let order = self.find_by_id(order_id).await?;
      
      // 记录变更历史
      if let Some(name) = &req.customer_name {
          self.history_service.record_change(
              order_id,
              "UPDATE",
              "customer_name",
              &order.customer_name,
              name,
              user_id,
              None,
          ).await?;
      }
      
      // ...
  }
  ```

- [ ] **步骤5：验证实现**
  ```
  编译: cargo build
  测试: 更新订单并验证历史记录
  ```

**验收标准**:
- [ ] 可查看订单变更历史
- [ ] 显示修改人、字段、新旧值
- [ ] 支持按时间筛选

---

### 任务 10：实现 FUNC-002 采购交期自动计算

**目标**: 根据供应商交货周期自动计算交货日期

**新增/修改文件**:
- `backend/src/services/purchase_delivery_calculator.rs`
- `backend/src/services/supplier_service.rs` (添加平均交期)

**详细步骤**:

- [ ] **步骤1：创建计算服务**
  ```rust
  pub struct PurchaseDeliveryCalculator {
      db: Arc<DatabaseConnection>,
  }
  
  impl PurchaseDeliveryCalculator {
      /// 获取供应商平均交货周期(天)
      pub async fn get_supplier_avg_lead_time(
          &self,
          supplier_id: i32,
      ) -> Result<i32, AppError> {
          // 查询该供应商已完成订单的平均交货天数
          let result = self.db.query_one(
              Statement::from_sql_and_values(
                  PostgresQueryBuilder,
                  r#"
                  SELECT AVG(EXTRACT(EPOCH FROM (receipt_date - order_date)) / 86400)::INTEGER as avg_days
                  FROM purchase_order po
                  JOIN purchase_receipt pr ON po.id = pr.order_id
                  WHERE po.supplier_id = $1
                  AND pr.receipt_date IS NOT NULL
                  "#,
                  vec![supplier_id.into()],
              )
          ).await?;
          
          Ok(result.and_then(|r| r.try_get::<i32>("", "avg_days")).ok().unwrap_or(7))
      }
      
      /// 计算预计交货日期
      pub async fn calculate_delivery_date(
          &self,
          supplier_id: i32,
          order_items: &[OrderItem],
      ) -> Result<NaiveDate, AppError> {
          // 1. 获取供应商平均交货周期
          let avg_lead_time = self.get_supplier_avg_lead_time(supplier_id).await?;
          
          // 2. 获取最大生产周期的物料
          let max_production_days = self.get_max_production_days(order_items).await?;
          
          // 3. 计算总准备时间
          let total_days = avg_lead_time + max_production_days;
          
          // 4. 考虑节假日(简化实现)
          let start_date = Utc::now().date_naive();
          let delivery_date = self.add_business_days(start_date, total_days);
          
          Ok(delivery_date)
      }
      
      /// 添加工作日
      fn add_business_days(&self, start: NaiveDate, days: i32) -> NaiveDate {
          let mut current = start;
          let mut remaining = days;
          
          while remaining > 0 {
              current = current.succ_opt().unwrap();
              let weekday = current.weekday();
              if weekday != Weekday::Sat && weekday != Weekday::Sun {
                  remaining -= 1;
              }
          }
          
          current
      }
  }
  ```

- [ ] **步骤2：在PurchaseOrderService中集成**
  ```rust
  pub async fn create_order(
      &self,
      req: CreateOrderRequest,
      user_id: i32,
  ) -> Result<PurchaseOrder, AppError> {
      // 计算建议交货日期
      let suggested_delivery = self.delivery_calculator
          .calculate_delivery_date(req.supplier_id, &req.items)
          .await?;
      
      // 创建订单时使用建议交货日期
      let order = self.create_order_internal(req, user_id, suggested_delivery).await?;
      
      Ok(order)
  }
  ```

- [ ] **步骤3：添加API端点**
  ```rust
  pub async fn calculate_delivery_date(
      State(state): State<AppState>,
      Json(req): Json<CalculateDeliveryRequest>,
  ) -> Result<Json<DeliveryDateResponse>, AppError> {
      let calculator = PurchaseDeliveryCalculator::new(state.db.clone());
      
      let delivery_date = calculator
          .calculate_delivery_date(req.supplier_id, &req.items)
          .await?;
      
      let avg_lead_time = calculator
          .get_supplier_avg_lead_time(req.supplier_id)
          .await?;
      
      Ok(Json(DeliveryDateResponse {
          suggested_date: delivery_date,
          avg_lead_time_days: avg_lead_time,
          calculation_basis: "基于供应商历史交货数据".to_string(),
      }))
  }
  ```

- [ ] **步骤4：验证实现**
  ```
  编译: cargo build
  测试: 调用计算接口验证结果
  ```

**验收标准**:
- [ ] 自动计算新订单的建议交货日期
- [ ] 显示计算依据
- [ ] 允许手动调整

---

### ============================================================================
### 阶段三：P2级问题修复（89小时）
### ============================================================================

### 任务 11：实现 FUNC-003 成本计算

**目标**: 支持加权平均/先进先出成本计算

**新增文件**:
- `backend/src/services/cost_calculation_service.rs`
- `backend/database/migration/003_cost_tables.sql`

**详细步骤**:

- [ ] **步骤1：创建成本历史表**
  ```sql
  CREATE TABLE inventory_cost_history (
      id SERIAL PRIMARY KEY,
      product_id INTEGER NOT NULL REFERENCES products(id),
      warehouse_id INTEGER NOT NULL REFERENCES warehouses(id),
      batch_no VARCHAR(50),
      quantity DECIMAL(18,4) NOT NULL,
      unit_cost DECIMAL(18,6) NOT NULL,
      total_cost DECIMAL(18,2) NOT NULL,
      costing_method VARCHAR(20) NOT NULL,
      source_type VARCHAR(20) NOT NULL,
      source_id INTEGER,
      record_date DATE NOT NULL,
      created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
  );
  
  CREATE TABLE sales_cost_records (
      id SERIAL PRIMARY KEY,
      order_id INTEGER NOT NULL REFERENCES sales_orders(id),
      order_item_id INTEGER NOT NULL,
      product_id INTEGER NOT NULL REFERENCES products(id),
      quantity DECIMAL(18,4) NOT NULL,
      unit_cost DECIMAL(18,6) NOT NULL,
      total_cost DECIMAL(18,2) NOT NULL,
      costing_method VARCHAR(20) NOT NULL,
      batch_no VARCHAR(50),
      created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
  );
  ```

- [ ] **步骤2：实现成本计算服务**
  ```rust
  pub enum CostingMethod {
      WeightedAverage,
      FIFO,
  }
  
  pub struct CostCalculationService {
      db: Arc<DatabaseConnection>,
      method: CostingMethod,
  }
  
  impl CostCalculationService {
      /// 加权平均法计算成本
      pub async fn weighted_average_cost(
          &self,
          product_id: i32,
          warehouse_id: i32,
          date: NaiveDate,
      ) -> Result<Decimal, AppError> {
          let result = self.db.query_one(
              Statement::from_sql_and_values(
                  PostgresQueryBuilder,
                  r#"
                  SELECT 
                      SUM(total_cost) as total_amount,
                      SUM(quantity) as total_quantity
                  FROM inventory_cost_history
                  WHERE product_id = $1
                  AND warehouse_id = $2
                  AND record_date <= $3
                  AND costing_method = 'WEIGHTED_AVG'
                  "#,
                  vec![product_id.into(), warehouse_id.into(), date.into()],
              )
          ).await?;
          
          let total_amount: Decimal = result.try_get("", "total_amount")?;
          let total_quantity: Decimal = result.try_get("", "total_quantity")?;
          
          if total_quantity == Decimal::ZERO {
              return Err(AppError::NoCostData);
          }
          
          Ok(total_amount / total_quantity)
      }
      
      /// 先进先出法计算成本
      pub async fn fifo_cost(
          &self,
          product_id: i32,
          warehouse_id: i32,
          quantity: Decimal,
          date: NaiveDate,
      ) -> Result<Decimal, AppError> {
          let batches = self.db.query_all(
              Statement::from_sql_and_values(
                  PostgresQueryBuilder,
                  r#"
                  SELECT * FROM inventory_cost_history
                  WHERE product_id = $1
                  AND warehouse_id = $2
                  AND record_date <= $3
                  AND costing_method = 'FIFO'
                  ORDER BY record_date ASC
                  "#,
                  vec![product_id.into(), warehouse_id.into(), date.into()],
              )
          ).await?;
          
          let mut remaining = quantity;
          let mut total_cost = Decimal::ZERO;
          
          for batch in batches {
              if remaining <= Decimal::ZERO {
                  break;
              }
              
              let batch_qty: Decimal = batch.try_get("", "quantity")?;
              let unit_cost: Decimal = batch.try_get("", "unit_cost")?;
              
              let used_qty = remaining.min(batch_qty);
              total_cost += unit_cost * used_qty;
              remaining -= used_qty;
          }
          
          if remaining > Decimal::ZERO {
              return Err(AppError::InsufficientStock);
          }
          
          Ok(total_cost / quantity)
      }
  }
  ```

- [ ] **步骤3：验证实现**
  ```
  编译: cargo build
  测试: 计算成本并验证结果
  ```

**验收标准**:
- [ ] 支持加权平均法
- [ ] 支持先进先出法
- [ ] 生成库存台账报表

---

### 任务 12-17: 继续实现其他P2级任务

(后续详细步骤)

---

### ============================================================================
### 阶段四：P3级问题修复（88小时）
### ============================================================================

### 任务 18-22: P3级架构优化任务

(后续详细步骤)

---

## 三、验证清单

### P0级验证

| 序号 | 验证项 | 命令 | 预期结果 |
|------|-------|------|---------|
| 1 | 认证中间件 | 无Token请求返回401 | cargo build && curl无Token请求 |
| 2 | 防暴力攻击 | 5次失败后返回429 | curl连续5次登录失败 |
| 3 | CSRF检查 | 跨域POST被拒绝 | curl -X POST -H "Origin: evil.com" |
| 4 | 配置安全 | .env变量生效 | 检查config.yaml无明文密码 |
| 5 | 审计密钥 | 从环境变量读取 | 检查代码无硬编码 |

### P1级验证

| 序号 | 验证项 | 命令 | 预期结果 |
|------|-------|------|---------|
| 6 | 用户身份 | 操作记录正确用户 | 查看audit_logs表 |
| 7 | 输入验证 | 超长输入被拒绝 | curl发送超长字段 |
| 8 | Token过期 | 2小时后Token失效 | 等待或调整时钟 |
| 9 | 订单历史 | 查看变更记录 | GET /api/sales/orders/{id}/history |
| 10 | 交期计算 | 返回建议日期 | GET /api/purchase/delivery-date |

### P2级验证

(后续详细验证步骤)

### P3级验证

(后续详细验证步骤)

---

## 四、总结

### 执行顺序

1. **P0级** (4.5h) - 必须首先完成，系统安全基石
2. **P1级** (37h) - 核心功能完善
3. **P2级** (89h) - 功能增强与性能优化
4. **P3级** (88h) - 架构优化与扩展

### 关键里程碑

| 里程碑 | 完成时间 | 验收标准 |
|--------|---------|---------|
| M1: P0完成 | 第1天 | 所有安全漏洞修复 |
| M2: P1完成 | 第6天 | 核心功能完善 |
| M3: P2完成 | 第17天 | 系统稳定运行 |
| M4: P3完成 | 第28天 | 系统可扩展 |

---

**计划制定完成**
