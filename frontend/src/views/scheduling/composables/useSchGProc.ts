/**
 * useSchGProc.ts - 排产甘特图业务流程 composable
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/gantt.vue）
 * 提供排产自动排程的执行 + 冲突展示流程
 * 行为完全保持一致（仅结构重构）
 */
import { reactive, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { schedulingApi, type SchedulingParams } from '@/api/scheduling'

// v11 批次 181 P2-1 修复：定义 deps 类型替代 any
// 注：deps 字段来自 useSchG 返回的 reactive 对象（ref 被解包），
// 但原代码以 .value 访问，保持行为一致使用 unknown + 类型断言
interface SchGConflictItem {
  work_center_name: string
  order_no_1: string
  order_no_2: string
  overlap_start: string
  overlap_end: string
  severity: string
  suggestion: string
}

interface SchGDeps {
  fetchGanttData: () => Promise<void>
  conflictList: unknown
  conflictDialogVisible: unknown
  scheduleForm: unknown
  autoScheduleDialogVisible: unknown
}

/**
 * 排产自动排程流程 composable
 * 集中管理"执行自动排程"和"展示冲突列表"两个流程
 */
export function useSchGProc(deps: SchGDeps) {
  // 自动排程进行中
  const scheduling = ref(false)

  /** 确认执行自动排程 */
  const confirmAutoSchedule = async () => {
    scheduling.value = true
    try {
      const res = await schedulingApi.autoSchedule(deps.scheduleForm as SchedulingParams)
      const result = res.data!
      ElMessage.success(
        `排程完成: ${result.scheduled_count} 个任务已排程, ${result.conflict_count} 个冲突`
      )
      ;(deps.autoScheduleDialogVisible as { value: boolean }).value = false
      if (result.conflict_count > 0) {
        ;(deps.conflictList as { value: SchGConflictItem[] }).value = result.conflicts
        ;(deps.conflictDialogVisible as { value: boolean }).value = true
      }
      await deps.fetchGanttData()
    } catch (error: unknown) {
      // v11 批次 181 P2-1 修复：catch (error: any) 改为 catch (error: unknown) + 类型守卫
      const errMsg = error instanceof Error ? error.message : String(error)
      ElMessage.error(errMsg || '自动排程失败')
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
