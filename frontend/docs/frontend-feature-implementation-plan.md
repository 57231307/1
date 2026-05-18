# 面料管理系统 - 前端功能实现规划

## 现状分析

### 后端已提供的功能模块（完整）
基于 `backend/src/routes/mod.rs` 分析，后端提供了以下功能：

1. **系统管理模块**
   - 用户管理 (users)
   - 角色权限管理 (roles)
   - 部门管理 (departments)
   - 数据权限管理 (data-permissions)
   - API密钥管理 (api-keys)
   - Webhook管理 (webhooks)
   - 系统更新 (system-update)
   - 初始化管理 (init)

2. **基础资料模块**
   - 产品管理 (products) - 含产品颜色、批量操作、导入导出
   - 产品类别管理 (product-categories)
   - 仓库管理 (warehouses) - 含库位管理
   - 客户管理 (customers)
   - 供应商管理 (suppliers) - 含供应商评估

3. **业务运营模块**
   - 销售管理 (sales)
     - 销售订单 (orders)
     - 面料销售订单 (fabric-orders)
     - 销售分析 (sales-analysis)
     - 销售价格 (sales-prices)
     - 销售退货 (sales-returns)
     - 销售合同 (sales-contracts)
     - 客户信用 (customer-credits)
   
   - 采购管理 (purchases)
     - 采购订单 (orders)
     - 采购收货 (receipts)
     - 采购检验 (inspections)
     - 采购退货 (returns)
     - 采购合同 (purchase-contracts)
     - 采购价格 (purchase-prices)

   - 库存管理 (inventory)
     - 库存查询 (stock)
     - 库存盘点 (counts)
     - 库存调整 (adjustments)
     - 库存调拨 (transfers)
     - 批次管理 (batches)
     - 分匹管理 (piece-split)
     - 扫码出库 (scanner)
     - 物流管理 (logistics)

4. **面料行业特色模块**
   - 染色批次管理 (dye-batches)
   - 坯布管理 (greige-fabrics)
   - 染色配方管理 (dye-recipes)

5. **财务模块**
   - 财务基础 (finance)
     - 财务发票 (invoices)
     - 财务支付 (payments)
     - 会计期间 (accounting-periods)
     - 财务报表 (balance-sheet, income-statement)
   - 总账管理 (gl)
     - 会计科目 (subjects)
     - 凭证管理 (vouchers)
   - 应付账款 (ap)
     - 应付发票 (invoices)
     - 应付支付 (payments)
     - 应付请款 (payment-requests)
     - 应付对账 (reconciliations)
     - 应付核销 (verifications)
     - 应付报表 (reports)
   - 应收账款 (ar)
     - 应收发票 (invoices)
     - 应收对账 (ar-reconciliations)
   - 成本管理 (cost-collections)
   - 预算管理 (budgets)
   - 固定资产 (fixed-assets)
   - 资金管理 (fund-management)
   - 财务分析 (financial-analysis)
   - 多币种 (currencies, exchange-rates)

6. **质量管理模块**
   - 质量标准 (quality-standards)
   - 质量检验 (quality-inspection)
     - 检验标准 (standards)
     - 检验记录 (records)
     - 缺陷管理 (defects)

7. **审批流程模块 (BPM)**
   - 流程实例
   - 审批任务
   - 流程监控

8. **生产计划模块 (MRP)**
   - 生产订单 (production/orders)

9. **客户关系管理 (CRM)**
   - 线索管理 (leads)
   - 商机管理 (opportunities)

10. **高级功能模块**
    - AI智能分析 (ai)
      - 销售预测
      - 库存优化
      - 异常检测
      - 智能推荐
    - 报表引擎 (reports)
      - 报表模板
      - 报表执行
      - 报表导出
    - 多租户SaaS (tenants)

11. **其他辅助模块**
    - 五维管理 (five-dimension)
    - 辅助核算 (assist-accounting)
    - 业务追溯 (business-trace)
    - 双单位换算 (dual-unit)
    - 全量审计 (omni-audit)
    - 通知中心 (notifications)
    - 用户通知设置 (user/notification-setting)
    - 页面访问统计 (tracking)

### 前端当前实现状态
基于 `frontend/src/views/` 分析，现有页面：

✅ **已实现**
- Dashboard - 仪表盘
- Login - 登录
- System - 系统管理（用户、角色、部门）
- Customer - 客户管理
- Supplier - 供应商管理
- Product - 产品管理
- Warehouse - 仓库管理
- Inventory - 库存管理
- Purchase - 采购管理
- Sales - 销售管理
- Finance - 财务管理
- Fabric - 面料行业特色
- Quality - 质量管理
- CRM - 客户关系管理
- Advanced - 高级功能
- AP - 应付管理
- AR - 应收管理
- BPM - 审批管理
- Purchase-Ext - 采购扩展
- Sales-Ext - 销售扩展
- Trading - 购销管理
- 403, 404 - 错误页面

---

## 缺失功能规划

### 阶段一：完善核心业务模块（高优先级）

#### 1. 生产计划管理 (MRP)
**功能点：**
- 生产订单列表/查询
- 生产订单创建/编辑
- 生产订单状态管理
- 生产订单详情

**文件创建：**
- `src/views/production/index.vue`
- `src/api/production.ts`
- 更新 `src/router/index.ts`

#### 2. 物流管理
**功能点：**
- 物流运单列表/查询
- 运单创建/编辑
- 运单状态更新
- 运单删除

**文件创建：**
- `src/views/logistics/index.vue`
- `src/api/logistics.ts`
- 更新路由

#### 3. 供应商评估
**功能点：**
- 供应商评估列表/查询
- 评估创建/编辑
- 评估指标管理
- 供应商排名

**文件创建：**
- `src/views/supplier-evaluation/index.vue`
- `src/api/supplier-evaluation.ts`
- 更新路由

#### 4. 成本归集管理
**功能点：**
- 成本归集列表/查询
- 成本归集创建/编辑
- 成本分析汇总
- 批次成本分析

**文件创建：**
- `src/views/cost/index.vue`
- `src/api/cost.ts`
- 更新路由

#### 5. 预算管理
**功能点：**
- 预算列表/查询
- 预算创建/编辑
- 预算审批
- 预算调整
- 预算执行记录
- 预算控制

**文件创建：**
- `src/views/budget/index.vue`
- `src/api/budget.ts`
- 更新路由

#### 6. 客户信用管理
**功能点：**
- 信用额度列表/查询
- 信用创建/编辑
- 信用评级
- 信用占用/释放
- 信用额度调整

**文件创建：**
- `src/views/customer-credit/index.vue`
- `src/api/customer-credit.ts`
- 更新路由

#### 7. 固定资产管理
**功能点：**
- 资产列表/查询
- 资产创建/编辑
- 资产折旧
- 资产删除

**文件创建：**
- `src/views/asset/index.vue` - 更新现有，因为已有 `asset.ts`
- 补充功能到现有页面

#### 8. 资金管理
**功能点：**
- 资金账户列表/查询
- 账户创建/编辑
- 存款/取款
- 资金冻结/解冻
- 资金划转

**文件创建：**
- `src/views/fund/index.vue`
- `src/api/fund.ts`
- 更新路由

#### 9. 财务分析
**功能点：**
- 财务报表列表/查询
- 报表创建/执行
- 财务指标
- 趋势分析

**文件创建：**
- `src/views/financial-analysis/index.vue`
- `src/api/financial-analysis.ts`
- 更新路由

---

### 阶段二：完善管理辅助模块（中优先级）

#### 10. 总账管理完整实现
**功能点：**
- 会计科目管理（树形结构）
- 凭证管理（创建、提交、审核、过账）
- 会计期间管理

**现有文件优化：**
- 优化 `src/views/finance/index.vue` - 补充凭证管理和会计期间

#### 11. 五维管理
**功能点：**
- 五维统计查询
- 五维数据搜索
- 五维解析

**文件创建：**
- `src/views/five-dimension/index.vue`
- `src/api/five-dimension.ts`
- 更新路由

#### 12. 辅助核算
**功能点：**
- 辅助核算维度查询
- 辅助核算记录查询
- 辅助核算汇总

**文件创建：**
- `src/views/assist-accounting/index.vue`
- `src/api/assist-accounting.ts`
- 更新路由

#### 13. 业务追溯
**功能点：**
- 五维码追溯
- 正向追溯
- 反向追溯
- 追溯快照

**文件创建：**
- `src/views/business-trace/index.vue`
- `src/api/business-trace.ts`
- 更新路由

#### 14. 批次管理
**功能点：**
- 批次列表/查询
- 批次创建/编辑
- 批次转移
- 批次删除

**文件创建：**
- `src/views/batch/index.vue`
- `src/api/batch.ts` - 已有，补充使用
- 更新路由

#### 15. 产品类别管理
**功能点：**
- 产品类别树
- 类别创建/编辑/删除

**现有文件优化：**
- 优化 `src/views/product/index.vue` - 补充产品类别管理

#### 16. 数据权限管理
**功能点：**
- 数据权限设置
- 权限范围类型
- 角色数据权限列表

**文件创建：**
- `src/views/data-permission/index.vue`
- `src/api/data-permission.ts`
- 更新路由

---

### 阶段三：系统及高级功能（中优先级）

#### 17. 通知中心
**功能点：**
- 通知列表/查询
- 通知详情
- 标记已读/全部已读
- 批量已读
- 通知删除
- 通知设置

**文件创建：**
- `src/views/notification/index.vue`
- `src/api/notification.ts` - 已有，补充使用
- 更新路由

#### 18. API密钥管理
**功能点：**
- API密钥列表
- 密钥创建
- 密钥撤销

**文件创建：**
- `src/views/api-key/index.vue`
- `src/api/api-key.ts`
- 更新路由

#### 19. Webhook管理
**功能点：**
- Webhook列表
- Webhook创建/删除

**文件创建：**
- `src/views/webhook/index.vue`
- `src/api/webhook.ts`
- 更新路由

#### 20. 多币种管理
**功能点：**
- 币种列表
- 币种创建
- 基准货币查询
- 汇率管理
- 汇率查询

**文件创建：**
- `src/views/currency/index.vue`
- `src/api/currency.ts`
- 更新路由

#### 21. 应收对账完整实现
**功能点：**
- 应收对账单列表
- 对账单创建
- 对账状态更新

**现有文件优化：**
- 优化 `src/views/ar/index.vue` - 补充应收对账功能

---

### 阶段四：特殊功能及优化（低优先级）

#### 22. 扫码出库功能
**功能点：**
- 扫码出库界面
- 扫码确认

**文件创建：**
- `src/views/scanner/index.vue`
- 更新路由

#### 23. 分匹管理
**功能点：**
- 面料分匹操作

**现有文件优化：**
- 优化 `src/views/inventory/index.vue` - 补充分匹功能

#### 24. 全量审计
**功能点：**
- 审计事件追踪
- 审计统计
- 审计日志查询

**文件创建：**
- `src/views/audit/index.vue`
- `src/api/audit.ts`
- 更新路由

#### 25. 仪表盘优化
**现有文件优化：**
- 优化 `src/views/Dashboard.vue` - 补充更多图表和统计
- 集成后端dashboard统计API

---

## 实施顺序建议

### 第一步：阶段一（核心业务）
1. 生产计划管理
2. 预算管理
3. 成本归集管理
4. 资金管理
5. 财务分析

### 第二步：阶段二（管理辅助）
1. 供应商评估
2. 客户信用管理
3. 物流管理
4. 批次管理
5. 业务追溯

### 第三步：阶段三（系统功能）
1. 通知中心
2. 数据权限管理
3. 多币种管理
4. API密钥/Webhook管理

### 第四步：优化完善
1. 其他辅助模块
2. 功能优化和美化
3. 性能优化

---

## 关键API文件对照

| 后端路由模块 | 前端API文件 | 状态 |
|------------|-----------|-----|
| production | production.ts | ⚠️ 需完善 |
| logistics | logistics.ts | ✅ 已有 |
| supplier-evaluation | (需创建) | ❌ 缺失 |
| cost-collections | (需创建) | ❌ 缺失 |
| budgets | (需创建) | ❌ 缺失 |
| customer-credits | (需创建) | ❌ 缺失 |
| fixed-assets | asset.ts | ✅ 已有 |
| fund-management | (需创建) | ❌ 缺失 |
| financial-analysis | (需创建) | ❌ 缺失 |
| five-dimension | (需创建) | ❌ 缺失 |
| assist-accounting | (需创建) | ❌ 缺失 |
| business-trace | (需创建) | ❌ 缺失 |
| batches | batch.ts | ✅ 已有 |
| data-permissions | (需创建) | ❌ 缺失 |
| notifications | notification.ts | ✅ 已有 |
| api-keys | (需创建) | ❌ 缺失 |
| webhooks | (需创建) | ❌ 缺失 |
| currencies | (需创建) | ❌ 缺失 |
| ar-reconciliations | ar.ts | ⚠️ 需完善 |
| audit | (需创建) | ❌ 缺失 |

---

## 总结

当前前端已实现约 60-70% 的后端功能，但仍有大量核心业务模块和高级功能需要实现。按照上述四个阶段逐步实现，可以完整覆盖后端的所有功能点。
