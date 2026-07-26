<!--
  VoucherListTable.vue - 凭证列表表格
  拆分自 voucher/tabs/VoucherListTab.vue（P14 批 2 I-3 第 1 批）
  批次 287：改造为 page/pageSize props + update:page/update:page-size emits
  行为完全保持一致（仅结构重构）
-->
<template>
  <ElTable
    :data="tableData"
    :loading="loading"
    border
    fit
    highlight-current-row
    :aria-label="t('voucher.voucherListTable.ariaLabel')"
    style="width: 100%"
  >
    <ElTableColumn
      prop="voucher_no"
      :label="t('voucher.voucherListTable.columnVoucherNo')"
      width="120"
    />
    <ElTableColumn
      prop="voucher_date"
      :label="t('voucher.voucherListTable.columnVoucherDate')"
      width="120"
    />
    <ElTableColumn prop="type" :label="t('voucher.voucherListTable.columnVoucherType')" width="100">
      <template #default="scope">
        {{ getTypeLabel(scope.row.type) }}
      </template>
    </ElTableColumn>
    <ElTableColumn
      prop="total_debit"
      :label="t('voucher.voucherListTable.columnDebit')"
      width="120"
      align="right"
    >
      <template #default="scope">{{ formatAmount(scope.row.total_debit) }}</template>
    </ElTableColumn>
    <ElTableColumn
      prop="total_credit"
      :label="t('voucher.voucherListTable.columnCredit')"
      width="120"
      align="right"
    >
      <template #default="scope">{{ formatAmount(scope.row.total_credit) }}</template>
    </ElTableColumn>
    <ElTableColumn prop="status" :label="t('voucher.voucherListTable.columnStatus')" width="100">
      <template #default="scope">
        <span :class="['status-tag', getStatusClass(scope.row.status)]">
          {{ getStatusLabel(scope.row.status) }}
        </span>
      </template>
    </ElTableColumn>
    <ElTableColumn
      prop="created_by_name"
      :label="t('voucher.voucherListTable.columnCreatedBy')"
      width="100"
    />
    <ElTableColumn
      prop="approved_by_name"
      :label="t('voucher.voucherListTable.columnApprovedBy')"
      width="100"
    />
    <ElTableColumn
      prop="posted_by_name"
      :label="t('voucher.voucherListTable.columnPostedBy')"
      width="100"
    />
    <ElTableColumn :label="t('voucher.voucherListTable.columnAction')" width="300" align="center">
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
          <Check /> {{ t('voucher.voucherListTable.buttonApprove') }}
        </ElButton>
        <ElButton
          v-if="scope.row.status === 'approved'"
          size="small"
          type="success"
          @click="emit('post', scope.row as VoucherEntity)"
        >
          <Check /> {{ t('voucher.voucherListTable.buttonPost') }}
        </ElButton>
        <ElButton
          v-if="scope.row.status === 'posted'"
          size="small"
          type="info"
          @click="emit('unpost', scope.row as VoucherEntity)"
        >
          <Refresh /> {{ t('voucher.voucherListTable.buttonUnpost') }}
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
      :current-page="page"
      :page-size="pageSize"
      :page-sizes="[10, 20, 50, 100]"
      :total="total"
      layout="total, sizes, prev, pager, next, jumper"
      :aria-label="t('voucher.voucherListTable.paginationAriaLabel')"
      @update:current-page="emit('update:page', $event as number)"
      @update:page-size="emit('update:page-size', $event as number)"
    />
  </div>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { Edit, Delete, View, Refresh, Check } from '@element-plus/icons-vue'
import type { VoucherEntity } from '@/api/voucher'
import { getStatusClass, formatAmount } from '../composables/vchrLstFmts'

const { t } = useI18n({ useScope: 'global' })

/**
 * 凭证列表表格组件
 * 仅做展示，行内操作通过 emit 通知父组件
 * 分页通过 v-model:page / v-model:page-size 与父组件双向绑定
 */
const props = defineProps<{
  // 列表数据
  tableData: VoucherEntity[]
  // 加载中
  loading: boolean
  // 总数
  total: number
  // 当前页码
  page: number
  // 每页大小
  pageSize: number
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
  'update:page': [page: number]
  // 每页大小
  'update:page-size': [size: number]
}>()

/** 状态 → 国际化标签（语言切换时响应式刷新） */
const getStatusLabel = (value: string) => {
  const map: Record<string, string> = {
    draft: t('voucher.voucherListTable.statusDraft'),
    approved: t('voucher.voucherListTable.statusApproved'),
    posted: t('voucher.voucherListTable.statusPosted'),
  }
  return map[value] || value
}

/** 凭证类型 → 国际化标签 */
const getTypeLabel = (type: string) => {
  const map: Record<string, string> = {
    general: t('voucher.voucherListTable.typeGeneral'),
    customized: t('voucher.voucherListTable.typeCustomized'),
  }
  return map[type] || type
}

void props
</script>
