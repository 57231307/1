# 简化版功能盘点报告（2026-09-13）

## 总体
- 后端端点 1843 / 前端封装 622（覆盖率 34%）
- 后端有而前端无入口：1017（约 55%）
- 前端孤儿调用（点击即 404）：105
- 完全实现模块 38 / 简化版模块 38 / 活跃占位 0

## P0 核心流程不闭环（13 项）
| 模块 | 缺口 | 后端能力 |
|------|------|---------|
| 销售退货 | 11 调用全 404（缺 /sales 前缀） | sales_return_handler 9 端点 |
| 交易管理 | 22 调用 404（后端仅 GET 列表） | 审批/执行端点未实现 |
| 流转卡 | 27 端点零前端 | flow_card_handler |
| 打样 | 22 端点零前端 | lab_dip_handler |
| 大货批色 | 19 端点零前端 | bulk_color_approval_handler |
| 销售分析 | 整页 404（缺 /crm 前缀） | 11 端点 |
| 产能 | 整页 404（缺 /production 前缀） | 11 端点 |
| 打印模板 | 写操作后端缺失 | 仅 2 GET |
| 销售订单 | 发货/完成/打印无入口 | ship/complete/print |
| 采购订单 | close/cancel/导出无入口 | 4 端点 |
| 生产配方 | 16 端点零前端 | production_recipe_handler |
| 质量 8D | 7 端点零前端 | quality_8d_handler |
| 坏账/催收 | 26 端点零前端 | 23 端点 |

## 修复优先级
1. 修 105 孤儿调用（路径前缀错位：sales-returns/sales-analysis/capacity/通知双前缀等，1-2 天）
2. 后端补 6 组写端点（print-templates/data-import 任务/system-update 备份/trading/vouchers unpost/quality archive）
3. P0 生产域补页面（流转卡/打样/批色/配方，复用 custom-orders 三页模式）
4. CRM 高级功能 Tab（漏斗/CLV/分配规则）
5. 合规/外贸/AI 治理域（只读列表先行）

完整 38 项 P1 + 25 项 P2 清单见盘点原始数据（explores 输出）。
