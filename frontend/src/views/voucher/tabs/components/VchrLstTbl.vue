<!--
  VchrLstTbl.vue - 凭证列表表格
  拆分自 voucher/tabs/VoucherListTab.vue（P14 批 2 I-3 第 1 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <ElTable
    :data="tableData"
    :loading="loading"
    border
    fit
    highlight-current-row
    style="width: 100%"
  >
    <ElTableColumn prop="voucher_no" label="凭证号" width="120" />
    <ElTableColumn prop="voucher_date" label="凭证日期" width="120" />
    <ElTableColumn prop="type" label="凭证类型" width="100">
      <template #default="scope">
        {{ getTypeLabel(scope.row.type) }}
      </template>
    </ElTableColumn>
    <ElTableColumn prop="total_debit" label="借方金额" width="120" align="right">
      <template #default="scope">{{ formatAmount(scope.row.total_debit) }}</template>
    </ElTableColumn>
    <ElTableColumn prop="total_credit" label="贷方金额" width="120" align="right">
      <template #default="scope">{{ formatAmount(scope.row.total_credit) }}</template>
    </ElTableColumn>
    <ElTableColumn prop="status" label="状态" width="100">
      <template #default="scope">
        <span :class="['status-tag', getStatusClass(scope.row.status)]">
          {{ getStatusLabel(scope.row.status) }}
        </span>
      </template>
    </ElTableColumn>
    <ElTableColumn prop="created_by_name" label="制单人" width="100" />
    <ElTableColumn prop="approved_by_name" label="审核人" width="100" />
    <ElTableColumn prop="posted_by_name" label="记账人" width="100" />
    <ElTableColumn label="操作" width="300" align="center">
      <template #default="scope">
        <ElButton size="small" @click="emit('view', scope.row as VoucherEntity)">
          <View />
        </ElButton>
        <ElButton
          v-if="scope.row.status === 'draft'"
          size="small"
          type="primary"
          @click="emit('edit', scope.row as VoucherEntity)"
        >
          <Edit />
        </ElButton>
        <ElButton
          v-if="scope.row.status === 'draft'"
          size="small"
          type="warning"
          @click="emit('approve', scope.row as VoucherEntity)"
        >
          <Check /> 审核
        </ElButton>
        <ElButton
          v-if="scope.row.status === 'approved'"
          size="small"
          type="success"
          @click="emit('post', scope.row as VoucherEntity)"
        >
          <Check /> 记账
        </ElButton>
        <ElButton
          v-if="scope.row.status === 'posted'"
          size="small"
          type="info"
          @click="emit('unpost', scope.row as VoucherEntity)"
        >
          <Refresh /> 反记账
        </ElButton>
        <ElButton
          v-if="scope.row.status !== 'posted'"
          size="small"
          type="danger"
          @click="emit('delete', scope.row as VoucherEntity)"
        >
          <Delete />
        </ElButton>
      </template>
    </ElTableColumn>
  </ElTable>

  <div class="pagination-wrapper">
    <ElPagination
      :current-page="pagination.page"
      :page-size="pagination.pageSize"
      :page-sizes="[10, 20, 50, 100]"
      :total="total"
      layout="total, sizes, prev, pager, next, jumper"
      @size-change="emit('page-size-change', $event as number)"
      @current-change="emit('page-change', $event as number)"
    />
  </div>
</template>

<script setup lang="ts">
import { Edit, Delete, View, Refresh, Check } from '@element-plus/icons-vue'
import type { VoucherEntity } from '@/api/voucher'
import { getStatusLabel, getStatusClass, getTypeLabel, formatAmount } from '../composables/vchrLstFmts'

interface VoucherPagination {
  page: number
  pageSize: number
}

/**
 * 凭证列表表格组件
 * 仅做展示，行内操作通过 emit 通知父组件
 */
const props = defineProps<{
  // 列表数据
  tableData: VoucherEntity[]
  // 加载中
  loading: boolean
  // 总数
  total: number
  // 分页
  pagination: VoucherPagination
}>()

const emit = defineEmits<{
  // 查看
  view: [row: VoucherEntity]
  // 编辑
  edit: [row: VoucherEntity]
  // 审核
  approve: [row: VoucherEntity]
  // 记账
  post: [row: VoucherEntity]
  // 反记账
  unpost: [row: VoucherEntity]
  // 删除
  delete: [row: VoucherEntity]
  // 翻页
  'page-change': [page: number]
  // 每页大小
  'page-size-change': [size: number]
}>()

void props
</script>
