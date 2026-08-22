# A.2.5 + A.2.7 代码重复率检测与注释完整性扫描报告

- 扫描范围：`backend/src/` 全目录，重点 `backend/src/services/`
- 扫描命令：`grep` + `awk` + `sort|uniq` 统计
- 扫描时间：2026-08-22
- 执行者：代码审计代理（只读扫描，未修改任何代码文件，未创建 PR，未推送）

## 一、扫描范围与规模

| 指标 | 数量 |
|------|------|
| `services/` 目录 Rust 文件数 | 405 |
| `services/` 目录代码总行数 | 133,871 |
| 最大单文件 `print_service.rs` | 4,995 行 |
| `pub async fn`（services 内） | 1,686 |
| `pub fn` + `pub async fn`（全 src） | 2,104 |

## 二、代码重复评估

### 2.1 CRUD 样板函数分布

| 样板函数名 | 出现次数 | 出现的 service 文件数 |
|------------|---------:|---------------------:|
| `pub async fn list*` | 196 | 140 |
| `pub async fn create*` | 172 | 137 |
| `pub async fn update*` | 114 | 103 |
| `pub async fn delete*` | 100 | 92 |
| `pub async fn get_by_id` | 72 | 71 |

> `services/` 共 405 个文件，其中 140 个文件（34.6%）包含 `list` 函数，137 个（33.8%）包含 `create` 函数，92 个（22.7%）包含 `delete` 函数。这表明 CRUD 样板代码在 service 层高度同质化。

### 2.2 命名变体碎片化（get 类函数）

同一"按 ID 查询"语义存在多种命名变体，缺乏统一约定：

| 命名变体 | 出现次数 |
|---------|---------:|
| `get_by_id` | 72 |
| `get_list` | 13 |
| `get_by_no` | 12 |
| `find_by_id` | 4 |
| `get_standard_by_id` | 2 |
| `get_price` | 2 |

`get_by_id` 与 `find_by_id` 并存（72 vs 4），命名未统一。

### 2.3 抽样对比：list 函数体模式重复

抽样 3 个 service 的 `list` 函数体，均呈现相同的骨架结构（Entity::find → 过滤 → 分页/计数 → 返回）：

- `currency_service.rs`：`CurrencyEntity::find().order_by_asc(...).all(&*self.db).await`
- `department_service.rs`：`DepartmentEntity::find()` + keyword `safe_like_pattern` + 分页
- `role_service.rs`：`role::Entity::find().paginate(&*self.db, page_size)` + `paginate_with_total`

三者的查询、过滤、分页逻辑结构高度相似，仅实体类型与列名不同，属于典型的"复制粘贴 + 改类型"重复模式。

### 2.4 高频 service 文件（`pub async fn` 密度 Top5）

| 文件 | `pub async fn` 数 | 代码行数 |
|------|------------------:|---------:|
| `budget_management_service.rs` | 31 | 1,328 |
| `fixed_asset_service.rs` | 21 | 1,454 |
| `crm/lead.rs` | 21 | 1,363 |
| `fund_management_service.rs` | 20 | — |
| `bulk_color_approval_service.rs` | 20 | 1,474 |

这些文件函数密度高，内部 CRUD 方法集中，重复模式集中度高。

### 2.5 重复评估结论

**评估等级：中**

理由：
1. CRUD 样板（list/create/update/delete）在 92~140 个 service 文件中重复出现，覆盖面广（22.7%~34.6%），但函数体因实体类型、字段、业务规则不同未达到逐字节重复，属于"结构性重复"而非"逐字复制"。
2. `get_by_id`/`find_by_id` 命名碎片化表明缺乏统一的泛型 trait 或基类抽象，但重复程度尚未达到"高"（未发现可直接抽取为宏或泛型的大段完全相同代码）。
3. `print_service.rs` 单文件 4,995 行、`budget_management_service.rs` 31 个公开异步函数，存在超大文件集中重复风险。

**典型重复示例**：
```
services/currency_service.rs     pub async fn list_currencies -> Entity::find().order_by_asc().all()
services/department_service.rs   pub async fn list           -> Entity::find().filter().paginate()
services/role_service.rs         pub async fn list_roles      -> Entity::find().paginate().paginate_with_total()
```
三者结构同构：`Entity::find() → 可选 filter → 分页/排序 → .all().await`。

## 三、注释完整性扫描

### 3.1 注释覆盖率统计

| 指标 | 数量 |
|------|------|
| 公开函数数（`pub fn` + `pub async fn`，全 src） | 2,104 |
| 文档注释行数（`///`，全 src） | 14,658 |
| 文档注释行数（`///`，services 内） | 6,037 |
| 公开异步函数数（services 内） | 1,686 |

**注释覆盖率（注释行/公开函数）**：
- 全 src：14,658 / 2,104 = **6.97 行/函数**
- services：6,037 / 1,686 = **3.58 行/函数**

> 说明：该比值为"文档注释行数 ÷ 公开函数数"，因一个函数可能有多行 `///` 注释（参数说明、返回值说明），比值 >1 属正常。比值越高说明单函数注释越详尽。

### 3.2 注释覆盖率评估

**评估等级：中等偏上**

- 全 src 比 6.97 表明平均每个公开函数有近 7 行文档注释，覆盖率较好。
- services 层比 3.58 低于全仓均值，说明业务 service 层文档注释密度低于工具/中间件层（后者如 `utils/cache.rs`、`middleware/auth.rs` 注释密集）。
- 未发现大面积"裸函数"（无任何注释），但 services 层存在部分 CRUD 函数仅有简短 `//` 行内注释而无 `///` 文档注释。

### 3.3 待办标记统计

| 标记类型 | 数量 | 说明 |
|---------|-----:|------|
| TODO | 10 | 真实待办（另有 2 处历史说明性注释含"TODO"字样但非待办） |
| FIXME | 0 | 无 |
| HACK | 0 | 无 |
| XXX | 出现在 TTL 配置说明中 | 非待办标记，是 `XXX_CACHE_TTL_SECS` 变量命名 |

**真实待办总数：10**

待办分布（与 `todo-fixme-scan.md` 报告一致，此处不重复明细）：
- `handlers/`：3 处（dashboard、dye_recipe、slow_query）
- `services/`：4 处（stock_alert ×2、supplier_service、data_permission_service）
- `utils/`：3 处（cache、redis_cache 命名含 XXX 但非标记、di_container）

> 注：`utils/redis_cache.rs:188` 与 `services/cache_service.rs:28` 中的 "XXX" 属于 `XXX_CACHE_TTL_SECS` 变量名，非待办标记，已剔除。`purchase_return_service.rs:180/:483` 为历史说明性注释，非真实待办。

## 四、建议

### 4.1 重复率改进（优先级：中）

1. **抽取 CRUD 泛型 trait**：为 list/get/create/update/delete 五类样板定义统一的 `CrudService<T>` trait 或泛型宏，将"Entity::find → filter → paginate → all"骨架抽取为公共方法，预计可消除 services 层 30%+ 的结构性重复。
2. **统一查询命名**：将 `find_by_id`（4 处）统一收敛为 `get_by_id`，消除命名碎片化。
3. **拆分超大文件**：`print_service.rs`（4,995 行）与 `budget_management_service.rs`（31 个 async fn）应按业务子领域拆分，降低单文件重复密度。

### 4.2 注释完整性改进（优先级：低-中）

1. **补齐 services 层 `///` 文档注释**：services 层 3.58 行/函数低于全仓均值，建议对 CRUD 函数补充 `///` 文档注释（参数、返回值、错误码），将 services 层比值提升至 5.0+。
2. **保持现状**：未发现 FIXME/HACK，代码异味标记控制良好，继续维持。

### 4.3 待办标记处理（优先级：中）

1. 10 个真实 TODO 中，6 个无 issue 编号或具体计划，建议逐一登记 issue 并指定负责人与排期。
2. `stock_alert.rs` 两处 `TODO(tech-debt)` 已有计划，可纳入下个批次处理。

## 五、返回结论

- **代码重复评估等级：中** —— CRUD 样板在 92~140 个 service 文件中结构性重复，命名碎片化（`get_by_id`/`find_by_id` 并存），但未发现大段逐字重复，属"结构性重复"，建议抽取泛型 trait 收敛。
- **注释覆盖率：3.58 行/函数（services）/ 6.97 行/函数（全 src）** —— 中等偏上，services 层低于均值，建议补齐 CRUD 文档注释。
- **待办标记数：10 个真实 TODO，0 个 FIXME/HACK** —— 控制良好，6 个无 issue 的 TODO 建议登记追踪。
