# 秉羲管理系统 - 测试指南

本文档详细说明了秉羲管理系统 Rust 版本的测试策略、测试框架和测试用例。

## 测试策略

### 测试层次

1. **单元测试 (Unit Tests)**
   - 测试单个函数或方法
   - 不依赖外部服务
   - 使用内存数据库或 Mock

2. **集成测试 (Integration Tests)**
   - 测试模块间交互
   - 使用真实数据库
   - 测试 API 端点

3. **端到端测试 (E2E Tests)**
   - 测试完整业务流程
   - 前端 + 后端联合测试
   - 模拟真实用户操作

## 测试框架

### 后端测试框架

- **测试运行器**: cargo test (内置)
- **异步测试**: tokio::test
- **Mock 框架**: mockall
- **断言**: 标准库 assert!

### 前端测试框架

- **测试运行器**: wasm-pack test
- **Web 测试**: wasm-bindgen-test
- **组件测试**: Yew 测试工具

## 单元测试

### 认证服务测试

**文件**: `backend/src/services/tests/auth_service_test.rs`

**测试用例**:

```rust
#[tokio::test]
async fn test_auth_service_creation() {
    let db = setup_test_db().await;
    let secret = "test_secret".to_string();
    
    let auth_service = AuthService::new(Arc::new(db), secret);
    
    assert!(auth_service.get_secret().len() > 0);
}

#[tokio::test]
async fn test_password_hashing() {
    let password = "test_password_123";
    
    let hash_result = AuthService::hash_password(password);
    
    assert!(hash_result.is_ok());
    let hash = hash_result.unwrap();
    assert_ne!(password, hash);
    assert!(hash.len() > 50);
}

#[tokio::test]
async fn test_password_verification() {
    let password = "test_password_123";
    let hash = AuthService::hash_password(password).unwrap();
    
    let db = setup_test_db().await;
    let secret = "test_secret".to_string();
    let auth_service = AuthService::new(Arc::new(db), secret);
    
    let result = auth_service.verify_password(password, &hash);
    assert!(result);
    
    let wrong_result = auth_service.verify_password("wrong_password", &hash);
    assert!(!wrong_result);
}
```

### 用户服务测试

**文件**: `backend/src/services/tests/user_service_test.rs`

**测试用例**:

```rust
#[tokio::test]
async fn test_create_user() {
    let db = setup_test_db().await;
    let user_service = UserService::new(Arc::new(db.clone()));
    
    let password_hash = AuthService::hash_password("password123").unwrap();
    
    let result = user_service.create_user(
        "test_user".to_string(),
        password_hash,
        Some("test@example.com".to_string()),
        Some("13800138000".to_string()),
        Some(1),
        Some(1),
    ).await;
    
    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user.username, "test_user");
}

#[tokio::test]
async fn test_find_user_by_username() {
    let db = setup_test_db().await;
    let user_service = UserService::new(Arc::new(db.clone()));
    
    let password_hash = AuthService::hash_password("password123").unwrap();
    
    user_service.create_user(
        "find_test_user".to_string(),
        password_hash,
        None,
        None,
        None,
        None,
    ).await.unwrap();
    
    let result = user_service.find_by_username("find_test_user").await;
    
    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user.username, "find_test_user");
}
```

## 运行测试

### 后端测试

```bash
# 运行所有测试
cd backend
cargo test

# 运行特定模块测试
cargo test --package bingxi-backend --lib services::tests::auth_service_test

# 运行单个测试
cargo test --package bingxi-backend --lib services::tests::auth_service_test::tests::test_password_hashing

# 显示输出
cargo test -- --nocapture

# 生成测试覆盖率报告 (需要 cargo-tarpaulin)
cargo tarpaulin --out Html
```

### 前端测试

```bash
# 安装测试工具
cargo install wasm-pack

# 运行测试
cd frontend
wasm-pack test --headless --firefox

# 运行 Node.js 测试
wasm-pack test --node
```

## 测试配置

### 环境变量

创建 `.env.test` 文件:

```bash
# 测试数据库配置
TEST_DATABASE_URL=sqlite::memory:
DATABASE_URL=postgresql://test:test@localhost:5432/bingxi_test?Version=18.0

# JWT 配置
JWT_SECRET=test_secret_key_for_testing_only
JWT_EXPIRATION_HOURS=24

# 服务器配置
SERVER_HOST=127.0.0.1
SERVER_PORT=8001
```

### 测试数据库

使用 SQLite 内存数据库进行快速测试:

```rust
async fn setup_test_db() -> DatabaseConnection {
    let db_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "sqlite::memory:".to_string());
    Database::connect(&db_url).await.unwrap()
}
```

## 测试最佳实践

### 1. 测试命名规范

```rust
#[test]
fn test_<module>_<function>_<scenario>() {
    // ...
}

// 示例
#[test]
fn test_auth_service_password_verification_success() {
    // ...
}

#[test]
fn test_user_service_create_user_with_valid_data() {
    // ...
}
```

### 2. AAA 模式 (Arrange-Act-Assert)

```rust
#[tokio::test]
async fn test_create_user() {
    // Arrange (准备)
    let db = setup_test_db().await;
    let user_service = UserService::new(Arc::new(db));
    let password_hash = AuthService::hash_password("password123").unwrap();
    
    // Act (执行)
    let result = user_service.create_user(
        "test_user".to_string(),
        password_hash,
        None,
        None,
        None,
        None,
    ).await;
    
    // Assert (断言)
    assert!(result.is_ok());
    let user = result.unwrap();
    assert_eq!(user.username, "test_user");
}
```

### 3. 测试隔离

- 每个测试使用独立的数据库连接
- 使用事务回滚保持数据隔离
- 避免测试间依赖

```rust
#[tokio::test]
async fn test_isolated() {
    let db = setup_test_db().await;
    // 每个测试都有新的数据库实例
    // 测试互不影响
}
```

### 4. 测试覆盖率目标

- **行覆盖率**: > 80%
- **分支覆盖率**: > 70%
- **关键模块**: > 90%

## 集成测试

### API 集成测试

**文件**: `backend/tests/api_test.rs`

```rust
use axum::Router;
use sea_orm::Database;
use std::sync::Arc;

async fn setup_app() -> Router {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    create_router(Arc::new(db))
}

#[tokio::test]
async fn test_login_success() {
    let app = setup_app().await;
    
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "username": "admin",
                        "password": "admin123"
                    }).to_string()
                ))
                .unwrap()
        )
        .await
        .unwrap();
    
    assert_eq!(response.status(), StatusCode::OK);
}
```

## 前端测试

### 组件测试

```rust
#[cfg(test)]
mod tests {
    use wasm_bindgen_test::*;
    use yew::prelude::*;
    use crate::pages::login::LoginPage;
    
    wasm_bindgen_test_configure!(run_in_browser);
    
    #[wasm_bindgen_test]
    fn test_login_page_renders() {
        let login_page = html! { <LoginPage /> };
        assert!(login_page.is_some());
    }
}
```

## 持续集成

### GitHub Actions 配置

**文件**: `.github/workflows/test.yml`

```yaml
name: Test

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v2
    
    - name: Install Rust
      uses: actions-rs/toolchain@v1
      with:
        toolchain: stable
    
    - name: Install dependencies
      run: |
        sudo apt-get update
        sudo apt-get install -y pkg-config libssl-dev
    
    - name: Run backend tests
      working-directory: ./backend
      run: cargo test --verbose
    
    - name: Run frontend tests
      working-directory: ./frontend
      run: |
        cargo install wasm-pack
        wasm-pack test --headless --firefox
```

## 测试报告

### 生成测试报告

```bash
# 安装 cargo-tarpaulin
cargo install cargo-tarpaulin

# 生成 HTML 报告
cd backend
cargo tarpaulin --out Html

# 查看报告
# 打开 target/tarpaulin-report.html
```

### 测试覆盖率要求

| 模块 | 最低覆盖率 | 目标覆盖率 |
|------|-----------|-----------|
| 服务层 | 80% | 90% |
| 处理器层 | 70% | 85% |
| 模型层 | 90% | 95% |
| 工具函数 | 80% | 90% |
| 总体 | 80% | 90% |

## 常见问题

### 1. 异步测试失败

**问题**: 异步测试无法运行

**解决**: 使用 `#[tokio::test]` 宏

```rust
#[tokio::test]
async fn test_async_function() {
    // ...
}
```

### 2. 数据库连接失败

**问题**: 测试数据库连接失败

**解决**: 设置正确的环境变量

```bash
export TEST_DATABASE_URL=sqlite::memory:
```

### 3. Mock 对象使用

**问题**: 如何 Mock 外部依赖

**解决**: 使用 mockall 库

```rust
use mockall::automock;

#[automock]
#[async_trait]
pub trait ExternalService {
    async fn call(&self) -> Result<(), String>;
}
```

## 测试检查清单

### 提交前检查

- [ ] 所有单元测试通过
- [ ] 所有集成测试通过
- [ ] 测试覆盖率达到要求
- [ ] 没有失败的测试
- [ ] 没有忽略的测试

### 发布前检查

- [ ] 完整的功能测试
- [ ] 性能测试
- [ ] 安全测试
- [ ] 兼容性测试
- [ ] 压力测试

## 总结

秉羲管理系统已经建立了完善的测试体系:

- ✅ 单元测试覆盖核心服务
- ✅ 集成测试验证 API 功能
- ✅ 清晰的测试文档
- ✅ 自动化测试流程

下一步将继续完善:
- [ ] 增加更多测试用例
- [ ] 提高测试覆盖率
- [ ] 实施持续集成
- [ ] 添加性能测试

---

**报告人**: AI 助手  
**完成时间**: 2026-03-15  
**测试框架**: cargo test + tokio::test + mockall
