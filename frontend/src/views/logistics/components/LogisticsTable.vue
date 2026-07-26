<!--
  LogisticsTable.vue - 物流管理运单表
  拆分自 logistics/index.vue（P14 批 2 I-3 第 4 批）
  批次 287：page/pageSize props + v-model 绑定分页
  迁移至 V2Table 虚拟滚动表格（行为完全保持一致，仅结构重构）
-->
<template>
  <el-card class="table-card">
    <V2Table
      :columns="columns"
      :data="data"
      :loading="loading"
      :page="page"
      :page-size="pageSize"
      :page-sizes="[10, 20, 50, 100]"
      :total="total"
      :height="600"
      @page-change="(v: number) => emit('update:page', v)"
      @size-change="(v: number) => emit('update:page-size', v)"
    />
  </el-card>
</template>

<script setup lang="ts">
import { h, computed } from 'vue'
import { ElButton, ElTag } from 'element-plus'
import { useI18n } from 'vue-i18n'
import V2Table from '@/components/V2Table/index.vue'
import type { ColumnDef } from '@/components/V2Table/types'
import type { LogisticsWaybill } from '@/api/logistics'
import { getStatusType, formatFreight } from '../composables/lgsFmts'

const { t } = useI18n({ useScope: 'global' })

/**
 * 物流运单列表组件（批次 287：page/pageSize props + v-model 绑定分页）
 * 迁移至 V2Table 虚拟滚动表格，保留原 props/emits 签名以维持父组件兼容
 */
defineProps<{
  // 列表数据
  data: LogisticsWaybill[]
  // 加载状态
  loading: boolean
  // 总数
  total: number
  // 当前页
  page: number
  // 每页条数
  pageSize: number
}>()

const emit = defineEmits<{
  view: [row: LogisticsWaybill]
  edit: [row: LogisticsWaybill]
  ship: [row: LogisticsWaybill]
  'update-status': [row: LogisticsWaybill]
  delete: [row: LogisticsWaybill]
  'update:page': [v: number]
  'update:page-size': [v: number]
}>()

/** 状态文本：优先 i18n，未知状态回退到原始 status 字符串 */
const statusText = (status: string): string => {
  const key = `logistics.common.status.${status}`
  const translated = t(key)
  return translated === key ? status : translated
}

/** 列定义：保持与原 el-table 列完全一致；computed 确保 locale 切换时表头响应式更新 */
const columns = computed<ColumnDef<LogisticsWaybill>[]>(() => [
  { key: 'waybill_no', title: t('logistics.table.column.waybillNo'), width: 140 },
  { key: 'order_no', title: t('logistics.table.column.relatedOrder'), width: 140 },
  { key: 'logistics_company', title: t('logistics.table.column.logisticsCompany'), width: 120 },
  { key: 'tracking_number', title: t('logistics.table.column.trackingNumber'), width: 150 },
  { key: 'driver_name', title: t('logistics.table.column.driverName'), width: 100 },
  { key: 'driver_phone', title: t('logistics.table.column.driverPhone'), width: 120 },
  {
    // 运费列：格式化为 ¥ 前缀字符串
    key: 'freight_fee',
    title: t('logistics.table.column.freight'),
    width: 100,
    formatter: (row) => formatFreight(row.freight_fee),
  },
  { key: 'expected_arrival', title: t('logistics.table.column.expectedArrival'), width: 120 },
  {
    // 状态列：渲染 el-tag，颜色与文本由状态映射决定
    key: 'status',
    title: t('logistics.table.column.status'),
    width: 100,
    align: 'center',
    renderCell: (row) =>
      h(
        ElTag,
        { type: getStatusType(row.status) },
        { default: () => statusText(row.status) }
      ),
  },
  {
    // 操作列：根据 row.status 条件渲染按钮（查看 / 编辑 / 发货 / 更新状态 / 删除）
    key: '__actions__',
    title: t('logistics.table.column.action'),
    width: 250,
    fixed: 'right',
    renderCell: (row) => {
      const buttons = [
        h(
          ElButton,
          { size: 'small', onClick: () => emit('view', row) },
          { default: () => t('logistics.table.action.view') }
        ),
      ]
      // 待发货状态：显示编辑 / 发货 / 删除
      if (row.status === 'pending') {
        buttons.push(
          h(
            ElButton,
            { size: 'small', type: 'primary', onClick: () => emit('edit', row) },
            { default: () => t('logistics.table.action.edit') }
          ),
          h(
            ElButton,
            { size: 'small', type: 'success', onClick: () => emit('ship', row) },
            { default: () => t('logistics.table.action.ship') }
          ),
          h(
            ElButton,
            { size: 'small', type: 'danger', onClick: () => emit('delete', row) },
            { default: () => t('logistics.table.action.delete') }
          )
        )
      }
      // 已发货 / 运输中：显示更新状态
      if (row.status === 'shipped' || row.status === 'in_transit') {
        buttons.push(
          h(
            ElButton,
            { size: 'small', type: 'warning', onClick: () => emit('update-status', row) },
            { default: () => t('logistics.table.action.updateStatus') }
          )
        )
      }
      return h('div', { class: 'action-cell' }, buttons)
    },
  },
])
</script>

<style scoped>
.table-card {
  margin-bottom: 20px;
}
.action-cell {
  display: flex;
  gap: 4px;
  align-items: center;
}
</style>
