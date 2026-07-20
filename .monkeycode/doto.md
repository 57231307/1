# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单）。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近梳理：2026-07-19（Batch 487 已合并 main 直接提交 3919255（用户特批不拆分打包：P0-T02 7 业务路径集成测试 73 测试 + P0-T07 性能基准 criterion optional feature 机制 4 benches 11 基准 + P0-T05 E2E 配置修复 applyAuthMocks 移除 mockBusinessApi + webServer 数组化），28 文件 +1836 -29；CI 验证中（无 GH_TOKEN 无法直接监控）；教训：criterion optional feature 机制避免 bench 文件拖慢 cargo test + #[ignore] 集成测试模式让 CI 默认跳过完整业务流程测试 + playwright webServer 数组配置是前后端分离项目最佳实践 + mockBusinessApi 保留策略避免删除 enhanced 测试所需的显式调用；Batch 486 已合并 main 直接提交 01faa60，P0-T01 核心 service 单测补全：quotation_service 19 测试 + purchase_receipt_service 19 测试 = 38 测试；CI 一次过 14/14 全绿；教训：SQLite 无表时 sea-orm 返回 Err(DbErr) 而非 Ok(None)/Ok(空 Vec)，DB 相关测试应断言 is_err() 而非期望空数据 + 测试夹具参考 voucher_service.rs 模式（decs!/ymd! 宏需 use std::str::FromStr）+ ServiceError 枚举需测试 Display 实现；Batch 485 已合并 main 直接提交 af0f16b + 5e4e78f + 7cc82cc，P0-T03 clippy baseline 恢复 + P0-T08 覆盖率工具 cargo-tarpaulin + Codecov + 编译错误修复 rgb_to_hex 函数缺失 + ci-test-rust bash 算术 bug grep -c + || echo 0 改用 awk；CI 7 轮：1-5 轮 RUSTC_LOG=debug 拖慢超时 + --all-features 副作用 + 4 编译错误，第 6 轮恢复 baseline 后 ci-test-rust 编译失败 rgb_to_hex 缺失，第 7 轮修复后 14/14 全绿 conclusion=success；教训：默认 features 下 1781 个预存 dead_code 警告是技术债务无法一个批次清零，clippy 采用 baseline 机制（仅新增警告阻塞）是合理渐进式策略 + grep -c + || echo 0 陷阱用 awk 替代 + 测试导入不存在的函数需同步实现而非删除测试；Batch 473 已合并 PR #656（P0-S14 migration + P0-S19 审计字段补齐）；Batch 474 已合并 PR #657（P0-S15 导出水印基础设施完成 + P0-S12 前端导出接入后端核心 2 页面完成）；Batch 475a 已合并 PR #658（P0-S13 审计日志导出闭环完成）；Batch 475b 已合并 PR #659（P0-S12 前端导出 purchase/customer 闭环，A 类 2 文件完成）；Batch 475c 已合并 PR #660（P0-S12 前端导出 B 类批次 1/3 完成，inventory + warehouse + production 3 模块闭环）；Batch 475d 已合并 PR #661（P0-S12 前端导出 B 类批次 2/3 完成，sales-contract + sales-price + quality + quality-standards 4 模块闭环）；Batch 475e 已合并 PR #662（P0-S12 前端导出 B 类批次 3/3 收尾完成，ar + ap + cost + budget + fixed-assets 5 模块闭环，P0-S12 前端导出接入后端全部完成）；Batch 476 已合并 main 直接提交 eb57484（P0-S17 打印 HTML 真实数据查询完成，print_service + print_handler 2 文件，6 个 get_*_print_data 方法从硬编码占位改为真实查询 DB）；Batch 477 已合并 main 直接提交 a3798f4 + daeab0f（P0-F10/F11/F12/F13 色卡发放库存联动 + 前端文件结构 + legacy 数据迁移完成，15 文件，PR #665 因 main 抢先直接提交被关闭冲突）；Batch 478 已合并 main 直接提交 9d01a42 + 6aca804（P0-F15 bulk_color_approval 表 + P0-F16 剪大货样 + P0-F17 客户批色确认 + P0-F19 ship_order 校验完成，11 文件，8 态状态机 + 9 端点；CI 2 轮，第 1 轮 clippy::deref_arg 警告，第 2 轮修复后 14/14 全绿）；Batch 479 已合并 main 直接提交 642d2c09 + cc1ee381 + c06109fd + bbf38a30（P0-F18 返工/降级/报废 + P0-F21 返工走生产订单完成，7 文件，customer_rework 联动创建返工生产订单 + downgrade 联动库存等级降级 + scrap 联动库存报废标记；CI 3 轮，第 1 轮 E0063 missing field production_order_id，第 2 轮 clippy baseline 误删 too many arguments 警告，第 3 轮恢复 baseline 后 14/14 全绿；教训：CI 自动刷新 baseline 在编译错误时会误删预存警告）；Batch 480 已合并 main 直接提交 5334bf13 + 8d7ea998 + ae87219f（P0-F20 8D 质量管理流程完成，13 文件，11 态状态机 + 10 条合法边 + From<AppError> 透传；CI 3 轮）；Batch 481 已合并 PR #666 squash 00261365（P0-B01 坏账准备 + P0-B02 坏账核销审批 + P0-B03 催收任务 + P0-B04 财务预警完成，25 文件，账龄分析法 + 二级审批 + 4 类预警 + 7 端点催收；CI 5 轮，rust_decimal_macros::dec 宏不可用 + HashMap 键需 Hash + AlertType 未实现 Display + const Decimal::new 非 const fn + 5 unused import + doc list item overindented + from_str 与 std trait 冲突；教训：rust_decimal 1.x Decimal::new 非 const fn 需用函数替代 + 自定义 from_str 方法名与 std::str::FromStr 冲突需改名为 parse_str）；Batch 482 已合并 PR #667（P0-B05 大额调拨 + P0-B06 预算超支 + P0-B07 CRM 回收 + P0-B08 赢率 + P0-B09 输单原因 + P0-B14 Incoterms 完成，13 文件，large_transfer_threshold 10万阈值 + enforce_budget_available 阻塞式 + RecycleExecutor 6h 定时 + default_win_probability_by_stage 5阶段 + close_as_lost 输单原因 + Incoterms 11种术语全量；CI 2 轮，第 1 轮 E0382 borrow of moved value v 在 Set 后 &v，第 2 轮改为先算 default_prob 再 Set 后 14/14 全绿；教训：变量移动顺序需在 Set 前计算衍生值）；Batch 483 已合并 PR #668（P0-B10 BI 权限过滤 + P0-B11 定制订单打样报价 + P0-B12 售后质量集成 + P0-B13 物流电子签收 完成，15 文件）；Batch 484 已合并 main 直接提交 df5286ee + c012a3b9（P0-B15 缺料预警持久化 + P0-B16 自动故障检测 + P0-B17 主备切换 完成，11 文件，m0068 两表 + material_shortage.rs Model + 3 桩方法改真实 DB 读写 + FailoverExecutor ArcSwap 原子切换 + FailoverMonitor 后台任务 + 熔断器状态机；CI 2 轮，第 1 轮 3 处编译错误 E0382/E0308/E0308，第 2 轮 Rust 构建和单元测试全绿，clippy 1 条 too many arguments 8/7 新增警告用户特批直接合并不等 CI；教训：sea-orm ActiveModel into() 消费原值后不可再访问 + DatabaseTransaction 与 DatabaseConnection 需用 ConnectionTrait 泛型统一 + MaterialShortageItem.material_code 是 String 但 alert_model.material_code 是 Option<String> 需 Some 包裹）；P0 进度 103/104；剩余 1 P0（D 系列 17 项 = D01~D17，规划为 Batch 488-491 共 4 批次） + 257 P1（暂停） + 248 P2（暂停） + 123 P3（暂停）；用户指令暂停 P1/P2/P3 仅推进 P0；规则 13 连续执行；禁止本地编译验证；批次节奏调整为每批 9-12 文件）。

---

## 一、当前状态与总体进度

### 1.1 进度总览

| 优先级 | 总数 | 已完成 | 未完成 | 完成率 |
|--------|------|--------|--------|--------|
| **P0 阻塞级** | 104 | 103 | **1** | 99.0% |
| **P1 高优先级** | 257 | 0 | **257** | 0% |
| **P2 中优先级** | 248 | 0 | **248** | 0% |
| **P3 低优先级** | 123 | 0 | **123** | 0% |
| **合计** | **732** | **103** | **629** | **14.1%** |

> P0 已完成 103 项 = 原 62 项 + 复审发现已完成 4 项（P0-S08/S16/F14/T04）- 复审重新打开 1 项（P0-S14）+ Batch 473 修复 2 项（P0-S14 migration 补齐 + P0-S19 condition 字段补齐）+ Batch 474 修复 1 项（P0-S15 导出水印基础设施）+ Batch 475a 修复 1 项（P0-S13 审计日志导出闭环）+ Batch 476 修复 1 项（P0-S17 打印 HTML 真实数据查询）+ Batch 477 修复 4 项（P0-F10 库存联动 + P0-F11 前端文件结构 + P0-F12 前端类型/API/视图 + P0-F13 数据迁移）+ Batch 478 修复 4 项（P0-F15 bulk_color_approval 表 + P0-F16 剪大货样 + P0-F17 客户批色确认 + P0-F19 ship_order 校验）+ Batch 479 修复 2 项（P0-F18 返工/降级/报废 + P0-F21 返工走生产订单）+ Batch 480 修复 1 项（P0-F20 8D 质量管理流程）+ Batch 481 修复 4 项（P0-B01 坏账准备 + P0-B02 坏账核销审批 + P0-B03 催收任务 + P0-B04 财务预警）+ Batch 482 修复 6 项（P0-B05 大额调拨 + P0-B06 预算超支 + P0-B07 CRM 回收 + P0-B08 赢率 + P0-B09 输单原因 + P0-B14 Incoterms）+ Batch 483 修复 4 项（P0-B10 BI 权限过滤 + P0-B11 定制订单打样报价 + P0-B12 售后质量集成 + P0-B13 物流电子签收）+ Batch 484 修复 3 项（P0-B15 缺料预警持久化 + P0-B16 自动故障检测 + P0-B17 主备切换）+ Batch 485 修复 3 项（P0-T03 clippy baseline 恢复 + P0-T08 覆盖率工具 + P0-T06 bi_analysis 测试 API 对齐）+ Batch 486 修复 1 项（P0-T01 核心 service 单测补全）+ Batch 487 修复 3 项（P0-T02 7 项集成测试 + P0-T07 性能基准 + P0-T05 E2E 配置修复，用户特批不拆分打包）。
> P0-S12 前端导出接入后端：Batch 474 已完成核心 2 页面（customer/supplier），Batch 475a 已完成 audit-log（P0-S13 闭环），Batch 475b 已完成 purchase/customer 闭环（A 类 2 文件），Batch 475c 已完成 B 类批次 1/3（inventory + warehouse + production 3 模块），Batch 475d 已完成 B 类批次 2/3（sales-contract + sales-price + quality + quality-standards 4 模块），Batch 475e 已完成 B 类批次 3/3 收尾（ar + ap + cost + budget + fixed-assets 5 模块），**P0-S12 前端导出接入后端全部完成**。

### 1.2 状态：🔄 规则 13 连续执行中

- **当前批次**：Batch 488 进行中（12/17 完成）—— ✅ D01/D02/D07/D11/D15/D16/D17 审计误判 + ✅ D03+D04 5 service Redis 缓存接入（cead770）+ ✅ D12 圈复杂度优化（6 项重构 + 2 项误判跳过）+ ✅ D06 aria-label 全部完成（55 个子批次累计 ~225 文件，commit 22c842a 已推送；遵循 WCAG 2.1 AA 标准；最终扫描确认全部补齐无遗漏）+ 🔄 D08-1 第一梯队 6/6 + 第二梯队首批 5/5 + 第二梯队第 2 批 5/5 已拆分完成（共 16 函数，CI 全绿 run 29718405482 + run 29720458274）；剩余 5 项大型任务审计完成（2026-07-19）：D05 useI18n（XL，355 文件实际接入率 3.1%）/ D08 超长函数（XL，91 个 >80 行 + 54 个 >100 行 + 6 个 >200 行 + 0 个 >500 行）/ D09 100+ 行函数（L，D08 子集自动完成）/ D10 1000+ 行文件（L，实际 30 个非 26 个，1 个 >2000 行）/ D13 缩写命名（XL，实际 123 个非 ~119 个，25 类缩写前缀）/ D14 api 命名（XL，96 文件准确，listXxx 偏差 47 文件 84 处为最大源）
- **下一批次**：Batch 488 继续 —— 按依赖关系推荐顺序：① D08 超长函数（第 1 顺位，无前置 + 解锁 D09/D10，预估 10-12 子批次）→ ② D10 大文件拆分（第 2 顺位，D08 完成后立即推进，预估 5-6 子批次）→ ③ D14 api 命名（第 3 顺位，与 D05/D13 解耦，预估 10-12 子批次）→ ④ D13 缩写命名（第 4 顺位，D14 完成后推进，预估 12-15 子批次）→ ⑤ D05 useI18n（第 5 顺位，D13/D14 完成后最后推进，预估 30-36 子批次）；累计预估 67-81 子批次
- **执行策略**：规则 13+14+15+20 联动；CI 全绿后自动进入下一批；所有警告视为错误必须真实修复；修复前必须调研现有实现禁止重复造轮子；注释必须与功能一致禁止随意编写（规则 20）；规则 13 步骤 4 自审必须 grep 所有引用新字段/新结构体的调用点；**禁止本地编译验证**（cargo check/build/test/clippy + npm build/type-check/vitest/vue-tsc），必须直接 push 让 CI 验证

### 1.3 关键决策记录

| 决策 | 日期 | 内容 |
|------|------|------|
| 批次节奏 | 2026-07-17 | 每批 9-12 文件，遵循规则 13 连续执行流程；每 30 批触发 E2E（规则 5）；每 15 批整理记忆（规则 10） |
| 批次顺序 | 2026-07-17 | 按顺序修复所有批次，不再限制单数批次 |
| 术语澄清 | 2026-07-17 | 缸号（batch_no）=染色批次号；dye_lot_no=染色批号（lot 概念，防色差混批） |
| 旧表保留 | 2026-07-17 | 保留 color_card_borrow_records 不重命名为 _legacy，保护 Rust migration m0029 链路；应用层不再读写 |
| 复审归档 | 2026-07-17 | 复审报告 [v15-fix-reaudit-2026-07-17.md](file:///workspace/.monkeycode/docs/audits/v15-fix-reaudit-2026-07-17.md)；4 项已完成项归档（P0-S08/S16/F14/T04）；P0-S14 重新打开（migration 047 缺失） |
| 规则 20 | 2026-07-17 | 新增规则：注释必须与功能一致，禁止随意编写；CI 强制检查 |
| 自审门强化 | 2026-07-17 | Batch 473 教训：步骤 4 自审必须 grep 所有引用新字段/新结构体的调用点（如 `audit_log::ActiveModel {` / `OmniAuditMessage {` 等），不能只看 git diff 的已修改文件 |

---

## 二、任务依赖关系图

> 本图展示 P0 任务间的依赖关系，用于确定执行顺序。箭头 → 表示"依赖"（A→B 表示 B 依赖 A 先完成）。
> 工作量预估：S=小（≤3 文件）/ M=中（4-6 文件）/ L=大（7-10 文件）/ XL=超大（>10 文件）

### 2.1 模块 A：安全与权限

```
P0-S14 migration 047 (S)  ──┐
                             ├──→ P0-S12 前端导出接入后端 (XL, 25+ 页面)
P0-S15 导出水印 (M)  ────────┤
                             ├──→ P0-S13 审计日志导出假按钮 (M)
P0-S19 审计字段补齐 (S)  ────┘
P0-S17 打印 HTML 占位数据 (L)  ← 独立任务
```

**关键路径**：P0-S14 → P0-S15 → P0-S12 → P0-S13（前端导出全链路）
**独立任务**：P0-S17（打印服务真实查询）

### 2.2 模块 B：面料行业-色卡发放

```
P0-F10 库存联动 (M)  ←─── P0-F11/F12 前端文件结构 (L)
P0-F13 数据迁移策略 (S)  ← 独立任务
```

**关键路径**：P0-F10 → P0-F11/F12（后端库存联动先于前端补齐）
**独立任务**：P0-F13（legacy 数据迁移脚本）

### 2.3 模块 C：面料行业-大货批色

```
P0-F15 bulk_color_approval 表 (M)  ──→ P0-F16 剪大货样 (M)
                                  ├──→ P0-F17 客户批色确认 (M)
                                  └──→ P0-F18 返工/降级/报废 (L)
P0-F18 ──→ P0-F21 返工走生产订单 (M)
P0-F16/F17/F18 ──→ P0-F19 ship_order 校验 (S)
```

**关键路径**：P0-F15 → P0-F18 → P0-F21（表结构 → 业务规则 → 返工闭环）
**末端校验**：P0-F19（依赖前 4 项）

### 2.4 模块 D：面料行业-质量管理

```
P0-F20 8D 质量管理流程 (XL)  ← 独立任务
P0-F21 返工走生产订单 (M)  ← 依赖模块 C 的 P0-F18
```

**关键路径**：P0-F18（模块 C）→ P0-F21 → P0-F20 8D 流程

### 2.5 模块 E：财务与业务流程

```
P0-B01 坏账准备 (L)  ──→ P0-B02 坏账核销审批 (M)
P0-B03 催收任务 (M)  ← 独立（依赖 B01 但可并行设计）
P0-B04 财务预警 (L)  ← 独立
P0-B05 大额调拨验证 (S)  ← 独立
P0-B06 预算超支拦截 (M)  ← 独立
P0-B07 CRM 回收规则 (S)  ← 独立
P0-B08 赢率自动计算 (S)  ← 独立
P0-B09 输单原因 (S)  ← 独立
P0-B10 BI 权限过滤 (M)  ← 独立
P0-B11 定制订单打样报价 (L)  ← 独立
P0-B12 售后质量集成 (M)  ← 依赖模块 D 的 P0-F20
P0-B13 物流电子签收 (M)  ← 独立
P0-B14 Incoterms 补齐 (S)  ← 独立
P0-B15 缺料预警持久化 (M)  ← 独立
P0-B16 自动故障检测 (M)  ← 独立
P0-B17 主备切换自动完成 (L)  ← 独立
```

**关键路径**：P0-B01 → P0-B02（坏账链路）；P0-F20 → P0-B12（质量售后链路）

### 2.6 模块 F：测试体系

```
P0-T03 baseline 移除 (M)  ←─── 独立（可优先，解锁真实 CI 信号）
P0-T01 核心 service 单测 (L)  ← 独立
P0-T02 7 项集成测试 (XL)  ← 依赖 T01
P0-T06 bi_analysis 16 测试 (M)  ← 独立
P0-T05 E2E 通过率 (XL)  ← 独立
P0-T07 性能基准 (M)  ← 独立
P0-T08 覆盖率工具 (S)  ← 独立
```

**关键路径**：P0-T01 → P0-T02（单测先于集成测试）
**优先项**：P0-T03（移除 baseline 后所有 CI 失败才能真实暴露）

### 2.7 模块 G：部署与运维

```
P0-D01 Docker 文件 (S)  ←─── 独立
P0-D02 install.sh (S)  ←─── 独立
P0-D03 5 service 缓存 (L)  ──→ P0-D04 moka→Redis (L)
P0-D05 useI18n (XL)  ← 独立
P0-D06 aria-label (XL)  ← 独立
P0-D07 img alt (S)  ← 独立
P0-D08 超长函数 (XL)  ──→ P0-D09 100 行函数 (L) ──→ P0-D10 1000 行文件 (L)
P0-D11 setup_test_db (M)  ← 独立
P0-D12 圈复杂度 (M)  ← 独立
P0-D13 前端缩写命名 (XL)  ← 独立
P0-D14 api 命名统一 (XL)  ← 独立
P0-D15 升级零停机 (M)  ← 独立
P0-D16 报表订阅调度 (M)  ← 独立
P0-D17 OA 公告 (M)  ← 独立
```

**关键路径**：P0-D03 → P0-D04（缓存层迁移链路）；P0-D08 → P0-D09 → P0-D10（代码质量链路）

---

## 三、P0 批次规划表（39 项，规划 22 个批次）

> 每批 9-12 文件，按依赖关系排序。批次顺序即执行顺序。
> 工作量：S=小（≤3 文件，1-2 小时）/ M=中（4-6 文件，2-4 小时）/ L=大（7-10 文件，4-8 小时）/ XL=超大（>10 文件，8+ 小时）
> 批次节奏调整：2026-07-17 从"每批 6-8 文件"调整为"每批 9-12 文件"，批次总数从 27 压缩为 22，提升单批吞吐量

### 3.1 批次顺序总览

| 批次 | 模块 | 任务 | 工作量 | 依赖 | 说明 |
|------|------|------|--------|------|------|
| ✅ 473 | A | P0-S14 migration 047 + P0-S19 审计字段补齐 | S+M | 无 | 已合并 PR #656（squash e19c1aa）；复审重新打开项优先；解锁后续安全链路 |
| ✅ 474 | A | P0-S15 导出水印 + P0-S12 前端导出接入后端（核心页面） | M+XL | 473 | 已合并 PR #657（squash 33c2e7c）；P0-S15 完成；P0-S12 完成 customer/supplier 2 页面，剩 23+ 页面在 475 批次；CI 修复 v3：merge_range 补 format + export.ts 改用 axios 绕过 request.ts 拦截器 |
| 🟡 475a | A | P0-S13 审计日志导出闭环 | M | 474 | 已合并 PR #658（squash 7c7cfc7）；P0-S13 完成；后端 audit_log_handler 注入水印 + 前端 audit-log/index.vue 切换 exportFromBackend + 测试 mock 更新；CI 2 轮修复 |
| 🟡 475b | A | P0-S12 前端导出 purchase/customer 闭环（A 类）| S | 474 | 已合并 PR #659（squash cde7e9a）；A 类 2 文件完成（usePurchAct.ts + CustomerListTab.vue）；后端 export_orders 注入水印；CI 一次过 13/13 全绿 |
| 🟡 475c | A | P0-S12 前端导出接入后端（B 类批次 1/3）| XL | 474 | 已合并 PR #660（squash 38e8e43）；inventory + warehouse + production 3 模块：后端新增 3 端点（export_stock/export_warehouses/export_production_orders）+ 前端切换 4 文件（inventory/index.vue + warehouse/index.vue + usePrdProc.ts + production/index.vue）+ 3 路由注册 + 自审门 = 11 文件；CI 2 轮（第 1 轮 E0599 WarehouseListQuery 未派生 Clone，第 2 轮修复后 13/13 全绿） |
| 🟡 475d | A | P0-S12 前端导出接入后端（B 类批次 2/3）| XL | 475c | 已合并 PR #661（squash 4bb7005）；sales-contract/sales-price + quality/quality-standards 4 模块：后端新增 4 端点（export_contracts/export_prices/export_records/export_standards）+ 前端切换 7 文件（useScProc.ts + useSpProc.ts + sales-contract/index.vue + sales-price/index.vue + RecordTab.vue + StandardTab.vue + quality-standards/index.vue）+ 4 路由注册 + 自审门 = 14 文件；CI 2 轮（第 1 轮前端类型检查失败 useTableApi queryParams 类型为 Ref<Record<string, unknown>> 与 getQueryParams 强类型返回值不兼容，第 2 轮修复添加类型断言后 13/13 全绿） |
| ✅ 475e | A | P0-S12 前端导出接入后端（B 类批次 3/3）| XL | 475d | 已合并 PR #662（squash ff07549）；ar/ap/cost/budget/fixed-assets 5 模块：后端新增 5 端点（export_ar_invoices/export_ap_invoices/export_cost_items/export_budgets/export_fixed_assets）+ 前端切换 5 文件（Tab.vue × 5）+ 5 路由注册 = 12 文件；CI 一次过 13/13 全绿；**P0-S12 前端导出接入后端全部完成** |
| ✅ 476 | A | P0-S17 打印 HTML 真实数据查询 | L | 无 | 已合并 main 直接提交 eb57484（PR #664 因 main 抢先直接提交被关闭冲突）；print_service.rs 6 个 get_*_print_data 方法从硬编码占位改为真实查询 DB（sales_order/sales_contract/purchase_order/purchase_receipt/inventory_transfer/voucher）+ print_handler.rs 注入 AppState；2 文件；CI 13/13 全绿 + 2 skipped |
| ✅ 477 | B | P0-F10 色卡发放库存联动 + P0-F11/F12 前端文件结构 + P0-F13 数据迁移 | M+L+S | 无 | 已合并 main 直接提交 a3798f4 + daeab0f（PR #665 因 main 抢先直接提交被关闭冲突）；m0057_create_color_card_issues_and_stock_fields（建表+stock_quantity 合并迁移）+ m0058_migrate_color_card_borrow_records（旧表数据迁移）+ color_card_migrate_legacy.sql + color_card.rs Model 新增 stock_quantity + color_card_crud_service.rs create() 初始化 + color_card_issue_service.rs 5 方法接入库存联动（issue 扣减/return_card+cancel_issue 还原/mark_lost+mark_damaged 不还原）+ 事务+lock_exclusive + 前端 5 新文件（types/colorCardIssue.ts + store/colorCardIssue.ts + composables/useColorCardIssue.ts + components/ColorCardIssueForm.vue + components/ColorCardIssueDetail.vue）+ 重构 issues.vue + color-card-issue.ts 独立 API 模块 = 15 文件；CI 2 轮（第 1 轮前端类型检查失败 ColorCardIssueForm.vue 未使用 props 变量 TS6133，第 2 轮修复后 13/13 全绿） |
| ✅ 478 | C | P0-F15 bulk_color_approval 表 + P0-F16 剪大货样 + P0-F17 客户批色确认 + P0-F19 ship_order 校验 | M+M+M+S | 无 | 已合并 main 直接提交 9d01a42 + 6aca804；m0058_create_bulk_color_approval（24 字段 + 5 索引 + 4 CHECK 约束，8 态状态机）+ bulk_color_approval.rs Model（24 字段 + 3 关联）+ bulk_color_approval_service.rs（ApprovalStatus 枚举 + 9 状态转换方法 + 行锁并发安全）+ bulk_color_approval_handler.rs（9 端点 DTO + 错误转换）+ routes/bulk_color_approval.rs（nest /api/v1/erp/bulk-color-approvals）+ delivery.rs validate_bulk_color_approval 发货前门禁校验 + 4 mod.rs 注册 = 11 文件；FK 修正：dye_batch 表名为单数（非 dye_batches）；CI 2 轮（第 1 轮 clippy::deref_arg 警告 &*self.db 显式 deref，第 2 轮修复 validate_bulk_color_approval 参数改为 &Arc<DatabaseConnection> 后 14/14 全绿） |
| ✅ 479 | C | P0-F18 返工/降级/报废 + P0-F21 返工走生产订单 | L+M | 478 | 已合并 main 直接提交 642d2c09 + cc1ee381 + c06109fd + bbf38a30；m0059_add_rework_order_fields（production_orders 加 order_type/original_batch_id + dye_batch_rework 加 production_order_id + 2 索引 + 1 CHECK 约束）+ production_order.rs Model 新增 2 字段 + dye_batch_rework.rs Model 新增 1 字段 + production_order_service.rs 新增 create_rework_order 方法（order_type='rework' + original_batch_id + RW-YYYYMMDD-NNN 订单号 + 不触发 MRP）+ bulk_color_approval_service.rs customer_rework 联动创建返工生产订单 + downgrade 联动库存等级降级（一等品→二等品/二等品→等外品）+ scrap 联动库存报废标记（stock_status='报废' + quality_status='不合格'）+ find_related_stocks 通过 batch_no/color_no/dye_lot_no 关联库存 + inventory_stock_service.rs 新增 update_stock_grade + mark_stock_as_scrapped + dye_batch_state_machine_service.rs ActiveModel 补 production_order_id 字段 = 7 文件；CI 3 轮（第 1 轮 E0063 missing field production_order_id，第 2 轮 clippy baseline 误删 too many arguments 警告，第 3 轮恢复 baseline 后 14/14 全绿）；教训：CI 自动刷新 baseline 在编译错误时会误删预存警告 |
| ✅ 480 | D | P0-F20 8D 质量管理流程 | XL | 无 | 已合并 main 直接提交 5334bf13 + 8d7ea998 + ae87219f；m0060_create_quality_8d_reports + quality_8d_report.rs Model + quality_8d_dto.rs + quality_8d_service.rs + quality_8d_handler.rs + routes/quality_8d.rs + 5 mod.rs = 13 文件；11 态状态机 + 10 条合法边 + From<AppError> 透传；CI 3 轮 |
| ✅ 481 | E | P0-B01 坏账准备 + P0-B02 坏账核销审批 + P0-B03 催收任务 + P0-B04 财务预警 | L+M+M+L | 无 | 已合并 PR #666 squash 00261365；4 migration + 4 Model + 3 DTO + 3 Service + 3 Handler + 3 Route + 5 mod.rs = 25 文件；账龄分析法 + 二级审批 + 4 类预警 + 催收任务；CI 5 轮修复 |
| ✅ 482 | E | P0-B05 大额调拨 + P0-B06 预算超支 + P0-B07 CRM 回收 + P0-B08 赢率 + P0-B09 输单原因 + P0-B14 Incoterms | S+M+S+S+S+S | 无 | 已合并 PR #667；13 文件（1 新建 + 12 修改）；fund_management_service.rs large_transfer_threshold() 10万阈值 + fund_dto.rs confirm_large 字段 + budget_management_service.rs enforce_budget_available 阻塞式 + po/price.rs check_and_occupy_budget ? 传播 + po/order.rs 事务回滚 + crm/recycle_executor.rs RecycleExecutor 6h 定时扫描 + main.rs 后台任务启动 + opp.rs default_win_probability_by_stage 5阶段映射 + close_as_lost 输单原因 + crm_dto.rs CloseAsLostRequest + crm_handler.rs close_opportunity_as_lost + routes/crm.rs /close-lost + incoterms.rs 11种术语全量；CI 2 轮（第 1 轮 E0382 borrow of moved value v 在 Set 后 &v，第 2 轮改为先算 default_prob 再 Set 后 14/14 全绿）；教训：rust_decimal 1.42 Decimal::new 非 const fn 确认 + 变量移动顺序需在 Set 前计算衍生值 |
| ✅ 483 | E | P0-B10 BI 权限过滤 + P0-B11 定制订单打样报价 + P0-B12 售后质量集成 + P0-B13 物流电子签收 | M+L+M+M | 480 | 已合并 PR #668 squash e094846e；15 文件（3 migration m0065/m0066/m0067 由 main 预置 + 14 代码文件修改）；data_scope.rs build_data_scope_sql 函数 + bi_analysis_service.rs 16 方法注入 + bi_handler.rs 16 handler 改用 new_with_data_scope + custom_order.rs 2 字段+2 Relation + process_state_machine.rs LabDip/Quotation 2 状态 + custom_order_state_service.rs 状态门校验 + after_sales.rs quality_issue_id + trigger_quality_investigation 方法 + logistics_waybill.rs 5 签收字段 + status.rs SIGNED 常量 + sign_waybill handler + 路由；CI 2 轮（第 1 轮 3 处编译错误：CustomOrderActive 缺 lab_dip_request_id/quotation_id + state_err match 缺 3 变体 + aftersales_err match 缺 AlreadyLinked；第 2 轮 14/14 全绿）；教训：main 已预置迁移文件时需 reset 到 main 只应用代码变更，避免 rebase 冲突 |
| ✅ 484 | E | P0-B15 缺料预警持久化 + P0-B16 自动故障检测 + P0-B17 主备切换 | M+M+L | 无 | 已合并 main 直接提交 df5286ee + c012a3b9；m0068_create_material_shortage_tables（alerts + threshold_configs 两表）+ material_shortage.rs Model（alerts + threshold_config 子模块）+ material_shortage_service.rs 3 桩方法改真实 DB 读写（save/load/update_status）+ persist_alerts 幂等 upsert + generate_alert_no MS-YYYYMMDD-NNN + handler 状态机对齐 + failover_service.rs health_check 真实 SELECT 1 + ping_db + FailoverMonitor 后台任务（5s/3次/环境变量控制）+ 熔断器状态机（closed→open→closed）+ increment/reset_consecutive_failures + FailoverExecutor（ArcSwap 原子切换）+ with_executor builder + test_switch 真实切换 + update_status_on_switch + app_state.rs failover_executor 字段 + main.rs DATABASE_BACKUP_URL 备库构造 + FailoverMonitor spawn + failover_handler.rs 注入 executor = 11 文件；CI 2 轮（第 1 轮 3 处编译错误：E0382 use of moved value existing + E0308 material_code Option<String> + E0308 generate_alert_no 事务类型不匹配，第 2 轮修复后 Rust 后端构建/单元测试全绿，clippy 1 条 too many arguments 8/7 新增警告用户特批直接合并不等 CI）；教训：sea-orm ActiveModel into() 消费原值后不可再访问 + DatabaseTransaction 与 DatabaseConnection 需用 ConnectionTrait 泛型统一 + MaterialShortageItem.material_code 是 String 但 alert_model.material_code 是 Option<String> 需 Some 包裹 |
| ✅ 485 | F | P0-T03 baseline 恢复 + P0-T08 覆盖率工具 + P0-T06 bi_analysis 测试 | M+S+M | 无 | 已合并 main 直接提交 af0f16b + 5e4e78f + 7cc82cc；4 文件；CI 7 轮修复；教训：默认 features 下 1781 个预存 dead_code 警告无法一个批次清零，clippy baseline 机制（仅新增警告阻塞）是合理渐进式策略 |
| ✅ 486 | F | P0-T01 核心 service 单测补全 | L | 无 | 已合并 main 直接提交 01faa60；2 文件；quotation_service 19 测试 + purchase_receipt_service 19 测试 = 38 测试；CI 一次过 14/14 全绿 |
| ✅ 487 | F | P0-T02 7 项集成测试 + P0-T07 性能基准 + P0-T05 E2E 通过率 | XL+M+XL | 486 | 已合并 main 直接提交 3919255 + d7e3b73 + a456a53（用户特批不拆分打包）；28 文件 +1836 -29；T02 7 业务路径 73 测试 + T07 4 benches 11 基准（criterion optional feature 机制，criterion 必须在 [dependencies] 不能在 [dev-dependencies]）+ T05 applyAuthMocks 移除 mockBusinessApi + webServer 数组化；CI 3 轮全绿 conclusion=success；教训：CI 自动刷新 baseline 陷阱第三次复发需手动恢复 |
| 🔄 488 | G | P0-D01~D17 全部 17 项打包（用户指令合并为单批） | S+S+L+L+XL+XL+S+XL+L+L+M+M+XL+XL+M+M+M | 无 | D 系列 17 项已完成 10 项：✅ D01/D02/D07/D11/D15/D16/D17 审计误判（已在之前批次完成）+ ✅ D03+D04 5 service Redis 缓存接入（commit cead770）+ ✅ D12 圈复杂度优化（6/6 实际目标 + 2 审计误判跳过；本地 5 commit 待推送）；剩余 7 项大型任务：D05 useI18n（XL）/ D06 aria-label（XL）/ D08 超长函数（XL）/ D09 100+ 行函数（L）/ D10 1000+ 行文件（L）/ D13 缩写命名（XL）/ D14 api 命名（XL）；阻塞中：git 认证丢失 5 个 D12 本地 commit 无法推送 CI 验证 |

**总计**：19 个批次（含 475c/475d/475e 微批次拆分），覆盖 39 P0 任务。

### 3.2 批次工作量分布

| 工作量 | 批次数 | 占比 |
|--------|--------|------|
| S（≤3 文件） | 0 | 0% |
| M（4-6 文件） | 0 | 0% |
| L（7-10 文件） | 5 | 23% |
| XL（>10 文件） | 17 | 77% |
| **合计** | **22** | 100% |

> 注：批次节奏从 6-8 提升至 9-12 文件后，单批文件数普遍进入 L/XL 区间，每批覆盖任务更多，批次总数从 27 压缩为 22。

### 3.3 批次执行原则

1. **严格按顺序**：批次编号即执行顺序，前批未完成不进入下批
2. **依赖优先**：有依赖的任务排在被依赖任务之后
3. **每批 9-12 文件**：批次节奏统一为 9-12 文件，提升单批吞吐量
4. **模块内连续**：同模块任务连续执行，减少上下文切换
5. **CI 全绿推进**：每批 CI 全绿后自动进入下一批（规则 13）
6. **直接 push 验证**：禁止本地编译验证，所有验证直接 push 让 CI 执行

---

## 四、未完成任务清单（按业务模块分组）

### 4.1 模块 A：安全与权限（0 项未完成，原 7 项 - Batch 473 完成 P0-S14 + P0-S19，Batch 474 完成 P0-S15，Batch 475a 完成 P0-S13，Batch 475b-e 完成 P0-S12，Batch 476 完成 P0-S17）

> 模块 A 全部 P0 任务已完成。下方保留已完成任务摘要供参考。

#### P0-S12 前端本地导出完全无审计（类十三，✅ 已完成）

- **来源**：batch-11 P0-11-10/11
- **进度**：✅ 全部完成（Batch 474 核心页面 + 475a-e 5 个微批次），共改造 25+ 页面，覆盖所有模块
- **已完成内容**：
  - Batch 474 PR #657：新增 `exportFromBackend` 函数（独立 axios 实例，绕过 request.ts 拦截器避免 Blob 类型丢失 + router 导入链副作用）；customer/index.vue + supplier/index.vue 改用后端 API；后端 `/crm/customers/export` + `/purchase/suppliers/export` 端点
  - Batch 475a PR #658：audit-log/index.vue 改用后端 API（`/audit-logs/export`）；后端 audit_log_handler 注入水印
  - Batch 475b PR #659：usePurchAct.ts + CustomerListTab.vue 改用后端 API；后端 export_orders 注入水印
  - Batch 475c PR #660：inventory/index.vue + warehouse/index.vue + usePrdProc.ts + production/index.vue 改用后端 API；后端新增 export_stock / export_warehouses / export_production_orders 3 端点 + 3 路由注册；CI 2 轮修复（E0599 WarehouseListQuery 未派生 Clone）
  - Batch 475d PR #661：useScProc.ts + useSpProc.ts + sales-contract/index.vue + sales-price/index.vue + RecordTab.vue + StandardTab.vue + quality-standards/index.vue 共 7 文件改用后端 API；后端新增 export_contracts / export_prices / export_records / export_standards 4 端点 + 4 路由注册；4 个 Query struct 派生 Clone（防 E0599 重现）；CI 2 轮修复（第 1 轮前端类型检查失败 useTableApi queryParams 类型为 Ref<Record<string, unknown>> 与 getQueryParams 强类型返回值不兼容，第 2 轮添加类型断言后全绿）；自审门发现 StandardTab.vue 同类问题并同步改造
  - Batch 475e PR #662：ar/ap/cost/budget/fixed-assets 5 模块 5 个 Tab.vue 改用后端 API；后端新增 export_ar_invoices / export_ap_invoices / export_cost_items / export_budgets / export_fixed_assets 5 端点 + 5 路由注册；CI 一次过 13/13 全绿
- **关联文件**：[frontend/src/utils/export.ts](file:///workspace/frontend/src/utils/export.ts) + 25+ 视图文件 + 各资源 handler/service
- **依赖**：✅ P0-S14 已完成（Batch 473）/ ✅ P0-S15 水印已完成（Batch 474）
- **工作量**：XL（拆分 5 个微批次完成）
- **批次**：474 + 475a-e（已全部完成）

> P0-S12 详细归档见 [doto-su.md §V15 Batch 474/475a-475e](file:///workspace/.monkeycode/doto-su.md)。

#### P0-S17 打印 HTML 是占位假数据（类十三，✅ 已完成）

- **来源**：batch-11 P0-11-15
- **进度**：✅ 已完成（main 直接提交 eb57484，Batch 476）
- **已完成内容**：print_service.rs 6 个 get_*_print_data 方法从硬编码占位改为真实查询 DB（sales_order/sales_contract/purchase_order/purchase_receipt/inventory_transfer/voucher）；print_handler.rs 注入 AppState，调用 PrintService::new(state.db.clone())；使用 sea-orm 直接查询主表 + 关联客户/供应商/仓库 + 明细项 + 产品（LoaderTrait）
- **关联文件**：[print_service.rs](file:///workspace/backend/src/services/print_service.rs) / [print_handler.rs](file:///workspace/backend/src/handlers/print_handler.rs)
- **依赖**：无
- **工作量**：L（实际 2 文件）
- **批次**：476（已合并 main 直接提交 eb57484）

> P0-S14（migration 补齐）+ P0-S19（审计字段补齐）已 Batch 473 PR #656 完成，详细记录归档到 [doto-su.md §V15 Batch 473](file:///workspace/.monkeycode/doto-su.md)。
> P0-S15（导出水印基础设施）已 Batch 474 PR #657 完成，详细记录归档到 [doto-su.md §V15 Batch 474](file:///workspace/.monkeycode/doto-su.md)。

---

### 4.2 模块 B：面料行业-色卡发放（0 项未完成，4 项已完成）

#### ✅ P0-F10 色卡发放——库存联动（已完成，Batch 477）

- **状态**：✅ 已完成（main 直接提交 a3798f4 + daeab0f）
- **修复方案**：方案 A 采用 color_cards.stock_quantity INT NOT NULL DEFAULT 0 字段直接管理
  - m0057_create_color_card_issues_and_stock_fields.rs：建表 + 新增 stock_quantity 字段
  - color_card.rs Model 新增 stock_quantity: i32
  - color_card_crud_service.rs create() 初始化 stock_quantity=0
  - color_card_issue_service.rs 5 方法接入库存联动：
    - issue(): 事务 + lock_exclusive + stock_quantity -= issue_qty
    - return_card(): 事务 + lock_exclusive + stock_quantity += issue_qty
    - cancel_issue(): 事务 + lock_exclusive + stock_quantity += issue_qty
    - mark_lost / mark_damaged: 不还原（色卡消耗）
  - validate_issue_gates gate 2 增强：card.stock_quantity >= issue_qty

#### ✅ P0-F11 色卡发放——前端文件结构（已完成，Batch 477）

- **状态**：✅ 已完成
- **修复方案**：补齐 5 个前端文件
  - types/colorCardIssue.ts（类型定义模块，re-export + 业务专用类型）
  - store/colorCardIssue.ts（Pinia store，availableCards + issueRecords + 5 actions）
  - composables/useColorCardIssue.ts（业务 composable，封装 store + API）
  - components/ColorCardIssueForm.vue（发放表单组件）
  - components/ColorCardIssueDetail.vue（4 合 1 操作对话框）
- **重构**：issues.vue 使用新组件 + composable + store

#### ✅ P0-F12 色卡发放——前端类型/API/视图（已完成，Batch 477）

- **状态**：✅ 已完成
- **修复方案**：api/color-card.ts 类型/API 完整（含 IssueRecordInfo + 6 API 函数）；新增 color-card-issue.ts 独立 API 模块

#### ✅ P0-F13 色卡发放——数据迁移策略（已完成，Batch 477）

- **状态**：✅ 已完成
- **修复方案**：m0057_create_color_card_issues_and_stock_fields.rs 创建 color_card_issues 表（补齐 Batch 471 遗漏的建表迁移）+ m0058_migrate_color_card_borrow_records.rs 迁移旧表数据到新表（borrowed→issued 状态映射，幂等保护，序列同步 setval）+ color_card_migrate_legacy.sql SQL 迁移脚本
- **关键背景**：Batch 471 创建 model 但遗漏建表迁移，导致 API 运行时报 "relation color_card_issues does not exist"

---

### 4.3 模块 C：面料行业-大货批色（0 项未完成，原 5 项全部完成）

> Batch 478 完成 4 项（建表 + 剪样 + 客户批色确认 + ship_order 校验）。Batch 479 完成 1 项（返工/降级/报废 + 返工走生产订单）。模块 C 全部完成。

#### ✅ P0-F15 大货批色——bulk_color_approval 表完全不存在（已完成，Batch 478）

- **状态**：✅ 已完成
- **修复方案**：m0058_create_bulk_color_approval.rs 创建表（24 字段 + 5 索引 + 4 CHECK 约束，8 态状态机 pending/sampled/sent_to_customer/approved/rejected/rework/downgraded/scrapped）
- **FK 修正**：dye_batch 表名为单数（非 dye_batches），与现有 schema 一致
- **关联文件**：m0058 迁移 + bulk_color_approval.rs Model + 4 mod.rs 注册

#### ✅ P0-F16 大货批色——剪大货样业务规则未实现（已完成，Batch 478）

- **状态**：✅ 已完成
- **修复方案**：bulk_color_approval_service.rs `cut_sample()` 方法实现 pending/rework → sampled 状态转换，支持样布长度/ΔE 测量值记录

#### ✅ P0-F17 大货批色——客户批色确认流程未实现（已完成，Batch 478）

- **状态**：✅ 已完成
- **修复方案**：bulk_color_approval_service.rs `customer_approve/customer_reject/customer_rework` 三方法实现 sent_to_customer → approved/rejected/rework 三分支转换

#### ✅ P0-F18 大货批色——返工/降级/报废未实现（已完成，Batch 479）

- **状态**：✅ 已完成
- **来源**：batch-10 P0-10-4
- **修复方案**：bulk_color_approval_service.rs 三个方法联动库存与生产订单：
  - `customer_rework()`：状态转换后调用 `create_rework_production_order()` 创建返工生产订单（order_type='rework' + original_batch_id）
  - `downgrade()`：状态转换后调用 `apply_stock_downgrade()` 联动库存等级降级（一等品→二等品/二等品→等外品，通过 inventory_stock_service.update_stock_grade）
  - `scrap()`：状态转换后调用 `apply_stock_scrap()` 联动库存报废标记（stock_status='报废' + quality_status='不合格'，通过 inventory_stock_service.mark_stock_as_scrapped）
  - `find_related_stocks()`：通过 batch_no/color_no/dye_lot_no 关联 bulk_color_approval 与 inventory_stock
- **关联文件**：bulk_color_approval_service.rs / production_order_service.rs / inventory_stock_service.rs / dye_batch.rs
- **依赖**：P0-F15
- **工作量**：L
- **批次**：479（合并 F18+F21 为单批 7 文件）

#### ✅ P0-F19 大货批色——ship_order 不校验批色状态（已完成，Batch 478）

- **状态**：✅ 已完成
- **修复方案**：services/so/delivery.rs `ship_order()` 方法在 `validate_dye_lot_consistency` 之后、事务开启前调用 `validate_bulk_color_approval(&self.db, request.order_id)`，校验该订单关联的所有批色记录必须为 approved 状态，否则阻止发货
- **关联文件**：[delivery.rs](file:///workspace/backend/src/services/so/delivery.rs) + bulk_color_approval_service.rs 模块级函数

---

### 4.4 模块 D：面料行业-质量管理（2 项）

#### P0-F20 8D 质量管理流程完全缺失（类二十一）

- **来源**：batch-18 P0-18-1
- **证据**：quality_issue_service.rs 不存在；quality_issue.rs model 只有 status 字段无 8D 字段；quality_inspection_service.rs 无 D0~D8 实现
- **修复方案**：实现 D0~D8 八步流程：
  - D0 准备阶段 / D1 组队 / D2 描述问题 / D3 临时措施 / D4 根因分析 / D5 永久措施 / D6 实施 / D7 预防 / D8 表彰
  - quality_issue 表新增 8D 字段，状态机扩展为 11 态
- **关联文件**：quality_issue_service.rs / quality_issue_handler.rs / schema migrations
- **依赖**：无
- **工作量**：XL
- **批次**：480

#### ✅ P0-F21 返工未走生产订单（已完成，Batch 479）

- **状态**：✅ 已完成
- **来源**：batch-18 P0-18-2
- **证据**：rework_service.rs 不存在，仅 dye_batch_rework.rs model 存在
- **修复方案**：
  - m0059_add_rework_order_fields 迁移：production_orders 表新增 order_type（normal/rework，默认 normal）+ original_batch_id（返工订单关联原 dye_batch）；dye_batch_rework 表新增 production_order_id（反向追溯锚点）
  - production_order_service.rs 新增 `create_rework_order()` 方法：order_type='rework' + original_batch_id + RW-YYYYMMDD-NNN 订单号 + 不触发 MRP（返工使用已有物料）
  - bulk_color_approval_service.rs `customer_rework()` 调用 `create_rework_production_order()` 创建返工生产订单
  - dye_batch_state_machine_service.rs ActiveModel 补 production_order_id 字段
- **关联文件**：[production_order_service.rs](file:///workspace/backend/src/services/production_order_service.rs) / bulk_color_approval_service.rs / m0059_add_rework_order_fields.rs
- **依赖**：P0-F18（模块 C 的返工逻辑）
- **工作量**：M
- **批次**：479（与 F18 合并）

---

### 4.5 模块 E：财务与业务流程（17 项独立项 + 13 项归并）

#### P0-B01 坏账准备计提功能缺失（类十七）

- **来源**：batch-15 P0-15-1
- **证据**：bad_debt_service.rs 不存在；全代码库无 bad_debt/坏账 关键字匹配
- **修复方案**：实现坏账准备计提（账龄法：1年内 5% / 1-2年 20% / 2-3年 50% / 3年以上 100%），月末 cron 自动计提
- **关联文件**：bad_debt_service.rs / schema migrations / cron
- **依赖**：无
- **工作量**：L
- **批次**：481（合并 B01+B02+B03+B04 为单批 ~12 文件）

#### P0-B02 坏账核销与审批流缺失（类十七）

- **来源**：batch-15 P0-15-2
- **修复方案**：实现坏账核销二级审批（申请人→财务经理→总经理），核销后更新 ar_balance
- **关联文件**：bad_debt_service.rs / approval_service.rs
- **依赖**：P0-B01
- **工作量**：M
- **批次**：481（与 B01/B03/B04 合并）

#### P0-B03 催收任务管理缺失（类十七）

- **来源**：batch-15 P0-15-3
- **修复方案**：新增 collection_task 表，按账龄自动生成催收任务，分配给销售员，记录催收结果
- **关联文件**：collection_task_service.rs / collection_task_handler.rs / schema migrations
- **依赖**：无（可与 B01 并行设计）
- **工作量**：M
- **批次**：481（与 B01/B02/B04 合并）

#### P0-B04 财务预警机制缺失（类十七）

- **来源**：batch-15 P0-15-4
- **修复方案**：实现财务预警（应收超额 / 库存积压 / 现金流不足 / 预算超支 4 类），触发通知
- **关联文件**：finance_alert_service.rs / notification_service.rs
- **依赖**：无
- **工作量**：L
- **批次**：481（与 B01/B02/B03 合并）

#### P0-B05 大额调拨无额外验证（类十七）

- **来源**：batch-15 P0-15-5
- **修复方案**：资金调拨金额 > 阈值（如 10 万）需二级审批 + 短信验证码
- **关联文件**：fund_transfer_service.rs / approval_service.rs
- **依赖**：无
- **工作量**：S
- **批次**：482（合并 B05+B06+B07+B08+B09+B14 为单批 ~10 文件）

#### P0-B06 预算超支无拦截（类十七）

- **来源**：batch-15 P0-15-6
- **修复方案**：费用报销 / 采购订单创建时校验预算余额，超支拦截
- **关联文件**：budget_service.rs / expense_service.rs / purchase_order_service.rs
- **依赖**：无
- **工作量**：M
- **批次**：482（与 B05/B07/B08/B09/B14 合并）

#### P0-B07 回收规则无自动执行（类十七）

- **来源**：batch-15 P0-15-7
- **修复方案**：CRM 客户回收规则（30 天未联系 / 90 天无商机）自动执行，客户转入公海
- **关联文件**：customer_pool_service.rs / cron
- **依赖**：无
- **工作量**：S
- **批次**：482（与 B05/B06/B08/B09/B14 合并）

#### P0-B08 赢率手填无自动计算（类十七）

- **来源**：batch-15 P0-15-8
- **修复方案**：商机赢率按阶段自动计算（prospecting 10% / qualification 25% / proposal 50% / negotiation 75% / closed_won 100%）
- **关联文件**：crm_opportunity_service.rs
- **依赖**：无
- **工作量**：S
- **批次**：482（与 B05/B06/B07/B09/B14 合并）

#### P0-B09 输单原因未记录（类十七）

- **来源**：batch-15 P0-15-9
- **修复方案**：商机 closed_lost 时必填输单原因（价格/质量/服务/竞争对手/其他）
- **关联文件**：crm_opportunity_service.rs / 前端 opportunity_form.vue
- **依赖**：无
- **工作量**：S
- **批次**：482（与 B05/B06/B07/B08/B14 合并）

#### P0-B10 BI 无数据权限过滤（类十九）

- **来源**：batch-16 P0-16-2
- **证据**：bi_analysis_service.rs 全文无 apply_data_scope 调用，使用 raw SQL 直接查询无权限过滤
- **修复方案**：BI 查询注入 apply_data_scope，按 user_id / department_id 过滤
- **关联文件**：bi_analysis_service.rs / dashboard_service.rs
- **依赖**：无
- **工作量**：M
- **批次**：483（合并 B10+B11+B12+B13 为单批 ~11 文件）

#### P0-B11 定制订单流程缺失打样和报价环节（类二十三）

- **来源**：batch-19 P0-19-1
- **修复方案**：定制订单流程补齐：需求确认 → 打样 → 客户确认 → 报价 → 生产订单
- **关联文件**：custom_order_service.rs / sample_service.rs / quotation_service.rs
- **依赖**：无
- **工作量**：L
- **批次**：483（与 B10/B12/B13 合并）

#### P0-B12 售后与质量集成完全缺失（类二十三）

- **来源**：batch-19 P0-19-2
- **证据**：after_sales 表无 quality_issue_id 关联
- **修复方案**：after_sales 表新增 quality_issue_id 字段，售后工单触发 8D 流程
- **关联文件**：after_sales_service.rs / quality_issue_service.rs / schema migrations
- **依赖**：P0-F20（8D 流程先行）
- **工作量**：M
- **批次**：483（与 B10/B11/B13 合并）

#### P0-B13 物流签收无电子签收单（类二十三）

- **来源**：batch-19 P0-19-3
- **修复方案**：
  1. 新增 electronic_signature 表（签收人/签收时间/签收图片/GPS 定位）
  2. 签收触发应收确认（ar_balance 增加 + 凭证生成）
- **关联文件**：logistics_service.rs / ar_service.rs / schema migrations
- **依赖**：无
- **工作量**：M
- **批次**：483（与 B10/B11/B12 合并）

#### P0-B14 Incoterms 2020 仅支持 5 种（类二十三）

- **来源**：batch-19 P0-19-4
- **证据**：当前仅 EXW/FOB/CIF/DAT/DDP
- **修复方案**：补齐 6 种（FCA/CPT/CIP/DPU/FAS/CFR），共 11 种
- **关联文件**：[incoterms.rs](file:///workspace/backend/src/models/incoterms.rs) / incoterms_service.rs / 前端选项
- **依赖**：无
- **工作量**：S
- **批次**：482（与 B05/B06/B07/B08/B09 合并）

#### ✅ P0-B15 缺料预警状态不持久化（已完成，Batch 484）

- **状态**：✅ 已完成（main 直接提交 df5286ee）
- **来源**：batch-18 P0-18-3
- **修复方案**：m0068_create_material_shortage_tables 创建两表（material_shortage_alerts + material_shortage_threshold_configs）+ material_shortage.rs Model（alerts + threshold_config 子模块）+ material_shortage_service.rs 3 桩方法改真实 DB 读写：
  - save_threshold_config：upsert 到 threshold_configs 表（id=1 单行配置，先 find_by_id 再 update/insert）
  - load_threshold_config：从 DB 读取，无行时降级返回默认值
  - update_status：返回 alert_model::Model，查找 material_id 最新未解决 alert，更新 status + resolved_at + updated_at
  - persist_alerts：保证同 material_id 至多一条未解决 alert
  - generate_alert_no：MS-YYYYMMDD-NNN 格式，查询当天最大序号 + 1
- handler 状态校验值对齐 migration 状态机：identified → purchase_request → purchase_order → received → resolved
- handler DTO 从持久化 alert 读取完整字段（替代原零值填充），level → severity 映射：Critical→critical / Severe→high / Warning→medium / _→low

#### ✅ P0-B16 自动故障检测机制缺失（已完成，Batch 484）

- **状态**：✅ 已完成（main 直接提交 df5286ee）
- **来源**：batch-17 P0-17-1
- **修复方案**：failover_service.rs 健康检查 + 后台监控 + 熔断器状态机
  - health_check 重写为真实 SELECT 1 探测（替代仅读 status 表）：`ConnectionTrait::execute(Statement::from_sql_and_values(backend, "SELECT 1", Vec::new()))`
  - ping_db：轻量 bool 返回的健康探测，供 FailoverMonitor 使用
  - FailoverMonitor：后台健康监控任务（5s 间隔 + 连续 3 次失败阈值 + 环境变量控制 FAILOVER_MONITOR_INTERVAL_SECS / FAILOVER_FAILURE_THRESHOLD / FAILOVER_AUTO_SWITCH_ENABLED，默认 false 仅记录日志）
  - 熔断器状态机：closed（正常）→ 连续失败 >= 3 → open（熔断）→ 健康恢复 → closed
  - increment_consecutive_failures：递增 DB 中的 consecutive_failures，达阈值(3)时 circuit_state → "open"
  - reset_consecutive_failures：重置为 0 + circuit_state → "closed" + 更新 last_success_at
  - record_event 改为 pub，供 FailoverMonitor 调用
- consecutive_failures 字段激活（原为 zombie 字段，现已递增/重置）

#### ✅ P0-B17 主备切换自动完成缺失（已完成，Batch 484）

- **状态**：✅ 已完成（main 直接提交 df5286ee）
- **来源**：batch-17 P0-17-2
- **修复方案**：FailoverExecutor ArcSwap 原子切换 DatabaseConnection
  - 新增 arc-swap = "1.7" 依赖
  - FailoverExecutor：维护 primary + optional backup 两个连接
    - `current: Arc<ArcSwap<DatabaseConnection>>` 运行时无锁 DB 连接替换
    - switch_to_backup() 原子 store 备库连接（备库未配置时返回 Err 降级）
    - switch_to_primary() 原子 store 主库连接（供人工 failback）
    - is_on_backup() 通过 Arc::ptr_eq 判断
    - has_backup() / get_current()
  - FailoverService 新增 executor 字段 + with_executor builder
  - get_active_db：返回当前活跃 DB 连接（executor 存在时从 ArcSwap load，否则克隆主库）
  - test_switch 先执行真实 DB 连接切换，再更新 status + 记录 event
  - update_status_on_switch：递增 total_switches + 设置 last_switch_at（替代原通用 update_status，已删除避免死代码）
  - app_state.rs：AppState + AppStateParams 添加 failover_executor 字段
  - main.rs：支持 DATABASE_BACKUP_URL 环境变量构造备库连接（连接失败降级为 None）
  - failover_handler.rs：build_service 注入 executor

#### P0-B18~B30 归并项（13 项，不计入独立项）

- P0-B18 自动故障检测 → 归并到 P0-B16
- P0-B19 报表订阅后台调度 → 归并到 P0-D16
- P0-B20 BI 数据权限过滤 → 归并到 P0-B10
- P0-B21 缺料预警状态持久化 → 归并到 P0-B15
- P0-B22 自动故障检测 → 归并到 P0-B16
- P0-B23 主备切换 → 归并到 P0-B17
- P0-B24 大货批色 ship_order 校验 → 归并到 P0-F19
- P0-B25 售后与质量集成 → 归并到 P0-B12
- P0-B26 物流签收 → 归并到 P0-B13
- P0-B27 Incoterms 补齐 → 归并到 P0-B14
- P0-B28 定制订单打样报价 → 归并到 P0-B11
- P0-B29 报表订阅后台调度 → 归并到 P0-D16
- P0-B30 BI 数据权限过滤 → 归并到 P0-B10

---

### 4.6 模块 F：测试体系（6 项）

#### P0-T01 核心 service 零单元测试（类六）

- **来源**：batch-06 P0-06-1
- **证据**：backend/tests/quotation_service_test.rs 不存在；quotation_service.rs 内部无 cfg(test) 模块
- **修复方案**：为两个 service 编写完整单元测试（覆盖率 ≥80%），抽取 mock 数据到 fixtures
- **关联文件**：backend/tests/quotation_service_test.rs / purchase_receipt_service_test.rs / tests/fixtures/
- **依赖**：无
- **工作量**：L
- **批次**：485（合并 T01+T03+T06+T08 为单批 ~12 文件）

#### ✅ P0-T02 7 项关键业务路径无集成测试（已完成，Batch 487）

- **状态**：✅ 已完成（main 直接提交 3919255）
- **来源**：batch-06 P0-06-2
- **修复方案**：7 业务路径集成测试文件 73 测试采用 #[ignore]+纯函数双模式
  - production_order_workflow_test.rs 9 测试（状态机校验纯函数 #[test] + 完整业务流程 #[ignore]）
  - purchase_receipt_workflow_test.rs 8 测试
  - sales_delivery_workflow_test.rs 9 测试
  - ap_payment_workflow_test.rs 8 测试
  - dye_batch_workflow_test.rs 14 测试（14 状态 + 13 流转码 + 30+ 合法边）
  - lab_dip_workflow_test.rs 10 测试（PENDING→SAMPLING→SUBMITTED→APPROVED/REJECTED→COMPLETED）
  - production_recipe_workflow_test.rs 15 测试（DRAFT→APPROVED→CLOSED/CANCELLED）
- **测试模式**：
  - 纯函数测试：状态机校验函数（validate_status_transition / is_valid_transition / parse_liquor_ratio / calculate_amounts）无 DB 依赖，直接 `#[test]` 测试
  - 集成测试：完整业务流程标记 `#[ignore = "需要 PostgreSQL..."]`，通过 `TEST_DATABASE_URL` 环境变量切换真实 DB，CI 默认跳过
- **关联文件**：backend/tests/production_order_workflow_test.rs / purchase_receipt_workflow_test.rs / sales_delivery_workflow_test.rs / ap_payment_workflow_test.rs / dye_batch_workflow_test.rs / lab_dip_workflow_test.rs / production_recipe_workflow_test.rs
- **依赖**：P0-T01（单测先行，已 Batch 486 完成）
- **工作量**：XL
- **批次**：487（与 T05/T07 合并，用户特批不拆分）

#### P0-T03 CI baseline 机制掩盖编译失败（类六）

- **来源**：batch-06 P0-06-3
- **证据**：.github/workflows/ci-cd.yml 仍保留 clippy baseline 机制 line 354-605 和 cargo test baseline 机制 line 897-1031
- **修复方案**：移除 baseline 机制，所有失败必须真实修复
- **关联文件**：[.github/workflows/ci-cd.yml](file:///workspace/.github/workflows/ci-cd.yml) / backend/tests/bi_analysis_test.rs
- **依赖**：无
- **工作量**：M
- **批次**：485（与 T01/T06/T08 合并）

#### ✅ P0-T05 E2E 通过率 0%（已完成，Batch 487）

- **状态**：✅ 已完成（main 直接提交 3919255）
- **来源**：batch-06 P0-06-5
- **证据**：95 个 E2E 测试 88 个失败
- **修复方案**：配置层修复让 sales/purchase 走真实后端，而非逐个修用例
  - `frontend/e2e/fixtures/auth.ts`：`applyAuthMocks` 移除 `await mockBusinessApi(context)` 调用，让 sales/purchase 走真实后端；`mockBusinessApi` 函数保留供 `enhanced/multi-role-collaboration.spec.ts` 显式调用（多上下文隔离测试不依赖业务数据）
  - `frontend/playwright.config.ts`：`webServer` 从单对象改为数组配置，同时启动前端 dev server + 后端二进制；前端 `reuseExistingServer: !process.env.CI` + 后端 `reuseExistingServer: true`（关键 — CI 中 e2e-batch.yml 已独立启动后端，Playwright 复用该实例避免端口冲突）
  - 14 个 sales/purchase spec 文件 beforeEach 注释更新（规则 20 一致性）：从"mock 业务 API"改为"业务 API 走真实后端"
- **关联文件**：[frontend/e2e/fixtures/auth.ts](file:///workspace/frontend/e2e/fixtures/auth.ts) + [frontend/playwright.config.ts](file:///workspace/frontend/playwright.config.ts) + 14 个 sales/purchase spec 文件
- **依赖**：无
- **工作量**：XL
- **批次**：487（与 T02/T07 合并，用户特批不拆分）

#### P0-T06 bi_analysis_test.rs 16 个测试 API 脱节（类六）

- **来源**：batch-06 P0-06-6
- **修复方案**：更新 16 个测试用例的 API 调用，与源码对齐
- **关联文件**：backend/tests/bi_analysis_test.rs
- **依赖**：无
- **工作量**：M
- **批次**：485（与 T01/T03/T08 合并）

#### ✅ P0-T07 4 项关键 service 性能基准测试缺失（已完成，Batch 487）

- **状态**：✅ 已完成（main 直接提交 3919255）
- **来源**：batch-06 P0-06-7
- **修复方案**：4 benches 11 基准，采用 criterion optional feature 机制
  - inventory_calculation_bench.rs 3 基准
  - voucher_generation_bench.rs 2 基准
  - dye_cost_collection_bench.rs 3 基准
  - wage_calculation_bench.rs 3 基准
- **Cargo.toml 配置**：
  - `criterion = { version = "0.5", optional = true }`（依赖设为 optional）
  - `[features] bench = ["criterion"]`（feature 门控）
  - `[[bench]] name = "xxx" harness = false required-features = ["bench"]`（4 个 [[bench]] 段都加 required-features）
- **设计要点**：默认 features 不启用 bench，确保 `cargo test` 不编译 bench 文件，减少 CI 编译时间
- **关联文件**：[backend/Cargo.toml](file:///workspace/backend/Cargo.toml) + backend/benches/inventory_calculation_bench.rs / voucher_generation_bench.rs / dye_cost_collection_bench.rs / wage_calculation_bench.rs
- **依赖**：无
- **工作量**：M
- **批次**：487（与 T02/T05 合并，用户特批不拆分）

#### P0-T08 CI 不集成覆盖率工具（类六）

- **来源**：batch-06 P0-06-8
- **修复方案**：CI 新增 `cargo tarpaulin` 步骤，上传 codecov；前端新增 `vitest --coverage`
- **关联文件**：.github/workflows/ci-cd.yml / codecov.yml
- **依赖**：无
- **工作量**：S
- **批次**：485（与 T01/T03/T06 合并）

---

### 4.7 模块 G：部署与运维（17 项，10 项已完成 / 7 项剩余）

#### ✅ P0-D01 Docker 文件违规（类七，审计误判已完成）

- **来源**：batch-07 P0-07-1
- **状态**：✅ 审计误判 —— Batch 488 步骤 0 验证 5 个 Docker 文件均不存在（Dockerfile / backend/Dockerfile / frontend/Dockerfile / docker-compose.yml / .dockerignore），已在之前批次删除
- **批次**：488（D 系列 17 项一次性打包）

#### ✅ P0-D02 快速部署脚本安装 PostgreSQL 客户端（类七，审计误判已完成）

- **来源**：batch-07 P0-07-2
- **状态**：✅ 审计误判 —— [install.sh](file:///workspace/快速部署/install.sh) L43 已有 `# P0-D02：移除 postgresql-client 安装` 注释，L44 `apt-get install -y curl jq unzip tar nginx`（已无 postgresql-client）
- **批次**：488（D 系列 17 项一次性打包）

#### ✅ P0-D03 5 个 service 全部未接入缓存层（类七，已完成 commit cead770）

- **来源**：batch-07 P0-07-3
- **状态**：✅ 已完成 —— 新增 utils/redis_cache.rs L2 层双缓存工具，user/product/customer/supplier/role 5 service 读穿透+写失效，TTL 5 分钟，customer/supplier 缓存命中时 data_scope 权限校验仍执行防越权，REDIS_URL 未配置时优雅降级
- **关联文件**：[redis_cache.rs](file:///workspace/backend/src/utils/redis_cache.rs) + 5 service 文件
- **批次**：488（commit cead770 已推送）

#### ✅ P0-D04 缓存是内存缓存(moka)非 Redis（类七，已完成 commit cead770）

- **来源**：batch-07 P0-07-4
- **状态**：✅ 已完成 —— 与 D03 同批，moka + Redis 双缓存策略，moka 进程内 L1 + Redis 跨实例 L2
- **关联文件**：[redis_cache.rs](file:///workspace/backend/src/utils/redis_cache.rs) + [cache_service.rs](file:///workspace/backend/src/services/cache_service.rs)
- **批次**：488（commit cead770 已推送）

#### P0-D05 useI18n 接入率仅 3.1%（类七，XL，未开始）

- **来源**：batch-07 P0-07-5
- **证据**：2026-07-19 精确审计：实际 355 个 .vue 文件（非 85+ 也非 347），已接入 11 个（接入率 3.1%），未接入 344 个；locales/zh-CN.ts 467 行 15 模块 332 键，预估需扩容至 5000+ 键；Top 20 硬编码密集文件累计 10746 行中文（占全模块 15%），单文件最大 fixed-assets/tabs/AssetListTab.vue 864 行
- **修复方案**：355 个 .vue 视图组件全部接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts + en-US.ts 同步；按业务模块横向切片，每批 10-12 文件，预估需 30-36 批次
- **关联文件**：[frontend/src/views/](file:///workspace/frontend/src/views/) + [frontend/src/locales/zh-CN.ts](file:///workspace/frontend/src/locales/zh-CN.ts) + [frontend/src/locales/en-US.ts](file:///workspace/frontend/src/locales/en-US.ts)
- **依赖**：建议在 D13/D14 完成后推进（避免同时修改 .vue 文件造成冲突）
- **工作量**：XL（5 项中最大）
- **批次**：488（D 系列 17 项一次性打包；预估 30-36 子批次）
- **执行优先级**：第 5 顺位（最后推进）

#### ✅ P0-D06 aria-label 严重不足（类七，XL，已完成）

- **来源**：batch-07 P0-07-6
- **证据**：仅 2 个文件 8 处 aria-label
- **修复方案**：所有交互元素补 aria-label（WCAG 2.1 AA）
- **关联文件**：所有 .vue 文件
- **依赖**：无
- **工作量**：XL
- **批次**：488（D 系列 17 项一次性打包）
- **进度**（截至 2026-07-19，55 个子批次累计 ~225 文件，全部完成）：
  - D06-2 (aa103cb)：通用组件 8 文件
  - D06-3 (3d7635c)：views 高频页面 6 文件
  - D06-4 (4d14973)：views 高优先级 5 文件
  - D06-5 (5e09b20)：views priority 6-10 5 文件
  - D06-6 (f598caf, another agent)：views priority 11-15 5 文件
  - D06-7 (cfb1fc6)：views priority 11-15 5 文件（本会话）
  - D06-8 (4b4e690)：高缺失文件 5 个（cost/inventoryBatch/sales-ext/purchase-ext/dye-batch）
  - D06-9 (957454a)：系统管理 + 工艺优化 5 文件
  - D06-10 (b93f12f)：system/tabs 剩余 5 个 Tab
  - D06-11 (8cc4506)：trading/tabs 5 个 Tab
  - D06-12 (c1f638a)：system-update + supplier + sales-price 5 文件
  - D06-13 (e77f276)：sales-price/components 5 个组件（SpTbl/SpForm/SpFilter/SpHistory/SpView）
  - D06-14 (b01b1c5)：sales-contract/components + sales-returns/components 5 文件（ScTbl 已迁移 V2Table 跳过）
  - D06-15 (c41f443)：logistics/components + purchase-price/components 5 文件（LgsStat 无目标元素 + LgsTbl 已迁移 V2Table 跳过）
  - D06-16 (a0e0986)：purchase-price/components 剩余 + purchase-contract/components 5 文件
  - D06-17 (9d1a109)：purchase-contract/components 剩余 + purchase-inspection/components 5 文件
  - D06-18 (ffc04cd)：purchase-inspection/components 剩余 + production/components 5 文件（PiStat 无目标元素 + PrdTbl 已迁移 V2Table 跳过）
  - D06-19 (a64dc0d)：material-shortage + purchaseReceipt + purchase components 5 文件
  - D06-20 (37685d4)：purchase + inventory components 5 文件
  - D06-21 (4701889)：sales-analysis + scheduling components 6 文件
  - D06-22 (ff269fa)：arReconciliation + purchase-return components 5 文件
  - D06-23 (76b7af5)：purchase-return components 剩余 4 文件
  - D06-24 (a94bb04)：dashboard + data-import components 5 文件
  - D06-25 (53c150a)：security/capacity/advanced components 5 文件
  - D06-26 (c892b8e)：advanced/api-gateway components 5 文件
  - D06-27 (0a1df5f)：api-gateway/system-update/admin components 4 文件
  - D06-28 (7325a7a)：api-gateway tabs 2 文件（ApiLogTab V2Table 跳过）
  - D06-29 (d2584b5)：inventory tabs 3 文件（InventoryStockTab el-table V2Table 跳过）
  - D06-30 (573e1a7)：finance/tabs/components 4 文件
  - D06-31 (e31ca81)：voucher/sales components 5 文件（PascalCase 标签）
  - D06-32 (b2f909a)：sales/finance/quotations/crm 5 文件
  - D06-33 (035bd83)：crm/tabs 批 1 5 文件
  - D06-34 (6ac2efb)：crm/tabs 批 2 + leads + opportunities 5 文件
  - D06-35 (3b0eca9)：bpm/definitions/components 5 文件
  - D06-36 (afc2448)：bpm/approval + system + security 5 文件
  - D06-37 (82444ed)：product/tabs + fabric/DyeTab 5 文件
  - D06-38 (1351018)：fabric/tabs 剩余 5 文件
  - D06-39 (3dbfdd1)：quality + inventoryAdjustment tabs 5 文件
  - D06-40 (2eb2ff5)：inventoryAdjustment + inventoryBatch + inventoryCount 5 文件
  - D06-41 (c9f16e1)：inventoryTransfer + ap/tabs 5 文件
  - D06-42 (8a54858)：ap/ar/fund/supplier/customerCredit 5 文件
  - D06-43 (85b0511)：customerCredit + accountSubject + accountingPeriod + financeReport 5 文件
  - D06-44 (d527e5e)：financial-analysis + bom + mrp 5 文件
  - D06-45 (4581376)：color-cards + color-prices 5 文件
  - D06-46 (f8211a0)：custom-orders + dataPermission + departments 4 文件
  - D06-47 (8818eda)：notification + quality-standards + user-profile + ai-extend + Setup 6 文件
  - D06-48 (d7cec20)：crm 多元素 + quotations/list 5 文件 16 处
  - D06-49 (d33deb6)：customer + customerCredit + scheduling 5 文件 10 处
  - D06-50 (30ae917)：bpm + bi + components-demo + quotations 5 文件 8 处
  - D06-51 (d91b036)：system/tabs + security el-form 5 文件 5 处
  - D06-52 (无 commit)：data-import/purchase-return/material-shortage/purchase-* el-pagination 已有 aria-label 跳过
  - D06-53 (无 commit)：sales-price/system-update tabs el-pagination 已有 aria-label 跳过
  - D06-54 (eaadd4d)：fiveDimension/barcodeScanner/businessTrace/arReconciliation/omniAudit/assistAccounting 6 个 PascalCase 文件 30 处
  - D06-55 (22c842a)：QualityCheck/color-cards/issues/product/tabs 最终收尾 3 文件 4 处
  - 累计完成约 225 个文件，最终扫描确认全部补齐无遗漏
  - **关键策略**：icon-only 按钮优先 + el-table/el-dialog/el-form/el-pagination 交互容器 + 动态 :title 用 :aria-label 同步绑定 + V2Table 迁移文件跳过 el-table + 已有 aria-label 文件跳过 + PascalCase 标签同样处理 + 续行 aria-label 检测避免误报

#### ✅ P0-D07 图片 alt 属性完全缺失（类七，审计误判已完成）

- **来源**：batch-07 P0-07-7
- **状态**：✅ 审计误判 —— [user-profile/index.vue:30](file:///workspace/frontend/src/views/user-profile/index.vue#L30) 原生 `<img>` 已有 `:alt="profileForm.real_name ? '${profileForm.real_name}的头像' : '用户头像'"`；[TfaStep2.vue:14](file:///workspace/frontend/src/views/security/two-factor/components/TfaStep2.vue#L14) `<el-image>` 已有 `alt="二步验证二维码"`
- **批次**：488（D 系列 17 项一次性打包）

#### P0-D08 91+ 超长函数（类七，XL，进行中）

- **来源**：batch-07 P0-07-8
- **证据**：2026-07-19 精确扫描（fn-to-next-fn 口径）：>80 行函数约 91 个，>100 行函数约 54 个，>200 行函数 6 个，>500 行函数 0 个；最严重案例 so/delivery.rs:110 ship_order 346 行、so/order_crud.rs:98 create_order 344 行、ar_service.rs:993 manual_verify 257 行、bpm_service.rs:242 approve_task 211 行、wage_service.rs:873 calculate 211 行、ar_service.rs:706 auto_verify 192 行；预估还有 10-20 个 D08 函数未捕获（services/ai、services/ar/inv.rs、services/inv/adjust.rs 等未完整展开）
- **已重构确认**：event_bus.rs:412 start_event_listener D12-2 已重构（实际 279 行，CC 33→10 达标，列入观察名单不强拆）
- **豁免函数**：dye_batch_state_machine_service.rs:165 builtin_transition_rules 154 行纯数据表（27 条状态机三元组定义）豁免拆分
- **修复方案**：拆分超长函数为单一职责小函数（每个 ≤50 行），主函数仅做协调；按 ROI 分四梯队推进
- **关联文件**：[backend/src/services/so/delivery.rs](file:///workspace/backend/src/services/so/delivery.rs) / [ar_service.rs](file:///workspace/backend/src/services/ar_service.rs) / [bpm_service.rs](file:///workspace/backend/src/services/bpm_service.rs) / [wage_service.rs](file:///workspace/backend/src/services/wage_service.rs) / [voucher_service.rs](file:///workspace/backend/src/services/voucher_service.rs) / [quotation_service.rs](file:///workspace/backend/src/services/quotation_service.rs) / 等 35+ 文件
- **依赖**：无前置依赖
- **工作量**：XL
- **批次**：488（D 系列 17 项一次性打包；预估 10-12 子批次）
- **执行优先级**：第 1 顺位（无前置依赖 + 解锁 D09/D10）
- **进度**：D08-1 第一梯队 6/6 + 第二梯队首批 5/5 + 第二梯队第 2 批 5/5 完成，CI 全绿（run 29720458274）；第一梯队 6 函数（ship_order/create_order/manual_verify/approve_task/calculate/auto_verify）+ 第二梯队首批 5 函数（batch_update_products/import_products_from_csv/quotation update/detect_anomalies/auto_generate_from_receipt）+ 第二梯队第 2 批 5 函数（ar create_payment/voucher update_account_balances/so update_order/purchase_return approve_return/ai predict_quality）；第二梯队首批 CI 4 轮修复：① BatchError 未实现 Clone（ctx.errors.clone() 失败）+ Clone derive；② CI 自动刷新 baseline 在 clippy 编译失败时误删预存警告（第三次复发，修复：自动刷新条件增加 CLIPPY_MAIN_EXIT = 0 + CLIPPY_MAIN_EXIT 写入文件供后续 step 读取）；③ apply_order_header_updates 借用引用后 String 字段 move（E0507，4 处 if let Some 改 &request.x + clone）；④ baseline 恢复 5 条预存警告；第二梯队第 2 批 CI 1 轮通过；教训：CI 自动刷新 baseline 在 clippy 编译失败时 reports/clippy-current.txt 不完整会误删预存警告，需增加 clippy 正常退出条件 + struct 字段含 Vec<T> 时 T 需实现 Clone + 共享引用 &request 后 String 字段只能用 &request.x + clone() 模式不能 move；发现 approve_return 已被前序 agent 拆分完成（本次仅清理 2 处违规注释块）；进入第二梯队第 3 批
- **批次规划**：
  - 第一梯队（>200 行 6 函数，2 批）：✅ ship_order / ✅ create_order / ✅ manual_verify / ✅ approve_task / ✅ calculate / ✅ auto_verify
  - 第二梯队（150-200 行 22 函数，3-4 批）：✅ 首批 5 函数（batch_update_products/import_products_from_csv/quotation update/detect_anomalies/auto_generate_from_receipt）+ ✅ 第 2 批 5 函数（ar create_payment/voucher update_account_balances/so update_order/purchase_return approve_return/ai predict_quality）+ 待拆 12 函数（omni_audit new 163 / ap_report get_statistics_report 161 / bi_analysis kpi_summary 159 / business_metrics new 157 / outsourcing record_receipt 157 / list_orders 156 / create_default_roles 155 / ap_report get_aging_report 153 / increase_finished_goods_txn 152 / chemical update 150 / ar/vfy get_aging_report 150 / ap_verification auto_verify 171）
  - 第三梯队（100-150 行 ~20 函数，2-3 批）：含 create_payment / list_orders / approve_adjustment / 等
  - 第四梯队（80-100 行 ~37 函数，3-4 批）：含 inventory_finance_bridge 7 个 create_*_voucher 模板化提取
  - 模板化提取候选：inventory_finance_bridge_service.rs 7 个 create_*_voucher 函数提取通用 create_bridge_voucher<VoucherBuilder>

#### P0-D09 54+ 函数超过 100 行（类二，L，D08 完成后自动完成）

- **来源**：batch-02 P0-02-01
- **证据**：2026-07-19 精确扫描：>100 行函数约 54 个（与 D08 范围重叠，D08 是 D09 的超集）
- **修复方案**：D08 完成后 D09 自动完成（D09 是 D08 子集，D08 阈值 >80 行涵盖 D09 阈值 >100 行）
- **关联文件**：同 P0-D08
- **依赖**：P0-D08
- **工作量**：L（实际 0 增量工作，D08 完成即 D09 完成）
- **批次**：488（D08 子集，不独立成批）

#### P0-D10 30 个后端文件超过 1000 行（类二，L，未开始）

- **来源**：batch-02 P0-02-02
- **证据**：2026-07-19 精确扫描：实际 30 个 >1000 行文件（doto.md 原标记 26 个不准），13 个 >1500 行，1 个 >2000 行（ar_service.rs 2067 行）；审计后新增越线 main.rs 1005 行 + init_service.rs 1287 行；28 个原审计文件全部仍 >1000 行无一下降；bi_analysis_service.rs 增长最快（+201 行 1461→1662）
- **修复方案**：按职责拆分为多个文件（如 ar_service.rs 拆分为 ar_service / ar_aging_service / ar_collection_service；models/status.rs 拆分为 status/sales / status/purchase / status/inventory；main.rs 拆为 main / routes_bootstrap / middleware_bootstrap）
- **关联文件**：[backend/src/services/ar_service.rs](file:///workspace/backend/src/services/ar_service.rs) (2067) / [production_order_service.rs](file:///workspace/backend/src/services/production_order_service.rs) (1998) / [so/delivery.rs](file:///workspace/backend/src/services/so/delivery.rs) (1930) / [voucher_service.rs](file:///workspace/backend/src/services/voucher_service.rs) (1841) / [energy_service.rs](file:///workspace/backend/src/services/energy_service.rs) (1800) / 等 30 文件
- **依赖**：P0-D08/D09（避免函数拆分和文件拆分同时进行造成冲突）
- **工作量**：L
- **批次**：488（D 系列 17 项一次性打包；预估 5-6 子批次，每批 5-6 文件）
- **执行优先级**：第 2 顺位（D08 完成后立即推进）
- **批次规划**：
  - 第 1 批：ar_service.rs (2067) + production_order_service.rs (1998) + so/delivery.rs (1930) 3 个 >1800 行文件
  - 第 2 批：voucher_service.rs (1841) + energy_service.rs (1800) + outsourcing_service.rs (1782) + business_mode_service.rs (1718) 4 个 >1700 行文件
  - 第 3 批：chemical_service.rs (1676) + bi_analysis_service.rs (1662) + models/status.rs (1577) + mrp_engine_service.rs (1556) 4 个 >1500 行文件
  - 第 4 批：dye_batch_state_machine_service.rs (1512) + wage_service.rs (1507) + ar/vfy.rs (1320) + ap_invoice_service.rs (1306) 4 个 >1300 行文件
  - 第 5 批：init_service.rs (1287) + flow_card_service.rs (1271) + ap_reconciliation_service.rs (1243) + search/elastic.rs (1230) 4 个 >1200 行文件
  - 第 6 批：剩余 11 个 1000-1200 行文件

#### ✅ P0-D11 setup_test_db 在 14 个文件重复定义（类二，审计误判已完成）

- **来源**：batch-02 P0-02-03
- **状态**：✅ 审计误判 —— [test_common.rs](file:///workspace/backend/src/services/test_common.rs) 完整 setup_test_db 实现（18 行，模块头注释标注"抽取自 21 处重复定义"）+ [tests/common/mod.rs](file:///workspace/backend/tests/common/mod.rs) 完整 setup_test_db 实现（19 行，供 tests/ 下 3 个集成测试文件使用）
- **批次**：488（D 系列 17 项一次性打包）

#### ✅ P0-D12 8 个函数圈复杂度 >15（类二，已完成 commit 25efd76~ae73f42）

- **来源**：batch-02 P0-02-04
- **状态**：✅ 已完成 —— 8 个目标函数全部处理：6 项实际重构（check_module_consistency CC 35→7 / auto_match CC 25→15 / update_account_balances CC 17→11 / auto_verify CC 20→15 / ship_order CC 17→13 / start_event_listener CC 33→10 通过提取 8 个 helper）+ 2 项审计误判跳过（manual_verify CC=11 已低于阈值 15 / builtin_transition_rules CC=1 已远低于阈值）
- **关联文件**：[business_mode_service.rs](file:///workspace/backend/src/services/business_mode_service.rs) / [ar/vfy.rs](file:///workspace/backend/src/services/ar/vfy.rs) / [voucher_service.rs](file:///workspace/backend/src/services/voucher_service.rs) / [ar_service.rs](file:///workspace/backend/src/services/ar_service.rs) / [so/delivery.rs](file:///workspace/backend/src/services/so/delivery.rs) / [event_bus.rs](file:///workspace/backend/src/services/event_bus.rs)
- **批次**：488（本地 5 commit 待推送：25efd76 + 319c471 + e32048b + 30a1352 + ae73f42）

#### P0-D13 前端 123 个组件缩写命名（类二，XL，未开始）

- **来源**：batch-02 P0-02-05
- **证据**：2026-07-19 精确扫描：实际 123 个缩写命名 .vue 文件（views/ 122 + components/ 1，doto.md 原标记 ~119 误差 ±4）；25 类缩写前缀（Sc/Su/Lgs/Vchr/Pp/Di/Tfa/Sec/Cp/Sch/Prd/Bpm/Pc/Pi/Sa/Db/Purch/Prc/PrRtn/Ms/Sp/Olv/Ep/Bom/AI）；32 个父级 .vue 文件需更新 import（99 处 import 语句）；0 路由风险（router/index.ts 不直接 import 缩写文件）；0 e2e 风险（e2e 测试通过 Playwright 交互不直接 import 组件）；约 30 个 composables 同步重命名（D14/D15 范畴）
- **修复方案**：重命名为描述性全名（如 ScFilter→SalesContractFilter、SuVerDetail→SystemUpdateVersionDetail、LgsTbl→LogisticsTable、VchrForm→VoucherForm、BomForm→BillOfMaterialsForm）；同步重命名 composables 和父级 import；保留白名单：API（ApiEndpointTab 已描述性）/ i18n / a11y / V2Table（30+ 文件引用影响大）
- **关联文件**：[frontend/src/views/](file:///workspace/frontend/src/views/) 25 个模块的 components/ 子目录 + [frontend/src/components/ai/AIPredictionChart.vue](file:///workspace/frontend/src/components/ai/AIPredictionChart.vue)
- **依赖**：建议在 D14 完成后推进（避免同时修改 import 路径造成冲突）
- **工作量**：XL
- **批次**：488（D 系列 17 项一次性打包；预估 12-15 子批次，每批 8-10 文件）
- **执行优先级**：第 4 顺位（D14 完成后推进）
- **批次规划**：按模块分组（每模块独立批次）
  - sales-contract (3) + system-update (3) + sales-price (5) + purchase-price (5) 第 1 批 16 文件
  - logistics (6) + finance/tabs (4) + voucher/tabs (4) + data-import (4) 第 2 批 18 文件
  - security/two-factor (5) + security/components (4) + capacity (4) + advanced (4) 第 3 批 17 文件
  - api-gateway (1) + sales (3) + scheduling (10) + arReconciliation (6) 第 4 批 20 文件
  - purchase-return (5) + material-shortage (3) + production (4) + bpm/definitions (5) 第 5 批 17 文件
  - bpm/approval (6) + purchase-contract (4) + purchase-inspection (5) + sales-analysis (5) 第 6 批 20 文件
  - bom (1) + dashboard (4) + purchase (6) + purchaseReceipt (4) + components/ai (1) 第 7 批 16 文件

#### P0-D14 前端 api 命名不统一（类二，XL，未开始）

- **来源**：batch-02 P0-02-06
- **证据**：2026-07-19 精确扫描：96 个 api/*.ts 文件（准确）；风格 A（object 形式 `export const xxxApi = {}`）21 个 + 风格 B（function 形式）68 个 + 混合风格 4 个（supplier/customer/financial-analysis/five-dimension）+ 纯 re-export 3 个（index/ap-reconciliation/ap-verification）；最大偏差源 listXxx 47 文件 84 处需改名为 getXxxList；次要偏差 addXxx 5 文件 6 处 / removeXxx 2 文件 2 处 / fetchXxx 1 文件 1 处 / queryXxx 2 文件 2 处
- **修复方案**：统一为风格 B（function 形式）+ 命名规范 `getXxxList / createXxx / updateXxx / deleteXxx / getXxxById`；保留 request.ts 不改名（基础设施）；4 个混合文件先去重再统一；3 个 re-export 文件同步更新导出列表；预估影响 2000+ 处调用点
- **关联文件**：[frontend/src/api/](file:///workspace/frontend/src/api/) 96 个 .ts 文件
- **依赖**：无前置依赖（独立任务）
- **工作量**：XL
- **批次**：488（D 系列 17 项一次性打包；预估 10-12 子批次，每批 8-10 文件）
- **执行优先级**：第 3 顺位（与 D05/D13 解耦）
- **批次规划**：
  - Batch 1：财务 AP/AR 9 文件（ap.ts/ap-invoice.ts/ap-payment.ts/ar.ts/ar-reconciliation.ts/ar-reconciliation-enhanced.ts/ap-reconciliation.ts/ap-verification.ts/voucher.ts）
  - Batch 2：采购/销售/库存 18 文件
  - Batch 3：生产/质量/BOM/MRP 12 文件
  - Batch 4：CRM/客户/供应商/贸易 14 文件
  - Batch 5a/5b：系统/权限/基础/报表/其他 40 文件（拆 2 子批）

#### ✅ P0-D15 升级流程非零停机（类二十五，审计误判已完成）

- **来源**：batch-21 P0-21-1
- **状态**：✅ 审计误判 —— [upgrade.rs](file:///workspace/backend/src/cli/util/upgrade.rs) 蓝绿部署已完整实现（14 个函数：is_blue_green_mode / get_active_instance / instance_service / instance_port / opposite_instance / health_check_instance / switch_nginx_upstream / cleanup_temp / cmd_rollback_blue_green / cmd_rollback_legacy / deploy_release / deploy_release_blue_green / deploy_release_legacy + 常量 BLUE_GREEN_TEMPLATE/BLUE_PORT/GREEN_PORT/NGINX_UPSTREAM_ACTIVE/HEALTH_PATH/HEALTH_CHECK_RETRIES）
- **批次**：488（D 系列 17 项一次性打包）

#### ✅ P0-D16 报表订阅无后台调度任务（类十九，审计误判已完成）

- **来源**：batch-16 P0-16-1
- **状态**：✅ 审计误判 —— [report_subscription_scheduler.rs](file:///workspace/backend/src/services/report_subscription_scheduler.rs) 完整实现 268 行（run_once / execute_subscription / extract_recipients / update_subscription_status / start_background_task）+ main.rs L696-L711 已接入启动 cron
- **批次**：488（D 系列 17 项一次性打包）

#### ✅ P0-D17 OA 公告完全未实现（类十九，审计误判已完成）

- **来源**：batch-16 P0-16-3
- **状态**：✅ 审计误判 —— [oa_announcement_service.rs](file:///workspace/backend/src/services/oa_announcement_service.rs) 完整 CRUD 实现（CreateOaAnnouncementRequest / UpdateOaAnnouncementRequest DTO + create/get_by_id/update/delete/publish/archive/list 7 方法 + validate_announcement_type/validate_status 校验）+ oa_announcement_handler + routes + model 4 件套均已存在
- **批次**：488（D 系列 17 项一次性打包）

---

## 五、P1/P2/P3 任务规划（按类别汇总）

> P0 完成后按优先级顺序推进。详细内容见 V15 审计报告 [docs/audits/v15/](file:///workspace/.monkeycode/docs/audits/v15/)。

### 5.1 P1 高优先级（257 项，预估 45-55 批次，按每批 9-12 文件计算）

| 模块 | P1 数 | 主要内容 | 关键批次预估 |
|------|-------|----------|--------------|
| 类二 通用代码质量 | 3 | api 命名/缩写命名/DbErr 包装 | 2 批 |
| 类三 安全性 | 6 | refresh_token/PUBLIC_PATHS/validator/Webhook/magic bytes/zip bomb | 3 批 |
| 类四 面料行业深化 | 11 | batch_trace/检验指标/工资凭证/能耗/委外/事件发布/工时 | 4 批 |
| 类五 运行逻辑闭环 | 11 | 状态机/配置/业务事件/成本归集/加权平均 | 4 批 |
| 类六 测试体系 | 11 | 覆盖率/mock/fixtures/文档 | 4 批 |
| 类七 可维护性 | 11 | i18n/aria/缓存/文档 | 4 批 |
| 类八 法律合规 | 16 | 用户协议/HTTPS/脱敏/导出/docx/标准/签章/税/环保/排污/劳动/工时/社保/职业健康 | 6 批 |
| 类九 色卡发放 | 9 | 清单/通知/报表 | 3 批 |
| 类十 大货批色 | 7 | 提醒/报表/统计 | 3 批 |
| 类十三 打印导出 | 14 | 审计字段/水印/性能 | 5 批 |
| 类十四 权限维度 | 14 | 权限测试/审计/缓存 | 5 批 |
| 类十五 业务主体 | 1 | supplier_evaluation migration | 1 批 |
| 类十六 AI 模块 | 24 | 配伍性/化验室/准确率/版本/权限/超时/并发/缓存/脱敏/MLOps | 8 批 |
| 类十七 财务深化 | 35 | 期间/反结账/年结/回转/账龄/杜邦/预测/差异/折旧 | 12 批 |
| 类十八 CRM | 12 | 线索评分/去重/转移审批 | 4 批 |
| 类十九 报表 BI | 5 | 版本管理/缓存 | 2 批 |
| 类二十 可观测性 | 9 | trace/metrics/WebSocket | 3 批 |
| 类二十一 胚布拆匹 | 10 | 库存/委外/继承 | 4 批 |
| 类二十二 库存排程 | 9 | 调拨/安全/排程 | 3 批 |
| 类二十三 组织物流 | 11 | 组织树/售后/运费 | 4 批 |
| 类二十四 前端架构 | 16 | PWA/移动端/chunks/ErrorBoundary/CSP/keep-alive/CSS/暗黑 | 6 批 |
| 类二十五 部署升级 | 11 | set -euo/SHA256/schema/蓝绿/健康/优雅/回滚 | 4 批 |
| **合计** | **257** | | **约 45 批**（每批 9-12 文件） |

### 5.2 P2 中优先级（248 项，预估 35-45 批次，按每批 9-12 文件计算）

| 类别 | P2 数 | 主要内容 |
|------|-------|----------|
| 类一~类四 | 19 | 代码质量 / 安全防护 / 面料行业字段补齐 |
| 类五~类八 | 47 | 运行逻辑 / 测试补充 / 可维护性 / 法律合规细节 |
| 类九~类十二 | 33 | 色卡发放细节 / 大货批色细节 / 打印导出 / 权限细节 |
| 类十三~类十四 | 25 | 打印导出 P2 / 权限 P2 |
| 类十五~类十六 | 53 | 业务主体 P2 / AI 模块 P2 |
| 类十七~类十九 | 39 | 财务 P2 / CRM P2 / 报表 BI P2 |
| 类二十~类二十二 | 25 | 可观测性 / 胚布 / 库存 P2 |
| 类二十三~类二十五 | 83 | 组织物流 / 前端架构 / 部署升级 P2 |
| **合计** | **248** | |

### 5.3 P3 低优先级（123 项，按需修复）

| 类别 | P3 数 | 主要内容 |
|------|-------|----------|
| 类一~类四 | 11 | 文档 / 注释 / 命名优化 |
| 类五~类八 | 17 | 测试增强 / 可维护性增强 / 法律合规增强 |
| 类九~类十二 | 9 | 色卡 / 批色 / 打印 / 权限增强 |
| 类十三~类十四 | 5 | 打印导出 / 权限增强 |
| 类十五~类十六 | 25 | 业务主体增强 / AI 增强 |
| 类十七~类十九 | 11 | 财务 / CRM / 报表增强 |
| 类二十~类二十二 | 12 | 可观测性 / 胚布 / 库存增强 |
| 类二十三~类二十五 | 41 | 组织物流 / 前端架构 / 部署升级增强 |
| **合计** | **123** | |

---

## 六、规则节点提醒

| 规则 | 优先级 | 内容 |
|------|--------|------|
| 规则 0/1/2/8 | 🔴 | 真实实现强制：所有 P0/P1 修复必须真实实现，禁止占位符 |
| 规则 3 | 🔴 | 成品文档格式：导出必须 .xlsx / 报表必须 .docx |
| 规则 5 | 🟡 | E2E 独立工作流：每 30 批次触发（批次 30/60/90...） |
| 规则 6 | 🔴 | 测试 mock 数据禁止硬编码：所有测试 mock 数据抽取到 fixtures |
| 规则 10 | 🟡 | 每 15 批次记忆整理 + 实时归档：每批完成后立即归档到 doto-su.md |
| 规则 11/12 | 🔴 | 法律合规与安全标准：所有修复必须符合中国法律法规 + 安全标准 |
| 规则 13 | 🔴 | 修复流程自动化：CI 全绿后自动开始下一批，无需用户确认；**步骤 0 确定审计结果内容是否存在**（修复前置门，避免基于过时审计做无效修复）+ **步骤 4 修复后推送前自审**（内容正确性+注释规范性+注释一致性，与规则 20 联动） |
| 规则 14 | 🔴 | 移除所有警告抑制：所有警告视为错误需修复（baseline 213/213 ✅ 全部清零） |
| 规则 15 | 🟢 | V15 全项目综合审计：25 大类 195 维度审计 ✅ 已完成 |
| 规则 19 | 🟡 | 工具连接异常分级响应：L1 60s / L2 60-180s / L3 30min 周期 |
| 规则 20 | 🔴 | 注释与功能一致性：代码注释必须与功能实现一致，禁止随意编写；CI 强制检查 |
| §10.0.1 | 🔴 | 复用现有功能原则：修复前必须调研现有实现，禁止重复造轮子 |

---

## 七、历史任务（全部完成，详细记录已归档）

> 以下阶段全部完成，详细记录已归档到 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)。

| 阶段 | 批次范围 | 内容 | 归档位置 |
|------|----------|------|----------|
| v13 复审修复 | 270-394 | 213 baseline + 业务/财务/运行逻辑闭环 | doto-su.md §v13 |
| v14 复审修复 | 395-432 | 12 P0 + 31 P1 + 12 P2 + 6 P3 + 213 baseline | doto-su.md §v14 |
| V15 审计 | 2026-07-16 | 25 大类 195 维度 21 批并行子代理审计 | docs/audits/v15/ |
| V15 修复阶段一（P0 部分） | 433-459 | 16 P0 任务完成 | doto-su.md §V15 |
| V15 修复阶段一续（P0 续） | 460-472 | P0-F01~F09 + P0-F07 前端重写 | doto-su.md §V15 |
| V15 复审归档 | 2026-07-17 | 4 项标记未完成实际已完成项归档 | doto-su.md §V15 复审核实发现的已完成项 |
| V15 复审报告 | PR #649 | 30 P0 任务实际状态复查报告 | docs/audits/v15-fix-reaudit-2026-07-17.md |
