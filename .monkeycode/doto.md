# 未完成任务

> 本文件**只记录未完成任务**（任务队列、待修复项、剩余清单），进度必须真实，禁止乐观偏差。
> 已完成任务见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，一句话总结见 [CHANGELOG.md](file:///workspace/.monkeycode/CHANGELOG.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。

---

## 一、P2 真实未完成项（2026-08-11 代码级核实，基于 HEAD=057af0d6）

| 编号 | 描述 | 代码证据 | 类别 | 状态 |
|------|------|----------|------|------|
| B02-P2-3 | handlers 未按业务域分子目录（平铺） | `backend/src/handlers/` 164 个 .rs 文件，仅 advanced/ + color_card/ 两个子目录 | 可维护性 | ❌ 未完成 |
| B04-P2-3 | 月末分摊缺端到端集成测试 | `backend/tests/test_energy_allocation.rs` 仅测试计算函数，无 HTTP/DB 集成测试 | 测试 | ⚠️ 部分完成 |
| batch-21 P2 25.4-I | 无长任务处理机制（状态持久化/断点续传） | `upgrade.rs:459-526` cmd_upgrade 顺序执行无状态保存；`deploy.sh:866-930` 线性流程 | 部署 | ❌ 未完成 |
| 前端 16 | vitest 覆盖率阈值仍为 1% | `frontend/vitest.config.ts:32-38` thresholds 全部 = 1 | 前端测试 | ❌ 未完成 |

**P2 真实剩余**：3 项未完成 + 1 项部分完成；P2 总数 248 项已完成约 244 项。

---

## 二、P3 低优先级（123 项，按需修复）

> 2026-08-07 explore 核实：**121 项未完成**（2 项已由 P2 顺带完成：25.1-F Nginx gzip、24.1-4 触屏按钮 ≥44px，均在 PR #833）。以下按审计 batch 统计未完成项。

| Batch | P3 未完成 | 主要内容 |
|-------|----------|----------|
| batch-01/08/11/16/19 | 0 | 回归验证/无 P3 缺陷 |
| batch-02/03/06/07 | 4 | MainLayout subMenus 硬编码 path（4-7 注释仍在）、extract_resource_info unknown 注释权衡、10 道精细工序模板未预置、行业词汇中英对照文档缺失、染整 service 测试函数名仍英文、颜色对比度 WCAG 无法自动验证 |
| batch-04/05 | 4-5 | 10 道精细工序模板未预置、行业词汇对照文档缺失、生产订单成本归集失败未接入 event_retry、AUDIT_RETENTION_DAYS 未走 AppSettings、工序映射文档不足 |
| batch-09 | 7 | 全部为流程执行类：E2E 报告保存 docs/audits、20/28/29 节奏监控、失败按优先级纳入、禁止死等、每 15 批记忆整理、实时归档、禁止跨批堆积 |
| batch-10/12 | 2-3 | RBAC 压力测试与性能监控、公共路径白名单测试、HTTP OPTIONS/HEAD 映射完整性 |
| batch-13 | ~15 | 供应商评估 model 重命名、供货历史查询、价格清单导入、色卡/染色/印花能力字段、加工商前端界面、委外加工报表、按匹号发货、客户多地址/多银行账户、客户特殊工艺、染色完成事件回写、离线报表 ETL、报表追溯、审计日志审查 cron、业务批次追溯 |
| batch-14 | 2-3 | 销售预测未与订单/库存补货联动、AI 操作审计未区分敏感操作、人工复核状态机不完整 |
| batch-15 | ~9-11 | 结账操作日志粒度、8 维度配置化、账龄档位配置化、行业基准配置化、调拨频率限制、预算考核报表、折旧起算日灵活、线索字段扩展性、公海客户来源记录、客户合并、转化耗时分析 |
| batch-17 | ~8-10 | 网络指标采集、企业微信/钉钉渠道、Alertmanager 目标启用、系统资源看板、慢查询阈值 100ms vs 500ms、慢查询周报、Retry-After 单位、迁移跳跃检测、system_version 模型接入、日志冷数据归档 |
| batch-18 | 2 | 拆匹未生成新匹号（仍 `{parent.piece_no}-CUT-{timestamp}`）、甘特图无拖拽调整后端接口 |
| batch-20 | ~26 | 触屏按钮尺寸规范、FCP/LCP/TTI 监控、system store、事件总线、组件文档、ColorCardGrid emits、composables try/catch、ARIA 标签、dataZoom 评估、重连降级轮询、票据鉴权、any 残留、chunk 阈值、env.d.ts、fixtures、no-v-html、CSRF cookie、CompanyTab localStorage、logger.error、按钮 loading+debounce、RTL、行级权限评估、超时重试幂等、CSRF 缺失提示、scoped 样式污染 |
| batch-21 | ~9 | 防火墙配置、回滚不恢复配置文件、UtilCommand --force、升级通知机制、多版本备份、配置热更新、RTO/RPO、缓存预热、升级演练要求、残留项10 历史迁移文件 |

**P3 真实剩余**：约 121 项未完成（按 batch 汇总约 90+，含流程执行类与测试增强类）。

---

## 三、技术债待办

> 2026-08-14 核实：原 T4（路由路径命名误导）已删除，不存在 `GET /api/v1/erp/export/csv/:export_type` 路由；`/csv` 路由用于导入（`import_csv`），导出路由为 `/export/pdf` 和 `/export/excel`。
