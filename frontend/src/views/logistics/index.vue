<template>
  <div class="logistics">
    <div class="page-header">
      <h2>物流管理</h2>
      <el-button type="primary" @click="handleCreate">
        <el-icon><Plus /></el-icon>
        新建运单
      </el-button>
    </div>

    <!-- 统计卡片 -->
    <el-row :gutter="20" class="stats-row">
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <div class="stat-label">总运单数</div>
            <div class="stat-value">{{ stats.total || 0 }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <div class="stat-label">待发货</div>
            <div class="stat-value text-warning">{{ stats.pending || 0 }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <div class="stat-label">运输中</div>
            <div class="stat-value text-primary">{{ stats.inTransit || 0 }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <div class="stat-label">已签收</div>
            <div class="stat-value text-success">{{ stats.delivered || 0 }}</div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 筛选区 -->
    <el-card class="filter-card">
      <el-form :inline="true" :model="queryParams">
        <el-form-item label="运单号">
          <el-input v-model="queryParams.keyword" placeholder="请输入运单号" clearable />
        </el-form-item>
        <el-form-item label="物流公司">
          <el-select v-model="queryParams.logistics_company" placeholder="选择物流公司" clearable>
            <el-option label="顺丰速运" value="顺丰速运" />
            <el-option label="中通快递" value="中通快递" />
            <el-option label="圆通速递" value="圆通速递" />
            <el-option label="韵达快递" value="韵达快递" />
            <el-option label="京东物流" value="京东物流" />
          </el-select>
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryParams.status" placeholder="选择状态" clearable>
            <el-option label="待发货" value="pending" />
            <el-option label="已发货" value="shipped" />
            <el-option label="运输中" value="in_transit" />
            <el-option label="已签收" value="delivered" />
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
        <el-table-column prop="waybill_no" label="运单号" min-width="140" />
        <el-table-column prop="order_no" label="关联订单" min-width="140" />
        <el-table-column prop="logistics_company" label="物流公司" min-width="120" />
        <el-table-column prop="tracking_number" label="快递单号" min-width="150" />
        <el-table-column prop="driver_name" label="司机姓名" min-width="100" />
        <el-table-column prop="driver_phone" label="司机电话" min-width="120" />
        <el-table-column prop="freight_fee" label="运费" min-width="100">
          <template #default="{ row }">
            <span>¥{{ row.freight_fee || 0 }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="expected_arrival" label="预计到达" min-width="120" />
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">
              {{ getStatusText(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="250" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="handleView(row as any)">查看</el-button>
            <el-button
              v-if="row.status === 'pending'"
              size="small"
              type="primary"
              @click="handleEdit(row as any)"
            >
              编辑
            </el-button>
            <el-button
              v-if="row.status === 'pending'"
              size="small"
              type="success"
              @click="handleShip(row as any)"
            >
              发货
            </el-button>
            <el-button
              v-if="row.status === 'shipped' || row.status === 'in_transit'"
              size="small"
              type="warning"
              @click="handleUpdateStatus(row as any)"
            >
              更新状态
            </el-button>
            <el-button
              v-if="row.status === 'pending'"
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
        v-model:page-size="queryParams.page_size"
        :total="total"
        :page-sizes="[10, 20, 50, 100]"
        layout="total, sizes, prev, pager, next, jumper"
        @size-change="fetchData"
        @current-change="fetchData"
      />
    </el-card>

    <!-- 新建/编辑对话框 -->
    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑运单' : '新建运单'" width="600px">
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <el-form-item label="关联订单" prop="order_id">
          <el-select v-model="formData.order_id" placeholder="选择关联订单" filterable>
            <el-option
              v-for="order in orders"
              :key="order.id"
              :label="order.order_no"
              :value="order.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="物流公司" prop="logistics_company">
          <el-select v-model="formData.logistics_company" placeholder="选择物流公司">
            <el-option label="顺丰速运" value="顺丰速运" />
            <el-option label="中通快递" value="中通快递" />
            <el-option label="圆通速递" value="圆通速递" />
            <el-option label="韵达快递" value="韵达快递" />
            <el-option label="京东物流" value="京东物流" />
          </el-select>
        </el-form-item>
        <el-form-item label="快递单号" prop="tracking_number">
          <el-input v-model="formData.tracking_number" placeholder="请输入快递单号" />
        </el-form-item>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="司机姓名">
              <el-input v-model="formData.driver_name" placeholder="请输入司机姓名" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="司机电话">
              <el-input v-model="formData.driver_phone" placeholder="请输入司机电话" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="运费">
              <el-input-number v-model="formData.freight_fee" :min="0" :precision="2" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="预计到达">
              <el-date-picker
                v-model="formData.expected_arrival"
                type="date"
                placeholder="选择预计到达日期"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="备注">
          <el-input v-model="formData.notes" type="textarea" :rows="3" placeholder="请输入备注" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>

    <!-- 查看详情对话框 -->
    <el-dialog v-model="detailDialogVisible" title="运单详情" width="600px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="运单号">{{ detailData.waybill_no }}</el-descriptions-item>
        <el-descriptions-item label="关联订单">{{ detailData.order_no }}</el-descriptions-item>
        <el-descriptions-item label="物流公司">{{
          detailData.logistics_company
        }}</el-descriptions-item>
        <el-descriptions-item label="快递单号">{{
          detailData.tracking_number
        }}</el-descriptions-item>
        <el-descriptions-item label="司机姓名">{{
          detailData.driver_name || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="司机电话">{{
          detailData.driver_phone || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="运费">¥{{ detailData.freight_fee || 0 }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="getStatusType(detailData.status)">
            {{ getStatusText(detailData.status) }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="预计到达">{{
          detailData.expected_arrival || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="实际到达">{{
          detailData.actual_arrival || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{
          detailData.notes || '-'
        }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>

    <!-- 更新状态对话框 -->
    <el-dialog v-model="statusDialogVisible" title="更新运单状态" width="400px">
      <el-form :model="statusForm" label-width="80px">
        <el-form-item label="当前状态">
          <el-tag :type="getStatusType(statusForm.currentStatus)">
            {{ getStatusText(statusForm.currentStatus) }}
          </el-tag>
        </el-form-item>
        <el-form-item label="新状态">
          <el-select v-model="statusForm.newStatus" placeholder="选择新状态">
            <el-option
              v-for="status in availableStatuses"
              :key="status.value"
              :label="status.label"
              :value="status.value"
            />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="statusDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="handleStatusSubmit">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { logisticsApi, type LogisticsWaybill } from '@/api/logistics'
import { logger } from '@/utils/logger'

// 统计数据
const stats = reactive({
  total: 0,
  pending: 0,
  inTransit: 0,
  delivered: 0,
})

// 表格数据
const tableData = ref<LogisticsWaybill[]>([])
const loading = ref(false)
const total = ref(0)
const dateRange = ref<[Date, Date] | null>(null)

// 查询参数
const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  logistics_company: '',
  status: '',
})

// 关联订单列表
const orders = ref<{ id: number; order_no: string }[]>([])

// 对话框
const dialogVisible = ref(false)
const isEdit = ref(false)
const submitLoading = ref(false)
const formRef = ref()
const formData = reactive({
  id: undefined as number | undefined,
  order_id: undefined as number | undefined,
  logistics_company: '',
  tracking_number: '',
  driver_name: '',
  driver_phone: '',
  freight_fee: 0,
  expected_arrival: '',
  notes: '',
})
const formRules = {
  logistics_company: [{ required: true, message: '请选择物流公司', trigger: 'change' }],
  tracking_number: [{ required: true, message: '请输入快递单号', trigger: 'blur' }],
}

// 详情对话框
const detailDialogVisible = ref(false)
const detailData = ref<LogisticsWaybill>({} as LogisticsWaybill)

// 状态更新对话框
const statusDialogVisible = ref(false)
const statusForm = reactive({
  id: 0,
  currentStatus: '',
  newStatus: '',
})

const availableStatuses = computed(() => {
  const map: Record<string, { label: string; value: string }[]> = {
    shipped: [
      { label: '运输中', value: 'in_transit' },
      { label: '已签收', value: 'delivered' },
    ],
    in_transit: [{ label: '已签收', value: 'delivered' }],
  }
  return map[statusForm.currentStatus] || []
})

const hasLoaded = createLazyLoader()

onMounted(() => {
  fetchData()
  loadIfNot('orders', fetchOrders, hasLoaded)
})

const fetchData = async () => {
  loading.value = true
  try {
    const params: any = { ...queryParams }
    if (dateRange.value) {
      params.start_date = dateRange.value[0].toISOString()
      params.end_date = dateRange.value[1].toISOString()
    }
    const res = await logisticsApi.list(params)
    tableData.value = res.data?.list || []
    total.value = res.data?.total || 0

    // 更新统计
    stats.total = total.value
    stats.pending = tableData.value.filter(i => i.status === 'pending').length
    stats.inTransit = tableData.value.filter(i => i.status === 'in_transit').length
    stats.delivered = tableData.value.filter(i => i.status === 'delivered').length
  } catch (error) {
    logger.error('获取数据失败:', error)
  } finally {
    loading.value = false
  }
}

const fetchOrders = async () => {
  orders.value = [
    { id: 1, order_no: 'SO20260101001' },
    { id: 2, order_no: 'SO20260101002' },
  ]
}

const handleQuery = () => {
  queryParams.page = 1
  fetchData()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.logistics_company = ''
  queryParams.status = ''
  dateRange.value = null
  queryParams.page = 1
  fetchData()
}

const handleCreate = () => {
  isEdit.value = false
  Object.assign(formData, {
    id: undefined,
    order_id: undefined,
    logistics_company: '',
    tracking_number: '',
    driver_name: '',
    driver_phone: '',
    freight_fee: 0,
    expected_arrival: '',
    notes: '',
  })
  dialogVisible.value = true
}

const handleEdit = (row: LogisticsWaybill) => {
  isEdit.value = true
  Object.assign(formData, {
    id: row.id,
    order_id: row.order_id,
    logistics_company: row.logistics_company,
    tracking_number: row.tracking_number,
    driver_name: row.driver_name,
    driver_phone: row.driver_phone,
    freight_fee: row.freight_fee,
    expected_arrival: row.expected_arrival,
    notes: row.notes,
  })
  dialogVisible.value = true
}

const handleView = async (row: LogisticsWaybill) => {
  try {
    const res = await logisticsApi.getById(row.id!)
    detailData.value = res.data
    detailDialogVisible.value = true
  } catch (error) {
    logger.error('获取详情失败:', error)
  }
}

const handleSubmit = async () => {
  try {
    await formRef.value?.validate()
    submitLoading.value = true
    if (isEdit.value && formData.id) {
      await logisticsApi.update(formData.id, formData)
      ElMessage.success('更新成功')
    } else {
      await logisticsApi.create(formData)
      ElMessage.success('创建成功')
    }
    dialogVisible.value = false
    fetchData()
  } catch (error) {
    logger.error('提交失败:', error)
  } finally {
    submitLoading.value = false
  }
}

const handleShip = async (row: LogisticsWaybill) => {
  try {
    await ElMessageBox.confirm('确定要发货吗？', '提示', { type: 'warning' })
    await logisticsApi.update(row.id!, { status: 'shipped' })
    ElMessage.success('发货成功')
    fetchData()
  } catch (error) {
    if (error !== 'cancel') {
      logger.error('发货失败:', error)
    }
  }
}

const handleUpdateStatus = (row: LogisticsWaybill) => {
  statusForm.id = row.id!
  statusForm.currentStatus = row.status
  statusForm.newStatus = ''
  statusDialogVisible.value = true
}

const handleStatusSubmit = async () => {
  try {
    await logisticsApi.update(statusForm.id, { status: statusForm.newStatus as any })
    ElMessage.success('状态更新成功')
    statusDialogVisible.value = false
    fetchData()
  } catch (error) {
    logger.error('状态更新失败:', error)
  }
}

const handleDelete = async (row: LogisticsWaybill) => {
  try {
    await ElMessageBox.confirm('确定要删除该运单吗？', '提示', { type: 'warning' })
    await logisticsApi.delete(row.id!)
    ElMessage.success('删除成功')
    fetchData()
  } catch (error) {
    if (error !== 'cancel') {
      logger.error('删除失败:', error)
    }
  }
}

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    pending: 'info',
    shipped: 'warning',
    in_transit: 'primary',
    delivered: 'success',
    cancelled: 'danger',
  }
  return map[status] || 'info'
}

const getStatusText = (status: string) => {
  const map: Record<string, string> = {
    pending: '待发货',
    shipped: '已发货',
    in_transit: '运输中',
    delivered: '已签收',
    cancelled: '已取消',
  }
  return map[status] || status
}
</script>

<style scoped>
.logistics {
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

.text-primary {
  color: #409eff;
}

.text-success {
  color: #67c23a;
}

.filter-card {
  margin-bottom: 20px;
}

.table-card {
  margin-bottom: 20px;
}

.el-pagination {
  margin-top: 20px;
  justify-content: flex-end;
}
</style>
