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
          <el-table :data="pendingTasks" stripe>
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
                <el-button type="primary" link size="small" @click="handleApprove(row as any)"
                  >审批</el-button
                >
                <el-button type="warning" link size="small" @click="handleDetail(row as any)"
                  >详情</el-button
                >
                <el-button type="info" link size="small" @click="handleTransfer(row as any)"
                  >转交</el-button
                >
                <el-button type="danger" link size="small" @click="handleUrge(row as any)"
                  >催办</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="我发起的" name="initiated">
        <el-card shadow="hover" class="table-card">
          <el-table :data="initiatedProcesses" stripe>
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
                <el-button type="primary" link size="small" @click="handleTrace(row as any)"
                  >追溯</el-button
                >
                <el-button type="info" link size="small" @click="handleCancel(row as any)"
                  >撤回</el-button
                >
              </template>
            </el-table-column>
          </el-table>
        </el-card>
      </el-tab-pane>

      <el-tab-pane label="已处理记录" name="processed">
        <el-card shadow="hover" class="table-card">
          <el-table :data="processedTasks" stripe>
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
          <el-table :data="processInstances" stripe>
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
                <el-button type="primary" link size="small" @click="handleViewProcess(row as any)"
                  >查看</el-button
                >
                <el-button type="info" link size="small" @click="handleProcessImage(row as any)"
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
import { bpmApi } from '@/api/bpm'
import { logger } from '@/utils/logger'

const activeTab = ref('pending')

const stats = ref({
  pendingTasks: 8,
  completedTasks: 156,
  urgentTasks: 3,
  avgProcessingTime: 4.5,
})

const pendingTasks = ref<any[]>([])
const initiatedProcesses = ref<any[]>([])
const processedTasks = ref<any[]>([])
const processInstances = ref<any[]>([])

const getPriorityType = (priority: string) => {
  const map: Record<string, any> = { high: 'danger', medium: 'warning', low: 'info' }
  return map[priority] || 'info'
}

const getPriorityText = (priority: string) => {
  const map: Record<string, string> = { high: '高', medium: '中', low: '低' }
  return map[priority] || priority
}

const getProcessStatusType = (status: string) => {
  const map: Record<string, any> = {
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
    const res = await bpmApi.queryTasks({ status: 'pending' })
    pendingTasks.value = res.data?.list || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取待处理任务失败')
    pendingTasks.value = []
  }
}

const fetchInitiatedProcesses = async () => {
  try {
    const res = await bpmApi.listInstancesForMonitor()
    initiatedProcesses.value = res.data?.list || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取发起流程失败')
    initiatedProcesses.value = []
  }
}

const fetchProcessedTasks = async () => {
  try {
    const res = await bpmApi.queryTasks({ status: 'completed' })
    processedTasks.value = res.data?.list || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取已处理任务失败')
    processedTasks.value = []
  }
}

const fetchProcessInstances = async () => {
  try {
    const res = await bpmApi.listInstancesForMonitor()
    processInstances.value = res.data?.list || []
  } catch (error: any) {
    ElMessage.error(error.message || '获取流程实例失败')
    processInstances.value = []
  }
}

const handleApprove = async (row: any) => {
  try {
    await ElMessageBox.confirm('确定审批通过此任务吗？', '确认', { type: 'info' })
    await bpmApi.approveTask({ task_id: row.task_id, comment: '同意' })
    ElMessage.success('审批成功')
    fetchPendingTasks()
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
  }
}

const handleDetail = (row: any) => {
  ElMessage.info(`查看详情：${row.business_key}`)
}

const handleTransfer = async (row: any) => {
  try {
    const { value: targetUserId } = await ElMessageBox.prompt('请输入接收人的用户 ID', '转交任务', {
      type: 'info',
      inputPattern: /^\d+$/,
      inputErrorMessage: '请输入有效的用户 ID',
    })
    await bpmApi.transferTask(row.task_id, parseInt(targetUserId), '工作转交')
    ElMessage.success('任务转交成功')
    fetchPendingTasks()
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
  }
}

const handleUrge = async (row: any) => {
  try {
    await ElMessageBox.confirm('确定催办此任务吗？', '确认', { type: 'warning' })
    await bpmApi.urgeTask(row.task_id)
    ElMessage.success('催办成功')
  } catch (e) {
    if (e !== 'cancel') logger.error(String(e))
  }
}

const handleTrace = (row: any) => {
  ElMessage.info(`追溯流程：${row.instance_id}`)
}
const handleCancel = (row: any) => {
  ElMessage.info(`撤回流程：${row.instance_id}`)
}
const handleViewProcess = (row: any) => {
  ElMessage.info(`查看流程：${row.instance_id}`)
}
const handleProcessImage = (row: any) => {
  ElMessage.info(`查看流程图：${row.instance_id}`)
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
