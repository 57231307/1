# 任务一句话总结

> 每个任务一行摘要，是 doto-su.md 中详细任务内容的一句话总结。禁止写入详细内容。
> 详细任务内容见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，未完成任务见 [doto.md](file:///workspace/.monkeycode/doto.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。

---

## 2026-08-14

| PR | 一句话总结 |
|----|-----------|
| PR #907 | 测试代码编译修复：批量修复 50+ 处符号路径错误、6 处 Model 字段名、3 处顶层模块导入；CI 添加 --keep-going；规则 18 并入规则 6、规则 19 并入规则 21 |

## 2026-08-09

| PR | 一句话总结 |
|----|-----------|
| PR #878 | P3 批量任务第三轮：composables try/catch + dashboard store 错误提示 + 结账日志 + system_version 接入 + Alertmanager 启用 + RTL 支持 + 企业微信/钉钉渠道 + 供应商评估 model 重命名；8 文件 +80 行 |

## 2026-08-08

| PR | 一句话总结 |
|----|-----------|
| PR #877 | P3 批量任务第二轮：Retry-After HTTP 头 + 成本归集 event_retry + CSRF 提示 + env.d.ts 类型声明 + 迁移跳跃检测 + no-v-html 规则 + v-html 安全注释；7 文件 +80 行 |
| PR #876 | P3 批量任务：拆匹号改进 + 甘特图增强 + AUDIT_RETENTION_DAYS 纳入 AppSettings + HTTP OPTIONS/HEAD + AI 操作审计 + 慢查询阈值 + ARIA 标签 + 按钮 loading；8 文件 +120 行 |
| PR #875 | batch-18 P3 任务：拆匹号改进 + 甘特图拖拽增强 + AUDIT_RETENTION_DAYS 纳入 AppSettings + HTTP OPTIONS/HEAD 映射；2 文件 +60 行 |
| PR #871 | 批量实现 4 个 P2 任务：B12-P2-1 权限码命名规范 + B12-P2-5 流式导出 + batch-13 P2 供应商余额+异常订单 + batch-16 P2-3 通知模板模型；10 文件 +275 行 |
| PR #870 | 前端 18 dynamic_router 实现 + batch-12 P2-9 权限测试 + B04-P2-3 月末分摊测试：EndpointCache + 动态路由中间件 + test_data_permission.rs 6 个测试 + test_energy_allocation.rs 12 个测试；4 文件 +221 -3 |
| PR #869 | batch-18 P2-6 调拨在途库存独立核算 + batch-12 P2-9 权限测试 + B04-P2-3 月末分摊测试：inv/batch.rs 更新 quantity_incoming + test_data_permission.rs 6 个测试 + test_energy_allocation.rs 12 个测试；5 文件 +282 行 |
| PR #868 | batch-18 P2-2 委外加工费按缸号/匹号核算：outsourcing_order_item 添加 processing_fee/freight_fee 字段 + migration + DTO 更新；7 文件 +48 -1 |
| PR #867 | batch-18 P2-4 瓶颈识别扩产/外包建议：BottleneckSuggestion + generate_suggestions + overview 自动生成建议；3 文件 +115 -3 |
| PR #866 | batch-18 P2-5 排程重复录入校验：apply_schedule_details_to_orders 添加状态校验（仅 DRAFT/SCHEDULED）和日期保护（None 不覆盖）；3 文件 +31 -11 |

## 2026-08-07

| PR | 一句话总结 |
|----|-----------|
| PR #865 | batch-18 P2-7 缺料月报能力：material_shortage_handler.rs get_monthly_report + service get_monthly_report + 路由注册；3 文件 +156 行 |
| PR #863 | P2 快速修复 + 导出技术债：B12-P2-2 字段级权限推广 + B12-P2-3 权限审计日志接口 + batch-12 P2-8 审计日志保留调度 + batch-11 P2-6 打印水印 + T1/T2/T3 CSV 中转去除；18 文件 +572 -258 |
| PR #862 | A1-A4 完成：57 个新 docx 打印端点（纺织专用 9 + P0 16 + P1 25 + P2 6），63 个 get_*_print_data + 63 个 handler，覆盖纺织专用/P0/P1/P2 全部未实现打印场景；CI 全绿 |
| A0b | `ExportService::export_pdf` 由纯文本改写 printpdf 真实 PDF，修复 `report_enhanced` `POST /export/pdf` 与 `export_template` pdf 分支以 PDF 名义交付文本的规则 3 硬违规 |
| PR #859 | A0 完成：6 个原 HTML 打印场景改为 docx 成品（接入 generate_docx）+ 会计凭证 `/vouchers/:id/print` 路由 + 删除 generate_pdf/escape_html 死代码 + 模板数据驱动改造；CI 全绿 |

## 2026-08-05

| PR | 一句话总结 |
|----|-----------|
| PR #854 | batch-21 部署升级：端口冲突 + .env 600 + 断点续传 + 版本降级 + API 兼容 + 配置迁移 + 日志持久化 + draining + 升级监控告警 + 多租户残留；11 文件 |
| PR #853 | P2-Batch-32：胚布追溯字段 + 拆匹强校验 + 告警去重 + 在途采购 + 排程冲突告警 + 负荷告警 + SPT 调度；7 文件 |
| PR #852 | P2-Batch-31 续作：慢查询告警/优化追踪 + 通知订阅调度 + 权限合规 + 供应商评估 + recipe_opt + PII脱敏 + 存货跌价 + 部门服务 |
| PR #848 | P2-Phase-9 CRM 数据权限+数据流转：客户字段权限配置 + 客户操作审计日志 + 转化数据双向同步 + 客户主数据关系 + 客户 CLV；5 文件 |
| PR #847 | P2-Phase-8 CRM 商机+公海管理增强：阶段停留时长 + 商机竞争对手 + 商机跟进记录 + 回收规则跟进/成交周期 + 回收规则部门差异化 + 公海客户保护机制；6 文件 |
| PR #846 | P2-Phase-7 CRM 线索管理增强：线索来源 ROI 跟踪 + 线索分配规则 + 线索培育流程；3 文件 |
| PR #844 | P2-Phase-6C 调拨审批流 + 资金日报/月报：按金额分级审批 + 资金日报/月报接口；2 文件 |
| PR #842 | P2-Phase-6B 预算版本管理 + 资产减值测试 + 折旧政策变更：预算版本管理 + 资产减值测试 + 折旧政策变更 + m0099 migration；3 文件 |
| PR #840 | P2-Phase-6 现金流比率 + 趋势分析增强：现金流比率（OPERATING_CF_RATIO/SALES_CF_RATIO/CF_ADEQUACY_RATIO）+ 趋势分析增强（线性回归+移动平均+趋势方向）；2 文件 |
| PR #839 | P2-Phase-5 预算科目-会计科目映射 + 资产分类管理：budget_items.account_subject_id + asset_categories 表 + CRUD + fixed_assets.asset_category_id + m0098 migration；2 文件 |
| PR #838 | P2-Phase-4 辅助核算余额增强+账龄业务员维度+穿透查询：期初/期末余额计算 + 账龄按 salesperson_id GROUP BY + 穿透查询总账到辅助明细；3 文件 |
| PR #836 | P2-Phase-3.5 接入未实现的修复项：m0094 processor_type 筛选接入 + m0095 sales_contract_items service/handler/route + m0096 period_report_snapshot service/handler + m0097 aging_alert_rules service/handler + mask_fields 接入 customer_handler + record_actual_grade handler 端点；6 文件 |
| PR #835 | P2-Phase-3 DB migration：m0093 suppliers category_id FK + m0094 is_processor+processor_type + m0095 sales_contract_items + m0096 period_report_snapshot + m0097 aging_alert_rules；5 文件 |
| PR #834 | P2-Batch-31 全域 P2 审计修复：慢查询告警/优化追踪 + 通知订阅调度 + 权限合规 + 供应商评估 + recipe_opt + PII脱敏 + 存货跌价 + 部门服务；20 文件 |

## 2026-08-04

| PR | 一句话总结 |
|----|-----------|
| PR #833 | P2-Batch-30 Nginx gzip + 移动端触屏按钮：gzip 压缩 + Touch targets 44px CSS；2 文件 |
| PR #832 | CI Release 清理排序修复：sort -V 混合段数版本号排序错误，改用 --order asc 按创建时间排序 |
| PR #831 | P2-Batch-29 WebSocket 心跳超时断开：30s Ping + 60s 超时断开；1 文件 |
| PR #830 | P2-Batch-28 角色命名校验 + is_system 约束 + 报表参数 Validate：角色编码规范 + admin 约束 + Validate 派生；3 文件 |
| PR #829 | P2-Batch-27 报表元数据 refresh/cache + AI 速率限制：refresh_strategy/cache_ttl_seconds 字段 + AI 端点专用速率限制 (10 req/min/user)；2 文件 |
| PR #827 | P2-Batch-25/26 前端优化 + 后端超时/事务/账龄基准日：visualizer + persistedstate + lazy loading + alt prop + baseline_date + batch atomicity + OTel 10% + manager_id + supplier qual CRUD + BI/dashboard timeout；14 文件 |
| PR #826 | P2-Batch-24 CI Release 清理修复：修复 --cleanup-tag 不生效 + 清理无 Release 旧 tag（保留 100 个）；1 文件 |
| PR #824 | P2-Batch-23 部署变更文件记录：部署时记录变更文件列表到 deploy-changes.log；1 文件 |
| PR #823 | P2-Batch-22 AI explanation + 前端性能/可访问性/权限缓存：explanation 字段 + 错误去重 + 焦点重置 + 懒加载 + 权限缓存 + 路由预取；6 文件 |
| PR #822 | P2-Batch-19 售后退货类型 + incoterms 责任划分：issue_type 增加 return_goods（前后端）+ incoterms cost_bearer/清关责任接入报价构成；2 文件 |
| PR #821 | P2-Batch-21 部署脚本加固：日志持久化 + 配置权限600 + 健康检查database + CLI权限/确认/校验/回退 + 回滚验证；10 文件 |
| PR #820 | P2-Batch-08 角色校验 + 通配匹配 + 测试：is_system/admin 校验 + matches_permission 通配 + require_admin_role 测试 + 文档单复数；4 文件 |
| PR #819 | P2-Batch-07 AI 输入校验 + 降级 + 推理耗时：create_process_optimization 长度/枚举校验 + anomaly_detection 降级 + 错误文案 + inference_latency_ms；4 文件 |
| PR #817 | P2-Batch-06 权限 fail-closed + PII 脱敏 + CRUD 审计：extract_resource_info unknown fail-closed + 手机号/身份证脱敏 + CRUD 审计；3 文件 |
| PR #815 | P2-Batch-05 导出审计 + 打印水印：3 个导出端点补 Export 审计 + 打印IP水印 + rate_limit确认全局挂载；3 文件 |
| PR #814 | P2-Batch-04 硬编码 role_id==1 修复 + v-role 指令删除；2 文件 |

## 2026-08-03

| PR | 一句话总结 |
|----|-----------|
| PR #812 | CI Cargo.toml SemVer 兼容：TAG/Release 保持 4 段式 YYYY.M.D.HHMM，Cargo.toml 转为 3 段式 YYYY.MDHHMM |
| PR #811 | CI 版本号格式修复：日期分隔 YYYY.MMDD.HHMM → YYYY.M.D.HHMM |
| PR #810 | CI Release 流程修复：用 gh CLI 替代 softprops/action-gh-release，添加三重验证 |
| PR #809 | CI 发布说明调试：添加发布说明生成调试输出和错误处理 |
| PR #808 | CI 改进：clippy 日志化 + fmt 自动修正 + 消除重复检查 |
| PR #807 | CI clippy 新增警告修复：修复 18 条 clippy 警告（11 条代码修复 + 7 条 dead_code 恢复 baseline） |

## 2026-08-02

| PR | 一句话总结 |
|----|-----------|
| PR #803 | P2-Batch-03 类八法律合规剩余 + 类九色卡发放：跨境合规 + 商检/产地证 + 色卡报表/成本/预警/统计 12 端点接入路由；75 文件 +2322 -40 |
| PR #801 | P2-Batch-02 类五运行闭环：反馈闭环 + 重染补染 + 告警死信 + 色卡状态 + CancellationToken + 染缸占用 + 设备连接 + 人工成本归集 + 能耗凭证归集 + 期末调整；46 文件 +3001 -51 |
| PR #799 | P2-Batch-01b 续作：Cookie 双写 + 缓存一致性 + SQL 参数化 + 表重叠 + 测试补齐 + service 拆分 + 差异化 TTL；34 文件 +1799 -848 |
| PR #797 | P2-Batch-01a 首批快速修复：CSP+Argon2+魔法数字+TODO+i18n 注释；9 文件 |

## 2026-07-31

| PR | 一句话总结 |
|----|-----------|
| PR #795 | P0 缺陷 10-4 审计日志导出二次审计机制：新建 audit_log_export_log 防篡改表 + BEFORE UPDATE/DELETE 触发器禁止篡改 + 导出文件 SHA256 指纹留存 + /audit-logs/export-logs 查询端点；CI 全绿合并 main 7b18573 |
| PR #793 | P1 后续 #2 业务追溯 producer 接入：record_purchase_receipt 接入采购收货创建后、record_sales_delivery 接入销售发货后；best-effort 集成不阻塞主流程；CI 全绿合并 main 8fa619e5 |
| PR #791 | 缺陷 9-2 染色批次导出全量查询：导出查询加 .limit(10000) + QuerySelect trait 导入；CI 全绿合并 main b2e7b419 |
| PR #785 | P1 预留服务路由接入消除 174 个 dead_code 警告：为 14 个 P1 预留服务创建 handler 和 route 文件并注册路由；37 文件 +2093 -11 行 |
| PR #783 | Clippy runner shutdown (exit 143) 修复 + Release 变更说明模板 |
| PR #777 | 彻底移除 Docker/K8s 引用，对齐 systemd 直部署；11 文件 -130 行 |
| PR #776 | CHANGELOG/doto 文档同步 PR #775 合并记录 |
| PR #775 | P1-batch11 缺陷 2-3 遗留修复：补齐 4 个前端页面导出/打印按钮 v-permission 指令 |
| PR #771 | P1-batch02+03 通用代码质量+安全性：9 项 P1 全部完成 |

## 2026-07-30

| PR | 一句话总结 |
|----|-----------|
| PR #790 | P1 主线八维后续修复：盘点契约 P0-1 前端契约对齐 + API 网关 PATCH rate_limit 范围校验；CI 全绿合并 main 85aec7de |
| PR #788 | P1 委外收货主链路统一：confirm 收敛为唯一事务主链路 + OutsourcingOrderCompleted 事件 + workflow tests；CI 全绿合并 main |
| PR #786 | V15 主线八维审计 + 快速修复 P0/P2 批次：P0 全部 11 项 + P2 全部 3 项；21 文件 +989/-229；CI 全绿合并 main 8cd956d |
