# 任务一句话总结

> 每个任务一行摘要，是 doto-su.md 中详细任务内容的一句话总结。禁止写入详细内容。
> 详细任务内容见 [doto-su.md](file:///workspace/.monkeycode/doto-su.md)，未完成任务见 [doto.md](file:///workspace/.monkeycode/doto.md)，规则见 [MEMORY.md](file:///workspace/.monkeycode/MEMORY.md)。
> 最近整理：2026-07-22（V15 Batch 477-484 归档到 archives/2026-07-22/，v8-v14 历史归档到 archives/2026-07-22/，压缩超长行为标准一句话总结）

---

## V15 修复阶段（2026-07-16 启动）

| 批次 | PR | 一句话总结 |
|------|-----|-----------|
| 核实 2026-07-23 | — | 4 项未完成任务代码级核实：D10 第 3 批实际 4/4 完成（doto 滞后记 2/4，models/status.rs + mrp_engine_service.rs 已拆分）+ 第 4/5 批 6 文件逆生长（wage +114/ap_invoice +99/ap_recon +103/init +60/ar-vfy +48/flow_card +14）+ 第 6 批实际 15 个（doto 记 11，含 ar_ops/verification.rs 1062 行 D10-1 副产物）+ 当前真实 23 个 >1000 行；D05 数据准确仅 AssetListTab.vue 864→609；D13 实际 111 个（25 类前缀）/121 个（27 类，doto 记 123）；D14 风格 A 25 个（doto 记 21）+ listXxx 59 文件 104 处（doto 记 47/84）工作量低估 23% |
| 488（进行中，15/17 完成） | main 多 commit + PR #669-#686 | V15 P0-D 系列 17 项打包：已完成 15 项（D01/D02/D07/D11/D15/D16/D17 审计误判 + D03+D04 Redis 缓存接入 + D12 圈复杂度优化 6 项 + D06 aria-label ~225 文件 + D08 全部 167 函数超长函数拆分 + D09 收尾 2 个100+行函数 + D10-1 ar_service.rs 2489→259 facade + 5 子模块 + D10-2 production_order_service.rs 2141→689 facade + 5 子模块 + D10-3 so/delivery.rs 2095→822 facade + 6 子模块 + D10-2a voucher_service.rs 2058→882 + energy_service.rs 1826→324 + D10-2b outsourcing_service.rs 1879→436 + business_mode_service.rs 1739→741），剩余 2 项大型任务（D05/D10 第 3-6 批/D13/D14） |
| D10-1 | #683 main 34b8cae | D10 第 1 批 1/3：ar_service.rs (2489行) 拆为 facade (259行) + ar_ops/{types 75, json_helpers 98, collection 676, verification 1062, report 422, mod 23}，49 方法按职责分散到多 impl 块，外部 17 处调用路径不变 |
| D10-2 | #684 main 0385401 | D10 第 1 批 2/3：production_order_service.rs (2141行) 拆为 facade (689行) + production_order_ops/{mod 17, types 87, crud 568, completion 667, approval 288}，41 方法按职责分散到多 impl 块，3 处外部调用路径不变 |
| D10-3 | #684 main 0385401 | D10 第 1 批 3/3：so/delivery.rs (2095行) 拆为 facade (822行) + delivery_ops/{mod 16, types 35, ship 588, inventory 357, cancel 270, export 136}，30 方法按职责分散到多 impl 块，DTO+validate+测试保留在 facade，外部调用路径不变 |
| D10-2a | #685 main f836552 | D10 第 2 批 1/2：voucher_service.rs (2058行) 拆为 facade (882行，含 747 行测试模块) + voucher_ops/{mod, crud 468, workflow, balance, assist}，39 方法按职责分散到多 impl 块（5 workflow + 12 balance + 11 assist + 11 crud），DTOs + VoucherTypeDefinition + VoucherDetail + 测试保留在 facade，db 字段改 pub(crate)，update_account_balances/write_assist_accounting_records_txn 改 pub(crate)，BalanceUpdateContext/AssistRecordContext 改 pub(super)，外部 34 处调用路径不变；energy_service.rs (1826行) 拆为 facade (324行) + energy_ops/{meter,consumption,allocation_rule,allocation_record} |
| D10-2b | #686 main 882cecc | D10 第 2 批 2/2：outsourcing_service.rs (1879行) 拆为 facade (436行) + outsourcing_ops/{mod,types,order 724,order_item,receipt,voucher}，4 Service 39 方法（17+5+9+8）按职责分散到多 impl 块，9 纯函数 + 20 测试保留在 facade，db 字段改 pub(crate)，ReceiptCalculation 改 pub(super)，DTOs 通过 pub use 二次 re-export；business_mode_service.rs (1739行) 拆为 facade (741行) + business_mode_ops/{mod,types,config,flow_step,rule,order_link}，4 Service 28 方法（11+5+5+7）按职责分散到多 impl 块，9 校验纯函数 + P0-D12 重构产物 + 测试保留在 facade，DTOs 通过 pub use 二次 re-export |
| D10-3a | #687 main d301de9 | D10 第 3 批 1/2：chemical_service.rs (1730行) 拆为 facade (349行) + chemical_ops/{mod 25, types 245, master 439, category 196, lot 327, requisition 316}，4 Service 43 方法（master 16 含10私有helper + category 6 + lot 10 + requisition 11 含私有 generate_requisition_no）按职责分散到多 impl 块，8 纯函数 + 18 测试保留在 facade，db 字段改 pub(crate)，DTOs 通过 pub use 二次 re-export，外部 chemical_handler.rs 调用路径不变；bi_analysis_service.rs (1711行) 拆为 facade (317行) + bi_analysis_ops/{mod 23, types 226, sales 391, profit 258, drilldown 352, olap 287}，1 Service 20+ 方法按职责分散到 4 impl 块（sales 6 + profit 2 + drilldown 4 + olap 4），3 helper 函数 + scope_sql 改 pub(crate)，8 response struct 通过 pub use 二次 re-export，外部 bi_handler.rs 调用路径不变 |
| D10-3b | #688 main 69de94f + #691 main 9818351 | D10 第 3 批 2/2：models/status.rs (1577行) 拆为 status/mod.rs + {common,master_data,production,purchase,sales,inventory,mrp,payment} 8 分组文件，按业务域分组状态常量；mrp_engine_service.rs (1593行) 拆为 facade (605行) + mrp_engine_ops/{mod,types,stock,bom,calculation,query,order} 7 子模块 22 方法，StockInfo 提升为 pub(crate)，facade 仅 pub use 8 个原 pub struct，测试模块直接从 ops 导入 StockInfo；CI 修复 3 轮：5 unused imports + 6 sea_orm trait 缺失 + 集成测试 common 模块名称遮蔽（use super::common 限定本地模块） |
| D10-4a | #692 main ac593a2 | D10 第 4 批 1/2：dye_batch_state_machine_service.rs (1512行) 拆为 facade (920行，含 11 纯验证函数 + 4 Service struct + new + 10 DTOs + 测试) + dye_batch_state_machine_ops/{mod 17, lifecycle_log 152, state_rule 195, rework 232, operation 117}，4 Service 27 方法按职责分散到多 impl 块（lifecycle_log 6 + state_rule 7 + rework 9 + operation 5），db 字段改 pub(crate)，外部调用路径不变；wage_service.rs (1621行) 拆为 facade (774行，含 9 纯函数 + 3 Service struct + new + 7 DTOs + 测试) + wage_ops/{mod 14, rate 351, record 242, calculation 357}，3 Service 29 方法按职责分散到多 impl 块（rate 11 + record 10 + calculation 8 含 3 私有 helper struct），db 字段改 pub(crate)，2 日期纯函数改 pub(crate) 供 calculation 复用，外部 wage_handler.rs 调用路径不变 |
| D10-4b | #693 main 6a480d9 | D10 第 4 批 2/2：ar/vfy.rs (1368行) 拆为 facade (568行，含 7 DTOs + 测试) + ar/vfy_ops/{mod 17, match 389, aging 158, reconciliation 221, confirm 113}，ArReconciliationService 5 公开方法 + helper 分散到多 impl 块，db 字段改 pub(crate)，外部调用路径不变；ap_invoice_service.rs (1405行) 拆为 facade (407行，含 6 DTOs + 3 校验函数 + impl_generate_no! 宏 + 测试) + ap_invoice_ops/{mod 16, types 159, receipt 390, crud 398, report 161}，ApInvoiceService 20 方法分散到多 impl 块（receipt 9 + crud 8 + report 3），ReceiptVoucherContext 移到 receipt.rs，db 字段改 pub(crate)，CI 修复 1 轮：receipt.rs 缺失 ColumnTrait |
| D10-4b docs | #694 main c6df976 | D10-4b 合并完成更新 CHANGELOG + doto 文档 |
| Clippy fix | #695 main c078e96 | 修复 9 个新增 Clippy 警告（D10 拆分副产物）：4 个中文测试函数名 snake_case + 5 个 unused import/DTO re-export |
| D10-5 | #696 main 6bc4dca | D10 第 5 批 4/4：init_service.rs (1347行) 拆为 facade (293行) + init_service_ops/{mod 11, setup 287, role 215, permission 387, dept_user 198}，10 核心方法分散到 4 impl 块，db 字段改 pub(crate)，5 跨子模块调用方法改 pub(crate)；flow_card_service.rs (1285行) 拆为 facade (386行) + flow_card_ops/{mod 16, route 151, card_crud 227, card_state 190, step 247, feedback 162}，4 Service 35 方法分散到 5 impl 块，5 纯函数改 pub(crate)；ap_reconciliation_service.rs (1346行) 拆为 facade (621行) + ap_reconciliation_ops/{mod 17, types 99, crud 189, confirm 182, report 111, auto 235}，18 方法分散到 5 子模块；search/elastic.rs (1230行) 拆为 facade (756行含测试 394行) + elastic_ops/{mod 4, client_ops 343, syncer_ops 41, types_ops 49}；CI 修复 1 轮：5 方法可见性私有→pub(crate) + 1 unused import SearchClient |
| D10-6a | #698 main 9d26d7d | D10 第 6 批 4/15：event_bus.rs (1196行) 拆为 facade (240行) + event_bus_ops/{mod 20, publish 298, subscribe 410, retry 234}，3 Service 23 方法分散到多 impl 块；po/order.rs (1184行) 拆为 facade (310行) + po/order_ops/{mod 17, crud 470, state 222, receive 188}，PurchaseOrderService 18 方法分散；auth_service.rs (1180行) 拆为 facade (459行含测试) + auth_service_ops/{mod 20, auth 487, jti 357}，AuthService 12 方法 + 7 JTI free functions 分散；inventory_finance_bridge_service.rs (1180行) 拆为 facade (410行含测试) + inventory_finance_bridge_ops/{mod 17, sync 350, recon 230, voucher 178}，3 Service 15 方法分散；db 字段改 pub(crate)，CI --admin 合并（覆盖率 infra 失败） |
| D10-6b-1 | #700 main 325dfed | D10 第 6 批 5-8/15：lab_dip_service.rs (1188行) 拆为 facade (230行) + lab_dip_ops/{mod 23, types 171, request 334, sample 305, resample 284}，LabDipService 16 方法分散；production_recipe_service.rs (1181行) 拆为 facade (631行含测试) + production_recipe_ops/{mod 24, recipe_crud 369, recipe_state 72, addition 185}，2 Service 14 方法分散；product_service.rs (1075行) 拆为 facade (170行) + product_ops/{mod 17, sync 58, crud 338, color 143, import_export 451}，ProductService 18 方法分散；system_update_service.rs (1074行) 拆为 facade (459行含测试) + system_update_ops/{mod 20, status 179, apply 221, backup 117, github 229}，SystemUpdateService 14 方法分散；db 字段改 pub(crate)，CI 修复 3 轮：2 unused sea_orm::prelude import + 6 doc list overindented + PaginatorTrait 缺失 + 8 新增警告入 baseline；--admin 合并（覆盖率 infra 失败） |
| 487 | main 3919255 + d7e3b73 + a456a53 | V15 P0-T02 7 项集成测试 73 测试 + P0-T07 性能基准 11 bench + P0-T05 E2E 配置修复（28 文件 +1836 -29） |
| 486 | main 01faa60 | V15 P0-T01 核心 service 单测补全（quotation + purchase_receipt 共 38 测试） |
| 485 | main af0f16b + 5e4e78f + 7cc82cc | V15 P0-T03 clippy baseline 恢复 + P0-T08 覆盖率工具 + 编译错误修复（4 文件，CI 7 轮） |
| 484 | main df5286ee + c012a3b9 | V15 P0-B15 缺料预警持久化 + P0-B16 自动故障检测 + P0-B17 主备切换（11 文件） |
| 483 | #668 | V15 P0-B10 BI 权限过滤 + P0-B11 定制订单打样报价 + P0-B12 售后质量集成 + P0-B13 物流电子签收（15 文件） |
| 482 | #667 | V15 P0-B05 大额调拨 + P0-B06 预算超支 + P0-B07 CRM 回收 + P0-B08 赢率 + P0-B09 输单原因 + P0-B14 Incoterms（13 文件） |
| 481 | #666 | V15 P0-B01 坏账准备 + P0-B02 坏账核销审批 + P0-B03 催收任务 + P0-B04 财务预警（25 文件，CI 5 轮） |
| 480 | main 5334bf13 + 8d7ea998 + ae87219f | V15 P0-F20 8D 质量管理流程（13 文件，11 态状态机） |
| 479 | main 642d2c09 + cc1ee381 + c06109fd + bbf38a30 | V15 P0-F18/F21 返工降级报废闭环 + 返工走生产订单（7 文件） |
| 478 | main 9d01a42 + 6aca804 | V15 P0-F15/F16/F17/F19 大货批色审批贯通（11 文件，8 态状态机） |
| 477 | main a3798f4 + daeab0f | V15 P0-F10/F11/F12/F13 色卡发放库存联动 + 前端文件结构 + legacy 数据迁移（15 文件） |
| 476 | main eb57484 | V15 P0-S17 打印 HTML 真实数据查询（2 文件，6 个方法从占位改真实查询） |
| 475e | #662 | V15 P0-S12 前端导出接入后端 B 类批次 3/3 收尾（12 文件，5 模块闭环） |
| 475d | #661 | V15 P0-S12 前端导出接入后端 B 类批次 2/3（14 文件，4 模块） |
| 475c | #660 | V15 P0-S12 前端导出接入后端 B 类批次 1/3（11 文件，3 模块） |
| 475b | #659 | V15 P0-S12 前端导出 purchase/customer 闭环（4 文件） |
| 规划 | - | 2026-07-17 批次节奏调整：每批 9-12 文件，P0 批次总数从 27 压缩为 22 |
| 475a | #658 | V15 P0-S13 审计日志导出闭环（3 文件） |
| 474 | #657 | V15 P0-S15 导出水印基础设施 + P0-S12 前端导出接入后端核心 2 页面（10 文件） |
| 473 | #656 | V15 P0-S14 + P0-S19 审计字段补齐（8 文件，2 新增 migration） |
| 461 | #643 | V15 P0-S14 敏感数据导出二级审批机制（migration 047 + 9 方法 + 7 端点） |
| 462 | - | V15 P0-S24 前后端权限边界一致性修复（7 文件） |
| 464 | - | V15 P0-S25 行级数据权限 RLS 策略启用（5 张敏感表） |
| 468 | - | V15 P0-S28 前端 v-permission 覆盖率提升（7 文件） |
| 469 | #644 | V15 P0-F01 dye_batch 表新增 dye_lot_no 字段补全四维标识（4 文件） |
| 470 | #645 | V15 P0-F02 面料行业关键业务约束 UNIQUE 补全（3 张表，1 文件） |
| 471 | #646 | V15 P0-F03~F08/F09 色卡发放模式重构-后端核心（21 文件） |
| 472 | #648 | V15 P0-F07 色卡发放前端 borrow.vue→issue.vue 完整重写（10 文件） |
| 复审 | #649 | V15 修复阶段已修复任务实际状态复查报告（30 P0 任务，3 子代理并行） |
| 433 | #611 | V15 P0-S03 修复超级权限注入漏洞 |
| 434 | #612 | V15 P0-S04 补齐 31 类业务角色覆盖面料行业全业务场景 |
| 435 | #613 | V15 P0-S20/S21/S22 权限资源缺口补齐（60+ 类权限资源 + 33 角色矩阵） |
| 436 | #614 | V15 P0-S01 行级数据权限基础设施（migration m0051 + data_scope.rs + 15 单测） |
| 437 | #616 | V15 P0-S18 新增 dye_recipe_master 角色 |
| 438 | #617 | V15 P0-S07 权限缓存不失效修复 |
| 439 | #618 | V15 P0-S05 SoD 职责分离互斥（8 条预置规则） |
| 440a | #619 | V15 P0-S06 权限变更审计基础设施（migration m0053） |
| 440b | #620 | V15 P0-S06 role_permission_service 接入审计日志 |
| 440c | #621 | V15 P0-S06 user_service 接入用户角色变更审计 |
| 441 | #622 | V15 P0-S10 method_to_action 升级识别 print/export/download |
| 442 | #623 | V15 P0-S09 染色域 export 端点补齐 AuthContext |
| 443 | #624 | V15 P0-S09 print_handler AuthContext 补齐（7 函数） |
| 444 | 无需 PR | V15 P0-S09 其他域 export AuthContext 核查（均已含） |
| 445 | #625 | V15 P0-S11 核心业务导出审计日志补齐第 1 批（5 文件 6 函数） |
| 446 | #626 | V15 P0-S11 报表染色域导出审计日志补齐第 2 批（5 文件 5 函数） |
| 447 | #627 | V15 P0-S01 行级数据权限注入-销售域 |
| 448 | #628 | V15 P0-S01 行级数据权限注入-采购域 |
| 449 | #629 | V15 P0-S01 行级数据权限注入-生产域 |
| 450 | #630 | V15 P0-S01 行级数据权限注入-CRM 域 |
| 451 | #631 | V15 P0-S01 行级数据权限注入-财务域（finance_payment+invoice） |
| 451b | #632 | V15 P0-S01 行级数据权限注入-财务域 AP 域 |
| 451c | #633 | V15 P0-S01 行级数据权限注入-财务域 AR 域 |
| 452 | #634 | V15 P0-S01 行级数据权限注入-库存域（调整+预留子域） |
| 452b | #635 | V15 P0-S01 行级数据权限注入-库存域（盘点子域） |
| 452c | #636 | V15 P0-S01 行级数据权限注入-库存域（调拨子域） |
| 453 | #637 | V15 P0-S02 IDOR 防护-handler 层（销售域） |
| 454 | #638 | V15 P0-S02 IDOR 防护-handler 层（采购域） |
| 455-457 | #639 | V15 P0-S02 IDOR 防护-handler 层（生产+CRM+财务域，7 文件 11 函数） |
| 458 | #640 | V15 P0-S02 IDOR 防护-handler 层（库存域+应收发票域，7 文件 11 函数） |

---

## V15 审计执行阶段（2026-07-16）

| 批次 | 日期 | 一句话总结 |
|------|------|-----------|
| 01-21 | 2026-07-16 | V15 全项目综合审计 21 批 195 维度全部完成，发现 732 个问题（104 P0 + 257 P1 + 248 P2 + 123 P3），汇总报告已生成 |

---

## V15 审计计划九轮升级（2026-07-15）

| 日期 | 一句话总结 |
|------|-----------|
| 2026-07-15 | V15 审计计划第三轮升级：类八法律合规扩展到 8 维度 + 新增类十一大货批色专项 6 维度，V15 升级为 11 大类 68 维度 |
| 2026-07-15 | V15 审计计划第四轮深化：类十色卡发放专项从 5 维度深化到 7 维度，V15 升级为 11 大类 70 维度 |
| 2026-07-15 | V15 审计计划第五轮升级：新增类十二 RBAC 权限控制机制专项 8 维度，V15 升级为 12 大类 78 维度 |
| 2026-07-15 | V15 审计计划第六轮升级：新增类十三打印导出审计与权限控制专项 10 维度，V15 升级为 13 大类 88 维度 |
| 2026-07-15 | V15 审计计划第七轮升级：新增类十四权限维度审计与角色合理性专项 12 维度，V15 升级为 14 大类 100 维度 |
| 2026-07-15 | V15 审计计划第八轮升级：新增类十五业务主体维度审计与数据流转专项 15 维度（含加工商完全未实现重大缺口），V15 升级为 15 大类 115 维度 |
| 2026-07-15 | V15 审计计划第九轮升级：基于后端+前端完整模块扫描，新增 9 个新类别共 75 维度（类十六 AI 模块 10 + 类十七财务深化 8 + 类十八 CRM 全链路 5 + 类十九报表 BI 通知 8 + 类二十可观测性运维 8 + 类二十一胚布拆匹质量 5 + 类二十二库存排程物料 6 + 类二十三组织定制物流 5 + 类二十四前端架构体验 20），V15 升级为 24 大类 190 维度最终版 |

---

## v8-v14 历史归档

> v8-v14 复审修复阶段的批次记录已归档到 [archives/2026-07-22/changelog-v8-v14.md](file:///workspace/.monkeycode/docs/archives/2026-07-22/changelog-v8-v14.md)。
> 批次 1-236 的更早期记录已归档到 [docs/archives/](file:///workspace/.monkeycode/docs/archives/)。
