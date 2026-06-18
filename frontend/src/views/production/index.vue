<!--
  production/index.vue - 生产计划管理主入口（V2Table 迁移版）
  ----------------------------------------------------------------
  迁移说明（2026-06-16 P2-1 PR-4）：
  - 替换 el-table 为 V2Table 组件（基于 el-table-v2 的虚拟滚动通用组件）
  - 使用 useTableApi composable 接管分页/loading/重试
  - 保留原交互：header 卡片 / 筛选表单 / 卡片头部新建打印导出按钮
                    el-dialog 新建/编辑(含校验规则) / el-dialog 详情 el-descriptions
  - 保留 PRODUCTION_ORDER_STATUS 常量 + 状态 el-tag 映射
  - 保留操作列条件渲染（draft→编辑/计划/删除；planned→开始生产；in_production→完成）
  - 保留业务方法（CRUD + 状态变更 + 导出 CSV + 打印）
  - 删除 el-pagination（V2Table 自带分页）
-->
<template>
  <div class="production-container">
    <el-card class="header-card">
      <div class="header-content">
        <h2>生产计划管理 (MRP)</h2>
        <p>管理和跟踪生产订单，制定和执行生产计划</p>
      </div>
    </el-card>

    <!-- 筛选区 -->
    <el-card class="filter-card">
      <el-form :inline="true" :model="queryForm">
        <el-form-item label="订单编号">
          <el-input v-model="queryForm.order_no" placeholder="请输入订单编号" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryForm.status" placeholder="请选择状态" clearable>
            <el-option
              v-for="(item, key) in PRODUCTION_ORDER_STATUS"
              :key="key"
              :label="item.label"
              :value="key"
            />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="fetchOrders">查询</el-button>
          <el-button @click="resetQuery">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- 操作区 -->
    <el-card class="table-card">
      <template #header>
        <div class="card-header">
          <span>生产订单列表</span>
          <div class="header-actions">
            <el-button type="primary" @click="openDialog('create')">
              <el-icon><Plus /></el-icon>新建订单
            </el-button>
            <el-button @click="handlePrint">
              <el-icon><Printer /></el-icon>打印
            </el-button>
            <el-button @click="handleExport">
              <el-icon><Download /></el-icon>导出
            </el-button>
          </div>
        </div>
      </template>

      <V2Table
        :columns="columns"
        :data="data"
        :loading="loading"
        :page="page"
        :page-size="pageSize"
        :total="total"
        :height="600"
        @page-change="handlePageChange"
        @size-change="handleSizeChange"
      />
    </el-card>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="dialogType === 'create' ? '新建生产订单' : '编辑生产订单'"
      width="600px"
      @close="resetForm"
    >
      <el-form ref="orderFormRef" :model="orderForm" :rules="orderRules" label-width="120px">
        <el-form-item label="订单编号" prop="order_no">
          <el-input v-model="orderForm.order_no" placeholder="请输入订单编号" />
        </el-form-item>
        <el-form-item label="产品ID" prop="product_id">
          <el-input-number v-model="orderForm.product_id" :min="1" style="width: 100%" />
        </el-form-item>
        <el-form-item label="计划数量" prop="planned_quantity">
          <el-input-number v-model="orderForm.planned_quantity" :min="1" style="width: 100%" />
        </el-form-item>
        <el-form-item label="计划开始">
          <el-date-picker
            v-model="orderForm.scheduled_start_date"
            type="date"
            placeholder="请选择日期"
            style="width: 100%"
            value-format="YYYY-MM-DD"
          />
        </el-form-item>
        <el-form-item label="计划结束">
          <el-date-picker
            v-model="orderForm.scheduled_end_date"
            type="date"
            placeholder="请选择日期"
            style="width: 100%"
            value-format="YYYY-MM-DD"
          />
        </el-form-item>
        <el-form-item label="优先级" prop="priority">
          <el-select v-model="orderForm.priority" placeholder="请选择优先级" style="width: 100%">
            <el-option :label="1" :value="1" />
            <el-option :label="2" :value="2" />
            <el-option :label="3" :value="3" />
            <el-option :label="4" :value="4" />
            <el-option :label="5" :value="5" />
          </el-select>
        </el-form-item>
        <el-form-item label="工作中心">
          <el-input-number v-model="orderForm.work_center_id" :min="1" style="width: 100%" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="orderForm.remark" type="textarea" :rows="3" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确认</el-button>
      </template>
    </el-dialog>

    <!-- 详情对话框 -->
    <el-dialog v-model="detailVisible" title="生产订单详情" width="600px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="订单编号">{{ currentOrder?.order_no }}</el-descriptions-item>
        <el-descriptions-item label="产品ID">{{ currentOrder?.product_id }}</el-descriptions-item>
        <el-descriptions-item label="计划数量">{{
          currentOrder?.planned_quantity
        }}</el-descriptions-item>
        <el-descriptions-item label="实际数量">{{
          currentOrder?.actual_quantity || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="计划开始">{{
          currentOrder?.scheduled_start_date || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="计划结束">{{
          currentOrder?.scheduled_end_date || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="实际开始">{{
          currentOrder?.actual_start_date || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="实际结束">{{
          currentOrder?.actual_end_date || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag
            :type="
              (PRODUCTION_ORDER_STATUS[
                currentOrder?.status as keyof typeof PRODUCTION_ORDER_STATUS
              ]?.type as ElTagType) || 'info'
            "
          >
            {{
              PRODUCTION_ORDER_STATUS[currentOrder?.status as keyof typeof PRODUCTION_ORDER_STATUS]
                ?.label
            }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="优先级">{{ currentOrder?.priority }}</el-descriptions-item>
        <el-descriptions-item label="工作中心">{{
          currentOrder?.work_center_id || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="创建时间">{{ currentOrder?.created_at }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{
          currentOrder?.remark || '-'
        }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
/**
 * 生产订单列表（V2Table 迁移版）
 * - V2Table：基于 el-table-v2 的虚拟滚动通用组件
 * - useTableApi：通用数据 composable（分页/筛选/loading/重试）
 * 保留原交互：header / 筛选 / 卡片头部新建打印导出 / 新建编辑对话框(带校验) /
 *           详情 el-descriptions / 状态 el-tag / 操作按钮条件渲染 /
 *           业务方法（CRUD + 状态变更 + 导出 + 打印）+ 成功调用 refresh()
 */
import { ref, reactive, h, onMounted } from 'vue'
import { ElMessage, ElMessageBox, ElTag, ElButton } from 'element-plus'
import { Plus, Download, Printer } from '@element-plus/icons-vue'
import { useTableApi } from '@/composables/useTableApi'
import V2Table from '@/components/V2Table/index.vue'
import type { ColumnDef } from '@/components/V2Table/types'
import {
  createProductionOrder,
  updateProductionOrder,
  deleteProductionOrder,
  updateProductionOrderStatus,
  type ProductionOrder,
  PRODUCTION_ORDER_STATUS,
} from '@/api/production'

// 生产订单列表（由 useTableApi 接管分页/loading/重试）
const {
  data,
  loading,
  page,
  pageSize,
  total,
  queryParams,
  refresh,
  reset,
} = useTableApi<ProductionOrder>('/production/orders')

// 提交对话框 loading
const submitLoading = ref(false)

// 对话框状态
const dialogVisible = ref(false)
const detailVisible = ref(false)
const dialogType = ref<'create' | 'edit'>('create')
const currentOrder = ref<ProductionOrder | null>(null)
const orderFormRef = ref()

// 本地查询表单（仅 UI 状态，提交时同步到 queryParams）
const queryForm = reactive({
  order_no: '',
  status: '',
})

// 订单表单
const orderForm = reactive<Partial<ProductionOrder>>({
  id: undefined,
  order_no: '',
  product_id: undefined,
  planned_quantity: undefined,
  scheduled_start_date: '',
  scheduled_end_date: '',
  status: 'draft',
  priority: 5,
  work_center_id: undefined,
  remark: '',
})

// 表单验证规则
const orderRules = {
  order_no: [{ required: true, message: '请输入订单编号', trigger: 'blur' }],
  product_id: [{ required: true, message: '请输入产品ID', trigger: 'blur' }],
  planned_quantity: [{ required: true, message: '请输入计划数量', trigger: 'blur' }],
  priority: [{ required: true, message: '请选择优先级', trigger: 'change' }],
}

// 状态 el-tag 类型别名（与 element-plus 类型保持一致）
type ElTagType = 'primary' | 'success' | 'warning' | 'info' | 'danger'

/**
 * 列定义
 * - 计划开始/结束：substring(0, 10) 取日期部分
 * - 状态：使用 PRODUCTION_ORDER_STATUS 嵌套映射 {label, type}
 * - 操作列：按 status 条件渲染不同按钮组
 */
const columns: ColumnDef[] = [
  { key: 'order_no', title: '订单编号', width: 160, fixed: 'left' },
  { key: 'product_name', title: '产品名称', minWidth: 160 },
  { key: 'planned_quantity', title: '计划数量', width: 120, align: 'right' },
  { key: 'actual_quantity', title: '实际数量', width: 120, align: 'right' },
  {
    key: 'scheduled_start_date',
    title: '计划开始',
    width: 140,
    formatter: (row: ProductionOrder) =>
      row.scheduled_start_date ? row.scheduled_start_date.substring(0, 10) : '-',
  },
  {
    key: 'scheduled_end_date',
    title: '计划结束',
    width: 140,
    formatter: (row: ProductionOrder) =>
      row.scheduled_end_date ? row.scheduled_end_date.substring(0, 10) : '-',
  },
  {
    key: 'status',
    title: '状态',
    width: 120,
    align: 'center',
    renderCell: (row: ProductionOrder) => {
      const statusConfig = PRODUCTION_ORDER_STATUS[row.status as keyof typeof PRODUCTION_ORDER_STATUS]
      // 取类型时直接用 ElTag 接受的类型别名，避免 as string 后类型过宽
      const tagType: ElTagType = (statusConfig?.type as ElTagType) || 'info'
      return h(ElTag, { type: tagType }, { default: () => statusConfig?.label || row.status })
    },
  },
  { key: 'priority', title: '优先级', width: 100, align: 'center' },
  {
    key: '__actions__',
    title: '操作',
    width: 280,
    fixed: 'right',
    renderCell: (row: ProductionOrder) => {
      const buttons = [
        h(
          ElButton,
          { type: 'primary', link: true, size: 'small', onClick: () => viewDetail(row) },
          { default: () => '查看' }
        ),
      ]
      if (row.status === 'draft') {
        buttons.push(
          h(
            ElButton,
            { type: 'success', link: true, size: 'small', onClick: () => openDialog('edit', row) },
            { default: () => '编辑' }
          ),
          h(
            ElButton,
            {
              type: 'warning',
              link: true,
              size: 'small',
              onClick: () => handleStatusChange(row, 'planned'),
            },
            { default: () => '计划' }
          ),
          h(
            ElButton,
            { type: 'danger', link: true, size: 'small', onClick: () => handleDelete(row) },
            { default: () => '删除' }
          )
        )
      }
      if (row.status === 'planned') {
        buttons.push(
          h(
            ElButton,
            {
              type: 'primary',
              link: true,
              size: 'small',
              onClick: () => handleStatusChange(row, 'in_production'),
            },
            { default: () => '开始生产' }
          )
        )
      }
      if (row.status === 'in_production') {
        buttons.push(
          h(
            ElButton,
            {
              type: 'success',
              link: true,
              size: 'small',
              onClick: () => handleStatusChange(row, 'completed'),
            },
            { default: () => '完成' }
          )
        )
      }
      return h('div', { class: 'action-cell' }, buttons)
    },
  },
]

// 将本地筛选同步到 useTableApi 的 queryParams 并查询
const fetchOrders = () => {
  const next: Record<string, unknown> = {}
  if (queryForm.order_no) next.order_no = queryForm.order_no
  if (queryForm.status) next.status = queryForm.status
  queryParams.value = next
  page.value = 1
  refresh()
}

// 重置查询
const resetQuery = () => {
  queryForm.order_no = ''
  queryForm.status = ''
  reset()
  refresh()
}

// 分页变化
const handlePageChange = (newPage: number) => {
  page.value = newPage
}

const handleSizeChange = (newSize: number) => {
  pageSize.value = newSize
}

// 打开对话框
const openDialog = (type: 'create' | 'edit', row?: ProductionOrder) => {
  dialogType.value = type
  resetForm()

  if (type === 'edit' && row) {
    Object.assign(orderForm, row)
  }

  dialogVisible.value = true
}

// 重置表单
const resetForm = () => {
  Object.assign(orderForm, {
    id: undefined,
    order_no: '',
    product_id: undefined,
    planned_quantity: undefined,
    scheduled_start_date: '',
    scheduled_end_date: '',
    status: 'draft',
    priority: 5,
    work_center_id: undefined,
    remark: '',
  })
  orderFormRef.value?.clearValidate()
}

// 提交表单
const handleSubmit = async () => {
  if (!orderFormRef.value) return

  await orderFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return

    submitLoading.value = true
    try {
      if (dialogType.value === 'create') {
        await createProductionOrder(orderForm)
        ElMessage.success('创建成功')
      } else {
        if (orderForm.id) {
          await updateProductionOrder(orderForm.id, orderForm)
          ElMessage.success('更新成功')
        }
      }

      dialogVisible.value = false
      refresh()
    } catch (e: unknown) {
      const err = e as { message?: string }
      ElMessage.error(err.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

// 查看详情
const viewDetail = (row: ProductionOrder) => {
  currentOrder.value = row
  detailVisible.value = true
}

// 状态变更
const handleStatusChange = async (row: ProductionOrder, status: string) => {
  try {
    await ElMessageBox.confirm(
      `确认将订单 ${row.order_no} 状态更改为 ${
        PRODUCTION_ORDER_STATUS[status as keyof typeof PRODUCTION_ORDER_STATUS]?.label
      } 吗？`,
      '确认',
      {
        type: 'warning',
      }
    )

    await updateProductionOrderStatus(row.id, status)
    ElMessage.success('状态更新成功')
    refresh()
  } catch (e: unknown) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '状态更新失败')
    }
  }
}

// 删除订单
const handleDelete = async (row: ProductionOrder) => {
  try {
    await ElMessageBox.confirm(`确认删除订单 ${row.order_no} 吗？此操作不可恢复。`, '删除确认', {
      type: 'warning',
      confirmButtonText: '确定',
      cancelButtonText: '取消',
    })

    await deleteProductionOrder(row.id)
    ElMessage.success('删除成功')
    refresh()
  } catch (e: unknown) {
    if (e !== 'cancel') {
      const err = e as { message?: string }
      ElMessage.error(err.message || '删除失败')
    }
  }
}

const getStatusLabel = (status: string) => {
  return PRODUCTION_ORDER_STATUS[status as keyof typeof PRODUCTION_ORDER_STATUS]?.label || status
}

const handleExport = () => {
  const csvContent = [
    ['订单编号', '产品名称', '计划数量', '实际数量', '计划开始', '计划结束', '状态', '优先级'],
    ...data.value.map((item: ProductionOrder) => [
      item.order_no,
      item.product_name,
      item.planned_quantity,
      item.actual_quantity || '-',
      item.scheduled_start_date?.substring(0, 10) || '-',
      item.scheduled_end_date?.substring(0, 10) || '-',
      getStatusLabel(item.status),
      item.priority,
    ]),
  ]
    .map(row => row.map(cell => `"${cell ?? ''}"`).join(','))
    .join('\n')
  const blob = new Blob(['\uFEFF' + csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `生产订单_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
}

const handlePrint = () => {
  const printWindow = window.open('', '_blank')
  if (!printWindow) {
    ElMessage.error('无法打开打印窗口')
    return
  }
  const rows = data.value
    .map(
      (item: ProductionOrder) => `
    <tr>
      <td>${item.order_no}</td><td>${item.product_name || '-'}</td>
      <td style="text-align:right">${item.planned_quantity}</td>
      <td style="text-align:right">${item.actual_quantity || '-'}</td>
      <td>${item.scheduled_start_date?.substring(0, 10) || '-'}</td>
      <td>${item.scheduled_end_date?.substring(0, 10) || '-'}</td>
      <td>${getStatusLabel(item.status)}</td><td>${item.priority}</td>
    </tr>
  `
    )
    .join('')
  printWindow.document.write(`<html><head><meta charset="utf-8"><title>生产订单</title>
    <style>@media print{@page{size:landscape;}}body{font-family:"Microsoft YaHei",sans-serif;font-size:12px;}h1{text-align:center;}table{width:100%;border-collapse:collapse;margin-top:12px;}th,td{border:1px solid #333;padding:6px 8px;}th{background:#f5f5f5;}.meta{text-align:center;color:#666;font-size:11px;}</style></head><body>
    <h1>生产订单列表</h1><div class="meta">打印日期: ${new Date().toISOString().split('T')[0]} | 共 ${data.value.length} 条</div>
    <table><thead><tr><th>订单编号</th><th>产品名称</th><th>计划数量</th><th>实际数量</th><th>计划开始</th><th>计划结束</th><th>状态</th><th>优先级</th></tr></thead><tbody>${rows}</tbody></table></body></html>`)
  printWindow.document.close()
  printWindow.onload = () => printWindow.print()
}

// 组件挂载时获取数据
onMounted(() => {
  refresh()
})
</script>

<style scoped>
.production-container {
  padding: 20px;
}

.header-card {
  margin-bottom: 20px;
}

.header-content h2 {
  margin: 0 0 8px 0;
  color: #303133;
}

.header-content p {
  margin: 0;
  color: #909399;
}

.filter-card {
  margin-bottom: 20px;
}

.table-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.header-actions {
  display: flex;
  gap: 8px;
}

.action-cell {
  display: flex;
  gap: 4px;
  align-items: center;
}
</style>
