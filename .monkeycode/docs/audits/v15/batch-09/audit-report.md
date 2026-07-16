# V15 批次节奏与色卡发放审计报告（类九+类十·批次 09）

- **审计子代理**：V15 审计子代理（类九批次节奏+类十色卡发放）
- **审计范围**：7 维度（类九 2 维度：批次节奏与 E2E 监控 / 记忆整理与归档；类十 5 维度：色卡发放业务规则修正专项 10.1-10.7）
- **审计依据**：
  - `/workspace/.monkeycode/docs/audits/v15-review-plan-2026-07-15.md` 第 1398-3331 行（类九+类十）
  - `/workspace/.monkeycode/MEMORY.md`（规则 5/10/13/14）
  - `/workspace/.github/workflows/e2e-batch.yml`（E2E 工作流）
- **审计方法**：Read 审计计划 + Grep 检索色卡相关代码 + Read 关键文件 + 对照审计计划核对
- **审计时间**：2026-07-16
- **审计原则**：只做审计不修改业务代码

---

## 类九 维度 1：批次节奏与 E2E 监控

### 检查方法

1. Read `/workspace/.github/workflows/e2e-batch.yml` 全文（337 行）
2. Grep 检索 `e2e-batch|workflow_dispatch|skip_reason|e2e-skipped`
3. 对照审计计划第 1404-1410 行 6 项检查要点

### 发现

#### ✅ 已落实的项

1. **E2E 工作流已独立到 e2e-batch.yml**（规则 5）
   - 证据：`/workspace/.github/workflows/e2e-batch.yml:1-337`
   - 工作流名称：`E2E 批次测试（每 30 批次）`（第 27 行）
   - 独立于主 CI/CD（ci-cd.yml），不阻塞主 CI

2. **workflow_dispatch 触发机制已实现**（检查要点 1）
   - 证据：`/workspace/.github/workflows/e2e-batch.yml:29-40`
   ```yaml
   on:
     workflow_dispatch:
       inputs:
         batch_number:
           description: '批次编号（如 270、300、330，30 的倍数）'
           required: true
           type: string
   ```
   - 支持手动触发，批次号通过输入参数指定

3. **skip_reason 参数 + e2e-skipped job 已实现**（检查要点 3）
   - 证据：`/workspace/.github/workflows/e2e-batch.yml:36-40`（skip_reason 输入参数）
   - 证据：`/workspace/.github/workflows/e2e-batch.yml:323-337`（e2e-skipped job）
   ```yaml
   e2e-skipped:
     name: ⏭️ E2E 跳过（批次 ${{ inputs.batch_number }}）
     if: ${{ inputs.skip_reason != '' }}
     steps:
       - name: 记录跳过原因
         run: |
           echo "**跳过原因**: ${{ inputs.skip_reason }}" >> "$GITHUB_STEP_SUMMARY"
           echo "本周期 E2E 已跳过，下一周期将在批次 $(( ${{ inputs.batch_number }} + 30 )) 恢复。" >> "$GITHUB_STEP_SUMMARY"
   ```
   - e2e-test job 通过 `if: ${{ inputs.skip_reason == '' }}` 控制跳过（第 68 行）

4. **E2E 测试报告上传 artifact**（检查要点 4 部分）
   - 证据：`/workspace/.github/workflows/e2e-batch.yml:308-318`
   ```yaml
   - name: 上传测试报告
     if: always()
     uses: actions/upload-artifact@v4
     with:
       name: e2e-report-batch-${{ inputs.batch_number }}
       path: |
         frontend/playwright-report/
         frontend/reports/e2e-report.md
         frontend/reports/backend.log
         frontend/test-results/
       retention-days: 30
   ```
   - 报告保留 30 天

5. **监控节奏说明已写入工作流注释**（检查要点 2/6）
   - 证据：`/workspace/.github/workflows/e2e-batch.yml:8-19`
   ```
   监控机制（由 agent 在批次节奏中执行，不在工作流内实现）：
   - 批次 N（30 的倍数）：触发本工作流（action: create）
   - 批次 N+20：查询本工作流最近的 run 状态（action: get）
   - 批次 N+28：再次查询（若 N+20 未完成）
   - 批次 N+29：最后一次查询，未完成则跳过 N+30 的 E2E 周期
   ```
   - 证据：`/workspace/.github/workflows/e2e-batch.yml:126` 明确"禁止死等 E2E 完成"

6. **E2E 工作流独立编译后端**（规则 5 要求）
   - 证据：`/workspace/.github/workflows/e2e-batch.yml:155-159`
   ```yaml
   - name: 编译后端（release 模式）
     working-directory: backend
     run: |
       cargo build --release --bin server --bin bingxi
   ```
   - 不依赖 ci-cd.yml artifact

7. **完整 E2E 环境**（PostgreSQL + 后端 + 前端 + Playwright）
   - 证据：`/workspace/.github/workflows/e2e-batch.yml:70-83`（PostgreSQL service container）
   - 证据：`/workspace/.github/workflows/e2e-batch.yml:194-214`（后端启动 + 健康检查）
   - 证据：`/workspace/.github/workflows/e2e-batch.yml:255-298`（Playwright 测试）

#### ⚠️ 待验证的项（流程要求，非代码缺陷）

1. **E2E 报告保存到 docs/audits/**（检查要点 4）
   - 现状：artifact 上传到 GitHub Actions（retention-days: 30），但未自动下载保存到 `.monkeycode/docs/audits/`
   - 历史报告：`.monkeycode/docs/audits/2026-07-08-batch190-e2e-report.md` 存在（说明流程曾执行）
   - 风险等级：P3（流程执行问题，非代码缺陷）
   - 说明：V15 修复阶段尚未开始（当前处于审计阶段），E2E 监控节奏尚未触发

2. **20/28/29 节奏监控 E2E run 状态**（检查要点 2）
   - 现状：监控节奏由 agent 在批次节奏中执行，不在工作流内实现（工作流注释明确）
   - 风险等级：P3（流程执行问题，V15 修复阶段开始后需严格执行）

3. **E2E 失败按 P0/P1/P2 优先级纳入后续批次**（检查要点 5）
   - 现状：流程要求（规则 5），无代码实现
   - 风险等级：P3（流程执行问题）

4. **禁止死等 E2E 完成**（检查要点 6）
   - 现状：工作流注释明确（第 126 行），流程要求
   - 风险等级：P3（流程执行问题）

### 维度结论

- **已落实**：4 项（工作流配置完整，skip_reason + e2e-skipped job 已实现，artifact 上传，独立编译）
- **待验证**：4 项（均为流程执行要求，V15 修复阶段开始后需严格执行）
- **缺陷**：0 项 P0/P1/P2
- **总计**：8 项检查点

---

## 类九 维度 2：记忆整理与归档

### 检查方法

1. Read `/workspace/.monkeycode/MEMORY.md`（701 行，规则记忆）
2. Read `/workspace/.monkeycode/doto.md`（100 行，未完成任务）
3. Read `/workspace/.monkeycode/doto-su.md`（80 行，已完成任务归档）
4. Read `/workspace/.monkeycode/CHANGELOG.md`（50 行，一句话总结）
5. Read `/workspace/.monkeycode/bug.md`（14 行，漏洞登记）
6. LS `/workspace/.monkeycode/docs/archives/`（历史归档目录）
7. 对照审计计划第 1414-1425 行 5 项检查要点

### 发现

#### ✅ 已落实的项

1. **MEMORY.md 只保留项目规则**（检查要点 3）
   - 证据：`/workspace/.monkeycode/MEMORY.md:1-5`
   ```
   # 项目规则记忆
   > 本文件是项目的**规则记忆**，记录必须遵守的规则、指令、偏好和工作流规范。
   > 最近整理：2026-07-16（v14 复审修复 430/430 全部完成并归档...）
   ```
   - 内容为规则 00-15 + 常规规则 1-9 + 文件定义 + 基础规范 + 安全规范 + CI/CD 强制 + 核心经验 + 工作流协作 + 代码规范 + 性能与错误处理 + 文档与持续改进 + 归档索引
   - 无任务相关内容（符合规则 10 要求）

2. **doto.md 只保留未完成任务**（检查要点 3）
   - 证据：`/workspace/.monkeycode/doto.md:1-5`
   ```
   # 未完成任务
   > 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单）。
   > 已完成任务见 doto-su.md，一句话总结见 CHANGELOG.md，规则见 MEMORY.md。
   ```
   - 内容：v14 复审修复进度表 + V15 待执行任务概览 + 规则节点提醒
   - 无已完成批次的详细表格（符合规则 10 要求）

3. **doto-su.md 已完成任务详细记录**（检查要点 3）
   - 证据：`/workspace/.monkeycode/doto-su.md:1-5`
   ```
   # 已完成任务归档
   > 本文件保存**已完成的任务**详细记录（修改内容、技术要点、CI 验证）。
   ```
   - 内容：批次 421/420 等详细记录（修改文件、技术要点、CI 验证）

4. **CHANGELOG.md 一句话总结**（检查要点 3）
   - 证据：`/workspace/.monkeycode/CHANGELOG.md:1-5`
   ```
   # 任务一句话总结
   > 每个任务一行摘要，是 doto-su.md 中详细任务内容的一句话总结。禁止写入详细内容。
   ```
   - 内容：V15 审计计划三轮升级表 + v14 修复批次表（每批一行）
   - 无展开详情（符合规则 10 要求）

5. **bug.md 空文件（漏洞登记占位）**（检查要点 3）
   - 证据：`/workspace/.monkeycode/bug.md:1-14`
   ```
   # 安全审计漏洞登记
   > 实时检测与修复（修复后删除条目）。所有漏洞修复完成后保留本文件为空。
   > 最近一次整理：2026-07-14（批次 407 安全修复后，所有已知漏洞已修复）
   （当前无待修复漏洞，所有已知漏洞已在批次 290-308 + 批次 407 修复完成）
   ```
   - 符合规则"所有漏洞修复完成后保留 bug.md 空文件"

6. **audit_assignment.md 存在**（检查要点 3）
   - 证据：`/workspace/.monkeycode/audit_assignment.md` 存在（LS 结果确认）
   - 用途：审计任务分配和复审规则

7. **历史归档到 docs/archives/**（检查要点 5）
   - 证据：`/workspace/.monkeycode/docs/archives/` 目录存在多个归档：
     - `2026-06-24/`（token-rotation）
     - `2026-07-10/`（CHANGELOG/MEMORY/doto pre-cleanup）
     - `2026-07-10-职责分工修正/`
     - `2026-07-11/`（CHANGELOG/MEMORY/bug/doto/doto-su pre-cleanup）
   - 按日期保留（符合规则 10 要求）

8. **文件分工表格明确**（检查要点 3）
   - 证据：`/workspace/.monkeycode/MEMORY.md:189-197`（规则 10 文件分工表）
   ```
   | 文件 | 用途 | 禁止内容 |
   |------|------|----------|
   | MEMORY.md | 规则记忆（规则本身） | 整理记录、批次摘要、历史详情 |
   | doto.md | 未完成任务（任务队列） | 已完成批次的详细表格、历史修复详情 |
   | doto-su.md | 已完成任务详细记录 | —（接收所有归档内容） |
   | CHANGELOG.md | 一句话总结 | 展开详情、技术要点 |
   | audit_assignment.md | 审计任务分配和复审规则 | 审计结果详情（保存到 docs/audits/） |
   ```

#### ⚠️ 待验证的项（流程要求）

1. **每 15 批次整理 .monkeycode/ 所有记忆文件**（检查要点 1）
   - 现状：V15 修复阶段尚未开始（当前处于审计阶段），15 批次整理节点尚未触发
   - 证据：`/workspace/.monkeycode/MEMORY.md:167-180`（规则 10 明确要求）
   - 风险等级：P3（流程执行问题，V15 修复阶段开始后需严格执行）

2. **实时归档：每批 CI 合并后立即归档到 doto-su.md**（检查要点 2）
   - 现状：V15 修复阶段尚未开始，实时归档尚未触发
   - 证据：`/workspace/.monkeycode/MEMORY.md:181-188`（规则 10 二次修正明确要求）
   - 风险等级：P3（流程执行问题）

3. **禁止跨批堆积**（检查要点 4）
   - 现状：当前状态符合（doto.md 无已完成批次详细表格）
   - 风险等级：P3（V15 修复阶段需持续维护）

### 维度结论

- **已落实**：8 项（文件分工明确，所有文件符合规则，历史归档完整）
- **待验证**：3 项（均为流程执行要求，V15 修复阶段开始后需严格执行）
- **缺陷**：0 项 P0/P1/P2
- **总计**：11 项检查点

---

## 类十 维度 1：色卡业务模式重构（10.1）

### 检查方法

1. Glob `backend/src/**/color_card*` 检索色卡相关文件
2. Read `/workspace/backend/src/services/color_card_borrow_service.rs`（487 行）
3. Read `/workspace/backend/src/models/color_card_borrow_record.rs`（71 行）
4. Read `/workspace/backend/src/routes/color_card.rs`（89 行）
5. Read `/workspace/backend/src/handlers/color_card/borrow.rs`（158 行）
6. Read `/workspace/frontend/src/api/color-card.ts`（279 行）
7. LS `/workspace/frontend/src/views/color-cards/`
8. Read `/workspace/backend/migrations/20260617000008_create_color_card_borrow_records/up.sql`（35 行）
9. Grep `color_card_issue|ColorCardIssue|issue_card|mark_received|mark_used|cancel_issue` 检索新模式代码
10. 对照审计计划第 1434-2392 行 10.1 节代码级实现规范

### 发现

#### ❌ 缺陷项

**缺陷 10.1-1：旧"借出/归还/遗失/损坏"模式完全存在，未重构为"发放"模式**
- **风险等级：P0**
- **证据**：
  - `/workspace/backend/src/services/color_card_borrow_service.rs:1-5`
  ```rust
  //! 色卡借出管理服务
  //! 提供借出 / 归还 / 遗失 / 损坏 / 历史查询业务
  //! 状态机：borrowed → returned / lost / damaged（终态不可再转换）
  ```
  - `/workspace/backend/src/services/color_card_borrow_service.rs:39-47`（BorrowStatus 枚举）
  ```rust
  pub enum BorrowStatus {
      Borrowed,
      Returned,
      Lost,
      Damaged,
      Cancelled,
  }
  ```
  - `/workspace/backend/src/models/color_card_borrow_record.rs:9-29`（旧模型，table_name = "color_card_borrow_records"）
  - `/workspace/backend/src/handlers/color_card/borrow.rs:1-5`（旧 handler）
  - `/workspace/backend/src/routes/color_card.rs:52-69`（旧路由）
  - `/workspace/frontend/src/api/color-card.ts:37-49`（前端旧状态枚举）
  - `/workspace/frontend/src/views/color-cards/borrow.vue`（前端旧借出页面）
- **业务影响**：违反用户 2026-07-15 明确要求"色卡只发放给客户，不借出"。当前实现是"借出-归还-遗失-损坏"模式，色卡会收回，与业务规则完全不符。
- **修复建议**：按审计计划 10.1.1-10.1.7 节完整重构为"发放"模式（发放→已收到→已使用→已过期），删除旧 borrow 相关代码，创建新 issue 相关代码。

**缺陷 10.1-2：新"发放"模式文件完全未创建**
- **风险等级：P0**
- **证据**：Grep `color_card_issue|ColorCardIssue` 在 backend/src 下无业务代码匹配（仅审计报告和计划文件匹配）
  - 缺失文件清单：
    - `backend/src/models/color_card_issue_record.rs`（新模型，10.1.4）
    - `backend/src/models/color_card_issue_dto.rs`（新 DTO，10.1.5）
    - `backend/src/services/color_card_issue_service.rs`（新服务，10.1.6）
    - `backend/src/handlers/color_card/issue.rs`（新 handler，10.1.7）
- **业务影响**：发放模式后端完全未实现，无法支持色卡发放业务
- **修复建议**：按审计计划 10.1.4-10.1.7 节代码骨架创建新文件

**缺陷 10.1-3：旧路由未删除，新路由未注册**
- **风险等级：P0**
- **证据**：`/workspace/backend/src/routes/color_card.rs:51-79`
  ```rust
  // 借出 / 归还 / 遗失 / 损坏
  .route("/borrow", post(color_card::borrow_color_card))
  .route("/return/:record_id", post(color_card::return_color_card))
  .route("/lost/:record_id", post(color_card::mark_lost_color_card))
  .route("/damaged/:record_id", post(color_card::mark_damaged_color_card))
  // 取消借出
  .route("/cancel/:record_id", post(color_card::cancel_borrow))
  // 借出历史
  .route("/borrow-records", get(color_card::list_borrow_records))
  .route("/borrow-records/:record_id", get(color_card::get_borrow_record))
  ```
  - 缺失路由：`/color-cards/:id/issue`、`/color-cards/issue`、`/color-cards/issue/:id`、`/color-cards/issue/:id/receive`、`/color-cards/issue/:id/use`、`/color-cards/issue/:id/cancel`
- **业务影响**：API 端点仍是借还模式，前端无法调用发放接口
- **修复建议**：删除旧借还路由，注册新发放路由（审计计划 10.1.7 节）

**缺陷 10.1-4：旧表未重命名为 legacy，新表未创建**
- **风险等级：P0**
- **证据**：
  - `/workspace/backend/migrations/20260617000008_create_color_card_borrow_records/up.sql:6-22`（旧表 color_card_borrow_records 仍存在）
  - Grep `color_card_issue_record` 在 SQL 迁移文件中无匹配
  - 旧表未重命名为 `color_card_borrow_record_legacy`
  - 新表 `color_card_issue_record` 未创建
- **业务影响**：数据库仍是借还模式，无法存储发放记录
- **修复建议**：按审计计划 10.1.3 节创建新表，按 10.7 节迁移旧数据

**缺陷 10.1-5：前端仍是借还模式，未重构为发放模式**
- **风险等级：P0**
- **证据**：
  - `/workspace/frontend/src/api/color-card.ts:221-268`（借出管理 API）
  ```typescript
  // ============== 借出管理 ==============
  export function borrowColorCard(dto: {...}) {...}
  export function returnColorCard(recordId: number, dto: {...}) {...}
  export function markLostColorCard(recordId: number, dto: {...}) {...}
  export function markDamagedColorCard(recordId: number, dto: {...}) {...}
  export function listBorrowRecords(params: {...}) {...}
  ```
  - `/workspace/frontend/src/views/color-cards/borrow.vue`（借出页面存在）
  - 缺失文件：
    - `frontend/src/api/color-card-issue.ts`（10.6.3）
    - `frontend/src/types/color-card-issue.ts`（10.6.2）
    - `frontend/src/composables/useColorCardIssue.ts`（10.6.4）
    - `frontend/src/views/color-card/IssueList.vue`（10.6.5）
    - `frontend/src/views/color-card/IssueForm.vue`（10.6.5）
    - `frontend/src/views/color-card/IssueDetail.vue`（10.6.5）
    - `frontend/src/router/modules/color-card-issue.ts`（10.6.6）
- **业务影响**：前端无法支持色卡发放业务
- **修复建议**：按审计计划 10.6 节完整重构前端

**缺陷 10.1-6：旧表 color_card_borrow_records 仍有 tenant_id 字段（多租户残留）**
- **风险等级：P2**
- **证据**：`/workspace/backend/migrations/20260617000008_create_color_card_borrow_records/up.sql:18`
  ```sql
  "tenant_id" BIGINT NOT NULL,
  ```
  - 索引：`CREATE INDEX IF NOT EXISTS "idx_borrow_tenant" ON "color_card_borrow_records"("tenant_id");`（第 28 行）
- **业务影响**：多租户功能已于 2026-06-28 完整下线（规则 8），但旧表仍有 tenant_id 残留。此问题属于类二十五 25.5 范围，但在色卡重构时需一并清理。
- **修复建议**：色卡重构时删除 tenant_id 字段和索引

#### ✅ 已落实的项

1. **旧表已有 dye_lot_no 字段**（v14 批次 419 T-P0-3 修复）
   - 证据：`/workspace/database/migration/034_v14_production_colorcard_dyelot.sql:32-37`
   ```sql
   -- 3. color_card_borrow_records 表：添加 dye_lot_no（T-P0-3）
   ALTER TABLE color_card_borrow_records ADD COLUMN IF NOT EXISTS dye_lot_no VARCHAR(50);
   CREATE INDEX IF NOT EXISTS idx_color_card_borrow_dye_lot_no ON color_card_borrow_records(dye_lot_no);
   ```
   - 新发放模式表设计中也包含 dye_lot_no 字段（审计计划 10.1.3 第 1515 行）

### 维度结论

- **已落实**：1 项（dye_lot_no 字段已存在）
- **缺陷**：6 项（5 个 P0 + 1 个 P2）
- **总计**：7 项检查点

---

## 类十 维度 2：色卡发放业务规则校验（10.2）

### 检查方法

1. Read `/workspace/backend/src/services/color_card_borrow_service.rs` 检查校验逻辑
2. Grep `color_card_issue|issue_card|mark_received|mark_used` 检索新校验代码
3. 对照审计计划第 2394-2461 行 10.2 节校验矩阵

### 发现

#### ❌ 缺陷项

**缺陷 10.2-1：发放前 5 道闸门校验完全未实现**
- **风险等级：P0**
- **证据**：无 `issue_card` 方法实现（Grep 无结果）
  - 缺失校验：
    1. 色卡存在性校验（`ColorCardNotFound`）
    2. 色卡状态校验（`status = 'active'`）
    3. 色卡库存校验（`total_colors >= quantity`）
    4. 客户有效性校验（`is_active = true`）
    5. 重复发放检查（同一 `(color_card_id, customer_id)` 无 `issued`/`received` 状态记录）
  - 现有 borrow 方法仅校验色卡存在性（`/workspace/backend/src/services/color_card_borrow_service.rs:128-132`），无库存校验、无客户校验、无重复借出检查
- **业务影响**：无法防止无效发放、超量发放、重复发放
- **修复建议**：按审计计划 10.2.1 节实现 5 道闸门校验

**缺陷 10.2-2：新状态流转校验完全未实现**
- **风险等级：P0**
- **证据**：无 `IssueStatus` 枚举（Grep 无结果）
  - 缺失状态机：`Issued → Received → Used` / `Issued → Cancelled` / `Issued/Received → Expired`
  - 缺失校验方法：`can_receive()`、`can_use()`、`can_cancel()`、`is_terminal()`
  - 现有 BorrowStatus 仍是借还状态机（`/workspace/backend/src/services/color_card_borrow_service.rs:39-101`）
- **业务影响**：无法控制发放状态流转，可能出现非法状态转换
- **修复建议**：按审计计划 10.1.6 节实现 IssueStatus 状态机和校验方法

**缺陷 10.2-3：库存联动规则未实现**
- **风险等级：P0**
- **证据**：`/workspace/backend/src/services/color_card_borrow_service.rs:119-165`（borrow 方法无库存扣减）
  - 现有 borrow 方法仅创建借出记录，不扣减色卡库存
  - 缺失：发放时扣减库存、取消发放时恢复库存
- **业务影响**：色卡库存与发放记录不一致，可能导致超量发放
- **修复建议**：按审计计划 10.2.3 节实现库存联动（事务内 + lock_exclusive）

**缺陷 10.2-4：客户专属色卡库规则未实现**
- **风险等级：P1**
- **证据**：无客户色卡库视图、无复购指定同缸号、无历史色卡追溯
- **业务影响**：无法支持客户色卡管理和复购同缸号业务
- **修复建议**：按审计计划 10.2.4 节实现客户专属色卡库

### 维度结论

- **已落实**：0 项
- **缺陷**：4 项（3 个 P0 + 1 个 P1）
- **总计**：4 项检查点

---

## 类十 维度 3：色卡发放与订单集成（10.3）

### 检查方法

1. Grep `sales_order_id.*color_card|color_card.*sales_order` 检索订单关联
2. Grep `复购|同缸号|reorder` 检索复购业务
3. 对照审计计划第 2462-2528 行 10.3 节

### 发现

#### ❌ 缺陷项

**缺陷 10.3-1：色卡发放记录与订单关联完全未实现**
- **风险等级：P1**
- **证据**：
  - 旧 borrow_record 模型无 sales_order_id 字段（`/workspace/backend/src/models/color_card_borrow_record.rs:12-29`）
  - 无 `GET /color-cards/issue?sales_order_id={id}` 接口
  - 无 `GET /sales-orders/{id}/color-cards` 接口
- **业务影响**：无法支持订单驱动发放色卡场景
- **修复建议**：按审计计划 10.3.1 节实现订单关联（强关联 + 弱关联 + 复购关联）

**缺陷 10.3-2：复购指定同缸号业务流程未实现**
- **风险等级：P1**
- **证据**：无复购查询客户历史色卡逻辑、无同缸号面料库存提示
- **业务影响**：无法支持面料行业复购同缸号颜色一致性业务（fabric-industry-research §4.1）
- **修复建议**：按审计计划 10.3.2 节实现复购同缸号流程

**缺陷 10.3-3：色卡发放报表未实现**
- **风险等级：P2**
- **证据**：无发放明细报表、发放汇总报表、客户色卡台账、过期未使用报表、订单关联报表
- **业务影响**：无法支持色卡发放数据分析
- **修复建议**：按审计计划 10.3.3 节实现 5 类报表（支持 .xlsx 导出，规则 3）

**缺陷 10.3-4：色卡成本核算未实现**
- **风险等级：P2**
- **证据**：无色卡制作成本归集、发放成本结转、取消发放恢复、过期损失核算
- **业务影响**：无法支持色卡成本财务管理
- **修复建议**：按审计计划 10.3.4 节实现成本核算（营销费用-色卡发放科目）

### 维度结论

- **已落实**：0 项
- **缺陷**：4 项（2 个 P1 + 2 个 P2）
- **总计**：4 项检查点

---

## 类十 维度 4：色卡发放权限管理（10.4）

### 检查方法

1. Read `/workspace/backend/src/handlers/color_card/borrow.rs` 检查权限校验
2. Grep `v-permission|permission.*color_card` 检索前端权限
3. 对照审计计划第 2529-2567 行 10.4 节权限矩阵

### 发现

#### ❌ 缺陷项

**缺陷 10.4-1：角色权限矩阵未实现**
- **风险等级：P1**
- **证据**：
  - `/workspace/backend/src/handlers/color_card/borrow.rs:27-50`（borrow_color_card 仅用 AuthContext.user_id，无角色校验）
  - 无 `color_card.issue.create`、`color_card.issue.receive`、`color_card.issue.use`、`color_card.issue.cancel`、`color_card.issue.view` 权限码
  - 前端无 `v-permission="'color_card.issue.create'"` 指令使用
- **业务影响**：任何登录用户均可发放色卡，违反最小权限原则
- **修复建议**：按审计计划 10.4.1 节实现 6 角色权限矩阵

**缺陷 10.4-2：数据权限规则未实现**
- **风险等级：P1**
- **证据**：
  - 无销售数据隔离（`WHERE customer_id IN (SELECT customer_id FROM customer_sales_rep WHERE sales_rep_id = ?)`）
  - 无客户门户数据隔离（`WHERE customer_id = ?`）
  - 无成本数据敏感过滤（`cost_amount` 字段按权限过滤）
- **业务影响**：销售可查看所有客户发放记录，客户可查看他人色卡，违反数据隔离原则
- **修复建议**：按审计计划 10.4.2 节实现 3 类数据权限

**缺陷 10.4-3：审计日志要求未实现**
- **风险等级：P1**
- **证据**：`/workspace/backend/src/services/color_card_borrow_service.rs` 无 audit_log 写入
  - 缺失审计日志：发放色卡、标记已收到、标记已使用、取消发放、定时过期
- **业务影响**：色卡发放操作无审计追溯，违反规则 12（敏感操作必须记录审计日志）
- **修复建议**：按审计计划 10.4.3 节实现 5 类操作审计日志

### 维度结论

- **已落实**：0 项
- **缺陷**：3 项（3 个 P1）
- **总计**：3 项检查点

---

## 类十 维度 5：色卡发放定时任务（10.5）

### 检查方法

1. Grep `tokio_cron_scheduler|init_scheduler|color_card_issue_expiry|color_card_stock_warning` 检索定时任务
2. Grep `color_card|色卡|expiry_check|stock_warning|daily_stats` 在 main.rs 中检索
3. Read `/workspace/backend/src/main.rs` 检查定时任务框架
4. 对照审计计划第 2569-2672 行 10.5 节

### 发现

#### ❌ 缺陷项

**缺陷 10.5-1：过期检查定时任务未实现**
- **风险等级：P1**
- **证据**：
  - Grep `color_card_issue_expiry|color_card_stock_warning|color_card_issue_daily_stats` 在 backend/src 无匹配
  - Grep `color_card|色卡|expiry_check|stock_warning|daily_stats` 在 main.rs 无匹配
  - main.rs 定时任务框架只有 admin 缓存清理 + JTI 黑名单清理 + 慢查询采集（`/workspace/backend/src/main.rs:77-80`）
- **业务影响**：已发放色卡超过有效期后不会自动标记为过期，状态机无法闭环
- **修复建议**：按审计计划 10.5.1 节实现 `color_card_issue_expiry_check` 定时任务（每日 02:00 执行）

**缺陷 10.5-2：库存预警定时任务未实现**
- **风险等级：P2**
- **证据**：无 `color_card_stock_warning` 定时任务
- **业务影响**：色卡库存不足时无法自动告警
- **修复建议**：按审计计划 10.5.2 节实现库存预警（每日 08:00 执行，黄色<5/红色<2/禁止=0）

**缺陷 10.5-3：发放统计定时任务未实现**
- **风险等级：P2**
- **证据**：无 `color_card_issue_daily_stats` 定时任务、无 `color_card_issue_daily_stats` 统计表
- **业务影响**：无法自动生成色卡发放日报
- **修复建议**：按审计计划 10.5.3 节实现发放统计（每日 23:00 执行）

**缺陷 10.5-4：定时任务单元测试未实现**
- **风险等级：P2**
- **证据**：无色卡发放相关单元测试（现有测试仅覆盖 BorrowStatus 状态机，`/workspace/backend/src/services/color_card_borrow_service.rs:401-487`）
- **业务影响**：无法验证定时任务正确性
- **修复建议**：按审计计划 10.5.4 节实现 23 项单元测试

#### ✅ 已落实的项

1. **定时任务框架存在**
   - 证据：`/workspace/backend/src/main.rs:77-95`（MAIN_BACKGROUND_TASKS + shutdown_main_background_tasks）
   - 证据：`/workspace/backend/src/main.rs:530-555`（tokio::spawn 后台任务 + 句柄保存）
   - 说明：框架可用，只需添加色卡发放相关定时任务

### 维度结论

- **已落实**：1 项（定时任务框架存在）
- **缺陷**：4 项（1 个 P1 + 3 个 P2）
- **总计**：5 项检查点

---

## 类十 维度 6：色卡发放前端重构（10.6）

### 检查方法

1. LS `/workspace/frontend/src/views/color-cards/`
2. Read `/workspace/frontend/src/api/color-card.ts`
3. Grep `color-card-issue|ColorCardIssue|useColorCardIssue` 检索前端新代码
4. 对照审计计划第 2854-3183 行 10.6 节

### 发现

#### ❌ 缺陷项

**缺陷 10.6-1：前端文件结构完全未创建**
- **风险等级：P0**
- **证据**：Grep `color-card-issue|ColorCardIssue|useColorCardIssue` 无匹配
  - 缺失文件清单：
    - `frontend/src/api/color-card-issue.ts`（10.6.3 API 模块）
    - `frontend/src/types/color-card-issue.ts`（10.6.2 类型定义）
    - `frontend/src/composables/useColorCardIssue.ts`（10.6.4 组合式函数）
    - `frontend/src/views/color-card/IssueList.vue`（10.6.5 列表页）
    - `frontend/src/views/color-card/IssueForm.vue`（10.6.5 发放表单）
    - `frontend/src/views/color-card/IssueDetail.vue`（10.6.5 详情页）
    - `frontend/src/router/modules/color-card-issue.ts`（10.6.6 路由配置）
- **业务影响**：前端完全无法支持色卡发放业务
- **修复建议**：按审计计划 10.6 节完整创建前端文件

**缺陷 10.6-2：前端类型定义未实现**
- **风险等级：P0**
- **证据**：`/workspace/frontend/src/api/color-card.ts:37-49` 仍是旧借出状态枚举
  ```typescript
  export const BORROW_STATUS = {
    borrowed: '借出中',
    returned: '已归还',
    lost: '遗失',
    damaged: '损坏',
  } as const
  ```
  - 缺失：`IssueStatus` 枚举（issued/received/used/expired/cancelled）
  - 缺失：`ISSUE_STATUS_LABELS`、`ISSUE_STATUS_COLORS` 映射
  - 缺失：`CreateIssueRecordDto`、`IssueRecordResponse`、`ListIssueRecordsQuery` 接口
- **业务影响**：前端类型系统不支持发放模式
- **修复建议**：按审计计划 10.6.2 节实现类型定义

**缺陷 10.6-3：前端 API 模块未实现**
- **风险等级：P0**
- **证据**：`/workspace/frontend/src/api/color-card.ts:221-268` 仍是借出管理 API
  - 缺失：`issueCard`、`markReceived`、`markUsed`、`cancelIssue`、`listIssueRecords`、`getIssueRecord` 函数
- **业务影响**：前端无法调用后端发放接口
- **修复建议**：按审计计划 10.6.3 节实现 API 模块

**缺陷 10.6-4：前端视图组件未实现**
- **风险等级：P0**
- **证据**：`/workspace/frontend/src/views/color-cards/borrow.vue` 仍存在（借出页面）
  - 缺失：`IssueList.vue`（必须移除"借出/归还/登记遗失/标记损坏/取消借出"按钮，新增"发放色卡/标记已收到/标记已使用/取消发放"按钮）
  - 缺失：`IssueForm.vue`（发放表单）
  - 缺失：`IssueDetail.vue`（发放记录详情）
- **业务影响**：用户界面仍是借还模式
- **修复建议**：按审计计划 10.6.5 节实现视图组件

**缺陷 10.6-5：前端路由配置未实现**
- **风险等级：P1**
- **证据**：无 `frontend/src/router/modules/color-card-issue.ts`
  - 缺失路由：`/color-card/issue`、`/color-card/issue/create`、`/color-card/issue/:id`
- **业务影响**：用户无法通过导航访问发放页面
- **修复建议**：按审计计划 10.6.6 节实现路由配置

**缺陷 10.6-6：前端权限指令未实现**
- **风险等级：P1**
- **证据**：无 `v-permission="'color_card.issue.create'"` 使用
  - 缺失：`directives/permission.ts` 指令实现
- **业务影响**：前端无按钮级权限控制
- **修复建议**：按审计计划 10.6.7 节实现权限指令

### 维度结论

- **已落实**：0 项
- **缺陷**：6 项（4 个 P0 + 2 个 P1）
- **总计**：6 项检查点

---

## 类十 维度 7：色卡发放 DB 数据迁移脚本（10.7）

### 检查方法

1. Glob `**/migration/*color*` 检索色卡迁移文件
2. Read `/workspace/backend/migrations/20260617000008_create_color_card_borrow_records/up.sql`
3. Grep `color_card_borrow_record_legacy|migrate_borrow_to_issue` 检索迁移脚本
4. 对照审计计划第 3185-3331 行 10.7 节

### 发现

#### ❌ 缺陷项

**缺陷 10.7-1：数据迁移策略未实现**
- **风险等级：P0**
- **证据**：
  - Grep `color_card_borrow_record_legacy|migrate_borrow_to_issue` 无匹配
  - 旧表 `color_card_borrow_records` 未重命名为 `color_card_borrow_record_legacy`
  - 新表 `color_card_issue_record` 未创建
  - 无数据迁移 SQL 脚本（`backend/migration/xxxx_migrate_borrow_to_issue_data.sql`）
- **业务影响**：历史借还记录无法迁移到发放记录表，数据丢失风险
- **修复建议**：按审计计划 10.7.1-10.7.2 节实现数据迁移（5 步：重命名旧表→创建新表→INSERT INTO SELECT→验证→保留 legacy）

**缺陷 10.7-2：代码层旧文件处理未实现**
- **风险等级：P0**
- **证据**：
  - `/workspace/backend/src/models/color_card_borrow_record.rs` 未重命名为 `color_card_borrow_record_legacy.rs`
  - `/workspace/backend/src/models/color_card_borrow_dto.rs` 未删除
  - `/workspace/backend/src/services/color_card_borrow_service.rs` 未删除
  - `/workspace/backend/src/handlers/color_card/borrow.rs` 未删除
  - 旧路由未删除（见缺陷 10.1-3）
- **业务影响**：旧借还代码残留，违反规则 0（真实实现强制）和规则 8（功能/接口/路由必须真实实现）
- **修复建议**：按审计计划 10.7.3 节处理旧文件（重命名 1 个 + 删除 3 个 + 删除旧路由），删除前 grep 确认无引用

**缺陷 10.7-3：迁移回滚方案未实现**
- **风险等级：P2**
- **证据**：无 `backend/migration/xxxx_migrate_borrow_to_issue_rollback.sql` 回滚脚本
- **业务影响**：V15 上线失败时无法回滚
- **修复建议**：按审计计划 10.7.4 节实现回滚方案（TRUNCATE + DROP + RENAME 恢复旧表）

#### ✅ 已落实的项

1. **旧表 color_card_borrow_records 存在且有数据**
   - 证据：`/workspace/backend/migrations/20260617000008_create_color_card_borrow_records/up.sql`（建表脚本）
   - 证据：`/workspace/database/migration/034_v14_production_colorcard_dyelot.sql:32-37`（v14 批次 419 添加 dye_lot_no 字段）
   - 说明：旧表数据需迁移到新表，不能直接 DROP

### 维度结论

- **已落实**：1 项（旧表存在）
- **缺陷**：3 项（2 个 P0 + 1 个 P2）
- **总计**：4 项检查点

---

## 审计结果汇总

| 维度 | P0 | P1 | P2 | P3 | 已落实 | 总检查项 |
|------|----|----|----|----|--------|----------|
| 9.1 批次节奏与 E2E 监控 | 0 | 0 | 0 | 4 | 4 | 8 |
| 9.2 记忆整理与归档 | 0 | 0 | 0 | 3 | 8 | 11 |
| 10.1 色卡业务模式重构 | 5 | 0 | 1 | 0 | 1 | 7 |
| 10.2 色卡发放业务规则校验 | 3 | 1 | 0 | 0 | 0 | 4 |
| 10.3 色卡发放与订单集成 | 0 | 2 | 2 | 0 | 0 | 4 |
| 10.4 色卡发放权限管理 | 0 | 3 | 0 | 0 | 0 | 3 |
| 10.5 色卡发放定时任务 | 0 | 1 | 3 | 0 | 1 | 5 |
| 10.6 色卡发放前端重构 | 4 | 2 | 0 | 0 | 0 | 6 |
| 10.7 色卡发放 DB 数据迁移 | 2 | 0 | 1 | 0 | 1 | 4 |
| **合计** | **14** | **9** | **7** | **7** | **15** | **52** |

---

## 修复优先级队列

### P0 阻塞级（14 项）— 必须首批修复

1. **缺陷 10.1-1**：旧"借出/归还/遗失/损坏"模式完全存在，未重构为"发放"模式
2. **缺陷 10.1-2**：新"发放"模式文件完全未创建（4 个后端文件）
3. **缺陷 10.1-3**：旧路由未删除，新路由未注册
4. **缺陷 10.1-4**：旧表未重命名为 legacy，新表未创建
5. **缺陷 10.1-5**：前端仍是借还模式，未重构为发放模式
6. **缺陷 10.2-1**：发放前 5 道闸门校验完全未实现
7. **缺陷 10.2-2**：新状态流转校验完全未实现
8. **缺陷 10.2-3**：库存联动规则未实现
9. **缺陷 10.6-1**：前端文件结构完全未创建（7 个前端文件）
10. **缺陷 10.6-2**：前端类型定义未实现
11. **缺陷 10.6-3**：前端 API 模块未实现
12. **缺陷 10.6-4**：前端视图组件未实现
13. **缺陷 10.7-1**：数据迁移策略未实现
14. **缺陷 10.7-2**：代码层旧文件处理未实现

### P1 高优先级（9 项）— 第二批修复

1. **缺陷 10.2-4**：客户专属色卡库规则未实现
2. **缺陷 10.3-1**：色卡发放记录与订单关联完全未实现
3. **缺陷 10.3-2**：复购指定同缸号业务流程未实现
4. **缺陷 10.4-1**：角色权限矩阵未实现
5. **缺陷 10.4-2**：数据权限规则未实现
6. **缺陷 10.4-3**：审计日志要求未实现
7. **缺陷 10.5-1**：过期检查定时任务未实现
8. **缺陷 10.6-5**：前端路由配置未实现
9. **缺陷 10.6-6**：前端权限指令未实现

### P2 中优先级（7 项）— 第三批修复

1. **缺陷 10.1-6**：旧表 color_card_borrow_records 仍有 tenant_id 字段（多租户残留）
2. **缺陷 10.3-3**：色卡发放报表未实现
3. **缺陷 10.3-4**：色卡成本核算未实现
4. **缺陷 10.5-2**：库存预警定时任务未实现
5. **缺陷 10.5-3**：发放统计定时任务未实现
6. **缺陷 10.5-4**：定时任务单元测试未实现
7. **缺陷 10.7-3**：迁移回滚方案未实现

### P3 低优先级（7 项）— 流程执行类，V15 修复阶段严格执行

1. **9.1 检查要点 4**：E2E 报告保存到 docs/audits/（artifact 已上传，需手动下载保存）
2. **9.1 检查要点 2**：20/28/29 节奏监控 E2E run 状态（流程要求）
3. **9.1 检查要点 5**：E2E 失败按 P0/P1/P2 优先级纳入后续批次（流程要求）
4. **9.1 检查要点 6**：禁止死等 E2E 完成（流程要求）
5. **9.2 检查要点 1**：每 15 批次整理 .monkeycode/ 所有记忆文件（流程要求）
6. **9.2 检查要点 2**：实时归档：每批 CI 合并后立即归档到 doto-su.md（流程要求）
7. **9.2 检查要点 4**：禁止跨批堆积（流程要求）

---

## 审计总结

### 类九：批次节奏与记忆治理类（2 维度）

**总体评价**：类九 2 维度审计结果良好，无 P0/P1/P2 缺陷。

1. **9.1 批次节奏与 E2E 监控**：E2E 工作流（e2e-batch.yml）配置完整，支持 workflow_dispatch 触发、skip_reason 参数、e2e-skipped job、artifact 上传、独立编译。监控节奏（20/28/29）和报告保存为流程要求，V15 修复阶段开始后需严格执行。

2. **9.2 记忆整理与归档**：6 个记忆文件分工明确且符合规则（MEMORY.md=规则 / doto.md=未完成任务 / doto-su.md=已完成任务 / CHANGELOG.md=一句话总结 / audit_assignment.md=审计任务 / bug.md=空文件）。历史归档目录完整（docs/archives/ 按日期保留）。每 15 批次整理和实时归档为流程要求，V15 修复阶段需严格执行。

### 类十：色卡发放业务规则修正专项（5 维度，实际 7 子维度）

**总体评价**：类十色卡发放专项完全未实现，是 V15 审计中缺陷最严重的类别，共 14 个 P0 阻塞级缺陷。

1. **10.1 色卡业务模式重构**：旧"借出/归还/遗失/损坏"模式完全存在，新"发放"模式完全未实现。后端 4 个新文件、前端 7 个新文件、新表、新路由全部缺失。旧文件未处理（重命名/删除）。

2. **10.2 色卡发放业务规则校验**：5 道闸门校验、状态流转校验、库存联动、客户专属色卡库全部未实现。

3. **10.3 色卡发放与订单集成**：订单关联、复购同缸号、报表、成本核算全部未实现。

4. **10.4 色卡发放权限管理**：角色权限矩阵、数据权限、审计日志全部未实现。

5. **10.5 色卡发放定时任务**：过期检查、库存预警、发放统计、单元测试全部未实现（定时任务框架存在）。

6. **10.6 色卡发放前端重构**：前端文件结构、类型定义、API 模块、视图组件、路由配置、权限指令全部未实现。

7. **10.7 色卡发放 DB 数据迁移脚本**：数据迁移策略、代码层旧文件处理、回滚方案全部未实现。

### 修复建议

色卡发放专项需作为 V15 修复阶段的重点工程，按以下顺序分批修复：

1. **第 1 批（P0 后端基础）**：10.1.3 新表创建 + 10.1.4 新模型 + 10.1.5 新 DTO + 10.1.6 新服务 + 10.1.7 新 handler + 新路由 + 10.7.1 数据迁移
2. **第 2 批（P0 前端基础）**：10.6.2 类型定义 + 10.6.3 API 模块 + 10.6.4 组合式函数 + 10.6.5 视图组件 + 10.6.6 路由
3. **第 3 批（P0 旧代码清理）**：10.7.2 旧文件处理（重命名 1 + 删除 3 + 删除旧路由）+ 10.1-6 tenant_id 残留清理
4. **第 4 批（P1 业务规则）**：10.2 校验矩阵 + 10.3 订单集成 + 10.4 权限管理 + 10.5.1 过期检查定时任务 + 10.6.5/6 前端路由权限
5. **第 5 批（P2 增强功能）**：10.3.3/4 报表成本 + 10.5.2/3/4 库存预警统计测试 + 10.7.3 回滚方案
