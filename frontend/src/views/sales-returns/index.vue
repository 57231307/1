<template>
  <div class="sales-returns-page">
    <div class="header">
      <h2>销售退货管理</h2>
      <el-button type="primary" @click="handleCreate">新建退货单</el-button>
    </div>

    <el-table v-loading="loading" :data="returnList" border>
      <el-table-column prop="returnNo" label="退货单号" />
      <el-table-column prop="salesOrderNo" label="销售订单号" />
      <el-table-column prop="customerName" label="客户名称" />
      <el-table-column prop="returnDate" label="退货日期" />
      <el-table-column prop="totalAmount" label="退货金额" />
      <el-table-column prop="status" label="状态">
        <template #default="{ row }">
          <el-tag :type="getStatusType(row.status)">
            {{ getStatusLabel(row.status) }}
          </el-tag>
        </template>
      </el-table-column>
      <el-table-column label="操作" width="250">
        <template #default="{ row }">
          <el-button size="small" @click="handleView(row as any)">详情</el-button>
          <el-button size="small" @click="handleEdit(row as any)">编辑</el-button>
          <el-button
            v-if="row.status === 'PENDING'"
            size="small"
            type="primary"
            @click="handleApprove(row as any)"
            >审核</el-button
          >
        </template>
      </el-table-column>
    </el-table>

    <!-- 详情对话框 -->
    <el-dialog v-model="viewDialogVisible" title="退货单详情" width="800px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="退货单号">{{ currentReturn.returnNo }}</el-descriptions-item>
        <el-descriptions-item label="销售订单号">{{
          currentReturn.salesOrderNo
        }}</el-descriptions-item>
        <el-descriptions-item label="客户名称">{{
          currentReturn.customerName
        }}</el-descriptions-item>
        <el-descriptions-item label="退货日期">{{ currentReturn.returnDate }}</el-descriptions-item>
        <el-descriptions-item label="退货金额"
          >¥{{ currentReturn.totalAmount }}</el-descriptions-item
        >
        <el-descriptions-item label="状态">
          <el-tag :type="getStatusType(currentReturn.status)">
            {{ getStatusLabel(currentReturn.status) }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="退货原因" :span="2">{{
          currentReturn.reason
        }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{
          currentReturn.remarks
        }}</el-descriptions-item>
      </el-descriptions>

      <div style="margin-top: 20px">
        <h4>退货明细</h4>
        <el-table :data="currentReturn.items || []" border size="small">
          <el-table-column prop="productName" label="产品名称" />
          <el-table-column prop="productCode" label="产品编码" />
          <el-table-column prop="quantity" label="退货数量" />
          <el-table-column prop="unitPrice" label="单价" />
          <el-table-column prop="amount" label="金额" />
          <el-table-column prop="reason" label="退货原因" />
        </el-table>
      </div>
    </el-dialog>

    <!-- 新建/编辑对话框 -->
    <el-dialog
      v-model="editDialogVisible"
      :title="dialogMode === 'create' ? '新建退货单' : '编辑退货单'"
      width="900px"
      @close="handleEditDialogClose"
    >
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="120px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="销售订单号" prop="salesOrderId">
              <el-select
                v-model="formData.salesOrderId"
                placeholder="请选择销售订单"
                style="width: 100%"
                filterable
                @change="handleSalesOrderChange"
              >
                <el-option
                  v-for="order in salesOrderList"
                  :key="order.id"
                  :label="order.order_no"
                  :value="order.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="客户" prop="customerId">
              <el-select
                v-model="formData.customerId"
                placeholder="请选择客户"
                style="width: 100%"
                filterable
              >
                <el-option
                  v-for="customer in customerList"
                  :key="customer.id"
                  :label="customer.customer_name"
                  :value="customer.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="退货日期" prop="returnDate">
              <el-date-picker
                v-model="formData.returnDate"
                type="date"
                placeholder="请选择退货日期"
                style="width: 100%"
                format="YYYY-MM-DD"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="退货原因" prop="reason">
              <el-select v-model="formData.reason" placeholder="请选择退货原因" style="width: 100%">
                <el-option label="质量问题" value="quality" />
                <el-option label="数量不符" value="quantity" />
                <el-option label="规格不符" value="specification" />
                <el-option label="包装破损" value="packaging" />
                <el-option label="其他" value="other" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>

        <el-row :gutter="20">
          <el-col :span="24">
            <el-form-item label="备注" prop="remarks">
              <el-input
                v-model="formData.remarks"
                type="textarea"
                :rows="3"
                placeholder="请输入备注"
              />
            </el-form-item>
          </el-col>
        </el-row>

        <el-divider />

        <el-form-item label="退货明细">
          <el-button type="primary" size="small" style="margin-bottom: 10px" @click="handleAddItem">
            添加明细
          </el-button>
          <el-table :data="formData.items" border style="width: 100%">
            <el-table-column label="产品名称" width="200">
              <template #default="{ row }">
                <el-select
                  v-model="row.productId"
                  placeholder="选择产品"
                  style="width: 100%"
                  filterable
                >
                  <el-option
                    v-for="product in productList"
                    :key="product.id"
                    :label="product.product_name"
                    :value="product.id"
                  />
                </el-select>
              </template>
            </el-table-column>
            <el-table-column label="数量" width="120">
              <template #default="{ row }">
                <el-input-number
                  v-model="row.quantity"
                  :min="1"
                  :precision="2"
                  style="width: 100%"
                  @change="calculateTotal"
                />
              </template>
            </el-table-column>
            <el-table-column label="单价" width="120">
              <template #default="{ row }">
                <el-input-number
                  v-model="row.unitPrice"
                  :min="0"
                  :precision="2"
                  style="width: 100%"
                  @change="calculateTotal"
                />
              </template>
            </el-table-column>
            <el-table-column label="金额" width="120">
              <template #default="{ row }">
                {{ (row.quantity * row.unitPrice).toFixed(2) }}
              </template>
            </el-table-column>
            <el-table-column label="退货原因" width="150">
              <template #default="{ row }">
                <el-input v-model="row.reason" placeholder="原因" size="small" />
              </template>
            </el-table-column>
            <el-table-column label="操作" width="80">
              <template #default="{ $index }">
                <el-button type="danger" size="small" @click="handleRemoveItem($index)"
                  >删除</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-form-item>

        <el-row :gutter="20">
          <el-col :span="12" :offset="12">
            <el-form-item label="退货总金额">
              <el-input-number
                v-model="formData.totalAmount"
                :precision="2"
                :disabled="true"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>

      <template #footer>
        <el-button @click="editDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit"> 确定 </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, reactive } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import type { FormInstance, FormRules } from 'element-plus'
import { salesReturnApi } from '@/api/sales-return'
import { salesApi } from '@/api/sales'
import { listCustomers } from '@/api/customer'
import { productApi } from '@/api/product'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { logger } from '@/utils/logger'

const loading = ref(false)
const submitLoading = ref(false)
const viewDialogVisible = ref(false)
const editDialogVisible = ref(false)
const dialogMode = ref<'create' | 'edit'>('create')
const formRef = ref<FormInstance>()
const returnList = ref<any[]>([])
const salesOrderList = ref<any[]>([])
const customerList = ref<any[]>([])
const productList = ref<any[]>([])
const currentReturn = ref<any>({ items: [] })

const formData = reactive<any>({
  id: null,
  salesOrderId: null,
  salesOrderNo: '',
  customerId: null,
  customerName: '',
  returnDate: '',
  reason: '',
  remarks: '',
  items: [] as any[],
  totalAmount: 0,
  status: 'PENDING',
})

const formRules: FormRules = {
  salesOrderId: [{ required: true, message: '请选择销售订单', trigger: 'change' }],
  customerId: [{ required: true, message: '请选择客户', trigger: 'change' }],
  returnDate: [{ required: true, message: '请选择退货日期', trigger: 'change' }],
  reason: [{ required: true, message: '请选择退货原因', trigger: 'change' }],
}

const getStatusType = (status: string) => {
  const types: Record<string, any> = {
    PENDING: 'warning',
    APPROVED: 'success',
    REJECTED: 'danger',
    COMPLETED: 'info',
  }
  return types[status] || 'info'
}

const getStatusLabel = (status: string) => {
  const labels: Record<string, string> = {
    PENDING: '待审核',
    APPROVED: '已通过',
    REJECTED: '已拒绝',
    COMPLETED: '已完成',
  }
  return labels[status] || status
}

const loadReturns = async () => {
  loading.value = true
  try {
    const res = await salesReturnApi.list()
    returnList.value = res.data?.list || []
  } catch (error: any) {
    ElMessage.error(error.message || '加载退货列表失败')
  } finally {
    loading.value = false
  }
}

const loadSalesOrders = async () => {
  try {
    const res = await salesApi.getOrderList({ status: 'completed' })
    salesOrderList.value = res.data?.list || []
  } catch (error: any) {
    logger.error('加载销售订单失败:', error)
  }
}

const loadCustomers = async () => {
  try {
    const res = await listCustomers()
    customerList.value = res.data?.list || []
  } catch (error: any) {
    logger.error('加载客户列表失败:', error)
  }
}

const loadProducts = async () => {
  try {
    const res = await productApi.list()
    productList.value = res.data?.list || []
  } catch (error: any) {
    logger.error('加载产品列表失败:', error)
  }
}

const handleCreate = () => {
  dialogMode.value = 'create'
  Object.assign(formData, {
    id: null,
    salesOrderId: null,
    salesOrderNo: '',
    customerId: null,
    customerName: '',
    returnDate: new Date().toISOString().split('T')[0],
    reason: '',
    remarks: '',
    items: [{ id: null, productId: null, quantity: 1, unitPrice: 0, reason: '' }],
    totalAmount: 0,
    status: 'PENDING',
  })
  editDialogVisible.value = true
}

const handleView = (row: any) => {
  currentReturn.value = { ...row }
  viewDialogVisible.value = true
}

const handleEdit = (row: any) => {
  dialogMode.value = 'edit'
  Object.assign(formData, {
    id: row.id,
    salesOrderId: row.salesOrderId,
    salesOrderNo: row.salesOrderNo,
    customerId: row.customerId,
    customerName: row.customerName,
    returnDate: row.returnDate,
    reason: row.reason,
    remarks: row.remarks,
    items: row.items ? [...row.items] : [],
    totalAmount: row.totalAmount,
    status: row.status,
  })
  editDialogVisible.value = true
}

const handleApprove = async (row: any) => {
  if (!row.id) return

  try {
    await ElMessageBox.confirm(`确定审核通过退货单 ${row.returnNo} 吗？`, '审核确认', {
      type: 'warning',
    })
    await salesReturnApi.approve(row.id)
    ElMessage.success('审核成功')
    await loadReturns()
  } catch (error: any) {
    if (error !== 'cancel') {
      ElMessage.error(error.message || '审核失败')
    }
  }
}

const handleSalesOrderChange = (orderId: number) => {
  const order = salesOrderList.value.find(o => o.id === orderId)
  if (order) {
    formData.salesOrderNo = order.order_no
    formData.customerId = order.customer_id
    formData.customerName = order.customer_name
    if (order.items) {
      formData.items = order.items.map((item: any) => ({
        productId: item.product_id,
        productName: item.product_name,
        productCode: item.product_code,
        quantity: 0,
        unitPrice: item.unit_price,
        amount: 0,
        reason: '',
      }))
    }
    calculateTotal()
  }
}

const handleAddItem = () => {
  formData.items.push({
    id: null,
    productId: null,
    productName: '',
    productCode: '',
    quantity: 1,
    unitPrice: 0,
    amount: 0,
    reason: '',
  })
}

const handleRemoveItem = (index: number) => {
  formData.items.splice(index, 1)
  calculateTotal()
}

const calculateTotal = () => {
  formData.totalAmount = formData.items.reduce((sum: number, item: any) => {
    return sum + item.quantity * item.unitPrice
  }, 0)
}

const handleSubmit = async () => {
  if (!formRef.value) return

  await formRef.value.validate(async (valid: boolean) => {
    if (!valid) return

    if (formData.items.length === 0) {
      ElMessage.warning('请至少添加一条退货明细')
      return
    }

    submitLoading.value = true
    try {
      const submitData = {
        ...formData,
        items: formData.items.map((item: any) => ({
          productId: item.productId,
          quantity: item.quantity,
          unitPrice: item.unitPrice,
          amount: item.quantity * item.unitPrice,
          reason: item.reason,
        })),
      }

      if (dialogMode.value === 'create') {
        await salesReturnApi.create(submitData)
        ElMessage.success('创建成功')
      } else {
        await salesReturnApi.update(formData.id, submitData)
        ElMessage.success('更新成功')
      }
      editDialogVisible.value = false
      await loadReturns()
    } catch (error: any) {
      ElMessage.error(error.message || (dialogMode.value === 'create' ? '创建失败' : '更新失败'))
    } finally {
      submitLoading.value = false
    }
  })
}

const handleEditDialogClose = () => {
  formRef.value?.resetFields()
}

const hasLoaded = createLazyLoader()

onMounted(() => {
  loadReturns()
  loadIfNot('salesOrders', loadSalesOrders, hasLoaded)
  loadIfNot('customers', loadCustomers, hasLoaded)
  loadIfNot('products', loadProducts, hasLoaded)
})
</script>

<style scoped>
.sales-returns-page {
  padding: 20px;
}

.header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

h4 {
  margin: 10px 0;
  font-size: 14px;
  color: #333;
}
</style>
