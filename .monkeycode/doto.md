# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单），进度必须真实，禁止乐观偏差。
> 已完成任务见 [doto-su.md](doto-su.md)，一句话总结见 [CHANGELOG.md](CHANGELOG.md)，规则见 [MEMORY.md](MEMORY.md)。

---

## 当前状态

**追溯字段不可空专项 + E2E 稳定性收敛修复已全部推送（9b863fd），CI run 验证中。**

---

## 未完成任务清单

### 匹号领域二期（一期已全部完成并归档）

- [ ] 二期：外发（其他工艺）/对账单据的匹号字段与校验

### 追溯字段增强

- [ ] 染色匹 color_no 现存空串（piece_domain_service create_piece_from_outsourcing_receipt 调用方无色号参数）——增强：从委外订单/库存行取 color_no 传入，完善染色匹色号追溯

### CI 观察（当前 run 34045847324，commit 9b863fd）

- [ ] 观察 CI：追溯列 NOT NULL DEFAULT '' 迁移 + UPDATE 兜底在 CI 库上生效；盘点/调拨/退货明细插入 DATABASE_ERROR 是否清零
- [ ] 观察 CI：**Playwright/Vite 页面级挂起**（run 34041167918 的 10 个 exit 124 分片，后端全程健康，挂起层在前端——safeGoto 页面加载或页内 JS；与后端假死不同层）若复现，需排查 Vite dev server 冷启动转换超时/页面 JS 死循环，考虑给 safeGoto 加总超时 + ensureTestEntities 加整体超时护栏
- [ ] 若 CI 仍有失败分片：拉日志→定位→统一修复→推送，直到全绿

### 流程备忘

- [ ] CI 失败若再出现新 job：按 doto 流程拉日志→记录→下批修复

后续新增任务请在此文件追加。
