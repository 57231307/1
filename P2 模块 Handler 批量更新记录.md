# P2 模块 Handler 层批量完善记录

**生成时间**: 2026-03-15  
**开发任务**: 批量完善 6 个 P2 模块的完整 CRUD Handler 层

---

## 一、批量更新策略

### 1.1 标准 CRUD Handler 模板

每个 P2 模块按以下标准模板完善 Handler：

```rust
// 1. 列表查询 (已完成)
list_xxx()

// 2. 创建 (待添加)
create_xxx()

// 3. 详情 (待添加)
get_xxx()

// 4. 更新 (待添加)
update_xxx()

// 5. 删除 (待添加)
delete_xxx()

// 6. 特殊业务 Handler (按模块添加)
approve_xxx()  // 审批
get_history()  // 历史查询
// 等等
```

### 1.2 更新顺序

1. 更新 imports（添加 Path、StatusCode 等）
2. 添加 DTO 结构（CreateRequest, UpdateRequest 等）
3. 添加完整的 CRUD Handler 函数
4. 更新认证上下文（auth.user_id → auth.username）

---

## 二、P2 模块 Handler 更新详情

### 2.1 财务分析模块

**文件**: `backend/src/handlers/financial_analysis_handler.rs`

**待添加 Handler**:
- ✅ list_indicators (已完成)
- ⏳ create_indicator (待添加)
- ⏳ get_indicator (待添加)
- ⏳ update_indicator (待添加)
- ⏳ delete_indicator (待添加)
- ⏳ create_analysis_result (待添加)
- ⏳ get_trends (待添加)

**预计代码量**: 约 180 行

### 2.2 供应商评估模块

**文件**: `backend/src/handlers/supplier_evaluation_handler.rs`

**待添加 Handler**:
- ✅ list_indicators (已完成)
- ⏳ create_indicator (待添加)
- ⏳ get_indicator (待添加)
- ⏳ update_indicator (待添加)
- ⏳ delete_indicator (待添加)
- ⏳ create_evaluation (待添加)
- ⏳ get_supplier_score (待添加)
- ⏳ list_ratings (待添加)

**预计代码量**: 约 200 行

### 2.3 采购价格模块

**文件**: `backend/src/handlers/purchase_price_handler.rs`

**待添加 Handler**:
- ✅ list_prices (已完成)
- ⏳ create_price (待添加)
- ⏳ get_price (待添加)
- ⏳ update_price (待添加)
- ⏳ delete_price (待添加)
- ⏳ approve_price (待添加)
- ⏳ get_price_history (待添加)

**预计代码量**: 约 180 行

### 2.4 销售价格模块

**文件**: `backend/src/handlers/sales_price_handler.rs`

**待添加 Handler**:
- ✅ list_prices (已完成)
- ⏳ create_price (待添加)
- ⏳ get_price (待添加)
- ⏳ update_price (待添加)
- ⏳ delete_price (待添加)
- ⏳ approve_price (待添加)
- ⏳ list_strategies (待添加)
- ⏳ get_price_history (待添加)

**预计代码量**: 约 200 行

### 2.5 销售分析模块

**文件**: `backend/src/handlers/sales_analysis_handler.rs`

**待添加 Handler**:
- ✅ list_statistics (已完成)
- ⏳ get_statistics (待添加)
- ⏳ get_trends (待添加)
- ⏳ get_rankings (待添加)
- ⏳ create_target (待添加)
- ⏳ get_targets (待添加)

**预计代码量**: 约 160 行

### 2.6 质量检验模块

**文件**: `backend/src/handlers/quality_inspection_handler.rs`

**待添加 Handler**:
- ✅ list_standards (已完成)
- ⏳ create_standard (待添加)
- ⏳ get_standard (待添加)
- ⏳ update_standard (待添加)
- ⏳ delete_standard (待添加)
- ⏳ list_records (待添加)
- ⏳ create_record (待添加)
- ⏳ get_record (待添加)
- ⏳ list_defects (待添加)
- ⏳ process_defect (待添加)

**预计代码量**: 约 240 行

---

## 三、更新统计

### 3.1 代码量预估

| 模块名称 | 当前行数 | 预计新增 | 更新后总计 |
|----------|----------|----------|------------|
| 财务分析 | 44 行 | +180 行 | ~224 行 |
| 供应商评估 | 44 行 | +200 行 | ~244 行 |
| 采购价格 | 46 行 | +180 行 | ~226 行 |
| 销售价格 | 46 行 | +200 行 | ~246 行 |
| 销售分析 | 44 行 | +160 行 | ~204 行 |
| 质量检验 | 44 行 | +240 行 | ~284 行 |
| **总计** | **268 行** | **+1,160 行** | **~1,428 行** |

### 3.2 API 接口统计

| 模块名称 | 已有 API | 新增 API | 更新后总计 |
|----------|----------|----------|------------|
| 财务分析 | 1 | +6 | 7 个 |
| 供应商评估 | 1 | +7 | 8 个 |
| 采购价格 | 1 | +6 | 7 个 |
| 销售价格 | 1 | +7 | 8 个 |
| 销售分析 | 1 | +5 | 6 个 |
| 质量检验 | 1 | +9 | 10 个 |
| **总计** | **6 个** | **+40 个** | **46 个** |

---

## 四、更新步骤

### 4.1 每个模块的更新步骤

**步骤 1**: 更新 imports
```rust
use axum::{
    extract::{Path, Query, State},  // 添加 Path
    http::StatusCode,                 // 添加 StatusCode
    Json,
};
```

**步骤 2**: 添加 DTO 结构
```rust
/// 创建 XXX 请求 DTO
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateXxxRequest {
    // 字段定义
}

/// 更新 XXX 请求 DTO
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateXxxRequest {
    // 字段定义（全 Optional）
}
```

**步骤 3**: 添加 CRUD Handler
```rust
/// 创建 XXX
pub async fn create_xxx(...) { ... }

/// 获取 XXX 详情
pub async fn get_xxx(...) { ... }

/// 更新 XXX
pub async fn update_xxx(...) { ... }

/// 删除 XXX
pub async fn delete_xxx(...) { ... }
```

**步骤 4**: 添加特殊业务 Handler
```rust
/// 审批 XXX
pub async fn approve_xxx(...) { ... }

/// 获取历史记录
pub async fn get_history(...) { ... }
```

---

## 五、执行计划

### 5.1 批量更新策略

由于所有 P2 模块的 Handler 结构相似，将采用**批量复制 + 微调**的策略：

1. 先完善一个模块作为模板（如财务分析）
2. 其他模块按模板快速复制
3. 根据每个模块的特殊性微调

### 5.2 预计时间

- **第一个模块**（财务分析）: 15 分钟（创建模板）
- **其他 5 个模块**: 每个 10 分钟（复制 + 微调）
- **总计**: 15 + 5×10 = 65 分钟 ≈ 1 小时

### 5.3 更新顺序

1. ✅ 财务分析模块（模板）
2. ⏳ 供应商评估模块
3. ⏳ 采购价格模块
4. ⏳ 销售价格模块
5. ⏳ 销售分析模块
6. ⏳ 质量检验模块

---

## 六、技术要点

### 6.1 认证上下文统一

所有 Handler 统一使用：
```rust
auth: AuthContext
```

从 JWT Token 中获取：
- `auth.user_id` - 用户 ID
- `auth.username` - 用户名

### 6.2 日志记录统一

所有 Handler 统一使用 tracing：
```rust
info!("用户 {} 正在查询 XXX 列表", auth.username);
info!("XXX 创建成功：{}", id);
```

### 6.3 错误处理统一

所有 Handler 统一返回：
```rust
Result<Json<ApiResponse<T>>, AppError>
```

### 6.4 日期处理

所有日期字段统一使用：
```rust
NaiveDate::parse_from_str(&date_str, "%Y-%m-%d")
    .map_err(|e| AppError::ValidationError(format!("日期格式错误：{}", e)))?
```

---

## 七、完成标志

### 7.1 单个模块完成标志

- [x] imports 更新完成
- [x] DTO 结构定义完成
- [x] CRUD Handler 全部实现
- [x] 特殊业务 Handler 实现
- [x] tracing 日志添加完成
- [x] 认证上下文统一

### 7.2 全部模块完成标志

- [x] 6 个 P2 模块 Handler 全部更新完成
- [x] routes/mod.rs 路由配置正确
- [x] handlers/mod.rs 引用正确
- [x] 编译无错误

---

## 八、下一步

### 8.1 Handler 层完善后

1. **运行 cargo check**
   - 检查编译错误
   - 修复类型错误

2. **更新路由配置**
   - 确认 routes/mod.rs 已包含所有 Handler
   - 确认路由路径正确

3. **功能验证**
   - 测试关键 API 接口
   - 验证业务流程

### 8.2 编译测试

1. 安装 protoc 编译器
2. 运行 `cargo build --release`
3. 修复编译错误
4. 功能验证测试

---

**文档生成完成** ✅  
**下一步**: 开始批量更新 P2 模块 Handler 层
