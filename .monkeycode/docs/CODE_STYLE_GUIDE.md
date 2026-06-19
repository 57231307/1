# 代码规范指南

本文档为项目代码规范指南，指导前后端开发、Git 提交及代码审查。

---

## 一、前端规范

### 1. TypeScript 类型规范

**禁止使用 `any`，必须使用具体类型。**

```typescript
// ❌ 错误示例
function getUserData(id: any): any {
  const data: any = fetchUser(id);
  return data;
}

// ✅ 正确示例
interface UserData {
  id: number;
  name: string;
  email: string;
}

function getUserData(id: number): Promise<UserData> {
  return fetchUser(id);
}
```

**使用 `unknown` 替代 `any` 处理不确定类型：**

```typescript
// ❌ 错误示例
function parseResponse(res: any) {
  return res.data;
}

// ✅ 正确示例
function parseResponse(res: unknown) {
  if (isApiResponse(res)) {
    return res.data;
  }
  throw new Error('无效的响应格式');
}
```

### 2. API 响应类型规范

**使用统一的响应类型定义：**

```typescript
// ✅ 统一响应类型
interface ApiResponse<T> {
  code: number;
  message: string;
  data: T;
}

interface PaginatedResponse<T> {
  code: number;
  message: string;
  data: T[];
  total: number;
  page: number;
  page_size: number;
}

// ✅ 使用示例
interface User {
  id: number;
  name: string;
}

async function getUsers(page: number): Promise<PaginatedResponse<User>> {
  const response = await fetch(`/api/users?page=${page}`);
  return response.json();
}
```

### 3. 日志规范

**禁止使用 `console.*`，统一使用项目 logger。**

```typescript
// ❌ 错误示例
console.log('用户登录成功:', userId);
console.error('请求失败:', error);
console.warn('参数缺失');

// ✅ 正确示例
import { logger } from '@/utils/logger';

logger.info('用户登录成功', { userId });
logger.error('请求失败', { error, url });
logger.warn('参数缺失', { paramName });
logger.debug('调试信息', { data });
```

**日志级别说明：**
- `error`: 系统错误、异常
- `warn`: 警告信息、潜在问题
- `info`: 重要业务事件
- `debug`: 调试信息（生产环境自动过滤）

### 4. 组件命名规范

```typescript
// ✅ 组件文件名使用 PascalCase
// UserProfile.tsx
// UserList.tsx
// LoginForm.tsx

// ✅ 组件名使用 PascalCase
const UserProfile: React.FC<Props> = (props) => {
  return <div>{props.name}</div>;
};

// ✅ 工具函数使用 camelCase
const formatDate = (date: Date): string => {
  return date.toISOString();
};

// ✅ 常量使用 UPPER_SNAKE_CASE
const MAX_RETRY_COUNT = 3;
const API_BASE_URL = '/api/v1';

// ✅ 类型/接口使用 PascalCase
interface UserFormData {
  name: string;
  age: number;
}
```

### 5. 文件组织规范

```
src/
├── components/          # 公共组件
│   ├── Button/
│   │   ├── index.tsx
│   │   ├── Button.tsx
│   │   └── Button.test.tsx
│   └── Modal/
├── pages/               # 页面组件
│   ├── Home/
│   └── User/
├── services/            # API 服务
│   ├── user.ts
│   └── order.ts
├── utils/               # 工具函数
│   ├── logger.ts
│   └── validator.ts
├── types/               # 类型定义
│   ├── api.ts
│   └── user.ts
└── hooks/               # 自定义 Hooks
    ├── useAuth.ts
    └── usePagination.ts
```

---

## 二、后端规范

### 1. Rust 代码规范

**禁止使用 `println!`，统一使用 `tracing` 日志系统。**

```rust
// ❌ 错误示例
println!("用户登录成功: {}", user_id);
eprintln!("数据库错误: {:?}", err);

// ✅ 正确示例
use tracing::{info, error, warn, debug};

info!(user_id = %user_id, "用户登录成功");
error!(?err, "数据库查询失败");
warn!(field = "email", "参数缺失");
debug!(?data, "调试信息");
```

**日志级别说明：**
- `error!`: 系统错误、异常
- `warn!`: 警告信息、潜在问题
- `info!`: 重要业务事件
- `debug!`: 调试信息
- `trace!`: 详细跟踪信息

### 2. 错误处理规范

**禁止使用 `unwrap()` / `expect()`，必须使用 `Result` 或适当的错误处理。**

```rust
// ❌ 错误示例
fn get_user(id: i64) -> User {
    let user = db.find_user(id).unwrap();  // 可能 panic
    user
}

// ✅ 正确示例
fn get_user(id: i64) -> Result<User, AppError> {
    let user = db.find_user(id)
        .map_err(|e| AppError::Database(e.to_string()))?;
    Ok(user)
}

// ✅ 在测试中可以使用 unwrap
#[cfg(test)]
mod tests {
    #[test]
    fn test_user_creation() {
        let user = create_user("test").unwrap();
        assert_eq!(user.name, "test");
    }
}
```

**错误处理最佳实践：**

```rust
// ✅ 使用 ? 运算符传播错误
async fn process_order(order_id: i64) -> Result<Order, AppError> {
    let order = order_repo.find_by_id(order_id).await?;
    let payment = payment_service.process(&order).await?;
    Ok(order.with_payment(payment))
}

// ✅ 提供有意义的错误信息
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("用户不存在: {0}")]
    UserNotFound(i64),
    
    #[error("数据库错误: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("参数验证失败: {0}")]
    ValidationError(String),
}
```

### 3. 文件命名规范

**避免使用 Rust 关键字作为文件名。**

```bash
# ❌ 错误示例（与 Rust 关键字冲突）
src/routes/async.rs
src/models/match.rs
src/handlers/impl.rs

# ✅ 正确示例
src/routes/async_routes.rs
src/routes/async_handler.rs
src/models/match_record.rs
src/handlers/impl_handler.rs
```

**文件命名使用 snake_case：**

```bash
# ✅ 正确
user_service.rs
order_repository.rs
auth_middleware.rs

# ❌ 错误
UserService.rs
order-repository.rs
authMiddleware.rs
```

### 4. 模块组织规范

```
src/
├── main.rs              # 程序入口
├── lib.rs               # 库入口
├── config/              # 配置模块
│   ├── mod.rs
│   └── database.rs
├── handlers/            # HTTP 处理器
│   ├── mod.rs
│   ├── user.rs
│   └── order.rs
├── services/            # 业务逻辑
│   ├── mod.rs
│   ├── user_service.rs
│   └── order_service.rs
├── repositories/        # 数据访问层
│   ├── mod.rs
│   ├── user_repo.rs
│   └── order_repo.rs
├── models/              # 数据模型
│   ├── mod.rs
│   ├── user.rs
│   └── order.rs
├── middleware/          # 中间件
│   ├── mod.rs
│   └── auth.rs
└── utils/               # 工具函数
    ├── mod.rs
    └── logger.rs
```

---

## 三、Git 提交规范

### 1. 提交信息格式

```
<type>(<scope>): <subject>

[可选的正文]

[可选的脚注]
```

### 2. Type 类型说明

| Type | 说明 |
|------|------|
| `feat` | 新功能 |
| `fix` | 修复 bug |
| `refactor` | 重构（不改变功能） |
| `style` | 代码格式调整（不影响逻辑） |
| `test` | 添加或修改测试 |
| `docs` | 文档变更 |
| `chore` | 构建、依赖或其他辅助性变动 |
| `perf` | 性能优化 |

### 3. 示例

```bash
# 新功能
git commit -m "feat(user): 添加用户注册功能"

# 修复 bug
git commit -m "fix(order): 修复订单金额计算错误"

# 重构
git commit -m "refactor(auth): 重构登录验证逻辑"

# 文档
git commit -m "docs(api): 更新 API 文档"

# 性能优化
git commit -m "perf(query): 优化用户列表查询性能"

# 测试
git commit -m "test(user): 添加用户服务单元测试"

# 构建/依赖
git commit -m "chore(deps): 升级 tokio 到 1.0 版本"
```

---

## 四、代码审查清单

### 1. 前端审查项

- [ ] 是否使用了具体类型（禁止 `any`）
- [ ] API 响应是否使用统一的 `ApiResponse` / `PaginatedResponse` 类型
- [ ] 是否使用 `logger` 替代 `console.*`
- [ ] 组件命名是否符合 PascalCase 规范
- [ ] 文件组织是否符合规范
- [ ] 是否有适当的错误处理
- [ ] 是否有必要的单元测试
- [ ] 是否有敏感信息硬编码

### 2. 后端审查项

- [ ] 是否使用 `tracing` 替代 `println!`
- [ ] 是否避免使用 `unwrap()` / `expect()`
- [ ] 错误处理是否使用 `Result` 和 `?` 运算符
- [ ] 文件名是否避免 Rust 关键字
- [ ] 模块组织是否符合规范
- [ ] 是否有适当的日志记录
- [ ] 是否有必要的单元测试
- [ ] 是否有 SQL 注入风险
- [ ] 租户隔离是否正确（使用 `extract_tenant_id`）
- [ ] 是否有敏感信息硬编码

---

## 五、持续改进

### 1. 定期代码审查

- 每周进行一次代码审查会议
- 审查重点：代码质量、安全性、性能、测试覆盖
- 记录审查结果和改进建议

### 2. 自动化检查

**前端自动化：**
- ESLint: 代码风格检查
- Prettier: 代码格式化
- TypeScript: 类型检查
- Jest: 单元测试

**后端自动化：**
- `cargo fmt`: 代码格式化
- `cargo clippy`: 代码质量检查
- `cargo test`: 单元测试
- `cargo audit`: 安全漏洞检查

### 3. 技术债务管理

- 使用 `TODO` / `FIXME` / `HACK` 标记技术债务
- 定期评估技术债务优先级
- 每个迭代预留 20% 时间处理技术债务

```rust
// TODO: 优化查询性能，当前存在 N+1 问题
// FIXME: 并发场景下可能存在竞态条件
// HACK: 临时解决方案，等待上游 API 修复
```

---

## 附录：常用命令

```bash
# 前端
npm run lint          # 代码检查
npm run format        # 代码格式化
npm run test          # 运行测试
npm run typecheck     # 类型检查

# 后端
cargo fmt             # 代码格式化
cargo clippy          # 代码质量检查
cargo test            # 运行测试
cargo audit           # 安全检查
cargo build           # 构建项目
```

---

**最后更新**: 2026-06-13  
**维护者**: 开发团队
