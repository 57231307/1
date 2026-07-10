# 项目规则记忆

> 本文件是项目的**规则记忆**，记录必须遵守的规则、指令、偏好和工作流规范。
> 历史归档与详细内容请查阅 [`.monkeycode/docs/archives/`](file:///workspace/.monkeycode/docs/archives/)。

---

## 一、关键项目规则（必读，按优先级排序）

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

### 🔴 规则 5（最高优先级，2026-07-08 追加）：E2E 测试加强

> 每 10 个批次必须完整跑完 E2E 测试一次，并给出报告；
> 现有的所有问题需要结合报告进行修复和评估。

**强制执行要求**：
- 每 10 个修复批次（如批次 190、200、210……）必须完整跑完 CI 中的 `ci-e2e` job，不得因 `continue-on-error` 跳过分析
- E2E 测试完成后必须下载 `playwright-report` artifact，生成 E2E 测试报告文档（保存到 `.monkeycode/docs/audits/` 或 `e2e-reports/`）
- 报告需包含：通过/失败用例数、失败用例清单、失败原因分类、修复优先级评估
- 报告中发现的 E2E 失败必须结合规则 0/1/2 进行修复和评估：占位符/未接入功能导致的失败必须真实接入，不允许以"E2E 非阻塞"为由跳过
- E2E 报告中的问题按 P0/P1/P2 优先级纳入后续批次修复队列
- 禁止以"E2E 测试存在已知设计缺陷"为理由推迟修复（违反规则 0/2）

**E2E 监控与并行工作策略**（2026-07-08 用户指示补充）：
- E2E 是 10 批次跑一次，**禁止死等 E2E 完成**
- CI 中其他 11 项核心 job（环境信息/前端 ESLint/前端类型检查/Rust Clippy/Rust 格式检查/Rust 单元测试/前端格式检查/依赖审计/依赖图记录/前端测试/前端构建/Rust 后端构建）全部完成且全绿时，即可立即推进下一个修复批次
- E2E job 在后台异步运行，完成后回头下载报告分析
- 推进修复批次时，可顺带提交修复，新 push 会取消当前 E2E（可接受，E2E 会在下一次规则 5 节点重新完整跑完）

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
- 与规则 5（每 10 批次 E2E）配合：批次 200 同时触发 E2E 报告 + 记忆整理

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

### 规则 1-9（常规规则）

1. **CI/CD Only 验证**：禁止本地编译/构建。所有验证必须通过 CI/CD pipeline。
2. **每项修复 1 commit**：bug 修复按"每项 1 commit"原则，便于回滚和审计。
3. **多语言禁止**：项目所有文本必须使用中文（注释、用户界面、文档）。
4. **任务管理**：使用 TodoWrite 跟踪进度，状态实时更新。
5. **memory 优先**：每次操作前查看 MEMORY.md / doto.md / bug.md。
6. **关键变更必记录**：CHANGELOG.md 记录所有重要变更。
7. **公开端点收敛**：当前仅登录/刷新/健康检查可匿名访问（2026-06-25 优化）。
8. **~~租户隔离~~**（2026-06-28 已删除）：租户功能已完整删除，`extract_tenant_id` 函数、`AuthContext.tenant_id`、`AppClaims.tenant_id`、所有 tenant_id 列/字段/过滤/索引/管理表均已移除。项目不再支持多租户。
9. **批次迭代工作流**（2026-06-27 确认）：每次修复批次完成后必须推送到 main 触发 CI 验证，CI 全绿后才继续下一批。流程：修复 → commit → push → 监控 CI → 全绿后继续。禁止积累多批未验证的修改。

---

## 二、当前任务状态（2026-07-09 批次 237 启动 - v14 深度调研报告修复，从并发 async 阻塞开始）

> 用户最高优先级规则已在「一、规则 0-12」固化，本节仅记录修复进度。
> **规则 10 梳理记录**：
> - 2026-07-10 批次 255 梳理（=17×15 触发，上次梳理批次 236 提前触发）
> - 2026-07-09 批次 236 梳理（提前于批次 240 触发，因用户明确要求"梳理项目的所有记忆"）
> - 2026-07-09 批次 228 梳理（15×15+3 触发，上次梳理批次 195）
> **批次 243-255 已完成**：v14 中风险 13 项（安全漏洞 + 性能 + 空实现 + 简化阉割 + 死代码 + 重复实现首批 + 项目规则符合性），详见 doto.md 批次明细表。
> **v14 高风险 6 项全部完成**（批次 237-242，P0-1 到 P0-6）。
> **v13 后端 P0/P1 全部完成**（批次 229-236）。
> **v14 深度调研报告**（2026-07-09，[bug.md](file:///workspace/.monkeycode/bug.md)）：12 维度全量扫描，15 高/25 中/74 低风险，共 114 个问题。修复进度：高风险 6/6 完成，中风险 13/25 完成（剩余 12 项：测试覆盖 7 + 重复实现 service 分页 31/35 + view 表格 30+）。

### v14 深度调研报告修复（进行中 🔄，批次 237+）

> 报告位置：[bug.md](file:///workspace/.monkeycode/bug.md)（2026-07-09 12 维度全量扫描）
> 修复策略：按风险等级（高→中→低）+ 影响范围（核心路径→边缘功能）排序，每批 1 commit，CI 全绿后合并 main。

**v14 高风险问题修复队列（15 项，按优先级排序）**：

1. **并发-async 阻塞 P0-1**（4 处高，最高优先级，影响登录核心路径）✅ 批次 237 完成
   - `backend/src/services/auth_service.rs:107` authenticate
   - `backend/src/services/auth_service.rs:243` verify_password
   - `backend/src/services/auth_service.rs:277` hash_password
   - `backend/src/handlers/user_handler.rs:196/538/563/578` create_user/change_password
   - 修复方案：`tokio::task::spawn_blocking(move || ...).await??` 包装 Argon2id 哈希计算
   - 实际修复：新增 verify_password_async / hash_password_async 异步方法，7 处生产调用点全部改用异步版本（auth_service authenticate + user_handler 4 处 + init_service 2 处），同步版本保留供测试夹具使用
   - CI run #29023784549：12/12 核心 job 全绿（Clippy + 单元测试 + 后端构建均通过），PR #414 squash merge 到 main（commit 7585097f）

2. **性能-全表扫描 P0-2**（1 处高，ar_service.rs:1274-1321 get_aging_report）✅ 批次 238 完成
   - 无日期范围 + 无 LIMIT 全表扫描，数据量增长后可能 OOM
   - 修复方案：SQL 层聚合（CASE WHEN + SUM GROUP BY bucket）或加默认近 1 年日期范围 + LIMIT 上限
   - 实际修复：单条 SQL CASE WHEN + SUM + COUNT 在数据库层完成分桶聚合，应用层只接收 1 行聚合结果，O(N) 内存 → O(1) 内存
   - 规则 12 合规：customer_id 参数化绑定，禁止字符串拼接
   - CI 修复：1 轮（Values 类型冲突 + query_one 调用方式 + try_get_by_index turbofish），CI run #29025818891 12/12 核心全绿，PR #415 squash merge 到 main（commit 775f7761）

3. **空实现-业务失效 P0-3**（2 处高，前端查看按钮 handler 空）✅ 批次 239 完成
   - `frontend/src/views/dye-batch/index.vue:341` handleView
   - `frontend/src/views/dye-recipe/index.vue:318` handleView
   - 修复方案：实现查看逻辑（打开详情对话框或路由跳转）
   - 实际修复：新增 isView 只读模式标志，复用现有对话框实现查看功能（el-form :disabled + footer 按钮调整），两个文件一致
   - CI run #29026950380：12/12 核心 job 全绿，PR #416 squash merge 到 main（commit 743a9595）

4. **测试覆盖-安全核心 P0-4**（1 处高，permission.rs 全文件零测试）✅ 批次 240 完成
   - `backend/src/middleware/permission.rs` 权限校验零测试，越权风险
   - 修复方案：新增 `#[cfg(test)] mod tests`，覆盖管理员短路/缓存命中/过期/resource_id 精确匹配/`*` 通配符/嵌套路径
   - 实际修复：提取 matches_permission 纯函数 + 23 个单元测试（extract_resource_info 8 + method_to_action 6 + CacheEntry 2 + matches_permission 9 含垂直越权防护）
   - CI run #29028249081：12/12 核心 job 全绿，PR #417 squash merge 到 main（commit c72982b9）

5. **API 文档缺失 P0-5**（2 处高，openapi.rs 仅覆盖 8/115 handlers 7%）✅ 批次 241 完成
   - `backend/src/openapi.rs` 是未注册的幽灵文件（无 mod 声明），编译器看不到
   - `backend/src/docs.rs` 是占位文件（ApiDoc 已删除），导致 `#[cfg(feature = "swagger")]` 编译失败
   - `backend/src/routes/mod.rs:319-322` 引用 `crate::docs::ApiDoc::openapi()`，但 ApiDoc 不存在
   - 仅 2 个 handler 有 `#[utoipa::path]` 注解：auth_handler::login + health_handler::health_check
   - 修复方案：恢复 docs.rs ApiDoc（只注册有注解的 2 个 handler + 5 个 schema）+ 删除 openapi.rs 死文件
   - 实际修复：docs.rs 恢复 ApiDoc struct + impl Default + TODO 注释（后续迭代补全 handler 注解）
   - CI run #29029806479：12/12 核心 job 全绿（E2E 失败为已知问题不阻塞），PR #418 squash merge 到 main（commit de1437f0）

6. **简化阉割-永久 P0-6**（1 处高，crm/cust.rs:265-275 get_rfm_distribution）✅ 批次 242 完成
   - 返回全 0 占位 JSON，RFM 分布功能形同虚设
   - 修复方案：真实批量计算所有客户 RFM 评分并聚合分布
   - 实际修复：一次性查询所有客户 ID + 订单聚合（GROUP BY customer_id），内存计算 RFM 评分，分桶聚合（VIP>=4.5/重要>=3.5/一般>=2.5/低价值<2.5），提取 OrderAggRow/CustomerOrderStats type 别名避免 clippy type_complexity 警告
   - CI run #29031527941：12/12 核心 job 全绿（1 轮 CI 修复：type_complexity），PR #419 squash merge 到 main（commit 146251d9）

**v14 中风险问题修复队列（25 项，已完成 13/25 🔄）**：
- 测试覆盖（7 项，⏳ 待修复）：handlers 100+ 文件覆盖率 10%、services 107 个无测试、frontend api 4.4%、ai 算法零测试等
- 空实现（4 项，全部完成 ✅）：
  - ✅ 批次 246：dye-recipe handleViewVersion（复用主对话框只读模式，PR #423 commit 16754cf7）
  - ✅ 批次 252：bi_analysis_service 3 处 unreachable!() + dual_unit_converter_handler 1 处 unreachable!() 改为返回 AppError 错误，新增 6 个单元测试（PR #429 commit faa9749）
  - ✅ 批次 253：AdvancedFilter handleLogicChange 空函数改为真实实现，新增 logicChange emit 事件（PR #430 commit da659f7）
- 简化阉割（3 项，全部完成 ✅）：
  - ✅ 批次 249：capacity_service 硬编码置信度 0.8 改为动态计算（PR #426 commit 82269a4）
  - ✅ 批次 250：budget_management 跳过审批流改为完整审批闭环（PR #427 commit b2520cd）
  - ✅ 批次 251：webhook retry 未持久化 payload（新增迁移 m0047 + 持久化 + retry_count 修复，PR #428 commit 226af53）
- 死代码（1 项，全部完成 ✅）：✅ 批次 254：14 个 composable 文件 eslint-disable any 清理（PR #431 commit d2abb55）
- 重复实现（2 项，进行中 🔄）：
  - service 分页逻辑接入 paginate_with_total（首批 4/35 完成于批次 255，剩余 31/35）✅ 批次 255：sales_price/ap_invoice/role/supplier 接入，修复 role_service fetch_page 偏移 bug（PR #432 commit 026fcc3）
  - 30+ view 表格逻辑重复接入 useTableApi ⏳ 待修复
- 项目规则符合性（1 项，全部完成 ✅）：✅ 批次 247：cli/util/service.rs 硬编码健康检查 URL 改为环境变量读取（PR #424 commit 47d86d86）
- 性能问题（5 项，全部完成 ✅）：
  - ✅ 批次 244：ar_service 3 报表 SQL 聚合（PR #421 commit dcd8488d）
  - ✅ 批次 245：ap_report_service 4 方法 SQL 聚合（PR #422 commit ae7d4619）
  - ✅ 批次 248：AR/AP 报表 8 端点接入 CacheService 缓存（PR #425 commit 53ce6b53）
- 安全漏洞（2 项，全部完成 ✅）：✅ 批次 243：report-templates XSS + tracking_handler 输入验证（PR #420 commit 0810fe3）
- API 契约（0 项高风险，2 项低风险，后续迭代）

**v14 低风险问题修复队列（74 项，后续迭代）**：
- 占位符/Mock 存根（21 项，全部合理设计或测试夹具，多数无需修复）
- 项目规则符合性（11 项，多为配置层默认值或 best-effort 合理模式）
- 死代码（8 项，均合规标注）
- 其他（34 项）

### 历史复审进度摘要（v7-v13，已全部完成 ✅）

> 详细修复明细已归档到 [docs/archives/2026-07-10/](file:///workspace/.monkeycode/docs/archives/2026-07-10/)。

- **v7 复审**（批次 103-120）：webhook 真实接入 + 占位符清理 + failover/cache 模块删除
- **v8 复审**（批次 121-129）：event_kafka 删除 + ElasticClient 真实实现 + SearchSyncer 接入
- **v9 复审**（批次 130-135）：bi_analysis 16 方法真实接入 + purchase_inspection 4 明细 CRUD
- **v11 复审**（批次 143-196）：P0 三项 + P1 dead_code 58 处真实接入 + 前端 any 清理
- **v12 复审**（批次 197-228）：P0/P1 全部 + P2 状态字符串 82 处替换 + 342 个测试补测
- **v13 复审**（批次 229-236）：P0/P1 全部（warehouse stub + 状态常量 25 域 + N+1 重构）

**关键经验**：
- N+1 修复模式：读用批量 IN + HashMap，写用 insert_many，乐观锁保持逐条
- 跨文件 impl 块需谨慎评估（批次 121 教训：误删 report/ds.rs 导致 CI 失败）

详细历史：见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md) 与 [docs/archives/](file:///workspace/.monkeycode/docs/archives/)

---

## 三、文件定义

| 文件 | 用途 | 说明 |
|------|------|------|
| `MEMORY.md` | 项目规则记忆 | 规则、规范、关键经验（必须遵守） |
| `doto.md` | 任务与历史 | 当前任务 + 历史归档索引（实时更新） |
| `CHANGELOG.md` | 任务精简总结 | 任务一句话摘要列表（PR 完成后更新） |
| `bug.md` | 漏洞登记 | 实时检测与修复（修复后删除条目） |
| `docs/archives/` | 历史归档 | 已优化前的完整内容（按日期保留） |

---

## 四、基础规范

### 沟通语言
- 使用中文进行回复和沟通

### 编码规范
- 禁止硬编码，所有文本需使用中文
- 代码注释必须使用中文

### 项目标识
- 项目名称统一（以 main 仓库 README 为准），所有文档/界面/输出信息一致

### 开发辅助
- 每次新增或修改功能时，必须调用合适的技能或 MCP 工具
- 严格按照技能规范进行开发

### 任务管理
- 使用中文建立待办任务（doto.md）
- 每完成一个待办任务，立即标记为"已完成"

### 记忆管理
- 实时查看和更新 `MEMORY.md` 规则记忆文档
- 关键内容存储在 `MEMORY.md`，变更记录到 `CHANGELOG.md`
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

### 任务规划管理
- 所有任务规划文件保存在 `.monkeycode/docs/` 下

### 数据库配置
- 数据库类型：PostgreSQL
- 连接方式：远程数据库连接模式

### 功能实现依据
- 新增功能接口、数据库操作需遵循现有规范

### 打包与发布要求
- 打包时必须进行全面测试：功能测试、兼容性测试、稳定性测试

---

## 五、安全规范

### 敏感信息保护
- 禁止硬编码敏感信息（密码、密钥、令牌等）
- 使用环境变量或配置管理工具

### 输入验证
- 所有用户输入必须验证和清理
- 使用参数化查询防止 SQL 注入
- 对输出进行编码防止 XSS 攻击

---

## 六、CI/CD 强制

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

---

## 七、核心经验（关键排错与开发经验）

### 集成测试跨 crate 调用私有函数
- `tests/` 目录下的集成测试编译为**独立二进制 crate**，`fn foo()` 对集成测试 crate 不可见
- 修复：`fn foo()` → `pub fn foo()`（或使用 `pub(crate)` 限制可见性）
- 错误模式：`error[E0624]: associated function compose_color_no is private`

### 沙箱网络限制
- **限制**：沙箱环境出站 22 端口（github.com SSH）被防火墙阻断
- **可用**：443 端口（github.com HTTPS）正常，包括 `git push` HTTPS 远程
- **应对策略**：沙箱内可通过 HTTPS 完成 commit → push → CI 全流程

### .monkeycode 目录 gitignore 规则
- `.gitignore` 默认忽略 `.monkeycode/`，仅白名单：`MEMORY.md` / `doto.md` / `bug.md` / `CHANGELOG.md`
- `.monkeycode/docs/` 子目录不在白名单
- **添加新归档文件**必须用 `git add -f` 强制添加

### 集成测试 `crate` 语义
- `tests/` 目录下的集成测试编译为独立二进制，`crate` 关键字指向**测试二进制本身**
- 引用 lib.rs 暴露的模块必须用 `Cargo.toml` 中的 `name` 字段（连字符 `-` 转下划线 `_`），即 `bingxi_backend`
- 单元测试（`src/` 内的 `#[cfg(test)]`）中 `crate` 指向 lib，两者语义不同

### Clippy Baseline 脆弱性
- `backend/.clippy-baseline.txt` 用 `comm -23` 精确行比较检测"新警告"
- 修改单行代码会导致 baseline 中后续行号全偏移，触发大量"假新警告"
- **修复**：删除 `backend/.clippy-baseline.txt`，让 CI 在 bootstrap 模式下重建
- **快速诊断**：CI 误报"大量新警告"时，先检查 baseline 首行内容

### Cache::get 返回值语义
- `backend/src/utils/cache.rs` 的 `Cache` trait 定义 `fn get(&self, key: &K) -> Option<V>`，返回值已 **Clone**（不是 `Option<&V>`）
- 不能在结果上调用 `.copied()`（仅 `Option<&T>` 或迭代器支持）

### JTI 黑名单→Redis 迁移设计
- **现状**：`auth_service.rs` 用 `static JTI_BLACKLIST: LazyLock<RwLock<HashMap<String, i64>>>`，多实例不共享
- **迁移方案**：优先用 Redis SETEX（`SET key value EX <ttl>`），TTL 到期自动清理
- **失败回退**：Redis 不可用时降级到原 HashMap（避免阻塞业务）

### SSRF 防护双重校验必要性
- **单次校验的弱点**：create 时校验 `url` 指向公网，但攻击者可注册合法公网域名后修改 DNS 记录为内网 IP（DNS Rebinding）
- **必须双重校验**：`create_webhook` 时校验 + `trigger_webhook` 发送前**再次**校验
- **校验内容**：协议白名单 + 主机名黑名单 + IP 黑名单（RFC1918/loopback/link-local 含云元数据 169.254.169.254）

### DashMap vs std::sync::Mutex 选型
- 高频/性能关键：DashMap
- 安全关键 + 锁中毒需防御：std::sync::Mutex + try_lock
- **关键模式**：`let Ok(mut g) = self.storage.try_lock() else { return; };`（Rust 1.65+ let-else）

### 日志脱敏按字符而非字节
- **风险**：截断 UTF-8 字符串用字节切片 `&s[..n]` 可能切到字符中间，panic
- **正确做法**：用 `chars().take(n)` 按 Unicode 字符截断

### totp-rs 5.5 熵源确认
- `Secret::generate_secret()` 内部用 `rand::thread_rng()` → `OsRng` → 操作系统 CSPRNG
- **安全等级**：密码学安全（160 bits 熵，符合 RFC 4226 推荐）

### GitHub Token 安全存储
- **绝不写入任何 git 跟踪文件**（.git/config / MEMORY.md / doto.md / CHANGELOG.md / commit message）
- **存储位置**：沙箱本地 `~/.git-credentials`（600 权限，git credential helper = store 自动读取）
- **沙箱网络限制**：SSH 22 端口被防火墙阻断，必须用 HTTPS push

### GitHub Actions Log 100KB 截断与详细日志获取
- **限制**：GitHub Web UI 的 CI run log 最多显示尾部 100KB
- **解决方案**：用 `https://api.github.com/repos/{owner}/{repo}/actions/jobs/{job_id}/logs` 获取**单 job 完整 log**

### u16 永真比较与 Clippy 极端比较警告
- **触发模式**：`x >= 0xff00 && x <= 0xffff`（u16 类型，`<= 0xffff` 永远为真）
- **Clippy lint**：`absurd_extreme_comparisons`
- **通用规则**：写数值比较前先想"类型边界"

### 分布式限流回退必须真实回退
- 错误设计：`check_redis_rate_limit` 返回 `Ok(true)`（未配置 Redis），`check_rate_limit` 直接放行
- 正确设计：返回 `Result<Option<bool>>`：`Ok(Some(allowed))` / `Ok(None)`（应回退）/ `Err(_)`（应回退）

### Cargo build --release vs cargo test 编译差异
- 某些编译错误在 `cargo test`（dev build）中不会触发，但在 `cargo build --release`（`opt-level=2`）会触发
- **CI 防护**：依赖 `🏗️ Rust 后端构建` job 跑 `cargo build --release` 早期发现问题

### `|| true` 反模式
- `assert!(some_expr.is_ok() || true)` 是恒真式断言，无测试价值却能**掩盖编译错误**
- CI 中应使用 `cargo check --tests` 或 `cargo test --no-run` 提前发现编译错误

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

### 子代理协作模式
- 大批量相似任务（如 40 个文件清理）使用 8 轮 × 5 个子代理的并行结构
- 子代理仅**编辑文件**，不直接推 PR；主代理汇总后开 1 个 PR
- 子代理不得操作 `.monkeycode/` 目录或 `CHANGELOG.md`（避免污染记忆）

### 子代理 sea_orm 清理警示
- 子代理清理 sea_orm trait 导入时**必须**先 grep 使用点，再决定是否删除

---

## 八、工作流协作

### 工作角色定位
- 主代理角色：总控（项目经理/架构师）
- 子代理（Task 工具）= 员工，负责具体执行
- 主代理职责：分析任务 → 拆解 → 分配 → 总结成果 → 推 PR

### GitHub 分支策略
- `main` 为主分支（正式版），不允许删除
- `test` 为测试分支，不允许删除
- 所有修复/功能变更在修复分支进行
- 验证后自动合并入 main
- 修复分支合并后自动删除

### 提交信息规范
- 使用中文编写提交信息
- 描述"做了什么"和"为什么"

### 代码审查
- 所有代码变更需经过审查
- 审查重点：代码质量、安全性、性能、测试覆盖

### 日志诊断技能自动触发
- 技能名：`/log-diagnosis` 日志诊断技能（自动触发）
- 触发关键词：日志、错误日志、异常日志、崩溃日志、服务器日志、traceId、错误码、异常堆栈
- 报告保存：`.diagnosis/reports/{YYYY-MM-DD}_{问题描述}.md`

---

## 九、代码规范

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

## 十、性能与错误处理

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

## 十一、文档与持续改进

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

## 十二、用户习惯与协作偏好（2026-07-05 整理）

> 本章固化用户在历次会话中明确表达的工作习惯与协作偏好，作为代理行为的强制约束。

### 批次修复工作流（用户 2026-06-27 确认）

> 开始进行修复，按修复一个批次，推送 ci，ci 全绿,合并到 main，删除修复分支，
> 修复下一个分支，创建修复分支，修复下一个分支依次类推，直到所有任务全部修复完成。
> 修复完成后进行全项目复审，对复审出来的问题按这个流程进行继续评估修复。直到复审没有问题。

**强制流程**：
1. 创建修复分支 `fix/batchN-xxx`
2. 修改代码（严格遵守 CI/CD Only，禁止本地编译）
3. `commit`（中文 commit message，描述"做了什么"和"为什么"）
4. `git push -u origin fix/batchN-xxx`
5. 创建 PR（中文标题 + 中文 body，列出修复项）
6. 监控 CI（用 GitHub API 轮询 check-runs，12 项必检全绿即可合并，E2E 非阻塞）
7. squash merge 到 main（commit_title 带 `(#PR号)` 后缀）
8. 删除本地 + 远程修复分支
9. `git checkout main && git pull origin main` 同步
10. 更新 MEMORY.md / doto.md / CHANGELOG.md 记录完成
11. 开始下一批次

**禁止**：
- 禁止积累多批未验证的修改
- 禁止本地 `cargo build` / `cargo test` / `npm run build` 等任何构建命令
- 禁止 `git push --force` 到 main / test 分支
- 禁止跳过 CI 直接合并（必须等 12 项必检全绿）

### 沟通偏好

- **回复语言**：中文（代码注释也用中文）
- **简洁高效**：直接给方案和结果，避免冗长解释
- **进度可见**：使用 TodoWrite 跟踪，每完成一项立即标记
- **错误透明**：遇到失败立即报告，不掩盖

### 记忆管理偏好

- `.monkeycode/` 文件夹定期整理优化（用户 2026-07-05 明确要求）
- 主文件（MEMORY/doto/CHANGELOG）保持精简，早期内容归档到 `docs/archives/YYYY-MM-DD/`
- 用户习惯和新规则必须写入项目规则文件（MEMORY.md 一、章节），不能只留在对话上下文
- 关键变更必须实时记录到 CHANGELOG.md

### CI 验证偏好

- **12 项必检全绿即可合并**：环境信息 / 依赖图 / Rust 构建 / Rust Clippy / Rust 格式 / Rust 单元测试 / 前端构建 / 前端格式 / 前端 ESLint / 前端类型检查 / 前端测试 / 依赖审计
- **E2E 非阻塞**：前端 E2E 测试 `continue-on-error`，不阻塞合并
- CI 失败时：用 `/actions/runs/{id}/jobs` 查 job 列表 → `/actions/jobs/{job_id}/logs` 拉单 job 完整 log（Web UI 限 100KB）

### 分支策略偏好

- `main`：主分支（正式版），不允许删除，不允许 force push
- `test`：测试分支，不允许删除
- 修复分支：`fix/batchN-简短描述`，合并后立即删除
- 修复分支可 force push（仅 amend commit message 时，无协作影响）

---

## 十三、归档索引

完整历史内容（整理前的详细记录）：

- 完整 MEMORY/doto/CHANGELOG（2026-07-10 整理前）：`.monkeycode/docs/archives/2026-07-10/`
- 完整 MEMORY/doto/CHANGELOG（2026-07-05 优化前）：`.monkeycode/docs/archives/2026-07-05/`
- 完整 MEMORY/CHANGELOG（2026-06-24 优化前）：`.monkeycode/docs/archives/`

历史审计报告：
- `.monkeycode/docs/audits/` 目录下保存历次复审报告（v5/v6/v7 等）
