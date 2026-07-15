# V15 全项目综合审计计划（2026-07-15，第九轮升级版）

> **本文件性质**：审计**计划**（非结果），是 V8-V14 七轮复审 + DB N+1 审计 + E2E 报告 + 面料行业真实业务调研的综合互补与升级版。
> **触发条件**：v14 复审全部修复完成后（含批次 425-432 流转卡/验布/产量工资/能耗/染化料/委外/多业务模式/缸号状态机）自动触发。
> **核心目标**：在 v14 面料行业 17 维度之上，**深化面料行业全模块业务闭环**，并把"长期治理维度"（性能/测试/i18n/部署运维/法律合规/RBAC 权限/打印导出审计/权限维度审计/业务主体维度审计/AI 模块/财务深化/CRM 全链路/报表 BI 通知/可观测性运维/胚布拆匹质量/库存排程物料/组织定制物流/前端架构体验）独立成专项审计，最终形成 **24 大类共 190 个审计维度** 的最严格审计体系。
> **执行策略**：规则 13+14+15 联动，复审完成后自动连续修复，每批 5-8 文件，CI 全绿后自动进入下一批，无需用户确认；所有警告视为错误必须真实修复；修复前必须调研现有实现禁止重复造轮子。
> **真实业务依据**：[fabric-industry-research.md](file:///workspace/.monkeycode/docs/research/fabric-industry-research.md)（13 章节，覆盖染整工艺/化验室打样/大货处方/流转卡/车间工序/验布打卷/产量工资/能耗管理/缸号状态机）。
>
> **📋 项目规则与个人规则符合性声明**（用户 2026-07-15 第三轮反馈要求）：
> - 本审计计划严格遵守 [project_rules.md](file:///workspace/.trae/rules/project_rules.md) 项目开发规范（数据库 PostgreSQL / CI/CD Only 验证 / 死代码处理 / 安全规范 / 性能规范 / 错误处理规范 等）
> - 本审计计划严格遵守 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md) 项目规则记忆（规则 0-15 全部纳入审计范围）
> - 本审计计划严格遵守个人规则：实时查阅 .monkeycode/docs 规划文档 / MEMORY.md 规则 / doto.md 任务 / CHANGELOG.md 变更 / audit_assignment.md 审计任务分配
> - 所有审计发现的问题修复方式必须符合 CI/CD Only 验证规则（禁止本地编译，所有验证经 GitHub Actions）
>
> **🔑 关键业务规则修正**（用户 2026-07-15 三轮反馈累计）：
> 1. **色卡只发放给客户，不借出**：现有"借出-归还-遗失-损坏"模式必须重构为"发放"模式（详见类十）
> 2. **纺织行业法律与财税合规**：补充纺织行业特定法律法规 + 财税核算合规审计（详见类八 8.5-8.8）
> 3. **交货前客户批色**：大货交货给客户前，必须剪大货样让客户批色确认，批色通过才能交货（详见类十一）

---

## 一、审计背景与综合互补逻辑

### 1.1 V8-V14 七轮复审维度汇总

| 轮次 | 启动日期 | 维度数 | 核心发现 | 修复状态 |
|------|----------|--------|----------|----------|
| v8 | 2026-07-11 | 5 | 安全完整性 / 代码质量 / 死代码 / 测试覆盖 / 规范合规 | ✅ 21 项全部修复 |
| v9 | 2026-07-12 | 4 | backup/upgrade/system_update/admin/webhook/elastic 安全回归 | ✅ 16 项全部修复 |
| v10 | 批次 325-339 | 1 | clippy 警告抑制移除 + too_many_arguments DTO 重构 | ✅ 53 项全部修复 |
| v11 | 批次 340-346 | 1 | 警告抑制移除 + dead_code 真实接入 | ✅ 27 项全部修复 |
| v12 | 批次 347-355 | 7 | 死代码常量/桩代码/unwrap 加固/状态字符串/事务保护/API 一致性 | ✅ 15 项全部修复 |
| v13 | 2026-07-13 | 8 | baseline 清零 + 业务/财务/运行逻辑闭环（规则 15） | ✅ 295 项全部修复 |
| v14 | 2026-07-15 | 17 | 面料行业 17 维度（通用 3 + 行业特性 7 + 模块专项 7）+ 一个面料多颜色 | 🔄 12 P0+12 P2+6 P3+11 P1 完成，剩余 11 P1 由批次 425-432 覆盖 |

### 1.2 历史专项审计

| 审计 | 日期 | 范围 | 关键发现 |
|------|------|------|----------|
| DB N+1 审计 | 2026-06-18 | 144 service | 16 处 N+1（4 高 + 7 中 + 5 低）+ Redis 缓存层设计 |
| E2E 测试报告 | 2026-07-08（批次 190） | 95 个 E2E 用例 | 0 通过 / 88 失败 / 7 未跑完，6 类配置缺陷 |
| 面料行业调研 | 2026-07-15 | 13 章节 | 染整/打样/处方/流转卡/验布/工资/能耗/缸号状态机全流程 |

### 1.3 V15 综合互补逻辑

V15 不是简单叠加，而是**互补升级**：

1. **回归验证层**：v8-v14 所有已修复项的回归扫描（避免修复回退）
2. **深化层**：v13 业务/财务/运行逻辑闭环 + v14 面料行业 17 维度 → 在 v14 真实业务流程贯通（批次 425-432）后做**业务深化闭环验证**
3. **补强层**：v4-v5 已发现但未独立成专项的"长期治理维度"（可维护性/i18n/部署运维）独立成专项
4. **新增层**：基于规则 11/12（法律合规/法律安全标准）新增法律合规专项审计
5. **持续监控层**：规则 5/10/13/14 节奏保持 + CI/CD pipeline 健康度
6. **业务规则修正层**（V15 新增）：色卡管理"借还模式"→"发放模式"的业务规则修正专项
7. **纺织行业法律财税合规层**（V15 第三轮新增）：纺织行业特定法律法规 + 财税核算 + 环保 + 劳动合规独立审计
8. **大货批色业务规则层**（V15 第三轮新增）：交货前客户批色（剪大货样批色）业务规则专项
9. **RBAC 权限控制层**（V15 第五轮新增）：基于角色的权限控制机制（RBAC）系统架构审计，覆盖数据模型/权限矩阵/中间件/前端/审计日志/动态授权/数据权限/安全审计 8 维度
10. **打印导出审计与权限控制层**（V15 第六轮新增）：打印导出全链路审计 + 角色级权限矩阵 + 敏感数据二级审批 + 前端本地导出强制走后端 + 文件水印防泄露 + 合规定期审查 10 维度
11. **权限维度审计与角色合理性层**（V15 第七轮新增）：角色清单合理性审计 + 权限分配矩阵审计 + 职责分离 SoD 审计 + 权限-路由匹配审计 + is_system 滥用治理 + 前后端权限边界一致性审计 + 业务角色权限矩阵设计审计 + 权限粒度（行级+字段级）+ 权限缓存与性能审计 + 权限审计日志与合规审查 + 权限测试覆盖率审计 + 权限安全审计 12 维度
12. **业务主体维度审计与数据流转层**（V15 第八轮新增）：供货商维度（主数据完整性/业务闭环/面料行业特性）+ 加工商维度（委外加工商完全未实现，重大功能缺口）+ 销售维度（订单数据模型/业务流程闭环/面料行业特性）+ 客户维度（主数据完整性/信用与应收/面料行业特性）+ 数据流转维度（跨模块流转/业务回写/报表追溯/审计与异常检测）15 维度
13. **AI 模块审计层**（V15 第九轮新增）：14 个 AI 模块（ai_process_optimization/ai_quality_prediction/ai/{detect,pred,rec,recipe_opt}/ai_extend_service/advanced/*）的模型可解释性/数据安全/训练推理/权限控制/配方优化/质量预测/推荐/补货/性能/测试监控 10 维度
14. **财务深化审计层**（V15 第九轮新增）：会计期间结账/辅助核算/应收催收/账龄分析/财务分析/资金管理/预算管理/固定资产 8 维度
15. **CRM 全链路审计层**（V15 第九轮新增）：线索管理/商机阶段/客户池公海私海/数据权限/与销售模块数据流转 5 维度
16. **报表 BI 与通知协同审计层**（V15 第九轮新增）：报表定义/订阅/BI 分析/仪表板/通知中心/邮件服务/OA 公告/五维度分析 8 维度
17. **可观测性与运维审计层**（V15 第九轮新增）：trace 链路/metrics 指标/WebSocket 推送/故障转移/慢查询/API 网关/系统版本/日志增强 8 维度
18. **胚布拆匹与质量处理审计层**（V15 第九轮新增）：胚布库存采购/委托加工/拆匹缸号匹号继承/8D 质量处理/不合格品处理 5 维度
19. **库存排程物料审计层**（V15 第九轮新增）：库存调拨/库存告警/物料短缺/排程算法/产能规划/工作中心调度 6 维度
20. **组织定制物流审计层**（V15 第九轮新增）：部门管理/定制订单/售后/物流/incoterms 国际贸易术语 5 维度
21. **前端架构与体验审计层**（V15 第九轮新增）：响应式设计/路由懒加载/Pinia 状态管理/组件设计/composables/ECharts/WebSocket 客户端/前端性能/Vite 构建/前端测试/XSS 防护/敏感数据/WCAG 可访问性/错误边界/表单验证/i18n 深化/权限粒度/路由元信息/API 拦截器/主题样式 20 维度

---

## 二、V15 审计维度全景（24 大类 / 190 维度）

```
V15 审计体系
├── 一、回归验证类（5 维度）— v8-v14 已修复项防回退
├── 二、通用代码质量类（10 维度）— 综合 v4-v13
├── 三、安全性独立审计类（6 维度）— 综合 v8-v9 + 规则 11/12
├── 四、面料行业深化审计类（17 维度）— v14 + fabric-industry-research 13 章节
├── 五、运行逻辑闭环深化类（7 维度）— v13 规则 15
├── 六、测试体系审计类（7 维度）— 综合 v4/v8/e2e + 规则 5/6
├── 七、可维护性与长期治理类（5 维度）— v5 维度 13-15 + 部署运维
├── 八、法律合规与安全标准类（8 维度）⭐ — 规则 11/12 + 纺织行业法律/财税/环保/劳动合规
├── 九、批次节奏与记忆治理类（2 维度）— 规则 5/10/13/14 持续监控
├── 十、色卡发放业务规则修正专项（7 维度）⭐ — 用户 2026-07-15 反复强调"色卡只发放给客户，不借出"
├── 十一、大货批色业务规则专项（6 维度）⭐ — 用户 2026-07-15 第三轮反馈"交货前客户批色，剪大货样"
├── 十二、基于角色的权限控制机制（RBAC）专项（8 维度）⭐ — 用户 2026-07-15 第五轮反馈"增加 RBAC 机制"
├── 十三、打印导出审计与权限控制专项（10 维度）⭐ — 用户 2026-07-15 第六轮反馈"打印进入审计+角色权限矩阵+敏感数据二级审批"
├── 十四、权限维度审计与角色合理性专项（12 维度）⭐ — 用户 2026-07-15 第七轮反馈"那些角色应该有什么权限，那些权限不合理，那些角色不合理，那些角色权限不匹配"
├── 十五、业务主体维度审计与数据流转专项（15 维度）⭐ — 用户 2026-07-15 第八轮反馈"供货商/加工商/销售/客户/数据流转的功能全不全？合不合理？为什么？"
├── 十六、AI 模块审计专项（10 维度）⭐ — 用户 2026-07-15 第九轮反馈"所有维度都应该被严格审计"，覆盖 14 个 AI 模块（ai_process_optimization/ai_quality_prediction/ai/advanced）
├── 十七、财务深化审计专项（8 维度）⭐ — 用户 2026-07-15 第九轮反馈，覆盖会计期间/辅助核算/应收催收/账龄/财务分析/资金管理/预算/固定资产
├── 十八、CRM 全链路审计专项（5 维度）⭐ — 用户 2026-07-15 第九轮反馈，覆盖线索/商机/客户池/数据权限/与销售模块数据流转
├── 十九、报表 BI 与通知协同审计专项（8 维度）⭐ — 用户 2026-07-15 第九轮反馈，覆盖报表定义/订阅/BI/仪表板/通知/邮件/OA/五维度分析
├── 二十、可观测性与运维审计专项（8 维度）⭐ — 用户 2026-07-15 第九轮反馈，覆盖 trace/metrics/WebSocket/failover/slow_query/api_gateway/system_version/log
├── 二十一、胚布拆匹与质量处理审计专项（5 维度）⭐ — 用户 2026-07-15 第九轮反馈，覆盖胚布/拆匹/8D/不合格品处理
├── 二十二、库存排程物料审计专项（6 维度）⭐ — 用户 2026-07-15 第九轮反馈，覆盖库存调拨/告警/物料短缺/排程/产能/工作中心
├── 二十三、组织定制物流审计专项（5 维度）⭐ — 用户 2026-07-15 第九轮反馈，覆盖部门/定制订单/售后/物流/incoterms
└── 二十四、前端架构与体验审计专项（20 维度）⭐ — 用户 2026-07-15 第九轮反馈，覆盖前端 75+ views + 17 components + 36 composables + 5 stores + 85+ api + 20 个前端独有维度
```

---

## 三、详细审计维度清单

### 类一：回归验证类（5 维度）

> **目的**：防止 v8-v14 修复项回退，确保所有历史修复仍然有效。

#### 1.1 baseline 警告归零监控

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| `backend/.clippy-baseline.txt` 行数 | 持续保持 0 行（或仅 CI 自动刷新的极少量条目） | v10/v13 规则 14 |
| CI clippy job 严格模式 | `cargo clippy --all-targets -- -D warnings`（非 baseline 模式） | 规则 14 |
| 新增警告 | 任何新增警告立即 CI 失败，禁止 `#[allow(...)]` 抑制 | 规则 14 |
| baseline 自动刷新机制 | main 分支修复历史警告后 CI 自动刷新基线 | v13 批次 395-396 |
| `backend/.clippy.toml` 配置 | `warn` 段开启 `dead_code`/`unused_imports`/`unused_variables` | 项目规则第六章 |

**回归扫描方法**：
```bash
# 检查 baseline 行数
wc -l backend/.clippy-baseline.txt
# 检查全项目 #[allow(...)] 抑制（除 backend/src/models/ SeaORM 例外）
grep -rn "#\[allow(" backend/src/ | grep -v "backend/src/models/" | wc -l
# 检查 CI clippy 命令
grep -A 5 "Rust Clippy" .github/workflows/ci-cd.yml
```

#### 1.2 死代码与警告抑制持续监测

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 文件级 `#![allow(dead_code)]` | 仅 `backend/src/models/` 下 SeaORM 自动生成模型可保留 | v11/v12 规则 14 |
| 项级 `#[allow(dead_code)] + TODO(tech-debt)` | 全部清零或真实接入业务 | 项目规则第六章 |
| `#[allow(unused_imports)]` / `#[allow(unused_variables)]` | 全部清零 | 规则 14 |
| `#[allow(clippy::too_many_arguments)]` | 11 个历史标注全部清理（批次 411-413 已规划） | audit_assignment §3.3 |
| 占位变量 `let _ =` | 仅允许在显式吞错且已加 `tracing::warn!` 的场景 | 规则 0 |

#### 1.3 v13 业务/财务/运行逻辑闭环回归

**业务全链路闭环**（来源 v13 规则 15）：
- 下单 → 履约 → 收付款 → 售后 → 报表全链路无断点
- 销售订单审批 → 库存预留/锁定（B-P0-1 修复保持）
- 销售出库 → 库存凭证 + 收入凭证 + 成本凭证（B-P0-2 + F-P0-3 修复保持）
- 生产订单完成 → 成本归集（B-P0-3 修复保持）
- 采购退货 → 财务凭证（B-P0-5 修复保持）
- 销售退货 → 财务凭证（B-P0-6 修复保持）

**财务全链路闭环**：
- 凭证 → 科目 → 账簿 → 报表 → 对账 → 结账全链路有效
- 凭证科目余额回写（F-P0-1 修复保持）
- 库存桥接凭证 create + post（F-P0-2 修复保持）
- AR 收款/AP 付款生成凭证（F-P0-4/F-P0-5 修复保持）
- 销售→应收/采购→应付链路（F-P0-6/F-P0-7 修复保持）
- AR/AP 核销生成凭证（F-P0-8 修复保持）

**运行逻辑环流程闭环**：
- 输入 → 处理 → 输出 → 反馈 → 输入闭环保持
- 异常路径闭环：所有 `Result<T, E>` 有错误处理路径
- 状态机闭环：所有枚举状态有终态
- 资源生命周期闭环：spawn 句柄保存到 EventBusState
- 配置依赖闭环：`.env.example` → `config.yaml` → `main.rs` 校验

#### 1.4 v8-v9 安全修复回归

| 安全修复项 | 期望状态 | 来源 |
|------------|----------|------|
| webhook SSRF 防护 | `validate_url_and_resolve` + `resolve_to_addrs` + scheme 白名单 | v8 H1/M2 + v9 M-5 |
| system_update SSRF 防护 | `download_update` + `fetch_latest_release` 全部对齐 SSRF 防护 | v8 M1 + v9 M-1 |
| backup Tar Slip 防护 | UUID 随机目录 + `tar -tf` 预校验 + 逐文件校验 + 递归深度限制 100 | v8 H2/H3/M4 |
| upgrade Tar Slip 防护 | 对齐 backup.rs 方案（UUID + 预校验 + 二次校验） | v9 H-1 |
| admin 密码处理 | `--password-stdin` 或 `BINGXI_ADMIN_PASSWORD` 环境变量 | v9 H-2 |
| webhook IDOR 校验 | 所有端点校验 `webhook.user_id == auth.user_id` | v9 M-4 |
| webhook retry 限流 | `retry_webhook` 接入 `check_rate_limit` | v9 M-3 |
| 日志脱敏 | URL 不记录凭据，只记录"已配置" | v8 H4 |
| 密钥生成 | base64 32（熵比 > 0.15），4 密钥自动生成 | 批次 401 |
| 部署路径一致性 | `grep -r '/etc/bingxi' deploy/` 全部一致 | 批次 398 |

#### 1.5 v14 面料行业核心约束保持

**四层级联关系**（v14 §2.2.1）：
```
面料（Fabric/Product）
  └── 颜色（Color）              ← 一个面料有多个颜色（核心特性）
       ├── 色号（ColorNo）        ← 颜色唯一编码
       ├── 色卡（ColorCard）      ← 颜色实物样卡（V15 修正：只发放，不借出）
       └── 缸号（DyeLotNo）       ← 每个颜色每批次染色有独立缸号
            ├── 批号/匹号（BatchNo）← 同一缸号下可分多个批次（匹号）
            ├── 库存数量          ← 按缸号/批号独立核算
            ├── 质检结果          ← 按缸号/批号独立检验
            ├── 成本单价          ← 按缸号/批号独立核算（实际成本）
            └── 库位              ← 按缸号分区存放
```

**关键业务约束**（v14 §2.2.2 必须强制执行）：
- **匹号唯一约束**：`UNIQUE(dye_lot_no, batch_no)` 在所有含 batch_no 的表保持有效
  - 库存表、入库表、发货表、退货表、盘点表、验布记录表、打卷记录表、产量记录表
  - Service 层：创建/更新时校验 `(dye_lot_no, batch_no)` 组合唯一
- **面料-颜色关联约束**：`UNIQUE(product_id, color_id)` 保持
- **缸号-颜色关联约束**：`UNIQUE(color_id, dye_lot_no)` 保持
- **库存四维标识**：`UNIQUE(product_id + color_id + dye_lot_no + batch_no)` 联合唯一索引保持
- **缸号同订单校验**：出库时同一订单必须使用相同缸号的面料

**回归扫描方法**：
```bash
# 检查所有含 batch_no 的表是否保留唯一索引
grep -rn "UNIQUE.*dye_lot_no.*batch_no\|UNIQUE.*batch_no.*dye_lot_no" backend/migration/
# 检查 Service 层唯一性校验
grep -rn "dye_lot_no.*batch_no\|batch_no.*dye_lot_no" backend/src/services/ | grep -i "unique\|exist\|conflict"
```

---

### 类二：通用代码质量类（10 维度）

> **目的**：综合 v4-v13 历史所有代码质量维度，持续保持。每个维度提供详细检查项 + 扫描方法 + 修复模式。

#### 2.1 事务边界与原子性

**检查要点**（来源 v4 维度 1 / v7）：
1. 状态机转换函数全部走 `txn.begin() → lock_exclusive → 状态门 → update_with_audit(&txn) → txn.commit()` 完整流程
2. CRUD 类 `update_with_audit(&*self.db)` 已原子化或迁移入事务
3. 单号生成器（如 order_no 生成）在事务内调用，避免幻单号
4. 跨表操作（如订单 + 明细 + 库存）在同一事务内
5. 事务 begin 后状态门 return Err 依赖 Drop 回滚（代码异味，建议显式 rollback + 日志）

**扫描方法**：
```bash
# 查找非事务的 update_with_audit 调用
grep -rn "update_with_audit(&\*self.db)" backend/src/services/
# 查找事务内但无 lock_exclusive 的状态门
grep -B 5 -A 10 "txn.begin()" backend/src/services/ | grep -v "lock_exclusive"
```

**修复模式**：
```rust
let txn = (*self.db).begin().await?;
let existing = Entity::find_by_id(id).lock_exclusive().one(&txn).await?
    .ok_or_else(|| AppError::not_found("..."))?;
// 状态门检查
if existing.status != Status::Draft {
    return Err(AppError::business("状态不允许此操作"));
}
// update_with_audit 传 &txn
update_with_audit(&txn, "auto_audit", "操作描述", Some(user_id)).await?;
txn.commit().await?;
```

#### 2.2 输入验证与 SQL 注入防护

**检查要点**（来源 v4-v5 维度 2）：
1. SeaORM 参数化查询保持（禁止字符串拼接 SQL）
2. 所有 DTO 含 `#[derive(Validate)]` + 字段级 `#[validate(range(min = 0))]` 等
3. 所有 handler 调用 `.validate()` 显式校验
4. 金额非负校验（`amount >= 0`）+ 自转转防护
5. URL scheme 白名单（webhook URL 校验 http/https）
6. 文件上传大小/类型/内容校验
7. PageRequest `page_size` 上限 clamp(1, 100) + page 饱和减 `saturating_sub(1)`
8. SQL 注入审计中间件大小写不敏感（防混合大小写绕过）

**扫描方法**：
```bash
# 查找无 Validate derive 的 DTO
grep -rn "pub struct.*Dto" backend/src/models/ | grep -v "Validate"
# 查找未调用 validate() 的 handler
grep -B 2 -A 5 "Json<.*Dto>" backend/src/handlers/ | grep -v "validate()"
```

#### 2.3 错误处理与日志完整性

**检查要点**（来源 v4-v5 维度 3 / v13 异常路径闭环）：
1. 禁止 `let _ =` 吞错（除显式 `tracing::warn!` 场景）
2. 所有 `Result<T, E>` 有错误处理路径
3. 金额服务（finance_payment/ap_payment/ar_service）tracing 覆盖
4. `Arc::try_unwrap().unwrap()` 改为 `unwrap_or_else` 优雅降级
5. `expect()` 改为 `unwrap_or_else(|_| { eprintln!(...); std::process::exit(1); })`（启动期）
6. 错误分类正确（business vs system，禁止 business 包装 system）
7. NotFound/Unauthorized/PermissionDenied 日志级别恰当
8. handler 层事件通知不静默吞错

**扫描方法**：
```bash
# 查找吞错
grep -rn "let _ = " backend/src/ | grep -v "test\|warn\|//"
# 查找 expect
grep -rn "\.expect(" backend/src/ | grep -v "test"
# 查找 Arc::try_unwrap().unwrap()
grep -rn "Arc::try_unwrap.*unwrap()" backend/src/
```

**修复模式参考**（MEMORY.md 规则 0）：
- `let _ = svc.method().await;` → `if let Err(e) = svc.method().await { tracing::warn!(error=%e, "描述"); }`
- `let _ = exists_check;` → `exists_check.await?;`
- `.expect("msg")` → `unwrap_or_else(|_| { eprintln!("友好提示"); std::process::exit(1); })`

#### 2.4 业务逻辑与状态机

**检查要点**（来源 v4-v5 维度 4 / v13 状态机闭环）：
1. 状态枚举大小写跨模块统一（v4 已发现根因问题）
   - 采购订单：大写 DRAFT/PENDING_APPROVAL/APPROVED/COMPLETED
   - 销售订单：小写 draft/pending/approved/cancelled
   - 凭证：小写 draft/submitted/reviewed/posted
   - **V15 要求**：评估是否需要全局统一为大小写一致（或显式记录差异原因）
2. 所有状态机有终态（无孤儿状态）
3. reject 后有 resubmit 路径（无死状态）
4. 状态转换合法性检查（如已 PAID 不可重复 mark_as_paid）
5. 库存调整 quantity_kg 计算逻辑正确
6. 销售订单 create_order 检查客户 active 状态
7. 采购订单 create_order 检查供应商 is_enabled
8. 销售订单 submit_order 信用额度检查后锁定额度

#### 2.5 并发与竞态

**检查要点**（来源 v4-v5 维度 5）：
1. TOCTOU 18 处全部修复（检查时序-使用时序一致）
2. 丢失更新 12 处全部修复（并发更新走 `lock_exclusive()`）
3. 重复操作 8 处全部修复（幂等性保证）
4. 库存超扣 3 处全部修复
5. 乐观锁使用正确（version 字段校验）
6. 悲观锁 `SELECT FOR UPDATE` 在并发敏感场景使用
7. 幂等性保证（事件 payload 唯一键，监听器消费前检查）
8. 分布式锁缺失（Redis 分布式锁在多实例场景）

**扫描方法**：
```bash
# 查找无 lock_exclusive 的状态机转换
grep -B 10 "txn.begin()" backend/src/services/ | grep -v "lock_exclusive" | grep "find_by_id"
```

#### 2.6 性能与 N+1 查询

**检查要点**（来源 DB N+1 审计 / v4-v5 维度 6）：
1. DB N+1 审计 16 处全部修复
   - 4 处高严重度（ap_verification_service 3 处 + ar/vfy.rs 1 处）
   - 7 处中严重度（ap_payment_request/crm pool/purchase_delivery/so delivery/inventory_adjustment/purchase_receipt/po receipt）
   - 5 处低严重度（ap_verification/bom/tenant_billing/mrp/scheduling）
2. 分页偏移 `(page-1)*page_size` 正确（禁止 `page*page_size` off-by-one）
3. 全表查询禁止（finance_invoice_service::list_invoices 等）
4. 循环内查询改预查询 + HashMap
5. 循环内 insert 改批量插入
6. 缓存层接入 5 个 service（user/product/customer/supplier/role）

**N+1 修复模板**（DB N+1 审计 §3）：
```rust
// 改造前
for item in &req.items {
    let obj = SomeEntity::find_by_id(item.obj_id).one(db).await?;
    process(obj);
}
// 改造后
let ids: Vec<i32> = req.items.iter().map(|i| i.obj_id).collect();
let objs = SomeEntity::find()
    .filter(SomeEntity::Column::Id.is_in(ids))
    .all(db).await?;
let map: HashMap<i32, SomeEntity::Model> =
    objs.into_iter().map(|o| (o.id, o)).collect();
for item in &req.items {
    process(map.get(&item.obj_id));
}
```

#### 2.7 依赖配置与敏感信息

**检查要点**（来源 v4-v5 维度 7 / v8 H4/M3/M7）：
1. `.env.example` 占位符不绕过 `validate_secret`（拒绝 `your_jwt_secret_at_least_32_chars_long` 等模式）
2. config.yaml/test.yaml 不硬编码密码
3. `AUDIT_SECRET_KEY` 非生产环境不使用默认值
4. ci-audit 不 `continue-on-error: true`
5. `init_service.rs` 不硬编码 `sslmode=disable`
6. `.cargo/audit.toml` 忽略漏洞有过期时间
7. `hash_password.rs` CLI 工具不默认使用 "admin123"
8. reqwest 客户端配置 TLS 最低版本
9. HTTP→HTTPS 重定向存在

#### 2.8 架构与死代码

**检查要点**（来源 v4-v5 维度 8-9）：
1. utils 反向依赖 services 已解耦（`utils/app_state.rs` 不依赖 17 个 services 模块）
2. handler 不跨层调用 model（违反 handler → service → model 分层）
3. 零引用 pub 项清零
4. 未使用 use 清零
5. 占位模块（bpm_service_stub/missing_handlers/performance_optimizer/n_plus_one）已真实实现或删除
6. inventory_count_service facade 11 个方法全部实现
7. 466 个未使用的前端导出清零

#### 2.9 前端 API 端点与类型安全

**检查要点**（来源 v4-v5 维度 9-10）：
1. `as any` 清零（v11 批次 160-196 已完成 frontend/src 清理，V15 验证回归）
2. baseURL 不双 `/api/v1/erp`（color-card/color-price/custom-order 3 个 API 文件）
3. `request.ts` 401 自动刷新逻辑链有效（业务码 401 触发刷新）
4. CSRF `isCsrfPublicPath` 精确匹配（非 `includes` 包含匹配）
5. `ErrorResponse.trace_id` 字段存在
6. `Setup.vue` 不使用原生 `fetch` 绕过 axios 拦截器
7. `api/quotation.ts` 无 `as any`
8. `types/api.ts` `PageResult<T = any>` 改为具体类型
9. `composables/useTableApi.ts` 无 `any`
10. `store/user.ts` token 半迁移残留清理
11. `MainLayout.vue` 银行账号等敏感业务数据不存 localStorage

#### 2.10 前端路由与权限

**检查要点**（来源 v4-v5 维度 10-11）：
1. `v-permission` 全量覆盖（v4 仅 1 文件使用，V15 验证 100+ 视图全覆盖）
2. 路由守卫检查 permission（非仅检查 token）
3. Open Redirect 白名单（`redirect` query 参数校验）
4. 权限码与后端 `init_admin_permissions.sql` 一致（如 `inventory:update` vs `inventory:stock:edit`）
5. `store/user.ts` permissions 数组有类型保护
6. `hasRoutePermission` 严格模式（admin 绕过 + 空权限放行 + 通配符评估）
7. `MainLayout.vue` 菜单按 permission 过滤逻辑与守卫一致
8. `checkInitStatus` 错误时不默认置为 `true`
9. 已登录用户访问 /login 跳转首页
10. 权限不足跳转 403 时有用户提示
11. `RoleTab.vue` 权限配置按钮有权限控制
12. `system/index.vue` 12 个 Tab 子组件懒加载
13. `MainLayout.vue` 响应式设计（移动端）

---

### 类三：安全性独立审计类（6 维度）

> **目的**：v8-v9 安全修复基础上的持续安全审计，叠加规则 11/12 法律安全标准。每个维度提供详细检查项 + 扫描方法。

#### 3.1 SSRF 防护完整性

**检查项**（来源 v8 H1/M1/M2/M5 / v9 H-1/M-1/M-2/M-5）：
1. webhook URL 走 `validate_url_and_resolve` + `resolve_to_addrs` 防 DNS Rebinding
2. webhook URL scheme 白名单（拒绝 `file://`、`http://169.254.169.254/` 内网元数据）
3. `system_update_service::download_update` 全部走 SSRF 防护
4. `system_update_service::fetch_latest_release` 对齐 `download_update` 的 SSRF 防护
5. `elastic.rs` ES base_url 走 SSRF 校验 + `redirect(Policy::none())`
6. `currency_service.rs` exchangerate API URL 走 SSRF 校验
7. `reqwest::Client::builder().build()` 不 `unwrap_or_default()`（避免丢弃 SSRF 配置）
8. `asset.name` 校验安全字符（拒绝 `/`、`..`、绝对路径）

**扫描方法**：
```bash
# 查找所有 reqwest 客户端构建
grep -rn "reqwest::Client::builder" backend/src/
# 查找 unwrap_or_default 在 reqwest 构建中
grep -B 5 "unwrap_or_default()" backend/src/ | grep "reqwest\|Client::builder"
# 查找无 SSRF 校验的外部 URL 调用
grep -rn "reqwest::get\|http::get" backend/src/
```

#### 3.2 路径穿越防护

**检查项**（来源 v8 H2/H3/M4 / v9 H-1/M-2）：
1. `backup.rs` Tar Slip 防护：UUID 随机目录 + `tar -tf` 预校验 + 逐文件校验
2. `backup.rs` `validate_dir_recursive` 递归深度限制 100 层
3. `backup.rs` 临时目录随机生成（非固定 `/tmp/bingxi_restore`）
4. `upgrade.rs` 对齐 backup.rs 方案（UUID + 预校验 + 二次校验）
5. `system_update_service.rs` `extract_update_package` 权限分别设置（目录 0o755 + 文件 0o600）
6. 文件上传校验路径（拒绝 `..`、绝对路径）
7. `validate_extracted_path` + `validate_dir_recursive` 抽取到共享模块（消除重复代码）

**扫描方法**：
```bash
# 查找固定临时路径
grep -rn "/tmp/bingxi" backend/src/
# 查找 tar 解压命令
grep -rn "tar.*-xzf\|tar.*-xzf" backend/src/
# 查找 validate_extracted_path 实现
grep -rn "fn validate_extracted_path\|fn validate_dir_recursive" backend/src/
```

#### 3.3 密钥与凭据安全

**检查项**（来源 v8 H4/M3/M7 / v9 H-2 / 批次 401）：
1. JWT_SECRET/COOKIE_SECRET/WEBHOOK_SECRET/AUDIT_SECRET_KEY 四密钥自动生成
2. 密钥生成算法 base64 32（熵比 > 0.15，64 种字符）
3. 密钥存储 600 权限（`~/.git-credentials` 等）
4. 密钥传输 HTTPS
5. 日志只记录"已配置"不记录 URL（避免凭据泄露）
6. 密码改 `--password-stdin` 或 `BINGXI_ADMIN_PASSWORD` 环境变量
7. GitHub Token 不写入任何 git 跟踪文件
8. 密钥生成在 `/etc/bingxi/.env` 持久化
9. `deploy-latest.sh` 首次部署自动检测并生成密钥
10. `deploy.sh` 密钥生成对齐 `deploy-latest.sh`

#### 3.4 认证与权限

**检查项**（来源 v4 维度 12 / 规则 1-9 第 7 条）：
1. 公开端点仅登录/刷新/健康检查（其他全部需认证）
2. CSRF 精确匹配（非 `includes` 包含匹配）
3. CORS 配置非 `*`（白名单）
4. JWT 过期时间可配置（非硬编码 7 天）
5. refresh token 支持撤销
6. Session Secure/SameSite 属性设置
7. 密码强度策略（强密码要求）
8. 2FA TOTP 支持（`is_totp_enabled` 字段）
9. JTI 黑名单 Redis 迁移
10. 登录失败锁定机制

#### 3.5 速率限制与防暴力破解

**检查项**（来源 v8 M6 / v9 M-3/M-4）：
1. 登录端点接入 `check_rate_limit`（Redis 优先 + 内存回退）
2. 重置密码端点接入 `check_rate_limit`
3. `test_webhook` 接入 `check_rate_limit`（v8 M6 修复）
4. `retry_webhook` 接入 `check_rate_limit`（v9 M-3 修复）
5. 分布式限流回退策略（Redis 故障时内存限流）
6. 限流器支持分布式部署（非纯内存 `MemoryRateLimiter`）
7. 接口速率限制防 DDoS
8. 文件上传频率限制

#### 3.6 IDOR 与越权校验

**检查项**（来源 v9 M-4 / v4 维度 2 P1）：
1. webhook 所有端点校验 `webhook.user_id == auth.user_id`
2. 所有 CRUD 操作前校验所有权
3. DataPermissionFilter 接入业务模块
4. 权限码命名一致（view/edit vs read/update）
5. 越权校验缺失（未校验 `created_by == user_id`）
6. 子表查询有 tenant/owner 过滤
7. `define_crud_handlers!` 宏支持权限校验
8. 4 个历史无认证 handler（logistics/greige_fabric/dye_recipe/dye_batch）已修复

---

### 类四：面料行业深化审计类（17 维度）

> **目的**：v14 17 维度基础上的面料行业全模块业务深化闭环验证，基于 fabric-industry-research.md 13 章节真实业务调研。
> **核心特性**：一个面料有多个颜色（面料→颜色→缸号→批号四层级联关系），所有维度均围绕此核心特性展开。
> **关键业务规则修正**（用户 2026-07-15）：色卡只发放给客户，不借出。

#### 4.1 通用审计维度（3 项，v14 10.1 升级）

##### 4.1.1 业务功能完整性

**V15 深化要点**（在 v14 CRUD 完整性基础上）：
1. **业务流程闭环**：每个面料行业模块必须形成"输入→处理→输出→反馈→输入"完整闭环
   - 化验室打样：打样通知单→ABCD 多版样→OK 样→复样→染色技术卡→反馈工艺优化
   - 大货处方：化验室 OK 样→大货处方单→审核→生产领用单→染色→反馈成本
   - 流转卡：扫码上报→工序流转→进度跟踪→反馈调度优化
   - 验布打卷：十项指标→A/B/C 分级→打卷入库→反馈质检改进
   - 产量工资：缸号产量→A/B/C 分级金额→班组汇总→反馈工资激励
   - 能耗管理：水电汽采集→按缸号归集→月末分摊→反馈节能优化
2. **并发安全**：在面料多颜色场景下保持（如同一面料多颜色并发创建）
3. **幂等性**：工序扫码上报/产量上报/能耗采集 幂等保证
4. **CRUD 完整**：所有面料行业模块 CRUD 完整（无 NotImplemented 占位）
5. **接入业务**：所有功能真实接入业务链路（路由 → handler → service → model → DB 全链路打通）

##### 4.1.2 逻辑完整性

**V15 深化要点**（在 v14 状态流转连贯性基础上）：
1. **缸号状态机闭环**：投染→染色→出缸→质检→入库→发货→退货全生命周期无悬挂中间态
2. **染整工单状态机**：包含 Failed/OnHold 异常态 + 恢复路径
3. **色卡状态机**（V15 修正）：发放 → 已收到 → 已使用 → 已过期（不借出/不归还/不遗失）
4. **事务边界正确**：跨表操作（如缸号状态变更 + 库存更新 + 凭证生成）在同一事务
5. **异常路径有恢复**：染整工序失败有恢复路径（重染/补染/降级）
6. **分支覆盖完整**：所有 if/else 分支有处理路径

##### 4.1.3 数据流转性

**V15 深化要点**（在 v14 跨模块数据连贯基础上）：
1. **面料行业业财一致性**：
   - 销售出库（按缸号）→ 收入凭证 + 成本凭证
   - 采购入库（按缸号）→ 存货凭证 + 应付凭证
   - 染整领料 → 成本凭证（按缸号归集）
   - 生产订单完成 → 成本归集（按缸号）
   - 库存调整（缸号维度）→ 差异凭证
   - 收付款 → 核销凭证
   - 月末能耗分摊 → 成本凭证
   - 产量工资 → 人工成本凭证
2. **跨模块数据连贯**：订单 → 生产 → 染整 → 入库 → 发货 → 售后 → 报表 全链路无断点
3. **主数据变更同步**：客户/供应商主数据变更同步关联单据（B-P1-3 修复保持）
4. **报表可追溯**：所有报表数据可穿透追溯到凭证/明细账（F-P2-2 修复保持）

#### 4.2 面料行业特性审计维度（7 项，v14 10.2 升级）

##### 4.2.1 面料行业转项特性优化

**V15 深化要点**（基于 fabric-industry-research §1.2 + §2）：
1. **双单位换算深化**：米/公斤/码/匹四单位动态换算
   - 公式：`公斤 = 米 × 幅宽(m) × 克重(g/m²) ÷ 1000`
   - 公式：`码 = 米 ÷ 0.9144`
   - 公式：`匹 = 总米数 ÷ 匹长`
   - `utils/dual_unit_converter` 接入所有面料相关模块
2. **缸号/色号/批号管理**：全链路保持（v14 §2.2.1 四层级联关系）
3. **面料编码体系**：SKU = 面料编码 + 色号 + 缸号 + 匹号
4. **27 项面料核心属性**（克重/幅宽/缩率/色牢度/经纬密度/成分百分比/纱支等）字段完整
5. **多计量单位**：应对不同客户对产品下单数量的计量单位不统一
6. **预留订单**：确定下单但未确定颜色等属性，企业可提前生产坯布

##### 4.2.2 面料行业术语

**V15 深化要点**（基于 fabric-industry-research §2.2）：
1. **全栈统一**：代码/注释/UI/DB/API 全栈使用行业术语
2. **术语定义统一**：
   - 色号（ColorNo）：面料基础颜色编码
   - 缸号（DyeLotNo/VatNo）：面料染色批次（同色不同缸存在肉眼可见色差，裁床严禁不同缸号面料混铺）
   - 卷号（RollNo）：单匹独立面料编号（面料库存最小管理单元）
   - 匹号（BatchNo）：同一缸号下分多个匹号
3. **染整工艺 10 道工序术语统一**：配布→精练→漂白→染色→对色→理布→烘干→定型→成品对色→成检
4. **印花工艺术语统一**：制版→调浆→印花→烘干→汽蒸→水洗→定型
5. **质检标准术语统一**：四分制/十分制疵点评分 + 色差 ΔE + A/B/C 级判定

##### 4.2.3 面料行业流程完善情况

**V15 深化要点**（基于 fabric-industry-research §3 + §11）：
1. **染整工艺完整闭环**：10 道工序（配布→精练→漂白→染色→对色→理布→烘干→定型→成品对色→成检）
2. **印花工艺完整闭环**：制版→调浆→印花→烘干→汽蒸→水洗→定型
3. **工艺路线可配置**（好业财模式）：
   - 主工艺路径下可嵌套预处理子流程、染色核心工序、后整理分支链路
   - 每道工序绑定：特定设备组、工时定额、水电汽基准消耗值、关键控制点（pH 值区间、温度曲线拐点）
   - 不预设固定工序数量，企业可依实际产线配置灵活增删节点
4. **色卡管理**（V15 修正）：
   - **色卡只发放给客户，不借出**（用户 2026-07-15 明确）
   - 客户专属色卡库
   - 历史色卡可追溯
   - 复购管理：客户复购指定同缸号（颜色一致性），系统提示库存
   - 现有"借出-归还-遗失-损坏"模式必须重构为"发放"模式（详见类十）
5. **质检流程**：四分制/十分制疵点评分 + 色差 ΔE + A/B/C 级判定
6. **成本核算流程**：按缸号实际成本法 + 染整成本归集 + 实际成本计算
7. **工艺参数与质量双向映射**：验布"色光偏黄"反向锁定染料批次+助剂顺序+蒸化温度+蒸汽压力曲线

##### 4.2.4 面料行业数据模型

**V15 深化要点**（基于 fabric-industry-research §2.1-§2.3）：
1. **四层级联关系**：面料→颜色→缸号→批号（匹号）
2. **缸号承载信息**（远超普通生产编号）：
   - 坯布来源（供应商 + 批次）
   - 染色配方（染料 + 助剂 + 工艺参数）
   - 操作班组
   - 设备机台
   - 水电气实耗
   - 首件确认结果
   - 全流程质检记录
   - 最终落布重量
   - 色差评级
3. **库存四维标识**：`UNIQUE(product_id + color_id + dye_lot_no + batch_no)` 在所有库存相关表保持
4. **匹号唯一约束**：`UNIQUE(dye_lot_no, batch_no)` 全局适用
5. **面料-颜色关联约束**：`UNIQUE(product_id, color_id)`
6. **缸号-颜色关联约束**：`UNIQUE(color_id, dye_lot_no)`
7. **缸号同订单校验**：出库时同一订单必须使用相同缸号的面料

##### 4.2.5 面料行业报表

**V15 深化要点**：
1. **染整报表**：按缸号产量/质量/能耗
2. **色卡报表**：客户专属/历史/复购（V15 修正：发放记录报表，非借还记录）
3. **成本报表**：按缸号实际成本 + 染整成本归集 + 月末能耗分摊
4. **库存周转报表**：按四维标识周转率
5. **产量工资报表**：按缸号计件 + A/B/C 分级金额
6. **能耗报表**：水电汽分摊 + 按缸号归集 + 月末分摊到成本
7. **报表穿透追溯**（F-P2-2 修复保持）：所有报表数据可穿透到凭证/明细账

##### 4.2.6 面料行业权限

**V15 深化要点**：
1. **色卡管理权限**（V15 修正）：
   - 色卡发放权限（发放给客户，非借出）
   - 客户专属色卡查看/编辑权限
   - 色卡历史追溯权限
2. **缸号管理权限**：
   - 缸号创建权限
   - 缸号状态变更权限
   - 缸号成本查看权限（敏感数据）
3. **成本查看权限**：按缸号实际成本敏感数据
4. **染化料管理权限**：染料/助剂/坯布主数据
5. **能耗数据查看权限**：水电汽实耗数据
6. **产量工资查看权限**：按缸号计件工资数据
7. **工艺配方查看权限**：染色配方/大货处方（商业机密）

##### 4.2.7 面料行业集成

**V15 深化要点**：
1. **色卡与订单集成**（V15 修正）：
   - 客户专属色卡 → 订单引用 → 复购指定同缸号
   - 色卡发放记录与订单关联（非借还记录）
2. **缸号与库存集成**：
   - 四维标识 → 库存核算 → 库位管理
   - 缸号分区存放
3. **成本与凭证集成**：
   - 按缸号实际成本 → 财务凭证 → 月末结转
4. **产量与工资集成**：
   - 缸号产量 → A/B/C 分级 → 工资计算
5. **能耗与成本集成**：
   - 水电汽分摊 → 缸号归集 → 月末分摊到成本
6. **染化料与库存集成**：
   - 染料/助剂/坯布主数据 → 批号管理 → 安全库存 → 领用退回
7. **化验室打样与大货处方集成**：
   - 打样 OK 样配方 → 大货处方单引用 → 染色配料单

#### 4.3 面料行业模块专项审计维度（7 项，v14 10.3 升级为面料行业真实模块）

> **v14 vs v15 差异**：v14 维度 10.3.1-10.3.7 是"人事/仓库/销售/公司/权限/财务/CRM"通用模块；v15 改为面料行业真实业务模块（基于 fabric-industry-research §11-§13 真实业务）。

##### 4.3.1 化验室打样模块

**真实业务依据**：fabric-industry-research §11.1 + §4.2.3
**V15 检查要点**：
1. **5 步闭环**：打样通知单 → ABCD 多版样（多版次送样） → OK 样确认 → 复样（客户要求重做） → 染色技术卡
2. **DATACOLOR 自动滴液设备对接**：自动读取客户确定后的颜色配方
3. **打样色号引用标准配方**：标准配方维护 + 打样色号引用
4. **打样计划管理**：来样登记 → 打样计划 → 打样跟踪 → 送样及取消管理 → 确认 OK 样及修改
5. **打样质量参数维护**：色差 ΔE + 色牢度 + 同色异谱评估
6. **复板计划管理**：制订/分配/确认复板计划
7. **与染色工艺管理集成**：打样 OK 样 → 染色工艺参数设定 → 大货处方引用
8. **打样进度查询**：打样计划查询 + 打样进度查询 + 送样登记查询

##### 4.3.2 大货处方与加料处方模块

**真实业务依据**：fabric-industry-research §11.2 + §4.2.4
**V15 检查要点**：
1. **大货处方单**：染色配料单（同工单唯一）
2. **审核后自动建生产领用单**：处方审核 → 自动生成染化料领用单
3. **加料处方**：中途加料记录（染色过程中补加染料/助剂）
4. **与化验室打样 OK 样配方联动**：大货处方引用打样 OK 样配方
5. **染料/助剂消耗自动归集到缸号成本**：处方执行 → 染化料消耗 → 缸号成本归集
6. **工序参数设定**：温度曲线 + 时间 + pH 值 + 助剂添加顺序
7. **染色工艺参数维护**：染色方式 + 色系色级 + 工艺路线
8. **处方变更管理**：处方变更需重新审核 + 历史追溯

##### 4.3.3 流转卡与车间工序模块

**真实业务依据**：fabric-industry-research §11.3 + §12 + §3.4
**V15 检查要点**：
1. **流转卡表 + 条码生成**：每张流转卡有唯一条码
2. **工序扫码上报**：PDA/工控终端扫码确认工序完成
3. **车间工序流转**：前处理 → 染色 → 印花 → 后整理 → 验布（完整车间工序）
4. **工序进度跟踪**：实时显示每张流转卡当前工序
5. **时间戳/操作人/设备 ID 采集**：扫码自动捕获
6. **实时采集参数**：红外测温仪读数 + 在线色差仪 ΔE 值
7. **工艺路线可配置**：主工艺路径 + 嵌套子流程 + 分支链路
8. **每道工序绑定**：特定设备组 + 工时定额 + 水电汽基准消耗值 + 关键控制点
9. **工艺路线设计**：前处理及染色工艺路线设计 + 按卡号制作工序 + 订单色号引用（染色）+ 订单工序查询
10. **工艺路线调整**：用户可根据生产实际给每个产品定义个性化的工艺路线，并可调整和变更

##### 4.3.4 验布打卷模块

**真实业务依据**：fabric-industry-research §11.4 + §3.5
**V15 检查要点**：
1. **十项指标检验**：
   - 色差 ΔE（与标准色卡对比）
   - 纬斜（度数）
   - 缩水率（%）
   - 克重（g/m²）
   - 幅宽（cm）
   - 色牢度（耐水洗/耐摩擦/耐光）
   - 疵点评分（四分制/十分制）
   - 手感
   - 毛效
   - 门幅稳定性
2. **A/B/C 分级联动质检**：
   - A 级：所有指标合格 → 全额入库 + 全额工资
   - B 级：轻微瑕疵 → 折扣入库 + 折扣工资
   - C 级：严重瑕疵 → 降级处理 + 不计工资
3. **打卷入库**：
   - 匹号生成（自动 + 唯一）
   - 匹号唯一校验（`UNIQUE(dye_lot_no, batch_no)`）
   - 卷号管理（每卷独立编号）
4. **与缸号状态机联动**：验布 → 入库状态流转
5. **工艺溯源图谱**：验布"色光偏黄"反向锁定染料批次+助剂顺序+蒸化温度+蒸汽压力曲线
6. **质检报告生成**：每卷布有完整质检报告
7. **疵点位置记录**：疵点在布匹上的具体位置（米数）

##### 4.3.5 产量工资核算模块

**真实业务依据**：fabric-industry-research §11.5 + §4.2.1
**V15 检查要点**：
1. **按缸号计件**：工资按缸号核算（非按订单）
2. **A/B/C 分级金额规则**：
   - A 级：全额工资（100%）
   - B 级：折扣工资（如 80%）
   - C 级：不计工资（0%）
3. **工序单价表**：按工序 + 设备 + 班组 维度配置单价
4. **班组汇总**：按班组 + 工序 + 缸号 汇总工资
5. **与验布分级联动**：A/B/C 级影响工资金额
6. **工艺工价维护**：工时定额 + 工序工艺料糟维护
7. **工资计算 Service**：纯函数复现场景（输入缸号产量 + 分级 + 工序单价 → 输出工资金额）
8. **工资报表生成**：按班组/工序/缸号/月份 维度

##### 4.3.6 能耗管理与成本归集模块

**真实业务依据**：fabric-industry-research §11.6 + §3.2
**V15 检查要点**：
1. **能耗记录表**：按设备 + 工序 + 时间维度采集
2. **水电汽分摊**：
   - 水（m³）
   - 电（kWh）
   - 汽（吨）
3. **分摊规则**：按缸号 + 工序权重 分摊
4. **成本归集联动**：月末分摊到缸号成本
5. **与产量工资报表联动**：单位产量能耗
6. **能耗基准消耗值**：每道工序的水电汽基准
7. **能耗异常告警**：超过基准值 ±20% 告警
8. **能耗报表**：按设备/工序/缸号/月份 维度
9. **缸号承载信息**：水电气实耗（v14 §2.2.1）

##### 4.3.7 缸号状态机与全链路追溯模块

**真实业务依据**：fabric-industry-research §13 + §3.2
**V15 检查要点**：
1. **缸号全生命周期**：投染 → 染色 → 皂洗 → 固色 → 脱水 → 烘干 → 验布 → 入库 → 发货 → 退货
2. **状态流转校验**：
   - 禁止跳跃（如投染 → 入库，跳过染色）
   - 禁止回退违规（如入库 → 染色）
   - 失败状态有恢复路径（Failed + OnHold + 重染/补染/降级）
3. **全链路追溯**：
   - 坯布来源（供应商 + 批次）
   - 染色配方（染料 + 助剂 + 工艺参数）
   - 操作班组
   - 设备机台
   - 水电气实耗
   - 首件确认结果
   - 全流程质检记录
   - 最终落布重量
   - 色差评级
   - 发货记录
   - 退货记录
4. **工艺溯源图谱**：验布"色光偏黄"反向锁定
   - 该缸所用染料批次
   - 助剂添加顺序
   - 蒸化温度记录
   - 当日蒸汽压力波动曲线
5. **状态机闭环**：所有状态有终态（无孤儿状态）
6. **PDA/工控终端扫码确认**：每环节操作扫码
7. **系统自动捕获**：时间戳 + 操作人 + 设备 ID + 实时采集参数

---

### 类五：运行逻辑闭环深化类（7 维度）

> **目的**：v13 规则 15 七大闭环维度的深化，叠加 v14 面料行业特性。每个维度提供详细检查项。

#### 5.1 业务流程闭环

**V15 深化要点**：
1. 输入 → 处理 → 输出 → 反馈 → 输入闭环在面料行业场景保持
2. 染整 10 道工序闭环：配布→精练→漂白→染色→对色→理布→烘干→定型→成品对色→成检 → 反馈工艺优化
3. 化验室打样 5 步闭环：打样通知单→ABCD 多版样→OK 样→复样→染色技术卡 → 反馈配方优化
4. 流转卡扫码上报闭环：扫码 → 工序流转 → 进度跟踪 → 反馈调度优化
5. 订单 → 生产 → 染整 → 入库 → 发货 → 售后 → 报表 全链路无断点
6. 销售订单审批 → 库存预留/锁定（B-P0-1 修复保持）
7. 销售出库 → 库存凭证 + 收入凭证 + 成本凭证（B-P0-2 + F-P0-3 修复保持）
8. 生产订单完成 → 成本归集（B-P0-3 修复保持）

#### 5.2 异常路径闭环

**V15 深化要点**：
1. 所有 `Result<T, E>` 有错误处理路径（禁止 `let _ =` 吞错）
2. 染整工序失败有恢复路径：
   - 重染（重新染色，新缸号）
   - 补染（局部补色，同缸号）
   - 降级（A 级 → B 级 → C 级）
3. 验布不合格有处理路径：
   - 降级（A → B → C）
   - 返工（重新验布/打卷）
   - 报废（C 级以下）
4. 缸号状态机异常有告警 + 死信队列
5. 事件处理失败有重试（指数退避）+ 死信队列 + 告警
6. 数据库事务 commit/rollback 路径完备
7. `Arc<Mutex<T>>` 锁中毒统一降级（`unwrap_or_else(|e| e.into_inner())`）
8. 文件句柄 Drop 闭环

#### 5.3 状态机闭环

**V15 深化要点**：
1. 所有枚举状态有终态（无孤儿状态）
2. **缸号状态机**：投染 → 染色 → 出缸 → 质检 → 入库 → 发货 → 退货（终态）
   - 异常态：Failed + OnHold + 恢复路径（重染/补染/降级）
3. **染整工单状态机**：DRAFT → SCHEDULED → IN_PROGRESS → COMPLETED（终态）
   - 异常态：Failed + OnHold
4. **色卡状态机**（V15 修正）：发放 → 已收到 → 已使用 → 已过期（终态）
   - 不借出/不归还/不遗失/不损坏（业务规则修正）
5. **BorrowStatus**（v13 L-22 修复）：borrowed → returned / lost / damaged / cancelled（终态）
   - **V15 要求**：评估是否需要完全重构为发放模式（详见类十）
6. **MatchStatus**（v13 L-21 修复）：增加 Disputed 终态
7. **DyeBatchStatus**（v13 L-23 修复）：增加 Failed + OnHold
8. **InitTaskStatus**：失败后恢复路径文档化
9. **process_state_machine.rs**：完整闭环（已验证通过）

#### 5.4 资源生命周期闭环

**V15 深化要点**：
1. 5 个后台定时任务有 `CancellationToken`（v13 L-26 修复保持）
2. Kafka 消费/事件监听/库存桥接/OmniAudit spawn 句柄保存到 `EventBusState`，shutdown 时 abort（v13 L-27/L-28/L-29/L-30 修复保持）
3. WebSocket 长连接 recv_task/send_task 句柄保存到 `ConnectionEntry`（v13 L-31 修复保持）
4. 审计日志异步落库改 mpsc channel + 单消费者模式（v13 L-32 修复保持）
5. 数据库事务 commit/rollback 路径完备（v13 L-33 验证通过）
6. `Arc<Mutex<T>>` 锁中毒处理统一降级（v13 L-34 验证通过）
7. 文件句柄 Drop 闭环（v13 L-35 验证通过）
8. **染缸设备资源**（V15 新增）：染缸占用/释放有显式释放路径
9. **PDA/工控终端连接**（V15 新增）：设备连接/断开有资源管理

#### 5.5 配置依赖闭环

**V15 深化要点**：
1. `.env.example` 声明所有环境变量
2. `deploy.sh` + `deploy-latest.sh` + `deploy-backend.sh` 在 config.yaml 生成时注入
3. `backend/config.yaml.example` 包含示例
4. main.rs/app_state.rs fail-fast 校验
5. 配置缺失 fail-fast（非 silent default）
6. **新增面料行业配置**（V15 新增）形成闭环：
   - 染缸设备数（DYEHOUSE_VAT_COUNT）
   - 工序单价基准（PROCESS_UNIT_PRICE_BASE）
   - 能耗分摊规则（ENERGY_ALLOCATION_RULE）
   - A/B/C 分级阈值（QUALITY_GRADE_THRESHOLD_A/B/C）
   - 缸号状态机超时（DYEBATCH_STATUS_TIMEOUT）
7. `AUTH_CHECK_USER_ACTIVE` 启动期显式读取并打印当前值（v13 L-36 修复保持）
8. `AUDIT_RETENTION_DAYS` 通过 AppSettings 读取（v13 L-37 修复保持）
9. `BINGXI_SLOW_QUERY_MS` 统一走 AppSettings（v13 L-38 修复保持）
10. `ELASTICSEARCH_URL` 启动期 warn 提示（v13 L-39 修复保持）
11. `RATE_LIMIT_REDIS_URL` 生产环境若未配置 Redis warn 提示（v13 L-42 修复保持）
12. `.env.example` 包含 `INIT_TOKEN` / `BINGXI_SYSTEMD_DIR` / `BINGXI_ENV_FILE` 显式声明（v13 L-43/L-44 修复保持）
13. `COOKIE_SECRET` / `WEBHOOK_SECRET` / `AUDIT_SECRET_KEY` fail-fast 已完备（v13 L-45 验证通过）

#### 5.6 事件闭环

**V15 深化要点**：
1. 所有业务事件有发布者 + 订阅者（无孤岛事件）
2. 事件处理失败有重试（指数退避）+ 死信队列 + 告警（v13 B-P1-7 修复保持）
3. 事件 payload 唯一键保证幂等性（v13 B-P1-8 修复保持）
4. `BpmProcessFinished` 事件处理覆盖全（含生产订单，v13 B-P1-9 修复保持）
5. **新增面料行业事件**（V15 新增）全部闭环：
   - 染整工序扫码上报事件
   - 缸号状态变更事件
   - 验布分级事件
   - 产量上报事件
   - 能耗采集事件
   - 色卡发放事件（V15 修正：非借出事件）
6. 销售订单状态变更发布事件（B-P1-4 修复保持）
7. 采购订单审批发布事件（B-P1-5 修复保持）
8. 库存盘点完成发布事件（B-P1-2 修复保持）
9. 客户/供应商主数据变更发布事件（B-P1-3 修复保持）

#### 5.7 业财一致性闭环

**V15 深化要点**（面料行业业财一致性）：
1. 销售出库（按缸号）→ 收入凭证 + 成本凭证
2. 采购入库（按缸号）→ 存货凭证 + 应付凭证
3. 染整领料 → 成本凭证（按缸号归集）
4. 生产订单完成 → 成本归集（按缸号）
5. 库存调整（缸号维度）→ 差异凭证
6. 收付款 → 核销凭证
7. 月末能耗分摊 → 成本凭证
8. 产量工资 → 人工成本凭证
9. 凭证科目余额回写（F-P0-1 修复保持）
10. 库存桥接凭证 create + post（F-P0-2 修复保持）
11. AR/AP 核销生成凭证（F-P0-8 修复保持）
12. 期末调整机制（暂估/摊销/预提，F-P2-1）
13. 报表穿透追溯（F-P2-2 修复保持）
14. 销售成本按移动加权平均法计算（F-P2-3）
15. AR/AP 对账单确认后生成凭证（F-P2-4）

---

### 类六：测试体系审计类（7 维度）

> **目的**：综合 v4 测试质量审计 + v8 测试覆盖 + 批次 190 E2E 报告 + 规则 5/6，建立完整测试体系。

#### 6.1 单元测试覆盖率

**V15 检查要点**：
1. 后端 service 模块覆盖率从 38% → 80%+（v4 基线）
2. 核心 service 100% 覆盖：
   - voucher_service
   - inventory_stock_service
   - quotation_service
   - purchase_receipt_service
   - sales_order_service
   - **染整 service**（V15 新增）
   - **化验室打样 service**（V15 新增）
   - **大货处方 service**（V15 新增）
   - **流转卡 service**（V15 新增）
   - **验布打卷 service**（V15 新增）
   - **产量工资 service**（V15 新增）
   - **能耗管理 service**（V15 新增）
   - **缸号状态机 service**（V15 新增）
3. 每个修复点同步补测（规则 1-9 第 5 条）
4. 测试函数名清晰描述场景（中文描述测试目的）
5. 测试覆盖率合理（不过度追求 100%）

#### 6.2 集成测试执行率

**V15 检查要点**：
1. CI 不再 `--lib` 跳过 backend/tests/ 47 个集成测试（v4 维度 12 P0-1 修复保持）
2. 集成测试 100% 执行率
3. 关键路径有真实集成测试：
   - 生产订单全流程
   - 采购收货全流程
   - 销售发货全流程
   - 付款全流程
   - **染整全流程**（V15 新增）
   - **打样全流程**（V15 新增）
   - **大货处方全流程**（V15 新增）
4. 集成测试不共享全局状态（避免测试数据污染）
5. 集成测试断言充分（非仅检查 status code）

#### 6.3 E2E 测试完整通过

**V15 检查要点**（来源批次 190 E2E 报告 + 规则 5）：
1. E2E 工作流（e2e-batch.yml）独立 + 每 30 批次触发
2. `playwright.config.ts` 修复：
   - `reporter: [['html'], ['line']]`（生成 HTML 报告）
   - `webServer` 数组（同时启动前端 + 后端）
   - `timeout: 60min`（job 超时）
3. 移除 `mockBusinessApi`（让业务 API 走真实后端）
4. CI 环境变量 `TEST_USERNAME` / `TEST_PASSWORD` 设置
5. E2E 通过率 ≥ 90%
6. E2E 失败用例按 P0/P1/P2 优先级纳入后续批次
7. E2E 报告保存到 `docs/audits/`
8. 单测试 timeout 60s（非 30s）
9. E2E 不 `continue-on-error: true`

#### 6.4 测试 mock 数据 fixtures 化

**V15 检查要点**（规则 6）：
1. 所有 mock 数据抽取到 `tests/fixtures/` + `e2e/fixtures/`
2. 按业务域组织：
   - `fixtures/sales.ts`
   - `fixtures/user.ts`
   - `fixtures/dyeing.ts`（V15 新增）
   - `fixtures/color_card.ts`（V15 新增）
   - `fixtures/production_order.ts`
3. 工厂函数 `createXxxMock(overrides?)` 优先
4. 禁止内联硬编码 mock JSON
5. mock 数据文件附中文注释说明用途
6. 现有测试同步迁移硬编码 mock 数据到 fixtures

#### 6.5 测试质量（禁止伪测试）

**V15 检查要点**（来源 v4 维度 12 P0）：
1. 删除 80+ 伪测试（玩具模型 + 仅 assert 常量字符串）
2. 测试必须调用真实生产代码
3. `tests/integration/*` 真实验证业务流程（非路由注册）
4. `tests/unit/Login.test.ts` 测真实 Login.vue（非 Mock 组件）
5. `tests/unit/utils.test.ts` import 真实 utils（非重新定义同名函数）
6. 测试断言充分（非仅 status code）
7. 测试不依赖执行顺序

#### 6.6 性能基准测试

**V15 检查要点**：
1. 关键 service 有性能基准测试：
   - 库存核算
   - 凭证生成
   - 染整成本归集（V15 新增）
   - 产量工资计算（V15 新增）
2. `cargo bench` 配置
3. 性能回归 CI 监控
4. 性能基准有合理阈值
5. 性能报告生成

#### 6.7 覆盖率报告生成

**V15 检查要点**：
1. CI 集成 `cargo tarpaulin` 或 `cargo llvm-cov` + codecov
2. 前端 vitest 覆盖率报告
3. 覆盖率门槛：
   - 核心模块 80%+
   - 全项目 60%+
4. 覆盖率趋势监控
5. 覆盖率下降告警

---

### 类七：可维护性与长期治理类（5 维度）

> **目的**：v5 维度 13-15 + 部署运维的持续治理。

#### 7.1 可维护性

**V15 检查要点**（来源 v5 维度 13）：
1. 函数复杂度（>50 行函数清零）
2. 魔法数字常量化（金额精度/超时时间/重试次数）
3. 重复代码抽取为宏/工具函数
4. `Arc::try_unwrap().unwrap()` 改为 `unwrap_or_else`（v5 P0-1 修复保持）
5. 模块循环依赖（utils↔services）解耦
6. 染整 service 拆分（避免 172 行超长函数）
7. 角色权限配置化（非硬编码）
8. CRM 规则持久化（非内存存储）
9. CRUD 模板可抽象为宏

#### 7.2 i18n 与可访问性

**V15 检查要点**（来源 v5 维度 14 / v4 维度 11 P0）：
1. i18n 接入率从 0 → 80%+
2. 200+ 视图硬编码中文逐步迁移
3. 表单 label/aria-label 全覆盖
4. 图片 alt 属性
5. 颜色对比度 WCAG 2.1 AA 达标
6. `useI18n` + `{{ $t('...') }}` 在所有视图使用
7. `i18n/index.ts` 4506 行资源文件实际调用
8. ElMessage 硬编码中文 291 处迁移
9. 后端 AppError 硬编码中文 163 处评估

#### 7.3 部署运维

**V15 检查要点**（来源规则 1-9 第 7 条 / 批次 398/401 / MEMORY.md 部署限制）：
1. **禁止 Docker**（不创建 Dockerfile/docker-compose.yml）
2. systemd 服务文件 `EnvironmentFile` 路径与 deploy 脚本 `CONFIG_DIR` 一致
3. 密钥自动生成（base64 32 熵比 > 0.15）
4. 部署脚本路径一致性检查（`grep -r '/etc/bingxi' deploy/`）
5. `bingxi update` CLI 工具完整
6. 服务名称：bingxi-backend（systemd）
7. 安装目录：`/opt/bingxi-erp`
8. 后端端口：8082
9. 日志目录：`/opt/bingxi-erp/backend/logs`
10. 备份目录：`/opt/bingxi-erp/backups`
11. 环境配置：`/etc/bingxi-erp/.env`
12. 不安装 PostgreSQL 客户端（用远程数据库 39.99.34.194:5432）
13. 不安装 Redis（用远程 Redis 服务器）
14. 只需安装 Nginx、curl
15. 部署方式：CICD 构建 → GitHub Release → 手动部署到生产服务器

#### 7.4 CI/CD pipeline 健康度

**V15 检查要点**（来源规则 1-9 第 1 条 / 规则 14）：
1. 10 项必检全绿：
   - 环境信息
   - 依赖图
   - Rust 构建（`cargo build --release`）
   - Rust Clippy（`cargo clippy --all-targets -- -D warnings`）
   - Rust 格式（`cargo fmt --check`）
   - Rust 单元测试
   - 前端构建
   - 前端格式
   - 前端 ESLint
   - 前端类型检查
   - 前端测试
   - 依赖审计
2. E2E 独立工作流不阻塞主 CI
3. baseline 自动刷新机制有效
4. clippy 严格模式（非 baseline 模式）
5. CI 失败时拉取 logs + annotations 修复
6. GitHub Actions Log 100KB 截断处理（用单 job logs API）

#### 7.5 性能优化与缓存策略

**V15 检查要点**（来源 DB N+1 审计 §4-5）：
1. Redis 缓存层接入 5 个 service：
   - user_service
   - product_service
   - customer_service
   - supplier_service
   - role_service
2. 缓存失效策略明确（写时 invalidate + TTL 300s 兜底）
3. 缓存命中率统计接入 Prometheus
4. 缓存层 graceful degradation（Redis 故障不影响业务）
5. 缓存键空间 `tenant:{tenant_id}:{entity_type}:{entity_id}`（注意：项目已删除租户，键空间需调整）
6. `CacheBackend` trait + Mock（单测不依赖真实 Redis）
7. TTL 调优（用户/产品 300s → 600s）
8. 键空间清理（按前缀批量失效 API）

---

### 类八：法律合规与安全标准类（8 维度）⭐ V15 升级（用户 2026-07-15 第二轮反馈扩展）

> **目的**：规则 11（法律合规标准）+ 规则 12（法律安全标准）独立专项审计。
> **V15 升级**：在原 4 维度（通用法律法规/安全标准/脱敏审计/文档格式）基础上，**新增 4 维度纺织行业法律与财税合规专项**（用户 2026-07-15 第二轮反馈明确要求）。
> **法律依据**：[fabric-industry-research.md](file:///workspace/.monkeycode/docs/research/fabric-industry-research.md) §5 成本核算体系 + 纺织行业真实法律法规调研。

#### 8.1 中国法律法规合规

**V15 检查要点**（规则 11）：
1. 符合《个人信息保护法》
2. 符合《数据安全法》
3. 符合《网络安全法》
4. 用户隐私数据（手机号/身份证/邮箱）存储合规
5. 用户隐私数据传输合规（HTTPS）
6. 用户隐私数据展示合规（脱敏）
7. 日志中禁止记录敏感信息明文
8. 数据导出支持脱敏 + 审计追溯
9. 用户协议/隐私政策在系统中真实接入（非占位）
10. 数据跨境传输合规评估

#### 8.2 法律安全标准

**V15 检查要点**（规则 12）：
1. 所有 API 进行身份认证和权限校验（除明确公开端点）
2. 密码存储使用强哈希（bcrypt/argon2）禁止明文/弱哈希
3. SQL 查询参数化禁止字符串拼接（防 SQL 注入）
4. 所有用户输入验证清理（防 XSS/CSRF）
5. 敏感操作（删除/修改/导出）记录审计日志
6. JWT token 合理过期时间 + refresh token 支持撤销
7. 文件上传校验类型/大小/内容
8. 接口速率限制防暴力破解和 DDoS

#### 8.3 数据脱敏与审计追溯

**V15 检查要点**：
1. 手机号展示脱敏（如 138****8888）
2. 身份证展示脱敏（如 110***********1234）
3. 邮箱展示脱敏（如 a***@example.com）
4. 日志脱敏（密码/token/身份证号不记录明文）
5. 数据导出支持脱敏选项
6. 审计日志不可篡改（签名/哈希链）
7. 审计日志保留期合规（`AUDIT_RETENTION_DAYS` 配置）
8. 敏感操作审计日志完整（删除/修改/导出）

#### 8.4 成品文档格式合规

**V15 检查要点**（规则 3）：
1. 所有数据导出支持 .xlsx 格式（Excel）
   - 线索导出
   - 商机导出
   - 库存导出
   - 报表导出
   - **染整报表导出**（V15 新增）
   - **色卡发放记录导出**（V15 修正）
   - **产量工资报表导出**（V15 新增）
   - **能耗报表导出**（V15 新增）
2. 所有报表/文档生成支持 .docx 格式（Word）
   - 合同
   - 发票
   - 报表
3. 禁止 CSV 作为最终交付格式（CSV 可作为内部调试格式）
4. 禁止 .txt / .rtf / .html 等非标准格式作为成品文档
5. 后端引入 rust_xlsxwriter / docx-rs 统一管理文档生成

#### 8.5 纺织行业法律法规合规（V15 新增）

> **背景**：用户 2026-07-15 第二轮反馈要求补充纺织行业法律合规。纺织行业作为传统制造业，有特定的法律法规要求，必须独立审计。

**V15 检查要点**：
1. **《纺织工业发展规划》合规**：产能淘汰/先进产能登记
2. **《印染行业规范条件》合规**（工信部）：
   - 印染企业准入条件校验（企业规模/工艺装备/环保设施）
   - 禁止使用落后印染设备目录的设备
   - 印染企业水重复利用率 ≥ 40%
3. **《产品质量法》合规**：
   - 面料产品标识完整（厂名/厂址/规格/等级/成分/执行标准号）
   - 面料执行标准登记（GB/T 系列，如 GB/T 406-2018 棉本色布）
   - 假冒伪劣产品预警机制
4. **《消费者权益保护法》合规**：
   - 面料成分真实标示（禁止虚标成分百分比）
   - 面料克重/幅宽真实标示（禁止虚标）
   - 七日无理由退货执行（适用于 B2C 业务）
   - 缺陷产品召回机制
5. **《进出口商品检验法》合规**（出口业务）：
   - 出口面料商检记录
   - 出口面料产地证/普惠制证书生成
   - 出口面料配额管理（适用时）
6. **《反不正当竞争法》合规**：
   - 色卡/报价/合同虚假宣传预警
   - 商业贿赂预警机制（销售/采购环节）
7. **《合同法》/《民法典》合同编合规**：
   - 销售合同模板合规（标的/数量/质量/价款/履行期限/违约责任）
   - 委托加工合同模板（客供坯布/来料加工场景）
   - 合同电子签章真实接入（非占位）

**扫描方法**：
```bash
# 检查面料执行标准字段是否存在
grep -rn "execution_standard\|gb_standard" backend/src/models/
# 检查产品标识完整性
grep -rn "factory_name\|factory_address\|product_spec\|product_grade" backend/src/models/
# 检查电子签章真实接入
grep -rn "electronic_signature\|e_sign\|contract_sign" backend/src/services/
```

#### 8.6 纺织行业财税合规（V15 新增）⭐ 重点

> **背景**：用户 2026-07-15 第二轮反馈明确要求纺织行业财税合规审计。面料行业财税核算复杂（按缸号实际成本法/委托加工物资/出口退税），财税合规是审计重点。
> **业务依据**：[fabric-industry-research.md](file:///workspace/.monkeycode/docs/research/fabric-industry-research.md) §5.4 委托加工物资核算 + §5.5 完整生产流程成本核算。

**V15 检查要点**：
1. **增值税合规**（制造业核心）：
   - 印染加工费进项税抵扣（13% 税率）
   - 染料/助剂采购进项税抵扣
   - 委托加工物资进项税抵扣（加工费 + 运费）
   - 销售面料销项税核算（13% 税率）
   - 进项税转出（非正常损耗按规定转出，正常损耗不转出）
2. **委托加工物资财税合规**（fabric-industry-research §5.4）：
   - 发出胚布：借委托加工物资 / 贷自制半成品-胚布（系统真实生成凭证）
   - 支付染费：借委托加工物资 + 应交税费-进项税额 / 贷银行存款（系统真实生成凭证）
   - 完工入库：借库存商品-成品布 / 贷委托加工物资（合理损耗只影响单位成本不影响总成本）
   - 非正常损耗（丢失/人为损坏）：计入营业外支出/管理费用，禁止进成本
3. **出口退税合规**（出口业务）：
   - 出口面料"免抵退"核算
   - 出口报关单/外汇核销单/增值税发票"单证齐全"校验
   - 出口退税申报表生成
4. **企业所得税合规**：
   - 按缸号实际成本法结转主营业务成本
   - 制造费用按产量分摊
   - 研发费用加计扣除（印染新工艺研发）
5. **印花税合规**：
   - 购销合同印花税（0.3‰）
   - 加工承揽合同印花税（0.5‰）
   - 借款合同印花税（0.05‰）
6. **个人所得税合规**（产量工资场景）：
   - 计件工资个税代扣代缴
   - 生产工人社保公积金缴纳
7. **环保税合规**（印染行业）：
   - 印染废水排放环保税核算
   - 大气污染物排放环保税核算
   - 固体废物排放环保税核算
8. **存货跌价准备**：
   - 面料季节性降价跌价准备
   - 库存呆滞面料（>180 天未动）跌价准备
   - 过期染料/助剂跌价准备

**扫描方法**：
```bash
# 检查委托加工物资凭证生成
grep -rn "委托加工\|consignment_processing\|processing_material" backend/src/services/finance/
# 检查进项税转出
grep -rn "input_tax_transfer\|进项税转出" backend/src/
# 检查出口退税
grep -rn "export_refund\|export_rebate\|免抵退" backend/src/
# 检查环保税
grep -rn "environmental_tax\|环保税" backend/src/
# 检查跌价准备
grep -rn "depreciation_reserve\|跌价准备\|inventory_provision" backend/src/
```

#### 8.7 纺织行业环保合规（V15 新增）

> **背景**：印染行业是环保监管重点行业，废水/废气/固废排放有严格法规要求，必须独立审计。

**V15 检查要点**：
1. **《环境保护法》合规**：
   - 排污许可证登记（印染企业必须持证排污）
   - 排污许可证到期预警
2. **《水污染防治法》合规**：
   - 印染废水排放浓度监控（COD ≤ 80mg/L、氨氮 ≤ 10mg/L、色度 ≤ 50 倍）
   - 废水排放量在线监测
   - 污水处理设施运行记录
3. **《大气污染防治法》合规**：
   - 定型机废气排放监控（VOCs ≤ 60mg/m³）
   - 烘干机废气排放监控
   - 锅炉废气排放监控
4. **《固体废物污染环境防治法》合规**：
   - 印染污泥处置记录（危废/一般固废分类）
   - 染料/助剂废包装处置记录
   - 废料处置联单制度
5. **《环境噪声污染防治法》合规**：
   - 厂界噪声监测（昼间 ≤ 65dB、夜间 ≤ 55dB）
6. **环境影响评价（环评）合规**：
   - 建设项目环评报告存档
   - 环评批复/竣工环保验收存档
7. **排污许可自行监测合规**：
   - 年度监测报告存档
   - 异常排放报告机制

**扫描方法**：
```bash
# 检查排污许可证字段
grep -rn "pollution_permit\|排污许可" backend/src/models/
# 检查废水监测
grep -rn "wastewater\|cod\|ammonia_nitrogen" backend/src/
# 检查能耗管理模块（v14 批次 431 实现）
grep -rn "energy_consumption\|能耗" backend/src/services/
```

#### 8.8 纺织行业劳动合规（V15 新增）

> **背景**：纺织行业劳动密集型，计件工资/工时/职业健康有特殊合规要求。

**V15 检查要点**：
1. **《劳动法》/《劳动合同法》合规**：
   - 劳动合同电子化管理
   - 试用期/合同期限合规
   - 试用期工资 ≥ 转正工资 80%
2. **工时与加班合规**：
   - 月加班时间 ≤ 36 小时
   - 加班费计算（平时 1.5 倍/周末 2 倍/法定节假日 3 倍）
   - 综合计时工时制审批（纺织行业常见）
3. **计件工资合规**：
   - 计件单价不得低于当地最低工资标准折算
   - 计件产量与质检结果挂钩（不合格产量不结算）
   - 计件工资台账保留 2 年以上
4. **社保公积金合规**：
   - 五险一金全员缴纳
   - 缴费基数合规（禁止按最低基数缴纳）
5. **职业健康合规**：
   - 印染车间职业危害因素检测（苯/甲醛/噪声/粉尘）
   - 职业健康体检档案（上岗前/在岗期间/离岗时）
   - 防护用品配备记录
6. **女职工与未成年工保护合规**：
   - 女职工孕期/产期/哺乳期保护
   - 禁止使用未成年工（<16 周岁）
7. **安全生产法合规**：
   - 染缸/定型机/烘干机操作证管理
   - 危险化学品（染料/助剂）MSDS 安全数据表
   - 安全生产事故报告机制

**扫描方法**：
```bash
# 检查劳动合同字段
grep -rn "labor_contract\|劳动合同" backend/src/models/
# 检查计件工资（v14 批次 430 实现）
grep -rn "piecework\|计件" backend/src/services/
# 检查职业健康
grep -rn "occupational_health\|职业健康" backend/src/
# 检查安全生产
grep -rn "safety_production\|安全生产" backend/src/
```

---

### 类九：批次节奏与记忆治理类（2 维度）

> **目的**：规则 5/10/13/14 持续监控。

#### 9.1 批次节奏与 E2E 监控

**V15 检查要点**（规则 5）：
1. 每 30 批次（如 450/480/510）触发 e2e-batch.yml workflow_dispatch
2. 20/28/29 节奏监控 E2E run 状态
3. 未完成跳过下一周期（skip_reason 参数 + e2e-skipped job）
4. E2E 报告保存到 docs/audits/
5. E2E 失败按 P0/P1/P2 优先级纳入后续批次
6. 禁止死等 E2E 完成（监控点之间正常推进修复批次）

#### 9.2 记忆整理与归档

**V15 检查要点**（规则 10 / 规则 13）：
1. 每 15 批次（如 435/450/465）整理 .monkeycode/ 所有记忆文件
2. **实时归档**（规则 10 二次修正）：每批 CI 合并后立即归档到 doto-su.md
3. **文件分工明确**：
   - `MEMORY.md` = 项目规则（禁止任务相关内容）
   - `doto.md` = 未完成任务（禁止已完成批次详细表格）
   - `doto-su.md` = 已完成任务详细记录
   - `CHANGELOG.md` = 一句话总结（禁止详细任务内容）
   - `audit_assignment.md` = 审计任务分配和复审规则（禁止审计结果详情）
   - `bug.md` = 漏洞登记（修复后删除条目，保留空文件）
4. 禁止跨批堆积
5. 历史归档到 `docs/archives/`（按日期保留）

---

### 类十：色卡发放业务规则修正专项（5 维度）⭐ V15 新增

> **背景**：用户 2026-07-15 明确"色卡只发放给客户，不借出"。现有实现是"借出-归还-遗失-损坏"模式（[color_card_borrow_service.rs](file:///workspace/backend/src/services/color_card_borrow_service.rs)），必须重构为"发放"模式。
> **影响范围**：[color_card_borrow_service.rs](file:///workspace/backend/src/services/color_card_borrow_service.rs) + [color_card_crud_service.rs](file:///workspace/backend/src/services/color_card_crud_service.rs) + [color_card_borrow_record.rs](file:///workspace/backend/src/models/color_card_borrow_record.rs) + [color_card_borrow_dto.rs](file:///workspace/backend/src/models/color_card_borrow_dto.rs) + [handlers/color_card/borrow.rs](file:///workspace/backend/src/handlers/color_card/borrow.rs) + [routes/color_card.rs](file:///workspace/backend/src/routes/color_card.rs) + DB 迁移 + 前端色卡管理界面

#### 10.1 色卡业务模式重构（详细代码级实现规范）

> **强制要求**：本节提供完整的代码级实现规范，包括 SQL 迁移脚本、SeaORM Model、DTO、Service、Handler、路由、前端 6 层重构的完整代码骨架。修复时必须严格按照本节代码骨架实现，禁止偏离业务语义。

##### 10.1.1 业务语义修正

- **旧模式**（禁止）：色卡借出 → 归还 / 遗失 / 损坏（借还模式，色卡会收回）
- **新模式**（强制）：色卡发放 → 已收到 → 已使用 → 已过期（发放模式，一次性发放给客户，**不收回**）
- **核心业务规则**（用户 2026-07-15 反复强调）：
  1. 色卡只发放给客户，**禁止借出语义**
  2. 发放后色卡归客户所有，**不收回**
  3. 没有"归还/遗失/损坏"概念，只有"已收到/已使用/已过期"
  4. 取消发放仅在"已发放未收到"状态可操作，且恢复库存
  5. 客户专属色卡库：每个客户有独立色卡档案，历史可追溯

##### 10.1.2 状态机重构（完整定义）

**旧状态机**（必须删除）：
```
Borrowed → Returned / Lost / Damaged / Cancelled
```

**新状态机**（必须实现）：
```
                    ┌─────────────┐
                    │   Issued    │ ← 发放（初始状态）
                    │  （已发放）  │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
              ▼            ▼            ▼
     ┌────────────┐  ┌──────────┐  ┌──────────┐
     │  Cancelled │  │ Received │  │ Expired  │
     │  （已取消） │  │（已收到）│  │（已过期）│
     │  终态       │  └────┬─────┘  │  终态    │
     └────────────┘       │        └──────────┘
                          │
                          ▼
                    ┌──────────┐
                    │   Used   │
                    │（已使用）│
                    │  终态    │
                    └──────────┘
```

**状态流转规则**：
| 当前状态 | 可转换到 | 触发条件 | 操作人 |
|----------|----------|----------|--------|
| Issued | Received | 客户确认收到 | 客户/销售 |
| Issued | Cancelled | 取消发放（仅未收到可取消） | 销售 |
| Issued | Expired | 超过有效期未收到（定时任务） | 系统 |
| Received | Used | 客户使用色卡 | 客户 |
| Received | Expired | 超过有效期未使用（定时任务） | 系统 |
| Cancelled | （终态） | — | — |
| Used | （终态） | — | — |
| Expired | （终态） | — | — |

**终态**：`Cancelled` / `Used` / `Expired`（不可再转换）

##### 10.1.3 数据模型重构（完整 SQL 迁移脚本）

**新表 `color_card_issue_record`**（替代 `color_card_borrow_record`）：

```sql
-- 迁移文件：backend/migration/xxxx_create_color_card_issue_record_table.sql
-- V15 类十 10.1.3：色卡发放业务模式重构（用户 2026-07-15 明确"只发放不借出"）

CREATE TABLE color_card_issue_record (
    id BIGSERIAL PRIMARY KEY,

    -- 关联色卡（必填）
    color_card_id BIGINT NOT NULL REFERENCES color_card(id) ON DELETE RESTRICT,

    -- 关联客户（必填，发放对象）
    customer_id BIGINT NOT NULL REFERENCES customer(id) ON DELETE RESTRICT,

    -- 关联色号（可选，按色号发放场景）
    color_id BIGINT REFERENCES color(id) ON DELETE SET NULL,

    -- 关联缸号（可选，指定缸号色卡场景，v14 批次 419 T-P0-3 已加 dye_lot_no）
    dye_lot_no VARCHAR(64),

    -- 关联销售订单（可选，发放时关联订单场景）
    sales_order_id BIGINT REFERENCES sales_order(id) ON DELETE SET NULL,

    -- 发放操作人（必填，从 AuthContext.user_id 注入）
    issued_by BIGINT NOT NULL REFERENCES user(id) ON DELETE RESTRICT,

    -- 发放时间（必填）
    issued_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    -- 客户收到时间（可选，客户确认收到时填充）
    received_at TIMESTAMPTZ,

    -- 客户使用时间（可选，客户标记已使用时填充）
    used_at TIMESTAMPTZ,

    -- 过期时间（可选，定时任务自动过期时填充）
    expired_at TIMESTAMPTZ,

    -- 预计有效期（必填，默认发放后 90 天，可配置）
    expected_valid_until TIMESTAMPTZ NOT NULL,

    -- 发放数量（必填，默认 1，支持一次发放多张）
    quantity INT NOT NULL DEFAULT 1 CHECK (quantity > 0),

    -- 发放状态（必填，状态机字段）
    -- 枚举值：issued / received / used / expired / cancelled
    issue_status VARCHAR(20) NOT NULL DEFAULT 'issued',

    -- 发放用途（可选，如"打样确认/大货前对色/客户存档"）
    purpose VARCHAR(200),

    -- 物流单号（可选，邮寄给客户时的物流单号）
    tracking_no VARCHAR(100),

    -- 客户反馈（可选，客户收到后的反馈）
    customer_feedback TEXT,

    -- 取消原因（可选，取消发放时必填）
    cancel_reason VARCHAR(200),

    -- 取消操作人（可选，取消发放时填充）
    cancelled_by BIGINT REFERENCES user(id) ON DELETE SET NULL,

    -- 取消时间（可选，取消发放时填充）
    cancelled_at TIMESTAMPTZ,

    -- 审计字段
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by BIGINT REFERENCES user(id) ON DELETE SET NULL,
    updated_by BIGINT REFERENCES user(id) ON DELETE SET NULL
);

-- 索引（按业务查询模式设计）
CREATE INDEX idx_issue_record_color_card_id ON color_card_issue_record(color_card_id);
CREATE INDEX idx_issue_record_customer_id ON color_card_issue_record(customer_id);
CREATE INDEX idx_issue_record_status ON color_card_issue_record(issue_status);
CREATE INDEX idx_issue_record_issued_at ON color_card_issue_record(issued_at);
CREATE INDEX idx_issue_record_issued_by ON color_card_issue_record(issued_by);
CREATE INDEX idx_issue_record_sales_order_id ON color_card_issue_record(sales_order_id);
CREATE INDEX idx_issue_record_dye_lot_no ON color_card_issue_record(dye_lot_no);
CREATE INDEX idx_issue_record_expected_valid_until ON color_card_issue_record(expected_valid_until) WHERE issue_status IN ('issued', 'received');
CREATE UNIQUE INDEX uq_issue_record_card_customer_active ON color_card_issue_record(color_card_id, customer_id) WHERE issue_status NOT IN ('cancelled');

-- 外键约束（关联现有表）
ALTER TABLE color_card_issue_record
    ADD CONSTRAINT fk_issue_record_color_card
        FOREIGN KEY (color_card_id) REFERENCES color_card(id) ON DELETE RESTRICT,
    ADD CONSTRAINT fk_issue_record_customer
        FOREIGN KEY (customer_id) REFERENCES customer(id) ON DELETE RESTRICT;
```

**旧表 `color_card_borrow_record` 处理**：
- **禁止直接 DROP**（历史数据需迁移，详见 10.7 节）
- 重命名为 `color_card_borrow_record_legacy`（备份）
- 数据迁移到 `color_card_issue_record` 后保留 legacy 表供审计追溯

##### 10.1.4 SeaORM Model 完整代码骨架

**新文件**：`backend/src/models/color_card_issue_record.rs`

```rust
//! 色卡发放记录模型（V15 类十 10.1.4）
//! 替代旧 color_card_borrow_record（借还模式），改为发放模式
//! 业务规则：色卡只发放给客户，不借出，不收回

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// 色卡发放记录
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "color_card_issue_record")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    /// 关联色卡 ID
    pub color_card_id: i64,
    /// 客户 ID（发放对象）
    pub customer_id: i64,
    /// 色号 ID（可选，按色号发放场景）
    pub color_id: Option<i64>,
    /// 缸号（可选，指定缸号色卡场景）
    pub dye_lot_no: Option<String>,
    /// 销售订单 ID（可选，发放时关联订单场景）
    pub sales_order_id: Option<i64>,
    /// 发放操作人
    pub issued_by: i32,
    /// 发放时间
    pub issued_at: DateTime<Utc>,
    /// 客户收到时间
    pub received_at: Option<DateTime<Utc>>,
    /// 客户使用时间
    pub used_at: Option<DateTime<Utc>>,
    /// 过期时间
    pub expired_at: Option<DateTime<Utc>>,
    /// 预计有效期（默认发放后 90 天）
    pub expected_valid_until: DateTime<Utc>,
    /// 发放数量
    pub quantity: i32,
    /// 发放状态（issued/received/used/expired/cancelled）
    pub issue_status: String,
    /// 发放用途
    pub purpose: Option<String>,
    /// 物流单号
    pub tracking_no: Option<String>,
    /// 客户反馈
    pub customer_feedback: Option<String>,
    /// 取消原因
    pub cancel_reason: Option<String>,
    /// 取消操作人
    pub cancelled_by: Option<i32>,
    /// 取消时间
    pub cancelled_at: Option<DateTime<Utc>>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
    /// 更新时间
    pub updated_at: DateTime<Utc>,
    /// 创建人
    pub created_by: Option<i32>,
    /// 更新人
    pub updated_by: Option<i32>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::color_card::Entity",
        from = "Column::ColorCardId",
        to = "super::color_card::Column::Id"
    )]
    ColorCard,
    #[sea_orm(
        belongs_to = "super::customer::Entity",
        from = "Column::CustomerId",
        to = "super::customer::Column::Id"
    )]
    Customer,
}

impl Related<super::color_card::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ColorCard.def()
    }
}

impl Related<super::customer::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Customer.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
```

##### 10.1.5 DTO 完整代码骨架

**新文件**：`backend/src/models/color_card_issue_dto.rs`

```rust
//! 色卡发放 DTO（V15 类十 10.1.5）
//! 替代旧 color_card_borrow_dto（借还模式）

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// 创建发放记录请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CreateIssueRecordDto {
    #[validate(range(min = 1))]
    pub color_card_id: i64,
    #[validate(range(min = 1))]
    pub customer_id: i64,
    pub color_id: Option<i64>,
    pub dye_lot_no: Option<String>,
    pub sales_order_id: Option<i64>,
    /// 发放数量（默认 1）
    #[validate(range(min = 1, max = 100))]
    pub quantity: Option<i32>,
    /// 发放用途
    #[validate(length(max = 200))]
    pub purpose: Option<String>,
    /// 物流单号
    #[validate(length(max = 100))]
    pub tracking_no: Option<String>,
    /// 预计有效期天数（默认 90，可配置）
    #[validate(range(min = 1, max = 365))]
    pub valid_days: Option<i32>,
}

/// 标记已收到请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MarkReceivedDto {
    #[validate(length(max = 1000))]
    pub customer_feedback: Option<String>,
}

/// 标记已使用请求
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct MarkUsedDto {
    #[validate(length(max = 1000))]
    pub customer_feedback: Option<String>,
}

/// 取消发放请求（仅 Issued 状态可取消）
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct CancelIssueDto {
    #[validate(length(min = 1, max = 200))]
    pub cancel_reason: String,
}

/// 发放记录列表查询
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ListIssueRecordsQuery {
    pub color_card_id: Option<i64>,
    pub customer_id: Option<i64>,
    pub issue_status: Option<String>,
    pub dye_lot_no: Option<String>,
    pub sales_order_id: Option<i64>,
    /// 发放时间起
    pub from_date: Option<DateTime<Utc>>,
    /// 发放时间止
    pub to_date: Option<DateTime<Utc>>,
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<u64>,
    #[validate(range(min = 1, max = 100))]
    pub page_size: Option<u64>,
}

/// 发放记录响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueRecordResponse {
    pub id: i64,
    pub color_card_id: i64,
    pub color_card_name: Option<String>,
    pub customer_id: i64,
    pub customer_name: Option<String>,
    pub color_id: Option<i64>,
    pub dye_lot_no: Option<String>,
    pub sales_order_id: Option<i64>,
    pub issued_by: i32,
    pub issued_by_name: Option<String>,
    pub issued_at: DateTime<Utc>,
    pub received_at: Option<DateTime<Utc>>,
    pub used_at: Option<DateTime<Utc>>,
    pub expired_at: Option<DateTime<Utc>>,
    pub expected_valid_until: DateTime<Utc>,
    pub quantity: i32,
    pub issue_status: String,
    pub purpose: Option<String>,
    pub tracking_no: Option<String>,
    pub customer_feedback: Option<String>,
    pub cancel_reason: Option<String>,
    pub cancelled_by: Option<i32>,
    pub cancelled_at: Option<DateTime<Utc>>,
}
```

##### 10.1.6 Service 完整方法签名与实现要点

**新文件**：`backend/src/services/color_card_issue_service.rs`

```rust
//! 色卡发放服务（V15 类十 10.1.6）
//! 替代旧 color_card_borrow_service（借还模式）
//! 业务规则：色卡只发放给客户，不借出，不收回，无归还/遗失/损坏概念

use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::str::FromStr;
use std::sync::Arc;
use thiserror::Error;

use crate::models::color_card::{self, Entity as ColorCardEntity};
use crate::models::color_card_issue_dto::{
    CancelIssueDto, CreateIssueRecordDto, ListIssueRecordsQuery, MarkReceivedDto, MarkUsedDto,
};
use crate::models::color_card_issue_record::{
    self, ActiveModel as IssueActive, Entity as IssueEntity,
};
use crate::utils::app_state::AppState;

/// 业务错误
#[derive(Debug, Error)]
pub enum IssueError {
    #[error("色卡不存在")]
    ColorCardNotFound,
    #[error("客户不存在或已停用")]
    CustomerNotFound,
    #[error("发放记录不存在")]
    RecordNotFound,
    #[error("色卡当前状态不允许此操作: {0}")]
    InvalidState(String),
    #[error("色卡库存不足（当前库存: {0}）")]
    InsufficientStock(i32),
    #[error("参数校验失败: {0}")]
    Validation(String),
    #[error("数据库错误: {0}")]
    Database(#[from] sea_orm::DbErr),
}

/// 发放状态（V15 新状态机）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueStatus {
    /// 已发放（初始状态）
    Issued,
    /// 已收到（客户确认收到）
    Received,
    /// 已使用（客户使用色卡）
    Used,
    /// 已过期（超过有效期，终态）
    Expired,
    /// 已取消（发放后取消，终态）
    Cancelled,
}

impl IssueStatus {
    /// 序列化为字符串（持久化到数据库的稳定字符串）
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Issued => "issued",
            Self::Received => "received",
            Self::Used => "used",
            Self::Expired => "expired",
            Self::Cancelled => "cancelled",
        }
    }

    /// 是否为终态
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Used | Self::Expired | Self::Cancelled)
    }

    /// 是否可取消（仅 Issued 状态可取消）
    pub fn can_cancel(&self) -> bool {
        matches!(self, Self::Issued)
    }

    /// 是否可标记已收到（仅 Issued 状态可标记）
    pub fn can_receive(&self) -> bool {
        matches!(self, Self::Issued)
    }

    /// 是否可标记已使用（仅 Received 状态可标记）
    pub fn can_use(&self) -> bool {
        matches!(self, Self::Received)
    }
}

/// IssueStatus 解析错误
#[derive(Debug, Clone)]
pub struct IssueStatusParseError(pub String);

impl std::fmt::Display for IssueStatusParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "IssueStatus 解析失败: {}", self.0)
    }
}

impl std::error::Error for IssueStatusParseError {}

impl FromStr for IssueStatus {
    type Err = IssueStatusParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "issued" => Ok(Self::Issued),
            "received" => Ok(Self::Received),
            "used" => Ok(Self::Used),
            "expired" => Ok(Self::Expired),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(IssueStatusParseError(s.to_string())),
        }
    }
}

/// 默认有效期天数
const DEFAULT_VALID_DAYS: i32 = 90;

/// 色卡发放服务
pub struct ColorCardIssueService {
    db: Arc<DatabaseConnection>,
}

impl ColorCardIssueService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub fn from_state(state: &AppState) -> Self {
        Self::new(state.db.clone())
    }

    /// 发放色卡（核心方法）
    ///
    /// 业务规则：
    /// 1. 色卡必须存在且未归档（status = active）
    /// 2. 色卡库存必须 >= 发放数量
    /// 3. 客户必须存在且 active
    /// 4. 同一客户同一色卡不能有未结束的发放记录（issued/received 状态）
    /// 5. 发放时扣减色卡库存
    /// 6. 全程事务化 + lock_exclusive 串行化并发
    pub async fn issue_card(
        &self,
        dto: CreateIssueRecordDto,
        user_id: i32,
    ) -> Result<color_card_issue_record::Model, IssueError> {
        let txn = self.db.begin().await?;

        // 1. 查询色卡（加 lock_exclusive 串行化并发发放）
        let card = ColorCardEntity::find_by_id(dto.color_card_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(IssueError::ColorCardNotFound)?;

        // 2. 校验色卡状态（必须 active，非 archived）
        if card.status != "active" {
            return Err(IssueError::InvalidState(format!(
                "色卡状态为 {}，仅 active 状态可发放",
                card.status
            )));
        }

        // 3. 校验色卡库存
        let quantity = dto.quantity.unwrap_or(1);
        let current_stock = card.total_colors; // 假设 total_colors 字段表示库存
        if current_stock < quantity {
            return Err(IssueError::InsufficientStock(current_stock));
        }

        // 4. 校验客户存在且 active（调用 customer service）
        // let customer = CustomerEntity::find_by_id(dto.customer_id)
        //     .one(&txn).await?
        //     .ok_or(IssueError::CustomerNotFound)?;
        // if !customer.is_active { return Err(IssueError::CustomerNotFound); }

        // 5. 校验同一客户同一色卡不能有未结束的发放记录
        let existing_active = IssueEntity::find()
            .filter(color_card_issue_record::Column::ColorCardId.eq(dto.color_card_id))
            .filter(color_card_issue_record::Column::CustomerId.eq(dto.customer_id))
            .filter(color_card_issue_record::Column::IssueStatus.is_in(["issued", "received"]))
            .one(&txn)
            .await?;
        if existing_active.is_some() {
            return Err(IssueError::InvalidState(
                "该客户已有未结束的色卡发放记录（已发放或已收到状态），请先结束现有记录".to_string(),
            ));
        }

        // 6. 计算预计有效期
        let valid_days = dto.valid_days.unwrap_or(DEFAULT_VALID_DAYS);
        let now = Utc::now();
        let expected_valid_until = now + Duration::days(valid_days as i64);

        // 7. 创建发放记录
        let active = IssueActive {
            id: Default::default(),
            color_card_id: Set(dto.color_card_id),
            customer_id: Set(dto.customer_id),
            color_id: Set(dto.color_id),
            dye_lot_no: Set(dto.dye_lot_no),
            sales_order_id: Set(dto.sales_order_id),
            issued_by: Set(user_id),
            issued_at: Set(now),
            received_at: Set(None),
            used_at: Set(None),
            expired_at: Set(None),
            expected_valid_until: Set(expected_valid_until),
            quantity: Set(quantity),
            issue_status: Set(IssueStatus::Issued.as_str().to_string()),
            purpose: Set(dto.purpose),
            tracking_no: Set(dto.tracking_no),
            customer_feedback: Set(None),
            cancel_reason: Set(None),
            cancelled_by: Set(None),
            cancelled_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            created_by: Set(Some(user_id)),
            updated_by: Set(Some(user_id)),
        };
        let result = active.insert(&txn).await?;

        // 8. 扣减色卡库存（事务内）
        // let mut card_active: color_card::ActiveModel = card.into();
        // card_active.total_colors = Set(current_stock - quantity);
        // card_active.updated_at = Set(now);
        // card_active.update(&txn).await?;

        // 9. 写审计日志（调用 audit_log_service）
        // AuditLogService::create_with_audit(&txn, "color_card_issue", ...).await?;

        txn.commit().await?;
        Ok(result)
    }

    /// 标记客户已收到（仅 Issued 状态可标记）
    pub async fn mark_received(
        &self,
        record_id: i64,
        dto: MarkReceivedDto,
        user_id: i32,
    ) -> Result<color_card_issue_record::Model, IssueError> {
        let txn = self.db.begin().await?;

        let existing = IssueEntity::find_by_id(record_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(IssueError::RecordNotFound)?;

        let current = IssueStatus::from_str(&existing.issue_status)
            .map_err(|_| IssueError::InvalidState(format!("未知状态: {}", existing.issue_status)))?;

        if !current.can_receive() {
            return Err(IssueError::InvalidState(format!(
                "当前状态 {} 不允许标记已收到（仅 issued 状态可标记）",
                existing.issue_status
            )));
        }

        let mut active: IssueActive = existing.into();
        active.issue_status = Set(IssueStatus::Received.as_str().to_string());
        active.received_at = Set(Some(Utc::now()));
        if let Some(fb) = dto.customer_feedback {
            active.customer_feedback = Set(Some(fb));
        }
        active.updated_at = Set(Utc::now());
        active.updated_by = Set(Some(user_id));

        let result = active.update(&txn).await?;
        txn.commit().await?;
        Ok(result)
    }

    /// 标记客户已使用（仅 Received 状态可标记）
    pub async fn mark_used(
        &self,
        record_id: i64,
        dto: MarkUsedDto,
        user_id: i32,
    ) -> Result<color_card_issue_record::Model, IssueError> {
        let txn = self.db.begin().await?;

        let existing = IssueEntity::find_by_id(record_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(IssueError::RecordNotFound)?;

        let current = IssueStatus::from_str(&existing.issue_status)
            .map_err(|_| IssueError::InvalidState(format!("未知状态: {}", existing.issue_status)))?;

        if !current.can_use() {
            return Err(IssueError::InvalidState(format!(
                "当前状态 {} 不允许标记已使用（仅 received 状态可标记）",
                existing.issue_status
            )));
        }

        let mut active: IssueActive = existing.into();
        active.issue_status = Set(IssueStatus::Used.as_str().to_string());
        active.used_at = Set(Some(Utc::now()));
        if let Some(fb) = dto.customer_feedback {
            active.customer_feedback = Set(Some(fb));
        }
        active.updated_at = Set(Utc::now());
        active.updated_by = Set(Some(user_id));

        let result = active.update(&txn).await?;
        txn.commit().await?;
        Ok(result)
    }

    /// 标记已过期（定时任务调用，Issued/Received 状态可过期）
    pub async fn mark_expired(
        &self,
        record_id: i64,
    ) -> Result<color_card_issue_record::Model, IssueError> {
        let txn = self.db.begin().await?;

        let existing = IssueEntity::find_by_id(record_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(IssueError::RecordNotFound)?;

        let current = IssueStatus::from_str(&existing.issue_status)
            .map_err(|_| IssueError::InvalidState(format!("未知状态: {}", existing.issue_status)))?;

        if current.is_terminal() {
            return Err(IssueError::InvalidState(format!(
                "当前状态 {} 为终态，不可标记过期",
                existing.issue_status
            )));
        }

        // 校验是否超过有效期
        if Utc::now() <= existing.expected_valid_until {
            return Err(IssueError::InvalidState("未超过有效期，不可标记过期".to_string()));
        }

        let mut active: IssueActive = existing.into();
        active.issue_status = Set(IssueStatus::Expired.as_str().to_string());
        active.expired_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());

        let result = active.update(&txn).await?;
        txn.commit().await?;
        Ok(result)
    }

    /// 取消发放（仅 Issued 状态可取消，恢复库存）
    pub async fn cancel_issue(
        &self,
        record_id: i64,
        dto: CancelIssueDto,
        user_id: i32,
    ) -> Result<color_card_issue_record::Model, IssueError> {
        let txn = self.db.begin().await?;

        let existing = IssueEntity::find_by_id(record_id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or(IssueError::RecordNotFound)?;

        let current = IssueStatus::from_str(&existing.issue_status)
            .map_err(|_| IssueError::InvalidState(format!("未知状态: {}", existing.issue_status)))?;

        if !current.can_cancel() {
            return Err(IssueError::InvalidState(format!(
                "当前状态 {} 不允许取消（仅 issued 状态可取消，已收到/已使用/已过期不可取消）",
                existing.issue_status
            )));
        }

        let mut active: IssueActive = existing.into();
        active.issue_status = Set(IssueStatus::Cancelled.as_str().to_string());
        active.cancel_reason = Set(Some(dto.cancel_reason));
        active.cancelled_by = Set(Some(user_id));
        active.cancelled_at = Set(Some(Utc::now()));
        active.updated_at = Set(Utc::now());
        active.updated_by = Set(Some(user_id));

        let result = active.update(&txn).await?;

        // 恢复色卡库存（事务内）
        // let card = ColorCardEntity::find_by_id(existing.color_card_id)
        //     .lock_exclusive().one(&txn).await?
        //     .ok_or(IssueError::ColorCardNotFound)?;
        // let mut card_active: color_card::ActiveModel = card.into();
        // card_active.total_colors = Set(card.total_colors + existing.quantity);
        // card_active.updated_at = Set(Utc::now());
        // card_active.update(&txn).await?;

        txn.commit().await?;
        Ok(result)
    }

    /// 按 ID 查询
    pub async fn get_by_id(
        &self,
        record_id: i64,
    ) -> Result<color_card_issue_record::Model, IssueError> {
        IssueEntity::find_by_id(record_id)
            .one(&*self.db)
            .await?
            .ok_or(IssueError::RecordNotFound)
    }

    /// 列表查询（分页 + 多条件）
    pub async fn list_records(
        &self,
        query: ListIssueRecordsQuery,
    ) -> Result<(Vec<color_card_issue_record::Model>, u64), IssueError> {
        let find = IssueEntity::find();
        let mut cond = Condition::all();

        if let Some(card_id) = query.color_card_id {
            cond = cond.add(color_card_issue_record::Column::ColorCardId.eq(card_id));
        }
        if let Some(cust_id) = query.customer_id {
            cond = cond.add(color_card_issue_record::Column::CustomerId.eq(cust_id));
        }
        if let Some(status) = query.issue_status {
            cond = cond.add(color_card_issue_record::Column::IssueStatus.eq(status));
        }
        if let Some(dye_lot) = query.dye_lot_no {
            cond = cond.add(color_card_issue_record::Column::DyeLotNo.eq(dye_lot));
        }
        if let Some(order_id) = query.sales_order_id {
            cond = cond.add(color_card_issue_record::Column::SalesOrderId.eq(order_id));
        }
        if let Some(from) = query.from_date {
            cond = cond.add(color_card_issue_record::Column::IssuedAt.gte(from));
        }
        if let Some(to) = query.to_date {
            cond = cond.add(color_card_issue_record::Column::IssuedAt.lte(to));
        }

        let page = query.page.unwrap_or(1);
        let page_size = query.page_size.unwrap_or(20).clamp(1, 100);

        let paginator = find
            .filter(cond)
            .order_by_desc(color_card_issue_record::Column::IssuedAt)
            .paginate(&*self.db, page_size);

        let total = paginator.num_items().await?;
        let items = paginator.fetch_page(page.clamp(1, 1000).saturating_sub(1)).await?;
        Ok((items, total))
    }

    /// 批量过期检查（定时任务调用，扫描所有已超期但未过期的记录）
    pub async fn batch_check_expired(&self) -> Result<u64, IssueError> {
        let now = Utc::now();
        let expired_records = IssueEntity::find()
            .filter(color_card_issue_record::Column::IssueStatus.is_in(["issued", "received"]))
            .filter(color_card_issue_record::Column::ExpectedValidUntil.lt(now))
            .all(&*self.db)
            .await?;

        let mut count = 0u64;
        for record in expired_records {
            if let Err(e) = self.mark_expired(record.id).await {
                tracing::warn!(error = %e, record_id = record.id, "色卡发放记录自动过期失败");
            } else {
                count += 1;
            }
        }
        Ok(count)
    }
}
```

##### 10.1.7 Handler 与路由完整规范

**新文件**：`backend/src/handlers/color_card/issue.rs`

```rust
//! 色卡发放 Handler（V15 类十 10.1.7）
//! 替代旧 borrow.rs（借还模式）

use axum::{
    extract::{Path, Query, State},
    Json,
};
use crate::utils::app_state::AppState;
use crate::utils::auth::AuthContext;
use crate::models::color_card_issue_dto::*;
use crate::services::color_card_issue_service::{ColorCardIssueService, IssueError};

/// 发放色卡 POST /color-cards/:id/issue
pub async fn issue_card(
    State(state): State<AppState>,
    Path(color_card_id): Path<i64>,
    auth: AuthContext,
    Json(mut dto): Json<CreateIssueRecordDto>,
) -> Result<Json<IssueRecordResponse>, AppError> {
    dto.color_card_id = color_card_id; // 路径参数覆盖
    dto.validate()?;
    let service = ColorCardIssueService::from_state(&state);
    let record = service.issue_card(dto, auth.user_id).await?;
    Ok(Json(record.into()))
}

/// 标记已收到 POST /color-cards/issue/:id/receive
pub async fn mark_received(
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
    auth: AuthContext,
    Json(dto): Json<MarkReceivedDto>,
) -> Result<Json<IssueRecordResponse>, AppError> {
    dto.validate()?;
    let service = ColorCardIssueService::from_state(&state);
    let record = service.mark_received(record_id, dto, auth.user_id).await?;
    Ok(Json(record.into()))
}

/// 标记已使用 POST /color-cards/issue/:id/use
pub async fn mark_used(
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
    auth: AuthContext,
    Json(dto): Json<MarkUsedDto>,
) -> Result<Json<IssueRecordResponse>, AppError> {
    dto.validate()?;
    let service = ColorCardIssueService::from_state(&state);
    let record = service.mark_used(record_id, dto, auth.user_id).await?;
    Ok(Json(record.into()))
}

/// 取消发放 POST /color-cards/issue/:id/cancel
pub async fn cancel_issue(
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
    auth: AuthContext,
    Json(dto): Json<CancelIssueDto>,
) -> Result<Json<IssueRecordResponse>, AppError> {
    dto.validate()?;
    let service = ColorCardIssueService::from_state(&state);
    let record = service.cancel_issue(record_id, dto, auth.user_id).await?;
    Ok(Json(record.into()))
}

/// 查询发放记录列表 GET /color-cards/issue
pub async fn list_issue_records(
    State(state): State<AppState>,
    Query(query): Query<ListIssueRecordsQuery>,
    _auth: AuthContext,
) -> Result<Json<PaginatedResponse<IssueRecordResponse>>, AppError> {
    query.validate()?;
    let service = ColorCardIssueService::from_state(&state);
    let (records, total) = service.list_records(query).await?;
    Ok(Json(PaginatedResponse::new(records.into_iter().map(Into::into).collect(), total)))
}

/// 查询单条发放记录 GET /color-cards/issue/:id
pub async fn get_issue_record(
    State(state): State<AppState>,
    Path(record_id): Path<i64>,
    _auth: AuthContext,
) -> Result<Json<IssueRecordResponse>, AppError> {
    let service = ColorCardIssueService::from_state(&state);
    let record = service.get_by_id(record_id).await?;
    Ok(Json(record.into()))
}
```

**路由注册**（`backend/src/routes/color_card.rs`）：

```rust
// 旧路由（必须删除）：
// - POST /color-cards/:id/borrow
// - POST /color-cards/:id/return
// - POST /color-cards/:id/lost
// - POST /color-cards/:id/damaged
// - POST /color-cards/:id/cancel-borrow

// 新路由（必须注册）：
pub fn color_card_issue_routes() -> Router<AppState> {
    Router::new()
        .route("/color-cards/:id/issue", post(handlers::color_card::issue::issue_card))
        .route("/color-cards/issue", get(handlers::color_card::issue::list_issue_records))
        .route("/color-cards/issue/:id", get(handlers::color_card::issue::get_issue_record))
        .route("/color-cards/issue/:id/receive", post(handlers::color_card::issue::mark_received))
        .route("/color-cards/issue/:id/use", post(handlers::color_card::issue::mark_used))
        .route("/color-cards/issue/:id/cancel", post(handlers::color_card::issue::cancel_issue))
}
```

#### 10.2 色卡发放业务规则校验（详细校验矩阵）

**V15 检查要点**：

##### 10.2.1 发放前校验矩阵（5 道闸门）

| 校验项 | 校验规则 | 失败错误码 | 实现位置 |
|--------|----------|-----------|----------|
| 色卡存在性 | `color_card_id` 对应记录必须存在 | `ColorCardNotFound` | `issue_card` 第 1 步 |
| 色卡状态 | `status = 'active'`（非 archived/lost） | `InvalidState` | `issue_card` 第 2 步 |
| 色卡库存 | `total_colors >= quantity` | `InsufficientStock` | `issue_card` 第 3 步 |
| 客户有效性 | 客户存在且 `is_active = true` | `CustomerNotFound` | `issue_card` 第 4 步 |
| 重复发放检查 | 同一 `(color_card_id, customer_id)` 无 `issued`/`received` 状态记录 | `InvalidState` | `issue_card` 第 5 步 |

**特殊场景校验**：
1. **色卡归档后禁止发放**：`archived` 状态色卡禁止发放，必须先取消归档
2. **色卡遗失后禁止发放**：`lost` 状态色卡禁止发放（旧 borrow 模式遗留状态）
3. **客户停用后禁止发放**：客户 `is_active = false` 时禁止发放
4. **数量超限**：单次发放数量 > 100 时拒绝（防止误操作）
5. **有效期超限**：`valid_days > 365` 时拒绝（最长 1 年有效期）

##### 10.2.2 状态流转校验矩阵

| 当前状态 | 操作 | 目标状态 | 校验规则 |
|----------|------|----------|----------|
| Issued | mark_received | Received | `can_receive()` = true（仅 Issued 可标记） |
| Received | mark_used | Used | `can_use()` = true（仅 Received 可标记） |
| Issued | cancel_issue | Cancelled | `can_cancel()` = true（仅 Issued 可取消） |
| Issued/Received | mark_expired（定时任务） | Expired | `is_terminal()` = false 且 `Utc::now() > expected_valid_until` |
| Cancelled/Used/Expired | 任何操作 | — | 拒绝（终态不可变更） |

**禁止的流转**（必须拒绝）：
- ❌ Issued → Used（必须先 Received，禁止跳过客户确认）
- ❌ Received → Cancelled（已收到后不可取消，色卡已发出）
- ❌ Used → Expired（已使用不会过期，Used 是终态）
- ❌ Cancelled → 任何状态（终态）

##### 10.2.3 库存联动规则

| 操作 | 库存变化 | 事务要求 |
|------|----------|----------|
| 发放（issue_card） | 色卡库存 `-quantity` | 同一事务内扣减 |
| 取消发放（cancel_issue） | 色卡库存 `+quantity` | 同一事务内恢复 |
| 标记已收到（mark_received） | 不变 | — |
| 标记已使用（mark_used） | 不变 | — |
| 标记已过期（mark_expired） | 不变 | 已过期不恢复库存（色卡已发出，客户责任） |

**并发安全**：
- 所有库存操作必须在事务内 + `lock_exclusive()` 串行化
- 禁止使用 `update().col_expr()` 直接更新库存（必须先查后改，避免覆盖）

##### 10.2.4 客户专属色卡库规则

1. **客户色卡库视图**：
   - 每个客户可查看自己收到的所有色卡（`issue_status IN ('received', 'used')`）
   - 按色卡类型（PANTONE/CNCS/CUSTOM）分类展示
   - 支持按缸号、色号、发放时间筛选

2. **复购指定同缸号**（fabric-industry-research §4.1）：
   - 客户复购时，系统查询该客户历史发放色卡关联的缸号
   - 提示库存中同缸号面料（颜色一致性）
   - 同缸号面料优先发货

3. **历史色卡追溯**：
   - 客户可查看所有历史发放记录（含已过期）
   - 支持按时间范围、色卡类型、缸号查询
   - 已过期色卡仅展示，不可操作

#### 10.3 色卡发放与订单集成（详细集成方案）

##### 10.3.1 色卡发放记录与订单关联

**关联模式**：
- **强关联**：发放时关联 `sales_order_id`（客户订单驱动发放色卡场景）
- **弱关联**：发放时不关联订单（主动营销发放，后续可补关联）
- **复购关联**：复购订单创建时，自动查询客户历史色卡，提示同缸号面料

**数据库字段**：
```sql
-- color_card_issue_record.sales_order_id（可空）
-- 非空 = 强关联，NULL = 弱关联
```

**API 查询接口**：
- `GET /color-cards/issue?sales_order_id={id}` — 查询订单关联的发放记录
- `GET /sales-orders/{id}/color-cards` — 查询订单关联的色卡清单

##### 10.3.2 复购指定同缸号业务流程

```
客户复购下单 → 系统查询客户历史色卡 → 提示同缸号面料库存
     ↓                                        ↓
创建销售订单                          同缸号面料优先发货
     ↓                                        ↓
关联历史色卡记录                    库存不足时提示补货或换缸号
```

**实现要点**：
1. 销售订单创建 handler 中，查询客户历史色卡
2. 查询库存中同缸号面料（`inventory WHERE dye_lot_no = ?`）
3. 返回前端提示"客户历史色卡缸号 XXX，库存 N 米"
4. 用户可选择指定缸号或换缸号

##### 10.3.3 色卡发放报表

**报表类型**：
1. **发放明细报表**：按客户/色卡/时间维度查询发放记录
2. **发放汇总报表**：按客户/色卡类型/时间维度汇总发放数量
3. **客户色卡台账**：每个客户的色卡库存清单
4. **过期未使用报表**：已过期但未使用的色卡清单
5. **订单关联报表**：发放记录与销售订单关联查询

**导出格式**（规则 3）：
- 所有报表支持 `.xlsx` 导出（使用 `rust_xlsxwriter`）
- 月报支持 `.docx` 生成（使用 `docx-rs`）

##### 10.3.4 色卡成本核算

**成本归集规则**：
1. **色卡制作成本**：色卡生产成本归集到"营销费用-色卡制作"
2. **色卡发放成本**：发放时成本结转到"营销费用-色卡发放"（非主营业务成本）
3. **取消发放恢复**：取消发放时成本回转
4. **过期损失**：已过期色卡的成本计入"营销费用-色卡损失"

**财务凭证**（fabric-industry-research §5.4 规范）：
```
发放时：
  借：营销费用-色卡发放
  贷：库存商品-色卡

取消发放时（红字冲回）：
  借：库存商品-色卡
  贷：营销费用-色卡发放
```

#### 10.4 色卡发放权限管理（详细权限矩阵）

##### 10.4.1 角色权限矩阵

| 角色 | 发放色卡 | 标记已收到 | 标记已使用 | 取消发放 | 查询发放记录 | 查看成本 |
|------|----------|-----------|-----------|----------|--------------|----------|
| 管理员 | ✅ | ✅ | ✅ | ✅ | ✅ 所有客户 | ✅ |
| 销售经理 | ✅ | ✅ | ✅ | ✅ | ✅ 本部门客户 | ✅ |
| 销售 | ✅ 自己客户 | ❌ | ❌ | ✅ 自己客户 | ✅ 自己客户 | ❌ |
| 客服 | ✅ | ✅ | ❌ | ❌ | ✅ 所有客户 | ❌ |
| 客户（门户） | ❌ | ✅ 自己收到 | ✅ 自己收到 | ❌ | ✅ 自己收到 | ❌ |
| 财务 | ❌ | ❌ | ❌ | ❌ | ✅ 所有客户 | ✅ |

**权限实现**：
- 后端：`AuthContext` 注入 `user_id` + `role`，handler 校验权限
- 前端：路由守卫 + 按钮级权限控制（`v-permission` 指令）

##### 10.4.2 数据权限规则

1. **销售数据隔离**：销售只能查看自己负责的客户发放记录
   - SQL：`WHERE customer_id IN (SELECT customer_id FROM customer_sales_rep WHERE sales_rep_id = ?)`

2. **客户门户数据隔离**：客户只能查看自己的色卡
   - SQL：`WHERE customer_id = ?`（客户登录后只能看自己）

3. **成本数据敏感**：色卡制作成本仅管理层/财务可查看
   - 响应 DTO 中 `cost_amount` 字段按权限过滤

##### 10.4.3 审计日志要求

| 操作 | 审计日志内容 |
|------|--------------|
| 发放色卡 | `user_id` + `color_card_id` + `customer_id` + `quantity` + `issued_at` |
| 标记已收到 | `user_id` + `record_id` + `received_at` |
| 标记已使用 | `user_id` + `record_id` + `used_at` |
| 取消发放 | `user_id` + `record_id` + `cancel_reason` + `cancelled_at` |
| 定时过期 | `system` + `record_id` + `expired_at` |

所有审计日志写入 `audit_log` 表，保留 2 年以上。

#### 10.5 色卡发放定时任务（详细 cron 配置）

##### 10.5.1 过期检查定时任务

**任务名称**：`color_card_issue_expiry_check`

**Cron 表达式**：`0 2 * * *`（每日凌晨 2:00 执行）

**任务逻辑**：
```rust
/// 定时任务：色卡发放记录过期检查
/// 每日 02:00 执行，扫描所有已超期但未过期的发放记录
pub async fn color_card_issue_expiry_check(app_state: &AppState) {
    let service = ColorCardIssueService::from_state(app_state);
    match service.batch_check_expired().await {
        Ok(count) => {
            tracing::info!(expired_count = count, "色卡发放记录过期检查完成");
        }
        Err(e) => {
            tracing::error!(error = %e, "色卡发放记录过期检查失败");
        }
    }
}
```

**注册方式**（使用 `tokio-cron-scheduler`）：
```rust
// backend/src/utils/scheduler.rs
use tokio_cron_scheduler::{Job, JobScheduler};

pub async fn init_scheduler(app_state: Arc<AppState>) -> Result<JobScheduler, Box<dyn std::error::Error>> {
    let scheduler = JobScheduler::new().await?;

    // 色卡发放记录过期检查（每日 02:00）
    let state = app_state.clone();
    scheduler.add(Job::new_async("0 2 * * *", move |_, _| {
        let state = state.clone();
        Box::pin(async move {
            color_card_issue_expiry_check(&state).await;
        })
    })?).await?;

    scheduler.start().await?;
    Ok(scheduler)
}
```

##### 10.5.2 库存预警定时任务

**任务名称**：`color_card_stock_warning`

**Cron 表达式**：`0 8 * * *`（每日 08:00 执行）

**任务逻辑**：
```rust
/// 定时任务：色卡库存预警
/// 每日 08:00 检查色卡库存，低于阈值时告警
pub async fn color_card_stock_warning(app_state: &AppState) {
    // 查询所有 active 状态色卡
    // 检查 total_colors < 阈值（默认 5，可配置）
    // 发送告警通知（邮件/站内信）
}
```

**告警规则**：
- 色卡库存 < 5：黄色预警（提醒补货）
- 色卡库存 < 2：红色预警（紧急补货）
- 色卡库存 = 0：禁止发放并告警

##### 10.5.3 发放统计定时任务

**任务名称**：`color_card_issue_daily_stats`

**Cron 表达式**：`0 23 * * *`（每日 23:00 执行）

**任务逻辑**：
```rust
/// 定时任务：色卡发放日统计
/// 每日 23:00 统计当日发放数量，写入统计表
pub async fn color_card_issue_daily_stats(app_state: &AppState) {
    // 查询当日所有发放记录
    // 按客户/色卡类型/销售员维度汇总
    // 写入 color_card_issue_daily_stats 表
    // 生成日报推送
}
```

**统计表**：
```sql
CREATE TABLE color_card_issue_daily_stats (
    id BIGSERIAL PRIMARY KEY,
    stat_date DATE NOT NULL,
    customer_id BIGINT,
    color_card_type VARCHAR(20),  -- PANTONE/CNCS/CUSTOM
    sales_rep_id BIGINT,
    total_issued INT NOT NULL DEFAULT 0,
    total_cancelled INT NOT NULL DEFAULT 0,
    total_expired INT NOT NULL DEFAULT 0,
    net_issued INT NOT NULL DEFAULT 0,  -- total_issued - total_cancelled
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(stat_date, customer_id, color_card_type, sales_rep_id)
);
```

##### 10.5.4 单元测试清单

**测试文件**：`backend/src/services/color_card_issue_service.rs` 的 `#[cfg(test)] mod tests`

**必须编写的单元测试**：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // ========== IssueStatus 状态机测试 ==========

    #[test]
    fn 测试_issue_status_as_str_全部状态映射() {
        assert_eq!(IssueStatus::Issued.as_str(), "issued");
        assert_eq!(IssueStatus::Received.as_str(), "received");
        assert_eq!(IssueStatus::Used.as_str(), "used");
        assert_eq!(IssueStatus::Expired.as_str(), "expired");
        assert_eq!(IssueStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn 测试_issue_status_is_terminal_终态判定() {
        assert!(!IssueStatus::Issued.is_terminal());
        assert!(!IssueStatus::Received.is_terminal());
        assert!(IssueStatus::Used.is_terminal());
        assert!(IssueStatus::Expired.is_terminal());
        assert!(IssueStatus::Cancelled.is_terminal());
    }

    #[test]
    fn 测试_issue_status_can_cancel_仅_issued_可取消() {
        assert!(IssueStatus::Issued.can_cancel());
        assert!(!IssueStatus::Received.can_cancel());
        assert!(!IssueStatus::Used.can_cancel());
        assert!(!IssueStatus::Expired.can_cancel());
        assert!(!IssueStatus::Cancelled.can_cancel());
    }

    #[test]
    fn 测试_issue_status_can_receive_仅_issued_可标记已收到() {
        assert!(IssueStatus::Issued.can_receive());
        assert!(!IssueStatus::Received.can_receive());
    }

    #[test]
    fn 测试_issue_status_can_use_仅_received_可标记已使用() {
        assert!(IssueStatus::Received.can_use());
        assert!(!IssueStatus::Issued.can_use());
        assert!(!IssueStatus::Used.can_use());
    }

    #[test]
    fn 测试_issue_status_from_str_合法解析() {
        assert!(matches!(IssueStatus::from_str("issued"), Ok(IssueStatus::Issued)));
        assert!(matches!(IssueStatus::from_str("received"), Ok(IssueStatus::Received)));
        assert!(matches!(IssueStatus::from_str("used"), Ok(IssueStatus::Used)));
        assert!(matches!(IssueStatus::from_str("expired"), Ok(IssueStatus::Expired)));
        assert!(matches!(IssueStatus::from_str("cancelled"), Ok(IssueStatus::Cancelled)));
    }

    #[test]
    fn 测试_issue_status_from_str_非法解析() {
        assert!(IssueStatus::from_str("borrowed").is_err());
        assert!(IssueStatus::from_str("returned").is_err());
        assert!(IssueStatus::from_str("lost").is_err());
        assert!(IssueStatus::from_str("damaged").is_err());
        assert!(IssueStatus::from_str("").is_err());
        assert!(IssueStatus::from_str("INVALID").is_err());
    }

    // ========== 状态机流转测试（mock 数据库） ==========

    #[tokio::test]
    async fn 测试_发放色卡_成功() {
        // 准备：active 色卡 + active 客户 + 无重复发放
        // 执行：issue_card
        // 断言：记录创建，状态 = issued，库存扣减
    }

    #[tokio::test]
    async fn 测试_发放色卡_色卡不存在_失败() {
        // 断言：返回 ColorCardNotFound
    }

    #[tokio::test]
    async fn 测试_发放色卡_色卡已归档_失败() {
        // 断言：返回 InvalidState
    }

    #[tokio::test]
    async fn 测试_发放色卡_库存不足_失败() {
        // 断言：返回 InsufficientStock
    }

    #[tokio::test]
    async fn 测试_发放色卡_重复发放_失败() {
        // 准备：已存在 issued/received 状态记录
        // 断言：返回 InvalidState（重复发放）
    }

    #[tokio::test]
    async fn 测试_标记已收到_成功() {
        // 准备：issued 状态记录
        // 执行：mark_received
        // 断言：状态 = received，received_at 填充
    }

    #[tokio::test]
    async fn 测试_标记已收到_已收到状态_失败() {
        // 准备：received 状态记录
        // 断言：返回 InvalidState（不能重复标记已收到）
    }

    #[tokio::test]
    async fn 测试_标记已使用_成功() {
        // 准备：received 状态记录
        // 执行：mark_used
        // 断言：状态 = used，used_at 填充
    }

    #[tokio::test]
    async fn 测试_标记已使用_issued状态_失败() {
        // 准备：issued 状态记录（未收到）
        // 断言：返回 InvalidState（必须先收到才能使用）
    }

    #[tokio::test]
    async fn 测试_取消发放_成功() {
        // 准备：issued 状态记录
        // 执行：cancel_issue
        // 断言：状态 = cancelled，库存恢复
    }

    #[tokio::test]
    async fn 测试_取消发放_已收到状态_失败() {
        // 准备：received 状态记录
        // 断言：返回 InvalidState（已收到不可取消）
    }

    #[tokio::test]
    async fn 测试_取消发放_已使用状态_失败() {
        // 准备：used 状态记录
        // 断言：返回 InvalidState（已使用不可取消）
    }

    #[tokio::test]
    async fn 测试_标记已过期_未超有效期_失败() {
        // 准备：issued 状态记录，expected_valid_until > now
        // 断言：返回 InvalidState（未超有效期）
    }

    #[tokio::test]
    async fn 测试_标记已过期_已超有效期_成功() {
        // 准备：issued 状态记录，expected_valid_until < now
        // 执行：mark_expired
        // 断言：状态 = expired，expired_at 填充
    }

    #[tokio::test]
    async fn 测试_标记已过期_终态状态_失败() {
        // 准备：used/cancelled 状态记录
        // 断言：返回 InvalidState（终态不可变更）
    }

    #[tokio::test]
    async fn 测试_批量过期检查_扫描超期记录() {
        // 准备：多条超期记录
        // 执行：batch_check_expired
        // 断言：所有超期记录标记为 expired
    }
}
```

**测试覆盖率要求**：
- 状态机所有流转路径 100% 覆盖
- 所有错误场景 100% 覆盖
- 业务规则校验 100% 覆盖
- 禁止使用 `#[ignore]` 跳过测试

#### 10.6 色卡发放前端重构详细规范（V15 新增）

> **前端技术栈**：Vue 3 + TypeScript + Pinia + Vue Router + Element Plus（或 Ant Design Vue）

##### 10.6.1 前端文件结构

```
frontend/src/
├── api/
│   └── color-card-issue.ts          # 色卡发放 API 模块
├── types/
│   └── color-card-issue.ts          # 色卡发放类型定义
├── composables/
│   └── useColorCardIssue.ts         # 色卡发放组合式函数
├── stores/
│   └── color-card-issue.ts          # Pinia 状态管理
├── views/
│   └── color-card/
│       ├── IssueList.vue            # 发放记录列表页
│       ├── IssueForm.vue            # 发放表单（发放色卡）
│       └── IssueDetail.vue          # 发放记录详情页
└── router/
    └── modules/
        └── color-card-issue.ts      # 路由配置
```

##### 10.6.2 类型定义（`types/color-card-issue.ts`）

```typescript
// 色卡发放状态枚举（与后端 IssueStatus 对应）
export enum IssueStatus {
  ISSUED = 'issued',
  RECEIVED = 'received',
  USED = 'used',
  EXPIRED = 'expired',
  CANCELLED = 'cancelled',
}

// 发放状态标签映射
export const ISSUE_STATUS_LABELS: Record<IssueStatus, string> = {
  [IssueStatus.ISSUED]: '已发放',
  [IssueStatus.RECEIVED]: '已收到',
  [IssueStatus.USED]: '已使用',
  [IssueStatus.EXPIRED]: '已过期',
  [IssueStatus.CANCELLED]: '已取消',
}

// 发放状态颜色映射（Element Plus tag type）
export const ISSUE_STATUS_COLORS: Record<IssueStatus, string> = {
  [IssueStatus.ISSUED]: 'warning',
  [IssueStatus.RECEIVED]: 'primary',
  [IssueStatus.USED]: 'success',
  [IssueStatus.EXPIRED]: 'info',
  [IssueStatus.CANCELLED]: 'danger',
}

// 创建发放记录请求
export interface CreateIssueRecordDto {
  color_card_id: number
  customer_id: number
  color_id?: number
  dye_lot_no?: string
  sales_order_id?: number
  quantity?: number
  purpose?: string
  tracking_no?: string
  valid_days?: number
}

// 发放记录响应
export interface IssueRecordResponse {
  id: number
  color_card_id: number
  color_card_name?: string
  customer_id: number
  customer_name?: string
  color_id?: number
  dye_lot_no?: string
  sales_order_id?: number
  issued_by: number
  issued_by_name?: string
  issued_at: string
  received_at?: string
  used_at?: string
  expired_at?: string
  expected_valid_until: string
  quantity: number
  issue_status: IssueStatus
  purpose?: string
  tracking_no?: string
  customer_feedback?: string
  cancel_reason?: string
  cancelled_by?: number
  cancelled_at?: string
}

// 列表查询参数
export interface ListIssueRecordsQuery {
  color_card_id?: number
  customer_id?: number
  issue_status?: IssueStatus
  dye_lot_no?: string
  sales_order_id?: number
  from_date?: string
  to_date?: string
  page?: number
  page_size?: number
}
```

##### 10.6.3 API 模块（`api/color-card-issue.ts`）

```typescript
import request from '@/utils/request'
import type {
  CreateIssueRecordDto,
  IssueRecordResponse,
  ListIssueRecordsQuery,
  MarkReceivedDto,
  MarkUsedDto,
  CancelIssueDto,
} from '@/types/color-card-issue'

const BASE_URL = '/color-cards'

// 发放色卡
export function issueCard(colorCardId: number, data: CreateIssueRecordDto) {
  return request.post<IssueRecordResponse>(`${BASE_URL}/${colorCardId}/issue`, data)
}

// 标记已收到
export function markReceived(recordId: number, data: MarkReceivedDto) {
  return request.post<IssueRecordResponse>(`${BASE_URL}/issue/${recordId}/receive`, data)
}

// 标记已使用
export function markUsed(recordId: number, data: MarkUsedDto) {
  return request.post<IssueRecordResponse>(`${BASE_URL}/issue/${recordId}/use`, data)
}

// 取消发放
export function cancelIssue(recordId: number, data: CancelIssueDto) {
  return request.post<IssueRecordResponse>(`${BASE_URL}/issue/${recordId}/cancel`, data)
}

// 查询发放记录列表
export function listIssueRecords(params: ListIssueRecordsQuery) {
  return request.get<{ items: IssueRecordResponse[]; total: number }>(`${BASE_URL}/issue`, { params })
}

// 查询单条发放记录
export function getIssueRecord(recordId: number) {
  return request.get<IssueRecordResponse>(`${BASE_URL}/issue/${recordId}`)
}
```

##### 10.6.4 组合式函数（`composables/useColorCardIssue.ts`）

```typescript
import { ref, reactive } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  issueCard as apiIssueCard,
  markReceived as apiMarkReceived,
  markUsed as apiMarkUsed,
  cancelIssue as apiCancelIssue,
  listIssueRecords as apiListIssueRecords,
} from '@/api/color-card-issue'
import type { CreateIssueRecordDto, IssueRecordResponse, ListIssueRecordsQuery } from '@/types/color-card-issue'

export function useColorCardIssue() {
  const loading = ref(false)
  const records = ref<IssueRecordResponse[]>([])
  const total = ref(0)

  const queryParams = reactive<ListIssueRecordsQuery>({
    page: 1,
    page_size: 20,
  })

  // 加载列表
  async function loadList() {
    loading.value = true
    try {
      const res = await apiListIssueRecords(queryParams)
      records.value = res.items
      total.value = res.total
    } finally {
      loading.value = false
    }
  }

  // 发放色卡
  async function issueCard(colorCardId: number, data: CreateIssueRecordDto) {
    await apiIssueCard(colorCardId, data)
    ElMessage.success('色卡发放成功')
    await loadList()
  }

  // 标记已收到
  async function markReceived(recordId: number, customerFeedback?: string) {
    await apiMarkReceived(recordId, { customer_feedback: customerFeedback })
    ElMessage.success('已标记为已收到')
    await loadList()
  }

  // 标记已使用
  async function markUsed(recordId: number, customerFeedback?: string) {
    await apiMarkUsed(recordId, { customer_feedback: customerFeedback })
    ElMessage.success('已标记为已使用')
    await loadList()
  }

  // 取消发放
  async function cancelIssue(recordId: number) {
    const { value } = await ElMessageBox.prompt('请输入取消原因', '取消发放', {
      confirmButtonText: '确认取消',
      cancelButtonText: '返回',
      inputType: 'textarea',
      inputValidator: (val) => val.trim().length > 0 || '取消原因不能为空',
    })
    await apiCancelIssue(recordId, { cancel_reason: value })
    ElMessage.success('已取消发放')
    await loadList()
  }

  return {
    loading,
    records,
    total,
    queryParams,
    loadList,
    issueCard,
    markReceived,
    markUsed,
    cancelIssue,
  }
}
```

##### 10.6.5 视图组件规范

**`IssueList.vue`（发放记录列表页）必须移除的旧功能**：
- ❌ "借出"按钮
- ❌ "归还"按钮
- ❌ "登记遗失"按钮
- ❌ "标记损坏"按钮
- ❌ "取消借出"按钮

**必须新增的新功能**：
- ✅ "发放色卡"按钮（跳转 IssueForm）
- ✅ "标记已收到"按钮（仅 issued 状态显示）
- ✅ "标记已使用"按钮（仅 received 状态显示）
- ✅ "取消发放"按钮（仅 issued 状态显示）
- ✅ 状态筛选下拉框（issued/received/used/expired/cancelled）
- ✅ 发放时间范围筛选
- ✅ 状态列使用 `<el-tag>` + ISSUE_STATUS_COLORS 颜色映射
- ✅ 导出 .xlsx 按钮

**`IssueForm.vue`（发放表单）字段**：
- 色卡选择（下拉，仅 active 状态）
- 客户选择（下拉，仅 active 客户）
- 色号选择（可选，下拉）
- 缸号输入（可选，文本框）
- 销售订单关联（可选，下拉）
- 发放数量（数字输入，默认 1，范围 1-100）
- 发放用途（下拉：打样确认/大货前对色/客户存档/其他）
- 物流单号（可选，文本框）
- 有效期天数（数字输入，默认 90，范围 1-365）

##### 10.6.6 路由配置（`router/modules/color-card-issue.ts`）

```typescript
import type { RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
  {
    path: '/color-card/issue',
    name: 'ColorCardIssueList',
    component: () => import('@/views/color-card/IssueList.vue'),
    meta: { title: '色卡发放记录', requiresAuth: true, permission: 'color_card.issue.view' },
  },
  {
    path: '/color-card/issue/create',
    name: 'ColorCardIssueCreate',
    component: () => import('@/views/color-card/IssueForm.vue'),
    meta: { title: '发放色卡', requiresAuth: true, permission: 'color_card.issue.create' },
  },
  {
    path: '/color-card/issue/:id',
    name: 'ColorCardIssueDetail',
    component: () => import('@/views/color-card/IssueDetail.vue'),
    meta: { title: '发放记录详情', requiresAuth: true, permission: 'color_card.issue.view' },
  },
]

export default routes
```

##### 10.6.7 权限指令（`directives/permission.ts`）

```typescript
// v-permission 指令实现
import { Directive } from 'vue'
import { useUserStore } from '@/stores/user'

export const permission: Directive = {
  mounted(el, binding) {
    const userStore = useUserStore()
    const requiredPermission = binding.value as string
    if (!userStore.hasPermission(requiredPermission)) {
      el.parentNode?.removeChild(el)
    }
  },
}
```

**模板使用示例**：
```vue
<el-button v-permission="'color_card.issue.create'" type="primary" @click="handleIssue">
  发放色卡
</el-button>
<el-button
  v-permission="'color_card.issue.receive'"
  v-if="record.issue_status === 'issued'"
  @click="handleMarkReceived(record.id)"
>
  标记已收到
</el-button>
```

#### 10.7 色卡发放 DB 数据迁移脚本（V15 新增）

> **强制要求**：旧 `color_card_borrow_record` 表中的历史数据必须迁移到新 `color_card_issue_record` 表，禁止直接 DROP。

##### 10.7.1 数据迁移策略

| 旧状态 | 新状态映射 | 迁移说明 |
|--------|-----------|----------|
| borrowed | issued | 借出中 → 已发放（未收到） |
| returned | used | 已归还 → 已使用（已归还视为已使用，色卡已发出） |
| lost | expired | 遗失 → 已过期（遗失色卡视为过期，不恢复库存） |
| damaged | expired | 损坏 → 已过期（损坏色卡视为过期，不恢复库存） |
| cancelled | cancelled | 已取消 → 已取消（直接映射） |

##### 10.7.2 数据迁移 SQL 脚本

```sql
-- 迁移文件：backend/migration/xxxx_migrate_borrow_to_issue_data.sql
-- V15 类十 10.7.2：将旧借还记录数据迁移到发放记录表

-- 步骤 1：重命名旧表为 legacy 备份
ALTER TABLE color_card_borrow_record RENAME TO color_card_borrow_record_legacy;

-- 步骤 2：创建新表 color_card_issue_record（详见 10.1.3）

-- 步骤 3：数据迁移（INSERT INTO ... SELECT ...）
INSERT INTO color_card_issue_record (
    color_card_id,
    customer_id,
    color_id,
    dye_lot_no,
    sales_order_id,
    issued_by,
    issued_at,
    received_at,
    used_at,
    expired_at,
    expected_valid_until,
    quantity,
    issue_status,
    purpose,
    tracking_no,
    customer_feedback,
    cancel_reason,
    cancelled_by,
    cancelled_at,
    created_at,
    updated_at,
    created_by,
    updated_by
)
SELECT
    b.color_card_id,
    b.customer_id,
    NULL AS color_id,
    b.dye_lot_no,
    NULL AS sales_order_id,
    b.borrowed_by AS issued_by,
    b.borrowed_at AS issued_at,
    -- received_at: returned 状态填充归还时间
    CASE WHEN b.status = 'returned' THEN b.actual_return_at ELSE NULL END AS received_at,
    -- used_at: returned 状态填充归还时间（视为已使用）
    CASE WHEN b.status = 'returned' THEN b.actual_return_at ELSE NULL END AS used_at,
    -- expired_at: lost/damaged 状态填充实际时间
    CASE WHEN b.status IN ('lost', 'damaged') THEN b.actual_return_at ELSE NULL END AS expired_at,
    -- expected_valid_until: 旧记录无此字段，默认 borrowed_at + 90 天
    b.borrowed_at + INTERVAL '90 days' AS expected_valid_until,
    1 AS quantity,
    -- issue_status 映射
    CASE b.status
        WHEN 'borrowed' THEN 'issued'
        WHEN 'returned' THEN 'used'
        WHEN 'lost' THEN 'expired'
        WHEN 'damaged' THEN 'expired'
        WHEN 'cancelled' THEN 'cancelled'
        ELSE 'issued'
    END AS issue_status,
    b.purpose,
    NULL AS tracking_no,
    b.notes AS customer_feedback,
    -- cancel_reason: cancelled 状态填充 notes
    CASE WHEN b.status = 'cancelled' THEN b.notes ELSE NULL END AS cancel_reason,
    NULL AS cancelled_by,
    CASE WHEN b.status = 'cancelled' THEN b.updated_at ELSE NULL END AS cancelled_at,
    b.created_at,
    b.updated_at,
    NULL AS created_by,
    NULL AS updated_by
FROM color_card_borrow_record_legacy b
WHERE b.status IS NOT NULL;

-- 步骤 4：验证迁移数据完整性
SELECT '迁移前后记录数对比' AS check_name,
    (SELECT COUNT(*) FROM color_card_borrow_record_legacy) AS old_count,
    (SELECT COUNT(*) FROM color_card_issue_record) AS new_count;

-- 步骤 5：验证状态映射正确性
SELECT issue_status, COUNT(*) AS cnt
FROM color_card_issue_record
GROUP BY issue_status
ORDER BY issue_status;

-- 步骤 6：保留 legacy 表（不删除，供审计追溯）
-- 添加注释标记
COMMENT ON TABLE color_card_borrow_record_legacy IS 'V15 迁移遗留表：旧借还模式历史数据备份，仅供审计追溯，禁止写入';
```

##### 10.7.3 代码层旧文件处理

| 旧文件 | 处理方式 | 说明 |
|--------|----------|------|
| `backend/src/models/color_card_borrow_record.rs` | 重命名为 `color_card_borrow_record_legacy.rs` | 仅保留 Entity 映射 legacy 表，仅供审计查询 |
| `backend/src/models/color_card_borrow_dto.rs` | 删除 | 旧 DTO 不再使用 |
| `backend/src/services/color_card_borrow_service.rs` | 删除 | 旧 service 不再使用 |
| `backend/src/handlers/color_card/borrow.rs` | 删除 | 旧 handler 不再使用 |
| 路由 `POST /color-cards/:id/borrow` 等 | 删除 | 旧路由全部删除 |

**删除前检查**（必须 grep 确认无引用）：
```bash
# 检查旧 borrow 相关代码是否还有引用
grep -rn "color_card_borrow\|borrow_card\|return_card\|mark_lost\|mark_damaged\|cancel_borrow" backend/src/
# 确认无业务引用后才能删除
```

##### 10.7.4 迁移回滚方案

```sql
-- 回滚脚本：backend/migration/xxxx_migrate_borrow_to_issue_rollback.sql
-- 仅在 V15 上线失败需要回滚时执行

-- 1. 删除新表数据
TRUNCATE color_card_issue_record;

-- 2. 删除新表
DROP TABLE IF EXISTS color_card_issue_record;

-- 3. 恢复旧表
ALTER TABLE color_card_borrow_record_legacy RENAME TO color_card_borrow_record;
```

**回滚触发条件**：
- 迁移后 24 小时内发现严重 bug
- 业务方明确要求回滚
- 数据迁移不完整（记录数不一致）

---

### 类十一：大货批色业务规则专项（6 维度）⭐ V15 新增（用户 2026-07-15 第二轮反馈）

> **背景**：用户 2026-07-15 第二轮反馈明确要求"在交货给客户前，客户需要批色，剪大货样进行批色"。这是面料行业的核心质量管控环节——大货生产完成后，必须从大货中剪取样布让客户批色确认，批色通过后才能正式交货。
> **业务依据**：[fabric-industry-research.md](file:///workspace/.monkeycode/docs/research/fabric-industry-research.md) §3.1 染整工艺 10 道工序（第 9 道"成品对色"、第 10 道"成检"）+ §4.7 质量检验模块（CIE D65 色差值 ΔE ≤ 1.2 同色判定）。
> **业务流程**：大货入库 → 剪大货样 → 发送客户 → 客户批色 → 批色通过 → 交货 / 批色不通过 → 返工/降级/报废

#### 11.1 大货批色数据模型与状态机

**V15 检查要点**：
1. **大货批色记录数据模型**（新增表 `bulk_color_approval`）：
   - `id`：主键
   - `production_order_id`：关联生产订单
   - `sales_order_id`：关联销售订单
   - `product_id` + `color_id` + `dye_lot_no` + `batch_no`：四维标识（关联大货面料，遵循 v14 四层级联约束）
   - `sample_piece_id`：剪样对应的 inventory_piece 记录
   - `sample_length_m`：剪样长度（米，通常 0.5m-1m）
   - `sample_weight_kg`：剪样重量（公斤）
   - `sent_to_customer_at`：发送客户时间
   - `customer_id`：客户 ID
   - `approval_status`：批色状态（待批色/批色通过/批色不通过/返工中/降级/报废）
   - `delta_e_value`：色差值（客户测量或我方测量，ΔE）
   - `customer_feedback`：客户反馈
   - `approved_at`：批色通过时间
   - `approved_by`：客户确认人
   - `delivery_blocking`：是否阻止交货（true=未批色通过前禁止交货）
   - `created_at` / `updated_at`

2. **状态机定义**：
   - `PendingSample`（待剪样）：大货入库后等待剪样
   - `Sampled`（已剪样）：已剪大货样，等待发送客户
   - `SentToCustomer`（已发送客户）：样布已发送，等待批色
   - `Approved`（批色通过）：客户确认通过，可交货（终态）
   - `Rejected`（批色不通过）：客户拒绝，进入返工/降级/报废流程
   - `Reworking`（返工中）：返工处理，完成后重新进入 PendingSample
   - `Downgraded`（降级）：降为 B 级品销售（终态）
   - `Scrapped`（报废）：报废处理（终态）

3. **状态流转规则**：
   - `PendingSample → Sampled`：剪样操作
   - `Sampled → SentToCustomer`：发送客户
   - `SentToCustomer → Approved`：客户批色通过
   - `SentToCustomer → Rejected`：客户批色不通过
   - `Rejected → Reworking`：选择返工
   - `Rejected → Downgraded`：选择降级
   - `Rejected → Scrapped`：选择报废
   - `Reworking → PendingSample`：返工完成，重新剪样
   - 终态：`Approved` / `Downgraded` / `Scrapped`

4. **交货门禁**：
   - `delivery_blocking = true` 当状态非 `Approved` 时
   - 销售出库 handler 必须校验 `approval_status = Approved`
   - 未批色通过的面料禁止出库交货

**扫描方法**：
```bash
# 检查大货批色模型是否存在
grep -rn "bulk_color_approval\|大货批色\|批色" backend/src/models/
# 检查交货门禁是否接入
grep -rn "delivery_blocking\|approval_status.*Approved" backend/src/services/sales/
```

#### 11.2 剪大货样业务规则

**V15 检查要点**：
1. **剪样前置条件**：
   - 大货已入库（inventory_piece 状态为 available）
   - 大货质检已完成（quality_inspection 状态为 passed）
   - 生产订单状态为 completed
2. **剪样数量规则**：
   - 每缸号每批次至少剪 1 个样布
   - 客户特殊要求可剪多个样布（不同位置取样）
   - 剪样长度默认 0.5m，可配置（客户要求 1m 时支持）
3. **剪样库存联动**：
   - 剪样时从大货库存中扣减剪样长度
   - 剪样生成独立的 inventory_piece 记录（状态为 `sample`，不参与正常销售）
   - 剪样扣减事务化（与库存扣减同一事务）
4. **剪样标识**：
   - 剪样样布有独立编号（如 `SAMPLE-<dye_lot_no>-<batch_no>-<seq>`）
   - 样布关联缸号/批号/剪样时间/剪样人
5. **剪样追溯**：
   - 剪样记录可反查大货来源
   - 大货库存可查询已剪样次数和总剪样长度
   - 剪样追溯链：样布 → 大货 inventory_piece → 生产订单 → 缸号 → 染色配方

**扫描方法**：
```bash
# 检查剪样库存联动
grep -rn "sample_piece\|剪样" backend/src/services/inventory/
# 检查剪样事务化
grep -B 5 -A 20 "cut_sample\|create_sample" backend/src/services/ | grep "txn.begin"
```

#### 11.3 客户批色确认流程

**V15 检查要点**：
1. **批色通知**：
   - 剪样后自动通知客户（系统内消息 + 短信/邮件）
   - 通知内容含：样布编号/缸号/批号/面料信息/批色截止时间
   - 客户可在客户门户查看待批色清单
2. **客户批色操作**：
   - 客户登录客户门户 → 查看待批色清单 → 查看 ΔE 测量值 → 批色通过/不通过
   - 批色不通过需填写拒绝原因（色差超标/手感不符/克重偏差等）
   - 客户可上传自己的色差测量数据
3. **批色时限**：
   - 默认批色时限 3 天（可配置）
   - 超时未批色自动提醒
   - 超时 7 天未批色自动标记为 `Rejected`（避免大货长期积压）
4. **色差判定标准**（fabric-industry-research §4.7）：
   - ΔE ≤ 1.2：同色判定通过
   - ΔE ≤ 2.5：可接受（让步接收）
   - ΔE > 2.5：不合格（必须返工或降级）
   - 高光敏感区域 ΔE ≤ 0.8
5. **批色结果处理**：
   - 通过 → 状态变为 `Approved`，解除交货门禁
   - 不通过 → 状态变为 `Rejected`，触发后续处理流程

**扫描方法**：
```bash
# 检查客户门户批色接口
grep -rn "customer_portal\|customer_approval\|批色" backend/src/handlers/
# 检查色差判定
grep -rn "delta_e\|color_difference\|色差" backend/src/services/
```

#### 11.4 批色不通过处理流程

**V15 检查要点**：
1. **返工流程**（`Rejected → Reworking → PendingSample`）：
   - 返工需创建返工生产订单（关联原生产订单）
   - 返工染色配方调整记录
   - 返工完成后重新剪样批色
   - 返工次数限制（同一缸号最多返工 2 次，超过则强制降级/报废）
2. **降级流程**（`Rejected → Downgraded`）：
   - 降级为 B 级品（fabric-industry-research §4.7 质检分级 A/B/C）
   - 降级后重新定价（降价比例可配置）
   - 降级品单独库存管理（库位 = 次品区）
   - 降级品销售需客户确认接受 B 级品
3. **报废流程**（`Rejected → Scrapped`）：
   - 报废需审批（生产主管 + 质量主管 + 财务主管三审）
   - 报废生成报废单（关联大货 inventory_piece）
   - 报废库存扣减（状态变为 scrapped）
   - 报废成本核算（计入营业外支出，非正常损耗不进成本）
4. **财务凭证联动**（fabric-industry-research §5.4）：
   - 返工：返工成本归集到生产成本
   - 降级：降级损失计入资产减值损失
   - 报废：报废成本计入营业外支出（非正常损耗规则）

**扫描方法**：
```bash
# 检查返工流程
grep -rn "rework\|返工" backend/src/services/production/
# 检查降级
grep -rn "downgrade\|降级" backend/src/services/inventory/
# 检查报废
grep -rn "scrap\|报废" backend/src/services/
```

#### 11.5 批色报表与统计

**V15 检查要点**：
1. **批色通过率报表**：
   - 按客户/面料/缸号/时间维度统计批色通过率
   - 一次性批色通过率（首次批色通过 / 总批色次数）
   - 批色通过率趋势分析
2. **返工统计报表**：
   - 返工次数统计
   - 返工原因分析（色差/手感/克重等）
   - 返工成本统计
3. **降级报废统计报表**：
   - 降级数量/降级损失金额
   - 报废数量/报废损失金额
   - 按缸号/面料/客户维度统计
4. **客户响应时间报表**：
   - 客户批色平均响应时间
   - 超时未批色统计
   - 客户批色效率排名
5. **报表导出格式**（规则 3）：
   - 所有批色报表支持 .xlsx 导出
   - 批色月报支持 .docx 生成

**扫描方法**：
```bash
# 检查批色报表
grep -rn "approval_report\|批色报表" backend/src/services/
# 检查报表导出格式
grep -rn "xlsx\|docx" backend/src/services/report/
```

#### 11.6 批色业务与其他模块集成

**V15 检查要点**：
1. **与生产订单集成**：
   - 生产订单完成后自动触发剪样流程
   - 返工生产订单关联原订单
2. **与销售订单集成**：
   - 销售订单出库前校验批色状态
   - 交货门禁：未批色通过禁止出库
3. **与库存模块集成**：
   - 剪样扣减大货库存
   - 降级品库位调整
   - 报废品库存状态变更
4. **与质检模块集成**：
   - 剪样前校验大货质检通过
   - 批色作为质检的延伸（客户侧质检）
5. **与财务模块集成**：
   - 返工成本归集
   - 降级损失核算
   - 报废损失核算
6. **与缸号状态机集成**（v14 批次 432）：
   - 批色状态作为缸号状态机的一环
   - 缸号状态：投缸 → 染色 → ... → 入库 → 待批色 → 批色通过 → 可交货

**扫描方法**：
```bash
# 检查批色与生产订单集成
grep -rn "production_order.*approval\|approval.*production_order" backend/src/services/
# 检查批色与销售出库集成
grep -rn "approval.*delivery\|delivery.*approval" backend/src/services/sales/
# 检查批色与缸号状态机集成
grep -rn "dye_lot.*approval\|approval.*dye_lot" backend/src/services/
```

---

### 类十二：基于角色的权限控制机制（RBAC）专项（8 维度）⭐ V15 新增（用户 2026-07-15 第五轮反馈）

> **背景**：用户 2026-07-15 第五轮反馈明确要求"增加基于角色的权限控制机制"。现有权限审计分散在类三 3.4（认证与权限）、类四 4.2.6（面料行业权限）、类十 10.4（色卡发放权限）中，缺乏**系统性的 RBAC 架构审计**。V15 新增独立专项，覆盖 RBAC 完整生命周期：数据模型 → 权限矩阵 → 中间件 → 前端集成 → 审计日志 → 动态授权 → 数据权限 → 安全审计。
> **业务依据**：[fabric-industry-research.md](file:///workspace/.monkeycode/docs/research/fabric-industry-research.md) §4.1 销售管理模块（客户分级与价格体系）+ §4.8 财务与成本核算模块（多维度利润报表权限）+ [project_rules.md](file:///workspace/.trae/rules/project_rules.md) §4.2 输入验证 + §4.1 敏感信息保护。
> **RBAC 模型**：用户（User）→ 角色（Role）→ 权限（Permission）→ 资源（Resource）四层模型，支持数据权限（行级/字段级）。

#### 12.1 RBAC 数据模型与权限架构

**V15 检查要点**：

1. **RBAC 四层模型完整性**：
   - 用户（User）：`user` 表，含 `role_id`（单角色）或通过 `user_role` 关联表（多角色）
   - 角色（Role）：`role` 表，含 `role_code`（如 `admin`/`sales_manager`/`sales`/`customer_service`/`finance`/`customer`/`warehouse`/`quality_inspector`）
   - 权限（Permission）：`permission` 表，含 `permission_code`（如 `color_card.issue.create`/`inventory.stock.view`）
   - 资源（Resource）：业务资源（色卡/订单/库存/缸号/客户等）

2. **角色-权限关联表**：
   ```sql
   -- 角色权限关联表（多对多）
   CREATE TABLE role_permission (
       id BIGSERIAL PRIMARY KEY,
       role_id BIGINT NOT NULL REFERENCES role(id) ON DELETE CASCADE,
       permission_id BIGINT NOT NULL REFERENCES permission(id) ON DELETE CASCADE,
       created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
       UNIQUE(role_id, permission_id)
   );

   -- 用户角色关联表（多对多，支持一个用户多角色）
   CREATE TABLE user_role (
       id BIGSERIAL PRIMARY KEY,
       user_id BIGINT NOT NULL REFERENCES user(id) ON DELETE CASCADE,
       role_id BIGINT NOT NULL REFERENCES role(id) ON DELETE CASCADE,
       assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
       assigned_by BIGINT REFERENCES user(id) ON DELETE SET NULL,
       created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
       UNIQUE(user_id, role_id)
   );
   ```

3. **权限码命名规范**：
   - 格式：`<模块>.<资源>.<操作>`
   - 示例：`color_card.issue.create` / `inventory.stock.view` / `sales.order.approve` / `finance.cost.view`
   - 操作枚举：`view` / `create` / `update` / `delete` / `approve` / `export` / `import`

4. **角色层级设计**（面料行业 ERP 典型角色）：
   | 角色 | role_code | 典型权限 |
   |------|-----------|----------|
   | 超级管理员 | `super_admin` | 所有权限 + 系统配置 |
   | 管理员 | `admin` | 所有业务权限，无系统配置 |
   | 销售经理 | `sales_manager` | 本部门销售管理 + 审批 |
   | 销售 | `sales` | 自己客户 + 订单创建 |
   | 客服 | `customer_service` | 客户服务 + 色卡发放 |
   | 仓库管理员 | `warehouse_manager` | 库存管理 + 出入库审批 |
   | 仓库操作员 | `warehouse` | 出入库操作 |
   | 质检员 | `quality_inspector` | 质检录入 + 报告 |
   | 质检主管 | `quality_manager` | 质检审批 + 降级报废 |
   | 财务 | `finance` | 财务凭证 + 成本核算 |
   | 生产主管 | `production_manager` | 生产订单 + 排产 |
   | 客户（门户） | `customer` | 自己订单 + 色卡批色 |

**扫描方法**：
```bash
# 检查 RBAC 表是否存在
grep -rn "role_permission\|user_role\|role.*permission" backend/src/models/
# 检查权限码命名规范
grep -rn "permission_code\|permission.*create\|permission.*view" backend/src/
# 检查角色定义
grep -rn "role_code\|super_admin\|sales_manager\|customer_service" backend/src/
```

#### 12.2 权限矩阵与最小权限原则

**V15 检查要点**：

1. **权限矩阵完整性**（角色 × 资源 × 操作）：
   - 必须有完整的权限矩阵文档（`docs/rbac-permission-matrix.md`）
   - 矩阵覆盖所有角色 × 所有资源 × 所有操作
   - 每个单元格明确"允许/拒绝/审批后允许"
   - 矩阵与代码实现一致（禁止文档与代码不符）

2. **最小权限原则**（Least Privilege）：
   - 每个角色仅授予完成职责所需的最小权限
   - 销售角色禁止访问财务成本数据
   - 客户角色禁止访问其他客户数据
   - 仓库操作员禁止审批出库（仅主管可审批）

3. **权限继承与互斥**：
   - 角色继承：`sales_manager` 继承 `sales` 的所有权限 + 额外审批权限
   - 权限互斥：`finance` 与 `sales` 不能同时拥有（财务与销售职责分离）
   - 系统校验：用户分配角色时检查互斥规则

4. **权限默认拒绝**（Default Deny）：
   - 未明确授权的资源默认拒绝访问
   - 禁止使用"黑名单"模式（默认允许 + 拒绝列表）
   - 必须使用"白名单"模式（默认拒绝 + 允许列表）

5. **权限粒度控制**：
   - 模块级：`color_card.*`（色卡模块所有操作）
   - 资源级：`color_card.issue.*`（色卡发放所有操作）
   - 操作级：`color_card.issue.create`（仅创建发放记录）
   - 字段级：`color_card.cost.view`（仅查看成本字段）

**扫描方法**：
```bash
# 检查权限矩阵文档
ls -la docs/rbac-permission-matrix.md
# 检查默认拒绝实现
grep -rn "default.*deny\|default_deny\|whitelist\|白名单" backend/src/middleware/
# 检查权限互斥
grep -rn "mutual_exclusive\|互斥\|conflict_role" backend/src/services/
```

#### 12.3 权限校验中间件与后端集成

**V15 检查要点**：

1. **权限校验中间件**（Axum middleware）：
   ```rust
   //! 权限校验中间件（V15 类十二 12.3）
   //! 基于 RBAC 的权限校验，所有业务接口必须经过权限校验

   use axum::{
       extract::Request,
       middleware::Next,
       response::Response,
   };
   use crate::utils::auth::AuthContext;
   use crate::services::permission_service::PermissionService;

   /// 权限校验中间件
   /// 用法：Router::new().route("/color-cards/:id/issue", post(handler).route_layer(require_permission("color_card.issue.create")))
   pub async fn require_permission(
       required_permission: &'static str,
   ) -> impl Fn(AuthContext, Request, Next) -> std::pin::Pin<Box<dyn Future<Output = Response> + Send>> + Clone + Send {
       move |auth: AuthContext, request: Request, next: Next| {
           Box::pin(async move {
               if !auth.has_permission(required_permission) {
                   return Response::builder()
                       .status(StatusCode::FORBIDDEN)
                       .body(Body::from(format!("权限不足：需要 {}", required_permission)))
                       .unwrap();
               }
               next.run(request).await
           })
       }
   }

   /// 管理员绕过权限（仅 super_admin）
   pub fn is_super_admin(auth: &AuthContext) -> bool {
       auth.role_code == "super_admin"
   }
   ```

2. **权限校验注解/宏**：
   - 使用 `define_crud_handlers!` 宏时支持权限参数
   - 示例：`define_crud_handlers!(ColorCard, "color_card");` 自动生成 `color_card.create/view/update/delete` 权限校验

3. **数据权限过滤**（行级权限）：
   - 销售仅能查询自己负责的客户：SQL 自动注入 `WHERE customer_id IN (SELECT customer_id FROM customer_sales_rep WHERE sales_rep_id = ?)`
   - 客户仅能查询自己的订单：SQL 自动注入 `WHERE customer_id = ?`
   - 仓库操作员仅能查询本仓库库存：SQL 自动注入 `WHERE warehouse_id = ?`

4. **字段级权限**：
   - 财务成本字段（`cost_amount`）仅 `finance`/`admin` 可见
   - 客户联系方式仅 `sales`/`customer_service` 可见
   - 实现方式：DTO 序列化时按权限过滤字段

5. **API 权限注解**：
   ```rust
   /// 发放色卡
   /// 权限：color_card.issue.create
   #[require_permission("color_card.issue.create")]
   pub async fn issue_card(...) -> ... { ... }
   ```

**扫描方法**：
```bash
# 检查权限中间件
grep -rn "require_permission\|permission_middleware\|check_permission" backend/src/middleware/
# 检查数据权限过滤
grep -rn "customer_id.*IN.*SELECT\|data_scope\|row_level_permission" backend/src/services/
# 检查字段级权限
grep -rn "skip_serializing_if\|field_permission\|字段级权限" backend/src/models/
# 检查无认证 handler（禁止存在）
grep -rn "pub async fn.*Handler\|pub async fn.*handler" backend/src/handlers/ | grep -v "auth\|login\|health"
```

#### 12.4 前端权限集成

**V15 检查要点**：

1. **路由守卫**（Vue Router）：
   ```typescript
   // router/guards.ts
   router.beforeEach((to, from, next) => {
     const userStore = useUserStore()
     if (to.meta.requiresAuth && !userStore.isLoggedIn) {
       next('/login')
     } else if (to.meta.permission && !userStore.hasPermission(to.meta.permission)) {
       next('/403')
     } else {
       next()
     }
   })
   ```

2. **按钮级权限控制**（v-permission 指令）：
   - 已在 10.6.7 节定义，所有按钮必须使用 `v-permission`
   - 禁止使用 `v-if="userRole === 'admin'"` 硬编码角色判断（应使用权限码）

3. **菜单动态加载**：
   - 后端返回用户权限列表
   - 前端根据权限动态生成菜单（无权限的菜单项不显示）
   - 实现方式：`menu.filter(item => hasPermission(item.permission))`

4. **API 响应 403 处理**：
   - 后端返回 403 时，前端统一拦截并提示"权限不足"
   - 禁止忽略 403 错误

5. **权限码与后端一致**：
   - 前端权限码必须与后端 `permission` 表的 `permission_code` 完全一致
   - 禁止前端自定义权限码（如 `canEdit` 之类的自定义命名）

**扫描方法**：
```bash
# 检查路由守卫
grep -rn "beforeEach\|requiresAuth\|permission.*meta" frontend/src/router/
# 检查 v-permission 指令使用
grep -rn "v-permission" frontend/src/views/
# 检查硬编码角色判断（禁止）
grep -rn "userRole.*===.*admin\|role.*===.*manager" frontend/src/
# 检查 403 处理
grep -rn "403\|权限不足\|Forbidden" frontend/src/utils/request.ts
```

#### 12.5 权限审计日志与追溯

**V15 检查要点**：

1. **权限变更审计**：
   - 角色创建/修改/删除：记录 `role_id` + `role_code` + 变更内容 + 操作人 + 时间
   - 权限分配/撤销：记录 `role_id` + `permission_id` + 变更类型 + 操作人 + 时间
   - 用户角色分配/撤销：记录 `user_id` + `role_id` + 操作人 + 时间

2. **权限校验日志**（可选，敏感操作必记）：
   - 敏感操作（删除/审批/财务凭证）必须记录权限校验结果
   - 权限拒绝时必须记录（`user_id` + `required_permission` + `resource_id` + `ip` + `user_agent`）

3. **审计日志表**：
   ```sql
   CREATE TABLE permission_audit_log (
       id BIGSERIAL PRIMARY KEY,
       user_id BIGINT NOT NULL,
       action VARCHAR(50) NOT NULL,  -- role.create/role.update/permission.assign/user.role.assign
       resource_type VARCHAR(50),
       resource_id BIGINT,
       old_value JSONB,
       new_value JSONB,
       ip_address INET,
       user_agent TEXT,
       created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
   );
   CREATE INDEX idx_permission_audit_user_id ON permission_audit_log(user_id);
   CREATE INDEX idx_permission_audit_action ON permission_audit_log(action);
   CREATE INDEX idx_permission_audit_created_at ON permission_audit_log(created_at);
   ```

4. **审计日志保留期限**：
   - 权限变更日志：保留 3 年以上（法律合规要求）
   - 权限拒绝日志：保留 1 年以上
   - 禁止自动清理权限审计日志

5. **审计日志查询接口**：
   - 管理员可查询所有权限变更记录
   - 按 `user_id`/`action`/`resource_type`/时间范围筛选
   - 支持导出 .xlsx（规则 3）

**扫描方法**：
```bash
# 检查权限审计日志表
grep -rn "permission_audit_log\|permission_audit" backend/src/models/
# 检查权限变更记录
grep -rn "audit.*role\|audit.*permission\|role.*audit" backend/src/services/
# 检查权限拒绝记录
grep -rn "permission.*denied\|权限拒绝\|forbidden.*log" backend/src/middleware/
```

#### 12.6 动态授权与权限委托

**V15 检查要点**：

1. **动态权限分配**：
   - 管理员可动态给角色添加/删除权限（无需重启服务）
   - 权限变更后立即生效（缓存失效机制）
   - 支持 Redis 缓存权限（`role:{role_id}:permissions`）

2. **权限委托**（Delegation）：
   - 销售经理可将部分权限委托给销售（如审批权限临时委托）
   - 委托必须有时限（`valid_from` + `valid_until`）
   - 委托必须记录审计日志
   - 委托不可再委托（禁止链式委托）

3. **权限缓存失效**：
   - 角色权限变更时，清除所有相关用户的权限缓存
   - 用户登出时，清除该用户权限缓存
   - 缓存失效失败时回退到数据库查询

4. **权限热更新**：
   - 权限配置变更后，通过 Redis pub/sub 通知所有服务实例
   - 各实例收到通知后清除本地权限缓存

5. **权限变更审批**（敏感角色）：
   - 分配 `super_admin`/`finance` 角色需双人审批
   - 审批流程：申请人提交 → 审批人审批 → 生效

**扫描方法**：
```bash
# 检查权限缓存
grep -rn "redis.*permission\|permission.*cache\|role.*cache" backend/src/services/
# 检查权限委托
grep -rn "delegation\|delegate\|委托" backend/src/services/
# 检查权限热更新
grep -rn "pub.*sub\|cache.*invalidat\|热更新" backend/src/
```

#### 12.7 数据权限（行级/字段级）

**V15 检查要点**：

1. **行级数据权限**（Row-Level Security）：
   - 销售数据隔离：销售仅能查询自己负责的客户
   - 客户门户隔离：客户仅能查询自己的订单/色卡/批色记录
   - 仓库隔离：仓库操作员仅能查询本仓库库存
   - 部门隔离：销售经理仅能查询本部门销售数据

2. **行级权限实现方式**：
   ```rust
   /// 数据权限过滤器（V15 类十二 12.7）
   /// 根据用户角色自动注入 SQL WHERE 条件
   pub fn apply_data_scope(query: Select, auth: &AuthContext) -> Select {
       match auth.role_code.as_str() {
           "sales" => {
               // 销售仅查询自己负责的客户
               query.filter(
                   Condition::any().add(
                       customer::Column::Id.in_subquery(
                           Query::select()
                               .column(customer_sales_rep::Column::CustomerId)
                               .from(customer_sales_rep::Entity)
                               .and_where(customer_sales_rep::Column::SalesRepId.eq(auth.user_id))
                               .to_owned()
                       )
                   )
               )
           }
           "customer" => {
               // 客户仅查询自己的数据
               query.filter(order::Column::CustomerId.eq(auth.customer_id.unwrap()))
           }
           "warehouse" => {
               // 仓库操作员仅查询本仓库
               query.filter(inventory::Column::WarehouseId.eq(auth.warehouse_id.unwrap()))
           }
           _ => query,  // admin/finance 等角色无数据隔离
       }
   }
   ```

3. **字段级数据权限**（Field-Level Security）：
   - 财务成本字段：`cost_amount`/`unit_cost`/`total_cost` 仅 `finance`/`admin` 可见
   - 客户联系方式：`phone`/`email`/`address` 仅 `sales`/`customer_service` 可见
   - 采购价格：`purchase_price` 仅 `admin`/`finance` 可见

4. **字段级权限实现方式**：
   ```rust
   // DTO 序列化时按权限过滤字段
   #[derive(Serialize)]
   pub struct ProductResponse {
       pub id: i64,
       pub name: String,
       // 成本字段仅财务可见
       #[serde(skip_serializing_if = "Option::is_none")]
       pub cost_amount: Option<Decimal>,
   }

   // handler 中按权限填充
   let mut response: ProductResponse = product.into();
   if auth.has_permission("finance.cost.view") {
       response.cost_amount = Some(product.cost_amount);
   }
   ```

5. **数据权限与业务逻辑结合**：
   - 销售创建订单时，客户下拉框仅显示自己负责的客户
   - 客户门户订单列表，自动过滤 `customer_id = current_customer_id`
   - 仓库操作员库存查询，自动过滤 `warehouse_id = current_warehouse_id`

**扫描方法**：
```bash
# 检查行级权限
grep -rn "apply_data_scope\|data_scope\|row_level" backend/src/services/
# 检查字段级权限
grep -rn "skip_serializing_if\|field_permission\|字段级" backend/src/models/
# 检查销售数据隔离
grep -rn "customer_sales_rep\|sales_rep_id" backend/src/services/
# 检查客户门户隔离
grep -rn "customer_id.*eq.*auth\|WHERE.*customer_id.*=" backend/src/handlers/
```

#### 12.8 RBAC 安全审计与漏洞防护

**V15 检查要点**：

1. **权限提升攻击防护**（Privilege Escalation）：
   - 禁止用户自行修改 `role_id`（即使前端发送 `role_id` 参数，后端必须忽略）
   - 禁止用户通过 API 给自己分配 `super_admin` 角色
   - 角色分配接口仅 `super_admin`/`admin` 可调用

2. **越权访问防护**（IDOR - Insecure Direct Object Reference）：
   - 所有 `/:id` 路由必须校验资源归属
   - 示例：销售查询订单 `/orders/123`，必须校验 `123` 是否属于该销售负责的客户
   - 实现方式：`if !auth.can_access_resource("order", id) { return 403 }`

3. **权限绕过防护**：
   - 禁止通过 `?admin=true` 等参数绕过权限
   - 禁止通过修改 HTTP Method 绕过权限（如 `GET` 改 `POST`）
   - 所有 HTTP Method 都必须经过权限校验

4. **会话固定攻击防护**：
   - 登录后必须重新生成 session ID
   - 权限变更后必须清除旧 session（强制重新登录）

5. **并发权限校验**：
   - 权限校验必须原子化（禁止 TOCTOU 漏洞）
   - 示例：先检查权限再执行操作，期间权限被撤销，操作仍会执行（TOCTOU）
   - 解决方案：在事务内校验权限 + `lock_exclusive`

6. **权限配置审计**：
   - 定期审计权限配置（每月生成权限配置快照）
   - 对比快照发现异常变更（如非工作时间权限变更）
   - 权限配置变更告警（邮件/短信通知管理员）

7. **RBAC 压力测试**：
   - 权限校验性能：单次权限校验 < 10ms（Redis 缓存命中时 < 1ms）
   - 高并发场景：1000 QPS 下权限校验不降级
   - 缓存失效场景：权限缓存全失效时系统仍可用（回退数据库查询）

**扫描方法**：
```bash
# 检查权限提升防护
grep -rn "role_id.*ignore\|role_id.*skip\|禁止.*role_id" backend/src/handlers/user/
# 检查 IDOR 防护
grep -rn "can_access_resource\|resource.*owner\|IDOR" backend/src/middleware/
# 检查权限绕过
grep -rn "admin.*true.*param\|bypass.*permission" backend/src/
# 检查 TOCTOU 防护
grep -rn "lock_exclusive.*permission\|permission.*lock" backend/src/services/
```

---

### 类十三：打印导出审计与权限控制专项（10 维度）⭐ V15 新增（用户 2026-07-15 第六轮反馈）

> **用户原话**：「打印也需要进入审计，那些地方需要进行打印，合不合理？那些地方不许打印，为什么？什么角色不能打印，什么角色不能导出，审计记录全不全等等都需要审计」
>
> **背景调研结论**（2026-07-15 完整代码扫描）：
> - **现有打印/导出端点**：13 个后端 print/export handler + 25+ 个前端本地导出按钮（`window.print`/`exportToExcel`）
> - **业务级审计覆盖**：仅 2 个通用导出 handler（`export_csv`/`export_excel_type`）接入 `OperationType::Export`；其余 11 个 print/export handler 仅被 `omni_audit_middleware` 以通用 `API_CALL` 记录，**无法按"打印/导出"业务语义筛选**
> - **权限模型缺口**：`method_to_action` 把所有 GET 映射为 `read`，**完全缺少 `print`/`export` 专属 action**；任何拥有 read 权限的角色可无限制导出敏感数据
> - **`AuthContext` 缺失**：`dye_batch_handler::export_dye_batches` 与 `dye_recipe_handler::export_dye_recipe_handler::export_dye_recipes` 函数签名无 `AuthContext`，无法关联调用者身份
> - **前端本地导出无审计**：25+ 个页面通过 `exportToExcel`/`printData`/`window.print` 直接生成文件，**完全不触发任何后端 API 与审计**
> - **敏感数据无差异化控制**：染色配方、缸号、色卡、AR 对账单等高敏感数据导出权限与普通列表查询完全相同
> - **未提供导出端点的敏感模块仍有风险**：化验室打样、大货处方、流转卡等模块无导出端点，但列表查询可被分页读取后前端本地导出
>
> **核心目标**：建立"打印/导出全链路审计 + 角色级权限矩阵 + 敏感数据差异化控制 + 前端导出强制走后端"四位一体的打印导出治理体系。

#### 13.1 打印导出端点合理性审计（1 维度）

**审计要点**：逐一核对所有 print/export 端点的"业务必要性、数据敏感性、是否冗余、是否缺失"。

**13.1.1 现有端点合理性矩阵**（逐一评估）

| 模块 | 端点 | 必要性 | 合理性评估 | 处置建议 |
|------|------|--------|-----------|----------|
| 销售订单打印 | `GET /sales/orders/:id/print` | ✅ 必要 | 合理：客户签约/发货单据 | 保留 + 补审计 |
| 销售合同打印 | `GET /sales/sales-contracts/:id/print` | ✅ 必要 | 合理：正式合同 | 保留 + 补审计 + 补专属权限 |
| 采购订单打印 | `GET /purchase/orders/:id/print` | ✅ 必要 | 合理：供应商对账 | 保留 + 补审计 |
| 采购收货单打印 | `GET /purchase/receipts/:id/print` | ✅ 必要 | 合理：入库确认 | 保留 + 补审计 |
| 库存调拨单打印 | `GET /inventory/transfers/:id/print` | ✅ 必要 | 合理：调出/入库凭证 | 保留 + 补审计 |
| 销售订单导出 | `GET /sales/orders/export` | ✅ 必要 | 合理：业务对账 | 保留 + 补审计 + 补专属权限 |
| 采购订单导出 | `GET /purchase/orders/export` | ✅ 必要 | 合理：采购对账 | 保留 + 补审计 + 补专属权限 |
| 缸号导出 | `GET /production/dye-batches/export` | ⚠️ 受控 | **不合理**：缸号含生产计划+配方关联，核心技术机密 | **改为需二级审批** + 补审计 + 补 AuthContext |
| 染色配方导出 | `GET /production/dye-recipes/export` | 🔴 禁止 | **极不合理**：染色配方是印染企业核心技术机密，泄露直接丧失竞争力 | **默认禁止** + 仅允许"配方主管"角色 + 二级审批 + 补审计 + 补 AuthContext |
| MRP 计算结果导出 | `GET /production/mrp-history/:id/export` | ✅ 必要 | 合理：物料需求分析 | 保留 + 补审计 |
| 色卡导出 | `GET /color-cards/export/:id` | ⚠️ 受控 | **不合理**：色卡含 RGB/CMYK/LAB/Pantone/CNCS 色值+客户化色号映射，企业色彩资产 | **改为需二级审批** + 补审计 + 补专属权限 |
| AR 对账单 PDF 导出 | `GET /ar-reconciliations-enhanced/:id/pdf` | ✅ 必要 | 合理：客户对账 | 保留 + 补审计（当前仅 `info!`） |
| 通用 CSV 导出 | `GET /export/csv/:export_type` | ✅ 必要 | 合理：通用导出 | 保留（已审计） |
| 通用 Excel 导出 | `GET /export/excel/:export_type` | ✅ 必要 | 合理：通用导出 | 保留（已审计） |
| 审计日志导出 | `GET /audit-logs/export` | ⚠️ 受控 | **需差异化**：审计日志含敏感操作记录，仅审计员可导出 | 补专属权限（仅 auditor 角色） |

**13.1.2 缺失端点清单**（应补齐或明确禁止）

| 模块 | 当前状态 | 评估 | 处置建议 |
|------|----------|------|----------|
| 化验室打样（lab_dip） | 无导出端点 | **禁止提供**：OK 样配方是染色配方雏形 | 明确禁止 + 前端禁用本地导出按钮 |
| 大货处方（production_recipe） | 无导出端点 | **禁止提供**：生产领料依据，泄露暴露成本结构 | 明确禁止 + 前端禁用本地导出按钮 |
| 流转卡（flow_card） | 无导出端点 | **禁止提供**：条码可被复制滥用 | 明确禁止 + 前端禁用本地导出按钮 |
| 销售报价单（quotations） | 无导出端点 | **应补齐**：客户报价场景需要 | 补齐 + 补审计 + 补专属权限 |
| 验布记录（fabric_inspection） | 无导出端点 | **应补齐**：质检报告交付客户 | 补齐 + 补审计 |
| 产量工资核算 | 无导出端点 | **应补齐**：工资单发放需要 | 补齐 + 补审计 + **仅 HR 角色** |
| 能耗管理报表 | 无导出端点 | **应补齐**：能耗分析需要 | 补齐 + 补审计 |

**13.1.3 前端本地导出（`exportToExcel`/`printData`/`window.print`）合理性审计**

| 页面 | 当前实现 | 合理性 | 处置建议 |
|------|----------|--------|----------|
| 客户列表 | 前端本地导出 | **不合理**：客户信息泄露 | 改走后端 + 补审计 + 补专属权限 |
| 供应商列表 | 前端本地导出 | **不合理**：供应商信息泄露 | 改走后端 + 补审计 |
| 库存列表 | 前端本地导出 | 不合理：库存量泄露 | 改走后端 + 补审计 |
| AP/AR 发票 | 前端本地导出 | **极不合理**：财务数据泄露 | 改走后端 + 补审计 + 补专属权限 |
| 凭证列表 | 前端本地导出 | **极不合理**：凭证泄露 | 改走后端 + 补审计 + 补专属权限 |
| 固定资产 | 前端本地导出 | 不合理 | 改走后端 + 补审计 |
| 预算 | 前端本地导出 | 不合理 | 改走后端 + 补审计 |
| 质量记录 | 前端 `window.print` | 合理：质检报告交付 | 保留但补前端审计埋点 |
| 生产工单 | 前端 `window.print` | 合理：生产单据 | 保留但补前端审计埋点 |
| CRM 客户列表 | 前端 `window.print` | 不合理 | 改走后端 + 补审计 |
| 审计日志列表 | 前端本地导出 | **极不合理**：审计日志二次泄露 | 改走后端 + 补审计 + **仅 auditor 角色** |

**校验命令**：
```bash
# 后端 print/export 端点清单
grep -rn "\.print\|/export" backend/src/routes/ | grep -v "//"
# 前端本地导出调用点
grep -rn "exportToExcel\|printData\|window\.print" frontend/src/
# 审计 OperationType::Export 接入点
grep -rn "OperationType::Export" backend/src/handlers/
```

---

#### 13.2 打印导出角色权限矩阵（1 维度）

**审计要点**：建立"角色 × 操作 × 资源"三维权限矩阵，明确什么角色不能打印、什么角色不能导出。

**13.2.1 角色 × 打印/导出操作权限矩阵**

| 角色 | 销售单据打印 | 销售单据导出 | 采购单据打印 | 采购单据导出 | 库存调拨打印 | 财务凭证打印 | 财务凭证导出 | 染色配方导出 | 缸号导出 | 色卡导出 | 审计日志导出 | AR 对账单导出 | 产量工资导出 |
|------|------------|------------|------------|------------|------------|------------|------------|------------|---------|---------|------------|-------------|------------|
| admin | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ 需二级审批 | ⚠️ 需二级审批 | ⚠️ 需二级审批 | ✅ | ✅ | ✅ |
| sales_manager | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| sales | ✅ 仅本人订单 | ✅ 仅本人订单 | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ 仅本人客户 | ❌ |
| purchase_manager | ❌ | ❌ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| purchase | ❌ | ❌ | ✅ 仅本人订单 | ✅ 仅本人订单 | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| warehouse | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| finance_manager | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| accountant | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ 仅本人客户 | ❌ |
| production_manager | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ⚠️ 需二级审批 | ⚠️ 需二级审批 | ❌ | ❌ | ❌ | ✅ |
| dye_recipe_master（新增） | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ 需二级审批 | ❌ | ❌ | ❌ | ❌ | ❌ |
| lab_technician | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| hr | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ |
| auditor（审计员） | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ | ❌ |
| customer（客户） | ✅ 仅本人订单 | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ 仅本人色卡 | ❌ | ✅ 仅本人对账 | ❌ |

**13.2.2 禁止规则（硬性约束）**

```rust
// 禁止打印的角色清单
const PRINT_DENIED_ROLES: &[&str] = &[
    "purchase",          // 采购员不能打印销售单据
    "warehouse",         // 仓库员不能打印业务单据
    "accountant",        // 会计不能打印业务单据
    "lab_technician",    // 化验员不能打印任何业务单据
    "auditor",           // 审计员不能打印业务单据（仅可导出审计日志）
    "customer",          // 客户不能打印（仅可在线查看）
];

// 禁止导出的角色清单
const EXPORT_DENIED_ROLES: &[&str] = &[
    "sales",             // 销售员不能导出（仅可打印本人订单）
    "purchase",          // 采购员不能导出（仅可打印本人订单）
    "warehouse",         // 仓库员不能导出
    "lab_technician",    // 化验员不能导出任何数据
    "customer",          // 客户不能导出（仅可在线查看 + 色卡/对账单特例）
];

// 禁止导出染色配方的角色（除 dye_recipe_master + 二级审批外全部禁止）
const DYE_RECIPE_EXPORT_DENIED_ROLES: &[&str] = &[
    "admin", "sales_manager", "sales", "purchase_manager", "purchase",
    "warehouse", "finance_manager", "accountant", "production_manager",
    "lab_technician", "hr", "auditor", "customer",
];
```

**13.2.3 权限码命名规范**

```sql
-- 新增 print/export 专属 action 权限码
INSERT INTO permission (code, name, action, resource) VALUES
-- 打印权限
('sales.order.print',           '销售订单打印',     'print', 'sales_order'),
('sales.contract.print',        '销售合同打印',     'print', 'sales_contract'),
('purchase.order.print',        '采购订单打印',     'print', 'purchase_order'),
('purchase.receipt.print',      '采购收货单打印',   'print', 'purchase_receipt'),
('inventory.transfer.print',    '库存调拨单打印',   'print', 'inventory_transfer'),
('finance.voucher.print',      '财务凭证打印',     'print', 'voucher'),
('quality.record.print',        '质检报告打印',     'print', 'quality_record'),
('production.order.print',      '生产工单打印',     'print', 'production_order'),
-- 导出权限
('sales.order.export',          '销售订单导出',     'export', 'sales_order'),
('purchase.order.export',       '采购订单导出',     'export', 'purchase_order'),
('dye.recipe.export',           '染色配方导出',     'export', 'dye_recipe'),
('dye.batch.export',            '缸号导出',         'export', 'dye_batch'),
('color.card.export',           '色卡导出',         'export', 'color_card'),
('ar.reconciliation.export',    'AR对账单导出',     'export', 'ar_reconciliation'),
('audit.log.export',            '审计日志导出',     'export', 'audit_log'),
('salary.export',               '产量工资导出',     'export', 'salary'),
('customer.export',             '客户列表导出',     'export', 'customer'),
('supplier.export',             '供应商列表导出',   'export', 'supplier'),
('inventory.stock.export',      '库存列表导出',     'export', 'inventory_stock');
```

**13.2.4 权限中间件升级**（`method_to_action` 增加 print/export 识别）

```rust
// /workspace/backend/src/middleware/permission.rs 升级
fn method_to_action(method: &Method, path: &str) -> &'static str {
    // 优先识别 print/export 路由后缀
    if path.ends_with("/print") || path.contains("/print/") {
        return "print";
    }
    if path.ends_with("/export") || path.contains("/export/") || path.ends_with("/pdf") {
        return "export";
    }
    // 兼容旧逻辑
    match method {
        &GET => "read",
        &POST => "create",
        &PUT | &PATCH => "update",
        &DELETE => "delete",
        _ => "other",
    }
}
```

---

#### 13.3 打印导出业务级审计补齐（1 维度）

**审计要点**：所有 print/export 操作必须接入 `AuditLogService::record_async` + `OperationType::Export`（或新增 `OperationType::Print`），记录用户、IP、资源、条数、导出范围。

**13.3.1 OperationType 枚举扩展**

```rust
// /workspace/backend/src/models/audit_log.rs 扩展
pub enum OperationType {
    Create, Update, Delete, Login, Logout, Export, Query, Other,
    Print,    // 新增：打印操作
    Download, // 新增：下载操作（模板下载等）
}
```

**13.3.2 print/export handler 审计补齐清单**

| Handler 文件 | 函数 | 当前审计 | 补齐方案 |
|-------------|------|---------|----------|
| [print_handler.rs](file:///workspace/backend/src/handlers/print_handler.rs) | `sales_order_print_html` 等 5 个 | ❌ 无 | 补 `OperationType::Print` 审计，记录资源 ID + 资源类型 |
| [sales_order_handler.rs#L405](file:///workspace/backend/src/handlers/sales_order_handler.rs#L405) | `export_orders` | ❌ 无 | 补 `OperationType::Export` + 导出条数 + 查询条件 |
| [purchase_order_handler.rs#L496](file:///workspace/backend/src/handlers/purchase_order_handler.rs#L496) | `export_orders` | ❌ 无 | 补 `OperationType::Export` + 导出条数 + 查询条件 |
| [dye_batch_handler.rs#L356](file:///workspace/backend/src/handlers/dye_batch_handler.rs#L356) | `export_dye_batches` | ❌ 无 + **缺 AuthContext** | 补 AuthContext 参数 + `OperationType::Export` + 二级审批 token |
| [dye_recipe_handler.rs#L199](file:///workspace/backend/src/handlers/dye_recipe_handler.rs#L199) | `export_dye_recipes` | ❌ 无 + **缺 AuthContext** | 补 AuthContext 参数 + `OperationType::Export` + 二级审批 token |
| [mrp_handler.rs#L304](file:///workspace/backend/src/handlers/mrp_handler.rs#L304) | `export_calculation` | ❌ 无 | 补 `OperationType::Export` |
| [scan_export.rs#L49](file:///workspace/backend/src/handlers/color_card/scan_export.rs#L49) | `export_color_card` | ❌ 无 | 补 `OperationType::Export` + 二级审批 token |
| [ar_reconciliation_handler.rs#L565](file:///workspace/backend/src/handlers/ar_reconciliation_handler.rs#L565) | `export_reconciliation_pdf` | ⚠️ 仅 `info!` | 改为 `OperationType::Export` 落库审计 |
| [import_export_handler.rs](file:///workspace/backend/src/handlers/import_export_handler.rs) | `export_csv`/`export_excel_type` | ✅ 已审计 | 补导出条数字段 |
| [audit_log_handler.rs#L301](file:///workspace/backend/src/handlers/audit_log_handler.rs#L301) | 审计日志导出 | ✅ 已审计 | 补专属权限校验（仅 auditor 角色） |

**13.3.3 审计日志字段补齐**（`audit_logs` 表扩展）

```sql
ALTER TABLE audit_logs ADD COLUMN IF NOT EXISTS
    export_record_count BIGINT,        -- 导出条数
    export_query_filter JSONB,         -- 导出时的查询条件（脱敏后）
    export_file_format VARCHAR(20),   -- 导出文件格式（xlsx/csv/pdf/print）
    export_approval_token UUID,       -- 二级审批 token（敏感数据导出）
    export_watermark_user VARCHAR(100); -- 导出文件水印用户名
```

**13.3.4 审计记录完整性校验矩阵**

| 校验项 | 校验规则 | 失败处置 |
|--------|---------|----------|
| user_id 非空 | 所有 print/export 审计记录必须有 user_id | handler 补 AuthContext |
| resource_type 非空 | 必须记录资源类型（sales_order/dye_recipe 等） | DTO 补字段 |
| resource_id 非空 | 单据打印必须记录 resource_id | handler 补参数 |
| export_record_count | 批量导出必须记录条数 | handler 补统计 |
| ip_address 非空 | 必须记录调用者 IP | 从 request extension 提取 |
| timestamp 精确到毫秒 | 必须记录精确时间 | DB 字段类型 TIMESTAMPTZ |
| 二级审批 token | 敏感数据导出必须有审批 token | handler 补校验 |

---

#### 13.4 敏感数据导出二级审批机制（1 维度）

**审计要点**：染色配方、缸号、色卡、AR 对账单等高敏感数据导出必须经过二级审批。

**13.4.1 敏感数据导出二级审批流程**

```
用户发起导出请求
  ├── 1. handler 校验角色权限（dye_recipe_master 等专属角色）
  ├── 2. handler 生成 export_approval_request（状态 pending）
  ├── 3. 通知审批人（admin 或 production_manager）
  ├── 4. 审批人审批（approve/reject）
  ├── 5. 审批通过 → 生成 export_approval_token（有效期 10 分钟）
  ├── 6. 用户凭 token 调用实际导出接口
  └── 7. 导出接口校验 token + 记录审计 + 生成带水印文件
```

**13.4.2 二级审批数据模型**

```sql
-- 敏感数据导出审批表
CREATE TABLE export_approval_request (
    id BIGSERIAL PRIMARY KEY,
    requester_id BIGINT NOT NULL REFERENCES user(id),
    approver_id BIGINT REFERENCES user(id),
    resource_type VARCHAR(50) NOT NULL,    -- dye_recipe/dye_batch/color_card/ar_reconciliation
    resource_filter JSONB,                  -- 导出范围（脱敏后）
    status VARCHAR(20) NOT NULL DEFAULT 'pending', -- pending/approved/rejected/expired
    approval_token UUID,                    -- 审批通过后生成的 token
    token_expires_at TIMESTAMPTZ,           -- token 有效期（10 分钟）
    requested_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    approved_at TIMESTAMPTZ,
    rejected_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_export_approval_status ON export_approval_request(status, requested_at);
CREATE INDEX idx_export_approval_requester ON export_approval_request(requester_id, requested_at);
```

**13.4.3 敏感数据导出水印**

```rust
// 所有敏感数据导出文件必须包含水印
pub fn add_watermark_to_xlsx(
    xlsx_bytes: Vec<u8>,
    username: &str,
    ip: &str,
    timestamp: chrono::DateTime<chrono::Utc>,
) -> Result<Vec<u8>, AppError> {
    // 在每页页眉添加：用户名 + IP + 时间 + "机密"
    // 在每页页脚添加：导出审批 token 前 8 位
    // 使用 rust_xlsxwriter 的 worksheet.set_header_footer()
}
```

**13.4.4 敏感数据导出禁止规则**

| 资源 | 禁止规则 | 例外 |
|------|---------|------|
| 染色配方（dye_recipe） | 默认禁止所有角色导出 | 仅 `dye_recipe_master` + 二级审批 |
| 化验室 OK 样配方（lab_dip） | **永久禁止导出** | 无例外 |
| 大货处方（production_recipe） | **永久禁止导出** | 无例外 |
| 流转卡条码（flow_card） | **永久禁止导出** | 无例外 |
| 缸号（dye_batch） | 默认禁止 | 仅 `production_manager` + 二级审批 |
| 色卡（color_card） | 默认禁止 | 仅 `dye_recipe_master` + 二级审批，或 customer 角色仅本人色卡 |
| 审计日志（audit_log） | 默认禁止 | 仅 `auditor` 角色 |

---

#### 13.5 前端本地导出强制走后端（1 维度）

**审计要点**：前端 25+ 个页面的本地 `exportToExcel`/`printData`/`window.print` 必须改为调用后端 API，否则无法审计。

**13.5.1 前端导出工具重构方案**

```typescript
// /workspace/frontend/src/utils/export.ts 重构
// 旧实现：纯前端生成文件，无审计
export function exportToExcel(data: any[], filename: string) { /* ... */ }

// 新实现：强制走后端 API
export async function exportToExcel(
    resourceType: string,    // 资源类型（customer/supplier/inventory_stock 等）
    queryFilter: Record<string, any>,  // 查询条件
    fileFormat: 'xlsx' | 'csv' | 'pdf' = 'xlsx',
) {
    // 1. 调用后端 /api/v1/erp/export/excel/:export_type
    const response = await api.get(`/export/excel/${resourceType}`, {
        params: queryFilter,
        responseType: 'blob',
    });
    // 2. 下载文件
    downloadBlob(response.data, `${resourceType}_${Date.now()}.xlsx`);
    // 3. 后端自动记录审计（OperationType::Export）
}

// 旧 printData 函数：纯前端 window.print
export function printData(html: string) { /* ... */ }

// 新实现：调用后端 print API
export async function printData(
    resourceType: string,
    resourceId: bigint,
) {
    // 调用后端 /api/v1/erp/:resource_type/:id/print
    const response = await api.get(`/${resourceType}/${resourceId}/print`);
    // 后端返回 HTML 并自动记录审计（OperationType::Print）
    const printWindow = window.open('', '_blank');
    printWindow?.document.write(response.data);
    printWindow?.print();
}
```

**13.5.2 前端导出按钮 v-permission 指令升级**

```vue
<!-- 旧：所有有 read 权限的角色都能看到导出按钮 -->
<el-button @click="exportToExcel">导出</el-button>

<!-- 新：导出按钮必须有 export 权限 -->
<el-button v-permission="'customer.export'" @click="exportToExcel">导出</el-button>
<el-button v-permission="'sales.order.print'" @click="printOrder">打印</el-button>
```

**13.5.3 前端导出页面改造清单**（25+ 页面）

| 页面 | 当前实现 | 改造方案 | 优先级 |
|------|---------|----------|--------|
| [customer/index.vue](file:///workspace/frontend/src/views/customer/index.vue) | 本地导出 | 走后端 + `customer.export` 权限 | P1 |
| [supplier/index.vue](file:///workspace/frontend/src/views/supplier/index.vue) | 本地导出 | 走后端 + `supplier.export` 权限 | P1 |
| [inventory/index.vue](file:///workspace/frontend/src/views/inventory/index.vue) | 本地导出 | 走后端 + `inventory.stock.export` 权限 | P1 |
| [ap/tabs/InvoiceTab.vue](file:///workspace/frontend/src/views/ap/tabs/InvoiceTab.vue) | 本地导出 | 走后端 + `ap.invoice.export` 权限 | P0 |
| [ar/tabs/InvoiceTab.vue](file:///workspace/frontend/src/views/ar/tabs/InvoiceTab.vue) | 本地导出 | 走后端 + `ar.invoice.export` 权限 | P0 |
| [voucher/tabs/...](file:///workspace/frontend/src/views/voucher/tabs/) | 本地导出 | 走后端 + `finance.voucher.export` 权限 | P0 |
| [system/audit-log/index.vue](file:///workspace/frontend/src/views/system/audit-log/index.vue) | 本地导出 | 走后端 + `audit.log.export` 权限 + **仅 auditor** | P0 |
| 其余 18 个页面 | 本地导出/打印 | 逐一改造 | P2 |

**13.5.4 前端审计埋点（保留 window.print 的场景）**

对于质检报告、生产工单等合理保留 `window.print` 的场景，必须补前端审计埋点：

```typescript
// /workspace/frontend/src/utils/print.ts 补埋点
export async function printData(html: string, resourceType: string, resourceId: bigint) {
    // 先调用后端审计接口
    await api.post('/audit/record', {
        operation_type: 'Print',
        resource_type: resourceType,
        resource_id: resourceId,
    });
    // 再执行打印
    const printWindow = window.open('', '_blank');
    printWindow?.document.write(html);
    printWindow?.print();
}
```

---

#### 13.6 打印导出审计日志完整性审计（1 维度）

**审计要点**：审计记录全不全？逐一核对所有 print/export 端点是否生成完整审计记录。

**13.6.1 审计完整性校验矩阵**

| 端点 | user_id | resource_type | resource_id | export_count | ip_address | timestamp | 二级审批 token | 水印 | 完整性 |
|------|---------|---------------|-------------|---------------|------------|-----------|---------------|------|--------|
| `sales_order_print_html` | ❌ 缺 | ❌ 缺 | ❌ 缺 | N/A | ❌ 缺 | ✅ omni | N/A | N/A | 🔴 不完整 |
| `sales_contract_print_html` | ❌ 缺 | ❌ 缺 | ❌ 缺 | N/A | ❌ 缺 | ✅ omni | N/A | N/A | 🔴 不完整 |
| `purchase_order_print_html` | ❌ 缺 | ❌ 缺 | ❌ 缺 | N/A | ❌ 缺 | ✅ omni | N/A | N/A | 🔴 不完整 |
| `purchase_receipt_print_html` | ❌ 缺 | ❌ 缺 | ❌ 缺 | N/A | ❌ 缺 | ✅ omni | N/A | N/A | 🔴 不完整 |
| `inventory_transfer_print_html` | ❌ 缺 | ❌ 缺 | ❌ 缺 | N/A | ❌ 缺 | ✅ omni | N/A | N/A | 🔴 不完整 |
| `sales_order_handler::export_orders` | ✅ AuthContext | ❌ 缺 | N/A | ❌ 缺 | ❌ 缺 | ✅ omni | N/A | N/A | 🔴 不完整 |
| `purchase_order_handler::export_orders` | ✅ AuthContext | ❌ 缺 | N/A | ❌ 缺 | ❌ 缺 | ✅ omni | N/A | N/A | 🔴 不完整 |
| `dye_batch_handler::export_dye_batches` | ❌ **缺 AuthContext** | ❌ 缺 | N/A | ❌ 缺 | ❌ 缺 | ✅ omni | ❌ 缺 | ❌ 缺 | 🔴 极不完整 |
| `dye_recipe_handler::export_dye_recipes` | ❌ **缺 AuthContext** | ❌ 缺 | N/A | ❌ 缺 | ❌ 缺 | ✅ omni | ❌ 缺 | ❌ 缺 | 🔴 极不完整 |
| `mrp_handler::export_calculation` | ✅ AuthContext | ❌ 缺 | ✅ 路径参数 | N/A | ❌ 缺 | ✅ omni | N/A | N/A | 🟡 部分完整 |
| `color_card::scan_export::export_color_card` | ✅ AuthContext | ❌ 缺 | ✅ 路径参数 | N/A | ❌ 缺 | ✅ omni | ❌ 缺 | ❌ 缺 | 🟡 部分完整 |
| `ar_reconciliation_handler::export_reconciliation_pdf` | ✅ AuthContext | ❌ 缺 | ✅ 路径参数 | N/A | ❌ 缺 | ✅ omni | N/A | N/A | 🟡 部分完整 |
| `export_csv`（通用） | ✅ | ✅ | N/A | ❌ 缺 | ❌ 缺 | ✅ | N/A | N/A | 🟡 部分完整 |
| `export_excel_type`（通用） | ✅ | ✅ | N/A | ❌ 缺 | ❌ 缺 | ✅ | N/A | N/A | 🟡 部分完整 |
| 前端 25+ 本地导出 | ❌ **完全无审计** | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | 🔴 完全无审计 |

**13.6.2 审计完整性修复优先级**

| 优先级 | 修复项 | 涉及端点数 |
|--------|--------|-----------|
| P0 | `dye_batch_handler::export_dye_batches` + `dye_recipe_handler::export_dye_recipes` 补 AuthContext + 补审计 + 二级审批 | 2 |
| P0 | 前端 AP/AR 发票/凭证/审计日志 4 个页面改走后端 + 补审计 | 4 |
| P1 | 5 个 print_html handler 补 `OperationType::Print` 审计 | 5 |
| P1 | 5 个 export handler 补 `OperationType::Export` 审计（sales/purchase/mrp/color_card/ar_reconciliation） | 5 |
| P2 | 通用 export_csv/export_excel_type 补导出条数字段 | 2 |
| P2 | 前端 18 个本地导出页面改走后端 | 18 |

**13.6.3 审计记录补齐代码模板**

```rust
// /workspace/backend/src/handlers/print_handler.rs 补审计模板
pub async fn sales_order_print_html(
    State(state): State<AppState>,
    AuthContext(auth): AuthContext,    // 补 AuthContext
    Path(id): Path<i64>,
) -> Result<Html<String>, AppError> {
    // 1. 权限校验（print 专属权限）
    if !auth.has_permission("sales.order.print") {
        return Err(AppError::Forbidden("无销售订单打印权限".into()));
    }
    // 2. 生成打印 HTML
    let html = PrintService::sales_order_html(&state.db, id).await?;
    // 3. 记录审计日志（业务级）
    AuditLogService::record_async(
        AuditEvent {
            user_id: Some(auth.user_id),
            operation: OperationType::Print,           // 新增枚举
            resource_type: Some("sales_order".into()),
            resource_id: Some(id),
            ip_address: auth.ip.clone(),
            user_agent: auth.user_agent.clone(),
            ..Default::default()
        },
        state.audit_log_service.clone(),
    ).await;
    Ok(Html(html))
}
```

---

#### 13.7 打印导出 omni_audit 中间件语义增强（1 维度）

**审计要点**：`omni_audit_middleware` 当前所有请求统一记录为 `API_CALL`，无法区分 print/export 业务语义。

**13.7.1 omni_audit 中间件增强方案**

```rust
// /workspace/backend/src/middleware/omni_audit.rs 增强
fn classify_operation(method: &Method, path: &str) -> &'static str {
    if path.ends_with("/print") || path.contains("/print/") {
        return "PRINT";
    }
    if path.ends_with("/export") || path.contains("/export/") || path.ends_with("/pdf") {
        return "EXPORT";
    }
    if path.ends_with("/download") || path.contains("/download/") {
        return "DOWNLOAD";
    }
    match method {
        &GET => "READ",
        &POST => "CREATE",
        &PUT | &PATCH => "UPDATE",
        &DELETE => "DELETE",
        _ => "API_CALL",
    }
}
```

**13.7.2 omni_audit_logs 表扩展**

```sql
ALTER TABLE omni_audit_logs ADD COLUMN IF NOT EXISTS
    operation_category VARCHAR(20),    -- PRINT/EXPORT/DOWNLOAD/READ/CREATE/UPDATE/DELETE
    export_record_count BIGINT,        -- 导出条数（仅 EXPORT 类）
    export_approval_token UUID;        -- 二级审批 token（仅敏感 EXPORT）
```

**13.7.3 审计报表分类查询能力**

```sql
-- 查询所有打印操作
SELECT * FROM omni_audit_logs WHERE operation_category = 'PRINT'
ORDER BY created_at DESC;

-- 查询所有导出操作
SELECT * FROM omni_audit_logs WHERE operation_category = 'EXPORT'
ORDER BY created_at DESC;

-- 查询某用户的敏感数据导出记录
SELECT * FROM omni_audit_logs
WHERE operation_category = 'EXPORT'
  AND path LIKE '%dye-recipe%' OR path LIKE '%dye-batch%' OR path LIKE '%color-card%'
  AND user_id = $1
ORDER BY created_at DESC;
```

---

#### 13.8 打印导出文件水印与防泄露（1 维度）

**审计要点**：导出文件必须包含水印（用户名+IP+时间），防止二次泄露后无法追溯。

**13.8.1 水印规范**

| 文件格式 | 水印位置 | 水印内容 | 实现方式 |
|---------|---------|---------|----------|
| xlsx | 页眉 + 页脚 | "机密 - {username} - {ip} - {timestamp}" | `rust_xlsxwriter` `worksheet.set_header_footer()` |
| csv | 文件首行注释 | `# 导出人:{username} 时间:{timestamp} IP:{ip}` | 文件头写入 |
| pdf | 每页背景水印 | "{username} {timestamp}" 半透明大字 | `pdf-writer` 或后处理 |
| 打印 HTML | 页眉 + 页脚 | "{username} - {timestamp}" | HTML `<thead>` + CSS `@media print` |

**13.8.2 水印代码模板**

```rust
// /workspace/backend/src/utils/xlsx_export.rs 增强
pub fn build_xlsx_with_watermark(
    headers: &[&str],
    rows: &[Vec<String>],
    watermark: &Watermark,
) -> Result<Vec<u8>, AppError> {
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    // 写入数据
    worksheet.write_row(0, headers)?;
    for (i, row) in rows.iter().enumerate() {
        worksheet.write_row(i as u16 + 1, row)?;
    }
    // 添加水印（页眉页脚）
    worksheet.set_header(&format!(
        "&L机密&C&\"Arial,Bold\"{}&R{}",
        watermark.username, watermark.timestamp
    ))?;
    worksheet.set_footer(&format!(
        "&LIP: {}&C导出审批: {}&R第 &P 页 / 共 &N 页",
        watermark.ip, watermark.approval_token_short
    ))?;
    Ok(workbook.save_to_buffer()?)
}

pub struct Watermark {
    pub username: String,
    pub ip: String,
    pub timestamp: String,
    pub approval_token_short: String,  // 审批 token 前 8 位
}
```

---

#### 13.9 打印导出性能与并发控制（1 维度）

**审计要点**：大量数据导出可能拖垮数据库与内存，必须有并发控制与条数上限。

**13.9.1 导出条数上限**

| 资源类型 | 单次导出上限 | 超限处置 |
|---------|------------|----------|
| 销售订单 | 10000 条 | 返回错误"超过导出上限，请缩小查询范围" |
| 采购订单 | 10000 条 | 同上 |
| 库存记录 | 50000 条 | 同上 |
| 客户/供应商 | 5000 条 | 同上 |
| 凭证 | 20000 条 | 同上 |
| 染色配方 | **禁止批量导出**（仅单条 + 二级审批） | 强制单条 |
| 缸号 | 5000 条 + 二级审批 | 同上 |
| 色卡 | 1000 条 + 二级审批 | 同上 |
| 审计日志 | 100000 条（仅 auditor） | 同上 |

**13.9.2 并发控制**

```rust
// /workspace/backend/src/handlers/import_export_handler.rs 增强
use std::sync::atomic::{AtomicUsize, Ordering};

static CONCURRENT_EXPORTS: AtomicUsize = AtomicUsize::new(0);
const MAX_CONCURRENT_EXPORTS: usize = 10;  // 全局最大并发导出数

pub async fn export_excel_type(
    State(state): State<AppState>,
    AuthContext(auth): AuthContext,
    Path(export_type): Path<String>,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, AppError> {
    // 1. 并发数控制
    let current = CONCURRENT_EXPORTS.fetch_add(1, Ordering::SeqCst);
    if current >= MAX_CONCURRENT_EXPORTS {
        CONCURRENT_EXPORTS.fetch_sub(1, Ordering::SeqCst);
        return Err(AppError::TooManyRequests("导出并发数已达上限，请稍后重试".into()));
    }
    // 2. 执行导出（用 scopeguard 确保计数器递减）
    let result = do_export(&state, &auth, &export_type, &query).await;
    CONCURRENT_EXPORTS.fetch_sub(1, Ordering::SeqCst);
    result
}
```

**13.9.3 大数据量导出流式处理**

```rust
// 超过 5000 条的数据必须流式导出，避免内存爆炸
pub async fn export_large_data(
    State(state): State<AppState>,
    AuthContext(auth): AuthContext,
    Path(export_type): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let stream = export_service::stream_export(&state.db, &export_type, &auth)
        .await?;
    // 使用 axum 的 StreamBody 流式返回
    Ok(Response::builder()
        .header("Content-Type", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet")
        .header("Content-Disposition", format!("attachment; filename=\"{}.xlsx\"", export_type))
        .body(StreamBody::new(stream))?)
}
```

---

#### 13.10 打印导出合规审计与定期审查（1 维度）

**审计要点**：建立打印导出操作的定期合规审查机制，识别异常导出行为。

**13.10.1 异常导出行为识别规则**

| 异常模式 | 检测规则 | 处置 |
|---------|---------|------|
| 高频导出 | 单用户 1 小时内导出 > 10 次 | 告警 + 临时锁定导出权限 30 分钟 |
| 大批量导出 | 单次导出 > 上限 80% | 告警 + 记录到安全审计 |
| 非工作时间导出 | 22:00-06:00 导出敏感数据 | 告警 + 二级审批强制 |
| 离职用户导出 | 用户状态 disabled 后仍有导出记录 | 严重告警 + 安全事件调查 |
| 跨权限导出 | 用户尝试导出无权限资源 | 告警 + 记录到安全审计 + 临时锁定 |
| 敏感数据无审批导出 | 染色配方/缸号导出无审批 token | 严重告警 + 安全事件调查 |

**13.10.2 定期合规审查机制**

```rust
// 新增定时任务：每日 02:00 审查前一天所有 print/export 操作
// cron: 0 2 * * *
pub async fn daily_export_compliance_review(state: AppState) {
    let yesterday = chrono::Utc::now().date_naive() - chrono::Duration::days(1);
    let exports = audit_log_service::query_exports_by_date(&state.db, yesterday).await?;
    for export in exports {
        // 1. 校验审计完整性
        if export.user_id.is_none() || export.resource_type.is_none() {
            security_alert("导出审计记录不完整", &export);
        }
        // 2. 校验权限合规性
        if !permission_service::had_permission(export.user_id, &format!("{}.export", export.resource_type)).await? {
            security_alert("越权导出", &export);
        }
        // 3. 校验敏感数据审批
        if is_sensitive_resource(&export.resource_type) && export.approval_token.is_none() {
            security_alert("敏感数据导出无审批", &export);
        }
        // 4. 生成日报
    }
    report_service::generate_daily_export_compliance_report(&state.db, yesterday).await?;
}
```

**13.10.3 审计日志保留期限**

| 操作类型 | 保留期限 | 存储位置 |
|---------|---------|----------|
| 普通 print/export 审计 | 3 年 | `audit_logs` 表 |
| 敏感数据 print/export 审计 | 7 年 | `audit_logs` 表（标记 `is_sensitive=true`） |
| omni_audit 全量记录 | 1 年 | `omni_audit_logs` 表（定期归档） |
| 安全告警记录 | 7 年 | `security_alert_log` 表（新增） |

**13.10.4 审计日志导出二次审计**

> **特殊规则**：审计日志本身的导出（`audit_log_handler::export`）必须被二次审计，防止审计员篡改审计记录。

```sql
-- 审计日志导出操作记录到独立表
CREATE TABLE audit_log_export_log (
    id BIGSERIAL PRIMARY KEY,
    auditor_id BIGINT NOT NULL,         -- 导出审计员
    query_filter JSONB,                 -- 导出时的查询条件
    export_record_count BIGINT,         -- 导出条数
    export_file_hash VARCHAR(64),       -- 导出文件 SHA256（用于完整性校验）
    approval_token UUID,                -- 二级审批 token（CEO 或 admin 审批）
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

#### 类十三审计执行命令汇总

```bash
# 1. 后端 print/export 端点清单
grep -rn "\.print\|/export\|/pdf" backend/src/routes/ | grep -v "//"
# 2. 前端本地导出调用点
grep -rn "exportToExcel\|printData\|window\.print" frontend/src/
# 3. 业务级审计接入点
grep -rn "OperationType::Export\|OperationType::Print" backend/src/handlers/
# 4. 缺失 AuthContext 的 export handler
grep -rn "export_" backend/src/handlers/ | grep -v "AuthContext"
# 5. 权限模型 print/export 维度
grep -rn "print\|export" backend/database/init_admin_permissions.sql
# 6. 权限中间件 method_to_action
grep -rn "method_to_action\|fn.*action" backend/src/middleware/permission.rs
# 7. omni_audit 中间件分类
grep -rn "API_CALL\|operation_category" backend/src/middleware/omni_audit.rs
# 8. 审计日志表结构
grep -rn "export_record_count\|export_approval_token" backend/src/models/audit_log.rs
# 9. 二级审批机制
grep -rn "export_approval\|approval_token" backend/src/
# 10. 水印实现
grep -rn "watermark\|set_header\|set_footer" backend/src/utils/xlsx_export.rs
```

---

### 类十四：权限维度审计与角色合理性专项（12 维度）⭐ V15 新增（用户 2026-07-15 第七轮反馈）

> **用户原话**：「添加权限维度的审计，那些角色应该有什么权限，那些权限不合理，那些角色不合理，那些角色权限不匹配？为什么？」
>
> **背景调研结论**（2026-07-15 完整代码 + DB schema 扫描）：
> - **预定义角色仅 3 个**（admin/manager/operator），但 014_init_role_permissions.sql 引用 role_id=4/5（采购经理/财务经理）**角色根本不存在**，所有 `WHERE EXISTS` 条件静默失败
> - **manager/operator 被设为 `is_system=true`**：登录后注入 `*:*` 超级通配权限，**operator 实际等同于 admin**，权限模型形同虚设
> - **业务专用角色完全缺失**：无销售经理/销售员/采购经理/采购员/仓库员/生产经理/染色操作员/化验室员/色卡管理员/财务会计/财务审核/HR/审计员/普通员工等 14 类业务角色
> - **权限资源覆盖严重不足**：admin 权限文件仅定义 11 类资源权限，路由暴露 70+ 类资源，**缺口约 60 类**，非 admin 用户在 60+ 类资源上无法通过权限检查
> - **模块前缀白名单滞后**：path_utils.rs 仅 28 个白名单，无法覆盖新增的 production/crm/analytics 等模块下的大量资源
> - **职责冲突**：财务经理期望同时 finance.create/update/delete（开凭证+改凭证+删凭证，无制衡）；admin 既是操作者又是审计者（违反职责分离）
> - **前后端权限边界不一致**：前端依赖 `*:*` 放行 manager/operator，后端 `is_admin_role` 只承认 role.code=="admin"，造成前端可见菜单但后端返回 403
> - **schema 与代码演进不同步**：001 schema 旧字段（permission_code），025 后补丁拆分为 resource_type/action，014 引用不存在的 role_id——三处独立维护缺乏一致性保障
>
> **核心目标**：建立"角色清单合理性审计 + 权限分配矩阵审计 + 职责分离 SoD 审计 + 权限-路由匹配审计 + is_system 滥用治理 + 业务角色补齐"六位一体的权限维度治理体系。

#### 14.1 角色清单合理性审计（1 维度）

**审计要点**：逐一核对所有预定义角色的"业务必要性、职责清晰度、是否冗余、是否缺失"。

**14.1.1 现有角色合理性矩阵**

| role_id | code | name | is_system | 合理性评估 | 处置建议 |
|---------|------|------|-----------|-----------|----------|
| 1 | `admin` | 系统管理员 | true | ✅ 合理：系统管理 | 保留，但拆分审计职责（见 14.3） |
| 2 | `manager` | 部门经理 | true | 🔴 **不合理**：is_system=true 导致持有 `*:*`，等同 admin；"部门经理"语义模糊，无具体业务职责 | **改为 is_system=false** + 重新定义权限范围 OR 删除并拆分为业务经理角色 |
| 3 | `operator` | 操作员 | true | 🔴 **极不合理**：is_system=true 导致持有 `*:*`；"操作员"语义模糊，014 期望分配 sales.create/update 但 operator 实际等同 admin | **改为 is_system=false** + 删除 OR 拆分为具体业务操作员角色 |
| 4（014 期望） | `purchase_manager` | 采购经理 | N/A | 🔴 **不存在**：014 引用但 schema/init_service 未创建 | 补齐角色定义（见 14.1.2） |
| 5（014 期望） | `finance_manager` | 财务经理 | N/A | 🔴 **不存在**：014 引用但 schema/init_service 未创建 | 补齐角色定义（见 14.1.2） |

**14.1.2 缺失业务角色补齐清单**（基于面料行业 ERP 业务场景）

| 角色 code | 名称 | 职责 | is_system | 应有权限范围 |
|-----------|------|------|-----------|------------|
| `sales_manager` | 销售经理 | 销售订单审批、客户管理、销售合同 | false | sales:R/C/U, customers:R/C/U, sales-contracts:R/C/U, sales-prices:R, sales-returns:R/C/U, ar-reconciliations:R |
| `sales` | 销售员 | 创建销售订单、维护本人客户 | false | sales:R/C（仅本人）, customers:R/C（仅本人）, sales-prices:R |
| `purchase_manager` | 采购经理 | 采购订单审批、供应商管理 | false | purchases:R/C/U, suppliers:R/C/U, purchase-contracts:R/C/U, purchase-prices:R, purchase-returns:R/C/U |
| `purchase` | 采购员 | 创建采购订单、查询供应商 | false | purchases:R/C（仅本人）, suppliers:R |
| `warehouse_manager` | 仓库经理 | 库存管理审批、盘点审批 | false | inventory:R/C/U/D, warehouses:R/C/U, transfers:R/C/U, counts:R/C/U, adjustments:R/C/U |
| `warehouse` | 仓库员 | 入库出库、库存调整 | false | inventory:R/C/U, warehouses:R, transfers:R/C, counts:R/C, stock:R/C/U |
| `production_manager` | 生产经理 | 生产计划、染色审批 | false | production:R/C/U, dye-batches:R/C/U, production-orders:R/C/U, mrp:R/C/U, capacity:R/C/U, cost-collections:R |
| `dye_operator` | 染色操作员 | 缸号管理、染色配方查看 | false | dye-batches:R/C, dye-recipes:R（只读）, flow-cards:R/C, production-orders:R |
| `lab_technician` | 化验室员 | 化验室打样、配方管理 | false | lab-dip:R/C/U, dye-recipes:R/C/U, quality-inspection:R/C, color-cards:R |
| `color_card_manager` | 色卡管理员 | 色卡发放、色卡库存 | false | color-cards:R/C/U/D, color-prices:R, inventory:R（色卡库存） |
| `finance_accountant` | 财务会计 | 凭证录入、AR/AP 发票 | false | finance:R/C, gl:R/C, vouchers:R/C, ap:R/C, ar:R/C, currencies:R, exchange-rates:R |
| `finance_reviewer` | 财务审核 | 凭证审核、期末结账 | false | finance:R/U（仅审核，不创建）, gl:R/U, vouchers:R/U, accounting-periods:R/U |
| `hr` | 人事专员 | 用户管理、部门管理、产量工资 | false | users:R/C/U, departments:R/C/U, salary:R/C/U, attendance:R/C/U |
| `auditor` | 审计员 | 审计日志查看、合规审查（**只读**） | false | audit:R, audit-logs:R, slow-queries:R, omni-audit:R, business-trace:R, security:R（**无任何 C/U/D**） |
| `customer` | 客户（外部） | 查看本人订单/色卡/对账单 | false | sales:R（仅本人）, color-cards:R（仅本人）, ar-reconciliations:R（仅本人） |
| `employee` | 普通员工 | 仪表板、个人信息 | false | dashboard:R, profile:R/U |

**14.1.3 角色命名规范**

```rust
// 角色码命名规范：{业务域}_{职责}
// 业务域：sales/purchase/inventory/production/finance/hr/audit/system
// 职责：manager/staff/operator/reviewer/auditor/external
const ROLE_CODE_PATTERN: &str = r"^(sales|purchase|inventory|production|finance|hr|audit|system)_(manager|staff|operator|reviewer|auditor|external)$";
// 例：sales_manager / sales_staff / finance_reviewer / audit_auditor
```

**校验命令**：
```bash
# 查询数据库现有角色
psql -c "SELECT id, code, name, is_system, description FROM roles ORDER BY id;"
# 检查 014 引用但不存在的 role_id
psql -c "SELECT DISTINCT role_id FROM role_permissions WHERE role_id NOT IN (SELECT id FROM roles);"
# 检查 is_system=true 的角色（应仅 admin）
psql -c "SELECT id, code FROM roles WHERE is_system = true;"
```

---

#### 14.2 权限分配矩阵审计（1 维度）

**审计要点**：逐一核对"角色 × 资源 × 操作"三维权限矩阵的合理性，识别权限过大、权限过小、权限不匹配。

**14.2.1 现有权限分配问题矩阵**

| 角色 | 资源 | 当前权限 | 问题类型 | 不合理原因 | 修复方案 |
|------|------|---------|---------|-----------|----------|
| manager | 所有资源 | `*:*`（因 is_system=true） | 🔴 权限过大 | "部门经理"持有超级权限，无业务边界 | 改 is_system=false + 按业务域分配 |
| operator | 所有资源 | `*:*`（因 is_system=true） | 🔴 权限过大 | "操作员"持有超级权限，语义模糊 | 改 is_system=false + 拆分为具体业务角色 |
| manager（014 期望） | sales/purchases/inventory 等 | 仅 7 个 read | 🟡 权限过小 | 无 audit:read/dashboard:read/users:read，前端菜单不可见 | 补齐 read 权限 + 按业务域扩展 |
| operator（014 期望） | sales/customers | sales.create/update + customers.create | 🟡 权限过小 | 无任何 read 权限，前端路由不可见 | 补齐对应 read 权限 |
| role_id=4（014 期望） | purchases | purchases.create/update + suppliers.create | 🔴 角色不存在 | 014 引用但 schema 未创建 | 补齐 purchase_manager 角色定义 |
| role_id=5（014 期望） | finance | finance.create/update/delete | 🔴 角色不存在 + 职责冲突 | 014 引用但 schema 未创建；即使创建，C/U/D 集于一身无制衡 | 补齐 finance_accountant + finance_reviewer 双角色制衡 |
| admin | audit | audit:read | 🟡 职责冲突 | admin 既是操作者又能审计自己 | 审计职责独立到 auditor 角色 |
| admin | 所有资源 | `*:*` + is_admin_role 绕过 | 🟡 权限过大 | admin 无任何限制，违反最小权限原则 | 拆分超级管理员（system_admin）与业务管理员（business_admin） |

**14.2.2 目标权限分配矩阵**（14 业务角色 × 11 核心资源）

| 角色 | sales | purchases | inventory | finance | customers | suppliers | products | warehouses | users | audit | dashboard |
|------|-------|-----------|-----------|---------|-----------|-----------|----------|-----------|-------|-------|-----------|
| admin | R/C/U/D | R/C/U/D | R/C/U/D | R/C/U/D | R/C/U/D | R/C/U/D | R/C/U/D | R/C/U/D | R/C/U/D | R | R |
| sales_manager | R/C/U | R | R | R | R/C/U | R | R | R | - | - | R |
| sales | R/C（本人） | - | R | - | R/C（本人） | - | R | - | - | - | R |
| purchase_manager | R | R/C/U | R | R | R | R/C/U | R | R | - | - | R |
| purchase | R | R/C（本人） | R | - | - | R | R | - | - | - | R |
| warehouse_manager | R | R | R/C/U/D | R | R | R | R | R/C/U | - | - | R |
| warehouse | - | R | R/C/U | - | - | - | R | R | - | - | R |
| production_manager | R | R | R/C/U | R | R | R | R/C/U | R | - | - | R |
| dye_operator | - | - | R/C/U | - | - | - | R | R | - | - | R |
| lab_technician | - | - | R | - | - | - | R | - | - | - | R |
| color_card_manager | R（色卡订单） | - | R/C/U（色卡库存） | - | R | - | R | R | - | - | R |
| finance_accountant | R | R | R | R/C | R | R | R | R | - | - | R |
| finance_reviewer | R | R | R | R/U（审核） | R | R | R | R | - | - | R |
| hr | - | - | - | - | - | - | - | - | R/C/U | - | R |
| auditor | R | R | R | R | R | R | R | R | R | R | R |
| customer | R（本人） | - | - | - | R（本人） | - | R | - | - | - | R |
| employee | - | - | - | - | - | - | - | - | R/U（本人 profile） | - | R |

**14.2.3 权限过大识别规则**

```rust
// 权限过大检测规则
pub fn detect_over_permission(role: &Role, permissions: &[Permission]) -> Vec<OverPermissionIssue> {
    let mut issues = Vec::new();
    // 1. is_system=true 但 code != "admin" → 权限过大
    if role.is_system && role.code != "admin" {
        issues.push(OverPermissionIssue {
            role_code: role.code.clone(),
            issue: "is_system=true 但非 admin 角色，持有 *:* 超级权限".into(),
            severity: "P0".into(),
        });
    }
    // 2. 非 admin 角色持有 *:* 通配权限
    if role.code != "admin" && permissions.iter().any(|p| p.resource_type == "*" && p.action == "*") {
        issues.push(OverPermissionIssue {
            role_code: role.code.clone(),
            issue: "非 admin 角色持有 *:* 超级通配权限".into(),
            severity: "P0".into(),
        });
    }
    // 3. 业务角色持有跨业务域权限（如销售角色持有 finance:C/U/D）
    // 4. 只读角色（auditor）持有 C/U/D 权限
    if role.code == "auditor" {
        for p in permissions {
            if matches!(p.action.as_str(), "create" | "update" | "delete") {
                issues.push(OverPermissionIssue {
                    role_code: role.code.clone(),
                    issue: format!("审计员角色不应持有 {}:{} 写权限", p.resource_type, p.action),
                    severity: "P0".into(),
                });
            }
        }
    }
    issues
}
```

**14.2.4 权限过小识别规则**

```rust
// 权限过小检测规则
pub fn detect_under_permission(role: &Role, permissions: &[Permission]) -> Vec<UnderPermissionIssue> {
    let mut issues = Vec::new();
    // 1. 业务角色无对应 read 权限（前端路由不可见）
    let business_resources = get_role_business_resources(&role.code);
    for resource in business_resources {
        if !permissions.iter().any(|p| p.resource_type == resource && p.action == "read") {
            issues.push(UnderPermissionIssue {
                role_code: role.code.clone(),
                issue: format!("业务角色缺少 {}:read 权限，前端路由不可见", resource),
                severity: "P1".into(),
            });
        }
    }
    // 2. 业务角色无 dashboard:read（无法进入系统）
    if !permissions.iter().any(|p| p.resource_type == "dashboard" && p.action == "read")
        && role.code != "admin" {
        issues.push(UnderPermissionIssue {
            role_code: role.code.clone(),
            issue: "缺少 dashboard:read，无法进入系统仪表板".into(),
            severity: "P1".into(),
        });
    }
    issues
}
```

---

#### 14.3 职责分离（SoD）审计（1 维度）

**审计要点**：检查角色权限分配是否违反职责分离原则（Segregation of Duties），识别"既当运动员又当裁判"的冲突。

**14.3.1 职责冲突矩阵**

| 冲突场景 | 冲突角色 | 冲突权限 | 冲突原因 | 修复方案 |
|---------|---------|---------|---------|----------|
| 开凭证 + 审核凭证 | finance_accountant + finance_reviewer（若合并为 finance_manager） | finance:C + finance:U | 会计开凭证后自己审核，无制衡 | 拆分为 finance_accountant（仅 C）+ finance_reviewer（仅 U 审核他人凭证） |
| 开凭证 + 删凭证 | finance_manager（014 期望 role_id=5） | finance:C + finance:D | 会计开凭证后可删除，无审计痕迹 | finance_accountant 仅 C，删除需 finance_reviewer 审批 |
| 创建采购 + 审批采购 | purchase_manager | purchases:C + purchases:approve | 采购经理自审自批 | 拆分 purchase_staff（C）+ purchase_manager（approve） |
| 创建销售 + 审批销售 | sales_manager | sales:C + sales:approve | 销售经理自审自批 | 拆分 sales_staff（C）+ sales_manager（approve） |
| 操作数据 + 审计操作 | admin | 所有 C/U/D + audit:R | admin 既能操作又能审计自己 | 审计职责独立到 auditor 角色（仅 R） |
| 用户管理 + 角色分配 | admin | users:C/U + roles:C/U | admin 既能创建用户又能分配角色 | 拆分 user_admin（用户 CRUD）+ security_admin（角色权限分配） |
| 库存调整 + 库存盘点 | warehouse_manager | inventory:adjust + inventory:count | 仓库经理既调整库存又盘点，可掩盖差异 | 拆分 warehouse_staff（adjust）+ warehouse_auditor（count） |
| 染色配方创建 + 染色配方审批 | lab_technician + production_manager | dye-recipes:C + dye-recipes:approve | 化验员创建配方后生产经理审批，但若同一人兼任两职则冲突 | 角色互斥校验（见 14.3.2） |

**14.3.2 角色互斥规则**

```sql
-- 角色互斥表（同一用户不能同时拥有互斥角色）
CREATE TABLE role_conflict (
    id BIGSERIAL PRIMARY KEY,
    role_a_code VARCHAR(50) NOT NULL,    -- 角色 A
    role_b_code VARCHAR(50) NOT NULL,    -- 角色 B（与 A 互斥）
    conflict_reason TEXT NOT NULL,       -- 冲突原因
    severity VARCHAR(10) NOT NULL,       -- P0/P1/P2
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(role_a_code, role_b_code)
);

-- 插入互斥规则
INSERT INTO role_conflict (role_a_code, role_b_code, conflict_reason, severity) VALUES
('finance_accountant', 'finance_reviewer', '会计开凭证不能同时审核凭证', 'P0'),
('finance_accountant', 'auditor', '会计操作不能同时审计', 'P0'),
('purchase_staff', 'purchase_manager', '采购员创建不能同时审批', 'P0'),
('sales_staff', 'sales_manager', '销售员创建不能同时审批', 'P0'),
('warehouse_staff', 'warehouse_auditor', '仓库调整不能同时盘点', 'P0'),
('lab_technician', 'production_manager', '化验室创建配方不能同时审批', 'P1'),
('hr', 'auditor', '人事管理不能同时审计', 'P1'),
('admin', 'auditor', '系统管理不能同时审计', 'P0');
```

**14.3.3 互斥校验代码**

```rust
// /workspace/backend/src/services/user_service.rs 增强
pub async fn assign_role_to_user(
    db: &DatabaseConnection,
    user_id: i64,
    new_role_id: i64,
) -> Result<(), AppError> {
    // 1. 查询用户现有角色
    let existing_roles = user_role::Entity::find()
        .filter(user_role::Column::UserId.eq(user_id))
        .find_also_related(role::Entity)
        .all(db).await?;
    // 2. 查询新角色
    let new_role = role::Entity::find_by_id(new_role_id).one(db).await?
        .ok_or(AppError::NotFound("角色不存在".into()))?;
    // 3. 互斥校验
    for (_, existing_role) in existing_roles {
        if let Some(existing) = existing_role {
            let conflict = role_conflict::Entity::find()
                .filter(
                    Condition::any()
                        .add(role_conflict::Column::RoleACode.eq(&existing.code))
                        .add(role_conflict::Column::RoleACode.eq(&new_role.code))
                )
                .one(db).await?;
            if let Some(c) = conflict {
                return Err(AppError::Conflict(format!(
                    "角色互斥冲突：{} 与 {} 不能同时拥有（原因：{}）",
                    existing.code, new_role.code, c.conflict_reason
                )));
            }
        }
    }
    // 4. 分配角色
    user_role::ActiveModel { user_id: Set(user_id), role_id: Set(new_role_id), ..Default::default() }
        .insert(db).await?;
    Ok(())
}
```

**14.3.4 职责分离审计校验命令**

```bash
# 检查现有用户是否违反互斥规则
psql -c "
SELECT u.username, r1.code AS role_a, r2.code AS role_b, rc.conflict_reason
FROM users u
JOIN user_role ur1 ON u.id = ur1.user_id
JOIN roles r1 ON ur1.role_id = r1.id
JOIN user_role ur2 ON u.id = ur2.user_id
JOIN roles r2 ON ur2.role_id = r2.id
JOIN role_conflict rc ON
  (rc.role_a_code = r1.code AND rc.role_b_code = r2.code)
  OR (rc.role_a_code = r2.code AND rc.role_b_code = r1.code)
WHERE r1.id < r2.id;"
```

---

#### 14.4 权限-路由匹配审计（1 维度）

**审计要点**：核对"权限定义 × 路由暴露"的双向匹配，识别"有路由无权限"和"有权限无路由"两类缺口。

**14.4.1 有路由无权限缺口**（60+ 类资源缺失权限定义）

| 业务域 | 缺失权限的资源类型 | 影响范围 | 优先级 |
|--------|------------------|----------|--------|
| 销售域 | orders, sales-contracts, sales-prices, sales-returns, fabric-orders | 非 admin 用户无法访问销售订单/合同/价格/退货 | P0 |
| 采购域 | orders（采购）, receipts, inspections, returns, purchase-contracts, purchase-prices, supplier-evaluations | 非 admin 用户无法访问采购单/收货/质检/退货 | P0 |
| 库存域 | stock, transfers, adjustments, reservations, counts, batches, logistics | 非 admin 用户无法访问库存/调拨/调整/盘点 | P0 |
| 财务域 | gl, fixed-assets, budgets, financial-analysis, fund-management, ap, ar, ar-reconciliations, currencies, exchange-rates | 非 admin 用户无法访问总账/资产/预算/AP/AR | P0 |
| 生产域 | dye-batches, greige-fabrics, dye-recipes, lab-dip, production-recipes, flow-cards, fabric-inspections, quality-inspection, cost-collections, production-orders, mrp, mrp-history, capacity | 非 admin 用户无法访问缸号/配方/打样/处方/流转卡/质检/成本/MRP | P0 |
| CRM 域 | customer-credits, five-dimension, sales-analysis, crm-customers, crm-tags, crm-pool, crm-assignments, crm-sales-users, crm-recycle-rules, crm-business | 非 admin 用户无法访问客户信用/CRM | P1 |
| IAM 域 | roles, departments, permissions, field-permissions | 非 admin 用户无法访问角色/部门/权限管理（但 admin 有 require_admin_role 二次校验） | P1 |
| 分析域 | dual-unit, assist-accounting, business-trace, scanner, reports-enhanced, imports, exports, security, emails, ai, reports, webhooks, api-gateway, data-permissions, notifications, advanced, bi, tracking | 非 admin 用户无法访问分析/报表/导入导出 | P1 |
| 系统域 | ws, system-update, bpm, audit-logs, slow-queries, init | 非 admin 用户无法访问 WebSocket/系统更新/BPM/审计日志 | P1 |

**14.4.2 有权限无路由冗余**

经核对，init_admin_permissions.sql 中定义的 11 类资源（purchases/sales/inventory/finance/customers/suppliers/products/warehouses/users/audit/dashboard）**全部有对应路由**，无冗余权限。

**但存在"权限码与路由资源类型不匹配"问题**：

| 权限码 | 路由实际资源类型 | 不匹配原因 | 修复方案 |
|--------|---------------|-----------|----------|
| `sales:read` | `orders`（/sales/orders） | 权限码用业务域 `sales`，路由资源类型用 `orders`，extract_resource_info 提取 `orders` 但权限表无 `orders` | 补齐 `orders:*` 权限 OR 修改 extract_resource_info 支持"业务域前缀映射" |
| `purchases:read` | `orders`（/purchases/orders） | 同上 | 同上 |
| `inventory:read` | `stock`/`transfers`/`counts` 等 | 权限码用 `inventory`，路由资源类型用具体子资源 | 补齐子资源权限 OR 修改校验支持业务域通配 |
| `finance:read` | `gl`/`ap`/`ar`/`vouchers` 等 | 同上 | 同上 |

**14.4.3 模块前缀白名单缺口**

| 路由模块 | 是否在 path_utils 白名单 | 影响 |
|---------|------------------------|------|
| iam | ❌ 缺失 | users/roles/departments 资源类型提取错误 |
| catalog | ❌ 缺失 | products/categories/warehouses/boms 提取错误 |
| production | ❌ 缺失 | dye-batches/dye-recipes/lab-dip 等提取错误 |
| analytics | ❌ 缺失 | 18 个子资源提取错误 |
| system | ❌ 缺失 | ws/bpm/audit-logs 等提取错误 |
| custom-orders | ❌ 缺失 | |
| color-cards | ❌ 缺失 | |

**修复方案**：补齐 path_utils.rs 模块前缀白名单至覆盖所有 70+ 资源。

**14.4.4 权限-路由匹配审计校验命令**

```bash
# 提取所有路由资源类型
grep -rn "Router::new\|\.route(" backend/src/routes/ | grep -oP '"(/[a-z0-9-]+)+"' | sort -u
# 对照权限表
psql -c "SELECT DISTINCT resource_type FROM role_permissions ORDER BY resource_type;"
# 找出有路由无权限的资源
# 找出有权限无路由的权限码
```

---

#### 14.5 is_system 滥用治理（1 维度）

**审计要点**：`is_system=true` 应仅用于 admin 角色，manager/operator 不应有此标记。

**14.5.1 is_system 滥用问题**

| 角色 | 当前 is_system | 登录后注入权限 | 问题 | 修复方案 |
|------|--------------|--------------|------|----------|
| admin | true | `*:*` | ✅ 合理：超级管理员 | 保留 |
| manager | true | `*:*` | 🔴 不合理：部门经理不应超级权限 | **改为 false** |
| operator | true | `*:*` | 🔴 不合理：操作员不应超级权限 | **改为 false** |

**14.5.2 is_system 语义规范**

```rust
// /workspace/backend/src/handlers/auth_handler.rs build_with_permissions 修正
fn build_with_permissions(user: &User, role: &Role, db_permissions: Vec<Permission>) -> Vec<String> {
    // 仅 admin 角色注入 *:* 超级通配
    if role.code == ADMIN_ROLE_CODE && role.is_system {
        return vec!["*:*".to_string()];
    }
    // 其他角色（包括 is_system=true 的 manager/operator）使用 DB 中的实际权限
    db_permissions.iter()
        .map(|p| format!("{}:{}", p.resource_type, p.action))
        .collect()
}
```

**14.5.3 is_system 字段审计规则**

```sql
-- 检查 is_system=true 的角色（应仅 admin）
SELECT id, code, name FROM roles WHERE is_system = true AND code != 'admin';
-- 预期结果：应为空集（manager/operator 不应出现）

-- 修复脚本
UPDATE roles SET is_system = false WHERE code IN ('manager', 'operator');
```

**14.5.4 登录权限注入审计**

```bash
# 检查 build_with_permissions 逻辑
grep -n "is_system\|\*:.*\*" backend/src/handlers/auth_handler.rs
# 预期：is_system 判断应同时检查 role.code == "admin"
```

---

#### 14.6 前后端权限边界一致性审计（1 维度）

**审计要点**：前端 `*:*` 放行与后端 `is_admin_role` 拒绝的不一致问题。

**14.6.1 不一致场景矩阵**

| 场景 | 前端行为（manager/operator 持有 `*:*`） | 后端行为（is_admin_role 仅承认 admin） | 用户体验 |
|------|--------------------------------------|--------------------------------------|----------|
| 角色管理菜单 | ✅ 可见（`*:*` 通过 hasRoutePermission） | ❌ 403（require_admin_role 拒绝） | 点击菜单后报错 |
| AP 付款审批 | ✅ 可见 | ❌ 403（MANAGER_ROLE_CODE 仅在 ap_payment_request_service 中承认，但 require_admin_role 不承认 manager） | 审批按钮可见但点击报错 |
| 用户管理 | ✅ 可见 | ❌ 403 | 同上 |
| 系统设置 | ✅ 可见 | ❌ 403 | 同上 |

**14.6.2 一致性修复方案**

```rust
// 方案 A（推荐）：统一前后端权限模型
// 1. 前端 hasRoutePermission 不再特殊处理 *:*，改为逐权限匹配
// 2. 后端 build_with_permissions 不再注入 *:*，改为注入实际权限列表
// 3. require_admin_role 改为 require_permission("system:admin")
// 4. 删除 is_admin_role 绕过机制，所有权限走统一 permission_middleware

// 方案 B（过渡）：保留 *:* 但前后端一致
// 1. 前端 *:* 保留（admin 专用）
// 2. 后端 is_admin_role 承认所有 is_system=true 的角色（不推荐，扩大权限）
// 3. manager/operator 改为 is_system=false，不再持有 *:*
```

**14.6.3 前端权限码与后端一致性审计**

| 前端权限码 | 后端权限码 | 是否一致 | 修复方案 |
|-----------|-----------|---------|----------|
| `users:read`（路由 meta） | `users:read`（admin 权限文件） | ✅ 一致 | — |
| `user:create`（v-permission 文档示例） | `users:create`（admin 权限文件） | ❌ 单复数不一致 | 统一为复数 `users:create` |
| `dashboard:read` | `dashboard:read` | ✅ 一致 | — |
| 路由全部用 `:read` | 后端有 `:create`/`:update`/`:delete` | 🟡 前端未控制写按钮可见性 | 补齐前端 `v-permission` 写权限控制 |

---

#### 14.7 业务角色权限矩阵设计审计（1 维度）

**审计要点**：为 14 个缺失业务角色设计完整的权限矩阵，确保每个角色"权限刚好够用，不多不少"。

**14.7.1 销售域角色权限矩阵**

| 角色 | sales | sales-contracts | sales-prices | sales-returns | customers | ar-reconciliations | dashboard |
|------|-------|-----------------|--------------|----------------|-----------|-------------------|-----------|
| sales_manager | R/C/U | R/C/U | R | R/C/U | R/C/U | R | R |
| sales | R/C（本人） | R | R | R/C（本人） | R/C（本人） | R（本人客户） | R |

**14.7.2 采购域角色权限矩阵**

| 角色 | purchases | purchase-contracts | purchase-prices | purchase-returns | suppliers | supplier-evaluations | dashboard |
|------|-----------|-------------------|-----------------|------------------|-----------|---------------------|-----------|
| purchase_manager | R/C/U | R/C/U | R | R/C/U | R/C/U | R/C/U | R |
| purchase | R/C（本人） | R | R | R/C（本人） | R | R | R |

**14.7.3 库存域角色权限矩阵**

| 角色 | inventory | stock | transfers | counts | adjustments | warehouses | dashboard |
|------|-----------|-------|-----------|--------|------------|-----------|-----------|
| warehouse_manager | R/C/U/D | R/C/U | R/C/U | R/C/U | R/C/U | R/C/U | R |
| warehouse | R/C/U | R/C/U | R/C | R/C | R/C | R | R |
| warehouse_auditor（盘点员） | R | R | R | R/C/U | R | R | R |

**14.7.4 生产域角色权限矩阵**

| 角色 | dye-batches | dye-recipes | lab-dip | production-recipes | flow-cards | production-orders | mrp | cost-collections | dashboard |
|------|------------|-------------|---------|-------------------|-----------|-------------------|-----|-----------------|-----------|
| production_manager | R/C/U | R/C/U（审批） | R | R/C/U（审批） | R | R/C/U | R/C/U | R | R |
| dye_operator | R/C | R（只读） | - | R | R/C | R | R | - | R |
| lab_technician | R | R/C/U | R/C/U | R/C/U | R | R | - | - | R |

**14.7.5 财务域角色权限矩阵**

| 角色 | finance | gl | vouchers | ap | ar | ar-reconciliations | fixed-assets | budgets | accounting-periods | dashboard |
|------|---------|-----|---------|-----|-----|-------------------|-------------|---------|-------------------|-----------|
| finance_accountant | R/C | R/C | R/C | R/C | R/C | R | R/C | R/C | R | R |
| finance_reviewer | R/U（审核） | R/U | R/U（审核） | R/U | R/U | R | R/U | R/U | R/C/U（结账） | R |

**14.7.6 其他角色权限矩阵**

| 角色 | users | departments | roles | audit | audit-logs | permissions | dashboard |
|------|-------|-------------|-------|-------|-----------|-------------|-----------|
| hr | R/C/U | R/C/U | R | - | - | - | R |
| auditor | R | R | R | R | R | R | R |
| customer | - | - | - | - | - | - | R + profile |
| employee | R/U（本人） | - | - | - | - | - | R |

---

#### 14.8 权限粒度审计（行级/字段级）（1 维度）

**审计要点**：检查权限粒度是否足够细，是否支持行级数据权限（如"销售员只能看本人订单"）和字段级权限（如"销售员不能看成本价"）。

**14.8.1 行级数据权限缺口**

| 业务场景 | 需要的行级权限 | 当前实现 | 缺口 |
|---------|--------------|---------|------|
| 销售员只能看本人订单 | sales:read WHERE salesperson_id = current_user_id | ❌ 无行级过滤 | apply_data_scope 未实现 |
| 采购员只能看本人采购单 | purchases:read WHERE buyer_id = current_user_id | ❌ 无行级过滤 | 同上 |
| 客户只能看本人订单 | sales:read WHERE customer_id = current_user.customer_id | ❌ 无行级过滤 | 同上 |
| 仓库员只能看本仓库库存 | inventory:read WHERE warehouse_id = current_user.warehouse_id | ❌ 无行级过滤 | 同上 |
| 化验员只能看本人打样记录 | lab-dip:read WHERE created_by = current_user_id | ❌ 无行级过滤 | 同上 |

**14.8.2 字段级权限缺口**

| 业务场景 | 需要的字段级权限 | 当前实现 | 缺口 |
|---------|----------------|---------|------|
| 销售员不能看成本价 | sales_order.cost_price HIDDEN | ❌ 无字段级控制 | field_permission 表未使用 |
| 销售员不能看客户信用额度 | customer.credit_limit HIDDEN | ❌ 无字段级控制 | 同上 |
| 采购员不能看供应商底价 | supplier.floor_price HIDDEN | ❌ 无字段级控制 | 同上 |
| 化验员不能看配方用量 | dye_recipe.quantity HIDDEN | ❌ 无字段级控制 | 同上 |

**14.8.3 行级数据权限实现方案**

```rust
// /workspace/backend/src/middleware/auth_context.rs 增强
pub struct AuthContext {
    pub user_id: i64,
    pub username: String,
    pub role_id: Option<i64>,
    pub role_code: String,
    pub customer_id: Option<i64>,    // 客户角色关联的客户 ID
    pub warehouse_id: Option<i64>,   // 仓库员关联的仓库 ID
    pub department_id: Option<i64>,   // 部门 ID
    // ...
}

// /workspace/backend/src/utils/data_scope.rs 新增
pub fn apply_data_scope(query: Select, auth: &AuthContext, resource: &str) -> Select {
    match auth.role_code.as_str() {
        "sales" => match resource {
            "sales_order" => query.filter(sales_order::Column::SalespersonId.eq(auth.user_id)),
            "customer" => query.filter(customer::Column::SalespersonId.eq(auth.user_id)),
            _ => query,
        },
        "purchase" => match resource {
            "purchase_order" => query.filter(purchase_order::Column::BuyerId.eq(auth.user_id)),
            _ => query,
        },
        "customer" => match resource {
            "sales_order" => query.filter(sales_order::Column::CustomerId.eq(auth.customer_id.unwrap())),
            "ar_reconciliation" => query.filter(ar_reconciliation::Column::CustomerId.eq(auth.customer_id.unwrap())),
            _ => query,
        },
        "warehouse" => match resource {
            "inventory_stock" => query.filter(inventory_stock::Column::WarehouseId.eq(auth.warehouse_id.unwrap())),
            _ => query,
        },
        _ => query,  // admin/manager 等无行级过滤
    }
}
```

**14.8.4 字段级权限实现方案**

```sql
-- field_permission 表已有（但未使用）
-- 数据模型：user_role → field_permission → resource_type + field_name + action（read/write）
INSERT INTO field_permission (role_id, resource_type, field_name, can_read) VALUES
-- 销售员不能看成本价
((SELECT id FROM roles WHERE code='sales'), 'sales_order', 'cost_price', false),
-- 销售员不能看客户信用额度
((SELECT id FROM roles WHERE code='sales'), 'customer', 'credit_limit', false),
-- 采购员不能看供应商底价
((SELECT id FROM roles WHERE code='purchase'), 'supplier', 'floor_price', false),
-- 化验员不能看配方用量（仅配方主管可见）
((SELECT id FROM roles WHERE code='lab_technician'), 'dye_recipe', 'quantity', false);
```

---

#### 14.9 权限缓存与性能审计（1 维度）

**审计要点**：权限校验有 5min 缓存，缓存失效可能导致权限变更延迟。

**14.9.1 缓存问题矩阵**

| 问题 | 当前实现 | 影响 | 修复方案 |
|------|---------|------|----------|
| 权限变更延迟 | is_admin_role 5min 缓存 + permission 5min 缓存 | 用户权限被撤销后 5min 内仍可访问 | 改为 Redis pub/sub 热更新 + 缩短缓存到 30s |
| 用户禁用延迟 | is_active 5min 缓存 | 用户被禁用后 5min 内仍可登录 | 改为 Redis 黑名单实时同步 |
| JTI 黑名单延迟 | Redis 分布式 + 进程内 HashMap 降级 | 进程内 HashMap 不跨实例同步 | 仅用 Redis，移除 HashMap 降级 |
| 角色权限变更不通知 | 无通知机制 | 角色权限变更后所有用户 5min 内仍用旧权限 | Redis pub/sub 通知所有实例清除缓存 |

**14.9.2 缓存失效方案**

```rust
// /workspace/backend/src/services/permission_cache_service.rs 增强
pub async fn invalidate_user_permission_cache(user_id: i64) -> Result<(), AppError> {
    // 1. 清除 Redis 缓存
    let key = format!("user:{}:permissions", user_id);
    redis.del(&key).await?;
    // 2. pub/sub 通知所有实例
    redis.publish("permission_invalidation", &user_id.to_string()).await?;
    Ok(())
}

pub async fn invalidate_role_permission_cache(role_id: i64) -> Result<(), AppError> {
    // 1. 查询该角色所有用户
    let user_ids = user_role::Entity::find()
        .filter(user_role::Column::RoleId.eq(role_id))
        .all(db).await?
        .into_iter().map(|ur| ur.user_id).collect::<Vec<_>>();
    // 2. 逐个清除缓存
    for user_id in user_ids {
        invalidate_user_permission_cache(user_id).await?;
    }
    Ok(())
}
```

---

#### 14.10 权限审计日志与合规审查（1 维度）

**审计要点**：建立权限变更审计 + 定期合规审查机制，识别异常权限分配。

**14.10.1 权限变更审计**

```sql
-- 权限变更审计日志表
CREATE TABLE permission_change_audit (
    id BIGSERIAL PRIMARY KEY,
    operator_id BIGINT NOT NULL,         -- 操作人
    target_type VARCHAR(20) NOT NULL,    -- user/role
    target_id BIGINT NOT NULL,          -- 用户 ID 或角色 ID
    change_type VARCHAR(20) NOT NULL,    -- grant/revoke/role_assign/role_unassign
    permission_code VARCHAR(100),        -- 权限码（grant/revoke 时）
    role_code VARCHAR(50),               -- 角色码（role_assign/role_unassign 时）
    old_value JSONB,                     -- 旧值
    new_value JSONB,                     -- 新值
    reason TEXT,                         -- 变更原因
    ip_address INET,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_permission_change_target ON permission_change_audit(target_type, target_id, created_at);
CREATE INDEX idx_permission_change_operator ON permission_change_audit(operator_id, created_at);
```

**14.10.2 异常权限分配识别规则**

| 异常模式 | 检测规则 | 处置 |
|---------|---------|------|
| 非工作时间权限变更 | 22:00-06:00 修改用户/角色权限 | 告警 + 二级审批 |
| 批量权限授予 | 单次操作授予 > 10 个权限 | 告警 + 二级审批 |
| 超级权限授予 | 授予 `*:*` 权限给非 admin 角色 | 严重告警 + 安全事件调查 |
| 互斥角色分配 | 用户被分配互斥角色 | 阻止 + 告警 |
| 离职用户权限未撤销 | 用户状态 disabled 但仍有角色 | 告警 + 自动撤销 |
| 权限回滚 | 短时间内频繁变更权限（撤销又授予） | 告警 + 审计 |

**14.10.3 定期合规审查**

```rust
// 新增定时任务：每周一 02:00 审查权限分配合规性
// cron: 0 2 * * 1
pub async fn weekly_permission_compliance_review(state: AppState) {
    // 1. 检查 is_system=true 的非 admin 角色
    let over_privileged = role::Entity::find()
        .filter(role::Column::IsSystem.eq(true))
        .filter(role::Column::Code.ne("admin"))
        .all(&state.db).await?;
    for role in over_privileged {
        security_alert("权限过大", &format!("角色 {} is_system=true 但非 admin", role.code));
    }
    // 2. 检查互斥角色冲突
    let conflicts = check_role_conflicts(&state.db).await?;
    for conflict in conflicts {
        security_alert("角色互斥冲突", &conflict);
    }
    // 3. 检查离职用户权限未撤销
    let orphan_permissions = check_orphan_permissions(&state.db).await?;
    for orphan in orphan_permissions {
        security_alert("离职用户权限未撤销", &orphan);
    }
    // 4. 检查有路由无权限的缺口
    let permission_gaps = check_permission_route_gaps(&state.db).await?;
    for gap in permission_gaps {
        security_alert("权限-路由缺口", &gap);
    }
    // 5. 生成周报
    report_service::generate_weekly_permission_compliance_report(&state.db).await?;
}
```

**14.10.4 权限审计日志保留期限**

| 日志类型 | 保留期限 | 存储位置 |
|---------|---------|----------|
| 权限变更审计日志 | 7 年 | `permission_change_audit` 表 |
| 权限合规审查报告 | 3 年 | `compliance_review_report` 表 |
| 安全告警记录 | 7 年 | `security_alert_log` 表 |

---

#### 14.11 权限测试覆盖率审计（1 维度）

**审计要点**：检查权限相关代码的测试覆盖率，确保权限校验逻辑有充分测试保障。

**14.11.1 权限测试缺口**

| 测试场景 | 当前覆盖 | 期望覆盖 | 缺口 |
|---------|---------|---------|------|
| admin 角色权限绕过 | ✅ 有测试 | admin 可访问所有资源 | — |
| 非 admin 角色权限拒绝 | ❌ 无 | 非 admin 访问受限资源返回 403 | 补测 |
| is_system=true 注入 `*:*` | ❌ 无 | manager/operator 登录后持有 `*:*` | 补测（修复后应不再持有） |
| 权限缓存失效 | ❌ 无 | 权限变更后缓存立即失效 | 补测 |
| 行级数据权限 | ❌ 无 | 销售员只能看本人订单 | 补测（实现后） |
| 字段级权限 | ❌ 无 | 销售员不能看成本价 | 补测（实现后） |
| 角色互斥校验 | ❌ 无 | 互斥角色分配被拒绝 | 补测（实现后） |
| 权限通配符匹配 | ❌ 无 | `*:*` / `resource:*` / `*:action` 匹配 | 补测 |
| 公共路径白名单 | ❌ 无 | /health 等无需认证 | 补测 |
| require_admin_role 二次校验 | ❌ 无 | 非 admin 角色管理返回 403 | 补测 |

**14.11.2 权限测试清单**（建议 30+ 单元测试）

```rust
#[cfg(test)]
mod permission_tests {
    // 1. admin 权限测试
    #[tokio::test]
    async fn test_admin_can_access_all_resources() { /* ... */ }
    #[tokio::test]
    async fn test_admin_bypass_permission_check() { /* ... */ }

    // 2. 非 admin 权限测试
    #[tokio::test]
    async fn test_non_admin_denied_without_permission() { /* ... */ }
    #[tokio::test]
    async fn test_non_admin_allowed_with_exact_permission() { /* ... */ }

    // 3. is_system 测试
    #[tokio::test]
    async fn test_manager_not_system_after_fix() { /* 修复后 manager 不再 is_system */ }
    #[tokio::test]
    async fn test_operator_not_system_after_fix() { /* 修复后 operator 不再 is_system */ }

    // 4. 通配符测试
    #[tokio::test]
    async fn test_wildcard_resource_match() { /* resource:* 匹配 */ }
    #[tokio::test]
    async fn test_wildcard_action_match() { /* *:action 匹配 */ }
    #[tokio::test]
    async fn test_super_wildcard_match() { /* *:* 匹配（仅 admin） */ }

    // 5. 行级权限测试
    #[tokio::test]
    async fn test_sales_can_only_see_own_orders() { /* ... */ }
    #[tokio::test]
    async fn test_customer_can_only_see_own_orders() { /* ... */ }

    // 6. 字段级权限测试
    #[tokio::test]
    async fn test_sales_cannot_see_cost_price() { /* ... */ }

    // 7. 角色互斥测试
    #[tokio::test]
    async fn test_conflicting_roles_rejected() { /* ... */ }

    // 8. 缓存失效测试
    #[tokio::test]
    async fn test_permission_cache_invalidation() { /* ... */ }

    // 9. 公共路径测试
    #[tokio::test]
    async fn test_public_routes_no_auth_required() { /* ... */ }

    // 10. require_admin_role 测试
    #[tokio::test]
    async fn test_require_admin_role_rejects_non_admin() { /* ... */ }
}
```

---

#### 14.12 权限安全审计（1 维度）

**审计要点**：检查权限系统的安全漏洞，包括权限提升、权限绕过、权限注入等。

**14.12.1 权限提升漏洞**

| 漏洞 | 描述 | 影响 | 修复方案 |
|------|------|------|----------|
| is_system 注入 `*:*` | manager/operator 登录后注入超级权限 | 权限提升为 admin | 仅 admin 注入 `*:*`（见 14.5） |
| require_admin_role 绕过 | 若 role.code 被篡改为 "admin" 则绕过 | 权限提升 | role.code 不可修改 + 数据库唯一约束 |
| 权限缓存投毒 | 攻击者篡改 Redis 缓存注入 `*:*` | 权限提升 | Redis 权限隔离 + 缓存签名 |
| JWT payload 篡改 | 篡改 JWT 中的 role_id | 权限提升 | JWT 签名 + 服务端验证 |

**14.12.2 权限绕过漏洞**

| 漏洞 | 描述 | 修复方案 |
|------|------|----------|
| 公共路径绕过 | PUBLIC_PATHS 匹配不精确 | 已修复（精确匹配） |
| 模块前缀不在白名单 | extract_resource_info 返回 None 导致放行 | 补齐白名单 + None 时 fail-closed |
| resource_id 为 None 通配 | matches_permission 中 None 匹配 None | None 时应 fail-closed（拒绝） |
| HTTP 方法未映射 | 非标准方法（如 HEAD/OPTIONS）未映射 action | 明确拒绝或映射为 read |

**14.12.3 权限注入漏洞**

| 漏洞 | 描述 | 修复方案 |
|------|------|----------|
| SQL 注入权限查询 | permission_middleware 查询 DB 时参数化 | 已使用 SeaORM 参数化（安全） |
| Redis 缓存投毒 | 攻击者直接写 Redis 注入缓存 | Redis 密码 + ACL + 缓存签名 |
| JWT 注入 | 攻击者伪造 JWT | JWT 签名密钥安全存储 + 过期时间 |

---

#### 类十四审计执行命令汇总

```bash
# 1. 角色清单查询
psql -c "SELECT id, code, name, is_system, description FROM roles ORDER BY id;"
# 2. 权限分配查询
psql -c "SELECT r.code, rp.resource_type, rp.action, rp.allowed FROM roles r JOIN role_permissions rp ON r.id = rp.role_id ORDER BY r.code, rp.resource_type, rp.action;"
# 3. is_system=true 的非 admin 角色
psql -c "SELECT id, code FROM roles WHERE is_system = true AND code != 'admin';"
# 4. 014 引用但不存在的 role_id
psql -c "SELECT DISTINCT role_id FROM role_permissions WHERE role_id NOT IN (SELECT id FROM roles);"
# 5. 路由暴露的资源类型
grep -rn "Router::new\|\.route(" backend/src/routes/ | grep -oP '"(/[a-z0-9-]+)+"' | sort -u
# 6. 权限定义的资源类型
psql -c "SELECT DISTINCT resource_type FROM role_permissions ORDER BY resource_type;"
# 7. 模块前缀白名单
grep -n "is_module_prefix\|MODULE_PREFIXES" backend/src/utils/path_utils.rs
# 8. is_system 注入逻辑
grep -n "is_system\|\*:\*" backend/src/handlers/auth_handler.rs
# 9. require_admin_role 校验
grep -n "require_admin_role\|ADMIN_ROLE_CODE" backend/src/handlers/role_handler.rs
# 10. 角色互斥表
grep -rn "role_conflict\|conflicting_roles" backend/src/
# 11. 行级数据权限
grep -rn "apply_data_scope\|data_scope" backend/src/
# 12. 字段级权限
grep -rn "field_permission\|FieldPermission" backend/src/
# 13. 权限变更审计
grep -rn "permission_change_audit\|PermissionChangeAudit" backend/src/
# 14. 权限测试覆盖
grep -rn "#\[tokio::test\]\|#\[test\]" backend/src/**/permission*.rs backend/tests/*permission*
```

---

### 类十五：业务主体维度审计与数据流转专项（15 维度）⭐ V15 新增（用户 2026-07-15 第八轮反馈）

> **核心目标**：建立"供货商维度审计 + 加工商维度审计 + 销售维度审计 + 客户维度审计 + 数据流转维度审计"五位一体的业务主体治理体系，覆盖功能完整性/合理性/补充需求/业务闭环/面料行业特性五个层面。
>
> **调研依据**（2026-07-15 完整代码扫描）：
> - **供货商**：完整实现（主表 + 7 张关联表 + SupplierService/SupplierEvaluationService + Handler + 路由 + 采购关联 + 对账单），但 schema 与 model 命名不一致、migration 缺失、分类未落地、资质管理不完整
> - **加工商**：**完全未实现**（无独立表、无 is_processor 标志、无委外加工单、无加工费核算、无收回入库、无 Service/Handler/路由/前端），仅 `cost_collections.processing_fee` 字段手工录入，是面料行业重大功能缺口
> - **销售**：完整实现（报价→订单→发货→收款→退货闭环 + 状态机 8 态 + 库存联动 + 信用检查 + BPM 审批 + 面料行业特性），销售合同缺明细行表、销售预测未实现
> - **客户**：完整实现（customers + 信用 + 联系人 + 色卡价格 + 行业/质量标准 + 应收账款关联），信用评级评估无自动触发
> - **数据流转**：主干完整（四条主链路全通 + 事件总线 21 事件 + 双后端 + 幂等 + 死信 + panic 隔离），染色→质检→入库监听器仅日志无业务回写、business_traces 表无写入、离线 ETL 未实现、主动异常检测未实现

#### 15.1 供货商主数据完整性审计（1 维度）

**功能全不全？合不合理？为什么？**

| 检查项 | 现状 | 合理性判定 | 原因 |
|--------|------|-----------|------|
| suppliers 主表字段完整性 | 完整（25+ 字段，含工商/税务/银行/联系/等级/状态） | ✅ 合理 | 覆盖供应商主数据全维度 |
| supplier_categories 分类表 | 表存在（[schema:1170-1190](file:///workspace/database/migration/001_consolidated_schema.sql)） | ❌ **不合理** | suppliers 主表无 `category_id` 外键，分类未落地到供应商，分类功能形同虚设 |
| supplier_qualifications 资质表 | 表存在（[schema:1299-1323](file:///workspace/database/migration/001_consolidated_schema.sql)） | ⚠️ 部分合理 | Service 仅有 list/create，缺 update/delete，资质过期无自动告警 |
| supplier_grades 等级表 | 表存在 + 4 个等级初始化数据 | ✅ 合理 | A/B/C/D 四级清晰 |
| supplier_contacts 联系人表 | 表存在 + Service 完整 CRUD | ✅ 合理 | 含 is_primary 唯一性保证 |
| supplier_blacklists 黑名单表 | 表存在 | ✅ 合理 | 含拉黑原因/解黑日期 |
| supplier_evaluations 评估表 | schema 定义完整（[schema:1227-1292](file:///workspace/database/migration/001_consolidated_schema.sql)） | ❌ **不合理** | model 层 `supplier_evaluation.rs` 实际对应 `supplier_evaluation_indicators` 表，命名不一致；`supplier_evaluations` 和 `supplier_evaluation_records` 表**无对应 migration 文件**，实际数据库可能不存在 |
| product_supplier_mappings 产品-供应商映射 | 表存在（含 supplier_price/min_order_quantity/lead_time/is_primary/priority） | ✅ 合理 | 起到价格清单的部分作用 |
| purchase_prices 采购价格表 | 表存在（含 effective_date/expiry_date/status/approved_by） | ✅ 合理 | 含价格有效期管理 |

**需要补充的功能**：
1. suppliers 主表添加 `category_id` 外键字段，关联 supplier_categories 表
2. 为 `supplier_evaluations` 和 `supplier_evaluation_records` 表补齐 sea-orm migration 文件
3. SupplierService 补齐资质 update/delete 方法 + 过期自动告警 cron
4. 供应商导入/导出/批量启用停用接口

**审计扫描命令**：
```bash
# 1. 检查 suppliers 主表是否有 category_id
grep -n "category_id" backend/src/models/supplier.rs
# 2. 检查 supplier_evaluations migration 是否存在
ls backend/migrations/ | grep -i supplier_eval
# 3. 检查资质 update/delete 方法
grep -n "update_supplier_qualification\|delete_supplier_qualification" backend/src/services/supplier_service.rs
# 4. 检查导入导出
grep -rn "supplier" backend/src/services/import_export_service.rs backend/src/handlers/import_export_handler.rs
```

#### 15.2 供货商业务闭环审计（1 维度）

**功能完整吗？有没有需要补充的？**

| 业务环节 | 实现状态 | 证据 | 补充需求 |
|---------|---------|------|---------|
| 供应商创建 → 联系人/资质事务性创建 | ✅ 完整 | [supplier_service.rs:47-151](file:///workspace/backend/src/services/supplier_service.rs) | — |
| 供应商更新 + 事件发布 + 审计日志 | ✅ 完整 | [supplier_service.rs:238-352](file:///workspace/backend/src/services/supplier_service.rs) | — |
| 供应商删除（前置校验 + lock_exclusive + 事务） | ✅ 完整 | [supplier_service.rs:356-422](file:///workspace/backend/src/services/supplier_service.rs) | — |
| 采购订单关联 supplier_id | ✅ 完整 | [purchase_order.rs:31](file:///workspace/backend/src/models/purchase_order.rs) | — |
| 采购入库/退货/质检关联 supplier_id | ✅ 完整 | schema:1610/1733/1785 | — |
| 应付账款关联（ap_invoice/ap_payment） | ✅ 完整 | [ap_invoice.rs:36](file:///workspace/backend/src/models/ap_invoice.rs) | — |
| 供应商对账单（ap_reconciliation） | ✅ 完整 | [ap_reconciliation.rs:17-81](file:///workspace/backend/src/models/ap_reconciliation.rs) | — |
| 供应商等级评估（加权得分 + A/B/C/D 评级） | ✅ 完整 | [supplier_evaluation_service.rs:131-276](file:///workspace/backend/src/services/supplier_evaluation_service.rs) | — |
| 供应商评估自动触发（季度/年度） | ❌ **未实现** | 无 cron 调度代码 | 需补 tokio-cron-scheduler 季度评估任务 |
| 供应商账户余额管理 | ❌ **未实现** | account_balance 按科目+期间维度，非供应商维度 | 需扩展供应商维度余额查询 |
| 供应商供货历史查询 | ❌ **未实现** | 无独立表/无 Service | 需通过 purchase_orders + purchase_receipts 联表查询 Service |
| 供应商价格清单导入 | ❌ **未实现** | 无 import_export 逻辑 | 需补批量导入 Excel 接口 |

**审计判定**：业务闭环 8/12 完整（67%），4 项缺失需补充。

#### 15.3 供货商面料行业特性审计（1 维度）

| 检查项 | 现状 | 合理性 | 原因 |
|--------|------|--------|------|
| supplier_type 字段区分染料/助剂/坯布供应商 | ✅ 有（fabric/dye/auxiliary/logistics/service/other） | ✅ 合理 | 覆盖面料行业核心供应商类型 |
| 供应商色卡能力字段 | ❌ **无** | ❌ **不合理** | 染料供应商应有色卡能力（能否提供色卡样、色差等级），助剂供应商应有助剂应用能力 |
| 供应商染色能力字段 | ❌ **无** | ❌ **不合理** | 委外染色供应商应有染色能力（缸号容量/染色类型/最大布重） |
| 供应商印花能力字段 | ❌ **无** | ❌ **不合理** | 印花供应商应有印花能力（印花类型/最大门幅/套色数） |
| 供应商质量认证（ISO9001/Bluesign/Oeko-Tex） | ⚠️ 通用资质表可覆盖 | ✅ 合理 | qualification_type 可扩展 |
| 供应商交期/起订量管理 | ✅ 有（product_supplier_mappings.min_order_quantity/lead_time） | ✅ 合理 | — |

**需要补充**：suppliers 主表或扩展表新增 `dyeing_capacity`（染色能力）、`printing_capacity`（印花能力）、`color_card_capability`（色卡能力）字段。

#### 15.4 加工商（委外加工商）维度审计（1 维度）

**功能全不全？为什么？**

> **重大功能缺口**：加工商维度**完全未实现**，是面料行业核心业务流程的重大缺失。

| 检查项 | 现状 | 合理性 | 原因 |
|--------|------|--------|------|
| 独立加工商表 | ❌ **无** | ❌ **不合理** | 面料行业委外加工频繁（染整/印花/整理外发），需独立管理加工商主数据 |
| suppliers 表 is_processor 标志 | ❌ **无** | ❌ **不合理** | 无法区分供应商与加工商 |
| 委外加工单表（outsourcing_order） | ❌ **无** | ❌ **不合理** | 无外发染整/印花/整理加工单管理 |
| 加工费核算表（processing_fee） | ⚠️ 仅 cost_collections.processing_fee 手工字段 | ❌ **不合理** | 无独立加工费核算系统，无法支撑对账/付款闭环 |
| 委外收回入库表（processor_receipt） | ❌ **无** | ❌ **不合理** | 外发加工后收回入库流程未实现 |
| 委外损耗处理 | ❌ **无** | ❌ **不合理** | 无损耗率字段、无损耗统计逻辑 |
| 加工商 Service/Handler/路由 | ❌ **无** | ❌ **不合理** | 完全空白 |
| 前端加工商管理界面 | ❌ **无** | ❌ **不合理** | 完全空白 |

**为什么不合理？**
面料行业三大业务模式之一就是"染色加工"和"印花加工"（[fabric-industry-research.md §6](file:///workspace/.monkeycode/docs/research/fabric-industry-research.md)），委外加工是常态。当前系统完全没有委外加工管理能力，意味着：
1. 外发染整/印花无法下单跟踪
2. 加工费无法自动核算
3. 收回入库无法走正常流程
4. 损耗无法统计
5. 成本核算中"外协加工费"只能手工录入

**需要补充的完整设计**：

```sql
-- 1. 加工商主表（复用 suppliers + is_processor 标志）
ALTER TABLE suppliers ADD COLUMN is_processor BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE suppliers ADD COLUMN processor_type VARCHAR(20); -- dyeing/printing/finishing
ALTER TABLE suppliers ADD COLUMN dyeing_capacity VARCHAR(100);
ALTER TABLE suppliers ADD COLUMN printing_capacity VARCHAR(100);

-- 2. 委外加工单表
CREATE TABLE outsourcing_orders (
    id BIGSERIAL PRIMARY KEY,
    order_no VARCHAR(50) NOT NULL UNIQUE,
    supplier_id BIGINT NOT NULL REFERENCES suppliers(id),
    outsource_type VARCHAR(20) NOT NULL, -- dyeing/printing/finishing
    source_order_id BIGINT, -- 关联生产订单/销售订单
    dye_lot_no VARCHAR(50), -- 关联缸号
    color_no VARCHAR(50),
    product_id BIGINT NOT NULL REFERENCES products(id),
    quantity_kg DECIMAL(14,3) NOT NULL,
    quantity_meters DECIMAL(14,3),
    processing_fee DECIMAL(14,2) NOT NULL,
    processing_fee_unit VARCHAR(10), -- per_kg/per_meter/per_batch
    outsource_date DATE NOT NULL,
    expected_return_date DATE,
    actual_return_date DATE,
    expected_loss_rate DECIMAL(5,2), -- 预期损耗率%
    actual_loss_rate DECIMAL(5,2), -- 实际损耗率%
    status VARCHAR(20) NOT NULL DEFAULT 'draft', -- draft/out_sourced/partial_returned/returned/cancelled
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_by BIGINT,
    remarks TEXT
);

-- 3. 委外收回入库表
CREATE TABLE outsourcing_receipts (
    id BIGSERIAL PRIMARY KEY,
    outsourcing_order_id BIGINT NOT NULL REFERENCES outsourcing_orders(id),
    receipt_no VARCHAR(50) NOT NULL UNIQUE,
    received_quantity_kg DECIMAL(14,3) NOT NULL,
    received_quantity_meters DECIMAL(14,3),
    loss_quantity_kg DECIMAL(14,3), -- 损耗数量
    loss_rate DECIMAL(5,2), -- 实际损耗率%
    warehouse_id BIGINT NOT NULL REFERENCES warehouses(id),
    batch_no VARCHAR(50), -- 入库批号
    received_date DATE NOT NULL,
    quality_result VARCHAR(20), -- passed/failed/rework
    status VARCHAR(20) NOT NULL DEFAULT 'draft',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 4. 加工费付款表（关联 ap_payment）
CREATE TABLE processor_payments (
    id BIGSERIAL PRIMARY KEY,
    outsourcing_order_id BIGINT NOT NULL REFERENCES outsourcing_orders(id),
    ap_payment_id BIGINT REFERENCES ap_payments(id),
    payment_amount DECIMAL(14,2) NOT NULL,
    payment_date DATE NOT NULL,
    status VARCHAR(20) NOT NULL DEFAULT 'pending'
);
```

#### 15.5 加工商业务流程闭环审计（1 维度）

**业务流程是否打通？**

| 流程环节 | 现状 | 需要补充 |
|---------|------|---------|
| 外发染整/印花/整理 | ❌ 未实现 | outsourcing_orders 创建 + 外发记录 |
| 加工费核算 | ⚠️ 仅手工字段 | 自动核算（数量×单价×加成）+ 关联成本归集 |
| 收回入库 | ❌ 未实现 | outsourcing_receipts + 库存回写 + 四维索引 |
| 损耗处理 | ❌ 未实现 | 实际损耗率 vs 预期损耗率 + 超损告警 |
| 加工费付款 | ❌ 未实现 | processor_payments + 关联 ap_payment |
| 委外进度跟踪 | ❌ 未实现 | 状态机 draft→out_sourced→partial_returned→returned |
| 缸号与委外加工单关联 | ❌ 未实现 | outsourcing_orders.dye_lot_no 关联 |
| 委外加工报表 | ❌ 未实现 | 加工商排名/加工费统计/损耗率分析 |

**审计判定**：业务流程 0/8 打通（0%），完全空白，需作为 V15 最高优先级补充项。

#### 15.6 销售订单数据模型与状态机审计（1 维度）

**功能完整吗？合不合理？**

| 检查项 | 现状 | 合理性 | 原因 |
|--------|------|--------|------|
| sales_orders 主表字段完整性 | ✅ 完整（order_no/customer_id/opportunity_id/order_date/required_date/ship_date/status/subtotal/tax_amount/discount_amount/shipping_cost/total_amount/paid_amount/balance_amount/shipping_address/billing_address/notes/approved_by/approved_at） | ✅ 合理 | 覆盖销售订单全维度 |
| sales_order_item 明细字段 | ✅ 完整（含面料行业字段 color_no/color_name/pantone_code/grade_required/quantity_meters/quantity_kg/gram_weight/width/batch_requirement/dye_lot_requirement/base_price/grade_price_diff/final_price/shipped_quantity_meters/shipped_quantity_kg/paper_tube_weight/is_net_weight） | ✅ 合理 | 面料行业特性完整 |
| 状态机 8 态 | ✅ 完整（draft/pending/approved/partial_shipped/shipped/completed/cancelled/rejected） | ✅ 合理 | 全状态门存在 |
| 销售报价单（sales_quotations） | ✅ 完整（currency/exchange_rate/price_terms/incoterms/tax_inclusive/tax_rate/moq/lead_time/customer_level/approval_instance_id/converted_sales_order_id） | ✅ 合理 | 含国际贸易条款 |
| 销售合同（sales_contracts） | ⚠️ 仅主表，缺明细行表 | ❌ **不合理** | 合同应有明细行（合同商品/数量/价格/交期），当前仅主表字段（contract_no/contract_name/contract_type/customer_id/total_amount/signed_date/effective_date/expiry_date/payment_terms） |
| 销售退货（sales_return） | ✅ 完整（状态机 DRAFT/SUBMITTED/APPROVED/REJECTED/COMPLETED + 关联 Customer/Warehouse/SalesOrder/Items） | ✅ 合理 | — |
| 销售预测 | ❌ 未实现 | ⚠️ 可选 | 销售分析有统计/排名，但无预测算法（时间序列/趋势外推） |

**需要补充**：销售合同明细行表（sales_contract_items）+ 销售预测算法（可选，低优先级）。

#### 15.7 销售业务流程闭环审计（1 维度）

**业务闭环是否完整？**

| 流程环节 | 实现状态 | 证据 |
|---------|---------|------|
| 报价 → 订单转换 | ✅ 完整 | [quotation_convert_service.rs:47-80](file:///workspace/backend/src/services/quotation_convert_service.rs) |
| 订单提交 → 信用检查（事务内 TOCTOU 防护） | ✅ 完整 | [order_workflow.rs:88-221](file:///workspace/backend/src/services/so/order_workflow.rs) |
| 订单审批 → BPM 审批 + MRP 触发 | ✅ 完整 | [order_workflow.rs:224-345](file:///workspace/backend/src/services/so/order_workflow.rs) |
| 库存预留（approve 后） | ✅ 完整 | [order_workflow.rs:282-313](file:///workspace/backend/src/services/so/order_workflow.rs) |
| 发货 → 库存扣减 + 防御性 WHERE + 双单位换算 | ✅ 完整 | [delivery.rs:110-514](file:///workspace/backend/src/services/so/delivery.rs) |
| 发货 → 生成应收单（AR） | ✅ 完整 | [delivery.rs:378-400](file:///workspace/backend/src/services/so/delivery.rs) |
| 发货 → 生成收入凭证 | ✅ 完整 | [delivery.rs:410-495](file:///workspace/backend/src/services/so/delivery.rs) |
| 退货 → 回写库存（四维索引）+ 红字应收单 | ✅ 完整 | [sales_return_service.rs:425-560](file:///workspace/backend/src/services/sales_return_service.rs) |
| 取消发货 → 对称恢复库存 + 预留恢复 | ✅ 完整 | [delivery.rs:916-1112](file:///workspace/backend/src/services/so/delivery.rs) |
| BPM 审批失败补偿回滚 | ✅ 完整 | [order_workflow.rs:186-209](file:///workspace/backend/src/services/so/order_workflow.rs) |
| 销售统计/排名 | ✅ 完整 | [sales_analysis_service.rs](file:///workspace/backend/src/services/sales_analysis_service.rs) |
| CSV 导出 | ✅ 完整 | [delivery.rs:1117-1228](file:///workspace/backend/src/services/so/delivery.rs) |

**审计判定**：销售业务闭环 12/12 完整（100%），实现质量高，含 TOCTOU 防护、双单位换算、对称恢复、BPM 补偿。

#### 15.8 销售面料行业特性审计（1 维度）

| 检查项 | 现状 | 合理性 | 原因 |
|--------|------|--------|------|
| 缸号同订单校验（防混缸色差） | ✅ 完整 | ✅ 合理 | [delivery.rs:68-97](file:///workspace/backend/src/services/so/delivery.rs) validate_dye_lot_consistency |
| 双单位（米/公斤）换算 | ✅ 完整 | ✅ 合理 | [delivery.rs:262-298](file:///workspace/backend/src/services/so/delivery.rs) DualUnitConverter |
| 等级价差/色差附加 | ✅ 完整 | ✅ 合理 | sales_order_item.base_price/grade_price_diff/color_extra_cost/final_price |
| 纸管重量/净重标记 | ✅ 完整 | ✅ 合理 | sales_order_item.paper_tube_weight/is_net_weight |
| 销售批色流程 | ⚠️ V15 类十一已规划 | ✅ 合理 | 交货前客户批色（剪大货样） |
| 按缸号发货/按匹号发货 | ⚠️ 部分 | ⚠️ 可改进 | 发货按 sales_order_item 级别，未细化到匹号（batch_no）级别 |

**审计判定**：面料行业特性 5/6 完整（83%），按匹号发货可后续优化。

#### 15.9 客户主数据完整性审计（1 维度）

**功能全不全？有没有需要补充的？**

| 检查项 | 现状 | 合理性 | 原因 |
|--------|------|--------|------|
| customers 主表字段 | ✅ 完整（customer_code/customer_name/contact_person/contact_phone/contact_email/address/city/province/country/postal_code/credit_limit/payment_terms/tax_id/bank_name/bank_account/status/customer_type/notes/customer_industry/main_products/annual_purchase/quality_requirement/inspection_standard） | ✅ 合理 | 覆盖客户全维度 |
| 客户分类（customer_type） | ✅ 有（retail/wholesale/vip/distributor/manufacturer/other） | ✅ 合理 | 含面料行业客户分级 |
| 客户信用评级表（customer_credit_ratings） | ✅ 完整（credit_level/credit_score/credit_limit/used_credit/available_credit/credit_days/last_assessment_date/next_assessment_date） | ✅ 合理 | 含下次评估日期 |
| 客户联系人表（customer_contacts） | ✅ 完整（is_primary 唯一性保证） | ✅ 合理 | — |
| 客户专属色卡价格表（customer_color_prices） | ✅ 完整（special_price/discount_percent/currency/valid_from/valid_until/approved_by） | ✅ 合理 | 面料行业核心特性 |
| 客户行业/主营产品/年采购额 | ✅ 有（customer_industry/main_products/annual_purchase） | ✅ 合理 | 支持客户画像 |
| 客户质量要求/检验标准 | ✅ 有（quality_requirement/inspection_standard） | ✅ 合理 | 面料行业核心字段 |
| 客户多地址表 | ❌ **无** | ⚠️ 可选 | 当前仅单地址（address 字段），大型客户可能有多个收货地址 |
| 客户多银行账户表 | ❌ **无** | ⚠️ 可选 | 当前仅单银行（bank_name/bank_account） |
| 客户标签（tags） | ✅ 有（CRM 路由 /customers/:id/tags） | ✅ 合理 | 支持客户分群 |

**需要补充**（低优先级）：客户多地址表 + 客户多银行账户表（大型客户场景）。

#### 15.10 客户信用与应收管理审计（1 维度）

**功能完整吗？合不合理？**

| 检查项 | 现状 | 合理性 | 原因 |
|--------|------|--------|------|
| 信用额度设置（set_credit_rating） | ✅ 完整 | ✅ 合理 | [customer_credit_limit.rs:23-81](file:///workspace/backend/src/services/customer_credit_limit.rs) |
| 信用占用（occupy_credit） | ✅ 完整 | ✅ 合理 | 销售订单提交时占用 |
| 信用释放（release_credit） | ✅ 完整 | ✅ 合理 | 订单取消时释放 |
| 信用额度调整（adjust_credit_limit） | ✅ 完整 | ✅ 合理 | decrease 不能低于已用 |
| 信用检查（事务内 TOCTOU 防护） | ✅ 完整 | ✅ 合理 | [customer_credit_limit.rs:257-273](file:///workspace/backend/src/services/customer_credit_limit.rs) check_credit_available_txn |
| 信用预警（80%阈值） | ✅ 完整 | ✅ 合理 | [customer_credit_limit.rs:276-305](file:///workspace/backend/src/services/customer_credit_limit.rs) check_credit_warning |
| 信用停用（有占用拒绝） | ✅ 完整 | ✅ 合理 | [customer_credit_limit.rs:312-340](file:///workspace/backend/src/services/customer_credit_limit.rs) deactivate |
| 信用评级评估算法（3 因子加权） | ✅ 完整 | ✅ 合理 | [customer_credit_evaluate.rs:1-80](file:///workspace/backend/src/services/customer_credit_evaluate.rs) |
| 信用评级自动触发（定时调度） | ❌ **未实现** | ❌ **不合理** | 评估算法存在但无 cron 自动触发，需手工调用 |
| 应收账款关联（销售→AR 链路） | ✅ 完整 | ✅ 合理 | [ar/inv.rs:120-195](file:///workspace/backend/src/services/ar/inv.rs) create_receivable |
| 对账单 PDF 导出 | ✅ 完整 | ✅ 合理 | [ar/inv.rs:41-97](file:///workspace/backend/src/services/ar/inv.rs) export_pdf |
| ES 同步最终一致性 | ✅ 完整 | ✅ 合理 | [customer_service.rs](file:///workspace/backend/src/services/customer_service.rs) sync_customer_to_es |

**需要补充**：信用评级自动触发 cron（按月度/季度自动评估）。

#### 15.11 客户面料行业特性审计（1 维度）

| 检查项 | 现状 | 合理性 | 原因 |
|--------|------|--------|------|
| 客户分级（零售/批发/VIP） | ✅ 完整 | ✅ 合理 | customer_type: retail/wholesale/vip |
| 客户专属色卡价格 | ✅ 完整 | ✅ 合理 | customer_color_prices 表 |
| 报价单按客户等级定价 | ✅ 完整 | ✅ 合理 | sales_quotation.customer_level 字段 |
| 客户批色确认能力 | ⚠️ V15 类十一已规划 | ✅ 合理 | bulk_color_approval 表（V15 新增） |
| 客户行业/主营产品 | ✅ 完整 | ✅ 合理 | customer_industry/main_products |
| 客户质量标准 | ✅ 完整 | ✅ 合理 | quality_requirement/inspection_standard |
| 客户色卡档案 | ⚠️ 色卡发放（类十）已规划 | ✅ 合理 | color_card_distribute 表（V15 新增） |
| 客户特殊工艺要求 | ❌ **无** | ⚠️ 可选 | 当前 quality_requirement 可部分覆盖，但无独立工艺要求字段 |

**审计判定**：面料行业特性 6/8 完整（75%），2 项已规划/可选。

#### 15.12 跨模块数据流转审计（1 维度）

**数据流转全不全？合不合理？**

| 检查项 | 现状 | 合理性 | 原因 |
|--------|------|--------|------|
| 销售→库存预留→发货→AR 链路 | ✅ 完整 | ✅ 合理 | 四链路全通 |
| 采购→入库→AP→付款链路 | ✅ 完整 | ✅ 合理 | 四链路全通 |
| 生产→领料→入库→成本链路 | ✅ 完整 | ✅ 合理 | 四链路全通 |
| 染色→缸号→质检→入库链路 | ⚠️ 事件定义有，监听器仅日志 | ❌ **不合理** | DyeBatchCompleted/QualityInspectionCompleted 监听器仅 tracing 日志，**无业务回写**，染色→质检→入库实际未打通 |
| 事件总线（21 事件 + 双后端） | ✅ 完整 | ✅ 合理 | Broadcast + Kafka + 自动降级 |
| 幂等去重（processed_events） | ✅ 完整 | ✅ 合理 | 主键 (consumer_id, event_key) |
| 死信队列（event_dead_letters） | ✅ 完整 | ✅ 合理 | 含重试计数/状态流转 |
| panic 隔离（AssertUnwindSafe） | ✅ 完整 | ✅ 合理 | 单事件 panic 不退出循环 |
| 事务一致性（外部事务复用） | ✅ 完整 | ✅ 合理 | create_receivable 接收外部 txn |
| TOCTOU 防护（事务内查询） | ✅ 完整 | ✅ 合理 | 信用检查/库存扣减均事务内 |
| 行锁（lock_exclusive / FOR UPDATE） | ✅ 完整 | ✅ 合理 | 客户/库存操作均加锁 |
| 主数据冗余字段刷新 | ✅ 完整 | ✅ 合理 | CustomerUpdated/SupplierUpdated 触发 5+2 张表刷新 |
| ES 同步最终一致性 | ✅ 完整 | ✅ 合理 | PG 提交后 sync，失败仅 warn |
| BPM 补偿回滚 | ✅ 完整 | ✅ 合理 | 新事务回滚订单状态 |

**需要补充**：DyeBatchCompleted/QualityInspectionCompleted 监听器补齐业务回写逻辑（染色完成→触发质检→质检通过→触发入库）。

#### 15.13 数据流转业务回写审计（1 维度）

**有没有需要补充的？**

| 检查项 | 现状 | 合理性 | 原因 |
|--------|------|--------|------|
| 库存财务桥接（7 种 transaction_type） | ✅ 完整 | ✅ 合理 | [inventory_finance_bridge_service.rs:216-282](file:///workspace/backend/src/services/inventory_finance_bridge_service.rs) |
| 库存财务桥接幂等（inventory_txn:{transaction_id}） | ✅ 完整 | ✅ 合理 | [inventory_finance_bridge_service.rs:222-240](file:///workspace/backend/src/services/inventory_finance_bridge_service.rs) |
| business_traces 业务追溯表 | ❌ **模型存在但无写入** | ❌ **不合理** | [business_trace.rs:1-84](file:///workspace/backend/src/models/business_trace.rs) 表存在但无 Service 写入代码，属潜在死代码或未完成功能 |
| CustomerUpdated 冗余刷新（5 张表） | ✅ 完整 | ✅ 合理 | ar_invoices/ar_collections/ar_reconciliations/customer_credits/sales_contracts |
| SupplierUpdated 冗余刷新（2 张表） | ✅ 完整 | ✅ 合理 | purchase_contracts/fixed_assets |
| 染色完成事件回写 | ❌ **仅日志** | ❌ **不合理** | DyeBatchCompleted 监听器仅 tracing::info，无业务回写 |
| 质检完成事件回写 | ❌ **仅日志** | ❌ **不合理** | QualityInspectionCompleted 监听器仅 tracing::info，无业务回写 |

**需要补充**：
1. business_traces 表写入 Service（或决策删除该表，归入死代码清理）
2. DyeBatchCompleted 监听器补齐：染色完成 → 自动创建质检单
3. QualityInspectionCompleted 监听器补齐：质检通过 → 自动触发入库

#### 15.14 数据流转报表与追溯审计（1 维度）

**功能完整吗？**

| 检查项 | 现状 | 合理性 | 原因 |
|--------|------|--------|------|
| 销售分析报表（概览/排名/趋势） | ✅ 完整 | ✅ 合理 | sales_analysis_service.rs |
| AP 应付账龄分析 | ✅ 完整 | ✅ 合理 | ap_invoice_service.rs get_aging_analysis |
| 库存财务一体化（凭证自动生成） | ✅ 完整 | ✅ 合理 | 业财一体化 7 种流水自动生成凭证 |
| 对账单 PDF 导出 | ✅ 完整 | ✅ 合理 | ar/inv.rs export_pdf |
| CSV 导出 | ✅ 完整 | ✅ 合理 | delivery.rs export_orders_to_csv |
| 财务指标刷新（事件驱动） | ✅ 完整 | ✅ 合理 | SalesOrderShipped → FinancialIndicatorUpdate |
| 离线报表/数据仓库 ETL | ❌ **未实现** | ⚠️ 可选 | 大数据量场景需 T+1 聚合，当前项目规模可暂不实现 |
| 业务追溯（business_traces 写入） | ❌ **未实现** | ❌ **不合理** | 模型存在但无写入，无法追溯缸号全链路 |
| 报表数据追溯到源单据 | ⚠️ 部分 | ⚠️ 可改进 | 销售统计可追溯到订单，但缸号全链路追溯未实现 |

**需要补充**：business_traces 写入 Service（缸号全链路追溯：投染→染色→质检→入库→发货→退货）。

#### 15.15 数据流转审计与异常检测审计（1 维度）

**审计记录全不全？有没有异常检测？**

| 检查项 | 现状 | 合理性 | 原因 |
|--------|------|--------|------|
| 操作日志（operation_logs） | ✅ 完整 | ✅ 合理 | [operation_log.rs:1-65](file:///workspace/backend/src/models/operation_log.rs) 含 module/action/request_method/request_uri/request_ip/user_agent/status/error_message/duration_ms/extra_data |
| 全链路审计（omni_audit_logs） | ✅ 完整 | ✅ 合理 | [omni_audit_log.rs:1-37](file:///workspace/backend/src/models/omni_audit_log.rs) 含 trace_id/span_id/parent_span_id/HMAC-SHA256 防篡改 |
| 事务内审计写入 | ✅ 完整 | ✅ 合理 | customer_service.rs update_with_audit 事务内原子写入 |
| 生产订单操作日志查询 | ✅ 完整 | ✅ 合理 | production_order_service.rs get_order_logs |
| 事件死信审计 | ✅ 完整 | ✅ 合理 | event_dead_letter.rs 失败事件落库 |
| 事件处理幂等审计 | ✅ 完整 | ✅ 合理 | processed_event.rs 已处理事件落库 |
| 业务批次追溯 | ❌ **模型存在无写入** | ❌ **不合理** | business_trace.rs 表存在但无 Service 写入 |
| 主动异常检测引擎 | ❌ **未实现** | ❌ **不合理** | 仅有 LowStockAlert/MaterialShortageAlert 被动触发，无主动异常检测（如：异常大额订单/异常频繁退货/异常库存波动） |
| 数据流转异常告警 | ❌ **未实现** | ❌ **不合理** | 无阈值告警引擎（如：事件处理延迟>5min 告警/死信>10 条告警/事务失败率>1% 告警） |
| 审计日志定期审查 cron | ⚠️ V15 类十三 13.10 已规划 | ✅ 合理 | 每日合规审查 cron |

**需要补充**：
1. business_traces 写入 Service（决策保留并接入业务，或删除归入死代码）
2. 主动异常检测引擎（异常大额/频繁退货/库存波动/事件延迟/死信堆积/事务失败率）
3. 数据流转异常告警（阈值告警 + 邮件/消息通知）

**审计扫描命令汇总**：
```bash
# 1. 供货商主数据完整性
grep -n "category_id" backend/src/models/supplier.rs
ls backend/migrations/ | grep -i supplier_eval
grep -n "update_supplier_qualification\|delete_supplier_qualification" backend/src/services/supplier_service.rs

# 2. 加工商维度（应为空，确认未实现）
grep -rn "outsourc\|processor\|is_processor\|委外\|外协" backend/src/ frontend/src/

# 3. 销售合同明细行表
grep -n "sales_contract_item" backend/src/models/ backend/src/services/ backend/src/handlers/

# 4. 客户信用评级自动触发
grep -rn "cron\|scheduler\|evaluate_credit" backend/src/services/customer_credit*.rs

# 5. 数据流转监听器业务回写
grep -A 20 "DyeBatchCompleted\|QualityInspectionCompleted" backend/src/services/event_bus.rs | grep -v "tracing\|info\|warn"

# 6. business_traces 写入
grep -rn "business_trace\|BusinessTrace" backend/src/services/ | grep -v "models/"

# 7. 异常检测引擎
grep -rn "anomaly_detect\|abnormal\|异常检测" backend/src/
```

---

### 类十六：AI 模块审计专项（10 维度）⭐ V15 新增（用户 2026-07-15 第九轮反馈"所有维度都应该被严格审计"）

> **审计对象**：14 个 AI 相关模块（ai_process_optimization/ai_quality_prediction/ai/{detect,pred,rec,recipe_opt}/ai_extend_service/advanced/{analytics,decide,forecast,quality_pred,rec,recipe_opt,reorder}），这些模块在 V15 前 15 大类 115 维度中**完全未覆盖**。
> **审计目标**：评估 AI 模块的功能完整性、业务正确性、数据安全、权限控制、可解释性、性能与监控。
> **合理性基础**：面料行业 AI 应用是染整数字化升级的核心能力（AI 配方优化/质量预测/补货决策），但 V15 此前完全未审计 AI 模块，存在重大风险盲区。

#### 16.1 AI 模型可解释性与透明度审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| AI 决策结果可解释性 | 每个 AI 预测/推荐/优化结果必须包含 `explanation`/`confidence_score`/`factors` 字段，禁止黑盒输出 | 通用 AI 治理 |
| AI 模型版本管理 | 模型版本号/训练日期/训练数据集大小可追溯，禁止无版本模型上线 | 通用 MLOps |
| AI 决策审计日志 | AI 每次调用记录 input/output/user_id/timestamp/latency 到 `ai_decision_log` 表 | 通用 AI 合规 |
| AI 人工干预机制 | 关键 AI 决策（如配方优化/补货）支持人工复核与覆盖，覆盖记录可追溯 | 通用 AI 治理 |

**合理性评估**：当前 AI 模块仅返回结果，无可解释性/版本/审计日志/人工干预机制，**不合理**。

#### 16.2 AI 数据安全与隐私审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| AI 训练数据脱敏 | 训练数据中客户/供应商敏感信息（手机/银行账号/身份证）必须脱敏 | 规则 11 数据保护 |
| AI 推理数据最小化 | AI 推理仅请求必要字段，禁止全表扫描传给模型 | 规则 11 数据最小化 |
| AI 数据存储加密 | AI 中间结果（model_cache/prediction_history）加密存储 | 规则 12 安全标准 |
| AI 接口认证授权 | 所有 AI 端点（/ai/*、/advanced/*）必须认证 + RBAC 权限校验 | 类三 3.4 + 类十二 |

**合理性评估**：AI 模块当前可能未做训练数据脱敏/推理数据最小化/中间结果加密，**需审计补齐**。

#### 16.3 AI 模型训练与推理正确性审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| AI 模型训练数据集合理性 | 训练数据采样无偏差，覆盖足够样本量（≥1000 条业务样本） | 通用 ML |
| AI 推理结果一致性 | 相同输入产生相同输出（除随机性参数外），禁止非确定性漂移 | 通用 ML |
| AI 模型评估指标 | 模型上线前必须有 accuracy/precision/recall/F1 评估报告 | 通用 MLOps |
| AI 模型漂移检测 | 模型上线后定期检测数据漂移/概念漂移，触发再训练 | 通用 MLOps |

**合理性评估**：当前 AI 模块缺乏模型评估报告和漂移检测，**不合理**。

#### 16.4 AI 权限控制与访问审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| AI 端点权限矩阵 | 14 个 AI 端点必须有明确的角色权限矩阵（admin/manager/analyst 可用，operator/customer 禁用） | 类十二/十四 |
| AI 资源权限码 | AI 端点必须注册到 `permissions` 表（如 `ai:predict:list`/`ai:recipe:optimize`），路径工具白名单补齐 | 类十四 14.4 |
| AI 数据权限 | AI 推理结果按用户数据范围过滤（如销售分析师只能看自己负责客户的预测） | 类十二 12.7 |
| AI 操作审计 | AI 调用必须写入 `omni_audit_logs`，记录调用者/参数/结果/耗时 | 类十三 13.6 |

**合理性评估**：AI 端点当前可能未注册权限码/未做数据范围过滤/未审计调用，**不合理**。

#### 16.5 AI 配方优化业务正确性审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 配方优化输入校验 | 输入的染料/助剂/目标色值必须经过面料行业规则校验（染料配伍性/助剂兼容性） | fabric-industry-research §11.2 |
| 配方优化输出合理性 | 优化后的配方总成本不能高于原配方 10%（除非显式标注"质量优先"模式） | 通用业务规则 |
| 配方优化历史回溯 | 每次优化记录原配方/优化配方/优化原因/采纳状态，支持回滚 | 通用审计 |
| 配方优化与化验室打样集成 | 优化配方可一键推送到化验室打样系统验证 | fabric-industry-research §11.1 |

**合理性评估**：当前 AI 配方优化可能与化验室打样脱节，**需审计补齐**。

#### 16.6 AI 质量预测准确性审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 质量预测准确率监控 | 预测准确率（A/B/C 级分类）必须 ≥ 80%，低于阈值触发告警 | 通用 ML 监控 |
| 质量预测特征完整性 | 预测特征必须包含染料/助剂/温度/时间/缸号/胚布来源等面料行业关键因子 | fabric-industry-research §11.4 |
| 质量预测误判成本 | 误判为 A 级实际 C 级的成本（客户索赔）必须可量化追踪 | 通用业务影响 |
| 质量预测与质检结果对账 | 每月对账预测结果与实际质检结果，生成准确率报告 | 通用 MLOps |

**合理性评估**：当前 AI 质量预测可能未做准确率监控和误判成本追踪，**不合理**。

#### 16.7 AI 推荐业务合理性审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 推荐结果多样性 | 推荐列表不能全是相似项，必须有 diversity_score ≥ 0.3 | 通用推荐系统 |
| 推荐冷启动处理 | 新用户/新产品必须有冷启动策略（默认热门/同类推荐） | 通用推荐系统 |
| 推荐反馈闭环 | 用户对推荐结果的采纳/拒绝反馈必须回流到模型再训练 | 通用推荐系统 |
| 推荐业务约束 | 推荐必须遵守业务约束（如缺货商品不下架、停用客户不推荐） | 通用业务规则 |

**合理性评估**：当前 AI 推荐可能未做多样性/冷启动/反馈闭环，**不合理**。

#### 16.8 AI 补货决策合理性审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 补货数量合理性 | 补货建议数量必须基于安全库存 + 历史消耗 + 采购提前期，禁止凭空推荐 | 通用供应链 |
| 补货时机判断 | 补货触发时机必须考虑染整生产周期（≥7 天提前期），不能仅看当前库存 | fabric-industry-research §11.2 |
| 补货供应商推荐 | 补货决策可推荐最优供应商（基于历史质量/价格/交期评分） | 类十五供货商评估 |
| 补货与 MRP 集成 | AI 补货建议必须与 MRP 引擎结果对账，差异超 20% 需人工复核 | 通用业务集成 |

**合理性评估**：当前 AI 补货可能与 MRP 脱节，**需审计补齐**。

#### 16.9 AI 接口性能与资源消耗审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| AI 推理响应时间 | AI 接口 P95 响应时间 ≤ 2s，超时返回降级结果（默认/缓存） | 类七 7.5 性能 |
| AI 模型资源占用 | 模型加载内存 ≤ 1GB，禁止 OOM 风险 | 通用运维 |
| AI 并发控制 | AI 接口必须有并发限制（如 max_concurrent=10），防止 GPU/CPU 过载 | 类十三 13.9 |
| AI 缓存策略 | 相同输入的 AI 结果必须缓存（TTL 5min），减少重复推理 | 类七 7.5 缓存 |

**合理性评估**：当前 AI 接口可能未做并发控制和缓存，**不合理**。

#### 16.10 AI 测试覆盖率与监控审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| AI 单元测试覆盖率 | AI service 单测覆盖率 ≥ 70%，包含正常/异常/边界场景 | 类六 6.1 |
| AI 集成测试 | AI 端点必须有 E2E 测试，覆盖认证/权限/输入校验/响应格式 | 类六 6.2 |
| AI 模型监控看板 | 必须有 Grafana 看板监控 AI 调用量/延迟/准确率/错误率 | 类二十 20.2 |
| AI 告警机制 | AI 准确率下降/响应超时/错误率激增必须触发告警 | 类二十 20.2 |

**合理性评估**：当前 AI 模块缺乏测试覆盖和监控告警，**不合理**。

---

### 类十七：财务深化审计专项（8 维度）⭐ V15 新增（用户 2026-07-15 第九轮反馈"所有维度都应该被严格审计"）

> **审计对象**：7 个财务深化子模块（accounting_period/assist_accounting/ar_collection/ar_aging_analysis/financial_analysis/fund_management/budget_management/fixed_asset），V15 前 15 大类仅间接覆盖。
> **审计目标**：补齐财务深化审计盲区，覆盖会计期间结账/辅助核算/应收催收/账龄/财务分析/资金管理/预算/固定资产。
> **合理性基础**：财务模块是 ERP 核心，V15 此前仅审计凭证/应付/应收主链路，深化模块（结账/辅助核算/催收/预算/资产）缺失审计存在合规风险。

#### 17.1 会计期间结账与跨期处理审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 会计期间状态机 | 期间状态：`open`→`closing`→`closed`→`reopened`（仅审计允许 reopen），禁止已结账期间录入凭证 | 通用财务 |
| 月结/年结流程 | 月结必须检查所有凭证已审核 + 试算平衡 + 损益结转，年结必须新增会计年度 + 余额结转 | 通用财务 |
| 跨期凭证处理 | 跨期凭证必须明确归属期间，禁止模糊跨期（如"待摊费用"必须有摊销计划） | 通用财务 |
| 结账锁定机制 | 已结账期间凭证禁止修改/删除，必须通过"反结账"流程（需审计权限） | 类十四权限 |

**合理性评估**：当前会计期间模块可能未做结账锁定/跨期处理/年结余额结转，**不合理**。

#### 17.2 多维度辅助核算完整性审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 辅助核算维度完整性 | 必须支持客户/供应商/部门/项目/产品 5 个标准维度，可选自定义维度 | 通用财务 |
| 辅助核算与主账平衡 | 辅助核算汇总余额必须与总账科目余额一致，差异自动告警 | 通用财务 |
| 辅助核算报表穿透 | 辅助核算报表支持从总账→辅助明细→凭证穿透查询 | 类一 1.3 财务闭环 |
| 辅助核算数据完整性 | 每张凭证必须录入完整辅助核算维度（如启用），禁止空值 | 通用财务 |

**合理性评估**：当前辅助核算可能未做主辅账平衡校验和报表穿透，**不合理**。

#### 17.3 应收催收流程与坏账处理审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 催收流程闭环 | 催收流程：账龄预警→催收任务分配→催收记录→催收结果→坏账申请→坏账审批→坏账核销 | 通用财务 |
| 催收任务自动分配 | 超期账款必须自动生成催收任务，按客户经理/区域分配 | 通用业务 |
| 坏账准备计提 | 必须按账龄法（30/60/90/180/365 天阶梯比例）自动计提坏账准备 | 通用财务 |
| 坏账核销审批 | 坏账核销必须经二级审批（财务经理+总经理），核销后账龄清零但保留历史 | 类十三 13.4 二级审批 |

**合理性评估**：当前应收催收可能未做自动任务分配和坏账准备计提，**不合理**。

#### 17.4 应收账龄分析准确性审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 账龄分段合理性 | 账龄分段必须支持自定义（默认 0-30/31-60/61-90/91-180/180+ 五段） | 通用财务 |
| 账龄计算基准日 | 账龄必须按"业务日期"计算（非"系统日期"），支持期末快照 | 通用财务 |
| 账龄与凭证对账 | 账龄分析总额必须与应收账款总账余额一致 | 通用财务 |
| 账龄历史趋势 | 账龄分析必须支持月度趋势图，识别恶化/改善趋势 | 通用财务 |

**合理性评估**：当前账龄分析可能未做期末快照和与总账对账，**不合理**。

#### 17.5 财务分析模型合理性审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 财务比率分析完整性 | 必须包含偿债能力（流动/速动）/营运能力（应收/存货周转）/盈利能力（毛利率/净利率）/发展能力（收入增长率）4 类比率 | 通用财务 |
| 杜邦分析完整性 | 杜邦分析必须分解：ROE = 净利率 × 总资产周转率 × 权益乘数，3 层分解 | 通用财务 |
| 趋势分析方法 | 趋势分析必须支持同比/环比/定基比 3 种方法 | 通用财务 |
| 财务预警机制 | 财务指标异常（如流动比率 < 1.5）必须触发预警通知 | 类二十 20.2 |

**合理性评估**：当前财务分析可能未做杜邦分解和预警机制，**不合理**。

#### 17.6 资金管理与调拨流程审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 资金账户管理 | 资金账户必须区分银行/现金/支付宝/微信，每个账户独立余额 | 通用财务 |
| 资金调拨审批 | 资金调拨必须经二级审批（调出方+调入方财务确认），调拨记录可追溯 | 类十三 13.4 |
| 资金预测模型 | 资金预测必须基于应收/应付到期日 + 历史现金流，预测未来 30/60/90 天现金流 | 通用财务 |
| 资金安全控制 | 大额调拨（>10 万）必须额外验证（短信验证码/UKey），防止误操作 | 类三 3.4 |

**合理性评估**：当前资金管理可能未做资金预测和大额调拨额外验证，**不合理**。

#### 17.7 预算编制执行调整闭环审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 预算编制方法 | 必须支持增量预算/零基预算/滚动预算 3 种方法 | 通用财务 |
| 预算执行控制 | 预算超支必须拦截（或预警），预算执行率实时可见 | 通用财务 |
| 预算调整审批 | 预算调整必须经审批（部门经理→财务经理→总经理），调整记录可追溯 | 类十三 13.4 |
| 预算差异分析 | 必须按月生成预算差异分析报告（实际 vs 预算，差异率/原因/责任人） | 通用财务 |

**合理性评估**：当前预算管理可能未做执行控制和差异分析，**不合理**。

#### 17.8 固定资产折旧处置盘点审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 折旧方法合理性 | 必须支持直线法/工作量法/年数总和法/双倍余额递减法 4 种，按资产类别配置 | 通用财务 |
| 折旧自动计提 | 月末必须自动计提折旧，生成折旧凭证，禁止手工录入 | 通用财务 |
| 资产处置流程 | 资产处置（出售/报废/毁损）必须走审批流程，处置损益自动计算并生成凭证 | 通用财务 |
| 资产盘点闭环 | 必须支持资产盘点（盘点单→盘点结果→盘盈盘亏处理→凭证），盘点差异可追溯 | 通用财务 |

**合理性评估**：当前固定资产可能未做自动折旧计提和盘点闭环，**不合理**。

---

### 类十八：CRM 全链路审计专项（5 维度）⭐ V15 新增（用户 2026-07-15 第九轮反馈"所有维度都应该被严格审计"）

> **审计对象**：3 个 CRM 子模块（crm_lead/crm_opportunity/crm_pool），V15 前 15 大类仅间接覆盖（N+1 修复/规则持久化）。
> **审计目标**：补齐 CRM 全链路审计盲区，覆盖线索/商机/客户池转化漏斗与回收策略。
> **合理性基础**：CRM 是销售前端核心，V15 此前仅审计销售订单/客户主数据（类十五），线索→商机→客户转化链路缺失审计存在业务盲区。

#### 18.1 CRM 线索管理与转化漏斗审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 线索评分模型 | 线索必须有评分（基于来源/行为/资料完整度），高分线索优先分配 | 通用 CRM |
| 线索转化漏斗 | 必须有线索→商机→客户转化漏斗报表，转化率可监控 | 通用 CRM |
| 线索来源追踪 | 线索来源必须追踪（广告/转介绍/官网/展会），ROI 可计算 | 通用 CRM |
| 线索重复去重 | 同一客户重复线索必须自动去重（手机/邮箱匹配），禁止重复跟进 | 通用 CRM |

**合理性评估**：当前线索管理可能未做评分模型和转化漏斗，**不合理**。

#### 18.2 商机阶段与赢率预测审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 商机阶段状态机 | 商机阶段必须标准化（线索→接触→报价→谈判→成交/失败），禁止跳跃 | 通用 CRM |
| 商机赢率自动计算 | 赢率必须按阶段自动计算（如报价阶段 50%），禁止手工随意填写 | 通用 CRM |
| 商机预测准确性 | 商机预测金额 × 赢率 = 加权预测金额，月度预测准确率 ≥ 70% | 通用 CRM |
| 商机赢/输原因分析 | 商机关闭必须记录赢/输原因，输单原因 TOP 5 月度分析 | 通用 CRM |

**合理性评估**：当前商机管理可能未做赢率自动计算和输单分析，**不合理**。

#### 18.3 客户池公海私海回收策略审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 公海私海规则 | 客户必须区分公海（无负责人）/私海（有负责人），私海客户上限限制（如 100 个/人） | 通用 CRM |
| 自动回收规则 | 超期未跟进（如 30 天）/超期未成交（如 90 天）的私海客户自动回收到公海 | 通用 CRM |
| 回收规则配置 | 回收规则必须可配置（跟进周期/成交周期/上限数量），按业务部门差异化 | 类七 7.1 CRM 规则持久化 |
| 公海领取限制 | 公海领取必须有限制（每日上限/冷却期），防止抢占 | 通用 CRM |

**合理性评估**：当前客户池可能未做自动回收规则和领取限制，**不合理**。

#### 18.4 CRM 数据权限与团队协作审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| CRM 数据权限 | 销售只能看自己负责的线索/商机/客户，经理能看团队所有，总监能看全部门 | 类十二 12.7 数据权限 |
| CRM 团队协作 | 商机支持团队成员协作（多人跟进），权限明确（主负责人/协助人/查看人） | 通用 CRM |
| CRM 数据共享 | 临时数据共享必须有时效（如 7 天）和审计日志 | 类十三 13.6 |
| CRM 客户转移 | 客户转移（负责人变更）必须经双方确认 + 审批，转移记录可追溯 | 通用 CRM |

**合理性评估**：当前 CRM 可能未做团队协作和客户转移审批，**不合理**。

#### 18.5 CRM 与销售模块数据流转审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 线索→客户转化 | 线索转化为客户时，必须自动创建 customer 记录并关联 lead_id | 类十五 15.9 客户主数据 |
| 商机→报价单转化 | 商机转化为报价单时，必须自动创建 quotation 并关联 opportunity_id | 类十五 15.6 销售订单 |
| 报价单→销售订单转化 | 报价单转销售订单必须复用 quotation_convert_service | 类十五 15.7 销售业务闭环 |
| CRM 与销售数据一致性 | 商机金额必须与关联报价单/销售订单金额一致，差异自动告警 | 通用业务集成 |

**合理性评估**：当前 CRM 与销售模块可能未做数据流转和一致性校验，**不合理**。

---

### 类十九：报表 BI 与通知协同审计专项（8 维度）⭐ V15 新增（用户 2026-07-15 第九轮反馈"所有维度都应该被严格审计"）

> **审计对象**：4 个报表子模块（report_definition/report_subscription/dashboard/bi_analysis）+ 2 个通知子模块（notification/email）+ OA 公告 + 用户行为/页面浏览/五维度分析。
> **审计目标**：补齐报表 BI 与通知协同审计盲区。
> **合理性基础**：报表 BI 是决策支持核心，通知中心是协同核心，V15 此前未独立审计存在决策与协同盲区。

#### 19.1 报表定义与模板管理审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 报表元数据完整性 | 报表定义必须包含：名称/分类/数据源/参数/权限/刷新策略/导出格式 | 通用报表 |
| 报表模板版本管理 | 报表模板必须有版本号，修改后版本递增，旧版本可回滚 | 通用报表 |
| 报表参数校验 | 报表参数必须校验（必填/类型/范围），禁止 SQL 注入风险 | 类三 3.6 IDOR |
| 报表权限控制 | 报表必须注册到权限系统（如 `report:sales:view`），按角色控制可见 | 类十二/十四 |

**合理性评估**：当前报表定义可能未做版本管理和权限控制，**不合理**。

#### 19.2 报表订阅与定时推送审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 订阅权限校验 | 订阅报表必须校验订阅者有该报表查看权限，禁止越权订阅 | 类十二 12.7 |
| 定时推送机制 | 订阅必须支持定时推送（日/周/月），通过邮件/站内信/Webhook 推送 | 通用报表 |
| 推送失败重试 | 推送失败必须重试 3 次（指数退避），重试失败记录到死信队列 | 类五 5.6 事件重试 |
| 订阅退订机制 | 用户必须可随时退订，退订后立即停止推送 | 通用合规 |

**合理性评估**：当前报表订阅可能未做权限校验和推送失败重试，**不合理**。

#### 19.3 BI 分析与多维钻取审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| BI 多维分析完整性 | BI 必须支持多维分析（时间/客户/产品/区域/销售员），支持上卷/下钻/切片/旋转 | 通用 BI |
| BI 数据缓存 | BI 查询结果必须缓存（如 5min），减少重复计算压力 | 类七 7.5 缓存 |
| BI 大数据性能 | BI 查询 100 万行数据 P95 响应时间 ≤ 5s，超时降级为异步导出 | 类七 7.5 性能 |
| BI 数据权限 | BI 必须按用户数据范围过滤（销售员只能看自己的销售数据） | 类十二 12.7 |

**合理性评估**：当前 BI 分析可能未做数据缓存和数据权限过滤，**不合理**。

#### 19.4 仪表板数据卡片实时刷新审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 仪表板卡片配置 | 仪表板必须支持自定义卡片（拖拽/调整大小/删除），配置持久化 | 通用仪表板 |
| 仪表板实时刷新 | 关键指标（库存/订单/财务）必须支持实时刷新（WebSocket 推送） | 类二十 20.3 WebSocket |
| 仪表板权限控制 | 仪表板必须按角色控制可见卡片（如财务卡片仅财务可见） | 类十二/十四 |
| 仪表板性能 | 仪表板首屏加载 ≤ 2s，单卡片查询 ≤ 500ms | 类七 7.5 性能 |

**合理性评估**：当前仪表板可能未做权限控制和实时刷新，**不合理**。

#### 19.5 通知中心多渠道去重审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 通知渠道完整性 | 必须支持站内信/邮件/短信/Webhook 4 渠道，按通知类型配置渠道 | 通用通知 |
| 通知去重机制 | 同一事件 5min 内重复触发必须去重，避免通知轰炸 | 通用通知 |
| 通知已读未读 | 站内信必须区分已读/未读，未读数量角标显示，支持批量已读 | 通用通知 |
| 通知模板管理 | 通知必须有模板（变量替换），模板支持多语言 | 类七 7.2 i18n |

**合理性评估**：当前通知中心可能未做多渠道和去重机制，**不合理**。

#### 19.6 邮件服务 SMTP 队列重试审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| SMTP 配置安全 | SMTP 密码必须加密存储，禁止明文配置 | 类三 3.3 密钥 |
| 邮件发送队列 | 邮件必须走异步队列（如 Redis 队列），禁止同步发送阻塞请求 | 类五 5.6 事件 |
| 邮件失败重试 | 邮件发送失败必须重试 3 次（指数退避），重试失败记录到 email_log | 通用邮件 |
| 邮件附件安全 | 邮件附件必须扫描病毒（如 ClamAV），附件大小限制 ≤ 25MB | 类三 3.2 路径 |

**合理性评估**：当前邮件服务可能未走异步队列和失败重试，**不合理**。

#### 19.7 OA 公告与用户行为分析审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| OA 公告权限控制 | 公告发布必须经审批，已发布公告不可删除（仅可撤回+审计日志） | 类十三 13.6 |
| OA 公告可见性 | 公告必须按部门/角色可见，禁止全员可见的敏感公告 | 类十二 12.7 数据权限 |
| 用户行为采集合规 | 用户行为采集必须告知用户（隐私政策），支持用户拒绝采集 | 规则 11 数据保护 |
| 用户行为数据脱敏 | 用户行为日志中敏感操作（如查看客户手机号）必须脱敏记录 | 类八 8.3 数据脱敏 |

**合理性评估**：当前 OA 公告和用户行为可能未做权限控制和隐私合规，**不合理**。

#### 19.8 五维度分析与页面浏览统计审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 五维度分析业务合理性 | 五维度（如人/机/料/法/环）必须与面料行业业务对齐，禁止泛化 | fabric-industry-research |
| 五维度数据来源 | 五维度数据必须从生产/质量/库存模块自动归集，禁止手工录入 | 通用业务集成 |
| 页面浏览统计准确性 | 页面浏览必须按 SPA 路由切换统计（非页面刷新），区分有效浏览/跳出 | 通用统计 |
| 页面浏览数据保留 | 页面浏览明细数据保留 90 天，超期自动归档为汇总数据 | 类八 8.3 日志保留 |

**合理性评估**：当前五维度分析和页面浏览可能未做业务对齐和数据归档，**不合理**。

---

### 类二十：可观测性与运维审计专项（8 维度）⭐ V15 新增（用户 2026-07-15 第九轮反馈"所有维度都应该被严格审计"）

> **审计对象**：observability/metrics/websocket/failover/slow_query/api_gateway/system_version/enhanced_logger 8 个运维模块。
> **审计目标**：补齐可观测性与运维审计盲区。
> **合理性基础**：可观测性是生产稳定性基石，V15 此前仅间接审计（资源生命周期），trace/metrics/failover 独立审计缺失存在运维盲区。

#### 20.1 可观测性 trace 链路完整性审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| trace 链路完整性 | 每个请求必须有完整 trace_id，跨服务传递（HTTP/Kafka/事件总线）保留 trace_id | 通用可观测性 |
| span 上下文传递 | span_context 必须包含 parent_span_id，支持调用链树状展示 | 通用可观测性 |
| trace 采样策略 | 生产环境采样率必须可配置（默认 10%），错误请求 100% 采样 | 通用可观测性 |
| trace 数据保留 | trace 明细数据保留 7 天，超期归档为统计数据 | 类八 8.3 日志保留 |

**合理性评估**：当前 observability 可能未做完整 trace 链路和采样策略，**不合理**。

#### 20.2 metrics 指标体系与告警审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| metrics 指标完整性 | 必须采集 QPS/延迟/错误率/CPU/内存/磁盘/网络 7 类核心指标 | 通用运维 |
| metrics 告警规则 | 必须配置告警规则（如错误率 > 5% / P95 > 2s / CPU > 80%），告警分级（P0/P1/P2） | 通用运维 |
| metrics 告警通知 | 告警必须通知到负责人（邮件/短信/钉钉），告警去重（5min 内不重复） | 类十九 19.5 通知 |
| metrics 看板 | 必须有 Grafana 看板，关键指标实时可视化 | 通用运维 |

**合理性评估**：当前 metrics 可能未做告警规则和分级，**不合理**。

#### 20.3 WebSocket 实时推送可靠性审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| WebSocket 连接管理 | 连接必须有超时/重连/心跳机制（30s ping，超时 60s 断开） | 通用实时通信 |
| WebSocket 消息可靠性 | 消息必须支持 ACK 机制，未确认消息重发（最多 3 次） | 通用实时通信 |
| WebSocket 多实例广播 | 多实例部署时必须用 Redis Pub/Sub 广播消息，确保所有连接收到 | 通用分布式 |
| WebSocket 鉴权安全 | 连接必须鉴权（JWT/票据），禁止匿名连接，票据 30s 过期 | 类三 3.4 认证 |

**合理性评估**：当前 WebSocket 可能未做 ACK 机制和多实例广播，**不合理**。

#### 20.4 故障转移主备切换回切审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 故障检测机制 | 必须有健康检查（5s 间隔），连续 3 次失败触发故障转移 | 通用高可用 |
| 主备切换流程 | 切换必须自动（10s 内完成），切换记录可追溯，禁止脑裂 | 通用高可用 |
| 数据同步一致性 | 主备数据必须实时同步（如 PostgreSQL 流复制），切换后无数据丢失 | 通用高可用 |
| 故障回切机制 | 故障恢复后必须人工确认回切（不能自动回切，防止抖动） | 通用高可用 |

**合理性评估**：当前 failover 可能未做数据同步和人工回切，**不合理**。

#### 20.5 慢查询阈值告警优化审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 慢查询阈值合理性 | 慢查询阈值必须可配置（默认 500ms），按业务场景差异化 | 通用数据库 |
| 慢查询自动告警 | 慢查询超阈值必须自动告警（含 SQL/耗时/调用方），每小时聚合去重 | 通用数据库 |
| 慢查询优化追踪 | 慢查询必须创建优化任务（Jira/工单），跟踪优化进度和效果 | 通用数据库 |
| 慢查询报表 | 必须有慢查询周报（TOP 10/趋势/优化进展），支持导出 | 通用数据库 |

**合理性评估**：当前慢查询可能未做自动告警和优化追踪，**不合理**。

#### 20.6 API 网关路由转发限流熔断审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| API 网关路由配置 | 路由必须可配置（动态加载），路由变更不重启服务 | 通用网关 |
| API 网关限流 | 必须按 IP/用户/接口限流，超限返回 429 + Retry-After 头 | 类三 3.5 速率限制 |
| API 网关熔断 | 下游服务故障必须熔断（5s 内失败率 > 50% 触发），熔断后快速失败 | 通用网关 |
| API 网关鉴权 | 网关必须统一鉴权（JWT 校验），下游服务信任网关身份 | 类三 3.4 认证 |

**合理性评估**：当前 API 网关可能未做熔断和统一鉴权，**不合理**。

#### 20.7 系统版本与升级管理审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 版本号管理 | 必须遵循语义化版本（major.minor.patch），版本变更记录可追溯 | 通用版本管理 |
| 升级流程 | 升级必须走灰度（先 10% → 50% → 100%），支持回滚 | 通用部署 |
| 数据库迁移版本 | migration 必须有序号，禁止跳跃，回滚脚本必须配套 | 通用数据库 |
| 升级兼容性 | 升级必须向后兼容（旧 API 至少保留 1 个版本），废弃 API 标注 deprecation | 通用 API |

**合理性评估**：当前系统版本可能未做灰度升级和回滚脚本，**不合理**。

#### 20.8 日志增强与系统日志完整性审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 日志分级合理性 | 日志必须分级（DEBUG/INFO/WARN/ERROR/FATAL），生产环境默认 INFO | 通用日志 |
| 日志脱敏 | 日志中敏感信息（手机/身份证/密码）必须脱敏 | 类八 8.3 数据脱敏 |
| 日志结构化 | 日志必须结构化（JSON 格式），支持 ELK/Loki 检索 | 通用日志 |
| 日志归档 | 日志必须按天归档，保留 90 天，超期自动清理 | 类八 8.3 日志保留 |

**合理性评估**：当前 enhanced_logger 可能未做结构化和归档，**不合理**。

---

### 类二十一：胚布拆匹与质量处理审计专项（5 维度）⭐ V15 新增（用户 2026-07-15 第九轮反馈"所有维度都应该被严格审计"）

> **审计对象**：胚布管理（greige_fabric）/拆匹（piece_split）/拆匹映射（piece_mapping）+ 质量问题（quality_issue）/不合格品（unqualified_product）。
> **审计目标**：补齐面料行业胚布流转与质量处理审计盲区。
> **合理性基础**：胚布是染整源头，拆匹是库存管理核心，质量问题处理是 8D 闭环，V15 此前未独立审计存在业务盲区。

#### 21.1 胚布库存与采购管理审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 胚布库存模型 | 胚布必须独立库存模型（区分胚布/成品），支持按卷/匹/公斤管理 | fabric-industry-research §3 |
| 胚布采购流程 | 胚布采购必须走采购订单流程，关联供应商 + 价格 + 入库 | 类十五 15.2 供货商业务 |
| 胚布库存预警 | 胚布库存必须有安全库存预警，低于阈值触发补货建议 | 类二十二 22.2 |
| 胚布批次追溯 | 胚布必须有批次号，支持从胚布→染色→成品全链路追溯 | 类四 4.3.7 全链路追溯 |

**合理性评估**：当前胚布管理可能未做独立库存和批次追溯，**不合理**。

#### 21.2 胚布委托加工流转审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 胚布委托加工流程 | 胚布委托加工必须走委外流程：胚布出库→加工→成品入库 | 类十五 15.4 加工商 |
| 胚布损耗核算 | 委外加工的胚布损耗必须核算（标准损耗 vs 实际损耗），差异自动告警 | 类十五 15.5 加工商业务闭环 |
| 胚布质量追溯 | 委外加工回来的胚布必须质检，质检不合格走质量问题处理 | 21.4 质量问题 |
| 胚布加工费核算 | 委外加工费必须按缸号/匹号核算，自动生成应付凭证 | 类十五 15.5 |

**合理性评估**：当前胚布委托加工可能未走委外流程，**不合理**。

#### 21.3 拆匹后缸号匹号继承规则审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 拆匹缸号继承 | 拆匹后子匹必须继承原缸号（dye_lot_no），禁止新建缸号 | 类一 1.5 缸号约束 |
| 拆匹匹号生成 | 拆匹后子匹必须有新匹号（batch_no），匹号在缸号内唯一 | 类一 1.5 匹号唯一 |
| 拆匹数量校验 | 拆匹后子匹数量之和必须等于原匹数量，差异自动告警 | 通用业务规则 |
| 拆匹历史追溯 | 拆匹必须记录原匹→子匹映射（piece_mapping），支持反向追溯 | 通用追溯 |

**合理性评估**：当前拆匹可能未做缸号继承和数量校验，**不合理**。

#### 21.4 质量问题 8D 处理流程审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 8D 流程完整性 | 质量问题必须走 8D 流程：D1 团队→D2 描述→D3 临时措施→D4 根因→D5 永久措施→D6 验证→D7 预防→D8 闭环 | 通用质量管理 |
| 根因分析方法 | 必须使用 5Why/鱼骨图等根因分析方法，记录分析过程 | 通用质量管理 |
| 纠正预防措施跟踪 | 永久措施必须有责任人和完成日期，超期自动告警 | 通用质量管理 |
| 8D 报表 | 必须有 8D 月报（问题数/关闭率/平均关闭周期），支持导出 | 通用质量管理 |

**合理性评估**：当前质量问题可能未走 8D 流程，**不合理**。

#### 21.5 不合格品降级返工报废流程审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 不合格品分类 | 不合格品必须分类：降级（A→B→C）/返工/报废，分类需审批 | fabric-industry-research §11.4 |
| 降级处理 | 降级必须更新库存等级 + 调整价格（按等级价差表） | 类十五 15.8 销售面料行业 |
| 返工流程 | 返工必须走生产订单（返工工单），返工成本归集到原缸号 | 类四 4.3.6 成本归集 |
| 报废流程 | 报废必须走审批（财务+总经理），报废损失自动计入成本 | 类十三 13.4 二级审批 |

**合理性评估**：当前不合格品可能未做分类处理和返工流程，**不合理**。

---

### 类二十二：库存排程物料审计专项（6 维度）⭐ V15 新增（用户 2026-07-15 第九轮反馈"所有维度都应该被严格审计"）

> **审计对象**：库存调拨（inventory_transfer）/库存告警（stock_alert）/物料短缺（material_shortage）+ 排程（scheduling）/产能（capacity）/工作中心（work_center）。
> **审计目标**：补齐库存调拨与生产排程审计盲区。
> **合理性基础**：库存调拨是跨仓流转核心，排程是生产计划核心，V15 此前未独立审计存在计划盲区。

#### 22.1 库存调拨跨库位跨缸号审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 调拨流程闭环 | 调拨必须走流程：申请→审批→出库→在途→入库→确认 | 通用库存 |
| 跨库位调拨 | 调拨必须支持跨仓库/跨库位，在途库存独立核算 | 通用库存 |
| 跨缸号调拨 | 调拨必须支持按缸号/匹号明细调拨，禁止混合缸号调拨 | 类一 1.5 缸号约束 |
| 调拨审批权限 | 调拨必须按金额/数量分级审批（如 >1 万需经理审批） | 类十三 13.4 |

**合理性评估**：当前库存调拨可能未做在途库存和分级审批，**不合理**。

#### 22.2 库存告警安全库存补货策略审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 安全库存设置 | 每个产品必须设置安全库存（按仓库/缸号），低于阈值触发告警 | 通用库存 |
| 补货策略 | 必须支持订货点法/EOQ/MRP 三种补货策略，按产品配置 | 通用供应链 |
| 告警通知机制 | 库存告警必须通知到采购员/计划员（站内信+邮件） | 类十九 19.5 通知 |
| 告警去重 | 同一产品 24h 内只告警一次，避免告警轰炸 | 类十九 19.5 去重 |

**合理性评估**：当前库存告警可能未做补货策略和告警去重，**不合理**。

#### 22.3 物料短缺预警闭环审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 物料短缺识别 | 必须基于生产订单 + BOM + 库存自动识别物料短缺 | 通用 MRP |
| 短缺预警分级 | 短缺必须分级（严重/中等/轻微），严重短缺立即通知 | 通用供应链 |
| 短缺处理闭环 | 短缺必须走处理流程：识别→采购申请→采购订单→入库→解除 | 通用供应链 |
| 短缺报表 | 必须有短缺月报（短缺次数/处理周期/影响生产），支持导出 | 通用供应链 |

**合理性评估**：当前物料短缺可能未做分级和处理闭环，**不合理**。

#### 22.4 自动排程算法合理性审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 排程算法 | 必须基于产能约束 + 订单优先级 + 交期 + 缸号批量，支持自动排程 | fabric-industry-research §11.2 |
| 排程冲突检测 | 排程必须检测冲突（如同一缸号同时排两单），冲突自动告警 | 通用排程 |
| 排程可视化 | 排程必须支持甘特图展示，支持拖拽调整 | 通用排程 |
| 排程与生产集成 | 排程结果必须自动生成生产订单，禁止手工重复录入 | 类四 4.3.2 |

**合理性评估**：当前排程可能未做冲突检测和可视化，**不合理**。

#### 22.5 产能规划与瓶颈识别审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 产能模型 | 必须按工作中心建模产能（标准工时/班次/设备数） | 通用生产 |
| 产能负荷 | 必须计算产能负荷（已排产工时/总产能工时），负荷 > 80% 告警 | 通用生产 |
| 瓶颈识别 | 必须自动识别瓶颈工作中心（负荷最高），建议扩产/外包 | 通用生产 |
| 产能报表 | 必须有产能月报（各工作中心负荷/利用率/瓶颈），支持导出 | 通用生产 |

**合理性评估**：当前产能可能未做瓶颈识别和负荷告警，**不合理**。

#### 22.6 工作中心调度与排程集成审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 工作中心模型 | 工作中心必须关联设备/人员/班次，支持多技能人员 | 通用生产 |
| 调度规则 | 调度必须按规则（FIFO/SPT/EDD/优先级），规则可配置 | 通用排程 |
| 排程与调度集成 | 排程结果必须自动下发到工作中心，禁止手工转移 | 通用生产 |
| 调度异常处理 | 调度异常（设备故障/人员请假）必须自动重排，通知计划员 | 通用生产 |

**合理性评估**：当前工作中心可能未做调度规则和异常重排，**不合理**。

---

### 类二十三：组织定制物流审计专项（5 维度）⭐ V15 新增（用户 2026-07-15 第九轮反馈"所有维度都应该被严格审计"）

> **审计对象**：部门管理（department）/定制订单（custom_order）/售后（after_sales）/物流（logistics）+ 国际贸易术语（incoterms）。
> **审计目标**：补齐组织定制物流审计盲区。
> **合理性基础**：组织架构是权限基础，定制订单/售后/物流是业务延伸，incoterms 是外贸核心，V15 此前未独立审计存在业务盲区。

#### 23.1 组织架构部门管理审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 部门树形结构 | 部门必须支持树形结构（parent_id），支持多级部门 | 通用组织 |
| 部门与权限关联 | 部门必须关联数据权限（部门负责人能看本部门数据） | 类十二 12.7 数据权限 |
| 部门与用户关联 | 用户必须归属部门，支持一人多部门（主部门+兼职） | 通用组织 |
| 部门变更审计 | 部门变更（新建/合并/拆分/撤销）必须审计，影响范围可追溯 | 类十三 13.6 |

**合理性评估**：当前部门可能未做权限关联和变更审计，**不合理**。

#### 23.2 定制订单流程与质量管控审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 定制订单流程 | 定制订单必须走流程：需求确认→打样→报价→生产→质检→交付 | fabric-industry-research §11.1 |
| 定制质量管控 | 定制订单必须有专属质量标准（客户签字确认），质检按客户标准 | 类四 4.3.4 质检 |
| 定制订单变更 | 定制订单变更必须经客户确认 + 二级审批 | 类十三 13.4 |
| 定制订单追溯 | 定制订单必须支持全链路追溯（需求→打样→生产→交付） | 类四 4.3.7 |

**合理性评估**：当前定制订单可能未做专属质量标准和客户确认，**不合理**。

#### 23.3 售后管理与工单流转审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 售后工单类型 | 必须支持退货/换货/维修/投诉 4 类售后工单 | 通用售后 |
| 售后流程闭环 | 售后必须走流程：申请→受理→处理→确认→评价→关闭 | 通用售后 |
| 售后原因分析 | 售后必须记录原因（质量/物流/客户偏好），月度 TOP 5 原因分析 | 通用售后 |
| 售后与质量集成 | 质量问题引发的售后必须关联 quality_issue，走 8D 流程 | 类二十一 21.4 |

**合理性评估**：当前售后可能未做原因分析和与质量集成，**不合理**。

#### 23.4 物流运单跟踪与运费核算审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 运单管理 | 运单必须关联销售订单/采购订单，支持多订单合并发货 | 通用物流 |
| 物流跟踪 | 必须支持物流跟踪（对接快递 API 或手工录入），状态实时更新 | 通用物流 |
| 运费核算 | 运费必须按重量/体积/距离核算，支持客户承担/公司承担分摊 | 通用财务 |
| 物流签收 | 必须支持电子签收（上传签收单），签收后自动触发应收确认 | 类十五 15.7 |

**合理性评估**：当前物流可能未做跟踪和运费核算，**不合理**。

#### 23.5 国际贸易术语 incoterms 完整性审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| incoterms 2020 完整性 | 必须支持 11 种国际贸易术语（EXW/FCA/CPT/CIP/DAP/DPU/DDP/FAS/FOB/CFR/CIF） | 通用外贸 |
| 术语与价格集成 | 不同术语必须关联不同价格构成（如 FOB 不含运费/保费，CIF 含运费/保费） | 通用外贸 |
| 术语与责任划分 | 必须明确风险转移点/费用承担方/出口进口清关责任 | 通用外贸 |
| 术语报表 | 必须有术语使用月报（按术语统计出口量/金额），支持合规审查 | 类八 8.5 法律合规 |

**合理性评估**：当前 incoterms 工具可能未与价格集成和责任划分，**不合理**。

---

### 类二十四：前端架构与体验审计专项（20 维度）⭐ V15 新增（用户 2026-07-15 第九轮反馈"所有维度都应该被严格审计"）

> **审计对象**：前端 75+ views + 17 components + 36 composables + 5 stores + 85+ api 文件 + 20 个前端独有维度（响应式/路由懒加载/状态管理/组件设计/composables/图表/WebSocket/性能/构建/测试/安全/可访问性/错误边界/表单/i18n/权限粒度/路由元信息/API 拦截器/主题样式/虚拟列表）。
> **审计目标**：补齐前端架构与体验审计盲区（V15 前 15 大类前端覆盖率 < 5%）。
> **合理性基础**：前端是用户直接接触层，V15 此前仅借道审计（权限/导出/i18n 3 点约 50 条），前端架构与体验缺失审计存在用户盲区。

#### 24.1 前端响应式设计与移动端适配审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 响应式断点 | 所有 views 必须使用 Element Plus 响应式断点（xs/sm/md/lg/xl），覆盖移动端 | 通用前端 |
| 移动端布局 | 移动端必须侧边栏抽屉化 + 顶部导航折叠 + 触屏按钮 ≥ 44px | 通用移动端 |
| PWA 支持 | 必须支持 PWA（manifest.json + Service Worker 离线访问） | 通用 PWA |
| 移动端性能 | 必须图片懒加载（loading="lazy"）+ 虚拟列表 + 减少 reflow | 类七 7.5 性能 |

**合理性评估**：当前前端可能仅 PC 端适配，移动端缺失，**不合理**。

#### 24.2 路由懒加载与代码分割审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 路由懒加载率 | 60+ 路由必须 100% 使用 `() => import()` 懒加载，禁止同步导入 | 通用前端 |
| chunk 大小 | 单 chunk ≤ 500KB，超限告警（rollup-plugin-visualizer） | 通用构建 |
| 首屏性能 | FCP < 1.8s，LCP < 2.5s，TTI < 3.8s | 通用性能 |
| prefetch 策略 | 关键路由必须 prefetch（如登录后预加载 dashboard） | 通用前端 |

**合理性评估**：当前路由可能未全部懒加载，首屏性能未监控，**不合理**。

#### 24.3 Pinia 状态管理与持久化审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| store 模块化 | 6 个 store 必须按业务域拆分（user/dashboard/fabric/inventory/sales/系统） | 通用状态管理 |
| store 持久化 | 必须用 pinia-plugin-persistedstate，敏感数据（token）不入 localStorage | 类三 3.4 |
| store 跨模块通信 | store A 调用 store B 必须避免循环依赖，必要时用事件总线解耦 | 通用状态管理 |
| store 测试覆盖率 | 每个 store 必须有单元测试，覆盖率 ≥ 70% | 类六 6.1 |

**合理性评估**：当前 store 可能未做持久化和测试，**不合理**。

#### 24.4 组件设计与 Props/Emits 类型安全审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| Props 类型安全 | 17 个组件必须用 `defineProps<T>()` 泛型，禁止 any | 类二 2.9 类型安全 |
| Emits 类型安全 | 必须用 `defineEmits<T>()` 泛型，事件名 kebab-case | 通用前端 |
| 组件复用 | Charts 4 组件必须复用 BaseChart，禁止重复图表代码 | 通用复用 |
| 组件文档 | 公共组件必须有 props/emits/slots 文档注释 | 类十 10.6 前端规范 |

**合理性评估**：当前组件可能未做泛型类型和复用，**不合理**。

#### 24.5 composables 响应式与内存泄漏审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 响应式正确性 | 36 composables 必须正确使用 ref/reactive/computed，禁止解构丢失响应式 | 通用前端 |
| 内存泄漏 | 定时器/事件监听必须在 onUnmounted 清理，禁止泄漏 | 通用前端 |
| 错误处理 | composables 必须有 try/catch，错误向上抛出由调用方处理 | 类二 2.3 |
| 命名规范 | 必须统一 useXxx 前缀，业务逻辑放 store 还是 composable 有明确边界 | 通用前端 |

**合理性评估**：当前 composables 可能未做内存清理和命名规范，**不合理**。

#### 24.6 ECharts 图表性能与无障碍审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 大数据性能 | 图表 >10000 数据点必须用 dataZoom 分段加载，禁止全量渲染 | 通用图表 |
| resize 监听 | 图表必须监听窗口 resize，自动调整大小 | 通用图表 |
| 内存泄漏 | 组件卸载必须 dispose() 销毁 ECharts 实例，禁止泄漏 | 通用前端 |
| 按需引入 | 必须用 echarts/core tree-shaking，禁止全量引入 | 通用构建 |

**合理性评估**：当前图表可能未做 dispose 和按需引入，**不合理**。

#### 24.7 WebSocket 客户端连接重连心跳审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 重连策略 | 必须指数退避（1s→30s），最大 10 次重连，超限降级轮询 | 类二十 20.3 |
| 心跳机制 | 必须 30s ping，超时 60s 断开重连 | 通用实时通信 |
| 内存清理 | disconnect 必须清理 heartbeatTimer/reconnectTimer，禁止泄漏 | 通用前端 |
| 票据鉴权 | 一次性票据 30s 过期，URL query 传递票据需评估泄露风险 | 类三 3.4 |

**合理性评估**：当前 WebSocket 客户端可能未做票据泄露评估，**不合理**。

#### 24.8 前端性能与 bundle 体积审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| bundle 体积 | vendor chunk ≤ 1MB，业务 chunk ≤ 500KB，总体 ≤ 3MB | 通用性能 |
| Tree Shaking | 未使用导出清零（V15 类二 2.8 已规划 466 个） | 类二 2.8 |
| 防抖节流 | 搜索/滚动/resize 必须用 debounce/throttle，禁止高频触发 | 通用性能 |
| 内存泄漏 | 组件卸载必须清理 ref/timer/eventListener，禁止泄漏 | 通用前端 |

**合理性评估**：当前前端可能未做 bundle 监控和防抖节流，**不合理**。

#### 24.9 Vite 构建与 Tree Shaking 审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| Vite 配置 | vite.config.ts 必须配置 build.optimizeDeps + rollupOptions.manualChunks | 通用构建 |
| Code Splitting | 必须按业务域分割 chunk（vendor/finance/sales/inventory/production） | 通用构建 |
| Source Map | 生产环境必须禁用 source map（或加密） | 类三 3.3 安全 |
| 环境变量 | import.meta.env.VITE_* 必须有类型定义，禁止运行时拼字符串 | 通用前端 |

**合理性评估**：当前 Vite 配置可能未做 Code Splitting 和 Source Map 控制，**不合理**。

#### 24.10 前端测试覆盖率与 mock fixtures 审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 单元测试 | Vitest 配置 + 覆盖率 ≥ 70%（statements/branches/functions/lines） | 类六 6.1 |
| 组件测试 | Vue Test Utils + @vue/test-utils，关键组件必须有测试 | 类六 6.2 |
| E2E 测试 | Cypress/Playwright 端到端测试，核心流程覆盖 | 类六 6.3 |
| mock fixtures | 测试 mock 数据必须 fixtures 化，禁止内联 mock | 规则 6 |

**合理性评估**：当前前端测试覆盖率可能为 0，**不合理**。

#### 24.11 前端 XSS 防护与 CSP 策略审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| v-html 使用 | 必须审计所有 v-html 使用点，未消毒数据禁止 v-html | 类三 3.6 安全 |
| DOMPurify 消毒 | 必须用 DOMPurify 消毒用户输入后再 v-html | 通用安全 |
| CSP 策略 | 必须配置 Content-Security-Policy 头，禁止 unsafe-inline | 类三 3.4 |
| Clickjacking | 必须配置 X-Frame-Options: DENY 或 CSP frame-ancestors | 通用安全 |

**合理性评估**：当前前端可能未做 XSS 消毒和 CSP 策略，**不合理**。

#### 24.12 敏感数据存储与 token 安全审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| localStorage 扫描 | 必须扫描全项目 localStorage.setItem，敏感数据（手机/身份证/银行）禁止入 localStorage | 类三 3.3 |
| token 存储 | access_token 必须用 httpOnly Cookie，refresh_token 评估安全性 | 类三 3.4 |
| 前端密钥泄露 | API Key/第三方密钥禁止入前端代码，必须走后端代理 | 类三 3.3 |
| 开放重定向 | redirect query 参数必须白名单校验，禁止任意重定向 | 类二 2.10 |

**合理性评估**：当前前端可能未做 localStorage 扫描和密钥泄露审计，**不合理**。

#### 24.13 前端可访问性 WCAG 2.1 AA 审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 键盘导航 | 必须支持 Tab/Shift+Tab/Esc/Enter 焦点管理，禁用 tabindex > 0 | 通用 a11y |
| 焦点管理 | 路由切换后焦点重置，模态框焦点陷阱（Tab 循环在框内） | 通用 a11y |
| ARIA 完整性 | 必须用 role/aria-label/aria-describedby/aria-live，屏幕阅读器兼容 | 类七 7.2 |
| 色彩对比度 | 文本/按钮/图标对比度 ≥ 4.5:1（WCAG AA），自动检测 | 通用 a11y |

**合理性评估**：当前前端可能未做键盘导航和 ARIA，**不合理**。

#### 24.14 错误边界与全局错误处理审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| ErrorBoundary | 必须用 Vue 3 errorCaptured 钩子实现 ErrorBoundary，错误时显示降级 UI | 通用前端 |
| 全局错误处理 | 必须配置 app.config.errorHandler 全局捕获，上报到监控 | 类二十 20.2 |
| 前端监控 | 必须接入 Sentry/Bugsnag/自研监控，错误自动上报 | 通用运维 |
| 错误去重 | 相同错误 5min 内不重复上报，避免监控爆炸 | 类十九 19.5 |

**合理性评估**：当前前端可能未做 ErrorBoundary 和监控接入，**不合理**。

#### 24.15 表单验证与异步校验审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 表单校验规则 | Element Plus Form rules 必须配置（required/pattern/validator），统一规范 | 通用前端 |
| 异步校验 | 唯一性校验（如客户编码）必须异步校验，校验期间禁用提交 | 通用前端 |
| 防重复提交 | 提交按钮必须 loading + debounce，禁止重复提交 | 通用前端 |
| 脏数据检测 | 表单有未保存修改时离开必须提示（beforeRouteLeave/beforeunload） | 通用前端 |

**合理性评估**：当前表单可能未做异步校验和防重复提交，**不合理**。

#### 24.16 i18n 国际化深化与复数 RTL 审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 资源文件同步 | zh-CN.ts/en-US.ts 必须同步，缺失 key 自动告警 | 类七 7.2 |
| 语言切换运行时 | 必须支持运行时切换语言，持久化用户偏好 | 类七 7.2 |
| 日期/数字格式化 | 必须用 Intl.DateTimeFormat/NumberFormat，禁止硬编码格式 | 通用 i18n |
| RTL 支持 | 阿拉伯语等 RTL 语言必须支持布局翻转 | 通用 i18n |

**合理性评估**：当前 i18n 可能未做资源同步检测和 RTL，**不合理**。

#### 24.17 前端权限粒度按钮字段行级审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 按钮级权限 | 所有按钮必须用 v-permission，覆盖率 100% | 类十二 12.4 |
| 字段级权限 | 表单字段必须支持 v-permission 控制显示/隐藏/只读 | 类十四 14.8 |
| 行级权限 | 列表数据必须由后端按数据范围过滤，前端不二次过滤 | 类十二 12.7 |
| 权限缓存刷新 | 权限变更后前端必须刷新权限缓存（5min TTL 或 WebSocket 推送） | 类十四 14.9 |

**合理性评估**：当前前端权限可能未覆盖到字段级和缓存刷新，**不合理**。

#### 24.18 路由元信息与动态路由审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| meta 完整性 | 路由 meta 必须包含 title/icon/permission/hidden/public 5 字段 | 类二 2.10 |
| 动态路由 | 必须支持基于权限的动态路由注册（如 admin 才能看到系统管理） | 类十二 12.4 |
| keep-alive | 必须配置 keep-alive，Tab 切换状态保留 | 通用前端 |
| breadcrumb | 必须基于路由 meta 自动生成面包屑 | 通用前端 |

**合理性评估**：当前路由可能未做动态路由和 keep-alive，**不合理**。

#### 24.19 API 请求拦截器与超时重试审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| 请求拦截器 | 必须注入 CSRF Token + Authorization 头 + Content-Type | 类二 2.9 |
| 响应拦截器 | 必须统一处理业务码 + loading 管理 + 错误提示 | 类二 2.9 |
| 请求取消 | 路由切换必须用 AbortController 取消旧请求 | 通用前端 |
| 超时重试 | 必须配置全局超时（30s）+ 指数退避重试（3 次） | 通用前端 |

**合理性评估**：当前请求拦截器可能未做取消和重试，**不合理**。

#### 24.20 主题样式与暗黑模式审计

| 检查项 | 期望状态 | 来源 |
|--------|----------|------|
| CSS 变量 | 主题色/间距/字体必须用 CSS 变量，禁止硬编码颜色 | 通用前端 |
| 暗黑模式 | 必须支持 Element Plus dark theme 适配，持久化用户偏好 | 通用前端 |
| 样式作用域 | 必须用 scoped，全局样式仅限 reset/变量 | 通用前端 |
| 主题切换 | 必须支持亮色/暗色/品牌主题切换，切换不刷新页面 | 通用前端 |

**合理性评估**：当前前端可能未做 CSS 变量化和暗黑模式，**不合理**。

---

## 四、V15 审计执行流程

### 4.1 触发条件

V15 审计在以下条件**全部满足**后自动触发：

1. v14 复审 12 P0 + 31 P1 + 12 P2 + 6 P3 全部修复完成
2. 批次 425-432（流转卡/验布/产量工资/能耗/染化料/委外/多业务模式/缸号状态机）全部合并
3. baseline 警告数持续保持 0
4. CI 全绿
5. 色卡发放业务规则修正专项（类十）已完成（或与 V15 并行执行）
6. 大货批色业务规则专项（类十一）已完成（或与 V15 并行执行）
7. 纺织行业法律财税合规模型设计完成（类八 8.5-8.8 数据模型已设计）
8. RBAC 权限控制机制架构设计完成（类十二 12.1 数据模型已设计）
9. 打印导出审计与权限控制架构设计完成（类十三 13.2 权限矩阵 + 13.4 二级审批数据模型已设计）
10. 权限维度审计与角色合理性架构设计完成（类十四 14.1 角色清单合理性矩阵 + 14.2 权限分配矩阵 + 14.3 职责分离 SoD + 14.5 is_system 滥用治理方案已设计）
11. 业务主体维度审计与数据流转架构设计完成（类十五 15.1 供货商主数据完整性矩阵 + 15.4 加工商维度缺口 + 15.7 销售业务闭环 + 15.10 客户信用与应收 + 15.12 跨模块数据流转矩阵已设计）
12. AI 模块审计架构设计完成（类十六 16.1 可解释性 + 16.4 权限矩阵 + 16.5 配方优化业务 + 16.9 性能监控已设计）
13. 财务深化审计架构设计完成（类十七 17.1 会计期间状态机 + 17.3 应收催收流程 + 17.7 预算闭环已设计）
14. CRM 全链路审计架构设计完成（类十八 18.1 线索评分模型 + 18.3 客户池回收策略已设计）
15. 报表 BI 与通知协同审计架构设计完成（类十九 19.1 报表定义 + 19.5 通知中心 + 19.7 OA/用户行为已设计）
16. 可观测性与运维审计架构设计完成（类二十 20.1 trace 链路 + 20.4 故障转移 + 20.6 API 网关已设计）
17. 胚布拆匹与质量处理审计架构设计完成（类二十一 21.1 胚布库存 + 21.3 拆匹规则 + 21.4 8D 流程已设计）
18. 库存排程物料审计架构设计完成（类二十二 22.1 库存调拨 + 22.4 排程算法 + 22.5 产能规划已设计）
19. 组织定制物流审计架构设计完成（类二十三 23.1 部门管理 + 23.2 定制订单 + 23.5 incoterms 已设计）
20. 前端架构与体验审计架构设计完成（类二十四 24.1 响应式 + 24.3 Pinia + 24.10 前端测试 + 24.13 WCAG 已设计）

### 4.2 审计执行方式

```
V15 审计阶段（自动执行，20 批并行子代理）
  ├── 第 1 批：类一回归验证（5 维度）+ 类二通用代码质量（10 维度）= 15 维度 × 1 子代理
  ├── 第 2 批：类三安全性独立审计（6 维度）+ 类八法律合规（8 维度）= 14 维度 × 2 子代理（法律合规拆分通用+纺织专项）
  ├── 第 3 批：类四面料行业深化（17 维度）= 17 维度 × 2 子代理（拆分前后端）
  ├── 第 4 批：类五运行逻辑闭环（7 维度）+ 类六测试体系（7 维度）= 14 维度 × 1 子代理
  ├── 第 5 批：类七可维护性长期治理（5 维度）+ 类九批次节奏（2 维度）= 7 维度 × 1 子代理
  ├── 第 6 批：类十色卡发放业务规则修正专项（7 维度）= 7 维度 × 1 子代理（专项深度审计）
  ├── 第 7 批：类十一大货批色业务规则专项（6 维度）= 6 维度 × 1 子代理（专项深度审计）
  ├── 第 8 批：类十二 RBAC 权限控制机制专项（8 维度）= 8 维度 × 1 子代理（专项深度审计）
  ├── 第 9 批：类十三打印导出审计与权限控制专项（10 维度）= 10 维度 × 1 子代理（专项深度审计）
  ├── 第 10 批：类十四权限维度审计与角色合理性专项（12 维度）= 12 维度 × 1 子代理（专项深度审计）
  ├── 第 11 批：类十五业务主体维度审计与数据流转专项（15 维度）= 15 维度 × 2 子代理（拆分业务主体+数据流转）
  ├── 第 12 批：类十六 AI 模块审计专项（10 维度）= 10 维度 × 2 子代理（拆分 AI 算法/权限+性能）
  ├── 第 13 批：类十七财务深化审计专项（8 维度）= 8 维度 × 1 子代理
  ├── 第 14 批：类十八 CRM 全链路审计专项（5 维度）= 5 维度 × 1 子代理
  ├── 第 15 批：类十九报表 BI 与通知协同审计专项（8 维度）= 8 维度 × 2 子代理（拆分报表 BI/通知协同）
  ├── 第 16 批：类二十可观测性与运维审计专项（8 维度）= 8 维度 × 1 子代理
  ├── 第 17 批：类二十一胚布拆匹与质量处理审计专项（5 维度）= 5 维度 × 1 子代理
  ├── 第 18 批：类二十二库存排程物料审计专项（6 维度）= 6 维度 × 1 子代理
  ├── 第 19 批：类二十三组织定制物流审计专项（5 维度）= 5 维度 × 1 子代理
  └── 第 20 批：类二十四前端架构与体验审计专项（20 维度）= 20 维度 × 4 子代理（拆分性能/安全/i18n+测试/组件+composables/路由+权限）

主代理汇总 → 生成 V15 复审报告 → 按优先级排序修复队列
```

### 4.3 审计报告格式

每份 V15 复审报告（保存到 `docs/audits/v15-review-YYYY-MM-DD.md`）必须包含：

```markdown
# V15 全项目综合复审报告（YYYY-MM-DD，规则 11-15 联动）

## 进度总览
| 类别 | 维度数 | 总发现 | P0 | P1 | P2 | P3 | 状态 |

## 一、回归验证类（5 维度）
### 🔴 P0 级（N 项 - 描述）
| # | 问题描述 | 文件位置 | 影响范围 | 修复方案 | 关联历史轮次 |

## 二、通用代码质量类（10 维度）
...（同上格式）

## 十、色卡发放业务规则修正专项（5 维度）
...（同上格式）

## 修复优先级
P0（N 项阻塞）→ P1（N 项高）→ P2（N 项中）→ P3（N 项低）

## 与 v8-v14 对比
| 轮次 | 维度数 | 总发现 | P0 | 趋势 |
```

### 4.4 修复阶段

1. **修复顺序**：P0 → P1 → P2 → P3 → 长期治理 → 色卡发放业务规则修正 → 打印导出审计补齐 → 权限维度修正 → 业务主体维度修正（含加工商功能补齐）→ AI 模块修正（含可解释性+权限+监控补齐）→ 财务深化修正（含会计期间结账+应收催收+预算闭环补齐）→ CRM 全链路修正（含线索评分+客户池回收补齐）→ 报表 BI 通知协同修正（含报表订阅+通知去重补齐）→ 可观测性运维修正（含 trace 链路+故障转移补齐）→ 胚布拆匹质量修正（含拆匹规则+8D 流程补齐）→ 库存排程物料修正（含调拨+排程算法补齐）→ 组织定制物流修正（含 incoterms 补齐）→ 前端架构体验修正（含响应式+WCAG+前端测试补齐）
2. **每批 5-8 文件**：按规则 13 流程执行（建分支 → 修改 → commit → push → PR → CI → merge → 下一批）
3. **CI 全绿后自动进入下一批**：无需用户确认
4. **复用现有功能原则**（v14 §2.2.3）：修复前必须调研现有实现，禁止重复造轮子
5. **所有警告视为错误**（规则 14）：必须真实修复，禁止 `#[allow(...)]` 抑制
6. **真实业务依据**：所有面料行业修复必须基于 [fabric-industry-research.md](file:///workspace/.monkeycode/docs/research/fabric-industry-research.md) 真实业务规则
7. **每批修复后更新**：doto.md 进度 + CHANGELOG.md 一句话总结 + 实时归档到 doto-su.md
8. **所有批次完成后进行 V15 回归验证**：确认无新增问题，触发 V16（如需）

---

## 五、V15 vs v8-v14 对比

| 维度类别 | v8-v14 最大维度数 | V15 维度数 | 提升 |
|----------|-------------------|-----------|------|
| 回归验证 | 0（无独立回归类） | 5 | ✅ 新增独立回归验证类 |
| 通用代码质量 | 12（v5） | 10 | ✅ 合并去重，聚焦核心 |
| 安全性独立审计 | 10（v5 维度 12） | 6 | ✅ 叠加规则 11/12 法律安全 |
| 面料行业深化 | 17（v14） | 17 | ✅ 17 维度全部深化（基于真实业务调研 13 章节） |
| 运行逻辑闭环 | 8（v13 规则 15） | 7 | ✅ 叠加 v14 面料行业特性 |
| 测试体系 | 49（v4 维度 12） | 7 | ✅ 综合所有测试维度独立成类 |
| 可维护性长期治理 | 36（v5 维度 15） | 5 | ✅ 聚焦长期治理 + 部署运维 |
| 法律合规与安全标准 | 0（无独立专项） | 8 | ✅ 规则 11/12 + 纺织行业法律/财税/环保/劳动合规（V15 第三轮升级） |
| 批次节奏与记忆治理 | 0（无独立维度） | 2 | ✅ 规则 5/10/13/14 持续监控 |
| **色卡发放业务规则修正** | 0（无专项） | 7 | ⭐ V15 新增（用户 2026-07-15 反复强调"只发放不借出"，含前端重构+DB 数据迁移） |
| **大货批色业务规则** | 0（无专项） | 6 | ⭐ V15 第三轮新增（用户 2026-07-15 明确"交货前客户批色，剪大货样"） |
| **RBAC 权限控制** | 0（无系统架构） | 8 | ⭐ V15 第五轮新增（用户 2026-07-15 明确"增加 RBAC 机制"，覆盖数据模型/权限矩阵/中间件/前端/审计/动态授权/数据权限/安全审计） |
| **打印导出审计与权限控制** | 0（无专项） | 10 | ⭐ V15 第六轮新增（用户 2026-07-15 明确"打印进入审计+角色权限矩阵+敏感数据二级审批"，覆盖端点合理性/角色权限矩阵/业务级审计补齐/二级审批/前端强制走后端/审计完整性/omni_audit 语义增强/水印防泄露/性能并发/合规定期审查） |
| **权限维度审计与角色合理性** | 0（无专项） | 12 | ⭐ V15 第七轮新增（用户 2026-07-15 明确"那些角色应该有什么权限，那些权限不合理，那些角色不合理，那些角色权限不匹配"，覆盖角色清单合理性/权限分配矩阵/职责分离 SoD/权限-路由匹配/is_system 滥用治理/前后端权限边界一致性/业务角色权限矩阵设计/权限粒度（行级+字段级）/权限缓存与性能/权限审计日志与合规审查/权限测试覆盖率/权限安全审计） |
| **业务主体维度审计与数据流转** | 0（无专项） | 15 | ⭐ V15 第八轮新增（用户 2026-07-15 明确"供货商/加工商/销售/客户/数据流转的功能全不全？合不合理？为什么？"，覆盖供货商主数据完整性/供货商业务闭环/供货商面料行业特性/加工商维度（完全未实现重大缺口）/加工商业务流程闭环/销售订单数据模型/销售业务流程闭环/销售面料行业特性/客户主数据完整性/客户信用与应收/客户面料行业特性/跨模块数据流转/数据流转业务回写/数据流转报表追溯/数据流转审计与异常检测） |
| **AI 模块审计** | 0（无专项） | 10 | ⭐ V15 第九轮新增（用户 2026-07-15 明确"所有维度都应该被严格审计"，覆盖 14 个 AI 模块：模型可解释性/数据安全/训练推理/权限控制/配方优化/质量预测/推荐/补货/性能/测试监控） |
| **财务深化审计** | 0（无专项） | 8 | ⭐ V15 第九轮新增（覆盖会计期间结账/辅助核算/应收催收/账龄分析/财务分析/资金管理/预算管理/固定资产） |
| **CRM 全链路审计** | 0（无专项） | 5 | ⭐ V15 第九轮新增（覆盖线索管理/商机阶段/客户池公海私海/数据权限/与销售模块数据流转） |
| **报表 BI 与通知协同审计** | 0（无专项） | 8 | ⭐ V15 第九轮新增（覆盖报表定义/订阅/BI 分析/仪表板/通知中心/邮件服务/OA 公告/五维度分析） |
| **可观测性与运维审计** | 0（无专项） | 8 | ⭐ V15 第九轮新增（覆盖 trace 链路/metrics 指标/WebSocket 推送/故障转移/慢查询/API 网关/系统版本/日志增强） |
| **胚布拆匹与质量处理审计** | 0（无专项） | 5 | ⭐ V15 第九轮新增（覆盖胚布库存采购/委托加工/拆匹缸号匹号继承/8D 质量处理/不合格品处理） |
| **库存排程物料审计** | 0（无专项） | 6 | ⭐ V15 第九轮新增（覆盖库存调拨/库存告警/物料短缺/排程算法/产能规划/工作中心调度） |
| **组织定制物流审计** | 0（无专项） | 5 | ⭐ V15 第九轮新增（覆盖部门管理/定制订单/售后/物流/incoterms 国际贸易术语） |
| **前端架构与体验审计** | 0（无专项） | 20 | ⭐ V15 第九轮新增（覆盖前端 75+ views + 17 components + 36 composables + 5 stores + 85+ api + 20 个前端独有维度：响应式/路由懒加载/Pinia/组件设计/composables/ECharts/WebSocket/性能/Vite/测试/XSS/敏感数据/WCAG/错误边界/表单/i18n/权限粒度/路由元信息/API 拦截器/主题样式） |
| **合计** | **~80**（最大单轮） | **190**（互补去重） | ✅ 24 大类 190 维度最全面 |

---

## 六、V15 审计验收标准

V15 审计完成的验收标准：

1. **24 大类 190 维度全部扫描完成**，每个维度生成独立报告
2. **总复审报告**保存到 `docs/audits/v15-review-YYYY-MM-DD.md`
3. **按优先级排序修复队列**（P0 → P1 → P2 → P3 → 色卡发放修正 → 大货批色修正 → 纺织合规修正 → RBAC 权限修正 → 打印导出审计补齐 → 权限维度修正 → 业务主体维度修正含加工商功能补齐 → AI 模块修正 → 财务深化修正 → CRM 全链路修正 → 报表 BI 通知协同修正 → 可观测性运维修正 → 胚布拆匹质量修正 → 库存排程物料修正 → 组织定制物流修正 → 前端架构体验修正）
4. **修复阶段**严格按规则 13 流程执行，每批 5-8 文件
5. **CI 全绿后自动进入下一批**，无需用户确认
6. **所有警告视为错误**，必须真实修复（规则 14）
7. **所有面料行业修复**必须基于真实业务调研（fabric-industry-research.md）
8. **复用现有功能原则**：禁止重复造轮子（v14 §2.2.3）
9. **回归验证**：v8-v14 所有已修复项无回退
10. **色卡发放业务规则修正**：完成"借还模式"→"发放模式"重构（类十）
11. **大货批色业务规则实现**：完成"剪大货样→客户批色→交货门禁"全流程（类十一）
12. **纺织行业法律财税合规审计**：完成法律法规/财税/环保/劳动四维合规扫描（类八 8.5-8.8）
13. **项目规则与个人规则符合性**：所有修复严格遵守 project_rules.md + MEMORY.md + 个人规则
14. **RBAC 权限控制机制实现**：完成 RBAC 四层模型（用户→角色→权限→资源）+ 权限矩阵 + 权限中间件 + 前端集成 + 审计日志 + 动态授权 + 数据权限 + 安全审计（类十二）
15. **打印导出审计与权限控制实现**：完成 print/export 全链路审计 + 角色级权限矩阵 + 敏感数据二级审批 + 前端本地导出强制走后端 + 文件水印防泄露 + 合规定期审查（类十三）
16. **权限维度审计与角色合理性实现**：完成角色清单合理性审计 + 14 个缺失业务角色补齐 + 权限分配矩阵（权限过大/过小识别）+ 职责分离 SoD（role_conflict 表 + 互斥校验）+ 权限-路由匹配审计（60+ 类缺失权限资源补齐）+ is_system 滥用治理（manager/operator 取消 is_system=true）+ 前后端权限边界一致性 + 业务角色权限矩阵设计（销售/采购/库存/生产/财务 6 域）+ 权限粒度（行级 apply_data_scope + 字段级 field_permission）+ 权限缓存与性能（5min TTL + Redis pub/sub）+ 权限审计日志（permission_change_audit 表 + 每周合规审查 cron）+ 权限测试覆盖率（30+ 单元测试）+ 权限安全审计（提升/绕过/注入漏洞矩阵）（类十四）
17. **业务主体维度审计与数据流转实现**：完成供货商主数据完整性修正（category_id 落地 + migration 补齐 + 资质 update/delete + 导入导出）+ 供货商业务闭环补齐（评估自动触发 cron + 账户余额 + 供货历史 + 价格清单导入）+ 供货商面料行业特性补齐（染色/印花/色卡能力字段）+ **加工商功能完全补齐**（is_processor 标志 + outsourcing_orders 表 + outsourcing_receipts 表 + processor_payments 表 + Service + Handler + 路由 + 前端）+ 加工商业务流程闭环（外发→核算→收回→损耗→付款→进度→缸号关联→报表）+ 销售合同明细行表补齐 + 销售面料行业特性优化（按匹号发货）+ 客户多地址/多银行表（可选）+ 客户信用评级自动触发 cron + 客户特殊工艺要求字段 + 数据流转染色→质检→入库监听器业务回写补齐 + business_traces 写入 Service + 主动异常检测引擎 + 数据流转异常告警（类十五）
18. **AI 模块审计与补齐实现**：完成 14 个 AI 模块的可解释性补齐（explanation/confidence_score/factors 字段 + 模型版本 + ai_decision_log 表 + 人工干预机制）+ 数据安全补齐（训练数据脱敏 + 推理数据最小化 + 中间结果加密）+ 权限控制补齐（14 个 AI 端点权限码注册 + 数据范围过滤 + 调用审计）+ 配方优化与化验室打样集成 + 质量预测准确率监控（≥80%）+ 推荐多样性（diversity_score ≥0.3）+ 补货与 MRP 集成对账 + 接口并发控制（max_concurrent=10）+ 缓存策略（TTL 5min）+ 单测覆盖率 ≥70% + Grafana 监控看板 + 告警机制（类十六）
19. **财务深化审计与补齐实现**：完成会计期间状态机（open→closing→closed→reopened）+ 月结/年结流程 + 跨期凭证处理 + 结账锁定机制 + 辅助核算主辅账平衡校验 + 报表穿透 + 应收催收自动任务分配 + 坏账准备计提 + 坏账核销二级审批 + 账龄期末快照 + 与总账对账 + 杜邦分析 3 层分解 + 财务预警机制 + 资金预测模型 + 大额调拨额外验证 + 预算执行控制 + 差异分析 + 折旧自动计提 + 资产处置流程 + 资产盘点闭环（类十七）
20. **CRM 全链路审计与补齐实现**：完成线索评分模型 + 转化漏斗报表 + 线索来源追踪 + 重复去重 + 商机阶段状态机 + 赢率自动计算 + 输单原因分析 + 客户池公海私海规则 + 自动回收规则 + 公海领取限制 + CRM 数据权限 + 团队协作 + 客户转移审批 + 线索→客户/商机→报价/报价→订单转化 + 数据一致性校验（类十八）
21. **报表 BI 与通知协同审计与补齐实现**：完成报表元数据完整性 + 模板版本管理 + 参数校验 + 权限控制 + 订阅权限校验 + 定时推送 + 失败重试 + 退订机制 + BI 多维分析 + 数据缓存 + 数据权限 + 仪表板自定义 + 实时刷新 + 通知多渠道 + 去重 + 已读未读 + 模板管理 + SMTP 加密 + 异步队列 + 失败重试 + OA 公告权限 + 用户行为隐私合规 + 五维度数据归集 + 页面浏览归档（类十九）
22. **可观测性与运维审计与补齐实现**：完成 trace 链路完整性 + 采样策略 + 数据保留 + metrics 7 类核心指标 + 告警规则 + 分级 + 通知去重 + Grafana 看板 + WebSocket ACK 机制 + 多实例广播 + 鉴权安全 + 故障检测 + 主备切换 + 数据同步 + 人工回切 + 慢查询告警 + 优化追踪 + 报表 + API 网关路由 + 限流 + 熔断 + 鉴权 + 版本号管理 + 灰度升级 + 回滚脚本 + 向后兼容 + 日志分级 + 脱敏 + 结构化 + 归档（类二十）
23. **胚布拆匹与质量处理审计与补齐实现**：完成胚布独立库存模型 + 采购流程 + 安全库存预警 + 批次追溯 + 委托加工走委外流程 + 损耗核算 + 质量追溯 + 加工费核算 + 拆匹缸号继承 + 匹号生成 + 数量校验 + 历史追溯 + 8D 流程完整性 + 根因分析 + 纠正预防措施跟踪 + 8D 月报 + 不合格品分类 + 降级处理 + 返工流程 + 报废流程（类二十一）
24. **库存排程物料审计与补齐实现**：完成调拨流程闭环 + 跨库位调拨 + 跨缸号调拨 + 分级审批 + 安全库存设置 + 补货策略 + 告警通知 + 去重 + 物料短缺识别 + 分级 + 处理闭环 + 月报 + 排程算法 + 冲突检测 + 可视化 + 与生产集成 + 产能模型 + 负荷告警 + 瓶颈识别 + 月报 + 工作中心模型 + 调度规则 + 排程下发 + 异常重排（类二十二）
25. **组织定制物流审计与补齐实现**：完成部门树形结构 + 权限关联 + 用户关联 + 变更审计 + 定制订单流程 + 专属质量标准 + 客户确认 + 全链路追溯 + 售后 4 类工单 + 流程闭环 + 原因分析 + 与质量集成 + 运单管理 + 物流跟踪 + 运费核算 + 电子签收 + incoterms 2020 11 种术语 + 术语与价格集成 + 责任划分 + 术语报表（类二十三）
26. **前端架构与体验审计与补齐实现**：完成响应式断点 + 移动端布局 + PWA + 移动端性能 + 路由懒加载率 100% + chunk 体积监控 + 首屏性能 + prefetch + Pinia 模块化 + 持久化 + 跨模块通信 + 测试 + Props/Emits 类型安全 + 组件复用 + 文档 + composables 响应式 + 内存清理 + 错误处理 + 命名规范 + ECharts 大数据性能 + resize + dispose + 按需引入 + WebSocket 客户端重连 + 心跳 + 票据鉴权 + bundle 体积 + Tree Shaking + 防抖节流 + 内存泄漏 + Vite 配置 + Code Splitting + Source Map + 环境变量 + Vitest 单测 70% + 组件测试 + E2E + mock fixtures + v-html 审计 + DOMPurify + CSP + Clickjacking + localStorage 扫描 + token 安全 + 密钥泄露 + 开放重定向 + 键盘导航 + 焦点管理 + ARIA + 色彩对比度 + ErrorBoundary + 全局错误处理 + 监控接入 + 错误去重 + 表单校验 + 异步校验 + 防重复提交 + 脏数据检测 + 资源文件同步 + 语言切换 + Intl 格式化 + RTL + 按钮级权限 + 字段级权限 + 行级权限 + 缓存刷新 + meta 完整性 + 动态路由 + keep-alive + breadcrumb + 请求/响应拦截器 + 请求取消 + 超时重试 + CSS 变量 + 暗黑模式 + 样式作用域 + 主题切换（类二十四）
27. **触发 V16**：V15 全部修复完成后视项目状态决定是否需要

---

## 七、关联文档

- [project_rules.md](file:///workspace/.trae/rules/project_rules.md) — 项目开发规范（项目规则）
- [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md) — 项目规则记忆（规则 11-15）
- [audit_assignment.md](file:///workspace/.monkeycode/audit_assignment.md) — 审计任务分配和复审规则
- [doto.md](file:///workspace/.monkeycode/doto.md) — 未完成任务（v14 批次 425-432）
- [fabric-industry-research.md](file:///workspace/.monkeycode/docs/research/fabric-industry-research.md) — 面料行业真实业务调研（13 章节）
- 历史复审报告：
  - [v8-review-2026-07-11.md](file:///workspace/.monkeycode/docs/audits/v8-review-2026-07-11.md)
  - [v9-review-2026-07-12.md](file:///workspace/.monkeycode/docs/audits/v9-review-2026-07-12.md)
  - [v13-review-2026-07-13.md](file:///workspace/.monkeycode/docs/audits/v13-review-2026-07-13.md)
  - [2026-06-18-db-n1-audit.md](file:///workspace/.monkeycode/docs/audits/2026-06-18-db-n1-audit.md)
  - [2026-07-08-batch190-e2e-report.md](file:///workspace/.monkeycode/docs/audits/2026-07-08-batch190-e2e-report.md)
  - v4/v5/v7 严格审计报告（归档）
- 色卡相关代码（V15 类十修正对象）：
  - [color_card_borrow_service.rs](file:///workspace/backend/src/services/color_card_borrow_service.rs)
  - [color_card_crud_service.rs](file:///workspace/backend/src/services/color_card_crud_service.rs)
  - [color_card_borrow_record.rs](file:///workspace/backend/src/models/color_card_borrow_record.rs)
  - [color_card_borrow_dto.rs](file:///workspace/backend/src/models/color_card_borrow_dto.rs)
  - [handlers/color_card/borrow.rs](file:///workspace/backend/src/handlers/color_card/borrow.rs)
  - [routes/color_card.rs](file:///workspace/backend/src/routes/color_card.rs)
- 大货批色相关代码（V15 类十一新增对象）：
  - `backend/src/models/bulk_color_approval.rs`（新增模型，V15 规划）
  - `backend/src/services/bulk_color_approval_service.rs`（新增服务，V15 规划）
  - `backend/src/handlers/bulk_color_approval/`（新增 handler，V15 规划）
  - `backend/migration/xxxx_create_bulk_color_approval_table.sql`（新增迁移，V15 规划）
  - 销售出库 handler（交货门禁接入点）：`backend/src/handlers/sales/delivery.rs`
  - 库存模块（剪样扣减接入点）：`backend/src/services/inventory/`
  - 生产订单模块（剪样触发点）：`backend/src/services/production/`
- 纺织行业法律财税合规相关代码（V15 类八 8.5-8.8 审计对象）：
  - 财务模块：`backend/src/services/finance/`（增值税/委托加工物资/出口退税/环保税凭证）
  - 人事模块：`backend/src/services/hr/`（劳动合同/计件工资/社保）
  - 能耗模块：`backend/src/services/energy/`（v14 批次 431 实现，环保税核算基础）
- RBAC 权限控制相关代码（V15 类十二审计对象）：
  - 权限模型：`backend/src/models/role.rs` + `backend/src/models/permission.rs` + `backend/src/models/role_permission.rs` + `backend/src/models/user_role.rs`（V15 新增）
  - 权限服务：`backend/src/services/permission_service.rs`（V15 新增）
  - 权限中间件：`backend/src/middleware/permission.rs`（V15 新增）
  - 权限审计日志：`backend/src/models/permission_audit_log.rs`（V15 新增）
  - 前端权限指令：`frontend/src/directives/permission.ts`
  - 前端路由守卫：`frontend/src/router/guards.ts`
  - 前端用户状态：`frontend/src/stores/user.ts`
- 权限维度审计相关代码（V15 类十四审计对象）：
  - schema 文件：`backend/migration/0001_init_schema.sql`（roles/permissions/role_permissions 初始结构）+ `backend/migration/0014_init_role_permissions.sql`（角色权限初始化，引用不存在的 role_id=4/5 静默失败）+ `backend/migration/0025_*.sql`（权限相关扩展）
  - 角色初始化：`backend/src/services/init_service.rs`（admin/manager/operator/customer 角色初始化 + is_system=true 滥用点）
  - 认证 handler：`backend/src/handlers/auth_handler.rs`（build_with_permissions 函数 is_system 注入 `*:*` 逻辑）
  - 权限校验：`backend/src/middleware/permission.rs`（require_permission 函数 + 资源/动作匹配 + 数据权限过滤）
  - 路径工具：`backend/src/utils/path_utils.rs`（模块前缀白名单仅 28 个，路由暴露 70+ 类资源）
  - 角色/权限模型：`backend/src/models/role.rs` + `backend/src/models/permission.rs` + `backend/src/models/role_permission.rs`
  - 职责分离：`backend/src/models/role_conflict.rs`（V15 新增 SoD 互斥表）+ `backend/src/services/role_conflict_service.rs`（V15 新增）
  - 行级数据权限：`backend/src/utils/data_scope.rs`（V15 新增 apply_data_scope 函数）
  - 字段级权限：`backend/src/models/field_permission.rs`（V15 新增）
  - 权限缓存：`backend/src/utils/permission_cache.rs`（V15 新增 5min TTL + Redis pub/sub 热更新）
  - 权限变更审计：`backend/src/models/permission_change_audit.rs`（V15 新增 + 每周合规审查 cron）
- 业务主体维度审计相关代码（V15 类十五审计对象）：
  - 供货商：`backend/src/models/supplier.rs` + `backend/src/models/supplier_category.rs` + `backend/src/models/supplier_qualification.rs` + `backend/src/models/supplier_grade.rs` + `backend/src/models/supplier_blacklist.rs` + `backend/src/models/supplier_contact.rs` + `backend/src/models/supplier_product.rs` + `backend/src/models/supplier_product_color.rs` + `backend/src/models/product_supplier_mapping.rs` + `backend/src/models/purchase_price.rs` + `backend/src/models/supplier_evaluation.rs` + `backend/src/models/supplier_evaluation_record.rs` + `backend/src/models/ap_reconciliation.rs` + `backend/src/services/supplier_service.rs` + `backend/src/services/supplier_evaluation_service.rs` + `backend/src/handlers/supplier_handler.rs` + `backend/src/handlers/supplier_evaluation_handler.rs`
  - 加工商（V15 新增）：`backend/src/models/outsourcing_order.rs`（新增）+ `backend/src/models/outsourcing_receipt.rs`（新增）+ `backend/src/models/processor_payment.rs`（新增）+ `backend/src/services/outsourcing_service.rs`（新增）+ `backend/src/handlers/outsourcing_handler.rs`（新增）+ `backend/src/routes/outsourcing.rs`（新增）
  - 销售：`backend/src/models/sales_order.rs` + `backend/src/models/sales_order_item.rs` + `backend/src/models/sales_quotation.rs` + `backend/src/models/sales_contract.rs` + `backend/src/models/sales_return.rs` + `backend/src/services/so/order_workflow.rs` + `backend/src/services/so/delivery.rs` + `backend/src/services/sales_return_service.rs` + `backend/src/services/sales_analysis_service.rs` + `backend/src/services/quotation_convert_service.rs` + `backend/src/handlers/sales_order_handler.rs`
  - 客户：`backend/src/models/customer.rs` + `backend/src/models/customer_credit.rs` + `backend/src/models/customer_contact.rs` + `backend/src/models/customer_color_price.rs` + `backend/src/services/customer_service.rs` + `backend/src/services/customer_credit_limit.rs` + `backend/src/services/customer_credit_service.rs` + `backend/src/services/customer_credit_evaluate.rs` + `backend/src/handlers/customer_handler.rs` + `backend/src/services/ar/inv.rs`
  - 数据流转：`backend/src/services/event_bus.rs` + `backend/src/services/inventory_finance_bridge_service.rs` + `backend/src/models/event_dead_letter.rs` + `backend/src/models/processed_event.rs` + `backend/src/models/business_trace.rs` + `backend/src/models/operation_log.rs` + `backend/src/models/omni_audit_log.rs` + `backend/src/services/dye_batch_cost_bridge_service.rs`

---

## 八、审计计划变更记录

| 日期 | 变更内容 | 变更者 |
|------|----------|--------|
| 2026-07-15 | V15 审计计划首版创建，综合 v8-v14 + DB N+1 + E2E + 面料行业调研 9 大类 56 维度 | 主代理 |
| 2026-07-15 | 用户反馈"色卡只发放给客户，不借出"，新增类十色卡发放业务规则修正专项（5 维度），V15 升级为 10 大类 58 维度 | 主代理 |
| 2026-07-15 | 用户第三轮反馈：①要求符合项目规则和个人规则 ②补充纺织行业法律和财税合规 ③交货前客户批色（剪大货样批色）。新增类八 8.5-8.8 纺织行业法律/财税/环保/劳动合规 4 维度 + 类十一大货批色业务规则专项 6 维度，V15 升级为 11 大类 68 维度 | 主代理 |
| 2026-07-15 | 用户第四轮反馈"V15 计划要特别详细，色卡只发放不借出"：类十从 5 维度深化到 7 维度，新增 10.6 前端重构详细规范（Vue 3 类型定义/API/composables/views/路由/权限指令）+ 10.7 DB 数据迁移脚本（旧借还记录数据迁移到发放记录 + 回滚方案）。类十 10.1-10.5 全面深化为代码级实现规范（完整 SQL/SeaORM Model/DTO/Service/Handler/路由代码骨架 + 校验矩阵 + 权限矩阵 + cron 配置 + 单元测试清单 23 项）。V15 升级为 11 大类 70 维度 | 主代理 |
| 2026-07-15 | 用户第五轮反馈"增加基于角色的权限控制机制"：新增类十二 RBAC 权限控制机制专项（8 维度）：12.1 数据模型与权限架构（RBAC 四层模型 + 关联表 DDL + 12 角色层级 + 权限码命名规范）/12.2 权限矩阵与最小权限原则（权限矩阵完整性 + 最小权限 + 继承互斥 + 默认拒绝 + 粒度控制）/12.3 权限校验中间件（Axum middleware + 数据权限过滤 + 字段级权限）/12.4 前端权限集成（路由守卫 + v-permission + 菜单动态加载 + 403 处理）/12.5 权限审计日志（变更审计 + 校验日志 + 审计日志表 DDL + 保留期限 + 查询接口）/12.6 动态授权与委托（动态分配 + 委托机制 + 缓存失效 + 热更新 + 变更审批）/12.7 数据权限（行级 RLS + 字段级 + 业务结合）/12.8 RBAC 安全审计（权限提升防护 + IDOR 防护 + 权限绕过防护 + 会话固定防护 + TOCTOU 并发防护 + 配置审计 + 压力测试）。V15 升级为 12 大类 78 维度 | 主代理 |
| 2026-07-15 | 用户第六轮反馈"打印也需要进入审计，那些地方需要进行打印，合不合理？那些地方不许打印，为什么？什么角色不能打印，什么角色不能导出，审计记录全不全等等都需要审计"：基于完整代码扫描（13 个后端 print/export handler + 25+ 个前端本地导出按钮）新增类十三打印导出审计与权限控制专项（10 维度）：13.1 端点合理性审计（14 现有端点合理性矩阵 + 7 缺失端点清单 + 11 前端本地导出合理性）/13.2 角色权限矩阵（14 角色 × 13 操作权限表 + 禁止打印/导出角色清单 + 19 个 print/export 权限码 SQL + method_to_action 升级代码）/13.3 业务级审计补齐（OperationType::Print/Download 新增 + 10 handler 审计补齐清单 + audit_logs 表扩展 5 字段 + 完整性校验矩阵 7 项）/13.4 敏感数据二级审批（审批流程图 + export_approval_request 表 DDL + 水印代码 + 7 资源禁止规则）/13.5 前端本地导出强制走后端（export.ts/print.ts 重构代码 + v-permission 升级 + 25+ 页面改造清单 + 保留 window.print 场景审计埋点）/13.6 审计日志完整性审计（15 端点 × 8 字段完整性矩阵 + P0/P1/P2 修复优先级 + 审计补齐代码模板）/13.7 omni_audit 中间件语义增强（classify_operation 代码 + omni_audit_logs 表扩展 + 审计报表分类查询 SQL）/13.8 文件水印与防泄露（4 格式水印规范 + build_xlsx_with_watermark 代码 + Watermark 结构体）/13.9 性能与并发控制（9 资源导出上限 + AtomicUsize 并发控制代码 + StreamBody 流式导出代码）/13.10 合规审计与定期审查（6 异常模式检测规则 + 每日合规审查 cron 代码 + 4 类审计日志保留期限 + 审计日志导出二次审计表）。V15 升级为 13 大类 88 维度最终版 | 主代理 |
| 2026-07-15 | 用户第七轮反馈"添加权限维度的审计，那些角色应该有什么权限，那些权限不合理，那些角色不合理，那些角色权限不匹配？为什么？"：基于完整 RBAC 代码扫描（schema 001/014/025 + init_service.rs + auth_handler.rs + permission.rs + path_utils.rs + 前端路由和指令）新增类十四权限维度审计与角色合理性专项（12 维度）：14.1 角色清单合理性审计（现有角色合理性矩阵 + 14 个缺失业务角色补齐清单 + 角色命名规范）/14.2 权限分配矩阵审计（8 项问题矩阵 + 14×11 目标权限矩阵 + 权限过大/过小识别规则代码）/14.3 职责分离 SoD 审计（8 项职责冲突矩阵 + role_conflict 表 DDL + 互斥校验 Rust 代码）/14.4 权限-路由匹配审计（60+ 类缺失权限资源 + 权限码不匹配问题 + 模块前缀白名单缺口）/14.5 is_system 滥用治理（build_with_permissions 修正代码 + SQL 修复脚本，manager/operator 取消 is_system=true）/14.6 前后端权限边界一致性审计（4 项不一致场景 + 修复方案 A/B）/14.7 业务角色权限矩阵设计审计（销售/采购/库存/生产/财务/其他 6 个域的完整权限矩阵）/14.8 权限粒度审计（行级数据权限 apply_data_scope 代码 + 字段级权限 SQL）/14.9 权限缓存与性能审计（缓存问题矩阵 + invalidate_user_permission_cache 代码 + 5min TTL + Redis pub/sub 热更新）/14.10 权限审计日志与合规审查（permission_change_audit 表 DDL + 6 项异常检测规则 + 每周合规审查 cron）/14.11 权限测试覆盖率审计（10 类测试缺口 + 30+ 单元测试清单）/14.12 权限安全审计（权限提升/绕过/注入漏洞矩阵）。V15 升级为 14 大类 100 维度最终版 | 主代理 |
| 2026-07-15 | 用户第八轮反馈"添加供货商维度/加工商维度/销售维度/客户维度/数据流传的审计，他们的功能全不全？功能完整吗？有没有需要补充的？合不合理？为什么？"：基于完整代码扫描（supplier_service.rs + supplier_evaluation_service.rs + purchase_order.rs + ap_reconciliation.rs + sales_order.rs + sales_order_item.rs + order_workflow.rs + delivery.rs + sales_return_service.rs + customer_service.rs + customer_credit_limit.rs + customer_credit_evaluate.rs + event_bus.rs + inventory_finance_bridge_service.rs + business_trace.rs）新增类十五业务主体维度审计与数据流转专项（15 维度）：15.1 供货商主数据完整性审计（suppliers 主表 + 7 张关联表 + schema/model 命名不一致 + migration 缺失 + 分类未落地 + 资质管理不完整 + 导入导出缺失）/15.2 供货商业务闭环审计（8/12 完整：创建/更新/删除/采购关联/对账单/等级评估 完整，评估自动触发/账户余额/供货历史/价格清单导入 缺失）/15.3 供货商面料行业特性审计（supplier_type 区分染料/助剂/坯布 合理，色卡能力/染色能力/印花能力字段 缺失不合理）/15.4 加工商维度审计（**完全未实现**重大功能缺口：无独立表/is_processor/委外加工单/加工费核算/收回入库/损耗/Service/Handler/路由/前端，含完整 4 表 DDL 设计：outsourcing_orders + outsourcing_receipts + processor_payments + suppliers 扩展）/15.5 加工商业务流程闭环审计（0/8 打通 0%：外发/核算/收回/损耗/付款/进度/缸号关联/报表 全部缺失）/15.6 销售订单数据模型与状态机审计（主表+明细+8 态状态机+报价单 完整，销售合同缺明细行表 不合理，销售预测 未实现可选）/15.7 销售业务流程闭环审计（12/12 完整 100%：报价→订单→发货→收款→退货闭环 + TOCTOU 防护 + 双单位换算 + 对称恢复 + BPM 补偿）/15.8 销售面料行业特性审计（5/6 完整 83%：缸号校验/双单位/等级价差/纸管重量 完整，按匹号发货 部分可改进）/15.9 客户主数据完整性审计（customers+信用+联系人+色卡价格+行业/质量标准 完整，多地址/多银行表 缺失可选）/15.10 客户信用与应收管理审计（11/12 完整：信用额度/占用/释放/调整/检查/预警/停用/评级算法/AR 关联/PDF/ES 合理，评级自动触发 cron 缺失不合理）/15.11 客户面料行业特性审计（6/8 完整 75%：分级/色卡价格/报价定价/行业/质量标准 完整，批色确认/色卡档案 已规划，特殊工艺要求 缺失可选）/15.12 跨模块数据流转审计（销售/采购/生产 三链路全通 + 事件总线 21 事件/双后端/幂等/死信/panic 隔离/事务一致性/TOCTOU/行锁/冗余刷新/ES 同步/BPM 补偿 合理，染色→质检→入库监听器仅日志无回写 不合理）/15.13 数据流转业务回写审计（库存财务桥接 7 种类型幂等 完整，business_traces 表模型存在无写入 不合理，DyeBatchCompleted/QualityInspectionCompleted 仅日志无回写 不合理）/15.14 数据流转报表与追溯审计（销售分析/AP 账龄/业财一体化/PDF/CSV/财务指标 完整，离线 ETL 未实现可选，business_traces 写入缺失不合理）/15.15 数据流转审计与异常检测审计（操作日志/omni_audit/事务内审计/死信审计/幂等审计 完整，business_traces 写入缺失/主动异常检测引擎/异常告警 缺失不合理）。V15 升级为 15 大类 115 维度最终版 | 主代理 |
| 2026-07-15 | 用户第九轮反馈"调研项目还有什么维度没有被审计到，所有维度都应该被严格审计"：基于后端完整模块扫描（services 130+/handlers 140+/models 180+/middleware 17/utils 34）+ 前端完整模块扫描（views 85+/components 17/composables 36+/store 6/api 90+）识别出**后端完全未覆盖 19 个模块 + 部分覆盖 54 个模块 + 前端覆盖率 < 5% + 20 个前端独有维度全部缺失**，新增 9 个新类别共 75 维度：类十六 AI 模块审计专项（10 维度：覆盖 14 个 AI 模块 ai_process_optimization/ai_quality_prediction/ai/{detect,pred,rec,recipe_opt}/ai_extend_service/advanced/{analytics,decide,forecast,quality_pred,rec,recipe_opt,reorder}，含模型可解释性/数据安全/训练推理/权限控制/配方优化/质量预测/推荐/补货/性能/测试监控）/类十七财务深化审计专项（8 维度：会计期间结账/辅助核算/应收催收/账龄/财务分析/资金管理/预算/固定资产）/类十八 CRM 全链路审计专项（5 维度：线索/商机/客户池/数据权限/与销售模块数据流转）/类十九报表 BI 与通知协同审计专项（8 维度：报表定义/订阅/BI/仪表板/通知中心/邮件/OA 公告/五维度分析）/类二十可观测性与运维审计专项（8 维度：trace/metrics/WebSocket/failover/slow_query/api_gateway/system_version/log）/类二十一胚布拆匹与质量处理审计专项（5 维度：胚布库存/委托加工/拆匹规则/8D 流程/不合格品处理）/类二十二库存排程物料审计专项（6 维度：库存调拨/库存告警/物料短缺/排程算法/产能规划/工作中心）/类二十三组织定制物流审计专项（5 维度：部门/定制订单/售后/物流/incoterms 国际贸易术语）/类二十四前端架构与体验审计专项（20 维度：响应式/路由懒加载/Pinia/组件设计/composables/ECharts/WebSocket 客户端/前端性能/Vite 构建/前端测试/XSS 防护/敏感数据/WCAG 可访问性/错误边界/表单验证/i18n 深化/权限粒度/路由元信息/API 拦截器/主题样式/虚拟列表）。同步更新：核心目标（24 大类 190 维度）+ 互补逻辑（13-21 层）+ 维度全景图（24 类树状图）+ 触发条件（20 项前置）+ 执行方式（20 批并行子代理）+ 修复顺序（含 9 类补齐）+ 对比表（新增 9 行）+ 验收标准（27 项）+ 修订记录（第九轮）。V15 升级为 24 大类 190 维度最终版 | 主代理 |
