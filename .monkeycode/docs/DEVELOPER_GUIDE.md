# 冰溪 ERP 开发者指南

## 项目目的

冰溪 ERP 是一个面向面料纺织行业的现代化企业资源计划系统，在更大系统中担任企业数字化转型的核心平台。它集成了采购、销售、库存、生产、财务、CRM 等核心业务模块，并引入 AI 智能分析能力。

**核心职责**:
- 管理企业核心业务流程（采购、销售、库存、生产）
- 提供财务核算和报表分析能力
- 支持客户关系管理和销售预测
- 实现多租户 SaaS 部署模式
- 提供 AI 智能分析和决策支持

**相关系统**:
- 面料行业上下游系统 - 数据交换和集成
- 第三方支付网关 - 支付处理
- 邮件/短信服务 - 通知推送
- AI 分析平台 - 智能预测和优化

## 环境搭建

### 前置条件

- **Rust**: 1.75+ (后端开发)
- **Node.js**: 18+ (前端开发)
- **PostgreSQL**: 15+ 或 MySQL 8.0+ (数据库)
- **Redis**: 7.0+ (缓存)
- **Docker**: 可选，用于容器化部署
- **Git**: 版本控制

### 安装

```bash
# 克隆仓库
git clone https://github.com/57231307/1.git
cd 1

# 后端依赖安装
cd backend
cargo build

# 前端依赖安装
cd ../frontend
npm install
```

### 环境变量

#### 后端环境变量 (backend/.env)

```bash
# 数据库配置
DATABASE_URL=postgresql://username:password@localhost:5432/bingxi_erp
# 或 MySQL
# DATABASE_URL=mysql://username:password@localhost:3306/bingxi_erp

# Redis 配置
REDIS_URL=redis://localhost:6379

# JWT 配置
JWT_SECRET=your-secret-key-here
JWT_EXPIRATION=3600
JWT_REFRESH_EXPIRATION=86400

# 服务器配置
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# 日志配置
RUST_LOG=info
RUST_BACKTRACE=1

# 邮件配置 (可选)
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USERNAME=your-email@example.com
SMTP_PASSWORD=your-email-password

# AI 服务配置 (可选)
AI_SERVICE_URL=http://localhost:5000
AI_SERVICE_API_KEY=your-ai-api-key
```

#### 前端环境变量 (frontend/.env.development)

```bash
# API 基础路径
VITE_API_BASE_URL=/api/v1/erp

# 是否使用 Mock 数据
VITE_USE_MOCK=false

# 调试模式
VITE_DEBUG=true

# 应用标题
VITE_APP_TITLE=冰溪 ERP
```

### 运行

#### 使用 Docker Compose (推荐)

```bash
# 启动所有服务
docker-compose up -d

# 查看日志
docker-compose logs -f

# 停止服务
docker-compose down
```

#### 手动启动

```bash
# 1. 启动数据库和 Redis
# 确保 PostgreSQL 和 Redis 已安装并运行

# 2. 数据库迁移
cd backend
cargo run --bin migrate

# 3. 启动后端服务
cargo run

# 4. 启动前端服务 (新终端)
cd frontend
npm run dev
```

#### 访问地址

- **前端界面**: http://localhost:5173
- **后端 API**: http://localhost:8080
- **Swagger UI**: http://localhost:8080/swagger-ui
- **Prometheus 指标**: http://localhost:9090

### 初始配置

首次访问系统时，会进入初始化向导：

1. **数据库配置**: 输入数据库连接信息
2. **管理员账户**: 创建系统管理员账户
3. **基础数据**: 初始化基础数据（可选）

## 开发工作流

### 代码质量工具

| 工具 | 命令 | 目的 |
|------|------|------|
| Rust Analyzer | `cargo check` | 代码检查和补全 |
| Clippy | `cargo clippy` | 代码质量检查 |
| Rustfmt | `cargo fmt` | 代码格式化 |
| TypeScript | `npm run typecheck` | 类型检查 |
| ESLint | `npm run lint` | 代码检查 |
| Prettier | `npm run format` | 代码格式化 |
| Tests | `cargo test` / `npm run test` | 单元/集成测试 |

### 提交前检查

这些会在提交时自动运行（通过 Git hooks）：

1. **后端检查**:
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   cargo test
   ```

2. **前端检查**:
   ```bash
   npm run lint
   npm run typecheck
   npm run test:run
   ```

手动运行所有检查：
```bash
# 后端
cd backend && cargo fmt && cargo clippy && cargo test

# 前端
cd frontend && npm run lint && npm run typecheck && npm run test:run
```

### 分支策略

- `main` - 生产就绪代码
- `develop` - 开发分支
- `feature/*` - 新功能
- `fix/*` - Bug 修复
- `hotfix/*` - 紧急修复
- `release/*` - 发布准备

### Pull Request 流程

1. 从 `develop` 创建功能分支
2. 编写代码和测试
3. 运行代码质量检查
4. 创建 PR 并填写描述
5. 处理审查反馈
6. Squash 合并到 `develop`

### 提交信息规范

遵循 [Conventional Commits](https://www.conventionalcommits.org/) 规范：

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

**类型**:
- `feat`: 新功能
- `fix`: Bug 修复
- `docs`: 文档更新
- `style`: 代码格式调整
- `refactor`: 代码重构
- `test`: 测试相关
- `chore`: 构建/工具相关

**示例**:
```
feat(sales): 添加销售订单审批流程

- 实现订单提交审批功能
- 添加审批状态管理
- 集成 BPM 工作流引擎

Closes #123
```

## 常见任务

### 添加新 API 端点

**需修改的文件**:
1. `backend/src/models/[entity].rs` - 定义数据模型
2. `backend/src/services/[entity]_service.rs` - 实现业务逻辑
3. `backend/src/handlers/[entity]_handler.rs` - 创建 HTTP 处理器
4. `backend/src/routes/mod.rs` - 注册路由
5. `frontend/src/api/[entity].ts` - 创建前端 API 调用
6. `frontend/src/views/[entity]/index.vue` - 创建前端页面

**步骤**:

1. **定义数据模型** (SeaORM Entity):
```rust
// backend/src/models/[entity].rs
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "entity_name")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub name: String,
    pub status: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
```

2. **实现服务层**:
```rust
// backend/src/services/[entity]_service.rs
use crate::models::entity_name::{self, Entity, Model};
use sea_orm::{DatabaseConnection, EntityTrait, Set};

pub struct EntityService;

impl EntityService {
    pub async fn find_by_id(db: &DatabaseConnection, id: Uuid) -> Result<Option<Model>, sea_orm::DbErr> {
        Entity::find_by_id(id).one(db).await
    }

    pub async fn create(db: &DatabaseConnection, data: CreateEntityRequest) -> Result<Model, sea_orm::DbErr> {
        let active_model = entity_name::ActiveModel {
            name: Set(data.name),
            status: Set("active".to_string()),
            ..Default::default()
        };
        
        Entity::insert(active_model).exec_with_returning(db).await
    }
}
```

3. **创建 HTTP 处理器**:
```rust
// backend/src/handlers/[entity]_handler.rs
use axum::{extract::Path, Json};
use crate::services::entity_service::EntityService;
use crate::utils::response::ApiResponse;

pub async fn get_entity(
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Model>>, StatusCode> {
    let entity = EntityService::find_by_id(&db, id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    match entity {
        Some(entity) => Ok(Json(ApiResponse::success(entity))),
        None => Err(StatusCode::NOT_FOUND),
    }
}
```

4. **注册路由**:
```rust
// backend/src/routes/mod.rs
use crate::handlers::entity_handler;

pub fn entity_routes() -> Router<AppState> {
    Router::new()
        .route("/api/v1/erp/entities", get(list_entities).post(create_entity))
        .route("/api/v1/erp/entities/{id}", get(get_entity).put(update_entity).delete(delete_entity))
}
```

5. **创建前端 API 调用**:
```typescript
// frontend/src/api/[entity].ts
import request from './request'

export interface Entity {
  id: string
  name: string
  status: string
  created_at: string
  updated_at: string
}

export interface CreateEntityRequest {
  name: string
  status?: string
}

export const entityApi = {
  getList(params?: { page?: number; pageSize?: number }) {
    return request.get<{ items: Entity[]; total: number }>('/entities', { params })
  },

  getById(id: string) {
    return request.get<Entity>(`/entities/${id}`)
  },

  create(data: CreateEntityRequest) {
    return request.post<Entity>('/entities', data)
  },

  update(id: string, data: Partial<CreateEntityRequest>) {
    return request.put<Entity>(`/entities/${id}`, data)
  },

  delete(id: string) {
    return request.delete(`/entities/${id}`)
  },
}
```

6. **创建前端页面**:
```vue
<!-- frontend/src/views/[entity]/index.vue -->
<template>
  <div class="entity-page">
    <el-card>
      <template #header>
        <div class="card-header">
          <span>实体管理</span>
          <el-button type="primary" @click="handleCreate">新建</el-button>
        </div>
      </template>
      
      <el-table :data="tableData" v-loading="loading">
        <el-table-column prop="name" label="名称" />
        <el-table-column prop="status" label="状态" />
        <el-table-column prop="created_at" label="创建时间" />
        <el-table-column label="操作">
          <template #default="{ row }">
            <el-button size="small" @click="handleEdit(row)">编辑</el-button>
            <el-button size="small" type="danger" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { entityApi, Entity } from '@/api/entity'

const tableData = ref<Entity[]>([])
const loading = ref(false)

const fetchData = async () => {
  loading.value = true
  try {
    const { data } = await entityApi.getList()
    tableData.value = data.items
  } finally {
    loading.value = false
  }
}

onMounted(() => {
  fetchData()
})
</script>
```

### 添加数据库迁移

**需创建的文件**:
1. `backend/migrations/[timestamp]_[name]/migration.sql`

**步骤**:

1. **生成迁移文件**:
```bash
cd backend
# 使用 SeaORM CLI 生成迁移
sea-orm-cli migrate generate add_entity_table
```

2. **编写迁移 SQL**:
```sql
-- backend/migrations/[timestamp]_add_entity_table/migration.sql
CREATE TABLE entity_name (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_entity_name ON entity_name(name);
CREATE INDEX idx_entity_status ON entity_name(status);
```

3. **运行迁移**:
```bash
cargo run --bin migrate
```

4. **测试回滚**:
```bash
# 如果需要回滚
sea-orm-cli migrate rollback
```

### 添加新前端页面

**需修改的文件**:
1. `frontend/src/views/[module]/index.vue` - 页面组件
2. `frontend/src/router/index.ts` - 路由配置
3. `frontend/src/api/[module].ts` - API 调用（如需要）

**步骤**:

1. **创建页面组件**:
```vue
<!-- frontend/src/views/[module]/index.vue -->
<template>
  <div class="module-page">
    <el-card>
      <template #header>
        <span>模块标题</span>
      </template>
      
      <!-- 页面内容 -->
    </el-card>
  </div>
</template>

<script setup lang="ts">
// 页面逻辑
</script>

<style scoped>
.module-page {
  padding: 20px;
}
</style>
```

2. **添加路由配置**:
```typescript
// frontend/src/router/index.ts
{
  path: '/module',
  name: 'Module',
  component: () => import('@/views/module/index.vue'),
  meta: {
    title: '模块名称',
    icon: 'Document',
    requiresAuth: true,
  },
}
```

3. **添加菜单项** (如果需要):
在 `MainLayout.vue` 的菜单配置中添加：
```typescript
{
  path: '/module',
  title: '模块名称',
  icon: 'Document',
}
```

### 修复 Bug

**流程**:

1. **复现问题**:
```bash
# 运行相关测试
cargo test [test_name]
npm run test -- [test_name]
```

2. **定位根因**:
```bash
# 查看错误日志
RUST_LOG=debug cargo run

# 使用调试器
cargo test -- --nocapture
```

3. **编写修复**:
- 最小化改动
- 保持代码风格一致
- 添加必要的注释

4. **验证修复**:
```bash
# 运行测试
cargo test
npm run test

# 手动测试
```

5. **检查其他地方**:
```bash
# 搜索类似问题
grep -r "similar_pattern" backend/src/
grep -r "similar_pattern" frontend/src/
```

### 添加新环境变量

**需修改的文件**:
1. `backend/.env.example` - 添加示例值
2. `backend/src/config/env.ts` - 添加验证
3. `.monkeycode/docs/DEVELOPER_GUIDE.md` - 文档化变量

**步骤**:

1. **在 `.env.example` 中添加占位符**:
```bash
# 新功能配置
NEW_FEATURE_ENABLED=true
NEW_FEATURE_API_KEY=your-api-key-here
```

2. **在配置中添加验证**:
```rust
// backend/src/config/mod.rs
#[derive(Debug, Deserialize)]
pub struct AppConfig {
    pub new_feature_enabled: bool,
    pub new_feature_api_key: Option<String>,
}
```

3. **在本指南中文档化**:
更新环境变量表格。

## 编码规范

### Rust 编码规范

#### 文件组织
- 每个模块一个文件
- 文件以其内容命名（小写，下划线分隔）
- 相关文件放在同一目录

#### 命名约定

| 类型 | 约定 | 示例 |
|------|------|------|
| 文件 | snake_case | `user_service.rs` |
| 模块 | snake_case | `user_service` |
| 结构体 | PascalCase | `UserService` |
| 枚举 | PascalCase | `OrderStatus` |
| 函数 | snake_case | `get_user_by_id` |
| 变量 | snake_case | `user_count` |
| 常量 | SCREAMING_SNAKE | `MAX_RETRY_COUNT` |

#### 错误处理

```rust
// 推荐：使用自定义错误类型
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("用户不存在")]
    UserNotFound,
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
    #[error("验证错误: {0}")]
    Validation(String),
}

// 使用 Result 返回错误
pub async fn get_user(db: &DatabaseConnection, id: Uuid) -> Result<User, AppError> {
    let user = User::find_by_id(id)
        .one(db)
        .await?
        .ok_or(AppError::UserNotFound)?;
    Ok(user)
}
```

#### 日志

```rust
use tracing::{info, warn, error, debug};

// 包含上下文
info!(user_id = %user.id, "用户登录成功");

// 使用适当级别
debug!(query = %sql, "执行数据库查询");
warn!(retry_count = count, "请求重试");
error!(error = %err, "操作失败");
```

### TypeScript/Vue 编码规范

#### 文件组织
- 每个组件一个文件
- 文件以其默认导出命名（PascalCase）
- 相关文件放在同一目录

#### 命名约定

| 类型 | 约定 | 示例 |
|------|------|------|
| 文件 | kebab-case | `user-service.ts` |
| 组件 | PascalCase | `UserProfile.vue` |
| 类 | PascalCase | `UserService` |
| 函数 | camelCase | `getUserById` |
| 变量 | camelCase | `userCount` |
| 常量 | SCREAMING_SNAKE | `MAX_RETRY_COUNT` |

#### Vue 组件规范

```vue
<template>
  <!-- 模板内容 -->
</template>

<script setup lang="ts">
// 使用 Composition API
import { ref, computed, onMounted } from 'vue'

// 定义 props
const props = defineProps<{
  userId: string
  title?: string
}>()

// 定义 emits
const emit = defineEmits<{
  (e: 'update', value: string): void
  (e: 'delete', id: string): void
}>()

// 响应式数据
const loading = ref(false)
const user = ref<User | null>(null)

// 计算属性
const fullName = computed(() => {
  if (!user.value) return ''
  return `${user.value.firstName} ${user.value.lastName}`
})

// 方法
const fetchUser = async () => {
  loading.value = true
  try {
    user.value = await userApi.getById(props.userId)
  } finally {
    loading.value = false
  }
}

// 生命周期
onMounted(() => {
  fetchUser()
})
</script>

<style scoped>
/* 组件样式 */
</style>
```

#### 错误处理

```typescript
// 推荐：使用统一的错误处理
try {
  const result = await userApi.create(userData)
  ElMessage.success('创建成功')
  emit('update', result)
} catch (error) {
  if (error instanceof ApiError) {
    ElMessage.error(error.message)
  } else {
    ElMessage.error('操作失败，请稍后重试')
  }
}
```

### 测试规范

#### Rust 测试

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Database, MockDatabase, MockExecResult};

    #[tokio::test]
    async fn test_find_user_by_id() {
        // 准备
        let db = MockDatabase::new()
            .append_query_results(vec![vec![user_model()]])
            .into_connection();
        
        // 执行
        let result = UserService::find_by_id(&db, Uuid::new_v4()).await;
        
        // 断言
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }

    #[tokio::test]
    async fn test_create_user_validation() {
        // 测试验证逻辑
        let invalid_data = CreateUserRequest {
            email: "invalid-email".to_string(),
            ..Default::default()
        };
        
        let result = UserService::create(&db, invalid_data).await;
        assert!(result.is_err());
    }
}
```

#### Vue 测试

```typescript
// frontend/tests/unit/UserProfile.test.ts
import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import UserProfile from '@/components/UserProfile.vue'

describe('UserProfile', () => {
  it('renders user name correctly', () => {
    const wrapper = mount(UserProfile, {
      props: {
        user: {
          id: '1',
          firstName: 'John',
          lastName: 'Doe',
        },
      },
    })
    
    expect(wrapper.text()).toContain('John Doe')
  })

  it('emits delete event when delete button clicked', async () => {
    const wrapper = mount(UserProfile, {
      props: {
        user: { id: '1', firstName: 'John', lastName: 'Doe' },
      },
    })
    
    await wrapper.find('[data-testid="delete-button"]').trigger('click')
    
    expect(wrapper.emitted('delete')).toBeTruthy()
    expect(wrapper.emitted('delete')![0]).toEqual(['1'])
  })
})
```

## 性能优化

### 后端性能

1. **数据库优化**:
   - 使用索引优化查询
   - 避免 N+1 查询问题
   - 使用连接池管理数据库连接

2. **缓存策略**:
   - 使用 Redis 缓存热点数据
   - 实现合理的缓存过期策略
   - 避免缓存穿透和雪崩

3. **并发处理**:
   - 使用异步处理耗时操作
   - 实现请求限流和熔断
   - 合理使用连接池

### 前端性能

1. **代码分割**:
   - 使用路由懒加载
   - 按需加载组件
   - 使用动态导入

2. **资源优化**:
   - 压缩图片和静态资源
   - 使用 CDN 加速
   - 实现浏览器缓存

3. **渲染优化**:
   - 使用虚拟滚动处理大列表
   - 避免不必要的重渲染
   - 使用 Web Workers 处理复杂计算

## 部署

### 生产环境部署

```bash
# 1. 构建后端
cd backend
cargo build --release

# 2. 构建前端
cd ../frontend
npm run build

# 3. 配置生产环境变量
export DATABASE_URL=postgresql://user:pass@host:5432/dbname
export REDIS_URL=redis://host:6379
export JWT_SECRET=your-production-secret

# 4. 启动服务
./target/release/server
```

### Docker 部署

```bash
# 构建镜像
docker build -t bingxi-erp:latest .

# 运行容器
docker run -d \
  -p 8080:8080 \
  -e DATABASE_URL=postgresql://... \
  -e REDIS_URL=redis://... \
  bingxi-erp:latest
```

### Kubernetes 部署

详见 [Kubernetes 部署指南](./deployment/kubernetes.md)

## 调试技巧

### 后端调试

```bash
# 启用详细日志
RUST_LOG=debug cargo run

# 使用 backtrace
RUST_BACKTRACE=1 cargo run

# 运行特定测试
cargo test test_name -- --nocapture

# 使用调试器
rust-gdb target/debug/server
```

### 前端调试

```bash
# 启用调试模式
VITE_DEBUG=true npm run dev

# 使用 Vue Devtools
# 安装浏览器扩展：Vue.js devtools

# 使用网络面板
# 检查 API 请求和响应
```

## 常见问题

### 数据库连接失败

**问题**: `Error: Connection refused`

**解决方案**:
1. 检查数据库服务是否运行
2. 验证连接字符串是否正确
3. 检查防火墙设置
4. 确认数据库用户权限

### 前端 API 调用失败

**问题**: `Network Error` 或 `401 Unauthorized`

**解决方案**:
1. 检查后端服务是否运行
2. 验证 API 基础路径配置
3. 检查 Token 是否过期
4. 查看浏览器网络面板错误详情

### 编译错误

**问题**: Rust 编译失败

**解决方案**:
1. 运行 `cargo check` 查看详细错误
2. 检查依赖版本兼容性
3. 清理构建缓存：`cargo clean`
4. 更新依赖：`cargo update`

## 贡献指南

### 如何贡献

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'feat: Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

### 代码审查

- 确保所有测试通过
- 遵循编码规范
- 添加必要的文档
- 保持提交信息清晰

### 行为准则

请遵循 [Contributor Covenant](./CODE_OF_CONDUCT.md)