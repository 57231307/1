# 项目规则记忆（索引版）

> 本文件是项目的规则记忆索引，记录规则一句话核心 + 链接到 MEMORY-SU.md 详细说明。
> 规则自我迭代日志见 [MEMORY-SU §六](file:///workspace/.monkeycode/MEMORY-SU.md#六规则自我迭代日志)。

---

## 一、关键项目规则（必读，按功能域分组）

> 优先级：IR（个人规则）> PR（项目规则）> PH（项目习惯）> IH（个人习惯）。

### 1.1 实现完整性（PR）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 0** | 所有预留 API/功能/占位符/路由必须真实实现，禁止 stub/placeholder | [MEMORY-SU §规则 0](file:///workspace/.monkeycode/MEMORY-SU.md#规则-0真实实现强制pr2026-07-04-追加2026-07-17-合并规则-8) |
| 🔴 **规则 1** | 修改前必须评估关联影响，所有修改为代码级修改 | [MEMORY-SU §规则 1](file:///workspace/.monkeycode/MEMORY-SU.md#规则-1修改前关联影响评估强制pr2026-07-11-追加) |

### 1.2 代码质量与编码规范（PR）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 2** | 禁止警告抑制，注释与功能一致；编码前先思考、保持简单、精准修改、不过度推断、正确处理错误 | [MEMORY-SU §规则 2](file:///workspace/.monkeycode/MEMORY-SU.md#规则-2代码质量与编码规范pr2026-07-12-追加2026-07-17-合并规则-14202026-08-14-合并规则-19) |

### 1.3 测试与流程（PR）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 3** | 修复按批次连续执行，CI 全绿自动下一批；步骤 0 核实审计内容 + 步骤 4 推送前自审 | [MEMORY-SU §规则 3](file:///workspace/.monkeycode/MEMORY-SU.md#规则-3修复流程自动化与连续执行pr2026-07-11-追加) |
| 🔴 **规则 4** | 复审按规矩进行，baseline 警告视为错误，8 维度闭环 | [MEMORY-SU §规则 4](file:///workspace/.monkeycode/MEMORY-SU.md#规则-4复审严格规范pr2026-07-13-追加) |
| 🔴 **规则 5** | 每 30 批次 E2E 测试（独立工作流不阻塞主 CI） | [MEMORY-SU §规则 5](file:///workspace/.monkeycode/MEMORY-SU.md#规则-5e2e-测试加强pr2026-07-08-追加2026-07-10-批次-262-修订) |
| 🔴 **规则 6** | 测试 mock 数据禁止硬编码；功能变更必须同步测试代码 | [MEMORY-SU §规则 6](file:///workspace/.monkeycode/MEMORY-SU.md#规则-6测试规范pr2026-07-08-追加2026-08-14-合并规则-18) |

### 1.4 安全合规（PR）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 7** | 符合中国法律法规 + API 认证/权限/加密/审计 | [MEMORY-SU §规则 7](file:///workspace/.monkeycode/MEMORY-SU.md#规则-7法律合规与安全标准pr2026-07-08-追加) |

### 1.5 记忆与文档管理（PR）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 8** | 每 15 批整理归档 + 实时归档；MEMORY.md 只存规则，doto.md 只存未完成任务 | [MEMORY-SU §规则 8](file:///workspace/.monkeycode/MEMORY-SU.md#规则-8记忆文件管理pr2026-07-08-追加2026-07-10-修正2026-07-14-二次修正) |
| 🔴 **规则 9** | `.monkeycode/` 全目录强制追踪，禁止忽略任何文件 | [MEMORY-SU §规则 9](file:///workspace/.monkeycode/MEMORY-SU.md#规则-9monkeycode-全目录强制追踪pr2026-07-17-追加) |
| 🔴 **规则 10** | 审计计划/复审规则变更时，关联文档必须同步更新 | [MEMORY-SU §规则 10](file:///workspace/.monkeycode/MEMORY-SU.md#规则-10审计文档同步规则pr2026-07-17-追加) |
| 🔴 **规则 11** | 规则自我迭代机制（四分类 PR/PH/IR/IH + 6 条触发条件 + 自动记录） | [MEMORY-SU §规则 11](file:///workspace/.monkeycode/MEMORY-SU.md#规则-11规则自我迭代机制pr2026-07-17-追加) |

### 1.6 工具与运维（PR）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 12** | 工具连接异常分级响应（L1 60s / L2 60-180s / L3 30min 周期）+ 非阻塞推理 | [MEMORY-SU §规则 12](file:///workspace/.monkeycode/MEMORY-SU.md#规则-12工具连接异常重试策略pr2026-07-17-追加) |

### 1.7 PR 流程规范（PR）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 13** | PR 描述必须依据 `/.github/PULL_REQUEST_TEMPLATE.md` 模板填写 | [MEMORY-SU §规则 13](file:///workspace/.monkeycode/MEMORY-SU.md#规则-13pr-描述强制依据模板填写pr2026-07-29-追加) |

### 1.8 工作流规范（PR）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 14** | 产品图谱驱动工作流：先读规则与图谱、复用成熟方案、极简交付并同步沉淀 | [MEMORY-SU §规则 14](file:///workspace/.monkeycode/MEMORY-SU.md#规则-14产品图谱驱动工作流pr2026-08-11-追加) |

### 1.9 个人规则（IR，最高优先级）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 15** | 个人规则（IR）高于项目规则（PR）；优先级 IR > PR > PH > IH | [MEMORY-SU §规则 15](file:///workspace/.monkeycode/MEMORY-SU.md#规则-15个人规则高于项目规则ir-个人规则2026-07-08-追加) |
| 🔴 **规则 16** | 成品导入/导出使用 .xlsx/.docx，禁止 CSV/txt/rtf/html 作为成品 | [MEMORY-SU §规则 16](file:///workspace/.monkeycode/MEMORY-SU.md#规则-16项目成品导入导出文档格式ir-个人规则2026-07-06-追加) |
| 🔴 **规则 17** | 禁止简洁方案，采用最合理/最准确/最符合业务需求的方案 | [MEMORY-SU §规则 17](file:///workspace/.monkeycode/MEMORY-SU.md#规则-17禁止简洁方案ir-个人规则2026-07-08-追加) |

---

## 二、常规规则

- **每项修复 1 commit**：bug 修复按"每项 1 commit"原则，便于回滚和审计
- **公开端点收敛**：仅登录/刷新/健康检查可匿名访问，其他所有端点必须认证
- **多租户已删除**（2026-06-28）：所有 tenant_id 列/字段/过滤/索引/管理表均已移除
- **CI/CD Only**：禁止本地构建，所有验证走 GitHub Actions（详见规则 3）
- **合并 PR 后自动清理分支**：每次 PR 合并后，自动删除对应的本地合并分支与远程 head 分支
- **单据号自动生成强制（IR，2026-09-17）**：所有单据的单据号由系统自动生成填入，表单输入框只读展示，禁止手动输入；前端生成后调 /document-no/check 查重，已存在则重新生成；后端"传了就用、没传才生成"+数据库 UNIQUE 兜底；引用类编号（关联订单号等）走单据选择器，同样禁手打
- **代码注释规范（IR，2026-09-17）**：注释只写"这段代码是干嘛用的"（功能/业务规则描述），禁止写变更日志式内容（批次号、"修复xxx"过程、v14/批次xxx/exit 143 排障史等）；变更说明放 commit message，注释永远只描述当前代码的用途
- **禁止兜底掩盖错误（IR，2026-09-17）**：代码禁止兜底逻辑（互填/默认值掩盖缺失字段等），兜底即掩盖真实错误；字段缺失/校验失败让错误真实暴露，由调用方或数据层修正
- **同缸出完允许换缸（IR，2026-09-17）**：发货缸号规则——同缸面料出完后允许使用其他缸面料继续供货，多缸号记警告日志（裁床分缸裁剪防色差），禁止阻断发货
- **修复流程纪律（IR，2026-09-17）**：每轮 CI 失败必须全部修复并自审（prettier/esbuild 静态检查）后才推送；先总结 PR 描述再推送，保证 PR 内容与代码同步
- **测试阶段每一步特别详细日志（IR，2026-09-03）**：E2E/CI 调试期间，每个关键步骤（登录、页面导航、表单填写、提交、响应、诊断输出）都必须输出显式详细日志（成功/失败均打印，含 URL/状态码/请求 payload 脱敏/响应 body/错误栈），禁止静默失败——任何一步无日志即视为流程黑盒，必须补日志后才能继续排查
- **E2E 真实数据强制（IR，2026-09-07）**：E2E 测试禁止使用 mock（page.route/网络层拦截/假响应一律禁止），必须用真实后端 + 真实 PostgreSQL 数据链路验证；Setup 向导类测试用独立空库（如 bingxi_setup_test）真实执行迁移与初始化
- **全程 CI 验证强制（IR，2026-09-07，重申）**：禁止本地编译、禁止本地启动服务做验证（含下载 Release 二进制本地运行）；真实链路验证一律通过 CI job 进行（如 ci-e2e-setup-wizard），本地仅做静态检查（esbuild/prettier/bash -n/yaml 校验）
- **推送冻结（IR，2026-09-07，2026-09-09 重申继续生效）**：git push 指令禁止生效，所有改动仅本地 commit；推送必须等用户明确授权后执行
- **源代码修改冻结解除（IR，2026-09-09）**：用户已解除源代码修改冻结，修复计划（docs/plans/one-round-fix-plan-2026-09-09.md，10 commit）可开始实施；**推送继续冻结**——全部改动仅本地 commit，推送等用户明确授权
- **文档归档规则（IR，2026-09-09）**：doto.md/bug.md 等任务文件过时即归档（用户确认），归档目录 .monkeycode/docs/archives/YYYY-MM-DD/，命名仿照 doto-YYYY-MM-DD-pre-cleanup.md；项目文档统一存放 .monkeycode/docs/（根目录 docs/ 已迁入），根目录只保留 README/CONTRIBUTING/LICENSE
- **CI 失败修复汇报制（IR，2026-09-11）**：每轮 CI 修复完成后，必须向用户汇报：①本轮修了哪些问题（逐条列出）②每个问题的根因归属（后端代码 bug / 测试文件错误 / 测试配置错误 / 环境级 flaky / 测试基础设施错误）③为什么会出现 ④怎么修复的。禁止只报"已修复"不给归因。修复前必须拉取全部失败 job 的日志（annotations + report zip + md 数据逐测试解析），禁止只看第一个错误就修
- **CI 失败日志分析方法（IR，2026-09-11）**：GitHub job log 只有 E2E-HEAD/TAIL 摘要（Playwright 输出被重定向到 reports/ 文件）；完整失败详情在 e2e-report-shard-N artifact 的 data/*.md 中（含每测试的 Name/Location/Expected/Received）；批量解析脚本需每分片刷新 git credential token（后台终端 token 会失效）
- **CI 失败判责纪律（IR，2026-09-11，用户强调）**：测试失败后必须先判责——源代码错误修源代码，测试文件错误修测试文件，禁止混淆两者瞎改。判责依据：对照后端真实 DTO/路由/约束（grep 源码确认），而非猜测。修复前先回答"这是谁的问题"
- **推送前自审强制（IR，2026-09-11，用户强调）**：git push 前必须完成自审——逐文件检查修改正确性（字段名/类型/路径/枚举值对照后端真实 DTO）、vue-tsc 编译通过、prettier 格式通过。自审通过后才能推送。禁止"改完直接推"
- **CI 日志 grep 精确模式（2026-09-12 排障经验）**：backend.log 里 grep "429" 会误匹配 trace_id/span_id 中的随机数字段（如 `span_id=6b5d4296...`），必须用 `grep -aE "状态: 429|status=429"` 精确模式；完整 backend.log 在 e2e-report-shard-N artifact 的 reports/ 目录下（此前误判"CI 未上传 backend.log"）
- **后台终端 gh/git credential 限制（2026-09-12 环境知识）**：后台终端（sh）里 `git credential fill` 拿不到 token（TOKEN_LEN=0，credential helper 依赖交互 shell 环境），且 cargo 不在 PATH（需 `PATH=$PATH:/root/.cargo/bin`）。对策：前台取 token 写 `/tmp/gh_token.txt`（chmod 600），后台终端 `GH_TOKEN=$(cat /tmp/gh_token.txt)`
- **/system 页隐藏 Tab DOM 假象（2026-09-12 测试知识）**：前端 /system 页多 Tab 共存，隐藏 Tab 的 .el-table__row 仍渲染在 DOM（E2E 遍历到 68 行假象）。测试定位行/搜索框必须用 `:visible` 伪类（`.el-table__row:visible`、`.filter-card input:visible`）；Element Plus checkbox 原生 input 透明隐藏，check() 会跳过，必须点击 .el-checkbox label 根
- **接线进度盘点工具（2026-09-18）**：零调用扫描用 `grep -rl -w <fn> src --exclude-dir=api` 全目录（src/store 单数！）；分类脚本在 /tmp/opencode/real4.txt（真实缺口）/dups4.txt（同 URL+同方法重复）。trading.ts 25 个缺口为同 URL 重复封装（页面已用 trading-contract/price/return.ts）。批量接线模式：ElMessageBox 确认/输入、行操作列、el-tabs 包多域、内联中文（页面既有 i18n 风格除外）、每域 1 commit + prettier/esbuild 验证 + 中英 i18n 键对齐校验（node 深度花括号配对脚本）
- **接线批次 12 域已完成（2026-09-18）**：system-governance(3336a03)/supplier-evaluation(63b5298)/color-price(08a0c42,新增 seasonal-rules+customer-special 页)/inventory-count(f2d4be4)/outsourcing(9d909bb,收回单tab)/bi(c06a06b)/customer(9bedd90)/report-enhanced(299fb36,SubscriptionPanel)/api-gateway(7f27f0a,统计tab)/supplier(06d5952,修复资质弹窗未声明崩溃)/inventory-adjustment(55ed636)/inventory-transfer(a2c5377) 共 76 函数。明细行维护模式：查看态回源 getInventoryTransfer 类 + 行编辑(PUT items/id)+删除+添加栏，状态门后端为准。剩余真实缺口约 270 个（sales-return 7/production-recipe 7/fabric-inspection 7/production 6/ai-extend 6/custom-order 6/compliance 6/warehouse 6/bpm-enhanced 6/system-update 6 等）
- **页面合并原则（IR，2026-09-18 用户强调）**：接线禁止新开独立页面，新功能一律挂现有页面 tab/弹窗；已建的 color-price seasonal-rules/customer-special 两页已合并为 list.vue 三合一 tab（a61e785，git mv 改 tabs/ 组件+删路由）。后续接线 sales-return 已按此模式执行（e781173）
- **接线批次续（2026-09-18）**：color-price 合并(a61e785) + sales-return 6 函数(e781173)。剩余真实缺口约 264 个，大域：production-recipe 7/fabric-inspection 7/production 6/ai-extend 6/custom-order 6/compliance 6/warehouse 6/bpm-enhanced 6/system-update 6/bom 5/crm-enhanced 5(recycle rules)/product 5/customer-share 5/purchase-inspection 5/data-permission 5/wage 5/bulk-color-approval 5 + 4 个以下小域。重复判定记录在各域 commit message
- **接线批次续 2（2026-09-18）**：production-recipe 7(4a7e665,加料处方+试算)+approved_by 硬编码修复(06dbdb4)、fabric-inspection 7(e46ce95+49441d6,打卷/疵点链路,新增 listFabricDefectsByInspection 封装)、production 5(f5dc140,提交审批/审批驳回/进度/日志/回源,后端大写状态)、ai-extend 6(0ecee30,应用/历史/批量/回源)。剩余缺口约 245 个：custom-order 6/compliance 6/warehouse 6/bpm-enhanced 6/system-update 6/bom 5/crm-enhanced 5/product 5/customer-share 5/purchase-inspection 5/data-permission 5/wage 5/bulk-color-approval 5 + 4 以下小域。生产页面既有小写状态按钮与后端大写不符属历史遗留未动
- **已开发功能核查原则（IR，2026-09-18 用户强调）**：接线前必须先核查该功能是否已有 UI（含内联 URL/useTableApi/composable 消费同端点）；有差异→修（如 crm 回收规则假保存/假删除/假字段 bca361e）；无差异→复用旧实现、封装判重复。扫描脚本：grep 零调用 fn 的 URL 字面量在 src 非 api 目录的出现（注意 router/MainLayout 是路径误匹配）
- **接线批次续 3（2026-09-18）**：crm-enhanced 5(bca361e,修复假保存/假删除/契约字段/销售员数据源+批量分配)、custom-order 3(7bf2c65,节点CRUD+日志)、compliance 6(b1c332e,新建环保合规页=EHS组,双tab)、warehouse 6(615e82b,详情+库位管理)、bpm-enhanced 2(226df96,定义/模板回源)、system-update 3(f48e4f0,备份/版本/任务回源)、wage 5(b3d42d7,编辑/删除/导出/按工人/生效费率)、bom 4(e6282ad,提交/审批/版本/回源)。BOM 状态机：submit 非PENDING→PENDING；approve 仅 PENDING→ACTIVE/INACTIVE。已内联消费判重复：api-gateway 3/audit 2/security/logistics/material-shortage/mrp/scheduling/slow-query/cost/five-dimension/data-import 2/purchase-inspection/purchase-receipt/notification/customer-credit 2/assist-accounting/bpm 3/system-update 3 等。剩余缺口：product 5、bulk-color-approval 5、customer-share 5、data-permission 5、asset 4、notification 4(batchMarkAsRead/getSettings/updateSetting)、mrp 4(cancelMrpCalculation/exportMrpResult/getMaterialRequirementDetail)、export-approvals 4、financial-analysis 4、purchase-return 4、purchase-receipt 4(generatePurchaseReceiptNo/items)、logistics 3 + 小域若干
- **接线批次续 4（2026-09-18）**：product 4(657952b,色号对话框模板缺失补全[按钮点击无反应假功能]+删除/批量色号/批量建改)、bulk-color-approval 5(0332f82,降级/报废/详情历史/统计)、customer-share 5(a91680a,共享查询/权限校验/团队查询/成员校验)、data-permission 4(b59cc48,驼峰/蛇形契约全面修正[原设置权限必422+表格空白列]+编辑回源+删改走 DELETE /{id})、compliance 后续、purchase-receipt 4(3b3eb99,产品下拉恒空[分页对象误当数组]+明细契约映射+item级同步+预生成单号)、logistics 3(44d463c,轨迹事件+关联采购单)、notification 3(b3af180,批量已读+设置契约修正[/notifications/settings→/user-notification-settings 单对象])、mrp 3(bfc856c,供需明细/取消/导出)。剩余：export-approvals 4、financial-analysis 4、purchase-return 4、asset 4 + 1-2 个小域若干 + request/omni-audit/email/auth 等内部/工具封装决策
- **purchase-return 待修差异（2026-09-18 精确盘点，下轮首个任务）**：后端契约 Create={receipt_id?,order_id?,supplier_id(必填),return_date(必填),warehouse_id?,department_id?,reason_type(必填),reason_detail?,notes?}，Update 仅 {reason_type?,reason_detail?,notes?}（PUT 不含 items）；明细 CreateReturnItemRequest={line_no,material_id,quantity_ordered?,quantity_returned,unit_price,tax_rate?,discount_percent?} 走 POST /returns/{id}/items + PUT/DELETE items/{item_id}（仅 DRAFT 态）。前端表单缺 supplier/reason_type 字段、items 发 product 契约会被忽略（编辑明细假保存）、updatePurchaseReturn 发驼峰全被忽略。修法：PurchaseReturnForm 加供应商/原因类型选择 → usePrRtn handleSubmit 按契约映射（create 映射 items 逐条 createPurchaseReturnItem；update 表头 PUT + item 级同步，removeItem 记 ID 走 deletePurchaseReturnItem，参考 purchase-receipt 3b3eb99 模式）
- **接线批次续 5（2026-09-18）**：export-approvals 4(d86ebdb,待我审批/发起审批/详情回源/令牌校验)、financial-analysis 4(1fd3467,详情回源/带参执行/指标创建/趋势查询)。asset 4（getAsset/getAssetList/adjustBudget/getBudget，MainLayout 路由为误匹配，仍待接）+ request/email/auth/omni-audit 等内部工具封装批量决策未做。零调用清单已接近收敛，硬编码清理（approved_by 等已部分修）与全量自审、PR 更新待续
- **接线批次续 6 / 收敛（2026-09-18）**：purchase-return 4(bfb6fa8,契约重构+假数据修复)、export-approvals 4(d86ebdb)、financial-analysis 4(1fd3467)、budget/asset(894bb0f+90c6b2a)。最终重扫剩余 214 个零调用，全部为已决策项：trading 25 同 URL 重复、api-gateway/bpm/ar-reconciliation/crm/inventory 等 useTableApi 与内联同操作重复、system-governance/chemical/custom-order 查询变体等价覆盖、request.ts 3 个为模块内部工具（SAFE_ERROR_MESSAGES/shouldRetry/getSafeErrorMessage 在 src/api/request.ts 内部消费，扫描排除 api 目录故误报）。全量自审通过：74 个改动文件 prettier 全过 + esbuild 全过。硬编码收尾仅剩 locales 两处 colCustomer 重复键（预存在，位于 en/zh ~6911 行）。后续待办：更新 PR #941 描述 → 用户指令推送 → 监控 CI
- **接线收尾批次 7 / 缺口清零（2026-09-18）**：精确重扫（修正 URL 提取 bug：request.get 泛型嵌套+函数体边界，先前 214 系误报）确认 83 真实零调用。本轮接线 19 个：trading（价格审批/删除 tabs 按钮+合同编辑对话框 07aefeb）、customer-credit（详情/编辑/评估/删除 d6f5329）、data-permission（角色级 get/update/delete 修正 by-id 契约错配 d6f5329）、bpm（监控统计实时化+发起流程+业务关系+待办表 7696d53）、email（发送对话框+模板回源 3f1de22）、product（分类树改后端接口）、security（resolveSecurityAlert 告警处理）、sales-return（明细行编辑 19557d1）、five-dimension（辅助核算钻取）、budget（BUDGET_STATUS 复用 b66e3bb）。修复真实 bug：capacity useCp.ts useTableApi URL 缺 /production 前缀（404）。colCustomer 重复键清理(2d1353f)。i18n 42 缺失键全量补齐（apModule 顶层 paymentRequest/report 为深层嵌套误位、security 系列、product 批量删除系列等），check-i18n 0 缺失。剩余 64 零调用全部为已决策项（48 DUP 同 URL/useTableApi 内联/别名导出/查询变体 + 16 批量保存等价覆盖如 purchase-inspection item CRUD）。判重工具：/tmp/opencode/classify_final2.py（分类）+ check_vue.sh（vue script 块 esbuild 校验）
- **零调用扫描方法论（2026-09-18）**：API 封装 URL 提取必须限定函数体（下一个 export 前泛 1200 字符会吞后续函数的 URL 导致 DUP 误判）；request.get<ApiResponse<X[]>> 嵌套泛型需递归正则；DUP 判定 = 精确 URL(去 ${} 归一)+method 匹配其他已消费封装，INLINE = URL 字面量在 src 非 api 目录出现（rg -F 固定串）；页面 useTableApi url 与封装同 URL 即等价消费（无需改页面调封装）
- **假通过反模式（2026-09-21 判责经验，最高优先级）**：读列表时 `多层 || 兜底 + if(!res.ok()) return []` 的空 reader，配上 `if (找到了) { 断言 } else { console.warn }` 的软分支，会让整条业务链路一条断言都不执行却全绿。三个同型根因（通知列表读 data.items 而后端 handler 返回 data.list；产品列表读 product_name 而后端实体是 name；BPM 流程定义节点 schema 与后端解析器不一致）全部靠这个模式潜伏了 16 轮。检查口诀：改测试前先问"这条断言到底有没有跑到"；取列表必须对照后端 handler 逐字校验响应 key；任何 else-warn 都要能给出可判责的日志与硬断言
- **CI 判责证据源优先级（2026-09-21 修正）**：分片 artifact 内 `reports/playwright-output.txt`（全量 console + pw:api 逐请求）与 `reports/backend.log` 才是完整信号；`error-context.md` 只有最后一条断言原文与页面快照，job 日志只有 E2E-HEAD/TAIL 摘要。判责顺序：先按用例前缀（如 `[31c-产品]`）grep playwright-output.txt，再看 error-context 快照，最后用 backend.log 交叉验证。对外脱敏为「业务处理失败」的 BUSINESS_ERROR 只能靠服务端日志定位（utils/error.rs 脱敏，真实消息由 log_error 写入）
- **S10/S9/S11/33-33b 判责结论（2026-09-18，拉 run 35204061536 shard-12 artifact 逐请求分析）**：31c 客户停用在最新 run 已通过（S10 关闭）；33/33b 全 401（期望 403）根因 = 远程分支 helpers.ts API_BASE 默认 127.0.0.1:8082，UI 登录 cookie 落 localhost 域，page.request.fetch 到 127.0.0.1 host 不匹配 → Cookie=false/false/Header=false → 401，属测试基建错误（非源代码/非测试逻辑错误），修复 = c2d9a4f 全量统一 localhost:8082（本地分支已含，待推送验证）。trace 网络层证据：请求 header 无 cookie 项。同根因覆盖 S9/S11 超时潮。判责方法：playwright trace.zip 内 0-trace.network 逐请求查 header + backend.log 对照 LOGIN_SUCCESS 时间线

---

## 三、文件分工

| 文件 | 用途 |
|------|------|
| `MEMORY.md` | 规则索引（一句话核心 + 链接） |
| `MEMORY-SU.md` | 规则详细说明 |
| `doto.md` | 未完成任务（任务队列） |
| `doto-su.md` | 已完成任务详细记录 |
| `CHANGELOG.md` | 任务一句话总结 |
| `audit_assignment.md` | 审计任务分配和复审规则 |

---

## 四、详细规范索引

| 规范域 | 链接 |
|--------|------|
| 基础规范（沟通/编码/工程/面料术语/Bug管理/数据库） | [MEMORY-SU §三](file:///workspace/.monkeycode/MEMORY-SU.md#三基础规范) |
| CI/CD 强制（本地编译禁止/CI 监控 API/服务器环境/部署限制） | [MEMORY-SU §四](file:///workspace/.monkeycode/MEMORY-SU.md#四cicd-强制) |
| 核心经验（沙箱网络/Clippy Baseline/is_production/SeaORM Trait 等） | [MEMORY-SU §五](file:///workspace/.monkeycode/MEMORY-SU.md#五核心经验关键排错与开发经验) |
| 规则自我迭代日志（个人习惯/项目习惯/迭代摘要） | [MEMORY-SU §六](file:///workspace/.monkeycode/MEMORY-SU.md#六规则自我迭代日志) |
| 归档索引（历史整理前内容/审计报告/迭代历史） | [MEMORY-SU §七](file:///workspace/.monkeycode/MEMORY-SU.md#七归档索引) |

---

## 五、规则冲突裁决原则（规则 15 落地）

- **优先级**：IR（个人规则）> PR（项目规则）> PH（项目习惯）> IH（个人习惯）
- **IR 规则"关键内容需存储在 MEMORY.md"** 的适用范围：仅限**规则相关关键内容**（如规则冲突裁决、规则优先级、规则迭代决策），**不含**任务进度/批次摘要/技术决策/PR 列表/架构信息等任务详情
- **规则 8 文件分工强制**：MEMORY.md 只存规则索引；任务详情归档到 doto-su.md；未完成任务到 doto.md；一句话总结到 CHANGELOG.md
- **docs/ 规划文档实时阅读**：作为开发依据，不复制内容到 MEMORY.md，仅在 doto-su.md 引用结论
