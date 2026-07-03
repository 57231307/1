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
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { listBatches, deleteBatch, type InventoryBatch } from '@/api/inventoryBatch'

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
      pageSize: pagination.pageSize,
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
const handleTransfer = (row: InventoryBatch) => {
  ElMessage.info(`为批次 ${row.batchNo} 创建调拨单`)
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
