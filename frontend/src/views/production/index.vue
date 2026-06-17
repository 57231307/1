<template>
  <div class="production-container">
    <el-card class="header-card">
      <div class="header-content">
        <h2>生产计划管理 (MRP)</h2>
        <p>管理和跟踪生产订单，制定和执行生产计划</p>
      </div>
    </el-card>

    <!-- 筛选区 -->
    <ProductionList
      :orders="orders"
      :total="total"
      :loading="loading"
      :query-params="queryParams"
      :status-type-map="statusTypeMap"
      :status-map="statusMap"
      :priority-type-map="priorityTypeMap"
      :priority-map="priorityMap"
      @search="fetchData"
      @update:query-params="(v: any) => Object.assign(queryParams, v)"
      @add="handleAdd"
      @view="handleView"
      @edit="handleEdit"
      @delete="handleDelete"
      @audit="handleAudit"
      @update-status="handleUpdateStatus"
    />

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
              PRODUCTION_ORDER_STATUS[currentOrder?.status as keyof typeof PRODUCTION_ORDER_STATUS]
                ?.type
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
import ProductionList from './ProductionList.vue'
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Download, Printer } from '@element-plus/icons-vue'
import {
  listProductionOrders,
  createProductionOrder,
  updateProductionOrder,
  type ProductionOrder,
  PRODUCTION_ORDER_STATUS,
} from '@/api/production'
import V2Table from '@/components/V2Table/index.vue'
import { useTableColumns } from '@/composables/useTableColumns'

// 生产订单列表列定义（V2Table 渲染）
const { columns: productionColumns } = useTableColumns([
  { key: 'order_no', title: '工单编号', width: 160, sortable: true },
  { key: 'product_name', title: '产品', width: 200 },
  { key: 'planned_quantity', title: '计划数量', width: 120, align: 'right' },
  {
    key: 'actual_quantity',
    title: '完成数量',
    width: 120,
    align: 'right',
    formatter: (row: any) => `${row.actual_quantity ?? 0} / ${row.planned_quantity ?? 0}`,
  },
  {
    key: 'scheduled_start_date',
    title: '开始日期',
    width: 120,
    formatter: (row: any) =>
      row.scheduled_start_date ? String(row.scheduled_start_date).substring(0, 10) : '-',
  },
  {
    key: 'scheduled_end_date',
    title: '结束日期',
    width: 120,
    formatter: (row: any) =>
      row.scheduled_end_date ? String(row.scheduled_end_date).substring(0, 10) : '-',
  },
  { key: 'status', title: '状态', width: 100, align: 'center' },
  { key: 'priority', title: '优先级', width: 100 },
])

// 行点击：触发查看详情
const handleProductionRowClick = (row: ProductionOrder) => {
  viewDetail(row)
}

// V2Table 内置分页事件处理
const handlePageChange = (newPage: number) => {
  queryForm.page = newPage
  fetchOrders()
}

const handleSizeChange = (newSize: number) => {
  queryForm.page_size = newSize
  queryForm.page = 1
  fetchOrders()
}

// 响应式数据
const loading = ref(false)
const submitLoading = ref(false)
const dialogVisible = ref(false)
const detailVisible = ref(false)
const dialogType = ref<'create' | 'edit'>('create')
const orderList = ref<ProductionOrder[]>([])
const currentOrder = ref<ProductionOrder | null>(null)
const orderFormRef = ref<FormInstance>()
const total = ref(0)

// 查询表单
const queryForm = reactive({
  page: 1,
  page_size: 20,
  order_no: '',
  status: '',
})

// 订单表单
const orderForm = reactive<Partial<ProductionOrder>>({
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
const orderRules: FormRules = {
  order_no: [{ required: true, message: '请输入订单编号', trigger: 'blur' }],
  product_id: [{ required: true, message: '请输入产品ID', trigger: 'blur' }],
  planned_quantity: [{ required: true, message: '请输入计划数量', trigger: 'blur' }],
  priority: [{ required: true, message: '请选择优先级', trigger: 'change' }],
}

// 获取生产订单列表
const fetchOrders = async () => {
  loading.value = true
  try {
    const res = await listProductionOrders(queryForm)
    orderList.value = res.data!.list || []
    total.value = res.data?.total || 0
  } catch (e: any) {
    ElMessage.error(e.message || '获取订单列表失败')
  } finally {
    loading.value = false
  }
}

// 重置查询
const resetQuery = () => {
  queryForm.page = 1
  queryForm.order_no = ''
  queryForm.status = ''
  fetchOrders()
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

  await orderFormRef.value.validate(async valid => {
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
      fetchOrders()
    } catch (e: any) {
      ElMessage.error(e.message || '操作失败')
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

const getStatusLabel = (status: string) => {
  return PRODUCTION_ORDER_STATUS[status as keyof typeof PRODUCTION_ORDER_STATUS]?.label || status
}

const handleExport = () => {
  const csvContent = [
    ['订单编号', '产品名称', '计划数量', '实际数量', '计划开始', '计划结束', '状态', '优先级'],
    ...orderList.value.map((item: any) => [
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
  const rows = orderList.value
    .map(
      (item: any) => `
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
    <h1>生产订单列表</h1><div class="meta">打印日期: ${new Date().toISOString().split('T')[0]} | 共 ${orderList.value.length} 条</div>
    <table><thead><tr><th>订单编号</th><th>产品名称</th><th>计划数量</th><th>实际数量</th><th>计划开始</th><th>计划结束</th><th>状态</th><th>优先级</th></tr></thead><tbody>${rows}</tbody></table></body></html>`)
  printWindow.document.close()
  printWindow.onload = () => printWindow.print()
}

// 组件挂载时获取数据
onMounted(() => {
  fetchOrders()
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

.pagination-container {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}
</style>
