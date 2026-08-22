# 迁移目录按域重组方案

> A.20.1：当前迁移目录按版本批次命名（v15_batch18…），随时间推移持续膨胀。
> 本文档设计按业务域重组方案，为后续 A.20.2/A.20.3 实施铺路。

## 现状

```
migration/src/
├── core_schema.rs          (6 子迁移)
├── business_tables.rs      (8)
├── fixes_enhancements.rs   (4，原 6 已删 m0017/m0018)
├── sales_crm.rs            (14)
├── production_quality.rs   (16)
├── finance_compliance.rs   (10)
├── v15_core.rs             (15)
├── v15_batch18.rs          (5)
├── v15_batch19.rs          (10)
├── v15_extensions.rs       (15)
├── v15_final.rs            (11)
└── lib.rs                  (11 聚合模块注册)
```

## 目标域分组

| 域目录 | 包含的聚合模块 | 子迁移数 | 说明 |
|--------|---------------|---------|------|
| `domain/system/` | core_schema | 6 | 用户/角色/部门/产品/仓库/供应商/客户基础表 |
| `domain/business/` | business_tables, fixes_enhancements | 12 | 采购/销售/库存/生产/财务扩展 |
| `domain/sales_crm/` | sales_crm | 14 | 报价单/色价/CRM |
| `domain/production/` | production_quality | 16 | 生产/质量/事件/failover |
| `domain/finance/` | finance_compliance | 10 | 合规/权限/RLS |
| `domain/v15_core/` | v15_core | 15 | 坏账/催收/预警/物料/8D |
| `domain/v15_ext/` | v15_batch18, v15_batch19, v15_extensions, v15_final | 41 | 合规/外贸/期末/设备/索引 |
| `lib.rs` | - | - | 统一注册入口 |

## 实施步骤

1. **A.20.2**：逐模块 `git mv` 迁移文件到域目录 + 更新 `#[path]` 引用
2. **A.20.3**：更新 `lib.rs` 注册 + 验证迁移顺序不变

## 风险

- 迁移顺序必须保持不变（lib.rs 的 `vec![]` 顺序决定执行顺序）
- `#[path]` 重映射需逐一验证（Rust 模块路径）
- 已部署库不受影响（seaql_migrations 表记录的是 version 不是文件路径）
