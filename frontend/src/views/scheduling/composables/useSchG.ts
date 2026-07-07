/**
 * useSchG.ts - 排产甘特图核心 composable
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/gantt.vue）
 * 提供甘特图数据加载、对话框状态管理、任务点击处理
 * ECharts 渲染由 SchGChart 子组件内部管理
 * 自动排程业务流程由 useSchGProc 提供
 * 行为完全保持一致（仅结构重构）
 */
import { ref, computed, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import { schedulingApi, type GanttData, type ScheduleTask, type SchedulingParams, type ConflictItem } from '@/api/scheduling'

/**
 * 排产甘特图 composable
 * 集中管理甘特图数据加载、对话框状态、任务点击处理
 */
export function useSchG() {
  // 日期范围
  const dateRange = ref<[Date, Date] | null>(null)
  // 加载状态
  const loading = ref(false)

  // 甘特图数据
  const ganttData = ref<GanttData>({
    work_centers: [],
    date_range: { start: '', end: '' },
    total_tasks: 0,
    conflict_count: 0,
  })

  // 对话框可见性
  const autoScheduleDialogVisible = ref(false)
  const adjustDialogVisible = ref(false)
  const conflictDialogVisible = ref(false)
  const adjusting = ref(false)

  // 调整任务
  const adjustTask = ref<ScheduleTask>({} as ScheduleTask)
  const adjustForm = ref({
    work_center_id: 0,
    start_time: '',
    end_time: '',
  })

  // 自动排程表单
  const scheduleForm = ref<SchedulingParams>({
    start_date: new Date().toISOString().split('T')[0],
    end_date: '',
    priority_mode: 'priority',
    optimization_target: 'balance_load',
  })

  // 冲突列表（v11 批次 181 P2-1 修复：统一使用 API 的 ConflictItem 类型，替代本地不完整接口）
  const conflictList = ref<ConflictItem[]>([])

  // 日期范围文本
  const dateRangeText = computed(() => {
    if (ganttData.value.date_range.start && ganttData.value.date_range.end) {
      return `${ganttData.value.date_range.start.slice(5)} ~ ${ganttData.value.date_range.end.slice(5)}`
    }
    const today = new Date()
    const end = new Date()
    end.setDate(end.getDate() + 30)
    return `${today.getMonth() + 1}/${today.getDate()} ~ ${end.getMonth() + 1}/${end.getDate()}`
  })

  /** 加载甘特图数据 */
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
    } catch (error: unknown) {
      // v11 批次 181 P2-1 修复：catch (error: any) 改为 catch (error: unknown) + 类型守卫
      const errMsg = error instanceof Error ? error.message : String(error)
      ElMessage.error(errMsg || '获取甘特图数据失败')
      // v11 批次 181 P2-1 修复：null as any 改为具体类型的空对象
      ganttData.value = {
        work_centers: [],
        date_range: { start: '', end: '' },
        total_tasks: 0,
        conflict_count: 0,
      }
    } finally {
      loading.value = false
    }
  }

  /** 任务点击 → 打开调整对话框 */
  const handleTaskClick = (task: ScheduleTask) => {
    adjustTask.value = task
    adjustForm.value = {
      work_center_id: task.work_center_id,
      start_time: task.start_time,
      end_time: task.end_time,
    }
    adjustDialogVisible.value = true
  }

  /** 确认调整 */
  const confirmAdjust = async () => {
    adjusting.value = true
    try {
      await schedulingApi.adjustTask(adjustTask.value.id, {
        start_time: adjustForm.value.start_time,
        end_time: adjustForm.value.end_time,
        work_center_id: adjustForm.value.work_center_id,
      })
      ElMessage.success('排程调整成功')
      adjustDialogVisible.value = false
      await fetchGanttData()
    } catch (error: unknown) {
      // v11 批次 181 P2-1 修复：catch (error: any) 改为 catch (error: unknown) + 类型守卫
      const errMsg = error instanceof Error ? error.message : String(error)
      ElMessage.error(errMsg || '排程调整失败')
    } finally {
      adjusting.value = false
    }
  }

  /** 打开自动排程对话框 */
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

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 数据
    dateRange,
    loading,
    ganttData,
    dateRangeText,
    // 对话框
    autoScheduleDialogVisible,
    adjustDialogVisible,
    conflictDialogVisible,
    adjusting,
    adjustTask,
    adjustForm,
    scheduleForm,
    conflictList,
    // 方法
    fetchGanttData,
    handleTaskClick,
    confirmAdjust,
    handleAutoSchedule,
  })
}
