<template>
  <div class="bpm-page">
    <div class="page-header">
      <div class="header-left">
        <h1 class="page-title">审批管理</h1>
        <el-breadcrumb separator="/">
          <el-breadcrumb-item :to="{ path: '/' }">首页</el-breadcrumb-item>
          <el-breadcrumb-item>审批管理</el-breadcrumb-item>
          <el-breadcrumb-item>我的审批</el-breadcrumb-item>
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
              <div class="stat-label">待审批</div>
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
              <div class="stat-label">已完成</div>
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
              <div class="stat-label">紧急任务</div>
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
              <div class="stat-label">平均处理时长</div>
              <div class="stat-value">{{ stats.avgProcessingTime }}h</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-tabs v-model="activeTab" @tab-change="handleTabChange">
      <el-tab-pane label="待审批任务" name="pending">
        <el-card shadow="hover" class="table-card">
          <el-table :data="pendingTasks" stripe aria-label="待审批任务列表">
            <el-table-column prop="task_name" label="任务名称" min-width="180" fixed />
            <el-table-column prop="process_name" label="流程名称" width="150" />
            <el-table-column prop="assignee_name" label="申请人" width="120" />
            <el-table-column prop="created_at" label="申请时间" width="160" />
            <el-table-column prop="due_date" label="截止时间" width="160">
              <template #default="{ row }">
                <span v-if="row.due_date" :class="{ overdue: isOverdue(row.due_date) }">
                  {{ row.due_date }}
                </span>
                <span v-else>-</span>
              </template>
            </el-table-column>
            <el-table-column prop="priority" label="优先级" width="100">
              <template #default="{ row }">
                <el-tag :type="getPriorityType(row.priority)" size="small">
                  {{ getPriorityText(row.priority) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="180" fixed="right">
              <template #default="{ row }">
                <el-button v-permission="'bpm_task:approve'" type="primary" link size="small" @click="handleApprove(row as BPMTask)"
                  >审批</el-button
                >
                <el-button type="warning" link size="small" @click="handleDetail(row as BPMTask)"
                  >详情</el-button
                >
                <el-button v-permission="'bpm_task:transfer'" type="info" link size="small" @click="handleTransfer(row as BPMTask)"
                  >转交</el-button
                >
                <el-button v-permission="'bpm_task:urge'" type="danger" link size="small" @click="handleUrge(row as BPMTask)"
                  >催办</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="我发起的" name="initiated">
        <el-card shadow="hover" class="table-card">
          <el-table :data="initiatedProcesses" stripe aria-label="我发起的流程列表">
            <el-table-column prop="process_name" label="流程名称" min-width="150" />
            <el-table-column prop="business_key" label="业务单号" width="180" />
            <el-table-column prop="start_time" label="发起时间" width="160" />
            <el-table-column prop="status" label="状态" width="100">
              <template #default="{ row }">
                <el-tag :type="getProcessStatusType(row.status)" size="small">
                  {{ getProcessStatusText(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column prop="current_activities" label="当前节点" width="150">
              <template #default="{ row }">
                <span>{{ row.current_activities?.join(', ') || '-' }}</span>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="150">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="handleTrace(row as BPMInstance)"
                  >追溯</el-button
                >
                <el-button v-permission="'bpm_process:cancel'" type="info" link size="small" @click="handleCancel(row as BPMInstance)"
                  >撤回</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="已处理记录" name="processed">
        <el-card shadow="hover" class="table-card">
          <el-table :data="processedTasks" stripe aria-label="已处理任务列表">
            <el-table-column prop="task_name" label="任务名称" min-width="150" />
            <el-table-column prop="process_name" label="流程名称" width="150" />
            <el-table-column prop="start_user_name" label="申请人" width="120" />
            <el-table-column prop="approved_at" label="审批时间" width="160" />
            <el-table-column prop="result" label="审批结果" width="100">
              <template #default="{ row }">
                <el-tag :type="row.result === 'approved' ? 'success' : 'danger'" size="small">
                  {{ row.result === 'approved' ? '同意' : '拒绝' }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column
              prop="comment"
              label="审批意见"
              min-width="150"
              show-overflow-tooltip
            />
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="流程监控" name="monitor">
        <el-card shadow="hover" class="table-card">
          <el-table :data="processInstances" stripe aria-label="流程实例监控列表">
            <el-table-column prop="instance_id" label="实例ID" width="180" />
            <el-table-column prop="process_name" label="流程名称" min-width="150" />
            <el-table-column prop="start_user_name" label="发起人" width="120" />
            <el-table-column prop="start_time" label="开始时间" width="160" />
            <el-table-column prop="end_time" label="结束时间" width="160">
              <template #default="{ row }">
                {{ row.end_time || '-' }}
              </template>
            </el-table-column>
            <el-table-column prop="status" label="状态" width="100">
              <template #default="{ row }">
                <el-tag :type="getProcessStatusType(row.status)" size="small">
                  {{ getProcessStatusText(row.status) }}
                </el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" width="150">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="handleViewProcess(row as BPMInstance)"
                  >查看</el-button
                >
                <el-button type="info" link size="small" @click="handleProcessImage(row as BPMInstance)"
                  >流程图</el-button
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
  const map: Record<string, string> = { high: '高', medium: '中', low: '低' }
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
    running: '进行中',
    completed: '已完成',
    cancelled: '已取消',
    suspended: '已挂起',
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
    ElMessage.error((error instanceof Error ? error.message : String(error)) || '获取待处理任务失败')
    pendingTasks.value = []
  }
}

const fetchInitiatedProcesses = async () => {
  try {
    const res = await getBpmInstanceListForMonitor()
    initiatedProcesses.value = res.data?.list || []
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || '获取发起流程失败')
    initiatedProcesses.value = []
  }
}

const fetchProcessedTasks = async () => {
  try {
    const res = await getBpmTaskList({ status: 'completed' })
    processedTasks.value = res.data?.list || []
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || '获取已处理任务失败')
    processedTasks.value = []
  }
}

const fetchProcessInstances = async () => {
  try {
    const res = await getBpmInstanceListForMonitor()
    processInstances.value = res.data?.list || []
  } catch (error: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (error: any) 改为 unknown + 类型守卫
    ElMessage.error((error instanceof Error ? error.message : String(error)) || '获取流程实例失败')
    processInstances.value = []
  }
}

const handleApprove = async (row: BPMTask) => {
  try {
    await ElMessageBox.confirm('确定审批通过此任务吗？', '确认', { type: 'info' })
    await approveBpmTask({ task_id: row.task_id, comment: '同意' })
    ElMessage.success('审批成功')
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
      ElMessage.warning('未找到流程实例 ID')
      return
    }
    const res = await getBpmInstanceById(String(instanceId))
    const d = res.data
    if (!d) {
      ElMessage.warning('未找到流程详情')
      return
    }
    const lines = [
      `实例 ID：${d.instance_id}`,
      `流程名称：${d.process_name}`,
      `发起人：${d.start_user}`,
      `发起时间：${d.start_time}`,
      `结束时间：${d.end_time || '-'}`,
      `当前状态：${getProcessStatusText(d.status)}`,
      `当前节点：${d.current_activities?.join(', ') || '-'}`,
    ]
    await ElMessageBox.alert(lines.join('\n'), '任务详情', {
      confirmButtonText: '关闭',
    })
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
    const err = e as Error
    ElMessage.error(err.message || '获取详情失败')
  }
}

const handleTransfer = async (row: BPMTask) => {
  try {
    const { value: targetUserId } = await ElMessageBox.prompt('请输入接收人的用户 ID', '转交任务', {
      type: 'info',
      inputPattern: /^\d+$/,
      inputErrorMessage: '请输入有效的用户 ID',
    })
    await transferBpmTask(row.task_id, parseInt(targetUserId), '工作转交')
    ElMessage.success('任务转交成功')
    fetchPendingTasks()
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
  }
}

const handleUrge = async (row: BPMTask) => {
  try {
    await ElMessageBox.confirm('确定催办此任务吗？', '确认', { type: 'warning' })
    await urgeBpmTask(row.task_id)
    ElMessage.success('催办成功')
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
      ElMessage.warning('未找到流程实例 ID')
      return
    }
    const res = await getBpmApprovalChain(String(instanceId))
    const chain = res.data || []
    if (chain.length === 0) {
      await ElMessageBox.alert('暂无审批链记录', '流程追溯', { confirmButtonText: '关闭' })
      return
    }
    const lines = chain.map(
      item =>
        `${item.order}. ${item.approver_name} - ${item.status}${item.comment ? `（${item.comment}）` : ''}${item.approved_at ? ` @ ${item.approved_at}` : ''}`
    )
    await ElMessageBox.alert(lines.join('\n'), `流程追溯：${instanceId}`, {
      confirmButtonText: '关闭',
    })
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
    const err = e as Error
    ElMessage.error(err.message || '获取审批链失败')
  }
}
// 批次 157d-3 修复：接入 cancelInstance API 真实撤回流程
const handleCancel = async (row: BPMInstance) => {
  try {
    const confirmRes = await ElMessageBox.confirm(
      `确认撤回流程 ${row.instance_id}？撤回后该流程将终止，所有待处理任务将被取消。`,
      '撤回确认',
      {
        type: 'warning',
        confirmButtonText: '确定撤回',
        cancelButtonText: '取消',
        inputPlaceholder: '撤回原因（选填）',
        showInput: true,
        inputType: 'textarea',
      }
    )
    const reason =
      typeof confirmRes === 'string' && confirmRes.trim() ? confirmRes.trim() : undefined
    // 后端 cancel_instance 接收 i32 主键 id（非字符串 instance_no）
    await cancelBpmInstance(row.id, reason)
    ElMessage.success('撤回成功')
    // 撤回按钮仅在 "我发起的" tab 内出现，刷新该列表即可
    fetchInitiatedProcesses()
  } catch (e: unknown) {
    if (e === 'cancel' || e === 'close') return
    const err = e as Error
    ElMessage.error(err.message || '撤回失败')
    logger.error('撤回流程失败', err.message)
  }
}
// 批次 157a P1-1 修复：接入 getInstanceDetail API 展示流程实例详情
const handleViewProcess = async (row: BPMInstance) => {
  try {
    const res = await getBpmInstanceById(String(row.instance_id))
    const d = res.data
    if (!d) {
      ElMessage.warning('未找到流程详情')
      return
    }
    const lines = [
      `实例 ID：${d.instance_id}`,
      `流程名称：${d.process_name}`,
      `发起人：${d.start_user}`,
      `发起时间：${d.start_time}`,
      `结束时间：${d.end_time || '-'}`,
      `当前状态：${getProcessStatusText(d.status)}`,
      `当前节点：${d.current_activities?.join(', ') || '-'}`,
    ]
    await ElMessageBox.alert(lines.join('\n'), '流程详情', {
      confirmButtonText: '关闭',
    })
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
    const err = e as Error
    ElMessage.error(err.message || '获取流程详情失败')
  }
}
// 批次 157a P1-1 修复：接入 getProcessVisualization API 展示流程图信息
const handleProcessImage = async (row: BPMInstance) => {
  try {
    const res = await getBpmProcessVisualization(String(row.instance_id))
    const d = res.data
    if (!d) {
      ElMessage.warning('未找到流程图信息')
      return
    }
    const lines = [
      `实例 ID：${d.instance_id}`,
      `流程名称：${d.process_name}`,
      `当前活动：${d.current_activity || '-'}`,
      `活动历史：${d.activity_history?.join(' → ') || '-'}`,
    ]
    await ElMessageBox.alert(lines.join('\n'), `流程图：${row.instance_id}`, {
      confirmButtonText: '关闭',
    })
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
    const err = e as Error
    ElMessage.error(err.message || '获取流程图失败')
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
