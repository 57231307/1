# TLS 合规与迁移后置校验扫描报告

- 扫描日期：2026-08-22
- 扫描范围：`backend/Cargo.toml`（reqwest TLS 配置）、`deploy/nginx.conf`（Nginx TLS 配置）、`backend/src/services/init_service_ops/setup.rs`（迁移后置校验）
- 关联任务：4.10 TLS 合规、27.8 迁移后置校验

## 一、4.10 TLS 合规

### 1.1 HTTP 客户端 TLS 配置（reqwest）

**扫描命令**：`grep -rn "reqwest|rustls|native-tls|tls" backend/Cargo.toml`

**扫描结果**：

- 修改前：`reqwest = { version = "0.13", features = ["json"] }`
- 未显式指定 `rustls-tls`，reqwest 0.13 默认启用 `default-tls`（即 native-tls，依赖 OpenSSL/GnuTLS 系统库）。
- 项目出站 HTTP 调用涉及 GitHub API、Webhook、邮件服务、汇率服务等外部接口，需保证 TLS 加密强度。

**修复动作**：

修改 `backend/Cargo.toml:63`，关闭默认 features 并显式启用 rustls-tls + 必要功能：

```toml
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls-tls", "charset", "http2", "stream"] }
```

- `default-features = false`：移除 `default-tls`（native-tls），避免依赖系统 OpenSSL。
- `rustls-tls`：使用纯 Rust 实现的 rustls 作为 TLS 后端，强制走 rustls 加密栈。
- `charset`、`http2`、`stream`：原默认 features 中项目实际用到的功能（`response.chunk()` 需要 `stream`，GitHub 下载需要 http2 支持）。
- 运行时 rustls 支持的 TLS 版本：仅 TLSv1.2 和 TLSv1.3（rustls 默认配置已禁用 TLSv1.0/1.1）。

**合规结论**：

| 检查项 | 状态 | 说明 |
|--------|------|------|
| TLS 后端 | rustls | 纯 Rust 实现，无系统 C 库依赖 |
| TLSv1.0/1.1 | 已禁用 | rustls 默认不支持，reqwest 0.13 + rustls-tls 仅协商 TLSv1.2/1.3 |
| TLSv1.2/1.3 | 支持 | rustls 默认支持，合规 |
| AEAD 套件 | 支持 | rustls 仅允许 AEAD 套件（GCM/ChaCha20-Poly1305），禁止 CBC 等非 AEAD 套件 |
| native-tls 依赖 | 已移除 | `default-features = false` 彻底剔除 |

### 1.2 Nginx TLS 配置

**扫描命令**：`cat deploy/nginx.conf | grep "ssl_protocols|ssl_ciphers"`

**扫描结果**（`deploy/nginx.conf:32-37`）：

```nginx
ssl_protocols TLSv1.2 TLSv1.3;
ssl_ciphers ECDHE-ECDSA-AES128-GCM-SHA256:ECDHE-RSA-AES128-GCM-SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:ECDHE-RSA-AES256-GCM-SHA384:ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305:DHE-RSA-AES128-GCM-SHA256:DHE-RSA-AES256-GCM-SHA384;
ssl_prefer_server_ciphers off;
ssl_session_cache shared:SSL:10m;
ssl_session_timeout 1d;
ssl_session_tickets off;
```

**合规分析**：

| 检查项 | 配置值 | 合规判定 |
|--------|--------|----------|
| ssl_protocols | TLSv1.2 TLSv1.3 | 合规：仅允许 TLSv1.2/1.3，禁用 TLSv1.0/1.1 |
| ssl_ciphers | ECDHE/DHE + GCM/CHACHA20-POLY1305 | 合规：全部为 AEAD 套件 |
| 密钥交换 | ECDHE / DHE | 合规：前向保密（PFS） |
| 认证/加密 | AES128-GCM / AES256-GCM / CHACHA20-POLY1305 | 合规：AEAD 认证加密 |
| ssl_prefer_server_ciphers | off | 合规：TLSv1.3 不受此指令影响，TLSv1.2 下客户端选择更安全（现代浏览器 cipher 列表更优） |
| ssl_session_tickets | off | 合规：禁用会话票据，避免前向保密被弱化 |

**套件逐项核验（全部 AEAD）**：

| 套件 | 密钥交换 | 认证 | 加密 | MAC/AEAD | 判定 |
|------|----------|------|------|----------|------|
| ECDHE-ECDSA-AES128-GCM-SHA256 | ECDHE | ECDSA | AES-128 | GCM (AEAD) | 合规 |
| ECDHE-RSA-AES128-GCM-SHA256 | ECDHE | RSA | AES-128 | GCM (AEAD) | 合规 |
| ECDHE-ECDSA-AES256-GCM-SHA384 | ECDHE | ECDSA | AES-256 | GCM (AEAD) | 合规 |
| ECDHE-RSA-AES256-GCM-SHA384 | ECDHE | RSA | AES-256 | GCM (AEAD) | 合规 |
| ECDHE-ECDSA-CHACHA20-POLY1305 | ECDHE | ECDSA | ChaCha20 | Poly1305 (AEAD) | 合规 |
| ECDHE-RSA-CHACHA20-POLY1305 | ECDHE | RSA | ChaCha20 | Poly1305 (AEAD) | 合规 |
| DHE-RSA-AES128-GCM-SHA256 | DHE | RSA | AES-128 | GCM (AEAD) | 合规 |
| DHE-RSA-AES256-GCM-SHA384 | DHE | RSA | AES-256 | GCM (AEAD) | 合规 |

**Nginx TLS 合规结论**：

- 协议版本：仅 TLSv1.2/1.3，禁用 TLSv1.0/1.1，合规。
- 加密套件：8 个套件全部为 AEAD（GCM 或 ChaCha20-Poly1305），无 CBC/RC4/3DES 等弱套件，合规。
- 前向保密：全部套件使用 ECDHE 或 DHE 密钥交换，支持 PFS，合规。

## 二、27.8 迁移后置校验

### 2.1 初始化流程分析

**扫描文件**：`backend/src/services/init_service_ops/setup.rs`

初始化流程（`initialize` 方法，setup.rs:80-127）：

1. `check_initialized()` — 检查是否已初始化
2. `run_migrations()` — 执行 SeaORM 迁移脚本
3. `hash_password_async()` — Argon2id 哈希管理员密码
4. `create_default_roles()` + `create_default_departments()` — 并行创建默认角色和部门
5. `create_default_role_permissions()` — 创建角色权限记录
6. `create_default_role_conflicts()` — 创建角色互斥规则
7. `create_admin_user()` — 创建管理员用户
8. **新增**：`verify_migration()` — 迁移后置校验

### 2.2 verify_migration() 实现

**位置**：`backend/src/services/init_service_ops/setup.rs:298-380`

**校验逻辑**：

#### 第一步：关键表存在性检查

通过 `information_schema.tables` 查询，检查以下 6 张核心表是否存在：

| 表名 | 来源 | 用途 |
|------|------|------|
| users | m0001 初始 schema | 用户主表 |
| roles | m0001 初始 schema | 角色主表 |
| departments | m0001 初始 schema | 部门主表 |
| role_permissions | m0001 初始 schema | 角色权限关联表 |
| user_departments | m0001 初始 schema | 用户部门关联表 |
| operation_logs | m0001 初始 schema | 操作日志表 |

查询 SQL（参数化，防注入）：

```sql
SELECT 1 FROM information_schema.tables
WHERE table_schema = 'public' AND table_name = $1
```

#### 第二步：主数据表行数检查

检查以下 3 张主数据表行数 > 0：

| 表名 | 预期数据 | 初始化来源 |
|------|----------|------------|
| roles | > 0 | `create_default_roles()` 创建 admin/manager/operator |
| departments | > 0 | `create_default_departments()` 创建默认部门 |
| role_permissions | > 0 | `create_default_role_permissions()` 创建权限记录 |

查询 SQL：

```sql
SELECT COUNT(*) AS cnt FROM "<table_name>"
```

### 2.3 失败处理策略

| 场景 | 处理方式 | 原因 |
|------|----------|------|
| 表不存在 | `tracing::warn` 告警 | 不阻塞启动，避免误杀 |
| 行数为 0 | `tracing::warn` 告警 | 可能是全新环境未执行初始化，非致命 |
| 查询失败 | `tracing::warn` 告警 | 数据库异常已由上层处理，此处仅记录 |
| 全部通过 | `tracing::info` 记录 | 确认迁移成功 |

**设计决策**：后置校验仅告警不阻塞，因为：

1. 初始化流程中 `run_migrations()` 已对迁移失败做了 `?` 错误传播。
2. 主数据创建各步骤已对失败做了 `?` 错误传播。
3. `verify_migration()` 是"双保险"校验，目的是发现迁移脚本成功执行但数据异常的边缘情况（如手动改库、迁移部分回滚），不应因校验本身的问题阻塞正常启动。

### 2.4 调用位置

在 `initialize` 方法最后一步（setup.rs:122-124），`create_admin_user()` 成功后、返回 `InitializationResult` 前调用：

```rust
self.create_admin_user(admin_username, &password_hash, admin_role.id, department_id)
    .await?;

// 27.8 迁移后置校验：初始化完成后校验关键表存在性 + 主数据行数
// 失败仅告警，不阻塞启动
self.verify_migration().await;

Ok(InitializationResult { ... })
```

## 三、修改文件列表

| 文件 | 修改类型 | 说明 |
|------|----------|------|
| `backend/Cargo.toml` | 修改 | reqwest 加 `rustls-tls` feature，禁用 default-features（移除 native-tls） |
| `backend/src/services/init_service_ops/setup.rs` | 新增方法 + 调用 | 新增 `verify_migration()` 后置校验函数，在 `initialize()` 成功后调用 |

## 四、结论

### 4.10 TLS 合规

- reqwest HTTP 客户端：已切换至 rustls-tls 后端，仅支持 TLSv1.2/1.3 + AEAD 套件，移除 native-tls 系统依赖，合规。
- Nginx 反向代理：`ssl_protocols TLSv1.2 TLSv1.3`，`ssl_ciphers` 全部为 AEAD 套件（GCM/ChaCha20-Poly1305），无弱套件，合规。

### 27.8 迁移后置校验

- 已实现 `verify_migration()` 后置校验函数，检查 6 张关键表存在性 + 3 张主数据表行数 > 0。
- 失败时 `tracing::warn` 告警，不阻塞启动。
- 已在初始化成功后调用（`initialize` 方法末尾）。
