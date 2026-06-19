/**
 * useSchGProc.ts - 排产甘特图业务流程 composable
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/gantt.vue）
 * 提供排产自动排程的执行 + 冲突展示流程
 * 行为完全保持一致（仅结构重构）
 */
import { reactive, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { schedulingApi, type SchedulingParams } from '@/api/scheduling'

/**
 * 排产自动排程流程 composable
 * 集中管理"执行自动排程"和"展示冲突列表"两个流程
 */
export function useSchGProc(deps: {
  fetchGanttData: () => Promise<void>
  conflictList: any
  conflictDialogVisible: any
  scheduleForm: any
  autoScheduleDialogVisible: any
}) {
  // 自动排程进行中
  const scheduling = ref(false)

  /** 确认执行自动排程 */
  const confirmAutoSchedule = async () => {
    scheduling.value = true
    try {
      const res = await schedulingApi.autoSchedule(deps.scheduleForm)
      const result = res.data!
      ElMessage.success(
        `排程完成: ${result.scheduled_count} 个任务已排程, ${result.conflict_count} 个冲突`
      )
      deps.autoScheduleDialogVisible.value = false
      if (result.conflict_count > 0) {
        deps.conflictList.value = result.conflicts
        deps.conflictDialogVisible.value = true
      }
      await deps.fetchGanttData()
    } catch (error: any) {
      ElMessage.error(error.message || '自动排程失败')
    } finally {
      scheduling.value = false
    }
  }

  // 使用 reactive 包装
  return reactive({
    scheduling,
    confirmAutoSchedule,
  })
}
