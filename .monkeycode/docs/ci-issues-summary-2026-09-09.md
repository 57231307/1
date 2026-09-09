# CI 问题全量总结（2026-07 ~ 2026-09）

> 汇总范围：doto-su.md 全部历史轮次 + docs/archives/ + run 4483 证据链（/tmp/*.log、jobs4483.json）。
> 用途：修复计划（docs/plans/one-round-fix-plan-2026-09-09.md）执行前的问题基线，避免重蹈覆辙。

---

## 一、构建与静态检查类

| # | 问题 | 根因 | 处置/状态 |
|---|------|------|----------|
| 1 | cargo fmt 检查 FAILURE（PR #810 轮次，5 处不一致） | 本地禁止跑 cargo fmt（IR），格式漂移 | 人工 Edit 修复；PR #808 后 fmt 失败自动修正 |
| 2 | clippy 新增警告反复出现（18 条：11 代码 + 7 dead_code；后续 unused imports ReconciliationDetail 等 3 DTO、empty_line_after_doc_comment、QueryOrder 未用、collapsible_if、doc lint） | 代码变更引入；工具链敏感度高 | PR #807 批量修复；dead_code 走 baseline；经验：**每次提交前 grep 本轮新增 use 与 pub use** |
| 3 | clippy 45min 超时 CANCELLED（PR #758） | 代码逻辑变更量大触发全量 lint；纯注释 PR #765 仅 20min | baseline 非硬阻塞，`--admin` 合并放行；教训：大变更拆 PR |
| 4 | ESLint 误报提前退出 | CI 脚本 `set -e` 下 grep/jq 零命中即退出 | PR #808 改单次扫描 `set +e` |
| 5 | 沙箱本地 rustc 阶段被 SIGKILL | 本地内存不足 OOM | **IR 成立：禁止本地编译，验证一律走 CI**；Release 二进制本地 glibc 不兼容同样废弃本地运行路线 |

## 二、sea-orm / sqlx / PostgreSQL 陷阱（PR #939 七轮 CI 修复）

| # | 问题 | 根因 | 处置/状态 |
|---|------|------|----------|
| 6 | ExprTrait 作用域编译错、E0382 reborrow、ActiveModel 补字段、collapsible_if | sea-orm 2.0 API 变化 | 七轮 commit 逐个修复（943c1c2→5e2e3ef） |
| 7 | AssertSqlSafe 路径、query_one_raw 按值、Value::Null 归一化、Value::Array 双参 | sqlx 0.9 API 坑 | 同上；**经验沉淀：raw SQL 必须走 AssertSqlSafe 包装** |
| 8 | RESET 两条 set_config 合一条 execute 失败 | PG prepared statement 不允许多命令 | 拆两条独立 execute |
| 9 | RLS 触发器运行时 NEW.owner_id 报错 | customers/crm_lead 用 owner_id，suppliers/sales_orders 用 created_by，单函数通写 | 拆 sync_data_department_by_owner / by_creator 两函数 |
| 10 | RLS GUC 同连接池不可见 | 连接池复用连接丢会话变量 | tokio task-local + sqlx before_acquire/after_connect 钩子；锚点测试 #[ignore]（CI 不跑，手动验证清单在 doto.md） |

## 三、发布流水线类

| # | 问题 | 根因 | 处置/状态 |
|---|------|------|----------|
| 11 | GitHub Release 静默失败（资产未上传但 job 报 success） | softprops/action-gh-release@v3 内部错误被吞 | PR #810：gh CLI 替代 + 三重验证（文件存在→Release 创建→资产上传） |
| 12 | Cargo.toml 版本号 `unexpected character '.' after patch` | 4 段式 YYYY.M.D.HHMM 违反 SemVer | PR #812：Cargo.toml 3 段式、TAG/Release 保持 4 段 |
| 13 | 🔴 **版本号自动提交失败（run 4483 / id 34347843357，2026-09-09）**：release job 本地 commit d38c9f6f 后 `git push origin HEAD:main` 被拒（`! [remote rejected]`，"Changes must be made through a pull request" + "Commits must have verified signatures"） | 分支保护：PR required + verified signatures，github-actions[bot] 签名不被认可；**机制性冲突：CI 自动写 main 与分支保护互斥** | commit 遗留 runner 磁盘未入 git；后果：main VERSION 停留 2026.723.1842、Cargo.toml 2026.810.1、Release v2026.9.9.1953 三方不一致 → get_current_version 读旧值 → check_for_updates 永远误报。**修复：计划 P1.5 方案 C**（get_current_version 编译期化 + release job 去 VERSION 推送） |

## 四、E2E job 类

| # | 问题 | 根因 | 处置/状态 |
|---|------|------|----------|
| 14 | ci-e2e-setup-wizard 首跑（run 34135035908）step#9 Playwright 安装失败，后续 step 全跳过 | `--with-deps` 的 apt 锁竞争 | 拆分 install/install-deps（deps 失败仅告警）+ Playwright 缓存层与 ci-e2e 共用 key |
| 15 | flow E2E 登录偶发超时（16 分片并发） | 后端 /auth/lock-status 并发挂起 5s+ | **被 loginViaUI 的 route.fulfill mock 掩盖**（helpers.ts:898）；真实修复：计划 P1.2（handler 匿名化）+ P2.4（去 mock） |
| 16 | run 34188696853（十五轮）在途时推送冻结生效 | 冻结时序 | 仅观察不干预；教训：**冻结指令需同步取消在途触发源** |

## 五、平台机制与操作类

| # | 问题 | 根因 | 处置/状态 |
|---|------|------|----------|
| 17 | merge commit 被拒（HTTP 405） | 仓库禁用 merge commit | 统一 squash merge（PR #939 16 commit → 25ae95e） |
| 18 | GitHub API 查 CI job 返回 0 条 | 不带 `filter=all` 默认只返回最新 incomplete 集 | 固定 `?filter=all&per_page=100` |
| 19 | run_number ≠ run_id（run 4483 = id 34347843357） | API 两套标识 | 查日志用 run_id |
| 20 | GitHub token 401 过期 | token 有时效 | `git credential fill` 刷新写 /tmp/gh_token |
| 21 | setup-wizard 登录表单 validate 失败 | agreedToTerms 复选框未勾 | 三层 fallback + submit 兜底（5e2e3ef） |
| 22 | PG 锚点测试 CI 永不执行 | `#[ignore]` + CI 不跑 ignored | 手动验证清单挂 doto.md（RLS GUC 同池测试） |

## 六、机制教训（对应修复计划的预防位）

1. **CI 自动写 main 与分支保护互斥**（#13）——任何"CI 内 commit + push"设计必须先确认分支规则，改走 API/PR 或编译期内联（P1.5）；
2. **mock 掩盖真实缺陷**（#15）——去 mock 化（P2.4）后并发挂起会重新暴露，属预期，届时按真实性能问题立项；
3. **Release 流水线每步必须显式验证**（#11）——三重验证模式保持；
4. **sea-orm/sqlx 升级期 API 坑密集**（#6-8）——大版本升级 PR 单独拆分，先跑 CI 编译再叠业务改动；
5. **环境差异**（#5、#14）——本地 SIGKILL 与 apt 锁竞争都指向"测试环境一致性"，新 CI step 优先复用既有缓存 key 与安装模式；
6. **版本口径单一来源**（#12+#13）——Cargo.toml（编译期）作为唯一版本源，VERSION 文件降级为部署元数据（P1.5 方案 C 的设计依据）。
