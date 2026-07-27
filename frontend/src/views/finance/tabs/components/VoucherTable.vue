<!--
  VoucherTable.vue - 凭证列表表格
  拆分自 VoucherTab.vue（P14 批 1 B3 I-2）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <el-table
      v-loading="voucherLoading"
      :data="vouchers"
      stripe
      :aria-label="t('finance.voucherTable.ariaLabel')"
    >
      <el-table-column
        prop="voucher_no"
        :label="t('finance.voucherTable.columnVoucherNo')"
        width="120"
      />
      <el-table-column
        prop="voucher_date"
        :label="t('finance.voucherTable.columnVoucherDate')"
        width="120"
      />
      <el-table-column
        prop="voucher_type"
        :label="t('finance.voucherTable.columnVoucherType')"
        width="100"
      />
      <el-table-column
        :label="t('finance.voucherTable.columnDebitAmount')"
        width="120"
        align="right"
      >
        <template #default="{ row }">
          {{ formatMoney(row.total_debit) }}
        </template>
      </el-table-column>
      <el-table-column
        :label="t('finance.voucherTable.columnCreditAmount')"
        width="120"
        align="right"
      >
        <template #default="{ row }">
          {{ formatMoney(row.total_credit) }}
        </template>
      </el-table-column>
      <el-table-column
        prop="status"
        :label="t('finance.voucherTable.columnStatus')"
        width="100"
        align="center"
      >
        <template #default="{ row }">
          <el-tag :type="getVoucherStatusType(row.status)" size="small">
            {{ getVoucherStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column
        prop="created_by_name"
        :label="t('finance.voucherTable.columnCreatedBy')"
        width="100"
      />
      <el-table-column
        prop="created_at"
        :label="t('finance.voucherTable.columnCreatedAt')"
        width="160"
      />
      <el-table-column :label="t('finance.voucherTable.columnAction')" width="200" fixed="right">
        <template #default="{ row }">
          <el-button type="primary" link size="small" @click="emit('view', row)">{{
            t('finance.voucherTable.buttonView')
          }}</el-button>
          <el-button
            v-if="row.status === 'draft'"
            type="primary"
            link
            size="small"
            @click="emit('submit', row)"
            >{{ t('finance.voucherTable.buttonSubmit') }}</el-button
          >
          <el-button
            v-if="row.status === 'submitted'"
            type="success"
            link
            size="small"
            @click="emit('review', row)"
            >{{ t('finance.voucherTable.buttonReview') }}</el-button
          >
          <el-button
            v-if="row.status === 'reviewed'"
            type="warning"
            link
            size="small"
            @click="emit('post', row)"
            >{{ t('finance.voucherTable.buttonPost') }}</el-button
          >
        </template>
      </el-table-column>
    </el-table>
    <div class="pagination-wrapper">
      <el-pagination
        :current-page="page"
        :page-size="pageSize"
        :page-sizes="[10, 20, 50, 100]"
        :total="voucherTotal"
        layout="total, sizes, prev, pager, next, jumper"
        :aria-label="t('finance.voucherTable.paginationAriaLabel')"
        @update:current-page="(v: number) => emit('update:page', v)"
        @update:page-size="(v: number) => emit('update:page-size', v)"
      />
    </div>
  </el-card>
</template>

<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { Voucher } from '@/api/finance';

const { t } = useI18n({ useScope: 'global' });

/**
 * 凭证列表表格组件
 * 仅做展示，行内操作通过 emit 通知父组件
 * 批次 289：分页改为 v-model:page/page-size 绑定（由 useTableApi watch 自动加载）
 */
const props = defineProps<{
  vouchers: Voucher[];
  voucherLoading: boolean;
  voucherTotal: number;
  page: number;
  pageSize: number;
  formatMoney: (amount: number) => string;
  getVoucherStatusLabel: (status?: string) => string;
  getVoucherStatusType: (status?: string) => string;
}>();

// 查看凭证 / 提交凭证 / 审核凭证 / 过账凭证 / 分页变化（由 useTableApi watch 自动加载）
const emit = defineEmits<{
  view: [row: Voucher];
  submit: [row: Voucher];
  review: [row: Voucher];
  post: [row: Voucher];
  'update:page': [page: number];
  'update:page-size': [pageSize: number];
}>();

void props;
</script>

<style scoped>
.pagination-wrapper {
  margin-top: 16px;
  display: flex;
  justify-content: flex-end;
}
</style>
