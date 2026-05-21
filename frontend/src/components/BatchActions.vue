<template>
  <div v-if="selectedRows.length > 0" class="batch-actions">
    <div class="batch-actions-bar">
      <span class="selected-info">
        已选择 <strong>{{ selectedRows.length }}</strong> 项
      </span>
      <el-space wrap>
        <el-button
          v-for="action in computedActions"
          :key="action.key"
          :type="action.type"
          :disabled="action.disabled"
          @click="handleAction(action)"
        >
          <el-icon v-if="action.icon"><component :is="action.icon" /></el-icon>
          {{ action.label }}
        </el-button>
        <el-button type="info" @click="handleClear">取消选择</el-button>
      </el-space>
    </div>

    <el-dialog
      v-model="confirmDialogVisible"
      :title="currentAction?.confirmTitle || '确认操作'"
      width="500px"
      :close-on-click-modal="false"
    >
      <div class="confirm-content">
        <p>{{ currentAction?.confirmMessage || `确定要对选中的 ${selectedRows.length} 项执行此操作吗？` }}</p>
        <el-alert
          v-if="currentAction?.warningMessage"
          :title="currentAction.warningMessage"
          type="warning"
          :closable="false"
          show-icon
          class="mb-10"
        />
      </div>
      <template #footer>
        <el-button @click="confirmDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="executing" @click="executeAction">
          确认执行
        </el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="progressDialogVisible"
      title="执行进度"
      width="500px"
      :close-on-click-modal="false"
      :show-close="false"
    >
      <div class="progress-content">
        <el-progress
          :percentage="progressPercent"
          :status="progressStatus"
          :stroke-width="20"
        />
        <p class="progress-text">{{ progressText }}</p>
      </div>
      <template #footer>
        <el-button
          v-if="progressStatus === 'success' || progressStatus === 'exception'"
          type="primary"
          @click="progressDialogVisible = false"
        >
          关闭
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { ElMessage } from 'element-plus'
import {
  Delete,
  Check,
  Edit
} from '@element-plus/icons-vue'

export interface BatchActionItem {
  key: string
  label: string
  type?: 'primary' | 'success' | 'warning' | 'danger' | 'info'
  icon?: any
  confirm?: boolean
  confirmTitle?: string
  confirmMessage?: string
  warningMessage?: string
  handler?: (rows: any[]) => Promise<void> | void
  disabled?: boolean
}

interface Props {
  selectedRows: any[]
  actions?: BatchActionItem[]
  showProgress?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  actions: () => [],
  showProgress: true
})

const emit = defineEmits<{
  clear: []
  action: [key: string, rows: any[]]
  complete: [key: string, success: boolean]
}>()

const defaultActions: BatchActionItem[] = [
  {
    key: 'batchDelete',
    label: '批量删除',
    type: 'danger',
    icon: Delete,
    confirm: true,
    confirmTitle: '确认删除',
    confirmMessage: '删除后无法恢复，确定要删除这些数据吗？',
    warningMessage: '此操作不可撤销！',
    handler: async () => {}
  },
  {
    key: 'batchApprove',
    label: '批量审批',
    type: 'success',
    icon: Check,
    confirm: true,
    confirmTitle: '确认审批',
    confirmMessage: '确定要批量审批通过选中的数据吗？',
    handler: async () => {}
  },
  {
    key: 'batchEdit',
    label: '批量修改',
    type: 'primary',
    icon: Edit,
    handler: async () => {}
  }
]

const computedActions = computed(() => {
  if (props.actions.length > 0) return props.actions
  return defaultActions
})

const confirmDialogVisible = ref(false)
const progressDialogVisible = ref(false)
const currentAction = ref<BatchActionItem | null>(null)
const executing = ref(false)
const progressPercent = ref(0)
const progressStatus = ref<'success' | 'exception' | ''>('')
const progressText = ref('')

const handleAction = async (action: BatchActionItem) => {
  currentAction.value = action

  if (action.confirm) {
    confirmDialogVisible.value = true
  } else {
    await executeAction()
  }
}

const executeAction = async () => {
  if (!currentAction.value) return

  confirmDialogVisible.value = false
  executing.value = true

  if (props.showProgress) {
    progressDialogVisible.value = true
    progressPercent.value = 0
    progressStatus.value = ''
    progressText.value = '正在执行...'
  }

  try {
    const total = props.selectedRows.length
    let completed = 0

    if (props.showProgress) {
      const items = [...props.selectedRows]
      for (const item of items) {
        try {
          await currentAction.value.handler?.([item])
        } catch (e) {
          console.error(`处理项失败:`, e)
        }
        completed++
        progressPercent.value = Math.round((completed / total) * 100)
        progressText.value = `已完成 ${completed}/${total}`
      }
    } else {
      await currentAction.value.handler?.(props.selectedRows)
      progressPercent.value = 100
    }

    progressStatus.value = 'success'
    progressText.value = '执行完成'
    emit('complete', currentAction.value.key, true)
    ElMessage.success('操作成功')
    handleClear()
  } catch (error: any) {
    progressStatus.value = 'exception'
    progressText.value = `执行失败: ${error.message || '未知错误'}`
    emit('complete', currentAction.value.key, false)
    ElMessage.error('操作失败')
  } finally {
    executing.value = false
  }
}

const handleClear = () => {
  emit('clear')
}

defineExpose({ confirmDialogVisible, progressDialogVisible })
</script>

<style scoped>
.batch-actions {
  margin-bottom: 16px;
}

.batch-actions-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 12px 16px;
  background: linear-gradient(135deg, #f5f7fa 0%, #e8ebef 100%);
  border-radius: 8px;
  border: 1px solid #dcdfe6;
}

.selected-info {
  font-size: 14px;
  color: #606266;
}

.selected-info strong {
  color: #409eff;
  font-size: 16px;
}

.confirm-content {
  padding: 10px 0;
}

.progress-content {
  padding: 20px 0;
  text-align: center;
}

.progress-text {
  margin-top: 16px;
  font-size: 14px;
  color: #606266;
}
</style>
