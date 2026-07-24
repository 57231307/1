<!--
  bpm/index.vue - 审批管理
  D05 Batch 2：接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts + en-US.ts
-->
<template>
  <div class="bpm-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">{{ $t('bpm.title') }}</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">{{ $t('bpm.breadcrumb.home') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('bpm.breadcrumb.bpm') }}</el-breadcrumb-item>
          <el-breadcrumb-item>{{ $t('bpm.breadcrumb.myApproval') }}</el-breadcrumb-item>
        </el-breadcrumb>
      </div>
    </div>

    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card warning">
          <div class="stat-content">
            <div class="stat-icon pending-icon">
              <el-icon><Clock /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ $t('bpm.stats.pendingTasks') }}</div>
              <div class="stat-value">{{ stats.pendingTasks }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon completed-icon">
              <el-icon><CircleCheck /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ $t('bpm.stats.completedTasks') }}</div>
              <div class="stat-value">{{ stats.completedTasks }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card highlight">
          <div class="stat-content">
            <div class="stat-icon urgent-icon">
              <el-icon><Warning /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ $t('bpm.stats.urgentTasks') }}</div>
              <div class="stat-value">{{ stats.urgentTasks }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon avg-icon">
              <el-icon><Timer /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">{{ $t('bpm.stats.avgProcessingTime') }}</div>
              <div class="stat-value">{{ stats.avgProcessingTime }}h</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-tabs v-model="activeTab" @tab-change="handleTabChange">
      <el-tab-pane :label="$t('bpm.tab.pending')" name="pending">
        <el-card shadow="hover" class="table-card">
          <el-table :data="pendingTasks" stripe :aria-label="$t('bpm.pendingTable.ariaLabel')">
            <el-table-column prop="task_name" :label="$t('bpm.pendingTable.taskName')" min-width="180" fixed />
            <el-table-column prop="process_name" :label="$t('bpm.pendingTable.processName')" width="150" />
            <el-table-column prop="assignee_name" :label="$t('bpm.pendingTable.applicant')" width="120" />
            <el-table-column prop="created_at" :label="$t('bpm.pendingTable.applyTime')" width="160" />
            <el-table-column prop="due_date" :label="$t('bpm.pendingTable.dueDate')" width="160">
              <template #default="{ row }">
                <span v-if="row.due_date" :class="{ overdue: isOverdue(row.due_date) }">
                  {{ row.due_date }}
                </span>
                <span v-else>-</span>
              </template>
            </el-table-column>
            <el-table-column prop="priority" :label="$t('bpm.pendingTable.priority')" width="100">
              <template #default="{ row }">
                <el-tag :type="getPriorityType(row.priority)" size="small">
                  {{ getPriorityText(row.priority) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column :label="$t('bpm.pendingTable.operation')" width="180" fixed="right">
              <template #default="{ row }">
                <el-button v-permission="'bpm_task:approve'" type="primary" link size="small" @click="handleApprove(row as BPMTask)"
                  >{{ $t('bpm.pendingTable.approve') }}</el-button
                >
                <el-button type="warning" link size="small" @click="handleDetail(row as BPMTask)"
                  >{{ $t('bpm.pendingTable.detail') }}</el-button
                >
                <el-button v-permission="'bpm_task:transfer'" type="info" link size="small" @click="handleTransfer(row as BPMTask)"
                  >{{ $t('bpm.pendingTable.transfer') }}</el-button
                >
                <el-button v-permission="'bpm_task:urge'" type="danger" link size="small" @click="handleUrge(row as BPMTask)"
                  >{{ $t('bpm.pendingTable.urge') }}</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane :label="$t('bpm.tab.initiated')" name="initiated">
        <el-card shadow="hover" class="table-card">
          <el-table :data="initiatedProcesses" stripe :aria-label="$t('bpm.initiatedTable.ariaLabel')">
            <el-table-column prop="process_name" :label="$t('bpm.initiatedTable.processName')" min-width="150" />
            <el-table-column prop="business_key" :label="$t('bpm.initiatedTable.businessKey')" width="180" />
            <el-table-column prop="start_time" :label="$t('bpm.initiatedTable.startTime')" width="160" />
            <el-table-column prop="status" :label="$t('bpm.initiatedTable.status')" width="100">
              <template #default="{ row }">
                <el-tag :type="getProcessStatusType(row.status)" size="small">
                  {{ getProcessStatusText(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="current_activities" :label="$t('bpm.initiatedTable.currentNode')" width="150">
              <template #default="{ row }">
                <span>{{ row.current_activities?.join(', ') || '-' }}</span>
              </template>
            </el-table-column>
            <el-table-column :label="$t('bpm.initiatedTable.operation')" width="150">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="handleTrace(row as BPMInstance)"
                  >{{ $t('bpm.initiatedTable.trace') }}</el-button
                >
                <el-button v-permission="'bpm_process:cancel'" type="info" link size="small" @click="handleCancel(row as BPMInstance)"
                  >{{ $t('bpm.initiatedTable.cancel') }}</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane :label="$t('bpm.tab.processed')" name="processed">
        <el-card shadow="hover" class="table-card">
          <el-table :data="processedTasks" stripe :aria-label="$t('bpm.processedTable.ariaLabel')">
            <el-table-column prop="task_name" :label="$t('bpm.processedTable.taskName')" min-width="150" />
            <el-table-column prop="process_name" :label="$t('bpm.processedTable.processName')" width="150" />
            <el-table-column prop="start_user_name" :label="$t('bpm.processedTable.applicant')" width="120" />
            <el-table-column prop="approved_at" :label="$t('bpm.processedTable.approvedAt')" width="160" />
            <el-table-column prop="result" :label="$t('bpm.processedTable.result')" width="100">
              <template #default="{ row }">
                <el-tag :type="row.result === 'approved' ? 'success' : 'danger'" size="small">
                  {{ row.result === 'approved' ? $t('bpm.processedTable.approved') : $t('bpm.processedTable.rejected') }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column
              prop="comment"
              :label="$t('bpm.processedTable.comment')"
              min-width="150"
              show-overflow-tooltip
            />
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane :label="$t('bpm.tab.monitor')" name="monitor">
        <el-card shadow="hover" class="table-card">
          <el-table :data="processInstances" stripe :aria-label="$t('bpm.monitorTable.ariaLabel')">
            <el-table-column prop="instance_id" :label="$t('bpm.monitorTable.instanceId')" width="180" />
            <el-table-column prop="process_name" :label="$t('bpm.monitorTable.processName')" min-width="150" />
            <el-table-column prop="start_user_name" :label="$t('bpm.monitorTable.startUser')" width="120" />
            <el-table-column prop="start_time" :label="$t('bpm.monitorTable.startTime')" width="160" />
            <el-table-column prop="end_time" :label="$t('bpm.monitorTable.endTime')" width="160">
              <template #default="{ row }">
                {{ row.end_time || '-' }}
              </template>
            </el-table-column>
            <el-table-column prop="status" :label="$t('bpm.monitorTable.status')" width="100">
              <template #default="{ row }">
                <el-tag :type="getProcessStatusType(row.status)" size="small">
                  {{ getProcessStatusText(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column :label="$t('bpm.monitorTable.operation')" width="150">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="handleViewProcess(row as BPMInstance)"
                  >{{ $t('bpm.monitorTable.view') }}</el-button
                >
                <el-button type="info" link size="small" @click="handleProcessImage(row as BPMInstance)"
                  >{{ $t('bpm.monitorTable.processImage') }}</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>
    </el-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import { Clock, CircleCheck, Warning, Timer } from '@element-plus/icons-vue'
// D14 Batch 5b：原 bpmApi 对象已转风格 B 函数
import {
  getBpmTaskList,
  getBpmInstanceListForMonitor,
  approveBpmTask,
  getBpmInstanceById,
  transferBpmTask,
  urgeBpmTask,
  getBpmApprovalChain,
  getBpmProcessVisualization,
  cancelBpmInstance,
} from '@/api/bpm'
import type { BPMTask, BPMInstance } from '@/api/bpm'
import { logger } from '@/utils/logger'

const { t } = useI18n({ useScope: 'global' })

// v11 批次 162 P2-1 修复：el-tag type 联合字面量类型，替代 Record<string, any>
type TagType = 'success' | 'warning' | 'info' | 'primary' | 'danger'

const activeTab = ref('pending')

const stats = ref({
  pendingTasks: 8,
  completedTasks: 156,
  urgentTasks: 3,
  avgProcessingTime: 4.5,
})

// v11 批次 162 P2-1 修复：any[] 改为具体类型 BPMTask[]/BPMInstance[]
const pendingTasks = ref<BPMTask[]>([])
const initiatedProcesses = ref<BPMInstance[]>([])
const processedTasks = ref<BPMTask[]>([])
const processInstances = ref<BPMInstance[]>([])

const getPriorityType = (priority: string): TagType => {
  const map: Record<string, TagType> = { high: 'danger', medium: 'warning', low: 'info' }
  return map[priority] || 'info'
}

const getPriorityText = (priority: string) => {
  const map: Record<string, string> = {
    high: t('bpm.priority.high'),
    medium: t('bpm.priority.medium'),
    low: t('bpm.priority.low'),
  }
  return map[priority] || priority
}

const getProcessStatusType = (status: string): TagType => {
  const map: Record<string, TagType> = {
    running: 'primary',
    completed: 'success',
    cancelled: 'info',
    suspended: 'warning',
  }
  return map[status] || 'info'
}

const getProcessStatusText = (status: string) => {
  const map: Record<string, string> = {
    running: t('bpm.processStatus.running'),
    completed: t('bpm.processStatus.completed'),
    cancelled: t('bpm.processStatus.cancelled'),
    suspended: t('bpm.processStatus.suspended'),
  }
  return map[status] || status
}

const isOverdue = (dueDate: string) => {
  return new Date(dueDate) < new Date()
}

const handleTabChange = (tabName: string) => {
  if (tabName === 'pending') {
    fetchPendingTasks()
  } else if (tabName === 'initiated') {
    fetchInitiatedProcesses()
  } else if (tabName === 'processed') {
    fetchProcessedTasks()
  } else if (tabName === 'monitor') {
    fetchProcessInstances()
  }
}

const fetchPendingTasks = async () => {
  try {
    const res = await getBpmTaskList({ status: 'pending' })
    pendingTasks.value = res.data?.list || []
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || t('bpm.message.fetchPendingFailed'))
    pendingTasks.value = []
  }
}

const fetchInitiatedProcesses = async () => {
  try {
    const res = await getBpmInstanceListForMonitor()
    initiatedProcesses.value = res.data?.list || []
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || t('bpm.message.fetchInitiatedFailed'))
    initiatedProcesses.value = []
  }
}

const fetchProcessedTasks = async () => {
  try {
    const res = await getBpmTaskList({ status: 'completed' })
    processedTasks.value = res.data?.list || []
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || t('bpm.message.fetchProcessedFailed'))
    processedTasks.value = []
  }
}

const fetchProcessInstances = async () => {
  try {
    const res = await getBpmInstanceListForMonitor()
    processInstances.value = res.data?.list || []
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || t('bpm.message.fetchInstancesFailed'))
    processInstances.value = []
  }
}

const handleApprove = async (row: BPMTask) => {
  try {
    await ElMessageBox.confirm(t('bpm.message.approveConfirm'), t('bpm.message.approveConfirmTitle'), { type: 'info' })
    await approveBpmTask({ task_id: row.task_id, comment: t('bpm.message.approveComment') })
    ElMessage.success(t('bpm.message.approveSuccess'))
    fetchPendingTasks()
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
  }
}

// 批次 157a P1-1 修复：接入 getInstanceDetail API 展示任务关联的流程实例详情
const handleDetail = async (row: BPMTask) => {
  try {
    const instanceId = row.process_instance_id
    if (!instanceId) {
      ElMessage.warning(t('bpm.message.instanceIdNotFound'))
      return
    }
    const res = await getBpmInstanceById(String(instanceId))
    const d = res.data
    if (!d) {
      ElMessage.warning(t('bpm.message.instanceDetailNotFound'))
      return
    }
    const lines = [
      `${t('bpm.detail.instanceId')}：${d.instance_id}`,
      `${t('bpm.detail.processName')}：${d.process_name}`,
      `${t('bpm.detail.startUser')}：${d.start_user}`,
      `${t('bpm.detail.startTime')}：${d.start_time}`,
      `${t('bpm.detail.endTime')}：${d.end_time || '-'}`,
      `${t('bpm.detail.currentStatus')}：${getProcessStatusText(d.status)}`,
      `${t('bpm.detail.currentNode')}：${d.current_activities?.join(', ') || '-'}`,
    ]
    await ElMessageBox.alert(lines.join('\n'), t('bpm.detail.taskDetailTitle'), {
      confirmButtonText: t('bpm.message.close'),
    })
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
    const err = e as Error
    ElMessage.error(err.message || t('bpm.message.fetchDetailFailed'))
  }
}

const handleTransfer = async (row: BPMTask) => {
  try {
    const { value: targetUserId } = await ElMessageBox.prompt(t('bpm.message.transferPrompt'), t('bpm.message.transferTitle'), {
      type: 'info',
      inputPattern: /^\d+$/,
      inputErrorMessage: t('bpm.message.transferUserIdInvalid'),
    })
    await transferBpmTask(row.task_id, parseInt(targetUserId), t('bpm.message.transferComment'))
    ElMessage.success(t('bpm.message.transferSuccess'))
    fetchPendingTasks()
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
  }
}

const handleUrge = async (row: BPMTask) => {
  try {
    await ElMessageBox.confirm(t('bpm.message.urgeConfirm'), t('bpm.message.urgeConfirmTitle'), { type: 'warning' })
    await urgeBpmTask(row.task_id)
    ElMessage.success(t('bpm.message.urgeSuccess'))
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
  }
}

// 批次 157a P1-1 修复：接入 getApprovalChain API 展示流程审批链追溯
// v11 批次 162 P2-1 修复：row 联合类型 BPMTask | BPMInstance，替代 any
const handleTrace = async (row: BPMTask | BPMInstance) => {
  try {
    const instanceId = 'instance_id' in row ? row.instance_id : row.process_instance_id
    if (!instanceId) {
      ElMessage.warning(t('bpm.message.instanceIdNotFound'))
      return
    }
    const res = await getBpmApprovalChain(String(instanceId))
    const chain = res.data || []
    if (chain.length === 0) {
      await ElMessageBox.alert(t('bpm.message.noApprovalChain'), t('bpm.message.approvalChainTitle'), { confirmButtonText: t('bpm.message.close') })
      return
    }
    const lines = chain.map(
      item =>
        `${item.order}. ${item.approver_name} - ${item.status}${item.comment ? `（${item.comment}）` : ''}${item.approved_at ? ` @ ${item.approved_at}` : ''}`
    )
    await ElMessageBox.alert(lines.join('\n'), t('bpm.detail.traceTitle', { instanceId }), {
      confirmButtonText: t('bpm.message.close'),
    })
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
    const err = e as Error
    ElMessage.error(err.message || t('bpm.message.fetchApprovalChainFailed'))
  }
}
// 批次 157d-3 修复：接入 cancelInstance API 真实撤回流程
const handleCancel = async (row: BPMInstance) => {
  try {
    const confirmRes = await ElMessageBox.confirm(
      t('bpm.message.cancelConfirm', { instanceId: row.instance_id }),
      t('bpm.message.cancelConfirmTitle'),
      {
        type: 'warning',
        confirmButtonText: t('bpm.message.cancelConfirmButton'),
        cancelButtonText: t('bpm.message.cancelCancelButton'),
        inputPlaceholder: t('bpm.message.cancelReasonPlaceholder'),
        showInput: true,
        inputType: 'textarea',
      }
    )
    const reason =
      typeof confirmRes === 'string' && confirmRes.trim() ? confirmRes.trim() : undefined
    // 后端 cancel_instance 接收 i32 主键 id（非字符串 instance_no）
    await cancelBpmInstance(row.id, reason)
    ElMessage.success(t('bpm.message.cancelSuccess'))
    // 撤回按钮仅在 "我发起的" tab 内出现，刷新该列表即可
    fetchInitiatedProcesses()
  } catch (e: unknown) {
    if (e === 'cancel' || e === 'close') return
    const err = e as Error
    ElMessage.error(err.message || t('bpm.message.cancelFailed'))
    logger.error('撤回流程失败', err.message)
  }
}
// 批次 157a P1-1 修复：接入 getInstanceDetail API 展示流程实例详情
const handleViewProcess = async (row: BPMInstance) => {
  try {
    const res = await getBpmInstanceById(String(row.instance_id))
    const d = res.data
    if (!d) {
      ElMessage.warning(t('bpm.message.instanceDetailNotFound'))
      return
    }
    const lines = [
      `${t('bpm.detail.instanceId')}：${d.instance_id}`,
      `${t('bpm.detail.processName')}：${d.process_name}`,
      `${t('bpm.detail.startUser')}：${d.start_user}`,
      `${t('bpm.detail.startTime')}：${d.start_time}`,
      `${t('bpm.detail.endTime')}：${d.end_time || '-'}`,
      `${t('bpm.detail.currentStatus')}：${getProcessStatusText(d.status)}`,
      `${t('bpm.detail.currentNode')}：${d.current_activities?.join(', ') || '-'}`,
    ]
    await ElMessageBox.alert(lines.join('\n'), t('bpm.detail.processDetailTitle'), {
      confirmButtonText: t('bpm.message.close'),
    })
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
    const err = e as Error
    ElMessage.error(err.message || t('bpm.message.fetchProcessDetailFailed'))
  }
}
// 批次 157a P1-1 修复：接入 getProcessVisualization API 展示流程图信息
const handleProcessImage = async (row: BPMInstance) => {
  try {
    const res = await getBpmProcessVisualization(String(row.instance_id))
    const d = res.data
    if (!d) {
      ElMessage.warning(t('bpm.message.processImageNotFound'))
      return
    }
    const lines = [
      `${t('bpm.detail.instanceId')}：${d.instance_id}`,
      `${t('bpm.detail.processName')}：${d.process_name}`,
      `${t('bpm.detail.currentActivity')}：${d.current_activity || '-'}`,
      `${t('bpm.detail.activityHistory')}：${d.activity_history?.join(' → ') || '-'}`,
    ]
    await ElMessageBox.alert(lines.join('\n'), t('bpm.detail.processImageTitle', { instanceId: row.instance_id }), {
      confirmButtonText: t('bpm.message.close'),
    })
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
    const err = e as Error
    ElMessage.error(err.message || t('bpm.message.fetchProcessImageFailed'))
  }
}

onMounted(() => {
  fetchPendingTasks()
})
</script>

<style scoped>
.bpm-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 24px;
}
.header-left .page-title {
  font-size: 28px;
  font-weight: 600;
  color: #303133;
  margin: 0 0 12px 0;
}
.stats-row {
  margin-bottom: 20px;
}
.stat-card {
  border-radius: 12px;
  transition: all 0.3s ease;
}
.stat-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}
.stat-card.highlight {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
}
.stat-card.highlight .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}
.stat-card.highlight .stat-label,
.stat-card.highlight .stat-value {
  color: white;
}
.stat-card.warning {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
}
.stat-card.warning .stat-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}
.stat-card.warning .stat-label,
.stat-card.warning .stat-value {
  color: white;
}
.stat-content {
  display: flex;
  align-items: center;
  gap: 16px;
}
.stat-icon {
  width: 56px;
  height: 56px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 28px;
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}
.stat-icon.pending-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}
.stat-icon.completed-icon {
  background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%);
}
.stat-icon.urgent-icon {
  background: rgba(255, 255, 255, 0.2);
  color: white;
}
.stat-icon.avg-icon {
  background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%);
}
.stat-info {
  flex: 1;
}
.stat-label {
  font-size: 14px;
  color: #909399;
  margin-bottom: 4px;
}
.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}
.table-card {
  margin-bottom: 20px;
}
.overdue {
  color: #f56c6c;
  font-weight: 600;
}
:deep(.el-tabs__header) {
  margin-bottom: 20px;
}
:deep(.el-card__header) {
  padding: 16px 20px;
  border-bottom: 1px solid #ebeef5;
}
:deep(.el-card__body) {
  padding: 20px;
}
</style>
