<!--
  ScTbl.vue - 销售合同列表表格
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
import { h } from 'vue'
import { ElButton, ElTag } from 'element-plus'
import V2Table from '@/components/V2Table/index.vue'
import type { ColumnDef } from '@/components/V2Table/types'
import type { SalesContract } from '@/api/sales-contract'
// P2-17 修复（批次 86 v2 复审）：h() 渲染函数无法使用 v-permission 指令，
// 改为复用 router 守卫的 hasRoutePermission + useUserStore 做权限判断，
// 行为与 v-permission 指令保持一致（无权限则不渲染该按钮）
import { hasRoutePermission } from '@/router'
import { useUserStore } from '@/store/user'
import { formatCurrency, getStatusType, getStatusLabel } from '../composables/scFmts'

// 状态 el-tag 类型别名（与 element-plus 类型保持一致）
type ElTagType = 'primary' | 'success' | 'warning' | 'info' | 'danger'

/**
 * 权限检查辅助函数（与 v-permission 指令行为等价）
 * @param required 所需权限码
 * @returns 当前用户是否持有该权限
 */
const can = (required: string): boolean => {
  const userStore = useUserStore()
  const permissions = userStore.userInfo?.permissions || []
  return hasRoutePermission(required, permissions)
}

/**
 * 销售合同列表表格组件（批次 284：page/pageSize props + v-model 绑定分页）
 * 迁移至 V2Table 虚拟滚动表格，移除 el-table / el-pagination
 */
defineProps<{
  // 列表数据
  contractList: SalesContract[]
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
  view: [row: SalesContract]
  edit: [row: SalesContract]
  'submit-approval': [row: SalesContract]
  approve: [row: SalesContract]
  execute: [row: SalesContract]
  delete: [row: SalesContract]
  'update:page': [v: number]
  'update:page-size': [v: number]
}>()

/**
 * 列定义
 * - 合同金额：formatCurrency 格式化为人民币
 * - 状态：el-tag 渲染（类型由 getStatusType 映射）
 * - 操作列：按 status 条件渲染不同按钮组（编辑/删除受权限控制）
 */
const columns: ColumnDef<SalesContract>[] = [
  { key: 'contract_no', title: '合同编号', width: 150 },
  { key: 'contract_name', title: '合同名称', minWidth: 200 },
  { key: 'customer_name', title: '客户', width: 150 },
  {
    key: 'total_amount',
    title: '合同金额',
    width: 120,
    align: 'right',
    formatter: (row) => formatCurrency(row.total_amount),
  },
  { key: 'signed_date', title: '签订日期', width: 120, align: 'center' },
  { key: 'effective_date', title: '生效日期', width: 120, align: 'center' },
  { key: 'expiry_date', title: '到期日期', width: 120, align: 'center' },
  {
    key: 'status',
    title: '状态',
    width: 100,
    align: 'center',
    renderCell: (row) => {
      // scFmts 的 getStatusType 返回 string，需收窄为 ElTagType 以满足 el-tag 类型约束
      const tagType: ElTagType = (getStatusType(row.status) as ElTagType) || 'info'
      return h(
        ElTag,
        { type: tagType },
        { default: () => getStatusLabel(row.status) }
      )
    },
  },
  {
    key: '__actions__',
    title: '操作',
    width: 250,
    fixed: 'right',
    align: 'center',
    renderCell: (row) => {
      const buttons: ReturnType<typeof h>[] = [
        h(
          ElButton,
          { type: 'primary', link: true, size: 'small', onClick: () => emit('view', row) },
          { default: () => '查看' }
        ),
      ]
      // 草稿状态：编辑 / 提交 / 删除（编辑/删除受权限控制）
      if (row.status === 'draft') {
        if (can('sales_contract:update')) {
          buttons.push(
            h(
              ElButton,
              { type: 'primary', link: true, size: 'small', onClick: () => emit('edit', row) },
              { default: () => '编辑' }
            )
          )
        }
        buttons.push(
          h(
            ElButton,
            { type: 'success', link: true, size: 'small', onClick: () => emit('submit-approval', row) },
            { default: () => '提交' }
          )
        )
        if (can('sales_contract:delete')) {
          buttons.push(
            h(
              ElButton,
              { type: 'danger', link: true, size: 'small', onClick: () => emit('delete', row) },
              { default: () => '删除' }
            )
          )
        }
      }
      // 待审批状态：审批
      if (row.status === 'pending') {
        buttons.push(
          h(
            ElButton,
            { type: 'success', link: true, size: 'small', onClick: () => emit('approve', row) },
            { default: () => '审批' }
          )
        )
      }
      // 执行中状态：执行
      if (row.status === 'active') {
        buttons.push(
          h(
            ElButton,
            { type: 'warning', link: true, size: 'small', onClick: () => emit('execute', row) },
            { default: () => '执行' }
          )
        )
      }
      return h('div', { class: 'action-cell' }, buttons)
    },
  },
]
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
