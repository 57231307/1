<!--
  BpmApprovalCompletedTable.vue - BPM 审批已办任务表
  拆分自 bpm/approval.vue（P14 批 2 I-3 第 4 批）
  批次 283：接入 useTableApi 模式（page/pageSize props + v-model 绑定分页）
  迁移：el-table + el-pagination → V2Table 虚拟滚动表格
-->
<template>
  <el-card shadow="hover" class="table-card">
    <V2Table
      :columns="columns"
      :data="tasks"
      :loading="loading"
      :page="page"
      :page-size="pageSize"
      :page-sizes="[10, 20, 50]"
      :total="total"
      :height="600"
      @page-change="(v: number) => emit('update:page', v)"
      @size-change="(v: number) => emit('update:page-size', v)"
    />
  </el-card>
</template>

<script setup lang="ts">
import { computed, h } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElButton, ElTag } from 'element-plus';
import V2Table from '@/components/V2Table/index.vue';
import type { ColumnDef } from '@/components/V2Table/types';
import type { ApprovalTask } from '@/api/bpm-enhanced';
import { getTaskStatusType, formatDateTime } from '../composables/bpmApFmts';

const { t } = useI18n({ useScope: 'global' });

/** 已办任务状态（completed/rejected/cancelled）显示文本，取值域与后端 bpm_task 词表同源 */
const getTaskStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    completed: t('bpm.approval.taskStatus.completed'),
    rejected: t('bpm.approval.taskStatus.rejected'),
    cancelled: t('bpm.approval.taskStatus.cancelled'),
    pending: t('bpm.approval.taskStatus.pending'),
  };
  return map[status] || status;
};

/**
 * 审批已办任务表组件
 */
defineProps<{
  // 任务列表
  tasks: ApprovalTask[];
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
  'view-chain': [row: ApprovalTask];
  'update:page': [v: number];
  'update:page-size': [v: number];
}>();

/** 列定义：与后端 bpm_task 实体字段逐一对应（任务编号/节点名称/流程实例/办理时间/办理结果/审批意见） */
const columns = computed<ColumnDef<ApprovalTask>[]>(() => [
  { key: 'task_no', title: t('bpm.approval.completedTable.taskNo'), minWidth: 160 },
  { key: 'node_name', title: t('bpm.approval.completedTable.taskName'), minWidth: 160 },
  {
    key: 'instance_id',
    title: t('bpm.approval.completedTable.instanceId'),
    width: 110,
    renderCell: (row: ApprovalTask) => h('span', String(row.instance_id)),
  },
  {
    key: 'handled_at',
    title: t('bpm.approval.completedTable.approvedAt'),
    width: 180,
    renderCell: (row: ApprovalTask) => h('span', formatDateTime(row.handled_at)),
  },
  {
    key: 'status',
    title: t('bpm.approval.completedTable.result'),
    width: 110,
    renderCell: (row: ApprovalTask) =>
      h(
        ElTag,
        {
          type: getTaskStatusType(row.status || '') as
            'success' | 'warning' | 'info' | 'primary' | 'danger',
          size: 'small',
        },
        { default: () => getTaskStatusLabel(row.status || '') }
      ),
  },
  {
    key: 'approval_opinion',
    title: t('bpm.approval.completedTable.comment'),
    minWidth: 200,
    renderCell: (row: ApprovalTask) => h('span', row.approval_opinion || '-'),
  },
  {
    key: '__actions__',
    title: t('bpm.approval.completedTable.operation'),
    width: 120,
    renderCell: (row: ApprovalTask) =>
      h(
        ElButton,
        { type: 'info', link: true, size: 'small', onClick: () => emit('view-chain', row) },
        { default: () => t('bpm.approval.completedTable.viewChain') }
      ),
  },
]);
</script>

<style scoped>
.table-card {
  margin-bottom: 20px;
}
</style>
