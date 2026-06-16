<template>
  <div class="purchase-inspection">
    <div class="page-header">
      <h2>采购检验</h2>
      <el-button type="primary" @click="handleCreate">
        <el-icon><Plus /></el-icon>
        新建检验单
      </el-button>
    </div>

    <!-- 统计卡片 -->
    <el-row :gutter="20" class="stats-row">
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <div class="stat-label">总检验单数</div>
            <div class="stat-value">{{ stats.total || 0 }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <div class="stat-label">待检验</div>
            <div class="stat-value text-warning">{{ stats.pending || 0 }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <div class="stat-label">检验合格</div>
            <div class="stat-value text-success">{{ stats.passed || 0 }}</div>
          </div>
        </el-card>
      </el-col>
      <el-col :span="6">
        <el-card shadow="hover">
          <div class="stat-item">
            <div class="stat-label">检验不合格</div>
            <div class="stat-value text-danger">{{ stats.failed || 0 }}</div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <!-- 筛选区 -->
    <el-card class="filter-card">
      <el-form :inline="true" :model="queryParams">
        <el-form-item label="检验单号">
          <el-input v-model="queryParams.keyword" placeholder="请输入检验单号" clearable />
        </el-form-item>
        <el-form-item label="供应商">
          <el-select
            v-model="queryParams.supplier_id"
            placeholder="选择供应商"
            clearable
            filterable
          >
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
            <el-option label="待检验" value="pending" />
            <el-option label="已完成" value="completed" />
            <el-option label="已拒绝" value="rejected" />
          </el-select>
        </el-form-item>
        <el-form-item label="检验结果">
          <el-select v-model="queryParams.result" placeholder="选择结果" clearable>
            <el-option label="合格" value="pass" />
            <el-option label="不合格" value="fail" />
            <el-option label="部分合格" value="partial" />
          </el-select>
        </el-form-item>
        <el-form-item label="检验日期">
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
        <el-table-column prop="inspection_no" label="检验单号" min-width="140" />
        <el-table-column prop="receipt_no" label="入库单号" min-width="140" />
        <el-table-column prop="supplier_name" label="供应商" min-width="150" />
        <el-table-column prop="inspection_date" label="检验日期" min-width="120" />
        <el-table-column prop="inspector_name" label="检验员" min-width="100" />
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">
              {{ getStatusText(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="result" label="检验结果" width="100" align="center">
          <template #default="{ row }">
            <el-tag v-if="row.result" :type="getResultType(row.result)">
              {{ getResultText(row.result) }}
            </el-tag>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column prop="remark" label="备注" min-width="150" show-overflow-tooltip />
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button size="small" @click="handleView(row as any)">查看</el-button>
            <el-button
              v-if="row.status === 'draft' || row.status === 'pending'"
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
              @click="handleComplete(row as any)"
            >
              完成
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
    <el-dialog v-model="dialogVisible" :title="isEdit ? '编辑检验单' : '新建检验单'" width="800px">
      <el-form ref="formRef" :model="formData" :rules="formRules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="入库单号" prop="receipt_id">
              <el-select
                v-model="formData.receipt_id"
                placeholder="选择入库单"
                filterable
                @change="handleReceiptChange"
              >
                <el-option
                  v-for="receipt in receipts"
                  :key="receipt.id"
                  :label="receipt.receipt_no"
                  :value="receipt.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="检验日期" prop="inspection_date">
              <el-date-picker
                v-model="formData.inspection_date"
                type="date"
                placeholder="选择检验日期"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="备注">
          <el-input v-model="formData.remark" type="textarea" :rows="3" placeholder="请输入备注" />
        </el-form-item>

        <!-- 检验明细 -->
        <el-divider content-position="left">检验明细</el-divider>
        <el-table :data="formData.items" border>
          <el-table-column prop="product_name" label="产品名称" min-width="150" />
          <el-table-column prop="expected_quantity" label="预期数量" width="100" />
          <el-table-column prop="inspected_quantity" label="检验数量" width="120">
            <template #default="{ row }">
              <el-input-number v-model="row.inspected_quantity" :min="0" size="small" />
            </template>
          </el-table-column>
          <el-table-column prop="passed_quantity" label="合格数量" width="120">
            <template #default="{ row }">
              <el-input-number v-model="row.passed_quantity" :min="0" size="small" />
            </template>
          </el-table-column>
          <el-table-column prop="failed_quantity" label="不合格数量" width="120">
            <template #default="{ row }">
              <el-input-number v-model="row.failed_quantity" :min="0" size="small" />
            </template>
          </el-table-column>
          <el-table-column prop="defect_reason" label="缺陷原因" min-width="150">
            <template #default="{ row }">
              <el-input v-model="row.defect_reason" size="small" />
            </template>
          </el-table-column>
        </el-table>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>

    <!-- 查看详情对话框 -->
    <el-dialog v-model="detailDialogVisible" title="检验单详情" width="800px">
      <el-descriptions :column="2" border>
        <el-descriptions-item label="检验单号">{{ detailData.inspection_no }}</el-descriptions-item>
        <el-descriptions-item label="入库单号">{{ detailData.receipt_no }}</el-descriptions-item>
        <el-descriptions-item label="供应商">{{ detailData.supplier_name }}</el-descriptions-item>
        <el-descriptions-item label="检验日期">{{
          detailData.inspection_date
        }}</el-descriptions-item>
        <el-descriptions-item label="检验员">{{ detailData.inspector_name }}</el-descriptions-item>
        <el-descriptions-item label="状态">
          <el-tag :type="getStatusType(detailData.status)">
            {{ getStatusText(detailData.status) }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="检验结果">
          <el-tag v-if="detailData.result" :type="getResultType(detailData.result)">
            {{ getResultText(detailData.result) }}
          </el-tag>
        </el-descriptions-item>
        <el-descriptions-item label="备注">{{ detailData.remark || '-' }}</el-descriptions-item>
      </el-descriptions>

      <el-divider content-position="left">检验明细</el-divider>
      <el-table :data="detailData.items" border>
        <el-table-column prop="product_name" label="产品名称" min-width="150" />
        <el-table-column prop="expected_quantity" label="预期数量" width="100" />
        <el-table-column prop="inspected_quantity" label="检验数量" width="100" />
        <el-table-column prop="passed_quantity" label="合格数量" width="100" />
        <el-table-column prop="failed_quantity" label="不合格数量" width="100" />
        <el-table-column prop="defect_reason" label="缺陷原因" min-width="150" />
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  purchaseInspectionApi,
  type PurchaseInspection,
  type PurchaseInspectionItem,
} from '@/api/purchase-inspection'
import { getReceiptItems, type ReceiptItem } from '@/api/purchaseReceipt'
import { logger } from '@/utils/logger'

// 统计数据
const stats = reactive({
  total: 0,
  pending: 0,
  passed: 0,
  failed: 0,
})

// 表格数据
const tableData = ref<PurchaseInspection[]>([])
const loading = ref(false)
const total = ref(0)
const dateRange = ref<[Date, Date] | null>(null)
// 入库单明细加载状态（P1-5 B2 子任务引入）
const receiptItemsLoading = ref(false)

// 查询参数
const queryParams = reactive({
  page: 1,
  page_size: 20,
  keyword: '',
  supplier_id: undefined as number | undefined,
  status: '',
  result: '',
})

// 供应商列表
const suppliers = ref<{ id: number; name: string }[]>([])

// 入库单列表
const receipts = ref<{ id: number; receipt_no: string }[]>([])

// 对话框
const dialogVisible = ref(false)
const isEdit = ref(false)
const submitLoading = ref(false)
const formRef = ref()
const formData = reactive({
  id: undefined as number | undefined,
  receipt_id: undefined as number | undefined,
  inspection_date: '',
  remark: '',
  items: [] as Partial<PurchaseInspectionItem>[],
})
const formRules = {
  receipt_id: [{ required: true, message: '请选择入库单', trigger: 'change' }],
  inspection_date: [{ required: true, message: '请选择检验日期', trigger: 'change' }],
}

// 详情对话框
const detailDialogVisible = ref(false)
const detailData = ref<PurchaseInspection>({} as PurchaseInspection)

const hasLoaded = createLazyLoader()

onMounted(() => {
  fetchData()
  loadIfNot('suppliers', fetchSuppliers, hasLoaded)
  loadIfNot('receipts', fetchReceipts, hasLoaded)
})

const fetchData = async () => {
  loading.value = true
  try {
    const params: any = { ...queryParams }
    if (dateRange.value) {
      params.inspection_date_from = dateRange.value[0].toISOString()
      params.inspection_date_to = dateRange.value[1].toISOString()
    }
    const res = await purchaseInspectionApi.list(params)
    tableData.value = res.data?.list || []
    total.value = res.data?.total || 0

    // 更新统计
    stats.total = total.value
    stats.pending = tableData.value.filter(i => i.status === 'pending').length
    stats.passed = tableData.value.filter(i => i.result === 'pass').length
    stats.failed = tableData.value.filter(i => i.result === 'fail').length
  } catch (error) {
    logger.error('获取数据失败:', error)
  } finally {
    loading.value = false
  }
}

const fetchSuppliers = async () => {
  // 从API获取供应商列表
  suppliers.value = [
    { id: 1, name: '供应商A' },
    { id: 2, name: '供应商B' },
  ]
}

const fetchReceipts = async () => {
  // 从API获取入库单列表
  receipts.value = [
    { id: 1, receipt_no: 'RK20260101001' },
    { id: 2, receipt_no: 'RK20260101002' },
  ]
}

const handleQuery = () => {
  queryParams.page = 1
  fetchData()
}

const handleReset = () => {
  queryParams.keyword = ''
  queryParams.supplier_id = undefined
  queryParams.status = ''
  queryParams.result = ''
  dateRange.value = null
  queryParams.page = 1
  fetchData()
}

const handleCreate = () => {
  isEdit.value = false
  Object.assign(formData, {
    id: undefined,
    receipt_id: undefined,
    inspection_date: '',
    remark: '',
    items: [],
  })
  dialogVisible.value = true
}

const handleEdit = (row: PurchaseInspection) => {
  isEdit.value = true
  Object.assign(formData, {
    id: row.id,
    receipt_id: row.receipt_id,
    inspection_date: row.inspection_date,
    remark: row.remark,
    items: row.items || [],
  })
  dialogVisible.value = true
}

const handleView = async (row: PurchaseInspection) => {
  try {
    const res = await purchaseInspectionApi.getById(row.id!)
    detailData.value = res.data
    detailDialogVisible.value = true
  } catch (error) {
    logger.error('获取详情失败:', error)
  }
}

const handleReceiptChange = async (receiptId: number) => {
  if (!receiptId) {
    formData.items = []
    return
  }
  receiptItemsLoading.value = true
  try {
    const res = await getReceiptItems(receiptId)
    // 类型已在 purchaseReceipt.ts 中声明，可直接解构
    const items: ReceiptItem[] = res.data?.items || []
    if (items.length === 0) {
      ElMessage.info('该入库单暂无明细')
      formData.items = []
      return
    }
    // 将入库单明细映射为检验单明细，初始化各数量字段
    formData.items = items.map(item => ({
      product_id: item.product_id,
      product_name: item.product_name,
      product_code: item.product_code,
      expected_quantity: item.quantity,
      inspected_quantity: 0,
      passed_quantity: 0,
      failed_quantity: 0,
      defect_reason: '',
    }))
  } catch (error) {
    const errMsg = error instanceof Error ? error.message : '获取入库单明细失败，请稍后重试'
    ElMessage.error(errMsg)
    logger.error('获取入库单明细失败:', error)
    formData.items = []
  } finally {
    receiptItemsLoading.value = false
  }
}

const handleSubmit = async () => {
  try {
    await formRef.value?.validate()
    submitLoading.value = true
    if (isEdit.value && formData.id) {
      await purchaseInspectionApi.update(formData.id, formData as any)
      ElMessage.success('更新成功')
    } else {
      await purchaseInspectionApi.create(formData as any)
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

const handleComplete = async (row: PurchaseInspection) => {
  try {
    await ElMessageBox.confirm('确定要完成该检验单吗？', '提示', { type: 'warning' })
    await purchaseInspectionApi.complete(row.id!)
    ElMessage.success('操作成功')
    fetchData()
  } catch (error) {
    if (error !== 'cancel') {
      logger.error('操作失败:', error)
    }
  }
}

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    draft: 'info',
    pending: 'warning',
    completed: 'success',
    rejected: 'danger',
  }
  return map[status] || 'info'
}

const getStatusText = (status: string) => {
  const map: Record<string, string> = {
    draft: '草稿',
    pending: '待检验',
    completed: '已完成',
    rejected: '已拒绝',
  }
  return map[status] || status
}

const getResultType = (result: string) => {
  const map: Record<string, string> = {
    pass: 'success',
    fail: 'danger',
    partial: 'warning',
  }
  return map[result] || 'info'
}

const getResultText = (result: string) => {
  const map: Record<string, string> = {
    pass: '合格',
    fail: '不合格',
    partial: '部分合格',
  }
  return map[result] || result
}
</script>

<style scoped>
.purchase-inspection {
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

.el-pagination {
  margin-top: 20px;
  justify-content: flex-end;
}
</style>
