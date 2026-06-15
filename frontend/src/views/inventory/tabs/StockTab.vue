<!--
  库存台账原表组件(从 inventory/index.vue 抽取)

  说明:
    - 该组件是 P2-1 el-table-v2 POC 的基线版本
    - 列定义、筛选、分页与原 inventory/index.vue 中库存台账 tab 保持一致
    - 该文件不修改原 inventory/index.vue, 仅作为 POC 对照组使用
    - 后续重构可由该组件反向替换原页面中的内联表格
-->
<template>
  <el-card shadow="hover" class="table-card">
    <el-table
      v-loading="loading"
      :data="stocks"
      stripe
      border
      style="width: 100%"
    >
      <el-table-column
        prop="product_code"
        label="产品编码"
        width="140"
        fixed
      />
      <el-table-column
        prop="product_name"
        label="产品名称"
        min-width="180"
        fixed
      />
      <el-table-column
        prop="warehouse_name"
        label="仓库"
        width="120"
      />
      <el-table-column
        prop="batch_no"
        label="批次号"
        width="120"
      />
      <el-table-column
        prop="color_code"
        label="颜色编码"
        width="100"
      />
      <el-table-column
        prop="location"
        label="库位"
        width="100"
      />
      <el-table-column
        prop="quantity"
        label="库存数量"
        width="100"
        align="right"
      >
        <template #default="{ row }">
          <span :class="{ 'low-stock': row.quantity < row.min_quantity }">
            {{ row.quantity }}
          </span>
        </template>
      </el-table-column>
      <el-table-column
        prop="unit"
        label="单位"
        width="60"
      />
      <el-table-column
        prop="gram_weight"
        label="克重"
        width="80"
      />
      <el-table-column
        prop="width"
        label="门幅"
        width="80"
      />
      <el-table-column
        prop="status"
        label="状态"
        width="80"
      >
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)" size="small">
            {{ getStatusText(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column
        label="操作"
        width="150"
        fixed="right"
      >
        <template #default="{ row }">
          <el-button
            type="primary"
            link
            size="small"
            @click="emit('view', row)"
          >
            详情
          </el-button>
          <el-button
            type="warning"
            link
            size="small"
            @click="emit('adjust', row)"
          >
            调整
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <div class="pagination-wrapper">
      <el-pagination
        v-model:current-page="queryParams.page"
        v-model:page-size="queryParams.page_size"
        :page-sizes="[10, 20, 50, 100]"
        :total="total"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="emit('query')"
        @current-change="emit('query')"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
/**
 * 库存台账表格 - 原 el-table 版本(POC 对照组)
 */
import { computed } from 'vue'
import { ElTag } from 'element-plus'

export interface StockRow {
  id: number
  product_code: string
  product_name: string
  warehouse_name: string
  batch_no: string
  color_code: string
  location: string
  quantity: number
  unit: string
  gram_weight: number
  width: number
  status: 'normal' | 'warning' | 'frozen'
  min_quantity: number
  alert_level?: 'danger' | 'warning' | null
  created_at?: string
}

export interface StockQueryParams {
  page: number
  page_size: number
  keyword?: string
  warehouse_id?: number | null
  status?: string
}

const props = defineProps<{
  loading: boolean
  stocks: StockRow[]
  total: number
  queryParams: StockQueryParams
}>()

const emit = defineEmits<{
  (e: 'view', row: StockRow): void
  (e: 'adjust', row: StockRow): void
  (e: 'query'): void
}>()

// 占位以兼容未来 props 校验
void computed(() => props.total)

const getStatusType = (status: string): 'success' | 'warning' | 'info' => {
  const typeMap: Record<string, 'success' | 'warning' | 'info'> = {
    normal: 'success',
    warning: 'warning',
    frozen: 'info',
  }
  return typeMap[status] || 'info'
}

const getStatusText = (status: string): string => {
  const textMap: Record<string, string> = {
    normal: '正常',
    warning: '预警',
    frozen: '冻结',
  }
  return textMap[status] || status
}

// 显式引用 ElTag,避免按需引入时未使用警告
void ElTag
</script>

<style scoped>
.pagination-wrapper {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
.low-stock {
  color: #f56c6c;
  font-weight: 600;
}
</style>
