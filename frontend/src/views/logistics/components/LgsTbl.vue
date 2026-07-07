<!--
  LgsTbl.vue - 物流管理运单表
  拆分自 logistics/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card class="table-card">
    <el-table v-loading="loading" :data="data" border stripe>
      <el-table-column prop="waybill_no" label="运单号" min-width="140" />
      <el-table-column prop="order_no" label="关联订单" min-width="140" />
      <el-table-column prop="logistics_company" label="物流公司" min-width="120" />
      <el-table-column prop="tracking_number" label="快递单号" min-width="150" />
      <el-table-column prop="driver_name" label="司机姓名" min-width="100" />
      <el-table-column prop="driver_phone" label="司机电话" min-width="120" />
      <el-table-column prop="freight_fee" label="运费" min-width="100">
        <template #default="{ row }">
          <span>¥{{ row.freight_fee || 0 }}</span>
        </template>
      </el-table-column>
      <el-table-column prop="expected_arrival" label="预计到达" min-width="120" />
      <el-table-column prop="status" label="状态" width="100" align="center">
        <template #default="{ row }">
          <el-tag :type="(getStatusTypeFmt(row.status) as TagType)">
            {{ getStatusTextFmt(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="250" fixed="right">
        <template #default="{ row }">
          <el-button size="small" @click="emit('view', row as LogisticsWaybill)">查看</el-button>
          <el-button
            v-if="row.status === 'pending'"
            size="small"
            type="primary"
            @click="emit('edit', row as LogisticsWaybill)"
          >
            编辑
          </el-button>
          <el-button
            v-if="row.status === 'pending'"
            size="small"
            type="success"
            @click="emit('ship', row as LogisticsWaybill)"
          >
            发货
          </el-button>
          <el-button
            v-if="row.status === 'shipped' || row.status === 'in_transit'"
            size="small"
            type="warning"
            @click="emit('update-status', row as LogisticsWaybill)"
          >
            更新状态
          </el-button>
          <el-button
            v-if="row.status === 'pending'"
            size="small"
            type="danger"
            @click="emit('delete', row as LogisticsWaybill)"
          >
            删除
          </el-button>
        </template>
      </el-table-column>
    </el-table>

    <el-pagination
      v-model:current-page="queryParams.page"
      v-model:page-size="queryParams.page_size"
      :total="total"
      :page-sizes="[10, 20, 50, 100]"
      layout="total, sizes, prev, pager, next, jumper"
      @size-change="emit('size-change')"
      @current-change="emit('current-change')"
    />
  </el-card>
</template>

<script setup lang="ts">
import type { LogisticsWaybill } from '@/api/logistics'
import { getStatusType, getStatusText } from '../composables/lgsFmts'

// v11 批次 178 P2-1 修复：el-tag type 字面量类型
type TagType = 'success' | 'warning' | 'info' | 'primary' | 'danger'

// 查询参数类型
interface QryParams {
  keyword: string
  logistics_company: string
  status: string
  page: number
  page_size: number
}

/**
 * 物流运单列表组件
 */
defineProps<{
  // 列表数据
  data: LogisticsWaybill[]
  // 加载状态
  loading: boolean
  // 总数
  total: number
  // 查询参数（用于分页）
  queryParams: QryParams
}>()

const emit = defineEmits<{
  view: [row: LogisticsWaybill]
  edit: [row: LogisticsWaybill]
  ship: [row: LogisticsWaybill]
  'update-status': [row: LogisticsWaybill]
  delete: [row: LogisticsWaybill]
  'size-change': []
  'current-change': []
}>()

// 透传格式化函数
const getStatusTypeFmt = getStatusType
const getStatusTextFmt = getStatusText
</script>

<style scoped>
.table-card {
  margin-bottom: 20px;
}
.el-pagination {
  margin-top: 20px;
  justify-content: flex-end;
}
</style>
