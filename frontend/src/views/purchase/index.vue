<!--
  purchase/index.vue - 采购管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-17 P1-3-Batch-2）：
  原 957 行"上帝组件"已拆分为以下独立子组件：
  - tabs/PurchaseStatsCards.vue（采购统计卡片，163 行）
  - tabs/PurchaseOrderFilter.vue（采购订单筛选，89 行）
  - tabs/PurchaseOrderList.vue（采购订单列表 V2Table 版，114 行）

  本主入口承担：页面布局 + 数据加载 + 业务方法 + 3 个对话框（创建/收货/查看）。
  通过 props/emit 通信。
-->
<template>
  <div class="purchase-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">采购管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>采购管理</el-breadcrumb-item>
          <el-breadcrumb-item>采购订单</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
      <div class="header-actions">
        <el-button type="primary" @click="handleCreate">
          <el-icon><Plus /></el-icon>
          新建采购单
        </el-button>
        <el-button @click="handlePrint">
          <el-icon><Printer /></el-icon>
          打印
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>
          导出
        </el-button>
      </div>
    </div>

    <PurchaseStatsCards :stats="stats" />

    <PurchaseOrderFilter
      :query-params="queryParams"
      :suppliers="suppliers"
      @query="handleQuery"
      @reset="handleReset"
    />

    <PurchaseOrderList
      :orders="orders"
      :total="total"
      :loading="loading"
      :query-params="queryParams"
      @view="handleView"
      @update:query-params="(v: PurchaseQuery) => Object.assign(queryParams, v)"
      @query="fetchData"
    />

    <!-- 新建采购单对话框 -->
    <el-dialog v-model="createDialogVisible" title="新建采购单" width="800px">
      <el-form ref="createFormRef" :model="createForm" :rules="createFormRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="供应商" required>
              <el-select
                v-model="createForm.supplier_id"
                placeholder="选择供应商"
                style="width: 100%"
              >
                <el-option
                  v-for="s in suppliers"
                  :key="s.id"
                  :label="s.supplier_name"
                  :value="s.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="订单日期" required>
              <el-date-picker
                v-model="createForm.order_date"
                type="date"
                placeholder="选择日期"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="要求交货日期">
              <el-date-picker
                v-model="createForm.required_date"
                type="date"
                placeholder="选择日期"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="备注">
              <el-input v-model="createForm.remark" placeholder="请输入备注" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="采购明细">
          <div class="items-table">
            <div class="items-header">
              <span class="col-product">产品</span>
              <span class="col-qty">数量</span>
              <span class="col-price">单价</span>
              <span class="col-amount">金额</span>
              <span class="col-action">操作</span>
            </div>
            <div v-for="(item, index) in createForm.items" :key="index" class="items-row">
              <el-select
                v-model="item.product_id"
                placeholder="选择产品"
                class="col-product"
                @change="handleProductSelect(index)"
              >
                <el-option
                  v-for="p in products"
                  :key="p.id"
                  :label="p.product_name"
                  :value="p.id"
                />
              </el-select>
              <el-input-number
                v-model="item.quantity"
                :min="1"
                class="col-qty"
                @change="calculateSubtotal(item)"
              />
              <el-input-number
                v-model="item.unit_price"
                :min="0"
                :precision="2"
                class="col-price"
                @change="calculateSubtotal(item)"
              />
              <el-input-number v-model="item.subtotal" :precision="2" class="col-amount" readonly />
              <el-button
                v-if="createForm.items.length > 1"
                size="small"
                type="danger"
                @click="removeItem(index)"
                >删除</el-button
              >
            </div>
            <el-button type="text" @click="addItem">+ 添加明细</el-button>
          </div>
        </el-form-item>
        <el-form-item label="合计金额">
          <span class="total-amount">¥{{ calculateTotal().toLocaleString() }}</span>
        </el-form-item>
      </el-form>
      <template #footer>
        <span class="dialog-footer">
          <el-button @click="createDialogVisible = false">取消</el-button>
          <el-button type="primary" @click="submitCreate">确定</el-button>
        </span>
      </template>
    </el-dialog>

    <!-- 收货对话框 -->
    <el-dialog v-model="receiveDialogVisible" title="采购收货" width="800px">
      <el-form :model="receiveForm" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="采购单号">
              <el-input v-model="receiveForm.order_no" readonly />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="供应商">
              <el-input v-model="receiveForm.supplier_name" readonly />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="收货日期" required>
              <el-date-picker
                v-model="receiveForm.receive_date"
                type="date"
                placeholder="选择日期"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="仓库" required>
              <el-select
                v-model="receiveForm.warehouse_id"
                placeholder="选择仓库"
                style="width: 100%"
              >
                <el-option
                  v-for="w in warehouses"
                  :key="w.id"
                  :label="w.warehouse_name"
                  :value="w.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="收货明细">
          <el-table :data="receiveForm.items" border style="width: 100%">
            <el-table-column prop="product_name" label="产品" width="150" />
            <el-table-column prop="ordered_quantity" label="订购数量" width="100" />
            <el-table-column prop="received_quantity" label="已收货" width="100" />
            <el-table-column label="本次收货" width="120">
              <template #default="{ row }">
                <el-input-number
                  v-model="row.receive_quantity"
                  :min="0"
                  :max="row.ordered_quantity - row.received_quantity"
                  size="small"
                />
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
          <el-button @click="receiveDialogVisible = false">取消</el-button>
          <el-button type="primary" @click="submitReceive">确定收货</el-button>
        </span>
      </template>
    </el-dialog>

    <!-- 查看采购单对话框 -->
    <el-dialog v-model="viewDialogVisible" title="采购单详情" width="800px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="采购单号">{{ viewData.order_no }}</el-descriptions-item>
        <el-descriptions-item label="供应商">{{ viewData.supplier_name }}</el-descriptions-item>
        <el-descriptions-item label="订单日期">{{ viewData.order_date }}</el-descriptions-item>
        <el-descriptions-item label="要求交货日期">{{
          viewData.required_date
        }}</el-descriptions-item>
        <el-descriptions-item label="订单金额"
          >¥{{ viewData.total_amount?.toLocaleString() }}</el-descriptions-item
        >
        <el-descriptions-item label="已收货金额"
          >¥{{ (viewData.received_amount || 0).toLocaleString() }}</el-descriptions-item
        >
        <el-descriptions-item label="订单状态">
          <el-tag :type="getStatusType(viewData.status)">{{
            getStatusText(viewData.status)
          }}</el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="创建人">{{ viewData.creator_name }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{
          viewData.remarks || '无'
        }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus, Printer, Download } from '@element-plus/icons-vue'
import printJS from 'print-js'
import { purchaseApi, type PurchaseOrder } from '@/api/purchase'
import { supplierApi, type Supplier } from '@/api/supplier'
import { productApi, type Product } from '@/api/product'
import { warehouseApi, type Warehouse } from '@/api/warehouse'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'
import PurchaseStatsCards from './tabs/PurchaseStatsCards.vue'
import PurchaseOrderFilter, { type PurchaseQuery } from './tabs/PurchaseOrderFilter.vue'
import PurchaseOrderList from './tabs/PurchaseOrderList.vue'

const hasLoaded = createLazyLoader()

const loading = ref(false)
const orders = ref<PurchaseOrder[]>([])
const suppliers = ref<Supplier[]>([])
const products = ref<Product[]>([])
const warehouses = ref<Warehouse[]>([])
const total = ref(0)

const stats = ref({
  monthOrders: 0,
  monthAmount: 0,
  pendingReceipt: 0,
  supplierCount: 0,
})

const queryParams = reactive<PurchaseQuery>({
  page: 1,
  page_size: 20,
  keyword: '',
  supplier_id: undefined,
  status: '',
})

// 对话框状态
const createDialogVisible = ref(false)
const createFormRef = ref()
const createFormRules = {
  supplier_id: [{ required: true, message: '请选择供应商', trigger: 'change' }],
  order_date: [{ required: true, message: '请选择订单日期', trigger: 'change' }],
}
const createForm = ref({
  supplier_id: undefined as number | undefined,
  order_date: new Date().toISOString().split('T')[0],
  required_date: '',
  remark: '',
  items: [{ product_id: undefined as number | undefined, quantity: 1, unit_price: 0, subtotal: 0 }],
})

const receiveDialogVisible = ref(false)
const receiveForm = ref({
  order_id: 0,
  order_no: '',
  supplier_name: '',
  receive_date: new Date().toISOString().split('T')[0],
  warehouse_id: undefined as number | undefined,
  items: [] as any[],
})

const viewDialogVisible = ref(false)
const viewData = ref<any>({})

const getStatusType = (status: string) => {
  const typeMap: Record<string, any> = {
    pending: 'warning',
    approved: 'primary',
    partial: 'info',
    completed: 'success',
    cancelled: 'danger',
  }
  return typeMap[status] || 'info'
}

const getStatusText = (status: string) => {
  const textMap: Record<string, string> = {
    pending: '待审批',
    approved: '已审批',
    partial: '部分收货',
    completed: '已完成',
    cancelled: '已取消',
  }
  return textMap[status] || status
}

const fetchData = async () => {
  loading.value = true
  try {
    const res = await purchaseApi.getOrderList(queryParams)
    orders.value = res.data!.list || []
    total.value = res.data?.total || 0
    stats.value.monthOrders = total.value
    stats.value.monthAmount = orders.value.reduce((sum, o) => sum + (o.total_amount || 0), 0)
    stats.value.pendingReceipt = orders.value.filter(o => o.status === 'approved').length
  } catch (error: any) {
    ElMessage.error(error.message || '获取采购单列表失败')
    orders.value = []
    total.value = 0
  } finally {
    loading.value = false
  }
}

const fetchSuppliers = async () => {
  try {
    const res = await supplierApi.list({ page_size: 1000 })
    suppliers.value = res.data!.list || []
    stats.value.supplierCount = suppliers.value.length
  } catch (error) {
    logger.error('获取供应商列表失败:', error)
  }
}

const fetchProducts = async () => {
  try {
    const res = await productApi.list({ page_size: 1000 })
    products.value = res.data!.list || []
  } catch (error) {
    logger.error('获取产品列表失败:', error)
  }
}

const fetchWarehouses = async () => {
  try {
    const res = await warehouseApi.list({ page_size: 1000 })
    warehouses.value = res.data!.list || []
  } catch (error) {
    logger.error('获取仓库列表失败:', error)
  }
}

const handleQuery = () => {
  queryParams.page = 1
  fetchData()
}
const handleReset = () => {
  queryParams.keyword = ''
  queryParams.supplier_id = undefined
  queryParams.status = ''
  handleQuery()
}

const handleCreate = () => {
  createForm.value = {
    supplier_id: undefined,
    order_date: new Date().toISOString().split('T')[0],
    required_date: '',
    remark: '',
    items: [{ product_id: undefined, quantity: 1, unit_price: 0, subtotal: 0 }],
  }
  createDialogVisible.value = true
}

const handleView = async (row: PurchaseOrder) => {
  try {
    const res = await purchaseApi.getOrderById(row.id)
    viewData.value = res.data || row
  } catch {
    viewData.value = row
  }
  viewDialogVisible.value = true
}

const handleApprove = async (row: PurchaseOrder) => {
  try {
    await ElMessageBox.confirm(`确定审批通过采购单 ${row.order_no} 吗？`, '审批确认', {
      type: 'success',
    })
    await purchaseApi.approveOrder(row.id)
    ElMessage.success(`采购单 ${row.order_no} 审批成功`)
    fetchData()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '审批失败')
    }
  }
}

const handleReceive = (row: PurchaseOrder) => {
  receiveForm.value = {
    order_id: row.id,
    order_no: row.order_no,
    supplier_name: row.supplier_name,
    receive_date: new Date().toISOString().split('T')[0],
    warehouse_id: undefined,
    items: (row.items || []).map((item: any) => ({
      ...item,
      receive_quantity: 0,
      remarks: '',
    })),
  }
  receiveDialogVisible.value = true
}

const handlePrint = () => {
  const printData = orders.value.map((item: any, index: number) => ({
    序号: index + 1,
    订单号: item.order_no,
    供应商: item.supplier_name,
    金额: `¥${item.total_amount}`,
    状态: getStatusText(item.status),
    创建时间: item.created_at,
  }))
  printJS({
    printable: printData,
    properties: Object.keys(printData[0] || {}),
    type: 'table' as any,
    header: '采购订单列表',
    style: 'padding: 20px; font-size: 14px;',
    headerStyle: 'font-size: 18px; font-weight: bold; margin-bottom: 20px;',
    gridHeaderStyle: 'font-weight: bold; background-color: #f5f7fa;',
    gridStyle: 'border-collapse: collapse; width: 100%;',
  })
}

const handleExport = () => {
  const csvContent = [
    ['订单号', '供应商', '金额', '状态', '创建时间'],
    ...orders.value.map((item: any) => [
      item.order_no,
      item.supplier_name,
      item.total_amount,
      getStatusText(item.status),
      item.created_at,
    ]),
  ]
    .map((row: any[]) => row.map((cell: any) => `"${cell}"`).join(','))
    .join('\n')
  const blob = new Blob([csvContent], { type: 'text/csv;charset=utf-8;' })
  const link = document.createElement('a')
  link.href = URL.createObjectURL(blob)
  link.download = `采购订单_${new Date().toISOString().split('T')[0]}.csv`
  link.click()
  ElMessage.success('导出成功')
}

const addItem = () => {
  createForm.value.items.push({ product_id: undefined, quantity: 1, unit_price: 0, subtotal: 0 })
}
const removeItem = (index: number) => {
  if (createForm.value.items.length > 1) {
    createForm.value.items.splice(index, 1)
  }
}
const handleProductSelect = (index: number) => {
  const product = products.value.find(p => p.id === createForm.value.items[index].product_id)
  if (product) {
    createForm.value.items[index].unit_price = product.price || 0
    calculateSubtotal(createForm.value.items[index])
  }
}
const calculateSubtotal = (item: any) => {
  item.subtotal = (item.quantity || 0) * (item.unit_price || 0)
}
const calculateTotal = () => {
  return createForm.value.items.reduce((sum: number, item: any) => sum + (item.subtotal || 0), 0)
}

const submitCreate = async () => {
  try {
    await createFormRef.value?.validate()
  } catch {
    return
  }
  const validItems = createForm.value.items.filter(item => item.product_id && item.quantity > 0)
  if (validItems.length === 0) {
    ElMessage.warning('请至少添加一条有效的采购明细')
    return
  }
  try {
    await purchaseApi.createOrder({
      ...createForm.value,
      items: validItems.map(item => ({
        id: 0,
        product_id: item.product_id!,
        product_name: '',
        product_code: '',
        quantity: item.quantity,
        unit_price: item.unit_price,
        subtotal: item.subtotal,
      })),
      total_amount: calculateTotal(),
    })
    ElMessage.success('采购单创建成功')
    createDialogVisible.value = false
    fetchData()
  } catch (error: any) {
    ElMessage.error(error.message || '创建失败')
  }
}

const submitReceive = async () => {
  if (!receiveForm.value.warehouse_id) {
    ElMessage.warning('请选择收货仓库')
    return
  }
  const validItems = receiveForm.value.items.filter(item => item.receive_quantity > 0)
  if (validItems.length === 0) {
    ElMessage.warning('请填写至少一项收货数量')
    return
  }
  try {
    await purchaseApi.createReceipt({
      order_id: receiveForm.value.order_id,
      receipt_date: receiveForm.value.receive_date,
      warehouse_id: receiveForm.value.warehouse_id,
      items: validItems.map(item => ({
        product_id: item.product_id,
        received_quantity: item.receive_quantity,
        remark: item.remarks,
      })),
    })
    ElMessage.success('收货成功')
    receiveDialogVisible.value = false
    fetchData()
  } catch (error: any) {
    ElMessage.error(error.message || '收货失败')
  }
}

const initPage = () => {
  loadIfNot('fetchData', fetchData, hasLoaded)
  loadIfNot('fetchSuppliers', fetchSuppliers, hasLoaded)
  loadIfNot('fetchProducts', fetchProducts, hasLoaded)
  loadIfNot('fetchWarehouses', fetchWarehouses, hasLoaded)
}

onMounted(() => {
  initPage()
})
</script>

<style scoped>
.purchase-page {
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

.items-table {
  width: 100%;
}

.items-header {
  display: flex;
  gap: 8px;
  padding: 8px 0;
  font-weight: 600;
  color: #303133;
  border-bottom: 1px solid #ebeef5;
}

.items-row {
  display: flex;
  gap: 8px;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid #f0f0f0;
}

.col-product {
  flex: 1;
  min-width: 200px;
}

.col-qty,
.col-price,
.col-amount {
  width: 110px;
}

.total-amount {
  font-size: 18px;
  font-weight: 700;
  color: #f56c6c;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
