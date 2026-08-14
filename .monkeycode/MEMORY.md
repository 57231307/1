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

### 1.2 代码质量（PR）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 2** | 禁止 `#[allow(...)]` 警告抑制，所有警告视为错误必须修复；注释必须与功能一致 | [MEMORY-SU §规则 2](file:///workspace/.monkeycode/MEMORY-SU.md#规则-2代码质量强制pr2026-07-12-追加2026-07-17-合并规则-1420) |

### 1.3 测试与流程（PR）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 3** | 修复按批次连续执行，CI 全绿自动下一批；步骤 0 核实审计内容 + 步骤 4 推送前自审 | [MEMORY-SU §规则 3](file:///workspace/.monkeycode/MEMORY-SU.md#规则-3修复流程自动化与连续执行pr2026-07-11-追加) |
| 🔴 **规则 4** | 复审按规矩进行，baseline 警告视为错误，8 维度闭环 | [MEMORY-SU §规则 4](file:///workspace/.monkeycode/MEMORY-SU.md#规则-4复审严格规范pr2026-07-13-追加) |
| 🔴 **规则 5** | 每 30 批次 E2E 测试（独立工作流不阻塞主 CI） | [MEMORY-SU §规则 5](file:///workspace/.monkeycode/MEMORY-SU.md#规则-5e2e-测试加强pr2026-07-08-追加2026-07-10-批次-262-修订) |
| 🔴 **规则 6** | 测试 mock 数据禁止硬编码，必须抽取到 fixtures 文件 | [MEMORY-SU §规则 6](file:///workspace/.monkeycode/MEMORY-SU.md#规则-6测试-mock-数据禁止硬编码pr2026-07-08-追加) |
| 🔴 **规则 18** | 功能变更必须同步测试代码：修改/新增/删除功能时，测试必须同步更新 | [MEMORY-SU §规则 18](file:///workspace/.monkeycode/MEMORY-SU.md#规则-18功能变更必须同步测试代码pr2026-08-14-追加) |
| 🔴 **规则 19** | 编码行为规范：编码前先思考、保持简单、精准修改、不过度推断、正确处理错误 | [MEMORY-SU §规则 19](file:///workspace/.monkeycode/MEMORY-SU.md#规则-19编码行为规范pr2026-08-14-追加) |

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
