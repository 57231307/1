# 更新日志（.monkeycode 版本）

> 本文件是 `/workspace/CHANGELOG.md` 的精简版，记录任务总结。
> 原文件包含完整的项目变更历史，本文件保留关键任务执行记录。

---

## 文件来源

| 文件 | 用途 | 说明 |
|------|------|------|
| `/workspace/CHANGELOG.md` | 完整更新日志 | 包含所有项目变更的详细记录 |
| `.monkeycode/CHANGELOG.md` | 任务总结精简版 | 记录 doto.md 的任务总结 |

---

## 最新任务总结

### Wave 1+2+3 修复（2026-06-19）

- **P0 - 3 个孤儿 migration 注册**：m0025/26/27 重命名 + lib.rs pub mod + Box::new（修复审计增强 + 慢查询审计）
- **P1 - 删除孤立目录**：mobile/ (17) + microservices/ (13) + deploy/{elasticsearch,grafana,helm,kafka,observability,prometheus}/ (24)
- **P2 - 删除 8 个空子目录**：.monkeycode/docs/{api,superpowers/reports,poc,requirements,db,专有概念,模块,releases}
- **变更**：1 修改 + 30 删除 = 31 文件
- **CI/CD 验证**：遵循"禁止本地编译"规则，仅依赖 GitHub Actions

### 推送 main + 清理根 CHANGELOG/MEMORY（2026-06-19）

- **删除**：`chore: 删除 test 合入的根 CHANGELOG.md / MEMORY.md`（2 文件 -1941 行）
- **原因**：与 .monkeycode/ 记忆体系重复，统一以 .monkeycode/ 为唯一记忆系统
- **最终 main HEAD**：`b99ec30`

### I-3 第 6 批合入 + feature 分支清理（2026-06-19）

- **cherry-pick**：`git cherry-pick -X theirs e4ba11d` 单点合入 p14 分支唯一提交，41 文件 +3600/-2421 行
- **拆分成果**：capacity 562→116 / Dashboard 549→99 / security 547→101 / TwoFactorSetup 540→2-factor 子目录 / sales-analysis 535→106
- **I-3 累计**：I-1 (3) + I-2 (3) + I-3 第 1~6 批 (23) = **29 个大 .vue 全部完成**
- **远端清理**：删除 2 个 feature 分支（p14 合并后冗余、p12 过期）→ 远端仅剩 main

### test 合并入 main（2026-06-19）

- **合并方式**：`git merge -X theirs origin/test`，81 个 UA 冲突以 test 版本为准，merge commit `3116afa`
- **.monkeycode/ 恢复**：用户要求"使用 main 的 .monkeycode 目录"→ 从 `main-backup-20260619-pre-testmerge` 标签 checkout 恢复，删除 100 个 test 独有文档，commit `19fb82f`（+143/-46049 行）
- **test 分支删除**：远端 `git push origin --delete test` + 本地 `git branch -rd origin/test` 完成清理
- **保留 test 内容**：mobile/ 目录、microservices/ 目录、P0~P9 业务功能、根 CHANGELOG.md、根 MEMORY.md
- **撤销兑底**：`main-backup-20260619-pre-testmerge` 标签保留可回退至合并前状态

### docs 合并 + main 同步（2026-06-19）

- **docs 整合**：将 3 个源 docs 目录（`/workspace/docs`、`/workspace/backend/docs`、`/workspace/frontend/docs`）移动到 `/workspace/.monkeycode/docs`，共 91 个文件，无冲突
- **main 同步**：远端已包含 `a0a25e8 chore: 合并 /workspace/docs 到 .monkeycode/docs`（自动化或外部提交），与本地 `390f101 feat: 项目评估` 形成分叉
- **解决方式**：`git pull --no-rebase` + `git push`，最终 merge commit `fb1d331`，**未使用强制推送**（保留远端所有历史）
- **关键经验**：用户口头"强制推送"在前端检查时本不需要；fetch 后才暴露分叉，最终选 merge 策略避免数据丢失

### P14 批 2 B3 拆分大 .vue（2026-06-19）

- **PR #195 ~ #199**：5 个 PR 全部 squash merge 入 main
- **累计进展**：18/24 大 .vue 已拆分
- **拆分成果**：
  - PR #195：VoucherListTab 870→141 + system-update 725→154 + sales-contract 717→129
  - PR #196：purchase-return 695→211 + scheduling/gantt 691→93 + scheduling/index 689→109
  - PR #197：sales-price 677→147 + OrderListView 644→125 + purchase-contract 644→142 + purchase-price 622→137
  - PR #198：bpm/approval 618→123 + production 611→172 + logistics 605→117 + purchaseReceipt 598→97
  - PR #199：data-import 596→127 + purchase-inspection 594→113 + material-shortage 590→85 + bpm/definitions 579→150
- **经验沉淀**：
  - composable 用 reactive({...}) 包装 return
  - v-model 不能用于 prop，必须用 :model-value + @update:model-value + emit
  - string/number/boolean 类型 prop 是 readonly，必须用 emit 模式

### P13 批 1（2026-06-18）

- **PR #191**：P3-2 审计日志增强（6 commit，CI 5 轮迭代）
- **PR #192**：B-慢查询审计（3 commit，CI 2 轮迭代）
- **PR #193**：B3 拆分大 .vue I-1（5 commit，CI 4 轮迭代）
- **P13 批 1 全部 3/3 PR 完成**

### P12 批 1+2+3（2026-06-17 ~ 2026-06-18）

- **12/12 PR 全部完成**
- P0 销售报价单端到端贯通（4 PR 串行）
- P2-1 V2Table 全面替代老 el-table（5 PR）
- P2-2 性能优化落地（Redis 缓存层 + DB N+1 审计）
- B-type-check CI 5 job（vue-tsc 真正起到拦截作用）
- P3-1 前端安全加固（TOTP 2FA + 修改密码 + 密码强度可视化）

### Wave 1-3（2026-06-15）

- **Wave 1**：4 PR 100% 合并（P0-2 销售→AR / P2-3 编译验证 / P1-1 generate-no / P1-5 入库单明细）
- **Wave 2**：6/6 完成（B3-1~4 拆分大 .vue + B5 POC + B6 清理）
- **Wave 3**：11 PR 100% 合并（B7 console.* 清理 + type-check 清理 + AI 深化）

---

## 关键经验

### TypeScript
- 对象字面量 excess property check 每次只报告第一个未知属性
- `String(e)` 转换是 unknown → string 的标准模式
- `vue-tsc` 不要带 `-b`（与 noEmit 冲突）

### Rust
- 项级 `#[allow(dead_code)]` + TODO(tech-debt) 是合规做法
- SeaORM 自动生成模型保留文件级抑制
- 子代理串行调度避免云端卡死

### Git
- worktree 占用导致本地分支无法删除：先 `git checkout main` 切到 main，再 `git branch -D`
- GitHub squash merge 后远端分支自动删除

### CI/CD
- 所有验证通过 `.github/workflows/ci-cd.yml`
- 后端 4 检查：clippy / build / fmt / test
- 前端 3 检查：build / test / lint
- 推送后等 CI 全绿（绿色 ✓）才算成功

---

## 完整变更历史

完整的项目变更历史请查看：`/workspace/CHANGELOG.md`
