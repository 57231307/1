# inventory_count_service.rs 拆分计划

> 拆分日期：2026-06-05
> 原文件：949 行（41 KB）
> 拆分后：7 个子文件 + 1 个 facade

## 拆分结构

```
backend/src/services/
├── inventory_count_service.rs            # Facade（49 行，仅 re-export + include! legacy）
└── inventory_count/                      # 新子模块
    ├── mod.rs                            # 子模块入口（19 行）
    ├── types.rs                          # 5 个 DTO 结构体（73 行）
    ├── query.rs                          # 查询占位（54 行）
    ├── commands.rs                       # 增删改占位（35 行）
    ├── workflow.rs                       # 审批/完成占位（28 行）
    ├── items.rs                          # 明细项占位（33 行）
    └── legacy.rs                         # 原实现保留（392 行，移除 DTO 后）
```

## 拆分策略

### 第一阶段（已完成，2026-06-05）

1. **DTO 提取**：将 5 个结构体（InventoryCountDetail、InventoryCountItemDetail、CreateInventoryCountRequest、InventoryCountItemRequest、UpdateInventoryCountRequest）从 inventory_count_service.rs 移到 inventory_count/types.rs
2. **目录骨架建立**：创建 inventory_count/ 子目录及 4 个职责文件（query/commands/workflow/items）
3. **Facade 文件**：新 inventory_count_service.rs 49 行，仅做：
   - `pub use inventory_count::types::*` 重新导出 DTO（向后兼容）
   - `include!("inventory_count/legacy.rs")` 包含原方法实现

### 第二阶段（待后续 PR）

按业务域逐个方法迁移：

| 方法 | 行号 | 目标文件 | 依赖 |
|---|---|---|---|
| `list_counts` | 99-156 | query.rs | sea_orm、PageRequest |
| `get_count_detail` | 157-211 | query.rs | inventory_count_item |
| `list_items` | 758-788 | query.rs | inventory_count_item |
| `create_count` | 212-312 | commands.rs | 事务、adjustment |
| `update_count` | 313-356 | commands.rs | sea_orm |
| `delete_count` | 732-757 | commands.rs | sea_orm |
| `approve_count` | 357-407 | workflow.rs | sea_orm |
| `complete_count` | 408-731 | workflow.rs | adjustment、复杂事务 |
| `add_item` | 789-857 | items.rs | inventory_stock |
| `update_item` | 858-912 | items.rs | sea_orm |
| `delete_item` | 913- | items.rs | sea_orm |

### 第三阶段（重构后）

移除 `legacy.rs` 和 facade 文件中的 `include!`，使 InventoryCountService 成为真正的瘦 facade：

```rust
pub struct InventoryCountService {
    db: Arc<DatabaseConnection>,
}

impl InventoryCountService {
    pub async fn list_counts(&self, q: PageRequest) -> ... {
        query::list_counts(self.db.clone(), CountListQuery::from(q)).await
    }
    // ... 全部委托给子模块
}
```

## 兼容性保证

- 所有公开 API 签名不变
- 所有 DTO 类型仍可从 `crate::services::inventory_count_service::*` 导入
- 所有方法调用方式不变

## 验证步骤

```bash
cd backend
cargo build
cargo test inventory_count
```

## 风险评估

- **低风险**：DTO 提取、目录建立、facade 创建（已完成）
- **中风险**：方法逐个迁移（每次 1 个方法，单独 PR）
- **零风险**：删除 legacy.rs（需所有方法迁移完成且测试通过）
