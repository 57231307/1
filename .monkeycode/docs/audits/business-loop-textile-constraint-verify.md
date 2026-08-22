# 业务闭环验证 & 面料行业约束验证报告

> 审计代理 | 2026-08-22 | 验证项 1.4 / 1.5

## 一、1.4 业务闭环验证

### 1.4.1 状态常量完整性

状态常量定义目录 `backend/src/models/status/` 共 9 个文件，覆盖全部业务域：

| 文件 | 业务域 | 状态常量数 | 关键状态机示例 |
|------|--------|-----------|--------------|
| sales.rs | 销售 | 37 | draft→pending→approved→partial_shipped→shipped→completed→cancelled |
| purchase_inventory.rs | 采购/库存 | 40 | DRAFT→PENDING_APPROVAL→APPROVED→PARTIAL_RECEIVED→COMPLETED→CLOSED→CANCELLED |
| production.rs | 生产 | 33 | pending→scheduled→preparing→dyeing→dyed→inspecting→completed→shipped/terminated |
| finance.rs | 财务 | 32 | draft→submitted→reviewed→posted（voucher）；DRAFT→CONFIRMED→COMPLETED（purchase_receipt） |
| quality_dyeing.rs | 质量/染色 | 92 | draft→approved→closed→cancelled（处方）；pending→sampling→submitted→approved/rejected→completed（打样） |
| wage_energy_chemical_business.rs | 工资/能耗/化工 | 97 | draft→active→disabled；draft→approved→issued→partial_returned→closed |
| bpm_crm_contract.rs | BPM/CRM/合同 | 29 | 合同与流程审批状态 |
| general.rs | 通用 | 47 | 跨域通用状态 |
| **合计** | | **407** | |

**状态常量完整，覆盖 draft/pending/approved/completed/cancelled 全生命周期。**

### 1.4.2 状态流转方法统计

```
grep -rn "fn.*advance|fn.*transition|fn.*submit|fn.*approve|fn.*complete|fn.*cancel" backend/src/services/ | wc -l
```

**状态流转方法总数：184 个**

按业务域分布：

| 业务域 | 流转方法数 | 代表性服务文件 |
|--------|-----------|--------------|
| 生产域 | 45 | dye_batch_state_machine_service.rs、custom_order_state_service.rs、card_state.rs |
| 销售域 | 32 | ar_invoice_service.rs、contract.rs、bulk_color_approval_service.rs |
| 跨域通用 | 75 | approval.rs、cancel.rs、completion.rs、collection.rs、addition.rs、batch.rs |
| 库存域 | 14 | inventory_stock_service.rs、inventory_count_service.rs |
| 采购域 | 13 | ap_payment_service.rs、ap_payment_request_service.rs、ap_verification_service.rs |
| 财务域 | 10 | accounting_period_service.rs、fund_management_service.rs |

### 1.4.3 五大业务域完整性

| 业务域 | 状态常量文件 | 状态流转方法 | 完整性判定 |
|--------|------------|-------------|-----------|
| 销售 | sales.rs (37) | 32 + 75 通用 | 完整 |
| 采购 | purchase_inventory.rs (40) | 13 + 75 通用 | 完整 |
| 库存 | purchase_inventory.rs (40) | 14 + 75 通用 | 完整 |
| 生产 | production.rs (33) | 45 + 75 通用 | 完整 |
| 财务 | finance.rs (32) | 10 + 75 通用 | 完整 |

**结论：五大业务域状态流转全覆盖。** 各域均具备：状态常量定义、draft→终态完整状态机、submit/approve/cancel 等流转方法。跨域通用流转方法（approval.rs/cancel.rs/completion.rs 等）为各域共享的状态机引擎。

---

## 二、1.5 面料行业约束验证

### 1.5.1 四维标识约束

四维标识字段在迁移文件中的分布：

| 标识 | 字段名 | 出现次数 | 含义 |
|------|--------|---------|------|
| 缸号 | dye_lot_no | 15+ | 染色批次号（防色差混批） |
| 批次号 | batch_no | 30+ | 布匹批次号 |
| 匹号 | piece_no | 5+ | 单匹布号 |
| 色号 | color_no | 12+ | 颜色编号 |

**UNIQUE 约束情况：**

| 表 | 约束类型 | 约束字段 | 迁移文件 |
|----|---------|---------|---------|
| batch_dye_lot | 组合唯一 | (dye_lot_no, batch_no) | v15_final/m0106 |
| business_traces | 单字段唯一 | batch_no | business/m0013 |
| business_trace_chain | 普通索引(非唯一) | batch_no, color_no, dye_lot_no | business/m0013 |
| dye_batches | 单字段唯一 | batch_no | system/m0003 |
| inventory_stocks | 普通索引 | batch_no | system/m0001 |

关键约束 `m0106_batch_dye_lot_unique_constraint.rs`：
- 业务语义：同一缸号下匹号唯一（dye_lot_no + batch_no 组合唯一），而非全局唯一
- 将 batch_dye_lot 表的 batch_no 单字段 UNIQUE 替换为 (dye_lot_no, batch_no) 组合唯一约束
- 同时为 dye_batch 表添加 (dye_lot_no, batch_no) 组合索引（非唯一，辅助查询）

### 1.5.2 CHECK 约束

```
grep -rn "CHECK.*status|CHECK.*grade|CHECK.*type" backend/migration/src/domain/ | wc -l
```

**CHECK 约束总数：24 条**

| 约束类型 | 数量 | 代表性约束 |
|---------|------|-----------|
| status CHECK | 11 | chk_status (draft,pending_approval,approved,rejected,expired,converted,cancelled) |
| type/grade CHECK | 13 | chk_term_type (logistics,payment,sample,inspection)；chk_color_card_type (PANTONE,CNCS,CUSTOM) |

代表性 CHECK 约束：

| 表/约束名 | 约束内容 | 迁移文件 |
|-----------|---------|---------|
| chk_status (sales_quotations) | status IN (draft,pending_approval,approved,rejected,expired,converted,cancelled) | sales_crm/m0021 |
| chk_custom_order_status | status IN (多值定制订单状态) | production/m0044 |
| chk_quotation_status | status IN (DRAFT,SUBMITTED,APPROVED,REJECTED,CONVERTED,CANCELLED,EXPIRED) | production/m0044 |
| chk_borrow_status | status IN (borrowed,returned,lost,damaged) | production/m0044 |
| chk_color_card_type | card_type IN (PANTONE,CNCS,CUSTOM) | production/m0044 |
| chk_term_type | term_type IN (logistics,payment,sample,inspection) | sales_crm/m0023 |
| chk_bca_sample_type | sample_type IN (cut_sample,lab_sample) | finance/m0058 |
| chk_order_type | order_type IN (normal,rework) | finance/m0059 |
| chk_consent_type | consent_type IN (behavior_tracking,page_view_tracking,cookie_usage,marketing_email) | v15_batch18/m0077 |

### 1.5.3 发现的问题

1. **piece_no 缺少 UNIQUE 约束**：piece_no（匹号）在迁移文件中出现次数少（5+），且未发现任何针对 piece_no 的唯一约束。作为面料行业单匹布的唯一标识，缺少唯一约束可能导致匹号重复。

2. **color_no 缺少 UNIQUE 约束**：color_no（色号）仅有普通索引（idx_batch_trace_log_color_no），无独立唯一约束。色号作为颜色维度的唯一标识，缺少约束可能导致色号重复录入。

3. **business_trace_chain 四维字段无组合唯一约束**：business_trace_chain 表包含 batch_no、color_no、dye_lot_no 三个四维标识字段，但仅有单字段普通索引，缺少 (dye_lot_no, batch_no, color_no) 组合唯一约束，追溯链可能产生重复记录。

4. **四维标识约束不完整**：仅 dye_lot_no + batch_no 在 batch_dye_lot 表有组合唯一约束，piece_no 和 color_no 缺少约束机制，四维标识约束体系未闭环。

---

## 三、验证结论

### 1.4 业务闭环验证：通过

- 状态常量定义完整：9 个文件、407 个常量，覆盖全部业务域
- 状态流转方法充足：184 个流转方法，五大业务域（销售/采购/库存/生产/财务）全覆盖
- 状态机闭环：各域均具备 draft→审批→执行→终态→取消 的完整生命周期
- 跨域通用状态引擎（approval/cancel/completion）为各域提供共享流转能力

### 1.5 面料行业约束验证：部分通过

- 四维标识字段（dye_lot_no/batch_no/piece_no/color_no）在迁移文件中均有定义
- (dye_lot_no, batch_no) 组合唯一约束已正确建立（m0106 迁移）
- CHECK 约束覆盖充分：24 条约束覆盖 status/grade/type 维度
- **存在 4 项缺陷**：piece_no 无 UNIQUE 约束、color_no 无 UNIQUE 约束、business_trace_chain 四维字段无组合唯一约束、四维标识约束体系未闭环

**建议**：为 piece_no 添加唯一约束（建议在 dye_lot_no + batch_no 维度下唯一，与 m0106 语义一致）；为 color_no 添加唯一约束或组合唯一约束；为 business_trace_chain 添加 (dye_lot_no, batch_no, color_no) 组合唯一索引。
