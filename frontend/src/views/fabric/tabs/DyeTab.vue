<!--
  DyeTab.vue - 染色批次 Tab
  来源：原 fabric/index.vue 中 染色批次 tab 内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="dye-tab">
    <div class="page-header">
      <h2 class="page-title">染色批次管理</h2>
      <el-button type="primary" @click="openCreate">
        <el-icon><Plus /></el-icon>
        新建批次
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="loading" :data="batches" stripe aria-label="染色批次列表">
        <el-table-column prop="batch_no" label="批次号" width="140" />
        <el-table-column prop="color_name" label="颜色" width="120" />
        <el-table-column prop="greige_fabric_name" label="坯布" width="150" />
        <el-table-column prop="planned_quantity" label="计划数量" width="100" align="right" />
        <el-table-column prop="actual_quantity" label="实际数量" width="100" align="right" />
        <el-table-column prop="status" label="状态" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="start_date" label="开始日期" width="120" />
        <el-table-column label="操作" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openEdit(row)">编辑</el-button>
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
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, defineEmits } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { completeDyeBatch, type DyeBatch } from '@/api/dye-batch'
import { logger } from '@/utils/logger'

const emit = defineEmits<{ openDialog: [row: DyeBatch | null] }>()

const batches = ref<DyeBatch[]>([])
const loading = ref(false)

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    pending: 'info',
    in_progress: 'warning',
    completed: 'success',
    cancelled: 'danger',
  }
  return map[status] || 'info'
}

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending: '待处理',
    in_progress: '进行中',
    completed: '已完成',
    cancelled: '已取消',
  }
  return map[status] || status
}

const fetchBatches = async () => {
  loading.value = true
  try {
    const { listDyeBatches } = await import('@/api/dye-batch')
    const res = await listDyeBatches()
    batches.value = (res.data as DyeBatch[] | undefined) || []
  } catch (error) {
    const err = error as Error
    logger.error('获取染色批次失败', err.message)
  } finally {
    loading.value = false
  }
}

const openCreate = () => emit('openDialog', null)
const openEdit = (row: DyeBatch) => emit('openDialog', row)

const handleComplete = async (row: DyeBatch) => {
  try {
    await ElMessageBox.confirm('确定完成此批次吗？', '确认', { type: 'info' })
    await completeDyeBatch(row.id)
    ElMessage.success('操作成功')
    fetchBatches()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '操作失败')
    }
  }
}

onMounted(() => fetchBatches())

defineExpose({ fetchBatches })
</script>
