# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单），进度必须真实，禁止乐观偏差。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。

---

## 当前状态

**全部审计任务已完成（268/268，100%）。**

### 2026-09-05 新增

- [ ] 观察 PR #937 下一轮 CI 的 E2E"登录卡死 120s"分片（#11/#12/#13 曾复现）：已修复 401 刷新死锁（请求自等待）并加登录响应状态日志；若仍复现，依据日志区分"登录请求慢/失败"与"跳转未发生"继续定位
- [ ] 产品缺口：布卷/匹号管理 API 缺失——inventory_piece 仅由色卡审批小样流程内部创建，无创建/列表/详情端点（仅 POST /piece-split 与 GET /scan-inventory?barcode 按条码查）；07-fabric-four-dim 与 10-extended 的匹号用例已改写为真实契约测试（拆匹 404/参数校验 + 缸号生命周期查询），完整匹号管理能力需业务决策后补 API
- [ ] 采购退货链（11-returns）依赖物料存在的数据链在产品页 toFixed 崩溃修复后应自愈；若 items 创建仍 500（DATABASE_ERROR），需查退货明细外键与错误映射（500→400 语义）

所有已完成项已归档到 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，审计报告在 [docs/audits/](file:///workspace/.monkeycode/docs/audits/)。

后续新增任务请在此文件追加。
