# 项目规则记忆

> 本文件是项目的**规则记忆**，记录必须遵守的规则、指令、偏好和工作流规范。
> 历史归档与详细内容请查阅 [`.monkeycode/docs/archives/`](file:///workspace/.monkeycode/docs/archives/)。
> 最近整理：2026-07-13（合并重复规则，精简规则 15，归档历史经验）。

---

## 一、关键项目规则（必读，按优先级排序）

### 🔴 规则 00（最高优先级，2026-07-11 追加）：修改前关联影响评估强制

> 每次修改后、推送 CI/CD 前，必须评估有没有关联影响。
> 所有修改均为代码级修改，以正常全新安装/系统更新为目标。

**强制执行要求**：
- **推送前评估**：每次 commit / push CI/CD 前，必须评估本次修改对关联模块的影响范围
  - 评估维度：配置文件 / 部署脚本 / 数据库迁移 / 环境变量 / 依赖关系 / API 契约 / 类型定义 / 前后端契约
  - 评估方式：grep 引用点 + 读取受影响文件 + 静态推理影响链路
- **代码级修改原则**：所有修改均为代码级修改，不依赖运行时环境假设
  - 目标：全新安装可正常启动 + 系统更新可平滑升级
  - 禁止：依赖手动步骤补齐配置 / 依赖外部环境变量未在 .env.example 声明 / 依赖未在 config.yaml 模板注入
- **部署同步检查**：修改后端配置字段时，必须同步检查：
  - `.env.example` 是否声明该环境变量
  - `deploy/deploy.sh` + `deploy/deploy-latest.sh` + `deploy/deploy-backend.sh` 是否在 config.yaml 生成时注入该字段
  - `backend/config.yaml.example` 是否包含该字段示例
  - main.rs / app_state.rs 的 fail-fast 校验是否与部署脚本一致
- **前后端契约同步**：修改 API 返回格式 / 字段名 / 分页结构时，必须同步检查：
  - 前端 composables（如 useTableApi）是否兼容新格式
  - 前端 api/*.ts 类型定义是否同步
  - 前端 views 中直接使用该字段的组件是否受影响
- **数据库迁移同步**：新增字段 / 修改字段时，必须同步：
  - 生成对应的 SQL 迁移文件（database/migration/）
  - 更新 SeaORM Entity 模型
  - 检查所有引用该字段的 service / handler
- **禁止假设"只改一处"**：任何代码修改都可能产生级联影响，必须主动评估而非事后修复

### 🔴 规则 0（最高优先级，2026-07-04 追加）：真实实现强制

> 对所有预留的 api 及预留的功能/占位符功能/路由进行实现，
> 对所有未真实接入的功能等需要真实接入，
> 对所有遇到的错误均进行统一修复，
> 对所有的功能均需要真实接入。

**强制执行要求**：
- 所有 `#[allow(dead_code)] + TODO(tech-debt)` 标记的预留 API 逐个评估并真实接入业务或删除
- 所有占位符功能（stub / placeholder / TODO 注释 / `let _ =` 占位）必须真实实现
- 所有未真实接入的功能 / 中间件 / 路由进行真实接入（不允许遗留死路由）
- 所有遇到的编译错误 / CI 错误 / 运行时错误必须统一修复，禁止 `|| true` / `unwrap_or_default()` 掩盖
- 所有功能必须真实接入业务链路：路由 → handler → service → model → DB 全链路打通

**修复模式参考**：
- `let _ = svc.method().await;`（吞错）→ `if let Err(e) = svc.method().await { tracing::warn!(error=%e, context, "描述"); }`
- `let _ = exists_check;`（检查存在性丢弃结果）→ 直接表达式语句 `exists_check.await?;`
- `.expect("msg")`（启动期 panic）→ `unwrap_or_else(|_| { eprintln!("友好提示"); std::process::exit(1); })`
- 占位符 `let _ = var;` → 变量名前缀 `_`（如 `_port_num`）或直接删除并加注释

### 🔴 规则 1（最高优先级，2026-07-06 追加）：扩展空间视为未实现

> 保留的功能扩展空间视为未实现功能需要进行实现。

**强制执行要求**：
- 所有"功能扩展空间"、"预留扩展点"、"未来支持"等标注的内容视为未实现功能
- 不能以"为未来扩展保留"为由推迟实现，必须在本批次真实实现
- 所有 `// TODO: 未来支持 xxx` / `// 预留扩展点` / `// 功能扩展空间` 标注必须立即实现

### 🔴 规则 2（最高优先级，2026-07-06 追加）：完全实现等于完整实现

> 对于未完全实现功能进行完全实现，完全实现等于完整实现。100%跟规划一致。

**强制执行要求**：
- 所有"部分实现"、"基础实现"、"占位实现"必须升级为完整实现
- 实现的完整度必须 100% 与规划文档一致，不允许 80%、90% 等部分实现
- 验收标准：对照规划文档逐项核对，任何偏差均视为未完成
- 完全实现 = 完整实现：功能闭环 + 数据闭环 + 错误处理闭环 + 测试闭环

### 🔴 规则 3（最高优先级，2026-07-06 追加）：项目成品导入/导出文档格式

> 项目成品使用 .xlsx 文档和 .docx 文档。
> （注：此处指项目的导入/导出功能所产生的文档格式，不是项目内部规则记忆文档格式）

**强制执行要求**：
- 所有数据导出功能（如线索导出、商机导出、库存导出、报表导出等）必须支持 .xlsx 格式（Excel）
- 所有报表/文档生成功能（如合同、发票、报表等）必须支持 .docx 格式（Word）
- 所有数据导入功能必须支持 .xlsx 格式（Excel）
- 禁止使用 CSV 作为最终交付格式（CSV 可作为内部调试格式，但面向用户的导出必须是 .xlsx）
- 禁止使用 .txt / .rtf / .html 等非标准格式作为成品文档
- 后端必须引入 xlsx/docx 生成库（如 rust_xlsxwriter / docx-rs）统一管理文档生成

### 🔴 规则 4（最高优先级，2026-07-06 追加）：项目文件 /// 注释精简规则

> 尽可能使用一句话描述项目的 `///` 注释，最好在 1 行，最多 2 行必须写完。
> 对发现的不符合规则的注释进行修正。

**强制执行要求**：
- 所有 `///` 文档注释必须精简为 1 行（首选），最多 2 行（复杂场景）
- 禁止 3 行及以上的 `///` 注释块（除 struct/enum 字段逐字段注释外）
- 修改代码时同步修正该文件内发现的不合规注释
- 注释内容使用中文，描述"做什么"而非"怎么做"

### 🔴 规则 5（最高优先级，2026-07-08 追加，2026-07-10 批次 262 修订）：E2E 测试加强

> 每 30 个批次运行一次 E2E 测试（独立工作流），不阻塞主 CI；
> 按 20/28/29 批次节奏监控，未完成则跳过下一个 E2E 周期。

**强制执行要求**（2026-07-10 批次 262 修订）：
- E2E 测试已从 `ci-cd.yml` 独立到 `.github/workflows/e2e-batch.yml`，不阻塞主 CI
- 每 30 个修复批次（如批次 270、300、330……）通过 `workflow_dispatch` 触发独立 E2E 工作流
- E2E 工作流独立编译后端（cargo build --release），不依赖 ci-cd.yml artifact
- E2E 测试完成后下载 `e2e-report-batch-N` artifact，生成 E2E 测试报告文档（保存到 `.monkeycode/docs/audits/`）
- 报告需包含：通过/失败用例数、失败用例清单、失败原因分类、修复优先级评估
- 报告中发现的 E2E 失败必须结合规则 0/1/2 进行修复和评估：占位符/未接入功能导致的失败必须真实接入
- E2E 报告中的问题按 P0/P1/P2 优先级纳入后续批次修复队列

**E2E 监控节奏**（2026-07-10 批次 262 修订，不实时监控）：
- 批次 N（30 倍数）：触发 e2e-batch.yml workflow_dispatch（action: create，batch_number=N）
- 批次 N+20：第 1 次监控 — GitHub API 查询 E2E run 状态（action: get）
  - 若已完成：下载报告分析，无需后续监控
  - 若未完成：继续推进修复批次，等 N+28 再查
- 批次 N+28：第 2 次监控 — 再次查询
  - 若已完成：下载报告分析
  - 若未完成：等 N+29 最后一次查询
- 批次 N+29：最后监控 — 查询 E2E run 状态
  - 若已完成：下载报告分析
  - 若仍未完成：跳过 N+30 的 E2E 周期（触发 e2e-batch.yml 时填 skip_reason 参数，执行 e2e-skipped job 记录跳过原因）
- **禁止死等 E2E 完成**：监控点之间正常推进修复批次，E2E 在后台异步运行

### 🔴 规则 6（最高优先级，2026-07-08 追加）：测试 mock 数据禁止硬编码

> 测试使用的 mock 数据禁止硬编码在测试用例中。

**强制执行要求**：
- 所有单元测试 / E2E 测试中的 mock 数据（用户信息、订单数据、API 响应等）必须抽取到独立的 fixtures / mock 数据文件（如 `tests/fixtures/`、`e2e/fixtures/`）
- 测试用例中禁止直接内联写死 mock JSON / 字段值，必须引用 fixtures 文件中的具名常量或工厂函数
- mock 数据文件需按业务域组织（如 `fixtures/sales.ts`、`fixtures/user.ts`），并附中文注释说明数据用途
- 工厂函数优先：对需动态生成的 mock 数据（如带时间戳的字段），使用工厂函数 `createXxxMock(overrides?)` 而非硬编码字面量
- 修复现有测试时同步迁移硬编码 mock 数据到 fixtures（违反规则 0/2 的占位符式写法）

### 🔴 规则 7（最高优先级，2026-07-08 追加）：禁止简洁方案

> 禁止简洁方案，只需要最合理、最准确、最符合项目功能实际需求的方案。

**强制执行要求**：
- 所有修复/实现必须采用最合理、最准确、最符合项目功能实际需求的方案，禁止为了"快速通过 CI"而采用简化方案
- 简洁不等于正确：宁可多写完整代码实现真实功能，也不写简化占位代码
- 类型清理、错误修复、功能实现均需按完整业务语义进行，不允许"看起来能过 CI 就行"的取巧写法
- 与规则 0/1/2 一致：所有方案必须服务于真实业务需求

### 🔴 规则 8（最高优先级，2026-07-08 追加）：真实实现强制（功能/接口/路由/api）

> 项目功能/接口/路由/api 等必须进行真实实现。

**强制执行要求**：
- 所有功能、接口、路由、API 必须真实实现，禁止占位、stub、mock 返回
- 与规则 0 形成双重强制：不仅是预留 API 要接入，所有新增/现有功能均需真实实现
- 任何"未实现"标记必须立即转为真实实现，不允许以"待后续迭代"为由推迟

### 🔴 规则 9（最高优先级，2026-07-08 追加）：个人规则高于项目规则

> 个人规则高于项目规则。

**强制执行要求**：
- 当个人规则与项目规则冲突时，以个人规则为准
- 个人规则包括本文件中的规则 7-12 及后续追加的个人规则
- 项目规则指 project_rules.md 中的规范

### 🔴 规则 10（最高优先级，2026-07-08 追加，2026-07-10 修正）：记忆文件定期整理归档

> 每 15 个批次，必须整理、归档、排序现有所有记忆，确保项目记忆高效简洁。

**强制执行要求**（2026-07-10 用户修正，原"记录梳理时间"过于表面）：
- 每完成 15 个修复批次（如批次 195、210、225……），必须对 `.monkeycode/` 下所有记忆文件做深度整理
- **整理**：梳理现有记忆内容，去重、去冗余、删除过期信息，合并重复条目
- **归档**：将已完成的历史批次、过期内容迁移到 `docs/archives/` 目录，主文件只保留当前活跃任务
- **排序**：重新组织记忆文件结构，按逻辑分类（规则→当前任务→历史→经验→规范），使其层次清晰
- **高效简洁**：主文件（MEMORY.md/doto.md/CHANGELOG.md）保持精简，避免历史细节堆积，确保代理能快速定位关键信息
- 梳理后需在 MEMORY.md 中记录整理时间和批次范围
- 与规则 5（每 30 批次 E2E 独立工作流）配合：批次 270 同时触发 E2E 报告 + 记忆整理

**整理记录**：
- 2026-07-14（批次 397 后，轻量整理）：批次 397 完成 v14 低风险修复首批（PR #571 已合并 CI 全绿）；**阶段 8 启动**；占位符/Mock 存根 21 项调研确认已清零（历史批次 290-308 已修复，代码库无 todo!()/unimplemented!() 宏调用，mock/stub 仅存在于 ElasticClient 双模式合法架构，9 处 TODO(tech-debt) 均为合理技术债务标注如 parking_lot 迁移/预留字段/i18n 覆盖率）；实际修复 4 处 unwrap_or_default 安全隐患：①omni_audit.rs:85 请求体读取失败时 warn 日志而非静默回退空字节 ②omni_audit.rs:152 响应体读取失败同上 ③audit_enhanced_handler.rs:111 created_at 改为 Option<String>（None 序列化为 JSON null 而非空字符串）④data_permission_handler.rs:89 序列化失败时 fail-fast 返回错误（避免跳过 SQL 注入安全检查）；trace_id 作用域修复（提前定义到请求体读取之前）；更新 doto.md 进度总览（96 完成 / 282 剩余，v14 低风险 4/74）+ CHANGELOG.md 追加批次 397 记录；下一批次 398：项目规则符合性评估（11 项）；下一个整理点批次 405
- 2026-07-14（批次 396 后，轻量整理）：批次 396 完成 baseline 警告清零收官（PR #570 已合并 CI 全绿 sha e0b0b5c）；**阶段 7 baseline 清零全部完成**（213/213 ✅）；修复 6 文件 7 类警告：①`.clippy.toml` 移除 disallowed-methods 配置（println/eprintln 是宏非方法，clippy 1.94 报 "does not refer to a reachable function"；改用 disallowed-macros 会触发 100+ 处 CLI 工具合法使用的新警告）②`process_state_machine.rs` inherent `from_str` 方法改为标准 `FromStr` trait 实现（消除 `should_implement_trait` 警告，调用方改用 `x.parse::<T>()`）③`purchase_delivery_calculator.rs` 删除未使用的 `AvgLeadTimeResult` struct + `FromQueryResult` 导入（dead_code，`get_supplier_avg_lead_time` 已用手动 `try_get_by_index` 实现）④`unwrap_safe.rs` 移除测试模块多余 `use super::*;`（宏通过 `#[macro_export]` 在 crate 级别导出，不依赖 super::*）⑤`middleware/auth.rs` 修复 `needless_borrow`（`&header_val`→`header_val`，已是 `&str`）⑥`webhook_service.rs` 修复 `needless_borrow`（`url::Url::parse(&url)`→`url::Url::parse(url)`）⑦too_many_arguments 警告经调研为过时 baseline 数据（当前所有函数均为 7 参数不算 &self，CI 重跑后自动消失）；更新 doto.md（阶段 7 标记完成 + 阶段 8-10 批次号前移 28 批：阶段 8 397-407 / 阶段 9 408-410 / 阶段 10 411+）+ CHANGELOG.md 追加批次 396 记录；下一阶段：阶段 8 v14 低风险修复（批次 397-407，74 项）；下一个整理点批次 405
- 2026-07-14（批次 395 后，轻量整理）：批次 395 完成 baseline 自动刷新机制（PR #568+#569 已合并 CI 全绿）；**阶段 7 baseline 清零首批完成**；关键技术点：baseline 文件过时（前 3 个高警告文件 bi_analysis_service/incoterms/password_policy_service 的 baseline 条目对应代码项已全部接入业务链路，非真实 dead_code）、CI clippy job 添加 main 分支自动刷新步骤（FIXED_COUNT>0 且 NEW_COUNT=0 时用当前警告替换 baseline 并提交）、shallow clone 下 `git log --all` 看不到历史提交需改用 `git ls-files --error-unmatch`、环境变量通过 `$GITHUB_ENV` 在步骤间传递、baseline 从 1465 行缩减到 310 行（摘要 213→7 条，移除 206 条已修复警告）；剩余 7 条真实警告（eprintln/println 不可达 + from_str 命名冲突 + AvgLeadTimeResult 未构造 + 不必要引用 + 函数参数过多 + unused import super::*）；更新 doto.md 进度总览（95 完成 / 283 剩余，baseline 警告清零 212/213）+ CHANGELOG.md 追加批次 395 记录；下一批次 396：处理剩余 7 条 baseline 警告；下一个整理点批次 405
- 2026-07-14（批次 394 后，轻量整理）：批次 394 完成测试覆盖补测第三批（data_permission_handler 0→6 SQL 注入防御 + print_handler 0→5 内置模板 + system_update_handler 0→6 ZIP 头校验+DTO + color_card/error_map 0→6 错误映射 14 变体，共 23 个新测试，PR #567 已合并 CI 全绿）；**阶段 6 测试覆盖补测全部完成**（批次 392-394 共 65 个新测试：service 42 + handler 23）；关键技术点：handler 私有纯函数必须用内嵌 `#[cfg(test)] mod tests` 测试（不能放 tests/ 目录）、validate_custom_condition_safe 白名单校验（字段名 `^[a-z_][a-z0-9_]{0,63}$` + FORBIDDEN 关键字列表）、verify_zip_magic 检查 `[0x50,0x4B,0x03,0x04]` 前缀、builtin_print_templates 返回 6 种单据类型静态模板、AppError 实现 Display 可用 `err.to_string()` 断言；更新 doto.md 进度总览（94 完成 / 284 剩余，测试覆盖补测 12/12 ✅ 全部完成）+ CHANGELOG.md 追加批次 394 记录；下一阶段：阶段 7 baseline 清零（批次 395-424，约 202 项）；下一个整理点批次 405
- 2026-07-14（批次 393 后，轻量整理）：批次 393 完成测试覆盖补测第二批（inventory_stock_service 0→6 + voucher_service 29→33 + ar_service 0→6 + ap_invoice_service 2→10，共 24 个新测试，PR #566 已合并 CI 全绿）；阶段 6 service 测试全部完成（批次 392-393 共 42 个新测试）；关键技术点：DualUnitConverter::meters_to_kg 公式（米×克重×幅宽(m)÷1000）、AP 状态机门（approve 仅 DRAFT / mark_as_paid 仅 AUDITED+PARTIAL_PAID P0 3-3 修复 / cancel 同白名单）、AR 收款核销贪心匹配（按发票顺序 min(remaining,unpaid)）、账龄分桶 6 区间（未到期/1-30/31-60/61-90/91-180/180+）、五维 ID 拼接（BATCH|COLOR|DYE_LOT|GRADE|WORKSHOP）；更新 doto.md 进度总览（93 完成 / 285 剩余，测试覆盖补测 7/12）+ CHANGELOG.md 追加批次 393 记录；下一批次 394：handler 集成测试；下一个整理点批次 405
- 2026-07-14（批次 392 后，轻量整理）：批次 392 完成测试覆盖补测首批（user_service 8 测试 + auth_service 4 异步密码 + po/order 6 状态校验门，共 18 个新测试，PR #565 已合并 CI 全绿）；更新 doto.md 进度总览新增"测试覆盖补测"维度（3/12 完成）+ CHANGELOG.md 追加批次 392 记录；下一批次 393：库存/财务 service 测试（inventory_stock/voucher/ar/ap）；下一个整理点批次 405
- 2026-07-14（批次 391 后，轻量整理）：批次 391 完成 useTableApi-6/7（AdjustmentListTab + TransferListTab 接入 useTableApi，PR #564 已合并 CI 全绿）；阶段 5 useTableApi 接入全部完成（批次 390-391 共 4 文件）；更新 doto.md 进度总览（88 完成 / 290 剩余，v13 前端/后端 P2 9/9 完成）+ CHANGELOG.md 追加批次 391 记录；下一阶段：阶段 6 测试覆盖补测（批次 392-394）；下一个整理点批次 405
- 2026-07-14（批次 390 后，轻量整理）：批次 390 完成useTableApi-8/9（assistAccounting + barcodeScanner 0-based 分页 bug 修复，PR #563 已合并 CI 全绿）；更新 doto.md 进度总览（86 完成 / 293 剩余）+ CHANGELOG.md 追加批次 390 记录 + doto.md 批次 390 表格补全实际完成状态 + 批次 391 调整为剩余 view 扫描；下一个整理点批次 405
- 2026-07-13（批次 389 后，轻量整理）：确认记忆文件结构清晰（MEMORY.md 697 行 / doto.md 582 行 / CHANGELOG.md 207 行），无冗余条目需清理；更新 doto.md 进度总览（85 完成 / 293 剩余）+ CHANGELOG.md 追加批次 389 记录；批次 390 进入 useTableApi 接入阶段
- 2026-07-13（批次 383 后）：合并 MEMORY.md 重复规则（规则 13 与用户习惯章节合并）+ 精简规则 15 检查清单 + 归档历史经验 + CHANGELOG.md 每条精简为一句话按阶段分段 + doto.md 删除归档批次详细记录重组四章节结构

### 🔴 规则 11（最高优先级，2026-07-08 追加）：法律合规标准

> 项目需要符合法律合规标准。

**强制执行要求**：
- 所有功能实现必须符合中国法律法规（项目主要面向中国市场）
- 数据处理符合《个人信息保护法》《数据安全法》《网络安全法》
- 用户隐私数据（手机号、身份证、邮箱等）的存储、传输、展示需合规
- 日志中禁止记录敏感信息明文（密码、token、身份证号等）
- 数据导出需支持数据脱敏和审计追溯
- 用户协议、隐私政策等合规文档需在系统中真实接入

### 🔴 规则 12（最高优先级，2026-07-08 追加）：法律安全标准

> 项目需要符合法律安全标准。

**强制执行要求**：
- 所有 API 必须进行身份认证和权限校验（除明确的公开端点外）
- 密码存储必须使用强哈希算法（bcrypt/argon2），禁止明文或弱哈希
- SQL 查询必须使用参数化查询，禁止字符串拼接（防 SQL 注入）
- 所有用户输入必须进行验证和清理（防 XSS、CSRF）
- 敏感操作（删除、修改、导出）必须记录审计日志
- JWT token 需设置合理过期时间，refresh token 需支持撤销
- 文件上传需校验类型、大小、内容，防止恶意文件
- 接口需做速率限制，防止暴力破解和 DDoS

### 🔴 规则 13（最高优先级，2026-07-11 追加）：修复流程自动化与连续执行

> 修复任务按批次连续执行，CI 全绿后自动开始下一批，直到所有任务完成。用户其他指令优先处理，处理完后继续修复流程。

**修复流程（每个批次必须严格按此执行）**：
1. **评估**：读取相关代码，评估修复方案和分页兼容性（useTableApi 是 1-based）
2. **建分支**：`git checkout -b fix/batchNNN-描述`（从最新 main 拉取）
3. **实现**：修改代码，遵循编码规范（中文注释、无硬编码、无死代码）
4. **提交**：`git commit`（heredoc 格式中文 commit message）+ `git push origin 分支`
5. **创建 PR**：GitHub API `POST /repos/57231307/1/pulls`（head: 修复分支, base: main）
6. **监控 CI**：`GET /commits/{sha}/check-runs`，等待 15 项全绿（13 success + 2 skipped 为正常）
   - 失败时：拉取 annotations → 修复 → 重新 push → 继续监控
7. **合并**：CI 全绿后 `PUT /pulls/{num}/merge`（squash 方式）
8. **清理**：`git checkout main && git pull && git branch -D 分支`
9. **记忆**：更新 doto.md（进度）+ CHANGELOG.md（一句话总结）+ 推送 main
10. **自动继续**：立即开始下一批次（回到步骤 1），无需用户确认

**连续执行规则**：
- CI 全绿后**自动**开始下一批次，不等用户指令
- 遇到用户其他指令时：**优先完成用户指令**，完成后**继续**修复流程（从断点恢复）
- 所有批次完成后进行**全项目复审**，复审问题按同样流程修复，直到复审无问题
- 每批次修复 1-3 个文件（view 表格迁移每批 2-3 个）
- 批次编号连续递增（不跳号）

**CI 失败处理**：
- 类型检查失败：拉取 annotations 定位错误 → 修复 → 重新 push
- Clippy 失败：通常是 dead_code 或未使用变量 → 按 doto.md 第六章死代码规范处理
- 测试失败：分析失败用例 → 修复代码或测试 → 重新 push
- 禁止 `|| true` / `unwrap_or_default()` 掩盖错误（违反规则 0）

**禁止**：
- 禁止积累多批未验证的修改
- 禁止本地 `cargo build` / `cargo test` / `npm run build` 等任何构建命令
- 禁止 `git push --force` 到 main / test 分支
- 禁止跳过 CI 直接合并（必须等 12 项必检全绿）

### 🔴 规则 14（最高优先级，2026-07-12 追加）：移除所有警告抑制，所有警告视为错误

> 移除所有的警告抑制，所有的警告视为错误，需要进行修复。不允许通过 `#[allow(...)]` 抑制警告来绕过修复。

**强制执行要求**：
- **禁止新增警告抑制**：新代码不允许使用 `#[allow(...)]` 抑制 clippy / rustc 警告
  - 禁止：`#[allow(dead_code)]`、`#[allow(unused_imports)]`、`#[allow(unused_variables)]`、`#![allow(...)]` 文件级抑制
  - 唯一例外：`backend/src/models/` 下 SeaORM 自动生成模型可保留 `#![allow(dead_code)]`（见项目规则第六章）
- **现有警告抑制清理**：现有 `#[allow(...)]` 标注需逐个评估并移除
  - 评估流程：检查被抑制的警告是否已消除 → 移除 `#[allow(...)]` → 验证 CI 全绿
  - 如果警告仍然存在：修复根本原因（删除死代码 / 接入业务 / 修复类型）而非抑制
- **clippy baseline 渐进清理**：现有 baseline 警告需逐步清理
  - 每批次修复至少清理 5 个 baseline 警告
  - 新增警告 = CI 失败（已有机制，baseline 严格化）
  - 目标：baseline 警告数归零，最终移除 baseline 机制改为 `cargo clippy -- -D warnings`
- **CI 强制**：
  - `.github/workflows/ci-cd.yml` 的 clippy 检查使用 baseline 机制判定新警告
  - `backend/.clippy.toml` 开启 `dead_code`/`unused_imports`/`unused_variables` 等警告
  - 任何新增警告或 `#[allow(...)]` 抑制都会让 CI 失败，必须立即修复

**修复模式参考**：
- `#[allow(dead_code)]` + 未使用函数 → 删除函数 或 接入业务调用
- `#[allow(unused_imports)]` → 删除未使用的 import
- `#[allow(unused_variables)]` → 使用变量 或 删除变量 或 前缀 `_`
- `#![allow(dead_code)]` 文件级 → 逐项评估，移除或接入业务（models/ 例外）

### 🔴 规则 15（最高优先级，2026-07-13 追加）：v13 复审严格规范 + 业务/财务/运行逻辑闭环

> v13 复审严格按照规矩进行复审，所有现存 baseline 警告视为错误需全部修复；额外增加运行逻辑环流程闭环复审维度；复审完自动开始修复，无需用户确认。

**强制执行要求**：

#### 一、v13 复审严格规范

1. **复审维度扩展**（在 v8-v12 复审基础上新增）：
   - **基础维度**：clippy baseline 警告清零（213 条摘要行 / ~993 个警告）
   - **新增维度：业务场景闭环** — 业务全链路闭环（下单→履约→收付款→售后→报表），跨模块数据/状态/事件必须贯通
   - **新增维度：财务场景闭环** — 财务全链路闭环（凭证→科目→账簿→报表→对账→结账），业财一致性 + 双向可追溯
   - **新增维度：运行逻辑环流程闭环** — 所有业务流程必须形成闭环（输入→处理→输出→反馈→输入）
   - **新增维度：异常路径闭环** — 所有错误处理必须有恢复/降级/告警路径，禁止吞错
   - **新增维度：状态机闭环** — 所有状态机必须有终态，禁止悬挂中间态
   - **新增维度：资源生命周期闭环** — 所有资源（连接/文件/锁/事务）必须有显式释放路径
   - **新增维度：配置依赖闭环** — 所有配置项必须在 .env.example → config.yaml → main.rs 校验形成闭环

2. **复审流程严格化**：
   - 复审必须按维度逐项扫描，每个维度生成独立报告
   - 复审报告必须包含：问题描述 + 影响范围 + 修复方案 + 优先级 + 关联规则
   - 复审完成后**自动开始修复**，无需用户确认（规则 13 连续执行）
   - 修复按优先级分批：P0（阻塞）→ P1（高）→ P2（中）→ P3（低），每批 5-6 文件

3. **baseline 警告全部视为错误**（用户 2026-07-13 明确）：
   - 现存所有 baseline 警告归类为错误，必须全部修复
   - 不允许"baseline 中已有的警告可以保留"的思维
   - 修复完成后必须同步清理 baseline 对应条目
   - 目标：baseline 警告数 → 0，最终移除 baseline 机制

#### 二、业务/财务/运行逻辑闭环核心检查清单

详细检查清单见 [v13 复审报告](file:///workspace/.monkeycode/docs/audits/v13-review-2026-07-13.md)，核心要求：

- **业务全链路闭环**：下单→履约→收付款→售后→报表，数据流连贯无断点
- **财务全链路闭环**：凭证→科目→账簿→报表→对账→结账，业财一致性 + 双向可追溯
- **业财一致性**：销售出库→收入凭证+成本凭证；采购入库→存货凭证+应付凭证；生产领料→成本凭证；库存调整→差异凭证；收付款→核销凭证
- **业务事件闭环**：所有业务事件必须有发布者+订阅者，失败有重试+死信队列+告警，幂等性保证
- **异常路径闭环**：所有 `Result<T, E>` 必须有错误处理路径（禁止 `let _ =`），所有 `unwrap()`/`expect()` 必须有上下文
- **状态机闭环**：所有枚举状态必须有终态，禁止"孤儿状态"，失败状态必须有恢复路径
- **资源生命周期闭环**：所有 `Arc<Mutex<T>>`/文件句柄/网络连接/数据库事务/定时任务必须有显式释放路径
- **配置依赖闭环**：环境变量必须在 `.env.example` 声明 + `main.rs` 校验 + `config.yaml` 注入，配置缺失必须 fail-fast

#### 三、v13 复审执行流程

1. **复审阶段**（自动执行）：
   - 扫描 clippy baseline 全部警告（按文件分组）
   - 扫描运行逻辑环流程闭环问题（按维度扫描）
   - 生成 v13 复审报告（保存到 `.monkeycode/docs/audits/v13-review-2026-07-13.md`）
   - 按优先级排序修复队列

2. **修复阶段**（复审完成后自动开始）：
   - 严格按规则 13 流程执行：建分支 → 修改 → commit → push → PR → CI → merge → 下一批
   - 每批 5-6 文件，CI 全绿后自动进入下一批，无需用户确认
   - 所有警告视为错误，必须真实修复（删除死代码 / 接入业务 / 修复类型）
   - 修复完成后同步清理 baseline 对应条目

3. **闭环验证**：
   - 每批修复后必须更新 doto.md 进度
   - 每批修复后必须更新 CHANGELOG.md 一句话总结
   - 所有批次完成后进行 v13 复审回归验证（确认无新增问题）

#### 四、v14 新一轮复审（v13 全部修复完成后触发）

> v13 复审阶段 1-9（批次 384-438）全部完成后，自动触发 v14 新一轮复审，新增 17 个审计维度。
> **核心特性：一个面料有多个颜色**（面料→颜色→缸号→批号四层级联关系），所有维度均围绕此核心特性展开。
> **最高准则：复用现有功能**，v14 复审的修复必须与现有功能优化结合，禁止重复实现类似功能。

**四层级联关系**（所有维度的基础）：
```
面料（Fabric/Product）
  └── 颜色（Color）              ← 一个面料有多个颜色
       ├── 色号（ColorNo）        ← 颜色唯一编码
       ├── 色卡（ColorCard）      ← 颜色实物样卡
       └── 缸号（DyeLotNo）       ← 每个颜色每批次染色有独立缸号
            ├── 批号（BatchNo）    ← 同一缸号下可分多个批次（匹号）
            ├── 库存数量          ← 按缸号/批号独立核算
            ├── 质检结果          ← 按缸号/批号独立检验
            ├── 成本单价          ← 按缸号/批号独立核算（实际成本）
            └── 库位              ← 按缸号分区存放
```

**关键业务约束**（必须强制执行）：
- **匹号唯一约束**：**同一缸号（dye_lot_no）下不能有相同的匹号（batch_no）**
  - DB 层：`UNIQUE(dye_lot_no, batch_no)` 唯一索引
  - Service 层：创建/更新时校验 (dye_lot_no, batch_no) 组合唯一
  - 全局适用：库存表、入库表、发货表、退货表、盘点表等所有含 batch_no 的表均需校验
  - 业务语义：一个缸号代表一次染色，同一缸内分多匹出布，每匹有独立匹号；匹号重复会导致库存和成本核算混乱
- **面料-颜色关联约束**：`UNIQUE(product_id, color_id)`
- **缸号-颜色关联约束**：`UNIQUE(color_id, dye_lot_no)`
- **库存四维标识**：库存唯一标识 = product_id + color_id + dye_lot_no + batch_no（联合唯一索引）

**复用现有功能原则**（v14 复审最高准则）：
- 修复前必须调研现有实现，禁止重复造轮子，优化而非新建
- 复用评估清单：现有 service/composable/component/utils/DB 表/API/事件是否已实现类似功能
- 共享工具优先：`utils/dual_unit_converter`、`utils/number_generator`、`utils/pagination` 等
- 前端 composable 优先：`useTableApi`、`useFormApi` 等已封装的 composable
- 后端 service 优先：`voucher_service`、`audit_log_service`、`event_bus` 等已封装的 service
- DB 模型扩展优先：新增字段优先扩展已有表，而非新建表
- API 契约兼容：修复时优先保持 API 返回格式兼容，避免破坏前端已有调用

**新增 17 个审计维度**（2026-07-13 用户追加，详见 [doto.md](file:///workspace/.monkeycode/doto.md) 阶段 10）：

**通用审计维度（3 项）**：
1. **业务功能完整性**：所有规划功能真实实现，无占位符/stub；功能闭环；CRUD 完整；批量操作；并发安全；幂等性
2. **逻辑完整性**：业务逻辑无断点；状态机闭环有终态；异常路径有恢复/降级/告警；分支覆盖完整；边界条件处理；事务边界正确
3. **数据流转性**：跨模块数据流转连贯；业财数据一致性；主数据变更同步关联单据；面料-颜色-缸号-批号数据全链路传递；双单位换算一致

**面料行业特性审计维度（7 项）**：
4. **面料行业转项特性优化**：色号/批号/缸号管理；**一个面料多颜色**（product↔color 多对一）；色差等级（A/B/C）；门幅/克重/纱支；双单位换算（米↔公斤）
5. **面料行业术语**：缸号/色号/批号/门幅/克重/纱支/染整/印花/色差/纬斜/缩水率；代码+注释+UI+DB+API 全一致；无外行表述
6. **面料行业流程完善情况**：打色→试样→大货→质检→入库→发货全流程；染整/印花工艺流程；面料检验十项指标；A/B/C 分级
7. **面料行业交易流程**：询价→报价→打样→大货→生产→发货→对账→收款；面料定价（按米/公斤/码）；色卡费/打样费独立计费
8. **面料行业调货流程**：厂际/仓际调拨；借条/还条；调货计价（成本/协议/市场价）；在途库存管理；调拨损耗处理
9. **面料行业专用词汇/术语配套使用**：缸号↔dye_lot_no、色号↔color_no、批号↔batch_no、门幅↔fabric_width、克重↔gram_weight、纱支↔yarn_count；术语在代码/DB/前后端一致配套
10. **面料行业业务流转**：接单→打色→采购→生产→染色→印花→质检→入库→销售→退货全链路；跨模块事件贯通；同一面料多颜色并行流转

**面料行业模块专项审计维度（7 项）**：
11. **面料行业人事管理**：岗位分类（染色工/印花工/质检员等）；三班倒考勤；计件工资（按米/公斤，A 级全额/B 级折扣/C 级不计）；绩效与产量挂钩
12. **面料行业仓库管理**：按色号/缸号分区入库；批次先进先出/指定批次发货；库存双单位；库位管理（染缸区/印花区/成品区）；盘点差异分类；库龄分析
13. **面料行业销售管理**：面料报价（含色卡费/打样费）；客户色卡管理；大货与打样关联；按缸号/批号退货追溯；信用额度按面料品类控制；色差纠纷处理
14. **面料行业公司管理**：多公司/多工厂架构；公司间调货/交易；五级组织架构；公司级工艺参数配置；独立核算+合并报表；产能管理
15. **面料行业权限细化**：按岗位/车间/面料品类/颜色授权；缸号级权限；敏感操作独立权限；分厂数据隔离；操作审计日志
16. **面料行业财务专项优化**：按缸号核算实际成本（染料/助剂/人工/水电分摊）；染整加工费归集；色卡/打样费用归集；批次成本估值；色差降级处理；染整税率（6%）与销售税率（13%）分开
17. **面料行业 CRM 专业优化**：客户面料+颜色偏好；客户色卡历史；打样记录追踪；报价历史；色差纠纷管理；复购预测（按面料+颜色）；客户价值分析

**v14 复审执行流程**：
1. 扫描 v13 复审全部维度（回归验证）+ 17 个新增维度
2. **重点核查"一个面料多颜色"核心特性在所有模块的实现**
3. **重点核查"同一缸号下匹号唯一"业务约束在所有含 batch_no 的表的实现**
4. 生成 v14 复审报告（保存到 `.monkeycode/docs/audits/v14-review-YYYY-MM-DD.md`）
5. 按优先级排序修复队列（P0 阻塞 → P1 高 → P2 中 → P3 低）
6. 自动开始修复，每批 5-8 文件，CI 全绿后自动进入下一批（规则 13）
7. 所有警告视为错误，必须真实修复（规则 14）
8. **复用现有功能原则**：修复前必须调研现有实现，禁止重复造轮子，优化而非新建
9. **匹号唯一约束**：所有涉及 batch_no 的修复，必须同步校验 (dye_lot_no, batch_no) 组合唯一
10. 完成后进行回归验证，通过后触发 v15 复审（如需）

### 规则 1-9（常规规则）

1. **CI/CD Only 验证**：禁止本地编译/构建。所有验证必须通过 CI/CD pipeline。
2. **每项修复 1 commit**：bug 修复按"每项 1 commit"原则，便于回滚和审计。
3. **多语言禁止**：项目所有文本必须使用中文（注释、用户界面、文档）。
4. **任务管理**：使用 TodoWrite 跟踪进度，状态实时更新。
5. **memory 优先**：每次操作前查看 MEMORY.md / doto.md / bug.md。
6. **关键变更必记录**：CHANGELOG.md 记录所有重要变更。
7. **公开端点收敛**：当前仅登录/刷新/健康检查可匿名访问（2026-06-25 优化）。
8. **~~租户隔离~~**（2026-06-28 已删除）：租户功能已完整删除，`extract_tenant_id` 函数、`AuthContext.tenant_id`、`AppClaims.tenant_id`、所有 tenant_id 列/字段/过滤/索引/管理表均已移除。项目不再支持多租户。
9. **批次迭代工作流**（2026-06-27 确认，2026-07-11 规则 13 细化）：每次修复批次完成后必须推送到 main 触发 CI 验证，CI 全绿后才继续下一批。流程详见规则 13。

---

## 二、文件定义

| 文件 | 用途 | 说明 |
|------|------|------|
| `MEMORY.md` | 项目规则记忆 | **只记录规则、规范、关键经验**，禁止写入任务相关内容 |
| `doto.md` | 未完成任务 | **只记录未完成任务**（任务队列、待修复项、进行中批次） |
| `doto-su.md` | 已完成任务归档 | **只记录已完成任务**（详细修改内容、技术要点、CI 验证） |
| `CHANGELOG.md` | 任务一句话总结 | **每个任务一句话摘要**（对应 doto-su.md 详细内容），禁止写入详细任务内容 |
| `bug.md` | 漏洞登记 | 实时检测与修复（修复后删除条目） |
| `docs/archives/` | 历史归档 | 已优化前的完整内容（按日期保留） |

---

## 三、基础规范

### 沟通语言
- 使用中文进行回复和沟通
- **简洁高效**：直接给方案和结果，避免冗长解释
- **进度可见**：使用 TodoWrite 跟踪，每完成一项立即标记
- **错误透明**：遇到失败立即报告，不掩盖

### 编码规范
- 禁止硬编码，所有文本需使用中文
- 代码注释必须使用中文

### 项目标识
- 项目名称统一（以 main 仓库 README 为准），所有文档/界面/输出信息一致

### 任务管理
- 使用中文建立待办任务（doto.md）
- 每完成一个待办任务，立即标记为"已完成"

### 记忆管理
- 实时查看和更新 `MEMORY.md` 规则记忆文档
- **三文件职责分工**（2026-07-10 用户明确修正）：
  - `MEMORY.md` = 项目规则，**禁止写入任务相关内容**（规则、规范、关键经验）
  - `doto.md` = 详细任务内容，**只记录任务及任务历史**（修改内容、技术要点、CI 验证等详细信息）
  - `CHANGELOG.md` = 任务一句话总结，**禁止写入详细任务内容**（每个任务仅一行摘要）
- **路径策略（2026-06-19 确认）**：test 分支合并入 main 时 `-X theirs` 会覆盖 `.monkeycode/`，必须以 main 版本为准

### 死代码与未使用文件处理
- **不使用的文件/代码/文件夹必须删除**（删除前评估影响范围，删除后更新受影响文件）
- 修改文件后保存前**必须交叉自审**（检查引用、配置、文档是否同步）
- **功能必须接入项目**（尽可能减少 TODO，禁止遗留占位代码）

### Bug.md 实时漏洞管理
- **实时检测** `.monkeycode/bug.md` 漏洞文件
- 发现漏洞 → 立即启动修复（按 P0/P1/P2 优先级）
- **修复一个漏洞后立即从 bug.md 删除对应条目**
- 所有漏洞修复完成后保留 `bug.md` **空文件**（不删除，作为漏洞登记占位）

### 数据库配置
- 数据库类型：PostgreSQL
- 连接方式：远程数据库连接模式

---

## 四、安全规范

### 敏感信息保护
- 禁止硬编码敏感信息（密码、密钥、令牌等）
- 使用环境变量或配置管理工具

### 输入验证
- 所有用户输入必须验证和清理
- 使用参数化查询防止 SQL 注入
- 对输出进行编码防止 XSS 攻击

---

## 五、CI/CD 强制

### 本地编译禁止
- **禁止**本地编译验证（`cargo build` / `cargo check` / `cargo test` / `cargo fmt -- --check` / `cargo clippy` / `npm run build` / `vue-tsc` / `pnpm typecheck` 等）
- **禁止**本地启动服务做端到端验证
- 所有验证走 GitHub Actions CI：修改代码 → commit → push → 监控 run → 失败拉 logs → 修复 → 重 push
- **唯一允许的本地操作**：文件 diff、语法、文本类（git status、cat、grep、sed、Edit、Write）

### CI 监控 API
- `/repos/{owner}/{repo}/commits/{sha}/check-runs` —— 查询 check run 状态
- `/repos/{owner}/{repo}/actions/runs/{id}/logs` —— 下载 logs zip
- `/repos/{owner}/{repo}/check-runs/{id}/annotations` —— 错误标注
- `/repos/{owner}/{repo}/actions/runs/{id}/jobs` —— 查询 job 列表

### 服务器环境
- 服务名称：bingxi-backend（systemd），安装目录：`/opt/bingxi-erp`
- 后端端口：8082，日志目录：`/opt/bingxi-erp/backend/logs`，备份目录：`/opt/bingxi-erp/backups`
- 环境配置：`/etc/bingxi-erp/.env`
- 部署命令：`bingxi update`（CLI 工具）
- 部署方式：CICD 构建 → GitHub Release → 手动部署到生产服务器
- **禁止** Docker 容器部署（不得创建 Dockerfile、docker-compose.yml）

### 部署限制
- 不安装 PostgreSQL 客户端（用远程数据库 39.99.34.194:5432）
- 不安装 Redis（用远程 Redis 服务器）
- 只需安装 Nginx、curl

### CI 验证偏好
- **10 项必检全绿即可合并**：环境信息 / 依赖图 / Rust 构建 / Rust Clippy / Rust 格式 / Rust 单元测试 / 前端构建 / 前端格式 / 前端 ESLint / 前端类型检查 / 前端测试 / 依赖审计
- **E2E 已独立**：E2E 测试从 ci-cd.yml 独立到 e2e-batch.yml（每 30 批次运行，不阻塞主 CI）
- CI 失败时：用 `/actions/runs/{id}/jobs` 查 job 列表 → `/actions/jobs/{job_id}/logs` 拉单 job 完整 log（Web UI 限 100KB）

---

## 六、核心经验（关键排错与开发经验）

### 集成测试跨 crate 调用私有函数
- `tests/` 目录下的集成测试编译为**独立二进制 crate**，`fn foo()` 对集成测试 crate 不可见
- 修复：`fn foo()` → `pub fn foo()`（或使用 `pub(crate)` 限制可见性）

### 沙箱网络限制
- **限制**：沙箱环境出站 22 端口（github.com SSH）被防火墙阻断
- **可用**：443 端口（github.com HTTPS）正常，包括 `git push` HTTPS 远程

### .monkeycode 目录 gitignore 规则
- `.gitignore` 默认忽略 `.monkeycode/`，仅白名单：`MEMORY.md` / `doto.md` / `bug.md` / `CHANGELOG.md`
- `.monkeycode/docs/` 子目录不在白名单
- **添加新归档文件**必须用 `git add -f` 强制添加

### Clippy Baseline 脆弱性
- `backend/.clippy-baseline.txt` 用 `comm -23` 精确行比较检测"新警告"
- 修改单行代码会导致 baseline 中后续行号全偏移，触发大量"假新警告"
- **修复**：删除 `backend/.clippy-baseline.txt`，让 CI 在 bootstrap 模式下重建
- **快速诊断**：CI 误报"大量新警告"时，先检查 baseline 首行内容

### Clippy Baseline 文件格式陷阱（批次 398 修复）
- **陷阱**：baseline 文件若包含完整渲染输出（代码片段、help、note 行），`grep -E '^(warning|error):'` 后只剩极少数摘要行
- **后果**：`comm -23` 比较时大量已存在警告被误判为新增（批次 398 修复前 274 行 baseline 只有 2 行摘要行，导致 116 条已存在警告被误判为新增）
- **正确格式**：baseline 文件**必须只含 `^(warning|error):` 开头的摘要行**，每行一条警告
- **重建方法**：从 CI 日志 `grep -E '^(warning|error):'` 提取纯摘要行，`sort -u` 去重后写入 baseline
- **验证**：重建后 baseline 行数应接近当前 clippy 警告摘要行数（批次 398 修复后 118 行）

### is_production() 部署陷阱（批次 398 修复）
- **陷阱**：`utils/config.rs::is_production()` 只读 `APP_ENV` 环境变量，不读 `AppSettings.env` 配置字段
- **后果**：config.yaml 设 `env: "production"` 但未设 `APP_ENV` 环境变量时，`is_production()` 返回 false，导致生产环境脱敏、Cookie Secure 等安全机制失效
- **修复**：`AppSettings::new()` 中 `load_sensitive_from_env()` 之后添加 APP_ENV 同步逻辑
  - APP_ENV 环境变量优先（已设置时不覆盖）
  - APP_ENV 未设置时从 config.yaml env 字段同步
- **配置优先级**：`APP_ENV` 环境变量 > `config.yaml` 的 `env` 字段 > 默认开发环境
- **部署建议**：生产环境同时在 .env 或 systemd Environment= 中显式设置 APP_ENV=production（双保险）

### systemd EnvironmentFile 路径一致性（批次 398 修复）
- **陷阱**：`deploy.sh` 的 `CONFIG_DIR` 与 systemd 服务文件的 `EnvironmentFile` 路径不一致
- **后果**：
  1. 清理 `/etc/bingxi/` 目录后重新部署时未重建该目录
  2. `cp /etc/bingxi-erp/.env /etc/bingxi/.env` 因目标父目录不存在而失败
  3. systemd 加载 EnvironmentFile 失败，后端二进制根本未被执行
- **修复**：所有部署脚本的 `CONFIG_DIR` 必须与 systemd 服务文件的 `EnvironmentFile` 路径一致
- **一致性检查清单**：
  - `deploy/deploy.sh`：`CONFIG_DIR="/etc/bingxi"`
  - `deploy/deploy-backend.sh`：`CONFIG_DIR="/etc/bingxi"`
  - `deploy/deploy-latest.sh`：`mkdir -p /etc/bingxi`
  - `deploy/bingxi-backend.service`：`EnvironmentFile=/etc/bingxi/.env`
- **关键教训**：部署脚本修改后必须用 `grep -r '/etc/bingxi' deploy/` 验证所有路径一致性

### SeaORM Trait 必导
- `Entity::find()` → 需 `use sea_orm::EntityTrait;`
- `.filter()` → 需 `use sea_orm::QueryFilter;`
- `.gte()/.lt()/.gt()/.lte()/.eq()` → 需 `use sea_orm::ColumnTrait;`
- `.count()/.all()/.paginate()` → 需 `use sea_orm::PaginatorTrait;`
- 清理 sea_orm trait 导入时**不能批量删**，必须**逐个静态验证**

### Clippy Lint 名规范
- rustc builtin lint：`unused_variables` / `unused_imports` / `dead_code`（不带 `clippy::` 前缀）
- clippy 内置 lint：`clippy::redundant_clone` / `clippy::too_many_arguments` 等
- `clippy::unused_variables` 是**无效 lint 名**，触发 `unknown_lints` 警告

### Validator 限制
- `#[validate(length(max = X))]` 只支持**整数字面量**
- 必须用：`length(max = 10_485_760)` ✅ 而非 `length(max = 10 * 1024 * 1024)` ❌

### GitHub Token 安全存储
- **绝不写入任何 git 跟踪文件**（.git/config / MEMORY.md / doto.md / CHANGELOG.md / commit message）
- **存储位置**：沙箱本地 `~/.git-credentials`（600 权限，git credential helper = store 自动读取）
- **沙箱网络限制**：SSH 22 端口被防火墙阻断，必须用 HTTPS push

### GitHub Actions Log 100KB 截断与详细日志获取
- **限制**：GitHub Web UI 的 CI run log 最多显示尾部 100KB
- **解决方案**：用 `https://api.github.com/repos/{owner}/{repo}/actions/jobs/{job_id}/logs` 获取**单 job 完整 log**

### Cargo build --release vs cargo test 编译差异
- 某些编译错误在 `cargo test`（dev build）中不会触发，但在 `cargo build --release`（`opt-level=2`）会触发
- **CI 防护**：依赖 `🏗️ Rust 后端构建` job 跑 `cargo build --release` 早期发现问题

### 子代理协作模式
- 大批量相似任务（如 40 个文件清理）使用 8 轮 × 5 个子代理的并行结构
- 子代理仅**编辑文件**，不直接推 PR；主代理汇总后开 1 个 PR
- 子代理不得操作 `.monkeycode/` 目录或 `CHANGELOG.md`（避免污染记忆）
- 子代理清理 sea_orm trait 导入时**必须**先 grep 使用点，再决定是否删除

> 更多历史经验（Cache::get 语义 / JTI 黑名单 Redis 迁移 / SSRF 双重校验 / DashMap vs Mutex / 日志脱敏 / totp-rs 熵源 / u16 永真比较 / 分布式限流回退 / `|| true` 反模式 等）已归档到 [docs/archives/](file:///workspace/.monkeycode/docs/archives/)。

---

## 七、工作流协作

### 工作角色定位
- 主代理角色：总控（项目经理/架构师）
- 子代理（Task 工具）= 员工，负责具体执行
- 主代理职责：分析任务 → 拆解 → 分配 → 总结成果 → 推 PR

### GitHub 分支策略
- `main` 为主分支（正式版），不允许删除，不允许 force push
- `test` 为测试分支，不允许删除
- 所有修复/功能变更在修复分支进行（`fix/batchN-简短描述`，合并后立即删除）
- 修复分支可 force push（仅 amend commit message 时，无协作影响）
- 验证后自动合并入 main

### 提交信息规范
- 使用中文编写提交信息，描述"做了什么"和"为什么"

### 代码审查
- 所有代码变更需经过审查
- 审查重点：代码质量、安全性、性能、测试覆盖

### 日志诊断技能自动触发
- 技能名：`/log-diagnosis` 日志诊断技能（自动触发）
- 触发关键词：日志、错误日志、异常日志、崩溃日志、服务器日志、traceId、错误码、异常堆栈
- 报告保存：`.diagnosis/reports/{YYYY-MM-DD}_{问题描述}.md`

---

## 八、代码规范

### 命名约定
- 使用有意义、描述性的名称
- 遵循项目或语言的命名规范
- 避免缩写和单字母变量（除约定俗成的，如循环中的 `i`）

### 代码组织
- 相关代码放在一起
- 保持适当的抽象层次
- 函数只做一件事，保持单一职责原则

### 注释与文档
- 注释解释"为什么"而不是"做什么"
- 为公共 API 提供清晰的文档
- 保持文档与代码同步更新

### 死代码处理规范
- **禁止**文件级 `#![allow(dead_code)]` 全局抑制（CI 会失败）
- **禁止**crate 级 `#![allow(unused_imports)]` / `#![allow(unused_variables)]`
- 真正未使用项**显式删除**（git 保留历史）；保留项加 `pub` 修饰或 `#[allow(dead_code)]` + TODO
- **例外**：`backend/src/models/` 下的 SeaORM 自动生成模型可保留文件级 `#![allow(dead_code)]`

### CI 死代码强制
- 配置：`backend/.clippy.toml` `warn` 段开启 `dead_code`/`unused_imports`/`unused_variables`
- 工作流：`.github/workflows/ci-cd.yml` `cargo clippy --all-targets -- -D warnings`
- 任何死代码警告都会让 CI 失败

---

## 九、性能与错误处理

### 数据库查询
- 优化查询，避免 N+1
- 使用适当索引
- 大数据量查询分页处理

### 缓存策略
- 合理使用缓存，明确失效策略
- 避免缓存过期数据

### 资源管理
- 及时释放不再使用的资源
- 避免内存泄漏
- 合理控制并发数量

### 错误处理
- 业务错误：返回友好提示
- 系统错误：记录详细日志，返回通用错误
- 验证错误：明确指出失败原因
- 尽可能实现优雅降级，提供重试机制

---

## 十、文档与持续改进

### API 文档
- 所有 API 接口必须有文档：接口路径、请求参数、响应格式、示例

### 代码文档
- 复杂逻辑必须有注释说明
- 公共函数必须有文档注释
- 保持文档与代码同步更新

### 持续改进
- 定期审查代码质量，及时重构
- 记录技术债务，制定偿还计划
- 关注新技术发展，定期团队分享

---

## 十一、归档索引

完整历史内容（整理前的详细记录）：

- 完整 MEMORY/doto/CHANGELOG/doto-su/bug（2026-07-13 整理前）：`.monkeycode/docs/archives/2026-07-13/`
- 完整 MEMORY/doto/CHANGELOG（2026-07-11 整理前）：`.monkeycode/docs/archives/2026-07-11/`
- 完整 MEMORY/doto/CHANGELOG（2026-07-10 整理前）：`.monkeycode/docs/archives/2026-07-10/`
- 完整 MEMORY/doto/CHANGELOG（2026-07-05 优化前）：`.monkeycode/docs/archives/2026-07-05/`
- 完整 MEMORY/CHANGELOG（2026-06-24 优化前）：`.monkeycode/docs/archives/`

历史审计报告：
- `.monkeycode/docs/audits/` 目录下保存历次复审报告（v5/v6/v7/v8/v9/v10/v11/v12/v13 等）
