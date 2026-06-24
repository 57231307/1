# 安全漏洞详细记录

> 本文档记录 2026-06-24 周期性安全审计中确认的漏洞。
> 审计范围：仓库全量代码，重点关注高风险攻击面。
> 审计依据：可论证的端到端利用路径（仅记录已确认漏洞，不含推测性风险）。

---

## 漏洞总览

| 编号 | 严重度 | 漏洞名称 | 状态 |
|------|--------|----------|------|
| #1 | 🔴 高危 | 静态资源路径遍历漏洞 | 未修复 |
| #2 | 🔴 高危 | WebSocket 通知服务认证绕过 | 未修复 |
| #3 | 🟠 中危 | 系统初始化接口匿名访问风险 | 未修复 |
| #4 | 🟡 低危 | 错误信息泄露内部细节 | 未修复 |
| #5 | 🟡 低危 | API Key 撤销后仍可被冒用 | 未修复 |
| #6 | 🟡 低危 | 内存速率限制器多实例失效 | 未修复 |
| #7 | 🟡 低危 | 弱密码黑名单策略不严 | 未修复 |
| #8 | 🟡 低危 | 调试模式错误响应泄露堆栈信息 | 未修复 |

---

## 漏洞 #1：静态资源路径遍历漏洞（Path Traversal）

### 基础信息

- **严重度**：🔴 高危（High）
- **CWE 分类**：CWE-22 路径遍历
- **CVSS 3.1 评分估计**：7.5（AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:N/A:N）
- **发现时间**：2026-06-24
- **影响版本**：当前 main 分支
- **修复状态**：未修复

### 漏洞位置

- 主文件：[routes/static.rs](file:///workspace/backend/src/routes/static.rs#L18-L57)
- 关键代码行：[22-57](file:///workspace/backend/src/routes/static.rs#L22-L57)

### 攻击者画像

- **类型**：外部匿名用户
- **所需权限**：无需任何认证
- **攻击入口**：`GET /static/*path` 路由

### 可控输入向量

URL 路径参数 `path`（通过 `/static/*path` 通配路由传入）。

### 完整利用路径

1. 攻击者构造恶意请求：`GET /static/../../../etc/passwd`
2. 请求进入 [static_assets_handler()](file:///workspace/backend/src/routes/static.rs#L18) 函数
3. 第 24 行通过 `Path<String>` 提取器获取用户输入的 `path` 变量
4. 第 27 行直接将用户可控的 `path` 拼接到目录路径：
   ```rust
   let static_path = format!("{}/{}", static_dir, path);
   ```
5. 第 28 行直接读取文件：
   ```rust
   tokio::fs::read(&static_path).await
   ```
6. 第 41 行还有第二个回退路径读取，同样无任何校验
7. **无任何路径规范化、`..` 段过滤或目录边界校验**

### 影响分析

- **数据泄露**：任意文件读取，攻击者可读取服务器上的任意文件
- **敏感信息暴露**：
  - `/etc/passwd`、`/etc/shadow` 等系统文件
  - 配置文件（如 `config.yaml`、`/etc/nginx/nginx.conf`）
  - 环境变量文件（`.env`）
  - 源代码（包含业务逻辑、密钥处理逻辑）
  - JWT 密钥、数据库密码等敏感配置
- **连锁风险**：结合 `.env` 文件或配置文件泄露，可能进一步获取数据库密码、JWT 密钥等，导致完全入侵
- **影响范围**：所有部署该服务的实例

### 修复建议

#### 推荐方案（优先级 1）

1. 使用 `std::fs::canonicalize()` 规范化路径后，验证结果路径是否在允许的目录范围内
2. 过滤掉包含 `..` 或 `//` 的路径段
3. 优先使用 `tower-http` 的 `ServeDir` 服务替代手写的文件读取逻辑

#### 示例代码

```rust
use std::path::{Component, Path, PathBuf};

fn sanitize_path(path: &str) -> Option<PathBuf> {
    let p = Path::new(path);
    if p.components().any(|c| matches!(c, Component::ParentDir)) {
        return None; // 拒绝包含 .. 的路径
    }
    Some(p.to_path_buf())
}

// 在 handler 中使用
let safe_path = match sanitize_path(&path) {
    Some(p) => p,
    None => return Err(StatusCode::BAD_REQUEST.into_response()),
};
```

#### 短期缓解

- 在 Nginx 等反向代理层添加 `location ~ \.\.` 规则拒绝包含 `..` 的请求
- 限制 `static_dir` 目录的权限

---

## 漏洞 #2：WebSocket 通知服务认证绕过

### 基础信息

- **严重度**：🔴 高危（High）
- **CWE 分类**：CWE-287 认证不当、CWE-285 授权不当
- **CVSS 3.1 评分估计**：8.1（AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:N）
- **发现时间**：2026-06-24
- **影响版本**：当前 main 分支
- **修复状态**：未修复

### 漏洞位置

- 主文件：[websocket/notifications.rs](file:///workspace/backend/src/websocket/notifications.rs)
- 关键代码行：[198-222](file:///workspace/backend/src/websocket/notifications.rs#L198-L222)
- 路由挂载：[routes/system.rs](file:///workspace/backend/src/routes/system.rs#L26-L35)

### 攻击者画像

- **类型**：外部匿名用户
- **所需权限**：无需任何认证
- **攻击入口**：`GET /ws/notifications?token=<token>`

### 可控输入向量

URL query 参数 `token`，通过 WebSocket 握手传入。

### 完整利用路径

1. 攻击者构造恶意请求：`GET /ws/notifications?token=1:1`
2. 请求进入 [ws_notifications_handler()](file:///workspace/backend/src/websocket/notifications.rs#L168) 函数
3. 第 184 行调用 `verify_jwt_token()` 进行"JWT 验证"
4. 但 [verify_jwt_token()](file:///workspace/backend/src/websocket/notifications.rs#L202) 函数只是占位实现，**未实际进行 JWT 签名验证**：
   ```rust
   // 临时占位：实际接入 jsonwebtoken 验证
   // 解析逻辑：token 格式 `<tenant_id>:<user_id>`（仅用于 demo）
   let parts: Vec<&str> = token.split(':').collect();
   ```
5. 只要格式为 `<tenant_id>:<user_id>` 且为正整数，就能通过"验证"
6. 成功建立 WebSocket 连接，注册到对应租户的对应用户频道
7. 攻击者随后可接收该用户的所有系统通知

### 影响分析

- **身份冒充**：攻击者可冒充**任意租户**的**任意用户**建立 WebSocket 连接
- **数据泄露**：可接收该用户的所有系统通知（可能包含敏感业务数据、个人信息、订单状态变更等）
- **攻击能力**：
  - 可向服务端发送 WebSocket 消息（标记已读、心跳等）
  - 接收其他用户推送的实时通知
  - 探测租户 ID 和用户 ID 范围
- **绕过防护**：与 HTTP 接口的认证体系不一致，绕过 `auth_middleware`

### 修复建议

#### 核心修复（必做）

1. 接入真实的 JWT 验证逻辑，复用 [auth_service.rs](file:///workspace/backend/src/services/auth_service.rs) 中的 `validate_token_static()` 函数
2. 移除占位的 `verify_jwt_token()` 函数及其测试代码
3. WebSocket 握手阶段的认证应与 HTTP 接口使用相同的安全强度

#### 示例代码

```rust
fn verify_jwt_token(token: &str, secret: &str) -> Result<AuthInfo, String> {
    use crate::services::auth_service::AuthService;
    
    let claims = AuthService::validate_token_static(token, secret)
        .map_err(|e| format!("JWT 验证失败: {}", e))?;
    
    Ok(AuthInfo {
        tenant_id: claims.tenant_id.ok_or("缺少 tenant_id")? as i64,
        user_id: claims.sub as i64,
    })
}
```

#### 加固建议

- 验证 `tenant_id` 与 `user_id` 的合法性（查询数据库确认存在）
- 添加 WebSocket 连接的频率限制
- 记录异常的 WebSocket 认证失败尝试

---

## 漏洞 #3：系统初始化接口匿名访问风险

### 基础信息

- **严重度**：🟠 中危（Medium）
- **CWE 分类**：CWE-1188 不安全初始化、CWE-306 关键功能缺少认证
- **CVSS 3.1 评分估计**：6.5（AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:H/A:N）
- **发现时间**：2026-06-24
- **影响版本**：当前 main 分支
- **修复状态**：未修复

### 漏洞位置

- 公开路径白名单：[middleware/public_routes.rs](file:///workspace/backend/src/middleware/public_routes.rs#L9)
- 初始化路由注册：[routes/system.rs](file:///workspace/backend/src/routes/system.rs#L236-L254)
- 初始化处理器：[handlers/init_handler.rs](file:///workspace/backend/src/handlers/init_handler.rs)

### 攻击者画像

- **类型**：外部匿名用户
- **所需权限**：无需任何认证
- **攻击时机**：系统首次部署时（数据库表为空状态）

### 可控输入向量

- 数据库配置参数（host、port、name、username、password）
- 管理员用户名、管理员密码

### 完整利用路径

1. `/api/v1/erp/init` 前缀在 [PUBLIC_PATHS](file:///workspace/backend/src/middleware/public_routes.rs#L1-L14) 中被标记为公开路径
2. `auth_middleware` 对该路径下的所有请求直接跳过 JWT 认证
3. 以下接口完全匿名可访问：
   - `/api/v1/erp/init/initialize` - [initialize_system()](file:///workspace/backend/src/handlers/init_handler.rs#L158-L169)
   - `/api/v1/erp/init/initialize-with-db` - [initialize_system_with_db()](file:///workspace/backend/src/handlers/init_handler.rs#L171-L182)
4. 虽然 `initialize()` 内部会调用 `check_initialized()` 检查系统是否已初始化，但在**系统首次部署时**（数据库无 users 表或 users 表为空）：
   - 攻击者可抢先访问初始化接口
   - 设置自己的管理员用户名和密码
   - 接管整个 ERP 系统
5. 部分接口（`test-database`、`task-status`、`reset-password`）在 handler 层有 admin 权限二次校验，**但 initialize 接口无此类校验**

### 影响分析

- **窗口期攻击**：系统首次部署窗口期存在被攻击者抢先初始化的风险
- **完全接管**：成功利用后攻击者获得管理员权限，可完全控制系统和所有租户数据
- **隐蔽性**：管理员难以发现系统已被攻击者初始化（直到合法管理员尝试初始化时才发现"已初始化"）
- **持续性**：一旦初始化，攻击者的管理员账号长期有效

### 修复建议

#### 立即修复（必做）

1. 初始化接口应增加额外的安全机制（如初始化令牌）
2. 初始化令牌应在环境变量或配置文件中预先设置，而非完全开放
3. 限制只有本地网络或特定 IP 段才能访问初始化接口

#### 增强防护

- 系统初始化后应立即锁定初始化接口（即使数据库表为空也不应无限期开放）
- 添加初始化操作审计日志（记录来源 IP、请求时间）
- 提供"已初始化"状态查询但不开放初始化接口

#### 示例代码

```rust
// 在 init 路径下添加初始化令牌校验中间件
async fn init_token_middleware(
    headers: HeaderMap,
    request: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let init_token = std::env::var("INIT_TOKEN")
        .map_err(|_| "INIT_TOKEN 未配置")?;
    let provided = headers.get("X-Init-Token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| unauthorized_response("缺少初始化令牌"))?;
    
    if !constant_time_eq(provided.as_bytes(), init_token.as_bytes()) {
        return Err(unauthorized_response("初始化令牌无效"));
    }
    
    Ok(next.run(request).await)
}
```

---

## 漏洞 #4：错误信息泄露内部细节

### 基础信息

- **严重度**：🟡 低危（Low）
- **CWE 分类**：CWE-209 生成包含敏感信息的错误信息、CWE-200 信息泄露
- **CVSS 3.1 评分估计**：3.7（AV:N/AC:H/PR:N/UI:N/S:U/C:L/I:N/A:N）
- **发现时间**：2026-06-24
- **影响版本**：当前 main 分支
- **修复状态**：未修复

### 漏洞位置

- 主文件：[utils/error.rs](file:///workspace/backend/src/utils/error.rs)
- 关键代码行：[297-315](file:///workspace/backend/src/utils/error.rs#L297-L315)、[90-101](file:///workspace/backend/src/utils/error.rs#L90-L101)

### 攻击者画像

- **类型**：外部匿名用户 / 已认证用户
- **所需权限**：无需权限，触发任意类型的错误即可
- **攻击条件**：系统未设置 `APP_ENV=production` 环境变量

### 可控输入向量

任意 HTTP 请求（通过触发不同类型错误获取错误响应）。

### 完整利用路径

1. 当系统未设置 `APP_ENV=production` 时（即开发/测试模式）
2. 服务端错误响应会携带 `error_type` 和 `detail` 字段（第 312-313 行）：
   ```rust
   serde_json::json!({
       "code": self.error_code(),
       "message": error_message,
       "trace_id": trace_id,
       "timestamp": timestamp,
       "error_type": error_type,
       "detail": log_detail,
   })
   ```
3. `detail` 字段包含 `severity` / `action_required` 等内部建议
4. `DatabaseError` 还会泄露数据库错误原始内容（[第 91-101 行](file:///workspace/backend/src/utils/error.rs#L90-L101)）
5. 攻击者通过触发不同类型的错误，可枚举系统内部结构

### 影响分析

- **协助攻击者识别后端技术栈**：错误分类、异常类型（如 `DatabaseError`、`ValidationError`）
- **泄露数据库查询错误细节**：列名、约束名（如 `unique constraint`、`foreign key constraint`）
- **暴露内部错误处理策略**：通过 `severity` 字段判断哪些操作更敏感
- **间接风险**：这些信息可被用于后续的针对性攻击

### 修复建议

#### 必做修复

- 部署时强制要求 `APP_ENV=production`
- 在 `IntoResponse` 中根据环境变量做基础脱敏，仅 `tracing` 详细日志
- CI/CD 流程增加 `APP_ENV` 检查门禁

#### 推荐修复

- 即使是开发环境，也只暴露 `code` 和 `message`（脱敏后），将 `error_type` 和 `detail` 仅写入 `tracing`
- 提供"详细错误模式"开关，由开发者在本地主动启用

---

## 漏洞 #5：API Key 撤销后仍可被冒用

### 基础信息

- **严重度**：🟡 低危（Low）
- **CWE 分类**：CWE-613 不足的会话过期
- **CVSS 3.1 评分估计**：4.3（AV:N/AC:H/PR:L/UI:N/S:U/C:L/I:L/A:N）
- **发现时间**：2026-06-24
- **影响版本**：当前 main 分支
- **修复状态**：未修复

### 漏洞位置

- 主文件：[services/api_key_service.rs](file:///workspace/backend/src/services/api_key_service.rs)
- 关键代码行：[69-85](file:///workspace/backend/src/services/api_key_service.rs#L69-L85)

### 攻击者画像

- **类型**：持有旧 API 密钥的内部服务/已被撤销密钥的用户
- **所需权限**：需要已经获得过有效的 API Key

### 可控输入向量

API Key 字符串（明文）。

### 完整利用路径

1. API 密钥撤销逻辑仅设置 `is_active = false`（[第 80-82 行](file:///workspace/backend/src/services/api_key_service.rs#L80-L82)）：
   ```rust
   let mut active_model: ApiKeyActiveModel = key.into();
   active_model.is_active = Set(false);
   active_model.updated_at = Set(Utc::now());
   active_model.update(self.db.as_ref()).await?;
   ```
2. 没有清理 `key_hash` 或加入撤销黑名单
3. 旧的明文 API Key 在撤销后无法被强制失效（仅依赖调用方不再使用）
4. 若 API Key 已被泄露并被攻击者截获，撤销仅阻止新建使用，**已泄露的密钥仍能通过撤销前缓存的副本使用**

### 影响分析

- **缺乏 API Key 强制吊销机制**
- **已泄露的密钥在撤销后仍可继续使用**（直到调用方主动清除）
- **缺乏审计追踪**：无法区分正常调用与已撤销密钥的冒用

### 修复建议

#### 必做修复

1. 添加 API Key 黑名单（Redis 存储已撤销的 key_hash）
2. 在认证中间件中检查黑名单
3. 撤销时主动通知相关服务刷新缓存

#### 示例代码

```rust
pub async fn revoke_api_key(&self, id: i32, tenant_id: i32) -> Result<(), AppError> {
    let key = ApiKey::find_by_id(id)
        .one(self.db.as_ref())
        .await?
        .ok_or_else(|| AppError::business("API 密钥不存在"))?;
    
    if key.tenant_id != tenant_id {
        return Err(AppError::permission_denied("无权操作此API密钥"));
    }
    
    // 加入黑名单（Redis 存储）
    let blacklist_key = format!("apikey:blacklist:{}", key.key_hash);
    redis_client.set_ex(&blacklist_key, "1", 86400 * 7).await?; // 7 天
    
    let mut active_model: ApiKeyActiveModel = key.into();
    active_model.is_active = Set(false);
    active_model.updated_at = Set(Utc::now());
    active_model.update(self.db.as_ref()).await?;
    
    Ok(())
}
```

---

## 漏洞 #6：内存速率限制器多实例失效

### 基础信息

- **严重度**：🟡 低危（Low）
- **CWE 分类**：CWE-770 无限制或受限的资源分配
- **CVSS 3.1 评分估计**：5.3（AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:L/A:L）
- **发现时间**：2026-06-24
- **影响版本**：当前 main 分支
- **修复状态**：未修复

### 漏洞位置

- 主文件：[middleware/rate_limit.rs](file:///workspace/backend/src/middleware/rate_limit.rs)
- 关键代码行：[76-79](file:///workspace/backend/src/middleware/rate_limit.rs#L76-L79)

### 攻击者画像

- **类型**：外部攻击者
- **所需权限**：无需任何权限
- **攻击条件**：多实例部署 + 负载均衡

### 可控输入向量

大量并发请求（通过不同 IP 分散到不同实例）。

### 完整利用路径

1. 全局限流器使用进程内 `LazyLock<MemoryRateLimiter>` 存储
2. 多实例部署时每个实例独立维护限流计数
3. 攻击者通过负载均衡器将请求分散到多个实例（请求数 ÷ 实例数）
4. 实际限流阈值 = 配置值 × 实例数
5. 单实例部署不受影响

### 影响分析

- **暴力破解防护被绕过**：登录接口的限流被分散
- **限流配置 N → 实际允许 N × 实例数 请求**
- **资源耗尽风险**：多实例部署下，整体限流防护显著弱化

### 修复建议

#### 必做修复

1. 使用 Redis 集中存储限流计数（项目已有 Redis 依赖）
2. 通过 `INCR` + `EXPIRE` 原子操作实现分布式限流

#### 示例代码

```rust
async fn rate_limit_redis(redis: &RedisClient, key: &str, max: i32, window: i32) -> bool {
    let count: i32 = redis.incr(key, 1).await.unwrap_or(0);
    if count == 1 {
        redis.expire(key, window).await.ok();
    }
    count <= max
}
```

---

## 漏洞 #7：弱密码黑名单策略不严

### 基础信息

- **严重度**：🟡 低危（Low）
- **CWE 分类**：CWE-521 弱密码要求
- **CVSS 3.1 评分估计**：3.7（AV:N/AC:H/PR:N/UI:N/S:U/C:L/I:N/A:N）
- **发现时间**：2026-06-24
- **影响版本**：当前 main 分支
- **修复状态**：未修复

### 漏洞位置

- 主文件：[utils/password_validator.rs](file:///workspace/backend/src/utils/password_validator.rs)
- 关键代码行：[84-100](file:///workspace/backend/src/utils/password_validator.rs#L84-L100)、[152-159](file:///workspace/backend/src/utils/password_validator.rs#L152-L159)

### 攻击者画像

- **类型**：已认证用户（注册/改密流程）
- **所需权限**：需要基本用户权限

### 可控输入向量

密码字段。

### 完整利用路径

1. `validate_password_strength` 通过 `validator` crate 的 `custom` 函数校验
2. 密码策略的 `is_valid = errors.is_empty()` 规则下，**所有校验项失败会平权扣分**
3. COMMON_PASSWORDS 检查仅扣 30 分（[第 158 行](file:///workspace/backend/src/utils/password_validator.rs#L158)）
4. 一个含 16+ 字符、含大小写数字特殊字符的密码即使命中 COMMON_PASSWORDS 黑名单，仍可能因分数 ≥ 50 通过 Medium 等级校验

### 影响分析

- **弱密码（虽然长了但被列入常见黑名单）可能被接受**
- **缺少强密码检测**：如键盘连续字符、特定人名、生日等
- **黑名单覆盖范围有限**：仅 15 个常见密码

### 修复建议

#### 必做修复

1. COMMON_PASSWORDS 命中应作为硬性拒绝（`is_valid = false`），而非仅扣分
2. 考虑接入 HIBP（Have I Been Pwned）API 或更大规模的弱密码库

#### 示例代码

```rust
// 修改 common passwords 检查逻辑
let lower_password = password.to_lowercase();
if COMMON_PASSWORDS
    .iter()
    .any(|common| lower_password.contains(common))
{
    errors.push("密码过于常见，不安全".to_string());
    // 不再扣分，直接保留在 errors 中，由 is_valid = errors.is_empty() 决定
}

// 或者更严格：作为硬性拒绝
if COMMON_PASSWORDS.iter().any(|common| lower_password == *common) {
    return PasswordValidationResult {
        strength: PasswordStrength::VeryWeak,
        is_valid: false,
        errors: vec!["密码过于常见，不允许使用".to_string()],
    };
}
```

---

## 漏洞 #8：调试模式错误响应泄露堆栈信息

### 基础信息

- **严重度**：🟡 低危（Low）
- **CWE 分类**：CWE-209 生成包含敏感信息的错误信息
- **CVSS 3.1 评分估计**：3.1（AV:N/AC:H/PR:N/UI:N/S:U/C:L/I:N/A:N）
- **发现时间**：2026-06-24
- **影响版本**：当前 main 分支
- **修复状态**：未修复

### 漏洞位置

- 主文件：[utils/error.rs](file:///workspace/backend/src/utils/error.rs)
- 关键代码行：[88](file:///workspace/backend/src/utils/error.rs#L88)、[297-315](file:///workspace/backend/src/utils/error.rs#L297-L315)

### 攻击者画像

- **类型**：外部匿名用户
- **所需权限**：无需任何权限

### 可控输入向量

触发各种异常类型的请求。

### 完整利用路径

1. 当 `APP_ENV` 未设置为 `production` 时
2. 错误响应中 `message` 字段直接返回 `Display` 完整内容
3. 部分错误携带 SQL 语句、文件路径、堆栈信息
4. 多处错误处理代码（如 [services/import_export_service.rs](file:///workspace/backend/src/services/import_export_service.rs)）会透传内部异常信息到响应中

### 影响分析

- 协助攻击者进行系统探测
- 暴露后端文件路径结构、SQL 语句结构
- 在非生产环境部署时同样危险（如开发/测试环境对外开放时）

### 修复建议

#### 必做修复

1. 默认始终脱敏，仅在 `tracing` 日志中输出完整信息
2. CI/CD 流程强制要求 `APP_ENV=production`
3. 服务启动时检测环境变量，未设置时打印警告

#### 示例代码

```rust
// 在 main.rs 启动时增加检查
if std::env::var("APP_ENV").is_err() {
    eprintln!("警告：APP_ENV 未设置，将按开发模式处理（可能泄露内部信息）");
    eprintln!("生产部署请设置 APP_ENV=production");
}
```

---

## 修复优先级建议

| 优先级 | 漏洞编号 | 预计修复时间 | 风险等级 |
|--------|----------|--------------|----------|
| P0 - 立即修复 | #1, #2 | 1-2 天 | 高危 |
| P1 - 本周内修复 | #3 | 3-5 天 | 中危 |
| P2 - 计划修复 | #4, #5, #6, #7, #8 | 2-3 周 | 低危 |

---

## 审计方法论

### 审计范围

1. **认证与访问控制**：登录流程、会话管理、角色/权限校验
2. **注入向量**：原始 SQL 查询、Shell 命令拼接、模板渲染、文件路径操作
3. **外部交互**：Webhook 处理器、出站网络请求、第三方 API 集成
4. **敏感数据处理**：代码或配置中的密钥、凭证或 PII 的日志记录、加密实践

### 审计方法

1. 梳理代码库架构 - 理解入口点、信任边界、数据流转
2. 系统性检查高风险攻击面
3. 对每个潜在发现，从攻击者可控输入到影响结果追踪完整代码路径
4. 仅保留能具体证明可利用性的发现

### 证据要求

每个报告的问题必须清晰说明：
- 攻击者画像（外部用户、已认证用户、内部服务等）
- 其可控的输入向量
- 从输入到漏洞的确切代码路径
- 造成的影响（数据泄露、权限提升、拒绝服务等）
- 建议的修复方案

---

**记录时间**：2026-06-24
**审计工具**：人工审计
**下次审计**：建议 30 天后
