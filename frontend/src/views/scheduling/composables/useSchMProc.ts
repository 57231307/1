/**
 * useSchMProc.ts - 排产管理自动排程流程 composable
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/index.vue）
 * 提供排产自动排程的执行流程（参数构造 + API 调用 + 冲突更新）
 * 行为完全保持一致（仅结构重构）
 */
import { reactive, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { schedulingApi } from '@/api/scheduling'

/**
 * 排产管理自动排程流程 composable
 */
export function useSchMProc(deps: {
  fetchTasks: () => Promise<void>
  dateRange: any
  scheduleParams: any
  conflictList: any
  stats: any
}) {
  // 自动排程进行中
  const scheduling = ref(false)

  /** 准备排程参数（基于日期范围或默认 +30 天） */
  const prepareScheduleParams = () => {
    if (deps.dateRange.value && deps.dateRange.value.length === 2) {
      deps.scheduleParams.value.start_date = deps.dateRange.value[0].toISOString().split('T')[0]
      deps.scheduleParams.value.end_date = deps.dateRange.value[1].toISOString().split('T')[0]
    } else {
      const today = new Date()
      const end = new Date()
      end.setDate(end.getDate() + 30)
      deps.scheduleParams.value.start_date = today.toISOString().split('T')[0]
      deps.scheduleParams.value.end_date = end.toISOString().split('T')[0]
    }
  }

  /** 执行自动排程 */
  const handleAutoSchedule = async () => {
    scheduling.value = true
    try {
      prepareScheduleParams()
      const res = await schedulingApi.autoSchedule(deps.scheduleParams.value)
      const result = res.data!
      ElMessage.success(`排程完成: ${result.scheduled_count} 个任务, ${result.conflict_count} 个冲突`)
      if (result.conflict_count > 0) {
        deps.conflictList.value = result.conflicts
        deps.stats.value.conflicts = result.conflict_count
      }
      await deps.fetchTasks()
    } catch (error: any) {
      ElMessage.error(error.message || '自动排程失败')
    } finally {
      scheduling.value = false
    }
  }

  // 使用 reactive 包装
  return reactive({
    scheduling,
    handleAutoSchedule,
  })
}
