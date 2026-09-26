<!--
  SalesOrderTable.vue - 销售订单列表 V2Table 包装（含操作列）
  拆分自 sales/views/OrderListView.vue（P14 批 2 I-3 第 3 批）
  行为完全保持一致（仅结构重构）
-->
<template>
  <el-card shadow="hover">
    <V2Table
      :columns="fullColumns"
      :data="data"
      :loading="loading"
      :page="page"
      :page-size="pageSize"
      :total="total"
      :height="600"
      @page-change="emit('page-change', $event)"
      @size-change="emit('size-change', $event)"
    />
  </el-card>
</template>

<script setup lang="ts">
import { computed, h } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElButton } from 'element-plus';
import V2Table from '@/components/V2Table/index.vue';
import type { ColumnDef } from '@/components/V2Table/types';
import type { SalesOrder } from '@/api/sales';
import { useUserStore } from '@/store/user';
import { canAccessDetailPermission } from '@/router';

const { t } = useI18n({ useScope: 'global' });
const userStore = useUserStore();

// 后端对 resource_id=NULL 权限行拒绝 /{id} 详情请求；"详情"会回源 /sales/orders/{id}，
// 非管理员点了必然 403，故与后端同源判定后隐藏该入口（"查看"用列表已有行，不隐藏）。
const canAccessOrderDetail = computed(() =>
  canAccessDetailPermission('sales-orders', 'read', userStore.userInfo?.permissions || [])
);

/**
 * 销售订单列表 V2Table 包装组件
 * - 列定义由父组件通过 columns prop 传入（来自 useOlv.composables）
 * - 操作列在本组件内部组装（查看 / 审批 / 发货 / 取消 按状态条件渲染）
 */
const props = defineProps<{
  // 列定义（不含操作列）
  columns: ColumnDef<SalesOrder>[];
  // 列表数据
  data: SalesOrder[];
  // 加载状态
  loading: boolean;
  // 当前页
  page: number;
  // 每页大小
  pageSize: number;
  // 总数
  total: number;
}>();

const emit = defineEmits<{
  'page-change': [page: number];
  'size-change': [size: number];
  view: [row: SalesOrder];
  approve: [row: SalesOrder];
  delivery: [row: SalesOrder];
  cancel: [row: SalesOrder];
  submitOrder: [row: SalesOrder];
  reject: [row: SalesOrder];
  deleteOrder: [row: SalesOrder];
  detail: [row: SalesOrder];
}>();

/** 组装完整列定义：父列 + 操作列 */
const fullColumns = computed<ColumnDef<SalesOrder>[]>(() => [
  ...props.columns,
  {
    key: '__actions__',
    title: t('sales.table.operation'),
    width: 340,
    fixed: 'right',
    renderCell: (row: SalesOrder) => {
      const buttons = [
        h(
          ElButton,
          { size: 'small', link: true, onClick: () => emit('view', row) },
          { default: () => t('sales.table.view') }
        ),
      ];
      // 详情入口回源 /sales/orders/{id}，后端对非管理员的 resource_id=NULL 权限行拒绝，
      // 与后端同源判定无权限时不渲染（隐藏）该按钮，避免出现点了必然 403 的死入口
      if (canAccessOrderDetail.value) {
        buttons.push(
          h(
            ElButton,
            {
              size: 'small',
              link: true,
              onClick: () => emit('detail', row),
            },
            { default: () => t('common.detail') }
          )
        );
      }
      if (row.status === 'draft') {
        buttons.push(
          h(
            ElButton,
            {
              size: 'small',
              link: true,
              type: 'warning',
              onClick: () => emit('submitOrder', row),
            },
            { default: () => t('sales.table.submit') }
          ),
          h(
            ElButton,
            {
              size: 'small',
              link: true,
              type: 'danger',
              onClick: () => emit('deleteOrder', row),
            },
            { default: () => t('sales.table.delete') }
          )
        );
      }
      // 待审核（提交后由后端写入 pending）：审核与驳回都只在这个状态下可用
      if (row.status === 'pending') {
        buttons.push(
          h(
            ElButton,
            {
              size: 'small',
              link: true,
              type: 'primary',
              onClick: () => emit('approve', row),
            },
            { default: () => t('sales.table.approve') }
          ),
          h(
            ElButton,
            {
              size: 'small',
              link: true,
              type: 'danger',
              onClick: () => emit('reject', row),
            },
            { default: () => t('sales.table.reject') }
          )
        );
      }
      if (row.status === 'approved') {
        buttons.push(
          h(
            ElButton,
            {
              size: 'small',
              link: true,
              type: 'success',
              onClick: () => emit('delivery', row),
            },
            { default: () => t('sales.table.deliver') }
          )
        );
      }
      if (row.status === 'pending' || row.status === 'approved') {
        buttons.push(
          h(
            ElButton,
            {
              size: 'small',
              link: true,
              type: 'danger',
              onClick: () => emit('cancel', row),
            },
            { default: () => t('sales.table.cancel') }
          )
        );
      }
      return h('div', { class: 'action-cell' }, buttons);
    },
  },
]);
</script>

<style scoped>
.action-cell {
  display: flex;
  gap: 4px;
  align-items: center;
}
</style>
