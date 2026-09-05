# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单），进度必须真实，禁止乐观偏差。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。

---

## 当前状态

**全部审计任务已完成（268/268，100%）。**

### 2026-09-05 新增

- [ ] 观察 PR #937 下一轮 CI 的 E2E"登录卡死 120s"分片（#11/#12/#13 曾复现）：已修复 401 刷新死锁（请求自等待）并加登录响应状态日志；若仍复现，依据日志区分"登录请求慢/失败"与"跳转未发生"继续定位
- [ ] 匹号领域功能（需求已由用户确认，设计文档 docs/piece-number-domain-design.md）：生产报工逐匹登记生产匹号+机台号；外发染色回仓生成染色匹号+缸号（净布工艺免缸号）；染色匹号贯穿入库/外发/销售/出库/对账，生产匹号仅限生产环节
  - [ ] 一期-迁移：inventory_piece 加 piece_type/machine_no、dye_lot_id 可空、唯一约束改部分唯一
  - [ ] 一期-报工：/flow-cards/feedbacks 支持逐匹登记 → 生成生产匹号
  - [ ] 一期-回仓：/outsourcing-receipts 生成染色匹号+缸号（强制，净布除外），parent_piece_id 建立血缘
  - [ ] 一期-单据：入库单/销售订单/销售出库条目加匹号字段并透传；E2E 07-fabric 重写为真实链路
  - [ ] 二期：外发（其他工艺）/对账单据的匹号字段与校验
- [ ] 采购退货链（11-returns）依赖物料存在的数据链在产品页 toFixed 崩溃修复后应自愈；若 items 创建仍 500（DATABASE_ERROR），需查退货明细外键与错误映射（500→400 语义）

所有已完成项已归档到 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，审计报告在 [docs/audits/](file:///workspace/.monkeycode/docs/audits/)。

后续新增任务请在此文件追加。
