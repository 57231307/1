# 项目规则记忆（索引版）

> 本文件是项目的规则记忆索引，记录规则一句话核心 + 链接到 MEMORY-SU.md 详细说明。
> 规则自我迭代日志（最近 5 条）见 [MEMORY-SU §六、规则自我迭代日志](file:///workspace/.monkeycode/MEMORY-SU.md#六规则自我迭代日志)。

---

## 一、关键项目规则（必读，按功能域分组）

> 优先级：IR（个人规则）> PR（项目规则）> PH（项目习惯）> IH（个人习惯）。规则编号保持不变，仅按功能域分组以提升可读性。

### 1.1 个人规则（IR，最高优先级）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 3** | 成品导入/导出使用 .xlsx/.docx，禁止 CSV/txt/rtf/html 作为成品 | [MEMORY-SU §规则 3](file:///workspace/.monkeycode/MEMORY-SU.md#规则-3项目成品导入导出文档格式ir-个人规则2026-07-06-追加) |
| 🔴 **规则 4** | `///` 注释精简为 1 行（首选），最多 2 行，禁止 3 行+注释块 | [MEMORY-SU §规则 4](file:///workspace/.monkeycode/MEMORY-SU.md#规则-4项目文件--注释精简规则ir-个人规则2026-07-06-追加) |
| 🔴 **规则 7** | 禁止简洁方案，采用最合理/最准确/最符合业务需求的方案 | [MEMORY-SU §规则 7](file:///workspace/.monkeycode/MEMORY-SU.md#规则-7禁止简洁方案ir-个人规则2026-07-08-追加) |
| 🔴 **规则 9** | 个人规则（IR）高于项目规则（PR）；优先级 IR > PR > PH > IH | [MEMORY-SU §规则 9](file:///workspace/.monkeycode/MEMORY-SU.md#规则-9个人规则高于项目规则ir-个人规则2026-07-08-追加2026-07-17-与规则-18-对齐) |

### 1.2 实现完整性（PR）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 00** | 每次修改后、推送 CI/CD 前必须评估关联影响，所有修改为代码级修改 | [MEMORY-SU §规则 00](file:///workspace/.monkeycode/MEMORY-SU.md#规则-00修改前关联影响评估强制pr2026-07-11-追加) |
| 🔴 **规则 0** | 所有预留 API/功能/占位符/路由必须真实实现，禁止 stub/placeholder | [MEMORY-SU §规则 0](file:///workspace/.monkeycode/MEMORY-SU.md#规则-0真实实现强制pr2026-07-04-追加2026-07-17-合并规则-8) |
| 🔴 **规则 1** | 功能扩展空间/预留扩展点/未来支持视为未实现，必须本批次实现 | [MEMORY-SU §规则 1](file:///workspace/.monkeycode/MEMORY-SU.md#规则-1扩展空间视为未实现pr2026-07-06-追加) |
| 🔴 **规则 2** | 完全实现 = 完整实现，100% 跟规划一致，禁止部分实现 | [MEMORY-SU §规则 2](file:///workspace/.monkeycode/MEMORY-SU.md#规则-2完全实现等于完整实现pr2026-07-06-追加) |
| 🔴 **规则 8** | 已合并到规则 0（2026-07-17），保留编号避免破坏引用 | [MEMORY-SU §规则 8](file:///workspace/.monkeycode/MEMORY-SU.md#规则-8真实实现强制已合并到规则-02026-07-17) |

### 1.3 代码质量（PR）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 14** | 禁止 `#[allow(...)]` 警告抑制，所有警告视为错误必须修复 | [MEMORY-SU §规则 14](file:///workspace/.monkeycode/MEMORY-SU.md#规则-14移除所有警告抑制所有警告视为错误pr2026-07-12-追加) |
| 🔴 **规则 20** | 注释必须与功能实现一致，禁止随意编写；含 doc comment/行内/TODO；CI 强制检查 | [MEMORY-SU §规则 20](file:///workspace/.monkeycode/MEMORY-SU.md#规则-20注释与功能一致性强制pr2026-07-17-追加) |

### 1.4 测试与流程（PR）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 5** | 每 30 批次 E2E 测试（独立工作流不阻塞主 CI），按 20/28/29 节奏监控 | [MEMORY-SU §规则 5](file:///workspace/.monkeycode/MEMORY-SU.md#规则-5e2e-测试加强pr2026-07-08-追加2026-07-10-批次-262-修订) |
| 🔴 **规则 6** | 测试 mock 数据禁止硬编码，必须抽取到 fixtures 文件 | [MEMORY-SU §规则 6](file:///workspace/.monkeycode/MEMORY-SU.md#规则-6测试-mock-数据禁止硬编码pr2026-07-08-追加) |
| 🔴 **规则 13** | 修复按批次连续执行，CI 全绿自动下一批；**步骤 0 确定审计结果内容是否存在** + **步骤 4 修复后推送前自审**（与规则 20 联动） | [MEMORY-SU §规则 13](file:///workspace/.monkeycode/MEMORY-SU.md#规则-13修复流程自动化与连续执行pr2026-07-11-追加) |
| 🔴 **规则 15** | 复审按规矩进行，baseline 警告视为错误，8 维度闭环 + 4 轮次状态 | [MEMORY-SU §规则 15](file:///workspace/.monkeycode/MEMORY-SU.md#规则-15复审严格规范--业务财务运行逻辑闭环pr2026-07-13-追加2026-07-17-精简) |

### 1.5 安全合规（PR）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 11** | 符合中国法律法规（个人信息保护法/数据安全法/网络安全法） | [MEMORY-SU §规则 11](file:///workspace/.monkeycode/MEMORY-SU.md#规则-11法律合规标准pr2026-07-08-追加) |
| 🔴 **规则 12** | API 必须认证/权限校验，密码强哈希，SQL 参数化，敏感操作审计 | [MEMORY-SU §规则 12](file:///workspace/.monkeycode/MEMORY-SU.md#规则-12法律安全标准pr2026-07-08-追加) |

### 1.6 记忆与文档管理（PR）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 10** | 每 15 批整理归档 + 实时归档；MEMORY.md 只存规则，doto.md 只存未完成任务 | [MEMORY-SU §规则 10](file:///workspace/.monkeycode/MEMORY-SU.md#规则-10记忆文件定期整理归档--实时归档pr2026-07-08-追加2026-07-10-修正2026-07-14-二次修正) |
| 🔴 **规则 16** | `.monkeycode/` 全目录强制追踪，禁止忽略任何文件 | [MEMORY-SU §规则 16](file:///workspace/.monkeycode/MEMORY-SU.md#规则-16monkeycode-全目录强制追踪pr2026-07-17-追加) |
| 🔴 **规则 17** | 审计计划/复审规则变更时，5 个关联文档必须同步更新 | [MEMORY-SU §规则 17](file:///workspace/.monkeycode/MEMORY-SU.md#规则-17审计文档同步规则pr2026-07-17-追加) |
| 🔴 **规则 18** | 规则自我迭代机制（四分类 PR/PH/IR/IH + 6 条触发条件 + 自动记录） | [MEMORY-SU §规则 18](file:///workspace/.monkeycode/MEMORY-SU.md#规则-18规则自我迭代机制pr2026-07-17-追加) |

### 1.7 工具与运维（PR）

| 规则 | 一句话核心 | 详细说明 |
|------|-----------|----------|
| 🔴 **规则 19** | 工具连接异常分级响应（L1 60s / L2 60-180s / L3 30min 周期）+ 非阻塞推理 | [MEMORY-SU §规则 19](file:///workspace/.monkeycode/MEMORY-SU.md#规则-19工具连接异常重试策略pr2026-07-17-追加2026-07-17-二次迭代增强) |

---

## 二、常规规则

- **每项修复 1 commit**：bug 修复按"每项 1 commit"原则，便于回滚和审计
- **公开端点收敛**：仅登录/刷新/健康检查可匿名访问，其他所有端点必须认证
- **多租户已删除**（2026-06-28 m0029）：所有 tenant_id 列/字段/过滤/索引/管理表均已移除
- **CI/CD Only**：禁止本地构建，所有验证走 GitHub Actions（详见规则 13/14）

---

## 三、文件分工

> 详见 [MEMORY-SU §规则 10](file:///workspace/.monkeycode/MEMORY-SU.md#规则-10记忆文件定期整理归档--实时归档pr2026-07-08-追加2026-07-10-修正2026-07-14-二次修正) 的文件分工表。

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

## 五、关键项目内容（IR 规则：关键内容存储）

> 按 IR 规则"关键内容需存储在 MEMORY.md"维护，与 §一规则索引并列。详细任务记录见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，未完成任务见 [doto.md](file:///workspace/.monkeycode/doto.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)。最近更新：2026-07-26。

### 5.1 项目阶段与 P0 任务进度（2026-07-26）

- **当前阶段**：V15 修复阶段（模块 G 共 17 项 P0 任务）
- **完成度**：15 ✅ / 0 待CI / 2 ⏳ / 0 ❌
- **已完成 15 项**：D01, D02, D03, D04, D06, D07, D09, D10, D11, D12, D13, D14, D15, D16, D17
- **进行中 2 项**：
  - **D05**（i18n 接入率 27.7%，271 文件未接入，10 批次规划见 doto.md §0.8）
  - **D08**（>80 行函数 95 个，已拆分 6 个 >200 行函数，2 批次规划见 doto.md §0.8.2-§0.8.3）

### 5.2 关键技术决策（最近）

- **AuditContext 结构体**（[omni_audit.rs](file:///workspace/backend/src/middleware/omni_audit.rs)）：跨 send_audit_log/build_audit_message/build_audit_payload 三函数复用，封装 12 个共享参数，函数参数从 13/14/9 减至 2/3/2
- **类型别名消除 type_complexity**（[permission.rs](file:///workspace/backend/src/services/init_service_ops/permission.rs)）：PermPair / RoleResourceGroup / RoleResourceSlice / RoleResourceGroups 四层别名
- **facade 模式**（product_service.rs 等）：service 拆分为 facade + ops/ 子模块，缓存接入跟踪到 impl 实际所在文件
- **Python 括号深度追踪脚本**：替代简单 awk 脚本，正确处理字符串/字符/注释/原始字符串，避免误判嵌套 `}` 为函数结尾

### 5.3 最近重要 PR（2026-07-26）

| PR | 状态 | 内容 |
|-----|------|------|
| #737 | ✅ 已合并 main 9768bbe | D09 拆分 9 个 >100 行函数 + D14 修复 4 处 api 命名 + clippy 3 警告修复 |
| #739 | 🔄 CI 进行中 | docs(p0): 更新 D09+D14 完成状态（PR #737 已合并到 main） |

### 5.4 项目架构关键信息（来自 [docs/ARCHITECTURE.md](file:///workspace/.monkeycode/docs/ARCHITECTURE.md)）

- **技术栈**：Rust 1.75+ / Axum 0.7 / SeaORM 1.0 / Vue 3.4+ / Element Plus / Pinia / Vite
- **代码规模**：后端 447 个 .rs 文件（10.8 万行）/ 前端 188 个 .ts+vue 文件（5.7 万行）/ 752 个路由
- **服务层拆分**：原 7 个超大 service 已拆为 22 个子域文件（po/so/crm/inv/ar/ai/report）
- **中间件顺序**（main.rs，axum 0.7 从外到内）：trace_context → metrics → TraceLayer → Cors → request_validator → permission → auth → security headers × 7 → timeout → handler
- **CI/CD Only**：禁止本地构建，所有验证走 GitHub Actions

### 5.5 文档分工与归档规则

| 文件 | 用途 | 维护规则 |
|------|------|----------|
| `MEMORY.md` | 规则索引 + 关键项目内容（本节） | IR 规则：关键内容存储；规则 10：规则索引部分只存规则 |
| `MEMORY-SU.md` | 规则详细说明 | 每条规则一句话核心 + 详细说明 + 迭代日志 |
| `doto.md` | 未完成任务（任务队列） | 只记录未完成任务，已完成迁移到 doto-su.md |
| `doto-su.md` | 已完成任务详细记录 | 每批次完成后从 doto.md 迁入 |
| `CHANGELOG.md` | 任务一句话总结 | 每个任务一行摘要，禁止写入详细内容 |
| `audit_assignment.md` | 审计任务分配和复审规则 | 复审按规矩进行，baseline 警告视为错误 |
| `docs/` | 项目规划文档（ARCHITECTURE/CODE_STYLE_GUIDE/DEVELOPER_GUIDE/INTERFACES/SECURITY/PROJECT_HEALTH_REPORT） | IR 规则：实时阅读，作为开发依据 |
