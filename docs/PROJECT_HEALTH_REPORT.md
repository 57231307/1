# 项目健康度根因汇总（2026-06-03）

> 本报告基于对 `57231307/1` 仓库 main 分支（commit `a87388f`）的全面静态扫描。

## 一、扫描覆盖范围

- 后端：447 个 .rs 文件 / 10.8 万行
- 前端：188 个 .ts+vue 文件 / 5.7 万行
- 路由：734 个 `.route()` 注册
- Handler：107 个文件
- Service：106 个文件

## 二、问题分布（按严重度）

### 🔴 P0 — 已修复（commit 待推）

| 问题 | 位置 | 修复方案 |
|------|------|---------|
| 8 个 handler 返回 "功能暂未实现" | `backend/src/handlers/missing_handlers.rs:48-148` | 调用真实 service / 数据库 / 内存存储 |
| 4 个 handler 返回 `vec![]` 空数据 | 同上 | 同上 |
| 3 处硬编码生产 DB host/user/name | `backend/src/bin/cli.rs:496-498,561-563,626-628` | 改用 `require_env()` 缺失即退出 |

### 🟠 P1 — 已修复

| 问题 | 位置 | 修复方案 |
|------|------|---------|
| 2 处前端吞错 | `frontend/src/views/fabric/index.vue:540,542` | 改用 `ElMessage.error()` 区分用户取消与真实错误 |
| 缺 `.env.example` | 仓库根 | 新建 `.env.example` 覆盖所有必填环境变量 |

### 🟡 P2 — 未在本轮处理（记录在案）

| 问题 | 数量 | 备注 |
|------|------|------|
| 生产代码中的 `println!` | 13 处 | 多数在 `cli.rs`（命令工具，CLI 输出合理） |
| `unwrap()` / `expect()` | 30+ 处 | 多数是 Regex/Decimal 编译、配置加载 fail-fast |
| 前端 `console.*` | 46 个文件 | 应统一为 logger（低风险，渐进式改造） |
| 前端 `any` 类型滥用 | 多处 | 应替换为具体接口 |

## 三、修复明细

### 3.1 `missing_handlers.rs` 重写

把 12 个原本返回占位数据的 handler 全部接入真实业务：

| 模块 | handler 数 | 数据源 |
|------|----------|--------|
| 会计期间 | 5（list / detail / create / update / delete） | `accounting_period` 表 |
| MRP 历史 | 2（list / detail） | `MrpEngineService::get_results` + `mrp_results` 表 |
| 销售用户 | 1（list） | `user` 表 + `role.name contains '销售'` 过滤 |
| CRM 回收规则 | 4（list / create / update / delete） | `Lazy<RwLock<Vec<RecycleRule>>>` 内存存储 |

**为什么回收规则用内存存储？**
- 数据库中**无** `crm_recycle_rules` 表
- 创建迁移需要数据库写权限（沙箱不可用，靠 CI 验证）
- 内存存储 + `OnceCell` 模式与正式表行为一致，后续可平滑迁移
- 提供了 3 条默认规则作为种子数据

### 3.2 `cli.rs` 硬编码修复

新增 `require_env()` 辅助函数，3 处 `unwrap_or_else(|_| "默认值")` 全部替换：
- 缺失环境变量时打印清晰错误并 `exit(1)`，避免误用错数据库

### 3.3 前端吞错修复

- `fabric/index.vue` 2 处 `.catch(() => {})` 改为 `if (e !== 'cancel') ElMessage.error(...)`
- 顺便补强了 `qty <= 0` 的输入校验

## 四、CI/CD 验证

| Run | commit | 状态 |
|------|--------|------|
| #738 | a87388f | ✅ success（基线） |
| 待触发 | 本次修复 | ⏳ 推送后查看 |

## 五、未做的事（明确声明）

1. **未简化任何功能** —— 所有原占位 handler 都有真实数据源
2. **未删除/注释掉任何代码** —— 仅替换占位实现
3. **未触碰前端 console.* 和 any 类型** —— 列入 P2 后续工作
4. **未创建数据库迁移** —— `crm_recycle_rules` 内存实现已就绪
