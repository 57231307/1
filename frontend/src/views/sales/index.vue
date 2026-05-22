<template>
  <div class="sales-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">销售管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>销售管理</el-breadcrumb-item>
          <el-breadcrumb-item>销售订单</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建订单
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon order-icon">
              <el-icon><Document /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">本月订单</div>
              <div class="stat-value">{{ stats.monthOrders }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card highlight">
          <div class="stat-content">
            <div class="stat-icon amount-icon">
              <el-icon><Money /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">本月销售额</div>
              <div class="stat-value">{{ formatCurrency(stats.monthAmount) }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card warning">
          <div class="stat-content">
            <div class="stat-icon pending-icon">
              <el-icon><Clock /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">待处理</div>
              <div class="stat-value">{{ stats.pendingOrders }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon deliver-icon">
              <el-icon><Van /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">待发货</div>
              <div class="stat-value">{{ stats.pendingDeliver }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="关键词">
          <el-input v-model="queryParams.keyword" placeholder="订单号/客户名" clearable @clear="handleQuery" />
        </el-form-item>
        <el-form-item label="客户">
          <el-select v-model="queryParams.customer_id" placeholder="选择客户" clearable @change="handleQuery">
            <el-option v-for="c in customers" :key="c.id" :label="c.customer_name" :value="c.id" />
          </el-select>
        </el-form-item>
        <el-form-item label="订单状态">
          <el-select v-model="queryParams.status" placeholder="选择状态" clearable @change="handleQuery">
            <el-option label="待审批" value="pending" />
            <el-option label="已审批" value="approved" />
            <el-option label="已发货" value="shipped" />
            <el-option label="已完成" value="completed" />
            <el-option label="已取消" value="cancelled" />
          </el-select>
        </el-form-item>
        <el-form-item label="日期范围">
          <el-date-picker
            v-model="dateRange"
            type="daterange"
            range-separator="至"
            start-placeholder="开始日期"
            end-placeholder="结束日期"
            @change="handleDateChange"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">
            <el-icon><Search /></el-icon>
            查询
          </el-button>
          <el-button @click="handleReset">
            <el-icon><Refresh /></el-icon>
            重置
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="orders" stripe @row-click="handleRowClick">
        <el-table-column prop="order_no" label="订单号" width="160" fixed>
          <template #default="{ row }">
            <el-link type="primary" @click.stop="handleView(row)">{{ row.order_no }}</el-link>
          </template>
        </el-table-column>
        <el-table-column prop="customer_name" label="客户名称" width="180" fixed />
        <el-table-column prop="order_date" label="订单日期" width="120" />
        <el-table-column prop="required_date" label="要求交货日期" width="120" />
        <el-table-column prop="total_amount" label="订单金额" width="120" align="right">
          <template #default="{ row }">
            <span class="amount">¥{{ row.total_amount.toLocaleString() }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="contact_person" label="联系人" width="100" />
        <el-table-column prop="contact_phone" label="联系电话" width="120" />
        <el-table-column prop="status" label="订单状态" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getStatusText(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="creator_name" label="创建人" width="100" />
        <el-table-column prop="created_at" label="创建时间" width="160" />
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click.stop="handleView(row)">详情</el-button>
            <el-button v-if="row.status === 'pending'" type="success" link size="small" @click.stop="handleApprove(row)">审批</el-button>
            <el-button v-if="row.status === 'approved'" type="warning" link size="small" @click.stop="handleDeliver(row)">发货</el-button>
            <el-button v-if="row.status === 'pending'" type="danger" link size="small" @click.stop="handleCancel(row)">取消</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="queryParams.page"
          v-model:page-size="queryParams.page_size"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleQuery"
          @current-change="handleQuery"
        />
      </div>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="dialogTitle" width="900px" destroy-on-close>
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <el-divider content-position="left">基本信息</el-divider>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="客户" prop="customer_id">
              <el-select v-model="formData.customer_id" placeholder="选择客户" style="width: 100%" @change="handleCustomerChange">
                <el-option v-for="c in customers" :key="c.id" :label="c.customer_name" :value="c.id" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="订单日期" prop="order_date">
              <el-date-picker v-model="formData.order_date" type="date" placeholder="选择日期" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="要求交货日期" prop="required_date">
              <el-date-picker v-model="formData.required_date" type="date" placeholder="选择日期" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="联系人" prop="contact_person">
              <el-input v-model="formData.contact_person" placeholder="联系人姓名" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="联系电话" prop="contact_phone">
          <el-input v-model="formData.contact_phone" placeholder="联系电话" style="width: 50%" />
        </el-form-item>
        <el-form-item label="收货地址" prop="delivery_address">
          <el-input v-model="formData.delivery_address" type="textarea" :rows="2" placeholder="详细收货地址" />
        </el-form-item>

        <el-divider content-position="left">订单明细</el-divider>
        <div class="order-items">
          <el-table :data="formData.items" border style="width: 100%">
            <el-table-column label="产品" width="200">
              <template #default="{ row, $index }">
                <el-select v-model="row.product_id" placeholder="选择产品" @change="handleProductSelect($index)">
                  <el-option v-for="p in products" :key="p.id" :label="p.product_name" :value="p.id" />
                </el-select>
              </template>
            </el-table-column>
            <el-table-column prop="quantity" label="数量" width="120">
              <template #default="{ row }">
                <el-input-number v-model="row.quantity" :min="1" size="small" @change="calculateSubtotal(row)" />
              </template>
            </el-table-column>
            <el-table-column prop="unit" label="单位" width="80" />
            <el-table-column prop="unit_price" label="单价" width="120">
              <template #default="{ row }">
                <el-input-number v-model="row.unit_price" :min="0" :precision="2" size="small" @change="calculateSubtotal(row)" />
              </template>
            </el-table-column>
            <el-table-column prop="subtotal" label="小计" width="120">
              <template #default="{ row }">
                <span class="amount">¥{{ (row.subtotal || 0).toLocaleString() }}</span>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="80">
              <template #default="{ $index }">
                <el-button type="danger" link size="small" @click="removeItem($index)">删除</el-button>
              </template>
            </el-table-column>
          </el-table>
          <el-button type="primary" plain size="small" @click="addItem" style="margin-top: 10px;">
            <el-icon><Plus /></el-icon>
            添加明细
          </el-button>
        </div>

        <el-divider content-position="left">其他信息</el-divider>
        <el-form-item label="备注">
          <el-input v-model="formData.remark" type="textarea" :rows="3" placeholder="订单备注信息" />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="8">
            <el-form-item label="订单总额">
              <div class="total-amount">¥{{ calculateTotal().toLocaleString() }}</div>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="税额">
              <div class="total-amount">¥{{ (calculateTotal() * 0.13).toLocaleString() }}</div>
            </el-form-item>
          </el-col>
          <el-col :span="8">
            <el-form-item label="价税合计">
              <div class="total-amount highlight">¥{{ (calculateTotal() * 1.13).toLocaleString() }}</div>
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="viewDialogVisible" title="订单详情" width="1000px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="订单号">{{ currentOrder?.order_no }}</el-descriptions-item>
        <el-descriptions-item label="订单状态">
          <el-tag :type="getStatusType(currentOrder?.status)" size="small">
            {{ getStatusText(currentOrder?.status) }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="客户名称">{{ currentOrder?.customer_name }}</el-descriptions-item>
        <el-descriptions-item label="订单日期">{{ currentOrder?.order_date }}</el-descriptions-item>
        <el-descriptions-item label="要求交货日期">{{ currentOrder?.required_date }}</el-descriptions-item>
        <el-descriptions-item label="联系人">{{ currentOrder?.contact_person }}</el-descriptions-item>
        <el-descriptions-item label="联系电话">{{ currentOrder?.contact_phone }}</el-descriptions-item>
        <el-descriptions-item label="收货地址" :span="2">{{ currentOrder?.delivery_address }}</el-descriptions-item>
        <el-descriptions-item label="订单金额">¥{{ currentOrder?.total_amount?.toLocaleString() }}</el-descriptions-item>
        <el-descriptions-item label="创建人">{{ currentOrder?.creator_name }}</el-descriptions-item>
      </el-descriptions>

      <el-divider content-position="left">订单明细</el-divider>
      <el-table :data="currentOrder?.items" border>
        <el-table-column prop="product_name" label="产品名称" />
        <el-table-column prop="product_code" label="产品编码" width="120" />
        <el-table-column prop="quantity" label="数量" width="80" align="right" />
        <el-table-column prop="unit" label="单位" width="60" />
        <el-table-column prop="unit_price" label="单价" width="100" align="right">
          <template #default="{ row }">
            ¥{{ row.unit_price.toLocaleString() }}
          </template>
        </el-table-column>
        <el-table-column prop="subtotal" label="小计" width="120" align="right">
          <template #default="{ row }">
            <strong>¥{{ row.subtotal.toLocaleString() }}</strong>
          </template>
        </el-table-column>
      </el-table>
    </el-dialog>

    <!-- 发货对话框 -->
    <el-dialog v-model="deliveryDialogVisible" title="销售发货" width="800px">
      <el-form :model="deliveryForm" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="销售单号">
              <el-input v-model="deliveryForm.order_no" readonly />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="客户">
              <el-input v-model="deliveryForm.customer_name" readonly />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="发货日期" required>
              <el-date-picker v-model="deliveryForm.delivery_date" type="date" placeholder="选择日期" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="仓库" required>
              <el-select v-model="deliveryForm.warehouse_id" placeholder="选择仓库" style="width: 100%">
                <el-option v-for="w in warehouses" :key="w.id" :label="w.name" :value="w.id" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="发货明细">
          <el-table :data="deliveryForm.items" border style="width: 100%">
            <el-table-column prop="product_name" label="产品" width="150" />
            <el-table-column prop="quantity" label="订单数量" width="100" />
            <el-table-column prop="delivered_quantity" label="已发货" width="100" />
            <el-table-column label="本次发货" width="120">
              <template #default="{ row }">
                <el-input-number v-model="row.deliver_quantity" :min="0" :max="row.quantity - (row.delivered_quantity || 0)" size="small" />
              </template>
            </el-table-column>
            <el-table-column prop="unit_price" label="单价" width="100" />
            <el-table-column label="备注" min-width="150">
              <template #default="{ row }">
                <el-input v-model="row.remarks" size="small" placeholder="备注" />
              </template>
            </el-table-column>
          </el-table>
        </el-form-item>
      </el-form>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="deliveryDialogVisible = false">取消</el-button>
          <el-button type="primary" @click="submitDelivery">确定发货</el-button>
        </span>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  Plus, Download, Search, Refresh, Document, Money, Clock, Van
} from '@element-plus/icons-vue'
import { salesApi, type SalesOrder } from '@/api/sales'
import { customerApi, type Customer } from '@/api/customer'
import { productApi, type Product } from '@/api/product'
import { warehouseApi } from '@/api/warehouse'

const loading = ref(false)
const orders = ref<SalesOrder[]>([])
const customers = ref<Customer[]>([])
const products = ref<Product[]>([])
const total = ref(0)
const dialogVisible = ref(false)
const viewDialogVisible = ref(false)
const dialogTitle = ref('')
const currentOrder = ref<SalesOrder | null>(null)
const formRef = ref()
const isEdit = ref(false)

const stats = ref({
  monthOrders: 89,
  monthAmount: 1250000,
  pendingOrders: 23,
  pendingDeliver: 15
})

const dateRange = ref<[Date, Date] | null>(null)

const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  customer_id: undefined as number | undefined,
  status: '',
  order_date_from: '',
  order_date_to: ''
})

const formData = reactive<any>({
  customer_id: undefined,
  customer_name: '',
  order_date: new Date(),
  required_date: '',
  contact_person: '',
  contact_phone: '',
  delivery_address: '',
  remark: '',
  items: []
})

const formRules = {
  customer_id: [{ required: true, message: '请选择客户', trigger: 'change' }],
  order_date: [{ required: true, message: '请选择订单日期', trigger: 'change' }]
}

const formatCurrency = (amount: number) => {
  return new Intl.NumberFormat('zh-CN', {
    style: 'currency',
    currency: 'CNY',
    minimumFractionDigits: 0
  }).format(amount)
}

const getStatusType = (status: string | undefined) => {
  const typeMap: Record<string, any> = {
    pending: 'warning',
    approved: 'primary',
    shipped: 'success',
    completed: 'info',
    cancelled: 'danger'
  }
  return typeMap[status || ''] || 'info'
}

const getStatusText = (status: string | undefined) => {
  const textMap: Record<string, string> = {
    pending: '待审批',
    approved: '已审批',
    shipped: '已发货',
    completed: '已完成',
    cancelled: '已取消'
  }
  return textMap[status || ''] || status || ''
}

const fetchData = async () => {
  loading.value = true
  try {
    const res = await salesApi.getOrderList(queryParams)
    orders.value = res.data!.list || []
    total.value = res.data?.total || 0
  } catch (error: any) {
    ElMessage.error(error.message || '获取订单列表失败')
    orders.value = []
    total.value = 0
  } finally {
    loading.value = false
  }
}

const fetchCustomers = async () => {
  try {
    const res = await customerApi.list({ page_size: 1000 })
    customers.value = res.data!.list || []
  } catch (error) {
    console.error('获取客户列表失败:', error)
  }
}

const fetchProducts = async () => {
  try {
    const res = await productApi.list({ page_size: 1000 })
    products.value = res.data!.list || []
  } catch (error) {
    console.error('获取产品列表失败:', error)
  }
}

const handleQuery = () => {
  queryParams.page = 1
  fetchData()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.customer_id = undefined
  queryParams.status = ''
  dateRange.value = null
  queryParams.order_date_from = ''
  queryParams.order_date_to = ''
  handleQuery()
}

const handleDateChange = () => {
  if (dateRange.value) {
    queryParams.order_date_from = dateRange.value[0].toISOString().split('T')[0]
    queryParams.order_date_to = dateRange.value[1].toISOString().split('T')[0]
  } else {
    queryParams.order_date_from = ''
    queryParams.order_date_to = ''
  }
  handleQuery()
}

const handleCreate = () => {
  isEdit.value = false
  dialogTitle.value = '新建销售订单'
  Object.assign(formData, {
    customer_id: undefined,
    customer_name: '',
    order_date: new Date(),
    required_date: '',
    contact_person: '',
    contact_phone: '',
    delivery_address: '',
    remark: '',
    items: [{ id: Date.now(), product_id: undefined, product_name: '', product_code: '', quantity: 1, unit: '米', unit_price: 0, subtotal: 0 }]
  })
  dialogVisible.value = true
}

const handleView = (row: any) => {
  currentOrder.value = row
  viewDialogVisible.value = true
}

const handleApprove = async (row: SalesOrder) => {
  try {
    await ElMessageBox.confirm(`确定审批通过订单 ${row.order_no} 吗？`, '审批确认', { type: 'success' })
    await salesApi.approveOrder(row.id)
    ElMessage.success(`订单 ${row.order_no} 审批成功`)
    fetchData()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '审批失败')
    }
  }
}

const handleDeliver = (row: SalesOrder) => {
  deliveryForm.value = {
    order_id: row.id,
    order_no: row.order_no,
    customer_name: row.customer_name,
    delivery_date: new Date().toISOString().split('T')[0],
    warehouse_id: undefined,
    items: (row.items || []).map((item: any) => ({
      ...item,
      deliver_quantity: 0,
      remarks: ''
    }))
  }
  deliveryDialogVisible.value = true
}

const handleCancel = async (row: SalesOrder) => {
  try {
    await ElMessageBox.confirm(`确定取消订单 ${row.order_no} 吗？`, '取消确认', { type: 'warning' })
    await salesApi.cancelOrder(row.id)
    ElMessage.success(`订单 ${row.order_no} 已取消`)
    fetchData()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '取消失败')
    }
  }
}

const handleRowClick = (row: any) => {
  handleView(row)
}

const handleCustomerChange = (customerId: number) => {
  const customer = customers.value.find(c => c.id === customerId)
  if (customer) {
    formData.customer_name = customer.customer_name
  }
}

const handleProductSelect = (index: number) => {
  const product = products.value.find(p => p.id === formData.items[index].product_id)
  if (product) {
    formData.items[index].product_name = product.product_name
    formData.items[index].product_code = product.product_code
    formData.items[index].unit_price = product.price || 0
    calculateSubtotal(formData.items[index])
  }
}

const calculateSubtotal = (item: any) => {
  item.subtotal = (item.quantity || 0) * (item.unit_price || 0)
}

const calculateTotal = () => {
  return formData.items.reduce((sum: number, item: any) => sum + (item.subtotal || 0), 0)
}

const addItem = () => {
  formData.items.push({
    id: Date.now(),
    product_id: undefined,
    product_name: '',
    product_code: '',
    quantity: 1,
    unit: '米',
    unit_price: 0,
    subtotal: 0
  })
}

const removeItem = (index: number) => {
  if (formData.items.length > 1) {
    formData.items.splice(index, 1)
  } else {
    ElMessage.warning('至少保留一条明细')
  }
}

const handleSubmit = async () => {
  try {
    await formRef.value.validate()
    formData.total_amount = calculateTotal()
    
    if (isEdit.value && currentOrder.value) {
      await salesApi.updateOrder(currentOrder.value.id, formData)
      ElMessage.success('订单更新成功')
    } else {
      await salesApi.createOrder(formData)
      ElMessage.success('订单创建成功')
    }
    
    dialogVisible.value = false
    fetchData()
  } catch (error: any) {
    if (error.message) {
      ElMessage.error(error.message || '操作失败')
    }
  }
}

const handleExport = () => {
  const csvContent = [
    ['订单号', '客户', '订单日期', '金额', '状态', '创建时间'],
    ...orders.value.map((item: any) => [item.order_no, item.customer_name, item.order_date, item.total_amount, getStatusText(item.status), item.created_at])
  ].map((row: any[]) => row.map((cell: any) => `"${cell}"`).join(',')).join('\n')
  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `销售订单_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
}

// 发货相关
const deliveryDialogVisible = ref(false)
const deliveryForm = ref({
  order_id: 0,
  order_no: '',
  customer_name: '',
  delivery_date: new Date().toISOString().split('T')[0],
  warehouse_id: undefined as number | undefined,
  items: [] as any[]
})

const warehouses = ref<any[]>([])

const fetchWarehouses = async () => {
  try {
    const res = await warehouseApi.list({ page_size: 1000 })
    warehouses.value = res.data!.list || []
  } catch (error) {
    console.error('获取仓库列表失败:', error)
  }
}

const submitDelivery = async () => {
  if (!deliveryForm.value.warehouse_id) {
    ElMessage.warning('请选择发货仓库')
    return
  }
  const validItems = deliveryForm.value.items.filter(item => item.deliver_quantity > 0)
  if (validItems.length === 0) {
    ElMessage.warning('请填写至少一项发货数量')
    return
  }
  try {
    await salesApi.createDelivery(deliveryForm.value.order_id, {
      delivery_date: deliveryForm.value.delivery_date,
      warehouse_id: deliveryForm.value.warehouse_id,
      items: validItems.map(item => ({
        product_id: item.product_id,
        quantity: item.deliver_quantity,
        remarks: item.remarks
      }))
    })
    ElMessage.success('发货成功')
    deliveryDialogVisible.value = false
    fetchData()
  } catch (error: any) {
    ElMessage.error(error.message || '发货失败')
  }
}

onMounted(() => {
  fetchData()
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
  align-items: flex-start;
  margin-bottom: 24px;
}

.header-left .page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}

.header-actions {
  display: flex;
  gap: 12px;
}

.stats-row {
  margin-bottom: 20px;
}

.stat-card {
  border-radius: 12px;
  transition: all 0.3s ease;
}

.stat-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}

.stat-card.highlight {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}

.stat-card.highlight .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.highlight .stat-label,
.stat-card.highlight .stat-value {
  color: white;
}

.stat-card.warning {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}

.stat-card.warning .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-card.warning .stat-label,
.stat-card.warning .stat-value {
  color: white;
}

.stat-content {
  display: flex;
  align-items: center;
  gap: 16px;
}

.stat-icon {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 28px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.stat-icon.order-icon {
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
}

.stat-icon.amount-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-icon.pending-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}

.stat-icon.deliver-icon {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}

.stat-info {
  flex: 1;
}

.stat-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 4px;
}

.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}

.filter-card {
  margin-bottom: 20px;
}

.table-card {
  margin-bottom: 20px;
}

.pagination-wrapper {
  margin-top: 20px;
  display: flex;
  justify-content: flex-end;
}

.amount {
  font-weight: 600;
  color: #f56c6c;
}

.total-amount {
  font-size: 20px;
  font-weight: 700;
  color: #303133;
}

.total-amount.highlight {
  color: #f56c6c;
}

.order-items {
  margin-bottom: 20px;
}

:deep(.el-card__header) {
  padding: 16px 20px;
  border-bottom: 1px solid #ebeef5;
}

:deep(.el-card__body) {
  padding: 20px;
}
</style>
