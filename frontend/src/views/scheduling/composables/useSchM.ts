/**
 * useSchM.ts - 排产管理核心 composable
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/index.vue）
 * 提供工单列表查询、冲突检测、统计计算、表单管理等核心方法
 * 自动排程业务流程由 useSchMProc 提供
 * 行为完全保持一致（仅结构重构）
 */
import { ref, reactive } from 'vue'
import { ElMessage } from 'element-plus'
import {
  schedulingApi,
  type ScheduleTask,
  type ConflictItem,
  type SchedulingParams,
} from '@/api/scheduling'
import { loadIfNot, createLazyLoader } from '@/utils/lazy-loader'

/**
 * 排产管理 composable
 * 集中管理工单列表、冲突列表、统计、对话框的业务状态
 */
export function useSchM() {
  // 状态
  const adjusting = ref(false)
  const taskLoading = ref(false)
  const conflictLoading = ref(false)
  const currentPage = ref(1)
  const pageSize = ref(10)
  const total = ref(0)
  const filterStatus = ref('')

  // 统计数据
  const stats = ref({
    pending: 0,
    scheduled: 0,
    running: 0,
    conflicts: 0,
  })

  // 列表数据
  const taskList = ref<ScheduleTask[]>([])
  const conflictList = ref<ConflictItem[]>([])

  // 日期范围
  const dateRange = ref<[Date, Date] | null>(null)

  // 自动排程参数
  const scheduleParams = ref<SchedulingParams>({
    start_date: '',
    end_date: '',
    priority_mode: 'priority',
    optimization_target: 'balance_load',
  })

  // 调整对话框
  const adjustDialogVisible = ref(false)
  const adjustTask = ref<ScheduleTask | null>(null)
  const adjustForm = ref({
    start_time: '',
    end_time: '',
  })

  // 懒加载标记
  const hasLoaded = createLazyLoader()

  /** 获取工单列表 */
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
    } catch (error: unknown) {
      // v11 批次 181 P2-1 修复：catch (error: any) 改为 catch (error: unknown) + 类型守卫
      const errMsg = error instanceof Error ? error.message : String(error)
      ElMessage.error(errMsg || '获取排程任务失败')
      taskList.value = []
      total.value = 0
    } finally {
      taskLoading.value = false
    }
  }

  /** 更新统计 */
  const updateStats = () => {
    stats.value.pending = taskList.value.filter(t => t.status === 'pending').length
    stats.value.scheduled = taskList.value.filter(t => t.status === 'scheduled').length
    stats.value.running = taskList.value.filter(t => t.status === 'running').length
    stats.value.conflicts = conflictList.value.length
  }

  /** 获取冲突列表 */
  const fetchConflicts = async () => {
    conflictLoading.value = true
    try {
      const res = await schedulingApi.detectConflicts()
      conflictList.value = res.data!
      stats.value.conflicts = conflictList.value.length
    } catch (error: unknown) {
      // v11 批次 181 P2-1 修复：catch (error: any) 改为 catch (error: unknown) + 类型守卫
      const errMsg = error instanceof Error ? error.message : String(error)
      ElMessage.error(errMsg || '获取冲突列表失败')
      conflictList.value = []
    } finally {
      conflictLoading.value = false
    }
  }

  /** 打开调整对话框 */
  const handleAdjust = (task: ScheduleTask) => {
    adjustTask.value = task
    adjustForm.value = {
      start_time: task.start_time,
      end_time: task.end_time,
    }
    adjustDialogVisible.value = true
  }

  /** 确认调整 */
  const confirmAdjust = async (): Promise<boolean> => {
    if (!adjustTask.value) return false
    adjusting.value = true
    try {
      await schedulingApi.adjustTask(adjustTask.value.id, {
        start_time: adjustForm.value.start_time,
        end_time: adjustForm.value.end_time,
      })
      ElMessage.success('排程调整成功')
      adjustDialogVisible.value = false
      await fetchTasks()
      return true
    } catch (error: unknown) {
      // v11 批次 181 P2-1 修复：catch (error: any) 改为 catch (error: unknown) + 类型守卫
      const errMsg = error instanceof Error ? error.message : String(error)
      ElMessage.error(errMsg || '排程调整失败')
      return false
    } finally {
      adjusting.value = false
    }
  }

  /** 显示冲突详情（轻量提示） */
  const showConflictDetail = (task: ScheduleTask) => {
    ElMessage.warning(`工单 ${task.order_no} 存在排程冲突: ${task.conflict_details || '时间重叠'}`)
  }

  /** 初始化加载（懒加载冲突） */
  const initLoad = () => {
    fetchTasks()
    loadIfNot('conflicts', fetchConflicts, hasLoaded)
  }

  // 使用 reactive 包装，访问字段时自动解包 ref
  return reactive({
    // 状态
    adjusting,
    taskLoading,
    conflictLoading,
    currentPage,
    pageSize,
    total,
    filterStatus,
    // 统计
    stats,
    // 列表
    taskList,
    conflictList,
    // 日期与排程参数
    dateRange,
    scheduleParams,
    // 调整对话框
    adjustDialogVisible,
    adjustTask,
    adjustForm,
    // 方法
    fetchTasks,
    fetchConflicts,
    handleAdjust,
    confirmAdjust,
    showConflictDetail,
    initLoad,
  })
}
