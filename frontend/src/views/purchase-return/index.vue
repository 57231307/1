<template>
  <div class="purchase-return">
    <div class="page-header">
      <h2>采购退货</h2>
      <el-button type="primary" @click="handleCreate">
        <el-icon><Plus /></el-icon>
        新建退货单
      </el-button>
    </div>

    <!-- 统计卡片 -->
    <el-row :gutter="20" class="stats-row">
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <div class="stat-label">总退货单数</div>
            <div class="stat-value">{{ stats.total || 0 }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <div class="stat-label">待审批</div>
            <div class="stat-value text-warning">{{ stats.pending || 0 }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <div class="stat-label">已审批</div>
            <div class="stat-value text-success">{{ stats.approved || 0 }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <div class="stat-label">退货金额</div>
            <div class="stat-value text-danger">¥{{ stats.amount || 0 }}</div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 筛选区 -->
    <el-card class="filter-card">
      <el-form :inline="true" :model="queryParams">
        <el-form-item label="退货单号">
          <el-input v-model="queryParams.keyword" placeholder="请输入退货单号" clearable />
        </el-form-item>
        <el-form-item label="供应商">
          <el-select v-model="queryParams.supplierId" placeholder="选择供应商" clearable filterable>
            <el-option
              v-for="supplier in suppliers"
              :key="supplier.id"
              :label="supplier.name"
              :value="supplier.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryParams.status" placeholder="选择状态" clearable>
            <el-option label="草稿" value="draft" />
            <el-option label="待审批" value="pending" />
            <el-option label="已审批" value="approved" />
            <el-option label="已拒绝" value="rejected" />
            <el-option label="已完成" value="completed" />
          </el-select>
        </el-form-item>
        <el-form-item label="退货日期">
          <el-date-picker
            v-model="dateRange"
            type="daterange"
            range-separator="至"
            start-placeholder="开始日期"
            end-placeholder="结束日期"
          />
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- 数据表格 -->
    <el-card class="table-card">
      <el-table v-loading="loading" :data="tableData" border stripe>
        <el-table-column prop="returnNo" label="退货单号" min-width="140" />
        <el-table-column prop="purchaseOrderNo" label="采购单号" min-width="140" />
        <el-table-column prop="supplierName" label="供应商" min-width="150" />
        <el-table-column prop="returnDate" label="退货日期" min-width="120" />
        <el-table-column prop="totalAmount" label="退货金额" min-width="100">
          <template #default="{ row }">
            <span class="amount">¥{{ row.totalAmount || 0 }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">
              {{ getStatusText(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="reason" label="退货原因" min-width="150" show-overflow-tooltip />
        <el-table-column label="操作" width="250" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="handleView(row as any)">查看</el-button>
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              type="primary"
              @click="handleEdit(row as any)"
            >
              编辑
            </el-button>
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              type="warning"
              @click="handleSubmit(row as any)"
            >
              提交
            </el-button>
            <el-button
              v-if="row.status === 'pending'"
              size="small"
              type="success"
              @click="handleApprove(row as any)"
            >
              审批
            </el-button>
            <el-button
              v-if="row.status === 'draft'"
              size="small"
              type="danger"
              @click="handleDelete(row as any)"
            >
              删除
            </el-button>
          </template>
        </el-table-column>
      </el-table>

      <el-pagination
        v-model:current-page="queryParams.page"
        v-model:page-size="queryParams.pageSize"
        :total="total"
        :page-sizes="[10, 20, 50, 100]"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="fetchData"
        @current-change="fetchData"
      />
    </el-card>

    <!-- 新建/编辑对话框 -->
    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑退货单' : '新建退货单'" width="900px">
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="采购订单" prop="purchaseOrderId">
              <el-select
                v-model="formData.purchaseOrderId"
                placeholder="选择采购订单"
                filterable
                @change="handleOrderChange"
              >
                <el-option
                  v-for="order in purchaseOrders"
                  :key="order.id"
                  :label="order.order_no"
                  :value="order.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="退货日期" prop="returnDate">
              <el-date-picker
                v-model="formData.returnDate"
                type="date"
                placeholder="选择退货日期"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="退货原因" prop="reason">
          <el-input
            v-model="formData.reason"
            type="textarea"
            :rows="3"
            placeholder="请输入退货原因"
          />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="formData.remarks" type="textarea" :rows="2" placeholder="请输入备注" />
        </el-form-item>

        <!-- 退货明细 -->
        <el-divider content-position="left">退货明细</el-divider>
        <el-button type="primary" size="small" class="mb-10" @click="handleAddItem">
          添加明细
        </el-button>
        <el-table :data="formData.items" border>
          <el-table-column prop="productName" label="产品名称" min-width="150">
            <template #default="{ row }">
              <el-select
                v-model="row.productId"
                filterable
                placeholder="选择产品"
                @change="(v: number) => handleProductChange(row, v)"
              >
                <el-option
                  v-for="product in products"
                  :key="product.id"
                  :label="product.name"
                  :value="product.id"
                />
              </el-select>
            </template>
          </el-table-column>
          <el-table-column prop="quantity" label="退货数量" width="120">
            <template #default="{ row }">
              <el-input-number v-model="row.quantity" :min="1" size="small" />
            </template>
          </el-table-column>
          <el-table-column prop="unitPrice" label="单价" width="120">
            <template #default="{ row }">
              <el-input-number v-model="row.unitPrice" :min="0" :precision="2" size="small" />
            </template>
          </el-table-column>
          <el-table-column prop="amount" label="金额" width="120">
            <template #default="{ row }">
              <span>¥{{ (row.quantity * row.unitPrice).toFixed(2) }}</span>
            </template>
          </el-table-column>
          <el-table-column prop="reason" label="退货原因" min-width="150">
            <template #default="{ row }">
              <el-input v-model="row.reason" size="small" />
            </template>
          </el-table-column>
          <el-table-column label="操作" width="80">
            <template #default="{ $index }">
              <el-button size="small" type="danger" @click="handleRemoveItem($index)"
                >删除</el-button
              >
            </template>
          </el-table-column>
        </el-table>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleFormSubmit"
          >确定</el-button
        >
      </template>
    </el-dialog>

    <!-- 查看详情对话框 -->
    <el-dialog v-model="detailDialogVisible" title="退货单详情" width="900px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="退货单号">{{ detailData.returnNo }}</el-descriptions-item>
        <el-descriptions-item label="采购单号">{{
          detailData.purchaseOrderNo
        }}</el-descriptions-item>
        <el-descriptions-item label="供应商">{{ detailData.supplierName }}</el-descriptions-item>
        <el-descriptions-item label="退货日期">{{ detailData.returnDate }}</el-descriptions-item>
        <el-descriptions-item label="退货金额">
          <span class="amount">¥{{ detailData.totalAmount || 0 }}</span>
        </el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="getStatusType(detailData.status || '')">
            {{ getStatusText(detailData.status || '') }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="退货原因" :span="2">{{
          detailData.reason || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{
          detailData.remarks || '-'
        }}</el-descriptions-item>
      </el-descriptions>

      <el-divider content-position="left">退货明细</el-divider>
      <el-table :data="detailData.items" border>
        <el-table-column prop="productName" label="产品名称" min-width="150" />
        <el-table-column prop="quantity" label="退货数量" width="100" />
        <el-table-column prop="unitPrice" label="单价" width="100" />
        <el-table-column prop="amount" label="金额" width="120" />
        <el-table-column prop="reason" label="退货原因" min-width="150" />
      </el-table>
    </el-dialog>

    <!-- 审批对话框 -->
    <el-dialog v-model="approveDialogVisible" title="审批退货单" width="500px">
      <el-form :model="approveForm" label-width="80px">
        <el-form-item label="审批意见">
          <el-input
            v-model="approveForm.remark"
            type="textarea"
            :rows="3"
            placeholder="请输入审批意见"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="approveDialogVisible = false">取消</el-button>
        <el-button type="danger" @click="handleReject">拒绝</el-button>
        <el-button type="success" @click="handleApproveConfirm">通过</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import {
  purchaseReturnApi,
  type PurchaseReturn,
  type PurchaseReturnItem,
} from '@/api/purchase-return'

// 统计数据
const stats = reactive({
  total: 0,
  pending: 0,
  approved: 0,
  amount: 0,
})

// 表格数据
const tableData = ref<PurchaseReturn[]>([])
const loading = ref(false)
const total = ref(0)
const dateRange = ref<[Date, Date] | null>(null)

// 查询参数
const queryParams = reactive({
  page: 1,
  pageSize: 20,
  keyword: '',
  supplierId: undefined as number | undefined,
  status: '',
})

// 供应商列表
const suppliers = ref<{ id: number; name: string }[]>([])

// 采购订单列表
const purchaseOrders = ref<{ id: number; order_no: string }[]>([])

// 产品列表
const products = ref<{ id: number; name: string; price: number }[]>([])

// 对话框
const dialogVisible = ref(false)
const isEdit = ref(false)
const submitLoading = ref(false)
const formRef = ref()
const formData = reactive({
  id: undefined as number | undefined,
  purchaseOrderId: undefined as number | undefined,
  returnDate: '',
  reason: '',
  remarks: '',
  items: [] as Partial<PurchaseReturnItem>[],
})
const formRules = {
  purchaseOrderId: [{ required: true, message: '请选择采购订单', trigger: 'change' }],
  returnDate: [{ required: true, message: '请选择退货日期', trigger: 'change' }],
  reason: [{ required: true, message: '请输入退货原因', trigger: 'blur' }],
}

// 详情对话框
const detailDialogVisible = ref(false)
const detailData = ref<PurchaseReturn>({} as PurchaseReturn)

// 审批对话框
const approveDialogVisible = ref(false)
const approveForm = reactive({
  id: 0,
  remark: '',
})

const hasLoaded = createLazyLoader()

onMounted(() => {
  fetchData()
  loadIfNot('suppliers', fetchSuppliers, hasLoaded)
  loadIfNot('purchaseOrders', fetchPurchaseOrders, hasLoaded)
  loadIfNot('products', fetchProducts, hasLoaded)
})

const fetchData = async () => {
  loading.value = true
  try {
    const params: any = { ...queryParams }
    if (dateRange.value) {
      params.startDate = dateRange.value[0].toISOString()
      params.endDate = dateRange.value[1].toISOString()
    }
    const res = await purchaseReturnApi.list(params)
    tableData.value = res.data?.list || []
    total.value = res.data?.total || 0

    // 更新统计
    stats.total = total.value
    stats.pending = tableData.value.filter((i) => i.status === 'pending').length
    stats.approved = tableData.value.filter((i) => i.status === 'approved').length
    stats.amount = tableData.value.reduce((sum, i) => sum + (i.totalAmount || 0), 0)
  } catch (error) {
    console.error('获取数据失败:', error)
  } finally {
    loading.value = false
  }
}

const fetchSuppliers = async () => {
  suppliers.value = [
    { id: 1, name: '供应商A' },
    { id: 2, name: '供应商B' },
  ]
}

const fetchPurchaseOrders = async () => {
  purchaseOrders.value = [
    { id: 1, order_no: 'CG20260101001' },
    { id: 2, order_no: 'CG20260101002' },
  ]
}

const fetchProducts = async () => {
  products.value = [
    { id: 1, name: '产品A', price: 100 },
    { id: 2, name: '产品B', price: 200 },
  ]
}

const handleQuery = () => {
  queryParams.page = 1
  fetchData()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.supplierId = undefined
  queryParams.status = ''
  dateRange.value = null
  queryParams.page = 1
  fetchData()
}

const handleCreate = () => {
  isEdit.value = false
  Object.assign(formData, {
    id: undefined,
    purchaseOrderId: undefined,
    returnDate: '',
    reason: '',
    remarks: '',
    items: [],
  })
  dialogVisible.value = true
}

const handleEdit = (row: PurchaseReturn) => {
  isEdit.value = true
  Object.assign(formData, {
    id: row.id,
    purchaseOrderId: row.purchaseOrderId,
    returnDate: row.returnDate,
    reason: row.reason,
    remarks: row.remarks,
    items: row.items || [],
  })
  dialogVisible.value = true
}

const handleView = async (row: PurchaseReturn) => {
  try {
    const res = await purchaseReturnApi.getById(row.id!)
    detailData.value = res.data
    detailDialogVisible.value = true
  } catch (error) {
    console.error('获取详情失败:', error)
  }
}

const handleOrderChange = (orderId: number) => {
  // 根据选择的采购订单加载明细
  const order = purchaseOrders.value.find((o) => o.id === orderId)
  if (order) {
    formData.items = [
      { productId: 1, productName: '产品A', quantity: 10, unitPrice: 100, reason: '' },
    ]
  }
}

const handleAddItem = () => {
  formData.items.push({
    productId: undefined,
    productName: '',
    quantity: 1,
    unitPrice: 0,
    reason: '',
  })
}

const handleRemoveItem = (index: number) => {
  formData.items.splice(index, 1)
}

const handleProductChange = (row: any, productId: number) => {
  const product = products.value.find((p) => p.id === productId)
  if (product) {
    row.productName = product.name
    row.unitPrice = product.price
  }
}

const handleFormSubmit = async () => {
  try {
    await formRef.value?.validate()
    submitLoading.value = true
    if (isEdit.value && formData.id) {
      await purchaseReturnApi.update(formData.id, formData as any)
      ElMessage.success('更新成功')
    } else {
      await purchaseReturnApi.create(formData as any)
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false
    fetchData()
  } catch (error) {
    console.error('提交失败:', error)
  } finally {
    submitLoading.value = false
  }
}

const handleSubmit = async (row: PurchaseReturn) => {
  try {
    await ElMessageBox.confirm('确定要提交该退货单吗？', '提示', { type: 'warning' })
    await purchaseReturnApi.submit(row.id!)
    ElMessage.success('提交成功')
    fetchData()
  } catch (error) {
    if (error !== 'cancel') {
      console.error('提交失败:', error)
    }
  }
}

const handleApprove = (row: PurchaseReturn) => {
  approveForm.id = row.id!
  approveForm.remark = ''
  approveDialogVisible.value = true
}

const handleApproveConfirm = async () => {
  try {
    await purchaseReturnApi.approve(approveForm.id)
    ElMessage.success('审批通过')
    approveDialogVisible.value = false
    fetchData()
  } catch (error) {
    console.error('审批失败:', error)
  }
}

const handleReject = async () => {
  try {
    await purchaseReturnApi.reject(approveForm.id, approveForm.remark)
    ElMessage.success('已拒绝')
    approveDialogVisible.value = false
    fetchData()
  } catch (error) {
    console.error('拒绝失败:', error)
  }
}

const handleDelete = async (row: PurchaseReturn) => {
  try {
    await ElMessageBox.confirm('确定要删除该退货单吗？', '提示', { type: 'warning' })
    await purchaseReturnApi.delete(row.id!)
    ElMessage.success('删除成功')
    fetchData()
  } catch (error) {
    if (error !== 'cancel') {
      console.error('删除失败:', error)
    }
  }
}

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    pending: 'warning',
    approved: 'success',
    rejected: 'danger',
    completed: 'success',
  }
  return map[status] || 'info'
}

const getStatusText = (status: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    pending: '待审批',
    approved: '已审批',
    rejected: '已拒绝',
    completed: '已完成',
  }
  return map[status] || status
}
</script>

<style scoped>
.purchase-return {
  padding: 20px;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}

.page-header h2 {
  margin: 0;
  font-size: 20px;
  font-weight: 600;
}

.stats-row {
  margin-bottom: 20px;
}

.stat-item {
  text-align: center;
}

.stat-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 10px;
}

.stat-value {
  font-size: 28px;
  font-weight: 600;
}

.text-warning {
  color: #e6a23c;
}

.text-success {
  color: #67c23a;
}

.text-danger {
  color: #f56c6c;
}

.filter-card {
  margin-bottom: 20px;
}

.table-card {
  margin-bottom: 20px;
}

.amount {
  font-weight: 600;
  color: #f56c6c;
}

.mb-10 {
  margin-bottom: 10px;
}

.el-pagination {
  margin-top: 20px;
  justify-content: flex-end;
}
</style>
