# 前端虚拟列表改造方案

> 制定日期：2026-06-05
> 适用版本：Element Plus 2.6+（项目已使用）

## 一、背景

当前 47 个 .vue 文件超过 500 行，其中多个销售/采购/库存模块使用 `el-table` 展示大列表（行数 1000+）。
`el-table` 是普通 DOM 表格，渲染 1000+ 行会显著影响首屏与滚动性能。

## 二、方案

**采用 Element Plus 内置的 `el-table-v2`**（自 2.0 起提供），无需引入第三方依赖。

### 核心差异

| 特性 | el-table（当前） | el-table-v2（推荐） |
|---|---|---|
| 渲染方式 | 全部 DOM 节点 | 虚拟滚动（仅渲染可见行） |
| 1000 行性能 | 显著卡顿 | 流畅 |
| API 兼容性 | 简单 | 需用 `Column` 数组配置 |
| 排序/筛选 | 内置 | 需手动实现 |

### 改造步骤

#### 步骤 1：识别目标组件

```bash
# 找出所有使用 el-table 且数据可能 > 100 行的文件
cd frontend/src/views
grep -lE "el-table.*:data" $(find . -name "*.vue") | xargs -I {} grep -lE "v-for|el-table-column" {} 2>/dev/null
```

#### 步骤 2：替换 el-table → el-table-v2

**改造前**：
```vue
<el-table :data="list" stripe>
  <el-table-column prop="id" label="ID" width="80" />
  <el-table-column prop="name" label="名称" />
  <el-table-column label="操作" width="150">
    <template #default="{ row }">
      <el-button @click="handleEdit(row)">编辑</el-button>
    </template>
  </el-table-column>
</el-table>
```

**改造后**：
```vue
<el-table-v2
  :columns="columns"
  :data="list"
  :width="tableWidth"
  :height="600"
  fixed
/>

<script setup>
import { ref, h } from 'vue'
import { ElButton } from 'element-plus'

const columns = [
  { key: 'id', title: 'ID', dataKey: 'id', width: 80, fixed: true },
  { key: 'name', title: '名称', dataKey: 'name', width: 200 },
  {
    key: 'actions',
    title: '操作',
    width: 150,
    cellRenderer: ({ rowData }) =>
      h(ElButton, { link: true, onClick: () => handleEdit(rowData) }, () => '编辑'),
  },
]
</script>
```

#### 步骤 3：配合分页

`el-table-v2` 不再内置分页 UI，需显式使用 `el-pagination`：

```vue
<div class="table-wrapper">
  <el-table-v2 :columns="columns" :data="pagedList" :width="..." :height="540" />
  <el-pagination
    v-model:current-page="query.page"
    v-model:page-size="query.page_size"
    :total="total"
    layout="total, sizes, prev, pager, next, jumper"
    @current-change="fetchList"
  />
</div>
```

## 三、目标改造文件（按优先级）

| 优先级 | 文件路径 | 行数 | 业务 | 预期收益 |
|---|---|---|---|---|
| P1 | views/sales/index.vue | 1102 | 销售订单列表 | 流畅渲染 5000+ 行 |
| P1 | views/purchase/index.vue | 954 | 采购订单列表 | 同上 |
| P1 | views/inventory/index.vue | 915 | 库存列表 | 同上 |
| P2 | views/product/index.vue | 841 | 产品列表 | 同上 |
| P2 | views/ar/index.vue | 960 | 应收列表 | 同上 |
| P2 | views/ap/index.vue | 1027 | 应付列表 | 同上 |
| P3 | views/sales-ext/index.vue | 1148 | 销售扩展 | 同上 |
| P3 | views/purchase-ext/index.vue | 1147 | 采购扩展 | 同上 |
| P3 | views/voucher/index.vue | 842 | 凭证列表 | 同上 |
| P3 | views/quality/index.vue | 800 | 质量检查 | 同上 |

## 四、API 差异与注意事项

1. **数据列定义**：el-table-v2 用 JS 数组 + 渲染函数，而非模板声明
2. **滚动事件**：使用 `@scroll` 事件而非 `@row-click`
3. **展开行**：需用 `expandedRowKeys` + 自定义 `rowRenderer`
4. **表头合并**：使用 `headerCellRenderer` 自定义
5. **斑马纹**：通过 `rowClass` 实现（`({ rowIndex }) => rowIndex % 2 ? 'stripe' : ''`）

## 五、渐进式迁移策略

**不建议一次性全量替换**。建议：
1. 优先改造"明显卡顿"的列表页（用户反馈驱动）
2. 每个文件单独 PR，确保功能对等
3. 短期可保留 el-table + 引入分页（page_size=100）作为过渡

## 六、验证清单

- [ ] 大数据量（1 万行）下滚动流畅（FPS > 30）
- [ ] 排序、筛选功能正常
- [ ] 行内操作按钮可点击
- [ ] 移动端响应式正常
- [ ] 暗色主题下视觉一致
- [ ] `npm run build` 通过，主包未增加 > 50KB
