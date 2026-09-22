<!--
  ProductionTable.vue - 生产管理订单表（V2Table 包装）
  拆分自 production/index.vue（P14 批 2 I-3 第 4 批）
  行为完全保持一致（仅结构重构）

  P9-3 清理：原文件级 vue/no-mutating-props disable 注释已删除
  （本组件仅读取 props 传给 V2Table 并 emit 事件，无 prop mutation 行为）
-->
<template>
  <V2Table
    :columns="columns"
    :data="data"
    :loading="loading"
    :page="page"
    :page-size="pageSize"
    :total="total"
    :height="600"
    @page-change="(p: number) => emit('page-change', p)"
    @size-change="(s: number) => emit('size-change', s)"
  />
</template>

<script setup lang="ts">
import { h, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElTag, ElButton } from 'element-plus';
import V2Table from '@/components/V2Table/index.vue';
import type { ColumnDef } from '@/components/V2Table/types';
import type { ProductionOrder } from '@/api/production';
import { getStatusLabel, getStatusType } from '../composables/prdFmts';
// P2-17 修复（批次 86 v2 复审）：h() 渲染函数无法使用 v-permission 指令，
// 改为复用 router 守卫的 hasRoutePermission + useUserStore 做权限判断，
// 行为与 v-permission 指令保持一致（无权限则不渲染该按钮）
import { hasRoutePermission, canAccessDetailPermission } from '@/router';
import { useUserStore } from '@/store/user';

const { t } = useI18n({ useScope: 'global' });

// 状态 el-tag 类型别名（与 element-plus 类型保持一致）

/** 权限检查辅助函数（与 v-permission 指令行为等价） */
const can = (required: string): boolean => {
  const userStore = useUserStore();
  const permissions = userStore.userInfo?.permissions || [];
  return hasRoutePermission(required, permissions);
};

// 详情/编辑入口回源/提交 `/production/.../orders/{id}`，后端对 resource_id=NULL 行拒绝 `/{id}`，
// 非管理员点了必然 403。与后端同源判定（canAccessDetailPermission）决定是否隐藏。
const canViewDetail = computed(() => {
  const userStore = useUserStore();
  return canAccessDetailPermission(
    'production-orders',
    'read',
    userStore.userInfo?.permissions || []
  );
});
const canEditDetail = computed(() => {
  const userStore = useUserStore();
  return canAccessDetailPermission(
    'production-orders',
    'update',
    userStore.userInfo?.permissions || []
  );
});

const statusLabel = getStatusLabel;

defineProps<{
  data: ProductionOrder[];
  loading: boolean;
  page: number;
  pageSize: number;
  total: number;
}>();

const emit = defineEmits<{
  'page-change': [page: number];
  'size-change': [size: number];
  'view-detail': [row: ProductionOrder];
  'open-edit': [row: ProductionOrder];
  'status-change': [row: ProductionOrder, status: string];
  delete: [row: ProductionOrder];
  'submit-approval': [row: ProductionOrder];
  'approve-order': [row: ProductionOrder, approved: boolean];
  'report-progress': [row: ProductionOrder];
  'view-logs': [row: ProductionOrder];
}>();

/** 创建操作按钮 vnode（≤50 行） */
const renderActionButtons = (row: ProductionOrder): ReturnType<typeof h>[] => {
  // 后端状态机为大写（DRAFT/PENDING_APPROVAL/SCHEDULED/IN_PROGRESS/COMPLETED/REJECTED）
  const buttons: ReturnType<typeof h>[] = [];
  // 详情入口：与后端同源判定无 /{id} 权限时不渲染，避免出现点了必然 403 的死入口
  if (canViewDetail.value) {
    buttons.push(
      h(
        ElButton,
        { type: 'primary', link: true, size: 'small', onClick: () => emit('view-detail', row) },
        { default: () => t('production.table.buttonView') }
      )
    );
  }
  buttons.push(
    h(
      ElButton,
      { type: 'info', link: true, size: 'small', onClick: () => emit('view-logs', row) },
      { default: () => '日志' }
    )
  );
  const upper = String(row.status || '').toUpperCase();
  if (upper === 'DRAFT') {
    buttons.push(
      h(
        ElButton,
        { type: 'warning', link: true, size: 'small', onClick: () => emit('submit-approval', row) },
        { default: () => '提交审批' }
      )
    );
  }
  if (upper === 'PENDING_APPROVAL') {
    buttons.push(
      h(
        ElButton,
        {
          type: 'success',
          link: true,
          size: 'small',
          onClick: () => emit('approve-order', row, true),
        },
        { default: () => '审批通过' }
      ),
      h(
        ElButton,
        {
          type: 'danger',
          link: true,
          size: 'small',
          onClick: () => emit('approve-order', row, false),
        },
        { default: () => '驳回' }
      )
    );
  }
  if (upper === 'IN_PROGRESS') {
    buttons.push(
      h(
        ElButton,
        { type: 'warning', link: true, size: 'small', onClick: () => emit('report-progress', row) },
        { default: () => '汇报进度' }
      )
    );
  }
  // 草稿可改可删；排产/开工/完工按状态机的下一步给出，
  // 目标状态取后端 PUT /status 白名单里的值（SCHEDULED/IN_PROGRESS/COMPLETED）
  if (upper === 'DRAFT') {
    // 编辑入口提交 PUT /orders/{id}，后端对 resource_id=NULL 行拒绝；与后端同源判定后隐藏
    if (can('production_order:update') && canEditDetail.value) {
      buttons.push(
        h(
          ElButton,
          { type: 'success', link: true, size: 'small', onClick: () => emit('open-edit', row) },
          { default: () => t('production.table.buttonEdit') }
        )
      );
    }
    if (can('production_order:delete')) {
      buttons.push(
        h(
          ElButton,
          { type: 'danger', link: true, size: 'small', onClick: () => emit('delete', row) },
          { default: () => t('production.table.buttonDelete') }
        )
      );
    }
  }
  if (upper === 'APPROVED') {
    buttons.push(
      h(
        ElButton,
        {
          type: 'warning',
          link: true,
          size: 'small',
          onClick: () => emit('status-change', row, 'SCHEDULED'),
        },
        { default: () => t('production.table.buttonPlan') }
      )
    );
  }
  if (upper === 'SCHEDULED') {
    buttons.push(
      h(
        ElButton,
        {
          type: 'primary',
          link: true,
          size: 'small',
          onClick: () => emit('status-change', row, 'IN_PROGRESS'),
        },
        { default: () => t('production.table.buttonStartProduction') }
      )
    );
  }
  if (upper === 'IN_PROGRESS') {
    buttons.push(
      h(
        ElButton,
        {
          type: 'success',
          link: true,
          size: 'small',
          onClick: () => emit('status-change', row, 'COMPLETED'),
        },
        { default: () => t('production.table.buttonComplete') }
      )
    );
  }
  return buttons;
};

/** 列定义：computed 确保 locale 切换时表头响应式更新 */
const columns = computed<ColumnDef<ProductionOrder>[]>(() => [
  { key: 'order_no', title: t('production.table.colOrderNo'), width: 160, fixed: 'left' },
  { key: 'product_name', title: t('production.table.colProductName'), minWidth: 160 },
  {
    key: 'planned_quantity',
    title: t('production.table.colPlannedQuantity'),
    width: 120,
    align: 'right',
  },
  {
    key: 'actual_quantity',
    title: t('production.table.colActualQuantity'),
    width: 120,
    align: 'right',
  },
  {
    key: 'scheduled_start_date',
    title: t('production.table.colScheduledStart'),
    width: 140,
    formatter: (row: ProductionOrder) =>
      row.scheduled_start_date ? row.scheduled_start_date.substring(0, 10) : '-',
  },
  {
    key: 'scheduled_end_date',
    title: t('production.table.colScheduledEnd'),
    width: 140,
    formatter: (row: ProductionOrder) =>
      row.scheduled_end_date ? row.scheduled_end_date.substring(0, 10) : '-',
  },
  {
    key: 'status',
    title: t('production.table.colStatus'),
    width: 120,
    align: 'center',
    renderCell: (row: ProductionOrder) =>
      h(ElTag, { type: getStatusType(row.status) }, { default: () => statusLabel(row.status) }),
  },
  { key: 'priority', title: t('production.table.colPriority'), width: 100, align: 'center' },
  {
    key: '__actions__',
    title: t('production.table.colAction'),
    width: 280,
    fixed: 'right',
    renderCell: (row: ProductionOrder) =>
      h('div', { class: 'action-cell' }, renderActionButtons(row)),
  },
]);
</script>
