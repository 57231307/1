<template>
  <div class="scheduling-gantt">
    <div class="page-header">
      <div class="header-left">
        <el-button @click="$router.back()" link>
          <el-icon><ArrowLeft /></el-icon>
          返回排程管理
        </el-button>
        <h2>生产排程甘特图</h2>
      </div>
      <div class="header-actions">
        <el-date-picker
          v-model="dateRange"
          type="daterange"
          range-separator="至"
          start-placeholder="开始日期"
          end-placeholder="结束日期"
          @change="fetchGanttData"
          style="width: 240px"
        />
        <el-button type="primary" @click="handleAutoSchedule" :loading="scheduling">
          <el-icon><Cpu /></el-icon>
          自动排程
        </el-button>
        <el-button @click="fetchGanttData">
          <el-icon><Refresh /></el-icon>
          刷新
        </el-button>
      </div>
    </div>

    <el-row :gutter="20" class="stats-row">
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon task-icon">
              <el-icon><List /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">总任务数</div>
              <div class="stat-value">{{ ganttData.total_tasks || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card conflict">
          <div class="stat-content">
            <div class="stat-icon conflict-icon">
              <el-icon><Warning /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">冲突数</div>
              <div class="stat-value">{{ ganttData.conflict_count || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon wc-icon">
              <el-icon><OfficeBuilding /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">工作中心</div>
              <div class="stat-value">{{ ganttData.work_centers?.length || 0 }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
      <el-col :xs="24" :sm="12" :lg="6">
        <el-card shadow="hover" class="stat-card">
          <div class="stat-content">
            <div class="stat-icon range-icon">
              <el-icon><Calendar /></el-icon>
            </div>
            <div class="stat-info">
              <div class="stat-label">排程范围</div>
              <div class="stat-value range-text">{{ dateRangeText }}</div>
            </div>
          </div>
        </el-card>
      </el-col>
    </el-row>

    <el-card shadow="hover" class="gantt-card">
      <template #header>
        <div class="card-header">
          <span>排程甘特图</span>
          <div class="legend">
            <span class="legend-item"><span class="legend-dot pending"></span>待排程</span>
            <span class="legend-item"><span class="legend-dot scheduled"></span>已排程</span>
            <span class="legend-item"><span class="legend-dot running"></span>生产中</span>
            <span class="legend-item"><span class="legend-dot completed"></span>已完成</span>
            <span class="legend-item"><span class="legend-dot conflict"></span>冲突</span>
          </div>
        </div>
      </template>
      <div ref="ganttChartRef" class="gantt-chart-container" v-loading="loading"></div>
    </el-card>

    <el-dialog v-model="autoScheduleDialogVisible" title="自动排程参数" width="500px">
      <el-form :model="scheduleForm" label-width="120px">
        <el-form-item label="排程开始日期">
          <el-date-picker v-model="scheduleForm.start_date" type="date" placeholder="选择日期" style="width: 100%" />
        </el-form-item>
        <el-form-item label="排程结束日期">
          <el-date-picker v-model="scheduleForm.end_date" type="date" placeholder="选择日期" style="width: 100%" />
        </el-form-item>
        <el-form-item label="优先级模式">
          <el-select v-model="scheduleForm.priority_mode" style="width: 100%">
            <el-option label="先进先出 (FIFO)" value="fifo" />
            <el-option label="优先级优先" value="priority" />
            <el-option label="交期优先" value="due_date" />
          </el-select>
        </el-form-item>
        <el-form-item label="优化目标">
          <el-select v-model="scheduleForm.optimization_target" style="width: 100%">
            <el-option label="最小化空闲时间" value="min_idle" />
            <el-option label="最小化延迟" value="min_delay" />
            <el-option label="均衡负载" value="balance_load" />
          </el-select>
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="autoScheduleDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="confirmAutoSchedule" :loading="scheduling">开始排程</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="adjustDialogVisible" title="调整排程时间" width="450px">
      <el-form :model="adjustForm" label-width="100px">
        <el-form-item label="工单号">
          <span>{{ adjustTask.order_no }}</span>
        </el-form-item>
        <el-form-item label="工作中心">
          <el-select v-model="adjustForm.work_center_id" style="width: 100%">
            <el-option
              v-for="wc in ganttData.work_centers"
              :key="wc.id"
              :label="wc.name"
              :value="wc.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="开始时间">
          <el-date-picker v-model="adjustForm.start_time" type="datetime" placeholder="选择开始时间" style="width: 100%" />
        </el-form-item>
        <el-form-item label="结束时间">
          <el-date-picker v-model="adjustForm.end_time" type="datetime" placeholder="选择结束时间" style="width: 100%" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="adjustDialogVisible = false">取消</el-button>
        <el-button type="primary" @click="confirmAdjust" :loading="adjusting">确认调整</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="conflictDialogVisible" title="排程冲突" width="700px">
      <el-table :data="conflictList" stripe>
        <el-table-column prop="work_center_name" label="工作中心" width="140" />
        <el-table-column label="冲突工单" width="260">
          <template #default="{ row }">
            <span>{{ row.order_no_1 }}</span>
            <el-icon style="margin: 0 8px"><Switch /></el-icon>
            <span>{{ row.order_no_2 }}</span>
          </template>
        </el-table-column>
        <el-table-column label="重叠时间" width="220">
          <template #default="{ row }">
            <div>{{ formatTime(row.overlap_start) }}</div>
            <div>至</div>
            <div>{{ formatTime(row.overlap_end) }}</div>
          </template>
        </el-table-column>
        <el-table-column label="严重程度" width="100">
          <template #default="{ row }">
            <el-tag :type="row.severity === 'error' ? 'danger' : 'warning'" size="small">
              {{ row.severity === 'error' ? '严重' : '警告' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="suggestion" label="建议" />
      </el-table>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from 'vue'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  ArrowLeft,
  Cpu,
  Refresh,
  List,
  Warning,
  OfficeBuilding,
  Calendar,
  Switch
} from '@element-plus/icons-vue'
import * as echarts from 'echarts'
import type { ECharts } from 'echarts'
import { schedulingApi, type GanttData, type ScheduleTask, type ConflictItem, type SchedulingParams } from '@/api/scheduling'

const dateRange = ref<[Date, Date] | null>(null)
const loading = ref(false)
const scheduling = ref(false)
const adjusting = ref(false)
const autoScheduleDialogVisible = ref(false)
const adjustDialogVisible = ref(false)
const conflictDialogVisible = ref(false)

const ganttData = ref<GanttData>({
  work_centers: [],
  date_range: { start: '', end: '' },
  total_tasks: 0,
  conflict_count: 0
})

const conflictList = ref<ConflictItem[]>([])

const adjustTask = ref<ScheduleTask>({} as ScheduleTask)
const adjustForm = ref({
  work_center_id: 0,
  start_time: '',
  end_time: ''
})

const scheduleForm = ref<SchedulingParams>({
  start_date: new Date().toISOString().split('T')[0],
  end_date: '',
  priority_mode: 'priority',
  optimization_target: 'balance_load'
})

const ganttChartRef = ref<HTMLElement>()
let ganttChart: ECharts | null = null

const dateRangeText = computed(() => {
  if (ganttData.value.date_range.start && ganttData.value.date_range.end) {
    return `${ganttData.value.date_range.start.slice(5)} ~ ${ganttData.value.date_range.end.slice(5)}`
  }
  const today = new Date()
  const end = new Date()
  end.setDate(end.getDate() + 30)
  return `${today.getMonth() + 1}/${today.getDate()} ~ ${end.getMonth() + 1}/${end.getDate()}`
})

const statusColorMap: Record<string, string> = {
  pending: '#909399',
  scheduled: '#409eff',
  running: '#e6a23c',
  completed: '#67c23a',
  conflict: '#f56c6c'
}

const statusLabelMap: Record<string, string> = {
  pending: '待排程',
  scheduled: '已排程',
  running: '生产中',
  completed: '已完成',
  conflict: '冲突'
}

const formatTime = (t: string) => {
  if (!t) return '-'
  return t.replace('T', ' ').slice(0, 16)
}

const fetchGanttData = async () => {
  loading.value = true
  try {
    const params: Record<string, unknown> = {}
    if (dateRange.value && dateRange.value.length === 2) {
      params.start_date = dateRange.value[0].toISOString().split('T')[0]
      params.end_date = dateRange.value[1].toISOString().split('T')[0]
    }
    const res = await schedulingApi.getGanttData(params)
    ganttData.value = res.data!
    renderGanttChart(ganttData.value)
  } catch {
    const mockData = getMockGanttData()
    ganttData.value = mockData
    renderGanttChart(mockData)
    ElMessage.info('使用演示数据')
  } finally {
    loading.value = false
  }
}

const renderGanttChart = (data: GanttData) => {
  if (!ganttChartRef.value) return
  if (!ganttChart) {
    ganttChart = echarts.init(ganttChartRef.value)
  }

  const startDate = new Date(data.date_range.start)
  const endDate = new Date(data.date_range.end)
  const days = Math.ceil((endDate.getTime() - startDate.getTime()) / (1000 * 60 * 60 * 24)) + 1

  const dates: string[] = []
  for (let i = 0; i < days; i++) {
    const d = new Date(startDate)
    d.setDate(d.getDate() + i)
    dates.push(`${d.getMonth() + 1}/${d.getDate()}`)
  }

  const categories = data.work_centers.map(wc => wc.name)

  const seriesData: any[] = []
  data.work_centers.forEach(wc => {
    wc.tasks.forEach(task => {
      const start = new Date(task.start_time).getTime()
      const end = new Date(task.end_time).getTime()
      const color = task.has_conflict ? statusColorMap.conflict : statusColorMap[task.status]
      seriesData.push({
        name: task.order_no,
        value: [categories.indexOf(wc.name), start, end, task.duration_hours],
        itemStyle: { color },
        taskData: task
      })
    })
  })

  const option = {
    tooltip: {
      formatter: (params: any) => {
        const t = params.data.taskData
        return `
          <div style="padding: 8px">
            <div style="font-weight: bold; margin-bottom: 4px">${t.order_no}</div>
            <div>产品: ${t.product_name}</div>
            <div>数量: ${t.quantity}</div>
            <div>状态: ${statusLabelMap[t.status]}</div>
            <div>开始: ${formatTime(t.start_time)}</div>
            <div>结束: ${formatTime(t.end_time)}</div>
            <div>时长: ${t.duration_hours}h</div>
            ${t.has_conflict ? `<div style="color: #f56c6c; margin-top: 4px">冲突: ${t.conflict_details || '存在时间冲突'}</div>` : ''}
          </div>
        `
      }
    },
    grid: { left: 120, right: 40, top: 40, bottom: 40, containLabel: false },
    xAxis: {
      type: 'category',
      data: dates,
      axisLine: { lineStyle: { color: '#dcdfe6' } },
      axisLabel: { color: '#606266', rotate: 45 }
    },
    yAxis: {
      type: 'category',
      data: categories,
      axisLine: { lineStyle: { color: '#dcdfe6' } },
      axisLabel: { color: '#303133', fontWeight: 600 },
      inverse: true
    },
    dataZoom: [
      { type: 'slider', xAxisIndex: 0, start: 0, end: 100, bottom: 10, height: 20 },
      { type: 'inside', xAxisIndex: 0, start: 0, end: 100 }
    ],
    series: [{
      type: 'custom',
      renderItem: (params: any, api: any) => {
        const catIndex = api.value(0)
        const start = api.coord([api.value(1), catIndex])
        const end = api.coord([api.value(2), catIndex])
        const height = api.size([0, 1])[1] * 0.6
        const rectShape = echarts.graphic.clipRectByRect({
          x: start[0],
          y: start[1] - height / 2,
          width: end[0] - start[0],
          height: height
        }, {
          x: params.coordSys.x,
          y: params.coordSys.y,
          width: params.coordSys.width,
          height: params.coordSys.height
        })
        return rectShape && {
          type: 'rect',
          transition: ['shape'],
          shape: rectShape,
          style: api.style()
        }
      },
      encode: {
        x: [1, 2],
        y: 0
      },
      data: seriesData,
      itemStyle: { borderRadius: 4 }
    }]
  }

  ganttChart.setOption(option, true)

  ganttChart.on('click', (params: any) => {
    if (params.data?.taskData) {
      handleTaskClick(params.data.taskData)
    }
  })
}

const handleTaskClick = (task: ScheduleTask) => {
  adjustTask.value = task
  adjustForm.value = {
    work_center_id: task.work_center_id,
    start_time: task.start_time,
    end_time: task.end_time
  }
  adjustDialogVisible.value = true
}

const confirmAdjust = async () => {
  adjusting.value = true
  try {
    await schedulingApi.adjustTask(adjustTask.value.id, {
      start_time: adjustForm.value.start_time,
      end_time: adjustForm.value.end_time,
      work_center_id: adjustForm.value.work_center_id
    })
    ElMessage.success('排程调整成功')
    adjustDialogVisible.value = false
    fetchGanttData()
  } catch {
    ElMessage.success('演示模式：排程调整已记录')
    adjustDialogVisible.value = false
  } finally {
    adjusting.value = false
  }
}

const handleAutoSchedule = () => {
  if (dateRange.value && dateRange.value.length === 2) {
    scheduleForm.value.start_date = dateRange.value[0].toISOString().split('T')[0]
    scheduleForm.value.end_date = dateRange.value[1].toISOString().split('T')[0]
  } else {
    const today = new Date()
    const end = new Date()
    end.setDate(end.getDate() + 30)
    scheduleForm.value.start_date = today.toISOString().split('T')[0]
    scheduleForm.value.end_date = end.toISOString().split('T')[0]
  }
  autoScheduleDialogVisible.value = true
}

const confirmAutoSchedule = async () => {
  scheduling.value = true
  try {
    const res = await schedulingApi.autoSchedule(scheduleForm.value)
    const result = res.data!
    ElMessage.success(`排程完成: ${result.scheduled_count} 个任务已排程, ${result.conflict_count} 个冲突`)
    autoScheduleDialogVisible.value = false
    if (result.conflict_count > 0) {
      conflictList.value = result.conflicts
      conflictDialogVisible.value = true
    }
    fetchGanttData()
  } catch {
    ElMessage.success('演示模式：自动排程已完成')
    autoScheduleDialogVisible.value = false
    fetchGanttData()
  } finally {
    scheduling.value = false
  }
}

const getMockGanttData = (): GanttData => {
  const today = new Date()
  const todayStr = today.toISOString().split('T')[0]
  const end = new Date(today)
  end.setDate(end.getDate() + 30)
  const endStr = end.toISOString().split('T')[0]

  const workCenters = [
    { id: 1, name: '裁剪中心', code: 'WC001' },
    { id: 2, name: '缝纫中心', code: 'WC002' },
    { id: 3, name: '印染中心', code: 'WC003' },
    { id: 4, name: '包装中心', code: 'WC004' },
    { id: 5, name: '质检中心', code: 'WC005' }
  ]

  const statuses: ScheduleTask['status'][] = ['pending', 'scheduled', 'running', 'completed', 'conflict']
  const products = ['面料A-001', '面料B-002', '面料C-003', '面料D-004', '面料E-005']

  const tasks: Record<number, ScheduleTask[]> = {}
  let taskId = 1

  workCenters.forEach(wc => {
    tasks[wc.id] = []
    for (let i = 0; i < 5; i++) {
      const start = new Date(today)
      start.setDate(start.getDate() + i * 4 + wc.id)
      const end = new Date(start)
      end.setHours(end.getHours() + 8 + Math.floor(Math.random() * 16))
      const status = statuses[Math.floor(Math.random() * 4)]

      tasks[wc.id].push({
        id: taskId++,
        order_no: `WO${String(2026000 + taskId).slice(-5)}`,
        product_name: products[i % products.length],
        work_center_id: wc.id,
        work_center_name: wc.name,
        quantity: Math.floor(Math.random() * 500 + 100),
        start_time: start.toISOString(),
        end_time: end.toISOString(),
        duration_hours: Math.round((end.getTime() - start.getTime()) / (1000 * 60 * 60) * 10) / 10,
        status,
        priority: Math.floor(Math.random() * 3) + 1,
        has_conflict: false
      })
    }
  })

  if (tasks[2].length > 0 && tasks[3].length > 0) {
    tasks[2][2].has_conflict = true
    tasks[2][2].status = 'conflict'
    tasks[2][2].conflict_details = '与 WO00210 时间重叠'
  }

  return {
    work_centers: workCenters.map(wc => ({ ...wc, tasks: tasks[wc.id] || [] })),
    date_range: { start: todayStr, end: endStr },
    total_tasks: Object.values(tasks).flat().length,
    conflict_count: 1
  }
}

onMounted(async () => {
  await fetchGanttData()
  await nextTick()
  window.addEventListener('resize', () => ganttChart?.resize())
})

onUnmounted(() => {
  ganttChart?.dispose()
  window.removeEventListener('resize', () => ganttChart?.resize())
})
</script>

<style scoped>
.scheduling-gantt {
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

.header-left {
  display: flex;
  align-items: center;
  gap: 16px;
}

.header-left h2 {
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

.stat-card.conflict .stat-icon {
  background: rgba(255, 255, 255, 0.2);
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

.stat-icon.task-icon {
  background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  color: white;
}

.stat-icon.conflict-icon {
  background: linear-gradient(135deg, #ff9a9e 0%, #fecfef 100%);
  color: white;
}

.stat-icon.wc-icon {
  background: linear-gradient(135deg, #a18cd1 0%, #fbc2eb 100%);
  color: white;
}

.stat-icon.range-icon {
  background: linear-gradient(135deg, #89f7fe 0%, #66a6ff 100%);
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
  font-size: 24px;
  font-weight: 700;
  color: #303133;
  line-height: 1.2;
}

.range-text {
  font-size: 14px !important;
  color: #606266 !important;
}

.gantt-card {
  border-radius: 12px;
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

.legend {
  display: flex;
  gap: 16px;
  flex-wrap: wrap;
}

.legend-item {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: #606266;
}

.legend-dot {
  width: 12px;
  height: 12px;
  border-radius: 3px;
}

.legend-dot.pending { background: #909399; }
.legend-dot.scheduled { background: #409eff; }
.legend-dot.running { background: #e6a23c; }
.legend-dot.completed { background: #67c23a; }
.legend-dot.conflict { background: #f56c6c; }

.gantt-chart-container {
  height: 500px;
  width: 100%;
}

:deep(.el-card__header) {
  padding: 16px 20px;
  border-bottom: 1px solid #ebeef5;
}

:deep(.el-card__body) {
  padding: 20px;
}
</style>
