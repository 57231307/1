# 全面功能修复计划

## 问题分类

### 1. 缺失的后端路由和 Handler

#### 1.1 打印功能（严重）
- [ ] 销售订单打印：`/sales/orders/:id/print`
- [ ] 采购订单打印：`/purchase/orders/:id/print`
- [ ] 入库单打印：`/purchase/receipts/:id/print`
- [ ] 出库单打印：`/inventory/outbound/:id/print`
- [ ] 调拨单打印：`/inventory/transfers/:id/print`
- [ ] 盘点单打印：`/inventory/counts/:id/print`

#### 1.2 销售模块
- [ ] 销售合同列表/CRUD
- [ ] 销售退货详细接口

#### 1.3 采购模块
- [ ] 采购订单列表（返回空）
- [ ] 采购合同列表/CRUD
- [ ] 采购入库列表（返回空）

#### 1.4 财务模块
- [ ] 会计科目列表（返回空）
- [ ] 凭证列表（返回空）

#### 1.5 库存模块
- [ ] 库存查询（返回空）

### 2. 前端功能缺失

#### 2.1 按钮未实现
- [ ] 所有打印按钮（后端接口缺失）
- [ ] 导出功能（部分缺失）
- [ ] 批量操作功能

#### 2.2 表单验证
- [ ] 保存时字段验证
- [ ] 必填项检查

### 3. API 响应格式问题

#### 3.1 响应格式不统一
- [ ] 部分 API 返回 `{data: [...]}` 
- [ ] 部分 API 返回 `{data: {data: [...]}}`
- [ ] 部分 API 返回空响应

## 修复方案

### 方案 1：批量添加缺失的路由和 Handler

创建通用 CRUD handler，支持：
- list (列表查询)
- get (详情)
- create (创建)
- update (更新)
- delete (删除)
- print (打印)
- export (导出)

### 方案 2：统一 API 响应格式

确保所有 API 返回统一格式：
```json
{
  "code": 200,
  "data": {
    "items": [...],
    "total": 100
  },
  "message": "success"
}
```

### 方案 3：前端统一错误处理

- 统一 API 请求拦截器
- 统一错误提示
- 统一 loading 状态

## 实施步骤

1. **第一阶段：修复后端路由** (优先级：高)
   - 添加所有缺失的路由
   - 实现通用 CRUD handler
   - 实现打印功能

2. **第二阶段：修复 API 响应** (优先级：高)
   - 统一所有 API 返回格式
   - 修复空响应问题

3. **第三阶段：前端功能完善** (优先级：中)
   - 实现打印按钮功能
   - 实现导出功能
   - 添加表单验证

4. **第四阶段：全面测试** (优先级：中)
   - 测试所有 CRUD 操作
   - 测试所有按钮功能
   - 回归测试

## 文件清单

### 后端需要修改的文件
- `backend/src/routes/mod.rs` - 添加缺失路由
- `backend/src/handlers/` - 创建缺失的 handlers
- `backend/src/services/print_service.rs` - 新建打印服务

### 前端需要修改的文件
- `frontend/src/api/*.ts` - 统一 API 调用
- `frontend/src/views/**/*.vue` - 修复按钮事件
- `frontend/src/utils/request.ts` - 统一错误处理

## 预计工作量

- 后端路由修复：4 小时
- API 响应统一：2 小时
- 前端功能修复：6 小时
- 测试验证：2 小时

**总计：约 14 小时**
