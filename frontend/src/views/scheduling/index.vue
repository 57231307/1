<template>
  <div class="scheduling-page">
    <div class="page-header">
      <h2>生产排程管理</h2>
      <div class="header-actions">
        <el-button type="primary" :loading="scheduling" @click="handleAutoSchedule">
          <el-icon><Cpu /></el-icon>
          自动排程
        </el-button>
        <el-button @click="$router.push('/scheduling/gantt')">
          <el-icon><TrendCharts /></el-icon>
          查看甘特图
        </el-button>
      </div>
    </div>

    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon pending-icon">
              <el-icon><Clock /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">待排程工单</div>
              <div class="stat-value">{{ stats.pending || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon scheduled-icon">
              <el-icon><Calendar /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">已排程工单</div>
              <div class="stat-value">{{ stats.scheduled || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon running-icon">
              <el-icon><Loading /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">生产中工单</div>
              <div class="stat-value">{{ stats.running || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon conflict-icon">
              <el-icon><Warning /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">冲突数量</div>
              <div class="stat-value">{{ stats.conflicts || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-row :gutter="20">
      <el-col :xs="24" :lg="16">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">
              <span>排程工单列表</span>
              <div class="header-ops">
                <el-select
                  v-model="filterStatus"
                  placeholder="筛选状态"
                  clearable
                  style="width: 140px; margin-right: 8px"
                  @change="fetchTasks"
                >
                  <el-option label="全部" value="" />
                  <el-option label="待排程" value="pending" />
                  <el-option label="已排程" value="scheduled" />
                  <el-option label="生产中" value="running" />
                  <el-option label="已完成" value="completed" />
                  <el-option label="冲突" value="conflict" />
                </el-select>
                <el-button type="primary" link @click="fetchTasks">
                  <el-icon><Refresh /></el-icon>
                  刷新
                </el-button>
              </div>
            </div>
          </template>
          <el-table v-loading="taskLoading" :data="taskList" stripe>
            <el-table-column prop="order_no" label="工单号" width="140" />
            <el-table-column prop="product_name" label="产品名称" width="160" />
            <el-table-column prop="work_center_name" label="工作中心" width="130" />
            <el-table-column prop="quantity" label="数量" width="80" />
            <el-table-column label="开始时间" width="170">
              <template #default="{ row }">{{ formatDateTime(row.start_time) }}</template>
            </el-table-column>
            <el-table-column label="结束时间" width="170">
              <template #default="{ row }">{{ formatDateTime(row.end_time) }}</template>
            </el-table-column>
            <el-table-column prop="duration_hours" label="时长(h)" width="80" />
            <el-table-column label="优先级" width="90">
              <template #default="{ row }">
                <el-tag :type="getPriorityType(row.priority)" size="small"
                  >P{{ row.priority }}</el-tag
                >
              </template>
            </el-table-column>
            <el-table-column label="状态" width="100">
              <template #default="{ row }">
                <el-tag :type="getStatusType(row.status)" effect="light">{{
                  getStatusLabel(row.status)
                }}</el-tag>
              </template>
            </el-table-column>
            <el-table-column label="操作" fixed="right" width="160">
              <template #default="{ row }">
                <el-button type="primary" link size="small" @click="handleAdjust(row)"
                  >调整</el-button
                >
                <el-button
                  v-if="row.has_conflict"
                  type="danger"
                  link
                  size="small"
                  @click="showConflictDetail(row)"
                  >详情</el-button
                >
              </template>
            </el-table-column>
          </el-table>
          <el-pagination
            v-model:current-page="currentPage"
            v-model:page-size="pageSize"
            :total="total"
            :page-sizes="[10, 20, 50]"
            layout="total, sizes, prev, pager, next"
            class="pagination"
            @size-change="fetchTasks"
            @current-change="fetchTasks"
          />
        </el-card>
      </el-col>

      <el-col :xs="24" :lg="8">
        <el-card shadow="hover" class="conflict-card">
          <template #header>
            <div class="card-header">
              <span>冲突检测</span>
              <el-tag :type="conflictList.length > 0 ? 'danger' : 'success'" size="small">
                {{ conflictList.length }} 个冲突
              </el-tag>
            </div>
          </template>
          <div class="conflict-actions">
            <el-button
              type="warning"
              size="small"
              :loading="conflictLoading"
              @click="fetchConflicts"
            >
              <el-icon><Search /></el-icon>
              检测冲突
            </el-button>
          </div>
          <div v-loading="conflictLoading" class="conflict-list">
            <div v-if="conflictList.length === 0" class="empty-state">
              <el-icon><CircleCheck /></el-icon>
              <p>暂无排程冲突</p>
            </div>
            <div v-for="item in conflictList" :key="item.id" class="conflict-item">
              <div class="conflict-header">
                <span class="conflict-wc">{{ item.work_center_name }}</span>
                <el-tag :type="item.severity === 'error' ? 'danger' : 'warning'" size="small">
                  {{ item.severity === 'error' ? '严重' : '警告' }}
                </el-tag>
              </div>
              <div class="conflict-orders">
                <el-tag size="small" type="info">{{ item.order_no_1 }}</el-tag>
                <el-icon style="margin: 0 4px"><Switch /></el-icon>
                <el-tag size="small" type="info">{{ item.order_no_2 }}</el-tag>
              </div>
              <div class="conflict-time">
                <el-icon><Time /></el-icon>
                <span
                  >{{ formatTime(item.overlap_start) }} ~ {{ formatTime(item.overlap_end) }}</span
                >
              </div>
              <div class="conflict-suggestion">
                <el-icon><ChatDotRound /></el-icon>
                <span>{{ item.suggestion }}</span>
              </div>
            </div>
          </div>
        </el-card>

        <el-card shadow="hover" class="param-card">
          <template #header>
            <div class="card-header">
              <span>排程参数</span>
            </div>
          </template>
          <el-form :model="scheduleParams" label-width="90px" size="small">
            <el-form-item label="排程范围">
              <el-date-picker
                v-model="dateRange"
                type="daterange"
                range-separator="至"
                start-placeholder="开始"
                end-placeholder="结束"
                style="width: 100%"
              />
            </el-form-item>
            <el-form-item label="优先级模式">
              <el-select v-model="scheduleParams.priority_mode" style="width: 100%">
                <el-option label="先进先出" value="fifo" />
                <el-option label="优先级优先" value="priority" />
                <el-option label="交期优先" value="due_date" />
              </el-select>
            </el-form-item>
            <el-form-item label="优化目标">
              <el-select v-model="scheduleParams.optimization_target" style="width: 100%">
                <el-option label="最小化空闲" value="min_idle" />
                <el-option label="最小化延迟" value="min_delay" />
                <el-option label="均衡负载" value="balance_load" />
              </el-select>
            </el-form-item>
            <el-form-item>
              <el-button
                type="primary"
                :loading="scheduling"
                style="width: 100%"
                @click="handleAutoSchedule"
              >
                <el-icon><Cpu /></el-icon>
                执行排程
              </el-button>
            </el-form-item>
          </el-form>
        </el-card>
      </el-col>
    </el-row>

    <el-dialog v-model="adjustDialogVisible" title="调整排程时间" width="450px">
      <el-form :model="adjustForm" label-width="100px">
        <el-form-item label="工单号">
          <span>{{ adjustTask?.order_no }}</span>
        </el-form-item>
        <el-form-item label="开始时间">
          <el-date-picker
            v-model="adjustForm.start_time"
            type="datetime"
            placeholder="选择开始时间"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="结束时间">
          <el-date-picker
            v-model="adjustForm.end_time"
            type="datetime"
            placeholder="选择结束时间"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="adjustDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="adjusting" @click="confirmAdjust">确认调整</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import {
  Cpu,
  TrendCharts,
  Clock,
  Calendar,
  Loading,
  Warning,
  Refresh,
  Search,
  CircleCheck,
  Switch,
  ChatDotRound,
} from '@element-plus/icons-vue'
import {
  schedulingApi,
  type ScheduleTask,
  type ConflictItem,
  type SchedulingParams,
} from '@/api/scheduling'

const scheduling = ref(false)
const adjusting = ref(false)
const taskLoading = ref(false)
const conflictLoading = ref(false)
const currentPage = ref(1)
const pageSize = ref(10)
const total = ref(0)
const filterStatus = ref('')

const stats = ref({
  pending: 0,
  scheduled: 0,
  running: 0,
  conflicts: 0,
})

const taskList = ref<ScheduleTask[]>([])
const conflictList = ref<ConflictItem[]>([])

const dateRange = ref<[Date, Date] | null>(null)
const scheduleParams = ref<SchedulingParams>({
  start_date: '',
  end_date: '',
  priority_mode: 'priority',
  optimization_target: 'balance_load',
})

const adjustDialogVisible = ref(false)
const adjustTask = ref<ScheduleTask | null>(null)
const adjustForm = ref({
  start_time: '',
  end_time: '',
})

const formatDateTime = (t: string) => {
  if (!t) return '-'
  return t.replace('T', ' ').slice(0, 16)
}

const formatTime = (t: string) => {
  if (!t) return '-'
  return t.replace('T', ' ').slice(0, 16)
}

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    pending: 'info',
    scheduled: 'primary',
    running: 'warning',
    completed: 'success',
    conflict: 'danger',
  }
  return map[status] || 'info'
}

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    pending: '待排程',
    scheduled: '已排程',
    running: '生产中',
    completed: '已完成',
    conflict: '冲突',
  }
  return map[status] || status
}

const getPriorityType = (priority: number) => {
  if (priority === 1) return 'danger'
  if (priority === 2) return 'warning'
  return 'info'
}

const fetchTasks = async () => {
  taskLoading.value = true
  try {
    const params: Record<string, unknown> = {
      page: currentPage.value,
      page_size: pageSize.value,
    }
    if (filterStatus.value) {
      params.status = filterStatus.value
    }
    const res = await schedulingApi.getScheduleTasks(params)
    taskList.value = res.data!.list
    total.value = res.data!.total
    updateStats()
  } catch (error: any) {
    ElMessage.error(error.message || '获取排程任务失败')
    taskList.value = []
    total.value = 0
  } finally {
    taskLoading.value = false
  }
}

const updateStats = () => {
  stats.value.pending = taskList.value.filter((t) => t.status === 'pending').length
  stats.value.scheduled = taskList.value.filter((t) => t.status === 'scheduled').length
  stats.value.running = taskList.value.filter((t) => t.status === 'running').length
  stats.value.conflicts = conflictList.value.length
}

const fetchConflicts = async () => {
  conflictLoading.value = true
  try {
    const res = await schedulingApi.detectConflicts()
    conflictList.value = res.data!
    stats.value.conflicts = conflictList.value.length
  } catch (error: any) {
    ElMessage.error(error.message || '获取冲突列表失败')
    conflictList.value = []
  } finally {
    conflictLoading.value = false
  }
}

const handleAutoSchedule = async () => {
  scheduling.value = true
  try {
    if (dateRange.value && dateRange.value.length === 2) {
      scheduleParams.value.start_date = dateRange.value[0].toISOString().split('T')[0]
      scheduleParams.value.end_date = dateRange.value[1].toISOString().split('T')[0]
    } else {
      const today = new Date()
      const end = new Date()
      end.setDate(end.getDate() + 30)
      scheduleParams.value.start_date = today.toISOString().split('T')[0]
      scheduleParams.value.end_date = end.toISOString().split('T')[0]
    }
    const res = await schedulingApi.autoSchedule(scheduleParams.value)
    const result = res.data!
    ElMessage.success(`排程完成: ${result.scheduled_count} 个任务, ${result.conflict_count} 个冲突`)
    if (result.conflict_count > 0) {
      conflictList.value = result.conflicts
      stats.value.conflicts = result.conflict_count
    }
    fetchTasks()
  } catch (error: any) {
    ElMessage.error(error.message || '自动排程失败')
  } finally {
    scheduling.value = false
  }
}

const handleAdjust = (task: ScheduleTask) => {
  adjustTask.value = task
  adjustForm.value = {
    start_time: task.start_time,
    end_time: task.end_time,
  }
  adjustDialogVisible.value = true
}

const confirmAdjust = async () => {
  if (!adjustTask.value) return
  adjusting.value = true
  try {
    await schedulingApi.adjustTask(adjustTask.value.id, {
      start_time: adjustForm.value.start_time,
      end_time: adjustForm.value.end_time,
    })
    ElMessage.success('排程调整成功')
    adjustDialogVisible.value = false
    fetchTasks()
  } catch (error: any) {
    ElMessage.error(error.message || '排程调整失败')
  } finally {
    adjusting.value = false
  }
}

const showConflictDetail = (task: ScheduleTask) => {
  ElMessage.warning(`工单 ${task.order_no} 存在排程冲突: ${task.conflict_details || '时间重叠'}`)
}

onMounted(() => {
  fetchTasks()
  fetchConflicts()
})
</script>

<style scoped>
.scheduling-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.page-header h2 {
  font-size: 24px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
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
}

.stat-icon.pending-icon {
  background: linear-gradient(135deg, #a8edea 0%, #fed6e3 100%);
  color: white;
}

.stat-icon.scheduled-icon {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.stat-icon.running-icon {
  background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%);
  color: white;
}

.stat-icon.conflict-icon {
  background: linear-gradient(135deg, #ff9a9e 0%, #fecfef 100%);
  color: white;
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

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-header span {
  font-size: 16px;
  font-weight: 600;
  color: #303133;
}

.header-ops {
  display: flex;
  align-items: center;
}

.pagination {
  margin-top: 16px;
  justify-content: flex-end;
}

.conflict-card,
.param-card {
  margin-bottom: 20px;
  border-radius: 12px;
}

.conflict-actions {
  margin-bottom: 16px;
}

.conflict-list {
  min-height: 200px;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 0;
  color: #909399;
}

.empty-state .el-icon {
  font-size: 48px;
  margin-bottom: 12px;
}

.conflict-item {
  padding: 12px;
  border-left: 3px solid #f56c6c;
  background: #fef0f0;
  border-radius: 4px;
  margin-bottom: 12px;
}

.conflict-item:last-child {
  margin-bottom: 0;
}

.conflict-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}

.conflict-wc {
  font-weight: 600;
  color: #303133;
  font-size: 14px;
}

.conflict-orders {
  display: flex;
  align-items: center;
  margin-bottom: 6px;
}

.conflict-time,
.conflict-suggestion {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #606266;
  margin-bottom: 4px;
}

.conflict-time .el-icon,
.conflict-suggestion .el-icon {
  font-size: 14px;
}

:deep(.el-card__header) {
  padding: 16px 20px;
  border-bottom: 1px solid #ebeef5;
}

:deep(.el-card__body) {
  padding: 20px;
}
</style>
