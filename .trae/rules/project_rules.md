# 项目开发规范

## 一、项目概述
每次任务开始时需要查找并调用合适的智能体和技能工具，优先进行任务拆解，然后对任务进行实现。

## 二、开发规范与要求

### 1. 基本要求

**任务管理**
- 使用中文建立待办任务
- 每完成一个待办任务，需立即标记为"已完成"状态

**沟通语言**
- 使用中文进行回复和沟通

**编码规范**
- 项目禁止硬编码，所有文本需要使用中文
- 代码注释必须使用中文

**开发辅助**
- 每次新增或修改功能时，必须调用合适的技能或MCP工具进行开发辅助
- 严格按照技能规范进行开发，不允许例外

**记忆管理**
- 实时查看和更新记忆，所有关键内容需存储在记忆中
- 生成的文档需实时更新，删除旧文档时需提取关键内容存储

### 2. 数据库配置

- **数据库类型**：PostgreSQL
- **连接方式**：使用远程数据库连接模式，确保数据库连接模块的稳定性和安全性

### 2.5 验证强制（CI/CD Only）

- **禁止**本地编译/构建验证（`cargo build`、`cargo check`、`cargo test`、`cargo fmt -- --check`、`cargo clippy`、`npm run build`、`vue-tsc`、`pnpm typecheck`、`pnpm lint` 等任何本地构建命令）
- **禁止**本地启动服务做端到端验证（`npm run dev`、`cargo run`、后端服务、前端 dev server）
- **唯一允许的本地操作**：文件 diff、语法、文本类（git status、cat、grep、sed、Edit、Write 等只读或文本编辑）
- **所有验证必须经 CI/CD**：
  1. 修改代码 → 写 commit → push 触发 CI
  2. 用 GitHub Actions API 监控 run 状态（`/actions/runs`、`/actions/runs/{id}/jobs`、`/actions/runs/{id}/logs`）
  3. 失败 → 拉取 logs zip → 解析 annotations → 修复 → 重新 push
  4. 循环直到 CI 全绿

### 3. 功能实现依据

- 功能实现必须严格按照技能进行推进
- 新增功能的接口、数据库操作需遵循现有规范

### 4. 打包与发布要求

打包时必须进行全面的项目测试，包括但不限于：
- 全面的功能测试
- 兼容性测试
- 稳定性测试

确保打包后的程序能够在目标环境中正常启动并完整运行所有功能模块，无运行时错误或功能缺失。

### 5. 项目标识

项目名称：询问用户项目基础信息，所有相关文档、界面及输出信息中必须统一使用。

## 三、代码规范

### 1. 命名约定

**命名原则**
- 使用有意义的、描述性的名称
- 遵循项目或语言的命名规范

**避免事项**
- 避免缩写和单字母变量（除非是约定俗成的，如循环中的 i）

### 2. 代码组织

**代码布局**
- 相关代码放在一起
- 保持适当的抽象层次

**函数设计**
- 函数只做一件事
- 保持单一职责原则

### 3. 注释与文档

**注释原则**
- 注释应该解释"为什么"，而不是"做什么"
- 为公共API提供清晰的文档

**文档维护**
- 更新注释以反映代码变化

## 四、安全规范

### 1. 敏感信息保护

- 禁止在代码中硬编码敏感信息（密码、密钥、令牌等）
- 使用环境变量或配置管理工具管理敏感信息
- 禁止将敏感信息提交到版本控制系统

### 2. 输入验证

- 所有用户输入必须进行验证和清理
- 使用参数化查询防止SQL注入
- 对输出进行编码防止XSS攻击

## 五、测试规范

### 1. 测试要求

- 新增功能必须编写单元测试
- 修改现有功能需要更新相关测试
- 测试覆盖率应保持在合理水平

### 2. 测试类型

- **单元测试**：测试单个函数或方法的功能
- **集成测试**：测试模块间的交互
- **端到端测试**：测试完整的用户流程

### 3. 测试命名

- 测试函数名应该清晰描述测试的场景
- 使用中文描述测试目的

## 六、死代码处理规范

### 1. 总体原则

- **禁止**使用文件级 `#![allow(dead_code)]` 全局抑制；CI 会在 clippy 检查中失败。
- **禁止**使用 crate 级 `#![allow(unused_imports)]` / `#![allow(unused_variables)]`。
- 真正未使用的项应**显式删除**；保留的项应接入业务或加 `pub` 修饰以表明意图。
- **例外**：[backend/src/models/](file:///workspace/backend/src/models/) 下的 SeaORM 自动生成模型可保留文件级 `#![allow(dead_code)]`，
  原因是模型字段由 SeaORM 派生宏使用，不能手工逐字段标注。**禁止**用此项例外绕过 utils/ 等核心模块的清理。

### 2. 处理流程

1. 编译器/clippy 报告具体 `dead_code` 位置
2. 评估该项是否仍需要：
   - 需要保留：接入业务、添加 `pub` 或 `pub(crate)` 修饰
   - 不再需要：立即删除（git 会保留历史）
3. 个别 `pub` API 当前未被业务引用时：
   - 在该项上加 `#[allow(dead_code)]` + TODO 注释
   - 编写计划任务在下一个迭代接入

### 3. TODO 注释标准模板

文件级抑制（仅限业务文件，不再用于 utils/ 等核心模块）：

```rust
#![allow(dead_code)]
// TODO(tech-debt): 业务接入或重评估后逐项移除；rustc 1.94+ 编译时由编译器报告具体死代码位置。
```

项级抑制（推荐）：

```rust
/// 商品全字段查询 DTO（预留 API）
#[allow(dead_code)] // TODO(tech-debt): 报表模块接入后移除
pub struct ProductFullDto { ... }
```

### 4. CI 强制

- 配置：[backend/.clippy.toml](file:///workspace/backend/.clippy.toml) `warn` 段开启 `dead_code`/`unused_imports`/`unused_variables`
- 工作流：[.github/workflows/ci-cd.yml](file:///workspace/.github/workflows/ci-cd.yml) `cargo clippy --all-targets -- -D warnings`

任何死代码警告都会让 CI 失败，开发者必须立即处理。

### 5. utils/ 模板（已建立）

`backend/src/utils/` 下的 8 个核心文件已**全部**开启死代码检查（移除文件级 `#![allow(dead_code)]`），
作为全项目的死代码处理模板：

| 文件 | 处理方式 |
|------|----------|
| [fabric_five_dimension.rs](file:///workspace/backend/src/utils/fabric_five_dimension.rs) | 删除 `FiveDimensionStatistics`、`FiveDimensionQueryBuilder` 及对应测试 |
| [di_container.rs](file:///workspace/backend/src/utils/di_container.rs) | 删除 `GLOBAL_CONTAINER`、`register`/`resolve`/`is_registered` 自由函数 |
| [cache.rs](file:///workspace/backend/src/utils/cache.rs) | 删除 `CacheKey` 枚举及 Display 实现 |
| [response.rs](file:///workspace/backend/src/utils/response.rs) | 全部已使用，无需变更 |
| [password_validator.rs](file:///workspace/backend/src/utils/password_validator.rs) | 全部已使用，无需变更 |
| [log_config.rs](file:///workspace/backend/src/utils/log_config.rs) | 全部已使用，无需变更 |
| [dual_unit_converter.rs](file:///workspace/backend/src/utils/dual_unit_converter.rs) | 全部已使用，无需变更 |
| tree_builder.rs | 整个文件已删除（无业务引用） |

后续 services/、handlers/、models/ 等模块按相同模板处理：评估 → 删除真实死代码或项级 `#[allow(dead_code)]` + TODO。

## 七、版本控制规范

### 1. 提交信息

- 使用有意义的提交信息
- 提交信息应该描述"做了什么"和"为什么"
- 使用中文编写提交信息

### 2. 分支管理

- 功能开发使用功能分支
- 修复bug使用修复分支
- 保持主分支的稳定性

### 3. 代码审查

- 所有代码变更需要经过审查
- 审查重点包括：代码质量、安全性、性能、测试覆盖

## 八、性能规范

### 1. 数据库查询

- 优化数据库查询，避免N+1查询问题
- 使用适当的索引
- 对大数据量查询进行分页处理

### 2. 缓存策略

- 合理使用缓存提高性能
- 明确缓存失效策略
- 避免缓存过期数据

### 3. 资源管理

- 及时释放不再使用的资源
- 避免内存泄漏
- 合理控制并发数量

## 九、错误处理规范

### 1. 错误处理原则

- 所有可能失败的操作都需要错误处理
- 提供有意义的错误信息
- 记录错误日志便于调试

### 2. 错误分类

- **业务错误**：返回友好的错误提示
- **系统错误**：记录详细日志，返回通用错误信息
- **验证错误**：明确指出验证失败的原因

### 3. 错误恢复

- 尽可能实现优雅降级
- 提供重试机制
- 保持系统稳定性

## 十、文档规范

### 1. API文档

- 所有API接口必须有文档说明
- 文档包括：接口路径、请求参数、响应格式、示例

### 2. 代码文档

- 复杂逻辑必须有注释说明
- 公共函数必须有文档注释
- 保持文档与代码同步更新

### 3. 用户文档

- 提供清晰的用户操作指南
- 包含常见问题解答
- 定期更新文档内容

## 十一、持续改进

### 1. 代码重构

- 定期审查代码质量
- 及时重构低质量代码
- 保持代码简洁清晰

### 2. 技术债务

- 记录技术债务
- 制定偿还计划
- 避免技术债务积累

### 3. 学习成长

- 关注新技术发展
- 定期团队技术分享
- 持续改进开发流程
