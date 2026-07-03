<!--
  CountListTab.vue - 库存盘点列表 Tab
  来源：原 inventoryCount/index.vue 中 列表/统计/过滤内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="count-list">
    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon total-icon">
              <el-icon><Document /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">盘点单数</div>
              <div class="stat-value">{{ stats.total }}</div>
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
              <div class="stat-label">进行中</div>
              <div class="stat-value">{{ stats.inProgress }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card success">
          <div class="stat-content">
            <div class="stat-icon approved-icon">
              <el-icon><CircleCheck /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">已完成</div>
              <div class="stat-value">{{ stats.completed }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card highlight">
          <div class="stat-content">
            <div class="stat-icon diff-icon">
              <el-icon><DataAnalysis /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">差异数量</div>
              <div class="stat-value">{{ stats.difference }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryParams" class="filter-form">
        <el-form-item label="盘点单号">
          <el-input v-model="queryParams.count_no" placeholder="输入单号" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryParams.status" placeholder="选择状态" clearable>
            <el-option label="进行中" value="in_progress" />
            <el-option label="已完成" value="completed" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleQuery">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
          <!-- P2-10 修复（批次 82 v1 复审）：补齐 v-permission 按钮权限 -->
          <el-button v-permission="'inventory:create'" type="primary" @click="emit('openForm', 'create', null)">
            <el-icon><Plus /></el-icon>新建
          </el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover" class="table-card">
      <el-table v-loading="loading" :data="counts" stripe>
        <el-table-column prop="count_no" label="盘点单号" width="160" fixed />
        <el-table-column prop="count_date" label="盘点日期" width="120" />
        <el-table-column prop="warehouse_name" label="仓库" width="120" />
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="created_by_name" label="创建人" width="100" />
        <el-table-column prop="created_at" label="创建时间" width="160" />
        <el-table-column prop="completed_at" label="完成时间" width="160">
          <template #default="{ row }">{{ row.completed_at || '-' }}</template>
        </el-table-column>
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="emit('openDetail', row)"
              >详情</el-button
            >
            <el-button
              v-if="row.status === 'in_progress'"
              type="primary"
              link
              size="small"
              @click="emit('openForm', 'edit', row)"
              >编辑</el-button
            >
            <el-button
              v-if="row.status === 'in_progress'"
              type="success"
              link
              size="small"
              @click="handleComplete(row)"
              >完成</el-button
            >
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
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted, defineEmits } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Document, Clock, CircleCheck, DataAnalysis, Plus } from '@element-plus/icons-vue'
import {
  listInventoryCounts,
  completeInventoryCount,
  type InventoryCountEntity,
} from '@/api/inventoryCount'

const emit = defineEmits<{
  openForm: [mode: 'create' | 'edit' | 'view', row: InventoryCountEntity | null]
  openDetail: [row: InventoryCountEntity]
}>()

const counts = ref<InventoryCountEntity[]>([])
const loading = ref(false)
const total = ref(0)

const stats = reactive({
  total: 0,
  inProgress: 0,
  completed: 0,
  difference: 0,
})

const queryParams = reactive({
  page: 1,
  page_size: 20,
  count_no: '',
  status: '',
})

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    in_progress: '进行中',
    completed: '已完成',
  }
  return map[status] || status
}
const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    in_progress: 'warning',
    completed: 'success',
  }
  return map[status] || 'info'
}

const fetchCounts = async () => {
  loading.value = true
  try {
    const res = (await listInventoryCounts(queryParams)) as unknown as {
      data?: { list?: InventoryCountEntity[]; total?: number }
    }
    const d = res.data
    counts.value = d?.list || []
    total.value = d?.total || 0
    stats.total = total.value
    stats.inProgress = counts.value.filter(c => c.status === 'in_progress').length
    stats.completed = counts.value.filter(c => c.status === 'completed').length
    stats.difference = 0 // 实际差异数需在 details 弹窗中累加
  } catch (error) {
    ElMessage.error((error as Error).message || '获取盘点单失败')
    counts.value = []
    total.value = 0
  } finally {
    loading.value = false
  }
}

const handleQuery = () => {
  queryParams.page = 1
  fetchCounts()
}
const handleReset = () => {
  queryParams.count_no = ''
  queryParams.status = ''
  handleQuery()
}

const handleComplete = async (row: InventoryCountEntity) => {
  try {
    await ElMessageBox.confirm('确定完成此盘点单吗？完成后将无法编辑。', '确认', {
      type: 'warning',
    })
    await completeInventoryCount(row.id as number)
    ElMessage.success('操作成功')
    fetchCounts()
  } catch (error) {
    if (error !== 'cancel') {
      ElMessage.error((error as Error).message || '操作失败')
    }
  }
}

defineExpose({ fetchCounts })
onMounted(() => fetchCounts())
</script>

<style scoped>
.stats-row {
  margin-bottom: 20px;
}
.stat-card {
  border-radius: 12px;
  transition: all 0.3s;
}
.stat-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}
.stat-card.warning {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}
.stat-card.warning :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
}
.stat-card.success {
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
}
.stat-card.success :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
}
.stat-card.highlight {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}
.stat-card.highlight :deep(.stat-icon) {
  background: rgba(255, 255, 255, 0.2);
}
.stat-card.warning :deep(.stat-label),
.stat-card.warning :deep(.stat-value),
.stat-card.success :deep(.stat-label),
.stat-card.success :deep(.stat-value),
.stat-card.highlight :deep(.stat-label),
.stat-card.highlight :deep(.stat-value) {
  color: white;
}
:deep(.stat-content) {
  display: flex;
  align-items: center;
  gap: 16px;
}
:deep(.stat-icon) {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 28px;
  color: white;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}
:deep(.stat-icon.total-icon) {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}
:deep(.stat-icon.pending-icon),
:deep(.stat-icon.approved-icon),
:deep(.stat-icon.diff-icon) {
  background: rgba(255, 255, 255, 0.2);
}
:deep(.stat-info) {
  flex: 1;
}
:deep(.stat-label) {
  font-size: 14px;
  color: #909399;
  margin-bottom: 4px;
}
:deep(.stat-value) {
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
</style>
