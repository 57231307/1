# 冰溪 ERP P0-3 测试场景

> **版本**: v1.0
> **时间**: 2026-06-17

## 1. 工艺流程测试用例

### 1.1 完整生命周期（happy path）

**场景**：客户下定制订单，完成全部 5 阶段工艺流程

**步骤**：
1. 销售员创建定制订单
   - customer_id=1, product_id=1, spec="100% 棉 200g/m²", quantity=100
   - 期望：返回 order_no + 状态 = draft
2. 推进到纱线采购
   - POST /:id/advance
   - 期望：状态 = yarn_purchasing
3. 推进到染整
   - 期望：状态 = dyeing
4. 推进到后整理
   - 期望：状态 = finishing
5. 推进到交付
   - 期望：状态 = delivery
6. 推进到售后
   - 期望：状态 = after_sales
7. 推进到完成
   - 期望：状态 = completed

**验证点**：
- 每个阶段切换时工艺节点状态正确
- 工艺日志记录完整
- 时间戳正确

### 1.2 阶段跳跃拒绝

**场景**：尝试从草稿直接跳到染整

**步骤**：
1. 创建草稿（status=draft）
2. 尝试推进两次

**期望**：
- 第一次推进：状态 = yarn_purchasing ✅
- 第二次推进：状态 = dyeing ✅
- 不会出现跳跃（draft → dyeing 不会被直接允许）

### 1.3 工艺节点阻塞

**场景**：纱线采购阶段供应链延迟

**步骤**：
1. 创建订单，推进到纱线采购
2. 阻塞当前节点：POST /:id/nodes/:nid/advance action=block, notes="原料供应延迟"
3. 解除阻塞：action=unblock
4. 继续推进

**期望**：
- 阻塞后节点状态 = blocked
- 解除后状态 = in_progress
- 完整日志记录

### 1.4 终态不可推进

**场景**：已完成订单尝试再次推进

**步骤**：
1. 创建订单，完整推进到 completed
2. 尝试 POST /:id/advance

**期望**：
- 返回 409 Conflict
- 状态保持 completed

### 1.5 取消订单

**场景**：客户主动取消草稿订单

**步骤**：
1. 创建草稿
2. DELETE /:id with reason="客户主动取消"

**期望**：
- 状态 = cancelled
- 记录取消原因

## 2. 行业规则校验测试

### 2.1 GB/T 26377-2022 色差标准

**场景**：染色色差超阈值

**步骤**：
1. 创建订单，推进到染整阶段
2. 上报色差异常：color_delta_e=4.2
3. 上报色差异常：color_delta_e=6.0

**期望**：
- ΔE=4.2：上报成功
- ΔE=6.0：上报成功 + 警告日志 "色差 ΔE=6.0 超过行业警告阈值 5.0"

### 2.2 ISO 105 色牢度

**场景**：色牢度不达标

**步骤**：
1. 上报色牢度异常：color_fastness_grade=2
2. 尝试 color_fastness_grade=6

**期望**：
- 等级 2：上报成功
- 等级 6：返回 400 Validation 错误

### 2.3 严重度枚举

**测试**：
- 有效值：low/medium/high/critical ✅
- 无效值：normal/urgent → 400

## 3. 售后工单测试

### 3.1 4 种类型创建

**步骤**：
1. 创建售后工单 issue_type=complaint
2. 创建售后工单 issue_type=repair
3. 创建售后工单 issue_type=exchange
4. 创建售后工单 issue_type=refund + refund_amount=2500

**期望**：
- 4 个工单全部创建成功
- refund 类型金额校验通过

### 3.2 退款类型必须填金额

**步骤**：
1. 创建 refund 类型工单，refund_amount=null

**期望**：
- 返回 400 Validation "退款类型工单必须填写退款金额"

### 3.3 售后状态机

**转换测试**：
- opened → processing ✅
- opened → rejected ✅
- processing → resolved ✅
- resolved → closed ✅
- closed → processing ❌
- opened → resolved ❌

## 4. 多租户隔离测试

### 4.1 跨租户访问拒绝

**步骤**：
1. 租户 A 创建订单 ID=1
2. 租户 B 访问 GET /api/v1/erp/custom-orders/1

**期望**：
- 返回 403 Forbidden
- 不返回任何订单信息

### 4.2 列表过滤

**步骤**：
1. 租户 A 创建 3 个订单
2. 租户 B 创建 2 个订单
3. 租户 A 列出订单

**期望**：
- 租户 A 列表仅返回 3 个订单
- 租户 B 列表仅返回 2 个订单

## 5. 边界场景

### 5.1 数量边界

- quantity=0：返回 400 "数量必须大于 0"
- quantity=0.01：创建成功
- quantity=999999.99：创建成功

### 5.2 规格空

- spec=""：返回 400 "规格不能为空"
- spec=" "（纯空格）：返回 400（实际：trim 后空）

### 5.3 重复订单号

- 自动生成 order_no 包含日期 + 序号，避免冲突

## 6. 性能测试

| 场景 | 期望 |
|------|------|
| 创建订单（含 5 节点） | < 200ms |
| 推进状态 | < 100ms |
| 列表查询（20 条/页） | < 300ms |
| 时间线查询 | < 200ms |

## 7. 集成测试

- `custom_order_e2e_test.rs`：完整生命周期
- `custom_order_state_test.rs`：状态机转换矩阵
- `custom_order_process_test.rs`：工艺节点
- `custom_order_quality_test.rs`：质检规则
- `custom_order_aftersales_test.rs`：售后流程

总计 19 个测试用例，覆盖率目标 > 80%。
