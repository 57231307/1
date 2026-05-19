# 冰溪 ERP 综合修复实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 完成 6 个高优先级修复任务：补全 purchase_orders 表、排查 gRPC 启动失败、实现 Redis 限流、实现 BPM 条件评估、排查 CI 缓存问题、部署 v2026.519.1900。

**Architecture:** 按依赖顺序执行，先修复数据库和代码问题，再构建部署。

**Tech Stack:** Rust + Axum + SeaORM + PostgreSQL + Redis + GitHub Actions

---

## 执行顺序总览

```
任务1 → 任务2 → 任务3 → 任务4 → 任务5 → 任务6
(数据库)  (gRPC)   (Redis)  (BPM)    (CI)    (部署)
```

---

## 任务 1: 修复 purchase_orders 表缺失问题

**问题分析:**
- 迁移脚本 `001_consolidated_schema.sql` 中创建的是 `purchase_order` (单数) 表
- 但后端代码和后续迁移脚本引用的是 `purchase_orders` (复数) 表
- 模型文件 `purchase_order.rs` 使用 `table_name = "purchase_order"`
- 迁移 `008_core_foreign_keys.sql` 引用 `purchase_orders`，`010_inventory_foreign_keys.sql` 也引用
- 这导致外键约束添加失败，打印警告

**解决方案:** 创建兼容视图或重命名表，使 `purchase_orders` 可用。

**Files:**
- Create: `backend/database/migration/020_fix_purchase_orders_table.sql`
- Modify: `backend/src/models/purchase_order.rs`

---

- [ ] **Step 1: 创建修复迁移脚本**

```sql
-- 迁移脚本: 020_fix_purchase_orders_table.sql
-- 描述: 为 purchase_order 表创建 purchase_orders 视图/别名，解决外键引用问题
-- 日期: 2026-05-19

BEGIN;

-- 检查 purchase_order 表是否存在而 purchase_orders 不存在
DO $$
BEGIN
    -- 如果 purchase_orders 表不存在但 purchase_order 存在，创建视图
    IF EXISTS (
        SELECT 1 FROM information_schema.tables 
        WHERE table_name = 'purchase_order'
    ) AND NOT EXISTS (
        SELECT 1 FROM information_schema.tables 
        WHERE table_name = 'purchase_orders'
    ) THEN
        -- 创建同义词视图，使 purchase_orders 可用
        CREATE OR REPLACE VIEW purchase_orders AS
        SELECT * FROM purchase_order;
        
        -- 为视图创建触发器，使 INSERT/UPDATE/DELETE 可操作
        CREATE OR REPLACE FUNCTION purchase_orders_view_trigger()
        RETURNS TRIGGER AS $$
        BEGIN
            IF TG_OP = 'INSERT' THEN
                INSERT INTO purchase_order (
                    order_no, supplier_id, order_date, expected_delivery_date,
                    actual_delivery_date, warehouse_id, department_id, purchaser_id,
                    currency, exchange_rate, total_amount, total_amount_foreign,
                    total_quantity, total_quantity_alt, order_status, payment_terms,
                    shipping_terms, notes, attachment_urls, created_by, created_at,
                    updated_by, updated_at, approved_by, approved_at, rejected_reason
                ) VALUES (
                    NEW.order_no, NEW.supplier_id, NEW.order_date, NEW.expected_delivery_date,
                    NEW.actual_delivery_date, NEW.warehouse_id, NEW.department_id, NEW.purchaser_id,
                    NEW.currency, NEW.exchange_rate, NEW.total_amount, NEW.total_amount_foreign,
                    NEW.total_quantity, NEW.total_quantity_alt, NEW.order_status, NEW.payment_terms,
                    NEW.shipping_terms, NEW.notes, NEW.attachment_urls, NEW.created_by, NEW.created_at,
                    NEW.updated_by, NEW.updated_at, NEW.approved_by, NEW.approved_at, NEW.rejected_reason
                );
                RETURN NEW;
            ELSIF TG_OP = 'UPDATE' THEN
                UPDATE purchase_order SET
                    order_no = NEW.order_no,
                    supplier_id = NEW.supplier_id,
                    order_date = NEW.order_date,
                    expected_delivery_date = NEW.expected_delivery_date,
                    actual_delivery_date = NEW.actual_delivery_date,
                    warehouse_id = NEW.warehouse_id,
                    department_id = NEW.department_id,
                    purchaser_id = NEW.purchaser_id,
                    currency = NEW.currency,
                    exchange_rate = NEW.exchange_rate,
                    total_amount = NEW.total_amount,
                    total_amount_foreign = NEW.total_amount_foreign,
                    total_quantity = NEW.total_quantity,
                    total_quantity_alt = NEW.total_quantity_alt,
                    order_status = NEW.order_status,
                    payment_terms = NEW.payment_terms,
                    shipping_terms = NEW.shipping_terms,
                    notes = NEW.notes,
                    attachment_urls = NEW.attachment_urls,
                    created_by = NEW.created_by,
                    created_at = NEW.created_at,
                    updated_by = NEW.updated_by,
                    updated_at = NEW.updated_at,
                    approved_by = NEW.approved_by,
                    approved_at = NEW.approved_at,
                    rejected_reason = NEW.rejected_reason
                WHERE id = OLD.id;
                RETURN NEW;
            ELSIF TG_OP = 'DELETE' THEN
                DELETE FROM purchase_order WHERE id = OLD.id;
                RETURN OLD;
            END IF;
            RETURN NULL;
        END;
        $$ LANGUAGE plpgsql;
        
        CREATE TRIGGER purchase_orders_view_trigger
        INSTEAD OF INSERT OR UPDATE OR DELETE ON purchase_orders
        FOR EACH ROW EXECUTE FUNCTION purchase_orders_view_trigger();
        
        RAISE NOTICE '已创建 purchase_orders 视图和触发器';
    END IF;
END $$;

-- 如果 purchase_orders 表已存在（可能是后来创建的），检查结构一致性
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.tables 
        WHERE table_name = 'purchase_orders'
    ) AND EXISTS (
        SELECT 1 FROM information_schema.tables 
        WHERE table_name = 'purchase_order'
    ) THEN
        -- 两个表都存在，可能需要同步数据或删除一个
        RAISE NOTICE 'purchase_order 和 purchase_orders 表同时存在，请检查数据一致性';
    END IF;
END $$;

COMMIT;
```

- [ ] **Step 2: 修改模型文件以支持复数表名**

修改 `backend/src/models/purchase_order.rs`:

```rust
#[sea_orm(table_name = "purchase_orders")]  // 改为复数形式
```

- [ ] **Step 3: 验证迁移脚本**

Run: `cd backend && cargo check`
Expected: 编译通过，无表名相关错误

- [ ] **Step 4: 提交更改**

```bash
git add backend/database/migration/020_fix_purchase_orders_table.sql backend/src/models/purchase_order.rs
git commit -m "fix: 创建 purchase_orders 视图解决表名不一致问题"
```

---

## 任务 2: 排查 gRPC 服务启动失败

**问题分析:**
- 日志显示 `gRPC 服务器启动失败: transport error`
- 可能原因：
  1. 端口 50051 被占用
  2. 绑定地址配置错误
  3. TLS/证书问题
  4. tonic 构建问题

**Files:**
- Modify: `backend/src/main.rs`
- Modify: `backend/src/grpc/service.rs`

---

- [ ] **Step 1: 增强 gRPC 启动日志**

修改 `backend/src/main.rs` 中 gRPC 启动部分：

```rust
let grpc_handle = if let Some(grpc_db) = grpc_db_opt {
    let grpc_addr: SocketAddr =
        format!("{}:{}", settings.grpc.host, settings.grpc.port).parse()?;
    let grpc_jwt_secret = settings.auth.jwt_secret.clone();
    
    // 检查端口是否可用
    match tokio::net::TcpListener::bind(grpc_addr).await {
        Ok(listener) => {
            let grpc_addr = listener.local_addr()?;
            drop(listener); // 释放端口，让 gRPC 服务器使用
            
            Some(tokio::spawn(async move {
                // ... 现有服务创建代码 ...
                
                info!("gRPC 服务器监听地址：{}", grpc_addr);
                if let Err(e) = grpc_server.serve(grpc_addr).await {
                    warn!("gRPC 服务器启动失败: {} (地址: {})", e, grpc_addr);
                }
            }))
        }
        Err(e) => {
            warn!("gRPC 端口 {} 不可用: {}，跳过 gRPC 服务启动", grpc_addr, e);
            None
        }
    }
} else {
    info!("数据库未连接，跳过gRPC服务启动");
    None
};
```

- [ ] **Step 2: 添加 gRPC 健康检查端点**

修改 `backend/src/grpc/service.rs`，在 UserService 中添加健康检查：

```rust
async fn health_check(&self, _request: Request<()>) -> Result<Response<HealthResponse>, Status> {
    Ok(Response::new(HealthResponse {
        status: "healthy".to_string(),
    }))
}
```

- [ ] **Step 3: 编译验证**

Run: `cd backend && cargo check`
Expected: 编译通过

- [ ] **Step 4: 提交更改**

```bash
git add backend/src/main.rs backend/src/grpc/service.rs
git commit -m "fix: 增强 gRPC 启动错误处理和日志"
```

---

## 任务 3: 实现 Redis 限流

**问题分析:**
- 当前使用 `DashMap` 内存实现限流，不支持分布式
- TODO 标注需要切换为 Redis

**解决方案:** 使用 `redis` crate + `deadpool-redis` 连接池实现分布式限流。

**Files:**
- Modify: `backend/Cargo.toml`
- Modify: `backend/src/middleware/rate_limit.rs`
- Modify: `backend/src/config/settings.rs`
- Modify: `backend/src/utils/app_state.rs`

---

- [ ] **Step 1: 添加 Redis 依赖**

修改 `backend/Cargo.toml`：

```toml
# Redis
redis = { version = "0.25", features = ["tokio-comp", "connection-manager"] }
deadpool-redis = "0.15"
```

- [ ] **Step 2: 添加 Redis 配置**

修改 `backend/src/config/settings.rs`：

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct RedisConfig {
    pub url: String,
    pub max_connections: usize,
}

// 在 AppSettings 中添加
pub redis: RedisConfig,
```

- [ ] **Step 3: 实现 Redis 限流器**

修改 `backend/src/middleware/rate_limit.rs`：

```rust
use redis::AsyncCommands;
use deadpool_redis::{Config, Pool, Runtime};

pub struct RedisRateLimiter {
    pool: Pool,
    max_requests: usize,
    window_secs: u64,
}

impl RedisRateLimiter {
    pub fn new(redis_url: &str, max_requests: usize, window_secs: u64) -> Result<Self, String> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg.create_pool(Some(Runtime::Tokio1))
            .map_err(|e| format!("Redis 连接池创建失败: {}", e))?;
        
        Ok(Self {
            pool,
            max_requests,
            window_secs,
        })
    }
    
    pub async fn check(&self, key: &str) -> Result<bool, AppError> {
        let mut conn = self.pool.get().await
            .map_err(|e| AppError::InternalError(format!("Redis 连接获取失败: {}", e)))?;
        
        let redis_key = format!("rate_limit:{}", key);
        let count: i64 = conn.incr(&redis_key, 1).await
            .map_err(|e| AppError::InternalError(format!("Redis 操作失败: {}", e)))?;
        
        if count == 1 {
            let _: () = conn.expire(&redis_key, self.window_secs as i64).await
                .map_err(|e| AppError::InternalError(format!("Redis 过期设置失败: {}", e)))?;
        }
        
        Ok(count <= self.max_requests as i64)
    }
}
```

- [ ] **Step 4: 更新中间件使用 Redis 限流**

```rust
pub async fn rate_limit_by_ip(
    State(state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let ip = req.extensions()
        .get::<axum::extract::ConnectInfo<SocketAddr>>()
        .map(|info| info.0.ip().to_string())
        .unwrap_or_else(|| "unknown_ip".to_string());

    let user_id = req.extensions()
        .get::<AuthContext>()
        .map(|auth| auth.user_id.to_string())
        .unwrap_or_else(|| "anonymous".to_string());

    let rate_key = format!("{}:{}", ip, user_id);

    if let Some(redis_limiter) = &state.redis_limiter {
        if !redis_limiter.check(&rate_key).await? {
            tracing::warn!("Rate limit exceeded for {}", rate_key);
            return Err(AppError::TooManyRequests {
                retry_after: Some(60),
                message: "请求过于频繁".to_string(),
            });
        }
    } else {
        // 回退到内存限流
        if !GLOBAL_LIMITER.check(&rate_key) {
            return Err(AppError::TooManyRequests {
                retry_after: Some(60),
                message: "请求过于频繁".to_string(),
            });
        }
    }

    Ok(next.run(req).await)
}
```

- [ ] **Step 5: 更新 AppState**

在 `app_state.rs` 中添加 `redis_limiter` 字段。

- [ ] **Step 6: 编译验证**

Run: `cd backend && cargo check`
Expected: 编译通过

- [ ] **Step 7: 提交更改**

```bash
git add backend/Cargo.toml backend/src/middleware/rate_limit.rs backend/src/config/settings.rs backend/src/utils/app_state.rs
git commit -m "feat: 实现 Redis 分布式限流"
```

---

## 任务 4: 实现 BPM 条件评估

**问题分析:**
- `bpm_service.rs` 第 163 行有 TODO：`evaluate conditions on edges if multiple`
- 当前只取第一条匹配的边，不支持条件分支

**解决方案:** 实现简单的条件表达式评估引擎。

**Files:**
- Modify: `backend/src/services/bpm_service.rs`

---

- [ ] **Step 1: 实现条件评估函数**

在 `bpm_service.rs` 中添加：

```rust
/// 评估 BPM 边条件
/// 支持的条件格式:
/// - `${amount} > 10000` - 变量比较
/// - `${status} == 'APPROVED'` - 字符串比较
/// - `${level} >= 3` - 数值比较
fn evaluate_condition(condition: &str, variables: &Option<serde_json::Value>) -> bool {
    let vars = match variables {
        Some(v) => v,
        None => return false,
    };
    
    // 解析条件表达式
    let condition = condition.trim();
    if condition.is_empty() {
        return true; // 无条件默认通过
    }
    
    // 提取变量名和比较操作
    let re = regex::Regex::new(r"\$\{(\w+)\}\s*(==|!=|>|<|>=|<=)\s*(.+)").unwrap();
    
    if let Some(caps) = re.captures(condition) {
        let var_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let operator = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let expected_value = caps.get(3).map(|m| m.as_str()).unwrap_or("").trim();
        
        // 获取实际变量值
        let actual_value = vars.get(var_name).and_then(|v| {
            v.as_str().map(|s| s.to_string())
                .or_else(|| v.as_i64().map(|i| i.to_string()))
                .or_else(|| v.as_f64().map(|f| f.to_string()))
        });
        
        match actual_value {
            Some(actual) => {
                // 尝试数值比较
                if let (Ok(actual_num), Ok(expected_num)) = (actual.parse::<f64>(), expected_value.parse::<f64>()) {
                    match operator {
                        ">" => actual_num > expected_num,
                        "<" => actual_num < expected_num,
                        ">=" => actual_num >= expected_num,
                        "<=" => actual_num <= expected_num,
                        "==" => (actual_num - expected_num).abs() < f64::EPSILON,
                        "!=" => (actual_num - expected_num).abs() >= f64::EPSILON,
                        _ => false,
                    }
                } else {
                    // 字符串比较
                    let expected = expected_value.trim_matches('\'').trim_matches('"');
                    match operator {
                        "==" => actual == expected,
                        "!=" => actual != expected,
                        _ => false,
                    }
                }
            }
            None => false,
        }
    } else {
        // 无法解析的条件，默认通过
        tracing::warn!("无法解析 BPM 条件: {}", condition);
        true
    }
}
```

- [ ] **Step 2: 修改 approve_task 使用条件评估**

```rust
// 查找匹配的边（支持条件）
let matching_edge = edges.iter().find(|e| {
    let source_match = e.get("source").and_then(|s| s.as_str()) == Some(&task.node_id);
    if !source_match {
        return false;
    }
    
    // 检查条件
    if let Some(condition) = e.get("condition").and_then(|c| c.as_str()) {
        evaluate_condition(condition, &instance.variables)
    } else {
        true // 无条件默认匹配
    }
});

if let Some(edge) = matching_edge {
    // ... 现有逻辑
}
```

- [ ] **Step 3: 编译验证**

Run: `cd backend && cargo check`
Expected: 编译通过

- [ ] **Step 4: 提交更改**

```bash
git add backend/src/services/bpm_service.rs
git commit -m "feat: 实现 BPM 条件表达式评估"
```

---

## 任务 5: 排查 CI 本地缓存问题

**问题分析:**
- CI 编译成功但二进制仍为旧版本
- 可能原因：Cargo 缓存了旧的编译产物

**Files:**
- Modify: `.github/workflows/ci-cd.yml`

---

- [ ] **Step 1: 清理 Cargo 缓存步骤**

在 CI workflow 的 `build-backend` job 中，构建前添加缓存清理：

```yaml
      - name: 清理旧编译产物
        working-directory: backend
        run: |
          cargo clean
          rm -rf target/release/.fingerprint/*
          rm -rf target/release/build/*
          rm -rf target/release/deps/*
          echo "已清理旧编译产物"

      - name: 构建后端 (Release模式)
        working-directory: backend
        run: |
          cargo build --release --bin server --bin bingxi
```

- [ ] **Step 2: 添加构建产物校验**

```yaml
      - name: 验证构建产物时间戳
        run: |
          echo "server 二进制信息:"
          ls -la backend/target/release/server
          file backend/target/release/server
          echo "bingxi 二进制信息:"
          ls -la backend/target/release/bingxi
          file backend/target/release/bingxi
```

- [ ] **Step 3: 提交更改**

```bash
git add .github/workflows/ci-cd.yml
git commit -m "ci: 清理编译缓存确保构建产物为最新版本"
```

---

## 任务 6: 部署 v2026.519.1900

**Files:**
- Modify: `backend/Cargo.toml` (版本号)
- Modify: `VERSION`

---

- [ ] **Step 1: 更新版本号**

```bash
# 更新 VERSION 文件
echo "2026.519.1900" > VERSION

# 更新 Cargo.toml
sed -i 's/^version = ".*"/version = "2026.519.1900"/' backend/Cargo.toml
```

- [ ] **Step 2: 本地构建验证**

Run:
```bash
cd backend
cargo build --release
```
Expected: 构建成功

- [ ] **Step 3: 运行测试**

Run:
```bash
cd backend
cargo test
```
Expected: 所有测试通过

- [ ] **Step 4: 构建前端**

Run:
```bash
cd frontend
npm install
npm run build
```
Expected: 构建成功

- [ ] **Step 5: 提交版本更新**

```bash
git add VERSION backend/Cargo.toml
git commit -m "chore(release): 更新版本号至 v2026.519.1900"
git tag v2026.519.1900
git push origin main --tags
```

- [ ] **Step 6: 等待 CI 构建完成**

检查 GitHub Actions 构建状态，等待发布包生成。

- [ ] **Step 7: 下载并部署到服务器**

```bash
# 在服务器上执行
sudo bingxi update
# 或手动部署
wget https://github.com/57231307/1/releases/download/v2026.519.1900/bingxi-erp-2026.519.1900.zip
unzip bingxi-erp-2026.519.1900.zip -d /tmp/bingxi-new/
sudo systemctl stop bingxi-backend
sudo cp /tmp/bingxi-new/backend/server /opt/bingxi/backend/
sudo cp /tmp/bingxi-new/backend/bingxi /opt/bingxi/bin/
sudo cp -r /tmp/bingxi-new/frontend/dist/* /opt/bingxi/frontend/dist/
sudo systemctl start bingxi-backend
sudo systemctl status bingxi-backend
```

- [ ] **Step 8: 验证部署**

```bash
# 健康检查
curl http://localhost:8082/api/v1/erp/health

# 版本检查
/opt/bingxi/bin/bingxi --version

# 日志检查
sudo journalctl -u bingxi-backend -n 50 --no-pager
```

---

## 验证清单

所有任务完成后，验证以下项目：

- [ ] `purchase_orders` 表/视图可正常访问
- [ ] gRPC 服务启动无 transport error
- [ ] Redis 限流正常工作（如无 Redis 则回退到内存限流）
- [ ] BPM 条件分支可正确评估
- [ ] CI 构建产物为最新版本
- [ ] 部署版本号为 v2026.519.1900
- [ ] 健康检查返回 healthy
- [ ] 所有 API 端点正常响应
