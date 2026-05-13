# 秉羲ERP未实现需求清单

**版本**: v1.0
**日期**: 2026-05-09
**状态**: 待开发

---

## 高优先级 (P1) - 建议立即开发

### REQ-001: 生产计划管理 (MRP)
**业务价值**: ★★★★★ (面料行业核心)
**技术复杂度**: ★★★★☆
**预估工期**: 3-4周

**需求描述**:
基于销售订单和库存情况，自动生成生产计划和物料需求计划。

**功能点**:
1. **生产订单管理**
   - 生产订单CRUD
   - 订单状态流转(草稿->已排产->生产中->已完成)
   - 订单优先级管理
   
2. **BOM物料清单**
   - BOM结构定义(多级)
   - BOM版本管理
   - BOM用量计算
   
3. **物料需求计算(MRP)**
   - 毛需求计算(基于销售订单)
   - 净需求计算(考虑库存/在途/在制)
   - 需求日期推算(基于提前期)
   
4. **产能负荷分析**
   - 设备产能维护
   - 负荷计算与可视化
   - 瓶颈识别
   
5. **生产排程**
   - 自动排程算法
   - 甘特图展示
   - 手动调整
   
6. **缺料预警**
   - 预警规则配置
   - 预警通知
   - 缺料报表

**涉及文件**:
- 新建: `models/production_order.rs`
- 新建: `models/bom.rs`
- 新建: `models/mrp_calculation.rs`
- 新建: `services/production_planning_service.rs`
- 新建: `handlers/production_order_handler.rs`
- 修改: `routes/mod.rs`

**数据库表**:
```sql
-- 生产订单表
CREATE TABLE production_orders (
    id SERIAL PRIMARY KEY,
    order_no VARCHAR(50) UNIQUE NOT NULL,
    sales_order_id INTEGER REFERENCES sales_orders(id),
    product_id INTEGER NOT NULL,
    planned_quantity DECIMAL(15,4) NOT NULL,
    actual_quantity DECIMAL(15,4),
    planned_start_date DATE,
    planned_end_date DATE,
    actual_start_date DATE,
    actual_end_date DATE,
    status VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    priority INTEGER DEFAULT 5,
    work_center_id INTEGER,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- BOM表
CREATE TABLE boms (
    id SERIAL PRIMARY KEY,
    product_id INTEGER NOT NULL,
    version INTEGER DEFAULT 1,
    is_default BOOLEAN DEFAULT false,
    status VARCHAR(20) DEFAULT 'ACTIVE',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- BOM明细表
CREATE TABLE bom_items (
    id SERIAL PRIMARY KEY,
    bom_id INTEGER REFERENCES boms(id),
    material_id INTEGER NOT NULL,
    quantity DECIMAL(15,4) NOT NULL,
    unit VARCHAR(20),
    scrap_rate DECIMAL(5,4) DEFAULT 0,
    sort_order INTEGER
);

-- MRP计算结果表
CREATE TABLE mrp_results (
    id SERIAL PRIMARY KEY,
    calculation_no VARCHAR(50) UNIQUE,
    product_id INTEGER NOT NULL,
    required_quantity DECIMAL(15,4),
    required_date DATE,
    source_type VARCHAR(20), -- SALES_ORDER/FORECAST
    source_id INTEGER,
    planned_order_quantity DECIMAL(15,4),
    planned_order_date DATE,
    status VARCHAR(20) DEFAULT 'PLANNED'
);
```

---

### REQ-002: 数据外键关联统一
**业务价值**: ★★★★★ (数据一致性)
**技术复杂度**: ★★★☆☆
**预估工期**: 1-2周

**需求描述**:
BPM/CRM/ERP核心实体间建立数据库外键关联，消除数据孤岛。

**改进清单**:

1. **BPM流程实例关联**
   ```rust
   // 当前: business_type: String, business_id: i32
   // 改进: 添加枚举约束 + 联合索引
   ```
   - 添加 `business_type` 枚举: SALES_ORDER/PURCHASE_ORDER/...
   - 添加联合索引 `(business_type, business_id)`
   - 添加外键约束(视具体业务表而定)

2. **CRM线索与客户关联**
   ```rust
   // 当前: crm_lead 表无 customer_id 字段
   // 改进: 添加 customer_id 外键
   ```
   - `crm_lead` 表添加 `customer_id INTEGER REFERENCES customers(id)`
   - 线索转换时自动关联客户

3. **成本归集与生产批次关联**
   ```rust
   // 当前: batch_no: String (字符串关联)
   // 改进: 添加 batch_id 外键
   ```
   - `cost_collection` 表添加 `batch_id INTEGER REFERENCES batches(id)`
   - 保留 `batch_no` 用于显示

4. **质量检验与采购入库关联**
   ```rust
   // 当前: 弱关联
   // 改进: 添加 purchase_receipt_id 外键
   ```
   - `quality_inspection_records` 表添加 `receipt_id INTEGER REFERENCES purchase_receipts(id)`

**涉及文件**:
- 修改: 多个models文件
- 新建: `database/migration/003_foreign_keys.sql`
- 修改: 相关services的查询逻辑

---

### REQ-003: 权限检查补全
**业务价值**: ★★★★★ (安全性)
**技术复杂度**: ★★☆☆☆
**预估工期**: 1周

**需求描述**:
审查所有Handler，确保敏感操作都有权限验证。

**检查清单**:
- [ ] 所有POST/PUT/DELETE操作检查权限
- [ ] 敏感查询添加数据范围限制
- [ ] 管理员操作添加二次确认
- [ ] 操作日志记录完整

**涉及文件**:
- 审查: `handlers/*.rs` (60+文件)
- 修改: `middleware/permission.rs`
- 修改: `services/audit_log_service.rs`

---

## 中优先级 (P2) - 建议近期开发

### REQ-004: 应收对账模块
**业务价值**: ★★★★☆ (财务闭环)
**技术复杂度**: ★★★☆☆
**预估工期**: 2-3周

**功能点**:
1. 应收对账单生成(参考应付对账)
2. 客户对账确认/争议
3. 账龄分析报表
4. 自动对账(匹配发票与收款)

**参考实现**:
- 复制 `ap_reconciliation_service.rs` 逻辑
- 修改为客户维度

---

### REQ-005: 多币种支持
**业务价值**: ★★★★☆ (外贸需求)
**技术复杂度**: ★★★☆☆
**预估工期**: 2周

**功能点**:
1. 币种管理(CNY/USD/EUR/...)
2. 汇率管理(手动/自动更新)
3. 多币种订单/发票
4. 本位币换算

**数据库表**:
```sql
CREATE TABLE currencies (
    id SERIAL PRIMARY KEY,
    code VARCHAR(3) UNIQUE NOT NULL,
    name VARCHAR(50),
    symbol VARCHAR(10),
    is_base BOOLEAN DEFAULT false
);

CREATE TABLE exchange_rates (
    id SERIAL PRIMARY KEY,
    from_currency VARCHAR(3),
    to_currency VARCHAR(3),
    rate DECIMAL(18,8),
    effective_date DATE,
    source VARCHAR(20) -- MANUAL/API
);
```

---

### REQ-006: 单元测试覆盖
**业务价值**: ★★★☆☆ (质量保障)
**技术复杂度**: ★★☆☆☆
**预估工期**: 2-3周

**测试范围**:
1. 核心Service层(优先)
   - auth_service
   - user_service
   - sales_service
   - purchase_order_service
   
2. 工具函数
   - password_validator
   - cache
   - dual_unit_converter

3. 集成测试
   - 登录流程
   - 订单创建流程
   - 审批流程

---

### REQ-007: 代码文档补全
**业务价值**: ★★★☆☆ (可维护性)
**技术复杂度**: ★☆☆☆☆
**预估工期**: 1周

**要求**:
- 所有pub函数添加 `///` 文档注释
- 模块级文档 `//!`
- 复杂逻辑添加示例代码

---

## 低优先级 (P3) - 长期规划

### REQ-008: AI智能分析
**业务价值**: ★★★★☆ (差异化竞争)
**技术复杂度**: ★★★★★
**预估工期**: 4-6周

**功能点**:
1. 销售预测(时间序列分析)
2. 库存优化(安全库存计算)
3. 异常检测(数据波动识别)
4. 智能推荐(采购建议)

**技术方案**:
- 使用Rust ML库(如smartcore)
- 或调用外部Python服务

---

### REQ-009: 多租户SaaS
**业务价值**: ★★★★☆ (商业模式)
**技术复杂度**: ★★★★☆
**预估工期**: 4-6周

**功能点**:
1. 租户数据隔离(Schema/Row级别)
2. 租户配置管理
3. 租户权限体系
4. 计费系统

---

### REQ-010: 插件机制
**业务价值**: ★★★☆☆ (生态扩展)
**技术复杂度**: ★★★★☆
**预估工期**: 3-4周

**功能点**:
1. 插件接口定义(WASM?)
2. 插件加载器
3. 插件生命周期管理
4. 插件市场(基础版)

---

## 需求优先级矩阵

```
            高业务价值
                |
    REQ-001     |     REQ-008
    (MRP)       |     (AI分析)
                |
    REQ-002     |     REQ-009
    (数据关联)   |     (多租户)
                |
    REQ-003     |     REQ-010
    (权限补全)   |     (插件机制)
                |
    REQ-004     |     REQ-005
    (应收对账)   |     (多币种)
                |
    REQ-006     |     REQ-007
    (单元测试)   |     (文档)
                |
低技术复杂度 ----+---- 高技术复杂度
```

---

## 开发建议顺序

### 第一阶段 (立即开始)
1. REQ-003 权限检查补全 (1周，高价值低复杂度)
2. REQ-002 数据外键关联 (1-2周，基础工作)

### 第二阶段 (第2-3周开始)
3. REQ-001 MRP生产计划 (3-4周，核心功能)
4. REQ-004 应收对账 (2-3周，参考现有)

### 第三阶段 (第5周开始)
5. REQ-005 多币种支持 (2周)
6. REQ-006 单元测试 (2-3周，可并行)
7. REQ-007 代码文档 (1周，可并行)

### 第四阶段 (长期规划)
8. REQ-008 AI智能分析
9. REQ-009 多租户SaaS
10. REQ-010 插件机制

---

**文档结束**
