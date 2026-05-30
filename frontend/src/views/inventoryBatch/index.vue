<template>
  <div class="inventory-batch">
    <el-card>
      <template #header>
        <div class="card-header">
          <span>批次管理</span>
        </div>
      </template>

      <div class="toolbar">
        <el-form inline>
          <el-form-item label="批次号">
            <el-input
              v-model="queryParams.batchNo"
              placeholder="请输入"
              clearable
              style="width: 180px"
            />
          </el-form-item>
          <el-form-item label="色号">
            <el-input
              v-model="queryParams.colorNo"
              placeholder="请输入"
              clearable
              style="width: 180px"
            />
          </el-form-item>
          <el-form-item label="等级">
            <el-select
              v-model="queryParams.grade"
              placeholder="请选择"
              clearable
              style="width: 120px"
            >
              <el-option label="一等品" value="一等品" />
              <el-option label="二等品" value="二等品" />
              <el-option label="三等品" value="三等品" />
            </el-select>
          </el-form-item>
          <el-form-item>
            <el-button type="primary" @click="fetchBatches">查询</el-button>
            <el-button @click="handleReset">重置</el-button>
          </el-form-item>
        </el-form>
        <div class="actions">
          <el-button type="primary" @click="handleCreate">新建批次</el-button>
        </div>
      </div>

      <el-table :data="batchList" border stripe>
        <el-table-column prop="batchNo" label="批次号" width="140" />
        <el-table-column prop="productName" label="产品名称" />
        <el-table-column prop="colorNo" label="色号" width="100" />
        <el-table-column prop="dyeLotNo" label="缸号" width="100" />
        <el-table-column prop="grade" label="等级" width="100">
          <template #default="{ row }">
            <el-tag v-if="row.grade === '一等品'" type="success">{{ row.grade }}</el-tag>
            <el-tag v-else-if="row.grade === '二等品'" type="warning">{{ row.grade }}</el-tag>
            <el-tag v-else type="danger">{{ row.grade }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="quantityMeters" label="数量(米)" width="120" />
        <el-table-column prop="quantityKg" label="数量(kg)" width="100" />
        <el-table-column prop="gramWeight" label="克重" width="100" />
        <el-table-column prop="width" label="幅宽" width="100" />
        <el-table-column prop="warehouseName" label="仓库" />
        <el-table-column prop="stockStatus" label="库存状态" width="100">
          <template #default="{ row }">
            <el-tag v-if="row.stockStatus === '正常'" type="success">{{ row.stockStatus }}</el-tag>
            <el-tag v-else type="warning">{{ row.stockStatus }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="qualityStatus" label="品质状态" width="100">
          <template #default="{ row }">
            <el-tag v-if="row.qualityStatus === '合格'" type="success">{{
              row.qualityStatus
            }}</el-tag>
            <el-tag v-else type="danger">{{ row.qualityStatus }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="productionDate" label="生产日期" width="120" />
        <el-table-column label="操作" fixed="right" width="220">
          <template #default="{ row }">
            <el-button link type="primary" @click="handleView(row)">查看</el-button>
            <el-button link type="primary" @click="handleEdit(row)">编辑</el-button>
            <el-button link type="primary" @click="handleTransfer(row)">调拨</el-button>
            <el-button link type="danger" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <el-pagination
        v-model:current-page="pagination.page"
        v-model:page-size="pagination.pageSize"
        :total="pagination.total"
        layout="total, prev, pager, next, jumper"
        @current-change="fetchBatches"
      />
    </el-card>

    <!-- 新建/编辑对话框 -->
    <el-dialog v-model="batchDialogVisible" :title="isEdit ? '编辑批次' : '新建批次'" width="700px">
      <el-form ref="batchFormRef" :model="batchForm" :rules="batchRules" label-width="120px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="批次号" prop="batchNo">
              <el-input v-model="batchForm.batchNo" placeholder="请输入批次号" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="产品" prop="productId">
              <el-select v-model="batchForm.productId" placeholder="请选择" style="width: 100%">
                <el-option label="产品A" :value="1" />
                <el-option label="产品B" :value="2" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="仓库" prop="warehouseId">
              <el-select v-model="batchForm.warehouseId" placeholder="请选择" style="width: 100%">
                <el-option label="仓库1" :value="1" />
                <el-option label="仓库2" :value="2" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="色号" prop="colorNo">
              <el-input v-model="batchForm.colorNo" placeholder="请输入色号" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="缸号" prop="dyeLotNo">
              <el-input v-model="batchForm.dyeLotNo" placeholder="请输入缸号" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="等级" prop="grade">
              <el-select v-model="batchForm.grade" placeholder="请选择" style="width: 100%">
                <el-option label="一等品" value="一等品" />
                <el-option label="二等品" value="二等品" />
                <el-option label="三等品" value="三等品" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="数量(米)" prop="quantityMeters">
              <el-input-number v-model="batchForm.quantityMeters" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="数量(kg)" prop="quantityKg">
              <el-input-number v-model="batchForm.quantityKg" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="克重" prop="gramWeight">
              <el-input-number v-model="batchForm.gramWeight" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="幅宽" prop="width">
              <el-input-number v-model="batchForm.width" :min="0" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="生产日期" prop="productionDate">
          <el-date-picker
            v-model="batchForm.productionDate"
            type="date"
            placeholder="选择日期"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="备注" prop="remarks">
          <el-input v-model="batchForm.remarks" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="batchDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSaveBatch">保存</el-button>
      </template>
    </el-dialog>

    <!-- 查看详情对话框 -->
    <el-dialog v-model="viewDialogVisible" title="批次详情" width="700px">
      <el-descriptions v-if="currentBatch" :column="2" border>
        <el-descriptions-item label="批次号">{{ currentBatch.batchNo }}</el-descriptions-item>
        <el-descriptions-item label="产品">{{ currentBatch.productName }}</el-descriptions-item>
        <el-descriptions-item label="仓库">{{ currentBatch.warehouseName }}</el-descriptions-item>
        <el-descriptions-item label="色号">{{ currentBatch.colorNo }}</el-descriptions-item>
        <el-descriptions-item label="缸号">{{ currentBatch.dyeLotNo || '-' }}</el-descriptions-item>
        <el-descriptions-item label="等级">{{ currentBatch.grade }}</el-descriptions-item>
        <el-descriptions-item label="数量(米)">{{
          currentBatch.quantityMeters
        }}</el-descriptions-item>
        <el-descriptions-item label="数量(kg)">{{ currentBatch.quantityKg }}</el-descriptions-item>
        <el-descriptions-item label="克重">{{
          currentBatch.gramWeight || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="幅宽">{{ currentBatch.width || '-' }}</el-descriptions-item>
        <el-descriptions-item label="库存状态">{{ currentBatch.stockStatus }}</el-descriptions-item>
        <el-descriptions-item label="品质状态">{{
          currentBatch.qualityStatus
        }}</el-descriptions-item>
        <el-descriptions-item label="生产日期" :span="2">{{
          currentBatch.productionDate || '-'
        }}</el-descriptions-item>
        <el-descriptions-item label="备注" :span="2">{{
          currentBatch.remarks || '-'
        }}</el-descriptions-item>
      </el-descriptions>
    </el-dialog>

    <!-- 调拨对话框 -->
    <el-dialog v-model="transferDialogVisible" title="批次调拨" width="500px">
      <el-form
        ref="transferFormRef"
        :model="transferForm"
        :rules="transferRules"
        label-width="120px"
      >
        <el-form-item label="目标仓库" prop="toWarehouseId">
          <el-select v-model="transferForm.toWarehouseId" placeholder="请选择" style="width: 100%">
            <el-option label="仓库1" :value="1" />
            <el-option label="仓库2" :value="2" />
          </el-select>
        </el-form-item>
        <el-form-item label="调拨数量(米)" prop="quantityMeters">
          <el-input-number v-model="transferForm.quantityMeters" :min="0" style="width: 100%" />
        </el-form-item>
        <el-form-item label="调拨数量(kg)" prop="quantityKg">
          <el-input-number v-model="transferForm.quantityKg" :min="0" style="width: 100%" />
        </el-form-item>
        <el-form-item label="备注" prop="remarks">
          <el-input v-model="transferForm.remarks" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="transferDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSaveTransfer"
          >确定</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  listBatches,
  getBatch,
  createBatch,
  updateBatch,
  deleteBatch,
  transferBatch,
  type InventoryBatch,
} from '@/api/inventoryBatch'

const batchList = ref<InventoryBatch[]>([])
const pagination = reactive({
  page: 1,
  pageSize: 20,
  total: 0,
})

const queryParams = reactive({
  batchNo: '',
  colorNo: '',
  grade: '',
})

const batchDialogVisible = ref(false)
const viewDialogVisible = ref(false)
const transferDialogVisible = ref(false)
const isEdit = ref(false)
const submitLoading = ref(false)
const batchFormRef = ref()
const transferFormRef = ref()
const currentBatch = ref<InventoryBatch | null>(null)
const currentTransferBatchId = ref<number | null>(null)

const batchForm = reactive({
  batchNo: '',
  productId: undefined as number | undefined,
  warehouseId: undefined as number | undefined,
  colorNo: '',
  dyeLotNo: '',
  grade: '一等品',
  quantityMeters: 0,
  quantityKg: 0,
  gramWeight: undefined as number | undefined,
  width: undefined as number | undefined,
  productionDate: '',
  remarks: '',
})

const transferForm = reactive({
  toWarehouseId: undefined as number | undefined,
  quantityMeters: 0,
  quantityKg: 0,
  remarks: '',
})

const batchRules = {
  batchNo: [{ required: true, message: '请输入批次号', trigger: 'blur' }],
  productId: [{ required: true, message: '请选择产品', trigger: 'change' }],
  warehouseId: [{ required: true, message: '请选择仓库', trigger: 'change' }],
  colorNo: [{ required: true, message: '请输入色号', trigger: 'blur' }],
  grade: [{ required: true, message: '请选择等级', trigger: 'change' }],
  quantityMeters: [{ required: true, message: '请输入数量', trigger: 'blur' }],
  quantityKg: [{ required: true, message: '请输入数量', trigger: 'blur' }],
}

const transferRules = {
  toWarehouseId: [{ required: true, message: '请选择目标仓库', trigger: 'change' }],
  quantityMeters: [{ required: true, message: '请输入调拨数量', trigger: 'blur' }],
  quantityKg: [{ required: true, message: '请输入调拨数量', trigger: 'blur' }],
}

const fetchBatches = async () => {
  try {
    const res: any = await listBatches({
      page: pagination.page,
      pageSize: pagination.pageSize,
      ...queryParams,
    })
    if (res.data) {
      batchList.value = res.data!.list || res.data! || []
      pagination.total = res.data!.total || res.data?.length || 0
    }
  } catch (e) {
    ElMessage.error('获取批次列表失败')
  }
}

const handleReset = () => {
  queryParams.batchNo = ''
  queryParams.colorNo = ''
  queryParams.grade = ''
  fetchBatches()
}

const handleCreate = () => {
  isEdit.value = false
  Object.assign(batchForm, {
    batchNo: '',
    productId: undefined,
    warehouseId: undefined,
    colorNo: '',
    dyeLotNo: '',
    grade: '一等品',
    quantityMeters: 0,
    quantityKg: 0,
    gramWeight: undefined,
    width: undefined,
    productionDate: '',
    remarks: '',
  })
  batchDialogVisible.value = true
}

const handleEdit = async (row: InventoryBatch) => {
  if (!row.id) return
  isEdit.value = true

  try {
    const res: any = await getBatch(row.id)
    if (res.data) {
      Object.assign(batchForm, {
        batchNo: res.data.batchNo,
        productId: res.data.productId,
        warehouseId: res.data.warehouseId,
        colorNo: res.data.colorNo,
        dyeLotNo: res.data.dyeLotNo,
        grade: res.data.grade,
        quantityMeters: res.data.quantityMeters,
        quantityKg: res.data.quantityKg,
        gramWeight: res.data.gramWeight,
        width: res.data.width,
        productionDate: res.data.productionDate,
        remarks: res.data.remarks,
      })
    }
  } catch (e) {
    ElMessage.error('获取批次详情失败')
    return
  }

  batchDialogVisible.value = true
}

const handleSaveBatch = async () => {
  if (!batchFormRef.value) return

  await batchFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return

    submitLoading.value = true
    try {
      if (isEdit.value && currentBatch.value) {
        await updateBatch(currentBatch.value.id!, batchForm as any)
        ElMessage.success('更新成功')
      } else {
        await createBatch(batchForm as any)
        ElMessage.success('创建成功')
      }
      batchDialogVisible.value = false
      fetchBatches()
    } catch (e: any) {
      ElMessage.error(e.message || '保存失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const handleView = async (row: InventoryBatch) => {
  if (!row.id) return
  try {
    const res: any = await getBatch(row.id)
    if (res.data) {
      currentBatch.value = res.data!
      viewDialogVisible.value = true
    }
  } catch (e) {
    ElMessage.error('获取批次详情失败')
  }
}

const handleTransfer = (row: InventoryBatch) => {
  if (!row.id) return
  currentTransferBatchId.value = row.id
  Object.assign(transferForm, {
    toWarehouseId: undefined,
    quantityMeters: row.quantityMeters || 0,
    quantityKg: row.quantityKg || 0,
    remarks: '',
  })
  transferDialogVisible.value = true
}

const handleSaveTransfer = async () => {
  if (!transferFormRef.value || !currentTransferBatchId.value) return

  await transferFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return

    submitLoading.value = true
    try {
      await transferBatch(currentTransferBatchId.value!, transferForm as any)
      ElMessage.success('调拨成功')
      transferDialogVisible.value = false
      fetchBatches()
    } catch (e: any) {
      ElMessage.error(e.message || '调拨失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const handleDelete = async (row: InventoryBatch) => {
  if (!row.id) return

  try {
    await ElMessageBox.confirm('确认删除该批次？', '提示', {
      confirmButtonText: '确认',
      cancelButtonText: '取消',
      type: 'warning',
    })

    await deleteBatch(row.id)
    ElMessage.success('删除成功')
    fetchBatches()
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(e.message || '删除失败')
    }
  }
}

onMounted(() => {
  fetchBatches()
})
</script>

<style scoped>
.inventory-batch .card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.inventory-batch .toolbar {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 16px;
}

.inventory-batch .el-table {
  margin-bottom: 16px;
}
</style>
