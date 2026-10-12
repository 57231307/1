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
    <!-- P0 修复：列键对齐后端 voucher::Model 真实键 voucher_type（原 prop="type" 恒空）。
         展示值直接使用后端 code（记/收/付/转），前端不维护第二套类型常量。 -->
    <ElTableColumn
      prop="voucher_type"
      :label="t('voucher.voucherListTable.columnVoucherType')"
      width="100"
    />
    <!--
      P0 修复（移除假契约列）：后端 GET /vouchers 列表返回 Vec<voucher::Model>，
      从不包含 total_debit/total_credit（无合计列）与 created_by_name/approved_by_name/
      posted_by_name（无姓名 JOIN，实体仅有 created_by/reviewed_by/posted_by 数值 ID）。
      原列读取恒 undefined（显示 0.00 / 空），已移除；如产品需要「列表合计/经办人姓名」，
      需后端 get_list 增加批量聚合与 users JOIN 富化（已在修复汇报中登记，待拍板）。
    -->
    <ElTableColumn prop="status" :label="t('voucher.voucherListTable.columnStatus')" width="100">
      <template #default="scope">
        <span :class="['status-tag', getStatusClass(scope.row.status)]">
          {{ getStatusLabel(scope.row.status) }}
        </span>
      </template>
    </ElTableColumn>
    <ElTableColumn :label="t('voucher.voucherListTable.columnAction')" width="300" align="center">
      <template #default="scope">
        <ElButton
          v-permission-detail="{ resource: 'vouchers', action: 'read' }"
          size="small"
          @click="emit('view', scope.row as VoucherEntity)"
        >
          <View />
        </ElButton>
        <ElButton
          v-if="scope.row.status === 'draft'"
          v-permission-detail="{ resource: 'vouchers', action: 'update' }"
          size="small"
          type="primary"
          @click="emit('edit', scope.row as VoucherEntity)"
        >
          <Edit />
        </ElButton>
        <!-- P0 修复（三端同源）：后端状态机 draft→submitted→reviewed→posted（voucher_ops/workflow.rs），
             审核（/review）门为 submitted ⇒ 草稿行先「提交」（POST /vouchers/:id/submit，
             routes/finance.rs 已挂载），提交后的行才出现「审核」按钮；
             原「审核」按钮门 draft 点击必 400（review 要求 submitted），已订正。 -->
        <ElButton
          v-if="scope.row.status === 'draft'"
          size="small"
          type="primary"
          @click="emit('submit', scope.row as VoucherEntity)"
        >
          <Check /> {{ t('voucher.voucherListTable.buttonSubmit') }}
        </ElButton>
        <ElButton
          v-if="scope.row.status === 'submitted'"
          size="small"
          type="warning"
          @click="emit('approve', scope.row as VoucherEntity)"
        >
          <Check /> {{ t('voucher.voucherListTable.buttonApprove') }}
        </ElButton>
        <!-- 「记账」按钮门对齐真实可达态 reviewed（后端 post 门=reviewed） -->
        <ElButton
          v-if="scope.row.status === 'reviewed'"
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
        <!-- P0 修复：后端 delete 状态门仅 draft（凭证服务 crud.rs::delete），
             原 status !== 'posted' 会在 submitted/reviewed 行露出删除按钮、点击必 400 -->
        <ElButton
          v-if="scope.row.status === 'draft'"
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
import { useI18n } from 'vue-i18n';
import { Edit, Delete, View, Refresh, Check } from '@element-plus/icons-vue';
import type { VoucherEntity } from '@/api/voucher';
import { getStatusClass } from '../composables/vchrLstFmts';

const { t } = useI18n({ useScope: 'global' });

/**
 * 凭证列表表格组件
 * 仅做展示，行内操作通过 emit 通知父组件
 * 分页通过 v-model:page / v-model:page-size 与父组件双向绑定
 */
const props = defineProps<{
  // 列表数据
  tableData: VoucherEntity[];
  // 加载中
  loading: boolean;
  // 总数
  total: number;
  // 当前页码
  page: number;
  // 每页大小
  pageSize: number;
}>();

const emit = defineEmits<{
  // 查看
  view: [row: VoucherEntity];
  // 编辑
  edit: [row: VoucherEntity];
  // 提交（draft→submitted，POST /vouchers/:id/submit）
  submit: [row: VoucherEntity];
  // 审核（submitted→reviewed，POST /vouchers/:id/review）
  approve: [row: VoucherEntity];
  // 记账
  post: [row: VoucherEntity];
  // 反记账
  unpost: [row: VoucherEntity];
  // 删除
  delete: [row: VoucherEntity];
  // 翻页
  'update:page': [page: number];
  // 每页大小
  'update:page-size': [size: number];
}>();

/** 状态 → 国际化标签（语言切换时响应式刷新；词表对齐后端 status::finance::voucher） */
const getStatusLabel = (value: string) => {
  const map: Record<string, string> = {
    draft: t('voucher.voucherListTable.statusDraft'),
    submitted: t('voucher.voucherListTable.statusSubmitted'),
    reviewed: t('voucher.voucherListTable.statusReviewed'),
    posted: t('voucher.voucherListTable.statusPosted'),
  };
  return map[value] || value;
};

void props;
</script>
