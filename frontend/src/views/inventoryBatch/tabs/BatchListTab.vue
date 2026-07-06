<!--
  BatchListTab.vue - 批次列表 Tab
  来源：原 inventoryBatch/index.vue 中 列表/过滤内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="batch-list">
    <el-card shadow="hover">
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
          <!-- P2-10 修复（批次 82 v1 复审）：补齐 v-permission 按钮权限 -->
          <el-button v-permission="'inventory:create'" type="primary" @click="handleCreate">新建批次</el-button>
        </div>
      </div>

      <el-table v-loading="loading" :data="batchList" border stripe>
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
            <el-button v-permission="'inventory:update'" link type="primary" @click="handleEdit(row)">编辑</el-button>
            <el-button v-permission="'inventory:transfer'" link type="primary" @click="handleTransfer(row)">调拨</el-button>
            <el-button v-permission="'inventory:delete'" link type="danger" @click="handleDelete(row)">删除</el-button>
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

    <!-- 批次 157b P1-1 修复：批次调拨对话框 -->
    <el-dialog v-model="transferDialogVisible" title="批次调拨" width="480px">
      <el-form ref="transferFormRef" :model="transferForm" :rules="transferRules" label-width="100px">
        <el-form-item label="调出仓库">
          <el-input :model-value="transferForm.fromWarehouseName" disabled />
        </el-form-item>
        <el-form-item label="目标仓库" prop="toWarehouseId">
          <el-select v-model="transferForm.toWarehouseId" placeholder="请选择目标仓库" style="width: 100%">
            <el-option
              v-for="w in warehouseOptions"
              :key="w.id"
              :label="w.warehouse_name"
              :value="w.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="调拨数量(米)" prop="quantityMeters">
          <el-input-number v-model="transferForm.quantityMeters" :min="0" style="width: 100%" />
        </el-form-item>
        <el-form-item label="调拨数量(kg)" prop="quantityKg">
          <el-input-number v-model="transferForm.quantityKg" :min="0" style="width: 100%" />
        </el-form-item>
        <el-form-item label="备注">
          <el-input v-model="transferForm.remarks" type="textarea" placeholder="选填" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="transferDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="transferSubmitting" @click="onSubmitTransfer">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import {
  listBatches,
  deleteBatch,
  transferBatch,
  type InventoryBatch,
  type TransferBatchRequest,
} from '@/api/inventoryBatch'
import { warehouseApi, type Warehouse } from '@/api/warehouse'

const emit = defineEmits<{ openForm: [row: InventoryBatch | null] }>()

const batchList = ref<InventoryBatch[]>([])
const loading = ref(false)

const queryParams = reactive({
  batchNo: '',
  colorNo: '',
  grade: '',
})

const pagination = reactive({ page: 1, pageSize: 20, total: 0 })

const fetchBatches = async () => {
  loading.value = true
  try {
    const res = (await listBatches({
      page: pagination.page,
      page_size: pagination.pageSize,
      ...queryParams,
    })) as unknown as { data?: { list?: InventoryBatch[]; total?: number } }
    const d = res.data
    batchList.value = d?.list || []
    pagination.total = d?.total || 0
  } catch (error) {
    ElMessage.error((error as Error).message || '获取批次列表失败')
    batchList.value = []
    pagination.total = 0
  } finally {
    loading.value = false
  }
}

const handleReset = () => {
  queryParams.batchNo = ''
  queryParams.colorNo = ''
  queryParams.grade = ''
  pagination.page = 1
  fetchBatches()
}

const handleCreate = () => emit('openForm', null)
const handleView = (row: InventoryBatch) => emit('openForm', row)
const handleEdit = (row: InventoryBatch) => emit('openForm', row)

// 批次 157b P1-1 修复：批次调拨接入 transferBatch API
const transferDialogVisible = ref(false)
const transferSubmitting = ref(false)
const transferFormRef = ref<FormInstance>()
const transferCurrentRow = ref<InventoryBatch | null>(null)
const warehouseOptions = ref<Warehouse[]>([])
const transferForm = reactive<{
  fromWarehouseName: string
  toWarehouseId: number | null
  quantityMeters: number
  quantityKg: number
  remarks: string
}>({
  fromWarehouseName: '',
  toWarehouseId: null,
  quantityMeters: 0,
  quantityKg: 0,
  remarks: '',
})
const transferRules: FormRules = {
  toWarehouseId: [{ required: true, message: '请选择目标仓库', trigger: 'change' }],
  quantityMeters: [{ required: true, message: '请输入调拨数量(米)', trigger: 'blur' }],
  quantityKg: [{ required: true, message: '请输入调拨数量(kg)', trigger: 'blur' }],
}

const fetchWarehouseOptions = async () => {
  try {
    const res = (await warehouseApi.list({ page: 1, page_size: 1000 })) as unknown as {
      data?: { list?: Warehouse[] }
    }
    warehouseOptions.value = res.data?.list || []
  } catch {
    warehouseOptions.value = []
  }
}

const handleTransfer = async (row: InventoryBatch) => {
  transferCurrentRow.value = row
  transferForm.fromWarehouseName = row.warehouseName || '-'
  transferForm.toWarehouseId = null
  transferForm.quantityMeters = row.quantityMeters || 0
  transferForm.quantityKg = row.quantityKg || 0
  transferForm.remarks = ''
  if (warehouseOptions.value.length === 0) {
    await fetchWarehouseOptions()
  }
  transferDialogVisible.value = true
}

const onSubmitTransfer = async () => {
  if (!transferFormRef.value || !transferCurrentRow.value) return
  await transferFormRef.value.validate(async valid => {
    if (!valid) return
    const row = transferCurrentRow.value
    if (!row.warehouseId || !transferForm.toWarehouseId) {
      ElMessage.warning('仓库信息不完整')
      return
    }
    transferSubmitting.value = true
    try {
      const payload: TransferBatchRequest = {
        fromWarehouseId: row.warehouseId,
        toWarehouseId: transferForm.toWarehouseId,
        quantityMeters: transferForm.quantityMeters,
        quantityKg: transferForm.quantityKg,
        remarks: transferForm.remarks || undefined,
      }
      await transferBatch(row.id as number, payload)
      ElMessage.success('批次调拨成功')
      transferDialogVisible.value = false
      fetchBatches()
    } catch (error) {
      ElMessage.error((error as Error).message || '批次调拨失败')
    } finally {
      transferSubmitting.value = false
    }
  })
}

const handleDelete = async (row: InventoryBatch) => {
  try {
    await ElMessageBox.confirm(`确定删除批次 ${row.batchNo} 吗？`, '删除确认', { type: 'warning' })
    await deleteBatch(row.id as number)
    ElMessage.success('删除成功')
    fetchBatches()
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error((error as Error).message || '删除失败')
    }
  }
}

defineExpose({ fetchBatches })
onMounted(() => fetchBatches())
</script>

<style scoped>
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-weight: 600;
}
.toolbar {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 16px;
}
.actions {
  display: flex;
  gap: 8px;
}
</style>
