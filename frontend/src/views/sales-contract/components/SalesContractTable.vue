<!--
  SalesContractTable.vue - 销售合同列表表格
  拆分自 sales-contract/index.vue（P14 批 2 I-3 第 1 批）
  批次 284：接入 useTableApi 模式（page/pageSize props + v-model 绑定分页）
  本次迁移：el-table + el-pagination → V2Table 虚拟滚动表格
-->
<template>
  <el-card shadow="hover" class="table-card">
    <V2Table
      :columns="columns"
      :data="contractList"
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
import { h } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElButton, ElTag } from 'element-plus';
import V2Table from '@/components/V2Table/index.vue';
import type { ColumnDef } from '@/components/V2Table/types';
import type { SalesContract } from '@/api/sales-contract';
// P2-17 修复（批次 86 v2 复审）：h() 渲染函数无法使用 v-permission 指令，
// 改为复用 router 守卫的 hasRoutePermission + useUserStore 做权限判断，
// 行为与 v-permission 指令保持一致（无权限则不渲染该按钮）
import { hasRoutePermission } from '@/router';
import { useUserStore } from '@/store/user';
import { formatCurrency, getStatusType } from '../composables/scFmts';

const { t } = useI18n({ useScope: 'global' });

// 状态 el-tag 类型别名（与 element-plus 类型保持一致）
type ElTagType = 'primary' | 'success' | 'warning' | 'info' | 'danger';

/**
 * 权限检查辅助函数（与 v-permission 指令行为等价）
 * @param required 所需权限码
 * @returns 当前用户是否持有该权限
 */
const can = (required: string): boolean => {
  const userStore = useUserStore();
  const permissions = userStore.userInfo?.permissions || [];
  return hasRoutePermission(required, permissions);
};

/**
 * 获取合同状态标签（i18n 响应式：语言切换后自动重算）
 * @param status 合同状态码
 * @returns 状态对应的本地化标签
 */
const getStatusLabel = (status: string): string => {
  const map: Record<string, string> = {
    draft: t('salesContract.table.statusDraft'),
    pending: t('salesContract.table.statusPending'),
    active: t('salesContract.table.statusActive'),
    completed: t('salesContract.table.statusCompleted'),
    cancelled: t('salesContract.table.statusCancelled'),
  };
  return map[status] || status;
};

/**
 * 销售合同列表表格组件（批次 284：page/pageSize props + v-model 绑定分页）
 * 迁移至 V2Table 虚拟滚动表格，移除 el-table / el-pagination
 */
defineProps<{
  // 列表数据
  contractList: SalesContract[];
  // 加载状态
  loading: boolean;
  // 总数
  total: number;
  // 当前页
  page: number;
  // 每页条数
  pageSize: number;
}>();

const emit = defineEmits<{
  view: [row: SalesContract];
  edit: [row: SalesContract];
  'submit-approval': [row: SalesContract];
  approve: [row: SalesContract];
  execute: [row: SalesContract];
  delete: [row: SalesContract];
  'update:page': [v: number];
  'update:page-size': [v: number];
}>();

/**
 * 构建操作列按钮组（按 status 条件渲染不同按钮，编辑/删除受权限控制）
 * @param row 当前行数据
 * @returns 按钮 vnode 数组
 */
const mkBtn = (
  type: 'primary' | 'success' | 'warning' | 'danger',
  labelKey: string,
  onClick: () => void
) => h(ElButton, { type, link: true, size: 'small', onClick }, { default: () => t(labelKey) });

const buildActionButtons = (row: SalesContract): ReturnType<typeof h>[] => {
  const buttons: ReturnType<typeof h>[] = [
    mkBtn('primary', 'salesContract.table.buttonView', () => emit('view', row)),
  ];
  // 草稿状态：编辑 / 提交 / 删除（编辑/删除受权限控制）
  if (row.status === 'draft') {
    if (can('sales_contract:update')) {
      buttons.push(mkBtn('primary', 'salesContract.table.buttonEdit', () => emit('edit', row)));
    }
    buttons.push(
      mkBtn('success', 'salesContract.table.buttonSubmit', () => emit('submit-approval', row))
    );
    if (can('sales_contract:delete')) {
      buttons.push(mkBtn('danger', 'salesContract.table.buttonDelete', () => emit('delete', row)));
    }
  }
  // 待审批状态：审批
  if (row.status === 'pending') {
    buttons.push(mkBtn('success', 'salesContract.table.buttonApprove', () => emit('approve', row)));
  }
  // 执行中状态：执行
  if (row.status === 'active') {
    buttons.push(mkBtn('warning', 'salesContract.table.buttonExecute', () => emit('execute', row)));
  }
  return buttons;
};

/**
 * 列定义
 * - 合同金额：formatCurrency 格式化为人民币
 * - 状态：el-tag 渲染（类型由 getStatusType 映射）
 * - 操作列：按 status 条件渲染不同按钮组（编辑/删除受权限控制）
 */
const columns: ColumnDef<SalesContract>[] = [
  { key: 'contract_no', title: t('salesContract.table.columnContractNo'), width: 150 },
  { key: 'contract_name', title: t('salesContract.table.columnContractName'), minWidth: 200 },
  { key: 'customer_name', title: t('salesContract.table.columnCustomer'), width: 150 },
  {
    key: 'total_amount',
    title: t('salesContract.table.columnTotalAmount'),
    width: 120,
    align: 'right',
    formatter: row => formatCurrency(row.total_amount),
  },
  {
    key: 'signed_date',
    title: t('salesContract.table.columnSignedDate'),
    width: 120,
    align: 'center',
  },
  {
    key: 'effective_date',
    title: t('salesContract.table.columnEffectiveDate'),
    width: 120,
    align: 'center',
  },
  {
    key: 'expiry_date',
    title: t('salesContract.table.columnExpiryDate'),
    width: 120,
    align: 'center',
  },
  {
    key: 'status',
    title: t('salesContract.table.columnStatus'),
    width: 100,
    align: 'center',
    renderCell: row => {
      // scFmts 的 getStatusType 返回 string，需收窄为 ElTagType 以满足 el-tag 类型约束
      const tagType: ElTagType = (getStatusType(row.status) as ElTagType) || 'info';
      return h(ElTag, { type: tagType }, { default: () => getStatusLabel(row.status) });
    },
  },
  {
    key: '__actions__',
    title: t('salesContract.table.columnAction'),
    width: 250,
    fixed: 'right',
    align: 'center',
    renderCell: row => h('div', { class: 'action-cell' }, buildActionButtons(row)),
  },
];
</script>

<style scoped>
.table-card {
  margin-bottom: 20px;
}
.action-cell {
  display: flex;
  gap: 4px;
  align-items: center;
  justify-content: center;
}
</style>
