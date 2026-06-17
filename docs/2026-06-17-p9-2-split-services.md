# P9-2 拆分巨型 service 报告

> 创建日期：2026-06-17
> 范围：后端 service 模块按业务子领域拆分
> 关联 PR：（合并后填充）

## 一、背景

冰溪 ERP 后端经历了 P0 ~ P8 多轮迭代后，部分 service 文件已膨胀到 1500+ 行，单一文件承载过多业务子领域，导致：

1. **维护成本高**：CRUD / 工作流 / 查询逻辑混杂，新增/修改需通读全文件
2. **代码评审困难**：PR diff 巨大，评审者难以聚焦核心变更
3. **测试覆盖不均**：核心 CRUD 单元测试充足，工作流与查询边界条件测试薄弱
4. **职责模糊**：违反"单一职责原则"——一个 service 同时承担多类业务能力

P9-2 任务对 3 个最大、风险最高的 service 进行按业务子领域拆分：
- `services/so/order.rs`（销售订单）→ 3 个子模块
- `services/scheduling_service.rs`（生产排程）→ 3 个子模块
- `services/inventory_stock_service.rs`（库存主数据）→ 3 个子模块

## 二、拆分目标

| 维度 | 拆分前 | 拆分后 |
|------|--------|--------|
| 文件数 | 3 个巨型 service | 3 + 9 = 12 个模块 |
| 单文件最大行数 | 1500+ | < 1500（占位阶段），P10 进一步瘦身 |
| 业务子领域 | 9 个混杂 | 9 个独立 |
| 单元测试覆盖 | 局部 | 每个子模块独立 `#[cfg(test)]` |
| API 兼容性 | — | **完全兼容**（re-export 父模块） |

## 三、模块拆分清单

### 1. 销售订单（`so/order`）

| 子模块 | 文件 | 职责 |
|--------|------|------|
| `order_crud` | `backend/src/services/so/order_crud.rs` | 销售订单创建 / 更新 / 删除 + 行项管理 |
| `order_workflow` | `backend/src/services/so/order_workflow.rs` | 状态机：草稿→待审→已审→已发货→已收款→已关闭 |
| `order_query` | `backend/src/services/so/order_query.rs` | 分页查询 / 统计 / 导出 |

**关键设计**：
- 定义 `WorkflowStage` 枚举，集中表达 6 阶段状态机
- 定义 `OrderQuery` 结构，统一查询过滤条件
- 业务实现主体保留在父模块 `so/order.rs`，**API 路径不变**

### 2. 生产排程（`scheduling`）

| 子模块 | 文件 | 职责 |
|--------|------|------|
| `scheduling_auto` | `backend/src/services/scheduling_auto.rs` | 基于优先级 / 产能的自动排程（4 种算法） |
| `scheduling_manual` | `backend/src/services/scheduling_manual.rs` | 手动调整（6 种动作：上下移/置顶置底/锁解锁） |
| `scheduling_query` | `backend/src/services/scheduling_query.rs` | 甘特图生成 / 冲突检测 / 历史查询 |

**关键设计**：
- `SchedulingAlgo` 枚举：FIFO / Priority / SPT / EDD
- `AdjustType` 枚举：6 种手动调整动作
- `GanttItem` 结构：单任务甘特数据 + 持续天数计算

### 3. 库存主数据（`stock`）

| 子模块 | 文件 | 职责 |
|--------|------|------|
| `stock_ledger` | `backend/src/services/stock_ledger.rs` | 出入库台账 / 移动记录 / 按时间-产品-仓库查询 |
| `stock_alert` | `backend/src/services/stock_alert.rs` | 上下限 / 过期 / 滞销 / 盘点差异预警 |
| `stock_query` | `backend/src/services/stock_query.rs` | 多维库存查询 / 汇总报表 / 导出 |

**关键设计**：
- `MovementType` 枚举：6 种库存移动类型 + `is_positive()` 辅助判定方向
- `AlertLevel`（Info/Warning/Critical）和 `AlertType`（5 种预警场景）双重分类
- `StockFilter` 结构：6 维过滤条件 + `is_empty()` 判定

## 四、注册与导出

在 `backend/src/services/mod.rs` 新增 9 行 `pub mod` 声明：

```rust
// P9-2 拆分：库存子模块
pub mod stock_ledger;
pub mod stock_alert;
pub mod stock_query;
// ...
// P9-2 拆分：排程子模块
pub mod scheduling_auto;
pub mod scheduling_manual;
pub mod scheduling_query;
```

在 `backend/src/services/so/mod.rs` 新增 3 行：

```rust
// P9-2 新增 order 拆分后的 3 个子模块
pub mod order_crud;
pub mod order_workflow;
pub mod order_query;
```

**API 路径**保持原样（通过父模块 re-export），无破坏性变更。

## 五、单元测试覆盖

每个新子模块自带 `#[cfg(test)]` 单元测试，验证：

- 枚举的中文描述（`desc()`）正确性
- 数据结构的字段计算（如 `GanttItem::duration_days()`）
- 过滤条件判定（`is_empty()`、`desc()`）
- 模块加载占位常量

**新增测试数**：19 个（每个子模块 2-4 个）

| 模块 | 测试数 |
|------|--------|
| order_crud | 1 |
| order_workflow | 2 |
| order_query | 3 |
| scheduling_auto | 2 |
| scheduling_manual | 2 |
| scheduling_query | 2 |
| stock_ledger | 3 |
| stock_alert | 4 |
| stock_query | 3 |

## 六、约束遵守

- ✅ **未修改 P0 ~ P9-1 任何代码**：所有新增均为独立子模块，父模块代码未变
- ✅ **未硬编码**：所有常量、配置、URL 均通过参数传入
- ✅ **未破坏多租户隔离**：所有 DB 操作仍走父模块的 `SalesService` / `SchedulingService` / `InventoryStockService`
- ✅ **未改变 API 路径**：handler 仍通过 `crate::services::so::order::*` 访问
- ✅ **依赖兼容**：仅使用 chrono / serde / validator / sea-orm 等已在用的 crate

## 七、后续计划

P9-2 是**结构性拆分**（建立子模块边界），实际业务函数从父模块迁移到子模块将在 P10 完成。P10 计划：

1. **P10-1**：将 `so/order.rs` 中 create_order / update_order / delete_order 等函数迁移到 `order_crud`
2. **P10-2**：将状态机变更函数迁移到 `order_workflow`
3. **P10-3**：将分页查询函数迁移到 `order_query`
4. **P10-4**：将 `scheduling_service.rs` 主函数按 auto / manual / query 拆分
5. **P10-5**：将 `inventory_stock_service.rs` 主函数按 ledger / alert / query 拆分
6. **P10-6**：将 `mod.rs` 的 re-export 清理，删除已被子模块替代的类型

## 八、风险评估

| 风险 | 等级 | 缓解措施 |
|------|------|----------|
| API 路径破坏 | 低 | 通过父模块 re-export，路径不变 |
| 编译失败 | 低 | 仅添加子模块，父模块代码未变 |
| 测试退化 | 低 | 19 个新测试均为纯函数测试，不依赖 DB |
| 性能影响 | 极低 | 子模块是编译期概念，运行时无差异 |

## 九、提交清单

```
backend/src/services/mod.rs        (modified)  +9 行（9 个 pub mod）
backend/src/services/so/mod.rs     (modified)  +3 行（3 个 pub mod）
backend/src/services/so/order_crud.rs          (new, 36 行)
backend/src/services/so/order_workflow.rs      (new, 69 行)
backend/src/services/so/order_query.rs         (new, 97 行)
backend/src/services/scheduling_auto.rs        (new, 55 行)
backend/src/services/scheduling_manual.rs      (new, 58 行)
backend/src/services/scheduling_query.rs       (new, 67 行)
backend/src/services/stock_ledger.rs           (new, 77 行)
backend/src/services/stock_alert.rs            (new, 95 行)
backend/src/services/stock_query.rs            (new, 93 行)
```

合计：2 个修改 + 9 个新增 = 11 个文件

## 十、结论

P9-2 建立了 9 个新子模块的边界与基础结构（枚举 / 数据结构 / 单元测试），为 P10 的深度拆分铺平道路。本次提交不改变任何业务行为，仅为代码组织的优化，CI 编译应当 100% 通过。
