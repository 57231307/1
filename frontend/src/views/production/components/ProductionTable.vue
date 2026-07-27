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
import { PRODUCTION_ORDER_STATUS, type ProductionOrder } from '@/api/production';
// P2-17 修复（批次 86 v2 复审）：h() 渲染函数无法使用 v-permission 指令，
// 改为复用 router 守卫的 hasRoutePermission + useUserStore 做权限判断，
// 行为与 v-permission 指令保持一致（无权限则不渲染该按钮）
import { hasRoutePermission } from '@/router';
import { useUserStore } from '@/store/user';

const { t } = useI18n({ useScope: 'global' });

// 状态 el-tag 类型别名（与 element-plus 类型保持一致）
type ElTagType = 'primary' | 'success' | 'warning' | 'info' | 'danger';

/** 权限检查辅助函数（与 v-permission 指令行为等价） */
const can = (required: string): boolean => {
  const userStore = useUserStore();
  const permissions = userStore.userInfo?.permissions || [];
  return hasRoutePermission(required, permissions);
};

/** 状态标签：优先 i18n，回退到原始 status 字符串 */
const statusLabel = (status: string): string => {
  const key = `production.table.status${status.charAt(0).toUpperCase() + status.slice(1)}`;
  const translated = t(key);
  return translated === key
    ? PRODUCTION_ORDER_STATUS[status as keyof typeof PRODUCTION_ORDER_STATUS]?.label || status
    : translated;
};

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
}>();

/** 创建操作按钮 vnode（≤50 行） */
const renderActionButtons = (row: ProductionOrder): ReturnType<typeof h>[] => {
  const buttons: ReturnType<typeof h>[] = [
    h(
      ElButton,
      { type: 'primary', link: true, size: 'small', onClick: () => emit('view-detail', row) },
      { default: () => t('production.table.buttonView') }
    ),
  ];
  if (row.status === 'draft') {
    if (can('production_order:update')) {
      buttons.push(
        h(
          ElButton,
          { type: 'success', link: true, size: 'small', onClick: () => emit('open-edit', row) },
          { default: () => t('production.table.buttonEdit') }
        )
      );
    }
    buttons.push(
      h(
        ElButton,
        {
          type: 'warning',
          link: true,
          size: 'small',
          onClick: () => emit('status-change', row, 'planned'),
        },
        { default: () => t('production.table.buttonPlan') }
      )
    );
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
  if (row.status === 'planned') {
    buttons.push(
      h(
        ElButton,
        {
          type: 'primary',
          link: true,
          size: 'small',
          onClick: () => emit('status-change', row, 'in_production'),
        },
        { default: () => t('production.table.buttonStartProduction') }
      )
    );
  }
  if (row.status === 'in_production') {
    buttons.push(
      h(
        ElButton,
        {
          type: 'success',
          link: true,
          size: 'small',
          onClick: () => emit('status-change', row, 'completed'),
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
    renderCell: (row: ProductionOrder) => {
      const statusConfig =
        PRODUCTION_ORDER_STATUS[row.status as keyof typeof PRODUCTION_ORDER_STATUS];
      const tagType: ElTagType = (statusConfig?.type as ElTagType) || 'info';
      return h(ElTag, { type: tagType }, { default: () => statusLabel(row.status) });
    },
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
