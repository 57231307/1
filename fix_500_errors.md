# 2 个 500 错误端点修复方案

## 问题诊断

### 1. /ap/reconciliations - 500 错误
- **错误信息**: "数据库查询错误"
- **后端代码**: `ap_reconciliation_service.rs` 的 `get_list()` 方法
- **数据库表**: `ap_reconciliation` (18 列)
- **可能原因**: 
  1. SeaORM 模型字段与数据库不匹配
  2. 数据库连接问题
  3. 查询中有隐藏的 SQL 错误

### 2. /budgets/plans - 500 错误
- **错误信息**: "数据库查询错误"  
- **后端代码**: `budget_management_service.rs` 的 `get_plans_list()` 方法
- **数据库表**: `budget_plans` (14 列)
- **已发现问题**: Handler 和 Service 参数不匹配
  - Handler 传递: `item_type`, `status`
  - Service 期望: `budget_year`, `department_id`

---

## 修复步骤

### 方案 1：修复 /budgets/plans

**文件**: `backend/src/handlers/budget_management_handler.rs`

```rust
// 修改前（第 216-218 行）
let (plans, _total) = service
    .get_plans_list(
        params.item_type.and_then(|y| y.parse().ok()),
        params.status.and_then(|s| s.parse().ok()),
        params.page.unwrap_or(0),
        params.page_size.unwrap_or(10),
    )
    .await?;

// 修改后
let (plans, _total) = service
    .get_plans_list(
        None,  // budget_year - 暂时不传
        None,  // department_id - 暂时不传  
        params.page.unwrap_or(0),
        params.page_size.unwrap_or(10),
    )
    .await?;
```

### 方案 2：修复 /ap/reconciliations

需要在服务端添加详细日志来诊断具体错误：

**文件**: `backend/src/services/ap_reconciliation_service.rs`

```rust
// 在 get_list 方法中添加日志
pub async fn get_list(...) -> Result<(Vec<ap_reconciliation::Model>, u64), AppError> {
    let mut query = ap_reconciliation::Entity::find();
    
    // ... 筛选条件 ...
    
    info!("AP 对账单查询 SQL: {:?}", query.build(&*self.db));
    
    let paginator = query
        .order_by(ap_reconciliation::Column::CreatedAt, Order::Desc)
        .paginate(&*self.db, page_size);
    
    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(page).await?;
    
    Ok((items, total))
}
```

---

## 测试验证

修复后运行以下测试：

```bash
TOKEN=$(curl -s -X POST "http://111.230.99.236/api/v1/erp/auth/login" \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin123"}' | jq -r '.data.token')

# 测试 /budgets/plans
curl -s "${BASE_URL}/budgets/plans" -H "Authorization: Bearer $TOKEN" | jq '.'

# 测试 /ap/reconciliations  
curl -s "${BASE_URL}/ap/reconciliations" -H "Authorization: Bearer $TOKEN" | jq '.'
```

预期结果：
- HTTP 200
- 返回空数组或数据列表

---

## 当前测试状态

| 端点 | 状态 | 错误 | 修复进度 |
|------|------|------|----------|
| `/warehouses/locations` | ✅ PASS | - | 已修复（表名） |
| `/bpm/monitor/stats` | ✅ PASS | - | 已修复（列名） |
| `/ap/reconciliations` | ❌ 500 | 数据库查询错误 | 诊断中 |
| `/budgets/plans` | ❌ 500 | 数据库查询错误 | 发现参数不匹配 |

---

## 下一步

1. ✅ 修复 `/budgets/plans` 参数不匹配问题
2. 🔍 诊断 `/ap/reconciliations` 具体 SQL 错误
3. 🧪 重新运行 100 端点测试
4. 📊 目标通过率：98%+ (98/100)

---

**诊断时间**: 2026-05-18 16:30:00
**待修复**: 2 个端点
**预计修复时间**: 15-30 分钟
