<!--
  DyeTab.vue - 染色批次 Tab
  来源：原 fabric/index.vue 中 染色批次 tab 内容
  拆分日期：2026-06-15 B3-4
-->
<template>
  <div class="dye-tab">
    <div class="page-header">
      <h2 class="page-title">{{ t('fabric.dyeTab.title') }}</h2>
      <el-button type="primary" @click="openCreate">
        <el-icon><Plus /></el-icon>
        {{ t('fabric.dyeTab.buttonCreate') }}
      </el-button>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="loading" :data="batches" stripe :aria-label="t('fabric.dyeTab.tableAriaLabel')">
        <el-table-column prop="batch_no" :label="t('fabric.dyeTab.columnBatchNo')" width="140" />
        <el-table-column prop="color_name" :label="t('fabric.dyeTab.columnColor')" width="120" />
        <el-table-column prop="greige_fabric_name" :label="t('fabric.dyeTab.columnGreige')" width="150" />
        <el-table-column prop="planned_quantity" :label="t('fabric.dyeTab.columnPlannedQuantity')" width="100" align="right" />
        <el-table-column prop="actual_quantity" :label="t('fabric.dyeTab.columnActualQuantity')" width="100" align="right" />
        <el-table-column prop="status" :label="t('fabric.dyeTab.columnStatus')" width="100" align="center">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)" size="small">
              {{ getStatusLabel(row.status) }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="start_date" :label="t('fabric.dyeTab.columnStartDate')" width="120" />
        <el-table-column :label="t('fabric.dyeTab.columnAction')" width="200" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openEdit(row)">{{ t('fabric.dyeTab.buttonEdit') }}</el-button>
            <el-button
              v-if="row.status === 'in_progress'"
              type="success"
              link
              size="small"
              @click="handleComplete(row)"
              >{{ t('fabric.dyeTab.buttonComplete') }}</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, defineEmits } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import { completeDyeBatch, type DyeBatch } from '@/api/dye-batch'
import { logger } from '@/utils/logger'

const { t } = useI18n({ useScope: 'global' })

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
    pending: t('fabric.dyeTab.statusPending'),
    in_progress: t('fabric.dyeTab.statusInProgress'),
    completed: t('fabric.dyeTab.statusCompleted'),
    cancelled: t('fabric.dyeTab.statusCancelled'),
  }
  return map[status] || status
}

const fetchBatches = async () => {
  loading.value = true
  try {
    const { getDyeBatchList } = await import('@/api/dye-batch')
    const res = await getDyeBatchList()
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
    await ElMessageBox.confirm(t('fabric.dyeTab.confirmCompleteContent'), t('fabric.common.confirmTitle'), { type: 'info' })
    await completeDyeBatch(row.id)
    ElMessage.success(t('fabric.common.success'))
    fetchBatches()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || t('fabric.common.failed'))
    }
  }
}

onMounted(() => fetchBatches())

defineExpose({ fetchBatches })
</script>
