<!--
  SchGChart.vue - 排产甘特图容器（内部管理 ECharts 渲染）
  任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/gantt.vue）
-->
<template>
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
    <div ref="chartRef" v-loading="loading" class="gantt-chart-container"></div>
  </el-card>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount, nextTick } from 'vue'
import * as echarts from 'echarts'
import type {
  ECharts,
  ECElementEvent,
  CallbackDataParams,
  CustomSeriesRenderItemParams,
  CustomSeriesRenderItemAPI,
} from 'echarts'
import type { GanttData, ScheduleTask } from '@/api/scheduling'
import { statusColorMap, statusLabelMap, formatTime } from '../composables/schGFmts'

/// 甘特图自定义 series 数据项（业务数据结构，附加 taskData 用于回调访问）
interface GanttSeriesItem {
  name: string
  value: [number, number, number, number]
  itemStyle: { color: string }
  taskData: ScheduleTask
}

// 排产甘特图容器属性
const props = defineProps<{
  // 甘特图数据
  ganttData: GanttData
  // 加载状态
  loading: boolean
}>()

// 定义事件
const emit = defineEmits<{
  // 任务点击
  (e: 'task-click', task: ScheduleTask): void
}>()

// 图表 DOM 引用
const chartRef = ref<HTMLElement | null>(null)
// ECharts 实例
let chart: ECharts | null = null

/** 渲染甘特图 */
const renderChart = (data: GanttData) => {
  if (!chartRef.value) return
  if (!chart) {
    chart = echarts.init(chartRef.value)
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

  const seriesData: GanttSeriesItem[] = []
  data.work_centers.forEach(wc => {
    wc.tasks.forEach(task => {
      const start = new Date(task.start_time).getTime()
      const end = new Date(task.end_time).getTime()
      const color = task.has_conflict ? statusColorMap.conflict : statusColorMap[task.status]
      seriesData.push({
        name: task.order_no,
        value: [categories.indexOf(wc.name), start, end, task.duration_hours],
        itemStyle: { color },
        taskData: task,
      })
    })
  })

  const option = {
    tooltip: {
      formatter: (params: CallbackDataParams) => {
        const item = params.data as GanttSeriesItem
        const t = item.taskData
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
      },
    },
    grid: { left: 120, right: 40, top: 40, bottom: 40, containLabel: false },
    xAxis: {
      type: 'category',
      data: dates,
      axisLine: { lineStyle: { color: '#dcdfe6' } },
      axisLabel: { color: '#606266', rotate: 45 },
    },
    yAxis: {
      type: 'category',
      data: categories,
      axisLine: { lineStyle: { color: '#dcdfe6' } },
      axisLabel: { color: '#303133', fontWeight: 600 },
      inverse: true,
    },
    dataZoom: [
      { type: 'slider', xAxisIndex: 0, start: 0, end: 100, bottom: 10, height: 20 },
      { type: 'inside', xAxisIndex: 0, start: 0, end: 100 },
    ],
    series: [
      {
        type: 'custom',
        renderItem: (params: CustomSeriesRenderItemParams, api: CustomSeriesRenderItemAPI) => {
          const catIndex = api.value(0) as number
          const start = api.coord([api.value(1) as number, catIndex])
          const end = api.coord([api.value(2) as number, catIndex])
          const height = api.size([0, 1])[1] * 0.6
          const rectShape = echarts.graphic.clipRectByRect(
            {
              x: start[0],
              y: start[1] - height / 2,
              width: end[0] - start[0],
              height: height,
            },
            {
              x: params.coordSys.x,
              y: params.coordSys.y,
              width: params.coordSys.width,
              height: params.coordSys.height,
            }
          )
          return (
            rectShape && {
              type: 'rect',
              transition: ['shape'],
              shape: rectShape,
              style: api.style(),
            }
          )
        },
        encode: {
          x: [1, 2],
          y: 0,
        },
        data: seriesData,
        itemStyle: { borderRadius: 4 },
      },
    ],
  }

  chart.setOption(option, true)

  chart.on('click', (params: ECElementEvent) => {
    // ECElementEvent.data 类型为 unknown，断言为业务数据项（seriesData 由本组件构造，类型确定）
    const item = params.data as GanttSeriesItem | undefined
    if (item?.taskData) {
      emit('task-click', item.taskData)
    }
  })
}

/** resize 处理 */
const handleResize = () => chart?.resize()

/** 监听数据变化重新渲染 */
watch(
  () => props.ganttData,
  (newData) => {
    if (newData) {
      nextTick(() => renderChart(newData))
    }
  },
  { deep: true }
)

onMounted(() => {
  nextTick(() => {
    if (props.ganttData) renderChart(props.ganttData)
  })
  window.addEventListener('resize', handleResize)
})

onBeforeUnmount(() => {
  chart?.dispose()
  chart = null
  window.removeEventListener('resize', handleResize)
})
</script>

<style scoped>
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

.legend-dot.pending {
  background: #909399;
}
.legend-dot.scheduled {
  background: #409eff;
}
.legend-dot.running {
  background: #e6a23c;
}
.legend-dot.completed {
  background: #67c23a;
}
.legend-dot.conflict {
  background: #f56c6c;
}

.gantt-chart-container {
  height: 500px;
  width: 100%;
}
</style>
