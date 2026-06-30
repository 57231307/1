# 贡献指南

> 感谢您对冰溪 ERP 项目的关注！我们欢迎所有形式的贡献（代码、文档、测试、反馈）。
> 本指南将帮助您快速上手并遵循项目的最佳实践。

---

## 📋 目录

- [行为准则](#-行为准则)
- [提交流程](#-提交流程)
- [提交规范](#-提交规范)
- [代码规范](#-代码规范)
- [测试要求](#-测试要求)
- [文档要求](#-文档要求)
- [PR 流程](#-pr-流程)
- [Review 流程](#-review-流程)
- [发布流程](#-发布流程)
- [沟通渠道](#-沟通渠道)
- [常见问题](#-常见问题)

---

## 🌟 行为准则

### 我们的承诺

为了营造一个开放和包容的环境，我们承诺：

- **尊重**：尊重不同观点和经验
- **包容**：欢迎所有背景的贡献者
- **合作**：以协作精神解决冲突
- **专业**：专注于对社区最有利的事情

### 不可接受的行为

- 使用性化语言或图像
- 侮辱性 / 贬损性评论
- 公开或私下骚扰
- 未经许可发布他人隐私信息
- 其他不道德或不专业的行为

### 举报

如遇违反行为准则的情况，请通过以下方式举报：

- **GitHub Issue**：[https://github.com/57231307/1/issues](https://github.com/57231307/1/issues)
- **邮箱**：conduct@bingxi-erp.example.com

---

## 🔄 提交流程

### 1. Fork 仓库

```bash
# 在 GitHub 上点击 Fork 按钮
# 克隆您的 fork
git clone https://github.com/YOUR_USERNAME/1.git
cd 1

# 添加 upstream 远程
git remote add upstream https://github.com/57231307/1.git

# 验证远程
git remote -v
```

### 2. 创建功能分支

```bash
# 同步最新代码
git checkout main
git pull upstream main

# 创建功能分支（命名规范见下）
git checkout -b feature/your-feature-name
# 或
git checkout -b fix/your-bug-fix
# 或
git checkout -b docs/your-doc-update
```

**分支命名规范**：

| 类型 | 格式 | 示例 |
|------|------|------|
| 新功能 | `feature/<name>` | `feature/add-new-report` |
| Bug 修复 | `fix/<name>` | `fix/login-redirect` |
| 文档 | `docs/<name>` | `docs/update-readme` |
| 重构 | `refactor/<name>` | `refactor/extract-service` |
| 测试 | `test/<name>` | `test/add-unit-tests` |
| 性能 | `perf/<name>` | `perf/optimize-query` |
| 安全 | `security/<name>` | `security/fix-cve-2024-xxx` |
| 运维 | `ops/<name>` | `ops/update-k8s-manifest` |

### 3. 编写代码

```bash
# 编写代码
# 遵循 [代码规范](#-代码规范) 和 [测试要求](#-测试要求)

# 运行本地检查
cd backend
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test

cd ../frontend
npm run lint
npm run test:unit
```

### 4. 提交变更

```bash
# 查看变更
git status
git diff

# 添加变更
git add .

# 提交（遵循 [提交规范](#-提交规范)）
git commit -m "feat(module): 添加 XXX 功能

详细描述变更原因、影响范围、相关 issue。

Closes #123"

# 推送到您的 fork
git push origin feature/your-feature-name
```

### 5. 创建 Pull Request

1. 在 GitHub 上访问您的 fork 页面
2. 点击 "Compare & pull request" 按钮
3. 选择目标分支：
   - **`main`**：仅限 hotfix / 紧急修复
   - **`test`**：常规功能（推荐）
4. 填写 PR 模板（见 [PR 流程](#-pr-流程)）
5. 点击 "Create pull request"

---

## 📝 提交规范

我们采用 [Conventional Commits](https://www.conventionalcommits.org/) 规范。

### 格式

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type（必填）

| Type | 说明 | 示例 |
|------|------|------|
| `feat` | 新功能 | `feat(quotation): 添加报价审批流` |
| `fix` | Bug 修复 | `fix(login): 修复登录重定向错误` |
| `docs` | 文档变更 | `docs(readme): 更新快速开始` |
| `style` | 代码格式（不影响功能） | `style(rust): 格式化 inventory 模块` |
| `refactor` | 重构（既不是新功能也不是 Bug 修复） | `refactor(service): 拆分 order_service` |
| `perf` | 性能优化 | `perf(query): 添加复合索引` |
| `test` | 测试相关 | `test(service): 添加 inventory 测试` |
| `build` | 构建系统 | `build(deps): 升级 axum 到 0.7` |
| `ci` | CI/CD | `ci(github): 添加 E2E 工作流` |
| `chore` | 其他杂项 | `chore(deps): 同步 Cargo.lock` |
| `revert` | 回滚 | `revert: 回滚 feat(xxx)` |

### Scope（必填）

Scope 是变更影响范围，常见值：

- 后端：`service:inventory` / `handler:order` / `model:customer`
- 前端：`vue:user-tab` / `component:v2table` / `store:auth`
- 文档：`docs:readme` / `docs:api`
- 基础设施：`k8s:deployment` / `docker:compose`
- 安全：`security:auth` / `security:csp`
- 性能：`perf:query` / `perf:cache`
- 通用：`ci` / `deps` / `config`

### Subject（必填）

- 使用中文
- 不超过 50 字符
- 首字母不大写
- 句末不加句号
- 使用祈使语气（"添加"而非"添加了"）

### Body（可选）

- 解释变更的**原因**和**影响**
- 与 Subject 之间空一行
- 每行不超过 72 字符

### Footer（可选）

- 关联 Issue：`Closes #123` / `Refs #456`
- 不兼容变更：`BREAKING CHANGE: <description>`
- 多个条目用空行分隔

### 示例

```
feat(quotation): 添加报价审批流

实现 4 级审批（业务员 → 主管 → 经理 → 总经理），支持会签和加签。

- 添加 approval_step / approval_record 表
- 实现 ApprovalService.approve() / reject() / transfer()
- 前端添加 ApprovalProgress 组件
- 添加 5 个集成测试

Closes #123
Refs #456
```

---

## 🛠️ 代码规范

### Rust 后端

#### 格式化

```bash
# 格式化（提交前必跑）
cargo fmt --all

# 检查格式
cargo fmt --all -- --check
```

#### Lint

```bash
# clippy（CI 强制 -D warnings）
cargo clippy --all-targets -- -D warnings

# 仅检查
cargo clippy --all-targets
```

#### 命名约定

| 类型 | 命名 | 示例 |
|------|------|------|
| 模块 | snake_case | `inventory_count` |
| 类型 | PascalCase | `InventoryItem` |
| 函数 | snake_case | `get_inventory` |
| 变量 | snake_case | `total_amount` |
| 常量 | SCREAMING_SNAKE | `MAX_PAGE_SIZE` |
| 错误类型 | PascalCase + Error | `InventoryError` |
| Trait | PascalCase | `Repository` |

#### 注释规范

```rust
/// 公共 API 必须有文档注释
///
/// # 参数
/// - `id`: 库存 ID
///
/// # 返回
/// - `Ok(InventoryItem)`: 成功
/// - `Err(InventoryError)`: 失败
///
/// # 示例
/// ```rust
/// let item = get_inventory(1).await?;
/// ```
pub async fn get_inventory(id: i64) -> Result<InventoryItem, InventoryError> {
    // 行内注释解释 WHY，不是 WHAT
    todo!()
}
```

#### 错误处理

- 使用 `Result<T, E>` 显式处理错误
- 严禁 `unwrap()` / `expect()` 在生产代码中使用
- 业务错误返回用户可读消息
- 系统错误记录详细日志 + 返回通用消息

### TypeScript / Vue 前端

#### 格式化

```bash
# ESLint
npm run lint

# 自动修复
npm run lint:fix

# Prettier
npm run format
```

#### 类型检查

```bash
# TypeScript
npm run typecheck

# 或
npx tsc --noEmit
```

#### 命名约定

| 类型 | 命名 | 示例 |
|------|------|------|
| 组件 | PascalCase | `UserTab.vue` |
| 组合式函数 | camelCase + use 前缀 | `useTableApi` |
| 工具函数 | camelCase | `formatDate` |
| 类型 | PascalCase | `UserInfo` |
| 接口 | PascalCase + I 前缀（可选） | `IUserInfo` |
| 常量 | UPPER_SNAKE | `API_BASE_URL` |
| CSS 类 | kebab-case | `user-tab` |

#### Vue 组件

```vue
<script setup lang="ts">
// 导入顺序：vue → 第三方 → 内部
import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import { useUserStore } from '@/stores/user'

// Props / Emits 类型必须显式声明
const props = defineProps<{
  userId: number
}>()

const emit = defineEmits<{
  (e: 'update', value: User): void
}>()
</script>

<template>
  <!-- 模板简洁，避免复杂表达式 -->
  <div class="user-tab">
    <p>{{ userName }}</p>
  </div>
</template>

<style scoped>
/* scoped 样式 */
.user-tab { /* ... */ }
</style>
```

### 数据库（PostgreSQL）

#### 迁移文件

```sql
-- 文件名格式：YYYYMMDDHHMMSS_<name>.sql
-- 2026_06_17_120000-add_inventory_index.sql

-- 必须包含事务
BEGIN;

-- 添加注释
COMMENT ON TABLE inventory IS '库存表';

-- 添加索引（使用 CONCURRENTLY 在线创建）
CREATE INDEX CONCURRENTLY idx_inventory_warehouse_product ON inventory(warehouse_id, product_id);

COMMIT;
```

#### 命名约定

| 类型 | 命名 | 示例 |
|------|------|------|
| 表 | snake_case + 复数 | `inventory_items` |
| 列 | snake_case | `total_amount` |
| 主键 | `id` | `id BIGSERIAL PRIMARY KEY` |
| 外键 | `<table>_id` | `user_id` |
| 索引 | `idx_<table>_<col>` | `idx_inventory_warehouse` |
| 唯一索引 | `uniq_<table>_<col>` | `uniq_users_email` |
| 时间戳 | `<verb>_at` | `created_at` / `updated_at` / `deleted_at` |
| 布尔 | `is_<adj>` 或 `has_<noun>` | `is_active` / `has_children` |

### Git 规范

#### Commit message

- 中文编写
- 一个 commit 做一件事
- 避免巨型 commit（> 500 行应拆分）
- 关键决策需在 commit body 中说明 WHY

#### .gitignore

- 不提交 `.env` / `.env.local`
- 不提交 `target/` / `dist/` / `node_modules/`
- 不提交 IDE 配置（.vscode / .idea）— 例外：共享的 settings.json
- 不提交密钥 / 凭证 / 私钥

---

## 🧪 测试要求

### 测试金字塔

| 层级 | 数量 | 速度 | 工具 | 覆盖率 |
|------|------|------|------|-------|
| 单元测试 | 152 | < 1s | cargo test / vitest | 75% |
| 集成测试 | 78 | < 30s | cargo test (integration) | — |
| E2E 测试 | 45 | < 5min | Playwright | — |
| 混沌测试 | 3 | < 30min | chaos-mesh | — |

### 编写测试

#### 后端单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// 测试库存服务获取正常场景
    #[tokio::test]
    async fn test_get_inventory_success() {
        let service = InventoryService::new(mock_db());
        let result = service.get(1).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, 1);
    }

    /// 测试库存服务获取不存在场景
    #[tokio::test]
    async fn test_get_inventory_not_found() {
        let service = InventoryService::new(mock_db());
        let result = service.get(999).await;
        assert!(matches!(result, Err(InventoryError::NotFound)));
    }
}
```

#### 前端单元测试

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import UserTab from './UserTab.vue'

describe('UserTab', () => {
  it('应正确渲染用户名', () => {
    const wrapper = mount(UserTab, { props: { userId: 1 } })
    expect(wrapper.text()).toContain('张三')
  })

  it('应在点击时触发 update 事件', async () => {
    const wrapper = mount(UserTab, { props: { userId: 1 } })
    await wrapper.find('button').trigger('click')
    expect(wrapper.emitted('update')).toBeTruthy()
  })
})
```

#### E2E 测试

```typescript
import { test, expect } from '@playwright/test'

test('用户登录流程', async ({ page }) => {
  // v9 P1-1 修复：对齐项目 fail-secure 模式，凭据从环境变量注入
  const TEST_USERNAME = process.env.TEST_USERNAME
  const TEST_PASSWORD = process.env.TEST_PASSWORD
  if (!TEST_USERNAME || !TEST_PASSWORD) {
    throw new Error('E2E 测试需要环境变量 TEST_USERNAME / TEST_PASSWORD（fail-secure 模式）')
  }
  await page.goto('http://localhost:5173/login')
  await page.fill('input[name=username]', TEST_USERNAME)
  await page.fill('input[name=password]', TEST_PASSWORD)
  await page.click('button[type=submit]')

  await expect(page).toHaveURL('http://localhost:5173/dashboard')
  await expect(page.locator('h1')).toContainText('仪表盘')
})
```

### 测试要求

- **新增功能必须带测试**（覆盖率门槛：服务层 70%）
- **修改现有功能需更新相关测试**
- **修复 Bug 必须先写复现测试**（TDD）
- **测试命名清晰描述场景**（中文）
- **禁止跳过测试**（`#[ignore]` / `it.skip`）— 例外：环境依赖

---

## 📚 文档要求

### 文档位置

| 类型 | 位置 |
|------|------|
| 项目规范 | 根目录（`README.md` / `CHANGELOG.md` / `CONTRIBUTING.md`） |
| API 文档 | `docs/*-api.md` |
| 用户手册 | `docs/*-user-manual.md` |
| 部署指南 | `docs/*-deployment-guide.md` |
| 评估报告 | `docs/2026-06-17-p[0-5]-*.md` |
| 架构文档 | `docs/architecture/` |
| 数据库文档 | `docs/database/` |
| 重构计划 | `docs/refactoring/` |

### 文档命名

- **API 文档**：`{module}-api.md`（例：`quotation-api.md`）
- **用户手册**：`{module}-user-manual.md`
- **部署指南**：`{module}-deployment-guide.md`
- **评估/计划**：`YYYY-MM-DD-p{N}-{name}.md`

### Markdown 规范

- 使用 `#` 一级标题作为文档标题
- 章节用 `##` 二级标题
- 小节用 `###` 三级标题
- 代码块标注语言（` ```rust ` / ` ```typescript ` / ` ```bash `）
- 表格必须有表头
- 链接使用相对路径（`./other-doc.md`）

### API 文档模板

```markdown
# 模块名 API 文档

> 简述

## 端点列表

### POST /api/v1/resource

#### 请求

```json
{
  "field1": "value1"
}
```

#### 响应

```json
{
  "code": 0,
  "data": { ... }
}
```

#### 错误码

| 状态码 | 含义 |
|--------|------|
| 400 | 参数错误 |
| 401 | 未认证 |
| 403 | 无权限 |
| 404 | 资源不存在 |
| 500 | 服务器错误 |
```

### 文档要求

- **新增 API 必须更新 `docs/*-api.md`**
- **新增模块必须有用户手册**
- **新增部署方式必须有部署指南**
- **重大变更必须更新 CHANGELOG**

---

## 🔁 PR 流程

### PR 模板

```markdown
## 变更说明

### 改动内容
- 简述变更 1
- 简述变更 2

### 改动原因
为什么需要这次变更？解决什么问题？

### 影响范围
- [ ] 后端
- [ ] 前端
- [ ] 数据库（migration）
- [ ] 文档
- [ ] CI/CD
- [ ] 部署
- [ ] 安全

### 关联 Issue
Closes #123
Refs #456

## 测试

### 单元测试
- [ ] 已添加新功能的单元测试
- [ ] 已更新现有功能的测试
- [ ] 本地 `cargo test` 通过
- [ ] 本地 `npm run test:unit` 通过

### 集成测试
- [ ] 已添加/更新集成测试
- [ ] 本地 `cargo test --test '*'` 通过

### E2E 测试
- [ ] 已添加/更新 E2E 测试（如适用）
- [ ] 本地 `npm run test:e2e` 通过

## 文档

- [ ] 已更新 README.md（如需要）
- [ ] 已更新 CHANGELOG.md
- [ ] 已更新 API 文档
- [ ] 已更新用户手册

## Checklist

- [ ] 代码遵循项目规范（rustfmt / clippy / eslint / tsc）
- [ ] 提交信息遵循 conventional commits
- [ ] 分支基于最新的 `test` / `main`
- [ ] CI 全部通过
- [ ] 没有遗留的 console.log / println! / dbg!
- [ ] 没有遗留的 TODO / FIXME
- [ ] 没有引入新的警告
```

### 分支策略

- **目标分支**：`test`（默认）/ `main`（仅 hotfix）
- **基于分支**：`test`
- **同步上游**：
  ```bash
  git fetch upstream
  git rebase upstream/test
  ```

### PR 标题

PR 标题 = 第一个 commit 的 subject。

### PR 描述

- 清晰说明变更原因
- 列出变更内容
- 关联相关 issue
- 截图 / 视频（UI 变更必须）

### 自动化检查

PR 创建后会自动运行：

- ✅ Rust clippy / rustfmt
- ✅ TypeScript tsc / eslint
- ✅ 单元测试
- ✅ 集成测试
- ✅ 文档格式（markdownlint）
- ✅ 死链检查（lychee）
- ✅ 依赖漏洞扫描（cargo-audit / npm audit）

**所有检查必须通过才能合入。**

---

## 👀 Review 流程

### Review 人数

- **常规 PR**：至少 1 个 reviewer
- **重大变更**（数据库 / 安全 / 架构）：至少 2 个 reviewer
- **Hotfix**：至少 1 个 reviewer

### Review 检查项

#### 功能

- [ ] 实现符合需求
- [ ] 边界场景已处理
- [ ] 错误处理完善
- [ ] 性能可接受

#### 代码质量

- [ ] 命名清晰
- [ ] 注释充分
- [ ] 单一职责
- [ ] 无重复代码
- [ ] 遵循 SOLID 原则

#### 安全

- [ ] 输入验证
- [ ] 权限检查
- [ ] SQL 注入防护
- [ ] XSS 防护
- [ ] 敏感信息保护

#### 测试

- [ ] 测试充分
- [ ] 测试覆盖关键路径
- [ ] 测试独立可运行

#### 文档

- [ ] 代码注释清晰
- [ ] 公开 API 有文档
- [ ] 变更已记录到 CHANGELOG

### Review 评论规范

- **建设性**：提出改进建议而非批评
- **具体**：指出具体行号 / 文件
- **解释**：说明为什么需要修改
- **示例**：提供修改示例

### Review 通过条件

- ✅ 自动化检查全部通过
- ✅ 所有评论已解决（resolved）
- ✅ 至少 1 个 reviewer Approve
- ✅ 没有 Request Changes
- ✅ 分支与目标分支无冲突

### 合并策略

- **常规 PR**：Squash and merge
- **复杂 PR**：Merge commit（保留历史）
- **Hotfix**：Merge commit

### 合并后

- 分支会被自动删除
- 相关 Issue 自动关闭（如使用 `Closes #123`）

---

## 🚀 发布流程

### 版本号规范

采用 [语义化版本](https://semver.org/)：

```
MAJOR.MINOR.PATCH
```

- **MAJOR**：不兼容的 API 变更
- **MINOR**：向后兼容的新功能
- **PATCH**：向后兼容的 Bug 修复

### 发布节奏

- **Major**：每年 1-2 次
- **Minor**：每月 1-2 次
- **Patch**：每周 1-2 次（如有）

### 发布步骤

1. **创建 release 分支**：
   ```bash
   git checkout test
   git pull origin test
   git checkout -b release/v1.2.0
   ```

2. **更新版本号**：
   - `backend/Cargo.toml`
   - `frontend/package.json`
   - `CHANGELOG.md`

3. **运行完整测试**：
   ```bash
   cargo test --all
   npm run test:all
   ```

4. **构建**：
   ```bash
   cargo build --release
   npm run build
   docker build -t bingxi-erp:1.2.0 .
   ```

5. **创建 PR**（release → main）

6. **合并并打 tag**：
   ```bash
   git tag -a v1.2.0 -m "Release v1.2.0"
   git push origin v1.2.0
   ```

7. **发布 GitHub Release**

8. **部署到生产环境**

9. **回 release 分支**：
   ```bash
   git checkout test
   git merge release/v1.2.0
   git push origin test
   git branch -d release/v1.2.0
   ```

### 发布后

- 更新 CHANGELOG.md 发布日期
- 发送 release notes
- 通知相关人员
- 监控生产环境

---

## 💬 沟通渠道

### GitHub

- **Issues**：[https://github.com/57231307/1/issues](https://github.com/57231307/1/issues)
  - Bug 报告
  - 功能请求
  - 安全漏洞（私密）
- **Discussions**：[https://github.com/57231307/1/discussions](https://github.com/57231307/1/discussions)
  - 一般讨论
  - 问题求助
  - 想法分享
- **Pull Requests**：代码审查

### 邮件

- **常规**：support@bingxi-erp.example.com
- **安全**：security@bingxi-erp.example.com
- **行为准则**：conduct@bingxi-erp.example.com

### 紧急情况

- **生产环境故障**：立即在 Issues 创建 `priority: critical` 标签
- **安全漏洞**：发送邮件到 security@bingxi-erp.example.com（请勿公开）

---

## ❓ 常见问题

### Q: 我是新手，能贡献什么？

**A**: 欢迎所有形式的贡献！新手友好的方向：

- 📝 文档改进（拼写、翻译、示例）
- 🐛 Bug 报告（详细复现步骤）
- 🧪 测试补充
- 🌐 国际化翻译
- 💡 功能建议

查看 [good first issue](https://github.com/57231307/1/issues?q=is%3Aopen+is%3Aissue+label%3A%22good+first+issue%22) 标签。

### Q: PR 创建后多久会被 Review？

**A**:

- **常规 PR**：1-3 个工作日
- **紧急 Hotfix**：4 小时内
- **大型 PR**（> 1000 行）：3-5 个工作日

如果超过预期时间未 Review，可以在 PR 评论中 `@` 团队成员。

### Q: 能否在 PR 中包含多个不相关的变更？

**A**: **强烈不建议**。请将不相关的变更拆分为多个 PR，原因：

- Review 困难
- 出问题难回滚
- 影响 CI 效率
- 难以识别问题来源

### Q: 如何处理 merge conflict？

```bash
git fetch upstream
git rebase upstream/test

# 解决冲突后
git add .
git rebase --continue

# 强制推送
git push --force-with-lease origin your-branch
```

### Q: CI 失败了怎么办？

1. 查看 CI 日志，定位失败原因
2. 本地重现并修复
3. 推送修复 commit
4. CI 会自动重新运行

### Q: 我的 PR 被拒绝，还能再次提交吗？

**A**: 完全可以！请：

1. 理解拒绝原因
2. 与 reviewer 讨论
3. 修改后重新提交
4. 在 PR 描述中说明修改内容

### Q: 如何成为 maintainer？

**A**: Maintainer 通过以下方式产生：

- 持续贡献（6+ 个月）
- 多次高质量 PR
- 帮助其他贡献者
- 参与决策讨论
- 现有 maintainer 推荐

---

## 📜 许可证

贡献者同意其贡献遵循项目 [LICENSE](LICENSE) 协议。

---

## 🙏 致谢

感谢所有为冰溪 ERP 做出贡献的开发者！

您的每一份贡献都让这个项目变得更好。

---

<div align="center">

**[⬆ 回到顶部](#贡献指南)**

Made with ❤️ by 冰溪 ERP Team

</div>
