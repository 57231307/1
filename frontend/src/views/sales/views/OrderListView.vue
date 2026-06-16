<!--
  sales/index.vue - 销售订单管理主入口（V2Table 迁移版）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-1）：
  原 1125 行"上帝组件"已拆分为以下 3 个独立对话框子组件，
  位于 views/sales/ 目录：

  | 子组件                | 职责                |
  | --------------------- | ------------------- |
  | OrderFormDialog.vue   | 订单编辑/新建对话框 |
  | OrderViewDialog.vue   | 订单详情对话框      |
  | DeliveryDialog.vue    | 销售发货对话框      |

  本主入口仅承担：页面布局 + 列表 + 数据拉取 + 业务方法。
  迁移日期：2026-06-16 P2-1（替换 el-table 为 V2Table + useTableApi）
-->
<template>
  <div class="sales-page">
    <div class="page-header">
      <h2 class="page-title">销售订单管理</h2>
      <el-button type="primary" @click="openCreateDialog">
        <el-icon><Plus /></el-icon> 新建订单
      </el-button>
    </div>

    <el-row :gutter="20" class="stats-cards">
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stats-item">
            <div class="stats-label">订单总数</div>
            <div class="stats-value">{{ stats.totalCount }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stats-item">
            <div class="stats-label">待审批</div>
            <div class="stats-value warning">{{ stats.pendingCount }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stats-item">
            <div class="stats-label">已审批</div>
            <div class="stats-value success">{{ stats.approvedCount }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stats-item">
            <div class="stats-label">订单总额</div>
            <div class="stats-value highlight">¥{{ stats.totalAmount.toLocaleString() }}</div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="filterForm">
        <el-form-item label="订单号">
          <el-input v-model="filterForm.order_no" placeholder="订单号" clearable />
        </el-form-item>
        <el-form-item label="客户">
          <el-input v-model="filterForm.customer_name" placeholder="客户名称" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="filterForm.status" placeholder="选择状态" clearable>
            <el-option label="待审批" value="pending" />
            <el-option label="已审批" value="approved" />
            <el-option label="已发货" value="shipped" />
            <el-option label="已完成" value="completed" />
            <el-option label="已取消" value="cancelled" />
          </el-select>
        </el-form-item>
        <el-form-item label="日期">
          <el-date-picker
            v-model="filterForm.dateRange"
            type="daterange"
            range-separator="至"
            start-placeholder="开始日期"
            end-placeholder="结束日期"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">查询</el-button>
          <el-button @click="resetFilter">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
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

    <!-- 拆分后的对话框子组件 -->
    <OrderFormDialog
      v-model:visible="formDialogVisible"
      :title="formDialogTitle"
      :form-data="formDataForChild"
      :customers="customers"
      :products="products"
      :submitting="submitting"
      @submit="handleFormSubmit as never"
    />

    <OrderViewDialog v-model:visible="viewDialogVisible" :order="currentOrder" />

    <DeliveryDialog
      v-model:visible="deliveryDialogVisible"
      :form="deliveryForm"
      :warehouses="warehouses"
    />
  </div>
</template>

<script setup lang="ts">
/**
 * 销售订单列表主入口（V2Table 迁移版）
 * - V2Table：基于 el-table-v2 的虚拟滚动通用组件
 * - useTableApi：通用数据 composable（分页/筛选/loading/重试）
 * 保留原交互：4 统计卡片 / 筛选表单 / 状态 el-tag / 操作按钮条件渲染 /
 *           3 对话框子组件 / 业务方法（approve/cancel/submit）+ 成功调用 refresh()
 */
import { computed, ref, reactive, watch, onMounted, h } from 'vue'
import { ElMessage, ElMessageBox, ElTag, ElButton } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { useTableApi } from '@/composables/useTableApi'
import V2Table from '@/components/V2Table/index.vue'
import type { ColumnDef } from '@/components/V2Table/types'
import { salesApi, type SalesOrder } from '@/api/sales'
import { request } from '@/api/request'
import type { Customer } from '@/api/customer'
import type { Product } from '@/api/product'
import OrderFormDialog from '../OrderFormDialog.vue'
import OrderViewDialog from '../OrderViewDialog.vue'
import DeliveryDialog from '../DeliveryDialog.vue'

// 销售订单列表（由 useTableApi 接管分页/loading/重试）
const {
  data,
  loading,
  page,
  pageSize,
  total,
  queryParams,
  refresh,
  reset,
} = useTableApi<SalesOrder>('/sales/orders')

// 辅助数据（不走 useTableApi，保留原 request.get 写法）
const customers = ref<Customer[]>([])
const products = ref<Product[]>([])
const warehouses = ref<{ id: number; warehouse_name?: string; name?: string }[]>([])

// 过滤表单（仅本地 UI 状态）
const filterForm = reactive({
  order_no: '',
  customer_name: '',
  status: '',
  dateRange: [] as Date[] | null,
})

// 统计
const stats = reactive({
  totalCount: 0,
  pendingCount: 0,
  approvedCount: 0,
  totalAmount: 0,
})

// 订单表单对话框
const formDialogVisible = ref(false)
const formDialogTitle = ref('新建销售订单')
const submitting = ref(false)
const formData = reactive({
  id: 0,
  customer_id: undefined as number | undefined,
  customer_name: '',
  order_date: new Date(),
  required_date: '',
  contact_person: '',
  contact_phone: '',
  delivery_address: '',
  remark: '',
  items: [] as {
    id: number
    product_id?: number
    product_name: string
    product_code: string
    quantity: number
    unit: string
    unit_price: number
    subtotal: number
  }[],
  total_amount: 0,
})

// 详情对话框
const viewDialogVisible = ref(false)
const currentOrder = ref<SalesOrder | null>(null)

// 发货对话框
const deliveryDialogVisible = ref(false)
const deliveryForm = reactive({
  order_id: 0,
  order_no: '',
  customer_name: '',
  delivery_date: '',
  warehouse_id: undefined as number | undefined,
  items: [] as {
    product_id: number
    product_name: string
    quantity: number
    delivered_quantity: number
    deliver_quantity: number
    unit_price: number
    remarks: string
  }[],
})

const getStatusType = (status: string) => {
  const typeMap: Record<string, string> = {
    pending: 'warning',
    approved: 'primary',
    shipped: 'success',
    completed: 'info',
    cancelled: 'danger',
  }
  return typeMap[status] || 'info'
}

const getStatusText = (status: string) => {
  const textMap: Record<string, string> = {
    pending: '待审批',
    approved: '已审批',
    shipped: '已发货',
    completed: '已完成',
    cancelled: '已取消',
  }
  return textMap[status] || status
}

// 监听列表数据变化，重新计算统计
watch(
  [data, total],
  ([orders, totalValue]) => {
    stats.totalCount = totalValue
    stats.pendingCount = orders.filter(o => o.status === 'pending').length
    stats.approvedCount = orders.filter(o => o.status === 'approved').length
    stats.totalAmount = orders.reduce((sum, o) => sum + (o.total_amount || 0), 0)
  },
  { immediate: true }
)

/**
 * 列定义：使用 renderCell 自定义渲染
 * - 订单金额：¥ + toLocaleString
 * - 状态 el-tag：5 种 type 映射
 * - 操作列：查看 / 审批 / 发货 / 取消（按状态条件渲染）
 */
const columns: ColumnDef[] = [
  { key: 'order_no', title: '订单号', width: 140, fixed: 'left' },
  { key: 'customer_name', title: '客户', minWidth: 150 },
  { key: 'order_date', title: '订单日期', width: 120 },
  { key: 'required_date', title: '交货日期', width: 120 },
  {
    key: 'total_amount',
    title: '订单金额',
    width: 120,
    align: 'right',
    renderCell: (row: SalesOrder) =>
      h('span', `¥${(row.total_amount || 0).toLocaleString()}`),
  },
  {
    key: 'status',
    title: '状态',
    width: 100,
    align: 'center',
    renderCell: (row: SalesOrder) =>
      h(
        ElTag,
        { type: getStatusType(row.status), size: 'small' },
        () => getStatusText(row.status)
      ),
  },
  { key: 'creator_name', title: '创建人', width: 100 },
  {
    key: '__actions__',
    title: '操作',
    width: 280,
    fixed: 'right',
    renderCell: (row: SalesOrder) => {
      const buttons = [
        h(
          ElButton,
          { size: 'small', link: true, onClick: () => viewOrder(row) },
          () => '查看'
        ),
      ]
      if (row.status === 'pending') {
        buttons.push(
          h(
            ElButton,
            {
              size: 'small',
              link: true,
              type: 'primary',
              onClick: () => approveOrder(row),
            },
            () => '审批'
          )
        )
      }
      if (row.status === 'approved') {
        buttons.push(
          h(
            ElButton,
            {
              size: 'small',
              link: true,
              type: 'success',
              onClick: () => openDeliveryDialog(row),
            },
            () => '发货'
          )
        )
      }
      if (row.status === 'pending' || row.status === 'approved') {
        buttons.push(
          h(
            ElButton,
            {
              size: 'small',
              link: true,
              type: 'danger',
              onClick: () => cancelOrder(row),
            },
            () => '取消'
          )
        )
      }
      return h('div', { class: 'action-cell' }, buttons)
    },
  },
]

/**
 * 将 filterForm 同步到 queryParams 并触发查询
 */
const handleQuery = () => {
  const next: Record<string, unknown> = {}
  if (filterForm.order_no) next.order_no = filterForm.order_no
  if (filterForm.customer_name) next.customer_name = filterForm.customer_name
  if (filterForm.status) next.status = filterForm.status
  if (filterForm.dateRange && filterForm.dateRange.length === 2) {
    next.start_date = filterForm.dateRange[0]
    next.end_date = filterForm.dateRange[1]
  }
  queryParams.value = next
  page.value = 1
  refresh()
}

const resetFilter = () => {
  filterForm.order_no = ''
  filterForm.customer_name = ''
  filterForm.status = ''
  filterForm.dateRange = null
  reset()
  refresh()
}

const handlePageChange = (newPage: number) => {
  page.value = newPage
}

const handleSizeChange = (newSize: number) => {
  pageSize.value = newSize
}

const fetchCustomers = async () => {
  try {
    const res = await request.get<{ list?: Customer[] } | Customer[]>('/customers')
    const d = res
    if (Array.isArray(d)) {
      customers.value = d
    } else if (d && typeof d === 'object' && 'list' in d) {
      customers.value = d.list || []
    } else {
      customers.value = []
    }
  } catch (error) {
    customers.value = []
    void error
  }
}

const fetchProducts = async () => {
  try {
    const res = await request.get<{ list?: Product[] } | Product[]>('/products')
    const d = res
    if (Array.isArray(d)) {
      products.value = d
    } else if (d && typeof d === 'object' && 'list' in d) {
      products.value = d.list || []
    } else {
      products.value = []
    }
  } catch (error) {
    products.value = []
    void error
  }
}

const fetchWarehouses = async () => {
  try {
    const res = await request.get<
      | { list?: { id: number; warehouse_name?: string; name?: string }[] }
      | { id: number; warehouse_name?: string; name?: string }[]
    >('/warehouses')
    const d = res
    if (Array.isArray(d)) {
      warehouses.value = d
    } else if (d && typeof d === 'object' && 'list' in d) {
      warehouses.value = d.list || []
    } else {
      warehouses.value = []
    }
  } catch (error) {
    warehouses.value = []
    void error
  }
}

const openCreateDialog = () => {
  formDialogTitle.value = '新建销售订单'
  Object.assign(formData, {
    id: 0,
    customer_id: undefined,
    customer_name: '',
    order_date: new Date(),
    required_date: '',
    contact_person: '',
    contact_phone: '',
    delivery_address: '',
    remark: '',
    items: [
      {
        id: Date.now(),
        product_id: undefined,
        product_name: '',
        product_code: '',
        quantity: 1,
        unit: '米',
        unit_price: 0,
        subtotal: 0,
      },
    ],
    total_amount: 0,
  })
  formDialogVisible.value = true
}

const viewOrder = (row: SalesOrder) => {
  currentOrder.value = row
  viewDialogVisible.value = true
}

const approveOrder = async (row: SalesOrder) => {
  try {
    await ElMessageBox.confirm('确定审批此订单吗？', '确认', { type: 'info' })
    await salesApi.approveOrder(row.id)
    ElMessage.success('审批成功')
    refresh()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const cancelOrder = async (row: SalesOrder) => {
  try {
    await ElMessageBox.confirm('确定取消此订单吗？', '确认', { type: 'warning' })
    await salesApi.cancelOrder(row.id)
    ElMessage.success('取消成功')
    refresh()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as { message?: string }
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const openDeliveryDialog = (row: SalesOrder) => {
  Object.assign(deliveryForm, {
    order_id: row.id,
    order_no: row.order_no,
    customer_name: row.customer_name,
    delivery_date: '',
    warehouse_id: undefined,
    items:
      row.items?.map(item => ({
        product_id: item.product_id,
        product_name: item.product_name,
        quantity: item.quantity,
        delivered_quantity: item.delivered_quantity || 0,
        deliver_quantity: 0,
        unit_price: item.unit_price,
        remarks: '',
      })) || [],
  })
  deliveryDialogVisible.value = true
}

const handleFormSubmit = async (data: never) => {
  submitting.value = true
  try {
    const d = data as unknown as { id: number }
    if (d.id) {
      await salesApi.updateOrder(d.id, data as unknown as Partial<SalesOrder>)
      ElMessage.success('更新成功')
    } else {
      await salesApi.createOrder(data as unknown as Partial<SalesOrder>)
      ElMessage.success('创建成功')
    }
    formDialogVisible.value = false
    refresh()
  } catch (error) {
    const err = error as { message?: string }
    ElMessage.error(err.message || '操作失败')
  } finally {
    submitting.value = false
  }
}

const formDataForChild = computed(() => formData as never)

onMounted(() => {
  refresh()
  fetchCustomers()
  fetchProducts()
  fetchWarehouses()
})
</script>

<style scoped>
.sales-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.page-title {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
.stats-cards {
  margin-bottom: 20px;
}
.stats-item {
  text-align: center;
  padding: 10px 0;
}
.stats-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 8px;
}
.stats-value {
  font-size: 28px;
  font-weight: 700;
  color: #303133;
}
.stats-value.warning {
  color: #e6a23c;
}
.stats-value.success {
  color: #67c23a;
}
.stats-value.highlight {
  color: #f56c6c;
}
.filter-card {
  margin-bottom: 20px;
}
.action-cell {
  display: flex;
  gap: 4px;
  align-items: center;
}
</style>
